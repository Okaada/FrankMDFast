//! Port of test/javascript/markdown_utils.test.js (lib/markdown_utils.js).
//! All texts here are ASCII, so UTF-16 and byte offsets coincide.
#![allow(clippy::unwrap_used)]

use frankmd_core::text::markdown::{find_code_block_at_position, find_table_at_position};

const TABLE_DOC: &str = "Some text\n\n| Name | Age |\n| --- | --- |\n| Alice | 30 |\n\nMore text";
const CODE_DOC: &str = "Some text\n\n```javascript\nconst x = 1\n```\n\nMore text";

fn idx(text: &str, needle: &str) -> usize {
    text.find(needle).unwrap()
}

// port: test/javascript/markdown_utils.test.js:8 "finds table when cursor is on table row"
#[test]
fn finds_table_when_cursor_is_on_table_row() {
    let result = find_table_at_position(TABLE_DOC, idx(TABLE_DOC, "| Alice")).expect("table");
    assert_eq!(result.lines.len(), 3);
    assert_eq!(result.lines[0], "| Name | Age |");
}

// port: test/javascript/markdown_utils.test.js:26 "returns null when not in table"
#[test]
fn returns_none_when_not_in_table() {
    assert_eq!(
        find_table_at_position(TABLE_DOC, idx(TABLE_DOC, "Some text")),
        None
    );
}

// port: test/javascript/markdown_utils.test.js:41 "returns null for single-line table"
#[test]
fn returns_none_for_single_line_table() {
    assert_eq!(find_table_at_position("| Just | One | Line |", 5), None);
}

// port: test/javascript/markdown_utils.test.js:48 "includes correct start and end positions"
#[test]
fn includes_correct_start_and_end_positions() {
    let text = "| A | B |\n| - | - |\n| 1 | 2 |";
    let result = find_table_at_position(text, 5).expect("table");
    assert_eq!(result.start_pos, 0);
    assert_eq!(result.end_pos, text.len());
}

// port: test/javascript/markdown_utils.test.js:59 "handles table at document start"
#[test]
fn handles_table_at_document_start() {
    let text = "| Header |\n| --- |\n| Value |\nSome other text";
    let result = find_table_at_position(text, 0).expect("table");
    assert_eq!(result.start_line, 0);
    assert_eq!(result.lines.len(), 3);
}

// port: test/javascript/markdown_utils.test.js:72 "handles table at document end"
#[test]
fn handles_table_at_document_end() {
    let text = "Some text\n| Header |\n| --- |\n| Value |";
    let result = find_table_at_position(text, text.len() - 1).expect("table");
    assert_eq!(result.lines.len(), 3);
}

// port: test/javascript/markdown_utils.test.js:84 "returns null for consecutive pipe lines without separator row"
#[test]
fn returns_none_for_pipe_lines_without_separator_row() {
    assert_eq!(find_table_at_position("| foo |\n| bar |", 0), None);
}

// port: test/javascript/markdown_utils.test.js:92 "returns null for pipe lines that are not a real table"
#[test]
fn returns_none_for_pipe_lines_that_are_not_a_table() {
    let text = "Some text\n| this is not\n| a real table\nMore text";
    assert_eq!(find_table_at_position(text, idx(text, "| this")), None);
}

// port: test/javascript/markdown_utils.test.js:104 "detects table with colon-aligned separator row"
#[test]
fn detects_table_with_colon_aligned_separator_row() {
    let text = "| Left | Center | Right |\n| :--- | :---: | ---: |\n| a | b | c |";
    let result = find_table_at_position(text, 0).expect("table");
    assert_eq!(result.lines.len(), 3);
}

// port: test/javascript/markdown_utils.test.js:115 "detects table when cursor is on separator row"
#[test]
fn detects_table_when_cursor_is_on_separator_row() {
    let text = "| H1 | H2 |\n| --- | --- |\n| V1 | V2 |";
    let result = find_table_at_position(text, idx(text, "| --- |")).expect("table");
    assert_eq!(result.lines.len(), 3);
}

// port: test/javascript/markdown_utils.test.js:129 "finds code block when cursor is inside"
#[test]
fn finds_code_block_when_cursor_is_inside() {
    let result = find_code_block_at_position(CODE_DOC, idx(CODE_DOC, "const x")).expect("block");
    assert_eq!(result.language, "javascript");
    assert!(result.content.contains("const x = 1"));
}

// port: test/javascript/markdown_utils.test.js:146 "returns null when not in code block"
#[test]
fn returns_none_when_not_in_code_block() {
    assert_eq!(
        find_code_block_at_position(CODE_DOC, idx(CODE_DOC, "Some text")),
        None
    );
}

// port: test/javascript/markdown_utils.test.js:161 "handles code block without language"
#[test]
fn handles_code_block_without_language() {
    let result = find_code_block_at_position("```\nplain code\n```", 10).expect("block");
    assert_eq!(result.language, "");
}

// port: test/javascript/markdown_utils.test.js:172 "finds correct block with multiple code blocks"
#[test]
fn finds_correct_block_with_multiple_code_blocks() {
    let text = "```javascript\nfirst\n```\n\n```python\nsecond\n```";
    let result = find_code_block_at_position(text, idx(text, "second")).expect("block");
    assert_eq!(result.language, "python");
}
