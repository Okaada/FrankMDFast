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
        match self {
            Param::Nil => false,
            Param::Bool(b) => *b,
            Param::Str(s) => !s.trim().is_empty(),
            Param::List(items) => !items.is_empty(),
            Param::Int(_) | Param::Float(_) => true,
        }
    }

    /// Ruby `to_s` (`nil` → `""`, `1.0` → `"1.0"`, `2` → `"2"`, `true` → `"true"`).
    pub fn ruby_to_s(&self) -> String {
        match self {
            Param::Nil => String::new(),
            Param::Bool(b) => b.to_string(),
            Param::Int(i) => i.to_string(),
            Param::Float(f) => ruby_float_to_s(*f),
            Param::Str(s) => s.clone(),
            Param::List(items) => {
                let inner: Vec<String> = items.iter().map(Param::ruby_inspect).collect();
                format!("[{}]", inner.join(", "))
            }
        }
    }

    /// Ruby `inspect` (only used for arrays' `to_s`).
    fn ruby_inspect(&self) -> String {
        match self {
            Param::Nil => "nil".to_owned(),
            Param::Str(s) => format!("{s:?}"),
            other => other.ruby_to_s(),
        }
    }

    /// `ActiveModel::Type::Boolean.new.cast`: `nil` and `""` → `None`;
    /// `false, 0, "0", "f", "F", "false", "FALSE", "off", "OFF"` → `Some(false)`;
    /// anything else → `Some(true)`.
    pub fn rails_boolean_cast(&self) -> Option<bool> {
        const FALSE_VALUES: [&str; 8] = ["0", "f", "F", "false", "FALSE", "off", "OFF", ""];
        match self {
            Param::Nil => None,
            Param::Bool(b) => Some(*b),
            Param::Int(i) => Some(*i != 0),
            Param::Float(f) => Some(*f != 0.0),
            Param::Str(s) if s.is_empty() => None,
            Param::Str(s) => Some(!FALSE_VALUES.contains(&s.as_str())),
            Param::List(_) => Some(true),
        }
    }
}

/// Ruby `Float#to_s`: shortest round-trip digits, always with a fractional
/// part (`1.0`, `0.5`), scientific notation outside `1e-4 ..= 1e16`.
pub(crate) fn ruby_float_to_s(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_owned();
    }
    if f.is_infinite() {
        return if f > 0.0 {
            "Infinity".to_owned()
        } else {
            "-Infinity".to_owned()
        };
    }
    let abs = f.abs();
    if abs != 0.0 && !(1e-4..1e16).contains(&abs) {
        let s = format!("{f:e}");
        let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mantissa = if mantissa.contains('.') {
            mantissa.to_owned()
        } else {
            format!("{mantissa}.0")
        };
        let exp: i32 = exp.parse().unwrap_or(0);
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{mantissa}e{sign}{:02}", exp.abs());
    }
    let s = format!("{f}");
    if s.contains('.') { s } else { format!("{s}.0") }
}
