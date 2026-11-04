//! Port of test/javascript/lib/codemirror_content_insertion.test.js.
//! Offsets: the original cases are ASCII except the emoji ones, where the
//! Rust API's byte offsets differ from JS UTF-16 (noted inline).
#![allow(clippy::unwrap_used)]

#[path = "text_common/fake_editor.rs"]
mod fake_editor;

use fake_editor::{Call, FakeEditor};
use frankmd_core::text::insertion::{
    BlockInsert, EditorSurface, InlineInsert, insert_block_content, insert_code_block, insert_image,
    insert_inline_content, insert_table, insert_video_embed,
};

const T: &str = "codemirror_content_insertion.test.js";

fn block(ed: &mut FakeEditor, content: &str, opts: &BlockInsert) -> usize {
    insert_block_content(dynm(ed), content, opts)
}

fn dynm(ed: &mut FakeEditor) -> Option<&mut dyn EditorSurface> {
    Some(ed)
}

fn ins(pos: usize, s: &str) -> Call {
    Call::InsertAt(pos, s.to_owned())
}

fn rep(s: &str, from: usize, to: usize) -> Call {
    Call::ReplaceRange(s.to_owned(), from, to)
}

fn edit(start: usize, end: usize) -> BlockInsert {
    BlockInsert { edit: Some(start..end), ..BlockInsert::default() }
}

// ---- insertBlockContent ----------------------------------------------------

// port: test/javascript/lib/codemirror_content_insertion.test.js:26 "returns 0 when controller is null"
#[test]
fn block_returns_0_when_controller_is_null() {
    assert_eq!(insert_block_content(None, "content", &BlockInsert::default()), 0, "{T}");
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:30 "returns 0 when content is empty"
#[test]
fn block_returns_0_when_content_is_empty() {
    let mut ed = FakeEditor::default();
    assert_eq!(block(&mut ed, "", &BlockInsert::default()), 0);
    assert!(ed.calls.is_empty(), "no edit for empty content: {:?}", ed.calls);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:34 "inserts content at cursor in empty document"
#[test]
fn block_inserts_content_at_cursor_in_empty_document() {
    let mut ed = FakeEditor::with_text("").cursor_at(0);
    block(&mut ed, "table content", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(0, "table content")]);
    assert!(!ed.selections().is_empty(), "setSelection called");
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:44 "adds double newline prefix when text before does not end with newlines"
#[test]
fn block_adds_double_newline_prefix() {
    let mut ed = FakeEditor::with_text("some text").cursor_at(9);
    block(&mut ed, "table", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(9, "\n\ntable")]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:53 "adds single newline prefix when text before ends with one newline"
#[test]
fn block_adds_single_newline_prefix() {
    let mut ed = FakeEditor::with_text("some text\n").cursor_at(10);
    block(&mut ed, "table", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(10, "\ntable")]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:62 "adds no newline prefix when text before ends with double newlines"
#[test]
fn block_adds_no_newline_prefix_after_blank_line() {
    let mut ed = FakeEditor::with_text("some text\n\n").cursor_at(11);
    block(&mut ed, "table", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(11, "table")]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:71 "adds double newline suffix when text after does not start with newlines"
#[test]
fn block_adds_double_newline_suffix() {
    let mut ed = FakeEditor::with_text("beforeafter").cursor_at(6);
    block(&mut ed, "table", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(6, "\n\ntable\n\n")]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:80 "adds single newline suffix when text after starts with one newline"
#[test]
fn block_adds_single_newline_suffix() {
    let mut ed = FakeEditor::with_text("before\nafter").cursor_at(6);
    block(&mut ed, "table", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(6, "\n\ntable\n")]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:89 "adds no newline suffix when text after starts with double newlines"
#[test]
fn block_adds_no_newline_suffix_before_blank_line() {
    let mut ed = FakeEditor::with_text("before\n\nafter").cursor_at(6);
    block(&mut ed, "table", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(6, "\n\ntable")]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:98 "replaces a slash query and preserves block spacing around it"
#[test]
fn block_replaces_slash_query_preserving_spacing() {
    let mut ed = FakeEditor::with_text("before /table after");
    block(&mut ed, "table", &BlockInsert { replace: Some(7..13), ..BlockInsert::default() });
    assert_eq!(ed.replaces(), vec![&rep("\n\ntable\n\n", 6, 14)]);
    assert!(ed.inserts().is_empty());
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:107 "uses replaceRange in edit mode"
#[test]
fn block_uses_replace_range_in_edit_mode() {
    let mut ed = FakeEditor::default();
    block(&mut ed, "new table", &edit(10, 50));
    assert_eq!(ed.replaces(), vec![&rep("new table", 10, 50)]);
    assert!(ed.inserts().is_empty());
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:118 "positions cursor at end in edit mode"
#[test]
fn block_positions_cursor_at_end_in_edit_mode() {
    let mut ed = FakeEditor::default();
    let pos = block(&mut ed, "new table", &edit(10, 50));
    assert_eq!(ed.selections(), vec![&Call::SetSelection(19, 19)]);
    assert_eq!(pos, 19);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:128 "respects cursorOffset in edit mode"
#[test]
fn block_respects_cursor_offset_in_edit_mode() {
    let mut ed = FakeEditor::default();
    let opts = BlockInsert { cursor_offset: Some(6), ..edit(10, 50) };
    block(&mut ed, "```js\ncode\n```", &opts);
    assert_eq!(ed.selections(), vec![&Call::SetSelection(16, 16)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:139 "respects cursorOffset in insert mode"
#[test]
fn block_respects_cursor_offset_in_insert_mode() {
    let mut ed = FakeEditor::with_text("").cursor_at(0);
    let opts = BlockInsert { cursor_offset: Some(6), ..BlockInsert::default() };
    block(&mut ed, "```js\ncode\n```", &opts);
    assert_eq!(ed.selections(), vec![&Call::SetSelection(6, 6)]);
}

// ---- insertInlineContent ---------------------------------------------------

// port: test/javascript/lib/codemirror_content_insertion.test.js:152 "returns 0 when controller is null"
#[test]
fn inline_returns_0_when_controller_is_null() {
    assert_eq!(insert_inline_content(None, "😀", &InlineInsert::default()), 0);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:156 "returns 0 when content is empty"
#[test]
fn inline_returns_0_when_content_is_empty() {
    let mut ed = FakeEditor::default();
    assert_eq!(insert_inline_content(dynm(&mut ed), "", &InlineInsert::default()), 0);
    assert!(ed.calls.is_empty());
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:160 "replaces selection by default"
#[test]
fn inline_replaces_selection_by_default() {
    let mut ed = FakeEditor::with_text("01234hello").selecting(5, 10);
    let pos = insert_inline_content(dynm(&mut ed), "😀", &InlineInsert::default());
    assert_eq!(ed.replaces(), vec![&rep("😀", 5, 10)]);
    // Byte offsets: 😀 is 4 UTF-8 bytes (JS: 2 UTF-16 units → 7).
    assert_eq!(pos, 9);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:168 "inserts at position when replaceSelection is false"
#[test]
fn inline_inserts_at_position_when_replace_selection_false() {
    let mut ed = FakeEditor::with_text("01234hello").selecting(5, 10);
    let opts = InlineInsert { replace_selection: false, replace: None };
    insert_inline_content(dynm(&mut ed), "😀", &opts);
    assert_eq!(ed.inserts(), vec![&ins(5, "😀")]);
    assert!(ed.replaces().is_empty());
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:176 "positions cursor after inserted content"
#[test]
fn inline_positions_cursor_after_inserted_content() {
    let mut ed = FakeEditor::with_text("01234hello").selecting(5, 10);
    insert_inline_content(dynm(&mut ed), "emoji", &InlineInsert::default());
    assert_eq!(ed.selections(), vec![&Call::SetSelection(10, 10)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:184 "replaces a specific slash query range"
#[test]
fn inline_replaces_specific_slash_query_range() {
    let mut ed = FakeEditor::default();
    let opts = InlineInsert { replace: Some(5..11), ..InlineInsert::default() };
    let pos = insert_inline_content(dynm(&mut ed), ":smile:", &opts);
    assert_eq!(ed.replaces(), vec![&rep(":smile:", 5, 11)]);
    assert_eq!(ed.selections(), vec![&Call::SetSelection(12, 12)]);
    assert_eq!(pos, 12);
}

// ---- insertImage ------------------------------------------------------------

// port: test/javascript/lib/codemirror_content_insertion.test.js:193 "returns 0 when controller is null"
#[test]
fn image_returns_0_when_controller_is_null() {
    assert_eq!(insert_image(None, "![alt](url)", None), 0);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:197 "returns 0 when markdown is empty"
#[test]
fn image_returns_0_when_markdown_is_empty() {
    let mut ed = FakeEditor::default();
    assert_eq!(insert_image(dynm(&mut ed), "", None), 0);
    assert!(ed.calls.is_empty());
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:201 "inserts image markdown without extra newlines at start of document"
#[test]
fn image_no_extra_newlines_at_start_of_document() {
    let mut ed = FakeEditor::with_text("").selecting(0, 0);
    insert_image(dynm(&mut ed), "![alt](url)", None);
    assert_eq!(ed.replaces(), vec![&rep("![alt](url)", 0, 0)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:210 "adds newline before when text before does not end with newline"
#[test]
fn image_adds_newline_before() {
    let mut ed = FakeEditor::with_text("some text").selecting(9, 9);
    insert_image(dynm(&mut ed), "![alt](url)", None);
    assert_eq!(ed.replaces(), vec![&rep("\n![alt](url)", 9, 9)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:219 "adds newline after when text after does not start with newline"
#[test]
fn image_adds_newline_after() {
    let mut ed = FakeEditor::with_text("beforeafter").selecting(6, 6);
    insert_image(dynm(&mut ed), "![alt](url)", None);
    assert_eq!(ed.replaces(), vec![&rep("\n![alt](url)\n", 6, 6)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:228 "replaces selection when text is selected"
#[test]
fn image_replaces_selection() {
    let mut ed = FakeEditor::with_text("before[selected]after").selecting(6, 16);
    insert_image(dynm(&mut ed), "![alt](url)", None);
    assert_eq!(ed.replaces(), vec![&rep("\n![alt](url)\n", 6, 16)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:237 "replaces a specific slash query range"
#[test]
fn image_replaces_specific_slash_query_range() {
    let mut ed = FakeEditor::with_text("before /image after");
    insert_image(dynm(&mut ed), "![alt](url)", Some(7..13));
    assert_eq!(ed.replaces(), vec![&rep("\n![alt](url)\n", 6, 14)]);
}

// ---- insertTable ------------------------------------------------------------

// port: test/javascript/lib/codemirror_content_insertion.test.js:247 "delegates to insertBlockContent"
#[test]
fn table_delegates_to_insert_block_content() {
    let mut ed = FakeEditor::with_text("").cursor_at(0);
    let md = "| A | B |\n|---|---|\n| 1 | 2 |";
    insert_table(dynm(&mut ed), md, &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(0, md)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:257 "replaces a slash query with a video block at the saved range"
#[test]
fn video_replaces_slash_query_at_saved_range() {
    let mut ed = FakeEditor::with_text("before /video after");
    let opts = BlockInsert { replace: Some(7..13), ..BlockInsert::default() };
    insert_video_embed(dynm(&mut ed), "<video></video>", &opts);
    assert_eq!(ed.replaces(), vec![&rep("\n\n<video></video>\n\n", 6, 14)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:265 "passes edit mode options through"
#[test]
fn table_passes_edit_mode_options_through() {
    let mut ed = FakeEditor::default();
    insert_table(dynm(&mut ed), "| A | B |", &edit(5, 20));
    assert_eq!(ed.replaces(), vec![&rep("| A | B |", 5, 20)]);
}

// ---- insertCodeBlock ----------------------------------------------------------

// port: test/javascript/lib/codemirror_content_insertion.test.js:278 "inserts code block with correct cursor offset"
#[test]
fn code_block_correct_cursor_offset() {
    let mut ed = FakeEditor::with_text("").cursor_at(0);
    let cb = "```javascript\nconsole.log('hello')\n```";
    insert_code_block(dynm(&mut ed), cb, "javascript", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(0, cb)]);
    assert_eq!(ed.selections(), vec![&Call::SetSelection(14, 14)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:290 "handles empty language"
#[test]
fn code_block_handles_empty_language() {
    let mut ed = FakeEditor::with_text("").cursor_at(0);
    insert_code_block(dynm(&mut ed), "```\ncode\n```", "", &BlockInsert::default());
    assert_eq!(ed.selections(), vec![&Call::SetSelection(4, 4)]);
}

// port: test/javascript/lib/codemirror_content_insertion.test.js:301 "works in edit mode"
#[test]
fn code_block_works_in_edit_mode() {
    let mut ed = FakeEditor::default();
    let cb = "```python\nprint('hi')\n```";
    insert_code_block(dynm(&mut ed), cb, "python", &edit(10, 50));
    assert_eq!(ed.replaces(), vec![&rep(cb, 10, 50)]);
    assert_eq!(ed.selections(), vec![&Call::SetSelection(20, 20)]);
}

// ---- insertVideoEmbed ---------------------------------------------------------

// port: test/javascript/lib/codemirror_content_insertion.test.js:316 "delegates to insertBlockContent"
#[test]
fn video_delegates_to_insert_block_content() {
    let mut ed = FakeEditor::with_text("text").cursor_at(4);
    insert_video_embed(dynm(&mut ed), "{{< youtube abc123 >}}", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(4, "\n\n{{< youtube abc123 >}}")]);
}

// ---- Rust-only: byte-offset honesty around multi-byte text ------------------

// Not in the original suite: guards the UTF-16 → byte port of the padding math.
#[test]
fn block_padding_uses_byte_offsets_after_multibyte_text() {
    // "日本" = 6 bytes (JS: 2 units). Cursor after it, at end of text.
    let mut ed = FakeEditor::with_text("日本").cursor_at(6);
    let pos = block(&mut ed, "table", &BlockInsert::default());
    assert_eq!(ed.inserts(), vec![&ins(6, "\n\ntable")]);
    assert_eq!(pos, 6 + "\n\ntable".len());
}
