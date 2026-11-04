//! Port of `test/javascript/image_sources/google_images.test.js` (21 tests).

#[path = "common/client_picker.rs"]
mod client_picker;

use client_picker::{FakeTransport, s};
use frankmd_media::picker::{GoogleImageSource, PickerCx, PickerResponse, external_grid_items};
use serde_json::{Value, json};

fn src_with(query: &str, next_start: Value, results: Vec<Value>) -> GoogleImageSource {
    GoogleImageSource {
        query: query.into(),
        next_start,
        results,
        ..GoogleImageSource::default()
    }
}

fn load_more(tr: &mut FakeTransport, src: &mut GoogleImageSource) -> Value {
    let mut cx = PickerCx {
        transport: tr,
        t: None,
    };
    src.load_more(&mut cx).expect("ok")
}

// port: test/javascript/image_sources/google_images.test.js:22 "initializes with default state"
#[test]
fn default_state() {
    let s = GoogleImageSource::default();
    assert!(!s.enabled);
    assert!(s.results.is_empty());
    assert_eq!(s.next_start, json!(0), "nextStart starts at 0");
    assert!(!s.loading);
    assert_eq!(s.query, "");
}

// port: test/javascript/image_sources/google_images.test.js:32 "clears all state except enabled"
#[test]
fn reset_keeps_enabled() {
    let mut s = GoogleImageSource {
        enabled: true,
        results: vec![json!({"url": "test.jpg"})],
        next_start: json!(10),
        loading: true,
        query: "cats".into(),
    };
    s.reset();
    assert!(s.enabled);
    assert!(s.results.is_empty());
    assert_eq!(s.next_start, json!(0));
    assert!(!s.loading);
    assert_eq!(s.query, "");
}

// port: test/javascript/image_sources/google_images.test.js:50 "returns error for empty query"
#[test]
fn empty_query() {
    let mut tr = FakeTransport::default();
    let mut cx = PickerCx {
        transport: &mut tr,
        t: None,
    };
    assert_eq!(
        GoogleImageSource::default()
            .search(&mut cx, &s(""))
            .expect("ok")["error"],
        "Please enter search keywords"
    );
}

// port: test/javascript/image_sources/google_images.test.js:55 "resets state and calls loadMore"
#[test]
fn search_resets_and_loads() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": [{"url": "new.jpg"}], "next_start": 11, "total": 100}));
    let mut src = src_with("", json!(10), vec![json!({"url": "old.jpg"})]);
    let mut cx = PickerCx {
        transport: &mut tr,
        t: None,
    };
    src.search(&mut cx, &s("cats")).expect("ok");
    assert_eq!(src.query, "cats");
    assert_eq!(src.next_start, json!(11));
    assert_eq!(tr.requests[0].url, "/images/search_google?q=cats&start=1");
    assert_eq!(src.results, vec![json!({"url": "new.jpg"})]);
}

// port: test/javascript/image_sources/google_images.test.js:75 "fetches images with pagination"
#[test]
fn load_more_paginates() {
    let mut tr = FakeTransport::default();
    tr.push_json(
        json!({"images": [{"url": "http://example.com/1.jpg"}], "next_start": 11, "total": 50}),
    );
    let mut src = src_with("cats", json!(1), vec![]);
    let r = load_more(&mut tr, &mut src);
    assert_eq!(tr.requests[0].method, "GET");
    assert_eq!(tr.requests[0].url, "/images/search_google?q=cats&start=1");
    assert_eq!(r["images"].as_array().map(Vec::len), Some(1));
    assert_eq!(r["total"], 50);
    assert_eq!(src.next_start, json!(11));
}

// port: test/javascript/image_sources/google_images.test.js:95 "appends to existing results"
#[test]
fn load_more_appends() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": [{"url": "second.jpg"}], "next_start": 21}));
    let mut src = src_with("cats", json!(11), vec![json!({"url": "first.jpg"})]);
    load_more(&mut tr, &mut src);
    assert_eq!(src.results.len(), 2);
    assert_eq!(src.results[0]["url"], "first.jpg");
    assert_eq!(src.results[1]["url"], "second.jpg");
}

// port: test/javascript/image_sources/google_images.test.js:114 "prevents concurrent loading"
#[test]
fn load_more_guard() {
    let mut tr = FakeTransport::default();
    let mut src = GoogleImageSource {
        loading: true,
        results: vec![json!({"url": "existing.jpg"})],
        ..GoogleImageSource::default()
    };
    let r = load_more(&mut tr, &mut src);
    assert_eq!(r["images"], json!([{"url": "existing.jpg"}]));
    assert!(tr.requests.is_empty());
}

// port: test/javascript/image_sources/google_images.test.js:127 "handles API error"
#[test]
fn load_more_api_error() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"error": "API key invalid"}));
    let mut src = src_with("cats", json!(1), vec![]);
    assert_eq!(load_more(&mut tr, &mut src)["error"], "API key invalid");
    assert!(!src.loading);
}

// port: test/javascript/image_sources/google_images.test.js:141 "handles network error"
#[test]
fn load_more_network_error() {
    let mut tr = FakeTransport::default();
    tr.replies
        .push_back(PickerResponse::NetworkError("Network error".into()));
    let mut src = src_with("cats", json!(1), vec![]);
    assert_eq!(
        load_more(&mut tr, &mut src)["error"],
        "Search failed. Please try again."
    );
    assert!(!src.loading);
}

// port: test/javascript/image_sources/google_images.test.js:154 "returns appropriate message for empty results"
#[test]
fn load_more_empty_message() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": []}));
    let mut src = src_with("cats", json!(1), vec![]);
    assert_eq!(load_more(&mut tr, &mut src)["message"], "No images found");
}

// port: test/javascript/image_sources/google_images.test.js:167 "returns count message for results"
#[test]
fn load_more_count_message_uses_total() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"images": [{"url": "1.jpg"}, {"url": "2.jpg"}], "total": 150}));
    let mut src = src_with("cats", json!(1), vec![]);
    assert_eq!(
        load_more(&mut tr, &mut src)["message"],
        "Found 150 images - click to select"
    );
}

fn with(loading: bool, n: usize) -> GoogleImageSource {
    GoogleImageSource {
        loading,
        results: vec![json!({"url": "test.jpg"}); n],
        ..GoogleImageSource::default()
    }
}

// port: test/javascript/image_sources/google_images.test.js:185 "returns true when near bottom and not loading"
#[test]
fn should_load_more_near_bottom() {
    assert!(with(false, 1).should_load_more(50.0));
}

// port: test/javascript/image_sources/google_images.test.js:192 "returns false when far from bottom"
#[test]
fn should_not_load_far_from_bottom() {
    assert!(!with(false, 1).should_load_more(200.0));
}

// port: test/javascript/image_sources/google_images.test.js:199 "returns false when loading"
#[test]
fn should_not_load_while_loading() {
    assert!(!with(true, 1).should_load_more(50.0));
}

// port: test/javascript/image_sources/google_images.test.js:206 "returns false when no results"
#[test]
fn should_not_load_without_results() {
    assert!(!with(false, 0).should_load_more(50.0));
}

// port: test/javascript/image_sources/google_images.test.js:215 "renders empty state when no results"
#[test]
fn grid_empty() {
    assert!(external_grid_items(&[]).is_empty());
}

// port: test/javascript/image_sources/google_images.test.js:224 "renders image buttons with all data attributes"
#[test]
fn grid_items() {
    let items = external_grid_items(&[
        json!({"url": "http://example.com/full.jpg", "thumbnail": "http://example.com/thumb.jpg",
        "title": "Cat Photo", "source": "example.com", "width": 1024, "height": 768}),
    ]);
    let i = &items[0];
    assert_eq!(i.url, "http://example.com/full.jpg");
    assert_eq!(i.thumbnail, "http://example.com/thumb.jpg");
    assert_eq!(i.title, "Cat Photo");
    assert_eq!(i.source, "example.com");
    assert_eq!(i.dimensions.as_deref(), Some("1024x768"));
}

// skipped (C, DOM event): google_images.test.js:247 "removes broken thumbnails without an inline error handler"
// skipped (C, HTML escaping): google_images.test.js:259 "escapes HTML in content"
// skipped (C, DOM selection classes): google_images.test.js:272 "removes selection classes"
// skipped (C, DOM): google_images.test.js:283 "handles null container gracefully"
