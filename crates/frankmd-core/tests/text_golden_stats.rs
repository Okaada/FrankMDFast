//! Golden conformance: tests/fixtures/golden/js_stats_utils.json (lib/stats_utils.js).
//!
//! Skipped (unrepresentable in the `u64` API): `formatFileSize` with fractional or
//! negative byte counts (JS gives `"512 undefined"` / `"NaN undefined"`).
//! Deliberate deviation: JS prints `"1.0 undefined"` for ≥ 1 TiB; Rust continues
//! the unit table (`TB`, `PB`), so those expectations are rewritten here.
#![allow(clippy::unwrap_used)]

#[path = "text_common/mod.rs"]
mod text_common;

use std::cell::Cell;

use frankmd_core::text::stats::{calculate_stats, format_file_size, format_read_time};
use frankmd_testkit::serde_json::json;
use frankmd_testkit::{assert_cases, golden};
use text_common::{arg, has_lone_surrogates};

const FIXTURE: &str = "js_stats_utils";

#[test]
fn golden_calculate_stats() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("calculateStats"), |c| {
        assert!(!has_lone_surrogates(&c.input));
        let text = arg(&c.input, 0);
        let s = calculate_stats(text.as_str());
        let got = json!({
            "wordCount": s.word_count,
            "charCount": s.char_count_utf16,
            "byteSize": s.byte_size,
            "readTimeMinutes": s.read_time_minutes,
        });
        if got == c.expected { Ok(()) } else { Err(format!("got {got}, want {}", c.expected)) }
    });
}

#[test]
fn golden_format_read_time() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("formatReadTime"), |c| {
        let minutes = arg(&c.input, 0).as_f64().unwrap();
        let got = format_read_time(minutes);
        if got == c.expected.as_str().unwrap() { Ok(()) } else { Err(format!("got {got:?}, want {}", c.expected)) }
    });
}

#[test]
fn golden_format_file_size() {
    let g = golden(FIXTURE);
    let skipped = Cell::new(0);
    assert_cases(FIXTURE, g.cases_for("formatFileSize"), |c| {
        let Some(bytes) = arg(&c.input, 0).as_u64() else {
            skipped.set(skipped.get() + 1);
            return Ok(());
        };
        let mut want = c.expected.as_str().unwrap().to_owned();
        if let Some(num) = want.strip_suffix(" undefined") {
            // ≥ 1 TiB: JS ran off its unit table.
            let unit = match bytes.ilog(1024) {
                4 => "TB",
                5 => "PB",
                n => panic!("unexpected magnitude {n}"),
            };
            want = format!("{num} {unit}");
        }
        let got = format_file_size(bytes);
        if got == want { Ok(()) } else { Err(format!("got {got:?}, want {want:?}")) }
    });
    assert_eq!(skipped.get(), 3, "fractional/negative formatFileSize cases");
}
