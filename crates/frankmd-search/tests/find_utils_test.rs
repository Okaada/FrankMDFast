//! Port of FrankMD's `test/javascript/lib/find_utils.test.js`.
#![allow(clippy::unwrap_used)]

use frankmd_search::find::{
    Direction, FindOptions, escape_regex_chars, find_all_matches, find_closest_match_index, replace_matches,
    validate_regex,
};

const REGEX: FindOptions = FindOptions { case_sensitive: false, use_regex: true, whole_word: false };

// port: test/javascript/lib/find_utils.test.js:11 "escapes regex special characters"
#[test]
fn escapes_regex_special_characters() {
    assert_eq!(escape_regex_chars("a+b*c?"), r"a\+b\*c\?");
}

// port: test/javascript/lib/find_utils.test.js:15 "returns empty string for empty input"
#[test]
fn escape_returns_empty_string_for_empty_input() {
    assert_eq!(escape_regex_chars(""), "");
}

// port: test/javascript/lib/find_utils.test.js:21 "returns empty array for empty search"
#[test]
fn returns_empty_array_for_empty_search() {
    assert!(find_all_matches("hello", "", FindOptions::default()).is_empty());
}

// port: test/javascript/lib/find_utils.test.js:25 "finds literal matches"
#[test]
fn finds_literal_matches() {
    let m = find_all_matches("Hello hello", "hello", FindOptions::default());
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].start, 0);
}

// port: test/javascript/lib/find_utils.test.js:31 "respects case sensitivity"
#[test]
fn respects_case_sensitivity() {
    let m = find_all_matches("Hello hello", "hello", FindOptions { case_sensitive: true, use_regex: false, whole_word: false });
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].text, "hello");
}

// port: test/javascript/lib/find_utils.test.js:37 "supports regex matches"
#[test]
fn supports_regex_matches() {
    assert_eq!(find_all_matches("cat cot cut", "c.t", REGEX).len(), 3);
}

// port: test/javascript/lib/find_utils.test.js:42 "captures regex groups"
#[test]
fn captures_regex_groups() {
    let m = find_all_matches("abc123", r"(abc)(\d+)", REGEX);
    assert_eq!(m[0].groups, vec![Some("abc".to_owned()), Some("123".to_owned())]);
}

// port: test/javascript/lib/find_utils.test.js:49 "replaces literal matches"
#[test]
fn replaces_literal_matches() {
    let m = find_all_matches("one two one", "one", FindOptions::default());
    assert_eq!(replace_matches("one two one", &m, "1", false), "1 two 1");
}

// port: test/javascript/lib/find_utils.test.js:55 "replaces with regex groups"
#[test]
fn replaces_with_regex_groups() {
    let m = find_all_matches("abc123", r"(abc)(\d+)", REGEX);
    assert_eq!(replace_matches("abc123", &m, "$2-$1", true), "123-abc");
}

fn b_matches() -> Vec<frankmd_search::find::Match> {
    find_all_matches("a b a b", "b", FindOptions::default())
}

// port: test/javascript/lib/find_utils.test.js:65 "finds next match from position"
#[test]
fn finds_next_match_from_position() {
    assert_eq!(find_closest_match_index(&b_matches(), 0, Direction::Next), Some(0));
}

// port: test/javascript/lib/find_utils.test.js:69 "wraps when reaching end"
#[test]
fn wraps_when_reaching_end() {
    assert_eq!(find_closest_match_index(&b_matches(), 10, Direction::Next), Some(0));
}

// port: test/javascript/lib/find_utils.test.js:73 "finds previous match from position"
#[test]
fn finds_previous_match_from_position() {
    assert_eq!(find_closest_match_index(&b_matches(), 5, Direction::Previous), Some(0));
}

// port: test/javascript/lib/find_utils.test.js:77 "wraps when going before start"
#[test]
fn wraps_when_going_before_start() {
    assert_eq!(find_closest_match_index(&b_matches(), 0, Direction::Previous), Some(1));
}

// port: test/javascript/lib/find_utils.test.js:83 "returns valid for proper regex"
#[test]
fn returns_valid_for_proper_regex() {
    assert!(validate_regex("abc").is_ok());
}

// port: test/javascript/lib/find_utils.test.js:87 "returns error for invalid regex"
#[test]
fn returns_error_for_invalid_regex() {
    let err = validate_regex("[").unwrap_err();
    assert!(!err.is_empty());
}
