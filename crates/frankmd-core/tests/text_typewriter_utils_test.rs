//! Port of test/javascript/lib/typewriter_utils.test.js.

use frankmd_core::text::typewriter::{
    DEFAULT_TARGET_RATIO, calculate_typewriter_scroll, calculate_typewriter_sync_ratio,
    get_typewriter_sync_data,
};

fn scroll(y: f64, h: f64) -> f64 {
    calculate_typewriter_scroll(y, h, DEFAULT_TARGET_RATIO)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// port: test/javascript/lib/typewriter_utils.test.js:12 "returns 0 when cursor is at top"
#[test]
fn scroll_zero_when_cursor_at_top() {
    assert!(close(scroll(50.0, 500.0), 0.0));
}

// port: test/javascript/lib/typewriter_utils.test.js:19 "calculates scroll to center cursor"
#[test]
fn scroll_to_center_cursor() {
    assert!(close(scroll(500.0, 400.0), 300.0));
}

// port: test/javascript/lib/typewriter_utils.test.js:26 "uses custom target ratio"
#[test]
fn scroll_uses_custom_target_ratio() {
    assert!(close(calculate_typewriter_scroll(500.0, 400.0, 0.3), 380.0));
}

// port: test/javascript/lib/typewriter_utils.test.js:33 "never returns negative"
#[test]
fn scroll_never_negative() {
    assert!(close(scroll(0.0, 500.0), 0.0));
    assert!(close(scroll(100.0, 500.0), 0.0));
}

// port: test/javascript/lib/typewriter_utils.test.js:38 "handles cursor near top of viewport"
#[test]
fn scroll_cursor_near_top_of_viewport() {
    assert!(close(scroll(200.0, 500.0), 0.0));
}

fn sync(text: &str, pos: usize) -> (usize, usize) {
    let d = get_typewriter_sync_data(text, pos);
    (d.current_line, d.total_lines)
}

// port: test/javascript/lib/typewriter_utils.test.js:47 "returns line 1 at start of text"
#[test]
fn sync_line_one_at_start() {
    assert_eq!(sync("line1\nline2\nline3", 0), (1, 3));
}

// port: test/javascript/lib/typewriter_utils.test.js:53 "returns correct line in middle"
#[test]
fn sync_line_in_middle() {
    assert_eq!(sync("line1\nline2\nline3", 8), (2, 3));
}

// port: test/javascript/lib/typewriter_utils.test.js:59 "returns last line at end"
#[test]
fn sync_last_line_at_end() {
    let text = "line1\nline2\nline3";
    assert_eq!(sync(text, text.len()), (3, 3));
}

// port: test/javascript/lib/typewriter_utils.test.js:66 "handles single line"
#[test]
fn sync_single_line() {
    assert_eq!(sync("single line", 5), (1, 1));
}

// port: test/javascript/lib/typewriter_utils.test.js:72 "handles empty text"
#[test]
fn sync_empty_text() {
    assert_eq!(sync("", 0), (1, 1));
}

// port: test/javascript/lib/typewriter_utils.test.js:78 "returns correct line at start of new line"
#[test]
fn sync_start_of_new_line() {
    assert_eq!(sync("line1\nline2", 6).0, 2);
}

// port: test/javascript/lib/typewriter_utils.test.js:85 "returns 0 for first line"
#[test]
fn ratio_zero_for_first_line() {
    assert!(close(calculate_typewriter_sync_ratio(1, 100), 0.0));
}

// port: test/javascript/lib/typewriter_utils.test.js:89 "returns 1 for last line"
#[test]
fn ratio_one_for_last_line() {
    assert!(close(calculate_typewriter_sync_ratio(100, 100), 1.0));
}

// port: test/javascript/lib/typewriter_utils.test.js:93 "returns 0.5 for middle line"
#[test]
fn ratio_middle_line() {
    // toBeCloseTo(0.495, 2): |x - 0.495| < 0.005
    assert!((calculate_typewriter_sync_ratio(50, 100) - 0.495).abs() < 0.005);
}

// port: test/javascript/lib/typewriter_utils.test.js:99 "handles single line document"
#[test]
fn ratio_single_line_document() {
    assert!(close(calculate_typewriter_sync_ratio(1, 1), 0.0));
}

// port: test/javascript/lib/typewriter_utils.test.js:103 "clamps to valid range"
#[test]
fn ratio_clamps_to_valid_range() {
    assert!(close(calculate_typewriter_sync_ratio(0, 100), 0.0));
    assert!(close(calculate_typewriter_sync_ratio(200, 100), 1.0));
}

// port: test/javascript/lib/typewriter_utils.test.js:108 "handles edge case with 2 lines"
#[test]
fn ratio_two_lines() {
    assert!(close(calculate_typewriter_sync_ratio(1, 2), 0.0));
    assert!(close(calculate_typewriter_sync_ratio(2, 2), 1.0));
}
