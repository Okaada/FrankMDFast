//! **[COMPAT]** the user's server image folder (`images_path`) half of
//! `app/services/images_service.rb`: listing, lookup, preview and delete.
//! Re-exported from [`crate::images`] (the original kept both halves in one service).
//!
//! The folder lives *outside* the vault, so containment is checked against
//! `images_path` itself (symlink-aware, like `PathSafety.contain`).

use std::path::{Path, PathBuf};

use crate::config::MediaConfig;
use crate::param::Param;

/// `SUPPORTED_EXTENSIONS` (listing filter; note `.svg` is listed but not uploadable).
pub const SUPPORTED_EXTENSIONS: [&str; 7] =
    [".jpg", ".jpeg", ".png", ".gif", ".webp", ".svg", ".bmp"];
/// `MAX_RESULTS`.
pub const MAX_RESULTS: usize = 10;

/// One listed server image (`GET /images` item).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerImage {
    /// Basename.
    pub name: String,
    /// Path relative to `images_path`.
    pub path: String,
    /// Absolute path.
    pub full_path: PathBuf,
    /// ISO-8601 mtime (local offset, no fraction).
    pub mtime: String,
    /// Bytes.
    pub size: u64,
    /// Pixel width (`None` if unreadable).
    pub width: Option<u32>,
    /// Pixel height.
    pub height: Option<u32>,
}

/// `{width:, height:}`; both `None` when the file is not a readable image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dimensions {
    /// Width.
    pub width: Option<u32>,
    /// Height.
    pub height: Option<u32>,
}

/// `enabled?`: `images_path` present.
pub fn enabled(config: &MediaConfig) -> bool {
    config
        .images_path
        .as_ref()
        .is_some_and(|p| !p.as_os_str().is_empty())
}

/// `clamp_limit`: `Integer(v).clamp(1, 100)`, [`MAX_RESULTS`] when `Integer()`
/// raises (`nil`, `"abc"`, `"12abc"`); floats truncate (`2.7` → 2).
pub fn clamp_limit(value: &Param) -> usize {
    ruby_integer(value).map_or(MAX_RESULTS, |n| n.clamp(1, 100) as usize)
}

/// `clamp_offset`: `max(Integer(v), 0)`, 0 when `Integer()` raises.
pub fn clamp_offset(value: &Param) -> usize {
    ruby_integer(value).map_or(0, |n| usize::try_from(n.max(0)).unwrap_or(usize::MAX))
}

/// `list(search:, limit:, offset:)`: recursive (dotfiles/hidden dirs skipped),
/// [`SUPPORTED_EXTENSIONS`] only (case-insensitive), case-insensitive basename
/// substring `search`, newest mtime first, then sliced. `[]` when disabled or
/// the folder is missing.
pub fn list(
    config: &MediaConfig,
    search: Option<&str>,
    limit: &Param,
    offset: &Param,
) -> Vec<ServerImage> {
    let Some(root) = existing_root(config) else {
        return Vec::new();
    };
    let limit = clamp_limit(limit);
    let offset = clamp_offset(offset);
    let mut files: Vec<(PathBuf, std::fs::Metadata)> = find_images(&root, search)
        .into_iter()
        .filter_map(|p| std::fs::metadata(&p).ok().map(|m| (p, m)))
        .collect();
    // `sort_by { -mtime.to_i }` (whole seconds; ties keep glob order).
    files.sort_by_key(|(_, m)| std::cmp::Reverse(frankmd_fs::meta::mtime_secs(m)));
    files
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(file, meta)| {
            let dimensions = image_dimensions(&file);
            ServerImage {
                name: file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: file
                    .strip_prefix(&root)
                    .map(|r| r.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                mtime: meta.modified().map(iso8601_local).unwrap_or_default(),
                size: meta.len(),
                width: dimensions.width,
                height: dimensions.height,
                full_path: file,
            }
        })
        .collect()
}

/// `count(search:)`: total matches (ignores paging); 0 when disabled/missing.
pub fn count(config: &MediaConfig, search: Option<&str>) -> usize {
    existing_root(config).map_or(0, |root| find_images(&root, search).len())
}

/// `find_image(path)`: contained in `images_path` (symlink-aware), an existing
/// regular file, extension in image + video upload lists. Else `None`.
pub fn find_image(config: &MediaConfig, rel: &str) -> Option<PathBuf> {
    let root = config.images_path.as_ref().filter(|_| enabled(config))?;
    if rel.trim().is_empty() {
        return None;
    }
    let full = frankmd_fs::path_safety::contain(root, Some(rel))?;
    if !full.is_file() {
        return None;
    }
    let ext = extname(&full.to_string_lossy()).to_lowercase();
    let allowed = config
        .image_upload_extensions
        .iter()
        .chain(&config.video_upload_extensions);
    allowed.into_iter().any(|e| *e == ext).then_some(full)
}

/// `delete(path)`: deletes what [`find_image`] accepts → `true`; anything else
/// (missing, traversal, non-image, IO error) → `false`.
pub fn delete(config: &MediaConfig, rel: &str) -> bool {
    let (Some(root), Some(full)) = (config.images_path.as_ref(), find_image(config, rel)) else {
        return false;
    };
    // The folder is outside the vault and belongs to the user: open it as a
    // foreign root so the removal goes through frankmd-fs (containment
    // re-checked there) while its lock lives in the app state dir — no app
    // file is ever created in the picture folder.
    let Ok(vault) = frankmd_fs::Vault::open_foreign(root, crate::pipeline::app_state_dir()) else {
        return false;
    };
    let Ok(real) = full.canonicalize() else {
        return false;
    };
    let Some(rel) = real
        .strip_prefix(vault.real_root())
        .ok()
        .and_then(|r| r.to_str())
    else {
        return false;
    };
    match vault.safe_path(rel, true) {
        Ok(contained) => vault.delete_note(&contained, None).is_ok(),
        Err(_) => false,
    }
}

/// `get_image_dimensions(path)`.
pub fn image_dimensions(path: &Path) -> Dimensions {
    let dims = image::ImageReader::open(path)
        .ok()
        .and_then(|r| r.with_guessed_format().ok())
        .and_then(|r| r.into_dimensions().ok());
    match dims {
        Some((w, h)) => Dimensions {
            width: Some(w),
            height: Some(h),
        },
        None => Dimensions::default(),
    }
}

/// `images_path` when enabled and existing.
fn existing_root(config: &MediaConfig) -> Option<PathBuf> {
    let root = config.images_path.as_ref().filter(|_| enabled(config))?;
    root.exists().then(|| root.clone())
}

/// `find_images(search)`: `Pathname.glob(images_path/**/*)` — pre-order,
/// entries sorted by name, dot entries skipped, symlinked directories not
/// descended (Ruby's `**/` does not follow them) — keeping regular files with
/// a supported extension whose basename contains `search` (case-insensitive).
///
/// Security: a file symlink is listed only when its realpath stays inside
/// `images_path` (Ruby listed any symlinked file; previewing goes through
/// [`find_image`]'s containment check anyway).
fn find_images(root: &Path, search: Option<&str>) -> Vec<PathBuf> {
    let real_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let needle = search
        .filter(|s| !s.trim().is_empty())
        .map(str::to_lowercase);
    let mut out = Vec::new();
    walk(root, &real_root, needle.as_deref(), &mut out);
    out
}

fn walk(dir: &Path, real_root: &Path, needle: Option<&str>, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let is_link = entry.file_type().is_ok_and(|t| t.is_symlink());
        if is_link
            && !path
                .canonicalize()
                .is_ok_and(|real| real.starts_with(real_root))
        {
            continue;
        }
        if path.is_file() {
            let supported = SUPPORTED_EXTENSIONS.contains(&extname(&name).to_lowercase().as_str());
            let matches = needle.is_none_or(|n| name.to_lowercase().contains(n));
            if supported && matches {
                out.push(path);
            }
        } else if !is_link && path.is_dir() {
            walk(&path, real_root, needle, out);
        }
    }
}

/// Ruby `Integer(value)` (strict): `None` where it raises.
fn ruby_integer(value: &Param) -> Option<i64> {
    match value {
        Param::Int(i) => Some(*i),
        Param::Float(f) if f.is_finite() => Some(f.trunc() as i64),
        Param::Str(s) => parse_ruby_integer(s),
        _ => None,
    }
}

/// `Integer("…")`: surrounding whitespace, optional sign, `0x`/`0b`/`0o`/`0`
/// radix prefixes, digits with single `_` separators; anything else raises.
fn parse_ruby_integer(raw: &str) -> Option<i64> {
    let s = raw.trim_matches(|c: char| c.is_ascii_whitespace() || c == '\0');
    let (negative, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let lower = s.to_ascii_lowercase();
    let (radix, digits) = if let Some(d) = lower.strip_prefix("0x") {
        (16, d)
    } else if let Some(d) = lower.strip_prefix("0b") {
        (2, d)
    } else if let Some(d) = lower.strip_prefix("0o") {
        (8, d)
    } else if lower.len() > 1 && lower.starts_with('0') {
        (8, lower.trim_start_matches('0').trim_start_matches('_'))
    } else {
        (10, lower.as_str())
    };
    if digits.is_empty() && radix == 8 && lower.chars().all(|c| c == '0' || c == '_') {
        return Some(0);
    }
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || digits.contains("__")
    {
        return None;
    }
    let clean: String = digits.chars().filter(|c| *c != '_').collect();
    let n = i64::from_str_radix(&clean, radix).ok()?;
    Some(if negative { -n } else { n })
}

/// Ruby `File.extname`: the last `.suffix` of the basename, ignoring leading
/// dots (`".bashrc"` → `""`, `"a."` → `"."`).
pub(crate) fn extname(path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    let trimmed = base.trim_start_matches('.');
    trimmed
        .rfind('.')
        .map_or_else(String::new, |i| trimmed[i..].to_owned())
}

/// `Time#iso8601` in the process zone, whole seconds (`2026-01-01T12:00:00+00:00`).
pub(crate) fn iso8601_local(time: std::time::SystemTime) -> String {
    jiff::Timestamp::try_from(time)
        .map(|ts| {
            ts.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%Y-%m-%dT%H:%M:%S%:z")
                .to_string()
        })
        .unwrap_or_default()
}

/// `ERB::Util.url_encode` of every `/` segment, rejoined with `/`.
pub(crate) fn url_encode_segments(path: &str) -> String {
    path.split('/')
        .map(url_encode)
        .collect::<Vec<_>>()
        .join("/")
}

/// `ERB::Util.url_encode`: everything but `[A-Za-z0-9_.~-]` percent-encoded (UTF-8).
pub(crate) fn url_encode(part: &str) -> String {
    let mut out = String::with_capacity(part.len());
    for b in part.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'~' | b'-') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
