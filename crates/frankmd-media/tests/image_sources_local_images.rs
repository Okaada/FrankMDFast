//! Port of `test/javascript/image_sources/local_images.test.js` (24 tests).

#[path = "common/client_picker.rs"]
mod client_picker;

use client_picker::{FakeTransport, s};
use frankmd_media::picker::{JsArg, LocalImageSource, PickerCx, PickerResponse, RequestBody};
use serde_json::{Value, json};

fn n(v: f64) -> JsArg {
    JsArg::Number(v)
}

fn load(tr: &mut FakeTransport, search: JsArg, limit: f64, offset: f64) -> Value {
    let mut cx = PickerCx { transport: tr, t: None };
    LocalImageSource::default().load(&mut cx, &search, &n(limit), &n(offset)).expect("ok")
}

fn fail(tr: &mut FakeTransport, json: Value) {
    tr.replies.push_back(PickerResponse::Reply { ok: false, status: 422, json });
}

// port: test/javascript/image_sources/local_images.test.js:22 "initializes with null selectedPath"
#[test]
fn initializes_without_selection() {
    assert_eq!(LocalImageSource::default().selected_path, None);
}

// port: test/javascript/image_sources/local_images.test.js:28 "clears selectedPath"
#[test]
fn reset_clears_selection() {
    let mut src = LocalImageSource { selected_path: Some("/images/photo.jpg".into()) };
    src.reset();
    assert_eq!(src.selected_path, None);
}

// port: test/javascript/image_sources/local_images.test.js:36 "fetches all images when no search, defaulting to limit 10 offset 0"
#[test]
fn load_defaults() {
    let data = json!({"images": [{"path": "photo1.jpg", "name": "Photo 1"}, {"path": "photo2.jpg", "name": "Photo 2"}], "total": 2, "limit": 10, "offset": 0});
    let mut tr = FakeTransport::default();
    tr.push_json(data.clone());
    let r = load(&mut tr, JsArg::Undefined, 10.0, 0.0);
    assert_eq!(tr.requests[0].method, "GET");
    assert_eq!(tr.requests[0].url, "/images?limit=10&offset=0");
    assert_eq!(r, data);
}

// port: test/javascript/image_sources/local_images.test.js:61 "fetches images with search query"
#[test]
fn load_with_search() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": [], "total": 0, "limit": 10, "offset": 0}));
    load(&mut tr, s("sunset"), 10.0, 0.0);
    assert_eq!(tr.requests[0].url, "/images?search=sunset&limit=10&offset=0");
}

// port: test/javascript/image_sources/local_images.test.js:75 "encodes search query"
#[test]
fn load_encodes_search() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": []}));
    load(&mut tr, s("cats & dogs"), 10.0, 0.0);
    assert_eq!(tr.requests[0].url, "/images?search=cats%20%26%20dogs&limit=10&offset=0");
}

// port: test/javascript/image_sources/local_images.test.js:89 "passes custom limit and offset as query params"
#[test]
fn load_custom_paging() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": []}));
    load(&mut tr, s(""), 25.0, 25.0);
    assert_eq!(tr.requests[0].url, "/images?limit=25&offset=25");
}

// port: test/javascript/image_sources/local_images.test.js:103 "handles failed response"
#[test]
fn load_failed_response() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, Value::Null);
    assert_eq!(load(&mut tr, JsArg::Undefined, 10.0, 0.0)["error"], "Error loading images");
}

// port: test/javascript/image_sources/local_images.test.js:113 "handles network error"
#[test]
fn load_network_error() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::NetworkError("Network error".into()));
    assert_eq!(load(&mut tr, JsArg::Undefined, 10.0, 0.0)["error"], "Error loading images");
}

// port: test/javascript/image_sources/local_images.test.js:123 "renders empty state when no images"
#[test]
fn grid_empty() {
    assert!(LocalImageSource::default().grid_items(&[]).is_empty());
}

// port: test/javascript/image_sources/local_images.test.js:132 "renders null images as empty"
#[test]
fn grid_null_is_empty() {
    // `renderGrid(null)` — the Rust caller passes an empty slice for a missing list.
    assert!(LocalImageSource::default().grid_items(&[]).is_empty());
}

// port: test/javascript/image_sources/local_images.test.js:140 "renders image grid items"
#[test]
fn grid_items() {
    let items = LocalImageSource::default().grid_items(&[json!({"path": "photos/cat.jpg", "name": "cat.jpg", "width": 800, "height": 600})]);
    assert_eq!(items[0].path, "photos/cat.jpg");
    assert_eq!(items[0].name, "cat.jpg");
    assert_eq!(items[0].dimensions.as_deref(), Some("800x600"));
}

// port: test/javascript/image_sources/local_images.test.js:155 "marks selected image"
#[test]
fn grid_marks_selected() {
    let src = LocalImageSource { selected_path: Some("photos/cat.jpg".into()) };
    let items = src.grid_items(&[json!({"path": "photos/cat.jpg", "name": "cat.jpg"}), json!({"path": "photos/dog.jpg", "name": "dog.jpg"})]);
    assert!(items[0].selected);
    assert!(!items[1].selected);
}

// port: test/javascript/image_sources/local_images.test.js:168 "generates preview URL with encoded path"
#[test]
fn grid_preview_url_encoded() {
    let items = LocalImageSource::default().grid_items(&[json!({"path": "photos/my cat.jpg", "name": "my cat.jpg"})]);
    assert_eq!(items[0].preview_url, "/images/preview/photos/my%20cat.jpg");
}

// skipped (C, HTML escaping): local_images.test.js:179 "escapes HTML in names"
// skipped (C, HTML attribute injection): local_images.test.js:190 "does not allow a quoted filename to inject image attributes"

// port: test/javascript/image_sources/local_images.test.js:197 "handles images without dimensions"
#[test]
fn grid_without_dimensions() {
    let items = LocalImageSource::default().grid_items(&[json!({"path": "test.jpg", "name": "test.jpg"})]);
    assert_eq!(items[0].dimensions, None);
}

// skipped (C, DOM selection classes): local_images.test.js:211 "removes selected class from all items"
// skipped (C, DOM): local_images.test.js:228 "handles null container"

// port: test/javascript/image_sources/local_images.test.js:234 "uploads image to S3"
#[test]
fn save_copy_posts_import() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"url": "https://s3.example.com/image.jpg"}));
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let r = LocalImageSource::default().save_copy(&mut cx, &json!("photos/cat.jpg"), &json!("0.5"), None).expect("ok");
    assert_eq!(r["url"], "https://s3.example.com/image.jpg");
    assert_eq!(tr.requests[0].method, "POST");
    assert_eq!(tr.requests[0].url, "/images/import");
    assert_eq!(tr.requests[0].body, RequestBody::Json(json!({"path": "photos/cat.jpg", "resize": "0.5"})));
}

// port: test/javascript/image_sources/local_images.test.js:253 "handles upload error"
#[test]
fn save_copy_error() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, json!({"error": "S3 bucket not configured"}));
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let e = LocalImageSource::default().save_copy(&mut cx, &json!("test.jpg"), &json!(""), None).expect_err("throws");
    assert_eq!(e.message, "S3 bucket not configured");
}

// port: test/javascript/image_sources/local_images.test.js:263 "handles generic upload error"
#[test]
fn save_copy_generic_error() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, json!({}));
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let e = LocalImageSource::default().save_copy(&mut cx, &json!("test.jpg"), &json!(""), None).expect_err("throws");
    assert_eq!(e.message, "Failed to upload to S3");
}

// port: test/javascript/image_sources/local_images.test.js:275 "sends a DELETE to the encoded file path"
#[test]
fn delete_sends_delete() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"success": true}));
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let r = LocalImageSource::default().delete_image(&mut cx, "photos/cat.jpg").expect("ok");
    assert_eq!(r["success"], true);
    assert_eq!(tr.requests[0].method, "DELETE");
    assert_eq!(tr.requests[0].url, "/images/file/photos/cat.jpg");
}

// port: test/javascript/image_sources/local_images.test.js:289 "throws with the server error message on failure"
#[test]
fn delete_error_message() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, json!({"error": "Image not found or could not be deleted"}));
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let e = LocalImageSource::default().delete_image(&mut cx, "gone.jpg").expect_err("throws");
    assert_eq!(e.message, "Image not found or could not be deleted");
}

// port: test/javascript/image_sources/local_images.test.js:299 "throws a generic error when the server gives none"
#[test]
fn delete_generic_error() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, json!({}));
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let e = LocalImageSource::default().delete_image(&mut cx, "gone.jpg").expect_err("throws");
    assert_eq!(e.message, "Failed to delete image");
}
