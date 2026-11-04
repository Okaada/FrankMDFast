//! The notes folder ("vault") and the note operations on it.

use crate::FsError;
use frankmd_core::tree::TreeNode;
use frankmd_core::{RelPath, Revision};
use std::path::{Path, PathBuf};

/// An opened notes root. Cheap to clone; holds no file handles.
#[derive(Debug, Clone)]
pub struct Vault {
    root: PathBuf,
    real_root: PathBuf,
    state_dir: PathBuf,
}

/// A path proven to be inside the vault (component-wise, symlink-aware).
/// Only obtainable from [`Vault::contain`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainedPath {
    rel: RelPath,
    abs: PathBuf,
}

impl ContainedPath {
    /// Vault-relative form.
    pub fn rel(&self) -> &RelPath {
        &self.rel
    }
    /// Absolute (lexical, not canonical) form.
    pub fn abs(&self) -> &Path {
        &self.abs
    }
}

/// A note as read from disk.
#[derive(Debug, Clone)]
pub struct NoteFile {
    /// Exact bytes (no BOM stripping, CRLF preserved, **[COMPAT]**).
    pub bytes: Vec<u8>,
    /// SHA-256 revision of `bytes`.
    pub revision: Revision,
    /// mtime, whole seconds.
    pub mtime: i64,
    /// `false` if the bytes were not valid UTF-8 (UI shows a warning and
    /// opens read-only rather than failing, HP-028).
    pub valid_utf8: bool,
}

impl Vault {
    /// Open (and `mkdir -p`) a notes root. `state_dir` is the app-owned
    /// per-vault state directory outside the vault (HP-012).
    pub fn open(root: impl Into<PathBuf>, state_dir: impl Into<PathBuf>) -> Result<Self, FsError> {
        todo!()
    }

    /// Notes root as given.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Canonical (realpath) notes root; `ContainedPath::abs` is built on it.
    pub fn real_root(&self) -> &Path {
        &self.real_root
    }

    /// **[COMPAT]** `<realpath root>/.frankmd-filesystem.lock`: the one lock
    /// file every instance (Rails or Rust) on this folder shares.
    pub fn filesystem_lock_path(&self) -> PathBuf {
        todo!("backend.md §2.7")
    }

    /// Prove `rel` is inside the vault (backend.md §2.4 semantics, plus
    /// per-component `lstat` for symlinks). Rejects reserved app files.
    pub fn contain(&self, rel: &RelPath) -> Result<ContainedPath, FsError> {
        todo!()
    }

    /// **[COMPAT]** `NotesService#safe_path(path, must_exist:)` on a raw path:
    /// `.frankmd-config.lock` → `InvalidPath("Reserved application path")`;
    /// rejected by `PathSafety.contain` → `InvalidPath("Invalid path: <path>")`;
    /// `must_exist` and missing (symlinks followed) →
    /// `NotFound("Path not found: <path>")`.
    pub fn safe_path(&self, raw: &str, must_exist: bool) -> Result<ContainedPath, FsError> {
        todo!("backend.md §2.4")
    }

    /// Build the Explorer tree under a shared filesystem lock.
    pub fn list_tree(&self) -> Result<Vec<TreeNode>, FsError> {
        todo!()
    }

    /// The tree below one folder (`Folder#children`; `[]` when missing).
    pub fn folder_children(&self, path: &ContainedPath) -> Result<Vec<TreeNode>, FsError> {
        todo!()
    }

    /// Read a note. Tolerates non-UTF-8.
    pub fn read_note(&self, path: &ContainedPath) -> Result<NoteFile, FsError> {
        todo!()
    }

    /// Create a new note; `AlreadyExists` if anything is there. Creates parents.
    pub fn create_note(&self, path: &ContainedPath, bytes: &[u8]) -> Result<Revision, FsError> {
        todo!()
    }

    /// Upsert (`NotesService#write`): `mkdir -p` the parent, atomic replace
    /// keeping the existing mode (new files get `0666 & ~umask`).
    pub fn write_note(&self, path: &ContainedPath, bytes: &[u8]) -> Result<Revision, FsError> {
        todo!()
    }

    /// Save over an **existing** note; `NotFound` if it is gone (never
    /// resurrects a renamed/deleted note, HP-006).
    pub fn update_existing(&self, path: &ContainedPath, bytes: &[u8]) -> Result<Revision, FsError> {
        todo!()
    }

    /// Rename/move a file or folder without clobbering.
    pub fn rename(&self, from: &ContainedPath, to: &ContainedPath) -> Result<(), FsError> {
        todo!()
    }

    /// Delete a note, optionally only if its revision still matches.
    pub fn delete_note(&self, path: &ContainedPath, expected: Option<&Revision>) -> Result<(), FsError> {
        todo!()
    }

    /// Create a folder (`AlreadyExists` if present).
    pub fn create_folder(&self, path: &ContainedPath) -> Result<(), FsError> {
        todo!()
    }

    /// Delete an empty folder (hidden files count, **[COMPAT]**).
    pub fn delete_empty_folder(&self, path: &ContainedPath) -> Result<(), FsError> {
        todo!()
    }
}
