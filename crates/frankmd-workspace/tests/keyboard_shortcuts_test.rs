//! Port of test/javascript/lib/keyboard_shortcuts.test.js.
#![allow(clippy::unwrap_used)]

use frankmd_workspace::shortcuts::{
    DEFAULT_SHORTCUTS, KeyEvent, Platform, Shortcut, ShortcutMap, format_shortcut, match_shortcut,
};
use std::collections::HashSet;

const KS: &str = "test/javascript/lib/keyboard_shortcuts.test.js";

fn ev(key: &str, ctrl: bool, shift: bool, alt: bool, meta: bool) -> KeyEvent {
    KeyEvent { key: key.to_owned(), ctrl, shift, alt, meta }
}
fn plain(key: &str) -> KeyEvent {
    ev(key, false, false, false, false)
}
fn ctrl(key: &str) -> KeyEvent {
    ev(key, true, false, false, false)
}
fn ctrl_shift(key: &str) -> KeyEvent {
    ev(key, true, true, false, false)
}

/// `SHORTCUTS.<action>` from the const table (independent of `ShortcutMap`).
fn sc(action: &str) -> &'static Shortcut {
    &DEFAULT_SHORTCUTS.iter().find(|(a, _)| *a == action).unwrap_or_else(|| panic!("{action} missing")).1
}

fn defaults() -> ShortcutMap {
    ShortcutMap::defaults()
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:18 "contains expected shortcut definitions"
#[test]
fn contains_expected_shortcut_definitions() {
    let _ = KS;
    assert_eq!(*sc("newNote"), Shortcut::key("n").ctrl());
    assert_eq!(*sc("save"), Shortcut::key("s").ctrl());
    assert_eq!(*sc("togglePreview"), Shortcut::key("V").ctrl().shift());
    assert_eq!(*sc("findInFile"), Shortcut::key("f").ctrl());
    assert_eq!(*sc("findReplace"), Shortcut::key("h").ctrl());
    assert_eq!(*sc("help"), Shortcut::key("F1"));
    // Bold/italic belong to the editor keymap, not the app table.
    assert!(!DEFAULT_SHORTCUTS.iter().any(|(a, _)| *a == "bold" || *a == "italic"));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:41 "matches simple ctrl+key shortcuts"
#[test]
fn matches_simple_ctrl_key_shortcuts() {
    assert!(match_shortcut(&ctrl("s"), Some(sc("save"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:46 "matches meta+key shortcuts (macOS cmd)"
#[test]
fn matches_meta_key_shortcuts() {
    assert!(match_shortcut(&ev("s", false, false, false, true), Some(sc("save"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:51 "matches ctrl+shift+key shortcuts"
#[test]
fn matches_ctrl_shift_key_shortcuts() {
    assert!(match_shortcut(&ctrl_shift("V"), Some(sc("togglePreview"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:56 "rejects wrong key"
#[test]
fn rejects_wrong_key() {
    assert!(!match_shortcut(&ctrl("x"), Some(sc("save"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:61 "rejects missing ctrl modifier"
#[test]
fn rejects_missing_ctrl_modifier() {
    assert!(!match_shortcut(&plain("s"), Some(sc("save"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:66 "rejects extra shift modifier"
#[test]
fn rejects_extra_shift_modifier() {
    assert!(!match_shortcut(&ctrl_shift("s"), Some(sc("save"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:71 "rejects missing shift modifier"
#[test]
fn rejects_missing_shift_modifier() {
    assert!(!match_shortcut(&ctrl("V"), Some(sc("togglePreview"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:76 "matches function keys without modifiers"
#[test]
fn matches_function_keys_without_modifiers() {
    assert!(match_shortcut(&plain("F1"), Some(sc("help"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:81 "rejects function key with extra modifiers"
#[test]
fn rejects_function_key_with_extra_modifiers() {
    assert!(!match_shortcut(&ctrl("F1"), Some(sc("help"))));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:86 "returns false for null/undefined shortcut"
#[test]
fn returns_false_for_missing_shortcut() {
    assert!(!match_shortcut(&ctrl("s"), None));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:92 "returns false for shortcut without key"
#[test]
fn returns_false_for_shortcut_without_key() {
    assert!(!match_shortcut(&ctrl("s"), Some(&Shortcut::key(""))));
    assert!(!match_shortcut(&ctrl("s"), Some(&Shortcut::key("").ctrl())));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:100 "formats simple ctrl+key shortcuts"
#[test]
fn formats_simple_ctrl_key_shortcuts() {
    assert_eq!(format_shortcut(Some(sc("save")), false), "Ctrl+S");
    assert_eq!(format_shortcut(Some(sc("newNote")), false), "Ctrl+N");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:105 "formats ctrl+shift+key shortcuts"
#[test]
fn formats_ctrl_shift_key_shortcuts() {
    assert_eq!(format_shortcut(Some(sc("togglePreview")), false), "Ctrl+Shift+V");
    assert_eq!(format_shortcut(Some(sc("contentSearch")), false), "Ctrl+Shift+F");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:110 "formats function keys"
#[test]
fn formats_function_keys() {
    assert_eq!(format_shortcut(Some(sc("help")), false), "F1");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:114 "formats backslash key"
#[test]
fn formats_backslash_key() {
    assert_eq!(format_shortcut(Some(sc("typewriterMode")), false), "Ctrl+\\");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:118 "uses Mac-style formatting when requested"
#[test]
fn uses_mac_style_formatting_when_requested() {
    assert_eq!(format_shortcut(Some(sc("save")), true), "⌘S");
    assert_eq!(format_shortcut(Some(sc("togglePreview")), true), "⌘⇧V");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:123 "returns empty string for null/undefined"
#[test]
fn format_returns_empty_string_for_missing() {
    assert_eq!(format_shortcut(None, false), "");
    assert_eq!(format_shortcut(Some(&Shortcut::key("")), false), "");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:131 "returns a boolean"
// (Rust: the platform is a typed value; current() must agree with the build target.)
#[test]
fn is_macos_reflects_build_target() {
    assert_eq!(Platform::current().is_macos(), cfg!(target_os = "macos"));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:137 "is an alias for SHORTCUTS"
// (Rust has one table; `ShortcutMap::defaults()` must equal the const, in order.)
#[test]
fn defaults_map_is_the_const_table() {
    let expected: Vec<(String, Shortcut)> =
        DEFAULT_SHORTCUTS.iter().map(|(a, s)| ((*a).to_owned(), s.clone())).collect();
    assert_eq!(defaults().entries(), expected.as_slice());
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:141 "includes closeDialogs with preventDefault disabled"
#[test]
fn includes_close_dialogs_with_prevent_default_disabled() {
    let s = sc("closeDialogs");
    assert_eq!(*s, Shortcut::key("Escape").no_prevent_default());
    assert!(!s.prevent_default && !s.ctrl && !s.shift && !s.alt);
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:148 "includes toggleScrollSync on Ctrl+Shift+\\, distinct from typewriter's Ctrl+\\"
#[test]
fn includes_toggle_scroll_sync_distinct_from_typewriter() {
    assert_eq!(*sc("toggleScrollSync"), Shortcut::key("\\").ctrl().shift());
    assert_eq!(*sc("typewriterMode"), Shortcut::key("\\").ctrl());
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:160 "includes toggleTask on Ctrl+Enter"
#[test]
fn includes_toggle_task_on_ctrl_enter() {
    assert_eq!(*sc("toggleTask"), Shortcut::key("Enter").ctrl());
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:167 "binds every chord at most once (no collisions)"
#[test]
fn binds_every_chord_at_most_once() {
    let sigs: Vec<_> = DEFAULT_SHORTCUTS.iter().map(|(_, s)| (s.key.clone(), s.ctrl, s.shift, s.alt)).collect();
    let unique: HashSet<_> = sigs.iter().cloned().collect();
    assert_eq!(unique.len(), sigs.len(), "duplicate chord in DEFAULT_SHORTCUTS");
    let actions: HashSet<_> = DEFAULT_SHORTCUTS.iter().map(|(a, _)| *a).collect();
    assert_eq!(actions.len(), DEFAULT_SHORTCUTS.len(), "duplicate action id");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:186 "returns action name when shortcut matches"
#[test]
fn find_returns_action_name_when_matching() {
    assert_eq!(defaults().find_matching_action(&ctrl("s")), Some("save"));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:191 "returns null when no shortcut matches"
#[test]
fn find_returns_none_without_match() {
    assert_eq!(defaults().find_matching_action(&ctrl("x")), None);
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:196 "matches shift+ctrl shortcuts correctly"
#[test]
fn find_matches_shift_ctrl() {
    assert_eq!(defaults().find_matching_action(&ctrl_shift("V")), Some("togglePreview"));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:201 "matches Ctrl+Shift+\\ as toggleScrollSync (not typewriterMode)"
#[test]
fn find_ctrl_shift_backslash_is_scroll_sync() {
    let m = defaults();
    assert_eq!(m.find_matching_action(&ctrl_shift("\\")), Some("toggleScrollSync"));
    assert_eq!(m.find_matching_action(&ctrl("\\")), Some("typewriterMode"));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:209 "matches Ctrl+Enter as toggleTask"
#[test]
fn find_ctrl_enter_is_toggle_task() {
    let m = defaults();
    assert_eq!(m.find_matching_action(&ctrl("Enter")), Some("toggleTask"));
    assert_eq!(m.find_matching_action(&plain("Enter")), None);
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:217 "matches Escape key"
#[test]
fn find_matches_escape() {
    assert_eq!(defaults().find_matching_action(&plain("Escape")), Some("closeDialogs"));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:235 "calls action handler when shortcut matches"
#[test]
fn dispatch_reports_action_on_match() {
    let d = defaults().dispatch(&ctrl("s")).expect("save should dispatch");
    assert_eq!(d.action, "save");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:245 "calls preventDefault by default"
#[test]
fn dispatch_prevents_default_by_default() {
    assert!(defaults().dispatch(&ctrl("s")).unwrap().prevent_default);
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:255 "does not call preventDefault when disabled for shortcut"
#[test]
fn dispatch_respects_disabled_prevent_default() {
    let d = defaults().dispatch(&plain("Escape")).unwrap();
    assert_eq!(d.action, "closeDialogs");
    assert!(!d.prevent_default);
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:266 "does not call handler when no match"
#[test]
fn dispatch_none_without_match() {
    assert_eq!(defaults().dispatch(&ctrl("x")), None);
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:279 "returns defaults when no user shortcuts provided"
#[test]
fn merge_without_overrides_returns_defaults() {
    let none: Vec<(String, Option<Shortcut>)> = Vec::new();
    assert_eq!(defaults().merge(none), defaults());
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:284 "overrides defaults with user shortcuts"
#[test]
fn merge_overrides_defaults() {
    let merged = defaults().merge([("save", Some(Shortcut::new("w").ctrl()))]);
    assert_eq!(merged.get("save"), Some(&Shortcut::new("w").ctrl()));
    assert_eq!(merged.get("newNote"), Some(sc("newNote")));
    // Position is preserved (JS object spread keeps key order).
    assert_eq!(merged.entries()[1].0, "save");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:294 "removes shortcuts set to null"
#[test]
fn merge_removes_none() {
    let merged = defaults().merge([("save", None)]);
    assert_eq!(merged.get("save"), None);
    assert!(merged.get("newNote").is_some());
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:304 "adds new shortcuts from user config"
#[test]
fn merge_adds_new_actions() {
    let custom = Shortcut::new("q").ctrl().shift();
    let merged = defaults().merge([("customAction", Some(custom.clone()))]);
    assert_eq!(merged.get("customAction"), Some(&custom));
    assert_eq!(merged.entries().last().unwrap().0, "customAction");
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:313 "does not mutate original defaults"
#[test]
fn merge_does_not_mutate_input() {
    let base = ShortcutMap::from_entries([("save", Shortcut::key("s").ctrl())]);
    let before = base.clone();
    let _ = base.merge([("save", None)]);
    assert_eq!(base, before);
    assert_eq!(base.get("save"), Some(&Shortcut::key("s").ctrl()));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:324 "returns shortcut definition for known action"
#[test]
fn get_returns_known_action() {
    assert_eq!(defaults().get("save"), Some(&Shortcut::key("s").ctrl()));
}

// port: test/javascript/lib/keyboard_shortcuts.test.js:328 "returns null for unknown action"
#[test]
fn get_returns_none_for_unknown() {
    assert_eq!(defaults().get("unknownAction"), None);
}
