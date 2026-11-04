//! GUI-free view models of the app shell. The first test of each group is
//! the level-a test from crates/frankmd-app/tests (ui_shell.rs, ui_banners.rs,
//! ui_overlays.rs), moved here as the plan asks (docs/testing/ui-test-plan.md
//! §3.4/§4); the rest come from the plan's §4 rows and ui-spec §4–§6.
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;
use std::time::{Duration, Instant};

use frankmd_workspace::autosave::SaveStatus;
use frankmd_workspace::banners::{BannerKind, BannerStack};
use frankmd_workspace::commands::{CommandContext, CommandRegistry, display_keystroke};
use frankmd_workspace::focus::FocusRegion;
use frankmd_workspace::layout::{
    LayoutState, PreviewMode, SidebarMode, ViewMode, WidthClass, text_column,
};
use frankmd_workspace::overlay::{OverlayKind, OverlayStack, PaletteMode};
use frankmd_workspace::quick_open::{NoteEntry, QuickOpenModel};
use frankmd_workspace::status::{MessageSlot, SaveState};

// ---- layout -----------------------------------------------------------------

// adapts: test/system/workspace_test.rb:538 (width-class part; ui-spec §4.1)
#[test]
fn width_class_has_24px_hysteresis() {
    let mut layout = LayoutState::new();
    assert_eq!(layout.on_resize(1280.), WidthClass::Wide);
    assert_eq!(
        layout.on_resize(1090.),
        WidthClass::Wide,
        "within 24 px below 1100 stays wide"
    );
    assert_eq!(layout.on_resize(1070.), WidthClass::Medium);
    assert_eq!(
        layout.on_resize(1110.),
        WidthClass::Medium,
        "within 24 px above 1100 stays medium"
    );
    assert_eq!(layout.on_resize(1130.), WidthClass::Wide);
    assert_eq!(layout.on_resize(690.), WidthClass::Narrow);
}

// adapts: test/javascript/controllers/app_view_mode.test.js:137 and :154 (§4.2)
#[test]
fn narrow_forces_single_pane_swap() {
    let mut layout = LayoutState::new();
    layout.on_resize(640.);
    let a = layout.arrangement(640.);
    assert_eq!(
        a.sidebar,
        SidebarMode::Hidden,
        "narrow: sidebar only as drawer when opened"
    );
    assert!(a.editor);
    assert_eq!(a.preview, PreviewMode::Hidden);
    layout.toggle_preview();
    let a = layout.arrangement(640.);
    assert_eq!(a.preview, PreviewMode::Swapped);
    assert!(!a.editor);
    layout.toggle_preview();
    assert!(layout.arrangement(640.).editor);
}

// adapts: test/system/workspace_test.rb:402 (§4.2: leaving the library restores preview visibility)
#[test]
fn media_library_restores_preview_visibility() {
    let mut layout = LayoutState::new();
    layout.toggle_preview(); // hide
    assert!(!layout.preview_visible);
    layout.toggle_media_library();
    layout.toggle_media_library();
    assert!(
        !layout.preview_visible,
        "preview stays hidden after leaving the library"
    );
}

// ui-spec §4.2 Zen
#[test]
fn zen_hides_chrome_and_combines_with_typewriter() {
    let mut layout = LayoutState::new();
    layout.typewriter = true;
    layout.toggle_zen();
    let a = layout.arrangement(1280.);
    assert_eq!(a.sidebar, SidebarMode::Hidden);
    assert_eq!(a.preview, PreviewMode::Hidden);
    assert!(!a.pane_header);
    assert!(a.status_compact);
    assert!(layout.typewriter, "zen keeps typewriter on");
    layout.toggle_zen();
    let a = layout.arrangement(1280.);
    assert_eq!(a.sidebar, SidebarMode::Docked);
    assert_eq!(
        a.preview,
        PreviewMode::Split,
        "zen restores the previous arrangement"
    );
}

// ui-spec §4.1 medium; mirrors ui_shell.rs `medium_width_auto_hides_sidebar_for_preview`
#[test]
fn medium_width_auto_hides_the_sidebar_for_the_preview_without_touching_the_preference() {
    let mut layout = LayoutState::new();
    layout.on_resize(900.);
    assert_eq!(
        layout.arrangement(900.).sidebar,
        SidebarMode::Docked,
        "248 + 320 + 280 fits in 900"
    );
    layout.on_resize(760.);
    let a = layout.arrangement(760.);
    assert_eq!(
        (a.sidebar, a.preview),
        (SidebarMode::Hidden, PreviewMode::Split)
    );
    assert!(
        layout.sidebar_visible,
        "the user's preference is not overwritten"
    );
    layout.toggle_preview();
    assert_eq!(
        layout.arrangement(760.).sidebar,
        SidebarMode::Docked,
        "comes back once the preview closes"
    );
}

// ui-spec §4.1 narrow drawer; mirrors ui_shell.rs `narrow_sidebar_is_a_drawer_closed_by_escape`
#[test]
fn narrow_sidebar_is_a_drawer_that_closes_on_escape_note_open_and_widening() {
    let mut layout = LayoutState::new();
    layout.on_resize(640.);
    layout.toggle_sidebar();
    assert_eq!(layout.arrangement(640.).sidebar, SidebarMode::Drawer);
    assert!(
        layout.sidebar_visible,
        "Ctrl+E at narrow width never changes the docked preference"
    );
    assert!(layout.close_drawer());
    assert_eq!(layout.arrangement(640.).sidebar, SidebarMode::Hidden);

    layout.toggle_sidebar();
    layout.note_opened();
    assert_eq!(
        layout.arrangement(640.).sidebar,
        SidebarMode::Hidden,
        "opening a note closes the drawer"
    );

    layout.toggle_sidebar();
    layout.on_resize(1280.);
    assert_eq!(
        layout.arrangement(1280.).sidebar,
        SidebarMode::Docked,
        "docked again at wide width"
    );
}

// ui-spec §4.2 Single; plan §4 layout "ViewMode::parse falls back to Split"
#[test]
fn single_view_mode_swaps_and_unknown_values_fall_back_to_split() {
    assert_eq!(ViewMode::parse(Some("single")), ViewMode::Single);
    assert_eq!(ViewMode::parse(Some("triple")), ViewMode::Split);
    assert_eq!(ViewMode::parse(None), ViewMode::Split);

    let mut layout = LayoutState::new();
    layout.view_mode = ViewMode::Single;
    assert_eq!(layout.arrangement(1280.).preview, PreviewMode::Hidden);
    layout.toggle_preview();
    let a = layout.arrangement(1280.);
    assert_eq!((a.editor, a.preview), (false, PreviewMode::Swapped));
    assert!(
        layout.preview_visible,
        "the split preference is untouched by swapping"
    );
}

// ui-spec §4.2 Media library: replaces editor + preview; opening a note leaves it.
#[test]
fn media_library_replaces_the_panes_and_closes_when_a_note_opens() {
    let mut layout = LayoutState::new();
    layout.toggle_media_library();
    let a = layout.arrangement(1280.);
    assert!(a.media_library && !a.editor);
    assert_eq!(a.preview, PreviewMode::Hidden);
    layout.note_opened();
    let a = layout.arrangement(1280.);
    assert!(!a.media_library && a.editor);
    assert_eq!(a.preview, PreviewMode::Split);
}

// plan §4 layout: text_column(pane_w, editor_width, char_w) -> (inline_margin, column_w)
#[test]
fn text_column_is_centred_and_bounded_by_the_pane() {
    assert_eq!(text_column(1000., 72, 10.), (140., 720.));
    let (margin, column) = text_column(500., 72, 10.);
    assert_eq!(
        (margin, column),
        (32., 436.),
        "narrower than editor_width: inline padding kept"
    );
}

// ---- focus --------------------------------------------------------------------

// adapts: test/javascript/controllers/app_editor_focus.test.js (it.each ×3); ui-spec §4.3
#[test]
fn f6_cycle_skips_hidden_regions() {
    let mut layout = LayoutState::new();
    let all = layout.arrangement(1280.);
    assert_eq!(FocusRegion::Sidebar.next(&all), FocusRegion::Editor);
    assert_eq!(FocusRegion::Editor.next(&all), FocusRegion::Preview);
    assert_eq!(FocusRegion::Preview.next(&all), FocusRegion::Sidebar);
    assert_eq!(FocusRegion::Sidebar.prev(&all), FocusRegion::Preview);
    layout.toggle_preview();
    let no_preview = layout.arrangement(1280.);
    assert_eq!(FocusRegion::Editor.next(&no_preview), FocusRegion::Sidebar);
}

// ui-spec §4.3: the status line joins the cycle only while it shows the ex command line.
#[test]
fn the_ex_command_line_joins_the_cycle_only_while_open() {
    let a = LayoutState::new().arrangement(1280.);
    assert_eq!(
        FocusRegion::Preview.step(&a, true, true),
        FocusRegion::StatusLine
    );
    assert_eq!(
        FocusRegion::StatusLine.step(&a, true, true),
        FocusRegion::Sidebar
    );
    assert_eq!(
        FocusRegion::Preview.step(&a, false, true),
        FocusRegion::Sidebar
    );
}

// ui-spec §4.3: hiding the focused region moves focus to the editor.
#[test]
fn focus_falls_back_to_the_editor_when_its_region_disappears() {
    let mut layout = LayoutState::new();
    layout.toggle_preview();
    let a = layout.arrangement(1280.);
    assert_eq!(FocusRegion::Preview.fallback(&a), FocusRegion::Editor);
    assert_eq!(FocusRegion::Sidebar.fallback(&a), FocusRegion::Sidebar);
}

// ---- overlays -------------------------------------------------------------------

// ui-spec §2 "Overlays (one at a time)"; §4.3 focus returns to the previous region.
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

// ui-spec §5 "Esc cancels every dialog except Recovery"; §5.1 prompt prefixes.
#[test]
fn escape_never_closes_the_recovery_dialog_and_prefixes_pick_palette_modes() {
    let mut stack = OverlayStack::default();
    stack.open(OverlayKind::Dialog("recovery"), FocusRegion::Editor);
    assert_eq!(stack.escape(), None);
    assert!(stack.is_open(&OverlayKind::Dialog("recovery")));
    stack.open(OverlayKind::Settings, FocusRegion::Editor);
    assert_eq!(stack.escape(), Some(FocusRegion::Editor));

    assert_eq!(
        PaletteMode::from_prefix(">zen"),
        Some(PaletteMode::Commands)
    );
    assert_eq!(PaletteMode::from_prefix(":42"), Some(PaletteMode::GoToLine));
    assert_eq!(PaletteMode::from_prefix("note"), None);
}

// ---- command registry -------------------------------------------------------------

// ui-spec §5.1; adapts: test/system/workspace_test.rb:491 (reachability is "it's in the palette")
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

    let in_table = CommandContext {
        markdown_open: true,
        caret_in_table: true,
        ..CommandContext::default()
    };
    let rows = reg.query("edit table", &in_table, &[]);
    assert_eq!(rows.first().map(|r| r.unavailable.is_none()), Some(true));
}

// ui-spec §5.1: toggle commands show their state; empty query lists recently used first.
#[test]
fn palette_shows_toggle_state_and_recent_first() {
    let reg = CommandRegistry::builtin();
    let cx = CommandContext {
        markdown_open: true,
        vim: true,
        ..CommandContext::default()
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

// architecture.md §3.9: "a test asserts there are no duplicate chords per context".
#[test]
fn keymap_generated_from_the_registry_has_no_duplicate_chords_per_context() {
    let mut seen = HashSet::new();
    for b in CommandRegistry::builtin().bindings() {
        assert!(
            seen.insert((b.context, b.keystroke)),
            "duplicate chord {} in {:?}",
            b.keystroke,
            b.context
        );
    }
}

// ui-spec §6: both shifted forms are bound where US layouts differ (FE-042), and every
// binding of the app's proposed keymap table exists in the registry.
#[test]
fn registry_covers_the_default_keymap_including_both_shifted_forms() {
    let bindings = CommandRegistry::builtin().bindings();
    let has = |key: &str, id: &str| {
        bindings
            .iter()
            .any(|b| b.keystroke == key && b.command == id)
    };
    assert!(
        has("ctrl-shift-\\", "view.toggle_scroll_sync") && has("ctrl-|", "view.toggle_scroll_sync")
    );
    assert!(has("ctrl-shift-=", "view.width_increase") && has("ctrl-+", "view.width_increase"));
    assert!(has("f11", "view.toggle_zen") && has("ctrl-shift-p", "app.command_palette"));
    assert_eq!(
        bindings.len(),
        59,
        "one row per binding of crates/frankmd-app/tests/support/actions.rs"
    );
}

#[test]
fn shortcuts_print_like_the_spec() {
    assert_eq!(display_keystroke("ctrl-shift-p"), "Ctrl+Shift+P");
    assert_eq!(display_keystroke("f11"), "F11");
    assert_eq!(display_keystroke("ctrl--"), "Ctrl+-");
    assert_eq!(display_keystroke("ctrl-\\"), "Ctrl+\\");
    assert_eq!(display_keystroke("ctrl-alt-left"), "Ctrl+Alt+←");
}

// ---- quick open ----------------------------------------------------------------------

// adapts: test/javascript/controllers/file_finder_controller.test.js:84 and :218/:227 (§5.2)
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

// adapts: file_finder_controller.test.js:144 and :154 (§5.2: offer Ctrl+N `query.md`)
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
    assert_eq!(qo.selected(), None);
    assert_eq!(qo.create_suggestion().as_deref(), Some("zzz-missing.md"));
}

// ui-spec §5.2: top 50.
#[test]
fn quick_open_shows_at_most_fifty_rows() {
    let notes = (0..80)
        .map(|i| NoteEntry {
            path: format!("n{i}.md"),
            mtime: i,
        })
        .collect();
    let qo = QuickOpenModel::new(notes, vec![]);
    assert_eq!(qo.rows().len(), 50);
    assert_eq!(qo.rows()[0], "n79.md", "newest first");
}

// ---- banners & status ------------------------------------------------------------------

// ui-spec §5.12: fixed priority order regardless of raise order; raising twice is a no-op.
#[test]
fn banners_stack_in_priority_order() {
    let mut stack = BannerStack::default();
    stack.raise(BannerKind::ContentLoss);
    stack.raise(BannerKind::VaultUnreachable);
    stack.raise(BannerKind::ContentLoss);
    stack.raise(BannerKind::ChangedOnDisk);
    assert_eq!(
        stack.visible(),
        vec![
            BannerKind::VaultUnreachable,
            BannerKind::ChangedOnDisk,
            BannerKind::ContentLoss
        ]
    );
    stack.clear(BannerKind::VaultUnreachable);
    assert_eq!(stack.visible().first(), Some(&BannerKind::ChangedOnDisk));
}

// adapts: autosave_controller.test.js:869 — content loss offers Undo / Save anyway.
#[test]
fn content_loss_banner_offers_undo_and_save_anyway() {
    assert_eq!(
        BannerStack::buttons(BannerKind::ContentLoss),
        vec![
            "banners.content_loss.undo",
            "banners.content_loss.save_anyway"
        ]
    );
    assert_eq!(
        BannerStack::buttons(BannerKind::VaultUnreachable),
        vec!["banners.offline.retry"]
    );
    assert_eq!(BannerKind::ContentLoss.element_id(), "banner-content-loss");
    assert_eq!(
        BannerKind::ContentLoss.button_id("banners.content_loss.undo"),
        "banner-content-loss-undo"
    );
}

// plan §4 banners: StorageFallback dismiss is sticky for the session.
#[test]
fn a_dismissed_storage_fallback_banner_stays_dismissed() {
    let mut stack = BannerStack::default();
    stack.raise(BannerKind::StorageFallback);
    stack.dismiss(BannerKind::StorageFallback);
    stack.raise(BannerKind::StorageFallback);
    assert!(!stack.is_raised(BannerKind::StorageFallback));
    stack.dismiss(BannerKind::ChangedOnDisk);
    stack.raise(BannerKind::ChangedOnDisk);
    assert!(
        stack.is_raised(BannerKind::ChangedOnDisk),
        "other banners come back when raised again"
    );
}

// changed (§2.1, §3.6): toasts → status-line message slot; 2 s info / 5 s errors, single instance.
#[test]
fn status_messages_expire_on_the_injected_clock() {
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    let mut slot = MessageSlot::default();
    slot.show("Saved", false, t0);
    assert_eq!(
        slot.current(at(1999)).map(|m| m.text.as_str()),
        Some("Saved")
    );
    assert_eq!(slot.current(at(2000)), None);
    slot.show("Save failed", true, at(10_000));
    assert!(slot.current(at(14_999)).is_some(), "errors stay 5 s");
    slot.show("Copied", false, at(11_000));
    assert_eq!(
        slot.current(at(11_000)).map(|m| m.text.as_str()),
        Some("Copied"),
        "single instance"
    );
}

#[test]
fn save_state_follows_the_autosave_session() {
    assert_eq!(
        SaveState::from_autosave(SaveStatus::Unsaved, true, true),
        SaveState::Saving
    );
    assert_eq!(
        SaveState::from_autosave(SaveStatus::Saved, false, false),
        SaveState::Saved
    );
    assert_eq!(
        SaveState::from_autosave(SaveStatus::DraftStorageError, false, true),
        SaveState::DraftNotStored
    );
    assert_eq!(
        SaveState::from_autosave(SaveStatus::Disconnected, false, true),
        SaveState::Unsaved
    );
    assert_eq!(
        SaveState::from_autosave(SaveStatus::Hidden, false, false),
        SaveState::Clean
    );
    assert_eq!(SaveState::Failed.label_key(), Some("status.error_saving"));
    assert!(SaveState::DraftNotStored.is_error());
}
