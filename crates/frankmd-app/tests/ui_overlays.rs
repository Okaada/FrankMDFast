//! Adapted UI tests: command palette, quick open, empty state, keymap table.
//!
//! See docs/testing/ui-test-plan.md. The keymap is generated from the command
//! registry; level-b tests drive the real shell headlessly.
#![allow(missing_docs, clippy::unwrap_used)]

extern crate gpui_kit as gpui;

mod support;

use std::collections::HashSet;

use gpui_kit::TestAppContext;
use support::harness::{self, WIDE};
use support::shell::{VaultFixture, open_shell};
use support::vm::{
    CommandContext, CommandRegistry, FocusRegion, NoteEntry, OverlayKind, OverlayStack,
    PaletteMode, QuickOpenModel,
};

// ---- keymap / registry (level a) --------------------------------------------

// architecture.md §3.9: "a test asserts there are no duplicate chords per context".
// Replaces the CodeMirror duplicate-binding quirks (ui-spec §11 "CodeMirror duplicates on Ctrl+F/G/Enter/M").
#[test]
fn keymap_has_no_duplicate_chords_per_context() {
    let mut seen = HashSet::new();
    for binding in support::actions::default_keymap() {
        let chord: Vec<String> = binding
            .keystrokes()
            .iter()
            .map(|k| k.unparse(gpui_kit::OperatingSystem::Linux))
            .collect();
        let context = binding
            .predicate()
            .map(|p| p.to_string())
            .unwrap_or_default();
        assert!(
            seen.insert((context.clone(), chord.clone())),
            "duplicate chord {chord:?} in context {context:?}"
        );
    }
}

// ui-spec §5.1: every command is in the palette; commands that don't apply are hidden,
// with an "Unavailable" tail only when the query matches nothing available.
// adapts: test/system/workspace_test.rb:491 "preview remains reachable from the menu after unpinning"
// (pins are gone; reachability is now "it's in the palette")
#[test]
fn palette_hides_unavailable_commands_and_explains_when_only_those_match() {
    let reg = CommandRegistry::builtin();
    let no_note = CommandContext::default();
    let rows = reg.query("preview", &no_note, &[]);
    assert!(rows.iter().any(|r| r.id == "view.toggle_preview"));

    let rows = reg.query("edit table", &no_note, &[]);
    assert!(
        rows.iter().all(|r| r.unavailable.is_some()),
        "only the Unavailable section"
    );
    assert!(
        !rows.is_empty(),
        "reason shown, e.g. 'Open a markdown note first'"
    );

    // mechanical: CommandContext grew fields (note_open, toggles); the rest default.
    let in_table = CommandContext {
        markdown_open: true,
        caret_in_table: true,
        vim: false,
        ..Default::default()
    };
    let rows = reg.query("edit table", &in_table, &[]);
    assert_eq!(rows.first().map(|r| r.unavailable.is_none()), Some(true));
}

// §5.1: toggle commands show their state; empty query lists recently used first.
#[test]
fn palette_shows_toggle_state_and_recent_first() {
    let reg = CommandRegistry::builtin();
    let cx = CommandContext {
        markdown_open: true,
        caret_in_table: false,
        vim: true,
        ..Default::default()
    };
    let vim = reg.query("vim mode", &cx, &[]);
    assert_eq!(vim.first().and_then(|r| r.checked), Some(true));
    let rows = reg.query("", &cx, &["view.toggle_zen"]);
    assert_eq!(rows.first().map(|r| r.id), Some("view.toggle_zen"));
    assert_eq!(
        rows.first().and_then(|r| r.shortcut.clone()).as_deref(),
        Some("F11")
    );
}

// §2 "Overlays (one at a time)"; §4.3 "Overlays trap focus and return it to the previous region".
#[test]
fn overlay_stack_is_single_and_restores_original_focus() {
    let mut stack = OverlayStack::default();
    stack.open(
        OverlayKind::Palette(PaletteMode::Commands),
        FocusRegion::Editor,
    );
    stack.open(OverlayKind::QuickOpen, FocusRegion::Sidebar);
    assert_eq!(
        stack.open.as_ref().map(|(k, _)| k.clone()),
        Some(OverlayKind::QuickOpen)
    );
    assert_eq!(
        stack.close(),
        Some(FocusRegion::Editor),
        "focus returns where the user was"
    );
    assert_eq!(stack.close(), None);
}

// ---- quick open (level a) ---------------------------------------------------

// adapts: test/javascript/controllers/file_finder_controller.test.js:84 "shows first 10 files initially"
// (changed §5.2: recents first, then mtime; top 50) and :218/:227 clamping → wrap (§5 intro).
#[test]
fn quick_open_lists_recents_first_and_wraps_selection() {
    let notes = vec![
        NoteEntry {
            path: "a.md".into(),
            mtime: 300,
        },
        NoteEntry {
            path: "b.md".into(),
            mtime: 200,
        },
        NoteEntry {
            path: "c.md".into(),
            mtime: 100,
        },
    ];
    let mut qo = QuickOpenModel::new(notes, vec!["c.md".into()]);
    assert_eq!(qo.rows(), vec!["c.md", "a.md", "b.md"]);
    assert_eq!(qo.selected(), Some(0));
    qo.select_prev();
    assert_eq!(qo.selected(), Some(2), "↑ on the first row wraps");
    qo.select_next();
    assert_eq!(qo.selected(), Some(0));
}

// adapts: file_finder_controller.test.js:144 "searches in full path including directory"
// and :154 "shows no files message when empty" (changed §5.2: offer Ctrl+N `query.md`).
#[test]
fn quick_open_matches_paths_and_offers_creation() {
    let notes = vec![
        NoteEntry {
            path: "blog/2026/brief.md".into(),
            mtime: 1,
        },
        NoteEntry {
            path: "inbox.md".into(),
            mtime: 2,
        },
    ];
    let mut qo = QuickOpenModel::new(notes, vec![]);
    qo.set_query("blog brief");
    assert_eq!(qo.rows(), vec!["blog/2026/brief.md"]);
    qo.set_query("zzz-missing");
    assert!(qo.rows().is_empty());
    assert_eq!(qo.create_suggestion().as_deref(), Some("zzz-missing.md"));
}

// ---- level b: real shell ------------------------------------------------------

// §5.1 open/close: Ctrl+Shift+P opens with the input focused, Esc closes and restores focus.
#[gpui_kit::test]
fn palette_opens_focused_and_escape_restores_editor_focus(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    let w = shell.window;
    harness::press(cx, w, "ctrl-2 ctrl-shift-p");
    assert_eq!(
        shell.overlay(cx),
        Some(OverlayKind::Palette(PaletteMode::Commands))
    );
    assert!(harness::has_focus(cx, w, "palette-input"));
    harness::press(cx, w, "escape");
    assert_eq!(shell.overlay(cx), None);
    assert!(!harness::is_shown(cx, w, "command-palette"));
    assert_eq!(shell.focus_region(cx), Some(FocusRegion::Editor));
}

// §5.1: running a command from the palette dispatches the same action as its shortcut.
#[gpui_kit::test]
fn palette_runs_command_through_the_registry(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    let w = shell.window;
    harness::press(cx, w, "ctrl-shift-p");
    harness::type_text(cx, w, "zen");
    harness::press(cx, w, "enter");
    assert!(shell.layout(cx).zen);
    assert_eq!(shell.overlay(cx), None);
}

// adapts: file_finder_controller.test.js:236 "calls selectCurrent on Enter" and :377
// "dispatches event and closes dialog"; test/system/notes_test.rb:33 "clicking a note opens it in the editor"
#[gpui_kit::test]
fn quick_open_filters_and_opens_note(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    let w = shell.window;
    harness::press(cx, w, "ctrl-p");
    assert_eq!(shell.overlay(cx), Some(OverlayKind::QuickOpen));
    harness::type_text(cx, w, "launch");
    harness::press(cx, w, "enter");
    assert_eq!(shell.overlay(cx), None);
    assert_eq!(
        shell.active_note(cx).as_deref(),
        Some("projects/Launch list.md")
    );
    assert_eq!(shell.focus_region(cx), Some(FocusRegion::Editor));
}

// adapts: test/system/home_recent_files_test.rb:6 "the recent section appears after opening a note..."
// and workspace_test.rb:185 "Home hides the header..." (changed §5.2: Home → docked quick open).
#[gpui_kit::test]
fn empty_state_docks_quick_open_with_input_focused(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample_no_note());
    let w = shell.window;
    assert!(harness::is_shown(cx, w, "empty-state"));
    assert!(
        harness::has_focus(cx, w, "quick-open-input"),
        "type immediately after launch"
    );
    assert_eq!(shell.overlay(cx), None, "docked, not an overlay (no scrim)");
    harness::type_text(cx, w, "inbox");
    harness::press(cx, w, "enter");
    assert_eq!(shell.active_note(cx).as_deref(), Some("Inbox.md"));
    assert!(!harness::is_shown(cx, w, "empty-state"));
}
