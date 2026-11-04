//! Editor cursor/selection value types shared by the insertion helpers, the
//! adapter accessors and typewriter sync (the canonical `CursorInfo`).

/// Current selection (`from == to` for a caret).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    /// Start byte offset.
    pub from: usize,
    /// End byte offset.
    pub to: usize,
    /// Selected text.
    pub text: String,
}

/// Cursor location. Defaults to line 1, column 1, offset 0 (**[COMPAT]**
/// `getCursorPosition` fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    /// 1-based line.
    pub line: usize,
    /// 1-based column (bytes from line start + 1).
    pub column: usize,
    /// Byte offset in the document.
    pub offset: usize,
}

impl Default for CursorPosition {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}

/// Line info for stats / scroll sync. Defaults to 1/1 (**[COMPAT]**).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorInfo {
    /// 1-based current line.
    pub current_line: usize,
    /// Total line count.
    pub total_lines: usize,
}

impl Default for CursorInfo {
    fn default() -> Self {
        Self {
            current_line: 1,
            total_lines: 1,
        }
    }
}
