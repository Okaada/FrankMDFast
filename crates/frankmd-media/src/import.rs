//! **[COMPAT]** port of `ImagesService.import_image` (`POST /images/import`):
//! exactly one string source, either an external URL (downloaded through the
//! egress guard) or a path inside the server image folder; stored with the
//! active provider.

use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::param::Param;

/// Import parameters (raw, as the endpoint received them).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportRequest {
    /// External URL.
    pub url: Param,
    /// Path relative to `images_path`.
    pub path: Param,
    /// Parsed resize ratio.
    pub resize: Option<f64>,
    /// Custom S3 key.
    pub s3_key: Option<String>,
    /// Custom S3 prefix.
    pub s3_prefix: Option<String>,
    /// Legacy destination flag.
    pub upload_to_s3: Param,
}

/// Accepted external content types (after `;` split, downcased).
pub const IMPORT_MIME_TYPES: [&str; 6] =
    ["image/jpeg", "image/png", "image/gif", "image/webp", "image/bmp", "image/x-ms-bmp"];

/// `import_image(...)` → `url`. Errors (all `Invalid` unless noted):
/// "Provide exactly one image URL or server image path", "Image not found",
/// "Image download failed" (fetch failure or egress block),
/// "Unsupported image content type", allow-list `Rejected`, S3 collision →
/// `Upstream("Image upload failed")`, other S3 errors propagate.
pub async fn import_image(env: &MediaEnv, req: &ImportRequest) -> Result<String, MediaError> {
    todo!("backend.md §7.7")
}
