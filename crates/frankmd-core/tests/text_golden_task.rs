//! Golden conformance: tests/fixtures/golden/js_task_utils.json (lib/task_utils.js).
#![allow(clippy::unwrap_used)]

#[path = "text_common/mod.rs"]
mod text_common;

use frankmd_core::text::task::{apply_task_toggle, task_marker_range, toggle_task_marker};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, golden};
use text_common::{arg_str, byte_to_u16, has_lone_surrogates};

const FIXTURE: &str = "js_task_utils";

fn check(got: Value, want: &Value) -> Result<(), String> {
    if &got == want { Ok(()) } else { Err(format!("got {got}, want {want}")) }
}

#[test]
fn golden_task_marker_range() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("taskMarkerRange"), |c| {
        assert!(!has_lone_surrogates(&c.input));
        let line = arg_str(&c.input, 0);
        let got = task_marker_range(&line).map_or(Value::Null, |r| {
            json!({"from": byte_to_u16(&line, r.from), "to": byte_to_u16(&line, r.to), "insert": r.insert})
        });
        check(got, &c.expected)
    });
}

#[test]
fn golden_toggle_task_marker() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("toggleTaskMarker"), |c| {
        assert!(!has_lone_surrogates(&c.input));
        check(json!(toggle_task_marker(&arg_str(&c.input, 0))), &c.expected)
    });
}

#[test]
fn golden_apply_task_toggle() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("applyTaskToggle"), |c| {
        assert!(!has_lone_surrogates(&c.input));
        check(json!(apply_task_toggle(&arg_str(&c.input, 0))), &c.expected)
    });
}
