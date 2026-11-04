//! Editor gutter line-number mode. **[COMPAT]** `app/javascript/lib/line_numbers.js`;
//! the numeric values are what `.fed` `editor_line_numbers` persists. Golden
//! `js_line_numbers.json`.
//!
//! Lives here (next to the line mapping) only because it was assigned with the
//! preview/line-mapping port; the editor/workspace may re-export or move it.

/// `LINE_NUMBER_MODES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineNumberMode {
    /// 0.
    Off,
    /// 1 (FrankMD's default fallback).
    #[default]
    Absolute,
    /// 2.
    Relative,
}

impl LineNumberMode {
    /// Persisted numeric value (0/1/2).
    pub fn value(self) -> u8 {
        match self {
            LineNumberMode::Off => 0,
            LineNumberMode::Absolute => 1,
            LineNumberMode::Relative => 2,
        }
    }

    /// Inverse of [`Self::value`].
    pub fn from_value(value: i64) -> Option<Self> {
        match value {
            0 => Some(LineNumberMode::Off),
            1 => Some(LineNumberMode::Absolute),
            2 => Some(LineNumberMode::Relative),
            _ => None,
        }
    }

    /// `nextLineNumberMode`: Off → Absolute → Relative → Off.
    pub fn next(self) -> Self {
        match self {
            LineNumberMode::Off => LineNumberMode::Absolute,
            LineNumberMode::Absolute => LineNumberMode::Relative,
            LineNumberMode::Relative => LineNumberMode::Off,
        }
    }
}

/// `normalizeLineNumberMode(value, fallback)` over the value's string form
/// (`None` = `undefined`/missing): JS `parseInt(value, 10)` semantics (leading
/// whitespace skipped, optional sign, leading digits only: `"2px"` → 2,
/// `" 1"` → 1, `"1.9"` → 1); anything that does not parse to 0/1/2 → `fallback`.
pub fn normalize_line_number_mode(value: Option<&str>, fallback: LineNumberMode) -> LineNumberMode {
    value.and_then(parse_int_prefix).and_then(LineNumberMode::from_value).unwrap_or(fallback)
}

/// JS `parseInt(s, 10)` reduced to what mode matching needs: `None` for NaN,
/// otherwise the value (saturated; `-0` is 0, which JS `=== 0` also accepts).
fn parse_int_prefix(s: &str) -> Option<i64> {
    let s = s.trim_start_matches(is_js_space);
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let digits: &str = &digits[..digits.bytes().take_while(u8::is_ascii_digit).count()];
    if digits.is_empty() {
        return None;
    }
    let magnitude = digits
        .bytes()
        .fold(0_i64, |acc, d| acc.saturating_mul(10).saturating_add(i64::from(d - b'0')));
    Some(if negative { -magnitude } else { magnitude })
}

/// JS `StrWhiteSpaceChar` (WhiteSpace + LineTerminator).
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}'
            | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}'
    )
}
