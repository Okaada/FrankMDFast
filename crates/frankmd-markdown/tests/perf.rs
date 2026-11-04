//! Render-time regression guards for pathological-but-plausible notes.
//! Timing is only meaningful in an optimized build, so these are ignored by
//! default: `cargo test --release -p frankmd-markdown --test perf -- --ignored`.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use frankmd_markdown::{RenderOptions, render};

const BUDGET: Duration = Duration::from_millis(1000);

fn assert_fast(name: &str, src: &str) {
    let t = Instant::now();
    let doc = render(src, &RenderOptions::default());
    let took = t.elapsed();
    eprintln!(
        "{name}: {} bytes, {} blocks, {took:?}",
        src.len(),
        doc.blocks.len()
    );
    assert!(took < BUDGET, "{name} took {took:?} (budget {BUDGET:?})");
}

fn repeat_to(chunk: &str, bytes: usize) -> String {
    chunk.repeat(bytes / chunk.len() + 1)
}

#[test]
#[ignore = "timing; run in release"]
fn mixed_markdown_5mb() {
    let chunk = "## Section\n\nPara with **bold**, *em*, [[Link]], $x_1$, `code`, [l](http://x.y) and :smile:.\n\n\
- item\n- [ ] task\n  - nested\n\n> quote\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```js\nlet a = 1;\n```\n\n";
    assert_fast("mixed 5 MB", &repeat_to(chunk, 5 * 1024 * 1024));
}

#[test]
#[ignore = "timing; run in release"]
fn duplicate_headings_256kb() {
    assert_fast(
        "duplicate headings 256 KB",
        &repeat_to("# Same\n\n", 256 * 1024),
    );
}

#[test]
#[ignore = "timing; run in release"]
fn loose_list_1mb() {
    assert_fast(
        "loose list 1 MB",
        &repeat_to("- item text here\n\n", 1024 * 1024),
    );
}
