//! **[COMPAT]** ports of `app/services/media_library_service.rb`,
//! `app/services/image_library_service.rb` and the data side of
//! `app/controllers/library_controller.rb` (the turbo-stream HTML of
//! `GET /library/picker` is Rails plumbing; its data contract is [`PickerPage`]).
//! Golden: `media.json` `MediaLibraryService#{list,delete}`, `ImageLibraryService#list`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::MediaKind;
use crate::config::MediaConfig;
use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::param::Param;
use crate::s3_library::S3MediaLibrary;
use crate::usage::MediaUsage;

/// `MEDIA_DIRECTORIES` (root → kind).
pub const MEDIA_DIRECTORIES: [(&str, MediaKind); 2] = [("images", MediaKind::Image), ("videos", MediaKind::Video)];

/// One library item (local or S3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaItem {
    /// Basename.
    pub name: String,
    /// Notes-root-relative path (`images/a b.png`) or S3 key.
    pub path: String,
    /// Bytes.
    pub size: u64,
    /// `stat.mtime.iso8601` (local offset, no fraction) / S3 `LastModified`.
    pub mtime: String,
    /// `/notes/<percent-encoded segments>` (local) or the object URL (S3).
    pub preview_url: String,
    /// `image` / `video`.
    pub kind: MediaKind,
    /// S3 only: object URL (`url`) and `provider: "s3"`.
    pub s3_url: Option<String>,
}

/// `{images: [...], videos: [...]}`, each sorted by mtime desc then path (case-insensitive).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryListing {
    /// Images.
    pub images: Vec<MediaItem>,
    /// Videos.
    pub videos: Vec<MediaItem>,
}

/// `MediaLibraryService`: managed media under `<notes>/images` and `<notes>/videos`.
/// Listing skips dotfiles, hidden dirs and every symlink; deletion is fully
/// contained (lstat walk, no symlink anywhere) and goes through frankmd-fs.
#[derive(Debug)]
pub struct MediaLibrary<'a> {
    vault: &'a frankmd_fs::Vault,
    image_extensions: Vec<String>,
    video_extensions: Vec<String>,
}

impl<'a> MediaLibrary<'a> {
    /// `MediaLibraryService.new(base_path: vault.root, config:)`.
    pub fn new(vault: &'a frankmd_fs::Vault, image_extensions: Vec<String>, video_extensions: Vec<String>) -> Self {
        Self { vault, image_extensions, video_extensions }
    }

    /// Extensions from config (`upload_extensions`).
    pub fn from_config(vault: &'a frankmd_fs::Vault, config: &MediaConfig) -> Self {
        Self::new(vault, config.image_upload_extensions.clone(), config.video_upload_extensions.clone())
    }

    /// `list`. Vanished/unreadable entries are skipped, never fatal.
    pub fn list(&self) -> LibraryListing {
        todo!("backend.md §7.8")
    }

    /// The containment check behind `delete` (`safe_media_file`): absolute path
    /// of a deletable file, or `None`.
    pub fn safe_media_file(&self, path: &str) -> Option<PathBuf> {
        todo!()
    }

    /// `delete(path)`: `Ok(false)` for missing/invalid/unsupported/symlinked
    /// paths (no probing outside `images/`/`videos/`); filesystem errors such as
    /// EACCES are returned as [`MediaError::Io`] and the file stays.
    pub fn delete(&self, path: &str) -> Result<bool, MediaError> {
        todo!()
    }
}

/// Picker collections (`ImageLibraryService`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerCollection {
    /// `<notes>/images` (default).
    Managed,
    /// The user's `images_path` server folder.
    Server,
}

/// One picker image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerImage {
    /// Basename (HTML-escaped by the view).
    pub name: String,
    /// Relative path.
    pub path: String,
    /// What gets inserted / imported: managed → `path`; server → `/images/preview/<enc>`.
    pub url: String,
    /// Thumbnail URL: managed → `/notes/images/<enc>`; server → same as `url`.
    pub preview_url: String,
    /// Managed collection: the full library item (`size`, `mtime`, `type`); server: `None`.
    pub item: Option<MediaItem>,
}

/// `ImageLibraryService#list` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerPage {
    /// Page items.
    pub images: Vec<PickerImage>,
    /// Collection served.
    pub collection: PickerCollection,
    /// Total matches.
    pub total: usize,
    /// Effective limit (`to_i.clamp(1, 100)`).
    pub limit: usize,
    /// Effective offset (`max(to_i, 0)`).
    pub offset: usize,
    /// `images_path` is an existing directory.
    pub server_available: bool,
}

/// `GET /library/picker` params (raw, Ruby-coerced: `limit.presence || 10`, `offset.to_i`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PickerQuery {
    /// `collection` (`nil`/blank → `managed`; anything but managed|server → error).
    pub collection: Param,
    /// `search` (case-insensitive substring of the name; blank = all).
    pub search: Param,
    /// `limit`.
    pub limit: Param,
    /// `offset`.
    pub offset: Param,
}

/// **[COMPAT]** `ImageLibraryService#list` (+ `LibraryController#picker` param
/// defaults). Unknown collection → [`LibraryError::BadRequest`]; filesystem
/// failure → [`LibraryError::LoadFailed`] (no path in the error).
pub fn picker_list(vault: &frankmd_fs::Vault, config: &MediaConfig, query: &PickerQuery) -> Result<PickerPage, LibraryError> {
    todo!()
}

/// `LibraryController` errors (HTTP status in the original → variant). User
/// text is the i18n key; nothing carries filesystem paths or provider detail.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    /// 400 (`ArgumentError`, e.g. "Unknown image collection").
    #[error("bad request: {0}")]
    BadRequest(String),
    /// 404 `library.media_not_found`.
    #[error("library.media_not_found")]
    NotFound,
    /// 422 `library.delete_failed` (EACCES & co).
    #[error("library.delete_failed")]
    DeleteFailed,
    /// 422 `library.rename_failed` (local provider, invalid name, collision).
    #[error("library.rename_failed")]
    RenameFailed,
    /// 422 picker load failure (`head :unprocessable_entity`).
    #[error("library load failed")]
    LoadFailed,
    /// 503 `library.usage_scan_failed` (ScanError / SystemCallError); never a partial map.
    #[error("library.usage_scan_failed")]
    UsageScanFailed,
    /// Anything else from the media layer (S3 config rejected, ...).
    #[error(transparent)]
    Media(#[from] MediaError),
}

impl LibraryError {
    /// The i18n key shown to the user, when the original rendered one.
    pub fn i18n_key(&self) -> Option<&'static str> {
        match self {
            LibraryError::NotFound => Some("library.media_not_found"),
            LibraryError::DeleteFailed => Some("library.delete_failed"),
            LibraryError::RenameFailed => Some("library.rename_failed"),
            LibraryError::UsageScanFailed => Some("library.usage_scan_failed"),
            _ => None,
        }
    }
}

/// `GET /library/usage` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageResponse {
    /// No `path` param: `{usage_counts: {media → distinct note count}}`.
    Counts(BTreeMap<String, usize>),
    /// `?path=`: `{usage_notes: [sorted note paths]}` (empty if unused).
    Notes(Vec<String>),
}

/// Renamed item (`PATCH /library/file/*key {name}` → `{name, path, url}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamedItem {
    /// New basename.
    pub name: String,
    /// New key.
    pub path: String,
    /// New object URL.
    pub url: String,
}

/// The provider-routed Library (`LibraryController#media_library`): local
/// [`MediaLibrary`] or [`S3MediaLibrary`] when the active provider is `s3`.
#[derive(Debug)]
pub struct Library<'a> {
    vault: &'a frankmd_fs::Vault,
    env: &'a MediaEnv,
}

impl<'a> Library<'a> {
    /// Bind to a vault and the injected media services (config read fresh per call).
    pub fn new(vault: &'a frankmd_fs::Vault, env: &'a MediaEnv) -> Self {
        Self { vault, env }
    }

    /// `GET /library`.
    pub async fn list(&self) -> Result<LibraryListing, LibraryError> {
        todo!()
    }

    /// `DELETE /library/file/*path` (`Ok` = 204).
    pub async fn delete(&self, path: &str) -> Result<(), LibraryError> {
        todo!()
    }

    /// `PATCH /library/file/*path {name}`: S3 only; the local provider refuses.
    pub async fn rename(&self, path: &str, new_name: &str) -> Result<RenamedItem, LibraryError> {
        todo!()
    }

    /// `GET /library/usage[?path=]`: builds the index exactly once per call.
    pub fn usage(&self, usage: &MediaUsage, path: Option<&str>) -> Result<UsageResponse, LibraryError> {
        todo!()
    }
}

/// Helper for S3 routing (exposed for the app's own wiring).
pub fn s3_library(env: &MediaEnv, config: &MediaConfig) -> Result<S3MediaLibrary, LibraryError> {
    todo!()
}
