//! Applies `frankmd_theme::UiTokens` to gpui-kit at runtime.
//!
//! Plan: build a `gpui_kit::component::theme::ThemeConfig` (the same serde
//! schema as gpui-kit's `themes/*.json`: `colors` + `highlight.syntax`) from
//! the 38 FrankMD tokens, then
//! `Theme::update(cx, |t| t.apply_config(&Rc::new(cfg)))` and
//! `cx.refresh_windows()`. Triggered on startup and on every
//! `WatchEvent::OmarchyThemeChanged` (target: < 100 ms end to end).

/// Apply tokens to the global theme (implemented with the theme crate).
pub(crate) fn apply(_tokens: &frankmd_theme::UiTokens, _cx: &mut gpui_kit::App) {
    // Intentionally empty in the scaffold.
}
