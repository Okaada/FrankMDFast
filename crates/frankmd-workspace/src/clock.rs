//! Injectable time source.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Monotonic clock abstraction.
pub trait Clock: Send + Sync {
    /// Now.
    fn now(&self) -> Instant;
}

/// Real clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Manually advanced clock for deterministic timer tests (the `vi.useFakeTimers()`
/// of the original suite). Clones share the same time.
#[derive(Debug, Clone)]
pub struct FakeClock {
    now: Arc<Mutex<Instant>>,
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeClock {
    /// Start at an arbitrary fixed origin.
    pub fn new() -> Self {
        Self { now: Arc::new(Mutex::new(Instant::now())) }
    }

    /// Move time forward.
    pub fn advance(&self, by: Duration) {
        let mut now = self.now.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        *now += by;
    }

    /// Move time forward by `ms` milliseconds.
    pub fn advance_ms(&self, ms: u64) {
        self.advance(Duration::from_millis(ms));
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        *self.now.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
