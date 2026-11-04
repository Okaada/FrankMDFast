//! Plain-text navigation helpers. Every position is a UTF-8 byte offset that
//! lies on a char boundary; lines are separated by `\n` exactly like the
//! editor's (CodeMirror-style) line model, so `"a\n"` has two lines.

/// Snap `pos` down to the nearest char boundary inside `t`.
pub(crate) fn clamp_boundary(t: &str, pos: usize) -> usize {
    let mut p = pos.min(t.len());
    while !t.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// The char starting at `pos`.
pub(crate) fn char_at(t: &str, pos: usize) -> Option<char> {
    t.get(pos..).and_then(|s| s.chars().next())
}

/// Offset just past the char at `pos` (or `pos` at the end of the text).
pub(crate) fn next_pos(t: &str, pos: usize) -> usize {
    char_at(t, pos).map_or(pos, |c| pos + c.len_utf8())
}

/// Offset of the char before `pos` (or `pos` at the start of the text).
pub(crate) fn prev_pos(t: &str, pos: usize) -> usize {
    t.get(..pos)
        .and_then(|s| s.chars().next_back())
        .map_or(pos, |c| pos - c.len_utf8())
}

/// Start of the line containing `pos`.
pub(crate) fn line_start(t: &str, pos: usize) -> usize {
    t[..pos].rfind('\n').map_or(0, |i| i + 1)
}

/// End of the line containing `pos` (offset of its `\n`, or the text length).
pub(crate) fn line_end(t: &str, pos: usize) -> usize {
    t[pos..].find('\n').map_or(t.len(), |i| pos + i)
}

/// Zero-based line number of `pos`.
pub(crate) fn line_index(t: &str, pos: usize) -> usize {
    t.as_bytes()[..pos].iter().filter(|&&b| b == b'\n').count()
}

/// Number of lines (always at least one).
pub(crate) fn line_count(t: &str) -> usize {
    t.bytes().filter(|&b| b == b'\n').count() + 1
}

/// Start offset of zero-based `line`, clamped to the last line.
pub(crate) fn line_start_of(t: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    t.match_indices('\n')
        .nth(line - 1)
        .map_or_else(|| line_start(t, t.len()), |(i, _)| i + 1)
}

/// First non-blank char of the line starting at `ls` (the line end when the line is blank).
pub(crate) fn first_non_blank(t: &str, ls: usize) -> usize {
    let le = line_end(t, ls);
    ls + t[ls..le]
        .find(|c: char| c != ' ' && c != '\t')
        .unwrap_or(le - ls)
}

/// Column of `pos` in chars from its line start.
pub(crate) fn column(t: &str, pos: usize) -> usize {
    t[line_start(t, pos)..pos].chars().count()
}

/// Offset of char column `col` on the line starting at `ls`, clamped to the line end.
pub(crate) fn at_column(t: &str, ls: usize, col: usize) -> usize {
    let le = line_end(t, ls);
    t[ls..le]
        .char_indices()
        .nth(col)
        .map_or(le, |(i, _)| ls + i)
}

/// A valid normal-mode caret: on a char, never on the `\n` of a non-empty line.
pub(crate) fn clamp_normal(t: &str, pos: usize) -> usize {
    let p = clamp_boundary(t, pos);
    let ls = line_start(t, p);
    if p == line_end(t, p) && p > ls {
        prev_pos(t, p)
    } else {
        p
    }
}

/// Whether the line starting at `ls` is empty.
pub(crate) fn is_empty_line(t: &str, ls: usize) -> bool {
    line_end(t, ls) == ls
}

/// Leading whitespace of the line starting at `ls`.
pub(crate) fn indentation(t: &str, ls: usize) -> &str {
    &t[ls..first_non_blank(t, ls)]
}

/// Vim character classes used by word motions and `iw`/`aw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharClass {
    /// Whitespace, including newlines.
    Space,
    /// Punctuation (only distinguished for small words).
    Punct,
    /// Letters, digits, `_`.
    Word,
}

/// Classify `c`; `big` = WORD semantics (everything non-blank is one class).
pub(crate) fn class(c: char, big: bool) -> CharClass {
    if c.is_whitespace() {
        CharClass::Space
    } else if big || c.is_alphanumeric() || c == '_' {
        CharClass::Word
    } else {
        CharClass::Punct
    }
}

/// Class of the char at `pos` (`Space` past the end).
pub(crate) fn class_at(t: &str, pos: usize, big: bool) -> CharClass {
    char_at(t, pos).map_or(CharClass::Space, |c| class(c, big))
}

/// Replace `range` with `new` as one edit, touching only the part that differs
/// (keeps the editor's undo step and cursor mapping small).
pub(crate) fn minimal_edit(
    old: &str,
    start: usize,
    new: &str,
) -> Option<(std::ops::Range<usize>, String)> {
    if old == new {
        return None;
    }
    let mut prefix = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let max_suffix = old.len().min(new.len()) - prefix;
    let mut suffix = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix) {
        suffix -= 1;
    }
    Some((
        start + prefix..start + old.len() - suffix,
        new[prefix..new.len() - suffix].to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines() {
        let t = "ab\n\ncd";
        assert_eq!(line_count(t), 3);
        assert_eq!(line_start_of(t, 2), 4);
        assert_eq!(line_start_of(t, 9), 4);
        assert_eq!(line_end(t, 0), 2);
        assert_eq!(line_index(t, 5), 2);
        assert_eq!(clamp_normal(t, 2), 1);
        assert_eq!(clamp_normal(t, 3), 3);
    }

    #[test]
    fn minimal_edit_trims_common_parts() {
        assert_eq!(
            minimal_edit("hello world", 10, "hello there world"),
            Some((16..16, "there ".into()))
        );
        assert_eq!(minimal_edit("aé", 0, "aè"), Some((1..3, "è".into())));
        assert_eq!(minimal_edit("x", 0, "x"), None);
    }
}
