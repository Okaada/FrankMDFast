//! **[COMPAT]** port of `app/controllers/concerns/media_storage_recovery.rb`.
//!
//! When `media_storage_provider = s3` but S3 is unusable (missing credentials,
//! invalid endpoint, or an S3 call failed with a definitive configuration
//! error), persist `local` and raise a **one-shot** warning
//! (`media_storage_fallback: true`). The Rails session flag becomes in-memory
//! app state here ([`StorageRecovery`]).

use std::sync::atomic::{AtomicBool, Ordering};

use crate::env::SettingsStore;
use crate::error::MediaError;

/// The pending-warning flag (Rails `session[:media_storage_fallback]`).
#[derive(Debug, Default)]
pub struct StorageRecovery {
    pending: AtomicBool,
}

impl StorageRecovery {
    /// No pending warning.
    pub fn new() -> Self {
        Self::default()
    }

    /// Is a warning pending (without consuming it)?
    pub fn pending(&self) -> bool {
        self.pending.load(Ordering::SeqCst)
    }

    /// `recover_media_storage(config, consume:)` (configuration refreshes,
    /// page loads, `/images/config`): if s3 is selected but unconfigured and
    /// persisting `local` succeeds, set the flag. Returns `flag && provider == local`;
    /// clears the flag when `consume`.
    pub fn recover_media_storage(&self, settings: &dyn SettingsStore, consume: bool) -> bool {
        if s3_selected_but_unusable(settings) && settings.set_provider_local() {
            self.pending.store(true, Ordering::SeqCst);
        }
        let recovered =
            self.pending() && settings.current().media_storage_provider.as_deref() == Some("local");
        if consume {
            self.clear();
        }
        recovered
    }

    /// `recover_missing_upload_storage` (before every upload): same check;
    /// on a successful switch, set the flag and return
    /// `Err(MediaError::StorageFallback)` so nothing is written.
    pub fn recover_missing_upload_storage(
        &self,
        settings: &dyn SettingsStore,
    ) -> Result<(), MediaError> {
        if s3_selected_but_unusable(settings) && settings.set_provider_local() {
            self.pending.store(true, Ordering::SeqCst);
            return Err(MediaError::StorageFallback);
        }
        Ok(())
    }

    /// `recover_s3_upload(error)` (after an upload failed): only for
    /// [`crate::upload_storage::invalid_s3_configuration_error`] while the provider
    /// is still s3 and persisting `local` succeeds → set the flag, `true`.
    pub fn recover_s3_upload(&self, settings: &dyn SettingsStore, error: &MediaError) -> bool {
        let MediaError::S3(s3_error) = error else {
            return false;
        };
        if !crate::upload_storage::invalid_s3_configuration_error(s3_error) {
            return false;
        }
        if settings.current().media_storage_provider.as_deref() != Some("s3")
            || !settings.set_provider_local()
        {
            return false;
        }
        self.pending.store(true, Ordering::SeqCst);
        true
    }

    /// Clear the flag (`PATCH /config` touching `media_storage_provider`).
    pub fn clear(&self) {
        self.pending.store(false, Ordering::SeqCst);
    }
}

/// `provider == "s3" && !feature_available?(:s3_upload)`.
fn s3_selected_but_unusable(settings: &dyn SettingsStore) -> bool {
    let config = settings.current();
    config.media_storage_provider.as_deref() == Some("s3")
        && !crate::upload_storage::s3_configured(&config)
}
