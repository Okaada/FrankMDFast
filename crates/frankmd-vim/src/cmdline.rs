//! The `:` line: line ranges, engine-native commands (`:<n>`, `:noh`, `:s`)
//! and FrankMD's app commands ([`crate::ex::parse`]).

use std::collections::HashMap;

use crate::ex::{self, ExCall};
use crate::text::{line_count, line_index};

/// `:[range]s/pattern/replacement/[flags]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Substitute {
    pub(crate) first: usize,
    pub(crate) last: usize,
    /// `None` = reuse the last search pattern (`:s//x/`).
    pub(crate) pattern: Option<String>,
    pub(crate) replacement: String,
    pub(crate) global: bool,
    pub(crate) ignore_case: bool,
}

/// What a `:` line means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExLine {
    /// Nothing to do (empty line).
    Empty,
    /// FrankMD app command.
    App(ExCall),
    /// `:<n>`: go to zero-based line.
    Goto(usize),
    /// `:noh[lsearch]`.
    NoHighlight,
    Substitute(Substitute),
    /// Error message for the status line.
    Error(String),
}

/// Where the cursor is, for resolving `.`, `$` and marks.
pub(crate) struct LineCtx<'a> {
    pub(crate) text: &'a str,
    pub(crate) cursor: usize,
    pub(crate) marks: &'a HashMap<char, usize>,
}

/// Parse and classify a `:` line (without the `:`).
pub(crate) fn parse(line: &str, cx: &LineCtx<'_>) -> ExLine {
    let line = line.trim();
    let (range, rest) = match parse_range(line, cx) {
        Ok(v) => v,
        Err(msg) => return ExLine::Error(msg),
    };
    let rest = rest.trim_start();
    if rest.is_empty() {
        return match range {
            Some((_, last)) => ExLine::Goto(last),
            None => ExLine::Empty,
        };
    }
    if let Some(call) = ex::parse(rest) {
        return ExLine::App(call);
    }
    let name_len = rest
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    let name = &rest[..name_len];
    if name.len() >= 3 && "nohlsearch".starts_with(name) {
        return ExLine::NoHighlight;
    }
    if !name.is_empty() && "substitute".starts_with(name) {
        let cur = line_index(cx.text, cx.cursor);
        let (first, last) = range.unwrap_or((cur, cur));
        return parse_substitute(&rest[name_len..], first, last);
    }
    ExLine::Error(format!("Not an editor command: {rest}"))
}

/// `[addr][,addr]` or `%`. Returns zero-based `(first, last)` and the remainder.
/// Zero-based inclusive line range.
type LineRange = (usize, usize);

fn parse_range<'s>(s: &'s str, cx: &LineCtx<'_>) -> Result<(Option<LineRange>, &'s str), String> {
    let last_line = line_count(cx.text) - 1;
    if let Some(rest) = s.strip_prefix('%') {
        return Ok((Some((0, last_line)), rest));
    }
    let (first, rest) = parse_address(s, cx)?;
    let Some(first) = first else {
        return Ok((None, s));
    };
    if let Some(after) = rest.strip_prefix([',', ';']) {
        let (second, rest) = parse_address(after, cx)?;
        let second = second.unwrap_or(first);
        let (a, b) = if first <= second {
            (first, second)
        } else {
            (second, first)
        };
        return Ok((Some((a.min(last_line), b.min(last_line))), rest));
    }
    Ok((Some((first.min(last_line), first.min(last_line))), rest))
}

/// One address with optional `+n`/`-n` offsets.
fn parse_address<'s>(s: &'s str, cx: &LineCtx<'_>) -> Result<(Option<usize>, &'s str), String> {
    let cur = line_index(cx.text, cx.cursor) as i64;
    let last = (line_count(cx.text) - 1) as i64;
    let mut rest = s;
    let mut base: Option<i64> = None;
    if let Some(r) = rest.strip_prefix('.') {
        base = Some(cur);
        rest = r;
    } else if let Some(r) = rest.strip_prefix('$') {
        base = Some(last);
        rest = r;
    } else if let Some(r) = rest.strip_prefix('\'') {
        let mut chars = r.chars();
        let name = chars.next().ok_or_else(|| "E20: Mark not set".to_owned())?;
        let pos = cx
            .marks
            .get(&name)
            .ok_or_else(|| "E20: Mark not set".to_owned())?;
        base = Some(line_index(cx.text, crate::text::clamp_boundary(cx.text, *pos)) as i64);
        rest = chars.as_str();
    } else {
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digits > 0 {
            let n: i64 = rest[..digits]
                .parse()
                .map_err(|_| "E16: Invalid range".to_owned())?;
            base = Some((n - 1).max(0));
            rest = &rest[digits..];
        }
    }
    // Offsets: `+3`, `-`, `.+1`.
    while let Some(sign) = rest.chars().next().filter(|c| *c == '+' || *c == '-') {
        let after = &rest[1..];
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let n: i64 = if digits == 0 {
            1
        } else {
            after[..digits]
                .parse()
                .map_err(|_| "E16: Invalid range".to_owned())?
        };
        let b = base.unwrap_or(cur);
        base = Some(if sign == '+' { b + n } else { b - n });
        rest = &after[digits..];
    }
    match base {
        Some(b) if b < 0 => Err("E16: Invalid range".to_owned()),
        Some(b) => Ok((Some(b.min(last) as usize), rest)),
        None => Ok((None, rest)),
    }
}

/// Split off one `delim`-terminated field; `\delim` is the delimiter itself.
fn field(s: &str, delim: char) -> (String, Option<&str>) {
    let mut out = String::new();
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some((_, d)) if d == delim => out.push(d),
                Some((_, d)) => {
                    out.push('\\');
                    out.push(d);
                }
                None => out.push('\\'),
            }
        } else if c == delim {
            return (out, Some(&s[i + c.len_utf8()..]));
        } else {
            out.push(c);
        }
    }
    (out, None)
}

fn parse_substitute(s: &str, first: usize, last: usize) -> ExLine {
    let Some(delim) = s
        .chars()
        .next()
        .filter(|c| !c.is_alphanumeric() && !c.is_whitespace() && *c != '"')
    else {
        return ExLine::Error("E35: No previous regular expression".to_owned());
    };
    let (pattern, rest) = field(&s[delim.len_utf8()..], delim);
    let (replacement, flags) = match rest {
        Some(r) => {
            let (rep, flags) = field(r, delim);
            (rep, flags.unwrap_or(""))
        }
        None => (String::new(), ""),
    };
    ExLine::Substitute(Substitute {
        first,
        last,
        pattern: (!pattern.is_empty()).then_some(pattern),
        replacement,
        global: flags.contains('g'),
        ignore_case: flags.contains('i'),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(line: &str) -> ExLine {
        let marks = HashMap::from([('a', 4usize)]);
        parse(
            line,
            &LineCtx {
                text: "l0\nl1\nl2\nl3\n",
                cursor: 3,
                marks: &marks,
            },
        )
    }

    #[test]
    fn native_commands() {
        assert_eq!(run("3"), ExLine::Goto(2));
        assert_eq!(run("$"), ExLine::Goto(4));
        assert_eq!(run(".+2"), ExLine::Goto(3));
        assert_eq!(run(""), ExLine::Empty);
        assert_eq!(run("noh"), ExLine::NoHighlight);
        assert_eq!(run("nohlsearch"), ExLine::NoHighlight);
        assert!(matches!(run("w"), ExLine::App(_)));
        assert!(matches!(run("bogus"), ExLine::Error(_)));
        assert_eq!(
            run("%s/a\\/b/c/gi"),
            ExLine::Substitute(Substitute {
                first: 0,
                last: 4,
                pattern: Some("a/b".into()),
                replacement: "c".into(),
                global: true,
                ignore_case: true,
            })
        );
        assert!(matches!(
            run("'a,$s/x/y/"),
            ExLine::Substitute(Substitute {
                first: 1,
                last: 4,
                ..
            })
        ));
        assert!(matches!(
            run("s//y"),
            ExLine::Substitute(Substitute {
                first: 1,
                last: 1,
                pattern: None,
                ..
            })
        ));
    }
}
