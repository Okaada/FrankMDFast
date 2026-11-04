//! Conformance: `tests/fixtures/golden/js_html_sanitizer.json` — FrankMD's
//! `sanitizeHtml` (DOMPurify 3.4.17 + `html_sanitizer.js` policy) over every
//! harvested call plus an XSS/edge corpus → [`html::sanitize`], compared after
//! `common::normalize_html` (all attributes kept, incl. `data-source-line`).
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::html::sanitize;
use frankmd_testkit::{assert_cases, golden};

const FIXTURE: &str = "js_html_sanitizer";

#[test]
fn golden_sanitize_html() {
    let g = golden(FIXTURE);
    let cases = g
        .cases_for("sanitizeHtml")
        .filter(|c| !is_known_divergence(FIXTURE, &c.name));
    assert_cases(FIXTURE, cases, |case| {
        // Non-string falsy input (null) returns "" in JS; "" is the Rust spelling.
        let input = arg_str(case, 0);
        let want = expected(case);
        let want = want.as_str().unwrap();
        let got = sanitize(&input);
        if want.is_empty() {
            return if got.is_empty() {
                Ok(())
            } else {
                Err(format!("expected \"\", got {got:?}"))
            };
        }
        let (a, e) = (normalize_html(&got, &[]), normalize_html(want, &[]));
        if a == e {
            Ok(())
        } else {
            Err(format!("\n    actual:   {a}\n    expected: {e}"))
        }
    });
}
