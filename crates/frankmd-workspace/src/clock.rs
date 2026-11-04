//! Injectable time source: re-exports the canonical clock of `frankmd-core`.
//!
//! Timers (autosave debounce, draft writes, TTLs) use [`Clock::instant`], the
//! monotonic time; [`Clock::now`] is wall-clock time for user-visible dates.

pub use frankmd_core::clock::{Clock, FakeClock, SystemClock};
