//! Conformance (render part of `tests/fixtures/golden/js_emoji_data.json`):
//! every shortcode in FrankMD's `getEmojiMap()` renders `:code:` as that emoji
//! in the preview (marked `emojiExtension`). The data tables themselves are
//! frankmd-core's (`frankmd_core::emoji`).
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_markdown::ir::{Block, Inline};
use frankmd_markdown::{RenderOptions, render_body};
use frankmd_testkit::{assert_cases, golden};

const FIXTURE: &str = "js_emoji_data";

#[test]
fn golden_every_shortcode_renders_in_preview() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("getEmojiMap"), |case| {
        let map = expected(case);
        let mut errs = Vec::new();
        for (code, emoji) in map.as_object().unwrap() {
            let doc = render_body(&format!(":{code}:"), 0, &RenderOptions::default());
            let want = Inline::Emoji { shortcode: code.clone(), emoji: emoji.as_str().unwrap().to_owned() };
            match doc.blocks.as_slice() {
                [Block::Paragraph { inlines, .. }] if inlines.as_slice() == [want.clone()] => {}
                other => errs.push(format!(":{code}: rendered {other:?}")),
            }
        }
        if errs.is_empty() { Ok(()) } else { Err(errs.join("; ")) }
    });
}
