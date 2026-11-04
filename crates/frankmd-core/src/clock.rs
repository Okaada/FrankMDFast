//! Injectable time source shared by every crate (canonical home; the
//! workspace's autosave timers and media's dated S3 keys both use it).
//!
//! A clock answers two different questions:
//! - [`Clock::now`]: wall-clock time in the zone dates are rendered in
//!   (Hugo dates, S3 prefixes, recent-files timestamps);
//! - [`Clock::instant`]: a monotonic instant for debounce/autosave timers.
//!
//! [`FakeClock`] advances both together so timer tests and dated output stay
//! consistent.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Injectable time source.
pub trait Clock: Send + Sync + std::fmt::Debug {
    /// Current wall-clock time (with its zone).
    fn now(&self) -> jiff::Zoned;

    /// Current monotonic instant (timers, debouncing).
    fn instant(&self) -> Instant;

    /// Current wall-clock instant as a UTC timestamp.
    fn timestamp(&self) -> jiff::Timestamp {
        self.now().timestamp()
    }
}

/// The real clock: system time in the process' local zone.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> jiff::Zoned {
        jiff::Zoned::now()
    }

    fn instant(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Debug)]
struct FakeState {
    wall: jiff::Zoned,
    instant: Instant,
}

/// Manually advanced clock for deterministic tests (the `travel_to` /
/// `vi.useFakeTimers()` of the original suites). Clones share the same time.
#[derive(Debug, Clone)]
pub struct FakeClock {
    state: Arc<Mutex<FakeState>>,
}

impl FakeClock {
    /// A clock frozen at `wall` (monotonic origin: the moment of creation).
    pub fn new(wall: jiff::Zoned) -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState {
                wall,
                instant: Instant::now(),
            })),
        }
    }

    /// A clock frozen at the Unix epoch in UTC.
    pub fn epoch() -> Self {
        Self::new(jiff::Timestamp::UNIX_EPOCH.to_zoned(jiff::tz::TimeZone::UTC))
    }

    /// Move both the wall clock and the monotonic clock forward by `by`.
    pub fn advance(&self, by: Duration) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.instant += by;
        let span = jiff::SignedDuration::try_from(by).unwrap_or(jiff::SignedDuration::MAX);
        if let Ok(next) = state.wall.checked_add(span) {
            state.wall = next;
        }
    }

    /// Move time forward by `ms` milliseconds.
    pub fn advance_ms(&self, ms: u64) {
        self.advance(Duration::from_millis(ms));
    }

    /// Jump the wall clock to `wall` (the monotonic clock is unchanged).
    pub fn set(&self, wall: jiff::Zoned) {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .wall = wall;
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::epoch()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> jiff::Zoned {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .wall
            .clone()
    }

    fn instant(&self) -> Instant {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .instant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_clock_advances_wall_and_instant_together() {
        let clock = FakeClock::epoch();
        let (w0, i0) = (clock.now(), clock.instant());
        clock.advance_ms(1500);
        assert_eq!(clock.instant() - i0, Duration::from_millis(1500));
        assert_eq!(
            clock.timestamp().as_millisecond() - w0.timestamp().as_millisecond(),
            1500
        );
        let shared = clock.clone();
        shared.advance_ms(500);
        assert_eq!(clock.instant() - i0, Duration::from_millis(2000));
    }
}
