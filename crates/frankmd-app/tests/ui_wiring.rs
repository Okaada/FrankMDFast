//! Level-b wiring tests: every dialog built outside `src/ui` (docs/testing/
//! wiring-media.md, wiring-insert.md) opens from its registry command in
//! the real shell, through the overlay stack, the palette, the main area or
//! the editor pane, and its events come back to the shell.
#![allow(missing_docs, clippy::unwrap_used)]

extern crate gpui_kit as gpui;

mod support;

use gpui_kit::TestAppContext;
use gpui_kit::test::TestWindowExt as _;
use support::harness::{self, WIDE};
use support::shell::{Shell, VaultFixture, open_shell};
use support::vm::{FocusRegion, OverlayKind};

fn run(cx: &mut TestAppContext, shell: &Shell, id: &'static str) {
    let view = shell.view.clone();
    harness::in_window(cx, shell.window, move |window, cx| {
        view.update(cx, |s, cx| s.run_command(id, window, cx))
    });
}

fn shown(cx: &mut TestAppContext, shell: &Shell, id: &'static str) -> bool {
    harness::is_shown(cx, shell.window, id)
}

fn table_fixture() -> VaultFixture {
    VaultFixture {
        notes: vec![("Table.md", "| a | b |\n| - | - |\n| 1 | 2 |\n")],
        recents: vec![],
        open: Some("Table.md"),
    }
}

// wiring-media.md §3: insert.image (Ctrl+Shift+I, header button) → the picker overlay.
#[gpui_kit::test]
fn insert_image_opens_the_picker_overlay(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    harness::press(cx, shell.window, "ctrl-2 ctrl-shift-i");
    assert_eq!(shell.overlay(cx), Some(OverlayKind::Dialog("insert-image")));
    assert!(shown(cx, &shell, "dialog-insert-image"));
    harness::press(cx, shell.window, "escape");
    assert_eq!(shell.overlay(cx), None);
    assert_eq!(shell.focus_region(cx), Some(FocusRegion::Editor));
}

// wiring-media.md §3: the pane header's image button runs the same command.
#[gpui_kit::test]
fn header_image_button_opens_the_picker(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    harness::in_window(cx, shell.window, |window, cx| {
        window.click("pane-insert-image", cx)
    });
    assert!(shown(cx, &shell, "dialog-insert-image"));
}

// wiring-media.md §3: insert.video → the video dialog overlay.
#[gpui_kit::test]
fn insert_video_opens_the_video_dialog(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    run(cx, &shell, "insert.video");
    assert_eq!(shell.overlay(cx), Some(OverlayKind::Dialog("insert-video")));
    assert!(shown(cx, &shell, "dialog-insert-video"));
}

// wiring-media.md §3: view.media_library (Ctrl+Shift+M) shows the real library pane.
#[gpui_kit::test]
fn media_library_shows_the_library_pane(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    harness::press(cx, shell.window, "ctrl-2 ctrl-shift-m");
    assert!(shell.layout(cx).media_library);
    assert!(shown(cx, &shell, "media-library"));
    assert!(!shown(cx, &shell, "editor-pane"));
}

// wiring-media.md §3/§7: ai.grammar_check with no AI configured reports it in the status line
// (the dialog only opens on GrammarEvent::Show after a check).
#[gpui_kit::test]
fn grammar_check_reports_through_the_status_line(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    harness::press(cx, shell.window, "ctrl-2 ctrl-shift-g");
    cx.run_until_parked();
    let overlay = shell.overlay(cx);
    let message = shell.view.read_with(cx, |s, _| s.status_message());
    assert!(
        overlay == Some(OverlayKind::Dialog("grammar")) || message.is_some(),
        "either the result dialog or a status message: {overlay:?} {message:?}"
    );
}

// wiring-insert.md §1: insert.table reaches the editor pane's table editor.
#[gpui_kit::test]
fn insert_table_opens_the_table_editor(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    harness::press(cx, shell.window, "ctrl-2");
    run(cx, &shell, "insert.table");
    assert!(shown(cx, &shell, "dialog-table"));
}

// wiring-insert.md §1: insert.code_block → the code block dialog.
#[gpui_kit::test]
fn insert_code_block_opens_the_code_dialog(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    harness::press(cx, shell.window, "ctrl-2");
    run(cx, &shell, "insert.code_block");
    assert!(shown(cx, &shell, "dialog-code-block"));
}

// wiring-insert.md §1: Ctrl+Shift+E → the emoji picker popover.
#[gpui_kit::test]
fn ctrl_shift_e_opens_the_emoji_picker(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    harness::press(cx, shell.window, "ctrl-2 ctrl-shift-e");
    assert!(shown(cx, &shell, "emoji-picker"));
}

// wiring-insert.md §1 + ui-spec §6.2: Ctrl+. inside a table opens the table editor,
// and the status line shows the context chip (§3.6).
#[gpui_kit::test]
fn ctrl_dot_in_a_table_edits_it_and_shows_the_chip(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, table_fixture());
    let w = shell.window;
    harness::press(cx, w, "ctrl-2 down down");
    assert!(shown(cx, &shell, "status-context-chip"));
    harness::press(cx, w, "ctrl-.");
    assert!(shown(cx, &shell, "dialog-table"));
}

// Palette rows run the hosted dialogs too (one registry, HP-018).
#[gpui_kit::test]
fn palette_runs_insert_image(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    let w = shell.window;
    harness::press(cx, w, "ctrl-2 ctrl-shift-p");
    harness::type_text(cx, w, "insert image");
    harness::press(cx, w, "enter");
    assert_eq!(shell.overlay(cx), Some(OverlayKind::Dialog("insert-image")));
}

// wiring-media.md §6: Settings › Media › Storage for new media is bound to the storage model.
#[gpui_kit::test]
fn settings_storage_row_is_bound_to_the_media_layer(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    let w = shell.window;
    harness::press(cx, w, "ctrl-2 ctrl-,");
    harness::in_window(cx, w, |window, cx| {
        window.click("settings-category-media", cx)
    });
    assert!(shown(cx, &shell, "setting-provider-local"));
    assert!(shown(cx, &shell, "setting-provider-s3"));
}
