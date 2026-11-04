//! Port of `test/javascript/image_sources/folder_images.test.js` (32 tests).
//! Blob/object-URL bookkeeping (`URL.createObjectURL`/`revokeObjectURL`) is
//! browser-only and skipped; decoded dimensions come from `PickedFile::dimensions`.

#[path = "common/client_picker.rs"]
mod client_picker;

use client_picker::{FakeTransport, file};
use frankmd_media::picker::{
    DirEntry, DirectoryPicker, FolderImageSource, FormField, JsArg, PickError, PickedFile, PickerCx, PickerResponse,
    RequestBody,
};
use serde_json::json;

struct Picker(Result<Vec<DirEntry>, PickError>);
impl DirectoryPicker for Picker {
    fn pick(&mut self) -> Result<Vec<DirEntry>, PickError> {
        self.0.clone()
    }
}

fn with_all(files: Vec<PickedFile>) -> FolderImageSource {
    FolderImageSource { all_images: files, ..FolderImageSource::default() }
}

fn numbered(n: usize) -> Vec<PickedFile> {
    (0..n).map(|i| file(&format!("image{i}.jpg"), i as i64, 1024)).collect()
}

fn three() -> FolderImageSource {
    with_all(vec![file("cat.jpg", 1000, 1024), file("dog.png", 2000, 2048), file("bird.jpg", 3000, 512)])
}

fn shown(src: &FolderImageSource) -> Vec<String> {
    src.displayed_images.iter().map(|d| d.file.name.clone()).collect()
}

// port: test/javascript/image_sources/folder_images.test.js:40 "initializes with empty arrays"
#[test]
fn initializes_empty() {
    let s = FolderImageSource::default();
    assert!(s.displayed_images.is_empty() && s.all_images.is_empty());
}

// port: test/javascript/image_sources/folder_images.test.js:47 "returns true when showDirectoryPicker is available"
#[test]
fn supported_when_picker_available() {
    let mut p = Picker(Ok(vec![]));
    let r = FolderImageSource::default().browse(None, Some(&mut p), None).expect("ok");
    assert_eq!(r, json!({"count": 0}));
}

// port: test/javascript/image_sources/folder_images.test.js:52 "returns false when showDirectoryPicker is not available"
#[test]
fn unsupported_without_picker() {
    let r = FolderImageSource::default().browse(None, None, None).expect("ok");
    assert_eq!(r["error"], "File System Access API not supported");
}

// port: test/javascript/image_sources/folder_images.test.js:59 "calls cleanup"
#[test]
fn reset_cleans_up() {
    let mut s = three();
    s.filter("", 0, 10).expect("ok");
    s.reset();
    assert!(s.all_images.is_empty() && s.displayed_images.is_empty());
}

// skipped (browser blob URLs): folder_images.test.js:67 "revokes object URLs"

// port: test/javascript/image_sources/folder_images.test.js:81 "clears arrays"
#[test]
fn cleanup_clears_arrays() {
    let mut s = with_all(vec![file("test.jpg", 0, 1)]);
    s.filter("", 0, 10).expect("ok");
    s.reset();
    assert!(s.all_images.is_empty() && s.displayed_images.is_empty());
}

// port: test/javascript/image_sources/folder_images.test.js:93 "returns error when API not supported"
#[test]
fn browse_unsupported() {
    assert_eq!(FolderImageSource::default().browse(None, None, None).expect("ok")["error"], "File System Access API not supported");
}

// port: test/javascript/image_sources/folder_images.test.js:101 "returns cancelled when user cancels picker"
#[test]
fn browse_cancelled() {
    let mut p = Picker(Err(PickError::Aborted));
    assert_eq!(FolderImageSource::default().browse(None, Some(&mut p), None).expect("ok")["cancelled"], true);
}

// port: test/javascript/image_sources/folder_images.test.js:114 "returns error on other errors"
#[test]
fn browse_error() {
    let mut p = Picker(Err(PickError::Failed("Permission denied".into())));
    assert_eq!(FolderImageSource::default().browse(None, Some(&mut p), None).expect("ok")["error"], "Error accessing folder");
}

// port: test/javascript/image_sources/folder_images.test.js:134 "returns all images when no search term"
#[test]
fn filter_all() {
    assert_eq!(three().filter("", 0, 10).expect("ok")["total"], 3);
}

// port: test/javascript/image_sources/folder_images.test.js:140 "filters by name case-insensitively"
#[test]
fn filter_case_insensitive() {
    assert_eq!(three().filter("CAT", 0, 10).expect("ok")["total"], 1);
}

// port: test/javascript/image_sources/folder_images.test.js:146 "limits displayed images to 10 by default"
#[test]
fn filter_default_limit() {
    let r = with_all(numbered(15)).filter("", 0, 10).expect("ok");
    assert_eq!(r["displayed"], 10);
    assert_eq!(r["total"], 15);
}

// port: test/javascript/image_sources/folder_images.test.js:160 "slices by offset and limit"
#[test]
fn filter_pages() {
    let mut s = with_all(numbered(15));
    let first = s.filter("", 0, 10).expect("ok");
    assert_eq!((first["displayed"].clone(), first["total"].clone()), (json!(10), json!(15)));
    assert_eq!(shown(&s), (0..10).map(|i| format!("image{i}.jpg")).collect::<Vec<_>>());
    let second = s.filter("", 10, 10).expect("ok");
    assert_eq!((second["displayed"].clone(), second["total"].clone()), (json!(5), json!(15)));
    assert_eq!(shown(&s), (10..15).map(|i| format!("image{i}.jpg")).collect::<Vec<_>>());
}

// port: test/javascript/image_sources/folder_images.test.js:183 "returns an empty page when offset is past the end"
#[test]
fn filter_offset_past_end() {
    let r = with_all(numbered(5)).filter("", 20, 10).expect("ok");
    assert_eq!(r["displayed"], 0);
    assert_eq!(r["total"], 5);
}

// port: test/javascript/image_sources/folder_images.test.js:197 "resets to the requested page when combined with a search term"
#[test]
fn filter_search_with_page() {
    let r = with_all(vec![file("cat1.jpg", 1, 1024), file("cat2.jpg", 2, 1024), file("dog.png", 3, 1024)]).filter("cat", 0, 10).expect("ok");
    assert_eq!(r["total"], 2);
    assert_eq!(r["displayed"], 2);
}

// skipped (browser blob URLs): folder_images.test.js:210 "revokes previous object URLs"
// skipped (browser blob URLs): folder_images.test.js:222 "creates new object URLs"

// port: test/javascript/image_sources/folder_images.test.js:230 "resolves with dimensions on load"
#[test]
fn dimensions_on_load() {
    let mut f = file("a.png", 0, 1);
    f.dimensions = Some((800, 600));
    let mut s = with_all(vec![f]);
    s.filter("", 0, 10).expect("ok");
    assert_eq!((s.displayed_images[0].width, s.displayed_images[0].height), (Some(800), Some(600)));
}

// port: test/javascript/image_sources/folder_images.test.js:237 "resolves with null dimensions on error"
#[test]
fn dimensions_null_on_error() {
    let mut s = with_all(vec![file("broken.png", 0, 1)]);
    s.filter("", 0, 10).expect("ok");
    assert_eq!((s.displayed_images[0].width, s.displayed_images[0].height), (None, None));
}

// port: test/javascript/image_sources/folder_images.test.js:254 "renders empty state when no images"
#[test]
fn grid_empty() {
    let s = FolderImageSource::default();
    assert!(s.grid_items().is_empty());
    assert_eq!(s.status_text(None, None), "No images found");
}

// port: test/javascript/image_sources/folder_images.test.js:263 "renders images with data attributes"
#[test]
fn grid_items() {
    let mut f = file("photo.jpg", 0, 102_400);
    f.dimensions = Some((800, 600));
    let mut s = with_all(vec![f]);
    s.filter("", 0, 10).expect("ok");
    let items = s.grid_items();
    assert_eq!(items[0].index, 0);
    assert_eq!(items[0].dimensions.as_deref(), Some("800x600"));
    assert_eq!(items[0].size_kb, 100);
}

// port: test/javascript/image_sources/folder_images.test.js:284 "updates status container"
#[test]
fn status_counts() {
    let mut s = with_all(vec![file("a.jpg", 0, 1024), file("b.jpg", 0, 1024)]);
    s.filter("", 0, 10).expect("ok");
    assert_eq!(s.status_text(None, None), "2 images found");
}

// port: test/javascript/image_sources/folder_images.test.js:297 "shows partial count when limited"
#[test]
fn status_partial() {
    let mut s = with_all(vec![file("a.jpg", 0, 1024)]);
    s.filter("", 0, 10).expect("ok");
    assert_eq!(s.status_text(None, Some(50)), "Showing 1 most recent of 50 images");
}

// skipped (C, HTML escaping): folder_images.test.js:309 "escapes HTML in names"
// skipped (C, HTML attribute injection): folder_images.test.js:320 "does not allow a quoted filename to inject image attributes"

// port: test/javascript/image_sources/folder_images.test.js:331 "returns image at index"
#[test]
fn get_image_at_index() {
    let mut s = with_all(vec![file("b.jpg", 1, 1), file("a.jpg", 2, 1)]);
    s.filter("", 0, 10).expect("ok");
    assert_eq!(s.get_image(1).map(|d| d.file.name.as_str()), Some("a.jpg"));
}

// port: test/javascript/image_sources/folder_images.test.js:340 "returns undefined for invalid index"
#[test]
fn get_image_out_of_range() {
    assert!(FolderImageSource::default().get_image(0).is_none());
}

// skipped (C, DOM selection classes): folder_images.test.js:348 "removes selected class from all items"
// skipped (C, DOM): folder_images.test.js:362 "handles null container"

fn upload(tr: &mut FakeTransport, resize: &str) -> Result<serde_json::Value, frankmd_media::picker::Thrown> {
    let mut cx = PickerCx { transport: tr, t: None };
    let f = PickedFile { name: "blob.jpg".into(), last_modified: 0, size: 4, mime: "image/jpeg".into(), dimensions: None };
    FolderImageSource::default().upload(&mut cx, &f, &JsArg::Str(resize.into()), None)
}

// port: test/javascript/image_sources/folder_images.test.js:368 "uploads file with FormData"
#[test]
fn upload_posts_form_data() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"url": "/images/uploaded.jpg"}));
    let r = upload(&mut tr, "0.5").expect("ok");
    assert_eq!(tr.requests[0].method, "POST");
    assert_eq!(tr.requests[0].url, "/images/upload");
    assert_eq!(
        tr.requests[0].body,
        RequestBody::Form(vec![
            ("file".into(), FormField::File { name: "blob.jpg".into(), size: 4, mime: "image/jpeg".into() }),
            ("resize".into(), FormField::Text("0.5".into())),
        ])
    );
    assert_eq!(r["url"], "/images/uploaded.jpg");
}

// port: test/javascript/image_sources/folder_images.test.js:389 "handles upload error"
#[test]
fn upload_error() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::Reply { ok: false, status: 422, json: json!({"error": "File too large"}) });
    assert_eq!(upload(&mut tr, "").expect_err("throws").message, "File too large");
}

// port: test/javascript/image_sources/folder_images.test.js:400 "handles generic error"
#[test]
fn upload_generic_error() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::Reply { ok: false, status: 500, json: json!({}) });
    assert_eq!(upload(&mut tr, "").expect_err("throws").message, "Upload failed");
}
