//! Injected services for the upload / import / recovery code paths.
//!
//! The original re-read `Config.new` on every call, so a provider switch made
//! by recovery was visible to the very next request. [`SettingsStore`] models
//! that: `current()` is always fresh, and `set_provider_local()` is the
//! `Config#set("media_storage_provider", "local")` write (which can fail).

use std::sync::{Arc, Mutex};

use crate::config::MediaConfig;
use crate::ctx::MediaCtx;

/// Source of the current media configuration plus the one write recovery needs.
pub trait SettingsStore: Send + Sync + std::fmt::Debug {
    /// Fresh snapshot (after any earlier write).
    fn current(&self) -> MediaConfig;
    /// Persist `media_storage_provider = local` (`.fed`, atomic, under the
    /// config lock). `false` when the write failed (read-only vault, ...).
    fn set_provider_local(&self) -> bool;
}

/// In-memory store (tests, previews). Optionally refuses writes to model
/// `Config#set` returning `false`.
#[derive(Debug)]
pub struct MemorySettings {
    config: Mutex<MediaConfig>,
    fail_writes: Mutex<bool>,
}

impl MemorySettings {
    /// Wrap a config.
    pub fn new(config: MediaConfig) -> Arc<Self> {
        Arc::new(Self { config: Mutex::new(config), fail_writes: Mutex::new(false) })
    }

    /// Mutate the stored config (stands in for the user editing `.fed`).
    pub fn update(&self, f: impl FnOnce(&mut MediaConfig)) {
        if let Ok(mut c) = self.config.lock() {
            f(&mut c);
        }
    }

    /// Make `set_provider_local` fail (`Config#set` → `false`).
    pub fn fail_writes(&self, fail: bool) {
        if let Ok(mut f) = self.fail_writes.lock() {
            *f = fail;
        }
    }
}

impl SettingsStore for MemorySettings {
    fn current(&self) -> MediaConfig {
        todo!()
    }

    fn set_provider_local(&self) -> bool {
        todo!()
    }
}

/// Size caps (**[COMPAT]** `UploadStorage::MAX_UPLOAD_BYTES` and
/// `max_base64_encoded_bytes`). Overridable so tests need not allocate 512 MB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadLimits {
    /// Max decoded upload size (536870912).
    pub max_upload_bytes: u64,
    /// Max base64 *encoded* size, checked before decoding (715828908).
    pub max_base64_encoded_bytes: u64,
}

impl Default for UploadLimits {
    fn default() -> Self {
        Self {
            max_upload_bytes: crate::upload_storage::MAX_UPLOAD_BYTES,
            max_base64_encoded_bytes: crate::upload_storage::max_base64_encoded_bytes(),
        }
    }
}

/// Everything an upload needs, injected.
#[derive(Debug, Clone)]
pub struct MediaEnv {
    /// Live settings.
    pub settings: Arc<dyn SettingsStore>,
    /// Clock + randomness.
    pub ctx: MediaCtx,
    /// Trusted HTTP client (S3).
    pub http: frankmd_net::TrustedClient,
    /// SSRF-guarded client (external image URLs).
    pub egress: frankmd_net::EgressClient,
    /// **Test seam**: send every S3 request to this base URL (path style,
    /// e.g. a wiremock server) while public URLs are still formatted from the
    /// configured endpoint/region exactly like the original (`s3_url`).
    pub s3_transport: Option<url::Url>,
    /// Size caps.
    pub limits: UploadLimits,
}

impl MediaEnv {
    /// Production wiring with default limits and no transport override.
    pub fn new(
        settings: Arc<dyn SettingsStore>,
        ctx: MediaCtx,
        http: frankmd_net::TrustedClient,
        egress: frankmd_net::EgressClient,
    ) -> Self {
        Self { settings, ctx, http, egress, s3_transport: None, limits: UploadLimits::default() }
    }

    /// S3 client for the given snapshot (honours [`MediaEnv::s3_transport`]).
    pub fn s3_client(&self, config: &MediaConfig) -> Result<crate::s3::S3Client, crate::s3::S3Error> {
        todo!()
    }
}
