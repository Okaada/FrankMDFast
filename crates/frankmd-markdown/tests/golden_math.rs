//! Conformance: `tests/fixtures/golden/js_math_renderer.json` — KaTeX 0.19
//! with FrankMD's options over `.math-inline`/`.math-block` placeholders.
//!
//! KaTeX's HTML is not reproduced (RaTeX paints natively). Each placeholder's
//! outcome is classified from KaTeX's output — clean, `katex-error` (parse
//! error / too many expansions), or rendered with red unknown/untrusted
//! commands — and must equal [`math::validate`] for the same TeX and style.
//! Placeholders already carrying `data-math-rendered` are skipped (as
//! `renderMathIn` does); the `null container` case has no Rust equivalent.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::math::{MathStyle, validate};
use frankmd_testkit::{assert_cases, golden};
use scraper::{Html, Selector};

const FIXTURE: &str = "js_math_renderer";

#[test]
fn golden_render_math_in() {
    let g = golden(FIXTURE);
    let cases = g
        .cases_for("renderMathIn")
        .filter(|c| !c.input["html"].is_null() && !is_known_divergence(FIXTURE, &c.name));
    let sel = Selector::parse(".math-inline, .math-block").unwrap();
    assert_cases(FIXTURE, cases, |case| {
        let before = Html::parse_fragment(case.input["html"].as_str().unwrap());
        let want = expected(case);
        let after = Html::parse_fragment(want.as_str().unwrap());
        let mut errs = Vec::new();
        for (b, a) in before.select(&sel).zip(after.select(&sel)) {
            if b.value().attr("data-math-rendered").is_some() {
                continue;
            }
            let style = if b.value().classes().any(|c| c == "math-block") {
                MathStyle::Display
            } else {
                MathStyle::Inline
            };
            let tex: String = b.text().collect();
            let verdict = katex_verdict(a);
            let got = validate(&tex, style);
            if !verdict.matches(&got) {
                errs.push(format!(
                    "{style:?} {tex:?}: validate {got:?}, KaTeX {verdict:?}"
                ));
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("; "))
        }
    });
}
