//! Test-only fault injection for the failure-path tests the Rails suite did
//! with mocha stubs (ENOSPC on the atomic rename, `link(2)` unsupported,
//! EXDEV on rename, ENOENT while `lstat`ing during a tree walk).
//!
//! Faults are thread-local and armed by a guard; compiled only with the
//! `fault-injection` feature (enabled for this crate's own tests through a
//! self dev-dependency).

use std::cell::RefCell;
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
    fn drop(&mut self) {
        ARMED.with(|armed| armed.borrow_mut().retain(|fault| fault.point != self.point));
    }
}

#[derive(Debug)]
struct Armed {
    point: FaultPoint,
    kind: io::ErrorKind,
    raw_os_error: Option<i32>,
    once: bool,
    spent: bool,
    hits: usize,
}

thread_local! {
    static ARMED: RefCell<Vec<Armed>> = const { RefCell::new(Vec::new()) };
}

/// Arm `point` to fail with `error` on the current thread until the guard drops.
/// `once` disarms it after the first hit.
pub fn inject(
    point: FaultPoint,
    error: io::ErrorKind,
    raw_os_error: Option<i32>,
    once: bool,
) -> FaultGuard {
    ARMED.with(|armed| {
        let mut armed = armed.borrow_mut();
        armed.retain(|fault| fault.point != point);
        armed.push(Armed {
            point: point.clone(),
            kind: error,
            raw_os_error,
            once,
            spent: false,
            hits: 0,
        });
    });
    FaultGuard { point }
}

/// Number of times `point` was reached (hit or not) since it was armed.
pub fn hits(point: &FaultPoint) -> usize {
    ARMED.with(|armed| {
        armed
            .borrow()
            .iter()
            .find(|fault| &fault.point == point)
            .map_or(0, |fault| fault.hits)
    })
}

/// Called by the crate at each fault point: counts the hit and returns the
/// armed error, if any.
pub(crate) fn check(point: &FaultPoint) -> io::Result<()> {
    ARMED.with(|armed| {
        let mut armed = armed.borrow_mut();
        let Some(fault) = armed.iter_mut().find(|fault| &fault.point == point) else {
            return Ok(());
        };
        fault.hits += 1;
        if fault.spent {
            return Ok(());
        }
        if fault.once {
            fault.spent = true;
        }
        Err(match fault.raw_os_error {
            Some(code) => crate::errno::error(code),
            None => io::Error::from(fault.kind),
        })
    })
}
