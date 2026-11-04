//! Port of test/javascript/lib/codemirror_extensions_vim.test.js (category C).
//!
//! Skipped (UI wiring with no engine logic; see docs/testing/port-editor.md):
//! - :8 "returns the vim extension when enabled" — CodeMirror extension object/options.
//! - :15 "returns nothing (empty) when disabled" — with vim off the app simply creates no
//!   `VimState`; nothing to assert in this crate.
//! - :19 "adds the themed block cursor after vim" — cursor theming, app crate.
//!
//! Ported: :30 the load-bearing precedence rule ("vim sees keydown before any keymap"),
//! expressed as the engine contract the app relies on: in normal mode the engine CONSUMES
//! Enter/Backspace/arrows (they must not fall through to the editor's default keymap); in
//! insert mode plain typing passes through.
#![allow(clippy::unwrap_used)]

mod common;

use common::{FakeBuffer, feed_str};
use frankmd_vim::keymap::Key;
use frankmd_vim::{Mode, Outcome, VimState};

// port: test/javascript/lib/codemirror_extensions_vim.test.js:30 "keeps the vim compartment first in createExtensions()"
#[test]
fn vim_layer_consumes_normal_mode_keys_before_editor_keymap() {
    let mut buf = FakeBuffer::new("first line\nsecond line\n");
    buf.sel = 3..3;
    let mut vim = VimState::new(false);
    for k in ["enter", "backspace", "left", "right", "up", "down"] {
        assert_eq!(
            vim.feed(&Key::plain(k), false, &mut buf),
            Outcome::Handled,
            "normal-mode {k}"
        );
    }
    assert_eq!(
        buf.text, "first line\nsecond line\n",
        "normal-mode keys must not edit text"
    );

    feed_str(&mut vim, &mut buf, "i");
    assert_eq!(vim.mode(), Mode::Insert);
    assert_eq!(
        vim.feed(&Key::plain("x"), false, &mut buf),
        Outcome::PassThrough
    );
    assert_eq!(
        vim.feed(&Key::plain("enter"), false, &mut buf),
        Outcome::PassThrough
    );
    // IME composition always passes through.
    assert_eq!(
        vim.feed(&Key::plain("a"), true, &mut buf),
        Outcome::PassThrough
    );
    assert!(vim.ime_allowed());
}
