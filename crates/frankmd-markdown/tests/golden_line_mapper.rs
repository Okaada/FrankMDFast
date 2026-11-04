//! Conformance: `tests/fixtures/golden/js_markdown_line_mapper.json`
//! (`markdown_line_mapper.js` against real marked + DOMPurify).
//!
//! - `parseWithLineNumbers(md, offset)` → [`render_body`]`(md, offset)`:
//!   structure via `to_html` (normalized) + IR line semantics.
//! - `findElementByLine` → [`LineMap::nearest_anchor`] (tag, line, first 40
//!   UTF-16 units of `textContent`).
//! - `findLineAtScroll` → `scroll::scroll_top_for_element` + `scroll::line_at_scroll`.
//! - `marked.lexer(types+raw)`: informative marked token dumps, no IR
//!   equivalent — not driven (counted as skipped in the port doc).
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::html::to_html;
use frankmd_markdown::ir::Block;
use frankmd_markdown::linemap::scroll::{ScrollEntry, line_at_scroll, scroll_top_for_element};
use frankmd_markdown::linemap::{AnchorTarget, LineMap};
use frankmd_markdown::{RenderDoc, RenderOptions, render_body};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, golden};

const FIXTURE: &str = "js_markdown_line_mapper";

#[test]
fn golden_parse_with_line_numbers() {
    let g = golden(FIXTURE);
    let cases = g.cases_for("parseWithLineNumbers").filter(|c| {
        // A negative frontmatter offset yields lines <= 0, which SourceSpan
        // (1-based u32) cannot carry; the only such case is "negative offset".
        let negative = arg(c, 1).as_i64().is_some_and(|o| o < 0);
        !negative && !is_known_divergence(FIXTURE, &c.name)
    });
    assert_cases(FIXTURE, cases, |case| {
        let md = arg_str(case, 0);
        let offset = u32::try_from(arg(case, 1).as_i64().unwrap_or(0)).unwrap();
        let want = expected(case);
        let want = want.as_str().unwrap();
        let doc = render_body(&md, offset, &RenderOptions::default());
        let mut errs = Vec::new();
        if let Err(e) = check_same_structure(&to_html(&doc), want) {
            errs.push(e);
        }
        if let Err(e) = check_anchors(&doc, want, 0) {
            errs.push(e);
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("\n    "))
        }
    });
}

/// `(tag, textContent)` of an anchor's target, as the DOM would report it.
fn anchor_info(doc: &RenderDoc, target: &AnchorTarget) -> (String, String) {
    match target {
        AnchorTarget::Block(ix) => {
            let b = &doc.blocks[*ix];
            (block_tag(b).unwrap_or_default(), block_text(b))
        }
        AnchorTarget::ListItem { block, path } => {
            let mut items = match &doc.blocks[*block] {
                Block::List { items, .. } => items,
                other => panic!("list-item anchor on {other:?}"),
            };
            let mut item = &items[path[0]];
            for &p in &path[1..] {
                items = item
                    .children
                    .iter()
                    .find_map(|c| match c {
                        Block::List { items, .. } => Some(items),
                        _ => None,
                    })
                    .unwrap();
                item = &items[p];
            }
            ("li".into(), item.children.iter().map(block_text).collect())
        }
    }
}

#[test]
fn golden_find_element_by_line() {
    let g = golden(FIXTURE);
    // The "empty container" case runs over hand-written HTML with no
    // annotations; its native equivalent is `LineMap::from_lines(&[])`.
    assert_cases(FIXTURE, g.cases_for("findElementByLine"), |case| {
        let target = case.input["targetLine"].as_i64().unwrap();
        let want = expected(case);
        let got = if let Some(md) = case.input["markdown"].as_str() {
            let doc = render_body(md, 0, &RenderOptions::default());
            let map = LineMap::new(&doc);
            map.nearest_anchor(target).map(|a| {
                let (tag, text) = anchor_info(&doc, &a.target);
                let cut: String =
                    String::from_utf16_lossy(&text.encode_utf16().take(40).collect::<Vec<_>>());
                json!({"tag": tag, "line": a.line.to_string(), "text": cut})
            })
        } else {
            LineMap::from_lines(&[])
                .nearest_anchor(target)
                .map(|a| json!({"line": a.line.to_string()}))
        };
        let got = got.unwrap_or(Value::Null);
        if got == want {
            Ok(())
        } else {
            Err(format!("got {got}, expected {want}"))
        }
    });
}

#[test]
fn golden_find_line_at_scroll() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("findLineAtScroll"), |case| {
        let i = &case.input;
        let lines: Vec<u32> = i["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        let tops: Vec<f64> = i["tops"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let ct = i["containerTop"].as_f64().unwrap();
        let cst = i["containerScrollTop"].as_f64().unwrap();
        let st = i["scrollTop"].as_f64().unwrap();
        let entries: Vec<ScrollEntry> = lines
            .iter()
            .zip(&tops)
            .map(|(&line, &top)| ScrollEntry {
                line,
                top: scroll_top_for_element(top, ct, cst),
            })
            .collect();
        let got = line_at_scroll(&entries, st).map_or(Value::Null, |l| json!(l));
        let want = expected(case);
        if got == want {
            Ok(())
        } else {
            Err(format!("got {got}, expected {want}"))
        }
    });
}
