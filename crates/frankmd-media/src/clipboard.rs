//! **[COMPAT]** pasted clipboard images (`app/javascript/lib/clipboard_image.js`).
//!
//! The canonical implementation lives in [`frankmd_core::text::clipboard`];
//! it is re-exported here because pasted images enter the media pipeline.
//! The synthesized name only decides which allow-list the bytes are checked
//! against — the pipeline still sniffs the content.

pub use frankmd_core::text::clipboard::{
    ClipboardItem, ClipboardItemKind, PastedFile, extension_for_mime, image_file_from_clipboard,
    normalize_image_file,
};
