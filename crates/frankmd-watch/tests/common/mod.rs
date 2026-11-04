//! Shared helpers: a channel-backed sink and bounded waits.
#![allow(dead_code)]

use frankmd_watch::{EventSink, WatchEvent};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// Upper bound for any wait; tests normally finish far sooner.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(5);

/// Forwards events into an mpsc channel.
pub(crate) struct ChanSink(pub(crate) Sender<WatchEvent>);

impl EventSink for ChanSink {
    fn send(&self, ev: WatchEvent) -> bool {
        self.0.send(ev).is_ok()
    }
}

/// A sink plus the receiving end.
pub(crate) fn channel() -> (ChanSink, Receiver<WatchEvent>) {
    let (tx, rx) = mpsc::channel();
    (ChanSink(tx), rx)
}

/// Wait (bounded) for an event matching `pred`; returns it and everything
/// received before it.
pub(crate) fn wait_for(
    rx: &Receiver<WatchEvent>,
    pred: impl Fn(&WatchEvent) -> bool,
) -> (Option<WatchEvent>, Vec<WatchEvent>) {
    let deadline = Instant::now() + TIMEOUT;
    let mut seen = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(ev) if pred(&ev) => return (Some(ev), seen),
            Ok(ev) => seen.push(ev),
            Err(_) => return (None, seen),
        }
    }
}

/// `true` if `ev` is a `VaultChanged` naming `path` in `paths`.
pub(crate) fn changed(ev: &WatchEvent, path: &str) -> bool {
    matches!(ev, WatchEvent::VaultChanged { paths, .. } if paths.iter().any(|p| p.as_str() == path))
}

/// `true` if `ev` is a `VaultChanged` naming `path` in `removed`.
pub(crate) fn removed(ev: &WatchEvent, path: &str) -> bool {
    matches!(ev, WatchEvent::VaultChanged { removed, .. } if removed.iter().any(|p| p.as_str() == path))
}

/// Drain until the sender side is gone (bounded); returns what was drained,
/// or `None` if the channel did not disconnect in time.
pub(crate) fn drain_until_disconnected(rx: &Receiver<WatchEvent>) -> Option<Vec<WatchEvent>> {
    let deadline = Instant::now() + TIMEOUT;
    let mut seen = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(ev) => seen.push(ev),
            Err(RecvTimeoutError::Disconnected) => return Some(seen),
            Err(RecvTimeoutError::Timeout) => return None,
        }
    }
}
