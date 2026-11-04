//! Typewriter-mode scroll math (port of `lib/typewriter_utils.js`).

use super::cursor::CursorInfo;
use super::{js_substring, line_count};

/// Default cursor position in the viewport (centre).
pub const DEFAULT_TARGET_RATIO: f64 = 0.5;

/// Line data used to sync the preview in typewriter mode: the shared
/// [`CursorInfo`] (1-based current line, `'\n'`-separated line count ≥ 1).
pub type TypewriterSyncData = CursorInfo;

/// Scroll top (px) that puts `cursor_y` at `viewport_height * target_ratio`,
/// never negative.
pub fn calculate_typewriter_scroll(cursor_y: f64, viewport_height: f64, target_ratio: f64) -> f64 {
    (cursor_y - viewport_height * target_ratio).max(0.0)
}

/// Current/total lines for a byte offset `cursor` (clamped to the text).
pub fn get_typewriter_sync_data(text: &str, cursor: usize) -> TypewriterSyncData {
    let before = js_substring(text, 0, cursor);
    CursorInfo {
        current_line: line_count(before),
        total_lines: line_count(text),
    }
}

/// `(current_line - 1) / (total_lines - 1)` clamped to `0..=1`; 0 when
/// `total_lines <= 1`.
pub fn calculate_typewriter_sync_ratio(current_line: usize, total_lines: usize) -> f64 {
    if total_lines <= 1 {
        return 0.0;
    }
    let ratio = (current_line as f64 - 1.0) / (total_lines as f64 - 1.0);
    ratio.clamp(0.0, 1.0)
}
