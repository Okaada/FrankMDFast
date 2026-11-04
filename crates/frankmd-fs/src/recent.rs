//! `RecentFilesService` (`app/services/recent_files_service.rb`): the recents
//! list stored in `.fed` `recent_files`, validated against the vault.
//!
//! Persistence is abstracted by [`RecentStore`] because `.fed` belongs to
//! `frankmd-config` (which depends on this crate). `frankmd_config::ConfigStore`
//! is expected to implement it (`Config#get` / `Config#modify` under the
//! **[COMPAT]** `.frankmd-config.lock`).

use crate::{FsError, Vault};
use frankmd_core::recent::RecentFile;

/// Where `recent_files` lives (`.fed` in production, a fake in tests).
pub trait RecentStore {
    /// The raw text of the `recent_files = <json>` value in `.fed`, `None`
    /// when the key is absent. The service parses it (malformed JSON ≙ Rails
    /// `Config#get` → nil, so it reads as empty).
    fn load(&self) -> Option<String>;

    /// `Config#modify("recent_files")`: read-modify-write under the
    /// **[COMPAT]** config lock. `f` gets the current raw value and returns the
    /// new compact JSON text (or an error that aborts the write and is
    /// propagated). Returns the `Config#set` result (`false` when the write
    /// failed).
    fn modify(&self, f: &mut dyn FnMut(Option<String>) -> Result<String, FsError>) -> Result<bool, FsError>;

    /// Whether the backing `.fed` is a regular file (NotesService only
    /// remaps/removes recents when it is, backend.md §2.6).
    fn is_present(&self) -> bool;
}

/// The recents service for one vault.
pub struct RecentFilesService<'a> {
    vault: &'a Vault,
    store: &'a dyn RecentStore,
}

impl<'a> RecentFilesService<'a> {
    /// Bind to a vault and a store.
    pub fn new(vault: &'a Vault, store: &'a dyn RecentStore) -> Self {
        Self { vault, store }
    }

    /// Normalized entries (missing files filtered on read, **not** persisted).
    /// A stat permission error counts as present (**[COMPAT]**).
    pub fn entries(&self) -> Vec<RecentFile> {
        todo!("backend.md §6")
    }

    /// Record an open at `now`. `NotFound("Note not found")` for an invalid,
    /// escaping (symlink-aware), non-`.md` or missing path; nothing is
    /// written then. Returns the store's write result.
    pub fn record(&self, path: Option<&str>, now: jiff::Timestamp) -> Result<bool, FsError> {
        todo!("backend.md §6")
    }

    /// Empty the history.
    pub fn clear(&self) -> Result<bool, FsError> {
        todo!()
    }

    /// Drop `path` and everything under `path/`.
    pub fn remove(&self, path: &str) -> Result<bool, FsError> {
        todo!()
    }

    /// Re-root `old` (and `old/…`) onto `new`, then normalize.
    pub fn remap(&self, old: &str, new: &str) -> Result<bool, FsError> {
        todo!()
    }
}
