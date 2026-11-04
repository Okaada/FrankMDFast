//! Markdown structure lookups (port of `lib/markdown_utils.js`): the table or
//! fenced code block under the cursor. Golden: `js_markdown_utils.json`.
//!
//! All positions are **UTF-8 byte offsets** into `text` (the JS used UTF-16
//! code units; the tests convert).

use super::{
    first_newline_from, is_js_whitespace, js_substring, js_trim, last_newline_at_or_before,
};

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
    if !current_line(text, pos)
        .trim_start_matches(is_js_whitespace)
        .starts_with('|')
    {
        return None;
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let current = line_index_of(&lines, pos);
    let is_row = |line: &str| js_trim(line).starts_with('|');

    let mut start_line = current;
    while start_line > 0 && is_row(lines[start_line - 1]) {
        start_line -= 1;
    }
    let mut end_line = current;
    while end_line + 1 < lines.len() && is_row(lines[end_line + 1]) {
        end_line += 1;
    }
    if end_line == start_line {
        return None;
    }

    let table_lines = &lines[start_line..=end_line];
    if !table_lines
        .iter()
        .any(|line| is_separator_row(js_trim(line)))
    {
        return None;
    }

    let start_pos: usize = lines[..start_line].iter().map(|l| l.len() + 1).sum();
    let table_len: usize = table_lines.iter().map(|l| l.len() + 1).sum();
    Some(TableAtPosition {
        start_line,
        end_line,
        start_pos,
        end_pos: start_pos + table_len - 1,
        lines: table_lines.iter().map(|l| (*l).to_owned()).collect(),
    })
}

/// Find the fenced code block containing byte offset `pos`
/// (inclusive at both ends), scanning non-overlapping matches of
/// ```` /```(\w*)\n([\s\S]*?)```/g ```` left to right.
pub fn find_code_block_at_position(text: &str, pos: usize) -> Option<CodeBlockAtPosition> {
    let mut search_from = 0;
    while let Some(block) = next_code_block(text, search_from) {
        if pos >= block.start_pos && pos <= block.end_pos {
            return Some(block);
        }
        search_from = block.end_pos;
    }
    None
}

/// Text of the line containing `pos`, computed the way the JS fast path does
/// (`lastIndexOf("\n", pos - 1)` / `indexOf("\n", pos)`, then `substring`).
fn current_line(text: &str, pos: usize) -> &str {
    let line_start = last_newline_at_or_before(text, pos.saturating_sub(1)).map_or(0, |i| i + 1);
    let line_end = first_newline_from(text, pos).unwrap_or(text.len());
    js_substring(text, line_start, line_end)
}

/// 0-based index of the line whose span (end inclusive) contains `pos`;
/// 0 when none does (**[COMPAT]** `pos` past the end).
fn line_index_of(lines: &[&str], pos: usize) -> usize {
    let mut offset = 0;
    for (i, line) in lines.iter().enumerate() {
        let end = offset + line.len();
        if pos >= offset && pos <= end {
            return i;
        }
        offset = end + 1;
    }
    0
}

/// `^\|[\s:]*-+[\s:]*(\|[\s:]*-+[\s:]*)*\|?\s*$`, matched greedily (the
/// pattern never needs backtracking: every alternative a shorter run would
/// open also fails).
fn is_separator_row(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    if chars.first() != Some(&'|') {
        return false;
    }
    let Some(mut i) = separator_cell(&chars, 1) else {
        return false;
    };
    while chars.get(i) == Some(&'|') {
        match separator_cell(&chars, i + 1) {
            Some(next) => i = next,
            None => {
                i += 1;
                break;
            }
        }
    }
    chars[i..].iter().all(|&c| is_js_whitespace(c))
}

/// `[\s:]*-+[\s:]*` starting at `i`; returns the index after it.
fn separator_cell(chars: &[char], mut i: usize) -> Option<usize> {
    let pad = |c: char| c == ':' || is_js_whitespace(c);
    while chars.get(i).is_some_and(|&c| pad(c)) {
        i += 1;
    }
    let dashes_start = i;
    while chars.get(i) == Some(&'-') {
        i += 1;
    }
    if i == dashes_start {
        return None;
    }
    while chars.get(i).is_some_and(|&c| pad(c)) {
        i += 1;
    }
    Some(i)
}

/// Next match of ```` /```(\w*)\n([\s\S]*?)```/g ```` at or after `from`.
fn next_code_block(text: &str, from: usize) -> Option<CodeBlockAtPosition> {
    const FENCE: &str = "```";
    let mut start = from;
    loop {
        let open = start + text.get(start..)?.find(FENCE)?;
        let after_fence = open + FENCE.len();
        let language_len = text[after_fence..]
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        let newline = after_fence + language_len;
        if text.as_bytes().get(newline) != Some(&b'\n') {
            start = open + 1;
            continue;
        }
        let body_start = newline + 1;
        // No closing fence after this opening means none for later ones either.
        let close = body_start + text[body_start..].find(FENCE)?;
        return Some(CodeBlockAtPosition {
            start_pos: open,
            end_pos: close + FENCE.len(),
            language: text[after_fence..newline].to_owned(),
            content: text[body_start..close].to_owned(),
        });
    }
}
