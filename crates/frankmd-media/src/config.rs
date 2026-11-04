//! The media crate's view of the configuration (read once from
//! `frankmd_config::ResolvedConfig`; plain data so tests can build it directly).

use std::path::PathBuf;

/// **[COMPAT]** `media_storage_provider` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageProvider {
    /// `local` (default): `<notes>/images/`, `<notes>/videos/`.
    Local,
    /// `s3`.
    S3,
}

/// S3 settings (`aws_*` keys). `force_path_style` defaults to false.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct S3Settings {
    /// `aws_access_key_id`.
    pub access_key_id: Option<String>,
    /// `aws_secret_access_key`.
    pub secret_access_key: Option<String>,
    /// `aws_s3_bucket`.
    pub bucket: Option<String>,
    /// `aws_region` (nil → `DEFAULT_S3_REGION` "us-east-1").
    pub region: Option<String>,
    /// `aws_s3_endpoint` (S3-compatible services; must be http(s) with a host, no userinfo).
    pub endpoint: Option<String>,
    /// `aws_s3_force_path_style`.
    pub force_path_style: bool,
}

/// Everything media code reads from config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaConfig {
    /// Notes root (`NOTES_PATH`); local uploads go to `<root>/images|videos`.
    pub notes_root: PathBuf,
    /// `images_path`: the user's "server" image folder (already `expand_path`ed).
    pub images_path: Option<PathBuf>,
    /// Raw `media_storage_provider` value (validated by `upload_storage::provider`).
    pub media_storage_provider: Option<String>,
    /// `image_upload_extensions`, parsed (`.png` form, lowercase).
    pub image_upload_extensions: Vec<String>,
    /// `video_upload_extensions`, parsed.
    pub video_upload_extensions: Vec<String>,
    /// `aws_*`.
    pub s3: S3Settings,
    /// `youtube_api_key`.
    pub youtube_api_key: Option<String>,
    /// `google_api_key`.
    pub google_api_key: Option<String>,
    /// `google_cse_id`.
    pub google_cse_id: Option<String>,
}

/// **[COMPAT]** default `image_upload_extensions`.
pub const DEFAULT_IMAGE_EXTENSIONS: [&str; 6] = [".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"];
/// **[COMPAT]** default `video_upload_extensions`.
pub const DEFAULT_VIDEO_EXTENSIONS: [&str; 7] = [".mp4", ".webm", ".mkv", ".mov", ".avi", ".m4v", ".ogv"];

impl MediaConfig {
    /// Defaults for a notes root: local provider, default extensions, nothing else set.
    pub fn new(notes_root: impl Into<PathBuf>) -> Self {
        Self {
            notes_root: notes_root.into(),
            images_path: None,
            media_storage_provider: Some("local".to_owned()),
            image_upload_extensions: DEFAULT_IMAGE_EXTENSIONS.iter().map(|s| (*s).to_owned()).collect(),
            video_upload_extensions: DEFAULT_VIDEO_EXTENSIONS.iter().map(|s| (*s).to_owned()).collect(),
            s3: S3Settings::default(),
            youtube_api_key: None,
            google_api_key: None,
            google_cse_id: None,
        }
    }

    /// Read from the resolved config (`.fed > ENV > default`).
    pub fn from_resolved(config: &frankmd_config::ResolvedConfig, notes_root: impl Into<PathBuf>) -> Self {
        todo!()
    }
}
