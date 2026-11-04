//! Port of `test/javascript/lib/clipboard_image.test.js` (6 tests).

use frankmd_media::clipboard::{ClipboardItem, PastedFile, image_file_from_clipboard, normalize_image_file};

fn file(name: &str, mime: &str) -> PastedFile {
    PastedFile { name: name.into(), mime: mime.into(), last_modified: None, bytes: b"x".to_vec() }
}

fn item(kind: &str, mime: &str, f: Option<PastedFile>) -> ClipboardItem {
    ClipboardItem { kind: kind.into(), mime: mime.into(), file: f }
}

// port: test/javascript/lib/clipboard_image.test.js:12 "returns the image File when the clipboard carries one"
#[test]
fn returns_the_image_file() {
    let items = [item("file", "image/png", Some(file("shot.png", "image/png")))];
    let got = image_file_from_clipboard(Some(&items)).expect("file");
    assert_eq!(got.name, "shot.png");
}

// port: test/javascript/lib/clipboard_image.test.js:19 "skips non-file and non-image items"
#[test]
fn skips_non_file_and_non_image_items() {
    let items = [item("string", "text/plain", None), item("file", "application/pdf", Some(file("doc.pdf", "application/pdf")))];
    assert_eq!(image_file_from_clipboard(Some(&items)), None);
}

// port: test/javascript/lib/clipboard_image.test.js:27 "returns null for empty or missing clipboard data"
#[test]
fn none_for_empty_or_missing_data() {
    assert_eq!(image_file_from_clipboard(None), None);
    assert_eq!(image_file_from_clipboard(Some(&[])), None);
}

// port: test/javascript/lib/clipboard_image.test.js:34 "synthesizes a filename with an extension from the MIME type"
#[test]
fn synthesizes_filename_from_mime() {
    let got = normalize_image_file(file("", "image/png"));
    assert!(got.name.starts_with("pasted-image-") && got.name.ends_with(".png"), "{}", got.name);
    assert_eq!(got.mime, "image/png");
    assert_eq!(normalize_image_file(PastedFile { last_modified: Some(42), ..file("", "image/jpeg") }).name, "pasted-image-42.jpg");
    assert_eq!(normalize_image_file(file("", "image/gif")).name, "pasted-image-clip.gif");
}

// port: test/javascript/lib/clipboard_image.test.js:41 "leaves a file that already has an extension untouched"
#[test]
fn leaves_named_file_untouched() {
    let named = file("photo.jpg", "image/jpeg");
    assert_eq!(normalize_image_file(named.clone()), named);
}

// port: test/javascript/lib/clipboard_image.test.js:46 "derives the extension from the MIME subtype for unknown image types"
#[test]
fn derives_extension_from_mime_subtype() {
    assert!(normalize_image_file(file("", "image/tiff")).name.ends_with(".tiff"));
    assert!(normalize_image_file(file("", "image/svg+xml")).name.ends_with(".svg"));
    assert!(normalize_image_file(file("", "image/")).name.ends_with(".bin"));
}
