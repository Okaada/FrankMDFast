//! Key representation and the non-macOS passthrough list.

/// A key event normalized by the app crate (from GPUI `Keystroke`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    /// Key name (`"a"`, `"escape"`, `"enter"`, `"backspace"`, `"left"`, ...), already shifted.
    pub key: String,
    /// Ctrl held.
    pub ctrl: bool,
    /// Alt held.
    pub alt: bool,
}

impl Key {
    /// Unmodified key.
    pub fn plain(key: &str) -> Self {
        Self { key: key.to_owned(), ctrl: false, alt: false }
    }
    /// Ctrl + key.
    pub fn ctrl(key: &str) -> Self {
        Self { key: key.to_owned(), ctrl: true, alt: false }
    }
    /// Vim notation (`"<C-f>"`, `"a"`, `"<Esc>"`).
    pub fn notation(&self) -> String {
        todo!()
    }
}

/// **[COMPAT]** Ctrl-keys left to the app on non-macOS (`vim_mode.js` `APP_OWNED_KEYS`):
/// find in file, new note, file finder, toggle sidebar, bold, italic, paste.
/// `<C-d>`/`<C-u>` paging and `<C-q>` (blockwise visual in place of `<C-v>`) stay with vim.
pub const PASSTHROUGH: [&str; 7] = ["f", "n", "p", "e", "b", "i", "v"];

/// The app-owned keys for a platform, in vim notation and original order
/// (`configureVimKeys`): `["<C-f>", "<C-n>", "<C-p>", "<C-e>", "<C-b>", "<C-i>", "<C-v>"]` on
/// Linux/Windows, empty on macOS (app shortcuts use Cmd there, vim keeps its whole keymap).
pub fn app_owned_keys(macos: bool) -> Vec<String> {
    todo!()
}

/// Whether `key` must be handed back to the app instead of vim on this platform.
pub fn is_app_owned(key: &Key, macos: bool) -> bool {
    todo!()
}
