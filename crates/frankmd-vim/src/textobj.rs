//! Text objects: `iw aw iW aW i" a" i' a' i` a` i( a( i[ a[ i{ a{ i< a< ip ap`.

use crate::region::Region;
use crate::text::{
    CharClass, char_at, class, is_empty_line, line_count, line_end, line_index, line_start,
    line_start_of, next_pos, prev_pos,
};

/// A text object selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextObject {
    Word {
        big: bool,
        around: bool,
    },
    Quote {
        ch: char,
        around: bool,
    },
    Bracket {
        open: char,
        close: char,
        around: bool,
    },
    Paragraph {
        around: bool,
    },
}

impl TextObject {
    /// The object for the key after `i`/`a`.
    pub(crate) fn from_key(c: char, around: bool) -> Option<Self> {
        Some(match c {
            'w' => Self::Word { big: false, around },
            'W' => Self::Word { big: true, around },
            '"' | '\'' | '`' => Self::Quote { ch: c, around },
            '(' | ')' | 'b' => Self::Bracket {
                open: '(',
                close: ')',
                around,
            },
            '[' | ']' => Self::Bracket {
                open: '[',
                close: ']',
                around,
            },
            '{' | '}' | 'B' => Self::Bracket {
                open: '{',
                close: '}',
                around,
            },
            '<' | '>' => Self::Bracket {
                open: '<',
                close: '>',
                around,
            },
            'p' => Self::Paragraph { around },
            _ => return None,
        })
    }

    /// The region of this object around `pos`.
    pub(crate) fn resolve(self, t: &str, pos: usize) -> Option<Region> {
        match self {
            Self::Word { big, around } => word(t, pos, big, around).map(Region::Chars),
            Self::Quote { ch, around } => quote(t, pos, ch, around).map(Region::Chars),
            Self::Bracket {
                open,
                close,
                around,
            } => bracket(t, pos, open, close, around).map(Region::Chars),
            Self::Paragraph { around } => {
                let (first, last) = paragraph(t, pos, around);
                Some(Region::Lines { first, last })
            }
        }
    }
}

/// Extent of the run of chars of the same class as the one at `pos`, within the line.
fn run(t: &str, pos: usize, big: bool) -> (usize, usize) {
    let ls = line_start(t, pos);
    let le = line_end(t, pos);
    let Some(c) = char_at(t, pos).filter(|_| pos < le) else {
        return (pos, pos);
    };
    let cls = class(c, big);
    let mut start = pos;
    while start > ls {
        let p = prev_pos(t, start);
        if char_at(t, p).map(|c| class(c, big)) != Some(cls) {
            break;
        }
        start = p;
    }
    let mut end = pos;
    while end < le && char_at(t, end).map(|c| class(c, big)) == Some(cls) {
        end = next_pos(t, end);
    }
    (start, end)
}

fn word(t: &str, pos: usize, big: bool, around: bool) -> Option<std::ops::Range<usize>> {
    let le = line_end(t, pos);
    if pos >= le {
        return None;
    }
    let (start, end) = run(t, pos, big);
    if !around {
        return Some(start..end);
    }
    let on_space = char_at(t, pos).map(|c| class(c, big)) == Some(CharClass::Space);
    if on_space {
        // Spaces plus the following word.
        let (_, wend) = if end < le {
            run(t, end, big)
        } else {
            (end, end)
        };
        return Some(start..wend);
    }
    if end < le && char_at(t, end).map(|c| class(c, big)) == Some(CharClass::Space) {
        let (_, send) = run(t, end, big);
        return Some(start..send);
    }
    // No trailing blanks: take the leading ones instead.
    let ls = line_start(t, pos);
    if start > ls {
        let p = prev_pos(t, start);
        if char_at(t, p).map(|c| class(c, big)) == Some(CharClass::Space) {
            let (sstart, _) = run(t, p, big);
            return Some(sstart..end);
        }
    }
    Some(start..end)
}

fn quote(t: &str, pos: usize, ch: char, around: bool) -> Option<std::ops::Range<usize>> {
    let ls = line_start(t, pos);
    let le = line_end(t, pos);
    let mut quotes = Vec::new();
    let mut escaped = false;
    for (i, c) in t[ls..le].char_indices() {
        if c == ch && !escaped {
            quotes.push(ls + i);
        }
        escaped = c == '\\' && !escaped;
    }
    let pairs = quotes.as_chunks::<2>().0;
    let &[open, close] = pairs
        .iter()
        .find(|p| p[0] <= pos && pos <= p[1])
        .or_else(|| pairs.iter().find(|p| p[0] > pos))?;
    if !around {
        return Some(open + ch.len_utf8()..close);
    }
    let mut end = close + ch.len_utf8();
    let after = t[end..le].len() - t[end..le].trim_start_matches([' ', '\t']).len();
    if after > 0 {
        end += after;
        return Some(open..end);
    }
    let before = t[ls..open].len() - t[ls..open].trim_end_matches([' ', '\t']).len();
    Some(open - before..end)
}

fn bracket(
    t: &str,
    pos: usize,
    open: char,
    close: char,
    around: bool,
) -> Option<std::ops::Range<usize>> {
    // Find the unmatched opener at or before `pos` (a closer under the caret belongs to its pair).
    let mut depth = 0usize;
    let mut start = None;
    let upto = if char_at(t, pos) == Some(close) {
        pos
    } else {
        next_pos(t, pos)
    };
    for (i, c) in t[..upto].char_indices().rev() {
        if c == close {
            depth += 1;
        } else if c == open {
            if depth == 0 {
                start = Some(i);
                break;
            }
            depth -= 1;
        }
    }
    let start = start?;
    let mut depth = 0usize;
    let mut end = None;
    for (i, c) in t[start..].char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                end = Some(start + i);
                break;
            }
        }
    }
    let end = end?;
    if around {
        Some(start..end + close.len_utf8())
    } else {
        Some(start + open.len_utf8()..end)
    }
}

/// `(first, last)` line of the paragraph (or blank block) around `pos`.
fn paragraph(t: &str, pos: usize, around: bool) -> (usize, usize) {
    let last_line = line_count(t) - 1;
    let blank = |l: usize| is_empty_line(t, line_start_of(t, l));
    let line = line_index(t, pos);
    let kind = blank(line);
    let mut first = line;
    while first > 0 && blank(first - 1) == kind {
        first -= 1;
    }
    let mut last = line;
    while last < last_line && blank(last + 1) == kind {
        last += 1;
    }
    if around {
        if last < last_line {
            let other = !kind;
            while last < last_line && blank(last + 1) == other {
                last += 1;
            }
        } else {
            while first > 0 && blank(first - 1) != kind {
                first -= 1;
            }
        }
    }
    (first, last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words() {
        let t = "foo bar baz";
        assert_eq!(word(t, 5, false, false), Some(4..7));
        assert_eq!(word(t, 5, false, true), Some(4..8));
        assert_eq!(word(t, 9, false, true), Some(7..11));
        assert_eq!(word(t, 3, false, true), Some(3..7));
    }

    #[test]
    fn quotes_and_brackets() {
        let t = r#"say "hi there" ok"#;
        assert_eq!(quote(t, 6, '"', false), Some(5..13));
        assert_eq!(quote(t, 0, '"', true), Some(4..15));
        let b = "f(a, (b), c)";
        assert_eq!(bracket(b, 3, '(', ')', false), Some(2..11));
        assert_eq!(bracket(b, 6, '(', ')', true), Some(5..8));
        assert_eq!(bracket(b, 11, '(', ')', false), Some(2..11));
    }

    #[test]
    fn paragraphs() {
        let t = "a\nb\n\n\nc";
        assert_eq!(paragraph(t, 0, false), (0, 1));
        assert_eq!(paragraph(t, 0, true), (0, 3));
        assert_eq!(paragraph(t, 6, true), (2, 4));
    }
}
