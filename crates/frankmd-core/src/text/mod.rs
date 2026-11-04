//! Editor text operations ported from FrankMD's JS `lib/*_utils.js`.
//!
//! **Offset unit.** Every offset/length in this module tree is a UTF-8 *byte*
//! offset into a `&str` unless a function name says otherwise. The original
//! JS works in UTF-16 code units; conversions are explicit (below) and tested
//! (`tests/text_offsets_test.rs`). Conformance tests convert the fixtures'
//! UTF-16 offsets before calling the Rust API.

/// Convert a UTF-16 code-unit offset into a UTF-8 byte offset.
///
/// Offsets past the end clamp to `text.len()`. An offset that falls between
/// the two halves of a surrogate pair snaps *down* to the start of that
/// character (a Rust `&str` cannot be split there).
pub fn utf16_to_byte(text: &str, utf16: usize) -> usize {
    let mut units = 0usize;
    for (byte, ch) in text.char_indices() {
        let next = units + ch.len_utf16();
        if next > utf16 {
            return byte;
        }
        units = next;
    }
    text.len()
}

/// Convert a UTF-8 byte offset into a UTF-16 code-unit offset.
///
/// Offsets past the end clamp to the UTF-16 length; an offset inside a
/// multi-byte character snaps *down* to that character's start.
pub fn byte_to_utf16(text: &str, byte: usize) -> usize {
    utf16_len(&text[..floor_char_boundary(text, byte)])
}

/// UTF-16 length of `text` (JS `string.length`).
pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Largest char boundary `<= byte`, clamped to `text.len()`.
pub(crate) fn floor_char_boundary(text: &str, byte: usize) -> usize {
    if byte >= text.len() {
        return text.len();
    }
    let mut b = byte;
    while !text.is_char_boundary(b) {
        b -= 1;
    }
    b
}

/// JS `substring(start, end)` on byte offsets: both ends clamp to the text
/// (and snap down to a char boundary); a reversed pair is swapped.
pub(crate) fn js_substring(text: &str, start: usize, end: usize) -> &str {
    let a = floor_char_boundary(text, start);
    let b = floor_char_boundary(text, end);
    if a <= b { &text[a..b] } else { &text[b..a] }
}

/// JS `\s`: Unicode `White_Space` minus U+0085 (NEL), plus U+FEFF (BOM).
pub(crate) fn is_js_whitespace(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}'
}

/// JS `String#trim`: strips [`is_js_whitespace`] from both ends.
pub(crate) fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_whitespace)
}

/// Number of `'\n'`-separated lines (JS `text.split("\n").length`, ≥ 1).
pub(crate) fn line_count(text: &str) -> usize {
    text.bytes().filter(|&b| b == b'\n').count() + 1
}

/// JS `text.lastIndexOf("\n", from)`: the last newline at or before `from`
/// (clamped to the text). Callers pass `pos.saturating_sub(1)` for JS
/// `pos - 1`, matching JS's clamping of a negative `fromIndex` to 0.
pub(crate) fn last_newline_at_or_before(text: &str, from: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    bytes[..=from.min(bytes.len() - 1)].iter().rposition(|&b| b == b'\n')
}

/// JS `text.indexOf("\n", from)`: the first newline at or after `from`.
pub(crate) fn first_newline_from(text: &str, from: usize) -> Option<usize> {
    text.as_bytes().get(from..)?.iter().position(|&b| b == b'\n').map(|i| i + from)
}

pub mod clipboard;
pub mod cursor;
pub mod diff;
pub mod editor;
pub mod indent;
pub mod insertion;
pub mod markdown;
pub mod stats;
pub mod table;
pub mod task;
pub mod typewriter;
pub mod utils;
