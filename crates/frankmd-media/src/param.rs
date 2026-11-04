//! Loosely-typed request parameters.
//!
//! The original controllers passed raw params (`nil`, booleans, numbers,
//! strings, arrays) straight into services that coerced them with Ruby
//! semantics (`Integer()`, `to_s`, `ActiveModel::Type::Boolean.cast`). The
//! golden fixtures record those inputs as JSON values; [`Param`] keeps the
//! distinction so the coercions can be reproduced exactly.

/// One request parameter as received.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Param {
    /// Absent / `nil`.
    #[default]
    Nil,
    /// JSON boolean.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float (Ruby `Float#to_s` formatting matters, e.g. `1.0` → `"1.0"`).
    Float(f64),
    /// String.
    Str(String),
    /// Array (e.g. `url[]=...`, rejected by import).
    List(Vec<Param>),
}

impl Param {
    /// Convenience: `Param::Str(s.into())`.
    pub fn str(s: impl Into<String>) -> Self {
        Param::Str(s.into())
    }

    /// `true` for `Nil`.
    pub fn is_nil(&self) -> bool {
        matches!(self, Param::Nil)
    }

    /// Rails `present?` (non-nil, and strings non-blank, arrays non-empty).
    pub fn is_present(&self) -> bool {
        todo!()
    }

    /// Ruby `to_s` (`nil` → `""`, `1.0` → `"1.0"`, `2` → `"2"`, `true` → `"true"`).
    pub fn ruby_to_s(&self) -> String {
        todo!()
    }

    /// `ActiveModel::Type::Boolean.new.cast`: `nil` and `""` → `None`;
    /// `false, 0, "0", "f", "F", "false", "FALSE", "off", "OFF"` → `Some(false)`;
    /// anything else → `Some(true)`.
    pub fn rails_boolean_cast(&self) -> Option<bool> {
        todo!()
    }
}
