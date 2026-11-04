//! Turning the pending key sequence into a command (normal and visual mode).
//!
//! Grammar: `["x] [count] (command | operator [count] (motion | text-object | operator))`.

use crate::buffer::ScrollAnchor;
use crate::keymap::Key;
use crate::motion::{FindSpec, Motion};
use crate::operator::Operator;
use crate::register;
use crate::state::Mode;
use crate::textobj::TextObject;

/// A key reduced to what the parser cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tok {
    Char(char),
    Ctrl(char),
    Esc,
    Enter,
    Backspace,
    Tab,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    /// Anything else (function keys, Alt combos, ...).
    Other,
}

impl Tok {
    /// Normalize a key.
    pub(crate) fn of(key: &Key) -> Self {
        let mut chars = key.key.chars();
        let single = match (chars.next(), chars.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        };
        if key.alt {
            return Self::Other;
        }
        if key.ctrl {
            return match single {
                Some('[') => Self::Esc,
                Some(c) => Self::Ctrl(c),
                None => Self::Other,
            };
        }
        match key.key.as_str() {
            "escape" | "esc" => Self::Esc,
            "enter" | "return" => Self::Enter,
            "backspace" => Self::Backspace,
            "tab" => Self::Tab,
            "space" => Self::Char(' '),
            "delete" => Self::Delete,
            "left" => Self::Left,
            "right" => Self::Right,
            "up" => Self::Up,
            "down" => Self::Down,
            "home" => Self::Home,
            "end" => Self::End,
            "pageup" => Self::PageUp,
            "pagedown" => Self::PageDown,
            _ => single.map_or(Self::Other, Self::Char),
        }
    }

    /// The char an argument key (`f<c>`, `r<c>`, `m<c>`) stands for.
    fn arg_char(self) -> Option<char> {
        match self {
            Self::Char(c) => Some(c),
            Self::Enter => Some('\n'),
            Self::Tab => Some('\t'),
            _ => None,
        }
    }
}

/// Parser verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Parse<T> {
    /// Need more keys.
    Incomplete,
    /// Not a vim key at all: hand it to the app/editor.
    Unbound,
    /// A vim sequence that means nothing: swallow it.
    Invalid,
    Done(T),
}

/// What an operator applies to in normal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpTarget {
    Motion(Motion),
    Object(TextObject),
    /// Doubled operator (`dd`, `g~~`): `count` lines.
    Line,
}

/// How to enter insert mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InsertAt {
    /// `i`.
    Cursor,
    /// `a`.
    After,
    /// `I`.
    LineStart,
    /// `A`.
    LineEnd,
    /// `o`.
    OpenBelow,
    /// `O`.
    OpenAbove,
}

/// Normal-mode actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Move(Motion),
    Operate(Operator, OpTarget),
    /// `d/pat` / `d?pat`: operator waiting for a search line.
    OperateSearch(Operator, bool),
    Put {
        before: bool,
    },
    Join,
    ToggleCaseChar,
    ReplaceChar(char),
    Insert(InsertAt),
    Visual(Mode),
    /// `gv`.
    Reselect,
    ReplaceMode,
    Undo,
    Redo,
    Repeat,
    /// `:` / `/` / `?`.
    CommandLine(char),
    /// `*` (`true`) / `#`.
    SearchWord(bool),
    SetMark(char),
    Scroll(ScrollAnchor),
    Nop,
}

/// Visual-mode actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisualAction {
    Move(Motion),
    Operate(Operator),
    /// Linewise operator from visual char mode (`D X Y C S R`).
    OperateLines(Operator),
    Object(TextObject),
    SwapEnds,
    Switch(Mode),
    Exit,
    Put,
    ReplaceChar(char),
    Join,
    /// `I` (`false`) / `A` (`true`).
    Insert(bool),
    CommandLine(char),
}

/// A parsed command with its register, count, and the indices of the keys
/// that made up register and count (so `.` can replay it with a new count).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parsed<A> {
    pub(crate) register: Option<char>,
    pub(crate) count: Option<usize>,
    pub(crate) action: A,
    /// Key indices that were register prefix or count digits.
    pub(crate) prefix_keys: Vec<usize>,
}

/// Cursor over the pending tokens.
struct Reader<'a> {
    toks: &'a [Tok],
    i: usize,
    register: Option<char>,
    count: Option<usize>,
    prefix_keys: Vec<usize>,
}

/// Bail out of a parse function with `Incomplete` when keys run out.
macro_rules! next {
    ($r:expr) => {
        match $r.next() {
            Some(t) => t,
            None => return Parse::Incomplete,
        }
    };
}

impl<'a> Reader<'a> {
    fn new(toks: &'a [Tok]) -> Self {
        Self {
            toks,
            i: 0,
            register: None,
            count: None,
            prefix_keys: Vec::new(),
        }
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.i).copied();
        if t.is_some() {
            self.i += 1;
        }
        t
    }

    fn peek(&self) -> Option<Tok> {
        self.toks.get(self.i).copied()
    }

    /// Read count digits (a leading `0` is a motion, not a count); multiplies into `count`.
    fn count(&mut self) {
        let mut n: Option<usize> = None;
        while let Some(Tok::Char(c @ '0'..='9')) = self.peek() {
            if c == '0' && n.is_none() {
                break;
            }
            let d = c as usize - '0' as usize;
            n = Some(n.unwrap_or(0).saturating_mul(10).saturating_add(d));
            self.prefix_keys.push(self.i);
            self.i += 1;
        }
        if let Some(n) = n {
            self.count = Some(self.count.unwrap_or(1).saturating_mul(n));
        }
    }

    /// `"x` register prefix(es) and the first count. Errs on an invalid register or missing name.
    fn prefix(&mut self) -> Result<(), Parse<()>> {
        loop {
            self.count();
            if self.peek() != Some(Tok::Char('"')) {
                return Ok(());
            }
            self.prefix_keys.push(self.i);
            self.i += 1;
            match self.next() {
                None => return Err(Parse::Incomplete),
                Some(Tok::Char(c)) if register::is_valid_name(c) => {
                    self.prefix_keys.push(self.i - 1);
                    self.register = Some(c);
                }
                Some(_) => return Err(Parse::Invalid),
            }
        }
    }

    fn done<A>(self, action: A) -> Parse<Parsed<A>> {
        Parse::Done(Parsed {
            register: self.register,
            count: self.count,
            action,
            prefix_keys: self.prefix_keys,
        })
    }
}

/// Result of trying to read a motion.
enum MotionParse {
    Motion(Motion),
    Incomplete,
    NotMotion,
}

/// Motion starting at `first` (more keys read from `r` if needed).
fn motion(first: Tok, r: &mut Reader<'_>) -> MotionParse {
    use Motion as M;
    let arg = |r: &mut Reader<'_>| r.next().map(|t| t.arg_char());
    let m = match first {
        Tok::Char('h') | Tok::Left | Tok::Backspace | Tok::Ctrl('h') => M::Left,
        Tok::Char('l' | ' ') | Tok::Right => M::Right,
        Tok::Char('j') | Tok::Down | Tok::Ctrl('n' | 'j') => M::Down,
        Tok::Char('k') | Tok::Up | Tok::Ctrl('p') => M::Up,
        Tok::Char('+') | Tok::Enter | Tok::Ctrl('m') => M::NextLineStart,
        Tok::Char('-') => M::PrevLineStart,
        Tok::Char('_') => M::CurrentLineStart,
        Tok::Char('w') => M::WordForward { big: false },
        Tok::Char('W') => M::WordForward { big: true },
        Tok::Char('b') => M::WordBackward { big: false },
        Tok::Char('B') => M::WordBackward { big: true },
        Tok::Char('e') => M::WordEnd { big: false },
        Tok::Char('E') => M::WordEnd { big: true },
        Tok::Char('0') | Tok::Home => M::LineStart,
        Tok::Char('^') => M::FirstNonBlank,
        Tok::Char('$') | Tok::End => M::LineEnd,
        Tok::Char('G') => M::LastLine,
        Tok::Char('%') => M::MatchPair,
        Tok::Char('}') => M::ParagraphForward,
        Tok::Char('{') => M::ParagraphBackward,
        Tok::Char('H') => M::ScreenTop,
        Tok::Char('M') => M::ScreenMiddle,
        Tok::Char('L') => M::ScreenBottom,
        Tok::Char('n') => M::SearchNext { reverse: false },
        Tok::Char('N') => M::SearchNext { reverse: true },
        Tok::Char(';') => M::RepeatFind { reverse: false },
        Tok::Char(',') => M::RepeatFind { reverse: true },
        Tok::Ctrl('d') => M::HalfPageDown,
        Tok::Ctrl('u') => M::HalfPageUp,
        Tok::Ctrl('f') | Tok::PageDown => M::PageDown,
        Tok::Ctrl('b') | Tok::PageUp => M::PageUp,
        Tok::Char(c @ ('f' | 'F' | 't' | 'T')) => match arg(r) {
            None => return MotionParse::Incomplete,
            Some(None) => return MotionParse::NotMotion,
            Some(Some(ch)) => M::Find(FindSpec {
                ch,
                forward: c.is_lowercase(),
                till: c == 't' || c == 'T',
            }),
        },
        Tok::Char(c @ ('\'' | '`')) => match arg(r) {
            None => return MotionParse::Incomplete,
            Some(None) => return MotionParse::NotMotion,
            Some(Some(name)) => M::Mark {
                name,
                exact: c == '`',
            },
        },
        Tok::Char('g') => match r.peek() {
            None => return MotionParse::Incomplete,
            Some(Tok::Char('g')) => M::FirstLine,
            Some(Tok::Char('e')) => M::WordEndBackward { big: false },
            Some(Tok::Char('E')) => M::WordEndBackward { big: true },
            Some(_) => return MotionParse::NotMotion,
        },
        _ => return MotionParse::NotMotion,
    };
    if first == Tok::Char('g') {
        r.i += 1;
    }
    MotionParse::Motion(m)
}

/// Text object after `i`/`a` (only valid after an operator or in visual mode).
fn text_object(first: Tok, r: &mut Reader<'_>) -> Option<Parse<TextObject>> {
    let around = match first {
        Tok::Char('i') => false,
        Tok::Char('a') => true,
        _ => return None,
    };
    Some(match r.next() {
        None => Parse::Incomplete,
        Some(Tok::Char(c)) => TextObject::from_key(c, around).map_or(Parse::Invalid, Parse::Done),
        Some(_) => Parse::Invalid,
    })
}

/// Parse a normal-mode sequence.
pub(crate) fn normal(toks: &[Tok]) -> Parse<Parsed<Action>> {
    let mut r = Reader::new(toks);
    if let Err(e) = r.prefix() {
        return match e {
            Parse::Incomplete => Parse::Incomplete,
            _ => Parse::Invalid,
        };
    }
    let first = next!(r);
    let started = r.i > 1;
    // Operators.
    let op = match first {
        Tok::Char(c) => match Operator::from_key(c) {
            Some(op) => Some((op, vec![first])),
            None if c == 'g' => match r.peek() {
                None => return Parse::Incomplete,
                Some(Tok::Char(g)) => {
                    Operator::from_g_key(g).map(|op| (op, vec![first, Tok::Char(g)]))
                }
                Some(_) => None,
            },
            None => None,
        },
        _ => None,
    };
    if let Some((op, keys)) = op {
        if keys.len() == 2 {
            r.i += 1;
        }
        return operator_target(op, &keys, r);
    }
    match motion(first, &mut r) {
        MotionParse::Motion(m) => return r.done(Action::Move(m)),
        MotionParse::Incomplete => return Parse::Incomplete,
        MotionParse::NotMotion => {}
    }
    use Action as A;
    let action = match first {
        // Shorthands for operator + motion.
        Tok::Char('x') | Tok::Delete => {
            A::Operate(Operator::Delete, OpTarget::Motion(Motion::Right))
        }
        Tok::Char('X') => A::Operate(Operator::Delete, OpTarget::Motion(Motion::Left)),
        Tok::Char('D') => A::Operate(Operator::Delete, OpTarget::Motion(Motion::LineEnd)),
        Tok::Char('C') => A::Operate(Operator::Change, OpTarget::Motion(Motion::LineEnd)),
        Tok::Char('s') => A::Operate(Operator::Change, OpTarget::Motion(Motion::Right)),
        Tok::Char('S') => A::Operate(Operator::Change, OpTarget::Line),
        Tok::Char('Y') => A::Operate(Operator::Yank, OpTarget::Line),
        Tok::Char('p') => A::Put { before: false },
        Tok::Char('P') => A::Put { before: true },
        Tok::Char('J') => A::Join,
        Tok::Char('~') => A::ToggleCaseChar,
        Tok::Char('r') => match next!(r).arg_char() {
            Some(c) => A::ReplaceChar(c),
            None => return Parse::Invalid,
        },
        Tok::Char('i') => A::Insert(InsertAt::Cursor),
        Tok::Char('a') => A::Insert(InsertAt::After),
        Tok::Char('I') => A::Insert(InsertAt::LineStart),
        Tok::Char('A') => A::Insert(InsertAt::LineEnd),
        Tok::Char('o') => A::Insert(InsertAt::OpenBelow),
        Tok::Char('O') => A::Insert(InsertAt::OpenAbove),
        Tok::Char('v') => A::Visual(Mode::Visual),
        Tok::Char('V') => A::Visual(Mode::VisualLine),
        Tok::Ctrl('q' | 'v') => A::Visual(Mode::VisualBlock),
        Tok::Char('R') => A::ReplaceMode,
        Tok::Char('u') => A::Undo,
        Tok::Ctrl('r') => A::Redo,
        Tok::Char('.') => A::Repeat,
        Tok::Char(c @ (':' | '/' | '?')) => A::CommandLine(c),
        Tok::Char('*') => A::SearchWord(true),
        Tok::Char('#') => A::SearchWord(false),
        Tok::Char('m') => match next!(r) {
            Tok::Char(c) if c.is_ascii_alphabetic() => A::SetMark(c),
            _ => return Parse::Invalid,
        },
        Tok::Char('g') => match next!(r) {
            Tok::Char('v') => A::Reselect,
            Tok::Char('J') => A::Join,
            _ => return Parse::Invalid,
        },
        Tok::Char('z') => match next!(r) {
            Tok::Char('t') | Tok::Enter => A::Scroll(ScrollAnchor::Top),
            Tok::Char('z' | '.') => A::Scroll(ScrollAnchor::Center),
            Tok::Char('b' | '-') => A::Scroll(ScrollAnchor::Bottom),
            _ => return Parse::Invalid,
        },
        Tok::Esc | Tok::Ctrl('c') if !started => A::Nop,
        // Vim's own Ctrl bindings without an effect in this editor.
        Tok::Ctrl('e' | 'y' | 'o' | 'i' | 'g' | 'l') => A::Nop,
        Tok::Ctrl(_) | Tok::Other if !started => return Parse::Unbound,
        _ => return Parse::Invalid,
    };
    r.done(action)
}

/// After an operator: optional count, then a motion, text object, or the operator repeated.
fn operator_target(op: Operator, op_keys: &[Tok], mut r: Reader<'_>) -> Parse<Parsed<Action>> {
    r.count();
    let first = next!(r);
    // Doubled operator: `dd`, `>>`, `g~~`, `g~g~`, `guu`, `gugu`.
    if first == op_keys[op_keys.len() - 1] && (op_keys.len() == 1 || first != Tok::Char('g')) {
        return r.done(Action::Operate(op, OpTarget::Line));
    }
    if op_keys.len() == 2 && first == op_keys[0] {
        return match r.next() {
            None => Parse::Incomplete,
            Some(t) if t == op_keys[1] => r.done(Action::Operate(op, OpTarget::Line)),
            Some(_) => {
                // `gugg` & co: `g` + motion.
                r.i -= 1;
                match motion(first, &mut r) {
                    MotionParse::Motion(m) => r.done(Action::Operate(op, OpTarget::Motion(m))),
                    MotionParse::Incomplete => Parse::Incomplete,
                    MotionParse::NotMotion => Parse::Invalid,
                }
            }
        };
    }
    if let Tok::Char(c @ ('/' | '?')) = first {
        return r.done(Action::OperateSearch(op, c == '/'));
    }
    if let Some(obj) = text_object(first, &mut r) {
        return match obj {
            Parse::Done(o) => r.done(Action::Operate(op, OpTarget::Object(o))),
            Parse::Incomplete => Parse::Incomplete,
            _ => Parse::Invalid,
        };
    }
    match motion(first, &mut r) {
        MotionParse::Motion(m) => r.done(Action::Operate(op, OpTarget::Motion(m))),
        MotionParse::Incomplete => Parse::Incomplete,
        MotionParse::NotMotion => Parse::Invalid,
    }
}

/// Parse a visual-mode sequence.
pub(crate) fn visual(toks: &[Tok]) -> Parse<Parsed<VisualAction>> {
    use VisualAction as V;
    let mut r = Reader::new(toks);
    if let Err(e) = r.prefix() {
        return match e {
            Parse::Incomplete => Parse::Incomplete,
            _ => Parse::Invalid,
        };
    }
    let first = next!(r);
    let started = r.i > 1;
    if let Some(obj) = text_object(first, &mut r) {
        return match obj {
            Parse::Done(o) => r.done(V::Object(o)),
            Parse::Incomplete => Parse::Incomplete,
            _ => Parse::Invalid,
        };
    }
    if first == Tok::Char('g') {
        match r.peek() {
            None => return Parse::Incomplete,
            Some(Tok::Char(c)) => {
                if let Some(op) = Operator::from_g_key(c) {
                    r.i += 1;
                    return r.done(V::Operate(op));
                }
                if c == 'J' {
                    r.i += 1;
                    return r.done(V::Join);
                }
            }
            Some(_) => {}
        }
    }
    match motion(first, &mut r) {
        MotionParse::Motion(m) => return r.done(V::Move(m)),
        MotionParse::Incomplete => return Parse::Incomplete,
        MotionParse::NotMotion => {}
    }
    let action = match first {
        Tok::Char(c) if Operator::from_key(c).is_some() => match Operator::from_key(c) {
            Some(op) => V::Operate(op),
            None => return Parse::Invalid,
        },
        Tok::Char('x') | Tok::Delete => V::Operate(Operator::Delete),
        Tok::Char('s') => V::Operate(Operator::Change),
        Tok::Char('~') => V::Operate(Operator::ToggleCase),
        Tok::Char('u') => V::Operate(Operator::Lowercase),
        Tok::Char('U') => V::Operate(Operator::Uppercase),
        Tok::Char('D' | 'X') => V::OperateLines(Operator::Delete),
        Tok::Char('Y') => V::OperateLines(Operator::Yank),
        Tok::Char('C' | 'S' | 'R') => V::OperateLines(Operator::Change),
        Tok::Char('o' | 'O') => V::SwapEnds,
        Tok::Char('v') => V::Switch(Mode::Visual),
        Tok::Char('V') => V::Switch(Mode::VisualLine),
        Tok::Ctrl('q' | 'v') => V::Switch(Mode::VisualBlock),
        Tok::Char('p' | 'P') => V::Put,
        Tok::Char('J') => V::Join,
        Tok::Char('I') => V::Insert(false),
        Tok::Char('A') => V::Insert(true),
        Tok::Char('r') => match next!(r).arg_char() {
            Some(c) => V::ReplaceChar(c),
            None => return Parse::Invalid,
        },
        Tok::Char(c @ (':' | '/' | '?')) => V::CommandLine(c),
        Tok::Esc | Tok::Ctrl('c') => V::Exit,
        Tok::Ctrl('e' | 'y' | 'o' | 'i' | 'g' | 'l') => return Parse::Invalid,
        Tok::Ctrl(_) | Tok::Other if !started => return Parse::Unbound,
        _ => return Parse::Invalid,
    };
    r.done(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        s.chars().map(Tok::Char).collect()
    }

    fn action(s: &str) -> Option<(Option<char>, Option<usize>, Action)> {
        match normal(&toks(s)) {
            Parse::Done(p) => Some((p.register, p.count, p.action)),
            _ => None,
        }
    }

    #[test]
    fn counts_registers_operators() {
        assert_eq!(
            action("\"a2d3w"),
            Some((
                Some('a'),
                Some(6),
                Action::Operate(
                    Operator::Delete,
                    OpTarget::Motion(Motion::WordForward { big: false })
                )
            ))
        );
        assert_eq!(
            action("dd"),
            Some((
                None,
                None,
                Action::Operate(Operator::Delete, OpTarget::Line)
            ))
        );
        assert_eq!(
            action("g~~"),
            Some((
                None,
                None,
                Action::Operate(Operator::ToggleCase, OpTarget::Line)
            ))
        );
        assert_eq!(
            action("gUgU"),
            Some((
                None,
                None,
                Action::Operate(Operator::Uppercase, OpTarget::Line)
            ))
        );
        assert_eq!(
            action("gugg"),
            Some((
                None,
                None,
                Action::Operate(Operator::Lowercase, OpTarget::Motion(Motion::FirstLine))
            ))
        );
        assert_eq!(
            action("0"),
            Some((None, None, Action::Move(Motion::LineStart)))
        );
        assert_eq!(
            action("10j"),
            Some((None, Some(10), Action::Move(Motion::Down)))
        );
        assert_eq!(normal(&toks("d")), Parse::Incomplete);
        assert_eq!(normal(&toks("di")), Parse::Incomplete);
        assert_eq!(normal(&toks("dz")), Parse::Invalid);
        assert_eq!(normal(&[Tok::Ctrl('s')]), Parse::Unbound);
        match normal(&toks("\"b3x")) {
            Parse::Done(p) => assert_eq!(p.prefix_keys, vec![0, 1, 2]),
            other => panic!("{other:?}"),
        }
    }
}
