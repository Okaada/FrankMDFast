//! Helpers shared by the text_*/guard_*/emoji_* integration tests.
//! Include with `#[path = "text_common/mod.rs"] mod text_common;`.
//!
//! The Rust text API works in UTF-8 byte offsets; the JS fixtures record UTF-16
//! code-unit offsets. These helpers convert independently of the crate under
//! test so a failing case points at the function being ported, not at the
//! conversion.
#![allow(dead_code, unreachable_pub, clippy::unwrap_used)]

use frankmd_testkit::serde_json::Value;

/// UTF-16 code-unit offset -> UTF-8 byte offset (clamped to `s.len()`).
/// Panics if the offset falls inside a surrogate pair.
pub fn u16_to_byte(s: &str, u16_off: usize) -> usize {
    let mut units = 0usize;
    for (b, ch) in s.char_indices() {
        if units == u16_off {
            return b;
        }
        assert!(
            units < u16_off,
            "utf16 offset {u16_off} splits a surrogate pair in {s:?}"
        );
        units += ch.len_utf16();
    }
    assert!(
        units <= u16_off,
        "utf16 offset {u16_off} splits a surrogate pair in {s:?}"
    );
    s.len()
}

/// UTF-8 byte offset -> UTF-16 code-unit offset.
pub fn byte_to_u16(s: &str, byte: usize) -> usize {
    s[..byte.min(s.len())].encode_utf16().count()
}

/// UTF-16 length of a string (JS `.length`).
pub fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// True if the raw (undecoded) fixture value contains a `$utf16` ill-formed string,
/// which a Rust `&str` cannot represent. Such cases are skipped and counted.
pub fn has_lone_surrogates(v: &Value) -> bool {
    match v {
        Value::Object(o) => o.contains_key("$utf16") || o.values().any(has_lone_surrogates),
        Value::Array(a) => a.iter().any(has_lone_surrogates),
        _ => false,
    }
}

/// Decoded positional argument `i` of a JS fixture case (`input` is an argument array).
pub fn arg(input: &Value, i: usize) -> Value {
    frankmd_testkit::decode_js(input.get(i).unwrap_or(&Value::Null))
}

/// `arg` as `&str`-owned String (panics if not a string).
pub fn arg_str(input: &Value, i: usize) -> String {
    arg(input, i)
        .as_str()
        .unwrap_or_else(|| panic!("arg {i} not a string: {input}"))
        .to_owned()
}

/// Like [`u16_to_byte`] but an offset inside a surrogate pair snaps DOWN to the
/// start of that character (JS lets callers pass such offsets; e.g.
/// `getTypewriterSyncData("日本\n👋\nx", 4)`). Line-based results are unaffected.
pub fn u16_to_byte_floor(s: &str, u16_off: usize) -> usize {
    let mut units = 0usize;
    for (b, ch) in s.char_indices() {
        units += ch.len_utf16();
        if units > u16_off {
            return b;
        }
    }
    s.len()
}
