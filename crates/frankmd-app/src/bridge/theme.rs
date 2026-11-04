//! Applies `frankmd_theme::UiTokens` (FrankMD's 38 colour keys) to gpui-kit's
//! global `Theme` (ui-spec §7.1 role table), plus the corner radius (§7.4).
//!
//! Called on startup, from the theme picker (live preview) and on every
//! `WatchEvent::OmarchyThemeChanged`. Mode first (it reloads the registered
//! base theme), then the colours in a second `Theme::update` so they win.

use frankmd_theme::{Mode, Rgb, UiTokens};
use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{App, Global, Hsla, SharedString, px, rgb};

/// Hyprland's default rounding is 0; without Hyprland the spec uses 6 (§7.4).
pub(crate) const DEFAULT_RADIUS: f32 = 6.0;

fn hsla(c: Rgb) -> Hsla {
    rgb((u32::from(c.0) << 16) | (u32::from(c.1) << 8) | u32::from(c.2)).into()
}

/// Colour of token `key` (black for an unknown key; tokens always carry all 38).
fn token(tokens: &UiTokens, key: &str) -> Hsla {
    hsla(tokens.get(key).unwrap_or(Rgb(0, 0, 0)))
}

/// `a` mixed toward `b` by `t` (sRGB, per channel).
fn mix(tokens: &UiTokens, a: &str, b: &str, t: f32) -> Hsla {
    let (Some(a), Some(b)) = (tokens.get(a), tokens.get(b)) else {
        return token(tokens, a);
    };
    hsla(a.mix(b, t))
}

/// System font families from fontconfig, resolved once at startup.
///
/// Kept as a global because `Theme::change` reloads the base theme (and its
/// fonts) whenever the light/dark mode flips.
#[derive(Debug, Clone, Default)]
pub(crate) struct SystemFonts {
    /// `sans-serif` family for the UI.
    pub(crate) ui: Option<SharedString>,
    /// `monospace` family for code and paths.
    pub(crate) mono: Option<SharedString>,
}

impl Global for SystemFonts {}

/// Apply `tokens` and corner radius `radius` (px) to the global theme.
pub(crate) fn apply(tokens: &UiTokens, radius: f32, cx: &mut App) {
    let mode = match tokens.mode {
        Mode::Light => ThemeMode::Light,
        Mode::Dark => ThemeMode::Dark,
    };
    if Theme::global(cx).mode != mode {
        Theme::change(mode, None, cx);
    }
    let fonts = cx.try_global::<SystemFonts>().cloned().unwrap_or_default();
    let r_sm = radius.min(4.0);
    Theme::update(cx, |theme| {
        if let Some(font) = fonts.ui {
            theme.font_family = font;
        }
        if let Some(font) = fonts.mono {
            theme.mono_font_family = font;
        }
        let c = &mut theme.colors;
        let base = token(tokens, "bg-primary");
        let recessed = token(tokens, "bg-secondary");
        let raised = token(tokens, "bg-tertiary");
        let hover = token(tokens, "bg-hover");
        let fg = token(tokens, "text-primary");
        let muted = token(tokens, "text-muted");
        let border = token(tokens, "border");
        let subtle = mix(tokens, "border", "bg-primary", 0.5);
        let accent = token(tokens, "accent");
        let accent_hover = token(tokens, "accent-hover");
        let on_accent = token(tokens, "accent-text");
        let selection = token(tokens, "selection");
        let success = token(tokens, "success");
        let warning = token(tokens, "warning");
        let danger = token(tokens, "error");

        c.background = base;
        c.foreground = fg;
        c.border = border;
        c.input = border;
        c.ring = accent;
        c.caret = token(tokens, "selection-text");
        c.selection = selection;
        c.muted = hover;
        c.muted_foreground = muted;
        c.accent = hover;
        c.accent_foreground = fg;
        c.primary = accent;
        c.primary_hover = accent_hover;
        c.primary_active = accent_hover;
        c.primary_foreground = on_accent;
        c.secondary = raised;
        c.secondary_hover = hover;
        c.secondary_active = hover;
        c.secondary_foreground = fg;
        c.popover = raised;
        c.popover_foreground = fg;
        c.sidebar = recessed;
        c.sidebar_foreground = fg;
        c.sidebar_border = subtle;
        c.sidebar_accent = hover;
        c.sidebar_accent_foreground = fg;
        c.sidebar_primary = accent;
        c.sidebar_primary_foreground = on_accent;
        c.title_bar = recessed;
        c.title_bar_border = subtle;
        c.status_bar = recessed;
        c.status_bar_border = subtle;
        c.tab_bar = recessed;
        c.tab = recessed;
        c.tab_active = base;
        c.tab_foreground = muted;
        c.tab_active_foreground = fg;
        c.list = base;
        c.list_hover = hover;
        c.list_active = selection;
        c.list_active_border = accent;
        c.list_even = base;
        c.list_head = recessed;
        c.link = accent;
        c.link_hover = accent_hover;
        c.link_active = accent_hover;
        c.scrollbar = gpui_kit::transparent_black();
        c.scrollbar_thumb = token(tokens, "scrollbar");
        c.scrollbar_thumb_hover = token(tokens, "scrollbar-hover");
        c.success = success;
        c.success_hover = success;
        c.success_active = success;
        c.success_foreground = on_accent;
        c.warning = warning;
        c.warning_hover = warning;
        c.warning_active = warning;
        c.warning_foreground = base;
        c.danger = danger;
        c.danger_hover = danger;
        c.danger_active = danger;
        c.danger_foreground = base;
        c.info = token(tokens, "heading-3");
        c.info_hover = c.info;
        c.info_active = c.info;
        c.info_foreground = base;
        c.drop_target = accent.opacity(0.12);
        c.drag_border = accent;
        c.window_border = subtle;
        c.overlay = base.opacity(0.6);
        c.switch = border;
        c.slider_bar = accent;
        c.slider_thumb = on_accent;
        c.progress_bar = accent;
        c.skeleton = hover;
        c.table = base;
        c.table_head = recessed;
        c.table_hover = hover;
        c.table_active = selection;
        c.table_row_border = subtle;
        theme.radius = px(r_sm);
        theme.radius_lg = px(radius);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip_through_hsla() {
        let c = hsla(Rgb(0x33, 0x66, 0x99));
        let back = gpui_kit::Rgba::from(c);
        assert!(
            (back.r - 0.2).abs() < 0.01
                && (back.g - 0.4).abs() < 0.01
                && (back.b - 0.6).abs() < 0.01
        );
    }
}
