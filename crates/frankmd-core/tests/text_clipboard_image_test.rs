//! Port of test/javascript/lib/clipboard_image.test.js (logic part; the JS
//! `File`/`DataTransferItem` objects become plain structs).
#![allow(clippy::unwrap_used)]

use frankmd_core::text::clipboard::{
    ClipboardItem, ClipboardItemKind, PastedFile, extension_for_mime, image_file_from_clipboard,
    normalize_image_file,
};

fn file(name: &str, mime: &str) -> PastedFile {
    PastedFile { name: name.into(), mime: mime.into(), last_modified: None, bytes: b"x".to_vec() }
}

fn item(kind: ClipboardItemKind, mime: &str, f: Option<PastedFile>) -> ClipboardItem {
    ClipboardItem { kind, mime: mime.into(), file: f }
}

fn has_pasted_name(name: &str, ext: &str) -> bool {
    name.starts_with("pasted-image-") && name.ends_with(ext) && name.len() > "pasted-image-".len() + ext.len()
}

// port: test/javascript/lib/clipboard_image.test.js:12 "returns the image File when the clipboard carries one"
#[test]
fn returns_image_file_when_clipboard_carries_one() {
    let png = file("shot.png", "image/png");
    let items = [item(ClipboardItemKind::File, "image/png", Some(png.clone()))];
    let got = image_file_from_clipboard(Some(&items)).expect("image file");
    assert_eq!(got.name, "shot.png");
    assert_eq!(got, png);
}

// port: test/javascript/lib/clipboard_image.test.js:19 "skips non-file and non-image items"
#[test]
fn skips_non_file_and_non_image_items() {
    let items = [
        item(ClipboardItemKind::String, "text/plain", None),
        item(ClipboardItemKind::File, "application/pdf", Some(file("doc.pdf", "application/pdf"))),
    ];
    assert_eq!(image_file_from_clipboard(Some(&items)), None);
}

// port: test/javascript/lib/clipboard_image.test.js:27 "returns null for empty or missing clipboard data"
#[test]
fn none_for_empty_or_missing_clipboard_data() {
    assert_eq!(image_file_from_clipboard(None), None);
    assert_eq!(image_file_from_clipboard(Some(&[])), None);
}

// port: test/javascript/lib/clipboard_image.test.js:34 "synthesizes a filename with an extension from the MIME type"
#[test]
fn synthesizes_filename_from_mime() {
    let got = normalize_image_file(file("", "image/png"));
    assert!(has_pasted_name(&got.name, ".png"), "{}", got.name);
    assert_eq!(got.mime, "image/png");
    assert_eq!(got.bytes, b"x");
}

// port: test/javascript/lib/clipboard_image.test.js:41 "leaves a file that already has an extension untouched"
#[test]
fn leaves_named_file_untouched() {
    let named = file("photo.jpg", "image/jpeg");
    assert_eq!(normalize_image_file(named.clone()), named);
}

// port: test/javascript/lib/clipboard_image.test.js:46 "derives the extension from the MIME subtype for unknown image types"
#[test]
fn derives_extension_from_mime_subtype_for_unknown_types() {
    assert!(normalize_image_file(file("", "image/tiff")).name.ends_with(".tiff"));
    assert!(normalize_image_file(file("", "image/svg+xml")).name.ends_with(".svg"));
}

// Rust-only: pins the exact synthesized names / MIME table read from clipboard_image.js.
#[test]
fn exact_names_and_extension_table() {
    let mut f = file("", "image/jpeg");
    f.last_modified = Some(1_700_000_000_000);
    assert_eq!(normalize_image_file(f).name, "pasted-image-1700000000000.jpg");
    let mut z = file("noext", "image/gif");
    z.last_modified = Some(0); // JS `0 || "clip"`
    assert_eq!(normalize_image_file(z).name, "pasted-image-clip.gif");
    for (mime, ext) in [
        ("image/png", ".png"),
        ("image/jpeg", ".jpg"),
        ("image/gif", ".gif"),
        ("image/webp", ".webp"),
        ("image/bmp", ".bmp"),
        ("image/x-icon", ".xicon"),
        ("image/SVG+xml", ".svg"),
        ("image", ".bin"),
        ("image/", ".bin"),
    ] {
        assert_eq!(extension_for_mime(mime), ext, "{mime}");
    }
    // Extension test is case-insensitive alnum.
    let upper = file("Shot.PNG", "image/png");
    assert_eq!(normalize_image_file(upper.clone()), upper);
}
