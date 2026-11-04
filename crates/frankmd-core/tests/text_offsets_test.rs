//! UTF-16 ⇄ UTF-8 offset conversion (`frankmd_core::text`). Not a port of a
//! single JS test: the JS code is UTF-16-native; these pin the conversion the
//! Rust port relies on (see brief: "be honest about the offset unit").

use frankmd_core::text::{byte_to_utf16, utf16_len, utf16_to_byte};

// "a😀é日\n" : a(1B,1u) 😀(4B,2u) é(2B,1u) 日(3B,1u) \n(1B,1u)
const S: &str = "a😀é日\n";

#[test]
fn utf16_to_byte_maps_char_boundaries() {
    let expect = [(0, 0), (1, 1), (3, 5), (4, 7), (5, 10), (6, 11)];
    for (u, b) in expect {
        assert_eq!(utf16_to_byte(S, u), b, "utf16 {u}");
    }
}

#[test]
fn byte_to_utf16_maps_char_boundaries() {
    let expect = [(0, 0), (1, 1), (5, 3), (7, 4), (10, 5), (11, 6)];
    for (b, u) in expect {
        assert_eq!(byte_to_utf16(S, b), u, "byte {b}");
    }
}

#[test]
fn utf16_to_byte_clamps_past_end() {
    assert_eq!(utf16_to_byte(S, 7), S.len());
    assert_eq!(utf16_to_byte(S, 1000), S.len());
    assert_eq!(utf16_to_byte("", 3), 0);
}

#[test]
fn byte_to_utf16_clamps_past_end() {
    assert_eq!(byte_to_utf16(S, 1000), 6);
    assert_eq!(byte_to_utf16("", 3), 0);
}

#[test]
fn utf16_inside_surrogate_pair_snaps_down() {
    // utf16 2 is between the halves of 😀 (starts at byte 1).
    assert_eq!(utf16_to_byte(S, 2), 1);
}

#[test]
fn byte_inside_multibyte_char_snaps_down() {
    for b in 2..5 {
        assert_eq!(byte_to_utf16(S, b), 1, "byte {b} inside 😀");
    }
    assert_eq!(byte_to_utf16(S, 6), 3, "inside é");
    assert_eq!(byte_to_utf16(S, 8), 4, "inside 日");
}

#[test]
fn utf16_len_matches_js_length() {
    assert_eq!(utf16_len(""), 0);
    assert_eq!(utf16_len("abc"), 3);
    assert_eq!(utf16_len(S), 6);
    assert_eq!(utf16_len("👨‍👩‍👧"), 8);
}

#[test]
fn round_trip_on_every_char_boundary() {
    let s = "x👍🏽 ção\r\n𝄞end";
    for (b, _) in s.char_indices().chain([(s.len(), ' ')]) {
        let u = byte_to_utf16(s, b);
        assert_eq!(utf16_to_byte(s, u), b, "byte {b}");
    }
}
