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
    todo!()
}

/// Keep a file whose name already ends in `.<alnum>+` (case-insensitive)
/// untouched; otherwise rename to `pasted-image-<lastModified|clip><ext>`
/// with [`extension_for_mime`].
pub fn normalize_image_file(file: PastedFile) -> PastedFile {
    todo!()
}

/// **[COMPAT]** `image/png→.png, jpeg→.jpg, gif→.gif, webp→.webp, bmp→.bmp`;
/// otherwise `.<subtype>` with any `+suffix` and non-alphanumerics removed,
/// lowercased (`image/svg+xml → .svg`, `image/tiff → .tiff`); `.bin` when
/// nothing remains.
pub fn extension_for_mime(mime: &str) -> String {
    todo!()
}
