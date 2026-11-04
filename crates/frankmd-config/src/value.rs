//! Typed config values and the **[COMPAT]** casting rules (backend.md §1.4).

use crate::schema::KeyType;
use crate::{ConfigError, Key};

/// A cast value. `None` at the call sites means Ruby `nil`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// String value.
    Str(String),
    /// Integer value (Ruby `to_i`: `"12px"` → 12, `"abc"` → 0).
    Int(i64),
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
        todo!()
    }

    /// String content, if a string.
    pub fn as_str(&self) -> Option<&str> {
        todo!()
    }

    /// Integer content, if an integer.
    pub fn as_int(&self) -> Option<i64> {
        todo!()
    }

    /// Boolean content, if a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        todo!()
    }
}

/// Cast a raw `.fed`/ENV string per type. Blank (after trim) → `None` for
/// every type. `Err` only for JSON / custom-theme parse failures (the `.fed`
/// line is then skipped with a warning).
pub fn cast(raw: &str, ty: KeyType) -> Result<Option<Value>, String> {
    todo!()
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
    todo!()
}
