//! A port of marked 15.0.6's lexer (`Lexer.ts` + `Tokenizer.ts`) with the
//! options FrankMD uses (`gfm: true, breaks: true`) and FrankMD's eight
//! extensions plugged in exactly where marked runs them.
//!
//! Why a port and not comrak: FrankMD's extensions are marked tokenizers
//! that run *before* marked's own inline rules at every position (so
//! `==**x**==` is a literal `<mark>`, `$x_1$ and $x_2$` is two math spans and
//! not emphasis) and a block tokenizer whose `start()` hint cuts paragraphs
//! (`text $$x$$` closes the paragraph). comrak has no hook for either, and
//! the preview must render notes exactly like the original (golden:
//! `js_preview_pipeline.json`, `js_marked_extensions.json`). The port is
//! also what gives every block its true source line.
//!
//! Positions are byte offsets where marked uses UTF-16 offsets; every
//! computation is relative, so the two agree.

mod block;
mod fast;
mod inline;
pub(crate) mod rules;
pub(crate) mod token;

use std::collections::HashMap;

pub(crate) use token::{BlockKind, BlockTok, InlineTok, ListItemTok};

/// A link reference definition (`[label]: href "title"`).
#[derive(Debug, Clone)]
pub(crate) struct LinkDef {
    pub(crate) href: String,
    pub(crate) title: Option<String>,
}

/// Lexer state (`Lexer.state` plus `tokens.links`).
#[derive(Debug, Default)]
pub(crate) struct Lexer {
    links: HashMap<String, LinkDef>,
    in_link: bool,
    in_raw_block: bool,
    top: bool,
}

impl Lexer {
    fn new() -> Self {
        Self {
            top: true,
            ..Self::default()
        }
    }
}

/// `marked.lexer(src)`: block tokens with inline tokens filled in. Token
/// lines are 0-based lines of `src` after marked's `\r\n|\r` → `\n`; the
/// returned map converts them to lines of `src` counted by `\n` only (a lone
/// `\r` starts a line for marked but not for the note's line count). It is
/// empty when the two agree.
pub(crate) fn lex(src: &str) -> (Vec<BlockTok>, Vec<u32>) {
    let crlf = src.replace("\r\n", "\n");
    let mut line_map = Vec::new();
    if crlf.contains('\r') {
        let mut line = 0;
        line_map.push(0);
        for b in crlf.bytes() {
            match b {
                b'\n' => {
                    line += 1;
                    line_map.push(line);
                }
                b'\r' => line_map.push(line),
                _ => {}
            }
        }
    }
    let src = crlf.replace('\r', "\n");
    let mut lexer = Lexer::new();
    let mut tokens = Vec::new();
    lexer.block_tokens(&src, &mut tokens, false, 0);
    lexer.fill_inline(&mut tokens);
    (tokens, line_map)
}

/// `marked.parseInline` lexing (`Lexer.lexInline`): no block pass, no links.
pub(crate) fn lex_inline(src: &str) -> Vec<InlineTok> {
    Lexer::new().inline_tokens(src)
}

/// Count of `\n` in `s`, as a line delta.
pub(crate) fn newlines(s: &str) -> u32 {
    u32::try_from(s.bytes().filter(|&b| b == b'\n').count()).unwrap_or(u32::MAX)
}

/// ECMAScript `String.prototype.trim` whitespace (`\s`).
pub(crate) fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\x0B' | '\x0C' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// JS `trim()`.
pub(crate) fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// JS `trimStart()`.
pub(crate) fn js_trim_start(s: &str) -> &str {
    s.trim_start_matches(is_js_space)
}

/// JS `trimEnd()`.
pub(crate) fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(is_js_space)
}

/// `s.substring(n)` for a byte count that may exceed the length or split a char.
pub(crate) fn tail(s: &str, n: usize) -> &str {
    if n >= s.len() {
        return "";
    }
    let mut n = n;
    while !s.is_char_boundary(n) {
        n += 1;
    }
    &s[n..]
}

/// `s.split('\n', 1)[0]`.
pub(crate) fn first_line(s: &str) -> &str {
    s.split('\n').next().unwrap_or("")
}
