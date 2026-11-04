//! **[COMPAT]** User custom themes (`app/services/custom_theme.rb` +
//! `app/javascript/lib/custom_theme.js`, backend.md §11.2).
//!
//! Stored in `.fed` as one compact JSON line
//! `custom_theme = {"version":1,"name":"…","base":"…","colors":{…}}`.

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
        todo!("backend.md §11.2")
    }

    /// **[COMPAT]** `CustomTheme.parse(hash)`: validate an already-decoded
    /// value (not size-checked). Check order: format, name, base, colors.
    /// `null` → `Ok(None)`.
    pub fn from_value(value: &serde_json::Value) -> Result<Option<Self>, CustomThemeError> {
        todo!()
    }

    /// Compact JSON exactly as Ruby `JSON.generate` writes it into `.fed`
    /// (key order `version, name, base, colors`; colors in input order).
    pub fn to_json_string(&self) -> String {
        todo!()
    }

    /// The same as a `serde_json::Value`.
    pub fn to_json(&self) -> serde_json::Value {
        todo!()
    }
}

/// **[COMPAT]** JS `normalizeColor`: trim + lowercase, then strict `#rrggbb`
/// (no 3-digit shorthand, no names, no CSS expressions).
pub fn normalize_color(value: &str) -> Option<String> {
    todo!()
}

/// **[COMPAT]** JS `themeColor(styles, key)` minus the browser-only named
/// color path: trim, expand a `#rgb` built-in (CSS-minified) value to
/// `#rrggbb`, then [`normalize_color`].
pub fn theme_color(raw_css_value: &str) -> Option<String> {
    todo!()
}

/// **[COMPAT]** JS `contrastRatio(a, b)`: `None` unless both normalize.
/// (The JS version yields `NaN` for padded inputs such as `" #808080 "`
/// because it slices the raw string; we use the normalized value.)
pub fn contrast_ratio_hex(a: &str, b: &str) -> Option<f64> {
    todo!()
}

/// **[COMPAT]** JS `applyCustomColors(colors)` filter: the `(key, color)`
/// pairs that would be applied, in input order — only [`crate::COLOR_KEYS`]
/// whose value is a string that normalizes. `null`/non-object → empty.
pub fn applicable_overrides(colors: &serde_json::Value) -> Vec<(&'static str, String)> {
    todo!()
}
