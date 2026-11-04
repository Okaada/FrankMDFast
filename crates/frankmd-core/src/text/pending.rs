//! Dialog insertions as single [`Edit`]s, and the pending `/query` range a
//! slash command leaves behind while its dialog is open.
//!
//! **[COMPAT]** `app_controller.js` `pendingSlashInsertionRange` /
//! `getPendingSlashInsertionRange` and `codemirror_content_insertion.js`:
//! - a slash command that opens a dialog remembers `{action, from, to, query}`;
//! - on insert, the content replaces `from..to` only if the document still has
//!   `query` there; otherwise the insertion is **dropped** (never clobbers text the
//!   user changed while the dialog was open);
//! - closing the dialog forgets the range.
//!
//! The functions here run the ported insertion helpers ([`super::insertion`]) on
//! a scratch copy of the text and return the change as one [`Edit`], so a dialog
//! insert is exactly one undo step in the editor.

use std::ops::Range;

use super::edit::Edit;
use super::insertion::{
    BlockInsert, CursorInfo, CursorPosition, EditorSurface, InlineInsert, Selection,
    insert_block_content, insert_image, insert_inline_content,
};

/// The dialog a pending slash range belongs to (JS `action`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PendingAction {
    /// `/table`.
    Table,
    /// `/image`.
    Image,
    /// `/video`.
    Video,
    /// `/emoji`.
    Emoji,
}

/// A `/query` waiting for its dialog to insert (byte offsets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingInsertion {
    /// Which dialog opened it.
    pub action: PendingAction,
    /// The slash and its query, `from..to`.
    pub range: Range<usize>,
    /// The text that was at `range` (starts with `/`).
    pub query: String,
}

impl PendingInsertion {
    /// Whether `text` still has the query at the remembered range.
    pub fn is_current(&self, text: &str) -> bool {
        text.get(self.range.clone()) == Some(self.query.as_str())
    }
}

/// Where a block (table, code, video) goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockTarget {
    /// Edit mode: replace exactly this range (the table / fence being edited), no padding.
    Replace(Range<usize>),
    /// At the caret (`selection.end`), padded with blank lines.
    Caret,
    /// Over a pending slash query (validated first), padded with blank lines.
    Slash(PendingInsertion),
}

/// The edit inserting block `content`, with the caret `cursor_offset` bytes
/// into the content (`None` = after the insertion). `None` when the content is
/// empty or a [`BlockTarget::Slash`] query went stale.
pub fn block_edit(
    text: &str,
    selection: Range<usize>,
    content: &str,
    target: &BlockTarget,
    cursor_offset: Option<usize>,
) -> Option<Edit> {
    let mut opts = BlockInsert { cursor_offset, ..BlockInsert::default() };
    match target {
        BlockTarget::Replace(range) => opts.edit = Some(range.clone()),
        BlockTarget::Caret => {}
        BlockTarget::Slash(pending) => {
            if !pending.is_current(text) {
                return None;
            }
            opts.replace = Some(pending.range.clone());
        }
    }
    let mut surface = Scratch::new(text, selection);
    insert_block_content(Some(&mut surface), content, &opts);
    surface.into_edit(text)
}

/// The edit inserting inline `content` (emoji, icon, kaomoji) over the
/// selection, or over a pending slash query (`None` when it went stale).
pub fn inline_edit(
    text: &str,
    selection: Range<usize>,
    content: &str,
    slash: Option<&PendingInsertion>,
) -> Option<Edit> {
    let mut opts = InlineInsert::default();
    if let Some(pending) = slash {
        if !pending.is_current(text) {
            return None;
        }
        opts.replace = Some(pending.range.clone());
    }
    let mut surface = Scratch::new(text, selection);
    insert_inline_content(Some(&mut surface), content, &opts);
    surface.into_edit(text)
}

/// The edit inserting image markdown padded with single newlines (**[COMPAT]**
/// `insertImage`), over the selection or a pending `/image` query (`None` when
/// it went stale).
pub fn image_edit(
    text: &str,
    selection: Range<usize>,
    markdown: &str,
    slash: Option<&PendingInsertion>,
) -> Option<Edit> {
    if slash.is_some_and(|p| !p.is_current(text)) {
        return None;
    }
    let mut surface = Scratch::new(text, selection);
    insert_image(Some(&mut surface), markdown, slash.map(|p| p.range.clone()));
    surface.into_edit(text)
}

/// An [`EditorSurface`] over a `String`, recording the changed span.
struct Scratch {
    text: String,
    selection: Range<usize>,
    changed: bool,
}

impl Scratch {
    fn new(text: &str, selection: Range<usize>) -> Self {
        let len = text.len();
        let clamp = |o: usize| o.min(len);
        Self {
            text: text.to_owned(),
            selection: clamp(selection.start)..clamp(selection.end),
            changed: false,
        }
    }

    /// The whole-text replacement (minimized) or `None` when nothing changed.
    fn into_edit(self, old: &str) -> Option<Edit> {
        if !self.changed {
            return None;
        }
        Some(Edit { range: 0..old.len(), text: self.text, selection: self.selection }.minimized(old))
    }

    fn splice(&mut self, from: usize, to: usize, insert: &str) {
        let len = self.text.len();
        let (from, to) = (from.min(len), to.min(len));
        if self.text.is_char_boundary(from) && self.text.is_char_boundary(to) && from <= to {
            self.text.replace_range(from..to, insert);
            self.changed = true;
        }
    }
}

impl EditorSurface for Scratch {
    fn text(&self) -> String {
        self.text.clone()
    }

    fn set_text(&mut self, text: &str) {
        self.text = text.to_owned();
        self.changed = true;
    }

    fn selection(&self) -> Selection {
        Selection {
            from: self.selection.start,
            to: self.selection.end,
            text: self.text.get(self.selection.clone()).unwrap_or_default().to_owned(),
        }
    }

    fn set_selection(&mut self, from: usize, to: usize) {
        self.selection = from..to;
    }

    fn cursor_position(&self) -> CursorPosition {
        let offset = self.selection.end;
        let before = &self.text[..offset.min(self.text.len())];
        let line = before.matches('\n').count() + 1;
        let column = before.len() - before.rfind('\n').map_or(0, |i| i + 1) + 1;
        CursorPosition { line, column, offset }
    }

    fn cursor_info(&self) -> CursorInfo {
        CursorInfo {
            current_line: self.cursor_position().line,
            total_lines: self.text.matches('\n').count() + 1,
        }
    }

    fn replace_range(&mut self, text: &str, from: usize, to: usize) {
        self.splice(from, to, text);
    }

    fn insert_at(&mut self, pos: usize, text: &str) {
        self.splice(pos, pos, text);
    }

    fn focus(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, edit: &Edit) -> String {
        let mut s = text.to_owned();
        edit.apply_to(&mut s);
        s
    }

    fn pending(action: PendingAction, range: Range<usize>, query: &str) -> PendingInsertion {
        PendingInsertion { action, range, query: query.to_owned() }
    }

    const TABLE: &str = "| A |\n| --- |\n| 1 |";

    #[test]
    fn slash_table_replaces_query_and_spaces_with_blank_lines() {
        // adapts: app_slash_commands.test.js:73 "replaces the saved query when the table editor inserts a new table"
        let text = "before /table after";
        let p = pending(PendingAction::Table, 7..13, "/table");
        let e = block_edit(text, 13..13, TABLE, &BlockTarget::Slash(p), None).expect("edit");
        assert_eq!(apply(text, &e), format!("before\n\n{TABLE}\n\nafter"));
        assert_eq!(e.selection.start, "before\n\n".len() + TABLE.len() + 2);
    }

    #[test]
    fn slash_emoji_replaces_exactly_the_query() {
        // adapts: app_slash_commands.test.js:85 "replaces the saved query with inline emoji text"
        let text = "before /emoji after";
        let p = pending(PendingAction::Emoji, 7..13, "/emoji");
        let e = inline_edit(text, 13..13, ":smile:", Some(&p)).expect("edit");
        assert_eq!(apply(text, &e), "before :smile: after");
        assert_eq!(e.selection, 14..14);
    }

    #[test]
    fn slash_video_is_a_block_insert() {
        // adapts: app_slash_commands.test.js:95 "replaces the saved query when the video dialog returns an embed"
        let text = "before /video after";
        let p = pending(PendingAction::Video, 7..13, "/video");
        let e = block_edit(text, 13..13, "<video></video>", &BlockTarget::Slash(p), None).expect("edit");
        assert_eq!(apply(text, &e), "before\n\n<video></video>\n\nafter");
    }

    #[test]
    fn edit_mode_replaces_the_table_range_only() {
        // adapts: app_slash_commands.test.js:105 "keeps an existing table edit range when handling a table action"
        let text = format!("{TABLE}\n\ntext");
        let e = block_edit(&text, 2..2, "| B |\n| --- |", &BlockTarget::Replace(0..TABLE.len()), None)
            .expect("edit");
        assert_eq!(apply(&text, &e), "| B |\n| --- |\n\ntext");
        assert_eq!(e.selection, 13..13);
    }

    #[test]
    fn stale_query_drops_the_insertion() {
        // adapts: app_slash_commands.test.js:172 "drops a stale query range if the document changed while the dialog was open"
        let p = pending(PendingAction::Image, 7..13, "/image");
        assert!(block_edit("before /imaXe after", 0..0, "x", &BlockTarget::Slash(p.clone()), None).is_none());
        assert!(inline_edit("before /imaXe after", 0..0, "x", Some(&p)).is_none());
        assert!(p.is_current("before /image after"));
    }

    #[test]
    fn caret_insert_pads_blank_lines_and_honours_cursor_offset() {
        let text = "para";
        let fence = "```js\n\n```";
        let e = block_edit(text, 4..4, fence, &BlockTarget::Caret, Some(6)).expect("edit");
        assert_eq!(apply(text, &e), "para\n\n```js\n\n```");
        assert_eq!(e.selection, 12..12);
        let e = block_edit("", 0..0, fence, &BlockTarget::Caret, None).expect("edit");
        assert_eq!(apply("", &e), fence);
    }

    #[test]
    fn slash_image_uses_single_newlines() {
        // adapts: app_slash_commands.test.js:63 "replaces the saved query when the image picker inserts successfully"
        let text = "before /image after";
        let p = pending(PendingAction::Image, 7..13, "/image");
        let e = image_edit(text, 13..13, "![photo](photo.png)", Some(&p)).expect("edit");
        assert_eq!(apply(text, &e), "before\n![photo](photo.png)\nafter");
        // adapts: app_slash_commands.test.js:140 "clears the pending range on dialog close so later toolbar insertions use the current selection"
        let e = image_edit("ab", 1..1, "![toolbar](toolbar.png)", None).expect("edit");
        assert_eq!(apply("ab", &e), "a\n![toolbar](toolbar.png)\nb");
    }

    #[test]
    fn inline_insert_replaces_selection() {
        let e = inline_edit("Hi there", 3..8, ":heart:", None).expect("edit");
        assert_eq!(apply("Hi there", &e), "Hi :heart:");
        assert_eq!(e.selection, 10..10);
        assert!(inline_edit("x", 0..0, "", None).is_none());
    }
}
