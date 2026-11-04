//! Autosave policy and the per-editor autosave/draft state machine
//! (**[COMPAT]** port of `autosave_controller.js`, frontend.md B.3/B.5).
//!
//! Pure over time: every timer-driven method takes `now: Instant` (from the
//! injected [`crate::clock::Clock`]); [`AutosaveSession::tick`] fires whatever
//! is due. Side effects the app must perform (HTTP save, status label, dialogs,
//! editor text changes) are queued as [`AutosaveEffect`]s and drained with
//! [`AutosaveSession::take_effects`]. The editor's text is mirrored through
//! [`AutosaveSession::set_editor_text`] (the JS reads `codemirror.getValue()`).

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use crate::draft_storage::{
    DraftBackend, DraftError, DraftRecord, DraftStorage, ItemKind, RemoveDraftsError,
};

/// **[COMPAT]** `SAVE_DEBOUNCE_MS`: wait 2 s after the last keystroke.
pub const SAVE_DEBOUNCE: Duration = Duration::from_millis(2000);
/// **[COMPAT]** `SAVE_MAX_INTERVAL_MS`: force a save every 30 s while typing continuously.
pub const SAVE_MAX_INTERVAL: Duration = Duration::from_millis(30_000);
/// **[COMPAT]** `DRAFT_DEBOUNCE_MS`: local draft write debounce.
pub const DRAFT_DEBOUNCE: Duration = Duration::from_millis(300);
/// **[COMPAT]** offline backup debounce (`scheduleOfflineBackup`).
pub const OFFLINE_BACKUP_DEBOUNCE: Duration = Duration::from_millis(1000);
/// **[COMPAT]** how long the "Saved" status stays before clearing.
pub const SAVED_STATUS_LINGER: Duration = Duration::from_millis(2000);

// ---------------------------------------------------------------------------
// Simple debounce timer (kept for the workspace store)
// ---------------------------------------------------------------------------

/// Debounce state for one document.
#[derive(Debug, Clone, Default)]
pub struct AutosaveTimer {
    last_edit: Option<Instant>,
    first_unsaved_edit: Option<Instant>,
}

/// Policy knobs.
#[derive(Debug, Clone, Copy)]
pub struct AutosavePolicy {
    /// Idle delay after the last keystroke.
    pub debounce: Duration,
    /// Max time a dirty buffer may stay unsaved while typing continuously.
    pub max_wait: Duration,
}

impl Default for AutosavePolicy {
    /// **[COMPAT]** 2000 ms / 30000 ms.
    fn default() -> Self {
        Self { debounce: SAVE_DEBOUNCE, max_wait: SAVE_MAX_INTERVAL }
    }
}

impl AutosaveTimer {
    /// Record an edit at `now`.
    pub fn edited(&mut self, now: Instant) {
        todo!()
    }
    /// Should we save at `now`? True once `debounce` passed since the last
    /// edit, or `max_wait` passed since the first unsaved edit.
    pub fn due(&self, now: Instant, policy: &AutosavePolicy) -> bool {
        todo!()
    }
    /// Earliest instant at which [`Self::due`] becomes true (for scheduling the tick).
    pub fn next_deadline(&self, policy: &AutosavePolicy) -> Option<Instant> {
        todo!()
    }
    /// Reset after a successful save.
    pub fn saved(&mut self) {
        *self = Self::default();
    }
}

// ---------------------------------------------------------------------------
// Session types
// ---------------------------------------------------------------------------

/// Timer configuration (defaults are the **[COMPAT]** constants above).
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct AutosaveConfig {
    pub save_debounce: Duration,
    pub save_max_interval: Duration,
    pub draft_debounce: Duration,
    pub offline_backup_debounce: Duration,
    pub saved_status_linger: Duration,
}

impl Default for AutosaveConfig {
    fn default() -> Self {
        Self {
            save_debounce: SAVE_DEBOUNCE,
            save_max_interval: SAVE_MAX_INTERVAL,
            draft_debounce: DRAFT_DEBOUNCE,
            offline_backup_debounce: OFFLINE_BACKUP_DEBOUNCE,
            saved_status_linger: SAVED_STATUS_LINGER,
        }
    }
}

/// The save-status label (i18n `status.*` / `connection.*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaveStatus {
    /// Hidden / empty.
    #[default]
    Hidden,
    /// `status.unsaved`.
    Unsaved,
    /// `status.saved`.
    Saved,
    /// `status.error_saving` (error style).
    ErrorSaving,
    /// `status.draft_storage_error` (error style): a local draft could not be persisted.
    DraftStorageError,
    /// `connection.disconnected` (error style).
    Disconnected,
}

impl SaveStatus {
    /// Rendered with the error style (`text-red-500`).
    pub fn is_error(self) -> bool {
        matches!(self, Self::ErrorSaving | Self::DraftStorageError | Self::Disconnected)
    }
}

/// Which copy a recovery dialog offers / which one the user picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverySource {
    /// Keep the server version.
    Server,
    /// A versioned local draft.
    Draft,
    /// A preserved conflict copy (rename collision / failed remap).
    DraftConflict,
    /// A legacy offline backup.
    Backup,
}

/// What `recoveryDiff.open(...)` receives.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryRequest {
    /// Note path.
    pub path: String,
    /// Server text.
    pub server_content: String,
    /// Local copy offered (`backupContent`).
    pub backup_content: String,
    /// Its timestamp (`backupTimestamp`, epoch ms).
    pub backup_timestamp: f64,
    /// Kind of local copy.
    pub source: RecoverySource,
    /// Draft revision of the offered/related draft (`null` → `None`).
    pub draft_revision: Option<String>,
    /// Set only for `DraftConflict`.
    pub conflict_id: Option<String>,
}

/// The `recovery-diff:resolved` event detail.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryResolution {
    /// `"server"` or the offered source.
    pub source: RecoverySource,
    /// Path.
    pub path: Option<String>,
    /// Chosen content (only when the local copy was accepted).
    pub content: Option<String>,
    /// Echoed draft revision.
    pub draft_revision: Option<String>,
    /// Echoed offered content.
    pub backup_content: Option<String>,
    /// Echoed offered timestamp.
    pub backup_timestamp: Option<f64>,
    /// Echoed conflict id.
    pub conflict_id: Option<String>,
    /// Server text (only echoed for a conflict resolved to the server).
    pub server_content: Option<String>,
}

impl RecoveryResolution {
    /// `acceptServer()`: `{source:"server", path, draftRevision, backupContent,
    /// backupTimestamp}` plus `conflictId` and `serverContent` for a conflict.
    pub fn accept_server(req: &RecoveryRequest) -> Self {
        todo!()
    }
    /// `acceptBackup()`: `{source: req.source, path, content: backupContent,
    /// backupContent, draftRevision, backupTimestamp}` plus `conflictId` if any.
    pub fn accept_local(req: &RecoveryRequest) -> Self {
        todo!()
    }
}

/// Identifies one in-flight save request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SaveTicket(pub u64);

/// A `PATCH /notes/<path>` the app must perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRequest {
    /// Ticket to report back with.
    pub ticket: SaveTicket,
    /// Note path at capture time.
    pub path: String,
    /// Body `{content}`.
    pub content: String,
    /// Baseline the snapshot was based on.
    pub base_revision: Option<String>,
}

/// How a save request ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    /// 2xx; `revision` = server's new `sha256` if the response carried one.
    Saved {
        /// New server revision.
        revision: Option<String>,
    },
    /// Non-2xx or network error.
    Failed,
}

/// Side effects for the app.
#[derive(Debug, Clone, PartialEq)]
pub enum AutosaveEffect {
    /// Perform this save and call [`AutosaveSession::save_finished`].
    Save(SaveRequest),
    /// Update the save-status label.
    Status(SaveStatus),
    /// Show/hide the content-loss banner.
    ContentLossBanner(bool),
    /// Arm/disarm the "unsaved draft" unload/close warning (`beforeunload`).
    UnloadGuard(bool),
    /// Open the recovery dialog.
    OpenRecovery(RecoveryRequest),
    /// Replace the editor text (`codemirror.setValue`).
    SetEditorText(String),
    /// Reload the editor with a fresh history (`codemirror.loadContent(content, path)`).
    LoadEditorText {
        /// Text.
        content: String,
        /// Path.
        path: String,
    },
    /// Legacy offline backup write (`offlineBackup.save`).
    OfflineBackup {
        /// Path.
        path: String,
        /// Text.
        content: String,
    },
    /// Legacy offline backup clear after a successful save.
    ClearOfflineBackup {
        /// Path.
        path: String,
    },
    /// `offline-changed` dispatch.
    OfflineChanged(bool),
    /// `.fed` was saved (`config-saved` dispatch).
    ConfigSaved,
}

/// Result of [`AutosaveSession::prepare_for_file_deletion`].
#[derive(Debug, Clone, PartialEq)]
pub enum DeletionPrep {
    /// Safe to `DELETE …?expected_revision=<revision>`.
    Ready {
        /// Latest server-acknowledged revision.
        revision: String,
    },
    /// A save is in flight; call [`AutosaveSession::resume_file_deletion`]
    /// after its [`AutosaveSession::save_finished`].
    AwaitSave,
    /// The active note changed meanwhile (`{ok:false, stale:true}`).
    Stale,
    /// Draft persistence failed or no known revision.
    Failed(DraftError),
}

// ---------------------------------------------------------------------------
// The session
// ---------------------------------------------------------------------------

/// Autosave + local-draft state machine for one editor.
pub struct AutosaveSession<B: DraftBackend> {
    drafts: DraftStorage<B>,
    config: AutosaveConfig,
    effects: Vec<AutosaveEffect>,
    editor_text: String,
    current_file: Option<String>,
    last_saved_content: Option<String>,
    base_revision: Option<String>,
    draft_revision: Option<String>,
    has_unsaved_changes: bool,
    is_offline: bool,
    file_version: u64,
    in_flight: Option<SaveTicket>,
    save_deadline: Option<Instant>,
    max_interval_deadline: Option<Instant>,
    offline_backup_deadline: Option<Instant>,
    draft_write_deadlines: BTreeMap<String, Instant>,
    known_base_revisions: BTreeMap<String, String>,
    draft_persistence_failures: BTreeSet<String>,
    draft_blocked_paths: BTreeSet<String>,
    pending_remap_recoveries: BTreeMap<String, String>,
    pending_recovery: Option<(String, Option<String>)>,
    content_loss_warning: bool,
    content_loss_override: Option<String>,
    status: SaveStatus,
}

impl<B: DraftBackend> std::fmt::Debug for AutosaveSession<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutosaveSession").field("current_file", &self.current_file).finish_non_exhaustive()
    }
}

impl<B: DraftBackend> AutosaveSession<B> {
    /// New session with no file.
    pub fn new(drafts: DraftStorage<B>, config: AutosaveConfig) -> Self {
        Self {
            drafts,
            config,
            effects: Vec::new(),
            editor_text: String::new(),
            current_file: None,
            last_saved_content: None,
            base_revision: None,
            draft_revision: None,
            has_unsaved_changes: false,
            is_offline: false,
            file_version: 0,
            in_flight: None,
            save_deadline: None,
            max_interval_deadline: None,
            offline_backup_deadline: None,
            draft_write_deadlines: BTreeMap::new(),
            known_base_revisions: BTreeMap::new(),
            draft_persistence_failures: BTreeSet::new(),
            draft_blocked_paths: BTreeSet::new(),
            pending_remap_recoveries: BTreeMap::new(),
            pending_recovery: None,
            content_loss_warning: false,
            content_loss_override: None,
            status: SaveStatus::Hidden,
        }
    }

    // ----- effects / drafts access -------------------------------------

    /// Drain queued effects.
    pub fn take_effects(&mut self) -> Vec<AutosaveEffect> {
        std::mem::take(&mut self.effects)
    }
    /// Draft store.
    pub fn drafts(&self) -> &DraftStorage<B> {
        &self.drafts
    }
    /// Mutable draft store (tests seed records and faults).
    pub fn drafts_mut(&mut self) -> &mut DraftStorage<B> {
        &mut self.drafts
    }

    // ----- editor mirror -------------------------------------------------

    /// Mirror the editor text without scheduling anything (programmatic change).
    pub fn set_editor_text(&mut self, text: &str) {
        text.clone_into(&mut self.editor_text);
    }
    /// Current mirrored editor text.
    pub fn editor_text(&self) -> &str {
        &self.editor_text
    }
    /// A user edit: mirror + [`Self::schedule_autosave`].
    pub fn editor_changed(&mut self, text: &str, now: Instant) {
        todo!()
    }

    // ----- file lifecycle ------------------------------------------------

    /// `setFile(path, content, revision)`: a note was loaded (resets dirty,
    /// warning state and draft revision; remembers the server revision).
    pub fn set_file(&mut self, path: &str, content: &str, revision: Option<&str>) {
        todo!()
    }
    /// `clearFile()`.
    pub fn clear_file(&mut self) {
        todo!()
    }
    /// `prepareForTransition()`: flush the outgoing draft synchronously, then
    /// cancel timers. On failure the file stays active and timers keep running;
    /// a dirty file without a server revision cannot be anchored → error.
    pub fn prepare_for_transition(&mut self) -> Result<Option<DraftRecord>, DraftError> {
        todo!()
    }
    /// `prepareForFileDeletion(path)` (first step).
    pub fn prepare_for_file_deletion(&mut self, path: &str) -> DeletionPrep {
        todo!()
    }
    /// Continue a [`DeletionPrep::AwaitSave`] after the in-flight save finished.
    pub fn resume_file_deletion(&mut self, path: &str) -> DeletionPrep {
        todo!()
    }
    /// `renameFile(old, new, kind)`: keep autosave attached through a rename,
    /// remap drafts, re-target a pending save. Returns whether the active file moved.
    pub fn rename_file(&mut self, old: &str, new: &str, kind: ItemKind, now: Instant) -> bool {
        todo!()
    }
    /// `deleteFile(path, kind)`: remove drafts/backups and invalidate state if
    /// the active note is affected (even when cleanup fails).
    pub fn delete_file(&mut self, path: &str, kind: ItemKind) -> Result<usize, RemoveDraftsError> {
        todo!()
    }
    /// `resumeAfterTransition()`: a rejected rename/delete lets the active file resume.
    pub fn resume_after_transition(&mut self, now: Instant) {
        todo!()
    }

    // ----- timers --------------------------------------------------------

    /// `scheduleAutoSave()`: on every editor change.
    pub fn schedule_autosave(&mut self, now: Instant) {
        todo!()
    }
    /// `scheduleDraftWrite()`: debounce a local draft write for the current file.
    pub fn schedule_draft_write(&mut self, now: Instant) {
        todo!()
    }
    /// `scheduleOfflineBackup()`: only while offline.
    pub fn schedule_offline_backup(&mut self, now: Instant) {
        todo!()
    }
    /// Fire every timer due at `now` (save debounce, max interval, draft
    /// writes, offline backup, status linger).
    pub fn tick(&mut self, now: Instant) {
        todo!()
    }
    /// Earliest pending deadline (the app arms one timer for it).
    pub fn next_deadline(&self) -> Option<Instant> {
        todo!()
    }

    // ----- saving --------------------------------------------------------

    /// `saveNow()`: emits [`AutosaveEffect::Save`] unless offline, recovering,
    /// blocked, unchanged, already saving, or a large deletion (→ banner + draft).
    pub fn save_now(&mut self, now: Instant) {
        todo!()
    }
    /// Report the outcome of a save request. Stale tickets (file renamed,
    /// deleted or switched since) never touch the active state.
    pub fn save_finished(&mut self, ticket: SaveTicket, outcome: SaveOutcome, now: Instant) {
        todo!()
    }

    // ----- drafts --------------------------------------------------------

    /// `flushDraftWrite(path, content, baseRevision)`.
    pub fn flush_draft_write(
        &mut self,
        path: &str,
        content: &str,
        base_revision: Option<&str>,
    ) -> Result<Option<DraftRecord>, DraftError> {
        todo!()
    }
    /// `writeDraftSnapshot({path, content, baseRevision})`. Blocked paths fail.
    pub fn write_draft_snapshot(
        &mut self,
        path: &str,
        content: &str,
        base_revision: &str,
    ) -> Result<Option<DraftRecord>, DraftError> {
        todo!()
    }
    /// `flushActiveDraft()`: window hidden / `pagehide` / app losing focus.
    pub fn flush_active_draft(&mut self) -> Result<Option<DraftRecord>, DraftError> {
        todo!()
    }
    /// `beforeunload`/window close: flush, and return `true` when the close
    /// must be warned about (the active draft could not be persisted).
    pub fn before_unload(&mut self) -> bool {
        todo!()
    }

    // ----- recovery ------------------------------------------------------

    /// `recoverDraft(serverContent, serverRevision)`: returns the text to show.
    /// Same baseline → auto-restore (dirty); draft equals server → drop it;
    /// diverged → recovery dialog; conflicts / newer legacy backup → dialog.
    pub fn recover_draft(&mut self, server_content: &str, server_revision: &str) -> String {
        todo!()
    }
    /// `checkOfflineBackup(serverContent)` against the legacy backup.
    pub fn check_offline_backup(&mut self, server_content: &str) {
        todo!()
    }
    /// `onRecoveryResolved(detail)`.
    pub fn recovery_resolved(&mut self, resolution: RecoveryResolution, now: Instant) {
        todo!()
    }
    /// `resolveDraftConflict(...)`.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_draft_conflict(
        &mut self,
        path: &str,
        conflict_id: &str,
        draft_revision: &str,
        content: Option<&str>,
        server_content: Option<&str>,
        accepted: bool,
        now: Instant,
    ) {
        todo!()
    }

    // ----- connection ----------------------------------------------------

    /// `onConnectionLost()`.
    pub fn connection_lost(&mut self) {
        todo!()
    }
    /// `onConnectionRestored()`.
    pub fn connection_restored(&mut self, now: Instant) {
        todo!()
    }

    // ----- content-loss guard -------------------------------------------

    /// `showContentLossWarning()`.
    pub fn show_content_loss_warning(&mut self) {
        todo!()
    }
    /// `dismissContentLossWarning()` (also clears the override).
    pub fn dismiss_content_loss_warning(&mut self) {
        todo!()
    }
    /// `checkContentRestored(content)`: auto-dismiss once no longer a large deletion.
    pub fn check_content_restored(&mut self, content: &str) {
        todo!()
    }
    /// `_contentLossOverride = value` (binds the approval to the current editor text).
    pub fn set_content_loss_override(&mut self, value: bool) {
        todo!()
    }
    /// `undoContentLoss()`: the app ran the editor's undo first and reports
    /// whether it changed anything (and mirrors the text). Without history the
    /// server baseline is reloaded ([`AutosaveEffect::LoadEditorText`]).
    pub fn undo_content_loss(&mut self, undid_editor_change: bool) {
        todo!()
    }
    /// `saveAnywayAfterWarning()`.
    pub fn save_anyway_after_warning(&mut self, now: Instant) {
        todo!()
    }

    // ----- queries -------------------------------------------------------

    /// Active note.
    pub fn current_file(&self) -> Option<&str> {
        self.current_file.as_deref()
    }
    /// Dirty.
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
    /// A save request is in flight.
    pub fn is_saving(&self) -> bool {
        self.in_flight.is_some()
    }
    /// Offline.
    pub fn is_offline(&self) -> bool {
        self.is_offline
    }
    /// Server text of the active note as last loaded/saved.
    pub fn last_saved_content(&self) -> Option<&str> {
        self.last_saved_content.as_deref()
    }
    /// Server revision of the active note.
    pub fn base_revision(&self) -> Option<&str> {
        self.base_revision.as_deref()
    }
    /// Draft revision of the active note's stored draft.
    pub fn draft_revision(&self) -> Option<&str> {
        self.draft_revision.as_deref()
    }
    /// Latest server-acknowledged revision for `path`.
    pub fn known_base_revision(&self, path: &str) -> Option<&str> {
        self.known_base_revisions.get(path).map(String::as_str)
    }
    /// Content-loss banner active for the current file+baseline.
    pub fn content_loss_warning_active(&self) -> bool {
        todo!()
    }
    /// "Save anyway" approval active for the current editor text.
    pub fn content_loss_override(&self) -> bool {
        todo!()
    }
    /// Debounced save armed (`saveTimeout`).
    pub fn save_timer_armed(&self) -> bool {
        self.save_deadline.is_some()
    }
    /// Max-interval save armed (`saveMaxIntervalTimeout`).
    pub fn max_interval_timer_armed(&self) -> bool {
        self.max_interval_deadline.is_some()
    }
    /// Number of pending debounced draft writes.
    pub fn pending_draft_writes(&self) -> usize {
        self.draft_write_deadlines.len()
    }
    /// Offline backup armed.
    pub fn offline_backup_armed(&self) -> bool {
        self.offline_backup_deadline.is_some()
    }
    /// Local draft writes blocked for `path` until its recovery is resolved.
    pub fn is_draft_blocked(&self, path: &str) -> bool {
        self.draft_blocked_paths.contains(path)
    }
    /// A recovery dialog is pending for `path`.
    pub fn has_pending_recovery(&self, path: &str) -> bool {
        todo!()
    }
    /// Source path whose preservation must be retried for destination `path`.
    pub fn pending_remap_recovery(&self, path: &str) -> Option<&str> {
        self.pending_remap_recoveries.get(path).map(String::as_str)
    }
    /// Unload warning armed.
    pub fn unload_guard_active(&self) -> bool {
        todo!()
    }
    /// Current status label.
    pub fn status(&self) -> SaveStatus {
        self.status
    }
}
