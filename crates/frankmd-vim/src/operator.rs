//! Operators (`d c y > < g~ gu gU`), put (`p`/`P`) and join (`J`).
//!
//! Every change is issued as a single [`VimBuffer::replace`] over the smallest
//! differing span, so one vim change is one undo step in the editor.

use std::ops::Range;

use crate::VimBuffer;
use crate::region::{Region, map_pieces};
use crate::register::{RegContent, RegKind, Registers};
use crate::text::{
    at_column, column, first_non_blank, line_count, line_end, line_index, line_start,
    line_start_of, minimal_edit, next_pos, prev_pos,
};

/// Indent unit for `>`/`<` (CodeMirror's default `indentUnit`).
const INDENT: &str = "  ";

/// An operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operator {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
    ToggleCase,
    Lowercase,
    Uppercase,
}

impl Operator {
    /// Operator for a single key (`d c y > <`); `g`-prefixed ones come from [`Self::from_g_key`].
    pub(crate) fn from_key(c: char) -> Option<Self> {
        Some(match c {
            'd' => Self::Delete,
            'c' => Self::Change,
            'y' => Self::Yank,
            '>' => Self::Indent,
            '<' => Self::Outdent,
            _ => return None,
        })
    }

    /// Operator for the key after `g` (`~ u U`).
    pub(crate) fn from_g_key(c: char) -> Option<Self> {
        Some(match c {
            '~' => Self::ToggleCase,
            'u' => Self::Lowercase,
            'U' => Self::Uppercase,
            _ => return None,
        })
    }

    /// Whether the operator modifies text (and so is repeatable with `.`).
    pub(crate) fn is_change(self) -> bool {
        self != Self::Yank
    }
}

/// Result of running an operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpOutcome {
    /// New caret.
    pub(crate) cursor: usize,
    /// Enter insert mode (`c`).
    pub(crate) insert: bool,
}

/// Replace `t[span]` with `new`, issuing only the minimal differing edit.
pub(crate) fn rewrite(buf: &mut dyn VimBuffer, t: &str, span: Range<usize>, new: &str) {
    if let Some((range, text)) = minimal_edit(&t[span.clone()], span.start, new) {
        buf.replace(range, &text);
    }
}

/// Register content for a region.
pub(crate) fn region_content(t: &str, region: &Region) -> RegContent {
    match region {
        Region::Chars(r) => RegContent {
            text: t[r.clone()].to_owned(),
            kind: RegKind::Chars,
        },
        Region::Lines { .. } => {
            let mut text = t[region.span(t)].to_owned();
            text.push('\n');
            RegContent {
                text,
                kind: RegKind::Lines,
            }
        }
        Region::Block(pieces) => RegContent {
            text: pieces
                .iter()
                .map(|r| &t[r.clone()])
                .collect::<Vec<_>>()
                .join("\n"),
            kind: RegKind::Block,
        },
    }
}

/// Byte range a linewise delete removes (the lines plus one adjoining newline).
fn line_delete_range(t: &str, first: usize, last: usize) -> Range<usize> {
    let ls = line_start_of(t, first);
    let le = line_end(t, line_start_of(t, last));
    if le < t.len() {
        ls..le + 1
    } else if ls > 0 {
        ls - 1..le
    } else {
        ls..le
    }
}

/// Run `op` over `region`. `count` is the shift amount for `>`/`<`.
pub(crate) fn apply(
    op: Operator,
    region: &Region,
    buf: &mut dyn VimBuffer,
    regs: &mut Registers,
    reg: Option<char>,
    cursor: usize,
    count: usize,
) -> OpOutcome {
    let t = buf.text().into_owned();
    let stay = OpOutcome {
        cursor,
        insert: false,
    };
    match op {
        Operator::Yank => {
            regs.store(reg, region_content(&t, region), true);
            let cursor = match region {
                Region::Chars(r) => r.start,
                Region::Lines { first, .. } if line_index(&t, cursor) > *first => {
                    at_column(&t, line_start_of(&t, *first), column(&t, cursor))
                }
                Region::Lines { .. } => cursor,
                Region::Block(pieces) => pieces.first().map_or(cursor, |r| r.start),
            };
            OpOutcome {
                cursor,
                insert: false,
            }
        }
        Operator::Delete | Operator::Change => {
            let change = op == Operator::Change;
            regs.store(reg, region_content(&t, region), false);
            match region {
                Region::Chars(r) => {
                    if !r.is_empty() {
                        buf.replace(r.clone(), "");
                    }
                    OpOutcome {
                        cursor: r.start,
                        insert: change,
                    }
                }
                Region::Lines { .. } if change => {
                    let span = region.span(&t);
                    if !span.is_empty() {
                        buf.replace(span.clone(), "");
                    }
                    OpOutcome {
                        cursor: span.start,
                        insert: true,
                    }
                }
                Region::Lines { first, last } => {
                    buf.replace(line_delete_range(&t, *first, *last), "");
                    let new = buf.text();
                    let line = (*first).min(line_count(&new) - 1);
                    let cursor = first_non_blank(&new, line_start_of(&new, line));
                    OpOutcome {
                        cursor,
                        insert: false,
                    }
                }
                Region::Block(pieces) => {
                    let span = region.span(&t);
                    let new = map_pieces(&t, &span, pieces, |_| String::new());
                    rewrite(buf, &t, span.clone(), &new);
                    OpOutcome {
                        cursor: span.start,
                        insert: change,
                    }
                }
            }
        }
        Operator::Indent | Operator::Outdent => {
            let (first, last) = region.lines(&t);
            let lines = Region::Lines { first, last };
            let span = lines.span(&t);
            let shift = |line: &str| -> String {
                let mut s = line.to_owned();
                for _ in 0..count.max(1) {
                    s = if op == Operator::Indent {
                        if s.is_empty() {
                            s
                        } else {
                            format!("{INDENT}{s}")
                        }
                    } else if let Some(rest) = s.strip_prefix('\t') {
                        rest.to_owned()
                    } else {
                        let n = s.len() - s.trim_start_matches(' ').len();
                        s[n.min(INDENT.len())..].to_owned()
                    };
                }
                s
            };
            let new = map_pieces(&t, &span, &lines.pieces(&t), shift);
            rewrite(buf, &t, span, &new);
            let new_text = buf.text();
            OpOutcome {
                cursor: first_non_blank(&new_text, line_start_of(&new_text, first)),
                insert: false,
            }
        }
        Operator::ToggleCase | Operator::Lowercase | Operator::Uppercase => {
            let span = region.span(&t);
            let f = |s: &str| -> String {
                match op {
                    Operator::Lowercase => s.to_lowercase(),
                    Operator::Uppercase => s.to_uppercase(),
                    _ => s
                        .chars()
                        .map(|c| {
                            if c.is_uppercase() {
                                c.to_lowercase().to_string()
                            } else {
                                c.to_uppercase().to_string()
                            }
                        })
                        .collect(),
                }
            };
            let new = map_pieces(&t, &span, &region.pieces(&t), f);
            rewrite(buf, &t, span.clone(), &new);
            match region {
                Region::Lines { .. } => stay,
                _ => OpOutcome {
                    cursor: span.start,
                    insert: false,
                },
            }
        }
    }
}

/// `p` (`before = false`) / `P`. Returns the new caret, or `None` for an empty register.
pub(crate) fn put(
    buf: &mut dyn VimBuffer,
    regs: &Registers,
    reg: Option<char>,
    before: bool,
    count: usize,
    cursor: usize,
) -> Option<usize> {
    let content = regs.get(reg)?.clone();
    let t = buf.text().into_owned();
    let text = content.text.repeat(count.max(1));
    match content.kind {
        RegKind::Lines => {
            if before {
                let at = line_start(&t, cursor);
                buf.replace(at..at, &text);
                Some(first_non_blank(&buf.text(), at))
            } else {
                let le = line_end(&t, cursor);
                if le < t.len() {
                    buf.replace(le + 1..le + 1, &text);
                } else {
                    let body = text.strip_suffix('\n').unwrap_or(&text);
                    buf.replace(le..le, &format!("\n{body}"));
                }
                Some(first_non_blank(&buf.text(), le + 1))
            }
        }
        RegKind::Chars | RegKind::Block => {
            let at = if before || cursor >= line_end(&t, cursor) {
                cursor
            } else {
                next_pos(&t, cursor)
            };
            buf.replace(at..at, &text);
            if text.contains('\n') {
                Some(at)
            } else {
                Some(prev_pos(&buf.text(), at + text.len()).max(at))
            }
        }
    }
}

/// `J`: join `count` lines (at least two) starting at line `line`. Returns the caret.
pub(crate) fn join(buf: &mut dyn VimBuffer, line: usize, count: usize) -> Option<usize> {
    let t = buf.text().into_owned();
    let last_line = line_count(&t) - 1;
    if line >= last_line {
        return None;
    }
    let last = (line + count.max(2) - 1).min(last_line);
    let span = Region::Lines { first: line, last }.span(&t);
    let mut joined = String::new();
    let mut caret = span.start;
    for (i, piece) in t[span.clone()].split('\n').enumerate() {
        if i == 0 {
            joined.push_str(piece);
            continue;
        }
        let rest = piece.trim_start_matches([' ', '\t']);
        let ends_blank = joined.ends_with([' ', '\t']);
        let needs_space = !rest.is_empty() && !rest.starts_with(')') && !ends_blank;
        caret = span.start + joined.len() - usize::from(!needs_space && ends_blank);
        if needs_space {
            joined.push(' ');
        }
        joined.push_str(rest);
    }
    rewrite(buf, &t, span, &joined);
    Some(caret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::ScrollAnchor;
    use std::borrow::Cow;

    struct Buf(String, Range<usize>);
    impl VimBuffer for Buf {
        fn text(&self) -> Cow<'_, str> {
            Cow::Borrowed(&self.0)
        }
        fn selection(&self) -> Range<usize> {
            self.1.clone()
        }
        fn set_selection(&mut self, range: Range<usize>) {
            self.1 = range;
        }
        fn replace(&mut self, range: Range<usize>, text: &str) {
            self.0.replace_range(range, text);
        }
        fn transact(&mut self, f: &mut dyn FnMut(&mut dyn VimBuffer)) {
            f(self);
        }
        fn undo(&mut self) {}
        fn redo(&mut self) {}
        fn visible_lines(&self) -> Range<usize> {
            0..10
        }
        fn scroll_to_line(&mut self, _line: usize, _anchor: ScrollAnchor) {}
    }

    #[test]
    fn delete_lines_and_put() {
        let mut b = Buf("a\nb\nc".into(), 0..0);
        let mut regs = Registers::default();
        let out = apply(
            Operator::Delete,
            &Region::Lines { first: 2, last: 2 },
            &mut b,
            &mut regs,
            None,
            4,
            1,
        );
        assert_eq!(b.0, "a\nb");
        assert_eq!(out.cursor, 2);
        assert_eq!(put(&mut b, &regs, None, true, 1, 0), Some(0));
        assert_eq!(b.0, "c\na\nb");
        assert_eq!(put(&mut b, &regs, None, false, 1, 4), Some(6));
        assert_eq!(b.0, "c\na\nb\nc");
    }

    #[test]
    fn indent_and_case() {
        let mut b = Buf("x\n\ny".into(), 0..0);
        let mut regs = Registers::default();
        apply(
            Operator::Indent,
            &Region::Lines { first: 0, last: 2 },
            &mut b,
            &mut regs,
            None,
            0,
            1,
        );
        assert_eq!(b.0, "  x\n\n  y");
        apply(
            Operator::Outdent,
            &Region::Lines { first: 0, last: 0 },
            &mut b,
            &mut regs,
            None,
            0,
            1,
        );
        assert_eq!(b.0, "x\n\n  y");
        apply(
            Operator::Uppercase,
            &Region::Chars(0..1),
            &mut b,
            &mut regs,
            None,
            0,
            1,
        );
        assert_eq!(b.0, "X\n\n  y");
    }

    #[test]
    fn join_lines() {
        let mut b = Buf("a\n   b\n)".into(), 0..0);
        assert_eq!(join(&mut b, 0, 3), Some(3));
        assert_eq!(b.0, "a b)");
    }
}
