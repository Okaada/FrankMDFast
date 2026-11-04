//! Port of `test/javascript/lib/scroll_utils.test.js` (32 tests) →
//! [`linemap::scroll`].
#![allow(clippy::unwrap_used)]

use frankmd_markdown::linemap::scroll::{
    DEFAULT_LINE_OFFSET_RATIO, ScrollEntry, line_at_scroll, line_from_scroll, scroll_for_line,
    scroll_ratio, scroll_to_center_line, scroll_top_for_element, synced_scroll_position,
};

fn for_line(line: f64, lh: f64, vh: f64) -> f64 {
    scroll_for_line(line, lh, vh, DEFAULT_LINE_OFFSET_RATIO)
}

fn entries() -> Vec<ScrollEntry> {
    vec![
        ScrollEntry { line: 1, top: 0.0 },
        ScrollEntry {
            line: 5,
            top: 100.0,
        },
        ScrollEntry {
            line: 10,
            top: 200.0,
        },
    ]
}

// port: test/javascript/lib/scroll_utils.test.js:16 "returns 0 when at top"
#[test]
fn ratio_zero_at_top() {
    assert_eq!(scroll_ratio(0.0, 1000.0, 200.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:20 "returns 1 when at bottom"
#[test]
fn ratio_one_at_bottom() {
    assert_eq!(scroll_ratio(800.0, 1000.0, 200.0), 1.0);
}

// port: test/javascript/lib/scroll_utils.test.js:26 "returns 0.5 when at middle"
#[test]
fn ratio_half_at_middle() {
    assert_eq!(scroll_ratio(400.0, 1000.0, 200.0), 0.5);
}

// port: test/javascript/lib/scroll_utils.test.js:30 "clamps to 0-1 range"
#[test]
fn ratio_clamps() {
    assert_eq!(scroll_ratio(-100.0, 1000.0, 200.0), 0.0);
    assert_eq!(scroll_ratio(1000.0, 1000.0, 200.0), 1.0);
}

// port: test/javascript/lib/scroll_utils.test.js:35 "returns 0 when content fits in viewport"
#[test]
fn ratio_zero_when_content_fits() {
    assert_eq!(scroll_ratio(0.0, 200.0, 200.0), 0.0);
    assert_eq!(scroll_ratio(0.0, 100.0, 200.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:42 "returns 0 for line 1 with default offset"
#[test]
fn scroll_for_line_one_is_zero() {
    assert_eq!(for_line(1.0, 20.0, 500.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:47 "calculates scroll for middle line"
#[test]
fn scroll_for_middle_line() {
    assert_eq!(for_line(50.0, 20.0, 500.0), 805.0);
}

// port: test/javascript/lib/scroll_utils.test.js:53 "uses custom offset ratio"
#[test]
fn scroll_for_line_custom_ratio() {
    assert_eq!(scroll_for_line(50.0, 20.0, 500.0, 0.5), 730.0);
}

// port: test/javascript/lib/scroll_utils.test.js:59 "never returns negative"
#[test]
fn scroll_for_line_never_negative() {
    assert_eq!(for_line(1.0, 20.0, 1000.0), 0.0);
    assert_eq!(for_line(2.0, 20.0, 1000.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:66 "returns line at center of viewport when at top"
#[test]
fn line_from_scroll_at_top() {
    assert_eq!(line_from_scroll(0.0, 200.0, 2000.0, 100.0), 5.0);
}

// port: test/javascript/lib/scroll_utils.test.js:74 "returns middle line when scrolled to middle"
#[test]
fn line_from_scroll_middle() {
    assert_eq!(line_from_scroll(900.0, 200.0, 2000.0, 100.0), 50.0);
}

// port: test/javascript/lib/scroll_utils.test.js:82 "returns last line when scrolled to bottom"
#[test]
fn line_from_scroll_bottom() {
    assert_eq!(line_from_scroll(1800.0, 200.0, 2000.0, 100.0), 95.0);
}

// port: test/javascript/lib/scroll_utils.test.js:90 "clamps to valid range"
#[test]
fn line_from_scroll_clamps() {
    assert!(line_from_scroll(0.0, 200.0, 2000.0, 100.0) >= 1.0);
    assert!(line_from_scroll(1800.0, 200.0, 2000.0, 100.0) <= 100.0);
}

// port: test/javascript/lib/scroll_utils.test.js:95 "handles edge case of 0 lines"
#[test]
fn line_from_scroll_zero_lines() {
    assert_eq!(line_from_scroll(0.0, 200.0, 200.0, 0.0), 1.0);
}

// port: test/javascript/lib/scroll_utils.test.js:101 "returns 0 for ratio 0"
#[test]
fn synced_zero_for_ratio_zero() {
    assert_eq!(synced_scroll_position(0.0, 1000.0, 200.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:105 "returns max scroll for ratio 1"
#[test]
fn synced_max_for_ratio_one() {
    assert_eq!(synced_scroll_position(1.0, 1000.0, 200.0), 800.0);
}

// port: test/javascript/lib/scroll_utils.test.js:110 "returns proportional scroll for ratio 0.5"
#[test]
fn synced_proportional() {
    assert_eq!(synced_scroll_position(0.5, 1000.0, 200.0), 400.0);
}

// port: test/javascript/lib/scroll_utils.test.js:114 "handles when content fits in viewport"
#[test]
fn synced_zero_when_content_fits() {
    assert_eq!(synced_scroll_position(0.5, 200.0, 200.0), 0.0);
    assert_eq!(synced_scroll_position(0.5, 100.0, 200.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:121 "returns 0 for first line"
#[test]
fn center_line_first_is_zero() {
    assert_eq!(scroll_to_center_line(1.0, 100.0, 2000.0, 200.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:125 "returns max scroll for last line"
#[test]
fn center_line_last_is_max() {
    assert_eq!(scroll_to_center_line(100.0, 100.0, 2000.0, 200.0), 1800.0);
}

// port: test/javascript/lib/scroll_utils.test.js:130 "returns proportional scroll for middle line"
#[test]
fn center_line_middle_is_proportional() {
    let r = scroll_to_center_line(50.0, 100.0, 2000.0, 200.0);
    assert!(r > 800.0 && r < 1000.0, "{r}");
}

// port: test/javascript/lib/scroll_utils.test.js:139 "handles single line document"
#[test]
fn center_line_single_line() {
    assert_eq!(scroll_to_center_line(1.0, 1.0, 200.0, 200.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:143 "handles zero lines"
#[test]
fn center_line_zero_lines() {
    assert_eq!(scroll_to_center_line(1.0, 0.0, 200.0, 200.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:149 "computes element top relative to container content"
#[test]
fn element_top_relative_to_content() {
    assert_eq!(scroll_top_for_element(150.0, 50.0, 200.0), 300.0);
}

// port: test/javascript/lib/scroll_utils.test.js:154 "is invariant to the container's page offset (editor -> preview direction)"
#[test]
fn element_top_invariant_to_page_offset() {
    assert_eq!(scroll_top_for_element(5150.0, 5050.0, 200.0), 300.0);
}

// port: test/javascript/lib/scroll_utils.test.js:160 "matches at a zero page offset"
#[test]
fn element_top_matches_at_zero_offset() {
    assert_eq!(
        scroll_top_for_element(150.0, 50.0, 200.0),
        scroll_top_for_element(5150.0, 5050.0, 200.0)
    );
}

// port: test/javascript/lib/scroll_utils.test.js:165 "accounts for positive container scrollTop"
#[test]
fn element_top_accounts_for_scroll() {
    assert_eq!(scroll_top_for_element(-50.0, 50.0, 100.0), 0.0);
}

// port: test/javascript/lib/scroll_utils.test.js:178 "returns first line at scroll top 0"
#[test]
fn line_at_scroll_top_zero() {
    assert_eq!(line_at_scroll(&entries(), 0.0), Some(1));
}

// port: test/javascript/lib/scroll_utils.test.js:182 "returns the last entry at or above the scroll position"
#[test]
fn line_at_scroll_last_at_or_above() {
    assert_eq!(line_at_scroll(&entries(), 100.0), Some(5));
    assert_eq!(line_at_scroll(&entries(), 150.0), Some(5));
    assert_eq!(line_at_scroll(&entries(), 500.0), Some(10));
}

// port: test/javascript/lib/scroll_utils.test.js:188 "returns first entry's line when scrolled above all entries (preview -> editor direction)"
#[test]
fn line_at_scroll_above_all() {
    assert_eq!(line_at_scroll(&entries(), -10.0), Some(1));
}

// port: test/javascript/lib/scroll_utils.test.js:192 "keeps the earlier entry when tops tie (matches previous behavior)"
#[test]
fn line_at_scroll_tie_keeps_earlier() {
    let e = [
        ScrollEntry { line: 2, top: 50.0 },
        ScrollEntry { line: 7, top: 50.0 },
    ];
    assert_eq!(line_at_scroll(&e, 60.0), Some(2));
}

// port: test/javascript/lib/scroll_utils.test.js:196 "returns null for empty entries"
// (JS `null` entries and `[]` are the same empty slice in Rust.)
#[test]
fn line_at_scroll_empty() {
    assert_eq!(line_at_scroll(&[], 100.0), None);
}
