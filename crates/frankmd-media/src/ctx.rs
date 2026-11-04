//! Injectable time and randomness (golden fixtures pin both).
//!
//! The clock is the workspace-wide [`frankmd_core::clock::Clock`]; it is
//! re-exported here so media callers keep one import path.

use std::sync::Arc;

pub use frankmd_core::clock::{Clock, FakeClock, SystemClock};

/// `SecureRandom.hex(n)` equivalent.
pub trait RandomHex: Send + Sync + std::fmt::Debug {
    /// `2 * n_bytes` lowercase hex chars.
    fn hex(&self, n_bytes: usize) -> String;
}

/// OS randomness (`getrandom(2)` on Unix).
#[derive(Debug, Clone, Copy, Default)]
pub struct OsRandom;

impl RandomHex for OsRandom {
    fn hex(&self, n_bytes: usize) -> String {
        frankmd_fs::atomic::random_hex(n_bytes)
    }
}

/// The golden fixtures' pinned `SecureRandom.hex`: `hex(n)` returns the first
/// `2n` chars of `"0123456789abcdef"` repeated (`hex(4)="01234567"`).
#[derive(Debug, Clone, Copy, Default)]
pub struct PinnedHex;

impl RandomHex for PinnedHex {
    fn hex(&self, n_bytes: usize) -> String {
        "0123456789abcdef"
            .chars()
            .cycle()
            .take(n_bytes * 2)
            .collect()
    }
}

/// Clock + randomness handed to every naming/upload function.
#[derive(Debug, Clone)]
pub struct MediaCtx {
    /// Clock.
    pub clock: Arc<dyn Clock>,
    /// Randomness.
    pub random: Arc<dyn RandomHex>,
}

impl MediaCtx {
    /// Real clock and OS randomness.
    pub fn system() -> Self {
        Self {
            clock: Arc::new(SystemClock),
            random: Arc::new(OsRandom),
        }
    }

    /// Golden pinning: fixed time (e.g. `2026-01-01T12:00:00Z` in UTC) + [`PinnedHex`].
    pub fn pinned(now: jiff::Zoned) -> Self {
        Self {
            clock: Arc::new(FakeClock::new(now)),
            random: Arc::new(PinnedHex),
        }
    }
}
