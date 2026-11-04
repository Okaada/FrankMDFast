//! Shared, stoppable delivery of events to the [`EventSink`].

use crate::{EventSink, WatchEvent};
use notify_debouncer_full::notify;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

/// The sink plus a stop flag, shared by every watcher thread. The sink is
/// dropped as soon as watching stops.
pub(crate) struct Emitter {
    sink: Mutex<Option<Box<dyn EventSink>>>,
    stopped: AtomicBool,
}

impl Emitter {
    pub(crate) fn new(sink: impl EventSink) -> Self {
        Self {
            sink: Mutex::new(Some(Box::new(sink))),
            stopped: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<Box<dyn EventSink>>> {
        self.sink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Deliver `ev` unless stopped; a sink returning `false` stops everything.
    pub(crate) fn emit(&self, ev: WatchEvent) -> bool {
        let mut guard = self.lock();
        let Some(sink) = guard.as_ref() else {
            return false;
        };
        let keep_going = sink.send(ev);
        if !keep_going {
            self.stopped.store(true, Ordering::SeqCst);
            *guard = None;
        }
        keep_going
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// Stop delivery and drop the sink. Taking the sink lock means any
    /// in-flight `send` has finished when this returns, so no event is
    /// delivered afterwards.
    pub(crate) fn stop(&self) {
        let mut guard = self.lock();
        self.stopped.store(true, Ordering::SeqCst);
        *guard = None;
    }
}

/// Short, path-free description of a watcher error for [`WatchEvent::Error`].
pub(crate) fn describe(watcher: &str, err: &notify::Error) -> String {
    use notify::ErrorKind;
    let detail = match &err.kind {
        ErrorKind::Generic(msg) => msg.clone(),
        ErrorKind::Io(io) => io.kind().to_string(),
        ErrorKind::PathNotFound => "path not found".to_owned(),
        ErrorKind::WatchNotFound => "watch not found".to_owned(),
        ErrorKind::InvalidConfig(_) => "invalid watcher configuration".to_owned(),
        ErrorKind::MaxFilesWatch => {
            "too many watched files (raise fs.inotify.max_user_watches)".to_owned()
        }
    };
    format!("{watcher} watcher: {detail}")
}
