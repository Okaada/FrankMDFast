//! Omarchy 4.5 integration (docs/research/omarchy-4.5.md §2): the active
//! palette lives in `~/.local/state/omarchy/current/theme/colors.toml`
//! (FrankMD still reads the dead `~/.config/omarchy/current`, see
//! [`crate::legacy`]). The theme id stays `omarchy` (**[COMPAT]** `.fed`).

use crate::{Mode, Rgb, ThemeError, UiTokens, contrast_ratio, ensure_contrast};
use std::collections::{BTreeMap, HashMap};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// `~/.local/state/omarchy/current` relative to `$HOME`.
pub const CURRENT_DIR: &str = ".local/state/omarchy/current";

/// Where the active Omarchy theme lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmarchyPaths {
    /// `~/.local/state/omarchy/current` (watch this dir, not the files: the
    /// `theme/` dir is replaced by rename on every switch).
    pub current: PathBuf,
}

impl OmarchyPaths {
    /// `<home>/.local/state/omarchy/current`.
    pub fn from_home(home: &Path) -> Self {
        Self {
            current: home.join(CURRENT_DIR),
        }
    }
    /// `<current>/theme/colors.toml`.
    pub fn colors_toml(&self) -> PathBuf {
        self.current.join("theme").join("colors.toml")
    }
    /// `<current>/theme/light.mode` (presence = light, legacy signal).
    pub fn light_mode_file(&self) -> PathBuf {
        self.current.join("theme").join("light.mode")
    }
    /// `<current>/theme.name`.
    pub fn theme_name(&self) -> PathBuf {
        self.current.join("theme.name")
    }
    /// `<current>/theme.generation` (opaque token rewritten on every switch, 4.5).
    pub fn generation(&self) -> PathBuf {
        self.current.join("theme.generation")
    }
}

/// Omarchy palette after the alias cascade.
#[derive(Debug, Clone, PartialEq)]
pub struct OmarchyPalette {
    /// Resolved mode (`mode` → `theme_type` → `light.mode` → luminance `R+G+B > 382`).
    pub mode: Mode,
    /// Canonical + derived + alias keys (`background`, `accent`, `orange`,
    /// `bright_red`, `color0`..`color15`, short names, ...).
    pub colors: BTreeMap<String, Rgb>,
}

impl OmarchyPalette {
    /// Parse `colors.toml` text; `light_mode_file` = presence of `light.mode`.
    /// Applies the full `resolve_theme_colors` cascade (§2.2).
    ///
    /// The file must be valid TOML (a half-written file is an error, not a
    /// default palette). Only top-level string values take part; values that
    /// are not `#rrggbb` (gradients, `rgba(...)`) are dropped from
    /// [`OmarchyPalette::colors`] after the cascade.
    pub fn parse(colors_toml: &str, light_mode_file: bool) -> Result<Self, ThemeError> {
        let table: toml::Table = colors_toml
            .parse()
            .map_err(|e: toml::de::Error| ThemeError::Invalid(e.message().to_owned()))?;
        let mut raw = Raw::default();
        for (key, value) in &table {
            if let toml::Value::String(s) = value {
                raw.set(key, s.trim().to_owned());
            }
        }
        resolve_theme_colors(&mut raw);
        let mode = resolve_theme_mode(&raw, light_mode_file);
        let colors = raw
            .0
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), Rgb::parse_hex(v)?)))
            .collect();
        Ok(Self { mode, colors })
    }

    /// One resolved color.
    pub fn get(&self, key: &str) -> Option<Rgb> {
        self.colors.get(key).copied()
    }
}

// --- the omarchy-theme-color cascade -----------------------------------------

/// The string table the bash resolver works on. Empty means unset
/// (`[[ ${THEME_COLORS[k]} ]]`).
#[derive(Debug, Default)]
struct Raw(HashMap<String, String>);

impl Raw {
    fn get(&self, key: &str) -> Option<&str> {
        self.0
            .get(key)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }

    /// `[[ ${k} ]] || k=value` (no-op when `value` is unset too).
    fn default_to(&mut self, key: &str, value: Option<String>) {
        if self.get(key).is_none()
            && let Some(v) = value.filter(|v| !v.is_empty())
        {
            self.set(key, v);
        }
    }

    /// `alias_theme_color key fallback`.
    fn alias(&mut self, key: &str, fallback: &str) {
        let value = self.get(fallback).map(str::to_owned);
        self.default_to(key, value);
    }

    /// First set key among `keys`.
    fn first(&self, keys: &[&str]) -> Option<String> {
        keys.iter().find_map(|k| self.get(k)).map(str::to_owned)
    }

    /// `mix_color "${k}" other amount`; unset when `key` is not a hex color.
    fn mixed(&self, key: &str, other: Rgb, amount: f32) -> Option<String> {
        let base = Rgb::parse_hex(self.get(key)?)?;
        Some(base.mix(other, amount).to_hex())
    }
}

/// Canonical name → legacy short name.
const SHORT_NAMES: [(&str, &str); 8] = [
    ("background", "bg"),
    ("dark_background", "dark_bg"),
    ("darker_background", "darker_bg"),
    ("lighter_background", "lighter_bg"),
    ("foreground", "fg"),
    ("dark_foreground", "dark_fg"),
    ("light_foreground", "light_fg"),
    ("bright_foreground", "bright_fg"),
];

/// Semantic name ← ANSI slot (`legacy_alias`).
const ANSI_FALLBACKS: [(&str, &str); 12] = [
    ("red", "color1"),
    ("green", "color2"),
    ("yellow", "color3"),
    ("blue", "color4"),
    ("magenta", "color5"),
    ("cyan", "color6"),
    ("bright_red", "color9"),
    ("bright_green", "color10"),
    ("bright_yellow", "color11"),
    ("bright_blue", "color12"),
    ("bright_magenta", "color13"),
    ("bright_cyan", "color14"),
];

/// ANSI slot ← semantic name (`ansi_alias`), exposed for old consumers.
const ANSI_ALIASES: [(&str, &str); 16] = [
    ("color0", "background"),
    ("color1", "red"),
    ("color2", "green"),
    ("color3", "yellow"),
    ("color4", "blue"),
    ("color5", "magenta"),
    ("color6", "cyan"),
    ("color7", "foreground"),
    ("color8", "muted"),
    ("color9", "bright_red"),
    ("color10", "bright_green"),
    ("color11", "bright_yellow"),
    ("color12", "bright_blue"),
    ("color13", "bright_magenta"),
    ("color14", "bright_cyan"),
    ("color15", "bright_foreground"),
];

const BLACK: Rgb = Rgb(0, 0, 0);
const WHITE: Rgb = Rgb(255, 255, 255);

/// Port of `resolve_theme_colors` in `bin/omarchy-theme-color`, step by step.
fn resolve_theme_colors(c: &mut Raw) {
    for (canonical, short) in SHORT_NAMES {
        c.alias(canonical, short);
    }

    c.alias("background", "color0");
    c.alias("foreground", "color7");
    if let Some(bg) = c.get("background").map(str::to_owned) {
        c.set("color0", bg);
    }
    if let Some(fg) = c.get("foreground").map(str::to_owned) {
        c.set("color7", fg);
    }

    for (name, slot) in ANSI_FALLBACKS {
        c.alias(name, slot);
    }
    c.alias("magenta", "purple");
    c.alias("bright_magenta", "bright_purple");

    let v = c.first(&["color7", "foreground"]);
    c.default_to("light_foreground", v);
    let v = c.first(&["color15", "foreground"]);
    c.default_to("bright_foreground", v);
    let cursor = c
        .get("bright_foreground")
        .map(str::to_owned)
        .unwrap_or_default();
    c.set("cursor", cursor);
    let v = c.first(&["color0", "background"]);
    c.default_to("lighter_background", v);
    let v = c.first(&["color8", "foreground"]);
    c.default_to("dark_foreground", v);
    let v = c.first(&["color8", "dark_foreground"]);
    c.default_to("muted", v);
    let v = c.first(&["selection_background", "color8", "color0", "background"]);
    c.default_to("selection", v);
    c.alias("selection_background", "selection");
    c.alias("selection_foreground", "bright_foreground");
    c.alias("orange", "yellow");
    let v = c.mixed("orange", BLACK, 0.5);
    c.default_to("brown", v);

    let v = c.mixed("background", BLACK, 0.25);
    c.default_to("dark_background", v);
    let v = c.mixed("background", BLACK, 0.5);
    c.default_to("darker_background", v);
    for name in ["red", "yellow", "green", "cyan", "blue", "magenta"] {
        let v = c.mixed(name, WHITE, 0.2);
        c.default_to(&format!("bright_{name}"), v);
    }
    c.alias("purple", "magenta");
    c.alias("bright_purple", "bright_magenta");

    for (slot, name) in ANSI_ALIASES {
        c.alias(slot, name);
    }
    for (canonical, short) in SHORT_NAMES {
        if let Some(v) = c.get(canonical).map(str::to_owned) {
            c.set(short, v);
        }
    }
}

/// Port of `resolve_theme_mode`.
fn resolve_theme_mode(c: &Raw, light_mode_file: bool) -> Mode {
    if let Some(mode) = c.first(&["mode", "theme_type"]) {
        return if mode == "light" {
            Mode::Light
        } else {
            Mode::Dark
        };
    }
    if light_mode_file {
        return Mode::Light;
    }
    match c.get("background").and_then(Rgb::parse_hex) {
        Some(Rgb(r, g, b)) if u32::from(r) + u32::from(g) + u32::from(b) > 382 => Mode::Light,
        _ => Mode::Dark,
    }
}

// --- palette → UI tokens -------------------------------------------------------

/// Contrast every text/syntax token must reach against `code-bg` (the
/// original's `ensure_contrast` target).
const TEXT_CONTRAST: f32 = 4.51;

/// Map an Omarchy palette to UI tokens (§2.6 table), contrast-correcting
/// syntax/text tokens to ≥ 4.5:1 against `code-bg` (OM-004).
///
/// A palette key that is missing even after the cascade falls back to the
/// matching token of the built-in `dark`/`light` theme (by mode), so the
/// result always has all 38 tokens. `text-faint` is deliberately left
/// uncorrected (it is the "barely there" tone for line numbers/placeholders).
pub fn from_omarchy(p: &OmarchyPalette) -> UiTokens {
    let base_id = if p.mode == Mode::Light {
        "light"
    } else {
        "dark"
    };
    let base = crate::builtin(base_id).unwrap_or(UiTokens {
        mode: p.mode,
        colors: BTreeMap::new(),
    });
    let fallback = |token: &str| {
        base.get(token)
            .unwrap_or(if p.mode == Mode::Light { BLACK } else { WHITE })
    };
    let get = |key: &str, token: &str| p.get(key).unwrap_or_else(|| fallback(token));

    let background = get("background", "bg-primary");
    let foreground = get("foreground", "text-primary");
    let dark_foreground = get("dark_foreground", "text-muted");
    let muted = p.get("muted").unwrap_or(dark_foreground);
    let accent = p
        .get("accent")
        .or_else(|| p.get("blue"))
        .unwrap_or_else(|| fallback("accent"));
    let selection = get("selection", "selection");
    let code_bg = get("dark_background", "code-bg");
    let readable = |c: Rgb| ensure_contrast(c, code_bg, TEXT_CONTRAST);

    let accent_text = if contrast_ratio(background, accent) >= 4.5 {
        background
    } else if contrast_ratio(WHITE, accent) > contrast_ratio(BLACK, accent) {
        WHITE
    } else {
        BLACK
    };

    let tokens: [(&'static str, Rgb); 38] = [
        ("bg-primary", background),
        ("bg-secondary", code_bg),
        ("bg-tertiary", get("lighter_background", "bg-tertiary")),
        ("bg-hover", background.mix(foreground, 0.08)),
        ("text-primary", readable(foreground)),
        ("text-secondary", readable(foreground.mix(background, 0.08))),
        ("text-muted", readable(dark_foreground)),
        ("text-faint", dark_foreground.mix(background, 0.35)),
        ("border", muted),
        ("accent", accent),
        ("accent-hover", accent.mix(foreground, 0.15)),
        ("accent-text", accent_text),
        ("selection", selection),
        (
            "selection-text",
            ensure_contrast(
                get("selection_foreground", "selection-text"),
                selection,
                TEXT_CONTRAST,
            ),
        ),
        ("scrollbar", muted),
        ("scrollbar-hover", dark_foreground),
        ("success", get("green", "success")),
        ("warning", get("yellow", "warning")),
        ("error", get("red", "error")),
        ("code-bg", code_bg),
        ("heading-1", get("magenta", "heading-1")),
        ("heading-2", get("blue", "heading-2")),
        ("heading-3", get("cyan", "heading-3")),
        ("folder-icon", accent),
        ("file-icon", dark_foreground),
        ("config-icon", get("yellow", "config-icon")),
        ("syntax-keyword", readable(get("magenta", "syntax-keyword"))),
        (
            "syntax-function",
            readable(get("yellow", "syntax-function")),
        ),
        ("syntax-type", readable(get("cyan", "syntax-type"))),
        ("syntax-string", readable(get("green", "syntax-string"))),
        ("syntax-number", readable(get("orange", "syntax-number"))),
        (
            "syntax-constant",
            readable(get("bright_cyan", "syntax-constant")),
        ),
        ("syntax-identifier", readable(foreground)),
        (
            "syntax-property",
            readable(get("bright_blue", "syntax-property")),
        ),
        ("syntax-operator", readable(foreground)),
        ("syntax-comment", readable(muted)),
        ("syntax-punctuation", readable(dark_foreground)),
        ("syntax-invalid", readable(get("red", "syntax-invalid"))),
    ];
    UiTokens {
        mode: p.mode,
        colors: tokens.into_iter().collect(),
    }
}

/// Read the active theme. `Ok(None)` when Omarchy is not installed (the
/// `current` dir does not exist). `Err` when `current` exists but
/// `theme/colors.toml` is missing (mid-swap: `rm -rf theme && mv next-theme
/// theme`), unreadable or unparsable — the caller keeps the last good theme
/// and retries on the next watcher event (HP-024). Returns `(theme name,
/// palette)`; the name is `theme.name` stripped (empty if absent).
pub fn load_current(paths: &OmarchyPaths) -> Result<Option<(String, OmarchyPalette)>, ThemeError> {
    if !paths.current.is_dir() {
        return Ok(None);
    }
    let bytes = match std::fs::read(paths.colors_toml()) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(ThemeError::Invalid(
                "theme/colors.toml is missing (theme switch in progress?)".to_owned(),
            ));
        }
        Err(e) => return Err(ThemeError::Io(e)),
    };
    let text = String::from_utf8(bytes).map_err(|_| ThemeError::Encoding)?;
    let light_mode = paths.light_mode_file().exists();
    let palette = OmarchyPalette::parse(&text, light_mode)?;
    let name = std::fs::read(paths.theme_name())
        .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
        .unwrap_or_default();
    Ok(Some((name, palette)))
}
