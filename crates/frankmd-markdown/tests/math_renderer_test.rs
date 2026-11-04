//! Port of `test/javascript/lib/math_renderer.test.js` (7 tests; 5 ported,
//! 2 skipped). The original mocks KaTeX and checks which placeholders get
//! rendered with which options; natively the IR carries the math nodes (style
//! = KaTeX `displayMode`) and [`math::validate`] applies FrankMD's options.
//!
//! Skipped (DOM-only, no native equivalent):
//! - :49 "is idempotent — a second call does not re-render" — the
//!   `data-math-rendered` marker guards re-rendering an already-rendered DOM;
//!   the IR is rebuilt from source and never re-reads rendered output.
//! - :67 "does nothing for a null container" — no null container in Rust.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::ir::{Block, Inline};
use frankmd_markdown::math::{MathError, MathPolicy, MathStyle, validate};
use frankmd_markdown::{RenderOptions, render_body};

fn doc(md: &str) -> frankmd_markdown::RenderDoc {
    render_body(md, 0, &RenderOptions::default())
}

// port: test/javascript/lib/math_renderer.test.js:17 "renders an inline placeholder with displayMode false"
#[test]
fn inline_math_is_inline_style() {
    let d = doc("$E = mc^2$");
    assert_eq!(collect_math(&d), vec![(MathStyle::Inline, "E = mc^2".to_owned())]);
    assert!(matches!(&d.blocks[0], Block::Paragraph { inlines, .. } if inlines == &[Inline::Math("E = mc^2".into())]));
}

// port: test/javascript/lib/math_renderer.test.js:27 "renders a block placeholder with displayMode true"
#[test]
fn block_math_is_display_style() {
    let d = doc("$$\nx = \\frac{1}{2}\n$$");
    assert_eq!(collect_math(&d), vec![(MathStyle::Display, r"x = \frac{1}{2}".to_owned())]);
}

// port: test/javascript/lib/math_renderer.test.js:35 "always passes the hardened security options (trust:false)"
#[test]
fn hardened_options() {
    let p = MathPolicy::FRANKMD;
    assert!(!p.trust);
    assert!(!p.throw_on_error);
    assert_eq!(p.max_expand, 1000);
    // trust:false → \href is not executed (KaTeX renders it as an unsupported command)
    assert_eq!(
        validate(r"\href{javascript:alert(1)}{x}", MathStyle::Inline),
        Err(MathError::Unsupported(vec![r"\href".to_owned()]))
    );
    // maxExpand bounds recursive macros
    assert!(matches!(validate(r"\def\a{\a\a}\a", MathStyle::Inline), Err(MathError::Parse(_))));
}

// port: test/javascript/lib/math_renderer.test.js:43 "renders multiple independent placeholders"
#[test]
fn multiple_independent_math_nodes() {
    let d = doc("$a$\n\nx\n\n$$b$$");
    assert_eq!(
        collect_math(&d),
        vec![(MathStyle::Inline, "a".to_owned()), (MathStyle::Display, "b".to_owned())]
    );
}

// port: test/javascript/lib/math_renderer.test.js:56 "marks a node that made KaTeX throw and keeps going"
#[test]
fn bad_math_does_not_affect_other_nodes() {
    let d = doc(r"$\frac{1}$ and $x$");
    let math = collect_math(&d);
    assert_eq!(math.len(), 2, "{math:?}");
    assert!(matches!(validate(&math[0].1, math[0].0), Err(MathError::Parse(_))));
    assert_eq!(validate(&math[1].1, math[1].0), Ok(()));
}
