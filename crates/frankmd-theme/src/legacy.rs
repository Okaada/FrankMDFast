//! **[COMPAT]** Faithful port of FrankMD's `app/services/omarchy_theme_service.rb`
//! (backend.md §11.1). It reads `<current>/theme.name` and
//! `<current>/theme/alacritty.toml`, where FrankMD hard-codes
//! `current = ~/.config/omarchy/current` (the pre-4.0 location; see
//! [`crate::omarchy`] for the 4.5 path). All math is the original float math
//! so the `omarchy_theme.json` golden cases match to the byte.
//!
//! Hex strings here are the original's loose strings (`"abcdef"` without `#`
//! is accepted, invalid digits parse as 0), not [`crate::Rgb`].

use crate::ThemeError;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `OmarchyThemeService::OMARCHY_DIR` relative to `$HOME`.
pub const LEGACY_CURRENT_DIR: &str = ".config/omarchy/current";
/// `THEME_NAME_FILE` relative to the current dir.
pub const THEME_NAME_FILE: &str = "theme.name";
/// `ALACRITTY_FILE` relative to the current dir.
pub const ALACRITTY_FILE: &str = "theme/alacritty.toml";

/// `theme_data` result (`GET /config/omarchy_theme`).
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeData {
    /// `theme.name` contents, stripped.
    pub theme_name: String,
    /// `--theme-<key>` → hex, in the original 38-entry order.
    pub variables: Vec<(String, String)>,
    /// `dark_background?(colors["background"])`.
    pub is_dark: bool,
}

/// `<home>/.config/omarchy/current`.
pub fn legacy_current_dir(home: &Path) -> PathBuf {
    todo!()
}

/// **[COMPAT]** `available?`: both files exist (following symlinks).
pub fn available(current_dir: &Path) -> bool {
    todo!()
}

/// **[COMPAT]** `theme_data`: `Ok(None)` when not [`available`] or when a
/// read fails with ENOENT/EACCES (rescued in the original); `Err` for other
/// read failures (EISDIR) and invalid UTF-8 in `alacritty.toml`.
pub fn theme_data(current_dir: &Path) -> Result<Option<ThemeData>, ThemeError> {
    todo!()
}

/// Pure core of [`theme_data`] given both file contents.
pub fn theme_data_from(theme_name_raw: &str, alacritty_toml: &str) -> ThemeData {
    todo!()
}

/// **[COMPAT]** `parse_alacritty`: line-based TOML subset → `primary` keys
/// as-is, others prefixed (`normal_red`, `bright_blue`, `cursor_text`,
/// `selection_background`). Only `#rrggbb` values.
pub fn parse_alacritty(content: &str) -> BTreeMap<String, String> {
    todo!()
}

/// **[COMPAT]** `map_variables`: the 38 `--theme-*` variables from parsed
/// alacritty colors (defaults for anything missing), in original order.
pub fn map_variables(colors: &BTreeMap<String, String>) -> Vec<(String, String)> {
    todo!()
}

/// **[COMPAT]** `hex_to_rgb`. `Err` unless the string (minus `#`) has exactly
/// six characters; non-hex digits parse as 0 (Ruby `to_i(16)`).
pub fn hex_to_rgb(hex: &str) -> Result<[u8; 3], ThemeError> {
    todo!()
}

/// **[COMPAT]** `rgb_to_hex`: clamp 0..=255, round half away from zero, lowercase.
pub fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    todo!()
}

/// **[COMPAT]** `lighten(hex, percent)`: `+ 255 * percent / 100` per channel.
pub fn lighten(hex: &str, percent: f64) -> String {
    todo!()
}

/// **[COMPAT]** `darken(hex, percent)`.
pub fn darken(hex: &str, percent: f64) -> String {
    todo!()
}

/// **[COMPAT]** `blend(fg, bg, factor)` = `fg + (bg - fg) * factor`.
pub fn blend(fg: &str, bg: &str, factor: f64) -> String {
    todo!()
}

/// **[COMPAT]** `dark_background?`: `None` → true; Rec.709 luma on
/// non-linearized channels `< 0.5`.
pub fn dark_background(hex: Option<&str>) -> bool {
    todo!()
}

/// **[COMPAT]** `relative_luminance` (WCAG, linearized).
pub fn relative_luminance(hex: &str) -> f64 {
    todo!()
}

/// **[COMPAT]** `contrast_ratio`.
pub fn contrast_ratio(fg: &str, bg: &str) -> f64 {
    todo!()
}

/// **[COMPAT]** `ensure_contrast(fg, bg)`: unchanged if ≥ 4.51, else 24
/// binary-search steps of `blend(fg, target, t)` toward black/white.
pub fn ensure_contrast(fg: &str, bg: &str) -> String {
    todo!()
}
