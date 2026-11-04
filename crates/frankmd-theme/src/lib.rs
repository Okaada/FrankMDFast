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

pub mod custom;
pub mod editor;
pub mod legacy;
pub mod omarchy;
mod ordered_json;

pub use custom::{CustomTheme, CustomThemeError};
pub use omarchy::{OmarchyPalette, OmarchyPaths, from_omarchy};

use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::ordered_json::OrderedJson;

/// **[COMPAT]** `CustomTheme::COLOR_KEYS` / `custom_theme.js COLOR_KEYS`, in
/// the original order (the `--theme-<key>` CSS variables of every theme).
pub const COLOR_KEYS: [&str; 38] = [
    "bg-primary",
    "bg-secondary",
    "bg-tertiary",
    "bg-hover",
    "text-primary",
    "text-secondary",
    "text-muted",
    "text-faint",
    "border",
    "accent",
    "accent-hover",
    "accent-text",
    "selection",
    "selection-text",
    "scrollbar",
    "scrollbar-hover",
    "success",
    "warning",
    "error",
    "code-bg",
    "heading-1",
    "heading-2",
    "heading-3",
    "folder-icon",
    "file-icon",
    "config-icon",
    "syntax-keyword",
    "syntax-function",
    "syntax-type",
    "syntax-string",
    "syntax-number",
    "syntax-constant",
    "syntax-identifier",
    "syntax-property",
    "syntax-operator",
    "syntax-comment",
    "syntax-punctuation",
    "syntax-invalid",
];

/// **[COMPAT]** `CustomTheme::BASE_THEMES`: ids of the 18 built-in themes
/// (also the valid `base` of a custom theme; `omarchy` and `custom` are not).
pub const BASE_THEMES: [&str; 18] = [
    "light",
    "dark",
    "catppuccin",
    "catppuccin-latte",
    "ethereal",
    "everforest",
    "flexoki-light",
    "gruvbox",
    "hackerman",
    "kanagawa",
    "matte-black",
    "nord",
    "osaka-jade",
    "ristretto",
    "rose-pine",
    "solarized-dark",
    "solarized-light",
    "tokyo-night",
];

/// The 12 syntax roles that must reach 4.5:1 against `code-bg`.
pub const SYNTAX_ROLES: [&str; 12] = [
    "keyword",
    "function",
    "type",
    "string",
    "number",
    "constant",
    "identifier",
    "property",
    "operator",
    "comment",
    "punctuation",
    "invalid",
];

/// sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Parse strict `#rrggbb`, either case (theme editor accepts nothing else, `09e5447`).
    pub fn parse_hex(s: &str) -> Option<Self> {
        let hex = s.strip_prefix('#')?;
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Rgb(channel(0)?, channel(2)?, channel(4)?))
    }
    /// Lowercase `#rrggbb`.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
    /// Linear per-channel sRGB mix with rounding (Omarchy `mix_color`):
    /// `t = 0` → `self`, `t = 1` → `other`.
    pub fn mix(self, other: Rgb, t: f32) -> Rgb {
        // awk: int(start * (1 - amount) + end * amount + 0.5), amount clamped to 0..=1.
        let t = f64::from(t).clamp(0.0, 1.0);
        let channel = |a: u8, b: u8| {
            let v = f64::from(a) * (1.0 - t) + f64::from(b) * t + 0.5;
            // In range by construction (a convex mix of two bytes, + 0.5).
            v.floor().clamp(0.0, 255.0) as u8
        };
        Rgb(
            channel(self.0, other.0),
            channel(self.1, other.1),
            channel(self.2, other.2),
        )
    }

    /// The color as `[r, g, b]` floats in `0.0..=255.0`.
    fn channels(self) -> [f64; 3] {
        [f64::from(self.0), f64::from(self.1), f64::from(self.2)]
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
        self.colors.get(key).copied()
    }

    /// Overlay a custom theme's colors on top of these tokens (its `base`
    /// is resolved by the caller via [`builtin`]).
    pub fn with_overrides(&self, custom: &CustomTheme) -> UiTokens {
        let mut out = self.clone();
        for (key, hex) in &custom.colors {
            if let (Some(key), Some(color)) = (color_key(key), Rgb::parse_hex(hex)) {
                out.colors.insert(key, color);
            }
        }
        out
    }
}

/// FrankMD's 18 built-in themes (38 `--theme-*` roles each), embedded from
/// `assets/themes/builtin.json` (extracted unchanged from FrankMD).
pub const BUILTIN_THEMES_JSON: &str = include_str!("../../../assets/themes/builtin.json");

/// Ids of the built-in themes, in `builtin.json` order.
pub fn builtin_ids() -> Vec<&'static str> {
    BUILTINS.iter().map(|(id, _)| id.as_str()).collect()
}

/// Built-in theme by **[COMPAT]** id (`dark`, `tokyo-night`, ...). `omarchy`
/// and `custom` are not built-ins (they are resolved by [`omarchy`] / [`custom`]).
/// Mode comes from the luminance of `bg-primary`.
pub fn builtin(id: &str) -> Option<UiTokens> {
    BUILTINS
        .iter()
        .find(|(theme_id, _)| theme_id == id)
        .map(|(_, tokens)| tokens.clone())
}

/// The [`COLOR_KEYS`] entry equal to `key` (gives it a `'static` lifetime).
pub fn color_key(key: &str) -> Option<&'static str> {
    COLOR_KEYS.iter().copied().find(|k| *k == key)
}

/// `builtin.json` parsed once, in file order.
static BUILTINS: LazyLock<Vec<(String, UiTokens)>> = LazyLock::new(parse_builtins);

/// Parse [`BUILTIN_THEMES_JSON`]. The file is embedded and covered by tests,
/// so a malformed entry is skipped rather than aborting the app.
fn parse_builtins() -> Vec<(String, UiTokens)> {
    let Ok(doc) = serde_json::from_str::<OrderedJson>(BUILTIN_THEMES_JSON) else {
        return Vec::new();
    };
    let Some(OrderedJson::Object(themes)) = doc.get("themes") else {
        return Vec::new();
    };
    themes
        .iter()
        .filter_map(|(id, theme)| {
            let Some(OrderedJson::Object(entries)) = theme.get("colors") else {
                return None;
            };
            let colors: BTreeMap<&'static str, Rgb> = entries
                .iter()
                .filter_map(|(k, v)| Some((color_key(k)?, Rgb::parse_hex(v.as_str()?)?)))
                .collect();
            let bg = entries
                .iter()
                .find(|(k, _)| k == "bg-primary")
                .and_then(|(_, v)| v.as_str());
            let mode = if legacy::dark_background(bg) {
                Mode::Dark
            } else {
                Mode::Light
            };
            Some((id.clone(), UiTokens { mode, colors }))
        })
        .collect()
}

/// WCAG relative luminance (sRGB linearized).
pub fn relative_luminance(c: Rgb) -> f64 {
    let [r, g, b] = c.channels().map(|v| {
        let v = v / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG contrast ratio `(Lmax + 0.05) / (Lmin + 0.05)`.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// **[COMPAT]** `ensure_contrast` binary search (omarchy_theme_service.rb):
/// returns `fg` unchanged when it already reaches `ratio`, otherwise blends it
/// toward black or white (whichever contrasts better with `bg`).
pub fn ensure_contrast(fg: Rgb, bg: Rgb, ratio: f32) -> Rgb {
    let ratio = f64::from(ratio);
    if contrast_ratio(fg, bg) >= ratio {
        return fg;
    }
    let white = Rgb(255, 255, 255);
    let black = Rgb(0, 0, 0);
    let target = if contrast_ratio(white, bg) > contrast_ratio(black, bg) {
        white
    } else {
        black
    };
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..24 {
        let amount = (low + high) / 2.0;
        if contrast_ratio(fg.mix(target, amount), bg) >= ratio {
            high = amount;
        } else {
            low = amount;
        }
    }
    fg.mix(target, high)
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
