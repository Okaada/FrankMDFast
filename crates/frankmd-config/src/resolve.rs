//! `.fed > ENV > default` resolution with provenance.

use crate::{ConfigError, EnvSnapshot, Key, Value};

/// Where a resolved value came from (shown in Settings: "set by ENV", ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Key present in `.fed` (possibly with an empty value → `None`).
    Fed,
    /// Taken from the named ENV variable.
    Env(&'static str),
    /// Schema default.
    Default,
}

impl Provenance {
    /// **[COMPAT]** `LogsController#config_source` label: `file`, `env`, `default`.
    pub fn as_str(self) -> &'static str {
        match self {
            Provenance::Fed => "file",
            Provenance::Env(_) => "env",
            Provenance::Default => "default",
        }
    }
}

/// A value plus its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    /// The value (`None` = nil).
    pub value: Option<Value>,
    /// Which layer supplied it.
    pub source: Provenance,
}

/// AI providers known to the original app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum AiProviderId {
    Openai,
    Anthropic,
    Openrouter,
    Ollama,
    Gemini,
}

impl AiProviderId {
    /// **[COMPAT]** `AI_PROVIDER_PRIORITY` (auto selection order).
    pub const PRIORITY: [AiProviderId; 5] = [
        AiProviderId::Openai,
        AiProviderId::Anthropic,
        AiProviderId::Openrouter,
        AiProviderId::Ollama,
        AiProviderId::Gemini,
    ];

    /// **[COMPAT]** order of `ai_providers_available` (`available_providers`).
    pub const AVAILABILITY_ORDER: [AiProviderId; 5] = [
        AiProviderId::Ollama,
        AiProviderId::Openrouter,
        AiProviderId::Anthropic,
        AiProviderId::Gemini,
        AiProviderId::Openai,
    ];

    /// Id as used in `.fed`/JSON (`openai`, ...).
    pub fn as_str(self) -> &'static str {
        todo!()
    }

    /// Parse an id (exact, lowercase). `auto` is not a provider → `None`.
    pub fn from_name(name: &str) -> Option<Self> {
        todo!()
    }

    /// The credential key that makes this provider available.
    pub fn credential_key(self) -> Key {
        todo!()
    }

    /// The `<provider>_model` key.
    pub fn model_key(self) -> Key {
        todo!()
    }
}

/// `feature_available?` features.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Feature {
    S3Upload,
    YoutubeSearch,
    GoogleSearch,
    LocalImages,
    Ai,
}

/// One `/logs/config` row (config introspection).
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigEntry {
    /// The key.
    pub key: Key,
    /// JSON value; `"****"` for a non-nil sensitive value.
    pub value: serde_json::Value,
    /// Source layer.
    pub source: Provenance,
    /// ENV variable name, if the key has one.
    pub env_var: Option<&'static str>,
    /// In `SENSITIVE_KEYS`.
    pub sensitive: bool,
}

/// An immutable, fully resolved view of the configuration for one vault.
/// Rebuilt (never mutated) whenever `.fed` changes on disk or is written.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    file: Vec<(Key, Option<Value>)>,
    env: EnvSnapshot,
    /// Problems found while loading (skipped lines, undecodable file, ...),
    /// surfaced in the UI. Each names the offending key when there is one.
    pub warnings: Vec<String>,
}

impl ResolvedConfig {
    /// Build from `.fed` text (or `None` if absent) and an ENV snapshot.
    pub fn new(fed_text: Option<&str>, env: EnvSnapshot) -> Self {
        todo!()
    }

    /// Build from raw `.fed` bytes. Invalid UTF-8 → no file values at all plus
    /// a warning (**[COMPAT]**: Ruby's parse raises and `@values = {}`).
    pub fn from_bytes(fed: Option<&[u8]>, env: EnvSnapshot) -> Self {
        todo!()
    }

    /// The ENV snapshot this view was built with.
    pub fn env(&self) -> &EnvSnapshot {
        &self.env
    }

    /// Keys present in `.fed` (Ruby `@values`), in first-insertion order.
    pub fn file_values(&self) -> &[(Key, Option<Value>)] {
        &self.file
    }

    /// **[COMPAT]** `Config#get`.
    pub fn get(&self, key: Key) -> Resolved {
        todo!("backend.md §1.3")
    }

    /// `get(key).value`.
    pub fn value(&self, key: Key) -> Option<Value> {
        todo!()
    }

    /// **[COMPAT]** `Config#get_ai` (file-only mode when any AI credential key
    /// is present in `.fed`: ENV ignored, `.fed` value or schema default).
    pub fn get_ai(&self, key: Key) -> Resolved {
        todo!()
    }

    /// **[COMPAT]** `ai_configured_in_file?`: any of [`Key::AI_CREDENTIALS`]
    /// present in `.fed`, even with an empty value.
    pub fn ai_configured_in_file(&self) -> bool {
        todo!()
    }

    /// **[COMPAT]** `ai_providers_available` (order: ollama, openrouter,
    /// anthropic, gemini, openai).
    pub fn ai_providers_available(&self) -> Vec<AiProviderId> {
        todo!()
    }

    /// **[COMPAT]** `effective_ai_provider` (priority openai > anthropic >
    /// openrouter > ollama > gemini).
    pub fn effective_ai_provider(&self) -> Option<AiProviderId> {
        todo!()
    }

    /// **[COMPAT]** `effective_ai_model`: `get_ai(ai_model)` if present, else
    /// `get_ai(<provider>_model)`; `None` without a provider.
    pub fn effective_ai_model(&self) -> Option<String> {
        todo!()
    }

    /// **[COMPAT]** image generation bypasses file-only mode
    /// (`@values["openrouter_api_key"] || ENV["OPENROUTER_API_KEY"]`; an empty
    /// file value is nil and falls through; a blank ENV value disables).
    pub fn image_generation_key(&self) -> Option<secrecy::SecretString> {
        todo!()
    }

    /// **[COMPAT]** `upload_extensions`: split on `,`, trim, downcase, drop blanks.
    pub fn upload_extensions(&self, key: Key) -> Vec<String> {
        todo!()
    }

    /// **[COMPAT]** `UploadStorage.s3_configured?`: access key, secret and bucket
    /// present, and `aws_s3_endpoint` (if set) a valid http(s) URL with a host.
    pub fn s3_configured(&self) -> bool {
        todo!()
    }

    /// **[COMPAT]** `feature_available?`.
    pub fn feature_available(&self, feature: Feature) -> bool {
        todo!()
    }

    /// **[COMPAT]** `ui_settings`: every `UI_KEYS` key with its resolved value.
    pub fn ui_settings(&self) -> Vec<(Key, Option<Value>)> {
        todo!()
    }

    /// **[COMPAT]** `all_settings` (masked): sensitive keys appear as
    /// `<key>_configured: bool`, everything else as its JSON value; schema order.
    pub fn all_settings_masked(&self) -> Vec<(String, serde_json::Value)> {
        todo!()
    }

    /// **[COMPAT]** `/logs/config` entries, one per schema key in order.
    pub fn entries(&self) -> Vec<ConfigEntry> {
        todo!()
    }

    /// **[COMPAT]** `format_value`: the canonical `key = value` line for a
    /// write-back, validating against this config (S3 must be configured for
    /// `media_storage_provider = s3`).
    pub fn format_line(&self, key: Key, value: Option<&Value>) -> Result<String, ConfigError> {
        todo!("backend.md §1.8")
    }
}
