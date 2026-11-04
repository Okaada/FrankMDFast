//! Port of `test/javascript/image_sources/pinterest_images.test.js` (15 tests).

#[path = "common/client_picker.rs"]
mod client_picker;

use client_picker::{FakeTransport, s};
use frankmd_media::picker::{
    JsArg, PickerCx, PickerResponse, PinterestImageSource, external_grid_items,
};
use serde_json::json;

fn search(tr: &mut FakeTransport, src: &mut PinterestImageSource, q: &JsArg) -> serde_json::Value {
    let mut cx = PickerCx {
        transport: tr,
        t: None,
    };
    src.search(&mut cx, q).expect("ok")
}

// port: test/javascript/image_sources/pinterest_images.test.js:22 "initializes with empty results"
#[test]
fn initializes_empty() {
    assert!(PinterestImageSource::default().results.is_empty());
}

// port: test/javascript/image_sources/pinterest_images.test.js:28 "clears results"
#[test]
fn reset_clears() {
    let mut src = PinterestImageSource {
        results: vec![json!({"url": "test.jpg"})],
    };
    src.reset();
    assert!(src.results.is_empty());
}

// port: test/javascript/image_sources/pinterest_images.test.js:36 "returns error for empty query"
#[test]
fn empty_query() {
    let mut tr = FakeTransport::default();
    assert_eq!(
        search(&mut tr, &mut PinterestImageSource::default(), &s(""))["error"],
        "Please enter search keywords"
    );
}

// port: test/javascript/image_sources/pinterest_images.test.js:41 "returns error for null query"
#[test]
fn null_query() {
    let mut tr = FakeTransport::default();
    assert_eq!(
        search(&mut tr, &mut PinterestImageSource::default(), &JsArg::Null)["error"],
        "Please enter search keywords"
    );
}

// port: test/javascript/image_sources/pinterest_images.test.js:46 "fetches images from Pinterest API"
#[test]
fn fetches_images() {
    let images = json!([{"url": "https://pinterest.com/1.jpg", "title": "Pin 1"}, {"url": "https://pinterest.com/2.jpg", "title": "Pin 2"}]);
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": images}));
    let mut src = PinterestImageSource::default();
    let r = search(&mut tr, &mut src, &s("home decor"));
    assert_eq!(tr.requests[0].method, "GET");
    assert_eq!(
        tr.requests[0].url,
        "/images/search_pinterest?q=home%20decor"
    );
    assert_eq!(r["images"], images);
    assert_eq!(r["message"], "Found 2 images - click to select");
    assert_eq!(json!(src.results), images);
}

// port: test/javascript/image_sources/pinterest_images.test.js:64 "handles API error response"
#[test]
fn api_error() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"error": "Pinterest blocked"}));
    let mut src = PinterestImageSource::default();
    assert_eq!(
        search(&mut tr, &mut src, &s("cats"))["error"],
        "Pinterest blocked"
    );
    assert!(src.results.is_empty());
}

// port: test/javascript/image_sources/pinterest_images.test.js:75 "handles empty results"
#[test]
fn empty_results() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": []}));
    let mut src = PinterestImageSource::default();
    assert_eq!(
        search(&mut tr, &mut src, &s("xyznonexistent"))["message"],
        "No images found"
    );
    assert!(src.results.is_empty());
}

// port: test/javascript/image_sources/pinterest_images.test.js:86 "handles network error"
#[test]
fn network_error() {
    let mut tr = FakeTransport::default();
    tr.replies
        .push_back(PickerResponse::NetworkError("Network error".into()));
    let mut src = PinterestImageSource::default();
    assert_eq!(
        search(&mut tr, &mut src, &s("cats"))["error"],
        "Search failed. Please try again."
    );
    assert!(src.results.is_empty());
}

// port: test/javascript/image_sources/pinterest_images.test.js:98 "renders empty state when no results"
#[test]
fn grid_empty() {
    assert!(external_grid_items(&[]).is_empty());
}

// port: test/javascript/image_sources/pinterest_images.test.js:107 "renders Pinterest image buttons"
#[test]
fn grid_items() {
    let items = external_grid_items(&[
        json!({"url": "https://pinterest.com/full.jpg", "thumbnail": "https://pinterest.com/thumb.jpg",
        "title": "DIY Project", "source": "pinterest.com", "width": 600, "height": 800}),
    ]);
    let i = &items[0];
    assert_eq!(i.url, "https://pinterest.com/full.jpg");
    assert_eq!(i.thumbnail, "https://pinterest.com/thumb.jpg");
    assert_eq!(i.title, "DIY Project");
    assert_eq!(i.source, "pinterest.com");
    assert_eq!(i.dimensions.as_deref(), Some("600x800"));
}

// skipped (C, DOM event): pinterest_images.test.js:129 "removes broken thumbnails without an inline error handler"

// port: test/javascript/image_sources/pinterest_images.test.js:141 "handles missing optional fields"
#[test]
fn grid_missing_optional_fields() {
    let items = external_grid_items(&[json!({"url": "https://pinterest.com/image.jpg"})]);
    assert_eq!(items[0].url, "https://pinterest.com/image.jpg");
    assert_eq!(items[0].title, "");
    assert_eq!(items[0].source, "");
    assert_eq!(items[0].thumbnail, "https://pinterest.com/image.jpg");
}

// skipped (C, HTML escaping): pinterest_images.test.js:154 "escapes HTML in titles"
// skipped (C, DOM selection classes): pinterest_images.test.js:168 "removes selection classes from all items"
// skipped (C, DOM): pinterest_images.test.js:185 "handles null container"
