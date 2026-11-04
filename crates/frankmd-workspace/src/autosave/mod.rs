//! Autosave policy and the per-editor autosave/draft state machine
//! (**[COMPAT]** port of `autosave_controller.js`, frontend.md B.3/B.5).
//!
//! Pure over time: every timer-driven method takes `now: Instant` (from the
//! injected [`crate::clock::Clock::instant`]); [`AutosaveSession::tick`] fires
//! whatever is due. Side effects the app must perform (the save itself, status
//! label, dialogs, editor text changes) are queued as [`AutosaveEffect`]s and
//! drained with [`AutosaveSession::take_effects`]. The editor's text is
//! mirrored through [`AutosaveSession::set_editor_text`] (the JS reads
//! `codemirror.getValue()`).
//!
//! **Identity is the [`DocId`](frankmd_core::DocId)** (HP-001): drafts, timers
//! and acknowledged revisions are keyed by it, and a rename only updates the
//! path ([`AutosaveSession::paths_changed`]). See [`crate::drafts`].

mod session;

pub use session::AutosaveSession;

use frankmd_core::DocId;
use std::time::{Duration, Instant};

use crate::draft_storage::DraftError;

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
        Self {
            debounce: SAVE_DEBOUNCE,
            max_wait: SAVE_MAX_INTERVAL,
        }
    }
}

impl AutosaveTimer {
    /// Record an edit at `now`.
    pub fn edited(&mut self, now: Instant) {
        self.last_edit = Some(now);
        self.first_unsaved_edit.get_or_insert(now);
    }
    /// Should we save at `now`? True once `debounce` passed since the last
    /// edit, or `max_wait` passed since the first unsaved edit.
    pub fn due(&self, now: Instant, policy: &AutosavePolicy) -> bool {
        self.next_deadline(policy)
            .is_some_and(|deadline| now >= deadline)
    }
    /// Earliest instant at which [`Self::due`] becomes true (for scheduling the tick).
    pub fn next_deadline(&self, policy: &AutosavePolicy) -> Option<Instant> {
        let debounce = self.last_edit? + policy.debounce;
        let max_wait = self.first_unsaved_edit.map(|first| first + policy.max_wait);
        Some(max_wait.map_or(debounce, |m| m.min(debounce)))
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
        matches!(
            self,
            Self::ErrorSaving | Self::DraftStorageError | Self::Disconnected
        )
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
        Self {
            source: RecoverySource::Server,
            path: Some(req.path.clone()),
            content: None,
            draft_revision: req.draft_revision.clone(),
            backup_content: Some(req.backup_content.clone()),
            backup_timestamp: Some(req.backup_timestamp),
            conflict_id: req.conflict_id.clone(),
            server_content: req.conflict_id.as_ref().map(|_| req.server_content.clone()),
        }
    }
    /// `acceptBackup()`: `{source: req.source, path, content: backupContent,
    /// backupContent, draftRevision, backupTimestamp}` plus `conflictId` if any.
    pub fn accept_local(req: &RecoveryRequest) -> Self {
        Self {
            source: req.source,
            path: Some(req.path.clone()),
            content: Some(req.backup_content.clone()),
            draft_revision: req.draft_revision.clone(),
            backup_content: Some(req.backup_content.clone()),
            backup_timestamp: Some(req.backup_timestamp),
            conflict_id: req.conflict_id.clone(),
            server_content: None,
        }
    }
}

/// Identifies one in-flight save request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SaveTicket(pub u64);

/// A save the app must perform (the original's `PATCH /notes/<path>`).
///
/// The app writes to the document's path *at execution time* (resolved from
/// `doc` through the registry), so a rename between request and execution
/// can never recreate the old path (HP-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRequest {
    /// Ticket to report back with.
    pub ticket: SaveTicket,
    /// Document to save.
    pub doc: DocId,
    /// Note path when the request was issued (informational; `.fed` check).
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
    /// The active note changed meanwhile, or it is not the active note
    /// (`{ok:false, stale:true}`).
    Stale,
    /// Draft persistence failed or no known revision.
    Failed(DraftError),
}
