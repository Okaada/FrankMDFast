//! Editor find/replace (**[COMPAT]** `find_utils.js`, frontend.md A.16; golden
//! `js_find_utils.json`).
//!
//! Offsets are **UTF-16 code units** like the original (the editor converts
//! with `frankmd_core::text`). Deliberate deviation: matching works on Unicode
//! scalar values, so a match (or an empty-match advance) never splits a
//! surrogate pair, whereas JS without the `u` flag can return lone surrogates.
//!
//! Patterns are JavaScript regex source; [`js_regex`] translates them to
//! `fancy-regex` with JS meanings (and V8's validation errors).

mod js_regex;

/// Find options (`{caseSensitive = false, useRegex = false}`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FindOptions {
    /// Case sensitive (otherwise the `i` flag).
    pub case_sensitive: bool,
    /// Interpret the query as a JS regex (otherwise it is escaped).
    pub use_regex: bool,
}

/// One match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Match {
    /// Start, UTF-16 code units.
    pub start: usize,
    /// End (exclusive), UTF-16 code units.
    pub end: usize,
    /// Matched text.
    pub text: String,
    /// Capture groups 1..; `None` for a group that did not participate
    /// (JS `undefined`).
    pub groups: Vec<Option<String>>,
}

/// Search direction for [`find_closest_match_index`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    /// `"next"` / `1` (the default).
    #[default]
    Next,
    /// `"previous"` / `-1`.
    Previous,
}

/// `escapeRegexChars`: backslash-escape `. * + ? ^ $ { } ( ) | [ ] \`.
pub fn escape_regex_chars(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if ".*+?^${}()|[]\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Compile a JS pattern with flags `g` (+ `i` unless case sensitive).
fn compile(pattern: &str, case_sensitive: bool) -> Option<fancy_regex::Regex> {
    let translated = js_regex::translate(pattern).ok()?;
    let source = if case_sensitive { translated } else { format!("(?i){translated}") };
    fancy_regex::Regex::new(&source).ok()
}

/// Incremental byte offset → UTF-16 offset conversion for ascending offsets.
struct Utf16Cursor<'a> {
    text: &'a str,
    byte: usize,
    unit: usize,
}

impl Utf16Cursor<'_> {
    fn advance_to(&mut self, byte: usize) -> usize {
        if byte < self.byte {
            self.byte = 0;
            self.unit = 0;
        }
        self.unit += self.text[self.byte..byte].encode_utf16().count();
        self.byte = byte;
        self.unit
    }
}

/// `findAllMatches(text, query, options)`: all non-overlapping matches
/// (flags `g` + `i` unless case sensitive). Empty text/query or an
/// invalid regex → empty. A zero-length match advances by one position.
pub fn find_all_matches(text: &str, query: &str, opts: FindOptions) -> Vec<Match> {
    if text.is_empty() || query.is_empty() {
        return Vec::new();
    }
    let pattern = if opts.use_regex { query.to_owned() } else { escape_regex_chars(query) };
    let Some(regex) = compile(&pattern, opts.case_sensitive) else { return Vec::new() };

    let mut matches = Vec::new();
    let mut cursor = Utf16Cursor { text, byte: 0, unit: 0 };
    let mut pos = 0;
    while pos <= text.len() {
        let Ok(Some(caps)) = regex.captures_from_pos(text, pos) else { break };
        let Some(whole) = caps.get(0) else { break };
        let groups = (1..caps.len()).map(|i| caps.get(i).map(|g| g.as_str().to_owned())).collect();
        let start = cursor.advance_to(whole.start());
        let end = cursor.advance_to(whole.end());
        matches.push(Match { start, end, text: whole.as_str().to_owned(), groups });
        pos = if whole.start() == whole.end() {
            // JS `lastIndex += 1`, by a whole char (see the module docs).
            match text[whole.end()..].chars().next() {
                Some(c) => whole.end() + c.len_utf8(),
                None => break,
            }
        } else {
            whole.end()
        };
    }
    matches
}

/// `replaceMatches`: replace from the last match backwards. In regex mode
/// the replacement expands `$$` → `$`, `$&`/`$0` → match, `$N` (any digit
/// run, `$01` = group 1) → group or `""`; literal mode inserts it verbatim.
/// Empty `text` or no matches → `text` unchanged.
pub fn replace_matches(text: &str, matches: &[Match], replacement: &str, use_regex: bool) -> String {
    if text.is_empty() || matches.is_empty() {
        return text.to_owned();
    }
    // JS `substring` semantics over UTF-16 units.
    let mut units: Vec<u16> = text.encode_utf16().collect();
    for m in matches.iter().rev() {
        let len = units.len();
        let (a, b) = (m.start.min(len), m.end.min(len));
        let (start, end) = (a.min(b), a.max(b));
        let resolved = if use_regex { resolve_regex_replacement(replacement, m) } else { replacement.to_owned() };
        units.splice(start..end, resolved.encode_utf16());
    }
    String::from_utf16_lossy(&units)
}

/// `resolveRegexReplacement`: `/\$(\$|&|\d+)/g`.
fn resolve_regex_replacement(replacement: &str, m: &Match) -> String {
    let mut out = String::with_capacity(replacement.len());
    let mut rest = replacement;
    while let Some(ix) = rest.find('$') {
        out.push_str(&rest[..ix]);
        let after = &rest[ix + 1..];
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        if let Some(tail) = after.strip_prefix('$') {
            out.push('$');
            rest = tail;
        } else if let Some(tail) = after.strip_prefix('&') {
            out.push_str(&m.text);
            rest = tail;
        } else if digits > 0 {
            let token = &after[..digits];
            if token == "0" {
                out.push_str(&m.text);
            } else {
                let index = token.parse::<usize>().unwrap_or(0);
                if let Some(Some(group)) = index.checked_sub(1).and_then(|i| m.groups.get(i)) {
                    out.push_str(group);
                }
            }
            rest = &after[digits..];
        } else {
            out.push('$');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// `findClosestMatchIndex`: Next → first match with `start >= position`,
/// wrapping to 0; Previous → last match with `end <= position`, wrapping to
/// the last. `None` (JS `-1`) when there are no matches.
pub fn find_closest_match_index(matches: &[Match], position: i64, direction: Direction) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    let as_i64 = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
    match direction {
        Direction::Previous => {
            Some(matches.iter().rposition(|m| as_i64(m.end) <= position).unwrap_or(matches.len() - 1))
        }
        Direction::Next => Some(matches.iter().position(|m| as_i64(m.start) >= position).unwrap_or(0)),
    }
}

/// `validateRegex`: `Ok` for an empty pattern or a valid JS RegExp (no
/// flags, non-unicode mode: `\p{L}` is valid, `x++` is not); otherwise
/// `Err("Invalid regular expression: /<pattern>/: <reason>")`.
pub fn validate_regex(pattern: &str) -> Result<(), String> {
    if pattern.is_empty() {
        return Ok(());
    }
    js_regex::translate(pattern)
        .map(|_| ())
        .map_err(|reason| format!("Invalid regular expression: /{pattern}/: {reason}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn texts(text: &str, query: &str) -> Vec<String> {
        let opts = FindOptions { case_sensitive: true, use_regex: true };
        find_all_matches(text, query, opts).into_iter().map(|m| m.text).collect()
    }

    #[test]
    fn js_specific_constructs_translate() {
        assert_eq!(texts("ab", "[]"), Vec::<String>::new());
        assert_eq!(texts("ab", "[^]"), vec!["a", "b"]);
        assert_eq!(texts("é a", r"\b\w\b"), vec!["a"]);
        assert_eq!(texts("a-1", r"[\d-a]+"), vec!["a-1"]);
        assert_eq!(texts("p{L}", r"\p{L}"), vec!["p{L}"]);
        assert_eq!(texts("aa", r"(a)\1"), vec!["aa"]);
        assert_eq!(texts("\u{8}", r"\10"), vec!["\u{8}"]);
        assert_eq!(texts("x\ry", "x.y"), Vec::<String>::new());
        assert_eq!(texts("a{b", "a{b"), vec!["a{b"]);
        assert_eq!(texts("ab", r"\uD800"), Vec::<String>::new());
        assert_eq!(texts("😀", r"😀"), vec!["😀"]);
        assert_eq!(texts("xay", "(?<=x)a(?=y)"), vec!["a"]);
        assert_eq!(texts("aa", r"(?<n>a)\k<n>"), vec!["aa"]);
    }

    #[test]
    fn validate_reports_v8_reasons() {
        assert_eq!(validate_regex("[b-a]").unwrap_err(), "Invalid regular expression: /[b-a]/: Range out of order in character class");
        assert!(validate_regex("(?<=a)*").is_err());
        assert!(validate_regex("(?=a)*").is_ok());
        assert!(validate_regex("a{,2}").is_ok());
        assert!(validate_regex("]").is_ok());
    }
}
