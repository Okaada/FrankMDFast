//! The single upload pipeline (HP-015, lesson 3.9): every way media enters
//! FrankMD — drop, paste, picker upload, base64/AI images, URL/server-folder
//! import, legacy S3 uploads and videos — ends in [`store_media`]. The
//! security checks therefore cannot drift apart between sources (the
//! original's duplicated paths did, which produced a stored XSS).

use frankmd_core::RelPath;

use crate::config::{MediaConfig, StorageProvider};
use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::param::Param;
use crate::s3::S3Error;
use crate::upload_storage::{self as us, UploadKind};
use crate::{MediaKind, UploadOrigin, images, sniff, video};

/// A request to store media.
#[derive(Debug, Clone)]
pub struct UploadRequest {
    /// Where the bytes came from (logging only; never changes policy).
    pub origin: UploadOrigin,
    /// Image or video (selects the allow-list, the folder and the sniffer).
    pub kind: MediaKind,
    /// Client/derived name: its extension is allow-listed and it names the
    /// stored file/object (always sanitized first).
    pub file_name: String,
    /// Name whose extension sets the S3 `Content-Type` of an image (the
    /// original used its temp file's extension, e.g. the mime-derived one).
    pub type_name: String,
    /// Raw bytes (already size-capped by the caller's reader).
    pub bytes: Vec<u8>,
    /// Resize ratio in `(0, 1]` (images only).
    pub resize: Option<f64>,
    /// User-chosen S3 key (sanitized, made unique, never overwrites).
    pub s3_key: Option<String>,
    /// User-chosen S3 prefix.
    pub s3_prefix: Option<String>,
}

impl UploadRequest {
    /// A request whose content type follows `file_name`.
    pub fn new(
        origin: UploadOrigin,
        kind: MediaKind,
        file_name: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Self {
        let file_name = file_name.into();
        Self {
            origin,
            kind,
            type_name: file_name.clone(),
            file_name,
            bytes,
            resize: None,
            s3_key: None,
            s3_prefix: None,
        }
    }
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

/// Validate (size cap, extension allow-list, content sniffing) and store
/// with the **backend-resolved** `provider` (from
/// [`crate::upload_storage::provider`], never a UI value).
///
/// Errors: `Rejected` for policy failures (messages as the original's, plus
/// [`sniff::rejection_message`]); an S3 conditional-create collision is
/// `Upstream("Image upload failed" | "Video upload failed")`; other S3 errors
/// propagate as [`MediaError::S3`] so recovery can classify them.
pub async fn store_media(
    env: &MediaEnv,
    config: &MediaConfig,
    provider: StorageProvider,
    req: UploadRequest,
) -> Result<StoredMedia, MediaError> {
    validate(env, config, &req)?;
    match provider {
        StorageProvider::Local => store_local(env, config, req),
        StorageProvider::S3 => store_s3(env, config, req).await,
    }
}

/// [`store_media`] for the local provider only, synchronously (callers that
/// already know the destination is the vault, e.g. `save_to_notes_directory`).
/// Runs exactly the same validation.
pub fn store_media_local(
    env: &MediaEnv,
    config: &MediaConfig,
    req: UploadRequest,
) -> Result<StoredMedia, MediaError> {
    validate(env, config, &req)?;
    store_local(env, config, req)
}

/// Validate extension + content without storing; returns the detected kind.
pub fn validate_bytes(
    file_name: &str,
    bytes: &[u8],
    allowed_exts: &[String],
) -> Result<MediaKind, MediaError> {
    let extension = us::ruby_extname(file_name).to_lowercase();
    if !allowed_exts.contains(&extension) {
        return Err(MediaError::Rejected(format!(
            "{} is not an accepted type. Accepted: {}",
            if extension.is_empty() {
                "no extension"
            } else {
                &extension
            },
            allowed_exts.join(", ")
        )));
    }
    let detected = sniff::detect(bytes)
        .ok_or_else(|| MediaError::Rejected(sniff::rejection_message(MediaKind::Image)))?;
    sniff::check(bytes, detected.kind, allowed_exts).map(|d| d.kind)
}

fn upload_kind(kind: MediaKind) -> UploadKind {
    match kind {
        MediaKind::Image => UploadKind::Image,
        MediaKind::Video => UploadKind::Video,
    }
}

fn validate(env: &MediaEnv, config: &MediaConfig, req: &UploadRequest) -> Result<(), MediaError> {
    let size = i64::try_from(req.bytes.len()).unwrap_or(i64::MAX);
    us::enforce_bytes(Some(size), env.limits.max_upload_bytes)?;
    us::validate_filename(config, &req.file_name, upload_kind(req.kind))?;
    let allowed = match req.kind {
        MediaKind::Image => &config.image_upload_extensions,
        MediaKind::Video => &config.video_upload_extensions,
    };
    sniff::check(&req.bytes, req.kind, allowed)?;
    Ok(())
}

/// `save_to_notes_directory` / `MediaService.store_local`: a fresh,
/// collision-resistant name under `<notes>/images|videos`, written no-clobber.
fn store_local(
    env: &MediaEnv,
    config: &MediaConfig,
    req: UploadRequest,
) -> Result<StoredMedia, MediaError> {
    let (dir, dest, bytes) = match req.kind {
        MediaKind::Video => (
            video::SUBDIR,
            us::dest_filename(&env.ctx, Some(&req.file_name)),
            req.bytes,
        ),
        MediaKind::Image => {
            let safe = us::sanitize_filename(&req.file_name);
            match req.resize.filter(|r| *r > 0.0) {
                Some(ratio) => {
                    let dest = us::dest_filename(
                        &env.ctx,
                        Some(&format!("{}.jpg", us::ruby_basename_without_ext(&safe))),
                    );
                    let resized = images::resize_and_compress(
                        &req.bytes,
                        &req.type_name,
                        Some(&dest),
                        &Param::Float(ratio),
                    );
                    ("images", dest, resized.bytes)
                }
                None => (
                    "images",
                    us::dest_filename(&env.ctx, Some(&safe)),
                    req.bytes,
                ),
            }
        }
    };
    let vault = frankmd_fs::Vault::open(&config.notes_root, lock_fallback_dir())?;
    let target = vault.safe_path(&format!("{dir}/{dest}"), false)?;
    vault.create_note(&target, &bytes)?;
    Ok(StoredMedia::Local(target.rel().clone()))
}

/// Private lock fallback for a notes root that cannot hold the compat lock
/// file (read-only mounts); never inside the vault.
fn lock_fallback_dir() -> std::path::PathBuf {
    app_state_dir()
}

/// The app-owned state directory (`$XDG_STATE_HOME/frankmd/media`), or a temp
/// directory when no home is known. Never inside a user folder.
pub(crate) fn app_state_dir() -> std::path::PathBuf {
    frankmd_config::dirs::AppDirs::discover()
        .map_or_else(|| std::env::temp_dir().join("frankmd-state"), |d| d.state)
        .join("media")
}

/// `upload_temp_to_s3` / `MediaService.store_s3`: conditional create
/// (`If-None-Match: *`), never an overwrite.
async fn store_s3(
    env: &MediaEnv,
    config: &MediaConfig,
    req: UploadRequest,
) -> Result<StoredMedia, MediaError> {
    let client = env.s3_client(config)?;
    let (key, bytes, content_type, collision) = match req.kind {
        MediaKind::Image => {
            let (bytes, content_type, filename) = match req.resize {
                Some(ratio) => {
                    let r = images::resize_and_compress(
                        &req.bytes,
                        &req.type_name,
                        Some(&req.file_name),
                        &Param::Float(ratio),
                    );
                    (r.bytes, r.content_type, r.filename)
                }
                None => (
                    req.bytes,
                    images::content_type_for(&req.type_name).to_owned(),
                    req.file_name.clone(),
                ),
            };
            let key = us::s3_key(
                &env.ctx,
                Some(&filename),
                req.s3_key.as_deref(),
                req.s3_prefix.as_deref(),
            );
            (key, bytes, content_type, "Image upload failed")
        }
        MediaKind::Video => {
            let key = us::s3_key(
                &env.ctx,
                Some(&req.file_name),
                None,
                req.s3_prefix.as_deref(),
            );
            let content_type = video::content_type_for_key(&key).to_owned();
            (key, req.bytes, content_type, "Video upload failed")
        }
    };
    match client.put_object(&key, bytes, &content_type).await {
        Ok(()) => {}
        Err(S3Error::Service { code, .. }) if code == "PreconditionFailed" => {
            return Err(MediaError::Upstream(collision.to_owned()));
        }
        // Buckets with ACLs disabled; the object was still written (images only, as in the original).
        Err(S3Error::Service { code, .. })
            if code == "AccessControlListNotSupported" && req.kind == MediaKind::Image => {}
        Err(e) => return Err(MediaError::S3(e)),
    }
    let bucket = config.s3.bucket.as_deref().unwrap_or_default();
    let url = us::s3_url(config, bucket, &us::s3_region(config), &key);
    Ok(StoredMedia::S3 { url, key })
}
