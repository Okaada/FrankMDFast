//! Conformance: `tests/fixtures/golden/js_scroll_utils.json`
//! (`scroll_utils.js`) → [`linemap::scroll`]. `f64` results are compared
//! exactly (same IEEE operations as V8); NaN equals NaN; the sign of zero is
//! not compared.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::linemap::scroll::{self, ScrollEntry};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};

const FIXTURE: &str = "js_scroll_utils";

/// JS number from a decoded fixture value (`undefined` → NaN, as in JS arithmetic).
fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap(),
        Value::String(s) => match s.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            "-0" => -0.0,
            other => panic!("not a number: {other}"),
        },
        _ => f64::NAN,
    }
}

fn n(case: &Case, i: usize) -> f64 {
    num(&arg(case, i))
}

fn same(got: f64, want: &Value) -> Result<(), String> {
    let w = num(want);
    if (got.is_nan() && w.is_nan()) || got == w {
        Ok(())
    } else {
        Err(format!("got {got}, expected {w}"))
    }
}

fn run(fn_name: &str, f: impl Fn(&Case) -> f64) {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for(fn_name), |case| {
        same(f(case), &expected(case))
    });
}

#[test]
fn golden_calculate_scroll_ratio() {
    run("calculateScrollRatio", |c| {
        scroll::scroll_ratio(n(c, 0), n(c, 1), n(c, 2))
    });
}

#[test]
fn golden_calculate_scroll_for_line() {
    run("calculateScrollForLine", |c| {
        let ratio = match arg(c, 3) {
            Value::Null => scroll::DEFAULT_LINE_OFFSET_RATIO,
            v => num(&v),
        };
        scroll::scroll_for_line(n(c, 0), n(c, 1), n(c, 2), ratio)
    });
}

#[test]
fn golden_calculate_line_from_scroll() {
    run("calculateLineFromScroll", |c| {
        scroll::line_from_scroll(n(c, 0), n(c, 1), n(c, 2), n(c, 3))
    });
}

#[test]
fn golden_calculate_synced_scroll_position() {
    run("calculateSyncedScrollPosition", |c| {
        scroll::synced_scroll_position(n(c, 0), n(c, 1), n(c, 2))
    });
}

#[test]
fn golden_calculate_scroll_to_center_line() {
    run("calculateScrollToCenterLine", |c| {
        scroll::scroll_to_center_line(n(c, 0), n(c, 1), n(c, 2), n(c, 3))
    });
}

#[test]
fn golden_scroll_top_for_element() {
    run("scrollTopForElement", |c| {
        scroll::scroll_top_for_element(num(&arg(c, 0)["top"]), num(&arg(c, 1)["top"]), n(c, 2))
    });
}

#[test]
fn golden_line_at_scroll() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("lineAtScroll"), |case| {
        // JS `null` entries behave like `[]`.
        let entries: Vec<ScrollEntry> = arg(case, 0)
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|e| ScrollEntry {
                        line: e["line"].as_u64().unwrap() as u32,
                        top: num(&e["top"]),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let got = scroll::line_at_scroll(&entries, n(case, 1)).map_or(Value::Null, |l| json!(l));
        let want = expected(case);
        if got == want {
            Ok(())
        } else {
            Err(format!("got {got}, expected {want}"))
        }
    });
}
