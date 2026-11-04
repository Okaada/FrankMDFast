//! Advisory locks shared with the original Rails app (**[COMPAT]**).
//!
//! Both apps `flock(2)` the same hidden file in the notes root, so a Rails
//! FrankMD and this app working on one folder serialize their mutations.
//! When the notes root cannot hold a lock file (read-only mount, symlinked
//! lock path, …) the lock degrades to a private file under the app's state
//! directory: the filesystem operation itself still reports its own error.

use crate::FsError;
use crate::error::libc_errno::{EACCES, EROFS};
use rustix::fs::{FlockOperation, Mode, OFlags, flock, open};
use std::io;
use std::os::fd::OwnedFd;
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
    fd: OwnedFd,
}

impl LockGuard {
    /// The lock file actually locked (the shared one or the fallback).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Closing the descriptor releases the lock too; unlocking first keeps
        // the release explicit (Rails `flock(LOCK_UN)` before `close`).
        let _ = flock(&self.fd, FlockOperation::Unlock);
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
    let (path, fd) = match open_shared(&shared, kind) {
        Some(fd) => (shared, fd),
        None => {
            let local = fallback_dir.join(name);
            (local.clone(), open_fallback(fallback_dir, &local)?)
        }
    };
    let op = match kind {
        LockKind::Shared => FlockOperation::LockShared,
        LockKind::Exclusive => FlockOperation::LockExclusive,
    };
    flock(&fd, op).map_err(io::Error::from)?;
    Ok(LockGuard { path, fd })
}

/// Acquire `name` under `dir` only (app-owned state directory, created
/// `0700`); used for folders that must not receive app files.
pub fn acquire_private(name: &str, kind: LockKind, dir: &Path) -> Result<LockGuard, FsError> {
    let path = dir.join(name);
    let fd = open_fallback(dir, &path)?;
    let op = match kind {
        LockKind::Shared => FlockOperation::LockShared,
        LockKind::Exclusive => FlockOperation::LockExclusive,
    };
    flock(&fd, op).map_err(io::Error::from)?;
    Ok(LockGuard { path, fd })
}

/// Open the lock file in the notes root, `None` when it cannot be opened.
fn open_shared(path: &Path, kind: LockKind) -> Option<OwnedFd> {
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    match open_checked(path, flags, Mode::from_raw_mode(0o600)) {
        Ok(fd) => Some(fd),
        Err(e) if matches!(e.raw_os_error(), Some(EACCES | EROFS)) && kind == LockKind::Shared => {
            open_checked(
                path,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .ok()
        }
        Err(_) => None,
    }
}

fn open_checked(path: &Path, flags: OFlags, mode: Mode) -> io::Result<OwnedFd> {
    crate::fault_check!(crate::fault::FaultPoint::LockOpen)?;
    Ok(open(path, flags, mode)?)
}

/// Open (creating `0700` parents) the private fallback lock file.
fn open_fallback(dir: &Path, path: &Path) -> Result<OwnedFd, FsError> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    Ok(open(path, flags, Mode::from_raw_mode(0o600)).map_err(io::Error::from)?)
}
