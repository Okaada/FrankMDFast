//! Level-b (headless GPUI) tests of the insert surfaces hosted by the editor pane:
//! slash menu, `[[` and Ctrl+K completion, the table editor, the code block dialog,
//! the emoji picker and the Ctrl+. context action (docs/testing/ui-test-plan.md
//! §7 notes_test table rows, §8 app_slash_commands, §9 code_dialog, §10
//! emoji_picker, §6 codemirror_slash_commands).
#![allow(missing_docs, clippy::unwrap_used)]

extern crate gpui_kit as gpui;

#[path = "editor_support/mod.rs"]
mod editor_support;

use std::cell::RefCell;
use std::rc::Rc;

use editor_support::{Fixture, open};
use frankmd_app::editor::{EditorRequest, EditorSettings};
use gpui_kit::{ClipboardItem, TestAppContext};

fn run(f: &Fixture, cx: &mut TestAppContext, command: &'static str) {
    f.with(cx, |pane, window, cx| {
        pane.update(cx, |p, cx| assert!(p.run_command(command, window, cx)))
    });
    cx.run_until_parked();
}

fn requests(f: &Fixture, cx: &mut TestAppContext) -> Rc<RefCell<Vec<EditorRequest>>> {
    let log: Rc<RefCell<Vec<EditorRequest>>> = Rc::default();
    let sink = Rc::clone(&log);
    cx.update(|cx| {
        cx.subscribe(&f.pane, move |_, ev: &EditorRequest, _| {
            sink.borrow_mut().push(ev.clone())
        })
        .detach()
    });
    log
}

// ----- slash menu -------------------------------------------------------------------------------

// adapts: codemirror_slash_commands.test.js:137 "renders the semantic SVG icons inside the completion menu"
// adapts: codemirror_slash_commands.test.js:167 "moves through slash options with arrow keys and accepts the selected command with Enter"
#[gpui_kit::test]
fn slash_menu_opens_moves_and_applies(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    f.type_text(cx, "/");
    assert!(f.is_shown(cx, "slash-menu"));
    assert!(f.is_shown(cx, "slash-menu-row-0"));
    // Down to Heading 2, up, down again.
    f.press(cx, "down up down");
    f.press(cx, "enter");
    assert!(!f.is_shown(cx, "slash-menu"));
    assert_eq!(f.text(cx), "## ");
}

// adapts: codemirror_slash_commands.test.js:207 "dismisses slash options with Escape and keeps the query unchanged"
#[gpui_kit::test]
fn escape_dismisses_the_slash_menu_and_keeps_the_query(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    f.type_text(cx, "/table");
    assert!(f.is_shown(cx, "slash-menu"));
    f.press(cx, "escape");
    assert!(!f.is_shown(cx, "slash-menu"));
    assert_eq!(f.text(cx), "/table");
    // Typing on doesn't bring back the dismissed menu.
    f.type_text(cx, "x");
    assert!(!f.is_shown(cx, "slash-menu"));
}

#[gpui_kit::test]
fn slash_does_not_trigger_inside_a_path(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    f.type_text(cx, "docs/a");
    assert!(!f.is_shown(cx, "slash-menu"));
}

// adapts: codemirror_slash_commands.test.js:259 "closes the completion synchronously before a dialog takes focus"
// adapts: app_slash_commands.test.js:73 "replaces the saved query when the table editor inserts a new table"
#[gpui_kit::test]
fn slash_table_opens_the_table_editor_and_replaces_the_query(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    f.type_text(cx, "Before /table");
    assert!(f.is_shown(cx, "slash-menu"));
    f.press(cx, "enter");
    assert!(!f.is_shown(cx, "slash-menu"));
    assert!(f.is_shown(cx, "dialog-table"));
    assert!(f.has_focus(cx, "table-cell-0-0") || f.has_focus(cx, "dialog-table"));
    // The document is untouched until Insert.
    assert_eq!(f.text(cx), "Before /table");
    f.press(cx, "ctrl-enter");
    assert!(!f.is_shown(cx, "dialog-table"));
    let text = f.text(cx);
    assert!(
        text.starts_with("Before\n\n| Header 1 | Header 2 | Header 3 |\n| -------- |"),
        "{text:?}"
    );
    assert!(!text.contains("/table"));
}

// ----- table editor -------------------------------------------------------------------------------

// adapts: notes_test.rb:469 "table editor dialog opens and shows grid"
// adapts: table_editor_controller.test.js:334 "renders correct number of rows and cells"
#[gpui_kit::test]
fn insert_table_shows_a_three_by_three_grid(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    run(&f, cx, "insert.table");
    assert!(f.is_shown(cx, "dialog-table"));
    assert!(f.is_shown(cx, "table-cell-2-2"));
    assert!(!f.is_shown(cx, "table-cell-3-0"));
    assert!(!f.is_shown(cx, "table-cell-0-3"));
    // Escape cancels without editing.
    f.press(cx, "escape");
    assert!(!f.is_shown(cx, "dialog-table"));
    assert_eq!(f.text(cx), "");
}

// adapts: notes_test.rb:526 "inserting a table adds markdown to content"
// adapts: table_editor_controller.test.js:280 "addRow delegates to incrementRowCount" (Tab in the last cell)
#[gpui_kit::test]
fn typing_in_cells_and_inserting_is_one_undo_step(cx: &mut TestAppContext) {
    let f = open(cx, "intro", EditorSettings::default());
    f.select(cx, 5..5);
    run(&f, cx, "insert.table");
    f.with(cx, |pane, window, cx| {
        pane.update(cx, |p, cx| p.insert_dialog().unwrap().focus(window, cx))
    });
    f.press(cx, "ctrl-a");
    f.type_text(cx, "Name");
    // Tab from the last cell adds a row.
    for _ in 0..9 {
        f.press(cx, "tab");
    }
    assert!(f.is_shown(cx, "table-cell-3-0"));
    f.press(cx, "ctrl-enter");
    let text = f.text(cx);
    assert!(text.starts_with("intro\n\n| Name"), "{text:?}");
    assert_eq!(text.lines().filter(|l| l.starts_with('|')).count(), 5);
    f.press(cx, "ctrl-z");
    assert_eq!(f.text(cx), "intro");
}

// adapts: notes_test.rb:554 "editing existing table in content"
// adapts: notes_test.rb:647 "move row down via context menu" (Alt+↓)
#[gpui_kit::test]
fn context_action_edits_the_table_under_the_caret(cx: &mut TestAppContext) {
    let doc = "| Col1 | Col2 |\n| :--- | ---: |\n| A | B |\n| C | D |\n\nafter";
    let f = open(cx, doc, EditorSettings::default());
    f.select(cx, 3..3);
    f.press(cx, "ctrl-.");
    assert!(f.is_shown(cx, "dialog-table"));
    // Focus row 1 (A) and move it down.
    f.with(cx, |pane, window, cx| {
        pane.update(cx, |p, cx| p.insert_dialog().unwrap().focus(window, cx))
    });
    f.press(cx, "tab tab alt-down");
    f.press(cx, "ctrl-enter");
    assert_eq!(
        f.text(cx),
        "| Col1 | Col2 |\n| :--- | ---: |\n| C    | D    |\n| A    | B    |\n\nafter"
    );
}

// ----- code block -------------------------------------------------------------------------------------

// adapts: code_dialog_controller.test.js:68 "opens dialog with empty fields for new code block"
// adapts: code_dialog_controller.test.js:128 "selects first suggestion on Tab when suggestions visible"
// adapts: code_dialog_controller.test.js:149 "calls insert on Ctrl+Enter"
#[gpui_kit::test]
fn code_dialog_autocompletes_and_inserts(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    run(&f, cx, "insert.code_block");
    assert!(f.is_shown(cx, "dialog-code-block"));
    f.type_text(cx, "pyth");
    assert!(f.is_shown(cx, "code-lang-suggestions"));
    assert!(f.is_shown(cx, "code-lang-python"));
    f.press(cx, "tab");
    assert!(!f.is_shown(cx, "code-lang-suggestions"));
    f.press(cx, "ctrl-enter");
    assert!(!f.is_shown(cx, "dialog-code-block"));
    assert_eq!(f.text(cx), "```python\n\n```");
    assert_eq!(f.selection(cx), 10..10);
}

// adapts: code_dialog_controller.test.js:141 "hides suggestions on Escape"
// adapts: code_dialog_controller.test.js:96 "closes the dialog"
#[gpui_kit::test]
fn escape_hides_suggestions_then_closes(cx: &mut TestAppContext) {
    let f = open(cx, "x", EditorSettings::default());
    run(&f, cx, "insert.code_block");
    f.type_text(cx, "ja");
    assert!(f.is_shown(cx, "code-lang-suggestions"));
    f.press(cx, "escape");
    assert!(!f.is_shown(cx, "code-lang-suggestions"));
    assert!(f.is_shown(cx, "dialog-code-block"));
    f.press(cx, "escape");
    assert!(!f.is_shown(cx, "dialog-code-block"));
    assert_eq!(f.text(cx), "x");
    assert!(f.has_focus(cx, "editor"));
}

// adapts: code_dialog_controller.test.js:280 "shows confirmation for unrecognized language"
#[gpui_kit::test]
fn unknown_language_shows_the_inline_warning(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    run(&f, cx, "insert.code_block");
    f.type_text(cx, "pyhton");
    f.press(cx, "ctrl-enter");
    assert!(f.is_shown(cx, "code-unknown-language"));
    assert_eq!(f.text(cx), "");
    f.with(cx, |_, window, cx| {
        use gpui_kit::test::TestWindowExt as _;
        window.click("code-use-suggested", cx)
    });
    cx.run_until_parked();
    assert_eq!(f.text(cx), "```python\n\n```");
}

// adapts: code_dialog_controller.test.js:77 "opens dialog with existing code block for edit mode"
#[gpui_kit::test]
fn context_action_in_a_fence_updates_it_in_place(cx: &mut TestAppContext) {
    let doc = "before\n\n```javascript\nconst x = 1;\n```\n\nafter";
    let f = open(cx, doc, EditorSettings::default());
    f.select(cx, 25..25);
    f.press(cx, "ctrl-.");
    assert!(f.is_shown(cx, "dialog-code-block"));
    f.press(cx, "ctrl-enter");
    assert_eq!(f.text(cx), doc);
}

// ----- emoji picker -----------------------------------------------------------------------------------

// adapts: emoji_picker_controller.test.js:497 "selects current item and dispatches event"
// adapts: emoji_picker_controller.test.js:513 "dispatches emoji with shortcode format"
#[gpui_kit::test]
fn emoji_picker_searches_and_inserts_a_shortcode(cx: &mut TestAppContext) {
    let f = open(cx, "Hi ", EditorSettings::default());
    f.select(cx, 3..3);
    f.press(cx, "ctrl-shift-e");
    assert!(f.is_shown(cx, "emoji-picker"));
    f.type_text(cx, "heart");
    f.press(cx, "enter");
    assert!(!f.is_shown(cx, "emoji-picker"));
    // The first match is 😉: its emojibase tags include "heartbreaker" (FrankMD's
    // localized search matches tags too), and it comes first in EMOJI_DATA order.
    assert_eq!(f.text(cx), "Hi :wink:");
}

// adapts: emoji_picker_controller.test.js:135 "closes the dialog"
// adapts: emoji_picker_controller.test.js:165 "switches to emoticons tab" (Ctrl+Tab)
#[gpui_kit::test]
fn emoji_picker_tabs_and_escape(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    f.press(cx, "ctrl-shift-e");
    f.press(cx, "ctrl-tab");
    f.press(cx, "enter");
    let text = f.text(cx);
    assert!(
        !text.is_empty() && !text.starts_with(':'),
        "an emoticon is raw text: {text:?}"
    );
    f.press(cx, "ctrl-shift-e");
    f.press(cx, "escape");
    assert!(!f.is_shown(cx, "emoji-picker"));
    assert_eq!(f.text(cx), text);
}

// adapts: app_slash_commands.test.js:85 "replaces the saved query with inline emoji text"
#[gpui_kit::test]
fn slash_emoji_replaces_the_query(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    f.type_text(cx, "a /emoji");
    f.press(cx, "enter");
    assert!(f.is_shown(cx, "emoji-picker"));
    f.press(cx, "enter");
    let text = f.text(cx);
    assert!(
        text.starts_with("a :") && !text.contains("/emoji"),
        "{text:?}"
    );
}

// ----- links ------------------------------------------------------------------------------------------

// adapts: codemirror_wikilink.test.js "completes note names after [["
#[gpui_kit::test]
fn wikilink_completion_inserts_the_note_path(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    f.pane.update(cx, |p, _| {
        p.set_vault_notes(vec!["notes/My Note.md".into(), "other.md".into()])
    });
    f.type_text(cx, "see [[my");
    assert!(f.is_shown(cx, "wikilink-completion"));
    f.press(cx, "enter");
    assert_eq!(f.text(cx), "see [[notes/My Note]]");
}

// ui-spec §5.5 "Link completion" (new on top of FE-058).
#[gpui_kit::test]
fn ctrl_k_offers_the_clipboard_url(cx: &mut TestAppContext) {
    let f = open(cx, "word", EditorSettings::default());
    cx.update(|cx| {
        cx.write_to_clipboard(ClipboardItem::new_string("https://example.com/x".into()))
    });
    f.select(cx, 0..4);
    f.press(cx, "ctrl-k");
    assert!(f.is_shown(cx, "link-completion"));
    f.press(cx, "enter");
    assert_eq!(f.text(cx), "[word](https://example.com/x)");
    // Without a choice, Esc keeps FrankMD's template with `url` selected.
    let g = open(cx, "w", EditorSettings::default());
    g.select(cx, 0..1);
    g.press(cx, "ctrl-k escape");
    assert_eq!(g.text(cx), "[w](url)");
    assert_eq!(g.selection(cx), 4..7);
}

// ui-spec §6.2 "Context action": links go to the shell.
#[gpui_kit::test]
fn context_action_on_links_requests_the_shell(cx: &mut TestAppContext) {
    let f = open(
        cx,
        "a [doc](https://x.dev) [[Note]] ![p](p.png)",
        EditorSettings::default(),
    );
    let log = requests(&f, cx);
    f.select(cx, 4..4);
    f.press(cx, "ctrl-.");
    f.select(cx, 26..26);
    f.press(cx, "ctrl-.");
    f.select(cx, 34..34);
    f.press(cx, "ctrl-.");
    assert_eq!(
        *log.borrow(),
        vec![
            EditorRequest::OpenLink("https://x.dev".into()),
            EditorRequest::OpenWikilink("Note".into()),
            EditorRequest::ViewImage("p.png".into()),
        ]
    );
}

// app_slash_commands.test.js:151 "does not insert %s when the active file is not Markdown"
#[gpui_kit::test]
fn non_markdown_files_get_no_slash_menu_or_dialogs(cx: &mut TestAppContext) {
    let f = open(cx, "", EditorSettings::default());
    f.pane.update(cx, |p, cx| {
        p.set_note_context(Some(".fed".into()), false, cx)
    });
    f.type_text(cx, "/");
    assert!(!f.is_shown(cx, "slash-menu"));
    run(&f, cx, "insert.table");
    assert!(!f.is_shown(cx, "dialog-table"));
}
