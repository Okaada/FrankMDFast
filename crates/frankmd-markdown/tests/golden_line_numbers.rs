//! Conformance: `tests/fixtures/golden/js_line_numbers.json`
//! (`line_numbers.js`) → [`line_numbers`].
//!
//! `normalizeLineNumberMode(value, fallback)` takes any JS value; the Rust API
//! takes its string form (what `.fed` stores), so each fixture value is
//! converted with JS `String(value)` semantics (`parseInt` stringifies first).
//! Cases whose fallback is not a valid mode only check results that parsed.
//! `nextLineNumberMode` on non-mode values (3, -1, null, "1" → OFF by strict
//! equality) is unrepresentable with the typed enum and is not driven.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::line_numbers::{LineNumberMode, normalize_line_number_mode};
use frankmd_testkit::serde_json::Value;
use frankmd_testkit::{assert_cases, golden};

const FIXTURE: &str = "js_line_numbers";

/// JS `String(v)` for the values the fixture uses (`undefined` → `None`).
fn js_string(v: &Value) -> Option<String> {
    match v {
        Value::Null => None, // both `undefined` and `null`: see below
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(if let Some(i) = n.as_i64() { i.to_string() } else { n.as_f64().unwrap().to_string() }),
        other => Some(other.to_string()),
    }
}

fn mode_of(v: &Value) -> Option<LineNumberMode> {
    v.as_i64().and_then(LineNumberMode::from_value)
}

#[test]
fn golden_normalize_line_number_mode() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("normalizeLineNumberMode"), |case| {
        // `null` stringifies to "null" (NaN) and `undefined` is skipped by
        // parseInt too: both fall back, which `None` expresses.
        let raw = case.input.get(0).cloned().unwrap_or(Value::Null);
        let value = match &raw {
            Value::Object(o) if o.contains_key("$number") => Some(o["$number"].as_str().unwrap().to_owned()),
            _ => js_string(&arg(case, 0)),
        };
        let fallback_v = arg(case, 1);
        let fallback = if fallback_v.is_null() { Some(LineNumberMode::Absolute) } else { mode_of(&fallback_v) };
        let want = mode_of(&expected(case)).unwrap();
        let Some(fallback) = fallback else {
            // Non-mode fallback ("off"): only meaningful when the value parsed.
            let got = normalize_line_number_mode(value.as_deref(), LineNumberMode::Absolute);
            return if got == want { Ok(()) } else { Err(format!("got {got:?}, expected {want:?}")) };
        };
        let got = normalize_line_number_mode(value.as_deref(), fallback);
        if got == want { Ok(()) } else { Err(format!("({value:?}, {fallback:?}) -> {got:?}, expected {want:?}")) }
    });
}

#[test]
fn golden_next_line_number_mode() {
    let g = golden(FIXTURE);
    let cases = g.cases_for("nextLineNumberMode").filter(|c| mode_of(&arg(c, 0)).is_some());
    assert_cases(FIXTURE, cases, |case| {
        let got = mode_of(&arg(case, 0)).unwrap().next();
        let want = mode_of(&expected(case)).unwrap();
        if got == want { Ok(()) } else { Err(format!("got {got:?}, expected {want:?}")) }
    });
}
