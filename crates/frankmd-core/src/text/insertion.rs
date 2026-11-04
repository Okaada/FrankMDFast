//! Block/inline content insertion into an editor surface (port of
//! `lib/codemirror_content_insertion.js`) and the null-safe editor accessors
//! of `lib/codemirror_adapter.js`.
//!
//! The JS functions take a CodeMirror Stimulus controller; here they take an
//! [`EditorSurface`], a GUI-free trait the app implements over the real
//! editor. All offsets are UTF-8 byte offsets (see [`crate::text`]).

use std::ops::Range;

pub use super::cursor::{CursorInfo, CursorPosition, Selection};
use super::js_substring;

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
        Self {
            replace_selection: true,
            replace: None,
        }
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
    let Some(editor) = editor.filter(|_| !content.is_empty()) else {
        return 0;
    };

    if let Some(edit) = &opts.edit {
        editor.replace_range(content, edit.start, edit.end);
        let cursor = edit.start + opts.cursor_offset.unwrap_or(content.len());
        editor.set_selection(cursor, cursor);
        return cursor;
    }

    let text = editor.text();
    let (from, to) = match &opts.replace {
        Some(range) => absorb_blanks(&text, range.start, range.end),
        None => {
            let offset = editor.cursor_position().offset;
            (offset, offset)
        }
    };
    let prefix = newline_prefix(js_substring(&text, 0, from));
    let suffix = newline_suffix(js_substring(&text, to, text.len()));
    let insert = [prefix, content, suffix].concat();
    if opts.replace.is_some() {
        editor.replace_range(&insert, from, to);
    } else {
        editor.insert_at(from, &insert);
    }

    let cursor = match opts.cursor_offset {
        Some(offset) => from + prefix.len() + offset,
        None => from + insert.len(),
    };
    editor.set_selection(cursor, cursor);
    cursor
}

/// Insert inline content (emoji, text); returns the new cursor offset.
pub fn insert_inline_content(
    editor: Option<&mut dyn EditorSurface>,
    content: &str,
    opts: &InlineInsert,
) -> usize {
    let Some(editor) = editor.filter(|_| !content.is_empty()) else {
        return 0;
    };
    let (from, to) = match &opts.replace {
        Some(range) => (range.start, range.end),
        None => {
            let selection = editor.selection();
            (selection.from, selection.to)
        }
    };
    if opts.replace_selection {
        editor.replace_range(content, from, to);
    } else {
        editor.insert_at(from, content);
    }
    let cursor = from + content.len();
    editor.set_selection(cursor, cursor);
    cursor
}

/// Insert image markdown, padded with single newlines (not blank lines);
/// always via `replace_range` over the selection or slash range.
pub fn insert_image(
    editor: Option<&mut dyn EditorSurface>,
    markdown: &str,
    replace: Option<Range<usize>>,
) -> usize {
    let Some(editor) = editor.filter(|_| !markdown.is_empty()) else {
        return 0;
    };
    let text = editor.text();
    let (from, to) = match replace {
        Some(range) => absorb_blanks(&text, range.start, range.end),
        None => {
            let selection = editor.selection();
            (selection.from, selection.to)
        }
    };
    let before = js_substring(&text, 0, from);
    let after = js_substring(&text, to, text.len());
    let newline_before = if !before.is_empty() && !before.ends_with('\n') {
        "\n"
    } else {
        ""
    };
    let newline_after = if !after.is_empty() && !after.starts_with('\n') {
        "\n"
    } else {
        ""
    };
    let insert = [newline_before, markdown, newline_after].concat();
    editor.replace_range(&insert, from, to);
    let cursor = from + insert.len();
    editor.set_selection(cursor, cursor);
    cursor
}

/// Insert or replace a table (delegates to [`insert_block_content`]).
pub fn insert_table(
    editor: Option<&mut dyn EditorSurface>,
    markdown: &str,
    opts: &BlockInsert,
) -> usize {
    insert_block_content(editor, markdown, opts)
}

/// Insert a fenced code block; the cursor lands on the first content line
/// (`cursor_offset = 3 + language.len() + 1`, overriding `opts.cursor_offset`).
pub fn insert_code_block(
    editor: Option<&mut dyn EditorSurface>,
    code_block: &str,
    language: &str,
    opts: &BlockInsert,
) -> usize {
    let opts = BlockInsert {
        cursor_offset: Some(3 + language.len() + 1),
        ..opts.clone()
    };
    insert_block_content(editor, code_block, &opts)
}

/// Insert a video embed (delegates to [`insert_block_content`]).
pub fn insert_video_embed(
    editor: Option<&mut dyn EditorSurface>,
    embed: &str,
    opts: &BlockInsert,
) -> usize {
    insert_block_content(editor, embed, opts)
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
        editor.map(|editor| Self {
            editor,
            input_handler: None,
        })
    }
    /// Document text (`adapter.value`).
    pub fn value(&self) -> String {
        self.editor.text()
    }
    /// Replace document text (`adapter.value = ...`).
    pub fn set_value(&mut self, text: &str) {
        self.editor.set_text(text);
    }
    /// Selection start.
    pub fn selection_start(&self) -> usize {
        self.editor.selection().from
    }
    /// Selection end.
    pub fn selection_end(&self) -> usize {
        self.editor.selection().to
    }
    /// Set selection.
    pub fn set_selection_range(&mut self, from: usize, to: usize) {
        self.editor.set_selection(from, to);
    }
    /// Focus the editor.
    pub fn focus(&mut self) {
        self.editor.focus();
    }
    /// Register the (single) `input` listener, replacing any previous one.
    pub fn add_input_listener(&mut self, handler: InputHandler<'a>) {
        self.input_handler = Some(handler);
    }
    /// Remove the `input` listener.
    pub fn remove_input_listener(&mut self) {
        self.input_handler = None;
    }
    /// Whether an `input` listener is registered.
    pub fn has_input_listener(&self) -> bool {
        self.input_handler.is_some()
    }
    /// Dispatch a synthetic input event; a no-op without a listener.
    pub fn dispatch_input(&mut self, event: &InputEvent) {
        if let Some(handler) = self.input_handler.as_mut() {
            handler(event);
        }
    }
}

/// Editor text, else the fallback textarea's value, else `""`.
pub fn get_editor_content(editor: Option<&dyn EditorSurface>, fallback: Option<&str>) -> String {
    match (editor, fallback) {
        (Some(editor), _) => editor.text(),
        (None, Some(fallback)) => fallback.to_owned(),
        (None, None) => String::new(),
    }
}

/// Editor selection, else `{0, 0, ""}`.
pub fn get_editor_selection(editor: Option<&dyn EditorSurface>) -> Selection {
    editor.map(EditorSurface::selection).unwrap_or_default()
}

/// Editor cursor position, else `{line 1, column 1, offset 0}`.
pub fn get_cursor_position(editor: Option<&dyn EditorSurface>) -> CursorPosition {
    editor
        .map(EditorSurface::cursor_position)
        .unwrap_or_default()
}

/// Editor cursor info, else `{current_line 1, total_lines 1}`.
pub fn get_cursor_info(editor: Option<&dyn EditorSurface>) -> CursorInfo {
    editor.map(EditorSurface::cursor_info).unwrap_or_default()
}

/// Newlines needed before block content so it starts on a blank line.
fn newline_prefix(before: &str) -> &'static str {
    if before.is_empty() || before.ends_with("\n\n") {
        ""
    } else if before.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    }
}

/// Newlines needed after block content so it ends with a blank line.
fn newline_suffix(after: &str) -> &'static str {
    if after.is_empty() || after.starts_with("\n\n") {
        ""
    } else if after.starts_with('\n') {
        "\n"
    } else {
        "\n\n"
    }
}

/// Widen a slash-command range over the spaces/tabs around it
/// (JS `/[ \t]*$/` on the text before, `/^[ \t]*/` on the text after).
fn absorb_blanks(text: &str, from: usize, to: usize) -> (usize, usize) {
    let is_blank = |c: char| c == ' ' || c == '\t';
    let before = js_substring(text, 0, from);
    let after = js_substring(text, to, text.len());
    let trailing = before.len() - before.trim_end_matches(is_blank).len();
    let leading = after.len() - after.trim_start_matches(is_blank).len();
    (from.saturating_sub(trailing), to + leading)
}
