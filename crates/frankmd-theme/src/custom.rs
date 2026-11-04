//! **[COMPAT]** User custom themes (`app/services/custom_theme.rb` +
//! `app/javascript/lib/custom_theme.js`, backend.md §11.2).
//!
//! Stored in `.fed` as one compact JSON line
//! `custom_theme = {"version":1,"name":"…","base":"…","colors":{…}}`.

use crate::ordered_json::OrderedJson;
use crate::{BASE_THEMES, COLOR_KEYS, Rgb};

/// Maximum raw JSON size in bytes (`CustomTheme::MAX_BYTES`).
pub const MAX_BYTES: usize = 16 * 1024;
/// The only supported `version`.
pub const VERSION: u32 = 1;
/// Maximum name length in characters, after strip.
pub const MAX_NAME_CHARS: usize = 60;

/// A validated, normalized custom theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomTheme {
    /// Always [`VERSION`].
    pub version: u32,
    /// Stripped name (1..=60 chars, no control characters).
    pub name: String,
    /// One of [`crate::BASE_THEMES`].
    pub base: String,
    /// Overrides in **input order**, keys ⊆ [`crate::COLOR_KEYS`], values
    /// lowercase `#rrggbb`.
    pub colors: Vec<(String, String)>,
}

/// Why a custom theme was rejected. `Display` is the original Ruby
/// `ArgumentError` message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CustomThemeError {
    /// Raw string larger than [`MAX_BYTES`].
    #[error("Theme is too large")]
    TooLarge,
    /// Not valid JSON (`JSON::ParserError`; message is ours, not Ruby's).
    #[error("invalid JSON: {0}")]
    Json(String),
    /// Not an object, unknown top-level key, or `version != 1`.
    #[error("Invalid theme format")]
    Format,
    /// Name missing, not a string, blank, too long, or has control chars.
    #[error("Invalid theme name")]
    Name,
    /// `base` not one of [`crate::BASE_THEMES`].
    #[error("Invalid base theme")]
    Base,
    /// `colors` not an object, unknown key, or a value that is not `#rrggbb`.
    #[error("Invalid theme colors")]
    Colors,
}

impl CustomTheme {
    /// **[COMPAT]** `CustomTheme.parse(string)`: size check, JSON parse,
    /// then [`CustomTheme::from_value`]. JSON `null` → `Ok(None)`.
    pub fn parse(raw_json: &str) -> Result<Option<Self>, CustomThemeError> {
        if raw_json.len() > MAX_BYTES {
            return Err(CustomThemeError::TooLarge);
        }
        let value: OrderedJson =
            serde_json::from_str(raw_json).map_err(|e| CustomThemeError::Json(e.to_string()))?;
        Self::validate(&value)
    }

    /// **[COMPAT]** `CustomTheme.parse(hash)`: validate an already-decoded
    /// value (not size-checked). Check order: format, name, base, colors.
    /// `null` → `Ok(None)`.
    pub fn from_value(value: &serde_json::Value) -> Result<Option<Self>, CustomThemeError> {
        Self::validate(&OrderedJson::from_value(value))
    }

    fn validate(value: &OrderedJson) -> Result<Option<Self>, CustomThemeError> {
        let entries = match value {
            OrderedJson::Null => return Ok(None),
            OrderedJson::Object(entries) => entries,
            _ => return Err(CustomThemeError::Format),
        };
        let known_keys = entries
            .iter()
            .all(|(k, _)| matches!(k.as_str(), "version" | "name" | "base" | "colors"));
        // Ruby `value["version"] == VERSION` is also true for `1.0`.
        let version_ok = matches!(value.get("version"), Some(OrderedJson::Number(n)) if n.as_f64() == Some(f64::from(VERSION)));
        if !known_keys || !version_ok {
            return Err(CustomThemeError::Format);
        }

        let name = match value.get("name") {
            Some(OrderedJson::String(name)) => name,
            _ => return Err(CustomThemeError::Name),
        };
        let stripped = ruby_strip(name);
        let length = stripped.chars().count();
        if !(1..=MAX_NAME_CHARS).contains(&length) || name.chars().any(char::is_control) {
            return Err(CustomThemeError::Name);
        }

        let base = match value.get("base") {
            Some(OrderedJson::String(base)) if BASE_THEMES.contains(&base.as_str()) => base,
            _ => return Err(CustomThemeError::Base),
        };

        let Some(OrderedJson::Object(color_entries)) = value.get("colors") else {
            return Err(CustomThemeError::Colors);
        };
        let colors = color_entries
            .iter()
            .map(|(key, color)| match color {
                OrderedJson::String(hex)
                    if COLOR_KEYS.contains(&key.as_str()) && Rgb::parse_hex(hex).is_some() =>
                {
                    Ok((key.clone(), hex.to_ascii_lowercase()))
                }
                _ => Err(CustomThemeError::Colors),
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(CustomTheme {
            version: VERSION,
            name: stripped.to_owned(),
            base: base.clone(),
            colors,
        }))
    }

    /// Compact JSON exactly as Ruby `JSON.generate` writes it into `.fed`
    /// (key order `version, name, base, colors`; colors in input order).
    pub fn to_json_string(&self) -> String {
        let quote = |s: &str| serde_json::Value::String(s.to_owned()).to_string();
        let colors: Vec<String> = self
            .colors
            .iter()
            .map(|(k, v)| format!("{}:{}", quote(k), quote(v)))
            .collect();
        format!(
            "{{\"version\":{},\"name\":{},\"base\":{},\"colors\":{{{}}}}}",
            self.version,
            quote(&self.name),
            quote(&self.base),
            colors.join(",")
        )
    }

    /// The same as a `serde_json::Value`.
    pub fn to_json(&self) -> serde_json::Value {
        let colors: serde_json::Map<String, serde_json::Value> = self
            .colors
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        serde_json::json!({ "version": self.version, "name": self.name, "base": self.base, "colors": colors })
    }
}

/// Ruby `String#strip`: ASCII whitespace and NUL (not NBSP or other Unicode spaces).
fn ruby_strip(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\0'))
}

/// **[COMPAT]** JS `normalizeColor`: trim + lowercase, then strict `#rrggbb`
/// (no 3-digit shorthand, no names, no CSS expressions).
pub fn normalize_color(value: &str) -> Option<String> {
    let color = js_trim(value).to_lowercase();
    Rgb::parse_hex(&color).map(|_| color)
}

/// JS `String.prototype.trim` (Unicode whitespace and line terminators).
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// **[COMPAT]** JS `themeColor(styles, key)` minus the browser-only named
/// color path: trim, expand a `#rgb` built-in (CSS-minified) value to
/// `#rrggbb`, then [`normalize_color`].
pub fn theme_color(raw_css_value: &str) -> Option<String> {
    let value = js_trim(raw_css_value);
    let short = value
        .strip_prefix('#')
        .filter(|h| h.len() == 3 && h.bytes().all(|b| b.is_ascii_hexdigit()));
    match short {
        Some(hex) => normalize_color(&format!(
            "#{}",
            hex.chars().flat_map(|c| [c, c]).collect::<String>()
        )),
        None => normalize_color(value),
    }
}

/// **[COMPAT]** JS `contrastRatio(a, b)`: `None` unless both normalize.
/// (The JS version yields `NaN` for padded inputs such as `" #808080 "`
/// because it slices the raw string; we use the normalized value.)
pub fn contrast_ratio_hex(a: &str, b: &str) -> Option<f64> {
    let a = Rgb::parse_hex(&normalize_color(a)?)?;
    let b = Rgb::parse_hex(&normalize_color(b)?)?;
    Some(crate::contrast_ratio(a, b))
}

/// **[COMPAT]** JS `applyCustomColors(colors)` filter: the `(key, color)`
/// pairs that would be applied, in input order — only [`crate::COLOR_KEYS`]
/// whose value is a string that normalizes. `null`/non-object → empty.
pub fn applicable_overrides(colors: &serde_json::Value) -> Vec<(&'static str, String)> {
    let Some(map) = colors.as_object() else {
        return Vec::new();
    };
    map.iter()
        .filter_map(|(key, value)| {
            Some((crate::color_key(key)?, normalize_color(value.as_str()?)?))
        })
        .collect()
}
