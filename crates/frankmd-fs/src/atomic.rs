//! Atomic replace and no-clobber create primitives.

use crate::FsError;
use std::path::Path;

/// What permission bits the replaced file gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModePolicy {
    /// Keep the existing file's `mode & 0o777` (notes, **[COMPAT]**).
    Preserve,
    /// Force a mode (`.fed` is always `0o600`, **[COMPAT]**).
    Fixed(u32),
}

/// Atomically replace `target` with `bytes` (temp in same dir + fsync + rename).
pub fn atomic_replace(target: &Path, bytes: &[u8], mode: ModePolicy) -> Result<(), FsError> {
    todo!()
}

/// Create `target` with `bytes`, failing with `AlreadyExists` if anything
/// (including a dangling symlink) is already there.
pub fn create_new(target: &Path, bytes: &[u8]) -> Result<(), FsError> {
    todo!()
}

/// Rename that never replaces an existing destination
/// (`renameat2(RENAME_NOREPLACE)`, EXDEV → copy + delete).
pub fn rename_noclobber(from: &Path, to: &Path) -> Result<(), FsError> {
    todo!()
}

/// **[COMPAT]** temp file name for `basename`: `.<basename>.<12 hex>.tmp`.
pub fn temp_name(basename: &str) -> String {
    todo!()
}
