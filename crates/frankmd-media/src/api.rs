//! Endpoint-level facade: what `ImagesController` / `MediaController` did on top
//! of the services — parameter gates, i18n error keys vs. service messages,
//! status classes, and storage recovery around every upload. The desktop UI
//! calls these; status classes keep the original's error taxonomy.

use std::path::PathBuf;

use crate::env::MediaEnv;
use crate::images::ServerImage;
use crate::import::ImportRequest;
use crate::param::Param;
use crate::recovery::StorageRecovery;
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
        Self { env, recovery: StorageRecovery::new() }
    }

    /// `GET /config` / page load: run recovery and **consume** the warning.
    pub fn storage_status(&self) -> StorageStatus {
        todo!()
    }

    /// `GET /images/config` (recovery runs, warning preserved).
    pub fn images_config(&self) -> ImagesConfigInfo {
        todo!()
    }

    /// `GET /images` (404 `errors.images_not_configured` when disabled).
    pub fn index(&self, search: Option<&str>, limit: &Param, offset: &Param) -> Result<ImageIndex, ApiError> {
        todo!()
    }

    /// `GET /images/preview/*path` → file to send inline (404 when disabled or not found).
    pub fn preview(&self, rel: &str) -> Result<PathBuf, ApiError> {
        todo!()
    }

    /// `DELETE /images/file/*path` (422 "Image not found or could not be deleted").
    pub fn destroy(&self, rel: &str) -> Result<(), ApiError> {
        todo!()
    }

    /// `POST /images/upload` (no file → 422 `errors.no_file_upload`).
    pub async fn upload(
        &self,
        file: Option<Upload>,
        resize: &Param,
        upload_to_s3: &Param,
        s3_key: Option<&str>,
    ) -> Result<String, ApiError> {
        todo!()
    }

    /// `POST /images/upload_base64` (blank data → 400 `errors.no_image_data`).
    pub async fn upload_base64(&self, req: &Base64Upload) -> Result<String, ApiError> {
        todo!()
    }

    /// `POST /images/import`.
    pub async fn import(&self, req: &ImportRequest) -> Result<String, ApiError> {
        todo!()
    }

    /// `POST /images/upload_to_s3` (S3 not configured → 422 `errors.s3_not_configured`;
    /// images_path required → 404).
    pub async fn upload_to_s3(&self, rel: &str, resize: &Param, s3_key: Option<&str>) -> Result<String, ApiError> {
        todo!()
    }

    /// `POST /images/upload_external_to_s3` (blank url → 400 `errors.url_required`).
    pub async fn upload_external_to_s3(
        &self,
        url: &str,
        resize: &Param,
        s3_prefix: Option<&str>,
    ) -> Result<String, ApiError> {
        todo!()
    }

    /// `POST /media/upload` (videos).
    pub async fn media_upload(
        &self,
        file: Option<Upload>,
        upload_to_s3: &Param,
        s3_prefix: Option<&str>,
    ) -> Result<String, ApiError> {
        todo!()
    }
}
