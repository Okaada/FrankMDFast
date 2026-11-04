//! Port of test/javascript/lib/vim_mode.test.js.
//!
//! The JS file tests glue over `@replit/codemirror-vim` through a mock (unmap/defineEx/
//! mode-change recording). Here the engine is ours, so the same contracts are asserted
//! against the engine API: the passthrough policy, the ex-command table and parser, mode
//! labels, and `next_note_index`.
#![allow(clippy::unwrap_used)]

mod common;

use common::{FakeBuffer, feed_str};
use frankmd_vim::ex::{self, EX_COMMANDS, ExCall, next_note_index};
use frankmd_vim::keymap::{Key, PASSTHROUGH, app_owned_keys, is_app_owned};
use frankmd_vim::{AppCommand, Mode, Outcome, VimState};
use std::collections::HashSet;

const APP_OWNED: [&str; 7] = [
    "<C-f>", "<C-n>", "<C-p>", "<C-e>", "<C-b>", "<C-i>", "<C-v>",
];

// port: test/javascript/lib/vim_mode.test.js:17 "hands the app-owned shortcuts back to FrankMD"
#[test]
fn hands_the_app_owned_shortcuts_back_to_frankmd() {
    // Non-macOS: exactly the app-owned list, in order.
    assert_eq!(app_owned_keys(false), APP_OWNED.map(str::to_owned).to_vec());
    // macOS: app shortcuts use Cmd, vim keeps its whole keymap.
    assert!(app_owned_keys(true).is_empty());

    // And the engine honours it: Ctrl-F in normal mode reaches the app on Linux/Windows...
    let mut buf = FakeBuffer::new("one\ntwo\n");
    let mut vim = VimState::new(false);
    for k in PASSTHROUGH {
        assert!(
            is_app_owned(&Key::ctrl(k), false),
            "<C-{k}> should be app-owned"
        );
        assert_eq!(
            vim.feed(&Key::ctrl(k), false, &mut buf),
            Outcome::PassThrough,
            "<C-{k}>"
        );
    }
    // ...but stays with vim on macOS.
    let mut vim = VimState::new(true);
    assert!(!is_app_owned(&Key::ctrl("f"), true));
    assert_eq!(vim.feed(&Key::ctrl("f"), false, &mut buf), Outcome::Handled);
}

// port: test/javascript/lib/vim_mode.test.js:30 "only mutates the shared keymap once"
// (JS guarded a module-global unmap; here the policy is a pure function of the platform,
// so repeated queries and repeated engines must agree.)
#[test]
fn passthrough_policy_is_stable_across_calls() {
    assert_eq!(app_owned_keys(false), app_owned_keys(false));
    assert_eq!(app_owned_keys(false).len(), APP_OWNED.len());
    let mut buf = FakeBuffer::new("x");
    let mut a = VimState::new(false);
    let mut b = VimState::new(false);
    assert_eq!(
        a.feed(&Key::ctrl("p"), false, &mut buf),
        b.feed(&Key::ctrl("p"), false, &mut buf)
    );
}

// port: test/javascript/lib/vim_mode.test.js:39 "covers the shortcuts the app and the markdown keymap bind"
#[test]
fn covers_the_shortcuts_the_app_and_markdown_keymap_bind() {
    let keys: HashSet<String> = app_owned_keys(false).into_iter().collect();
    for k in APP_OWNED {
        assert!(keys.contains(k), "{k} missing");
    }
    assert!(!keys.contains("<C-d>"));
    assert!(!keys.contains("<C-q>"));
    assert!(!is_app_owned(&Key::ctrl("d"), false));
    assert!(!is_app_owned(&Key::ctrl("u"), false));
    assert!(!is_app_owned(&Key::ctrl("q"), false));
    // Notation round-trip used by the list.
    assert_eq!(Key::ctrl("f").notation(), "<C-f>");
}

// port: test/javascript/lib/vim_mode.test.js:51 "defines one ex-command per entry, with its abbreviation prefix"
#[test]
fn defines_one_ex_command_per_entry_with_prefix() {
    let names: Vec<&str> = EX_COMMANDS.iter().map(|c| c.name).collect();
    assert_eq!(names.len(), 10);
    assert!(names.contains(&"write"));
    assert!(names.contains(&"quit"));
    assert!(names.contains(&"Explore"));
    // :noh is intentionally NOT overridden (engine owns it).
    assert!(!names.contains(&"nohlsearch"));
    for c in EX_COMMANDS {
        assert!(
            c.name.starts_with(c.prefix),
            "{} prefix {}",
            c.name,
            c.prefix
        );
    }
    // Full table, verbatim.
    let table: Vec<(&str, &str, &str)> = EX_COMMANDS
        .iter()
        .map(|c| (c.name, c.prefix, c.command.logical_name()))
        .collect();
    assert_eq!(
        table,
        vec![
            ("write", "w", "save"),
            ("quit", "q", "close"),
            ("wq", "wq", "save-and-close"),
            ("xit", "x", "save-and-close"),
            ("help", "help", "help"),
            ("edit", "e", "finder"),
            ("find", "find", "finder"),
            ("Explore", "Explore", "toggle-sidebar"),
            ("next", "n", "next-note"),
            ("prev", "prev", "prev-note"),
        ]
    );
    // Abbreviations resolve; :noh, :s and :<n> stay native.
    let cmd = |l: &str| ex::parse(l).map(|c| c.command);
    assert_eq!(cmd("w"), Some(AppCommand::Save));
    assert_eq!(cmd("wri"), Some(AppCommand::Save));
    assert_eq!(cmd("wq"), Some(AppCommand::SaveAndClose));
    assert_eq!(cmd("x"), Some(AppCommand::SaveAndClose));
    assert_eq!(cmd("q"), Some(AppCommand::CloseDialogs));
    assert_eq!(cmd("e"), Some(AppCommand::OpenFinder));
    assert_eq!(cmd("find"), Some(AppCommand::OpenFinder));
    assert_eq!(cmd("n"), Some(AppCommand::NextNote));
    assert_eq!(cmd("prev"), Some(AppCommand::PrevNote));
    assert_eq!(cmd("pre"), None, "below the prefix");
    assert_eq!(cmd("Exp"), None, "below the prefix");
    assert_eq!(cmd("explore"), None, "case-sensitive");
    assert_eq!(cmd("noh"), None);
    assert_eq!(cmd("nohlsearch"), None);
    assert_eq!(cmd("s/a/b/"), None);
    assert_eq!(cmd("12"), None);
    assert_eq!(cmd("writex"), None);
}

// port: test/javascript/lib/vim_mode.test.js:63 "invokes onCommand with the logical command name when an ex-command runs"
#[test]
fn ex_command_invokes_logical_command() {
    assert_eq!(
        ex::parse("write"),
        Some(ExCall {
            command: AppCommand::Save,
            args: vec![]
        })
    );
    assert_eq!(AppCommand::Save.logical_name(), "save");
    assert_eq!(
        ex::parse("Explore").unwrap().command.logical_name(),
        "toggle-sidebar"
    );
    assert_eq!(
        ex::parse("e notes/a.md"),
        Some(ExCall {
            command: AppCommand::OpenFinder,
            args: vec!["notes/a.md".into()]
        })
    );

    // Through the engine: typing `:w<Enter>` in normal mode yields the app command.
    let mut buf = FakeBuffer::new("hello\n");
    let mut vim = VimState::new(false);
    let out = feed_str(&mut vim, &mut buf, ":w\n");
    assert_eq!(
        out.last(),
        Some(&Outcome::App(ExCall {
            command: AppCommand::Save,
            args: vec![]
        }))
    );
    assert_eq!(vim.mode(), Mode::Normal);
    assert_eq!(buf.text, "hello\n", ":w must not edit the buffer");

    let out = feed_str(&mut vim, &mut buf, ":Explore\n");
    assert_eq!(
        out.last(),
        Some(&Outcome::App(ExCall {
            command: AppCommand::ToggleSidebar,
            args: vec![]
        }))
    );

    // While typing, the command line is visible.
    let _ = feed_str(&mut vim, &mut buf, ":wq");
    assert_eq!(vim.command_line(), Some("wq"));
}

// port: test/javascript/lib/vim_mode.test.js:76 "is idempotent — a second call does not double-register"
// (Rust: the table is a const; assert names are unique so no command is registered twice.)
#[test]
fn ex_table_has_no_duplicates() {
    let names: HashSet<&str> = EX_COMMANDS.iter().map(|c| c.name).collect();
    assert_eq!(names.len(), EX_COMMANDS.len());
    let prefixes: HashSet<&str> = EX_COMMANDS.iter().map(|c| c.prefix).collect();
    assert_eq!(prefixes.len(), EX_COMMANDS.len());
}

// port: test/javascript/lib/vim_mode.test.js:84 "maps mode-change events to onModeChange, including visual submode"
#[test]
fn maps_mode_changes_including_visual_submode() {
    assert_eq!(Mode::Normal.label(), "normal");
    assert_eq!(Mode::Insert.label(), "insert");
    assert_eq!(Mode::Visual.label(), "visual");
    assert_eq!(Mode::VisualLine.label(), "visual-linewise");
    assert_eq!(Mode::VisualBlock.label(), "visual-blockwise");
    assert_eq!(Mode::Replace.label(), "replace");
    assert_eq!(Mode::CommandLine.label(), "normal");

    let mut buf = FakeBuffer::new("abc\ndef\n");
    let mut vim = VimState::new(false);
    feed_str(&mut vim, &mut buf, "i");
    assert_eq!(vim.take_mode_change(), Some(Mode::Insert));
    feed_str(&mut vim, &mut buf, "\x1b");
    assert_eq!(vim.take_mode_change(), Some(Mode::Normal));
    feed_str(&mut vim, &mut buf, "V");
    assert_eq!(
        vim.take_mode_change().map(Mode::label),
        Some("visual-linewise")
    );
}

// port: test/javascript/lib/vim_mode.test.js:101 "ignores events with no mode"
// (Rust: keys that do not change mode report no change.)
#[test]
fn no_mode_change_reported_without_change() {
    let mut buf = FakeBuffer::new("abc\ndef\n");
    let mut vim = VimState::new(false);
    assert_eq!(vim.take_mode_change(), None);
    feed_str(&mut vim, &mut buf, "j");
    assert_eq!(vim.take_mode_change(), None);
    feed_str(&mut vim, &mut buf, "i");
    assert_eq!(vim.take_mode_change(), Some(Mode::Insert));
    assert_eq!(vim.take_mode_change(), None, "a change is reported once");
}

// port: test/javascript/lib/vim_mode.test.js:111 "wraps forward and backward"
#[test]
fn next_note_index_wraps() {
    assert_eq!(next_note_index(3, Some(0), true), Some(1));
    assert_eq!(next_note_index(3, Some(2), true), Some(0));
    assert_eq!(next_note_index(3, Some(0), false), Some(2));
    assert_eq!(next_note_index(3, Some(1), false), Some(0));
}

// port: test/javascript/lib/vim_mode.test.js:119 "starts at first (forward) or last (backward) when nothing is selected"
#[test]
fn next_note_index_without_selection() {
    assert_eq!(next_note_index(3, None, true), Some(0));
    assert_eq!(next_note_index(3, None, false), Some(2));
}

// port: test/javascript/lib/vim_mode.test.js:125 "returns -1 when there are no notes"
#[test]
fn next_note_index_no_notes() {
    assert_eq!(next_note_index(0, None, true), None);
    assert_eq!(next_note_index(0, None, false), None);
}

// Cross-crate registry check (architecture.md "one registry", HP-018): every ex command's
// shortcut actions exist in the app shortcut table, and the passthrough Ctrl keys that are
// app shortcuts are bound there on Ctrl (not swallowed by vim).
// port: app/javascript/controllers/app_controller.js:3353 onVimCommand (logic, no JS test)
#[test]
fn ex_commands_dispatch_registered_shortcut_actions() {
    use frankmd_workspace::shortcuts::DEFAULT_SHORTCUTS;
    let actions: HashSet<&str> = DEFAULT_SHORTCUTS.iter().map(|(a, _)| *a).collect();
    for c in EX_COMMANDS {
        for a in c.command.shortcut_actions() {
            assert!(actions.contains(a), "{} → unknown action {a}", c.name);
        }
    }
    assert_eq!(
        AppCommand::SaveAndClose.shortcut_actions(),
        &["save", "closeDialogs"]
    );
    assert_eq!(
        AppCommand::CloseDialogs.shortcut_actions(),
        &["closeDialogs"]
    );
    assert_eq!(AppCommand::OpenFinder.shortcut_actions(), &["fileFinder"]);
    assert_eq!(
        AppCommand::ToggleSidebar.shortcut_actions(),
        &["toggleSidebar"]
    );
    assert!(AppCommand::NextNote.shortcut_actions().is_empty());
    for k in ["f", "n", "p", "e"] {
        assert!(
            DEFAULT_SHORTCUTS
                .iter()
                .any(|(_, s)| s.key == k && s.ctrl && !s.shift && !s.alt),
            "Ctrl+{k} should be an app shortcut"
        );
    }
}
