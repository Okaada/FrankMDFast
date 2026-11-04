//! JavaScript `encodeURIComponent` / `decodeURIComponent` for storage keys.

/// Characters `encodeURIComponent` leaves alone.
fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
        )
}

/// `encodeURIComponent(s)`: percent-encode every UTF-8 byte outside the
/// unreserved set, with uppercase hex digits.
pub(crate) fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if is_unreserved(b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `decodeURIComponent(s)`; `None` where the original throws `URIError`
/// (a `%` not followed by two hex digits, or escapes that are not valid UTF-8).
pub(crate) fn decode_uri_component(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    // Literal characters are valid UTF-8 on their own, so the whole buffer is
    // valid exactly when every escaped sequence is (JS rejects the same inputs).
    String::from_utf8(out).ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_bad_escapes() {
        assert_eq!(encode_uri_component("a b/ç!~*'()"), "a%20b%2F%C3%A7!~*'()");
        assert_eq!(decode_uri_component("a%20b%2F%C3%A7").unwrap(), "a b/ç");
        assert_eq!(decode_uri_component("%"), None);
        assert_eq!(decode_uri_component("%E0%A4%A"), None);
        assert_eq!(decode_uri_component("%C3é"), None);
        assert_eq!(decode_uri_component("%ED%A0%80"), None);
    }
}
