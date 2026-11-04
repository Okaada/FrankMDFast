//! Motions: where a key sequence moves the caret, and how an operator treats
//! the span (`linewise`, `inclusive`).

use std::collections::HashMap;
use std::ops::Range;

use crate::search::{self, Search};
use crate::text::{
    CharClass, at_column, char_at, class_at, column, first_non_blank, is_empty_line, line_count,
    line_end, line_index, line_start, line_start_of, next_pos, prev_pos,
};

/// `f`/`F`/`t`/`T` target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FindSpec {
    pub(crate) ch: char,
    pub(crate) forward: bool,
    pub(crate) till: bool,
}

/// Every motion the engine knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Motion {
    Left,
    Right,
    Up,
    Down,
    /// `+`, `<CR>`: first non-blank `count` lines down.
    NextLineStart,
    /// `-`: first non-blank `count` lines up.
    PrevLineStart,
    /// `_`: first non-blank `count - 1` lines down.
    CurrentLineStart,
    WordForward {
        big: bool,
    },
    WordBackward {
        big: bool,
    },
    WordEnd {
        big: bool,
    },
    WordEndBackward {
        big: bool,
    },
    /// `0`.
    LineStart,
    /// `^`.
    FirstNonBlank,
    /// `$`.
    LineEnd,
    /// `gg` (count = line number, default first).
    FirstLine,
    /// `G` (count = line number, default last).
    LastLine,
    Find(FindSpec),
    /// `;` (`reverse = false`) / `,`.
    RepeatFind {
        reverse: bool,
    },
    /// `%` (with a count: that percentage of the file).
    MatchPair,
    ParagraphForward,
    ParagraphBackward,
    ScreenTop,
    ScreenMiddle,
    ScreenBottom,
    HalfPageDown,
    HalfPageUp,
    PageDown,
    PageUp,
    /// `n` (`reverse = false`) / `N`.
    SearchNext {
        reverse: bool,
    },
    /// `` `x `` (`exact`) / `'x`.
    Mark {
        name: char,
        exact: bool,
    },
}

/// Where a motion lands and how operators treat it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) pos: usize,
    pub(crate) linewise: bool,
    pub(crate) inclusive: bool,
}

impl Target {
    fn exclusive(pos: usize) -> Self {
        Self {
            pos,
            linewise: false,
            inclusive: false,
        }
    }
    fn inclusive(pos: usize) -> Self {
        Self {
            pos,
            linewise: false,
            inclusive: true,
        }
    }
    fn line(t: &str, line: usize) -> Self {
        Self {
            pos: first_non_blank(t, line_start_of(t, line)),
            linewise: true,
            inclusive: false,
        }
    }
}

/// Everything a motion may consult.
pub(crate) struct MotionCtx<'a> {
    pub(crate) text: &'a str,
    pub(crate) pos: usize,
    pub(crate) count: Option<usize>,
    pub(crate) goal_col: Option<usize>,
    pub(crate) last_find: Option<FindSpec>,
    pub(crate) search: &'a Search,
    pub(crate) marks: &'a HashMap<char, usize>,
    pub(crate) visible: Range<usize>,
}

impl Motion {
    /// Motions that record the previous position for `''` / ``` `` ```.
    pub(crate) fn is_jump(self) -> bool {
        matches!(
            self,
            Self::FirstLine
                | Self::LastLine
                | Self::MatchPair
                | Self::ParagraphForward
                | Self::ParagraphBackward
                | Self::ScreenTop
                | Self::ScreenMiddle
                | Self::ScreenBottom
                | Self::SearchNext { .. }
                | Self::Mark { .. }
        )
    }

    /// Whether the motion keeps the desired column (`j`/`k` family).
    pub(crate) fn is_vertical(self) -> bool {
        matches!(
            self,
            Self::Up
                | Self::Down
                | Self::HalfPageDown
                | Self::HalfPageUp
                | Self::PageDown
                | Self::PageUp
        )
    }

    /// Resolve the motion. `None` = the motion fails (vim beeps, nothing happens).
    pub(crate) fn resolve(self, cx: &MotionCtx<'_>) -> Option<Target> {
        let t = cx.text;
        let pos = cx.pos;
        let n = cx.count.unwrap_or(1).max(1);
        let ls = line_start(t, pos);
        let le = line_end(t, pos);
        let cur_line = line_index(t, pos);
        let last_line = line_count(t) - 1;
        match self {
            Self::Left => {
                let mut p = pos;
                for _ in 0..n {
                    if p <= ls {
                        break;
                    }
                    p = prev_pos(t, p);
                }
                Some(Target::exclusive(p))
            }
            Self::Right => {
                let mut p = pos;
                for _ in 0..n {
                    if p >= le {
                        break;
                    }
                    p = next_pos(t, p);
                }
                Some(Target::exclusive(p))
            }
            Self::Up
            | Self::Down
            | Self::HalfPageDown
            | Self::HalfPageUp
            | Self::PageDown
            | Self::PageUp => {
                let page = cx.visible.len().max(1);
                let step = match self {
                    Self::HalfPageDown | Self::HalfPageUp => (page / 2).max(1),
                    Self::PageDown | Self::PageUp => page.saturating_sub(2).max(1),
                    _ => n,
                };
                let down = matches!(self, Self::Down | Self::HalfPageDown | Self::PageDown);
                let target = if down {
                    (cur_line + step).min(last_line)
                } else {
                    cur_line.saturating_sub(step)
                };
                if target == cur_line {
                    return None;
                }
                let col = cx.goal_col.unwrap_or_else(|| column(t, pos));
                let p = at_column(t, line_start_of(t, target), col);
                Some(Target {
                    pos: p,
                    linewise: true,
                    inclusive: false,
                })
            }
            Self::NextLineStart => {
                (cur_line < last_line).then(|| Target::line(t, (cur_line + n).min(last_line)))
            }
            Self::PrevLineStart => {
                (cur_line > 0).then(|| Target::line(t, cur_line.saturating_sub(n)))
            }
            Self::CurrentLineStart => Some(Target::line(t, (cur_line + n - 1).min(last_line))),
            Self::WordForward { big } => {
                let mut p = pos;
                for _ in 0..n {
                    p = word_forward(t, p, big);
                }
                Some(Target::exclusive(p))
            }
            Self::WordBackward { big } => {
                let mut p = pos;
                for _ in 0..n {
                    p = word_backward(t, p, big);
                }
                Some(Target::exclusive(p))
            }
            Self::WordEnd { big } => {
                let mut p = pos;
                for _ in 0..n {
                    p = word_end(t, p, big);
                }
                Some(Target::inclusive(p))
            }
            Self::WordEndBackward { big } => {
                let mut p = pos;
                for _ in 0..n {
                    p = word_end_backward(t, p, big);
                }
                Some(Target::inclusive(p))
            }
            Self::LineStart => Some(Target::exclusive(ls)),
            Self::FirstNonBlank => Some(Target::exclusive(first_non_blank(t, ls))),
            Self::LineEnd => {
                let line = (cur_line + n - 1).min(last_line);
                Some(Target::exclusive(line_end(t, line_start_of(t, line))))
            }
            Self::FirstLine => Some(Target::line(
                t,
                cx.count.map_or(0, |c| c.saturating_sub(1)).min(last_line),
            )),
            Self::LastLine => Some(Target::line(
                t,
                cx.count
                    .map_or(last_line, |c| c.saturating_sub(1))
                    .min(last_line),
            )),
            Self::Find(spec) => find_char(t, pos, spec, n, false).map(Target::inclusive),
            Self::RepeatFind { reverse } => {
                let mut spec = cx.last_find?;
                if reverse {
                    spec.forward = !spec.forward;
                }
                find_char(t, pos, spec, n, true).map(Target::inclusive)
            }
            Self::MatchPair => match cx.count {
                Some(c) => {
                    let lines = last_line + 1;
                    let line = (c.min(100) * lines).div_ceil(100).max(1) - 1;
                    Some(Target::line(t, line))
                }
                None => match_pair(t, pos).map(Target::inclusive),
            },
            Self::ParagraphForward => {
                let mut line = cur_line;
                for _ in 0..n {
                    line = paragraph_forward(t, line, last_line);
                }
                let p = if line >= last_line && !is_empty_line(t, line_start_of(t, line)) {
                    t.len()
                } else {
                    line_start_of(t, line)
                };
                Some(Target::exclusive(p))
            }
            Self::ParagraphBackward => {
                let mut line = cur_line;
                for _ in 0..n {
                    line = paragraph_backward(t, line);
                }
                Some(Target::exclusive(line_start_of(t, line)))
            }
            Self::ScreenTop | Self::ScreenMiddle | Self::ScreenBottom => {
                let first = cx.visible.start.min(last_line);
                let last = cx.visible.end.saturating_sub(1).clamp(first, last_line);
                let line = match self {
                    Self::ScreenTop => (first + n - 1).min(last),
                    Self::ScreenBottom => last.saturating_sub(n - 1).max(first),
                    _ => first + (last - first) / 2,
                };
                Some(Target::line(t, line))
            }
            Self::SearchNext { reverse } => {
                let pattern = cx.search.pattern.as_deref()?;
                let forward = cx.search.forward != reverse;
                search::find(t, pos, pattern, forward, n).map(Target::exclusive)
            }
            Self::Mark { name, exact } => {
                let key = if name == '`' { '\'' } else { name };
                let p = crate::text::clamp_boundary(t, *cx.marks.get(&key)?);
                if exact {
                    Some(Target::exclusive(p))
                } else {
                    Some(Target::line(t, line_index(t, p)))
                }
            }
        }
    }
}

/// `w` / `W` once. Stops on empty lines like vim.
pub(crate) fn word_forward(t: &str, pos: usize, big: bool) -> usize {
    let len = t.len();
    let mut p = pos;
    let start = class_at(t, p, big);
    if start != CharClass::Space {
        while p < len && class_at(t, p, big) == start {
            p = next_pos(t, p);
        }
    }
    while p < len && class_at(t, p, big) == CharClass::Space {
        if char_at(t, p) == Some('\n') && char_at(t, p + 1) == Some('\n') {
            return p + 1;
        }
        p = next_pos(t, p);
    }
    p
}

/// `b` / `B` once.
pub(crate) fn word_backward(t: &str, pos: usize, big: bool) -> usize {
    let mut p = pos;
    if p == 0 {
        return 0;
    }
    p = prev_pos(t, p);
    while p > 0 && class_at(t, p, big) == CharClass::Space {
        // An empty line is a word of its own.
        if char_at(t, p) == Some('\n') && p > 0 && char_at(t, p - 1) == Some('\n') {
            return p;
        }
        p = prev_pos(t, p);
    }
    let cls = class_at(t, p, big);
    while p > 0 {
        let q = prev_pos(t, p);
        if class_at(t, q, big) != cls {
            break;
        }
        p = q;
    }
    p
}

/// `e` / `E` once.
pub(crate) fn word_end(t: &str, pos: usize, big: bool) -> usize {
    let len = t.len();
    let mut p = next_pos(t, pos);
    while p < len && class_at(t, p, big) == CharClass::Space {
        p = next_pos(t, p);
    }
    if p >= len {
        return prev_pos(t, len).max(pos.min(len));
    }
    let cls = class_at(t, p, big);
    loop {
        let q = next_pos(t, p);
        if q >= len || class_at(t, q, big) != cls {
            return p;
        }
        p = q;
    }
}

/// `ge` / `gE` once.
pub(crate) fn word_end_backward(t: &str, pos: usize, big: bool) -> usize {
    let mut p = pos;
    let cls = class_at(t, p, big);
    // Leave the current word.
    if cls != CharClass::Space {
        while p > 0 && class_at(t, p, big) == cls {
            p = prev_pos(t, p);
        }
    }
    while p > 0 && class_at(t, p, big) == CharClass::Space {
        p = prev_pos(t, p);
    }
    p
}

/// `f`/`t`/`F`/`T` `count` times within the line. `repeat` skips the adjacent
/// match a `t`/`T` would otherwise stick to (`;` after `t`).
fn find_char(t: &str, pos: usize, spec: FindSpec, count: usize, repeat: bool) -> Option<usize> {
    let ls = line_start(t, pos);
    let le = line_end(t, pos);
    let mut p = pos;
    if spec.till && repeat {
        p = if spec.forward {
            next_pos(t, p).min(le)
        } else {
            prev_pos(t, p).max(ls)
        };
    }
    for _ in 0..count {
        p = if spec.forward {
            let from = next_pos(t, p).min(le);
            from + t[from..le].find(spec.ch)?
        } else {
            t[ls..p].rfind(spec.ch).map(|i| ls + i)?
        };
    }
    if spec.till {
        p = if spec.forward {
            prev_pos(t, p)
        } else {
            next_pos(t, p)
        };
    }
    Some(p)
}

/// `%`: the bracket at/after the caret on its line and its partner.
fn match_pair(t: &str, pos: usize) -> Option<usize> {
    const PAIRS: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];
    let le = line_end(t, pos);
    let (start, ch) = t[pos..le]
        .char_indices()
        .map(|(i, c)| (pos + i, c))
        .find(|(_, c)| "()[]{}".contains(*c))?;
    let (open, close, forward) = PAIRS.iter().find_map(|&(o, c)| {
        if ch == o {
            Some((o, c, true))
        } else if ch == c {
            Some((o, c, false))
        } else {
            None
        }
    })?;
    let mut depth = 0usize;
    if forward {
        for (i, c) in t[start..].char_indices() {
            if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 {
                    return Some(start + i);
                }
            }
        }
    } else {
        for (i, c) in t[..=start].char_indices().rev() {
            if c == close {
                depth += 1;
            } else if c == open {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

/// Line of the next paragraph boundary (`}`), skipping blank lines first.
fn paragraph_forward(t: &str, mut line: usize, last_line: usize) -> usize {
    let empty = |l: usize| is_empty_line(t, line_start_of(t, l));
    while line < last_line && empty(line) {
        line += 1;
    }
    while line < last_line && !empty(line) {
        line += 1;
    }
    line
}

/// Line of the previous paragraph boundary (`{`).
fn paragraph_backward(t: &str, mut line: usize) -> usize {
    let empty = |l: usize| is_empty_line(t, line_start_of(t, l));
    while line > 0 && empty(line) {
        line -= 1;
    }
    while line > 0 && !empty(line) {
        line -= 1;
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words() {
        let t = "foo.bar  baz\n\nqux";
        assert_eq!(word_forward(t, 0, false), 3);
        assert_eq!(word_forward(t, 0, true), 9);
        assert_eq!(word_forward(t, 9, false), 13);
        assert_eq!(word_forward(t, 13, false), 14);
        assert_eq!(word_backward(t, 14, false), 13);
        assert_eq!(word_backward(t, 13, false), 9);
        assert_eq!(word_backward(t, 9, false), 4);
        assert_eq!(word_end(t, 0, false), 2);
        assert_eq!(word_end(t, 2, false), 3);
        assert_eq!(word_end(t, 0, true), 6);
        assert_eq!(word_end_backward(t, 9, false), 6);
    }

    #[test]
    fn find_and_match() {
        let t = "a(b(c)d)e";
        let f = FindSpec {
            ch: ')',
            forward: true,
            till: false,
        };
        assert_eq!(find_char(t, 0, f, 2, false), Some(7));
        let tt = FindSpec {
            ch: ')',
            forward: true,
            till: true,
        };
        assert_eq!(find_char(t, 0, tt, 1, false), Some(4));
        assert_eq!(match_pair(t, 0), Some(7));
        assert_eq!(match_pair(t, 7), Some(1));
        assert_eq!(match_pair(t, 4), Some(3));
    }
}
