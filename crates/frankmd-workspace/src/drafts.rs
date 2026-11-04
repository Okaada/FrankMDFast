//! Crash-safe drafts keyed by [`DocId`] (HP-001/HP-007), stored under the
//! per-vault state dir (`$XDG_STATE_HOME/frankmd/vaults/<hash>/`), never in
//! the notes folder.
//!
//! **Decision: drafts are keyed by DocId, not by path.** The record engine is
//! the ported [`crate::draft_storage::DraftStorage`] (FrankMD's record format,
//! verify-before-delete, legacy backups), but the *subject* every record is
//! filed under is [`draft_subject`]`(doc)` instead of the note path. A rename
//! or move therefore never touches draft storage: the DocId registry changes
//! the path and the draft stays attached. That removes the whole class of
//! path-as-identity bugs the original fought (drafts stranded by a failed
//! remap, collisions with another note's draft at the destination, late save
//! responses for the old path). The engine's path remap stays available for
//! re-keying a draft from one DocId to another (with collision preservation).
//!
//! [`FileDraftBackend`] persists the engine's key→value store as one JSON
//! file written atomically (temp + fsync + rename) through `frankmd-fs`.

use std::path::{Path, PathBuf};

use frankmd_core::DocId;

use crate::draft_storage::{BackendError, DraftBackend, DraftStorage};

/// File name of the draft store inside the per-vault state dir.
pub const DRAFTS_FILE: &str = "drafts.json";

/// The draft-engine subject for `doc` (its hyphenated UUID).
pub fn draft_subject(doc: DocId) -> String {
    doc.0.hyphenated().to_string()
}

/// Ordered key→value store persisted as `[[key, value], …]` in one file.
///
/// Every mutation rewrites the whole file atomically with mode `0600`
/// (drafts are private text); the in-memory copy changes only after the
/// write succeeded, so a failed write never reports a draft as stored.
#[derive(Debug)]
pub struct FileDraftBackend {
    path: PathBuf,
    entries: Vec<(String, String)>,
}

impl FileDraftBackend {
    /// Open `dir/drafts.json`. A missing file is an empty store; an
    /// unreadable or corrupt one is an error (it is never overwritten, so its
    /// drafts stay recoverable by hand). `dir` must already exist.
    pub fn open(dir: &Path) -> Result<Self, BackendError> {
        let path = dir.join(DRAFTS_FILE);
        let entries = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Vec<(String, String)>>(&bytes)
                .map_err(|e| BackendError::new(format!("Corrupt draft store: {e}")))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => {
                return Err(BackendError::new(format!(
                    "Unable to read draft store: {e}"
                )));
            }
        };
        Ok(Self { path, entries })
    }

    /// Location of the store file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn persist(&mut self, next: Vec<(String, String)>) -> Result<(), BackendError> {
        let bytes = serde_json::to_vec(&next).map_err(|e| BackendError::new(e.to_string()))?;
        frankmd_fs::atomic::atomic_replace(
            &self.path,
            &bytes,
            frankmd_fs::atomic::ModePolicy::Fixed(0o600),
        )
        .map_err(|e| BackendError::new(format!("Unable to write draft store: {e}")))?;
        self.entries = next;
        Ok(())
    }
}

impl DraftBackend for FileDraftBackend {
    fn len(&self) -> Result<usize, BackendError> {
        Ok(self.entries.len())
    }

    fn key(&self, index: usize) -> Result<Option<String>, BackendError> {
        Ok(self.entries.get(index).map(|(k, _)| k.clone()))
    }

    fn get_item(&self, key: &str) -> Result<Option<String>, BackendError> {
        Ok(self
            .entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone()))
    }

    fn set_item(&mut self, key: &str, value: &str) -> Result<(), BackendError> {
        let mut next = self.entries.clone();
        match next.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => value.clone_into(&mut entry.1),
            None => next.push((key.to_owned(), value.to_owned())),
        }
        self.persist(next)
    }

    fn remove_item(&mut self, key: &str) -> Result<(), BackendError> {
        if !self.entries.iter().any(|(k, _)| k == key) {
            return Ok(());
        }
        let next = self
            .entries
            .iter()
            .filter(|(k, _)| k != key)
            .cloned()
            .collect();
        self.persist(next)
    }
}

/// Open the on-disk draft store for one vault (`vault_state_dir` is
/// `AppDirs::vault_state(realpath)` from `frankmd-config`).
pub fn open_draft_storage(
    vault_state_dir: &Path,
) -> Result<DraftStorage<FileDraftBackend>, BackendError> {
    FileDraftBackend::open(vault_state_dir).map(DraftStorage::new)
}
