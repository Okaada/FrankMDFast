//! Configuration errors.

use crate::Key;

/// Errors from parsing, validating or writing configuration. The `Display`
/// strings of the validation variants are the original Ruby messages.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A string value contains `"`, `\r` or `\n` (would inject `.fed` lines).
    #[error("String configuration values cannot contain quotes or line breaks")]
    UnsafeString {
        /// Config key.
        key: Key,
    },
    /// `media_storage_provider` is not `local`/`s3`.
    #[error("Unknown media storage provider")]
    UnknownMediaProvider,
    /// `media_storage_provider = s3` while S3 is not configured.
    #[error("S3 is not configured")]
    S3NotConfigured,
    /// A custom theme failed validation.
    #[error(transparent)]
    CustomTheme(#[from] frankmd_theme::CustomThemeError),
    /// A JSON key was given an invalid JSON string.
    #[error("invalid JSON for {key:?}: {reason}")]
    InvalidJson {
        /// Config key.
        key: Key,
        /// Parser message.
        reason: String,
    },
    /// An input value of the wrong kind (e.g. a boolean for an integer key;
    /// the original raised `NoMethodError`).
    #[error("invalid value for {key:?}: {reason}")]
    InvalidValue {
        /// Config key.
        key: Key,
        /// Human-readable reason (not shown verbatim to users).
        reason: String,
    },
    /// A Settings-UI update contained no `UI_KEYS` key (i18n `errors.no_valid_settings`,
    /// "No valid settings provided").
    #[error("No valid settings provided")]
    NoValidSettings,
    /// The key may not be stored in the global config (`recent_files`, `auth_token`).
    #[error("{key:?} cannot be set in the global config")]
    GlobalKeyNotAllowed {
        /// Config key.
        key: Key,
    },
    /// The global config file is malformed or cannot be serialized.
    #[error("global config: {0}")]
    GlobalFile(String),
    /// No global config path is attached to the store.
    #[error("no global config attached")]
    NoGlobalConfig,
    /// `.fed` could not be read or written (directory, read-only mount,
    /// symlinked lock file, ...). Reads degrade to defaults; writes fail.
    #[error(transparent)]
    Fs(#[from] frankmd_fs::FsError),
}
