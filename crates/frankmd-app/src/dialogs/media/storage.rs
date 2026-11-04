//! The window's "Storage for new media" model (FE-355…357): one entity shared
//! by the Insert image / Insert video footers and the Settings row.

use std::sync::Arc;

use frankmd_media::vm::storage::{MediaStorageState, Provider, Ready};
use gpui_kit::{Context, EventEmitter, Task};

use super::services::{MediaServices, StorageSnapshot};

/// What changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageEvent {
    /// The acknowledged provider / availability (or the pending flag) changed.
    Changed,
    /// A provider save finished; queued uploads proceed when `saved`.
    Acked {
        /// The new value was written.
        saved: bool,
    },
    /// Recovery switched S3 → Local: announce once (`StorageFallback` banner).
    Fallback,
}

/// The shared provider state.
pub struct StorageModel {
    services: Arc<MediaServices>,
    state: MediaStorageState,
    save: Option<Task<()>>,
    refresh: Option<Task<()>>,
}

impl EventEmitter<StorageEvent> for StorageModel {}

impl StorageModel {
    /// Start from Local and read the real value in the background.
    pub fn new(services: Arc<MediaServices>, cx: &mut Context<Self>) -> Self {
        let mut model = Self {
            services,
            state: MediaStorageState::new(Provider::Local, false),
            save: None,
            refresh: None,
        };
        model.refresh(cx);
        model
    }

    /// The state.
    pub fn state(&self) -> &MediaStorageState {
        &self.state
    }

    /// Uploads call this first.
    pub fn ready(&mut self) -> Ready {
        self.state.ready()
    }

    /// The user picked a provider: persist it now.
    pub fn request(&mut self, provider: Provider, cx: &mut Context<Self>) {
        if !self.state.request(provider) {
            return;
        }
        cx.emit(StorageEvent::Changed);
        cx.notify();
        let services = Arc::clone(&self.services);
        let work = cx.background_executor().spawn(async move {
            let saved = services.settings.set_provider(provider).map(|_| ());
            services.api.recovery.clear();
            saved.map(|()| {
                let snapshot = services.storage_snapshot();
                (snapshot.provider, snapshot.s3_available)
            })
        });
        self.save = Some(cx.spawn(async move |this, cx| {
            let result = work.await;
            this.update(cx, |this, cx| {
                let outcome = this.state.ack(result.map_err(|_| None));
                cx.emit(StorageEvent::Acked {
                    saved: outcome.saved,
                });
                cx.emit(StorageEvent::Changed);
                cx.notify();
            })
            .ok();
        }));
    }

    /// Re-read `.fed` (dialog open, after an S3 failure, `.fed` changed).
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let services = Arc::clone(&self.services);
        let work = cx
            .background_executor()
            .spawn(async move { services.storage_snapshot() });
        self.refresh = Some(cx.spawn(async move |this, cx| {
            let snapshot = work.await;
            this.update(cx, |this, cx| this.apply(snapshot, cx)).ok();
        }));
    }

    /// Apply a snapshot read elsewhere (e.g. with the picker config).
    pub fn apply(&mut self, snapshot: StorageSnapshot, cx: &mut Context<Self>) {
        let announce =
            self.state
                .apply(snapshot.provider, snapshot.s3_available, snapshot.fallback);
        if announce {
            cx.emit(StorageEvent::Fallback);
        }
        cx.emit(StorageEvent::Changed);
        cx.notify();
    }
}
