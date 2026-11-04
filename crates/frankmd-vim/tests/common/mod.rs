//! In-process fake editor for driving the vim engine in tests.
#![allow(dead_code, unreachable_pub)]

use frankmd_vim::buffer::{ScrollAnchor, VimBuffer};
use frankmd_vim::keymap::Key;
use frankmd_vim::{Outcome, VimState};
use std::borrow::Cow;
use std::ops::Range;

/// String-backed buffer with a linear undo stack.
#[derive(Debug, Default)]
pub struct FakeBuffer {
    pub text: String,
    pub sel: Range<usize>,
    undo: Vec<(String, Range<usize>)>,
    redo: Vec<(String, Range<usize>)>,
}

impl FakeBuffer {
    pub fn new(text: &str) -> Self {
        Self { text: text.to_owned(), ..Self::default() }
    }
}

impl VimBuffer for FakeBuffer {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.text)
    }
    fn selection(&self) -> Range<usize> {
        self.sel.clone()
    }
    fn set_selection(&mut self, range: Range<usize>) {
        self.sel = range;
    }
    fn replace(&mut self, range: Range<usize>, text: &str) {
        self.undo.push((self.text.clone(), self.sel.clone()));
        self.redo.clear();
        self.text.replace_range(range.clone(), text);
        let end = range.start + text.len();
        self.sel = end..end;
    }
    fn transact(&mut self, f: &mut dyn FnMut(&mut dyn VimBuffer)) {
        f(self);
    }
    fn undo(&mut self) {
        if let Some((t, s)) = self.undo.pop() {
            self.redo.push((std::mem::replace(&mut self.text, t), std::mem::replace(&mut self.sel, s)));
        }
    }
    fn redo(&mut self) {
        if let Some((t, s)) = self.redo.pop() {
            self.undo.push((std::mem::replace(&mut self.text, t), std::mem::replace(&mut self.sel, s)));
        }
    }
    fn visible_lines(&self) -> Range<usize> {
        0..self.text.lines().count().max(1)
    }
    fn scroll_to_line(&mut self, _line: usize, _anchor: ScrollAnchor) {}
}

/// Feed a sequence of plain characters (each char one key; `\n` = enter, `\x1b` = escape).
pub fn feed_str(vim: &mut VimState, buf: &mut FakeBuffer, keys: &str) -> Vec<Outcome> {
    keys.chars()
        .map(|c| {
            let name = match c {
                '\n' => "enter".to_owned(),
                '\x1b' => "escape".to_owned(),
                c => c.to_string(),
            };
            vim.feed(&Key::plain(&name), false, buf)
        })
        .collect()
}
