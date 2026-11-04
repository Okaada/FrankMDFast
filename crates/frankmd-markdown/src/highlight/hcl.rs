//! **[COMPAT]** FrankMD's hand-written Terraform/HCL stream tokenizer
//! (`terraform_hcl_stream_parser.js`), line-at-a-time like CodeMirror's
//! `StreamLanguage`. Golden: `js_terraform_hcl_stream_parser.json`.

/// Tokenizer state carried across lines (`startState()`; `copyState` = `Clone`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HclState {
    /// Inside `/* ... */`.
    pub block_comment: bool,
    /// Inside a `<<EOF` / `<<-EOF` heredoc: the closing marker.
    pub heredoc_marker: Option<String>,
    /// The previous token was a `.` (next identifier is a property).
    pub after_dot: bool,
}

/// StreamLanguage style of a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum HclStyle {
    Keyword,
    String,
    Comment,
    /// `true`/`false`/`null`.
    Atom,
    Number,
    /// `variableName.function`: identifier followed by `(`.
    Function,
    /// After a `.`, or followed by `=`.
    Property,
    Variable,
    Operator,
    Punctuation,
}

/// Block keywords (`KEYWORDS`).
const KEYWORDS: [&str; 18] = [
    "check",
    "connection",
    "data",
    "dynamic",
    "else",
    "for",
    "if",
    "import",
    "in",
    "locals",
    "module",
    "output",
    "provider",
    "provisioner",
    "removed",
    "resource",
    "terraform",
    "variable",
];

/// Multi-char operators first, then single chars (the JS alternation order).
const OPERATORS: [&str; 8] = ["...", "==", "!=", "<=", ">=", "&&", "||", "=>"];
const SINGLE_OPERATORS: &str = "=<>!&|+-*/%?:";
const PUNCTUATION: &str = "{}[](),";

/// Tokenize one non-empty line: `(token text, style)` in order, covering
/// the whole line; whitespace runs are `(text, None)`.
pub fn tokenize_line(line: &str, state: &mut HclState) -> Vec<(String, Option<HclStyle>)> {
    let mut stream = Stream { s: line, pos: 0 };
    let mut out = Vec::new();
    while !stream.eol() {
        let start = stream.pos;
        let style = token(&mut stream, state);
        // CodeMirror guarantees progress; guard anyway.
        if stream.pos == start {
            stream.next();
        }
        out.push((line[start..stream.pos].to_owned(), style));
    }
    out
}

/// `blankLine(state)`: called for empty lines (resets `after_dot`).
pub fn blank_line(state: &mut HclState) {
    state.after_dot = false;
}

/// Minimal CodeMirror `StringStream` over one line (byte positions).
struct Stream<'a> {
    s: &'a str,
    pos: usize,
}

impl Stream<'_> {
    fn eol(&self) -> bool {
        self.pos >= self.s.len()
    }

    fn rest(&self) -> &str {
        &self.s[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_to_end(&mut self) {
        self.pos = self.s.len();
    }

    fn eat_while(&mut self, f: impl Fn(char) -> bool) -> usize {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if !f(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        self.pos - start
    }
}

/// JS `\s` (WhiteSpace + LineTerminator).
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// JS `.` without the `s` flag excludes line terminators.
fn is_js_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn token(stream: &mut Stream<'_>, state: &mut HclState) -> Option<HclStyle> {
    if let Some(marker) = &state.heredoc_marker {
        if stream.s.trim_matches(is_js_space) == marker {
            state.heredoc_marker = None;
        }
        stream.skip_to_end();
        return Some(HclStyle::String);
    }

    if state.block_comment {
        return Some(consume_block_comment(stream, state));
    }
    if stream.eat_while(is_js_space) > 0 {
        return None;
    }

    let rest = stream.rest();
    // `/^#.*$/` and `/^\/\/.*$/`: only when no line terminator follows.
    if (rest.starts_with('#') || rest.starts_with("//")) && !rest.chars().any(is_js_line_terminator)
    {
        stream.skip_to_end();
        return Some(HclStyle::Comment);
    }
    if rest.starts_with("/*") {
        stream.pos += 2;
        return Some(consume_block_comment(stream, state));
    }

    if let Some(len) = match_heredoc(rest) {
        let marker_start = if rest.as_bytes().get(2) == Some(&b'-') {
            3
        } else {
            2
        };
        state.heredoc_marker = Some(rest[marker_start..len].to_owned());
        stream.pos += len;
        return Some(HclStyle::String);
    }

    if stream.peek() == Some('"') {
        consume_quoted_string(stream);
        return Some(HclStyle::String);
    }

    if let Some(len) = match_atom(rest) {
        stream.pos += len;
        return Some(HclStyle::Atom);
    }
    if let Some(len) = match_number(rest) {
        stream.pos += len;
        return Some(HclStyle::Number);
    }

    if let Some(len) = match_identifier(rest) {
        let name = &rest[..len];
        let after = rest[len..].trim_start_matches(is_js_space);
        let style = if state.after_dot {
            HclStyle::Property
        } else if KEYWORDS.contains(&name) {
            HclStyle::Keyword
        } else if after.starts_with('(') {
            HclStyle::Function
        } else if after.starts_with('=') {
            HclStyle::Property
        } else {
            HclStyle::Variable
        };
        stream.pos += len;
        state.after_dot = false;
        return Some(style);
    }

    if rest.starts_with('.') {
        stream.pos += 1;
        state.after_dot = true;
        return Some(HclStyle::Operator);
    }

    if let Some(op) = OPERATORS.iter().find(|op| rest.starts_with(**op)) {
        stream.pos += op.len();
        state.after_dot = false;
        return Some(HclStyle::Operator);
    }
    if let Some(c) = stream.peek() {
        if SINGLE_OPERATORS.contains(c) {
            stream.next();
            state.after_dot = false;
            return Some(HclStyle::Operator);
        }
        if PUNCTUATION.contains(c) {
            stream.next();
            state.after_dot = false;
            return Some(HclStyle::Punctuation);
        }
    }

    state.after_dot = false;
    stream.next();
    None
}

fn consume_quoted_string(stream: &mut Stream<'_>) {
    stream.next();
    let mut escaped = false;
    while let Some(c) = stream.next() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            break;
        }
    }
}

fn consume_block_comment(stream: &mut Stream<'_>, state: &mut HclState) -> HclStyle {
    match stream.rest().find("*/") {
        None => {
            stream.skip_to_end();
            state.block_comment = true;
        }
        Some(ix) => {
            stream.pos += ix + 2;
            state.block_comment = false;
        }
    }
    HclStyle::Comment
}

/// `^[A-Za-z_][\w-]*` → byte length.
fn match_identifier(s: &str) -> Option<usize> {
    let first = s.chars().next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    let tail = s[1..]
        .chars()
        .take_while(|&c| is_word(c) || c == '-')
        .count();
    Some(1 + tail)
}

/// `^<<-?([A-Za-z_][\w-]*)` → byte length of the whole match.
fn match_heredoc(s: &str) -> Option<usize> {
    let after = s.strip_prefix("<<")?;
    let (dash, after) = match after.strip_prefix('-') {
        Some(a) => (1, a),
        None => (0, after),
    };
    match_identifier(after).map(|len| 2 + dash + len)
}

/// `^(?:true|false|null)\b`.
fn match_atom(s: &str) -> Option<usize> {
    ["true", "false", "null"].iter().find_map(|w| {
        let rest = s.strip_prefix(w)?;
        match rest.chars().next() {
            Some(c) if is_word(c) => None,
            _ => Some(w.len()),
        }
    })
}

/// Count ASCII bytes of `s` satisfying `f` from the start.
fn run(s: &[u8], f: impl Fn(u8) -> bool) -> usize {
    s.iter().take_while(|&&b| f(b)).count()
}

/// A byte predicate of the number regex.
type ByteClass = fn(u8) -> bool;

/// The number regex of the original, alternatives in order.
fn match_number(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let digit_ = |c: u8| c.is_ascii_digit() || c == b'_';
    if b.len() >= 3 && b[0] == b'0' {
        let (ok, cont): (ByteClass, ByteClass) = match b[1] {
            b'x' | b'X' => (
                |c| c.is_ascii_hexdigit(),
                |c| c.is_ascii_hexdigit() || c == b'_',
            ),
            b'b' | b'B' => (
                |c| c == b'0' || c == b'1',
                |c| c == b'0' || c == b'1' || c == b'_',
            ),
            b'o' | b'O' => (
                |c| (b'0'..=b'7').contains(&c),
                |c| (b'0'..=b'7').contains(&c) || c == b'_',
            ),
            _ => (|_| false, |_| false),
        };
        if ok(b[2]) {
            return Some(3 + run(&b[3..], cont));
        }
    }
    let mut len = if b.first().is_some_and(u8::is_ascii_digit) {
        let mut l = 1 + run(&b[1..], digit_);
        if b.get(l) == Some(&b'.') {
            l += 1 + run(&b[l + 1..], digit_);
        }
        l
    } else if b.first() == Some(&b'.') && b.get(1).is_some_and(u8::is_ascii_digit) {
        2 + run(&b[2..], digit_)
    } else {
        return None;
    };
    if matches!(b.get(len), Some(b'e' | b'E')) {
        let mut l = len + 1;
        if matches!(b.get(l), Some(b'+' | b'-')) {
            l += 1;
        }
        let exp = run(&b[l.min(b.len())..], digit_);
        if exp > 0 {
            len = l + exp;
        }
    }
    Some(len)
}
