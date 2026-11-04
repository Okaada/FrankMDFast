//! Line indent/unindent and the `editor_indent` setting (port of
//! `lib/indent_utils.js`). Golden: `js_indent_utils.json`.

/// A raw `editor_indent` value as it may arrive (from `.fed`, settings UI or
/// a JSON payload). Mirrors the JS value kinds the original accepted.
#[derive(Debug, Clone, PartialEq)]
pub enum IndentValue {
    /// A number (may be fractional or NaN).
    Number(f64),
    /// A string, parsed like JS `parseInt(s, 10)` (leading whitespace, sign,
    /// digits; trailing junk ignored; `"0x4"` → 0).
    Text(String),
    /// A boolean (always the default).
    Bool(bool),
}

/// Default indent: two spaces.
pub const DEFAULT_INDENT: &str = "  ";

/// Resolve the indent unit: missing/empty/unparsable/negative → two spaces;
/// `0` → tab; `1..=6` → that many spaces; larger values clamp to 6.
/// Fractions truncate (`2.7` → 2 spaces).
pub fn parse_indent_setting(value: Option<&IndentValue>) -> &'static str {
    const SPACES: [&str; 6] = [" ", "  ", "   ", "    ", "     ", "      "];
    let parsed = match value {
        None => None,
        Some(IndentValue::Text(text)) if text.is_empty() => None,
        Some(IndentValue::Text(text)) => parse_int(text),
        Some(IndentValue::Number(number)) => parse_int_number(*number),
        Some(IndentValue::Bool(_)) => None,
    };
    match parsed {
        None => DEFAULT_INDENT,
        // `-0` is not `< 0` in JS, and `=== 0`.
        Some(JsInt { magnitude: 0, .. }) => "\t",
        Some(JsInt { negative: true, .. }) => DEFAULT_INDENT,
        Some(JsInt { magnitude, .. }) => SPACES[magnitude.clamp(1, 6) as usize - 1],
    }
}

/// Prefix every `\n`-separated line (including empty ones) with `indent`.
pub fn indent_lines(text: &str, indent: &str) -> String {
    text.split('\n').map(|line| format!("{indent}{line}")).collect::<Vec<_>>().join("\n")
}

/// Remove one level of indentation from each line: the exact `indent` if
/// present; else, for a space indent, up to `indent.len()` leading spaces;
/// else a single leading tab.
pub fn unindent_lines(text: &str, indent: &str) -> String {
    text.split('\n').map(|line| unindent_line(line, indent)).collect::<Vec<_>>().join("\n")
}

/// One line of [`unindent_lines`].
fn unindent_line<'a>(line: &'a str, indent: &str) -> &'a str {
    if let Some(rest) = line.strip_prefix(indent) {
        return rest;
    }
    if indent != "\t" {
        // `indent.length` is UTF-16 units, but only ASCII spaces are removed.
        let limit = super::utf16_len(indent);
        let spaces = line.bytes().take(limit).take_while(|&b| b == b' ').count();
        if spaces > 0 {
            return &line[spaces..];
        }
    }
    line.strip_prefix('\t').unwrap_or(line)
}

/// Result of JS `parseInt`, keeping the sign of zero.
#[derive(Debug, Clone, Copy)]
struct JsInt {
    negative: bool,
    magnitude: u64,
}

/// JS `parseInt(text, 10)`: leading whitespace, optional sign, decimal digits;
/// trailing junk ignored; `None` (NaN) without digits.
fn parse_int(text: &str) -> Option<JsInt> {
    let rest = text.trim_start_matches(super::is_js_whitespace);
    let (negative, digits) = match rest.as_bytes().first() {
        Some(b'-') => (true, &rest[1..]),
        Some(b'+') => (false, &rest[1..]),
        _ => (false, rest),
    };
    let digits: Vec<u8> = digits.bytes().take_while(u8::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let magnitude = digits.iter().fold(0u64, |acc, d| acc.saturating_mul(10).saturating_add(u64::from(d - b'0')));
    Some(JsInt { negative, magnitude })
}

/// JS `parseInt(number, 10)`, which first formats the number with
/// `String(number)`: exponent notation (`1e+21`, `1e-7`) keeps only the
/// leading mantissa digit; otherwise the value truncates.
fn parse_int_number(number: f64) -> Option<JsInt> {
    if !number.is_finite() {
        return None;
    }
    let negative = number.is_sign_negative();
    let abs = number.abs();
    let uses_exponent = abs >= 1e21 || (abs != 0.0 && abs < 1e-6);
    let magnitude = if uses_exponent {
        let formatted = format!("{abs:e}");
        formatted.bytes().next().map_or(0, |d| u64::from(d.saturating_sub(b'0')))
    } else {
        // Below 1e21, so the truncated value fits a u64 saturating cast.
        abs.trunc() as u64
    };
    Some(JsInt { negative, magnitude })
}
