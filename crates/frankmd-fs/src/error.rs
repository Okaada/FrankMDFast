//! Typed filesystem errors (mapped to localized UI messages by the app).
//!
//! Each variant carries the **[COMPAT]** English message of the Rails
//! `NotesService` exception it replaces (`Display` is that message verbatim,
//! e.g. `"Note not found: a.md"`), and [`FsError::compat_class`] names the
//! Rails class so the golden fixtures' `{error, message}` can be compared.

use std::io;

/// Errors from vault operations.
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    /// Destination already exists (create / rename target):
    /// `"Destination already exists: <path>"`. A subclass of InvalidPath in Rails.
    #[error("{0}")]
    AlreadyExists(String),
    /// Source does not exist (or vanished mid-operation):
    /// `"Note not found: <path>"`, `"Folder not found: <path>"`, `"Path not found: <path>"`.
    #[error("{0}")]
    NotFound(String),
    /// The file changed since it was loaded (revision mismatch on delete):
    /// `"Note has changed: <path>"`.
    #[error("{0}")]
    RevisionConflict(String),
    /// Folder delete refused because it is not empty (hidden files count):
    /// `"Folder not empty: <path>"` (an InvalidPathError in Rails).
    #[error("{0}")]
    NotEmpty(String),
    /// Path escapes the vault, traverses a symlink outside it, or is reserved:
    /// `"Invalid path: <path>"`, `"Reserved application path"`, `"Invalid path"`
    /// (backlinks' `ArgumentError`).
    #[error("{0}")]
    InvalidPath(String),
    /// EACCES / EPERM / EROFS.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    /// Any other I/O error.
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl FsError {
    /// The Rails exception class this error replaces
    /// (`"NotesService::NotFoundError"`, `"NotesService::AlreadyExistsError"`,
    /// `"NotesService::InvalidPathError"`, `"NotesService::RevisionConflictError"`,
    /// `"Errno::EACCES"`, …). Backlinks' invalid path is `"ArgumentError"` and is
    /// produced as [`FsError::InvalidPath`] with message `"Invalid path"`.
    pub fn compat_class(&self) -> &'static str {
        todo!()
    }

    /// `true` for AlreadyExists, NotEmpty and InvalidPath (Rails
    /// `rescue NotesService::InvalidPathError` catches all three → 422).
    pub fn is_invalid_path(&self) -> bool {
        matches!(self, Self::AlreadyExists(_) | Self::NotEmpty(_) | Self::InvalidPath(_))
    }
}
