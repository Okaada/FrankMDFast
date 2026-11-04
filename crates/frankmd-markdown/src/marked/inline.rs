//! Inline lexer: `Lexer.inlineTokens` and the inline tokenizers of
//! `Tokenizer.ts`, with FrankMD's inline extensions tried first.

use super::rules::{self, exec, exec_from, group, has, search};
use super::token::{BlockKind, BlockTok, InlineTok};
use super::{Lexer, LinkDef, js_trim, tail};
use crate::parse::ext::{self, Extension};

/// FrankMD's inline extensions in the order marked tries them (`marked.use`
/// `unshift`s each tokenizer, so the last registered runs first).
const INLINE_EXTENSIONS: [Extension; 7] = [
    Extension::MathInline,
    Extension::Wikilink,
    Extension::Icon,
    Extension::Emoji,
    Extension::Highlight,
    Extension::Subscript,
    Extension::Superscript,
];

impl Lexer {
    /// Lex the queued inline text of every block, in document order.
    pub(super) fn fill_inline(&mut self, tokens: &mut [BlockTok]) {
        for t in tokens {
            match &mut t.kind {
                BlockKind::Heading { inl, .. } | BlockKind::Paragraph { inl } | BlockKind::Text { inl } => {
                    inl.tokens = self.inline_tokens(&inl.text);
                }
                BlockKind::Table { header, rows, .. } => {
                    for c in header.iter_mut().chain(rows.iter_mut().flatten()) {
                        c.tokens = self.inline_tokens(&c.text);
                    }
                }
                BlockKind::Blockquote { children, .. } => self.fill_inline(children),
                BlockKind::List { items, .. } => {
                    for it in items {
                        self.fill_inline(&mut it.children);
                    }
                }
                BlockKind::Space
                | BlockKind::Code { .. }
                | BlockKind::Hr
                | BlockKind::Html { .. }
                | BlockKind::MathBlock { .. } => {}
            }
        }
    }

    /// `inlineTokens(src)`.
    pub(super) fn inline_tokens(&mut self, src: &str) -> Vec<InlineTok> {
        let masked = self.mask(src);
        let mut tokens: Vec<InlineTok> = Vec::new();
        let mut rest = src;
        let mut keep_prev_char = false;
        let mut prev_char: Option<char> = None;
        while !rest.is_empty() {
            if !keep_prev_char {
                prev_char = None;
            }
            keep_prev_char = false;
            let masked_rest = &masked[masked.len() - rest.len()..];
            let Some((len, tok)) = self.inline_step(rest, masked_rest, prev_char, &mut keep_prev_char) else {
                break;
            };
            if let InlineTok::Text { raw, .. } = &tok
                && keep_prev_char
                && let Some(c) = raw.chars().last()
                && c != '_'
            {
                prev_char = Some(c);
            }
            push_merging(&mut tokens, tok);
            rest = tail(rest, len.max(1));
        }
        tokens
    }

    /// Masked copy of `src` (same byte length): reflinks, `blockSkip`
    /// spans and escaped punctuation hidden from the emphasis delimiters.
    fn mask(&self, src: &str) -> String {
        let mut masked = src.to_owned();
        if !self.links.is_empty() {
            let mut last = 0;
            while let Some(whole) = match_range(&rules::REFLINK_SEARCH, &masked, last) {
                let text = &masked[whole.clone()];
                let label_start = text.rfind('[').map_or(0, |i| i + 1);
                let label = &text[label_start..text.len().saturating_sub(1)];
                if self.links.contains_key(label) {
                    masked = blot(&masked, whole.clone(), '[', 'a', ']');
                }
                last = if whole.is_empty() { whole.end + 1 } else { whole.end };
            }
        }
        let mut last = 0;
        while let Some(whole) = match_range(&rules::BLOCK_SKIP, &masked, last) {
            masked = blot(&masked, whole.clone(), '[', 'a', ']');
            last = if whole.is_empty() { whole.end + 1 } else { whole.end };
        }
        let mut last = 0;
        while let Some(whole) = match_range(&rules::ANY_PUNCTUATION, &masked, last) {
            masked = blot(&masked, whole.clone(), '+', '+', '+');
            last = whole.end.max(last + 1);
        }
        masked
    }

    /// One iteration of the `inlineTokens` loop: `(consumed, token)`.
    fn inline_step(
        &mut self,
        src: &str,
        masked: &str,
        prev_char: Option<char>,
        keep_prev_char: &mut bool,
    ) -> Option<(usize, InlineTok)> {
        if let Some(r) = extension(src) {
            return Some(r);
        }
        // escape
        if let Some(cap) = exec(&rules::ESCAPE, src) {
            return Some((group(&cap, 0).len(), InlineTok::Escape(group(&cap, 1).to_owned())));
        }
        // tag
        if let Some(cap) = exec(&rules::TAG, src) {
            let raw = group(&cap, 0);
            if !self.in_link && exec(&rules::START_A_TAG, raw).is_some() {
                self.in_link = true;
            } else if self.in_link && exec(&rules::END_A_TAG, raw).is_some() {
                self.in_link = false;
            }
            if !self.in_raw_block && exec(&rules::START_PRE_SCRIPT_TAG, raw).is_some() {
                self.in_raw_block = true;
            } else if self.in_raw_block && exec(&rules::END_PRE_SCRIPT_TAG, raw).is_some() {
                self.in_raw_block = false;
            }
            return Some((raw.len(), InlineTok::Html(raw.to_owned())));
        }
        // link
        if let Some(r) = self.link(src) {
            return Some(r);
        }
        // reflink, nolink
        if let Some(r) = self.reflink(src) {
            return Some(r);
        }
        // em & strong
        if let Some(r) = self.em_strong(src, masked, prev_char) {
            return Some(r);
        }
        // code
        if let Some(cap) = exec(&rules::CODESPAN, src) {
            let mut text = group(&cap, 2).replace('\n', " ");
            let has_non_space = text.bytes().any(|b| b != b' ');
            if has_non_space && text.starts_with(' ') && text.ends_with(' ') {
                text = text[1..text.len() - 1].to_owned();
            }
            return Some((group(&cap, 0).len(), InlineTok::Codespan(text)));
        }
        // br
        if let Some(cap) = exec(&rules::BR, src) {
            return Some((group(&cap, 0).len(), InlineTok::Br));
        }
        // del (gfm)
        if let Some(cap) = exec(&rules::DEL, src) {
            let children = self.inline_tokens(group(&cap, 2));
            return Some((group(&cap, 0).len(), InlineTok::Del(children)));
        }
        // autolink
        if let Some(cap) = exec(&rules::AUTOLINK, src) {
            let text = group(&cap, 1).to_owned();
            let href = if group(&cap, 2) == "@" { format!("mailto:{text}") } else { text.clone() };
            return Some((group(&cap, 0).len(), link_token(href, text)));
        }
        // url (gfm)
        if !self.in_link
            && let Some(cap) = exec(&rules::URL, src)
        {
            let (text, href) = if group(&cap, 2) == "@" {
                let text = group(&cap, 0).to_owned();
                (text.clone(), format!("mailto:{text}"))
            } else {
                let mut cur = group(&cap, 0).to_owned();
                loop {
                    let next = search(&rules::BACKPEDAL, &cur).map_or(String::new(), |m| group(&m, 0).to_owned());
                    if next == cur {
                        break;
                    }
                    cur = next;
                }
                let href = if group(&cap, 1) == "www." { format!("http://{cur}") } else { cur.clone() };
                (cur, href)
            };
            if !text.is_empty() {
                return Some((text.len(), link_token(href, text)));
            }
        }
        // text, clipped at the next extension start
        let mut cut = src;
        if let Some(first) = src.chars().next() {
            let skip = first.len_utf8();
            let next = Extension::ALL
                .iter()
                .filter(|e| e.level() == ext::Level::Inline)
                .filter_map(|e| ext::start(*e, &src[skip..]))
                .min();
            if let Some(ix) = next {
                cut = &src[..skip + ix];
            }
        }
        let cap = exec(&rules::INLINE_TEXT, cut)?;
        let raw = group(&cap, 0);
        *keep_prev_char = true;
        Some((raw.len(), InlineTok::Text { raw: raw.to_owned(), text: raw.to_owned(), escaped: self.in_raw_block }))
    }

    /// `Tokenizer.link`.
    fn link(&mut self, src: &str) -> Option<(usize, InlineTok)> {
        let cap = exec(&rules::LINK, src)?;
        let mut whole = group(&cap, 0);
        let label = group(&cap, 1);
        let mut href_src = group(&cap, 2);
        let mut title_src = group(&cap, 3);
        let trimmed_url = js_trim(href_src);
        if trimmed_url.starts_with('<') {
            if !trimmed_url.ends_with('>') {
                return None;
            }
            let inner = &trimmed_url[..trimmed_url.len() - 1];
            let slashes = inner.len() - inner.trim_end_matches('\\').len();
            if (slashes + 1).is_multiple_of(2) {
                return None;
            }
        } else if let Some(last_paren) = find_closing_bracket(href_src, b'(', b')') {
            let start = if whole.starts_with('!') { 5 } else { 4 };
            let link_len = start + label.len() + last_paren;
            href_src = &href_src[..last_paren];
            whole = js_trim(&whole[..link_len.min(whole.len())]);
            title_src = "";
        }
        let title = if title_src.is_empty() { String::new() } else { title_src[1..title_src.len() - 1].to_owned() };
        let mut href = js_trim(href_src);
        if href.starts_with('<') {
            href = if href.len() >= 2 { &href[1..href.len() - 1] } else { "" };
        }
        let href = rules::unescape_punctuation(href);
        let title = rules::unescape_punctuation(&title);
        let def = LinkDef { href, title: (!title.is_empty()).then_some(title) };
        Some((whole.len(), self.output_link(whole, label, &def)))
    }

    /// `Tokenizer.reflink` (+ `nolink`).
    fn reflink(&mut self, src: &str) -> Option<(usize, InlineTok)> {
        let cap = exec(&rules::REFLINK, src).or_else(|| exec(&rules::NOLINK, src))?;
        let whole = group(&cap, 0);
        let label = group(&cap, 1);
        let reference = if has(&cap, 2) { group(&cap, 2) } else { label };
        let key = collapse_js_space(reference).to_lowercase();
        let Some(def) = self.links.get(&key).cloned() else {
            let first = whole.chars().next().map_or_else(String::new, String::from);
            return Some((first.len(), InlineTok::Text { raw: first.clone(), text: first, escaped: false }));
        };
        Some((whole.len(), self.output_link(whole, label, &def)))
    }

    /// `outputLink(cap, link, raw)`.
    fn output_link(&mut self, whole: &str, label: &str, def: &LinkDef) -> InlineTok {
        let text = label.replace("\\[", "[").replace("\\]", "]");
        if whole.starts_with('!') {
            return InlineTok::Image { href: def.href.clone(), title: def.title.clone(), text };
        }
        self.in_link = true;
        let children = self.inline_tokens(&text);
        self.in_link = false;
        InlineTok::Link { href: def.href.clone(), title: def.title.clone(), children }
    }

    /// `Tokenizer.emStrong(src, maskedSrc, prevChar)`.
    fn em_strong(&mut self, src: &str, masked: &str, prev_char: Option<char>) -> Option<(usize, InlineTok)> {
        let m = exec(&rules::EM_STRONG_LDELIM, src)?;
        let prev = prev_char.map(String::from).unwrap_or_default();
        if has(&m, 3) && search(&rules::UNICODE_ALNUM, &prev).is_some() {
            return None;
        }
        let next_char = has(&m, 1) || has(&m, 2);
        if !(!next_char || prev.is_empty() || exec(&rules::PUNCTUATION, &prev).is_some()) {
            return None;
        }
        let m0 = group(&m, 0);
        let l_length = m0.chars().count() - 1;
        let l_bytes = m0.len() - m0.chars().last().map_or(0, char::len_utf8);
        let mut delim_total = l_length as i64;
        let mut mid_delim_total: i64 = 0;
        let end_re = if m0.starts_with('*') { &rules::EM_STRONG_RDELIM_AST } else { &rules::EM_STRONG_RDELIM_UND };
        let masked = tail(masked, l_bytes);
        let mut last_index = 0;
        while let Some(mm) = exec_from(end_re, masked, last_index) {
            let whole = mm.get(0).map_or(0..0, |g| g.range());
            last_index = if whole.is_empty() { whole.end + 1 } else { whole.end };
            let Some(r_delim) = (1..=6).find(|&i| has(&mm, i)).map(|i| group(&mm, i)) else { continue };
            let mut r_length = r_delim.chars().count() as i64;
            if has(&mm, 3) || has(&mm, 4) {
                delim_total += r_length;
                continue;
            } else if (has(&mm, 5) || has(&mm, 6)) && !l_length.is_multiple_of(3) && (l_length as i64 + r_length) % 3 == 0 {
                mid_delim_total += r_length;
                continue;
            }
            delim_total -= r_length;
            if delim_total > 0 {
                continue;
            }
            r_length = r_length.min(r_length + delim_total + mid_delim_total);
            let last_char_len = masked[whole.start..].chars().next().map_or(0, char::len_utf8);
            let end = l_bytes + whole.start + last_char_len + usize::try_from(r_length).unwrap_or(0);
            let raw = src.get(..end)?;
            let em = (l_length as i64).min(r_length) % 2 == 1;
            let cut = if em { 1 } else { 2 };
            let text = raw.get(cut..raw.len().checked_sub(cut)?)?.to_owned();
            let children = self.inline_tokens(&text);
            return Some((raw.len(), if em { InlineTok::Em(children) } else { InlineTok::Strong(children) }));
        }
        None
    }
}

/// Try FrankMD's inline extension tokenizers at the start of `src`.
fn extension(src: &str) -> Option<(usize, InlineTok)> {
    for e in INLINE_EXTENSIONS {
        let r = match e {
            Extension::MathInline => ext::scan_math_inline(src).map(|s| (s.raw_len, InlineTok::Math(s.value))),
            Extension::Wikilink => ext::scan_wikilink(src)
                .map(|s| (s.raw_len, InlineTok::Wikilink { target: s.value.target, display: s.value.display })),
            Extension::Icon => ext::scan_icon(src).map(|s| {
                let v = s.value;
                (s.raw_len, InlineTok::Icon { name: v.name, path: v.path, view_box: v.view_box })
            }),
            Extension::Emoji => ext::scan_emoji(src).map(|s| {
                let shortcode = src[1..s.raw_len - 1].to_owned();
                (s.raw_len, InlineTok::Emoji { shortcode, emoji: s.value })
            }),
            Extension::Highlight => ext::scan_highlight(src).map(|s| (s.raw_len, InlineTok::Highlight(s.value))),
            Extension::Subscript => ext::scan_subscript(src).map(|s| (s.raw_len, InlineTok::Subscript(s.value))),
            Extension::Superscript => {
                ext::scan_superscript(src).map(|s| (s.raw_len, InlineTok::Superscript(s.value)))
            }
            Extension::MathBlock => None,
        };
        if r.is_some() {
            return r;
        }
    }
    None
}

/// marked appends a text token to a preceding text token.
fn push_merging(tokens: &mut Vec<InlineTok>, tok: InlineTok) {
    if let InlineTok::Text { raw, text, .. } = &tok
        && let Some(InlineTok::Text { raw: last_raw, text: last_text, .. }) = tokens.last_mut()
    {
        last_raw.push_str(raw);
        last_text.push_str(text);
        return;
    }
    tokens.push(tok);
}

/// A link token whose only child is its text (autolinks, GFM urls).
fn link_token(href: String, text: String) -> InlineTok {
    InlineTok::Link {
        href,
        title: None,
        children: vec![InlineTok::Text { raw: text.clone(), text, escaped: false }],
    }
}

/// Byte range of the next match of a global rule from `last`.
fn match_range(re: &fancy_regex::Regex, s: &str, last: usize) -> Option<std::ops::Range<usize>> {
    exec_from(re, s, last).and_then(|m| m.get(0).map(|g| g.range()))
}

/// Replace `range` of `s` with `open` + `fill`… + `close`, keeping the byte length.
fn blot(s: &str, range: std::ops::Range<usize>, open: char, fill: char, close: char) -> String {
    let n = range.len();
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..range.start]);
    if n >= 2 {
        out.push(open);
        out.extend(std::iter::repeat_n(fill, n - 2));
        out.push(close);
    } else {
        out.extend(std::iter::repeat_n(fill, n));
    }
    out.push_str(&s[range.end..]);
    out
}

/// `findClosingBracket(str, '()')` (byte index, `None` for -1).
fn find_closing_bracket(s: &str, open: u8, close: u8) -> Option<usize> {
    let b = s.as_bytes();
    if !b.contains(&close) {
        return None;
    }
    let mut level: i64 = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            i += 1;
        } else if b[i] == open {
            level += 1;
        } else if b[i] == close {
            level -= 1;
            if level < 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// `s.replace(/\s+/g, ' ')`.
fn collapse_js_space(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    for c in s.chars() {
        if super::is_js_space(c) {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}
