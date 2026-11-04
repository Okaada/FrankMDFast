//! # frankmd-media
//!
//! **One upload pipeline (HP-015, HP-009 lesson 3.9).** Drop, paste, picker,
//! URL import and AI-generated images all call [`store_media`]; there is no
//! other path that writes media. It:
//! 1. checks the extension against `image_upload_extensions` /
//!    `video_upload_extensions` (**[COMPAT]** parsing) *and* magic bytes,
//! 2. optionally resizes/re-encodes images,
//! 3. picks a random, collision-free name and writes no-clobber,
//! 4. targets the **backend-resolved** active provider (`media_storage_provider`),
//!    never a UI-supplied one (HP-014).
//!
//! Module map (each mirrors one original service/controller):
//! - [`api`]: endpoint-level facade (what the Rails controllers did: status classes,
//!   i18n error keys, the one-shot `media_storage_fallback` flag).
//! - [`env`] / [`param`]: injected services (settings store, HTTP clients, S3 transport
//!   seam, limits) and loosely-typed request parameters.
//! - [`config`] / [`ctx`]: the media view of `ResolvedConfig`; injectable clock + randomness.
//! - [`upload_storage`]: `UploadStorage` (naming, validation, provider, S3 keys/URLs).
//! - [`s3`]: minimal S3 client (PutObject `If-None-Match: *`, List/Head/Copy/Delete).
//! - [`images`]: `ImagesService` (server image folder, uploads, base64, S3, external fetch).
//! - [`video`]: `MediaService` (video uploads).
//! - [`import`]: `ImagesController#import` (one source: external URL or server path).
//! - [`recovery`]: `MediaStorageRecovery` (S3 unusable → persist `local` + one-shot warning).
//! - [`library`]: `MediaLibraryService` + `ImageLibraryService` (picker collections).
//! - [`usage`]: `MediaUsageService` (which notes reference which media).
//! - [`s3_library`] — `S3MediaLibraryService`.
//! - [`search`]: web image search (DuckDuckGo, Google CSE, Pinterest).
//! - [`youtube`]: YouTube Data API search.
//! - [`picker`]: client-side image-source logic ported from `lib/image_sources/*.js`.
//! - [`s3_key`], [`url_utils`], [`clipboard`]: small JS helpers.
#![allow(unused_variables, dead_code)] // skeleton

pub mod api;
pub mod clipboard;
pub mod config;
pub mod ctx;
pub mod env;
pub mod error;
pub mod images;
pub mod import;
pub mod library;
pub mod param;
pub mod picker;
pub mod recovery;
pub mod s3;
pub mod s3_key;
pub mod s3_library;
pub mod search;
pub mod upload_storage;
pub mod url_utils;
pub mod usage;
pub mod video;
pub mod youtube;

pub use config::{MediaConfig, S3Settings, StorageProvider};
pub use ctx::{Clock, MediaCtx, RandomHex};
pub use env::{MediaEnv, MemorySettings, SettingsStore, UploadLimits};
pub use error::MediaError;
pub use param::Param;

use frankmd_core::RelPath;

/// Where the bytes came from (for logging/metrics; never changes policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum UploadOrigin {
    Drop,
    Paste,
    Picker,
    UrlImport,
    AiGenerated,
}

/// Media kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum MediaKind {
    Image,
    Video,
}

/// A request to store media.
#[derive(Debug, Clone)]
pub struct UploadRequest {
    /// Origin.
    pub origin: UploadOrigin,
    /// Original file name (only its extension is trusted, after checks).
    pub file_name: String,
    /// Raw bytes (already size-capped by the caller's reader).
    pub bytes: Vec<u8>,
    /// Requested resize (images only), e.g. `Some(1920)` max width.
    pub max_width: Option<u32>,
    /// Optional S3 key chosen by the user (validated, never overwrites).
    pub s3_key: Option<String>,
}

/// Result: what to insert into the note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredMedia {
    /// Stored in the vault (`images/…`); markdown uses the relative path.
    Local(RelPath),
    /// Stored in S3; markdown uses the public URL.
    S3 {
        /// Public URL.
        url: String,
        /// Object key.
        key: String,
    },
}

impl StoredMedia {
    /// The `url` the original endpoints returned (`images/<file>` or the S3 URL).
    pub fn url(&self) -> &str {
        match self {
            StoredMedia::Local(p) => p.as_str(),
            StoredMedia::S3 { url, .. } => url,
        }
    }
}

/// The single entry point for storing media.
pub async fn store_media(
    vault: &frankmd_fs::Vault,
    config: &frankmd_config::ResolvedConfig,
    req: UploadRequest,
) -> Result<StoredMedia, MediaError> {
    todo!("backend.md §7.2-7.6")
}

/// Validate extension + magic bytes without storing.
pub fn validate(file_name: &str, bytes: &[u8], allowed_exts: &[String]) -> Result<MediaKind, MediaError> {
    todo!()
}
