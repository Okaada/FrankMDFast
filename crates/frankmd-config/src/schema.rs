//! The `.fed` schema (`Config::SCHEMA`, backend.md §1.2). One row per key.

/// Value type of a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// Free string.
    String,
    /// Ruby `to_i` semantics.
    Integer,
    /// `true|1|yes|on` (case-insensitive) → true, anything else false.
    Boolean,
    /// JSON value (`recent_files`).
    Json,
    /// Validated custom theme JSON object.
    CustomTheme,
}

impl KeyType {
    /// Ruby schema type name (`string`, `integer`, `boolean`, `json`, `custom_theme`).
    pub fn as_str(self) -> &'static str {
        todo!()
    }
}

/// Static description of one config key.
#[derive(Debug, Clone, Copy)]
pub struct KeySpec {
    /// Key as written in `.fed`.
    pub name: &'static str,
    /// Value type.
    pub ty: KeyType,
    /// Default as it would be written in `.fed` (`None` = nil), e.g. `"14"`,
    /// `"true"`, `"[]"`; cast with [`crate::value::cast`].
    pub default: Option<&'static str>,
    /// ENV variable consulted when the key is absent from `.fed`.
    pub env: Option<&'static str>,
    /// Exposed to / writable from the Settings UI (`UI_KEYS`).
    pub ui: bool,
    /// Masked in logs and never sent to the UI (`SENSITIVE_KEYS`).
    pub secret: bool,
}

/// Typed handle for a schema key. Variants mirror backend.md §1.2 one-to-one,
/// **in schema order** (the order of [`Key::ALL`] is observable: `/logs/config`
/// entries, `all_settings`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Key {
    Theme, RecentFiles, CustomTheme, Locale, EditorFont, EditorFontSize, PreviewZoom,
    SidebarVisible, ViewMode, TypewriterMode, CursorRecenterOnClick, VimMode, ScrollSync,
    EditorIndent, EditorLineNumbers, ShowStats, EditorWidth, ImagesPath, MediaStorageProvider,
    ImageUploadExtensions, VideoUploadExtensions, AwsAccessKeyId, AwsSecretAccessKey,
    AwsS3Bucket, AwsRegion, AwsS3Endpoint, AwsS3ForcePathStyle, YoutubeApiKey, GoogleApiKey,
    GoogleCseId, AiProvider, AiModel, OllamaApiBase, OllamaModel, OpenrouterApiKey,
    OpenrouterModel, AnthropicApiKey, AnthropicModel, GeminiApiKey, GeminiModel, OpenaiApiKey,
    OpenaiModel, HugoPathStyle, AuthToken, ImageGenerationModel,
}

impl Key {
    /// Every key in schema order (45).
    pub const ALL: [Key; 45] = [
        Key::Theme, Key::RecentFiles, Key::CustomTheme, Key::Locale, Key::EditorFont,
        Key::EditorFontSize, Key::PreviewZoom, Key::SidebarVisible, Key::ViewMode,
        Key::TypewriterMode, Key::CursorRecenterOnClick, Key::VimMode, Key::ScrollSync,
        Key::EditorIndent, Key::EditorLineNumbers, Key::ShowStats, Key::EditorWidth,
        Key::ImagesPath, Key::MediaStorageProvider, Key::ImageUploadExtensions,
        Key::VideoUploadExtensions, Key::AwsAccessKeyId, Key::AwsSecretAccessKey,
        Key::AwsS3Bucket, Key::AwsRegion, Key::AwsS3Endpoint, Key::AwsS3ForcePathStyle,
        Key::YoutubeApiKey, Key::GoogleApiKey, Key::GoogleCseId, Key::AiProvider, Key::AiModel,
        Key::OllamaApiBase, Key::OllamaModel, Key::OpenrouterApiKey, Key::OpenrouterModel,
        Key::AnthropicApiKey, Key::AnthropicModel, Key::GeminiApiKey, Key::GeminiModel,
        Key::OpenaiApiKey, Key::OpenaiModel, Key::HugoPathStyle, Key::AuthToken,
        Key::ImageGenerationModel,
    ];

    /// Static spec for this key.
    pub fn spec(self) -> &'static KeySpec {
        todo!("table from backend.md §1.2")
    }

    /// `.fed` name (`spec().name`).
    pub fn name(self) -> &'static str {
        self.spec().name
    }

    /// Exact, case-sensitive lookup by schema name (`update("THEME" => …)` is
    /// skipped in the original). The `.fed` *parser* lowercases line keys
    /// before calling this (**[COMPAT]** `THEME = x` lines work).
    pub fn from_name(name: &str) -> Option<Key> {
        todo!()
    }

    /// The five `AI_CREDENTIAL_KEYS` that switch on file-only AI mode.
    pub const AI_CREDENTIALS: [Key; 5] = [
        Key::OllamaApiBase,
        Key::OpenrouterApiKey,
        Key::AnthropicApiKey,
        Key::GeminiApiKey,
        Key::OpenaiApiKey,
    ];

    /// The 12 AI keys whose presence (even commented, `/^#?\s*key\s*=/i`)
    /// suppresses the AI-section upgrade (backend.md §1.6).
    pub const AI_UPGRADE_MARKERS: [Key; 12] = [
        Key::AiProvider, Key::AiModel, Key::OllamaApiBase, Key::OllamaModel,
        Key::OpenrouterApiKey, Key::OpenrouterModel, Key::AnthropicApiKey, Key::AnthropicModel,
        Key::GeminiApiKey, Key::GeminiModel, Key::OpenaiApiKey, Key::OpenaiModel,
    ];

    /// `UI_KEYS` in original order (17 keys).
    pub fn ui_keys() -> Vec<Key> {
        todo!()
    }
}
