//! Port of test/javascript/stats_utils.test.js.

use frankmd_core::text::stats::{calculate_stats, format_file_size, format_read_time};

// port: test/javascript/stats_utils.test.js:9 "counts words correctly"
#[test]
fn counts_words_correctly() {
    assert_eq!(calculate_stats(Some("Hello world this is a test")).word_count, 6);
}

// port: test/javascript/stats_utils.test.js:14 "counts characters correctly"
#[test]
fn counts_characters_correctly() {
    assert_eq!(calculate_stats(Some("Hello")).char_count_utf16, 5);
}

// port: test/javascript/stats_utils.test.js:19 "handles empty text"
#[test]
fn handles_empty_text() {
    let s = calculate_stats(Some(""));
    assert_eq!(s.word_count, 0);
    assert_eq!(s.char_count_utf16, 0);
    assert_eq!(s.byte_size, 0);
    assert_eq!(s.read_time_minutes, 0);
}

// port: test/javascript/stats_utils.test.js:27 "handles null/undefined"
#[test]
fn handles_null_undefined() {
    // JS null and undefined both map to `None`.
    assert_eq!(calculate_stats(None).word_count, 0);
}

// port: test/javascript/stats_utils.test.js:32 "handles multiple spaces between words"
#[test]
fn handles_multiple_spaces_between_words() {
    assert_eq!(calculate_stats(Some("Hello    world")).word_count, 2);
}

// port: test/javascript/stats_utils.test.js:37 "handles newlines"
#[test]
fn handles_newlines() {
    assert_eq!(calculate_stats(Some("Hello\nworld\ntest")).word_count, 3);
}

// port: test/javascript/stats_utils.test.js:42 "calculates read time"
#[test]
fn calculates_read_time() {
    let words = vec!["word"; 200].join(" ");
    assert_eq!(calculate_stats(Some(&words)).read_time_minutes, 1);
}

// port: test/javascript/stats_utils.test.js:49 "rounds read time up"
#[test]
fn rounds_read_time_up() {
    let words = vec!["word"; 201].join(" ");
    assert_eq!(calculate_stats(Some(&words)).read_time_minutes, 2);
}

// port: test/javascript/stats_utils.test.js:56 "calculates byte size for ASCII"
#[test]
fn calculates_byte_size_for_ascii() {
    assert_eq!(calculate_stats(Some("Hello")).byte_size, 5);
}

// port: test/javascript/stats_utils.test.js:61 "calculates byte size for UTF-8"
#[test]
fn calculates_byte_size_for_utf8() {
    assert_eq!(calculate_stats(Some("café")).byte_size, 5);
}

// port: test/javascript/stats_utils.test.js:68 "shows '< 1 min' for 0 minutes"
#[test]
fn read_time_less_than_one_for_zero() {
    assert_eq!(format_read_time(0.0), "< 1 min");
}

// port: test/javascript/stats_utils.test.js:72 "shows '< 1 min' for 1 minute"
#[test]
fn read_time_less_than_one_for_one() {
    assert_eq!(format_read_time(1.0), "< 1 min");
}

// port: test/javascript/stats_utils.test.js:76 "shows minutes for 2+"
#[test]
fn read_time_shows_minutes_for_two_plus() {
    assert_eq!(format_read_time(2.0), "2 min");
    assert_eq!(format_read_time(10.0), "10 min");
}

// port: test/javascript/stats_utils.test.js:83 "formats bytes"
#[test]
fn formats_bytes() {
    assert_eq!(format_file_size(0), "0 B");
    assert_eq!(format_file_size(100), "100 B");
    assert_eq!(format_file_size(1023), "1023 B");
}

// port: test/javascript/stats_utils.test.js:89 "formats kilobytes"
#[test]
fn formats_kilobytes() {
    assert_eq!(format_file_size(1024), "1.0 KB");
    assert_eq!(format_file_size(1536), "1.5 KB");
    assert_eq!(format_file_size(10240), "10 KB");
}

// port: test/javascript/stats_utils.test.js:95 "formats megabytes"
#[test]
fn formats_megabytes() {
    assert_eq!(format_file_size(1_048_576), "1.0 MB");
    assert_eq!(format_file_size(1_572_864), "1.5 MB");
}

// port: test/javascript/stats_utils.test.js:100 "formats gigabytes"
#[test]
fn formats_gigabytes() {
    assert_eq!(format_file_size(1_073_741_824), "1.0 GB");
}

// port: test/javascript/stats_utils.test.js:104 "shows decimal only when meaningful"
#[test]
fn shows_decimal_only_when_meaningful() {
    assert_eq!(format_file_size(1536), "1.5 KB");
    assert_eq!(format_file_size(10240), "10 KB");
}
