//! Markdown structure lookups (port of `lib/markdown_utils.js`): the table or
//! fenced code block under the cursor. Golden: `js_markdown_utils.json`.
//!
//! All positions are **UTF-8 byte offsets** into `text` (the JS used UTF-16
//! code units; the tests convert).

/// A pipe table found around a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableAtPosition {
    /// 0-based index of the first table line (split on `\n`).
    pub start_line: usize,
    /// 0-based index of the last table line (inclusive).
    pub end_line: usize,
    /// Byte offset of the start of `start_line`.
    pub start_pos: usize,
    /// Byte offset of the end of `end_line` (exclusive, before its `\n`).
    pub end_pos: usize,
    /// The raw table lines, untrimmed.
    pub lines: Vec<String>,
}

/// A fenced code block (```` ```lang\n...``` ````) found around a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlockAtPosition {
    /// Byte offset of the opening fence.
    pub start_pos: usize,
    /// Byte offset just past the closing fence.
    pub end_pos: usize,
    /// Info string (ASCII `\w*`; may be empty).
    pub language: String,
    /// Body between the opening line and the closing fence (keeps its trailing `\n`).
    pub content: String,
}

/// Find the markdown table containing byte offset `pos`.
///
/// **[COMPAT]** quirks kept from the JS:
/// - The fast path checks only the line containing `pos`; it must start with
///   `|` after leading whitespace.
/// - The table is the maximal run of adjacent lines whose trimmed text starts
///   with `|`; it needs ≥2 lines and at least one separator row matching
///   `^\|[\s:]*-+[\s:]*(\|[\s:]*-+[\s:]*)*\|?\s*$` (on the trimmed line).
/// - `pos` past the end of `text`: the fast path looks at the last line, but
///   the line search then falls back to line 0.
pub fn find_table_at_position(text: &str, pos: usize) -> Option<TableAtPosition> {
    todo!("markdown_utils.js findTableAtPosition")
}

/// Find the fenced code block containing byte offset `pos`
/// (inclusive at both ends), scanning non-overlapping matches of
/// ```` /```(\w*)\n([\s\S]*?)```/g ```` left to right.
pub fn find_code_block_at_position(text: &str, pos: usize) -> Option<CodeBlockAtPosition> {
    todo!("markdown_utils.js findCodeBlockAtPosition")
}
