//! Port of `test/javascript/image_sources/web_images.test.js` (24 tests).
//! `fetch` mocks become [`FakeTransport`] replies. `renderGrid` assertions are
//! ported against the grid model ([`external_grid_items`]); pure-DOM ones are
//! skipped (listed in docs/testing/port-media-client.md).

#[path = "common/client_picker.rs"]
mod client_picker;

use client_picker::{FakeTransport, Table, s};
use frankmd_media::picker::{JsArg, PickerCx, PickerResponse, RequestBody, WebImageSource, external_grid_items};
use serde_json::json;

fn run<T>(tr: &mut FakeTransport, f: impl FnOnce(&mut PickerCx<'_>) -> T) -> T {
    let mut cx = PickerCx { transport: tr, t: None };
    f(&mut cx)
}

// port: test/javascript/image_sources/web_images.test.js:26 "initializes with empty results"
#[test]
fn initializes_with_empty_results() {
    assert!(WebImageSource::default().results.is_empty());
}

// port: test/javascript/image_sources/web_images.test.js:32 "clears results"
#[test]
fn reset_clears_results() {
    let mut src = WebImageSource { results: vec![json!({"url": "test.jpg"})] };
    src.reset();
    assert!(src.results.is_empty());
}

// port: test/javascript/image_sources/web_images.test.js:40 "returns error for empty query"
#[test]
fn search_empty_query_errors() {
    let mut tr = FakeTransport::default();
    let r = run(&mut tr, |cx| WebImageSource::default().search(cx, &s(""))).expect("ok");
    assert_eq!(r["error"], "Please enter search keywords");
    assert!(tr.requests.is_empty());
}

// port: test/javascript/image_sources/web_images.test.js:45 "returns error for null query"
#[test]
fn search_null_query_errors() {
    let mut tr = FakeTransport::default();
    let r = run(&mut tr, |cx| WebImageSource::default().search(cx, &JsArg::Null)).expect("ok");
    assert_eq!(r["error"], "Please enter search keywords");
}

// port: test/javascript/image_sources/web_images.test.js:50 "fetches images from API"
#[test]
fn search_fetches_images() {
    let images = json!([{"url": "http://example.com/1.jpg", "title": "Image 1"}, {"url": "http://example.com/2.jpg", "title": "Image 2"}]);
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": images}));
    let mut src = WebImageSource::default();
    let r = run(&mut tr, |cx| src.search(cx, &s("cats"))).expect("ok");
    assert_eq!(tr.requests[0].method, "GET");
    assert_eq!(tr.requests[0].url, "/images/search_web?q=cats");
    assert_eq!(r["images"], images);
    assert_eq!(r["message"], "Found 2 images - click to select");
    assert_eq!(json!(src.results), images);
}

// port: test/javascript/image_sources/web_images.test.js:68 "encodes query parameters"
#[test]
fn search_encodes_query() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": []}));
    let _ = run(&mut tr, |cx| WebImageSource::default().search(cx, &s("cats & dogs")));
    assert_eq!(tr.requests[0].url, "/images/search_web?q=cats%20%26%20dogs");
}

// port: test/javascript/image_sources/web_images.test.js:78 "handles API error response"
#[test]
fn search_api_error() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"error": "API rate limit exceeded"}));
    let mut src = WebImageSource { results: vec![json!({"url": "old"})] };
    let r = run(&mut tr, |cx| src.search(cx, &s("cats"))).expect("ok");
    assert_eq!(r["error"], "API rate limit exceeded");
    assert!(src.results.is_empty());
}

// port: test/javascript/image_sources/web_images.test.js:89 "handles empty results"
#[test]
fn search_empty_results() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": []}));
    let mut src = WebImageSource::default();
    let r = run(&mut tr, |cx| src.search(cx, &s("xyznonexistent"))).expect("ok");
    assert_eq!(r["message"], "No images found");
    assert!(src.results.is_empty());
}

// port: test/javascript/image_sources/web_images.test.js:100 "uses localized empty and result messages while keeping server diagnostics intact"
#[test]
fn localized_messages_keep_server_diagnostics() {
    let t = Table(vec![
        ("dialogs.image_picker.no_images_found", "No pictures in this locale"),
        ("dialogs.image_picker.search_result_many", "%{count} localized pictures"),
        ("dialogs.image_picker.search_keywords_required", "Localized search prompt"),
    ]);
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": []}));
    tr.push_json(json!({"images": [{"url": "one"}, {"url": "two"}]}));
    tr.push_json(json!({"error": "Provider rate limit reached"}));
    let mut src = WebImageSource::default();
    let mut cx = PickerCx { transport: &mut tr, t: Some(&t) };
    assert_eq!(src.search(&mut cx, &s("")).expect("ok")["error"], "Localized search prompt");
    assert_eq!(src.search(&mut cx, &s("no results")).expect("ok")["message"], "No pictures in this locale");
    assert_eq!(src.search(&mut cx, &s("two results")).expect("ok")["message"], "2 localized pictures");
    assert_eq!(src.search(&mut cx, &s("server error")).expect("ok")["error"], "Provider rate limit reached");
}

// port: test/javascript/image_sources/web_images.test.js:118 "handles empty results with note"
#[test]
fn empty_results_with_note() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": [], "note": "Try different keywords"}));
    let r = run(&mut tr, |cx| WebImageSource::default().search(cx, &s("xyznonexistent"))).expect("ok");
    assert_eq!(r["message"], "Try different keywords");
}

// port: test/javascript/image_sources/web_images.test.js:128 "handles network error"
#[test]
fn network_error() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::NetworkError("Network error".into()));
    let mut src = WebImageSource::default();
    let r = run(&mut tr, |cx| src.search(cx, &s("cats"))).expect("ok");
    assert_eq!(r["error"], "Search failed. Please try again.");
    assert!(src.results.is_empty());
}

// port: test/javascript/image_sources/web_images.test.js:139 "renders empty state when no results"
#[test]
fn grid_empty_state() {
    assert!(external_grid_items(&[]).is_empty());
}

// port: test/javascript/image_sources/web_images.test.js:148 "renders image buttons"
#[test]
fn grid_renders_items() {
    let items = external_grid_items(&[json!({"url": "http://example.com/1.jpg", "title": "Cat", "source": "example.com"})]);
    assert_eq!(items[0].url, "http://example.com/1.jpg");
    assert_eq!(items[0].title, "Cat");
    assert_eq!(items[0].source, "example.com");
    assert_eq!(items[0].index, 0);
}

// skipped (C, DOM event): web_images.test.js:162 "removes broken thumbnails without an inline error handler"

// port: test/javascript/image_sources/web_images.test.js:174 "renders dimensions when available"
#[test]
fn grid_dimensions() {
    let items = external_grid_items(&[json!({"url": "http://example.com/1.jpg", "width": 800, "height": 600})]);
    assert_eq!(items[0].dimensions.as_deref(), Some("800x600"));
}

// port: test/javascript/image_sources/web_images.test.js:185 "uses thumbnail when available"
#[test]
fn grid_uses_thumbnail() {
    let items = external_grid_items(&[json!({"url": "http://example.com/full.jpg", "thumbnail": "http://example.com/thumb.jpg"})]);
    assert_eq!(items[0].image_src, "http://example.com/thumb.jpg");
    assert_eq!(items[0].thumbnail, "http://example.com/thumb.jpg");
}

// skipped (C, HTML escaping — the GPUI view renders plain text): web_images.test.js:197 "escapes HTML in titles"
// skipped (C, HTML attribute injection): web_images.test.js:209 "does not allow a quoted title to inject attributes"

// port: test/javascript/image_sources/web_images.test.js:218 "rejects unsafe image source URLs"
#[test]
fn grid_rejects_unsafe_image_urls() {
    let items = external_grid_items(&[json!({"url": "javascript:alert(1)", "title": "Image"})]);
    assert_eq!(items[0].image_src, "");
}

// skipped (C, DOM selection classes): web_images.test.js:229 "removes selection classes from all items"
// skipped (C, DOM): web_images.test.js:246 "handles null container"

// port: test/javascript/image_sources/web_images.test.js:252 "uploads external URL to S3"
#[test]
fn save_copy_posts_import() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"url": "https://s3.example.com/uploaded.jpg"}));
    let r = run(&mut tr, |cx| WebImageSource::default().save_copy(cx, &json!("https://example.com/image.jpg"), &json!("0.5"), None)).expect("ok");
    assert_eq!(tr.requests[0].method, "POST");
    assert_eq!(tr.requests[0].url, "/images/import");
    assert_eq!(tr.requests[0].body, RequestBody::Json(json!({"url": "https://example.com/image.jpg", "resize": "0.5"})));
    assert_eq!(r["url"], "https://s3.example.com/uploaded.jpg");
}

// port: test/javascript/image_sources/web_images.test.js:271 "handles upload error"
#[test]
fn save_copy_error_throws_server_message() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::Reply { ok: false, status: 422, json: json!({"error": "Image too large"}) });
    let err = run(&mut tr, |cx| WebImageSource::default().save_copy(cx, &json!("https://example.com/big.jpg"), &json!(""), None)).expect_err("throws");
    assert_eq!(err.message, "Image too large");
}

// port: test/javascript/image_sources/web_images.test.js:281 "handles generic upload error"
#[test]
fn save_copy_generic_error() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::Reply { ok: false, status: 500, json: json!({}) });
    let err = run(&mut tr, |cx| WebImageSource::default().save_copy(cx, &json!("https://example.com/image.jpg"), &json!(""), None)).expect_err("throws");
    assert_eq!(err.message, "Failed to upload to S3");
}
