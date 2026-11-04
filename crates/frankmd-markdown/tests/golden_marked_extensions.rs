//! Conformance: `tests/fixtures/golden/js_marked_extensions.json`
//! (FrankMD's marked extensions and marked 15 with the app's config).
//!
//! - `<ext>.start` → [`ext::start`] (byte offset converted to UTF-16 units;
//!   JS `-1`/`undefined` ↔ `None`).
//! - `<ext>.tokenizer` → `ext::scan_*` (raw, payload).
//! - `<ext>.renderer` → [`html::render_inline`] / [`html::render_math_block`],
//!   compared **exactly** (they are FrankMD's literal renderer strings).
//! - `marked.parse` → [`render_body`] + [`html::to_html`], normalized. Raw
//!   `marked.parse` output is unsanitized, so inputs containing `<` are not
//!   compared here; the same corpus entries are compared sanitized in
//!   `golden_preview.rs`.
//! - `marked.parseInline` → [`parse_inline`] + [`html::inlines_to_html`], normalized.
//! - `allExtensions` → [`ext::Extension::ALL`] names/levels.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::html::{inlines_to_html, render_inline, render_math_block, to_html};
use frankmd_markdown::ir::Inline;
use frankmd_markdown::parse::ext::{self, Extension, Level};
use frankmd_markdown::{RenderOptions, parse_inline, render_body};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};

const FIXTURE: &str = "js_marked_extensions";

fn ext_of(prefix: &str) -> Extension {
    match prefix {
        "superscriptExtension" => Extension::Superscript,
        "subscriptExtension" => Extension::Subscript,
        "highlightExtension" => Extension::Highlight,
        "emojiExtension" => Extension::Emoji,
        "iconExtension" => Extension::Icon,
        "wikilinkExtension" => Extension::Wikilink,
        "mathBlockExtension" => Extension::MathBlock,
        "mathInlineExtension" => Extension::MathInline,
        other => panic!("unknown extension {other}"),
    }
}

fn split_fn(case: &Case) -> (Extension, &str) {
    let f = case.func.as_deref().unwrap();
    let (e, m) = f.split_once('.').unwrap();
    (ext_of(e), m)
}

fn cases_with_suffix<'a>(g: &'a frankmd_testkit::Golden, suffix: &'a str) -> impl Iterator<Item = &'a Case> + 'a {
    g.cases.iter().filter(move |c| {
        c.func.as_deref().is_some_and(|f| f.ends_with(suffix) && f.contains("Extension."))
            && !is_known_divergence(FIXTURE, &c.name)
    })
}

#[test]
fn golden_all_extensions_order() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("allExtensions"), |case| {
        let got: Vec<Value> = Extension::ALL
            .iter()
            .map(|e| json!({"name": e.name(), "level": if e.level() == Level::Block { "block" } else { "inline" }}))
            .collect();
        let got = Value::Array(got);
        if got == expected(case) { Ok(()) } else { Err(format!("got {got}")) }
    });
}

#[test]
fn golden_extension_start() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, cases_with_suffix(&g, ".start"), |case| {
        let (e, _) = split_fn(case);
        let src = arg_str(case, 0);
        let got = ext::start(e, &src).map(|b| byte_to_utf16(&src, b));
        let want = match expected(case) {
            Value::Null => None,
            v if v.as_i64() == Some(-1) => None,
            v => Some(v.as_u64().unwrap() as usize),
        };
        if got == want { Ok(()) } else { Err(format!("got {got:?}, expected {want:?}")) }
    });
}

/// Scan with the extension's tokenizer and encode like the JS token.
fn scan_json(e: Extension, src: &str) -> Value {
    let raw = |n: usize| src[..n].to_owned();
    match e {
        Extension::Superscript | Extension::Subscript | Extension::Highlight | Extension::MathBlock | Extension::MathInline => {
            let r = match e {
                Extension::Superscript => ext::scan_superscript(src),
                Extension::Subscript => ext::scan_subscript(src),
                Extension::Highlight => ext::scan_highlight(src),
                Extension::MathBlock => ext::scan_math_block(src),
                _ => ext::scan_math_inline(src),
            };
            r.map_or(Value::Null, |s| json!({"type": e.name(), "raw": raw(s.raw_len), "text": s.value}))
        }
        Extension::Emoji => ext::scan_emoji(src)
            .map_or(Value::Null, |s| json!({"type": "emoji", "raw": raw(s.raw_len), "emoji": s.value})),
        Extension::Icon => ext::scan_icon(src).map_or(Value::Null, |s| {
            json!({"type": "icon", "raw": raw(s.raw_len), "path": s.value.path, "viewBox": s.value.view_box})
        }),
        Extension::Wikilink => ext::scan_wikilink(src).map_or(Value::Null, |s| {
            json!({"type": "wikilink", "raw": raw(s.raw_len), "target": s.value.target, "displayText": s.value.display})
        }),
    }
}

#[test]
fn golden_extension_tokenizer() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, cases_with_suffix(&g, ".tokenizer"), |case| {
        let (e, _) = split_fn(case);
        let got = scan_json(e, &arg_str(case, 0));
        let want = expected(case);
        if got == want { Ok(()) } else { Err(format!("got {got}, expected {want}")) }
    });
}

#[test]
fn golden_extension_renderer() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, cases_with_suffix(&g, ".renderer"), |case| {
        let (e, _) = split_fn(case);
        let tok = arg(case, 0);
        let s = |k: &str| tok[k].as_str().unwrap_or("").to_owned();
        let got = match e {
            Extension::MathBlock => render_math_block(&s("text")),
            _ => render_inline(&match e {
                Extension::Superscript => Inline::Superscript(s("text")),
                Extension::Subscript => Inline::Subscript(s("text")),
                Extension::Highlight => Inline::Highlight(s("text")),
                Extension::MathInline => Inline::Math(s("text")),
                Extension::Emoji => Inline::Emoji { shortcode: String::new(), emoji: s("emoji") },
                Extension::Icon => Inline::Icon { name: String::new(), path: s("path"), view_box: s("viewBox") },
                Extension::Wikilink => {
                    Inline::WikiLink { target: s("target"), label: tok["displayText"].as_str().map(str::to_owned) }
                }
                Extension::MathBlock => unreachable!(),
            }),
        };
        let want = expected(case);
        let want = want.as_str().unwrap();
        if got == want { Ok(()) } else { Err(format!("got {got:?}, expected {want:?}")) }
    });
}

#[test]
fn golden_marked_parse() {
    let g = golden(FIXTURE);
    let cases = g
        .cases_for("marked.parse")
        .filter(|c| !arg_str(c, 0).contains('<') && !is_known_divergence(FIXTURE, &c.name));
    assert_cases(FIXTURE, cases, |case| {
        let doc = render_body(&arg_str(case, 0), 0, &RenderOptions::default());
        let want = expected(case);
        check_same_structure(&to_html(&doc), want.as_str().unwrap())
    });
}

#[test]
fn golden_marked_parse_inline() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("marked.parseInline"), |case| {
        let got = inlines_to_html(&parse_inline(&arg_str(case, 0)));
        let want = expected(case);
        check_same_structure(&got, want.as_str().unwrap())
    });
}
