//! Plain-text editor helpers (port of `lib/text_editor_utils.js`).
//!
//! Offsets are UTF-8 byte offsets on char boundaries. **[COMPAT]** slicing
//! follows JS `substring`: out-of-range offsets clamp to the text and a
//! reversed `start > end` pair is swapped *for slicing only*; returned
//! cursors are computed from the raw, unclamped arguments exactly like the
//! original (so `insert_at_position("ab", 99, "x").cursor == 100`).
//! Lines are split on `'\n'` only (`"\r"` stays part of the line).

use super::{first_newline_from, js_substring, last_newline_at_or_before, line_count, utf16_len};

/// Result of [`wrap_selection`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapResult {
    /// New document text.
    pub text: String,
    /// `start + prefix.len()`.
    pub cursor_start: usize,
    /// `end + prefix.len()`.
    pub cursor_end: usize,
}

/// Result of [`insert_at_position`] / [`replace_range`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditResult {
    /// New document text.
    pub text: String,
    /// Caret after the inserted text.
    pub cursor: usize,
}

/// Result of [`get_line_boundaries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineBoundaries {
    /// Start of the first touched line.
    pub line_start: usize,
    /// End (before `'\n'`) of the last touched line. **[COMPAT]** can be
    /// *less* than `line_start` for an empty selection right after a newline
    /// (JS `indexOf("\n", end - 1)` finds the preceding newline).
    pub line_end: usize,
    /// `text[line_start..line_end]` split on `'\n'` (JS `substring`, so a
    /// reversed pair is swapped).
    pub lines: Vec<String>,
}

/// Result of [`get_cursor_info`] (line/column for the status bar; not the
/// editor-surface [`CursorInfo`](super::cursor::CursorInfo)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineColumnInfo {
    /// 1-based line of the cursor.
    pub line: usize,
    /// **[COMPAT]** 1-based column in UTF-16 code units (what the JS status
    /// bar shows); past-the-end cursors keep counting (`999` on a 40-unit
    /// text gives a column beyond the line).
    pub column_utf16: usize,
    /// Number of `'\n'`-separated lines (≥ 1).
    pub total_lines: usize,
}

/// `before + prefix + selected + suffix + after`.
pub fn wrap_selection(
    text: &str,
    start: usize,
    end: usize,
    prefix: &str,
    suffix: &str,
) -> WrapResult {
    let before = js_substring(text, 0, start);
    let selected = js_substring(text, start, end);
    let after = js_substring(text, end, text.len());
    WrapResult {
        text: [before, prefix, selected, suffix, after].concat(),
        cursor_start: start + prefix.len(),
        cursor_end: end + prefix.len(),
    }
}

/// Insert `insertion` at `position`.
pub fn insert_at_position(text: &str, position: usize, insertion: &str) -> EditResult {
    let before = js_substring(text, 0, position);
    let after = js_substring(text, position, text.len());
    EditResult {
        text: [before, insertion, after].concat(),
        cursor: position + insertion.len(),
    }
}

/// Expand a selection to whole lines.
pub fn get_line_boundaries(text: &str, start: usize, end: usize) -> LineBoundaries {
    let line_start = last_newline_at_or_before(text, start.saturating_sub(1)).map_or(0, |i| i + 1);
    let line_end = first_newline_from(text, end.saturating_sub(1)).unwrap_or(text.len());
    let lines = js_substring(text, line_start, line_end)
        .split('\n')
        .map(str::to_owned)
        .collect();
    LineBoundaries {
        line_start,
        line_end,
        lines,
    }
}

/// Line/column of a cursor.
pub fn get_cursor_info(text: &str, cursor: usize) -> LineColumnInfo {
    let before = js_substring(text, 0, cursor);
    // The JS column is `cursorPos - lastNewline` in UTF-16 units; a cursor past
    // the end keeps counting one unit per byte beyond the text.
    let cursor_utf16 = utf16_len(before) + cursor.saturating_sub(text.len());
    let column_utf16 = match before.rfind('\n') {
        Some(newline) => cursor_utf16 - utf16_len(&before[..newline]),
        None => cursor_utf16 + 1,
    };
    LineColumnInfo {
        line: line_count(before),
        column_utf16,
        total_lines: line_count(text),
    }
}

/// 1-based line containing `position` (clamped).
pub fn get_line_at_position(text: &str, position: usize) -> usize {
    line_count(js_substring(text, 0, position))
}

/// Byte offset of the start of 1-based `line_number` (`0` and `1` → 0).
/// **[COMPAT]** past the last line it returns `text.len() + 1`.
pub fn get_position_for_line(text: &str, line_number: usize) -> usize {
    text.split('\n')
        .take(line_number.saturating_sub(1))
        .map(|line| line.len() + 1)
        .sum()
}

/// Replace `text[start..end]` with `replacement`; cursor = `start + replacement.len()`.
pub fn replace_range(text: &str, start: usize, end: usize, replacement: &str) -> EditResult {
    let before = js_substring(text, 0, start);
    let after = js_substring(text, end, text.len());
    EditResult {
        text: [before, replacement, after].concat(),
        cursor: start + replacement.len(),
    }
}
