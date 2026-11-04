//! Level-b tests for the explorer (ui-spec §5.14, frontend.md §F.3–F.12):
//! drag & drop incl. multi-select, drop on a note, selection gestures,
//! keyboard, persisted expansion, and the "Linked from" backlinks section.
#![allow(missing_docs, clippy::unwrap_used)]

extern crate gpui_kit as gpui;

mod support;

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{Modifiers, TestAppContext};
use support::harness::{self, WIDE};
use support::shell::{Shell, VaultFixture, open_shell};
use support::vm::FocusRegion;

fn click(cx: &mut TestAppContext, shell: &Shell, id: &'static str) {
    harness::in_window(cx, shell.window, |window, cx| window.click(id, cx));
}

fn ctrl_click(cx: &mut TestAppContext, shell: &Shell, id: &'static str) {
    harness::in_window(cx, shell.window, |window, cx| {
        window.click_with_modifiers(id, Modifiers::control(), cx)
    });
}

fn drag(cx: &mut TestAppContext, shell: &Shell, from: &'static str, to: &'static str) {
    harness::in_window(cx, shell.window, |window, cx| window.drag_to(from, to, cx));
    harness::in_window(cx, shell.window, |_, _| ());
}

fn exists(shell: &Shell, path: &str) -> bool {
    shell.root.join(path).exists()
}

fn selection(cx: &mut TestAppContext, shell: &Shell) -> Vec<String> {
    shell.view.read_with(cx, |s, _| {
        s.explorer()
            .selection()
            .into_iter()
            .map(|i| i.path)
            .collect()
    })
}

// adapts: test/system/notes_test.rb:369 "drag and drop moves a note into a folder" (F.12)
#[gpui_kit::test]
fn dragging_a_note_onto_a_folder_moves_it(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    drag(cx, &shell, "explorer-row-Inbox.md", "explorer-row-projects");
    assert!(exists(&shell, "projects/Inbox.md"));
    assert!(!exists(&shell, "Inbox.md"));
    // The destination is revealed (§5.14).
    assert!(
        shell
            .view
            .read_with(cx, |s, _| s.explorer().expanded().contains("projects"))
    );
}

// adapts: test/system/multi_drag_test.rb:9 "dragging a multi-selection moves every selected item"
#[gpui_kit::test]
fn dragging_a_multi_selection_moves_every_item(cx: &mut TestAppContext) {
    let fixture = VaultFixture::sample_with(&[
        ("first.md", "1"),
        ("second.md", "2"),
        ("rooted.md", "r"),
        ("target/keep.md", "k"),
    ]);
    let shell = open_shell(cx, WIDE, fixture);
    click(cx, &shell, "explorer-row-first.md");
    ctrl_click(cx, &shell, "explorer-row-second.md");
    drag(cx, &shell, "explorer-row-first.md", "explorer-row-target");
    assert!(exists(&shell, "target/first.md"));
    assert!(exists(&shell, "target/second.md"));
    assert!(exists(&shell, "rooted.md"), "an unselected item stays");
}

// adapts: multi_drag_test.rb:31 "dragging an unselected item moves only it"
#[gpui_kit::test]
fn dragging_an_unselected_item_moves_only_it(cx: &mut TestAppContext) {
    let fixture = VaultFixture::sample_with(&[
        ("first.md", "1"),
        ("second.md", "2"),
        ("target/keep.md", "k"),
    ]);
    let shell = open_shell(cx, WIDE, fixture);
    click(cx, &shell, "explorer-row-first.md");
    drag(cx, &shell, "explorer-row-second.md", "explorer-row-target");
    assert!(exists(&shell, "target/second.md"));
    assert!(exists(&shell, "first.md"));
}

// ui-spec §5.14 quirk fixed: dropping on a note moves into that note's folder (was the root).
#[gpui_kit::test]
fn dropping_on_a_note_moves_into_its_folder(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    click(cx, &shell, "explorer-row-projects");
    drag(
        cx,
        &shell,
        "explorer-row-Inbox.md",
        "explorer-row-projects/Launch list.md",
    );
    assert!(exists(&shell, "projects/Inbox.md"));
}

// F.12: moving the open note keeps it open at its new path.
#[gpui_kit::test]
fn moving_the_open_note_follows_it(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    drag(
        cx,
        &shell,
        "explorer-row-blog/2026/Project brief.md",
        "explorer-row-reference",
    );
    assert_eq!(
        shell.active_note(cx).as_deref(),
        Some("reference/Project brief.md")
    );
}

// adapts: test/system/notes_test.rb:33 F.4 gestures: Ctrl toggles without opening, Shift ranges.
#[gpui_kit::test]
fn ctrl_and_shift_clicks_select_without_opening(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    click(cx, &shell, "explorer-row-blog");
    ctrl_click(cx, &shell, "explorer-row-Inbox.md");
    assert_eq!(
        selection(cx, &shell),
        vec!["blog".to_owned(), "Inbox.md".to_owned()]
    );
    assert_eq!(
        shell.active_note(cx).as_deref(),
        Some("blog/2026/Project brief.md"),
        "ctrl-click doesn't open"
    );
    harness::in_window(cx, shell.window, |window, cx| {
        window.click_with_modifiers("explorer-row-projects", Modifiers::shift(), cx)
    });
    // Range from the anchor (Inbox.md) over visible rows: projects, reference, Inbox.md.
    let mut sel = selection(cx, &shell);
    sel.sort();
    assert_eq!(
        sel,
        vec![
            "Inbox.md".to_owned(),
            "projects".to_owned(),
            "reference".to_owned()
        ]
    );
}

// ui-spec §5.14: Esc clears a multi-selection; a left click on empty space clears too.
#[gpui_kit::test]
fn escape_clears_the_multi_selection(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    click(cx, &shell, "explorer-row-blog");
    ctrl_click(cx, &shell, "explorer-row-Inbox.md");
    harness::press(cx, shell.window, "escape");
    assert!(selection(cx, &shell).is_empty());
}

// ui-spec §5.14: type-ahead jumps to the next name starting with the typed letters.
#[gpui_kit::test]
fn type_ahead_moves_the_cursor(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample_no_note());
    let w = shell.window;
    harness::press(cx, w, "ctrl-1 r");
    let cursor = shell
        .view
        .read_with(cx, |s, _| s.explorer().cursor().map(str::to_owned));
    assert_eq!(cursor.as_deref(), Some("reference"));
    harness::press(cx, w, "enter down enter");
    assert_eq!(
        shell.active_note(cx).as_deref(),
        Some("reference/Glossary.md")
    );
    assert_eq!(shell.focus_region(cx), Some(FocusRegion::Sidebar));
}

// ui-spec §5.14: expansion state is persisted per vault (quirk fixed: F.3 forgot it).
#[gpui_kit::test]
fn expansion_is_persisted_in_the_state_dir(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample_no_note());
    click(cx, &shell, "explorer-row-reference");
    let state = shell
        .view
        .read_with(cx, |s, _| s.explorer().expanded().contains("reference"));
    assert!(state);
    let saved = shell.ui_state_json();
    assert!(saved.contains("\"reference\""), "{saved}");
}

// ui-spec §3.3 / BE-044: "Linked from" lists notes linking to the open note.
#[gpui_kit::test]
fn backlinks_list_linking_notes_and_open_them(cx: &mut TestAppContext) {
    let fixture = VaultFixture {
        open: Some("reference/Glossary.md"),
        ..VaultFixture::sample_with(&[("links.md", "# Links\n\nSee [[Glossary]] for terms.\n")])
    };
    let shell = open_shell(cx, WIDE, fixture);
    cx.run_until_parked();
    let w = shell.window;
    assert_eq!(
        shell.view.read_with(cx, |s, _| s.backlink_paths()),
        vec!["links.md".to_owned()]
    );
    assert!(harness::is_shown(cx, w, "backlinks"));
    click(cx, &shell, "backlink-links.md-3");
    assert_eq!(shell.active_note(cx).as_deref(), Some("links.md"));
    // links.md itself has no backlinks: the section disappears.
    cx.run_until_parked();
    assert!(!harness::is_shown(cx, w, "backlinks"));
}
