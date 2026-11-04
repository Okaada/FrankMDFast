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
    todo!()
}

/// Convert a UTF-8 byte offset into a UTF-16 code-unit offset.
///
/// Offsets past the end clamp to the UTF-16 length; an offset inside a
/// multi-byte character snaps *down* to that character's start.
pub fn byte_to_utf16(text: &str, byte: usize) -> usize {
    todo!()
}

/// UTF-16 length of `text` (JS `string.length`).
pub fn utf16_len(text: &str) -> usize {
    todo!()
}

pub mod clipboard;
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
