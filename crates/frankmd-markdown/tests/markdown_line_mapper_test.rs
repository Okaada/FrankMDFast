//! Port of `test/javascript/lib/markdown_line_mapper.test.js` (14 tests).
//!
//! `parseWithLineNumbers` → [`render_body`] spans (+ `to_html` attributes);
//! `findElementByLine` → [`LineMap::nearest_anchor`]; the `findLineAtScroll`
//! tests (DOM rects, category C) are re-expressed with the pure geometry they
//! exercise: `scroll::scroll_top_for_element` + `scroll::line_at_scroll`.
//! The original mocked `marked.lexer`/`marked.parse`; here the real pipeline runs.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;

use common::*;
use frankmd_markdown::highlight::BuiltinHighlighter;
use frankmd_markdown::html::to_html;
use frankmd_markdown::ir::{Block, TokenClass};
use frankmd_markdown::linemap::LineMap;
use frankmd_markdown::linemap::scroll::{ScrollEntry, line_at_scroll, scroll_top_for_element};
use frankmd_markdown::{RenderOptions, render_body};

fn lines(md: &str, offset: u32) -> Vec<u32> {
    render_body(md, offset, &RenderOptions::default())
        .blocks
        .iter()
        .map(|b| b.span().start_line)
        .collect()
}

/// A preview container at `page_offset` whose annotated elements sit at
/// content-relative `tops`, scrolled by `container_scroll` (the mocked
/// `getBoundingClientRect` geometry of the original `buildContainer`).
fn entries(tops: &[(u32, f64)], page_offset: f64, container_scroll: f64) -> Vec<ScrollEntry> {
    tops.iter()
        .map(|&(line, top)| ScrollEntry {
            line,
            top: scroll_top_for_element(
                page_offset + top - container_scroll,
                page_offset,
                container_scroll,
            ),
        })
        .collect()
}

const GEOMETRY: &[(u32, f64)] = &[(1, 0.0), (5, 100.0), (10, 200.0)];

// port: test/javascript/lib/markdown_line_mapper.test.js:47 "annotates block elements with 1-based source lines"
#[test]
fn annotates_blocks_with_one_based_lines() {
    assert_eq!(lines("# First\n\n# Second\n\n", 0), vec![1, 3]);
    let html = to_html(&render_body(
        "# First\n\n# Second\n\n",
        0,
        &RenderOptions::default(),
    ));
    assert!(html.contains(r#"data-source-line="1""#), "{html}");
    assert!(html.contains(r#"data-source-line="3""#), "{html}");
}

// port: test/javascript/lib/markdown_line_mapper.test.js:53 "applies the frontmatter line offset to annotations"
#[test]
fn applies_frontmatter_line_offset() {
    assert_eq!(lines("# First\n\n# Second\n\n", 4), vec![5, 7]);
}

// port: test/javascript/lib/markdown_line_mapper.test.js:60 "highlights supported fenced code without losing its source line"
#[test]
fn highlighted_code_keeps_source_line() {
    let opts = RenderOptions {
        highlighter: Some(Arc::new(BuiltinHighlighter)),
        ..RenderOptions::default()
    };
    let doc = render_body("```typescript\nconst x: number = 1;\n```", 0, &opts);
    let Block::Code { lang, lines, span } = &doc.blocks[0] else {
        panic!("{:?}", doc.blocks)
    };
    assert_eq!(span.start_line, 1);
    assert_eq!(lang.as_deref(), Some("typescript"));
    let toks = flatten_runs(lines);
    assert_token_class(&toks, "const", TokenClass::Keyword);
    assert_token_class(&toks, "1", TokenClass::Number);
    let html = to_html(&doc);
    assert!(
        html.contains(r#"data-source-line="1""#) && html.contains(r#"class="language-typescript""#),
        "{html}"
    );
}

// port: test/javascript/lib/markdown_line_mapper.test.js:82 "annotates display math blocks so following preview lines stay aligned"
#[test]
fn display_math_blocks_are_annotated() {
    let doc = render_body(
        "Before\n\n$$\nx + y\n$$\n\nAfter",
        0,
        &RenderOptions::default(),
    );
    assert!(
        matches!(&doc.blocks[1], Block::Math { tex, span } if tex == "x + y" && span.start_line == 3),
        "{:?}",
        doc.blocks
    );
    assert_eq!(
        doc.blocks
            .iter()
            .map(|b| b.span().start_line)
            .collect::<Vec<_>>(),
        vec![1, 3, 7]
    );
}

// port: test/javascript/lib/markdown_line_mapper.test.js:150 "annotates each li with its own source line, including nested items"
#[test]
fn annotates_each_list_item_including_nested() {
    let doc = render_body(
        "# Intro\n\n- [ ] a\n- plain\n- parent\n  - [x] child\n",
        0,
        &RenderOptions::default(),
    );
    assert_eq!(ir_li_lines(&doc), vec![3, 4, 5, 6]);
    let Block::List { items, .. } = &doc.blocks[1] else {
        panic!("{:?}", doc.blocks)
    };
    assert_eq!(items[0].task, Some(false));
    assert_eq!(items[1].task, None);
    let Block::List { items: nested, .. } = &items[2].children[1] else {
        panic!("{:?}", items[2])
    };
    assert_eq!(nested[0].task, Some(true));
}

// port: test/javascript/lib/markdown_line_mapper.test.js:160 "shifts li lines by the frontmatter offset"
#[test]
fn shifts_list_item_lines_by_offset() {
    let doc = render_body(
        "- [ ] a\n- plain\n- parent\n  - [x] child\n",
        3,
        &RenderOptions::default(),
    );
    assert_eq!(ir_li_lines(&doc), vec![4, 5, 6, 7]);
}

// port: test/javascript/lib/markdown_line_mapper.test.js:206 "returns empty string for empty input"
#[test]
fn empty_input_renders_nothing() {
    let doc = render_body("", 0, &RenderOptions::default());
    assert!(doc.blocks.is_empty());
    assert_eq!(to_html(&doc), "");
}

// port: test/javascript/lib/markdown_line_mapper.test.js:222 "finds the closest annotated element for a line"
#[test]
fn nearest_anchor_finds_closest() {
    let map = LineMap::from_lines(&[1, 5, 10]);
    assert_eq!(map.nearest_anchor(5).map(|a| a.line), Some(5));
    assert_eq!(map.nearest_anchor(7).map(|a| a.line), Some(5));
    assert_eq!(map.nearest_anchor(1).map(|a| a.line), Some(1));
}

// port: test/javascript/lib/markdown_line_mapper.test.js:228 "returns null when there are no annotations"
#[test]
fn nearest_anchor_none_without_anchors() {
    assert!(LineMap::from_lines(&[]).nearest_anchor(1).is_none());
}

// port: test/javascript/lib/markdown_line_mapper.test.js:240 "returns the line of the element at/above the scroll position"
#[test]
fn line_at_scroll_element_at_or_above() {
    let e = entries(GEOMETRY, 0.0, 0.0);
    assert_eq!(line_at_scroll(&e, 150.0), Some(5));
    assert_eq!(line_at_scroll(&e, 0.0), Some(1));
    assert_eq!(line_at_scroll(&e, 999.0), Some(10));
}

// port: test/javascript/lib/markdown_line_mapper.test.js:248 "is invariant to non-zero container page offset (preview -> editor direction)"
#[test]
fn line_at_scroll_invariant_to_page_offset() {
    let near = entries(GEOMETRY, 0.0, 0.0);
    let far = entries(GEOMETRY, 100_000.0, 0.0);
    assert_eq!(line_at_scroll(&far, 150.0), line_at_scroll(&near, 150.0));
    assert_eq!(line_at_scroll(&far, 150.0), Some(5));
}

// port: test/javascript/lib/markdown_line_mapper.test.js:259 "accounts for the container's current scrollTop in rect math"
#[test]
fn line_at_scroll_accounts_for_container_scroll() {
    let e = entries(GEOMETRY, 100_000.0, 100.0);
    assert_eq!(line_at_scroll(&e, 100.0), Some(5));
}

// port: test/javascript/lib/markdown_line_mapper.test.js:267 "returns first element's line when scrolled above all elements"
#[test]
fn line_at_scroll_above_all_returns_first() {
    let e = entries(&[(3, 50.0), (8, 150.0)], 0.0, 0.0);
    assert_eq!(line_at_scroll(&e, 10.0), Some(3));
}

// port: test/javascript/lib/markdown_line_mapper.test.js:272 "returns null when there are no annotated elements"
#[test]
fn line_at_scroll_none_without_entries() {
    assert_eq!(line_at_scroll(&[], 100.0), None);
}
