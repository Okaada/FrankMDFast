//! # frankmd-theme
//!
//! Produces a GUI-free [`UiTokens`] set (FrankMD's 38 `COLOR_KEYS`) from:
//! - the active **Omarchy** palette (`~/.local/state/omarchy/current/theme/colors.toml`)
//!   with the full `omarchy-theme-color` alias/derivation cascade (OM-001/002), see [`omarchy`],
//! - one of the built-in themes ([`builtin`]), or
//! - a user custom theme (`.fed custom_theme`, **[COMPAT]**, see [`custom`]).
//!
//! [`legacy`] is a byte-faithful port of FrankMD's `OmarchyThemeService`
//! (alacritty.toml under the pre-4.0 `~/.config/omarchy/current`), kept so the
//! golden fixtures pin the original math (`ensure_contrast`, `lighten`, ...).
//!
//! The app crate maps `UiTokens` onto gpui-kit's `ThemeConfig` (UI colors +
//! `highlight` syntax colors) and applies it at runtime; [`editor`] holds the
//! editor-chrome / markdown-highlight role mapping ported from
//! `codemirror_theme.js`.
#![allow(unused_variables, dead_code)] // skeleton

pub mod custom;
pub mod editor;
pub mod legacy;
pub mod omarchy;

pub use custom::{CustomTheme, CustomThemeError};
pub use omarchy::{OmarchyPalette, OmarchyPaths, from_omarchy};

use std::collections::BTreeMap;

/// **[COMPAT]** `CustomTheme::COLOR_KEYS` / `custom_theme.js COLOR_KEYS`, in
/// the original order (the `--theme-<key>` CSS variables of every theme).
pub const COLOR_KEYS: [&str; 38] = [
    "bg-primary", "bg-secondary", "bg-tertiary", "bg-hover", "text-primary", "text-secondary",
    "text-muted", "text-faint", "border", "accent", "accent-hover", "accent-text", "selection",
    "selection-text", "scrollbar", "scrollbar-hover", "success", "warning", "error", "code-bg",
    "heading-1", "heading-2", "heading-3", "folder-icon", "file-icon", "config-icon",
    "syntax-keyword", "syntax-function", "syntax-type", "syntax-string", "syntax-number",
    "syntax-constant", "syntax-identifier", "syntax-property", "syntax-operator",
    "syntax-comment", "syntax-punctuation", "syntax-invalid",
];

/// **[COMPAT]** `CustomTheme::BASE_THEMES`: ids of the 18 built-in themes
/// (also the valid `base` of a custom theme; `omarchy` and `custom` are not).
pub const BASE_THEMES: [&str; 18] = [
    "light", "dark", "catppuccin", "catppuccin-latte", "ethereal", "everforest", "flexoki-light",
    "gruvbox", "hackerman", "kanagawa", "matte-black", "nord", "osaka-jade", "ristretto",
    "rose-pine", "solarized-dark", "solarized-light", "tokyo-night",
];

/// The 12 syntax roles that must reach 4.5:1 against `code-bg`.
pub const SYNTAX_ROLES: [&str; 12] = [
    "keyword", "function", "type", "string", "number", "constant", "identifier", "property",
    "operator", "comment", "punctuation", "invalid",
];

/// sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Parse strict `#rrggbb`, either case (theme editor accepts nothing else, `09e5447`).
    pub fn parse_hex(s: &str) -> Option<Self> {
        todo!()
    }
    /// Lowercase `#rrggbb`.
    pub fn to_hex(self) -> String {
        todo!()
    }
    /// Linear per-channel sRGB mix with rounding (Omarchy `mix_color`):
    /// `t = 0` → `self`, `t = 1` → `other`.
    pub fn mix(self, other: Rgb, t: f32) -> Rgb {
        todo!()
    }
}

/// Light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Mode {
    Light,
    Dark,
}

/// FrankMD's semantic UI tokens (`custom_theme.rb` `COLOR_KEYS`).
#[derive(Debug, Clone, PartialEq)]
pub struct UiTokens {
    /// Mode for gpui-kit's light/dark component defaults.
    pub mode: Mode,
    /// Token name (`bg-primary`, `syntax-keyword`, ...) → color. Always all 38 keys.
    pub colors: BTreeMap<&'static str, Rgb>,
}

impl UiTokens {
    /// Color of one token (`None` for a name outside [`COLOR_KEYS`]).
    pub fn get(&self, key: &str) -> Option<Rgb> {
        todo!()
    }

    /// Overlay a custom theme's colors on top of these tokens (its `base`
    /// is resolved by the caller via [`builtin`]).
    pub fn with_overrides(&self, custom: &CustomTheme) -> UiTokens {
        todo!()
    }
}

/// FrankMD's 18 built-in themes (38 `--theme-*` roles each), embedded from
/// `assets/themes/builtin.json` (extracted unchanged from FrankMD).
pub const BUILTIN_THEMES_JSON: &str = include_str!("../../../assets/themes/builtin.json");

/// Ids of the built-in themes, in `builtin.json` order.
pub fn builtin_ids() -> Vec<&'static str> {
    todo!("parse BUILTIN_THEMES_JSON once (LazyLock)")
}

/// Built-in theme by **[COMPAT]** id (`dark`, `tokyo-night`, ...). `omarchy`
/// and `custom` are not built-ins (they are resolved by [`omarchy`] / [`custom`]).
/// Mode comes from the luminance of `bg-primary`.
pub fn builtin(id: &str) -> Option<UiTokens> {
    todo!("parse BUILTIN_THEMES_JSON once (LazyLock) and map to UiTokens")
}

/// WCAG relative luminance (sRGB linearized).
pub fn relative_luminance(c: Rgb) -> f64 {
    todo!()
}

/// WCAG contrast ratio `(Lmax + 0.05) / (Lmin + 0.05)`.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    todo!()
}

/// **[COMPAT]** `ensure_contrast` binary search (omarchy_theme_service.rb):
/// returns `fg` unchanged when it already reaches `ratio`, otherwise blends it
/// toward black or white (whichever contrasts better with `bg`).
pub fn ensure_contrast(fg: Rgb, bg: Rgb, ratio: f32) -> Rgb {
    todo!()
}

/// Theme errors.
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    /// `colors.toml` was unreadable (e.g. mid atomic swap; caller keeps the last good theme, HP-024).
    #[error("invalid palette: {0}")]
    Invalid(String),
    /// I/O failure reading an Omarchy file that exists (EISDIR, ...).
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// A file that must be UTF-8 was not.
    #[error("invalid byte sequence in UTF-8")]
    Encoding,
}
