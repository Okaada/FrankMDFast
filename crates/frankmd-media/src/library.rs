//! **[COMPAT]** ports of `app/services/media_library_service.rb`,
//! `app/services/image_library_service.rb` and the data side of
//! `app/controllers/library_controller.rb` (the turbo-stream HTML of
//! `GET /library/picker` is Rails plumbing; its data contract is [`PickerPage`]).
//! Golden: `media.json` `MediaLibraryService#{list,delete}`, `ImageLibraryService#list`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::MediaKind;
use crate::config::MediaConfig;
use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::param::Param;
use crate::recovery::StorageRecovery;
use crate::s3_library::S3MediaLibrary;
use crate::server_folder::{extname, iso8601_local, url_encode_segments};
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
        self.try_list().unwrap_or_default()
    }

    /// `list`, reporting a directory that cannot be read (Ruby let the
    /// `SystemCallError` of `children` escape; per-file `stat` failures are skipped).
    pub fn try_list(&self) -> Result<LibraryListing, MediaError> {
        Ok(LibraryListing {
            images: self.list_directory("images", MediaKind::Image)?,
            videos: self.list_directory("videos", MediaKind::Video)?,
        })
    }

    fn extensions(&self, kind: MediaKind) -> &[String] {
        match kind {
            MediaKind::Image => &self.image_extensions,
            MediaKind::Video => &self.video_extensions,
        }
    }

    fn base(&self) -> &Path {
        self.vault.root()
    }

    /// `list_directory`: the managed root (skipped when missing or a symlink),
    /// walked without following any symlink.
    fn list_directory(&self, directory: &str, kind: MediaKind) -> Result<Vec<MediaItem>, MediaError> {
        let root = self.base().join(directory);
        let is_link = std::fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink());
        if is_link || !root.is_dir() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        media_files(&root, &mut files).map_err(|e| MediaError::Io(e.kind()))?;
        let extensions = self.extensions(kind);
        let mut items: Vec<(f64, MediaItem)> = files
            .into_iter()
            .filter_map(|file| {
                let name = file.file_name()?.to_str()?.to_owned();
                if !extensions.contains(&extname(&name).to_lowercase()) {
                    return None;
                }
                let rel = file.strip_prefix(self.base()).ok()?.to_str()?.to_owned();
                if frankmd_fs::path_safety::contain(self.base(), Some(&rel)).as_deref() != Some(file.as_path()) {
                    return None;
                }
                // Vanished/unreadable entries are skipped (`rescue SystemCallError`).
                let meta = std::fs::metadata(&file).ok()?;
                let modified = meta.modified().ok()?;
                let sort_mtime = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or_else(|e| -e.duration().as_secs_f64(), |d| d.as_secs_f64());
                let item = MediaItem {
                    name,
                    preview_url: format!("/notes/{}", url_encode_segments(&rel)),
                    path: rel,
                    size: meta.len(),
                    mtime: iso8601_local(modified),
                    kind,
                    s3_url: None,
                };
                Some((sort_mtime, item))
            })
            .collect();
        items.sort_by(|(ma, a), (mb, b)| {
            mb.total_cmp(ma).then_with(|| a.path.to_lowercase().cmp(&b.path.to_lowercase()))
        });
        Ok(items.into_iter().map(|(_, item)| item).collect())
    }

    /// The containment check behind `delete` (`safe_media_file`): absolute path
    /// of a deletable file, or `None`.
    pub fn safe_media_file(&self, path: &str) -> Option<PathBuf> {
        let segments: Vec<&str> = path.split('/').collect();
        let bad = |seg: &&str| seg.trim().is_empty() || *seg == "." || *seg == ".." || seg.contains('\\') || seg.contains('\0');
        if segments.len() < 2 || segments.iter().any(bad) {
            return None;
        }
        let directory = segments[0];
        let kind = MEDIA_DIRECTORIES.iter().find(|(d, _)| *d == directory).map(|(_, k)| *k)?;
        let last = segments.last()?;
        if !self.extensions(kind).contains(&extname(last).to_lowercase()) {
            return None;
        }
        let rel = segments.join("/");
        let base = self.base();
        let file = frankmd_fs::path_safety::contain(base, Some(&rel))?;
        if file != base.join(&rel) {
            return None;
        }
        let managed_root = base.join(directory);
        if frankmd_fs::path_safety::contain(&managed_root, Some(&segments[1..].join("/"))).as_deref() != Some(file.as_path())
        {
            return None;
        }
        // lstat every component: no symlink anywhere, directories then a regular file.
        let mut current = base.to_path_buf();
        for (index, segment) in segments.iter().enumerate() {
            current.push(segment);
            let meta = std::fs::symlink_metadata(&current).ok()?;
            let file_type = meta.file_type();
            if file_type.is_symlink() {
                return None;
            }
            let leaf = index == segments.len() - 1;
            if (!leaf && !file_type.is_dir()) || (leaf && !file_type.is_file()) {
                return None;
            }
        }
        Some(file)
    }

    /// `delete(path)`: `Ok(false)` for missing/invalid/unsupported/symlinked
    /// paths (no probing outside `images/`/`videos/`); filesystem errors such as
    /// EACCES are returned as [`MediaError::Io`] and the file stays.
    pub fn delete(&self, path: &str) -> Result<bool, MediaError> {
        if self.safe_media_file(path).is_none() {
            return Ok(false);
        }
        let contained = match self.vault.safe_path(path, true) {
            Ok(contained) => contained,
            Err(_) => return Ok(false),
        };
        match self.vault.delete_note(&contained, None) {
            Ok(()) => Ok(true),
            Err(e) if e.is_enoent() || matches!(e, frankmd_fs::FsError::NotFound(_)) => Ok(false),
            Err(e) if e.raw_os_error() == Some(rustix::io::Errno::NOTDIR.raw_os_error()) => Ok(false),
            Err(e) if e.is_permission_denied() => Err(MediaError::Io(std::io::ErrorKind::PermissionDenied)),
            Err(e) => Err(MediaError::Fs(e)),
        }
    }
}

/// `media_files`: recursive, skipping symlinks and dot entries; regular files only.
/// A directory that cannot be read aborts the listing (as in the original).
fn media_files(directory: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(directory)?.filter_map(Result::ok).collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            media_files(&path, out)?;
        } else if file_type.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// Ruby `String#to_i` / `Integer#to_i` / `Float#to_i`; `None` where Ruby has no `to_i`.
fn ruby_to_i(value: &Param) -> Option<i64> {
    match value {
        Param::Nil => Some(0),
        Param::Int(i) => Some(*i),
        Param::Float(f) if f.is_finite() => Some(f.trunc() as i64),
        Param::Str(s) => Some(string_to_i(s)),
        _ => None,
    }
}

/// `String#to_i`: leading whitespace, optional sign, digits with single `_`
/// separators; stops at the first other char (`"12abc"` → 12, `"abc"` → 0).
fn string_to_i(s: &str) -> i64 {
    let s = s.trim_start();
    let (negative, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let mut n: i64 = 0;
    let mut previous_underscore = false;
    for (i, c) in rest.chars().enumerate() {
        match c {
            '0'..='9' => {
                n = n.saturating_mul(10).saturating_add(i64::from(c as u8 - b'0'));
                previous_underscore = false;
            }
            '_' if i > 0 && !previous_underscore => previous_underscore = true,
            _ => break,
        }
    }
    if negative { -n } else { n }
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
    let collection = if query.collection.is_present() { query.collection.ruby_to_s() } else { "managed".to_owned() };
    let collection = match collection.as_str() {
        "managed" => PickerCollection::Managed,
        "server" => PickerCollection::Server,
        _ => return Err(LibraryError::BadRequest("Unknown image collection".to_owned())),
    };
    let limit_param = if query.limit.is_present() { query.limit.clone() } else { Param::Int(10) };
    let limit = ruby_to_i(&limit_param).unwrap_or(10).clamp(1, 100) as usize;
    let offset = usize::try_from(ruby_to_i(&query.offset).unwrap_or(0).max(0)).unwrap_or(usize::MAX);
    let search = query.search.ruby_to_s();
    let search_present = !search.trim().is_empty();

    let (images, total) = match collection {
        PickerCollection::Managed => {
            let library = MediaLibrary::from_config(vault, config);
            let mut all = library.try_list().map_err(|_| LibraryError::LoadFailed)?.images;
            if search_present {
                let needle = search.to_lowercase();
                all.retain(|image| image.name.to_lowercase().contains(&needle));
            }
            let total = all.len();
            let images = all
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|item| PickerImage {
                    name: item.name.clone(),
                    path: item.path.clone(),
                    url: item.path.clone(),
                    preview_url: item.preview_url.clone(),
                    item: Some(item),
                })
                .collect();
            (images, total)
        }
        PickerCollection::Server => {
            let search = search_present.then_some(search.as_str());
            let limit_param = Param::Int(i64::try_from(limit).unwrap_or(100));
            let offset_param = Param::Int(i64::try_from(offset).unwrap_or(i64::MAX));
            let images = crate::server_folder::list(config, search, &limit_param, &offset_param)
                .into_iter()
                .map(|image| {
                    let url = format!("/images/preview/{}", url_encode_segments(&image.path));
                    PickerImage { name: image.name, path: image.path, preview_url: url.clone(), url, item: None }
                })
                .collect();
            (images, crate::server_folder::count(config, search))
        }
    };
    let server_available = crate::server_folder::enabled(config) && config.images_path.as_ref().is_some_and(|p| p.is_dir());
    Ok(PickerPage { images, collection, total, limit, offset, server_available })
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
    recovery: Option<&'a StorageRecovery>,
}

impl<'a> Library<'a> {
    /// Bind to a vault and the injected media services (config read fresh per call).
    pub fn new(vault: &'a frankmd_fs::Vault, env: &'a MediaEnv) -> Self {
        Self { vault, env, recovery: None }
    }

    /// Report storage recovery (`media_storage_fallback`) through the app's
    /// one-shot flag instead of a throwaway one.
    pub fn with_recovery(mut self, recovery: &'a StorageRecovery) -> Self {
        self.recovery = Some(recovery);
        self
    }

    /// `media_library`: run recovery, then route by the persisted provider.
    fn route(&self) -> Result<Route<'a>, LibraryError> {
        let throwaway = StorageRecovery::new();
        let recovery = self.recovery.unwrap_or(&throwaway);
        recovery.recover_media_storage(&*self.env.settings, false);
        let config = self.env.settings.current();
        if config.media_storage_provider.as_deref() == Some("s3") {
            Ok(Route::S3(Box::new(s3_library(self.env, &config)?)))
        } else {
            Ok(Route::Local(MediaLibrary::from_config(self.vault, &config)))
        }
    }

    /// `GET /library`.
    pub async fn list(&self) -> Result<LibraryListing, LibraryError> {
        match self.route()? {
            Route::Local(library) => Ok(library.try_list()?),
            Route::S3(library) => Ok(library.list().await?),
        }
    }

    /// `DELETE /library/file/*path` (`Ok` = 204).
    pub async fn delete(&self, path: &str) -> Result<(), LibraryError> {
        let deleted = match self.route()? {
            Route::Local(library) => library.delete(path).map_err(|e| match e {
                MediaError::Io(_) | MediaError::Fs(_) => LibraryError::DeleteFailed,
                other => LibraryError::Media(other),
            })?,
            Route::S3(library) => library.delete(path).await?,
        };
        if deleted { Ok(()) } else { Err(LibraryError::NotFound) }
    }

    /// `PATCH /library/file/*path {name}`: S3 only; the local provider refuses.
    pub async fn rename(&self, path: &str, new_name: &str) -> Result<RenamedItem, LibraryError> {
        match self.route()? {
            // The local service has no `rename` (**[QUIRK-DROP]** candidate).
            Route::Local(_) => Err(LibraryError::RenameFailed),
            Route::S3(library) => match library.rename(path, new_name).await {
                Ok(Some(item)) => Ok(item),
                Ok(None) | Err(MediaError::S3(_) | MediaError::Io(_) | MediaError::Fs(_)) => {
                    Err(LibraryError::RenameFailed)
                }
                Err(other) => Err(LibraryError::Media(other)),
            },
        }
    }

    /// `GET /library/usage[?path=]`: builds the index exactly once per call.
    pub fn usage(&self, usage: &MediaUsage, path: Option<&str>) -> Result<UsageResponse, LibraryError> {
        let index = usage.build_index().map_err(|_| LibraryError::UsageScanFailed)?;
        Ok(match path {
            Some(path) => UsageResponse::Notes(index.get(path).cloned().unwrap_or_default()),
            None => UsageResponse::Counts(crate::usage::usage_counts(&index)),
        })
    }
}

/// The library selected by the active provider.
enum Route<'a> {
    Local(MediaLibrary<'a>),
    S3(Box<S3MediaLibrary>),
}

/// Helper for S3 routing (exposed for the app's own wiring).
pub fn s3_library(env: &MediaEnv, config: &MediaConfig) -> Result<S3MediaLibrary, LibraryError> {
    Ok(S3MediaLibrary::new(env, config)?)
}
