//! Open-time draft recovery, the recovery dialog's resolution and the
//! content-loss guard (`recoverDraft`, `onRecoveryResolved`,
//! `*ContentLoss*` in `autosave_controller.js`).
//!
//! Conflict copies (`frankmd:draft-conflict:*`) are only produced by the
//! engine's path remap, which a DocId-keyed session never runs, so the
//! recovery flow here offers drafts and legacy backups only.

use std::time::Instant;

use frankmd_core::DocId;
use frankmd_core::guard::is_large_deletion;

use super::{AutosaveSession, ContentLossState, PendingRecovery};
use crate::autosave::{AutosaveEffect, RecoveryRequest, RecoveryResolution, RecoverySource, SaveStatus};
use crate::draft_storage::{DraftBackend, LegacyBackup};
use crate::drafts::draft_subject;

/// The local copy a recovery dialog offers.
struct Offer {
    server_content: String,
    content: String,
    timestamp: f64,
    source: RecoverySource,
    draft_revision: Option<String>,
}

impl<B: DraftBackend> AutosaveSession<B> {
    // ----- recovery ------------------------------------------------------

    /// `recoverDraft(serverContent, serverRevision)`: returns the text to show.
    /// Same baseline → auto-restore (dirty); draft equals server → drop it;
    /// diverged → recovery dialog; a newer, different legacy backup → dialog.
    pub fn recover_draft(&mut self, server_content: &str, server_revision: &str) -> String {
        let Some(doc) = self.current_doc() else {
            return server_content.to_owned();
        };
        let subject = draft_subject(doc);
        let draft = match self.drafts.read_draft(&subject) {
            Ok(read) => read.draft,
            Err(_) => {
                self.show_draft_storage_error();
                if let Some(backup) = self.read_legacy_backup(doc, server_content) {
                    self.open_backup_recovery(doc, server_content, backup, None);
                }
                return server_content.to_owned();
            }
        };
        let backup = self.read_legacy_backup(doc, server_content);
        let Some(draft) = draft else {
            if let Some(backup) = backup {
                self.open_backup_recovery(doc, server_content, backup, None);
            }
            return server_content.to_owned();
        };

        // Drafts are the source of truth; a legacy backup wins only as an
        // explicit choice when it is a different, later snapshot.
        if let Some(backup) = backup {
            if backup.content != draft.content && backup.timestamp > draft.updated_at {
                self.open_backup_recovery(doc, server_content, backup, Some(draft.draft_revision));
                return server_content.to_owned();
            }
            if self.drafts.remove_backup(&subject).is_err() {
                self.show_draft_storage_error();
            }
        }

        if draft.content == server_content {
            let _ = self.remove_draft_if_revision(doc, &draft.draft_revision);
            return server_content.to_owned();
        }
        if draft.base_revision == server_revision {
            self.draft_revision = Some(draft.draft_revision);
            self.has_unsaved_changes = true;
            self.set_status(SaveStatus::Unsaved);
            if is_large_deletion(Some(server_content), &draft.content) {
                self.show_content_loss_warning();
            }
            return draft.content;
        }
        self.open_recovery(
            doc,
            Offer {
                server_content: server_content.to_owned(),
                content: draft.content,
                timestamp: draft.updated_at,
                source: RecoverySource::Draft,
                draft_revision: Some(draft.draft_revision),
            },
        );
        server_content.to_owned()
    }

    /// `checkOfflineBackup(serverContent)` against the legacy backup.
    pub fn check_offline_backup(&mut self, server_content: &str) {
        let Some(doc) = self.current_doc() else {
            return;
        };
        if let Some(backup) = self.drafts.backup_check(&draft_subject(doc), server_content) {
            self.open_backup_recovery(doc, server_content, backup, None);
        }
    }

    /// `onRecoveryResolved(detail)`.
    pub fn recovery_resolved(&mut self, resolution: RecoveryResolution, now: Instant) {
        let doc = self.current_doc();
        if resolution.conflict_id.is_some() {
            // No conflict copy exists for a DocId session (see module docs).
            self.show_draft_storage_error();
            return;
        }
        if let (Some(revision), Some(doc)) = (resolution.draft_revision.as_deref(), doc) {
            match self.drafts.read_draft(&draft_subject(doc)) {
                Err(_) => {
                    self.show_draft_storage_error();
                    return;
                }
                Ok(read) if read.draft.as_ref().is_some_and(|d| d.draft_revision != revision) => {
                    // The local draft changed while the dialog was open.
                    self.show_draft_storage_error();
                    return;
                }
                Ok(_) => {}
            }
        }

        match (resolution.source, resolution.content.as_deref()) {
            (RecoverySource::Server, _) => {
                if let (Some(doc), Some(revision)) = (doc, resolution.draft_revision.as_deref())
                    && self.remove_draft_if_revision(doc, revision).is_err()
                {
                    return;
                }
                self.remove_selected_backup(doc, &resolution);
                if resolution.draft_revision == self.draft_revision {
                    self.draft_revision = None;
                }
                self.pending_recovery = None;
                self.has_unsaved_changes = false;
                self.set_status(SaveStatus::Hidden);
            }
            (RecoverySource::Backup | RecoverySource::Draft, Some(content)) => {
                let Some(doc) = doc else {
                    return;
                };
                self.pending_recovery = None;
                let content = content.to_owned();
                self.effects.push(AutosaveEffect::SetEditorText(content.clone()));
                self.set_editor_text(&content);
                self.draft_writes.remove(&doc);
                self.has_unsaved_changes = true;
                self.set_content_loss_override(false);

                let warn = is_large_deletion(self.last_saved_content.as_deref(), &content);
                let base = self.base_revision.clone();
                match self.flush_draft_write(doc, &content, base.as_deref()) {
                    Ok(Some(written)) => {
                        self.draft_revision = Some(written.draft_revision);
                        self.remove_selected_backup(Some(doc), &resolution);
                    }
                    _ => {
                        // Keep the selected source intact if it could not be
                        // migrated into the versioned draft store.
                        if let Some(revision) = resolution.draft_revision {
                            self.draft_revision = Some(revision);
                        }
                    }
                }
                if warn {
                    self.show_content_loss_warning();
                    self.clear_pending_timers();
                } else {
                    self.schedule_autosave(now);
                }
            }
            _ => {}
        }
    }

    fn remove_selected_backup(&mut self, doc: Option<DocId>, resolution: &RecoveryResolution) {
        let (Some(doc), Some(content), Some(timestamp)) =
            (doc, resolution.backup_content.as_deref(), resolution.backup_timestamp)
        else {
            return;
        };
        if timestamp.is_finite()
            && self.drafts.remove_backup_if_snapshot(&draft_subject(doc), content, timestamp).is_err()
        {
            self.show_draft_storage_error();
        }
    }

    /// The legacy backup unless it equals the server text (then it is removed).
    fn read_legacy_backup(&mut self, doc: DocId, server_content: &str) -> Option<LegacyBackup> {
        let subject = draft_subject(doc);
        let backup = match self.drafts.read_backup(&subject) {
            Ok(read) => read.backup?,
            Err(_) => {
                self.show_draft_storage_error();
                return None;
            }
        };
        if backup.content == server_content {
            if self.drafts.remove_backup(&subject).is_err() {
                self.show_draft_storage_error();
            }
            return None;
        }
        Some(backup)
    }

    fn open_backup_recovery(
        &mut self,
        doc: DocId,
        server_content: &str,
        backup: LegacyBackup,
        draft_revision: Option<String>,
    ) {
        let offer = Offer {
            server_content: server_content.to_owned(),
            content: backup.content,
            timestamp: backup.timestamp,
            source: RecoverySource::Backup,
            draft_revision,
        };
        self.open_recovery(doc, offer);
    }

    /// `openRecovery(...)`: pauses autosave for `doc` until the user decides.
    fn open_recovery(&mut self, doc: DocId, offer: Offer) {
        if self.current_doc() == Some(doc) {
            self.draft_writes.remove(&doc);
            self.clear_pending_timers();
        }
        let request = RecoveryRequest {
            path: self.current_file().unwrap_or_default().to_owned(),
            server_content: offer.server_content,
            backup_content: offer.content,
            backup_timestamp: offer.timestamp,
            source: offer.source,
            draft_revision: offer.draft_revision,
            conflict_id: None,
        };
        self.effects.push(AutosaveEffect::OpenRecovery(request));
        self.pending_recovery = Some(PendingRecovery { doc });
    }

    // ----- content-loss guard -------------------------------------------

    /// The guard state for the current note and baseline (reset on mismatch).
    fn content_loss_state(&mut self) -> &mut ContentLossState {
        let doc = self.current_doc();
        if self.content_loss.doc != doc || self.content_loss.base_revision != self.base_revision {
            self.content_loss = ContentLossState {
                doc,
                base_revision: self.base_revision.clone(),
                ..ContentLossState::default()
            };
        }
        &mut self.content_loss
    }

    fn content_loss_is_current(&self) -> bool {
        self.content_loss.doc == self.current_doc() && self.content_loss.base_revision == self.base_revision
    }

    /// `showContentLossWarning()`.
    pub fn show_content_loss_warning(&mut self) {
        self.content_loss_state().warning_active = true;
        self.effects.push(AutosaveEffect::ContentLossBanner(true));
    }

    /// `dismissContentLossWarning()` (also clears the override).
    pub fn dismiss_content_loss_warning(&mut self) {
        self.content_loss_state().warning_active = false;
        self.set_content_loss_override(false);
        self.effects.push(AutosaveEffect::ContentLossBanner(false));
    }

    /// `checkContentRestored(content)`: auto-dismiss once no longer a large deletion.
    pub fn check_content_restored(&mut self, content: &str) {
        if !self.content_loss_warning_active() {
            return;
        }
        let Some(baseline) = self.last_saved_content.as_deref() else {
            return;
        };
        if !is_large_deletion(Some(baseline), content) {
            self.dismiss_content_loss_warning();
        }
    }

    /// `_contentLossOverride = value` (binds the approval to the current editor text).
    pub fn set_content_loss_override(&mut self, value: bool) {
        let content = value.then(|| self.editor_text.clone());
        let state = self.content_loss_state();
        state.override_active = value;
        state.override_content = content;
    }

    /// `undoContentLoss()`: the app ran the editor's undo first and reports
    /// whether it changed anything (and mirrors the text). Without history the
    /// server baseline is reloaded ([`AutosaveEffect::LoadEditorText`]).
    pub fn undo_content_loss(&mut self, undid_editor_change: bool) {
        if !undid_editor_change
            && self.content_loss_warning_active()
            && let Some(baseline) = self.last_saved_content.clone()
        {
            let path = self.current_file().unwrap_or_default().to_owned();
            self.set_editor_text(&baseline);
            self.effects.push(AutosaveEffect::LoadEditorText { content: baseline, path });
        }
        let content = self.editor_text.clone();
        self.check_content_restored(&content);
        if let Some(doc) = self.current_doc() {
            let base = self.base_revision.clone();
            let _ = self.flush_draft_write(doc, &content, base.as_deref());
        }
    }

    /// `saveAnywayAfterWarning()`.
    pub fn save_anyway_after_warning(&mut self) {
        self.dismiss_content_loss_warning();
        self.set_content_loss_override(true);
        self.save_now();
    }

    /// Content-loss banner active for the current file+baseline.
    pub fn content_loss_warning_active(&self) -> bool {
        self.content_loss_is_current() && self.content_loss.warning_active
    }

    /// "Save anyway" approval active for the current editor text.
    pub fn content_loss_override(&self) -> bool {
        self.content_loss_is_current() && self.content_loss.override_active
    }
}
