//! Recording fake of `EditorSurface` (the vitest `mockController`).
//! Include with `#[path = "text_common/fake_editor.rs"] mod fake_editor;`.
//! Mutating calls are only recorded (like `vi.fn()`), never applied.
#![allow(dead_code, unreachable_pub, missing_docs)]

use frankmd_core::text::insertion::{CursorInfo, CursorPosition, EditorSurface, Selection};

/// One recorded call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    SetText(String),
    SetSelection(usize, usize),
    ReplaceRange(String, usize, usize),
    InsertAt(usize, String),
    Focus,
}

/// Fake editor with canned return values.
#[derive(Debug, Default)]
pub struct FakeEditor {
    pub text: String,
    pub selection: Selection,
    pub cursor: CursorPosition,
    pub info: CursorInfo,
    pub calls: Vec<Call>,
    pub text_reads: std::cell::Cell<usize>,
}

impl FakeEditor {
    pub fn with_text(text: &str) -> Self {
        Self { text: text.to_owned(), ..Self::default() }
    }
    pub fn cursor_at(mut self, offset: usize) -> Self {
        self.cursor = CursorPosition { line: 1, column: 1, offset };
        self
    }
    pub fn selecting(mut self, from: usize, to: usize) -> Self {
        let text = self.text.get(from..to).unwrap_or_default().to_owned();
        self.selection = Selection { from, to, text };
        self
    }
    pub fn inserts(&self) -> Vec<&Call> {
        self.calls.iter().filter(|c| matches!(c, Call::InsertAt(..))).collect()
    }
    pub fn replaces(&self) -> Vec<&Call> {
        self.calls.iter().filter(|c| matches!(c, Call::ReplaceRange(..))).collect()
    }
    pub fn selections(&self) -> Vec<&Call> {
        self.calls.iter().filter(|c| matches!(c, Call::SetSelection(..))).collect()
    }
}

impl EditorSurface for FakeEditor {
    fn text(&self) -> String {
        self.text_reads.set(self.text_reads.get() + 1);
        self.text.clone()
    }
    fn set_text(&mut self, text: &str) {
        self.calls.push(Call::SetText(text.to_owned()));
    }
    fn selection(&self) -> Selection {
        self.selection.clone()
    }
    fn set_selection(&mut self, from: usize, to: usize) {
        self.calls.push(Call::SetSelection(from, to));
    }
    fn cursor_position(&self) -> CursorPosition {
        self.cursor
    }
    fn cursor_info(&self) -> CursorInfo {
        self.info
    }
    fn replace_range(&mut self, text: &str, from: usize, to: usize) {
        self.calls.push(Call::ReplaceRange(text.to_owned(), from, to));
    }
    fn insert_at(&mut self, pos: usize, text: &str) {
        self.calls.push(Call::InsertAt(pos, text.to_owned()));
    }
    fn focus(&mut self) {
        self.calls.push(Call::Focus);
    }
}
