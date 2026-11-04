//! # frankmd-watch
//!
//! Thin layer over `notify` + `notify-debouncer-full` (added when implemented)
//! that turns raw events into domain [`WatchEvent`]s on a channel. GUI-free;
//! the workspace store consumes the events.
//!
//! - Vault: recursive, debounced ~200 ms, dotfiles ignored except `.fed`.
//!   Our own writes are suppressed via [`SelfWrites`] tokens (path + revision)
//!   so we never "reload" what we just saved.
//! - Omarchy: watch `~/.local/state/omarchy/current/` (the `theme/` dir is
//!   *replaced* on switch), debounce 80–150 ms, re-arm, fall back to the
//!   nearest existing ancestor (OM-003).
//! - `~/.config/omarchy/language`, `~/.config/fontconfig/` similarly.
#![allow(unused_variables, dead_code)] // skeleton

use frankmd_core::{RelPath, Revision};
use std::path::PathBuf;

/// Domain-level change notifications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// A note or folder changed outside FrankMD (create/modify/remove/rename).
    VaultChanged {
        /// Affected paths (deduplicated, debounced batch).
        paths: Vec<RelPath>,
    },
    /// `.fed` changed outside FrankMD.
    ConfigChanged,
    /// Omarchy theme switched / refreshed.
    OmarchyThemeChanged,
    /// Omarchy language file changed.
    SystemLanguageChanged,
    /// fontconfig changed (re-resolve `monospace`).
    FontsChanged,
    /// The watcher itself failed; UI falls back to manual refresh.
    Error(String),
}

/// Record of writes we performed, used to drop our own echo events.
#[derive(Debug, Default)]
pub struct SelfWrites {
    pending: Vec<(PathBuf, Revision)>,
}

impl SelfWrites {
    /// Register a write we are about to perform.
    pub fn expect(&mut self, path: PathBuf, rev: Revision) {
        todo!()
    }
    /// `true` if an observed change is our own write (consumes the token).
    pub fn is_own(&mut self, path: &std::path::Path, current: &Revision) -> bool {
        todo!()
    }
}

/// Handle that keeps watchers alive; dropping it stops them.
#[derive(Debug)]
pub struct Watchers {
    _private: (),
}

/// Sink the watcher thread pushes events into (an `async-channel` sender in the app).
pub trait EventSink: Send + 'static {
    /// Deliver one event; return `false` to stop watching.
    fn send(&self, ev: WatchEvent) -> bool;
}

/// Start all watchers for `vault_root`.
pub fn start(vault_root: PathBuf, sink: impl EventSink) -> Result<Watchers, String> {
    todo!()
}
