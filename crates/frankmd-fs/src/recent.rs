//! `RecentFilesService` (`app/services/recent_files_service.rb`): the recents
//! list stored in `.fed` `recent_files`, validated against the vault.
//!
//! Persistence is abstracted by [`RecentStore`] because `.fed` belongs to
//! `frankmd-config` (which depends on this crate). `frankmd_config::ConfigStore`
//! is expected to implement it (`Config#get` / `Config#modify` under the
//! **[COMPAT]** `.frankmd-config.lock`).

use crate::errno::EACCES;
use crate::{FsError, Vault, path_safety};
use frankmd_core::recent::{self, RecentFile, RecentFiles};
use serde_json::Value;
use std::io;

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
    fn modify(
        &self,
        f: &mut dyn FnMut(Option<String>) -> Result<String, FsError>,
    ) -> Result<bool, FsError>;

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
        self.normalize(&parse(self.store.load())).0
    }

    /// Record an open at `now`. `NotFound("Note not found")` for an invalid,
    /// escaping (symlink-aware), non-`.md` or missing path; nothing is
    /// written then. Returns the store's write result.
    pub fn record(&self, path: Option<&str>, now: jiff::Timestamp) -> Result<bool, FsError> {
        self.change(true, |items| {
            let path = path.filter(|p| self.valid_path(p) && self.present(p));
            let Some(path) = path else {
                return Err(FsError::NotFound("Note not found".to_owned()));
            };
            let mut list = vec![RecentFile {
                path: path.to_owned(),
                opened_at: recent::format_opened_at(now),
            }];
            list.extend(
                from_value(items)
                    .into_iter()
                    .filter(|item| item.path != path),
            );
            list.truncate(recent::LIMIT);
            Ok(list)
        })
    }

    /// Empty the history.
    pub fn clear(&self) -> Result<bool, FsError> {
        self.change(true, |_| Ok(Vec::new()))
    }

    /// Drop `path` and everything under `path/`.
    pub fn remove(&self, path: &str) -> Result<bool, FsError> {
        self.change(true, |items| {
            Ok(from_value(items)
                .into_iter()
                .filter(|item| !scoped(&item.path, path))
                .collect())
        })
    }

    /// Re-root `old` (and `old/…`) onto `new`, then normalize.
    pub fn remap(&self, old: &str, new: &str) -> Result<bool, FsError> {
        // Remap before checking existence: the old path is already gone.
        self.change(false, |items| {
            Ok(RecentFiles::remap_raw(&items, old, new, |p| self.contained_and_present(p)).0)
        })
    }

    /// `change(normalize_first:)`: locked read-modify-write of the store.
    /// `f` receives the stored array (first [`recent::MAX_INPUT`] items,
    /// normalized when asked) and returns the new list.
    fn change(
        &self,
        normalize_first: bool,
        f: impl FnOnce(Value) -> Result<Vec<RecentFile>, FsError>,
    ) -> Result<bool, FsError> {
        let mut f = Some(f);
        self.store.modify(&mut |raw| {
            let items = match parse(raw) {
                Value::Array(items) => {
                    Value::Array(items.into_iter().take(recent::MAX_INPUT).collect())
                }
                _ => Value::Array(Vec::new()),
            };
            let items = if normalize_first {
                entries_value(&self.normalize(&items).0)
            } else {
                items
            };
            let f = f
                .take()
                .ok_or_else(|| FsError::Io(io::Error::other("recent files modified twice")))?;
            Ok(RecentFiles(f(items)?).to_json())
        })
    }

    /// `normalize` with the vault-aware half of `valid_path?` and `present?`.
    fn normalize(&self, value: &Value) -> RecentFiles {
        RecentFiles::normalize(value, |p| self.contained_and_present(p))
    }

    fn contained_and_present(&self, path: &str) -> bool {
        path_safety::contain(self.vault.real_root(), Some(path)).is_some() && self.present(path)
    }

    /// **[COMPAT]** `valid_path?`: the lexical rules plus symlink-aware containment.
    fn valid_path(&self, path: &str) -> bool {
        recent::is_valid_path(path)
            && path_safety::contain(self.vault.real_root(), Some(path)).is_some()
    }

    /// **[COMPAT]** `present?`: a regular file (symlinks followed); a
    /// permission error is not evidence of deletion, so it counts as present.
    fn present(&self, path: &str) -> bool {
        let stat = crate::fault_check!(crate::fault::FaultPoint::Stat(path.to_owned()))
            .and_then(|()| std::fs::metadata(self.vault.real_root().join(path)));
        match stat {
            Ok(meta) => meta.is_file(),
            Err(e) => crate::errno::of(&e) == Some(EACCES),
        }
    }
}

/// The stored JSON text as a value; absent or malformed reads as `null`
/// (Rails `Config#get` → nil).
fn parse(raw: Option<String>) -> Value {
    raw.and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

/// Entries already normalized by [`RecentFilesService::change`].
fn from_value(items: Value) -> Vec<RecentFile> {
    serde_json::from_value(items).unwrap_or_default()
}

fn entries_value(entries: &[RecentFile]) -> Value {
    serde_json::to_value(entries).unwrap_or(Value::Array(Vec::new()))
}

/// **[COMPAT]** `scoped?(candidate, path)`: equal, or below `path + "/"`.
fn scoped(candidate: &str, path: &str) -> bool {
    candidate == path
        || candidate
            .strip_prefix(path)
            .is_some_and(|rest| rest.starts_with('/'))
}
