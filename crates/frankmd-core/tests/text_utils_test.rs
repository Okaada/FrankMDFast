//! Port of test/javascript/text_utils.test.js (lib/text_utils.js).

use frankmd_core::text::utils::{
    escape_html, escape_html_string, fuzzy_score, is_safe_image_url, levenshtein_distance,
};


// port: test/javascript/text_utils.test.js:11 "escapes HTML and both quote types for attribute interpolation"
#[test]
fn escape_html_escapes_html_and_both_quote_types() {
    assert_eq!(
        escape_html(Some("<img \"quoted\" 'single'>")),
        "&lt;img &quot;quoted&quot; &#39;single&#39;&gt;"
    );
}

// port: test/javascript/text_utils.test.js:17 "accepts supported image URL schemes and rejects javascript URLs"
#[test]
fn is_safe_image_url_accepts_supported_schemes_rejects_javascript() {
    assert!(is_safe_image_url("http://example.com/image.png"));
    assert!(is_safe_image_url("https://example.com/image.png"));
    assert!(is_safe_image_url("data:image/png;base64,abc"));
    assert!(is_safe_image_url("blob:https://app.example/id"));
    assert!(is_safe_image_url("/images/preview/folder/a.jpg"));
    assert!(!is_safe_image_url("javascript:alert(1)"));
    assert!(!is_safe_image_url("//evil.example/a.jpg"));
    assert!(!is_safe_image_url("/\\evil.example/a.jpg"));
    assert!(!is_safe_image_url(" javascript:alert(1)"));
    assert!(!is_safe_image_url("\n/images/preview/a.jpg"));
    assert!(!is_safe_image_url("file:///tmp/image.png"));
}

// port: test/javascript/text_utils.test.js:33 "escapes HTML special characters"
#[test]
fn escape_html_string_escapes_special_characters() {
    assert_eq!(
        escape_html_string(Some("<script>alert('xss')</script>")),
        "&lt;script&gt;alert(&#039;xss&#039;)&lt;/script&gt;"
    );
}

// port: test/javascript/text_utils.test.js:38 "escapes ampersands"
#[test]
fn escape_html_string_escapes_ampersands() {
    assert_eq!(escape_html_string(Some("foo & bar")), "foo &amp; bar");
}

// port: test/javascript/text_utils.test.js:42 "escapes quotes"
#[test]
fn escape_html_string_escapes_quotes() {
    assert_eq!(escape_html_string(Some("He said \"hello\"")), "He said &quot;hello&quot;");
}

// port: test/javascript/text_utils.test.js:46 "handles empty string"
#[test]
fn escape_html_string_handles_empty_string() {
    assert_eq!(escape_html_string(Some("")), "");
}

// port: test/javascript/text_utils.test.js:50 "handles null/undefined"
#[test]
fn escape_html_string_handles_null_undefined() {
    assert_eq!(escape_html_string(None), "");
}

// port: test/javascript/text_utils.test.js:55 "preserves normal text"
#[test]
fn escape_html_string_preserves_normal_text() {
    assert_eq!(escape_html_string(Some("Hello World")), "Hello World");
}

// port: test/javascript/text_utils.test.js:61 "returns 0 for no match"
#[test]
fn fuzzy_score_returns_0_for_no_match() {
    assert_eq!(fuzzy_score("hello", "xyz"), 0);
}

// port: test/javascript/text_utils.test.js:65 "returns positive score for match"
#[test]
fn fuzzy_score_returns_positive_score_for_match() {
    assert!(fuzzy_score("hello", "hel") > 0);
}

// port: test/javascript/text_utils.test.js:69 "gives higher score to consecutive matches"
#[test]
fn fuzzy_score_gives_higher_score_to_consecutive_matches() {
    assert!(fuzzy_score("hello", "hel") > fuzzy_score("hello", "hlo"));
}

// port: test/javascript/text_utils.test.js:75 "gives bonus for matching after separator"
#[test]
fn fuzzy_score_gives_bonus_after_separator() {
    assert!(fuzzy_score("src/button", "button") > fuzzy_score("srcxbutton", "button"));
}

// port: test/javascript/text_utils.test.js:82 "gives bonus for matching at start"
#[test]
fn fuzzy_score_gives_bonus_at_start() {
    assert!(fuzzy_score("readme.md", "read") > fuzzy_score("unreadme.md", "read"));
}

// port: test/javascript/text_utils.test.js:88 "prefers shorter strings with same match"
#[test]
fn fuzzy_score_prefers_shorter_strings() {
    assert!(fuzzy_score("app.js", "app") > fuzzy_score("application.js", "app"));
}

// port: test/javascript/text_utils.test.js:94 "handles empty query"
#[test]
fn fuzzy_score_handles_empty_query() {
    // Empty query gives base length bonus: max(0, 10 - (5 - 0)) = 5
    assert_eq!(fuzzy_score("hello", ""), 5);
}

// port: test/javascript/text_utils.test.js:101 "returns 0 for identical strings"
#[test]
fn levenshtein_returns_0_for_identical_strings() {
    assert_eq!(levenshtein_distance("hello", "hello"), 0);
}

// port: test/javascript/text_utils.test.js:105 "returns string length for empty comparison"
#[test]
fn levenshtein_returns_length_for_empty_comparison() {
    assert_eq!(levenshtein_distance("hello", ""), 5);
    assert_eq!(levenshtein_distance("", "hello"), 5);
}

// port: test/javascript/text_utils.test.js:110 "counts single character difference"
#[test]
fn levenshtein_counts_single_character_difference() {
    assert_eq!(levenshtein_distance("hello", "hallo"), 1);
}

// port: test/javascript/text_utils.test.js:114 "counts insertion"
#[test]
fn levenshtein_counts_insertion() {
    assert_eq!(levenshtein_distance("hello", "hellos"), 1);
}

// port: test/javascript/text_utils.test.js:118 "counts deletion"
#[test]
fn levenshtein_counts_deletion() {
    assert_eq!(levenshtein_distance("hello", "helo"), 1);
}

// port: test/javascript/text_utils.test.js:122 "counts multiple changes"
#[test]
fn levenshtein_counts_multiple_changes() {
    assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
}

// port: test/javascript/text_utils.test.js:126 "is symmetric"
#[test]
fn levenshtein_is_symmetric() {
    assert_eq!(levenshtein_distance("abc", "def"), levenshtein_distance("def", "abc"));
}

// port: test/javascript/text_utils.test.js:130 "handles empty strings"
#[test]
fn levenshtein_handles_empty_strings() {
    assert_eq!(levenshtein_distance("", ""), 0);
}
