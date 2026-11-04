//! Conformance: `tests/fixtures/golden/js_preview_pipeline.json` — the full
//! FrankMD preview render (`preview_controller.render`): stripFrontmatter →
//! parseWithLineNumbers (marked + line injection + DOMPurify) → code
//! highlighting → KaTeX.
//!
//! Per `previewRender` case we check, against the IR from [`render`]:
//! 1. `total_lines` == `totalSourceLines`;
//! 2. `html::to_html` ≅ `sanitized_html` (normalized, `data-source-line` dropped);
//! 3. line semantics: top-level block lines and `<li>` lines == the original's
//!    `data-source-line`s, corrected for FrankMD's front-matter off-by-one
//!    (the IR carries true lines) and skipped where the original's sequential
//!    slot assignment is visibly shifted (see `common::ExpectedAnchors`);
//! 4. code blocks: `data-code-language` label and "was highlighted" from `final_html`;
//! 5. math: placeholders' TeX and KaTeX's verdict (ok / parse error / red
//!    unsupported commands) == `math::validate`.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;

use common::*;
use frankmd_markdown::frontmatter;
use frankmd_markdown::highlight::{BuiltinHighlighter, language_for_fence};
use frankmd_markdown::html::to_html;
use frankmd_markdown::math::validate;
use frankmd_markdown::{RenderOptions, render};
use frankmd_testkit::{assert_cases, golden};

const FIXTURE: &str = "js_preview_pipeline";

fn opts() -> RenderOptions {
    RenderOptions {
        highlighter: Some(Arc::new(BuiltinHighlighter)),
        ..RenderOptions::default()
    }
}

#[test]
fn golden_strip_frontmatter() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("stripFrontmatter"), |case| {
        let src = arg_str(case, 0);
        let exp = expected(case);
        let got = frontmatter::strip(&src);
        let want_content = exp["content"].as_str().unwrap_or("");
        let want_lines = exp["frontmatterLines"].as_u64().unwrap();
        if got.content == want_content && u64::from(got.frontmatter_lines) == want_lines {
            Ok(())
        } else {
            Err(format!(
                "got ({:?}, {}), expected ({want_content:?}, {want_lines})",
                got.content, got.frontmatter_lines
            ))
        }
    });
}

#[test]
fn golden_preview_render() {
    let g = golden(FIXTURE);
    let cases = g
        .cases_for("previewRender")
        .filter(|c| !is_known_divergence(FIXTURE, &c.name));
    assert_cases(FIXTURE, cases, |case| {
        let src = arg_str(case, 0);
        let exp = expected(case);
        let sanitized = exp["sanitized_html"].as_str().unwrap();
        let final_html = exp["final_html"].as_str().unwrap();
        let fm = &exp["frontmatter"];
        let shift = frontmatter_line_shift(
            &src,
            fm["content"].as_str().unwrap_or(""),
            fm["frontmatterLines"].as_i64().unwrap(),
        );

        let doc = render(&src, &opts());
        let mut errs = Vec::new();

        // 1. total lines
        let want_total = exp["totalSourceLines"].as_u64().unwrap();
        if u64::from(doc.total_lines) != want_total {
            errs.push(format!("total_lines {} != {want_total}", doc.total_lines));
        }
        // 2. structure
        if let Err(e) = check_same_structure(&to_html(&doc), sanitized) {
            errs.push(e);
        }
        // 3. line semantics
        if let Err(e) = check_anchors(&doc, sanitized, shift) {
            errs.push(e);
        }
        // 4. code labels + highlight decision
        let want_code = html_code_blocks(final_html);
        let got_code = collect_code(&doc);
        let got_classed: Vec<_> = got_code.iter().filter(|(l, _)| l.is_some()).collect();
        if want_code.len() != got_classed.len() {
            errs.push(format!(
                "{} classed code blocks, expected {}",
                got_classed.len(),
                want_code.len()
            ));
        } else {
            for (i, (want, (lang, lines))) in want_code.iter().zip(&got_classed).enumerate() {
                let label = lang
                    .as_deref()
                    .and_then(language_for_fence)
                    .map(|l| l.label.to_owned());
                if label != want.label {
                    errs.push(format!(
                        "code #{i}: label {label:?}, expected {:?}",
                        want.label
                    ));
                }
                if want.highlighted() && !is_highlighted(lines) {
                    errs.push(format!(
                        "code #{i} ({:?}): expected token highlighting",
                        want.lang
                    ));
                }
                if want.label.is_none() && is_highlighted(lines) {
                    errs.push(format!(
                        "code #{i} ({:?}): highlighted an unsupported language",
                        want.lang
                    ));
                }
            }
        }
        // 5. math
        let want_math = katex_verdicts(sanitized, final_html);
        let got_math = collect_math(&doc);
        if want_math.len() != got_math.len() {
            errs.push(format!(
                "{} math nodes, expected {}",
                got_math.len(),
                want_math.len()
            ));
        } else {
            for ((ws, wt, wv), (gs, gt)) in want_math.iter().zip(&got_math) {
                if ws != gs || wt != gt {
                    errs.push(format!("math {gs:?} {gt:?}, expected {ws:?} {wt:?}"));
                } else if !wv.matches(&validate(gt, *gs)) {
                    errs.push(format!(
                        "math {gt:?}: validate {:?}, KaTeX {wv:?}",
                        validate(gt, *gs)
                    ));
                }
            }
        }

        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("\n    "))
        }
    });
}
