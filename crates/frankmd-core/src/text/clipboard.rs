//! Pasted-image naming (logic of `lib/clipboard_image.js`).
//!
//! The image-picker Drop ingest validates by filename extension, and
//! clipboard images often arrive with an empty or extension-less name, so a
//! name is synthesized from the MIME type.

/// A file taken from the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PastedFile {
    /// File name (often empty for screenshots).
    pub name: String,
    /// MIME type, e.g. `image/png`.
    pub mime: String,
    /// `lastModified` in ms since the epoch; `None`/`0` → `"clip"` in the name.
    pub last_modified: Option<u64>,
    /// File contents.
    pub bytes: Vec<u8>,
}

/// Kind of a clipboard item (DataTransferItem `kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ClipboardItemKind {
    File,
    String,
}

/// One clipboard item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardItem {
    /// Item kind.
    pub kind: ClipboardItemKind,
    /// Item MIME type.
    pub mime: String,
    /// The file, when the platform could materialize one (`getAsFile()`).
    pub file: Option<PastedFile>,
}

/// First `file` item whose MIME starts with `image/` and that yields a file,
/// normalized with [`normalize_image_file`]; `None` for no data / no image.
pub fn image_file_from_clipboard(items: Option<&[ClipboardItem]>) -> Option<PastedFile> {
    items?
        .iter()
        .filter(|item| item.kind == ClipboardItemKind::File && item.mime.starts_with("image/"))
        .find_map(|item| item.file.clone())
        .map(normalize_image_file)
}

/// Keep a file whose name already ends in `.<alnum>+` (case-insensitive)
/// untouched; otherwise rename to `pasted-image-<lastModified|clip><ext>`
/// with [`extension_for_mime`].
pub fn normalize_image_file(file: PastedFile) -> PastedFile {
    if has_extension(&file.name) {
        return file;
    }
    let stamp = file.last_modified.filter(|&ms| ms != 0).map_or_else(|| "clip".to_owned(), |ms| ms.to_string());
    let name = format!("pasted-image-{stamp}{}", extension_for_mime(&file.mime));
    PastedFile { name, ..file }
}

/// **[COMPAT]** `image/png→.png, jpeg→.jpg, gif→.gif, webp→.webp, bmp→.bmp`;
/// otherwise `.<subtype>` with any `+suffix` and non-alphanumerics removed,
/// lowercased (`image/svg+xml → .svg`, `image/tiff → .tiff`); `.bin` when
/// nothing remains.
pub fn extension_for_mime(mime: &str) -> String {
    let known = match mime {
        "image/png" => Some(".png"),
        "image/jpeg" => Some(".jpg"),
        "image/gif" => Some(".gif"),
        "image/webp" => Some(".webp"),
        "image/bmp" => Some(".bmp"),
        _ => None,
    };
    if let Some(ext) = known {
        return ext.to_owned();
    }
    let subtype = mime.split('/').nth(1).unwrap_or("");
    let subtype = subtype.split_once('+').map_or(subtype, |(head, _)| head);
    let cleaned: String = subtype.chars().filter(char::is_ascii_alphanumeric).collect();
    if cleaned.is_empty() { ".bin".to_owned() } else { format!(".{}", cleaned.to_ascii_lowercase()) }
}

/// `/\.[a-z0-9]+$/i`.
fn has_extension(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| !ext.is_empty() && ext.bytes().all(|b| b.is_ascii_alphanumeric()))
}
