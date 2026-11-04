//! The vim layer's bridge: GPUI keystrokes → `frankmd_vim::Key`, and
//! [`EditorBuffer`], the `frankmd_vim::VimBuffer` over gpui-kit's `EditorState`.
//!
//! # One vim command = one undo step
//!
//! gpui-kit keeps its undo grouping private (`EditIntent`, `begin_transaction`;
//! gpui-kit#2617 is open) and records every programmatic `replace` as its own
//! *atomic* undo entry that never coalesces. So the buffer does not forward each
//! engine `replace` to the editor. It applies them to a **shadow copy** of the text
//! and, when the engine returns from one key ([`EditorBuffer::flush`]), diffs the
//! shadow against the editor text and commits the difference as **one** replace over
//! the changed span. Whatever the engine did for that key (`3dd`, `:s/a/b/g`, a
//! blockwise insert, a `.` replay of an insert session, `3ifoo<Esc>`) becomes a
//! single undo step. Undo/redo inside a key flush first and then dispatch gpui-kit's
//! own `Undo`/`Redo` actions synchronously on the editor's focus handle.
//!
//! Known limit: text typed in insert mode is inserted by the editor itself (IME,
//! auto-pairs and smart indent keep working). gpui-kit coalesces that typing into
//! one step, but it stays a separate step from the command that opened the insert
//! (`cw`, `o`), so `u` after `cwfoo<Esc>` needs two presses. Lifting this needs an
//! upstream `EditorState::transact` (offered in docs/research/gpui-stack.md §6.2).

use std::borrow::Cow;
use std::ops::Range;

use frankmd_vim::VimBuffer;
use frankmd_vim::buffer::ScrollAnchor;
use frankmd_vim::keymap::Key;
use gpui_kit::component::input::{EditorState, Redo, RopeExt as _, Undo};
use gpui_kit::{App, Entity, FocusHandle, Focusable as _, Keystroke, Window, point, px};

/// Keys that only report a modifier going down.
const MODIFIER_KEYS: [&str; 6] = ["shift", "control", "alt", "platform", "function", "fn"];

/// Normalize a GPUI keystroke for the engine; `None` for keys vim never sees
/// (bare modifiers, Super/Cmd combos).
pub(crate) fn vim_key(keystroke: &Keystroke) -> Option<Key> {
    let m = keystroke.modifiers;
    if m.platform || m.function || MODIFIER_KEYS.contains(&keystroke.key.as_str()) {
        return None;
    }
    let ctrl = m.control;
    let alt = m.alt;
    let key = match keystroke.key_char.as_deref() {
        Some(" ") => "space".to_owned(),
        Some(c) if !ctrl && !alt && c.chars().count() == 1 => c.to_owned(),
        _ if m.shift && !ctrl && keystroke.key.chars().count() == 1 => keystroke.key.to_uppercase(),
        _ => keystroke.key.clone(),
    };
    Some(Key { key, ctrl, alt })
}

/// `VimBuffer` over the editor entity for the duration of one key.
pub(crate) struct EditorBuffer<'a> {
    state: &'a Entity<EditorState>,
    focus: FocusHandle,
    window: &'a mut Window,
    cx: &'a mut App,
    /// The logical text (editor text plus pending edits), loaded on first use.
    text: std::cell::OnceCell<String>,
    /// Editor text when the first pending edit was made (diff base for [`Self::flush`]).
    original: Option<String>,
    selection: Range<usize>,
    selection_dirty: bool,
    scroll: Option<(usize, ScrollAnchor)>,
}

impl<'a> EditorBuffer<'a> {
    /// Wrap the editor for one engine call.
    pub(crate) fn new(
        state: &'a Entity<EditorState>,
        window: &'a mut Window,
        cx: &'a mut App,
    ) -> Self {
        let (selection, focus) = {
            let s = state.read(cx);
            (s.selected_range(), s.focus_handle(cx))
        };
        Self {
            state,
            focus,
            window,
            cx,
            text: std::cell::OnceCell::new(),
            original: None,
            selection,
            selection_dirty: false,
            scroll: None,
        }
    }

    fn loaded(&self) -> &String {
        self.text
            .get_or_init(|| self.state.read(self.cx).text().to_string())
    }

    fn load(&mut self) -> &mut String {
        self.loaded();
        self.text.get_mut().expect("text was just loaded")
    }

    /// Commit pending edits as one replace, then the selection and scroll requests.
    pub(crate) fn flush(&mut self) {
        if let (Some(original), Some(text)) = (self.original.take(), self.text.get()) {
            let (range, new) = super::edit::minimal_replacement(&original, 0, text);
            if !range.is_empty() || !new.is_empty() {
                let new = new.to_owned();
                let window = &mut *self.window;
                self.state.update(self.cx, |s, cx| {
                    s.set_selected_range(range, cx);
                    s.replace(new, window, cx);
                });
                self.selection_dirty = true;
            }
        }
        if self.selection_dirty {
            let selection = self.selection.clone();
            self.state
                .update(self.cx, |s, cx| s.set_selected_range(selection, cx));
            self.selection_dirty = false;
        }
        if let Some((line, anchor)) = self.scroll.take() {
            scroll_to_line(self.state, line, anchor, self.cx);
        }
    }

    fn history(&mut self, undo: bool) {
        self.flush();
        if undo {
            self.focus.dispatch_action(&Undo, self.window, self.cx);
        } else {
            self.focus.dispatch_action(&Redo, self.window, self.cx);
        }
        self.text = std::cell::OnceCell::new();
        self.selection = self.state.read(self.cx).selected_range();
    }
}

impl VimBuffer for EditorBuffer<'_> {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.loaded())
    }

    fn selection(&self) -> Range<usize> {
        self.selection.clone()
    }

    fn set_selection(&mut self, range: Range<usize>) {
        let len = self
            .text
            .get()
            .map_or_else(|| self.state.read(self.cx).text().len(), String::len);
        let range = range.start.min(len)..range.end.min(len);
        if range != self.selection {
            self.selection = range;
            self.selection_dirty = true;
        }
    }

    fn replace(&mut self, range: Range<usize>, text: &str) {
        if self.original.is_none() {
            self.original = Some(self.loaded().clone());
        }
        let buf = self.load();
        let range = range.start.min(buf.len())..range.end.min(buf.len());
        buf.replace_range(range.clone(), text);
        let end = range.start + text.len();
        self.selection = end..end;
        self.selection_dirty = true;
    }

    fn transact(&mut self, f: &mut dyn FnMut(&mut dyn VimBuffer)) {
        // Every key is already one transaction (see the module docs).
        f(self);
    }

    fn undo(&mut self) {
        self.history(true);
    }

    fn redo(&mut self) {
        self.history(false);
    }

    fn visible_lines(&self) -> Range<usize> {
        visible_lines(self.state, self.cx)
    }

    fn scroll_to_line(&mut self, line: usize, anchor: ScrollAnchor) {
        self.scroll = Some((line, anchor));
    }
}

/// Buffer lines currently on screen (falls back to the whole document before layout).
pub(crate) fn visible_lines(state: &Entity<EditorState>, cx: &App) -> Range<usize> {
    let s = state.read(cx);
    let map = s.display_map();
    match s.visible_row_range() {
        Some(rows) if !rows.is_empty() => {
            let first = map.display_row_to_buffer_line(rows.start);
            let last = map.display_row_to_buffer_line(rows.end.saturating_sub(1));
            first..last + 1
        }
        _ => 0..s.text().lines_len().max(1),
    }
}

/// Scroll so buffer `line` is at the top/centre/bottom of the viewport.
pub(crate) fn scroll_to_line(
    state: &Entity<EditorState>,
    line: usize,
    anchor: ScrollAnchor,
    cx: &mut App,
) {
    state.update(cx, |s, cx| {
        let line_height = s.line_height().unwrap_or(px(20.));
        let row = s.display_map().buffer_line_to_display_row(line);
        let viewport = s.input_bounds().size.height;
        let top = line_height * row as f32;
        let y = match anchor {
            ScrollAnchor::Top => top,
            ScrollAnchor::Center => top - (viewport - line_height) / 2.,
            ScrollAnchor::Bottom => top - (viewport - line_height),
        };
        let x = s.scroll_offset().x;
        s.set_scroll_offset(point(x, -y.max(px(0.))), cx);
    });
}
