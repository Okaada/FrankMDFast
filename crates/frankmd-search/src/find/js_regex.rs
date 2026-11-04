//! JavaScript `RegExp` source (no `u` flag, Annex B syntax) → `fancy-regex`
//! syntax, validating as V8 does.
//!
//! The translation pins down the JS meaning of constructs whose Rust
//! meaning differs: `\d`, `\w`, `\b` are ASCII-only, `.` also excludes `\r`,
//! U+2028 and U+2029, `\s` is the ECMAScript whitespace set, and Annex B's
//! lenient escapes (`\p` = `p`, `\8` = `8`, legacy octal, a lone `{` or `]`
//! as a literal) are spelled out. Literal characters are emitted as
//! `\x{..}` so no Rust metacharacter can leak through.

use std::fmt::Write as _;

/// ECMAScript `WhiteSpace` + `LineTerminator` (the `\s` class body).
const JS_SPACE: &str = r"\t\n\x{0B}\x{0C}\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";
const ASCII_WORD: &str = "0-9A-Za-z_";
const NEVER: &str = r"[^\x{0}-\x{10FFFF}]";

/// Translate `src`; `Err` carries V8's reason text (e.g. `"Nothing to repeat"`).
pub(super) fn translate(src: &str) -> Result<String, String> {
    let chars: Vec<char> = src.chars().collect();
    let (group_count, has_named) = prescan(&chars);
    let mut parser = Parser { chars, pos: 0, out: String::new(), group_count, has_named };
    parser.disjunction()?;
    if parser.pos < parser.chars.len() {
        // `disjunction` only stops early at a `)` with no open group.
        return Err("Unmatched ')'".into());
    }
    Ok(parser.out)
}

/// Count capturing groups and detect named groups (backreference resolution
/// depends on the total, which may be defined later in the pattern).
fn prescan(chars: &[char]) -> (usize, bool) {
    let (mut count, mut named, mut in_class, mut i) = (0, false, false, 0);
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            '[' => in_class = true,
            ']' => in_class = false,
            '(' if !in_class => {
                if chars.get(i + 1) != Some(&'?') {
                    count += 1;
                } else if chars.get(i + 2) == Some(&'<') && !matches!(chars.get(i + 3), Some('=' | '!')) {
                    count += 1;
                    named = true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    (count, named)
}

/// A class item: a single char, or a predefined set (`\d`, `\W`...).
enum ClassAtom {
    Char(char),
    Set(String),
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    out: String,
    group_count: usize,
    has_named: bool,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn disjunction(&mut self) -> Result<(), String> {
        loop {
            self.alternative()?;
            if self.eat('|') {
                self.out.push('|');
            } else {
                return Ok(());
            }
        }
    }

    fn alternative(&mut self) -> Result<(), String> {
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let quantifiable = self.term()?;
            if quantifiable {
                self.quantifier()?;
            }
        }
        Ok(())
    }

    /// One assertion or atom. Returns whether a quantifier may follow.
    fn term(&mut self) -> Result<bool, String> {
        let Some(c) = self.peek() else { return Ok(false) };
        self.pos += 1;
        match c {
            '^' | '$' => {
                self.out.push(c);
                Ok(false)
            }
            '\\' if matches!(self.peek(), Some('b' | 'B')) => {
                let negated = self.peek() == Some('B');
                self.pos += 1;
                self.out.push_str(&word_boundary(negated));
                Ok(false)
            }
            '(' => self.group(),
            '[' => {
                self.class()?;
                Ok(true)
            }
            '*' | '+' | '?' => Err("Nothing to repeat".into()),
            '{' => {
                self.pos -= 1;
                if self.braced_quantifier()?.is_some() {
                    return Err("Nothing to repeat".into());
                }
                self.pos += 1;
                push_literal(&mut self.out, '{');
                Ok(true)
            }
            '.' => {
                self.out.push_str(r"[^\n\r\x{2028}\x{2029}]");
                Ok(true)
            }
            '\\' => {
                self.atom_escape()?;
                Ok(true)
            }
            _ => {
                push_literal(&mut self.out, c);
                Ok(true)
            }
        }
    }

    /// After `(`.
    fn group(&mut self) -> Result<bool, String> {
        let quantifiable = if self.eat('?') {
            match self.peek() {
                Some(':') => {
                    self.pos += 1;
                    self.out.push_str("(?:");
                    true
                }
                Some('=' | '!') => {
                    let c = self.peek().unwrap_or('=');
                    self.pos += 1;
                    let _ = write!(self.out, "(?{c}");
                    true
                }
                Some('<') if matches!(self.peek_at(1), Some('=' | '!')) => {
                    let c = self.peek_at(1).unwrap_or('=');
                    self.pos += 2;
                    let _ = write!(self.out, "(?<{c}");
                    false
                }
                Some('<') => {
                    self.pos += 1;
                    let name = self.group_name()?;
                    let _ = write!(self.out, "(?P<{name}>");
                    true
                }
                _ => return Err("Invalid group".into()),
            }
        } else {
            self.out.push('(');
            true
        };
        self.disjunction()?;
        if !self.eat(')') {
            return Err("Unterminated group".into());
        }
        self.out.push(')');
        Ok(quantifiable)
    }

    /// `name>` after `(?<` or `\k<`.
    fn group_name(&mut self) -> Result<String, String> {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == '>' {
                let valid = name.chars().next().is_some_and(|f| f.is_alphabetic() || f == '_' || f == '$')
                    && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$');
                return if valid { Ok(name) } else { Err("Invalid capture group name".into()) };
            }
            name.push(c);
        }
        Err("Invalid capture group name".into())
    }

    /// `{n}`, `{n,}` or `{n,m}` at `pos`: consumes it and returns `(min, max)`;
    /// leaves `pos` alone and returns `None` when it is not a quantifier.
    fn braced_quantifier(&mut self) -> Result<Option<(u64, Option<u64>)>, String> {
        let start = self.pos;
        if !self.eat('{') {
            return Ok(None);
        }
        let min = self.digits();
        let result = match min {
            None => None,
            Some(min) if self.eat('}') => Some((min, Some(min))),
            Some(min) if self.eat(',') => {
                let max = self.digits();
                if self.eat('}') { Some((min, max)) } else { None }
            }
            Some(_) => None,
        };
        match result {
            None => {
                self.pos = start;
                Ok(None)
            }
            Some((min, Some(max))) if max < min => Err("numbers out of order in {} quantifier".into()),
            some => Ok(some),
        }
    }

    fn digits(&mut self) -> Option<u64> {
        let start = self.pos;
        let mut value: u64 = 0;
        while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
            value = value.saturating_mul(10).saturating_add(u64::from(d));
            self.pos += 1;
        }
        (self.pos > start).then_some(value)
    }

    fn quantifier(&mut self) -> Result<(), String> {
        match self.peek() {
            Some(c @ ('*' | '+' | '?')) => {
                self.pos += 1;
                self.out.push(c);
            }
            Some('{') => match self.braced_quantifier()? {
                Some((min, Some(max))) if min == max => {
                    let _ = write!(self.out, "{{{min}}}");
                }
                Some((min, Some(max))) => {
                    let _ = write!(self.out, "{{{min},{max}}}");
                }
                Some((min, None)) => {
                    let _ = write!(self.out, "{{{min},}}");
                }
                None => return Ok(()),
            },
            _ => return Ok(()),
        }
        if self.eat('?') {
            self.out.push('?');
        }
        Ok(())
    }

    /// After `\` outside a class (`\b`/`\B` are handled by `term`).
    fn atom_escape(&mut self) -> Result<(), String> {
        let Some(c) = self.peek() else { return Err(r"\ at end of pattern".into()) };
        self.pos += 1;
        match c {
            '1'..='9' => {
                self.pos -= 1;
                let start = self.pos;
                let n = self.digits().unwrap_or(0);
                if usize::try_from(n).is_ok_and(|n| n <= self.group_count) {
                    let _ = write!(self.out, r"(?:\{n})");
                } else {
                    self.pos = start;
                    let ch = self.legacy_octal_or_digit();
                    push_literal(&mut self.out, ch);
                }
            }
            'k' if self.has_named => {
                if !self.eat('<') {
                    return Err("Invalid named reference".into());
                }
                let name = self.group_name().map_err(|_| "Invalid named reference".to_owned())?;
                let _ = write!(self.out, r"\k<{name}>");
            }
            _ => match self.common_escape(c, false) {
                ClassAtom::Char(ch) => push_literal(&mut self.out, ch),
                ClassAtom::Set(set) => {
                    self.out.push('[');
                    self.out.push_str(&set);
                    self.out.push(']');
                }
            },
        }
        Ok(())
    }

    /// Annex B `\NNN` that is not a backreference: up to three octal digits
    /// (value ≤ 0o377), or `\8`/`\9` as the digit itself. `pos` is at the digit.
    fn legacy_octal_or_digit(&mut self) -> char {
        let first = self.peek().unwrap_or('0');
        self.pos += 1;
        let Some(mut value) = first.to_digit(8) else { return first };
        for _ in 0..2 {
            match self.peek().and_then(|c| c.to_digit(8)) {
                Some(d) if value * 8 + d <= 0o377 => {
                    value = value * 8 + d;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        char::from_u32(value).unwrap_or('\0')
    }

    /// Escapes valid both in and out of classes; `c` is the char after `\`.
    fn common_escape(&mut self, c: char, in_class: bool) -> ClassAtom {
        match c {
            'd' => ClassAtom::Set("0-9".into()),
            'D' => ClassAtom::Set("^0-9".into()),
            'w' => ClassAtom::Set(ASCII_WORD.into()),
            'W' => ClassAtom::Set(format!("^{ASCII_WORD}")),
            's' => ClassAtom::Set(JS_SPACE.into()),
            'S' => ClassAtom::Set(format!("^{JS_SPACE}")),
            'n' => ClassAtom::Char('\n'),
            'r' => ClassAtom::Char('\r'),
            't' => ClassAtom::Char('\t'),
            'v' => ClassAtom::Char('\u{0B}'),
            'f' => ClassAtom::Char('\u{0C}'),
            '0' if !self.peek().is_some_and(|d| d.is_ascii_digit()) => ClassAtom::Char('\0'),
            '0'..='9' => {
                self.pos -= 1;
                ClassAtom::Char(self.legacy_octal_or_digit())
            }
            'c' => match self.peek() {
                Some(l) if l.is_ascii_alphabetic() || (in_class && (l.is_ascii_digit() || l == '_')) => {
                    self.pos += 1;
                    ClassAtom::Char(char::from_u32(u32::from(l) % 32).unwrap_or('\0'))
                }
                // `\c` without a control letter is a literal backslash; the `c`
                // is parsed again as an ordinary character.
                _ => {
                    self.pos -= 1;
                    ClassAtom::Char('\\')
                }
            },
            'x' => match self.hex(2) {
                Some(v) => ClassAtom::Char(char::from_u32(v).unwrap_or('\0')),
                None => ClassAtom::Char('x'),
            },
            'u' => self.unicode_escape(),
            other => ClassAtom::Char(other),
        }
    }

    /// After `\u`: four hex digits (a surrogate pair `👋` combines;
    /// a lone surrogate can never match a Rust string), else a literal `u`.
    fn unicode_escape(&mut self) -> ClassAtom {
        let Some(unit) = self.hex(4) else { return ClassAtom::Char('u') };
        if (0xD800..0xDC00).contains(&unit) && self.peek() == Some('\\') && self.peek_at(1) == Some('u') {
            let save = self.pos;
            self.pos += 2;
            match self.hex(4) {
                Some(low) if (0xDC00..0xE000).contains(&low) => {
                    let cp = 0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00);
                    return ClassAtom::Char(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                }
                _ => self.pos = save,
            }
        }
        match char::from_u32(unit) {
            Some(ch) => ClassAtom::Char(ch),
            None => ClassAtom::Set(r"^\x{0}-\x{10FFFF}".into()),
        }
    }

    fn hex(&mut self, len: usize) -> Option<u32> {
        let digits: String = self.chars.get(self.pos..self.pos + len)?.iter().collect();
        if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(&digits, 16).ok()?;
        self.pos += len;
        Some(value)
    }

    /// After `[`.
    fn class(&mut self) -> Result<(), String> {
        let negated = self.eat('^');
        let mut body = String::new();
        loop {
            match self.peek() {
                None => return Err("Unterminated character class".into()),
                Some(']') => {
                    self.pos += 1;
                    break;
                }
                Some(_) => {}
            }
            let first = self.class_atom()?;
            let is_range = self.peek() == Some('-') && !matches!(self.peek_at(1), None | Some(']'));
            if !is_range {
                push_class_atom(&mut body, first);
                continue;
            }
            self.pos += 1;
            let second = self.class_atom()?;
            match (first, second) {
                (ClassAtom::Char(a), ClassAtom::Char(b)) => {
                    if a > b {
                        return Err("Range out of order in character class".into());
                    }
                    let _ = write!(body, r"\x{{{:X}}}-\x{{{:X}}}", u32::from(a), u32::from(b));
                }
                // Annex B: a range with a class escape is three literal items.
                (first, second) => {
                    push_class_atom(&mut body, first);
                    push_class_atom(&mut body, ClassAtom::Char('-'));
                    push_class_atom(&mut body, second);
                }
            }
        }
        match (body.is_empty(), negated) {
            (true, false) => self.out.push_str(NEVER),
            (true, true) => self.out.push_str(r"[\x{0}-\x{10FFFF}]"),
            (false, neg) => {
                let _ = write!(self.out, "[{}{body}]", if neg { "^" } else { "" });
            }
        }
        Ok(())
    }

    fn class_atom(&mut self) -> Result<ClassAtom, String> {
        let Some(c) = self.peek() else { return Err("Unterminated character class".into()) };
        self.pos += 1;
        if c != '\\' {
            return Ok(ClassAtom::Char(c));
        }
        let Some(e) = self.peek() else { return Err(r"\ at end of pattern".into()) };
        self.pos += 1;
        Ok(match e {
            'b' => ClassAtom::Char('\u{08}'),
            '-' => ClassAtom::Char('-'),
            _ => self.common_escape(e, true),
        })
    }
}

fn push_class_atom(body: &mut String, atom: ClassAtom) {
    match atom {
        ClassAtom::Char(c) => push_literal(body, c),
        ClassAtom::Set(set) => {
            body.push('[');
            body.push_str(&set);
            body.push(']');
        }
    }
}

/// Alphanumerics verbatim, everything else as `\x{..}` (safe in and out of classes).
fn push_literal(out: &mut String, c: char) {
    if c.is_alphanumeric() {
        out.push(c);
    } else {
        let _ = write!(out, r"\x{{{:X}}}", u32::from(c));
    }
}

/// ASCII word boundary (JS `\b` without `u`) built from look-around.
fn word_boundary(negated: bool) -> String {
    let w = format!("[{ASCII_WORD}]");
    if negated {
        format!("(?:(?<={w})(?={w})|(?<!{w})(?!{w}))")
    } else {
        format!("(?:(?<={w})(?!{w})|(?<!{w})(?={w}))")
    }
}
