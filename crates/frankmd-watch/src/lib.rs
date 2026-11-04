//! # frankmd-watch
//!
//! Thin layer over `notify` + `notify-debouncer-full` that turns raw events
//! into domain [`WatchEvent`]s pushed into an [`EventSink`]. GUI-free and
//! read-only (it never writes files); the workspace store consumes the events.
//!
//! - Vault: recursive, debounced ~200 ms, dotfiles ignored except the root
//!   `.fed` ([`WatchEvent::ConfigChanged`]). Our own writes are suppressed via
//!   [`SelfWrites`] tokens (path + revision) so we never "reload" what we
//!   just saved.
//! - Omarchy: watch `~/.local/state/omarchy/current/` (the `theme/` dir is
//!   *replaced* on switch), debounce ~100 ms, re-arm, fall back to the
//!   nearest existing ancestor (OM-003).
//! - `~/.config/omarchy/language`, `~/.config/fontconfig/` similarly.

mod classify;
mod config;
mod emit;
mod self_writes;
mod system;
mod vault;

pub use classify::{
    CONFIG_LOCK_FILE, FILESYSTEM_LOCK_FILE, PathClass, classify, is_omarchy_theme_entry,
};
pub use config::{SYSTEM_DEBOUNCE, VAULT_DEBOUNCE, WatchConfig};
pub use self_writes::{DEFAULT_TOKEN_TTL, MAX_PENDING, SETTLE_GRACE, SelfWrites, SharedSelfWrites};

use emit::Emitter;
use frankmd_core::RelPath;
use std::path::PathBuf;
use std::sync::Arc;

/// Domain-level change notifications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// Notes or folders changed outside FrankMD (create/modify/remove/rename;
    /// a rename reports both sides).
    VaultChanged {
        /// Every affected vault-relative path in the debounced batch, sorted
        /// and deduplicated; includes the entries of `removed`. A root path
        /// (`RelPath::root()`) means events were lost: rescan everything.
        paths: Vec<RelPath>,
        /// Subset of `paths` that no longer existed when the batch was
        /// processed (deleted, or the source side of a rename). Everything
        /// else in `paths` exists and should be re-read.
        removed: Vec<RelPath>,
    },
    /// The root `.fed` changed (or was removed) outside FrankMD.
    ConfigChanged,
    /// Omarchy theme switched / refreshed (re-read the palette and compare
    /// content; a read may race the swap).
    OmarchyThemeChanged,
    /// Omarchy language file changed.
    SystemLanguageChanged,
    /// fontconfig changed (re-resolve `monospace`).
    FontsChanged,
    /// A watcher failed; the UI falls back to manual refresh. The message is
    /// short and carries no paths.
    Error(String),
}

/// Sink the watcher threads push events into (an `async-channel` sender in
/// the app). Calls are serialized; they come from background threads.
pub trait EventSink: Send + 'static {
    /// Deliver one event; return `false` to stop watching.
    fn send(&self, ev: WatchEvent) -> bool;
}

/// Handle that keeps watchers alive; dropping it stops them.
///
/// Once `drop` returns no further event is delivered and the sink has been
/// dropped; the worker threads exit within a few tens of milliseconds. A sink
/// returning `false` has the same effect (the handle then does nothing).
pub struct Watchers {
    emitter: Arc<Emitter>,
    self_writes: SharedSelfWrites,
}

impl Watchers {
    /// Token store the vault watcher consults. Register
    /// `expect(PathBuf::from(rel.as_str()), Revision::of(&bytes))` (or
    /// `expect_removal`) *before* each write/delete/rename FrankMD performs.
    pub fn self_writes(&self) -> SharedSelfWrites {
        Arc::clone(&self.self_writes)
    }
}

impl std::fmt::Debug for Watchers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watchers")
            .field("stopped", &self.emitter.is_stopped())
            .finish()
    }
}

impl Drop for Watchers {
    fn drop(&mut self) {
        self.emitter.stop();
    }
}

/// Start all watchers for `vault_root`, with the system watchers at their
/// default locations ([`WatchConfig::with_system_defaults`]).
pub fn start(vault_root: PathBuf, sink: impl EventSink) -> Result<Watchers, String> {
    start_with(WatchConfig::with_system_defaults(vault_root), sink)
}

/// Start the watchers described by `config`.
///
/// Fails only if the vault watcher cannot be armed. Problems with the
/// optional system watchers (missing dirs are fine: they re-arm) are reported
/// as [`WatchEvent::Error`].
pub fn start_with(config: WatchConfig, sink: impl EventSink) -> Result<Watchers, String> {
    let emitter = Arc::new(Emitter::new(sink));
    let self_writes = config
        .self_writes
        .clone()
        .unwrap_or_else(SelfWrites::shared);
    let watchers = Watchers {
        emitter: Arc::clone(&emitter),
        self_writes: Arc::clone(&self_writes),
    };

    vault::spawn(
        config.vault_root.clone(),
        config.vault_debounce,
        self_writes,
        Arc::clone(&emitter),
    )?;

    let mut specs = Vec::new();
    if let Some(dir) = config.omarchy_current_dir {
        specs.push(system::Spec {
            name: "omarchy-theme",
            target: dir,
            recursive: true,
            relevant: Box::new(is_omarchy_theme_entry),
            event: WatchEvent::OmarchyThemeChanged,
            debounce: config.system_debounce,
        });
    }
    if let Some(file) = config.omarchy_language_file
        && let (Some(dir), Some(name)) = (file.parent(), file.file_name())
    {
        let name = PathBuf::from(name);
        specs.push(system::Spec {
            name: "omarchy-language",
            target: dir.to_path_buf(),
            recursive: false,
            relevant: Box::new(move |rel| rel == name),
            event: WatchEvent::SystemLanguageChanged,
            debounce: config.system_debounce,
        });
    }
    if let Some(dir) = config.fontconfig_dir {
        specs.push(system::Spec {
            name: "fontconfig",
            target: dir,
            recursive: true,
            relevant: Box::new(|_| true),
            event: WatchEvent::FontsChanged,
            debounce: config.system_debounce,
        });
    }
    for spec in specs {
        if let Err(message) = system::spawn(spec, Arc::clone(&emitter)) {
            emitter.emit(WatchEvent::Error(message));
        }
    }
    Ok(watchers)
}
