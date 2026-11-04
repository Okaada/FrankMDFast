//! Port of `test/javascript/url_utils.test.js` (16 tests).

use frankmd_media::url_utils::{encode_path, extract_youtube_id};

fn yt(s: &str) -> Option<String> {
    extract_youtube_id(Some(s))
}

// port: test/javascript/url_utils.test.js:8 "encodes path segments"
#[test]
fn encodes_path_segments() {
    assert_eq!(encode_path(Some("hello world/test file.md")), "hello%20world/test%20file.md");
}

// port: test/javascript/url_utils.test.js:12 "preserves forward slashes"
#[test]
fn preserves_forward_slashes() {
    assert_eq!(encode_path(Some("a/b/c")), "a/b/c");
}

// port: test/javascript/url_utils.test.js:16 "encodes special characters"
#[test]
fn encodes_special_characters() {
    assert_eq!(encode_path(Some("file#1.md")), "file%231.md");
    assert_eq!(encode_path(Some("file?query.md")), "file%3Fquery.md");
}

// port: test/javascript/url_utils.test.js:21 "handles empty path"
#[test]
fn handles_empty_path() {
    assert_eq!(encode_path(Some("")), "");
    assert_eq!(encode_path(None), "");
}

// port: test/javascript/url_utils.test.js:27 "handles deeply nested paths"
#[test]
fn handles_deeply_nested_paths() {
    assert_eq!(encode_path(Some("a/b/c/d/e.md")), "a/b/c/d/e.md");
}

// port: test/javascript/url_utils.test.js:31 "encodes unicode characters"
#[test]
fn encodes_unicode_characters() {
    assert_eq!(encode_path(Some("café/résumé.md")), "caf%C3%A9/r%C3%A9sum%C3%A9.md");
}

// port: test/javascript/url_utils.test.js:37 "extracts ID from standard watch URL"
#[test]
fn extracts_id_from_watch_url() {
    assert_eq!(yt("https://www.youtube.com/watch?v=dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
}

// port: test/javascript/url_utils.test.js:42 "extracts ID from short URL"
#[test]
fn extracts_id_from_short_url() {
    assert_eq!(yt("https://youtu.be/dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
}

// port: test/javascript/url_utils.test.js:47 "extracts ID from embed URL"
#[test]
fn extracts_id_from_embed_url() {
    assert_eq!(yt("https://www.youtube.com/embed/dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
}

// port: test/javascript/url_utils.test.js:52 "extracts ID from v/ URL"
#[test]
fn extracts_id_from_v_url() {
    assert_eq!(yt("https://www.youtube.com/v/dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
}

// port: test/javascript/url_utils.test.js:57 "returns the ID if just passed an ID"
#[test]
fn returns_bare_id() {
    assert_eq!(yt("dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
}

// port: test/javascript/url_utils.test.js:61 "returns null for non-YouTube URLs"
#[test]
fn none_for_non_youtube_urls() {
    assert_eq!(yt("https://vimeo.com/123456"), None);
}

// port: test/javascript/url_utils.test.js:65 "returns null for invalid URLs"
#[test]
fn none_for_invalid_urls() {
    assert_eq!(yt("not a url"), None);
}

// port: test/javascript/url_utils.test.js:69 "returns null for empty input"
#[test]
fn none_for_empty_input() {
    assert_eq!(yt(""), None);
    assert_eq!(extract_youtube_id(None), None);
}

// port: test/javascript/url_utils.test.js:75 "handles URL with additional parameters"
#[test]
fn handles_additional_parameters() {
    assert_eq!(yt("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=30s").as_deref(), Some("dQw4w9WgXcQ"));
}

// port: test/javascript/url_utils.test.js:80 "handles URL without www"
#[test]
fn handles_url_without_www() {
    assert_eq!(yt("https://youtube.com/watch?v=dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
}
