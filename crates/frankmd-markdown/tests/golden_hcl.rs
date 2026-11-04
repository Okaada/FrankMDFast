//! Conformance: `tests/fixtures/golden/js_terraform_hcl_stream_parser.json` —
//! FrankMD's Terraform/HCL StreamLanguage tokenizer driven line by line
//! (empty line → `blankLine`, no tokens) → [`highlight::hcl`].
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::highlight::hcl::{HclState, HclStyle, blank_line, tokenize_line};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, golden};

const FIXTURE: &str = "js_terraform_hcl_stream_parser";

fn style_name(s: HclStyle) -> &'static str {
    match s {
        HclStyle::Keyword => "keyword",
        HclStyle::String => "string",
        HclStyle::Comment => "comment",
        HclStyle::Atom => "atom",
        HclStyle::Number => "number",
        HclStyle::Function => "variableName.function",
        HclStyle::Property => "property",
        HclStyle::Variable => "variable",
        HclStyle::Operator => "operator",
        HclStyle::Punctuation => "punctuation",
    }
}

fn state_json(s: &HclState) -> Value {
    json!({"blockComment": s.block_comment, "heredocMarker": s.heredoc_marker, "afterDot": s.after_dot})
}

fn state_from(v: &Value) -> HclState {
    HclState {
        block_comment: v["blockComment"].as_bool().unwrap(),
        heredoc_marker: v["heredocMarker"].as_str().map(str::to_owned),
        after_dot: v["afterDot"].as_bool().unwrap(),
    }
}

#[test]
fn golden_hcl_tokenize() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("tokenize"), |case| {
        let src = arg_str(case, 0);
        let mut state = HclState::default();
        let mut lines = Vec::new();
        for line in src.split('\n') {
            if line.is_empty() {
                blank_line(&mut state);
                lines.push(json!([]));
            } else {
                let toks: Vec<Value> = tokenize_line(line, &mut state)
                    .into_iter()
                    .map(|(t, s)| json!([t, s.map(style_name)]))
                    .collect();
                lines.push(Value::Array(toks));
            }
        }
        let got = json!({"lines": lines, "endState": state_json(&state)});
        let want = expected(case);
        if got == want { Ok(()) } else { Err(format!("\n    got:      {got}\n    expected: {want}")) }
    });
}

#[test]
fn golden_hcl_states() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("startState"), |case| {
        let got = state_json(&HclState::default());
        if got == expected(case) { Ok(()) } else { Err(format!("got {got}")) }
    });
    assert_cases(FIXTURE, g.cases_for("copyState"), |case| {
        let original = state_from(&arg(case, 0));
        let got = state_json(&original.clone());
        if got == expected(case) { Ok(()) } else { Err(format!("got {got}")) }
    });
}
