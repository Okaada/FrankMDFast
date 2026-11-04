//! Status-line view model (ui-spec §3.6): the save-state slot and the
//! transient message slot (2 s info / 5 s error, single instance) on the
//! injected clock.

use std::time::{Duration, Instant};

use crate::autosave::SaveStatus;

/// How long an info message stays.
pub const INFO_TTL: Duration = Duration::from_secs(2);
/// How long an error message stays.
pub const ERROR_TTL: Duration = Duration::from_secs(5);

/// Save-state slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveState {
    /// Nothing to report (clean; "Saved" already faded).
    Clean,
    /// ● Unsaved.
    Unsaved,
    /// ↻ Saving.
    Saving,
    /// ✓ Saved.
    Saved,
    /// ✕ Save failed.
    Failed,
    /// ⚠ Draft not stored.
    DraftNotStored,
}

impl SaveState {
    /// From the autosave session's label and whether a save is in flight.
    /// Disconnected maps to Unsaved: edits are kept as drafts and the
    /// vault-unreachable banner carries the connection state.
    pub fn from_autosave(status: SaveStatus, saving: bool, dirty: bool) -> Self {
        if saving {
            return Self::Saving;
        }
        match status {
            SaveStatus::Saved => Self::Saved,
            SaveStatus::ErrorSaving => Self::Failed,
            SaveStatus::DraftStorageError => Self::DraftNotStored,
            SaveStatus::Unsaved => Self::Unsaved,
            SaveStatus::Disconnected | SaveStatus::Hidden if dirty => Self::Unsaved,
            SaveStatus::Disconnected | SaveStatus::Hidden => Self::Clean,
        }
    }

    /// i18n key of the label (`None` when the slot shows nothing).
    pub fn label_key(self) -> Option<&'static str> {
        match self {
            Self::Clean => None,
            Self::Unsaved => Some("status.unsaved"),
            Self::Saving => Some("status.saving"),
            Self::Saved => Some("status.saved"),
            Self::Failed => Some("status.error_saving"),
            Self::DraftNotStored => Some("status.draft_storage_error"),
        }
    }

    /// Rendered with the danger colour.
    pub fn is_error(self) -> bool {
        matches!(self, Self::Failed | Self::DraftNotStored)
    }
}

/// One message in the slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusMessage {
    /// Text (already localized by the caller, or an i18n key it resolves).
    pub text: String,
    /// Error style.
    pub error: bool,
    /// When it disappears.
    pub expires: Instant,
}

/// Status-line message slot with TTL; a new message replaces the old one.
#[derive(Debug, Default)]
pub struct MessageSlot {
    current: Option<StatusMessage>,
}

impl MessageSlot {
    /// Show a message now (replaces the previous one).
    pub fn show(&mut self, text: &str, error: bool, now: Instant) {
        let ttl = if error { ERROR_TTL } else { INFO_TTL };
        self.current = Some(StatusMessage { text: text.to_owned(), error, expires: now + ttl });
    }

    /// Current message at `now`, if not expired.
    pub fn current(&self, now: Instant) -> Option<&StatusMessage> {
        self.current.as_ref().filter(|m| now < m.expires)
    }

    /// When the slot next changes (the app arms one timer for it).
    pub fn next_deadline(&self) -> Option<Instant> {
        self.current.as_ref().map(|m| m.expires)
    }

    /// Drop the message.
    pub fn clear(&mut self) {
        self.current = None;
    }
}
