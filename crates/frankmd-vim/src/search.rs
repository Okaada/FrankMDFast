//! `/`, `?`, `n`, `N`, `*`, `#` and the pattern side of `:s`.

use regex::{Regex, RegexBuilder};

/// Last search state (shared by `n`/`N`, `:s//` and the highlight).
#[derive(Debug, Clone, Default)]
pub(crate) struct Search {
    /// Last pattern, in vim-ish syntax as typed.
    pub(crate) pattern: Option<String>,
    /// Direction of the last `/` (true) or `?` (false).
    pub(crate) forward: bool,
    /// Whether matches should be highlighted (`:noh` clears it until the next search).
    pub(crate) highlight: bool,
}

impl Search {
    /// Remember a new search.
    pub(crate) fn set(&mut self, pattern: String, forward: bool) {
        self.pattern = Some(pattern);
        self.forward = forward;
        self.highlight = true;
    }
}

/// Compile a vim pattern: `\<`/`\>` become word boundaries, everything else is
/// regex syntax; an invalid regex is searched literally.
pub(crate) fn compile(pattern: &str) -> Option<Regex> {
    compile_with(pattern, false)
}

/// [`compile`] with optional case folding (`:s///i`).
pub(crate) fn compile_with(pattern: &str, ignore_case: bool) -> Option<Regex> {
    let translated = pattern.replace("\\<", "\\b").replace("\\>", "\\b");
    RegexBuilder::new(&translated)
        .case_insensitive(ignore_case)
        .build()
        .or_else(|_| {
            RegexBuilder::new(&regex::escape(pattern))
                .case_insensitive(ignore_case)
                .build()
        })
        .ok()
}

/// Start of the `count`-th match after (`forward`) or before `from`, wrapping
/// around the end of the text like vim's `wrapscan`.
pub(crate) fn find(
    text: &str,
    from: usize,
    pattern: &str,
    forward: bool,
    count: usize,
) -> Option<usize> {
    let re = compile(pattern)?;
    let starts: Vec<usize> = re.find_iter(text).map(|m| m.start()).collect();
    if starts.is_empty() {
        return None;
    }
    let mut pos = from;
    for _ in 0..count.max(1) {
        pos = if forward {
            starts
                .iter()
                .copied()
                .find(|&s| s > pos)
                .unwrap_or(starts[0])
        } else {
            starts
                .iter()
                .copied()
                .rev()
                .find(|&s| s < pos)
                .unwrap_or(starts[starts.len() - 1])
        };
    }
    Some(pos)
}

/// The keyword under or after the caret on its line, for `*` / `#`.
pub(crate) fn word_at(text: &str, pos: usize) -> Option<(usize, &str)> {
    let le = crate::text::line_end(text, pos);
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut start = pos;
    // Find a word char at or after the caret.
    while start < le && !text[start..].starts_with(is_word) {
        start = crate::text::next_pos(text, start);
    }
    if start >= le {
        return None;
    }
    while start > 0 {
        let p = crate::text::prev_pos(text, start);
        if !text[p..].starts_with(is_word) {
            break;
        }
        start = p;
    }
    let len = text[start..]
        .find(|c: char| !is_word(c))
        .unwrap_or(text.len() - start);
    Some((start, &text[start..start + len]))
}

/// Translate a vim `:s` replacement to `regex` syntax: `\1`..`\9` and `&` are
/// groups, `\&` / `\\` / `\/` are literal, `\r` and `\n` insert a newline.
pub(crate) fn translate_replacement(rep: &str) -> String {
    let mut out = String::with_capacity(rep.len());
    let mut chars = rep.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(d @ '0'..='9') => {
                    out.push_str("${");
                    out.push(d);
                    out.push('}');
                }
                Some('r' | 'n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('$') => out.push_str("$$"),
                Some(other) => out.push(other),
                None => out.push('\\'),
            },
            '&' => out.push_str("${0}"),
            '$' => out.push_str("$$"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_wraps() {
        let t = "foo bar foo baz";
        assert_eq!(find(t, 0, "foo", true, 1), Some(8));
        assert_eq!(find(t, 8, "foo", true, 1), Some(0));
        assert_eq!(find(t, 0, "foo", false, 1), Some(8));
        assert_eq!(find(t, 0, "nope", true, 1), None);
        assert_eq!(find(t, 0, "\\<ba", true, 2), Some(12));
    }

    #[test]
    fn replacement() {
        assert_eq!(translate_replacement(r"[\1]&\&$"), "[${1}]${0}&$$");
    }

    #[test]
    fn word_under_cursor() {
        assert_eq!(word_at("  hello, x", 0), Some((2, "hello")));
        assert_eq!(word_at("hello", 3), Some((0, "hello")));
        assert_eq!(word_at("...", 0), None);
    }
}
