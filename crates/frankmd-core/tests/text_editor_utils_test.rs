//! Port of test/javascript/lib/text_editor_utils.test.js. All inputs are ASCII,
//! so the JS UTF-16 offsets equal the Rust byte offsets.

use frankmd_core::text::editor::{
    get_cursor_info, get_line_at_position, get_line_boundaries, get_position_for_line, insert_at_position,
    replace_range, wrap_selection,
};

// port: test/javascript/lib/text_editor_utils.test.js:16 "wraps selected text with prefix and suffix"
#[test]
fn wraps_selected_text_with_prefix_and_suffix() {
    let r = wrap_selection("hello world", 6, 11, "**", "**");
    assert_eq!(r.text, "hello **world**");
    assert_eq!(r.cursor_start, 8);
    assert_eq!(r.cursor_end, 13);
}

// port: test/javascript/lib/text_editor_utils.test.js:23 "handles empty selection"
#[test]
fn wrap_handles_empty_selection() {
    let r = wrap_selection("hello", 3, 3, "[", "]");
    assert_eq!(r.text, "hel[]lo");
    assert_eq!(r.cursor_start, 4);
    assert_eq!(r.cursor_end, 4);
}

// port: test/javascript/lib/text_editor_utils.test.js:30 "wraps at start of text"
#[test]
fn wraps_at_start_of_text() {
    let r = wrap_selection("hello", 0, 5, "_", "_");
    assert_eq!(r.text, "_hello_");
    assert_eq!(r.cursor_start, 1);
    assert_eq!(r.cursor_end, 6);
}

// port: test/javascript/lib/text_editor_utils.test.js:37 "handles multiline selection"
#[test]
fn wrap_handles_multiline_selection() {
    let r = wrap_selection("line1\nline2\nline3", 0, 11, "```\n", "\n```");
    assert_eq!(r.text, "```\nline1\nline2\n```\nline3");
}

// port: test/javascript/lib/text_editor_utils.test.js:44 "inserts text at position"
#[test]
fn inserts_text_at_position() {
    let r = insert_at_position("hello world", 5, " beautiful");
    assert_eq!(r.text, "hello beautiful world");
    assert_eq!(r.cursor, 15);
}

// port: test/javascript/lib/text_editor_utils.test.js:50 "inserts at start"
#[test]
fn inserts_at_start() {
    let r = insert_at_position("world", 0, "hello ");
    assert_eq!(r.text, "hello world");
    assert_eq!(r.cursor, 6);
}

// port: test/javascript/lib/text_editor_utils.test.js:56 "inserts at end"
#[test]
fn inserts_at_end() {
    let r = insert_at_position("hello", 5, " world");
    assert_eq!(r.text, "hello world");
    assert_eq!(r.cursor, 11);
}

// port: test/javascript/lib/text_editor_utils.test.js:62 "handles empty text"
#[test]
fn insert_handles_empty_text() {
    let r = insert_at_position("", 0, "new text");
    assert_eq!(r.text, "new text");
    assert_eq!(r.cursor, 8);
}

const FOUR: &str = "line1\nline2\nline3\nline4";
const THREE: &str = "line1\nline2\nline3";

// port: test/javascript/lib/text_editor_utils.test.js:72 "gets boundaries for single line selection"
#[test]
fn boundaries_single_line_selection() {
    let r = get_line_boundaries(FOUR, 7, 10);
    assert_eq!((r.line_start, r.line_end), (6, 11));
    assert_eq!(r.lines, ["line2"]);
}

// port: test/javascript/lib/text_editor_utils.test.js:79 "gets boundaries for multiline selection"
#[test]
fn boundaries_multiline_selection() {
    let r = get_line_boundaries(FOUR, 7, 14);
    assert_eq!((r.line_start, r.line_end), (6, 17));
    assert_eq!(r.lines, ["line2", "line3"]);
}

// port: test/javascript/lib/text_editor_utils.test.js:86 "handles selection at start of file"
#[test]
fn boundaries_selection_at_start_of_file() {
    let r = get_line_boundaries(FOUR, 0, 3);
    assert_eq!((r.line_start, r.line_end), (0, 5));
    assert_eq!(r.lines, ["line1"]);
}

// port: test/javascript/lib/text_editor_utils.test.js:93 "handles selection at end of file"
#[test]
fn boundaries_selection_at_end_of_file() {
    let r = get_line_boundaries(FOUR, 18, 23);
    assert_eq!((r.line_start, r.line_end), (18, 23));
    assert_eq!(r.lines, ["line4"]);
}

// port: test/javascript/lib/text_editor_utils.test.js:104 "returns line 1 at start"
#[test]
fn cursor_info_line_one_at_start() {
    let r = get_cursor_info(THREE, 0);
    assert_eq!((r.line, r.column_utf16, r.total_lines), (1, 1, 3));
}

// port: test/javascript/lib/text_editor_utils.test.js:111 "returns correct line and column in middle"
#[test]
fn cursor_info_middle() {
    let r = get_cursor_info(THREE, 8);
    assert_eq!((r.line, r.column_utf16), (2, 3));
}

// port: test/javascript/lib/text_editor_utils.test.js:117 "returns correct info at end of line"
#[test]
fn cursor_info_end_of_line() {
    let r = get_cursor_info(THREE, 5);
    assert_eq!((r.line, r.column_utf16), (1, 6));
}

// port: test/javascript/lib/text_editor_utils.test.js:123 "returns correct info at start of line"
#[test]
fn cursor_info_start_of_line() {
    let r = get_cursor_info(THREE, 6);
    assert_eq!((r.line, r.column_utf16), (2, 1));
}

// port: test/javascript/lib/text_editor_utils.test.js:129 "handles empty text"
#[test]
fn cursor_info_empty_text() {
    let r = get_cursor_info("", 0);
    assert_eq!((r.line, r.total_lines), (1, 1));
}

// port: test/javascript/lib/text_editor_utils.test.js:139 "returns 1 at position 0"
#[test]
fn line_at_position_zero() {
    assert_eq!(get_line_at_position(THREE, 0), 1);
}

// port: test/javascript/lib/text_editor_utils.test.js:143 "returns 2 at start of second line"
#[test]
fn line_at_start_of_second_line() {
    assert_eq!(get_line_at_position(THREE, 6), 2);
}

// port: test/javascript/lib/text_editor_utils.test.js:147 "returns 3 at third line"
#[test]
fn line_at_third_line() {
    assert_eq!(get_line_at_position(THREE, 12), 3);
}

// port: test/javascript/lib/text_editor_utils.test.js:155 "returns 0 for line 1"
#[test]
fn position_for_line_one() {
    assert_eq!(get_position_for_line(THREE, 1), 0);
}

// port: test/javascript/lib/text_editor_utils.test.js:159 "returns correct position for line 2"
#[test]
fn position_for_line_two() {
    assert_eq!(get_position_for_line(THREE, 2), 6);
}

// port: test/javascript/lib/text_editor_utils.test.js:163 "returns correct position for line 3"
#[test]
fn position_for_line_three() {
    assert_eq!(get_position_for_line(THREE, 3), 12);
}

// port: test/javascript/lib/text_editor_utils.test.js:167 "handles line beyond end"
#[test]
fn position_for_line_beyond_end() {
    // [COMPAT] past the end by one ("would be past end but safe").
    assert_eq!(get_position_for_line(THREE, 10), THREE.len() + 1);
}

// port: test/javascript/lib/text_editor_utils.test.js:174 "replaces text in range"
#[test]
fn replaces_text_in_range() {
    let r = replace_range("hello world", 6, 11, "universe");
    assert_eq!(r.text, "hello universe");
    assert_eq!(r.cursor, 14);
}

// port: test/javascript/lib/text_editor_utils.test.js:180 "replaces at start"
#[test]
fn replaces_at_start() {
    let r = replace_range("hello world", 0, 5, "goodbye");
    assert_eq!(r.text, "goodbye world");
    assert_eq!(r.cursor, 7);
}

// port: test/javascript/lib/text_editor_utils.test.js:186 "replaces at end"
#[test]
fn replaces_at_end() {
    let r = replace_range("hello world", 6, 11, "there");
    assert_eq!(r.text, "hello there");
    assert_eq!(r.cursor, 11);
}

// port: test/javascript/lib/text_editor_utils.test.js:192 "deletes when replacement is empty"
#[test]
fn deletes_when_replacement_is_empty() {
    let r = replace_range("hello world", 5, 11, "");
    assert_eq!(r.text, "hello");
    assert_eq!(r.cursor, 5);
}
