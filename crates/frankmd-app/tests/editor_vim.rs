//! Level-b tests of the vim layer wired into the editor pane: key precedence over the
//! editor's own bindings, modes and status events, one undo step per command, the
//! app-owned Ctrl keys and ex commands (ui-spec §6.3; gpui-stack.md §6).
#![allow(missing_docs, clippy::unwrap_used)]

extern crate gpui_kit as gpui;

mod editor_support;

use editor_support::{open, vim};
use frankmd_app::editor::{EditorEvent, EditorSettings};
use frankmd_vim::{AppCommand, Mode};
use gpui_kit::TestAppContext;

fn mode(f: &editor_support::Fixture, cx: &mut TestAppContext) -> Option<Mode> {
    f.pane.read_with(cx, |p, _| p.vim_mode())
}

// The ordering invariant (FrankMD cc83922): in normal mode `j` moves the caret, it
// does not type "j".
#[gpui_kit::test]
fn normal_mode_keys_reach_vim_before_the_editor(cx: &mut TestAppContext) {
    let f = open(cx, "one\ntwo\nthree", vim());
    f.press(cx, "j");
    assert_eq!(f.text(cx), "one\ntwo\nthree");
    assert_eq!(f.selection(cx), 4..4);
    f.press(cx, "l l");
    assert_eq!(f.selection(cx), 6..6);
    // Arrows and Escape (bound by gpui-kit's Input) go to vim too.
    f.press(cx, "down");
    assert_eq!(f.selection(cx), 10..10);
}

#[gpui_kit::test]
fn insert_mode_types_through_the_editor_and_escape_returns_to_normal(cx: &mut TestAppContext) {
    let f = open(cx, "ab", vim());
    assert_eq!(mode(&f, cx), Some(Mode::Normal));
    f.take_events();
    f.press(cx, "i");
    assert_eq!(mode(&f, cx), Some(Mode::Insert));
    assert!(f.take_events().contains(&EditorEvent::VimStatusChanged));
    f.type_text(cx, "xy");
    assert_eq!(f.text(cx), "xyab");
    f.press(cx, "escape");
    assert_eq!(mode(&f, cx), Some(Mode::Normal));
    assert_eq!(
        f.selection(cx),
        1..1,
        "caret steps back onto the last typed char"
    );
}

// One vim command = one undo step (shadow buffer + single replace).
#[gpui_kit::test]
fn counted_delete_is_one_undo_step(cx: &mut TestAppContext) {
    let f = open(cx, "1\n2\n3\n4\n5", vim());
    f.press(cx, "3 d d");
    assert_eq!(f.text(cx), "4\n5");
    f.press(cx, "u");
    assert_eq!(f.text(cx), "1\n2\n3\n4\n5");
    f.press(cx, "ctrl-r");
    assert_eq!(f.text(cx), "4\n5");
}

#[gpui_kit::test]
fn substitute_all_is_one_undo_step(cx: &mut TestAppContext) {
    let f = open(cx, "a a\na", vim());
    for key in [":", "%", "s", "/", "a", "/", "b", "/", "g", "enter"] {
        f.press(cx, key);
    }
    assert_eq!(f.text(cx), "b b\nb");
    f.press(cx, "u");
    assert_eq!(f.text(cx), "a a\na");
}

#[gpui_kit::test]
fn dot_repeats_a_change_word(cx: &mut TestAppContext) {
    let f = open(cx, "one two", vim());
    f.press(cx, "c w");
    f.type_text(cx, "uno");
    f.press(cx, "escape w .");
    assert_eq!(f.text(cx), "uno uno");
    // The repeat is one step.
    f.press(cx, "u");
    assert_eq!(f.text(cx), "uno two");
}

#[gpui_kit::test]
fn visual_mode_selects_and_deletes(cx: &mut TestAppContext) {
    let f = open(cx, "hello world", vim());
    f.press(cx, "v e");
    assert_eq!(mode(&f, cx), Some(Mode::Visual));
    assert_eq!(f.selection(cx), 0..5);
    f.press(cx, "d");
    assert_eq!(f.text(cx), " world");
    assert_eq!(mode(&f, cx), Some(Mode::Normal));
}

#[gpui_kit::test]
fn ex_commands_reach_the_app(cx: &mut TestAppContext) {
    let f = open(cx, "text", vim());
    f.take_events();
    f.press(cx, ":");
    assert_eq!(
        f.pane.read_with(cx, |p, _| p.vim_command_line()),
        Some(String::new())
    );
    f.press(cx, "w");
    assert_eq!(
        f.pane.read_with(cx, |p, _| p.vim_command_line()),
        Some("w".to_owned())
    );
    f.press(cx, "enter");
    let calls: Vec<AppCommand> = f
        .take_events()
        .into_iter()
        .filter_map(|e| match e {
            EditorEvent::VimCommand(call) => Some(call.command),
            _ => None,
        })
        .collect();
    assert_eq!(calls, vec![AppCommand::Save]);
    assert_eq!(f.text(cx), "text");
}

// [COMPAT] app-owned Ctrl keys pass through on Linux: Ctrl+F opens the find bar,
// Ctrl+B formats, even in normal mode.
#[gpui_kit::test]
fn app_owned_ctrl_keys_pass_through(cx: &mut TestAppContext) {
    let f = open(cx, "word", vim());
    f.press(cx, "v e ctrl-b");
    assert_eq!(f.text(cx), "**word**");
    f.press(cx, "escape ctrl-f");
    assert!(f.is_shown(cx, "find-bar"));
    assert!(f.has_focus(cx, "find-input"));
}

// Escape in normal mode also closes the find bar (the app's Escape chain).
#[gpui_kit::test]
fn escape_in_normal_mode_closes_the_find_bar(cx: &mut TestAppContext) {
    let f = open(cx, "word", vim());
    f.press(cx, "ctrl-f escape");
    assert!(!f.is_shown(cx, "find-bar"));
    assert!(f.has_focus(cx, "editor"));
    f.press(cx, "ctrl-f");
    f.with(cx, |pane, window, cx| {
        pane.update(cx, |p, cx| p.focus(window, cx))
    });
    f.press(cx, "escape");
    assert!(!f.is_shown(cx, "find-bar"));
}

#[gpui_kit::test]
fn turning_vim_off_restores_plain_typing(cx: &mut TestAppContext) {
    let f = open(cx, "", vim());
    f.apply_settings(cx, EditorSettings::default());
    assert_eq!(mode(&f, cx), None);
    f.type_text(cx, "jk");
    assert_eq!(f.text(cx), "jk");
}

// IME: while a composition is active no key is intercepted (ui-spec §8.3).
#[gpui_kit::test]
fn composition_disables_interception(cx: &mut TestAppContext) {
    use gpui_kit::EntityInputHandler as _;
    let f = open(cx, "ab", vim());
    f.press(cx, "i");
    f.with(cx, |pane, window, cx| {
        let state = pane.read(cx).editor_state().clone();
        state.update(cx, |s, cx| {
            s.replace_and_mark_text_in_range(None, "か", None, window, cx)
        });
    });
    f.press(cx, "escape");
    // Esc went to the composition (gpui-kit), not to vim: still in insert mode.
    assert_eq!(mode(&f, cx), Some(Mode::Insert));
}
