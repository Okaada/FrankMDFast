//! Keyboard shortcut table, matching and formatting (port of lib/keyboard_shortcuts.js).
//!
//! **[COMPAT]** semantics kept from the original:
//! - Matching is on the exact DOM-style `key` string (`"V"` for Ctrl+Shift+V, `"Enter"`,
//!   `"Escape"`, `"F1"`, `"\\"`). The app crate normalizes GPUI keystrokes into
//!   [`KeyEvent`] with these names before calling in.
//! - `ctrl` in a shortcut matches Ctrl **or** Meta (Cmd) on the event; `shift` and `alt`
//!   must match exactly. A shortcut never inspects Meta on its own.
//! - The table is ordered: [`ShortcutMap::find_matching_action`] returns the FIRST match.
//! - User overrides merge per action; `None` removes the action; new actions append.
//!
//! Action ids are the original camelCase names (`save`, `closeDialogs`, `fileFinder`, ...).
//! Vim ex commands reach the same actions through `frankmd_vim::AppCommand::shortcut_actions`
//! (one registry, HP-018).
//!
//! [`DEFAULT_SHORTCUTS`] is real data (it *is* the spec, recorded verbatim from the JS);
//! everything else is a `todo!()` skeleton.

use std::borrow::Cow;

/// A key press, in DOM `KeyboardEvent` terms.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyEvent {
    /// DOM `event.key` (already shifted: `"V"`, `"+"`, `"_"`; named keys: `"Enter"`, `"F1"`).
    pub key: String,
    /// Ctrl held.
    pub ctrl: bool,
    /// Shift held.
    pub shift: bool,
    /// Alt/Option held.
    pub alt: bool,
    /// Meta/Cmd/Super held.
    pub meta: bool,
}

/// One shortcut definition. An empty `key` means "no key" (never matches, formats as `""`),
/// mirroring the JS `!shortcut.key` checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// DOM key name, case-sensitive.
    pub key: Cow<'static, str>,
    /// Ctrl (or Meta) required.
    pub ctrl: bool,
    /// Shift required.
    pub shift: bool,
    /// Alt required.
    pub alt: bool,
    /// Whether dispatch should swallow the native event (`preventDefault`). Default true;
    /// only `closeDialogs` (Escape) opts out.
    pub prevent_default: bool,
}

impl Shortcut {
    /// Plain key, no modifiers, prevents default.
    pub const fn key(key: &'static str) -> Self {
        Self { key: Cow::Borrowed(key), ctrl: false, shift: false, alt: false, prevent_default: true }
    }
    /// Owned-key constructor (user overrides).
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: Cow::Owned(key.into()), ctrl: false, shift: false, alt: false, prevent_default: true }
    }
    /// Require Ctrl/Meta.
    pub const fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }
    /// Require Shift.
    pub const fn shift(mut self) -> Self {
        self.shift = true;
        self
    }
    /// Require Alt.
    pub const fn alt(mut self) -> Self {
        self.alt = true;
        self
    }
    /// Let the native event through after dispatch.
    pub const fn no_prevent_default(mut self) -> Self {
        self.prevent_default = false;
        self
    }
}

/// **[COMPAT]** `DEFAULT_SHORTCUTS`, in the original insertion order (order is load-bearing:
/// first match wins). Bold/italic (Ctrl+B/I) are editor keymap bindings, not app shortcuts.
pub const DEFAULT_SHORTCUTS: &[(&str, Shortcut)] = &[
    ("newNote", Shortcut::key("n").ctrl()),
    ("save", Shortcut::key("s").ctrl()),
    ("togglePreview", Shortcut::key("V").ctrl().shift()),
    ("findInFile", Shortcut::key("f").ctrl()),
    ("findReplace", Shortcut::key("h").ctrl()),
    ("jumpToLine", Shortcut::key("g").ctrl()),
    ("lineNumbers", Shortcut::key("l").ctrl()),
    ("contentSearch", Shortcut::key("F").ctrl().shift()),
    ("fileFinder", Shortcut::key("p").ctrl()),
    ("toggleSidebar", Shortcut::key("e").ctrl()),
    ("typewriterMode", Shortcut::key("\\").ctrl()),
    ("toggleTask", Shortcut::key("Enter").ctrl()),
    ("toggleScrollSync", Shortcut::key("\\").ctrl().shift()),
    ("textFormat", Shortcut::key("m").ctrl()),
    ("emojiPicker", Shortcut::key("E").ctrl().shift()),
    ("increaseWidth", Shortcut::key("+").ctrl().shift()),
    ("decreaseWidth", Shortcut::key("_").ctrl().shift()),
    ("logViewer", Shortcut::key("O").ctrl().shift()),
    ("help", Shortcut::key("F1")),
    ("closeDialogs", Shortcut::key("Escape").no_prevent_default()),
];

/// Ordered `action -> shortcut` table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShortcutMap {
    entries: Vec<(String, Shortcut)>,
}

/// What a key handler decided for one event (`createKeyHandler`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyDispatch {
    /// Matched action id.
    pub action: String,
    /// Whether to swallow the native event.
    pub prevent_default: bool,
}

impl ShortcutMap {
    /// The [`DEFAULT_SHORTCUTS`] table.
    pub fn defaults() -> Self {
        todo!("ShortcutMap::from_entries(DEFAULT_SHORTCUTS)")
    }

    /// Build from ordered entries. A repeated action keeps its first position and the last
    /// value (JS object-literal semantics).
    pub fn from_entries<S: Into<String>>(entries: impl IntoIterator<Item = (S, Shortcut)>) -> Self {
        todo!()
    }

    /// Entries in order (`Object.entries`).
    pub fn entries(&self) -> &[(String, Shortcut)] {
        &self.entries
    }

    /// `getShortcutForAction`. Unknown action → `None` (the JS prototype leak where
    /// `"constructor"`/`"toString"` returned functions is not replicated).
    pub fn get(&self, action: &str) -> Option<&Shortcut> {
        todo!()
    }

    /// `findMatchingAction`: first action (in order) whose shortcut matches.
    pub fn find_matching_action(&self, event: &KeyEvent) -> Option<&str> {
        todo!()
    }

    /// `createKeyHandler` decision: the matched action and whether to prevent default.
    /// `None` = no match (handler not called, default not prevented).
    pub fn dispatch(&self, event: &KeyEvent) -> Option<KeyDispatch> {
        todo!()
    }

    /// `mergeShortcuts`: overrides replace in place, new actions append, `None` removes.
    /// `self` is not mutated.
    pub fn merge<S: AsRef<str>>(&self, overrides: impl IntoIterator<Item = (S, Option<Shortcut>)>) -> Self {
        todo!()
    }
}

/// `matchShortcut`. `None` or an empty-key shortcut never matches.
pub fn match_shortcut(event: &KeyEvent, shortcut: Option<&Shortcut>) -> bool {
    todo!()
}

/// `formatShortcut`: `"Ctrl+Shift+V"`, or with `mac` `"⌘⇧V"` (no separator; `⌥` for Alt).
/// Single-character keys are upper-cased; `F1`..`F12`, `\` and named keys stay as-is.
/// `None` / empty key → `""`.
pub fn format_shortcut(shortcut: Option<&Shortcut>, mac: bool) -> String {
    todo!()
}

/// Host platform, injected instead of reading a global `navigator` (`isMacOS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// macOS: app shortcuts use Cmd; vim keeps its full Ctrl keymap.
    MacOS,
    /// Linux / Windows.
    Other,
}

/// The parts of a browser `navigator` that `isMacOS` inspected (fixture compatibility).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NavigatorInfo {
    /// `navigator.userAgentData` present → `Some(platform)` (empty if it had no platform).
    pub user_agent_data_platform: Option<String>,
    /// `navigator.userAgent`.
    pub user_agent: Option<String>,
}

impl Platform {
    /// Compile-time platform of this build.
    pub fn current() -> Self {
        todo!("cfg!(target_os = \"macos\")")
    }

    /// `isMacOS` over a navigator: none → Other; `userAgentData` wins when present
    /// (`platform == "macOS"`); else `/Mac|iPod|iPhone|iPad/` on the user agent.
    pub fn from_navigator(nav: Option<&NavigatorInfo>) -> Self {
        todo!()
    }

    /// Convenience.
    pub fn is_macos(self) -> bool {
        self == Self::MacOS
    }
}
