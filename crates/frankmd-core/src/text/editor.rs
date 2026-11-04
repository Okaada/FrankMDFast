//! Plain-text editor helpers (port of `lib/text_editor_utils.js`).
//!
//! Offsets are UTF-8 byte offsets on char boundaries. **[COMPAT]** slicing
//! follows JS `substring`: out-of-range offsets clamp to the text and a
//! reversed `start > end` pair is swapped *for slicing only*; returned
//! cursors are computed from the raw, unclamped arguments exactly like the
//! original (so `insert_at_position("ab", 99, "x").cursor == 100`).
//! Lines are split on `'\n'` only (`"\r"` stays part of the line).

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

/// Result of [`get_cursor_info`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorInfo {
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
pub fn wrap_selection(text: &str, start: usize, end: usize, prefix: &str, suffix: &str) -> WrapResult {
    todo!()
}

/// Insert `insertion` at `position`.
pub fn insert_at_position(text: &str, position: usize, insertion: &str) -> EditResult {
    todo!()
}

/// Expand a selection to whole lines.
pub fn get_line_boundaries(text: &str, start: usize, end: usize) -> LineBoundaries {
    todo!()
}

/// Line/column of a cursor.
pub fn get_cursor_info(text: &str, cursor: usize) -> CursorInfo {
    todo!()
}

/// 1-based line containing `position` (clamped).
pub fn get_line_at_position(text: &str, position: usize) -> usize {
    todo!()
}

/// Byte offset of the start of 1-based `line_number` (`0` and `1` → 0).
/// **[COMPAT]** past the last line it returns `text.len() + 1`.
pub fn get_position_for_line(text: &str, line_number: usize) -> usize {
    todo!()
}

/// Replace `text[start..end]` with `replacement`; cursor = `start + replacement.len()`.
pub fn replace_range(text: &str, start: usize, end: usize, replacement: &str) -> EditResult {
    todo!()
}
