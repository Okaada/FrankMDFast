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
use std::io::ErrorKind;
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
    home.join(LEGACY_CURRENT_DIR)
}

/// **[COMPAT]** `available?`: both files exist (following symlinks).
pub fn available(current_dir: &Path) -> bool {
    current_dir.join(THEME_NAME_FILE).exists() && current_dir.join(ALACRITTY_FILE).exists()
}

/// **[COMPAT]** `theme_data`: `Ok(None)` when not [`available`] or when a
/// read fails with ENOENT/EACCES (rescued in the original); `Err` for other
/// read failures (EISDIR) and invalid UTF-8 in `alacritty.toml`.
pub fn theme_data(current_dir: &Path) -> Result<Option<ThemeData>, ThemeError> {
    if !available(current_dir) {
        return Ok(None);
    }
    // Ruby raises on the first `strip` of a string with an invalid byte, so
    // any invalid UTF-8 is that error (the name is stripped before the
    // alacritty file is read).
    let Some(name) = read_rescued(&current_dir.join(THEME_NAME_FILE))? else {
        return Ok(None);
    };
    let name = String::from_utf8(name).map_err(|_| ThemeError::Encoding)?;
    let Some(alacritty) = read_rescued(&current_dir.join(ALACRITTY_FILE))? else {
        return Ok(None);
    };
    let alacritty = String::from_utf8(alacritty).map_err(|_| ThemeError::Encoding)?;
    Ok(Some(theme_data_from(&name, &alacritty)))
}

/// `File.read` with the original's `rescue Errno::ENOENT, Errno::EACCES` → `None`.
fn read_rescued(path: &Path) -> Result<Option<Vec<u8>>, ThemeError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::PermissionDenied) => Ok(None),
        Err(e) => Err(ThemeError::Io(e)),
    }
}

/// Pure core of [`theme_data`] given both file contents.
pub fn theme_data_from(theme_name_raw: &str, alacritty_toml: &str) -> ThemeData {
    let colors = parse_alacritty(alacritty_toml);
    ThemeData {
        theme_name: ruby_strip(theme_name_raw).to_owned(),
        variables: map_variables(&colors),
        is_dark: dark_background(colors.get("background").map(String::as_str)),
    }
}

/// Ruby `String#strip`: ASCII whitespace and NUL.
fn ruby_strip(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\0'))
}

/// Ruby regex `\s`.
fn is_ruby_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Ruby regex `\w` (ASCII).
fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// `\A\[colors\.(\w+)\]\z`
fn section_name(line: &str) -> Option<&str> {
    let name = line.strip_prefix("[colors.")?.strip_suffix(']')?;
    (!name.is_empty() && name.bytes().all(is_word)).then_some(name)
}

/// `\A(\w+)\s*=\s*["']?(#[0-9a-fA-F]{6})["']?\s*\z` → `(key, hex)`.
fn color_line(line: &str) -> Option<(&str, &str)> {
    let b = line.as_bytes();
    let key_end = b.iter().position(|&c| !is_word(c)).unwrap_or(b.len());
    if key_end == 0 {
        return None;
    }
    let skip_spaces = |mut i: usize| {
        while i < b.len() && is_ruby_space(b[i]) {
            i += 1;
        }
        i
    };
    let mut i = skip_spaces(key_end);
    if b.get(i) != Some(&b'=') {
        return None;
    }
    i = skip_spaces(i + 1);
    if matches!(b.get(i), Some(b'"' | b'\'')) {
        i += 1;
    }
    let hex_start = i;
    let hex_ok = b.get(i) == Some(&b'#')
        && b.len() >= i + 7
        && b[i + 1..i + 7].iter().all(u8::is_ascii_hexdigit);
    if !hex_ok {
        return None;
    }
    i += 7;
    if matches!(b.get(i), Some(b'"' | b'\'')) {
        i += 1;
    }
    (skip_spaces(i) == b.len()).then(|| (&line[..key_end], &line[hex_start..hex_start + 7]))
}

/// **[COMPAT]** `parse_alacritty`: line-based TOML subset → `primary` keys
/// as-is, others prefixed (`normal_red`, `bright_blue`, `cursor_text`,
/// `selection_background`). Only `#rrggbb` values.
pub fn parse_alacritty(content: &str) -> BTreeMap<String, String> {
    let mut colors = BTreeMap::new();
    let mut section: Option<&str> = None;
    // `each_line` splits on "\n" only (a lone "\r" does not end a line).
    for raw in content.split_inclusive('\n') {
        let line = ruby_strip(raw);
        if line.starts_with('[') {
            section = section_name(line);
            continue;
        }
        let Some(current) = section else { continue };
        let Some((key, hex)) = color_line(line) else {
            continue;
        };
        let name = match current {
            "primary" => key.to_owned(),
            "cursor" | "selection" | "normal" | "bright" => format!("{current}_{key}"),
            _ => continue,
        };
        colors.insert(name, hex.to_owned());
    }
    colors
}

/// **[COMPAT]** `map_variables`: the 38 `--theme-*` variables from parsed
/// alacritty colors (defaults for anything missing), in original order.
pub fn map_variables(colors: &BTreeMap<String, String>) -> Vec<(String, String)> {
    let c = |key: &str| colors.get(key).map(String::as_str);
    let first =
        |keys: &[&str], default: &str| keys.iter().find_map(|k| c(k)).unwrap_or(default).to_owned();

    let bg = c("background").unwrap_or("#121212");
    let fg = c("foreground").unwrap_or("#d0d0d0");
    let dim_fg = c("dim_foreground");
    let blue = c("normal_blue").unwrap_or("#7ea7c9");
    let bright_blue = c("bright_blue");
    let bright_black = c("normal_black").unwrap_or("#555555");
    let dark = dark_background(Some(bg));
    let shade = |percent: f64| {
        if dark {
            lighten(bg, percent)
        } else {
            darken(bg, percent)
        }
    };
    let code_bg = shade(7.0);
    let dim_or_blend = || dim_fg.map_or_else(|| blend(fg, bg, 0.45), str::to_owned);

    let syntax = [
        (
            "syntax-keyword",
            first(&["normal_magenta", "normal_red"], "#c792ea"),
        ),
        (
            "syntax-function",
            first(&["normal_yellow", "bright_yellow"], "#ffd866"),
        ),
        (
            "syntax-type",
            first(&["normal_cyan", "bright_cyan"], "#65d5cb"),
        ),
        (
            "syntax-string",
            first(&["normal_green", "bright_green"], "#95d96a"),
        ),
        (
            "syntax-number",
            first(&["bright_yellow", "normal_yellow"], "#ffad66"),
        ),
        (
            "syntax-constant",
            first(&["bright_cyan", "normal_cyan"], "#79c0ff"),
        ),
        ("syntax-identifier", fg.to_owned()),
        ("syntax-property", bright_blue.unwrap_or(blue).to_owned()),
        (
            "syntax-operator",
            c("normal_white").unwrap_or(fg).to_owned(),
        ),
        (
            "syntax-comment",
            dim_fg
                .or(c("normal_black"))
                .map_or_else(|| blend(fg, bg, 0.45), str::to_owned),
        ),
        (
            "syntax-punctuation",
            c("normal_white").unwrap_or(fg).to_owned(),
        ),
        ("syntax-invalid", first(&["normal_red"], "#ff8787")),
    ];

    let base = [
        ("bg-primary", bg.to_owned()),
        ("bg-secondary", shade(3.0)),
        ("bg-tertiary", shade(7.0)),
        ("bg-hover", shade(12.0)),
        ("text-primary", fg.to_owned()),
        ("text-secondary", blend(fg, bg, 0.08)),
        ("text-muted", dim_or_blend()),
        ("text-faint", blend(fg, bg, 0.65)),
        ("border", shade(12.0)),
        ("accent", blue.to_owned()),
        (
            "accent-hover",
            bright_blue.map_or_else(|| lighten(blue, 15.0), str::to_owned),
        ),
        ("accent-text", bg.to_owned()),
        (
            "selection",
            c("selection_background").map_or_else(|| blend(fg, bg, 0.25), str::to_owned),
        ),
        (
            "selection-text",
            c("selection_text").unwrap_or(fg).to_owned(),
        ),
        ("scrollbar", bright_black.to_owned()),
        ("scrollbar-hover", lighten(bright_black, 10.0)),
        ("success", first(&["normal_green"], "#8673d4")),
        ("warning", first(&["normal_yellow"], "#ca2edc")),
        ("error", first(&["normal_red"], "#ff4da6")),
        ("code-bg", code_bg.clone()),
        ("heading-1", first(&["normal_magenta"], "#b683c3")),
        ("heading-2", blue.to_owned()),
        ("heading-3", first(&["normal_cyan"], "#8adb8a")),
        ("folder-icon", blue.to_owned()),
        ("file-icon", dim_or_blend()),
        ("config-icon", first(&["normal_yellow"], "#ca2edc")),
    ];

    base.into_iter()
        .map(|(k, v)| (format!("--theme-{k}"), v))
        .chain(
            syntax
                .into_iter()
                .map(|(k, v)| (format!("--theme-{k}"), ensure_contrast(&v, &code_bg))),
        )
        .collect()
}

/// Ruby `String#to_i(16)` on a short substring: optional leading
/// whitespace and sign, then hex digits (underscores between digits allowed);
/// anything else stops the scan, and no digits is 0.
fn ruby_hex_to_i(s: &str) -> i64 {
    let t = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let (negative, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let mut value: i64 = 0;
    let mut prev_digit = false;
    for c in digits.chars() {
        if let Some(d) = c.to_digit(16) {
            value = value * 16 + i64::from(d);
            prev_digit = true;
        } else if c == '_' && prev_digit {
            prev_digit = false;
        } else {
            break;
        }
    }
    if negative { -value } else { value }
}

/// Ruby `str[a..b]` on characters: `None` when `a` is past the end
/// (`"abc"[4..5]` is nil, `"abcd"[4..5]` is `""`).
fn ruby_slice(chars: &[char], start: usize, end_inclusive: usize) -> Option<String> {
    if start > chars.len() {
        return None;
    }
    let end = (end_inclusive + 1).min(chars.len());
    Some(chars[start..end].iter().collect())
}

/// The channels as Ruby computes them (`[r, g, b]`, possibly out of 0..=255).
fn channels(hex: &str) -> Result<[i64; 3], ThemeError> {
    let chars: Vec<char> = hex.strip_prefix('#').unwrap_or(hex).chars().collect();
    let pair = |start: usize| {
        ruby_slice(&chars, start, start + 1)
            .map(|s| ruby_hex_to_i(&s))
            // `nil.to_i(16)`: NilClass#to_i takes no arguments.
            .ok_or_else(|| {
                ThemeError::Invalid("wrong number of arguments (given 1, expected 0)".to_owned())
            })
    };
    Ok([pair(0)?, pair(2)?, pair(4)?])
}

/// Channels as floats; a malformed hex (never produced by
/// [`parse_alacritty`] or the defaults) counts as black instead of raising.
fn float_channels(hex: &str) -> [f64; 3] {
    channels(hex).map_or([0.0; 3], |c| c.map(|v| v as f64))
}

/// **[COMPAT]** `hex_to_rgb`. `Err` when the string (minus `#`) is shorter
/// than four characters (Ruby calls `nil.to_i(16)`); non-hex digits parse as
/// 0 (Ruby `to_i(16)`).
pub fn hex_to_rgb(hex: &str) -> Result<[u8; 3], ThemeError> {
    // Two characters of `to_i(16)` are at most 0xff; a sign can push it below 0.
    Ok(channels(hex)?.map(|v| v.clamp(0, 255) as u8))
}

/// **[COMPAT]** `rgb_to_hex`: clamp 0..=255, round half away from zero, lowercase.
pub fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    let channel = |v: f64| v.clamp(0.0, 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", channel(r), channel(g), channel(b))
}

/// **[COMPAT]** `lighten(hex, percent)`: `+ 255 * percent / 100` per channel.
pub fn lighten(hex: &str, percent: f64) -> String {
    let [r, g, b] = float_channels(hex);
    let amount = 255.0 * percent / 100.0;
    rgb_to_hex(r + amount, g + amount, b + amount)
}

/// **[COMPAT]** `darken(hex, percent)`.
pub fn darken(hex: &str, percent: f64) -> String {
    let [r, g, b] = float_channels(hex);
    let amount = 255.0 * percent / 100.0;
    rgb_to_hex(r - amount, g - amount, b - amount)
}

/// **[COMPAT]** `blend(fg, bg, factor)` = `fg + (bg - fg) * factor`.
pub fn blend(fg: &str, bg: &str, factor: f64) -> String {
    let [fr, fg_, fb] = float_channels(fg);
    let [br, bg_, bb] = float_channels(bg);
    rgb_to_hex(
        fr + (br - fr) * factor,
        fg_ + (bg_ - fg_) * factor,
        fb + (bb - fb) * factor,
    )
}

/// **[COMPAT]** `dark_background?`: `None` → true; Rec.709 luma on
/// non-linearized channels `< 0.5`.
pub fn dark_background(hex: Option<&str>) -> bool {
    let Some(hex) = hex else { return true };
    let [r, g, b] = float_channels(hex);
    let luminance = 0.2126 * (r / 255.0) + 0.7152 * (g / 255.0) + 0.0722 * (b / 255.0);
    luminance < 0.5
}

/// **[COMPAT]** `relative_luminance` (WCAG, linearized).
pub fn relative_luminance(hex: &str) -> f64 {
    let [r, g, b] = float_channels(hex).map(|channel| {
        let value = channel / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// **[COMPAT]** `contrast_ratio`.
pub fn contrast_ratio(fg: &str, bg: &str) -> f64 {
    let (f, b) = (relative_luminance(fg), relative_luminance(bg));
    (f.max(b) + 0.05) / (f.min(b) + 0.05)
}

/// The original's contrast target.
const MIN_CONTRAST: f64 = 4.51;

/// **[COMPAT]** `ensure_contrast(fg, bg)`: unchanged if ≥ 4.51, else 24
/// binary-search steps of `blend(fg, target, t)` toward black/white.
pub fn ensure_contrast(fg: &str, bg: &str) -> String {
    if contrast_ratio(fg, bg) >= MIN_CONTRAST {
        return fg.to_owned();
    }
    let target = if contrast_ratio("#ffffff", bg) > contrast_ratio("#000000", bg) {
        "#ffffff"
    } else {
        "#000000"
    };
    let (mut low, mut high) = (0.0_f64, 1.0_f64);
    for _ in 0..24 {
        let amount = (low + high) / 2.0;
        if contrast_ratio(&blend(fg, target, amount), bg) >= MIN_CONTRAST {
            high = amount;
        } else {
            low = amount;
        }
    }
    blend(fg, target, high)
}
