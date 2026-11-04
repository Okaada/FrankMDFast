//! GPUI ⇄ tokio bridge.
//!
//! GPUI's executors are not tokio; reqwest/aws-sdk/genai futures need a tokio
//! reactor. `frankmd_net::NetRuntime` owns that runtime (a GPUI `Global`
//! here); UI code calls [`spawn_net`], which spawns on tokio and awaits the
//! `JoinHandle` from a GPUI background task, then hops back to the
//! foreground with `cx.spawn` to update entities. This is the same pattern
//! as Zed's `gpui_tokio` crate.

use gpui_kit::{App, Global, Task};
use std::future::Future;
use std::sync::Arc;

/// Global holder for the network runtime.
pub(crate) struct NetRuntimeGlobal(pub(crate) Arc<frankmd_net::NetRuntime>);
impl Global for NetRuntimeGlobal {}

/// Spawn `fut` on tokio; the returned GPUI task resolves with its output.
pub(crate) fn spawn_net<F, R>(cx: &App, fut: F) -> Task<anyhow::Result<R>>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let rt = cx.global::<NetRuntimeGlobal>().0.clone();
    let handle = rt.spawn(fut);
    cx.background_executor()
        .spawn(async move { Ok(handle.await?) })
}
