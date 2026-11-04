//! Typed config values and the **[COMPAT]** casting rules (backend.md §1.4).

use crate::schema::KeyType;
use crate::{ConfigError, Key};

/// A cast value. `None` at the call sites means Ruby `nil`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// String value.
    Str(String),
    /// Integer value (Ruby `to_i`: `"12px"` → 12, `"abc"` → 0). Wider than
    /// `i64` because Ruby integers are unbounded and a huge `.fed` value must
    /// round-trip (**[COMPAT]**); [`Value::as_int`] narrows it.
    Int(i128),
    /// Boolean value.
    Bool(bool),
    /// JSON value (`recent_files`).
    Json(serde_json::Value),
    /// Validated custom theme.
    CustomTheme(frankmd_theme::CustomTheme),
}

impl Value {
    /// JSON form, as the original returned it to the frontend (`/config`,
    /// `/logs/config`); a custom theme becomes its normalized object.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::Int(i) => int_json(*i),
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Json(j) => j.clone(),
            Value::CustomTheme(t) => t.to_json(),
        }
    }

    /// String content, if a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Integer content, if an integer that fits in `i64`.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => i64::try_from(*i).ok(),
            _ => None,
        }
    }

    /// Boolean content, if a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// An integer as JSON; beyond `i64` it degrades to a float like JSON readers do.
fn int_json(i: i128) -> serde_json::Value {
    match i64::try_from(i) {
        Ok(small) => serde_json::Value::from(small),
        // Lossy on purpose: JSON has no integers this large.
        #[allow(clippy::cast_precision_loss)]
        Err(_) => serde_json::Value::from(i as f64),
    }
}

/// **[COMPAT]** Ruby `String#strip`: ASCII whitespace and NUL only (NBSP and
/// other Unicode spaces are kept).
pub(crate) fn ruby_strip(s: &str) -> &str {
    s.trim_matches(is_ruby_space)
}

fn is_ruby_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\0')
}

/// **[COMPAT]** ActiveSupport `present?` for strings: not empty after
/// removing Unicode whitespace.
pub(crate) fn present_str(s: &str) -> bool {
    !s.trim().is_empty()
}

/// **[COMPAT]** Ruby `String#to_i`: optional leading whitespace and sign, then
/// decimal digits with single `_` separators; anything else ends the number
/// (`"12px"` → 12, `"1__0"` → 1, `"abc"` → 0). Saturates beyond `i128`.
pub(crate) fn ruby_to_i(s: &str) -> i128 {
    let s = s.trim_start_matches(|c: char| c != '\0' && is_ruby_space(c));
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let bytes = digits.as_bytes();
    let mut value: i128 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_digit() {
            let d = i128::from(b - b'0');
            value = if negative {
                value.saturating_mul(10).saturating_sub(d)
            } else {
                value.saturating_mul(10).saturating_add(d)
            };
        } else if b == b'_'
            && i > 0
            && bytes[i - 1].is_ascii_digit()
            && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
        {
            // A single underscore between digits is a separator.
        } else {
            break;
        }
        i += 1;
    }
    value
}

/// **[COMPAT]** boolean rule: `true|1|yes|on`, case-insensitive.
fn ruby_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "true" | "1" | "yes" | "on")
}

/// Cast a raw `.fed`/ENV string per type. Blank (after trim) → `None` for
/// every type. `Err` only for JSON / custom-theme parse failures (the `.fed`
/// line is then skipped with a warning).
pub fn cast(raw: &str, ty: KeyType) -> Result<Option<Value>, String> {
    if ruby_strip(raw).is_empty() {
        return Ok(None);
    }
    Ok(match ty {
        KeyType::String => Some(Value::Str(raw.to_owned())),
        KeyType::Integer => Some(Value::Int(ruby_to_i(raw))),
        KeyType::Boolean => Some(Value::Bool(ruby_bool(raw))),
        KeyType::Json => json_value(serde_json::from_str(raw).map_err(|e| e.to_string())?),
        KeyType::CustomTheme => frankmd_theme::CustomTheme::parse(raw)
            .map_err(|e| e.to_string())?
            .map(Value::CustomTheme),
    })
}

/// JSON `null` is Ruby `nil`.
fn json_value(v: serde_json::Value) -> Option<Value> {
    (!v.is_null()).then_some(Value::Json(v))
}

/// **[COMPAT]** `cast_value` applied to a value handed to `set`/`update`
/// (already-typed input from the UI or a caller, modelled as JSON):
/// - `null` or a blank string → `None`;
/// - string keys take `to_s` (`42` → `"42"`), integers take Ruby `to_i`
///   (`"12px"` → 12, `18.7` → 18, `"abc"` → 0; a boolean is an error),
/// - booleans take the `true|1|yes|on` rule on `to_s`,
/// - JSON keys accept a value or a JSON string (invalid JSON string → error),
/// - custom themes accept an object or a JSON string and are validated.
pub fn coerce(key: Key, input: &serde_json::Value) -> Result<Option<Value>, ConfigError> {
    use serde_json::Value as J;
    let invalid = |reason: &str| ConfigError::InvalidValue {
        key,
        reason: reason.to_owned(),
    };
    match input {
        J::Null => return Ok(None),
        J::String(s) if ruby_strip(s).is_empty() => return Ok(None),
        _ => {}
    }
    let ty = key.spec().ty;
    Ok(match ty {
        KeyType::String => Some(Value::Str(
            scalar_to_s(input).ok_or_else(|| invalid("not a scalar"))?,
        )),
        KeyType::Integer => Some(Value::Int(match input {
            J::String(s) => ruby_to_i(s),
            J::Number(n) => number_to_i(n),
            _ => return Err(invalid("not an integer")),
        })),
        KeyType::Boolean => Some(Value::Bool(ruby_bool(
            &scalar_to_s(input).ok_or_else(|| invalid("not a boolean"))?,
        ))),
        KeyType::Json => match input {
            J::String(s) => {
                json_value(
                    serde_json::from_str(s).map_err(|e| ConfigError::InvalidJson {
                        key,
                        reason: e.to_string(),
                    })?,
                )
            }
            other => json_value(other.clone()),
        },
        KeyType::CustomTheme => match input {
            J::String(s) => frankmd_theme::CustomTheme::parse(s)?,
            other => frankmd_theme::CustomTheme::from_value(other)?,
        }
        .map(Value::CustomTheme),
    })
}

/// Ruby `to_s` of a JSON scalar (`None` for arrays/objects, whose Ruby
/// `inspect` form could never be written back safely anyway).
fn scalar_to_s(v: &serde_json::Value) -> Option<String> {
    use serde_json::Value as J;
    match v {
        J::String(s) => Some(s.clone()),
        J::Number(n) => Some(n.to_string()),
        J::Bool(b) => Some(b.to_string()),
        J::Null => Some(String::new()),
        J::Array(_) | J::Object(_) => None,
    }
}

/// Ruby `Integer#to_i` / `Float#to_i` (truncation toward zero).
fn number_to_i(n: &serde_json::Number) -> i128 {
    if let Some(i) = n.as_i64() {
        i128::from(i)
    } else if let Some(u) = n.as_u64() {
        i128::from(u)
    } else {
        // `as` saturates for out-of-range floats and maps NaN to 0.
        #[allow(clippy::cast_possible_truncation)]
        let t = n.as_f64().unwrap_or(0.0).trunc() as i128;
        t
    }
}
