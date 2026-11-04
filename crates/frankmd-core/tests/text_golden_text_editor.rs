//! Golden conformance: tests/fixtures/golden/js_text_editor_utils.json
//! (lib/text_editor_utils.js).
//!
//! Offsets: fixture UTF-16 offsets are converted to UTF-8 bytes. JS does not
//! clamp the cursor arithmetic, so offsets *past the end* are carried over as
//! "virtual" units (one past-the-end UTF-16 unit == one past-the-end byte) in
//! both directions. Cases with negative offsets are unrepresentable in the
//! `usize` API and skipped (counted below).
#![allow(clippy::unwrap_used)]

#[path = "text_common/mod.rs"]
mod text_common;

use std::cell::Cell;

use frankmd_core::text::editor::{
    get_cursor_info, get_line_at_position, get_line_boundaries, get_position_for_line,
    insert_at_position, replace_range, wrap_selection,
};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};
use text_common::{arg, arg_str, has_lone_surrogates, js_len, u16_to_byte};

const FIXTURE: &str = "js_text_editor_utils";

/// UTF-16 offset → byte offset, extending linearly past the end.
fn to_byte(s: &str, off: i64) -> usize {
    let off = usize::try_from(off).unwrap();
    let len16 = js_len(s);
    if off <= len16 {
        u16_to_byte(s, off)
    } else {
        s.len() + (off - len16)
    }
}

/// Byte offset → UTF-16 offset, extending linearly past the end. A byte offset
/// inside a character counts that character as passed.
fn to_u16(s: &str, b: usize) -> usize {
    if b > s.len() {
        return js_len(s) + (b - s.len());
    }
    s.char_indices()
        .take_while(|(i, _)| *i < b)
        .map(|(_, ch)| ch.len_utf16())
        .sum()
}

fn num(c: &Case, i: usize) -> i64 {
    arg(&c.input, i).as_i64().unwrap()
}

fn check(got: Value, want: &Value) -> Result<(), String> {
    if &got == want {
        Ok(())
    } else {
        Err(format!("got {got}, want {want}"))
    }
}

/// Runs `f` for every case of `func`, skipping cases with a negative numeric arg.
fn run(func: &str, expected_skips: usize, mut f: impl FnMut(&Case) -> Result<(), String>) {
    let g = golden(FIXTURE);
    let skipped = Cell::new(0);
    assert_cases(FIXTURE, g.cases_for(func), |c| {
        assert!(
            !has_lone_surrogates(&c.input),
            "lone surrogates not expected in this fixture"
        );
        let negative = c
            .input
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_i64().is_some_and(|n| n < 0));
        if negative {
            skipped.set(skipped.get() + 1);
            return Ok(());
        }
        f(c)
    });
    assert_eq!(
        skipped.get(),
        expected_skips,
        "{func}: skipped negative-offset cases"
    );
}

#[test]
fn golden_wrap_selection() {
    run("wrapSelection", 0, |c| {
        let text = arg_str(&c.input, 0);
        let (prefix, suffix) = (arg_str(&c.input, 3), arg_str(&c.input, 4));
        let r = wrap_selection(
            &text,
            to_byte(&text, num(c, 1)),
            to_byte(&text, num(c, 2)),
            &prefix,
            &suffix,
        );
        let got = json!({
            "text": r.text,
            "cursorStart": to_u16(&r.text, r.cursor_start),
            "cursorEnd": to_u16(&r.text, r.cursor_end),
        });
        check(got, &c.expected)
    });
}

#[test]
fn golden_insert_at_position() {
    run("insertAtPosition", 1, |c| {
        let text = arg_str(&c.input, 0);
        let r = insert_at_position(&text, to_byte(&text, num(c, 1)), &arg_str(&c.input, 2));
        check(
            json!({"text": r.text, "cursorPosition": to_u16(&r.text, r.cursor)}),
            &c.expected,
        )
    });
}

#[test]
fn golden_replace_range() {
    run("replaceRange", 0, |c| {
        let text = arg_str(&c.input, 0);
        let r = replace_range(
            &text,
            to_byte(&text, num(c, 1)),
            to_byte(&text, num(c, 2)),
            &arg_str(&c.input, 3),
        );
        check(
            json!({"text": r.text, "cursorPosition": to_u16(&r.text, r.cursor)}),
            &c.expected,
        )
    });
}

#[test]
fn golden_get_line_boundaries() {
    run("getLineBoundaries", 0, |c| {
        let text = arg_str(&c.input, 0);
        let r = get_line_boundaries(&text, to_byte(&text, num(c, 1)), to_byte(&text, num(c, 2)));
        let got = json!({
            "lineStart": to_u16(&text, r.line_start),
            "lineEnd": to_u16(&text, r.line_end),
            "lines": r.lines,
        });
        check(got, &c.expected)
    });
}

#[test]
fn golden_get_cursor_info() {
    run("getCursorInfo", 0, |c| {
        let text = arg_str(&c.input, 0);
        let r = get_cursor_info(&text, to_byte(&text, num(c, 1)));
        check(
            json!({"line": r.line, "column": r.column_utf16, "totalLines": r.total_lines}),
            &c.expected,
        )
    });
}

#[test]
fn golden_get_line_at_position() {
    run("getLineAtPosition", 0, |c| {
        let text = arg_str(&c.input, 0);
        check(
            json!(get_line_at_position(&text, to_byte(&text, num(c, 1)))),
            &c.expected,
        )
    });
}

#[test]
fn golden_get_position_for_line() {
    run("getPositionForLine", 1, |c| {
        let text = arg_str(&c.input, 0);
        let line = usize::try_from(num(c, 1)).unwrap();
        check(
            json!(to_u16(&text, get_position_for_line(&text, line))),
            &c.expected,
        )
    });
}
