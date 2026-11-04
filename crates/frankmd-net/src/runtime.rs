//! The tokio runtime owner.

use std::future::Future;

/// Owns the tokio runtime used for all network / cloud / AI work.
#[derive(Debug)]
pub struct NetRuntime {
    rt: tokio::runtime::Runtime,
}

impl NetRuntime {
    /// Build a small multi-thread runtime (2 workers; network-bound work only).
    pub fn new() -> std::io::Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("frankmd-net")
            .enable_all()
            .build()?;
        Ok(Self { rt })
    }

    /// Spawn a tokio future; await the handle from any executor (GPUI's
    /// background executor included — `JoinHandle` is runtime-agnostic to poll).
    pub fn spawn<F>(&self, fut: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.rt.spawn(fut)
    }

    /// Handle for crates that need to enter the runtime (aws-sdk builders).
    pub fn handle(&self) -> &tokio::runtime::Handle {
        self.rt.handle()
    }
}
