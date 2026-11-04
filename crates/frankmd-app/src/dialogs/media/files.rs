//! Local files for the dialogs: paths dropped or chosen through the portal,
//! clipboard images, folder listings. Blocking reads; callers run them on the
//! background executor.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use frankmd_core::text::clipboard::{PastedFile, normalize_image_file};
use frankmd_media::vm::picker::LocalFile;

/// Upload size cap (**[COMPAT]** 512 MB), checked before reading a file.
const MAX_READ_BYTES: u64 = frankmd_media::upload_storage::MAX_UPLOAD_BYTES;

/// Describe a file on disk (size, mtime, image dimensions). `None` for
/// directories and unreadable entries.
pub fn describe(path: &Path) -> Option<LocalFile> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    let dims = frankmd_media::server_folder::image_dimensions(path);
    let name = path.file_name()?.to_string_lossy().into_owned();
    Some(LocalFile {
        name,
        path: Some(path.to_path_buf()),
        bytes: None,
        size: meta.len(),
        modified_ms,
        dims: dims.width.zip(dims.height),
    })
}

/// Describe dropped / chosen paths (directories skipped).
pub fn describe_all(paths: &[PathBuf]) -> Vec<LocalFile> {
    paths.iter().filter_map(|p| describe(p)).collect()
}

/// The top-level files of a folder (non-recursive; the picker filters by extension).
pub fn list_folder(dir: &Path) -> std::io::Result<Vec<LocalFile>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type().is_ok_and(|t| t.is_file())
            && let Some(file) = describe(&entry.path())
        {
            files.push(file);
        }
    }
    Ok(files)
}

/// The bytes of a file to upload (in memory already, or read with the size cap).
pub fn read_bytes(file: &LocalFile) -> Result<Vec<u8>, String> {
    if let Some(bytes) = &file.bytes {
        return Ok(bytes.as_ref().clone());
    }
    let path = file
        .path
        .as_ref()
        .ok_or_else(|| "No file provided".to_owned())?;
    let size = std::fs::metadata(path)
        .map_err(|_| "Upload failed".to_owned())?
        .len();
    if size > MAX_READ_BYTES {
        return Err(frankmd_media::upload_storage::TOO_LARGE_MESSAGE.to_owned());
    }
    std::fs::read(path).map_err(|_| "Upload failed".to_owned())
}

/// A clipboard image as an upload (`pasted-image-<ms>.<ext>`, FE-359).
pub fn pasted_image(mime: &str, bytes: Vec<u8>, now_ms: u64) -> LocalFile {
    let file = normalize_image_file(PastedFile {
        name: String::new(),
        mime: mime.to_owned(),
        last_modified: Some(now_ms),
        bytes,
    });
    LocalFile {
        name: file.name,
        path: None,
        size: file.bytes.len() as u64,
        bytes: Some(Arc::new(file.bytes)),
        modified_ms: i64::try_from(now_ms).unwrap_or(i64::MAX),
        dims: None,
    }
}

/// The first image on the GPUI clipboard, if any (paste handling, FE-358).
pub fn clipboard_image(item: &gpui_kit::ClipboardItem, now_ms: u64) -> Option<LocalFile> {
    item.entries().iter().find_map(|entry| match entry {
        gpui_kit::ClipboardEntry::Image(image) => Some(pasted_image(
            image.format.mime_type(),
            image.bytes().to_vec(),
            now_ms,
        )),
        _ => None,
    })
}

/// Milliseconds since the epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}
