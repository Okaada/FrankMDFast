//! "Storage for new media" (`media_storage_provider`, FE-355…357) and the
//! storage row in the Insert image / Insert video footers (FE-315/316).
//!
//! [`MediaStorageState`] is the app-wide value: choosing a provider persists
//! it at once (`.fed`), every control shows the **acknowledged** value and is
//! disabled while the save is pending, and uploads wait for the save
//! ([`MediaStorageState::ready`]). Uploads always go to the backend-resolved
//! provider (HP-014), so the dialog's provider select *is* this setting, as in
//! FrankMD.
//!
//! [`StorageRow`] is one dialog's resize / destination-key controls.
//! Decisions (docs/testing/ui-test-plan.md OQ-68/69): resize is offered only
//! for S3 (options 25/50/75/100 %, default 50 %), and `reset` returns to the
//! global provider.

use super::Text;

/// A media storage provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Provider {
    /// `<notes>/images`, `<notes>/videos`.
    #[default]
    Local,
    /// The configured S3 bucket.
    S3,
}

impl Provider {
    /// From the `.fed` value (`s3` → S3, anything else → Local).
    pub fn from_setting(value: Option<&str>) -> Self {
        if value == Some("s3") { Self::S3 } else { Self::Local }
    }

    /// The `.fed` value.
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
        }
    }

    /// Label key (`media_storage.local` / `media_storage.s3`).
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Local => "media_storage.local",
            Self::S3 => "media_storage.s3",
        }
    }
}

/// Whether an upload may start now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ready {
    /// No save pending.
    Go,
    /// A provider save is pending; the upload was queued until [`MediaStorageState::ack`].
    Wait,
}

/// What an [`MediaStorageState::ack`] released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AckOutcome {
    /// The new provider was saved.
    pub saved: bool,
    /// Uploads that waited; they proceed when `saved`, else fail with
    /// `media_storage.save_failed`.
    pub released: usize,
}

/// The app-wide provider state (`media_storage_controller.js`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaStorageState {
    acknowledged: Provider,
    pending: Option<Provider>,
    s3_available: bool,
    error: Option<Text>,
    fallback_announced: bool,
    waiting: usize,
}

impl MediaStorageState {
    /// From the persisted provider and whether S3 is configured.
    pub fn new(provider: Provider, s3_available: bool) -> Self {
        Self { acknowledged: provider, pending: None, s3_available, error: None, fallback_announced: false, waiting: 0 }
    }

    /// The acknowledged provider (what every control shows).
    pub fn provider(&self) -> Provider {
        self.acknowledged
    }

    /// S3 is configured.
    pub fn s3_available(&self) -> bool {
        self.s3_available
    }

    /// A save is in flight.
    pub fn is_saving(&self) -> bool {
        self.pending.is_some()
    }

    /// Provider controls are disabled while saving.
    pub fn controls_disabled(&self) -> bool {
        self.is_saving()
    }

    /// Why S3 can't be chosen (`media_storage.not_configured`), if it can't.
    pub fn s3_disabled_reason(&self) -> Option<Text> {
        (!self.s3_available).then(|| Text::key("media_storage.not_configured"))
    }

    /// The inline error under the provider controls.
    pub fn error(&self) -> Option<&Text> {
        self.error.as_ref()
    }

    /// The user picked `provider`. `true` = the caller must persist it now
    /// and report back with [`Self::ack`]; `false` = nothing to do (same
    /// value, a save already pending, or S3 not configured).
    pub fn request(&mut self, provider: Provider) -> bool {
        if self.pending.is_some() || provider == self.acknowledged {
            return false;
        }
        if provider == Provider::S3 && !self.s3_available {
            return false;
        }
        self.error = None;
        self.pending = Some(provider);
        true
    }

    /// The save finished: `Ok((provider, s3_available))` from the re-read
    /// config, or `Err(message)`. A failure keeps the previous provider and
    /// shows `media_storage.save_failed` (with the cause when known).
    pub fn ack(&mut self, result: Result<(Provider, bool), Option<String>>) -> AckOutcome {
        let released = std::mem::take(&mut self.waiting);
        if self.pending.take().is_none() {
            return AckOutcome { saved: false, released };
        }
        match result {
            Ok((provider, s3_available)) => {
                self.acknowledged = provider;
                self.s3_available = s3_available;
                self.error = None;
                AckOutcome { saved: true, released }
            }
            Err(cause) => {
                self.error = Some(match cause {
                    Some(cause) if !cause.is_empty() => Text::Raw(cause),
                    _ => Text::key("media_storage.save_failed"),
                });
                AckOutcome { saved: false, released }
            }
        }
    }

    /// Uploads call this first; [`Ready::Wait`] queues the caller until the pending save acks.
    pub fn ready(&mut self) -> Ready {
        if self.pending.is_some() {
            self.waiting += 1;
            Ready::Wait
        } else {
            Ready::Go
        }
    }

    /// A configuration refresh (startup, `.fed` reload, after an upload
    /// failure). `fallback` = storage recovery switched S3 → Local. Returns
    /// `true` exactly once per session when the fallback must be announced
    /// (the `StorageFallback` banner, ui-spec §5.12).
    pub fn apply(&mut self, provider: Provider, s3_available: bool, fallback: bool) -> bool {
        if self.pending.is_none() {
            self.acknowledged = provider;
        }
        self.s3_available = s3_available;
        if fallback && provider == Provider::Local && !self.fallback_announced {
            self.fallback_announced = true;
            return true;
        }
        false
    }
}

/// How a dialog source stores the chosen image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowMode {
    /// Always stored (Upload, Folder, paste, video upload).
    #[default]
    Upload,
    /// Hotlinked unless "Save a copy in the active storage" is ticked (Web,
    /// Google, Pinterest, server image folder).
    Reupload,
    /// Stored; only an S3 destination folder, no resize (AI generate).
    S3Only,
}

/// Destination field shape (`key_mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyMode {
    /// Full object key `frankmd/YYYY/MM/<file>`.
    #[default]
    Full,
    /// Key prefix `frankmd/YYYY/MM` (the filename is appended by the store).
    Prefix,
}

/// The resize ratios offered for S3 (OQ-68).
pub const RESIZE_OPTIONS: [f64; 4] = [0.25, 0.5, 0.75, 1.0];

/// Default S3 resize ratio.
pub const DEFAULT_RESIZE: f64 = 0.5;

/// One dialog's storage row.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageRow {
    provider: Provider,
    s3_available: bool,
    resize: f64,
    key: String,
    key_dirty: bool,
    mode: RowMode,
    key_mode: KeyMode,
    save_copy: bool,
}

impl Default for StorageRow {
    fn default() -> Self {
        Self::new(Provider::Local, false)
    }
}

impl StorageRow {
    /// A row on `provider` (the global setting).
    pub fn new(provider: Provider, s3_available: bool) -> Self {
        let provider = if s3_available { provider } else { Provider::Local };
        Self {
            provider,
            s3_available,
            resize: DEFAULT_RESIZE,
            key: String::new(),
            key_dirty: false,
            mode: RowMode::Upload,
            key_mode: KeyMode::Full,
            save_copy: false,
        }
    }

    /// The row's provider.
    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// Follow the acknowledged global provider. Leaving S3 resets the resize.
    pub fn set_provider(&mut self, provider: Provider) {
        let provider = if provider == Provider::S3 && !self.s3_available { Provider::Local } else { provider };
        if self.provider == Provider::S3 && provider != Provider::S3 {
            self.resize = DEFAULT_RESIZE;
        }
        self.provider = provider;
    }

    /// S3 configuration appeared or disappeared.
    pub fn set_s3_available(&mut self, available: bool) {
        self.s3_available = available;
        if !available {
            self.set_provider(Provider::Local);
        }
    }

    /// Whether S3 can be picked.
    pub fn s3_enabled(&self) -> bool {
        self.s3_available
    }

    /// Configure for the selected source.
    pub fn set_mode(&mut self, mode: RowMode, key_mode: KeyMode) {
        self.mode = mode;
        self.key_mode = key_mode;
        if mode != RowMode::Reupload {
            self.save_copy = false;
        }
    }

    /// The source's row mode.
    pub fn mode(&self) -> RowMode {
        self.mode
    }

    /// The destination field shape.
    pub fn key_mode(&self) -> KeyMode {
        self.key_mode
    }

    /// The "Save a copy" checkbox is shown (reupload sources).
    pub fn save_copy_visible(&self) -> bool {
        self.mode == RowMode::Reupload
    }

    /// The checkbox state.
    pub fn save_copy(&self) -> bool {
        self.save_copy
    }

    /// Tick/untick "Save a copy". Unticking resets the resize.
    pub fn set_save_copy(&mut self, on: bool) {
        self.save_copy = on && self.mode == RowMode::Reupload;
        if !self.save_copy {
            self.resize = DEFAULT_RESIZE;
        }
    }

    /// The image is stored (uploaded / imported) rather than hotlinked.
    pub fn stores(&self) -> bool {
        self.mode != RowMode::Reupload || self.save_copy
    }

    /// Stores to S3.
    pub fn uses_s3(&self) -> bool {
        self.provider == Provider::S3 && self.stores()
    }

    /// The resize select is enabled (S3 only, not for AI images).
    pub fn resize_enabled(&self) -> bool {
        self.uses_s3() && self.mode != RowMode::S3Only
    }

    /// The offered ratios.
    pub fn resize_options(&self) -> [f64; 4] {
        RESIZE_OPTIONS
    }

    /// The selected ratio (shown even when disabled).
    pub fn resize(&self) -> f64 {
        self.resize
    }

    /// Pick a ratio (ignored when not one of [`RESIZE_OPTIONS`]).
    pub fn set_resize(&mut self, ratio: f64) {
        if RESIZE_OPTIONS.contains(&ratio) {
            self.resize = ratio;
        }
    }

    /// The ratio sent with the store request (`None` for local).
    pub fn resize_ratio(&self) -> Option<f64> {
        self.resize_enabled().then_some(self.resize)
    }

    /// The destination field is shown.
    pub fn key_visible(&self) -> bool {
        self.uses_s3()
    }

    /// A new selection's default key; keeps a key the user typed.
    pub fn set_default_key(&mut self, key: &str) {
        if !self.key_dirty {
            self.key = key.to_owned();
        }
    }

    /// The user edited the destination field.
    pub fn on_key_edited(&mut self, key: &str) {
        self.key = key.to_owned();
        self.key_dirty = true;
    }

    /// The field's text.
    pub fn key_text(&self) -> &str {
        &self.key
    }

    /// The key/prefix sent: S3 only, trimmed, non-empty.
    pub fn key_value(&self) -> Option<String> {
        let key = self.key.trim();
        (self.key_visible() && !key.is_empty()).then(|| key.to_owned())
    }

    /// Dialog reopen: back to the global provider, default resize, empty key.
    pub fn reset(&mut self, global: Provider) {
        self.key.clear();
        self.key_dirty = false;
        self.resize = DEFAULT_RESIZE;
        self.save_copy = false;
        self.provider = if self.s3_available { global } else { Provider::Local };
    }
}
