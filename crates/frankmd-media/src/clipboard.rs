//! **[COMPAT]** port of `app/javascript/lib/clipboard_image.js`: turn a pasted
//! clipboard image into a named file the Drop ingest (and `store_media`) can
//! validate by extension.

/// One clipboard entry (`DataTransferItem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardItem {
    /// `kind` (`"file"` / `"string"`).
    pub kind: String,
    /// MIME type.
    pub mime: String,
    /// `getAsFile()` result.
    pub file: Option<PastedFile>,
}

/// A file-like blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PastedFile {
    /// File name (often empty for screenshots).
    pub name: String,
    /// MIME type.
    pub mime: String,
    /// `lastModified` (ms), `None`/0 → `"clip"` in the synthesized name.
    pub last_modified: Option<i64>,
    /// Bytes.
    pub bytes: Vec<u8>,
}

/// `EXT_BY_MIME`.
pub const EXT_BY_MIME: [(&str, &str); 5] =
    [("image/png", ".png"), ("image/jpeg", ".jpg"), ("image/gif", ".gif"), ("image/webp", ".webp"), ("image/bmp", ".bmp")];

/// `imageFileFromClipboard(data)`: first `kind == "file"` item whose type starts
/// with `image/` and yields a file, normalized; else `None`.
pub fn image_file_from_clipboard(items: Option<&[ClipboardItem]>) -> Option<PastedFile> {
    todo!()
}

/// `normalizeImageFile(file)`: unchanged if the name already ends in
/// `.[a-z0-9]+` (case-insensitive); else renamed
/// `pasted-image-<lastModified|clip><ext>` with ext from [`EXT_BY_MIME`] or the
/// MIME subtype (`+suffix` dropped, non-alnum removed, lowercased; empty → `.bin`).
pub fn normalize_image_file(file: PastedFile) -> PastedFile {
    todo!()
}
