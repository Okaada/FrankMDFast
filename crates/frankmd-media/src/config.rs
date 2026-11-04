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
        use frankmd_config::Key;
        let string = |key: Key| present_string(config.value(key));
        let home = config.env().raw("HOME").map(PathBuf::from);
        Self {
            notes_root: notes_root.into(),
            images_path: string(Key::ImagesPath).map(|p| expand_path(&p, home.as_deref())),
            // The provider is validated (not blank-filtered) by `upload_storage::provider`.
            media_storage_provider: match config.value(Key::MediaStorageProvider) {
                Some(frankmd_config::Value::Str(s)) => Some(s),
                _ => None,
            },
            image_upload_extensions: config.upload_extensions(Key::ImageUploadExtensions),
            video_upload_extensions: config.upload_extensions(Key::VideoUploadExtensions),
            s3: S3Settings {
                access_key_id: string(Key::AwsAccessKeyId),
                secret_access_key: string(Key::AwsSecretAccessKey),
                bucket: string(Key::AwsS3Bucket),
                region: string(Key::AwsRegion),
                endpoint: string(Key::AwsS3Endpoint),
                force_path_style: matches!(config.value(Key::AwsS3ForcePathStyle), Some(frankmd_config::Value::Bool(true))),
            },
            youtube_api_key: string(Key::YoutubeApiKey),
            google_api_key: string(Key::GoogleApiKey),
            google_cse_id: string(Key::GoogleCseId),
        }
    }
}

/// A present (non-blank) string value.
fn present_string(value: Option<frankmd_config::Value>) -> Option<String> {
    match value {
        Some(frankmd_config::Value::Str(s)) if !s.trim().is_empty() => Some(s),
        _ => None,
    }
}

/// Ruby `Pathname#expand_path`: `~` / `~/x` from `HOME`, relative paths
/// against the process working directory, then lexically cleaned.
fn expand_path(raw: &str, home: Option<&std::path::Path>) -> PathBuf {
    let path = match (raw.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home.to_path_buf(),
        (Some(rest), Some(home)) if rest.starts_with('/') => home.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(raw),
    };
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().map_or(path.clone(), |cwd| cwd.join(&path))
    };
    let mut clean = PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other),
        }
    }
    clean
}
