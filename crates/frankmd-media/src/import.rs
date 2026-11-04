//! **[COMPAT]** port of `ImagesService.import_image` (`POST /images/import`):
//! exactly one string source, either an external URL (downloaded through the
//! egress guard) or a path inside the server image folder; stored with the
//! active provider.

use crate::config::StorageProvider;
use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::images::{self, basename};
use crate::param::Param;
use crate::pipeline::{UploadRequest, store_media};
use crate::upload_storage::{self as us, UploadKind};
use crate::{MediaKind, UploadOrigin};

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
pub const IMPORT_MIME_TYPES: [&str; 6] = [
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/x-ms-bmp",
];

/// `import_image(...)` → `url`. Errors (all `Invalid` unless noted):
/// "Provide exactly one image URL or server image path", "Image not found",
/// "Image download failed" (fetch failure or egress block),
/// "Unsupported image content type", allow-list `Rejected`, S3 collision →
/// `Upstream("Image upload failed")`, other S3 errors propagate.
pub async fn import_image(env: &MediaEnv, req: &ImportRequest) -> Result<String, MediaError> {
    let config = env.settings.current();
    let provider = us::provider(&config, &req.upload_to_s3)?;
    let one_source = [&req.url, &req.path]
        .iter()
        .filter(|p| p.is_present())
        .count()
        == 1
        && [&req.url, &req.path]
            .iter()
            .all(|p| matches!(p, Param::Nil | Param::Str(_)));
    if !one_source {
        return Err(MediaError::Invalid(
            "Provide exactly one image URL or server image path".to_owned(),
        ));
    }

    if let Param::Str(rel) = &req.path
        && req.path.is_present()
    {
        let source = images::find_image(&config, rel)
            .ok_or_else(|| MediaError::Invalid("Image not found".to_owned()))?;
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        us::validate_filename(&config, &name, UploadKind::Image)?;
        let bytes = std::fs::read(&source).map_err(|e| MediaError::Io(e.kind()))?;
        let mut upload = UploadRequest::new(UploadOrigin::Picker, MediaKind::Image, name, bytes);
        return store(env, &config, provider, req, &mut upload).await;
    }

    let Param::Str(url) = &req.url else {
        return Err(MediaError::Invalid(
            "Provide exactly one image URL or server image path".to_owned(),
        ));
    };
    let fetched = images::fetch_external_image(env, url)
        .await
        .map_err(|_| MediaError::Invalid("Image download failed".to_owned()))?;
    let mime = fetched.content_type.as_deref().unwrap_or_default();
    let mime = mime.split(';').next().unwrap_or_default().to_lowercase();
    if !IMPORT_MIME_TYPES.contains(&mime.as_str()) {
        return Err(MediaError::Invalid(
            "Unsupported image content type".to_owned(),
        ));
    }
    let extension = images::extension_for_content_type(Some(&mime)).unwrap_or(".jpg");
    us::validate_filename(&config, &format!("image{extension}"), UploadKind::Image)?;
    let sanitized = us::sanitize_filename(basename(fetched.url.path()));
    let stem = us::ruby_basename_without_ext(&sanitized);
    let stem = if stem.trim().is_empty() {
        "image"
    } else {
        stem
    };
    let mut upload = UploadRequest::new(
        UploadOrigin::UrlImport,
        MediaKind::Image,
        format!("{stem}{extension}"),
        fetched.bytes,
    );
    // The original's temp file carried the mime-derived extension.
    upload.type_name = format!("import{extension}");
    store(env, &config, provider, req, &mut upload).await
}

/// `store_import`: resize, key and prefix apply to either source.
async fn store(
    env: &MediaEnv,
    config: &crate::config::MediaConfig,
    provider: StorageProvider,
    req: &ImportRequest,
    upload: &mut UploadRequest,
) -> Result<String, MediaError> {
    upload.resize = req.resize;
    if provider == StorageProvider::S3 {
        upload.s3_key.clone_from(&req.s3_key);
        upload.s3_prefix.clone_from(&req.s3_prefix);
    }
    Ok(store_media(env, config, provider, upload.clone())
        .await?
        .url()
        .to_owned())
}
