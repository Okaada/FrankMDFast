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
    todo!("indent_utils.js parseIndentSetting")
}

/// Prefix every `\n`-separated line (including empty ones) with `indent`.
pub fn indent_lines(text: &str, indent: &str) -> String {
    todo!("indent_utils.js indentLines")
}

/// Remove one level of indentation from each line: the exact `indent` if
/// present; else, for a space indent, up to `indent.len()` leading spaces;
/// else a single leading tab.
pub fn unindent_lines(text: &str, indent: &str) -> String {
    todo!("indent_utils.js unindentLines")
}
