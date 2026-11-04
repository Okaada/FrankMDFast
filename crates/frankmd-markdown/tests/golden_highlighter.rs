//! Conformance: `tests/fixtures/golden/js_code_block_highlighter.json` —
//! FrankMD's preview code highlighting over sanitized HTML (vendored Lezer).
//!
//! The native preview highlights IR code blocks, not HTML, so for every
//! `pre > code[class]` block of a case we check:
//! - the label: `data-code-language` == [`plan`]'s language label (blocks the
//!   plan does not inspect keep whatever label they had);
//! - the budget decision: original highlighted ⇒ plan highlights, plan does
//!   not highlight ⇒ original has no token spans (one-directional because the
//!   shipped Dockerfile parser failed to load — see the fixture notes; we
//!   highlight Dockerfile);
//! - the code text is preserved.
//!
//! Token-exact parity with Lezer is measured by the ignored
//! `golden_tokens_strict` (not a goal: the app uses tree-sitter/syntect, and
//! the role-level contracts live in `code_block_highlighter_test.rs`).
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::highlight::{BuiltinHighlighter, CodeHighlighter, HighlightBudget, language_for_fence, plan};
use frankmd_testkit::{Case, assert_cases, golden};

const FIXTURE: &str = "js_code_block_highlighter";

fn check_blocks(input_html: &str, output_html: &str) -> Result<(), String> {
    let before = html_code_blocks(input_html);
    let after = html_code_blocks(output_html);
    if before.len() != after.len() {
        return Err(format!("{} code blocks before, {} after", before.len(), after.len()));
    }
    let lens: Vec<(&str, usize)> = before.iter().map(|b| (b.lang.as_str(), utf16_len(&b.text))).collect();
    let planned = plan(&lens, &HighlightBudget::default());
    if planned.len() != before.len() {
        return Err(format!("plan returned {} entries for {} blocks", planned.len(), before.len()));
    }
    let mut errs = Vec::new();
    for (i, ((b, a), p)) in before.iter().zip(&after).zip(&planned).enumerate() {
        let want_label = p.language.map(|l| l.label.to_owned()).or_else(|| b.label.clone());
        if a.label != want_label {
            errs.push(format!("#{i} {:?}: label {:?}, plan gives {want_label:?}", b.lang, a.label));
        }
        if a.highlighted() && !p.highlight {
            errs.push(format!("#{i} {:?}: original highlighted, plan does not", b.lang));
        }
        if a.text != b.text {
            errs.push(format!("#{i}: text changed {:?} -> {:?}", b.text, a.text));
        }
    }
    if errs.is_empty() { Ok(()) } else { Err(errs.join("; ")) }
}

#[test]
fn golden_highlight_code_blocks() {
    let g = golden(FIXTURE);
    // `misc "null"` (null in → null out) is a JS falsy guard with no Rust equivalent.
    let cases = g
        .cases_for("highlightCodeBlocks")
        .filter(|c| c.input[0].is_string() && !is_known_divergence(FIXTURE, &c.name));
    assert_cases(FIXTURE, cases, |case| {
        let want = expected(case);
        check_blocks(&arg_str(case, 0), want.as_str().unwrap())
    });
}

#[test]
fn golden_highlight_code_blocks_in_element() {
    let g = golden(FIXTURE);
    // `null root` → `false` is a DOM guard with no Rust equivalent.
    let cases = g.cases_for("highlightCodeBlocksInElement").filter(|c| c.input["html"].is_string());
    assert_cases(FIXTURE, cases, |case| {
        let want = expected(case);
        check_blocks(case.input["html"].as_str().unwrap(), want["html"].as_str().unwrap())
    });
}

#[test]
fn golden_language_labels() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("LANGUAGE_PARSERS(labels)"), |case: &Case| {
        let want = expected(case);
        let mut errs = Vec::new();
        for (alias, label) in want.as_object().unwrap() {
            let got = language_for_fence(alias).map(|l| l.label);
            if got != label.as_str() {
                errs.push(format!("{alias}: {got:?} != {label}"));
            }
            let upper = alias.to_uppercase();
            if language_for_fence(&upper).map(|l| l.label) != label.as_str() {
                errs.push(format!("{upper}: lookup must be case-insensitive"));
            }
        }
        if errs.is_empty() { Ok(()) } else { Err(errs.join("; ")) }
    });
}

#[test]
#[ignore = "measures token-level drift from Lezer; parity is not a goal (see module docs)"]
fn golden_tokens_strict() {
    let g = golden(FIXTURE);
    let cases = g.cases_for("highlightCodeBlocks").filter(|c| c.expected.is_string());
    assert_cases(FIXTURE, cases, |case| {
        let want = expected(case);
        let mut errs = Vec::new();
        for b in html_code_blocks(want.as_str().unwrap()).into_iter().filter(HtmlCodeBlock::highlighted) {
            let Some(lang) = language_for_fence(&b.lang) else { continue };
            let want_toks: Vec<(String, _)> =
                b.spans.iter().filter_map(|(t, c)| lezer_style(c).map(|s| (t.clone(), s))).collect();
            let got = BuiltinHighlighter.highlight(lang, &b.text).map(|l| flatten_runs(&l)).unwrap_or_default();
            if got != want_toks {
                errs.push(format!("{}: got {got:?}\n      expected {want_toks:?}", b.lang));
            }
        }
        if errs.is_empty() { Ok(()) } else { Err(errs.join("\n    ")) }
    });
}
