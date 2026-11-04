//! Render-related parts of `test/javascript/controllers/preview_controller.test.js`.
//!
//! Only one test there is category A for this crate ("render produces h1/p
//! with data-source-line"); the zoom tests (A) belong to the app/workspace
//! settings port. Three C tests assert render semantics the IR must carry and
//! are adapted here (marked "adapted"); the rest are DOM/controller behaviour
//! for the frankmd-app port.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::html::to_html;
use frankmd_markdown::ir::Block;
use frankmd_markdown::{RenderOptions, render};

// port: test/javascript/controllers/preview_controller.test.js:194 "renders markdown content"
#[test]
fn render_produces_heading_and_paragraph_with_lines() {
    let doc = render("# Hello\n\nWorld", &RenderOptions::default());
    assert!(
        matches!(&doc.blocks[0], Block::Heading { level: 1, span, .. } if span.start_line == 1),
        "{:?}",
        doc.blocks
    );
    assert!(
        matches!(&doc.blocks[1], Block::Paragraph { span, .. } if span.start_line == 3),
        "{:?}",
        doc.blocks
    );
    let html = to_html(&doc);
    assert!(
        html.contains("Hello</h1>") && html.contains("World</p>"),
        "{html}"
    );
    assert!(html.contains("data-source-line"), "{html}");
}

// port: test/javascript/controllers/preview_controller.test.js:1127 "renders task checkboxes enabled with per-item source lines" (adapted)
#[test]
fn task_items_carry_their_source_lines() {
    let doc = render(
        "# Title\n\n- [ ] open\n- [x] done\n",
        &RenderOptions::default(),
    );
    assert_eq!(ir_li_lines(&doc), vec![3, 4]);
    let Block::List { items, .. } = &doc.blocks[1] else {
        panic!("{:?}", doc.blocks)
    };
    assert_eq!(
        items.iter().map(|i| i.task).collect::<Vec<_>>(),
        vec![Some(false), Some(true)]
    );
    let html = to_html(&doc);
    assert!(
        !html.contains("disabled"),
        "task checkboxes must stay clickable: {html}"
    );
}

// port: test/javascript/controllers/preview_controller.test.js:1004 "typing path accounts for stripped frontmatter via absolute line anchors" (adapted: render half)
#[test]
fn front_matter_is_stripped_and_lines_stay_absolute() {
    let doc = render(
        "---\ntitle: Test\n---\n\n# First\n\n# Second\n\n",
        &RenderOptions::default(),
    );
    assert_eq!(
        doc.front_matter
            .as_deref()
            .map(|f| f.contains("title: Test")),
        Some(true)
    );
    assert_eq!(
        doc.blocks
            .iter()
            .map(|b| b.span().start_line)
            .collect::<Vec<_>>(),
        vec![5, 7]
    );
}

// port: test/javascript/controllers/preview_controller.test.js:875 "typing scenario: render does not cause editor sync" (adapted: its totalLines == 1 for "# Hello World")
#[test]
fn total_lines_counts_source_lines() {
    assert_eq!(
        render("# Hello World", &RenderOptions::default()).total_lines,
        1
    );
    assert_eq!(render("", &RenderOptions::default()).total_lines, 1);
    assert_eq!(render("a\nb\n", &RenderOptions::default()).total_lines, 3);
}

// port: test/javascript/controllers/preview_controller.test.js:210 "handles empty content" (adapted)
#[test]
fn empty_content_renders_nothing() {
    let doc = render("", &RenderOptions::default());
    assert!(doc.blocks.is_empty());
    assert_eq!(to_html(&doc), "");
}
