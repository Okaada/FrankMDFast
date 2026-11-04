//! Crash-safe drafts keyed by [`DocId`], stored under the per-vault state dir
//! (never in the vault), carried through rename/move/delete (HP-007).
//!
//! **Reconciliation with the compat model.** The behaviour contract (golden
//! `js_draft_storage.json`, `draft_storage.test.js`, autosave tests) is the
//! path-keyed [`crate::draft_storage::DraftStorage`]: `frankmd:draft:<path>`
//! records with a server `baseRevision`, conflict copies on remap collisions,
//! legacy `frankmd:backup:<path>` records. The native app runs that exact
//! logic over a file-backed [`crate::draft_storage::DraftBackend`] living in
//! this directory (one key→value file store, atomic writes through
//! `frankmd-fs`). `DraftStore` is the DocId-indexed view used once the
//! DocId registry exists: with stable ids a rename needs no remap, but the
//! remap/conflict path stays for drafts imported from FrankMD and for
//! external renames the watcher could not attribute to a DocId.
//! OPEN QUESTION (flagged in docs/testing/port-editor.md): whether to keep the
//! path keys as the primary index (exact compat, importable from a browser
//! profile) or key by DocId and keep `path` only inside the record.

use frankmd_core::{DocId, Revision};

/// A persisted draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// Document.
    pub doc: DocId,
    /// Unsaved text.
    pub text: String,
    /// Disk revision the draft was based on (recovery dialog compares it).
    pub base_revision: Revision,
}

/// Draft store over `$XDG_STATE_HOME/frankmd/vaults/<hash>/drafts/`.
#[derive(Debug)]
pub struct DraftStore {
    dir: std::path::PathBuf,
}

impl DraftStore {
    /// Open the draft directory.
    pub fn open(dir: std::path::PathBuf) -> Self {
        Self { dir }
    }
    /// Persist (atomic write through frankmd-fs).
    pub fn put(&self, draft: &Draft) -> Result<(), frankmd_fs::FsError> {
        todo!()
    }
    /// Load the draft for `doc`.
    pub fn get(&self, doc: DocId) -> Option<Draft> {
        todo!()
    }
    /// Drop after a successful save or explicit discard.
    pub fn discard(&self, doc: DocId) -> Result<(), frankmd_fs::FsError> {
        todo!()
    }
}
