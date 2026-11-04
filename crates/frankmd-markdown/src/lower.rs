//! marked tokens → render IR, with true source spans and planned highlighting.

use std::sync::Arc;

use crate::embed;
use crate::highlight::{self, BlockPlan};
use crate::ir::{Block, HighlightedLine, Inline, ListItem, SafeUrl, SourceSpan};
use crate::marked::{BlockKind, BlockTok, InlineTok, ListItemTok, newlines};
use crate::parse::RenderOptions;

/// Lower block tokens; `line_offset` is added to every (1-based) line.
/// `line_map` maps lexer lines to note lines (empty = identity, see `marked::lex`).
pub(crate) fn blocks(tokens: &[BlockTok], line_map: &[u32], line_offset: u32, opts: &RenderOptions) -> Vec<Block> {
    let mut cx = Cx { slugs: Slugger::default(), line_map, offset: line_offset };
    let mut out = lower_blocks(tokens, &mut cx);
    apply_highlighting(&mut out, opts);
    out
}

/// Lowering context.
struct Cx<'a> {
    /// Heading anchors used so far.
    slugs: Slugger,
    line_map: &'a [u32],
    offset: u32,
}

impl Cx<'_> {
    fn span(&self, raw: &str, line: u32) -> SourceSpan {
        let map = |l: u32| self.line_map.get(l as usize).copied().unwrap_or(l);
        let end = line + newlines(raw.trim_end_matches('\n'));
        SourceSpan { start_line: self.offset + map(line) + 1, end_line: self.offset + map(end) + 1 }
    }
}

fn lower_blocks(tokens: &[BlockTok], cx: &mut Cx<'_>) -> Vec<Block> {
    tokens.iter().filter_map(|t| lower_block(t, cx)).collect()
}

fn lower_block(t: &BlockTok, cx: &mut Cx<'_>) -> Option<Block> {
    let span = cx.span(&t.raw, t.line);
    Some(match &t.kind {
        BlockKind::Space => return None,
        BlockKind::Code { lang, text } => {
            let lang = lang.as_deref().map(first_word).filter(|l| !l.is_empty()).map(str::to_owned);
            Block::Code { lang, lines: Arc::new(highlight::plain_lines(&code_text(text))), span }
        }
        BlockKind::Heading { depth, inl } => {
            let inlines = inlines(&inl.tokens);
            let anchor = cx.slugs.slug(&crate::html::plain_text(&inlines));
            Block::Heading { level: *depth, inlines, anchor, span }
        }
        BlockKind::Hr => Block::Rule { span },
        BlockKind::Blockquote { children, .. } => {
            Block::Quote { alert: None, children: lower_blocks(children, cx), span }
        }
        BlockKind::List { ordered, start, loose, items } => Block::List {
            ordered: ordered.then_some(*start),
            loose: *loose,
            items: items.iter().map(|it| list_item(it, cx)).collect(),
            span,
        },
        BlockKind::Html { text } => match embed::from_html_block(text) {
            Some(e) => Block::Embed { embed: e, html: text.clone(), span },
            None => Block::Html { text: text.clone(), span },
        },
        BlockKind::Table { align, header, rows } => Block::Table {
            align: align.clone(),
            header: header.iter().map(|c| inlines(&c.tokens)).collect(),
            rows: rows.iter().map(|r| r.iter().map(|c| inlines(&c.tokens)).collect()).collect(),
            span,
        },
        BlockKind::Paragraph { inl } | BlockKind::Text { inl } => Block::Paragraph { inlines: inlines(&inl.tokens), span },
        BlockKind::MathBlock { text } => Block::Math { tex: text.clone(), span },
    })
}

fn list_item(it: &ListItemTok, cx: &mut Cx<'_>) -> ListItem {
    ListItem { task: it.task, span: cx.span(&it.raw, it.line), children: lower_blocks(&it.children, cx) }
}

/// marked's renderer: `text.replace(/\n$/, '') + '\n'`.
fn code_text(text: &str) -> String {
    format!("{}\n", text.strip_suffix('\n').unwrap_or(text))
}

/// `lang.match(/^\S*/)[0]`.
fn first_word(lang: &str) -> &str {
    let end = lang.find(crate::marked::is_js_space).unwrap_or(lang.len());
    &lang[..end]
}

/// Lower inline tokens.
pub(crate) fn inlines(tokens: &[InlineTok]) -> Vec<Inline> {
    let mut out = Vec::with_capacity(tokens.len());
    for t in tokens {
        match t {
            InlineTok::Escape(s) => out.push(Inline::Text(s.clone())),
            InlineTok::Html(s) => out.push(Inline::Html(s.clone())),
            InlineTok::Link { href, title, children } => out.push(Inline::Link {
                url: safe_url(href, false),
                title: title.as_deref().map(decode_entities),
                children: inlines(children),
            }),
            InlineTok::Image { href, title, text } => out.push(Inline::Image {
                src: safe_url(href, true),
                alt: decode_entities(text),
                title: title.as_deref().map(decode_entities),
            }),
            InlineTok::Strong(c) => out.push(Inline::Strong(inlines(c))),
            InlineTok::Em(c) => out.push(Inline::Emph(inlines(c))),
            InlineTok::Del(c) => out.push(Inline::Strike(inlines(c))),
            InlineTok::Codespan(s) => out.push(Inline::Code(s.clone())),
            InlineTok::Br => out.push(Inline::HardBreak),
            InlineTok::Text { text, escaped: true, .. } => out.push(Inline::Html(text.clone())),
            InlineTok::Text { text, .. } => out.push(Inline::Text(decode_entities(text))),
            InlineTok::Superscript(s) => out.push(Inline::Superscript(s.clone())),
            InlineTok::Subscript(s) => out.push(Inline::Subscript(s.clone())),
            InlineTok::Highlight(s) => out.push(Inline::Highlight(s.clone())),
            InlineTok::Emoji { shortcode, emoji } => {
                out.push(Inline::Emoji { shortcode: shortcode.clone(), emoji: (*emoji).to_owned() });
            }
            InlineTok::Icon { name, path, view_box } => {
                out.push(Inline::Icon { name: name.clone(), path: path.clone(), view_box: view_box.clone() });
            }
            InlineTok::Wikilink { target, display } => {
                out.push(Inline::WikiLink { target: target.clone(), label: display.clone() });
            }
            InlineTok::Math(s) => out.push(Inline::Math(s.clone())),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Highlighting
// ---------------------------------------------------------------------------

/// Plan (FrankMD budgets) and run highlighting over every classed code block
/// in document order.
fn apply_highlighting(blocks: &mut [Block], opts: &RenderOptions) {
    let mut codes: Vec<(&mut Option<String>, &mut Arc<Vec<HighlightedLine>>)> = Vec::new();
    collect_code(blocks, &mut codes);
    let texts: Vec<String> = codes.iter().map(|(_, lines)| join_lines(lines)).collect();
    let listed: Vec<(&str, usize)> = codes
        .iter()
        .zip(&texts)
        .filter_map(|((lang, _), text)| lang.as_deref().map(|l| (l, text.encode_utf16().count())))
        .collect();
    let plans: Vec<BlockPlan> = highlight::plan(&listed, &opts.highlight_budget);
    let Some(highlighter) = &opts.highlighter else { return };
    let mut plans = plans.into_iter();
    for ((lang, lines), text) in codes.into_iter().zip(&texts) {
        if lang.is_none() {
            continue;
        }
        let Some(plan) = plans.next() else { break };
        if let (true, Some(language)) = (plan.highlight, plan.language)
            && let Some(hl) = highlighter.highlight(language, text)
        {
            *lines = Arc::new(hl);
        }
    }
}

fn collect_code<'a>(blocks: &'a mut [Block], out: &mut Vec<(&'a mut Option<String>, &'a mut Arc<Vec<HighlightedLine>>)>) {
    for b in blocks {
        match b {
            Block::Code { lang, lines, .. } => out.push((lang, lines)),
            Block::Quote { children, .. } => collect_code(children, out),
            Block::List { items, .. } => {
                for it in items {
                    collect_code(&mut it.children, out);
                }
            }
            _ => {}
        }
    }
}

fn join_lines(lines: &[HighlightedLine]) -> String {
    lines.iter().map(|l| format!("{}\n", l.text)).collect()
}

// ---------------------------------------------------------------------------
// URLs, entities, slugs
// ---------------------------------------------------------------------------

/// marked's `cleanUrl` (`encodeURI`, `%25` → `%`), the browser's attribute
/// entity decoding, then DOMPurify's URI allowlist (`data:` only for images).
fn safe_url(href: &str, image: bool) -> Option<SafeUrl> {
    let url = decode_entities(&encode_uri(href));
    let squeezed: String = url.chars().filter(|&c| !is_attr_whitespace(c)).collect();
    if is_allowed_uri(&squeezed) || (image && url.starts_with("data:")) { Some(SafeUrl(url)) } else { None }
}

/// DOMPurify `ATTR_WHITESPACE`.
fn is_attr_whitespace(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{20}' | '\u{A0}' | '\u{1680}' | '\u{180E}' | '\u{2000}'..='\u{2029}' | '\u{205F}' | '\u{3000}')
}

/// DOMPurify's `IS_ALLOWED_URI`: `^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|matrix):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))`, case-insensitive.
fn is_allowed_uri(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    const SCHEMES: [&str; 10] =
        ["ftp:", "ftps:", "http:", "https:", "mailto:", "tel:", "callto:", "sms:", "cid:", "xmpp:"];
    if SCHEMES.iter().any(|s| lower.starts_with(s)) || lower.starts_with("matrix:") {
        return true;
    }
    let mut chars = lower.chars();
    match chars.next() {
        None => false,
        Some(c) if !c.is_ascii_lowercase() => true,
        Some(_) => {
            let is_scheme_char = |c: char| c.is_ascii_lowercase() || matches!(c, '+' | '.' | '-');
            // `[a-z+.\-]+` then a char that is not one of those nor `:` (or the end).
            for c in lower.chars() {
                if !is_scheme_char(c) {
                    return c != ':';
                }
            }
            true
        }
    }
}

/// JS `encodeURI` (leaving `%` alone, as marked's `%25` → `%` undo does).
fn encode_uri(s: &str) -> String {
    const KEEP: &str = ";,/?:@&=+$-_.!~*'()#%";
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || KEEP.contains(c) {
            out.push(c);
        } else {
            let mut buf = [0; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// Decode the character references marked leaves unescaped (`&name;`,
/// `&#123;`, `&#x1F;`) as an HTML parser would; unknown names stay literal.
pub(crate) fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        match decode_one(rest) {
            Some((decoded, len)) => {
                out.push_str(&decoded);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// One reference at the start of `s` (which starts with `&`): `(text, consumed)`.
fn decode_one(s: &str) -> Option<(String, usize)> {
    let semi = s.find(';')?;
    let body = &s[1..semi];
    if let Some(num) = body.strip_prefix('#') {
        let (digits, radix) = match num.strip_prefix(['x', 'X']) {
            Some(h) if (1..=6).contains(&h.len()) && h.chars().all(|c| c.is_ascii_hexdigit()) => (h, 16),
            None if (1..=7).contains(&num.len()) && num.chars().all(|c| c.is_ascii_digit()) => (num, 10),
            _ => return None,
        };
        let code = u32::from_str_radix(digits, radix).ok()?;
        return Some((numeric_char(code).to_string(), semi + 1));
    }
    if body.is_empty() || !body.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let (a, b) = *markup5ever::data::NAMED_ENTITIES.get(&s[1..=semi])?;
    let mut text = String::new();
    text.extend(char::from_u32(a));
    if b != 0 {
        text.extend(char::from_u32(b));
    }
    Some((text, semi + 1))
}

/// The HTML numeric character reference rules (C1 remap, invalid → U+FFFD).
fn numeric_char(code: u32) -> char {
    if code == 0 || code > 0x10FFFF || (0xD800..=0xDFFF).contains(&code) {
        return '\u{FFFD}';
    }
    if (0x80..=0x9F).contains(&code)
        && let Some(Some((_, c))) = markup5ever::data::C1_REPLACEMENTS.get((code - 0x80) as usize).map(|r| r.map(|c| ((), c)))
    {
        return c;
    }
    char::from_u32(code).unwrap_or('\u{FFFD}')
}

/// GitHub-style heading anchors, de-duplicated within the document: the
/// first `x` is `x`, then `x-1`, `x-2`… skipping any slug already taken
/// (e.g. by a heading literally named "x 1").
#[derive(Debug, Default)]
struct Slugger {
    /// Every slug handed out.
    seen: std::collections::HashSet<String>,
    /// Per base: the next suffix to try. Suffixes below it are all taken, so
    /// each heading costs O(1) amortized instead of rescanning from 1.
    next: std::collections::HashMap<String, usize>,
}

impl Slugger {
    fn slug(&mut self, text: &str) -> String {
        let mut base = String::new();
        for c in text.trim().to_lowercase().chars() {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                base.push(c);
            } else if c.is_whitespace() {
                base.push('-');
            }
        }
        let mut slug = base.clone();
        let n = self.next.entry(base.clone()).or_insert(1);
        while self.seen.contains(&slug) {
            slug = format!("{base}-{n}");
            *n += 1;
        }
        self.seen.insert(slug.clone());
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_dedupe_like_before() {
        let mut s = Slugger::default();
        let got: Vec<String> = ["a", "a", "a 1", "a", "b", "a-2", "a"].iter().map(|t| s.slug(t)).collect();
        assert_eq!(got, ["a", "a-1", "a-1-1", "a-2", "b", "a-2-1", "a-3"]);
    }

    #[test]
    fn entities() {
        assert_eq!(decode_entities("&amp; &copy; &#169; &#x1F600; &nope; & x"), "& © © 😀 &nope; & x");
    }

    #[test]
    fn urls() {
        assert!(safe_url("javascript:alert(1)", false).is_none());
        assert!(safe_url("JaVaScRiPt:alert(1)", false).is_none());
        assert!(safe_url("data:text/html;base64,x", false).is_none());
        assert!(safe_url("data:image/png;base64,x", true).is_some());
        assert!(safe_url("/relative", false).is_some());
        assert!(safe_url("mailto:a@b.c", false).is_some());
        assert!(safe_url("folder/note.md", false).is_some());
        assert_eq!(safe_url("a b", false), Some(SafeUrl("a%20b".into())));
    }
}
