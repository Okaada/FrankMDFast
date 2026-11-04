//! Golden conformance: `tests/fixtures/golden/backlinks.json`
//! (`BacklinksService#find_backlinks`, via `NotesService::backlinks`).
//!
//! The original returns results in readdir order; the fixture (and this test)
//! sort by `(path, line_number)`. `context_lines` defaults to 3. `"http"`
//! cases live in `golden_http.rs`.
//!
//! **[QUIRK]** "invalid UTF-8 source file raises": Rails raised
//! `ArgumentError: invalid byte sequence in UTF-8` from the regex match. Only
//! "an error is returned" is asserted here; the Rust error kind is not pinned.
#![allow(clippy::unwrap_used)]

mod common;
mod support_d;

use common::Sandbox;
use frankmd_fs::NotesService;
use frankmd_testkit::serde_json::{self, Value};
use frankmd_testkit::{Case, assert_cases, golden};

fn run_case(case: &Case) -> Result<(), String> {
    let input = &case.input;
    let sb = Sandbox::from_specs(&input["tree"], &input["outside"], None);
    let vault = sb.vault();
    let svc = NotesService::new(&vault);
    let target = input["target"].as_str().unwrap();
    let ctx = input["options"]["context_lines"].as_u64().unwrap_or(3) as usize;
    match (svc.backlinks(target, ctx), &case.expected) {
        (Ok(mut hits), exp @ Value::Array(_)) => {
            hits.sort_by(|a, b| (a.path.as_str(), a.line_number).cmp(&(b.path.as_str(), b.line_number)));
            common::check_eq("backlinks", &serde_json::to_value(&hits).unwrap(), exp)
        }
        (Err(e), exp) if exp.get("error").is_some() => {
            if exp["message"] == "invalid byte sequence in UTF-8" {
                return Ok(());
            }
            common::check_eq("error", &common::fs_error_json(&e), exp)
        }
        (Ok(hits), exp) => Err(format!("expected {exp}, got {} results", hits.len())),
        (Err(e), exp) => Err(format!("expected {exp}, got error {e} ({})", e.compat_class())),
    }
}

#[test]
fn golden_backlinks() {
    let g = golden("backlinks");
    assert_cases("backlinks", g.cases_for("BacklinksService#find_backlinks"), run_case);
}
