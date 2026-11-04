//! Advisory locks shared with the original Rails app (**[COMPAT]**).

use crate::FsError;
use std::path::{Path, PathBuf};

/// Lock file guarding `.fed` read-modify-write.
pub const CONFIG_LOCK: &str = ".frankmd-config.lock";
/// Lock file guarding note/folder mutations and tree reads.
pub const FILESYSTEM_LOCK: &str = ".frankmd-filesystem.lock";

/// Shared (tree reads) or exclusive (mutations) lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKind {
    /// `LOCK_SH`.
    Shared,
    /// `LOCK_EX`.
    Exclusive,
}

/// RAII guard; the flock is released on drop.
#[derive(Debug)]
pub struct LockGuard {
    path: PathBuf,
}

/// Acquire `name` under `notes_root` (opened `O_RDWR|O_CREAT|O_NOFOLLOW`, 0600).
/// On EACCES/EROFS falls back to a local lock under `fallback_dir`
/// (`$XDG_STATE_HOME/frankmd/locks/<sha256(realpath root)>/`).
pub fn acquire(
    notes_root: &Path,
    name: &str,
    kind: LockKind,
    fallback_dir: &Path,
) -> Result<LockGuard, FsError> {
    todo!("backend.md §1.8, §2.7")
}
