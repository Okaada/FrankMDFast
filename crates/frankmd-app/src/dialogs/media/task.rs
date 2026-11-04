//! Cancellable network work for the dialogs.
//!
//! [`spawn_net`] runs a future on the `frankmd-net` tokio runtime (the
//! window's [`NetRuntimeGlobal`](crate::bridge::tokio::NetRuntimeGlobal), or a
//! process-wide fallback so headless tests work) and returns a GPUI task plus
//! an [`Abort`] guard: dropping the guard (closing the dialog, Esc, a newer
//! request) aborts the tokio task, so nothing keeps talking to the network
//! after the user moved on.

use std::future::Future;
use std::sync::{Arc, OnceLock};

use gpui_kit::{App, Task};

/// Aborts the tokio task when dropped.
pub struct Abort(Option<Box<dyn FnOnce() + Send>>);

impl std::fmt::Debug for Abort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Abort")
    }
}

impl Drop for Abort {
    fn drop(&mut self) {
        if let Some(abort) = self.0.take() {
            abort();
        }
    }
}

fn runtime(cx: &App) -> Option<Arc<frankmd_net::NetRuntime>> {
    if let Some(global) = cx.try_global::<crate::bridge::tokio::NetRuntimeGlobal>() {
        return Some(Arc::clone(&global.0));
    }
    static FALLBACK: OnceLock<Option<Arc<frankmd_net::NetRuntime>>> = OnceLock::new();
    FALLBACK
        .get_or_init(|| match frankmd_net::NetRuntime::new() {
            Ok(rt) => Some(Arc::new(rt)),
            Err(err) => {
                tracing::error!(%err, "dialogs: no network runtime");
                None
            }
        })
        .clone()
}

/// Spawn `fut` on tokio. `None` (with an error result) when no runtime exists.
pub fn spawn_net<F, R>(cx: &App, fut: F) -> (Option<Abort>, Task<Option<R>>)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let Some(rt) = runtime(cx) else {
        return (None, Task::ready(None));
    };
    let handle = rt.spawn(fut);
    let abort_handle = handle.abort_handle();
    let abort = Abort(Some(Box::new(move || abort_handle.abort())));
    let task = cx.background_executor().spawn(async move { handle.await.ok() });
    (Some(abort), task)
}
