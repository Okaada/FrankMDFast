//! Golden conformance: tests/fixtures/golden/js_diff_utils.json (lib/diff_utils.js).
#![allow(clippy::unwrap_used)]

#[path = "text_common/mod.rs"]
mod text_common;

use frankmd_core::text::diff::{DiffKind, DiffOp, compute_word_diff, lcs_word_diff, tokenize};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, golden};
use text_common::{arg, arg_str, has_lone_surrogates};

const FIXTURE: &str = "js_diff_utils";

fn encode(ops: &[DiffOp]) -> Value {
    Value::Array(
        ops.iter()
            .map(|o| {
                let kind = match o.kind {
                    DiffKind::Equal => "equal",
                    DiffKind::Insert => "insert",
                    DiffKind::Delete => "delete",
                };
                json!({"type": kind, "value": o.value})
            })
            .collect(),
    )
}

fn check_eq(got: Value, want: &Value) -> Result<(), String> {
    if &got == want { Ok(()) } else { Err(format!("got {got}, want {want}")) }
}

#[test]
fn golden_tokenize() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("tokenize"), |c| {
        assert!(!has_lone_surrogates(&c.input));
        let s = arg_str(&c.input, 0);
        check_eq(json!(tokenize(&s)), &c.expected)
    });
}

#[test]
fn golden_compute_word_diff() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("computeWordDiff"), |c| {
        assert!(!has_lone_surrogates(&c.input));
        let (a, b) = (arg_str(&c.input, 0), arg_str(&c.input, 1));
        check_eq(encode(&compute_word_diff(&a, &b)), &c.expected)
    });
}

#[test]
fn golden_lcs_word_diff() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("lcsWordDiff"), |c| {
        let toks = |i: usize| -> Vec<String> {
            arg(&c.input, i).as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_owned()).collect()
        };
        let (old, new) = (toks(0), toks(1));
        let old: Vec<&str> = old.iter().map(String::as_str).collect();
        let new: Vec<&str> = new.iter().map(String::as_str).collect();
        check_eq(encode(&lcs_word_diff(&old, &new)), &c.expected)
    });
}
