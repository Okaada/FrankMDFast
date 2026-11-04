//! Test-only fault injection for the failure-path tests the Rails suite did
//! with mocha stubs (ENOSPC on the atomic rename, `link(2)` unsupported,
//! EXDEV on rename, ENOENT while `lstat`ing during a tree walk).
//!
//! Faults are thread-local and armed by a guard; compiled only with the
//! `fault-injection` feature (enabled for this crate's own tests through a
//! self dev-dependency).

use std::io;

/// Where to fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultPoint {
    /// The final `rename(2)` of an atomic replace.
    AtomicRename,
    /// The `link(2)` of a no-clobber create (Rails `File.link`).
    HardLink,
    /// The `rename(2)` of a rename/move (Rails `FileUtils.mv`), e.g. EXDEV.
    Rename,
    /// `lstat` of the entry with this basename during a tree walk.
    Lstat(String),
    /// `stat` of the entry with this vault-relative path (recents `present?`).
    Stat(String),
    /// Opening the shared lock file in the notes root (e.g. EROFS).
    LockOpen,
}

/// RAII guard: the fault stays armed until dropped.
#[derive(Debug)]
pub struct FaultGuard {
    point: FaultPoint,
}

impl Drop for FaultGuard {
    fn drop(&mut self) {}
}

/// Arm `point` to fail with `error` on the current thread until the guard drops.
/// `once` disarms it after the first hit.
pub fn inject(point: FaultPoint, error: io::ErrorKind, raw_os_error: Option<i32>, once: bool) -> FaultGuard {
    todo!()
}

/// Number of times `point` was reached (hit or not) since it was armed.
pub fn hits(point: &FaultPoint) -> usize {
    todo!()
}
