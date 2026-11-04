//! The text-buffer abstraction the vim engine drives.

use std::ops::Range;

/// Minimal editor surface the engine needs. Offsets are UTF-8 bytes.
pub trait VimBuffer {
    /// Whole text (cheap for rope-backed editors via snapshot).
    fn text(&self) -> std::borrow::Cow<'_, str>;
    /// Primary selection (`start == end` for a caret). Head is `end`.
    fn selection(&self) -> Range<usize>;
    /// Set selection/caret.
    fn set_selection(&mut self, range: Range<usize>);
    /// Replace `range` with `text` as one undoable edit.
    fn replace(&mut self, range: Range<usize>, text: &str);
    /// Begin/end an undo group (an operator = one undo step).
    fn transact(&mut self, f: &mut dyn FnMut(&mut dyn VimBuffer));
    /// Undo / redo through the editor's own history.
    fn undo(&mut self);
    /// Redo.
    fn redo(&mut self);
    /// Visible line range (for `H M L`, `Ctrl-D/U`).
    fn visible_lines(&self) -> Range<usize>;
    /// Scroll so `line` is at top/center/bottom (`zt zz zb`).
    fn scroll_to_line(&mut self, line: usize, anchor: ScrollAnchor);
}

/// Where to place a line when scrolling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ScrollAnchor {
    Top,
    Center,
    Bottom,
}
