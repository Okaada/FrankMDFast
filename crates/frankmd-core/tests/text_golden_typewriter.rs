//! Golden conformance: tests/fixtures/golden/js_typewriter_utils.json
//! (lib/typewriter_utils.js). Negative cursor offsets (JS `substring` clamps
//! them to 0) are unrepresentable in the `usize` API and skipped.
#![allow(clippy::unwrap_used)]

#[path = "text_common/mod.rs"]
mod text_common;

use std::cell::Cell;

use frankmd_core::text::typewriter::{
    DEFAULT_TARGET_RATIO, calculate_typewriter_scroll, calculate_typewriter_sync_ratio, get_typewriter_sync_data,
};
use frankmd_testkit::serde_json::json;
use frankmd_testkit::{assert_cases, golden};
use text_common::{arg, arg_str, has_lone_surrogates, u16_to_byte_floor};

const FIXTURE: &str = "js_typewriter_utils";

fn close(got: f64, want: f64) -> Result<(), String> {
    if (got - want).abs() < 1e-9 { Ok(()) } else { Err(format!("got {got}, want {want}")) }
}

#[test]
fn golden_calculate_typewriter_scroll() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("calculateTypewriterScroll"), |c| {
        let n = |i: usize| arg(&c.input, i).as_f64();
        let ratio = n(2).unwrap_or(DEFAULT_TARGET_RATIO);
        close(calculate_typewriter_scroll(n(0).unwrap(), n(1).unwrap(), ratio), c.expected.as_f64().unwrap())
    });
}

#[test]
fn golden_calculate_typewriter_sync_ratio() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("calculateTypewriterSyncRatio"), |c| {
        let n = |i: usize| usize::try_from(arg(&c.input, i).as_u64().unwrap()).unwrap();
        close(calculate_typewriter_sync_ratio(n(0), n(1)), c.expected.as_f64().unwrap())
    });
}

#[test]
fn golden_get_typewriter_sync_data() {
    let g = golden(FIXTURE);
    let skipped = Cell::new(0);
    assert_cases(FIXTURE, g.cases_for("getTypewriterSyncData"), |c| {
        assert!(!has_lone_surrogates(&c.input));
        let text = arg_str(&c.input, 0);
        let Some(pos) = arg(&c.input, 1).as_u64() else {
            skipped.set(skipped.get() + 1);
            return Ok(());
        };
        // Case #12 passes a JS offset inside an emoji's surrogate pair; snap down.
        let d = get_typewriter_sync_data(&text, u16_to_byte_floor(&text, usize::try_from(pos).unwrap()));
        let got = json!({"currentLine": d.current_line, "totalLines": d.total_lines});
        if got == c.expected { Ok(()) } else { Err(format!("got {got}, want {}", c.expected)) }
    });
    assert_eq!(skipped.get(), 1, "negative-offset cases");
}
