//! Hand-compiled forms of the hottest marked rules. They return exactly what
//! the regex in [`super::rules`] would (the tests run both on the same
//! inputs); fancy-regex's backtracking VM made these rules most of a render.

use std::sync::LazyLock;

use super::rules::{self, exec};

/// A `fences` match: `cap[0]` length, `cap[2]` (info) and `cap[3]` (body).
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Fence<'a> {
    pub(crate) raw_len: usize,
    pub(crate) info: &'a str,
    pub(crate) body: Option<&'a str>,
}

/// `rules::FENCES`:
/// `^ {0,3}(`{3,}(?=[^`\n]*(?:\n|$))|~{3,})([^\n]*)(?:\n|$)(?:|([\s\S]*?)(?:\n|$))(?: {0,3}\1[~`]* *(?=\n|$)|$)`.
pub(crate) fn fences(src: &str) -> Option<Fence<'_>> {
    let spaces = src.len() - src.trim_start_matches(' ').len();
    if spaces > 3 {
        return None;
    }
    let c = *src.as_bytes().get(spaces)?;
    if c != b'`' && c != b'~' {
        return None;
    }
    let run_end = spaces + src[spaces..].bytes().take_while(|&b| b == c).count();
    if run_end - spaces < 3 {
        return None;
    }
    let line_end = src[run_end..].find('\n').map_or(src.len(), |i| run_end + i);
    let info = &src[run_end..line_end];
    if c == b'`' && info.contains('`') {
        return None;
    }
    let fence = &src[spaces..run_end];
    let closing = |pos: usize| -> Option<usize> {
        let s = &src[pos..];
        let sp = s.len() - s.trim_start_matches(' ').len();
        if sp > 3 || !s[sp..].starts_with(fence) {
            return None;
        }
        let mut j = sp + fence.len();
        j += s[j..].bytes().take_while(|&b| b == b'~' || b == b'`').count();
        j += s[j..].bytes().take_while(|&b| b == b' ').count();
        (j == s.len() || s.as_bytes()[j] == b'\n').then_some(pos + j)
    };
    let pos0 = if line_end < src.len() { line_end + 1 } else { src.len() };
    // Empty-body alternative first.
    if let Some(end) = closing(pos0) {
        return Some(Fence { raw_len: end, info, body: None });
    }
    if pos0 == src.len() {
        return Some(Fence { raw_len: pos0, info, body: None });
    }
    // Lazy body: the first `\n` (from pos0) after which a closing fence or the end follows.
    let mut n = pos0;
    loop {
        match src[n..].find('\n') {
            Some(i) => {
                let nl = n + i;
                let after = nl + 1;
                if let Some(end) = closing(after) {
                    return Some(Fence { raw_len: end, info, body: Some(&src[pos0..nl]) });
                }
                if after == src.len() {
                    return Some(Fence { raw_len: after, info, body: Some(&src[pos0..nl]) });
                }
                n = after;
            }
            None => return Some(Fence { raw_len: src.len(), info, body: Some(&src[pos0..]) }),
        }
    }
}

/// `rules::NEWLINE` (`^(?:[ \t]*(?:\n|$))+`): length of the match, 0 for none.
pub(crate) fn newline(src: &str) -> usize {
    let mut p = 0;
    loop {
        let q = p + src[p..].bytes().take_while(|&b| b == b' ' || b == b'\t').count();
        match src.as_bytes().get(q) {
            Some(b'\n') => p = q + 1,
            None => return q,
            Some(_) => return p,
        }
    }
}

/// `rules::PARAGRAPH` (GFM): the length of `cap[0]` (= `cap[1]`).
pub(crate) fn paragraph(src: &str) -> Option<usize> {
    let mut p = src.find('\n').unwrap_or(src.len());
    if p == 0 {
        return None;
    }
    while src.as_bytes().get(p) == Some(&b'\n') {
        let rest = &src[p + 1..];
        let line = rest.split('\n').next().unwrap_or("");
        if line.is_empty() || paragraph_interrupted(rest) {
            break;
        }
        p += 1 + line.len();
    }
    Some(p)
}

/// The paragraph's `(?!hr|heading|blockquote|fences|list|html|table| +\n)`.
fn paragraph_interrupted(rest: &str) -> bool {
    let c = rest.trim_start_matches(' ').as_bytes().first().copied().unwrap_or(b'\n');
    let maybe = matches!(c, b'-' | b'_' | b'*' | b'#' | b'>' | b'`' | b'~' | b'+' | b'<' | b'\n') || c.is_ascii_digit();
    (maybe && exec(&PARAGRAPH_STOP, rest).is_some()) || (table_second_line(rest) && table(rest).is_some())
}

/// Cheap necessary condition for [`table`]: the second line looks like a delimiter row.
pub(crate) fn table_second_line(src: &str) -> bool {
    src.split('\n')
        .nth(1)
        .is_some_and(|l| l.contains('-') && l.bytes().all(|b| matches!(b, b'|' | b':' | b'-' | b' ')))
}

static PARAGRAPH_STOP: LazyLock<fancy_regex::Regex> = LazyLock::new(rules::paragraph_stop);

/// A GFM `table` match: `cap[0]` length and groups 1–3.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TableMatch<'a> {
    pub(crate) raw_len: usize,
    pub(crate) header: &'a str,
    pub(crate) align: &'a str,
    pub(crate) rows: Option<&'a str>,
}

/// `^ {0,3}(DELIM)(?:\n|$)` (no look-around: the plain `regex` engine).
static DELIMITER: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^ {0,3}((?:\| *)?:?-+:? *(?:\| *:?-+:? *)*(?:\| *)?)(?:\n|$)").expect("valid regex")
});

/// JS `.`: anything but a line terminator.
fn is_js_dot(c: char) -> bool {
    !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `rules::TABLE`: header line, delimiter row, then rows up to a blank line
/// or a block that may interrupt a table (`rules::TABLE_ROW_STOP`).
pub(crate) fn table(src: &str) -> Option<TableMatch<'_>> {
    // ^ *([^\n ].*)\n
    let lead = src.len() - src.trim_start_matches(' ').len();
    let first = src[lead..].chars().next()?;
    if first == '\n' || first == ' ' {
        return None;
    }
    let header_end = lead + src[lead..].find(|c: char| !is_js_dot(c)).unwrap_or(src.len() - lead);
    if src.as_bytes().get(header_end) != Some(&b'\n') {
        return None;
    }
    let header = &src[lead..header_end];
    let delim_start = header_end + 1;
    let d = DELIMITER.captures(&src[delim_start..])?;
    let group = d.get(1)?;
    let align = &src[delim_start + group.start()..delim_start + group.end()];
    let after_delim = delim_start + d.get(0)?.end();
    if !src[..after_delim].ends_with('\n') {
        // `$` alternative: the delimiter row is the last line.
        return Some(TableMatch { raw_len: after_delim, header, align, rows: None });
    }
    let rows_start = after_delim;
    let mut pos = rows_start;
    while pos < src.len() {
        if exec(&TABLE_ROW_STOP, &src[pos..]).is_some() {
            break;
        }
        let line_len = src[pos..].find(|c: char| !is_js_dot(c)).unwrap_or(src.len() - pos);
        let end = pos + line_len;
        match src.as_bytes().get(end) {
            None => {
                pos = end;
                break;
            }
            Some(b'\n') => pos = end + 1,
            Some(_) => break,
        }
    }
    let rows = &src[rows_start..pos];
    let raw_len = pos + src[pos..].bytes().take_while(|&b| b == b'\n').count();
    Some(TableMatch { raw_len, header, align, rows: Some(rows) })
}

/// The table rows' negative look-ahead, as a rule of its own.
static TABLE_ROW_STOP: LazyLock<fancy_regex::Regex> = LazyLock::new(rules::table_row_stop);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marked::rules::group;

    const SAMPLES: &[&str] = &[
        "```js\nlet a = 1;\n```\n\nnext", "```\n```", "```", "``` x`\n", "~~~ruby\nputs 1\n~~~~\n", "````\na\n```\nb\n````",
        "   ```\n x\n   ```  \n", "    ```\nx\n```", "```\nno close\nat all", "```\nno close\n", "~~~\n~~~x\n~~~",
        "```js title=\"a\"\n\n\n```\n", "```\n```\n```\n", "~~\n", "``` \n `` \n```",
        "| a | b |\n|---|---|\n| 1 | 2 |\n\nafter", "a | b\n--|--\n1\n1 | 2 | 3", "| h1 | h2 |\n| --- | --- |",
        "| a |\n|:-:|\n| x |\n# heading", "| a |\n| - |\n| x |\n> quote\n", "| a |\n|-|\n|x|\n    code\n",
        "| a |\n|-|\n- list\n", "| a |\n|-|\n1. one\n", "| a |\n|-|\n<div>\n", "| a |\n|-|\nrow\u{2028}x\nmore",
        "| a |\n|-|\n```\ncode\n```\n", "| a |\n|-|\n---\n", "a\n- b\n", "  |a|\n|-|  \nx\n\n\n\ny", "| a |\n|-| x\n",
    ];

    #[test]
    fn newline_and_paragraph_match_the_rules() {
        let extra = [
            "a\nb\n\nc", "a\n# h", "a\n#h", "a\n> q", "a\n- l", "a\n2. x", "a\n1. x", "a\n```\nx", "a\n***",
            "a\n<div>", "a\n<span>", "a\n   \nb", "a\n   ", "a\n| x |\n|---|", "a\n\tb", " \n\t\n x", "\n\n\n", "   ",
            "a\n    code", "a\n===", "a\n---\nb", "a\n~~~\n", "a\n<!-- c -->",
        ];
        for s in SAMPLES.iter().copied().chain(extra) {
            for start in s.char_indices().map(|(i, _)| i) {
                let src = &s[start..];
                let nl = exec(&rules::NEWLINE, src).map_or(0, |c| group(&c, 0).len());
                assert_eq!(newline(src), nl, "newline {src:?}");
                let para = exec(&rules::PARAGRAPH, src).map(|c| group(&c, 0).len());
                assert_eq!(paragraph(src), para, "paragraph {src:?}");
            }
        }
    }

    #[test]
    fn fences_match_the_rule() {
        for s in SAMPLES {
            let want = exec(&rules::FENCES, s).map(|c| Fence {
                raw_len: group(&c, 0).len(),
                info: group(&c, 2),
                body: c.get(3).map(|m| m.as_str()),
            });
            assert_eq!(fences(s), want, "{s:?}");
        }
    }

    #[test]
    fn table_matches_the_rule() {
        for s in SAMPLES {
            let want = exec(&rules::TABLE, s).map(|c| TableMatch {
                raw_len: group(&c, 0).len(),
                header: group(&c, 1),
                align: group(&c, 2),
                rows: c.get(3).map(|m| m.as_str()),
            });
            assert_eq!(table(s), want, "{s:?}");
        }
    }
}
