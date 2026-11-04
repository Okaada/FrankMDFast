//! Advisory locks shared with the original Rails app (**[COMPAT]**).
//!
//! Both apps `flock(2)` the same hidden file in the notes root, so a Rails
//! FrankMD and this app working on one folder serialize their mutations.
//! When the notes root cannot hold a lock file (read-only mount, symlinked
//! lock path, …) the lock degrades to a private file under the app's state
//! directory: the filesystem operation itself still reports its own error.

use crate::FsError;
use crate::errno::{EACCES, EROFS};
use std::io;
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

/// RAII guard; the lock is released on drop.
#[derive(Debug)]
pub struct LockGuard {
    path: PathBuf,
    handle: sys::Handle,
}

impl LockGuard {
    /// The lock file actually locked (the shared one or the fallback).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Closing the handle releases the lock too; unlocking first keeps
        // the release explicit (Rails `flock(LOCK_UN)` before `close`).
        sys::unlock(&self.handle);
    }
}

/// Acquire `name` under `notes_root` (opened `O_RDWR|O_CREAT|O_NOFOLLOW`, 0600).
/// On EACCES/EROFS falls back to a local lock under `fallback_dir`
/// (`$XDG_STATE_HOME/frankmd/locks/<sha256(realpath root)>/`).
///
/// **[COMPAT]** like Rails: a shared lock on a read-only root first retries
/// the file read-only; any failure to open the shared file degrades to the
/// fallback lock instead of failing the operation.
pub fn acquire(
    notes_root: &Path,
    name: &str,
    kind: LockKind,
    fallback_dir: &Path,
) -> Result<LockGuard, FsError> {
    let shared = notes_root.join(name);
    let (path, handle) = match open_shared(&shared, kind) {
        Some(handle) => (shared, handle),
        None => {
            let local = fallback_dir.join(name);
            (local.clone(), open_fallback(fallback_dir, &local)?)
        }
    };
    sys::lock(&handle, kind)?;
    Ok(LockGuard { path, handle })
}

/// Acquire `name` under `dir` only (app-owned state directory, created
/// `0700`); used for folders that must not receive app files.
pub fn acquire_private(name: &str, kind: LockKind, dir: &Path) -> Result<LockGuard, FsError> {
    let path = dir.join(name);
    let handle = open_fallback(dir, &path)?;
    sys::lock(&handle, kind)?;
    Ok(LockGuard { path, handle })
}

/// Open the lock file in the notes root, `None` when it cannot be opened.
fn open_shared(path: &Path, kind: LockKind) -> Option<sys::Handle> {
    match open_checked(path, true) {
        Ok(handle) => Some(handle),
        Err(e) if matches!(crate::errno::of(&e), Some(EACCES | EROFS)) && kind == LockKind::Shared => {
            open_checked(path, false).ok()
        }
        Err(_) => None,
    }
}

fn open_checked(path: &Path, writable: bool) -> io::Result<sys::Handle> {
    crate::fault_check!(crate::fault::FaultPoint::LockOpen)?;
    sys::open(path, writable)
}

/// Open (creating `0700` parents) the private fallback lock file.
fn open_fallback(dir: &Path, path: &Path) -> Result<sys::Handle, FsError> {
    sys::create_private_dir(dir)?;
    Ok(sys::open(path, true)?)
}

/// `flock(2)` through rustix, opened `O_NOFOLLOW | O_CLOEXEC`.
#[cfg(unix)]
mod sys {
    use super::LockKind;
    use rustix::fs::{FlockOperation, Mode, OFlags, flock};
    use std::io;
    use std::os::fd::OwnedFd;
    use std::path::Path;

    pub(super) type Handle = OwnedFd;

    pub(super) fn open(path: &Path, writable: bool) -> io::Result<Handle> {
        let (flags, mode) = if writable {
            (OFlags::RDWR | OFlags::CREATE, Mode::from_raw_mode(0o600))
        } else {
            (OFlags::RDONLY, Mode::empty())
        };
        Ok(rustix::fs::open(path, flags | OFlags::NOFOLLOW | OFlags::CLOEXEC, mode)?)
    }

    pub(super) fn create_private_dir(dir: &Path) -> io::Result<()> {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)
    }

    pub(super) fn lock(handle: &Handle, kind: LockKind) -> io::Result<()> {
        let op = match kind {
            LockKind::Shared => FlockOperation::LockShared,
            LockKind::Exclusive => FlockOperation::LockExclusive,
        };
        Ok(flock(handle, op)?)
    }

    pub(super) fn unlock(handle: &Handle) {
        let _ = flock(handle, FlockOperation::Unlock);
    }
}

/// `LockFileEx` through std's `File::lock`. A symlinked lock path is refused
/// (`ELOOP`), the effect `O_NOFOLLOW` has on Unix.
#[cfg(not(unix))]
mod sys {
    use super::LockKind;
    use crate::errno::{ELOOP, error};
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::path::Path;

    pub(super) type Handle = File;

    pub(super) fn open(path: &Path, writable: bool) -> io::Result<Handle> {
        if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(error(ELOOP));
        }
        OpenOptions::new()
            .read(true)
            .write(writable)
            .create(writable)
            .truncate(false)
            .open(path)
    }

    pub(super) fn create_private_dir(dir: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dir)
    }

    pub(super) fn lock(handle: &Handle, kind: LockKind) -> io::Result<()> {
        match kind {
            LockKind::Shared => handle.lock_shared(),
            LockKind::Exclusive => handle.lock(),
        }
    }

    pub(super) fn unlock(handle: &Handle) {
        let _ = handle.unlock();
    }
}
