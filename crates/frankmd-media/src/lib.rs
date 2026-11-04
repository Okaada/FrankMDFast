//! # frankmd-media
//!
//! **One upload pipeline (HP-015, HP-009 lesson 3.9).** Drop, paste, picker,
//! base64/AI images, URL and server-folder imports, the legacy S3 endpoints
//! and videos all end in [`store_media`]; there is no other path that writes
//! media. It:
//! 1. enforces the size cap and the extension allow-list
//!    (`image_upload_extensions` / `video_upload_extensions`, **[COMPAT]** messages),
//! 2. **sniffs the content** ([`sniff`]): only real images/videos of an
//!    allow-listed type are stored — SVG, HTML, text and polyglots are refused
//!    whatever their extension (security decision; the original trusted the name),
//! 3. optionally resizes/re-encodes images (`image` crate),
//! 4. picks a random, collision-resistant name and writes no-clobber
//!    (`frankmd-fs`) or `PutObject If-None-Match: *` (S3),
//! 5. targets the **backend-resolved** provider (`media_storage_provider`),
//!    never a UI-supplied one (HP-014).
//!
//! Module map (each mirrors one original service/controller):
//! - [`pipeline`] / [`sniff`]: the single store path and its content checks.
//! - [`api`]: endpoint-level facade (what the Rails controllers did: status classes,
//!   i18n error keys, the one-shot `media_storage_fallback` flag).
//! - [`env`] / [`param`]: injected services (settings store, HTTP clients, S3 transport
//!   seam, limits) and loosely-typed request parameters.
//! - [`config`] / [`ctx`]: the media view of `ResolvedConfig`; injectable clock + randomness.
//! - [`upload_storage`]: `UploadStorage` (naming, validation, provider, S3 keys/URLs).
//! - [`s3`]: `aws-sdk-s3` client over `TrustedClient` (PutObject `If-None-Match: *`, List/Head/Copy/Delete).
//! - [`images`]: `ImagesService` (uploads, base64, legacy S3, external fetch, resizing).
//! - [`server_folder`]: the `images_path` half of `ImagesService` (list, preview, delete).
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
//! - [`vm`]: GUI-free view models of the media dialogs (Insert image/video, library, storage).

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
pub mod pipeline;
pub mod recovery;
pub mod s3;
mod s3_http;
pub mod s3_key;
pub mod s3_library;
pub mod search;
pub mod server_folder;
pub mod sniff;
pub mod upload_storage;
pub mod url_utils;
pub mod usage;
pub mod video;
pub mod vm;
pub mod youtube;

pub use config::{MediaConfig, S3Settings, StorageProvider};
pub use ctx::{Clock, MediaCtx, RandomHex};
pub use env::{MediaEnv, MemorySettings, SettingsStore, UploadLimits};
pub use error::MediaError;
pub use param::Param;
pub use pipeline::{StoredMedia, UploadRequest, store_media, validate_bytes};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum MediaKind {
    Image,
    Video,
}
