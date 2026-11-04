//! The engine.

use crate::{VimBuffer, ex::ExCall, keymap::Key};

/// Editing mode (drives the `VIM · NORMAL` indicator and cursor shape).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
    Replace,
    /// Typing an ex command after `:` (or a search after `/`, `?`).
    CommandLine,
}

/// What the app should do with the key after the engine saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Consumed; do not forward to the editor.
    Handled,
    /// Not ours (insert-mode text, IME composition, passthrough Ctrl-key):
    /// forward to the editor / app keymap.
    PassThrough,
    /// An ex command mapped to an app action.
    App(ExCall),
}

impl Mode {
    /// **[COMPAT]** indicator label (`observeVimMode`): `"normal"`, `"insert"`, `"visual"`,
    /// `"visual-linewise"`, `"visual-blockwise"`, `"replace"`. While typing a `:`/`/` line
    /// codemirror-vim still reports normal mode, so [`Mode::CommandLine`] → `"normal"`.
    pub fn label(self) -> &'static str {
        todo!()
    }
}

/// Engine state. One per editor view.
#[derive(Debug, Default)]
pub struct VimState {
    mode: Mode,
    pending: Vec<Key>,
    command_line: String,
    /// Platform: on macOS vim keeps its whole Ctrl keymap (no passthrough).
    macos: bool,
    /// Mode change not yet observed by the app.
    mode_changed: Option<Mode>,
}

impl VimState {
    /// New engine in normal mode. `macos` selects the passthrough policy
    /// ([`crate::keymap::app_owned_keys`]).
    pub fn new(macos: bool) -> Self {
        Self { macos, ..Self::default() }
    }

    /// The mode the engine switched to since the last call, if it changed
    /// (`vim-mode-change`; no change → `None`, like events without a mode).
    pub fn take_mode_change(&mut self) -> Option<Mode> {
        self.mode_changed.take()
    }

    /// Current mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Text of the `:` / `/` line being typed, if any.
    pub fn command_line(&self) -> Option<&str> {
        (self.mode == Mode::CommandLine).then_some(self.command_line.as_str())
    }

    /// Feed one key. `composing` = IME preedit active (always PassThrough).
    pub fn feed(&mut self, key: &Key, composing: bool, buf: &mut dyn VimBuffer) -> Outcome {
        todo!()
    }

    /// Whether IME should be enabled (insert/replace only, OM-007).
    pub fn ime_allowed(&self) -> bool {
        matches!(self.mode, Mode::Insert | Mode::Replace | Mode::CommandLine)
    }
}
