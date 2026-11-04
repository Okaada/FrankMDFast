//! **[COMPAT]** port of `app/services/s3_media_library_service.rb`: the S3
//! Library (objects under `frankmd/` only).

use crate::config::MediaConfig;
use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::library::{LibraryListing, RenamedItem};

/// `PREFIX` = `"#{S3_KEY_PREFIX}/"`.
pub const PREFIX: &str = "frankmd/";
/// Max key length in bytes (`valid_key?`).
pub const MAX_KEY_BYTES: usize = 1024;
/// `MAX_OBJECTS` guard while paginating (`"S3 Library contains too many objects"`).
pub const MAX_OBJECTS: usize = 10_000;

/// S3 Library bound to one config snapshot.
#[derive(Debug)]
pub struct S3MediaLibrary {
    config: MediaConfig,
    client: crate::s3::S3Client,
}

impl S3MediaLibrary {
    /// `S3MediaLibraryService.new(config:)` (client from `env.s3_client`).
    pub fn new(env: &MediaEnv, config: &MediaConfig) -> Result<Self, MediaError> {
        todo!()
    }

    /// `list`: `ListObjectsV2(prefix: "frankmd/")` until not truncated; supported
    /// extensions only; items carry `provider: "s3"` and the `s3_url` URL.
    /// A truncated page without a token → `Rejected("S3 Library pagination failed")`.
    pub async fn list(&self) -> Result<LibraryListing, MediaError> {
        todo!()
    }

    /// `delete(key)`: `Ok(false)` without any request for foreign, traversal or
    /// unsupported keys; else `DeleteObject` → `Ok(true)`.
    pub async fn delete(&self, key: &str) -> Result<bool, MediaError> {
        todo!()
    }

    /// `rename(key, new_name)`: validate the name (basename, sanitized, same
    /// media kind, ≤ 1024-byte key), `HeadObject` the sibling (404 = free), then
    /// `CopyObject(copy_source: "<bucket>/<url-encoded old key>")`, then
    /// `DeleteObject(old)`. `Ok(None)` (no mutation) for invalid names or an
    /// existing target.
    pub async fn rename(&self, key: &str, new_name: &str) -> Result<Option<RenamedItem>, MediaError> {
        todo!()
    }
}
