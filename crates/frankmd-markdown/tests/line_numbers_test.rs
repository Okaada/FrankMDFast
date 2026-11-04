//! Port of `test/javascript/lib/line_numbers.test.js` (10 tests).
#![allow(clippy::unwrap_used)]

use frankmd_markdown::line_numbers::{LineNumberMode, normalize_line_number_mode};

// port: test/javascript/lib/line_numbers.test.js:13 "defines OFF as 0"
#[test]
fn off_is_zero() {
    assert_eq!(LineNumberMode::Off.value(), 0);
}

// port: test/javascript/lib/line_numbers.test.js:17 "defines ABSOLUTE as 1"
#[test]
fn absolute_is_one() {
    assert_eq!(LineNumberMode::Absolute.value(), 1);
}

// port: test/javascript/lib/line_numbers.test.js:21 "defines RELATIVE as 2"
#[test]
fn relative_is_two() {
    assert_eq!(LineNumberMode::Relative.value(), 2);
}

// port: test/javascript/lib/line_numbers.test.js:27 "returns fallback for undefined value"
#[test]
fn undefined_returns_fallback() {
    assert_eq!(normalize_line_number_mode(None, LineNumberMode::Off), LineNumberMode::Off);
}

// port: test/javascript/lib/line_numbers.test.js:31 "parses string values"
#[test]
fn parses_string_values() {
    assert_eq!(normalize_line_number_mode(Some("1"), LineNumberMode::Off), LineNumberMode::Absolute);
}

// port: test/javascript/lib/line_numbers.test.js:35 "accepts numeric values"
// (numbers arrive as their string form, e.g. from `.fed`)
#[test]
fn accepts_numeric_values() {
    assert_eq!(normalize_line_number_mode(Some(&2.to_string()), LineNumberMode::Off), LineNumberMode::Relative);
}

// port: test/javascript/lib/line_numbers.test.js:39 "returns fallback for invalid values"
#[test]
fn invalid_returns_fallback() {
    assert_eq!(normalize_line_number_mode(Some("99"), LineNumberMode::Absolute), LineNumberMode::Absolute);
    assert_eq!(normalize_line_number_mode(Some("invalid"), LineNumberMode::Off), LineNumberMode::Off);
}

// port: test/javascript/lib/line_numbers.test.js:46 "cycles OFF to ABSOLUTE"
#[test]
fn cycles_off_to_absolute() {
    assert_eq!(LineNumberMode::Off.next(), LineNumberMode::Absolute);
}

// port: test/javascript/lib/line_numbers.test.js:50 "cycles ABSOLUTE to RELATIVE"
#[test]
fn cycles_absolute_to_relative() {
    assert_eq!(LineNumberMode::Absolute.next(), LineNumberMode::Relative);
}

// port: test/javascript/lib/line_numbers.test.js:54 "cycles RELATIVE to OFF"
#[test]
fn cycles_relative_to_off() {
    assert_eq!(LineNumberMode::Relative.next(), LineNumberMode::Off);
}
