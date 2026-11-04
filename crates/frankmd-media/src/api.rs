//! Endpoint-level facade: what `ImagesController` / `MediaController` did on top
//! of the services — parameter gates, i18n error keys vs. service messages,
//! status classes, and storage recovery around every upload. The desktop UI
//! calls these; status classes keep the original's error taxonomy.

use std::path::PathBuf;

use crate::config::MediaConfig;
use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::images::{self, ServerImage};
use crate::import::ImportRequest;
use crate::param::Param;
use crate::recovery::StorageRecovery;
use crate::search::search_flags;
use crate::upload_storage::Upload;

/// The original HTTP status class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiStatus {
    /// 400.
    BadRequest,
    /// 404.
    NotFound,
    /// 422.
    Unprocessable,
    /// 503.
    ServiceUnavailable,
}

/// Error text: an i18n key (controller `t(...)`) or a service message verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiMessage {
    /// Locale key, e.g. `errors.no_file_upload`, `media_storage.retry_local`.
    Key(&'static str),
    /// Service message, e.g. `".html is not an accepted image type. ..."`.
    Text(String),
}

impl ApiMessage {
    /// Text for assertions: the message, or the key itself.
    pub fn as_str(&self) -> &str {
        match self {
            ApiMessage::Key(k) => k,
            ApiMessage::Text(t) => t,
        }
    }
}

/// A failed endpoint call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    /// Status class.
    pub status: ApiStatus,
    /// Message.
    pub message: ApiMessage,
    /// `media_storage_fallback: true` was reported (recovery switched to local).
    pub media_storage_fallback: bool,
}

/// `GET /images/config`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagesConfigInfo {
    /// Server folder configured.
    pub enabled: bool,
    /// S3 configured.
    pub s3_enabled: bool,
    /// Persisted provider (after recovery).
    pub media_storage_provider: Option<String>,
    /// Pending warning (**not** consumed here).
    pub media_storage_fallback: bool,
    /// Always true (DuckDuckGo).
    pub web_search_enabled: bool,
    /// Google key + CSE id present.
    pub google_enabled: bool,
    /// Always true (DDG site filter).
    pub pinterest_enabled: bool,
    /// Parsed `image_upload_extensions`.
    pub image_extensions: Vec<String>,
    /// Parsed `video_upload_extensions`.
    pub video_extensions: Vec<String>,
}

/// `GET /images`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageIndex {
    /// Page.
    pub images: Vec<ServerImage>,
    /// Total matches.
    pub total: usize,
    /// Effective limit (`to_i.clamp(1, 100)`, default 10).
    pub limit: usize,
    /// Effective offset (`max(to_i, 0)`, default 0).
    pub offset: usize,
}

/// Configuration refresh result (`GET /config`, `GET /`, note pages): the
/// settings' provider plus the **consumed** warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageStatus {
    /// Persisted provider.
    pub media_storage_provider: Option<String>,
    /// Warning shown now (then cleared).
    pub media_storage_fallback: bool,
}

/// `POST /images/upload_base64` parameters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Base64Upload {
    /// Base64 data.
    pub data: String,
    /// MIME type (may be blank).
    pub mime_type: String,
    /// Filename (may be blank).
    pub filename: String,
    /// Legacy flag.
    pub upload_to_s3: Param,
    /// S3 prefix.
    pub s3_prefix: Option<String>,
}

/// The media endpoints.
#[derive(Debug)]
pub struct MediaApi {
    /// Services.
    pub env: MediaEnv,
    /// One-shot fallback warning state.
    pub recovery: StorageRecovery,
}

impl MediaApi {
    /// Build.
    pub fn new(env: MediaEnv) -> Self {
        Self {
            env,
            recovery: StorageRecovery::new(),
        }
    }

    /// `GET /config` / page load: run recovery and **consume** the warning.
    pub fn storage_status(&self) -> StorageStatus {
        let media_storage_fallback = self
            .recovery
            .recover_media_storage(self.env.settings.as_ref(), true);
        StorageStatus {
            media_storage_provider: self.config().media_storage_provider,
            media_storage_fallback,
        }
    }

    /// `GET /images/config` (recovery runs, warning preserved).
    pub fn images_config(&self) -> ImagesConfigInfo {
        let media_storage_fallback = self
            .recovery
            .recover_media_storage(self.env.settings.as_ref(), false);
        let config = self.config();
        let flags = search_flags(&config);
        ImagesConfigInfo {
            enabled: images::enabled(&config),
            s3_enabled: images::s3_enabled(&config),
            media_storage_provider: config.media_storage_provider.clone(),
            media_storage_fallback,
            web_search_enabled: flags.web_search_enabled,
            google_enabled: flags.google_enabled,
            pinterest_enabled: flags.pinterest_enabled,
            image_extensions: config.image_upload_extensions,
            video_extensions: config.video_upload_extensions,
        }
    }

    /// `GET /images` (404 `errors.images_not_configured` when disabled).
    pub fn index(
        &self,
        search: Option<&str>,
        limit: &Param,
        offset: &Param,
    ) -> Result<ImageIndex, ApiError> {
        let config = self.images_enabled()?;
        let limit = if limit.is_present() {
            ruby_to_i(limit).clamp(1, 100)
        } else {
            10
        };
        let offset = if offset.is_present() {
            ruby_to_i(offset).max(0)
        } else {
            0
        };
        let images = images::list(&config, search, &Param::Int(limit), &Param::Int(offset));
        let total = images::count(&config, search);
        Ok(ImageIndex {
            images,
            total,
            limit: limit as usize,
            offset: offset as usize,
        })
    }

    /// `GET /images/preview/*path` → file to send inline (404 when disabled or not found).
    pub fn preview(&self, rel: &str) -> Result<PathBuf, ApiError> {
        let config = self.images_enabled()?;
        images::find_image(&config, rel).ok_or_else(|| {
            ApiError::new(
                ApiStatus::NotFound,
                ApiMessage::Text("Not Found".to_owned()),
            )
        })
    }

    /// `DELETE /images/file/*path` (422 "Image not found or could not be deleted").
    pub fn destroy(&self, rel: &str) -> Result<(), ApiError> {
        let config = self.images_enabled()?;
        if images::delete(&config, rel) {
            Ok(())
        } else {
            Err(ApiError::text(
                ApiStatus::Unprocessable,
                "Image not found or could not be deleted",
            ))
        }
    }

    /// `POST /images/upload` (no file → 422 `errors.no_file_upload`).
    pub async fn upload(
        &self,
        file: Option<Upload>,
        resize: &Param,
        upload_to_s3: &Param,
        s3_key: Option<&str>,
    ) -> Result<String, ApiError> {
        self.before_upload()?;
        let file =
            file.ok_or_else(|| ApiError::key(ApiStatus::Unprocessable, "errors.no_file_upload"))?;
        let result = images::upload_file(
            &self.env,
            Some(file),
            images::parse_resize_ratio(resize),
            upload_to_s3,
            s3_key,
        )
        .await;
        result.map_err(|e| self.upload_error(e, "errors.upload_failed"))
    }

    /// `POST /images/upload_base64` (blank data → 400 `errors.no_image_data`).
    pub async fn upload_base64(&self, req: &Base64Upload) -> Result<String, ApiError> {
        self.before_upload()?;
        if req.data.trim().is_empty() {
            return Err(ApiError::key(ApiStatus::BadRequest, "errors.no_image_data"));
        }
        let result = images::upload_base64_data(
            &self.env,
            &req.data,
            Some(&req.mime_type),
            Some(&req.filename),
            &req.upload_to_s3,
            req.s3_prefix.as_deref(),
        )
        .await;
        result.map_err(|e| self.upload_error(e, "errors.upload_failed"))
    }

    /// `POST /images/import`.
    pub async fn import(&self, req: &ImportRequest) -> Result<String, ApiError> {
        self.before_upload()?;
        match crate::import::import_image(&self.env, req).await {
            Ok(url) => Ok(url),
            // `store_import` turned a nil S3 result into this message.
            Err(MediaError::Upstream(m)) => Err(ApiError::text(ApiStatus::Unprocessable, m)),
            Err(e) => Err(self.upload_error(e, "errors.upload_failed")),
        }
    }

    /// `POST /images/upload_to_s3` (S3 not configured → 422 `errors.s3_not_configured`;
    /// images_path required → 404).
    pub async fn upload_to_s3(
        &self,
        rel: &str,
        resize: &Param,
        s3_key: Option<&str>,
    ) -> Result<String, ApiError> {
        self.before_upload()?;
        let config = self.images_enabled()?;
        if !images::s3_enabled(&config) {
            return Err(ApiError::key(
                ApiStatus::Unprocessable,
                "errors.s3_not_configured",
            ));
        }
        let result =
            images::upload_to_s3(&self.env, rel, images::parse_resize_ratio(resize), s3_key).await;
        result.map_err(|e| self.legacy_s3_error(e))
    }

    /// `POST /images/upload_external_to_s3` (blank url → 400 `errors.url_required`).
    pub async fn upload_external_to_s3(
        &self,
        url: &str,
        resize: &Param,
        s3_prefix: Option<&str>,
    ) -> Result<String, ApiError> {
        self.before_upload()?;
        let config = self.images_enabled()?;
        if !images::s3_enabled(&config) {
            return Err(ApiError::key(
                ApiStatus::Unprocessable,
                "errors.s3_not_configured",
            ));
        }
        let url = url.trim();
        if url.is_empty() {
            return Err(ApiError::key(ApiStatus::BadRequest, "errors.url_required"));
        }
        let result = images::download_and_upload_to_s3(
            &self.env,
            url,
            images::parse_resize_ratio(resize),
            s3_prefix,
        )
        .await;
        result.map_err(|e| self.legacy_s3_error(e))
    }

    /// `POST /media/upload` (videos).
    pub async fn media_upload(
        &self,
        file: Option<Upload>,
        upload_to_s3: &Param,
        s3_prefix: Option<&str>,
    ) -> Result<String, ApiError> {
        self.before_upload()?;
        let file =
            file.ok_or_else(|| ApiError::key(ApiStatus::Unprocessable, "errors.no_file_upload"))?;
        let result =
            crate::video::save_upload(&self.env, Some(file), upload_to_s3, s3_prefix).await;
        result.map_err(|e| self.upload_error(e, "errors.upload_failed"))
    }

    /// Fresh settings snapshot.
    fn config(&self) -> MediaConfig {
        self.env.settings.current()
    }

    /// `require_images_enabled`.
    fn images_enabled(&self) -> Result<MediaConfig, ApiError> {
        let config = self.config();
        if images::enabled(&config) {
            Ok(config)
        } else {
            Err(ApiError::key(
                ApiStatus::NotFound,
                "errors.images_not_configured",
            ))
        }
    }

    /// `recover_missing_upload_storage` (before every upload endpoint).
    fn before_upload(&self) -> Result<(), ApiError> {
        self.recovery
            .recover_missing_upload_storage(self.env.settings.as_ref())
            .map_err(|_| ApiError::fallback())
    }

    /// The upload endpoints' error rendering: service rejections verbatim,
    /// recovery for definitive S3 configuration errors, else the generic key
    /// (provider details are never shown).
    fn upload_error(&self, error: MediaError, generic: &'static str) -> ApiError {
        match error {
            MediaError::Rejected(m) | MediaError::Invalid(m) => {
                ApiError::text(ApiStatus::Unprocessable, m)
            }
            MediaError::StorageFallback => ApiError::fallback(),
            other
                if self
                    .recovery
                    .recover_s3_upload(self.env.settings.as_ref(), &other) =>
            {
                ApiError::fallback()
            }
            _ => ApiError::key(ApiStatus::Unprocessable, generic),
        }
    }

    /// The legacy S3 endpoints: a nil service result is `errors.failed_to_upload`;
    /// rejections were rendered as `"<class>: <message>"`.
    fn legacy_s3_error(&self, error: MediaError) -> ApiError {
        match error {
            MediaError::Rejected(m) => ApiError::text(
                ApiStatus::Unprocessable,
                format!("UploadStorage::RejectedError: {m}"),
            ),
            MediaError::StorageFallback => ApiError::fallback(),
            other
                if self
                    .recovery
                    .recover_s3_upload(self.env.settings.as_ref(), &other) =>
            {
                ApiError::fallback()
            }
            _ => ApiError::key(ApiStatus::Unprocessable, "errors.failed_to_upload"),
        }
    }
}

impl ApiError {
    fn new(status: ApiStatus, message: ApiMessage) -> Self {
        Self {
            status,
            message,
            media_storage_fallback: false,
        }
    }

    fn key(status: ApiStatus, key: &'static str) -> Self {
        Self::new(status, ApiMessage::Key(key))
    }

    fn text(status: ApiStatus, text: impl Into<String>) -> Self {
        Self::new(status, ApiMessage::Text(text.into()))
    }

    /// `render_storage_recovery`: 422 + `media_storage_fallback: true`.
    fn fallback() -> Self {
        Self {
            media_storage_fallback: true,
            ..Self::key(ApiStatus::Unprocessable, "media_storage.retry_local")
        }
    }
}

/// Ruby `to_i` on a request parameter (`"12abc"` → 12, `"abc"` → 0).
fn ruby_to_i(value: &Param) -> i64 {
    match value {
        Param::Int(i) => *i,
        Param::Float(f) => *f as i64,
        Param::Str(s) => {
            let t = s.trim_start();
            let (sign, digits) = match t.strip_prefix('-') {
                Some(rest) => (-1, rest),
                None => (1, t.strip_prefix('+').unwrap_or(t)),
            };
            let n: String = digits.chars().take_while(char::is_ascii_digit).collect();
            n.parse::<i64>().map_or(0, |v| sign * v)
        }
        _ => 0,
    }
}
