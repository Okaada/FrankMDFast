//! Block/inline content insertion into an editor surface (port of
//! `lib/codemirror_content_insertion.js`) and the null-safe editor accessors
//! of `lib/codemirror_adapter.js`.
//!
//! The JS functions take a CodeMirror Stimulus controller; here they take an
//! [`EditorSurface`], a GUI-free trait the app implements over the real
//! editor. All offsets are UTF-8 byte offsets (see [`crate::text`]).

use std::ops::Range;

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
        Self { line: 1, column: 1, offset: 0 }
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
        Self { current_line: 1, total_lines: 1 }
    }
}

/// The editor surface the insertion helpers drive (the controller API of
/// the original). Each mutating call is one undoable edit.
pub trait EditorSurface {
    /// Whole document text.
    fn text(&self) -> String;
    /// Replace the whole document.
    fn set_text(&mut self, text: &str);
    /// Primary selection.
    fn selection(&self) -> Selection;
    /// Set the selection (caret when `from == to`).
    fn set_selection(&mut self, from: usize, to: usize);
    /// Cursor position.
    fn cursor_position(&self) -> CursorPosition;
    /// Cursor line info.
    fn cursor_info(&self) -> CursorInfo;
    /// Replace `from..to` with `text`.
    fn replace_range(&mut self, text: &str, from: usize, to: usize);
    /// Insert `text` at `pos`.
    fn insert_at(&mut self, pos: usize, text: &str);
    /// Focus the editor.
    fn focus(&mut self);
}

/// Options for [`insert_block_content`] (JS `options`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockInsert {
    /// Edit mode: replace exactly this range with the content, no newline
    /// padding (JS `editMode` + `startPos`/`endPos`).
    pub edit: Option<Range<usize>>,
    /// Slash-command replace range (JS `from`/`to`): the pending `/query` is
    /// replaced, absorbing spaces/tabs on both sides.
    pub replace: Option<Range<usize>>,
    /// Final cursor offset from the start of the inserted content (after the
    /// newline prefix). `None` = after the whole insertion.
    pub cursor_offset: Option<usize>,
}

/// Options for [`insert_inline_content`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineInsert {
    /// Replace the selection (default) or insert at its start.
    pub replace_selection: bool,
    /// Slash-command replace range (overrides the selection).
    pub replace: Option<Range<usize>>,
}

impl Default for InlineInsert {
    fn default() -> Self {
        Self { replace_selection: true, replace: None }
    }
}

/// Insert block content (table, code, video, ...) surrounded by blank lines
/// as needed; returns the new cursor offset, or 0 when there is no editor
/// or `content` is empty.
///
/// **[COMPAT]** prefix: `""` at doc start or after `\n\n`, `"\n"` after one
/// `\n`, else `"\n\n"`; suffix mirrors it on the text after.
pub fn insert_block_content(
    editor: Option<&mut dyn EditorSurface>,
    content: &str,
    opts: &BlockInsert,
) -> usize {
    todo!()
}

/// Insert inline content (emoji, text); returns the new cursor offset.
pub fn insert_inline_content(
    editor: Option<&mut dyn EditorSurface>,
    content: &str,
    opts: &InlineInsert,
) -> usize {
    todo!()
}

/// Insert image markdown, padded with single newlines (not blank lines);
/// always via `replace_range` over the selection or slash range.
pub fn insert_image(
    editor: Option<&mut dyn EditorSurface>,
    markdown: &str,
    replace: Option<Range<usize>>,
) -> usize {
    todo!()
}

/// Insert or replace a table (delegates to [`insert_block_content`]).
pub fn insert_table(editor: Option<&mut dyn EditorSurface>, markdown: &str, opts: &BlockInsert) -> usize {
    todo!()
}

/// Insert a fenced code block; the cursor lands on the first content line
/// (`cursor_offset = 3 + language.len() + 1`, overriding `opts.cursor_offset`).
pub fn insert_code_block(
    editor: Option<&mut dyn EditorSurface>,
    code_block: &str,
    language: &str,
    opts: &BlockInsert,
) -> usize {
    todo!()
}

/// Insert a video embed (delegates to [`insert_block_content`]).
pub fn insert_video_embed(editor: Option<&mut dyn EditorSurface>, embed: &str, opts: &BlockInsert) -> usize {
    todo!()
}

// ---------------------------------------------------------------------------
// codemirror_adapter.js
// ---------------------------------------------------------------------------

/// A synthetic input event (JS `{type: "input"}`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputEvent;

/// The adapter's single `input` listener.
pub type InputHandler<'a> = Box<dyn FnMut(&InputEvent) + 'a>;

/// Textarea-shaped view over an [`EditorSurface`], used by find/replace,
/// jump-to-line and text-format (JS `createTextareaAdapter`).
pub struct TextareaAdapter<'a> {
    editor: &'a mut dyn EditorSurface,
    input_handler: Option<InputHandler<'a>>,
}

impl std::fmt::Debug for TextareaAdapter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextareaAdapter")
            .field("has_input_handler", &self.input_handler.is_some())
            .finish_non_exhaustive()
    }
}

impl<'a> TextareaAdapter<'a> {
    /// `None` when there is no editor (JS returns `null`).
    pub fn new(editor: Option<&'a mut dyn EditorSurface>) -> Option<Self> {
        todo!()
    }
    /// Document text (`adapter.value`).
    pub fn value(&self) -> String {
        todo!()
    }
    /// Replace document text (`adapter.value = ...`).
    pub fn set_value(&mut self, text: &str) {
        todo!()
    }
    /// Selection start.
    pub fn selection_start(&self) -> usize {
        todo!()
    }
    /// Selection end.
    pub fn selection_end(&self) -> usize {
        todo!()
    }
    /// Set selection.
    pub fn set_selection_range(&mut self, from: usize, to: usize) {
        todo!()
    }
    /// Focus the editor.
    pub fn focus(&mut self) {
        todo!()
    }
    /// Register the (single) `input` listener, replacing any previous one.
    pub fn add_input_listener(&mut self, handler: InputHandler<'a>) {
        todo!()
    }
    /// Remove the `input` listener.
    pub fn remove_input_listener(&mut self) {
        todo!()
    }
    /// Whether an `input` listener is registered.
    pub fn has_input_listener(&self) -> bool {
        todo!()
    }
    /// Dispatch a synthetic input event; a no-op without a listener.
    pub fn dispatch_input(&mut self, event: &InputEvent) {
        todo!()
    }
}

/// Editor text, else the fallback textarea's value, else `""`.
pub fn get_editor_content(editor: Option<&dyn EditorSurface>, fallback: Option<&str>) -> String {
    todo!()
}

/// Editor selection, else `{0, 0, ""}`.
pub fn get_editor_selection(editor: Option<&dyn EditorSurface>) -> Selection {
    todo!()
}

/// Editor cursor position, else `{line 1, column 1, offset 0}`.
pub fn get_cursor_position(editor: Option<&dyn EditorSurface>) -> CursorPosition {
    todo!()
}

/// Editor cursor info, else `{current_line 1, total_lines 1}`.
pub fn get_cursor_info(editor: Option<&dyn EditorSurface>) -> CursorInfo {
    todo!()
}
