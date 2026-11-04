//! Adapters between GUI-free crates and GPUI.
//!
//! - [`tokio`]: run `frankmd-net` futures from GPUI tasks.
//! - [`theme`]: `frankmd_theme::UiTokens` → gpui-kit `ThemeConfig`.
//! - [`vim`]: `frankmd_vim::VimBuffer` over gpui-kit `EditorState`.

pub(crate) mod theme;
pub(crate) mod tokio;
pub(crate) mod vim;
