//! Typewriter-mode scroll math (port of `lib/typewriter_utils.js`).

/// Default cursor position in the viewport (centre).
pub const DEFAULT_TARGET_RATIO: f64 = 0.5;

/// Line data used to sync the preview in typewriter mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypewriterSyncData {
    /// 1-based line of the cursor.
    pub current_line: usize,
    /// Number of `'\n'`-separated lines (≥ 1).
    pub total_lines: usize,
}

/// Scroll top (px) that puts `cursor_y` at `viewport_height * target_ratio`,
/// never negative.
pub fn calculate_typewriter_scroll(cursor_y: f64, viewport_height: f64, target_ratio: f64) -> f64 {
    todo!()
}

/// Current/total lines for a byte offset `cursor` (clamped to the text).
pub fn get_typewriter_sync_data(text: &str, cursor: usize) -> TypewriterSyncData {
    todo!()
}

/// `(current_line - 1) / (total_lines - 1)` clamped to `0..=1`; 0 when
/// `total_lines <= 1`.
pub fn calculate_typewriter_sync_ratio(current_line: usize, total_lines: usize) -> f64 {
    todo!()
}
