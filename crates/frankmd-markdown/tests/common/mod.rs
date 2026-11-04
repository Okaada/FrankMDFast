//! Shared helpers for the frankmd-markdown conformance tests.
//!
//! - [`normalize_html`]: the documented HTML normalization used to compare
//!   `html::to_html` with marked + DOMPurify output (see
//!   `docs/testing/port-markdown-search.md` §Normalization).
//! - [`ExpectedAnchors`]: `data-source-line` annotations read from an original
//!   output, plus detection of FrankMD's sequential-slot quirk.
//! - IR walkers (plain text, math nodes, list-item lines, code blocks).
//! - Lezer `tok-*` class → [`TokenStyle`] mapping (from FrankMD's `prose.css`).
//! - UTF-16 offset conversion for JS fixtures.
#![allow(dead_code, unreachable_pub, clippy::unwrap_used, clippy::missing_panics_doc)]

use frankmd_markdown::RenderDoc;
use frankmd_markdown::highlight::{BuiltinHighlighter, CodeHighlighter, language_for_fence};
use frankmd_markdown::ir::{Block, HighlightedLine, Inline, ListItem, TokenClass, TokenStyle};
use frankmd_markdown::math::{MathError, MathStyle};
use frankmd_testkit::serde_json::Value;
use frankmd_testkit::{Case, decode_js};
use scraper::{ElementRef, Html, Node};

// ---------------------------------------------------------------------------
// Known divergences
// ---------------------------------------------------------------------------

/// Golden cases whose original output we deliberately do NOT reproduce, as
/// `(fixture, case name, reason)`. Every entry must also be listed in
/// `docs/testing/port-markdown-search.md`. The implementation phase may add
/// entries only with a written justification.
pub const KNOWN_DIVERGENCES: &[(&str, &str, &str)] = &[(
    "js_marked_extensions",
    r#"icon.renderer {"path":"\"><script>","viewBox":"0 0 1 1"}"#,
    "FrankMD interpolates icon path data into the SVG unescaped; ours escapes attribute values. \
     Identical for every real Phosphor path (trusted, no quotes/angle brackets).",
)];

/// True when `(fixture, case)` is an accepted divergence.
pub fn is_known_divergence(fixture: &str, case: &str) -> bool {
    KNOWN_DIVERGENCES.iter().any(|(f, c, _)| *f == fixture && *c == case)
}

// ---------------------------------------------------------------------------
// Fixture argument access
// ---------------------------------------------------------------------------

/// Decoded positional argument `i` of a JS case (`$undefined` → `Null`).
pub fn arg(case: &Case, i: usize) -> Value {
    decode_js(case.input.get(i).unwrap_or(&Value::Null))
}

/// Positional string argument (`null`/`undefined` → `""`, JS `if (!x) return ""`).
pub fn arg_str(case: &Case, i: usize) -> String {
    match arg(case, i) {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => panic!("{}: arg {i} is not a string: {other}", case.name),
    }
}

/// Decoded expected value.
pub fn expected(case: &Case) -> Value {
    decode_js(&case.expected)
}

// ---------------------------------------------------------------------------
// UTF-16 offsets
// ---------------------------------------------------------------------------

/// Byte offset of UTF-16 code-unit offset `u16_off` in `s`.
pub fn utf16_to_byte(s: &str, u16_off: usize) -> usize {
    let mut units = 0;
    for (b, ch) in s.char_indices() {
        if units >= u16_off {
            return b;
        }
        units += ch.len_utf16();
    }
    s.len()
}

/// UTF-16 code-unit offset of byte offset `byte` in `s`.
pub fn byte_to_utf16(s: &str, byte: usize) -> usize {
    s[..byte].encode_utf16().count()
}

/// JS `string.length`.
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

// ---------------------------------------------------------------------------
// HTML normalization
// ---------------------------------------------------------------------------

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr",
];

/// Elements around which whitespace is insignificant (block-level boxes, plus
/// `br`, after/before which leading/trailing whitespace does not render).
const BLOCKISH: &[&str] = &[
    "address", "article", "aside", "blockquote", "br", "dd", "details", "dialog", "div", "dl", "dt",
    "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header",
    "hr", "li", "main", "nav", "ol", "p", "pre", "section", "summary", "table", "tbody", "td", "tfoot",
    "th", "thead", "tr", "ul", "iframe", "video", "source", "picture",
];

fn is_blockish(name: &str) -> bool {
    BLOCKISH.contains(&name)
}

/// Canonical serialization of an HTML fragment, used on both sides of every
/// HTML comparison:
///
/// 1. parse with html5ever (fragment, `<body>` context) — so entity spelling,
///    attribute quoting, `<br>` vs `<br />`, `checked` vs `checked=""`,
///    self-closing SVG children, and implied end tags stop mattering;
/// 2. attributes sorted by name; names in `drop_attrs` removed;
/// 3. outside `<pre>`/`<code>`/`<textarea>`: whitespace runs collapse to one
///    space, whitespace-only text next to (or at the edge of) a block-level
///    element or `<br>` is dropped, and text is trimmed on the side that
///    touches such an element;
/// 4. text escaped as `& < >`, attribute values as `& "`; comments kept.
pub fn normalize_html(html: &str, drop_attrs: &[&str]) -> String {
    let frag = Html::parse_fragment(html);
    let mut out = String::new();
    write_children(frag.root_element(), drop_attrs, false, true, &mut out);
    out
}

/// [`normalize_html`] dropping `data-source-line` (line semantics are checked
/// on the IR instead, see [`ExpectedAnchors`]).
pub fn normalize_structure(html: &str) -> String {
    normalize_html(html, &["data-source-line"])
}

fn write_children(el: ElementRef<'_>, drop: &[&str], in_pre: bool, parent_blockish: bool, out: &mut String) {
    let kids: Vec<_> = el.children().collect();
    let elem_name = |i: usize| -> Option<String> {
        kids.get(i).and_then(|k| match k.value() {
            Node::Element(e) => Some(e.name().to_owned()),
            _ => None,
        })
    };
    for (i, child) in kids.iter().enumerate() {
        match child.value() {
            Node::Text(t) => {
                if in_pre {
                    out.push_str(&escape_text(t));
                    continue;
                }
                let prev_block = if i == 0 { parent_blockish } else { elem_name(i - 1).is_some_and(|n| is_blockish(&n)) };
                let next_block =
                    if i + 1 == kids.len() { parent_blockish } else { elem_name(i + 1).is_some_and(|n| is_blockish(&n)) };
                let mut s = collapse_ws(t);
                if prev_block {
                    s = s.trim_start().to_owned();
                }
                if next_block {
                    s = s.trim_end().to_owned();
                }
                if !s.is_empty() {
                    out.push_str(&escape_text(&s));
                }
            }
            Node::Comment(c) => {
                out.push_str("<!--");
                out.push_str(c);
                out.push_str("-->");
            }
            Node::Element(e) => {
                let child_el = ElementRef::wrap(*child).unwrap();
                let name = e.name();
                out.push('<');
                out.push_str(name);
                let mut attrs: Vec<(&str, &str)> = e.attrs().filter(|(k, _)| !drop.contains(k)).collect();
                attrs.sort();
                for (k, v) in attrs {
                    out.push(' ');
                    out.push_str(k);
                    out.push_str("=\"");
                    out.push_str(&v.replace('&', "&amp;").replace('"', "&quot;"));
                    out.push('"');
                }
                out.push('>');
                if VOID.contains(&name) {
                    continue;
                }
                let pre = in_pre || matches!(name, "pre" | "code" | "textarea");
                write_children(child_el, drop, pre, is_blockish(name), out);
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
            _ => {}
        }
    }
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_ws = false;
    for ch in s.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0c') {
            if !in_ws {
                out.push(' ');
            }
            in_ws = true;
        } else {
            out.push(ch);
            in_ws = false;
        }
    }
    out
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Compare two HTML strings after [`normalize_structure`].
pub fn check_same_structure(actual: &str, expected: &str) -> Result<(), String> {
    let (a, e) = (normalize_structure(actual), normalize_structure(expected));
    if a == e { Ok(()) } else { Err(format!("HTML differs after normalization\n    actual:   {a}\n    expected: {e}")) }
}

// ---------------------------------------------------------------------------
// data-source-line anchors from original output
// ---------------------------------------------------------------------------

/// Block tags FrankMD's line mapper annotates (`markdown_line_mapper.js`).
pub const ANNOTATED_TAGS: &[&str] =
    &["h1", "h2", "h3", "h4", "h5", "h6", "p", "ul", "ol", "blockquote", "pre", "hr", "table", "div"];

/// The `data-source-line` annotations of an original FrankMD output.
#[derive(Debug, Default)]
pub struct ExpectedAnchors {
    /// Top-level elements whose tag is in [`ANNOTATED_TAGS`]: `(tag, line)`.
    pub top: Vec<(String, Option<i64>)>,
    /// Lines of every annotated `<li>`, document order.
    pub li: Vec<i64>,
    /// Number of `<li>` elements (annotated or not).
    pub li_total: usize,
    /// Some non-`li` element below top level carries `data-source-line`.
    pub nested_annotated: bool,
}

impl ExpectedAnchors {
    /// Read the annotations of `html`.
    pub fn read(html: &str) -> Self {
        let frag = Html::parse_fragment(html);
        let mut a = ExpectedAnchors::default();
        for child in frag.root_element().children() {
            if let Some(el) = ElementRef::wrap(child) {
                let name = el.value().name();
                if ANNOTATED_TAGS.contains(&name) {
                    a.top.push((name.to_owned(), line_attr(el)));
                }
                if name == "li" {
                    a.li_total += 1;
                    if let Some(l) = line_attr(el) {
                        a.li.push(l);
                    }
                }
                for d in el.descendants().skip(1).filter_map(ElementRef::wrap) {
                    if d.value().name() == "li" {
                        a.li_total += 1;
                        if let Some(l) = line_attr(d) {
                            a.li.push(l);
                        }
                    } else if line_attr(d).is_some() {
                        a.nested_annotated = true;
                    }
                }
            }
        }
        a
    }

    /// FrankMD assigns top-level lines to the first N block tags of marked's
    /// HTML **sequentially**, so a nested block (`<p>` inside `<blockquote>`, a
    /// nested `<ul>`) or block tags inside raw HTML steal later slots. When that
    /// happened the original's top-level lines are wrong and are not compared.
    pub fn top_slot_shifted(&self) -> bool {
        self.nested_annotated || self.top.iter().any(|(_, l)| l.is_none())
    }

    /// `<li>` lines are position-based and only shift when some `<li>` (e.g. in
    /// a list inside a blockquote) was not collected.
    pub fn li_slot_shifted(&self) -> bool {
        self.li.len() != self.li_total
    }
}

fn line_attr(el: ElementRef<'_>) -> Option<i64> {
    el.value().attr("data-source-line").and_then(|v| v.parse().ok())
}

/// Element tag the original renders for a top-level IR block (`None` for raw
/// HTML/embeds, whose structure is the note's own).
pub fn block_tag(b: &Block) -> Option<String> {
    Some(match b {
        Block::Heading { level, .. } => format!("h{level}"),
        Block::Paragraph { .. } => "p".into(),
        Block::Code { .. } => "pre".into(),
        Block::Math { .. } => "div".into(),
        Block::Quote { .. } => "blockquote".into(),
        Block::List { ordered: Some(_), .. } => "ol".into(),
        Block::List { ordered: None, .. } => "ul".into(),
        Block::Table { .. } => "table".into(),
        Block::Rule { .. } => "hr".into(),
        Block::Embed { .. } | Block::Html { .. } => return None,
    })
}

/// IR list-item lines in FrankMD's annotation order: top-level lists only,
/// each item then its directly nested lists' items, depth-first.
pub fn ir_li_lines(doc: &RenderDoc) -> Vec<i64> {
    fn items(list: &[ListItem], out: &mut Vec<i64>) {
        for it in list {
            out.push(i64::from(it.span.start_line));
            for c in &it.children {
                if let Block::List { items: sub, .. } = c {
                    items(sub, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    for b in &doc.blocks {
        if let Block::List { items: list, .. } = b {
            items(list, &mut out);
        }
    }
    out
}

/// Compare IR line semantics with an original's annotations. `line_shift` is
/// added to IR lines before comparing (FrankMD's front-matter off-by-one, see
/// [`frontmatter_line_shift`]).
pub fn check_anchors(doc: &RenderDoc, expected_html: &str, line_shift: i64) -> Result<(), String> {
    let exp = ExpectedAnchors::read(expected_html);
    let mut errs = Vec::new();
    let has_raw = doc.blocks.iter().any(|b| block_tag(b).is_none());
    if !exp.top_slot_shifted() && !has_raw {
        let actual: Vec<(String, i64)> = doc
            .blocks
            .iter()
            .map(|b| (block_tag(b).unwrap(), i64::from(b.span().start_line) + line_shift))
            .collect();
        let want: Vec<(String, i64)> = exp.top.iter().map(|(t, l)| (t.clone(), l.unwrap())).collect();
        if actual != want {
            errs.push(format!("top-level blocks (tag, line): actual {actual:?}, expected {want:?}"));
        }
    }
    if !exp.li_slot_shifted() {
        let actual: Vec<i64> = ir_li_lines(doc).into_iter().map(|l| l + line_shift).collect();
        if actual != exp.li {
            errs.push(format!("list-item lines: actual {actual:?}, expected {:?}", exp.li));
        }
    }
    if errs.is_empty() { Ok(()) } else { Err(errs.join("; ")) }
}

/// FrankMD's preview reports body lines as `bodyLine + frontmatterLines`, where
/// the body was `trimStart()`ed and `frontmatterLines` counts one extra line.
/// Returns `js_line - true_line` for this source (0 without front matter):
/// `1 - (blank lines trimmed after the front matter)`.
pub fn frontmatter_line_shift(source: &str, stripped_content: &str, frontmatter_lines: i64) -> i64 {
    if frontmatter_lines == 0 || stripped_content.is_empty() {
        return 0;
    }
    let prefix = &source[..source.len() - stripped_content.len()];
    let true_first_line = prefix.matches('\n').count() as i64 + 1;
    (frontmatter_lines + 1) - true_first_line
}

// ---------------------------------------------------------------------------
// IR walkers
// ---------------------------------------------------------------------------

/// DOM-`textContent`-like plain text of inlines.
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for i in inlines {
        match i {
            Inline::Text(t)
            | Inline::Code(t)
            | Inline::Math(t)
            | Inline::Superscript(t)
            | Inline::Subscript(t)
            | Inline::Highlight(t) => s.push_str(t),
            Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) => s.push_str(&plain_text(c)),
            Inline::Link { children, .. } => s.push_str(&plain_text(children)),
            Inline::WikiLink { target, label } => {
                s.push_str(label.as_deref().unwrap_or_else(|| target.rsplit('/').next().unwrap_or(target)));
            }
            Inline::Emoji { emoji, .. } => s.push_str(emoji),
            Inline::FootnoteRef(l) => s.push_str(l),
            Inline::SoftBreak => s.push('\n'),
            Inline::Icon { .. } | Inline::Image { .. } | Inline::Html(_) | Inline::HardBreak => {}
        }
    }
    s
}

/// `textContent` of a block.
pub fn block_text(b: &Block) -> String {
    match b {
        Block::Heading { inlines, .. } | Block::Paragraph { inlines, .. } => plain_text(inlines),
        Block::Code { lines, .. } => lines.iter().map(|l| format!("{}\n", l.text)).collect(),
        Block::Math { tex, .. } => tex.clone(),
        Block::Quote { children, .. } => children.iter().map(block_text).collect(),
        Block::List { items, .. } => items.iter().map(|i| i.children.iter().map(block_text).collect::<String>()).collect(),
        Block::Table { header, rows, .. } => {
            let mut s: String = header.iter().map(|c| plain_text(c)).collect();
            for r in rows {
                s.extend(r.iter().map(|c| plain_text(c)));
            }
            s
        }
        Block::Rule { .. } | Block::Embed { .. } | Block::Html { .. } => String::new(),
    }
}

/// Visit every inline of a document (recursively, in document order).
pub fn walk_inlines(doc: &RenderDoc, f: &mut dyn FnMut(&Inline)) {
    fn inl(v: &[Inline], f: &mut dyn FnMut(&Inline)) {
        for i in v {
            f(i);
            match i {
                Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) => inl(c, f),
                Inline::Link { children, .. } => inl(children, f),
                _ => {}
            }
        }
    }
    walk_blocks(doc, &mut |b| match b {
        Block::Heading { inlines, .. } | Block::Paragraph { inlines, .. } => inl(inlines, f),
        Block::Table { header, rows, .. } => {
            for c in header {
                inl(c, f);
            }
            for r in rows {
                for c in r {
                    inl(c, f);
                }
            }
        }
        _ => {}
    });
}

/// Visit every block (recursively, pre-order).
pub fn walk_blocks(doc: &RenderDoc, f: &mut dyn FnMut(&Block)) {
    fn blk(v: &[Block], f: &mut dyn FnMut(&Block)) {
        for b in v {
            f(b);
            match b {
                Block::Quote { children, .. } => blk(children, f),
                Block::List { items, .. } => {
                    for it in items {
                        blk(&it.children, f);
                    }
                }
                _ => {}
            }
        }
    }
    blk(&doc.blocks, f);
}

/// Math nodes in document order. Inline math inside a block is visited when
/// its block is, i.e. before the next block's math (DOM order).
pub fn collect_math(doc: &RenderDoc) -> Vec<(MathStyle, String)> {
    let mut out = Vec::new();
    fn inl(v: &[Inline], out: &mut Vec<(MathStyle, String)>) {
        for i in v {
            match i {
                Inline::Math(t) => out.push((MathStyle::Inline, t.clone())),
                Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) => inl(c, out),
                Inline::Link { children, .. } => inl(children, out),
                _ => {}
            }
        }
    }
    walk_blocks(doc, &mut |b| match b {
        Block::Math { tex, .. } => out.push((MathStyle::Display, tex.clone())),
        Block::Heading { inlines, .. } | Block::Paragraph { inlines, .. } => inl(inlines, &mut out),
        Block::Table { header, rows, .. } => {
            for c in header {
                inl(c, &mut out);
            }
            for r in rows {
                for c in r {
                    inl(c, &mut out);
                }
            }
        }
        _ => {}
    });
    out
}

/// Code blocks in document order: `(lang, lines)`.
pub fn collect_code(doc: &RenderDoc) -> Vec<(Option<String>, Vec<HighlightedLine>)> {
    let mut out = Vec::new();
    walk_blocks(doc, &mut |b| {
        if let Block::Code { lang, lines, .. } = b {
            out.push((lang.clone(), lines.as_ref().clone()));
        }
    });
    out
}

/// Whether highlighted lines carry any non-plain token.
pub fn is_highlighted(lines: &[HighlightedLine]) -> bool {
    lines.iter().any(|l| l.runs.iter().any(|(_, s)| s.class != TokenClass::Plain))
}

// ---------------------------------------------------------------------------
// Highlighting
// ---------------------------------------------------------------------------

/// Lezer class attribute (`"tok-function tok-definition"`) → role, per
/// FrankMD's `prose.css`. `None` for classes with no color role.
pub fn lezer_style(class_attr: &str) -> Option<TokenStyle> {
    let mut class = None;
    let mut definition = false;
    for c in class_attr.split_whitespace() {
        let role = match c {
            "tok-keyword" | "tok-modifier" => TokenClass::Keyword,
            "tok-function" => TokenClass::Function,
            "tok-operator" | "tok-derefOperator" => TokenClass::Operator,
            "tok-string" | "tok-string2" | "tok-character" | "tok-attributeValue" => TokenClass::String,
            "tok-number" | "tok-unit" => TokenClass::Number,
            "tok-constant" | "tok-bool" | "tok-null" | "tok-atom" => TokenClass::Constant,
            "tok-comment" | "tok-docComment" => TokenClass::Comment,
            "tok-typeName" | "tok-className" | "tok-namespace" | "tok-tagName" => TokenClass::Type,
            "tok-propertyName" | "tok-attributeName" => TokenClass::Property,
            "tok-variableName" => TokenClass::Identifier,
            "tok-punctuation" | "tok-bracket" | "tok-separator" => TokenClass::Punctuation,
            "tok-invalid" => TokenClass::Invalid,
            "tok-definition" => {
                definition = true;
                continue;
            }
            _ => continue,
        };
        class.get_or_insert(role);
    }
    class.map(|class| TokenStyle { class, definition })
}

/// Style shorthand.
pub fn style(class: TokenClass, definition: bool) -> TokenStyle {
    TokenStyle { class, definition }
}

/// Highlight `code` with the builtin highlighter under fence alias `alias`,
/// flattened to `(token text, style)` for every non-plain run.
pub fn tokens(alias: &str, code: &str) -> Vec<(String, TokenStyle)> {
    let lang = language_for_fence(alias).unwrap_or_else(|| panic!("no language for fence {alias:?}"));
    let lines = BuiltinHighlighter.highlight(lang, code).unwrap_or_else(|| panic!("{alias}: not highlighted"));
    flatten_runs(&lines)
}

/// Flatten highlighted lines to `(text, style)` for non-plain runs.
pub fn flatten_runs(lines: &[HighlightedLine]) -> Vec<(String, TokenStyle)> {
    let mut out = Vec::new();
    for l in lines {
        for (r, s) in &l.runs {
            if s.class != TokenClass::Plain {
                out.push((l.text[r.clone()].to_owned(), *s));
            }
        }
    }
    out
}

/// Assert a token with exactly this text and style exists.
pub fn assert_token(toks: &[(String, TokenStyle)], text: &str, class: TokenClass, definition: bool) {
    let want = style(class, definition);
    assert!(
        toks.iter().any(|(t, s)| t == text && *s == want),
        "no token {text:?} with {want:?}; got {toks:?}"
    );
}

/// Assert some token has this text and class (any `definition` flag).
pub fn assert_token_class(toks: &[(String, TokenStyle)], text: &str, class: TokenClass) {
    assert!(toks.iter().any(|(t, s)| t == text && s.class == class), "no token {text:?} with {class:?}; got {toks:?}");
}

/// One `pre > code[class]` block read from an HTML string.
#[derive(Debug, Clone)]
pub struct HtmlCodeBlock {
    /// First `language-*` class value (as written), `""` when the code has a
    /// class but none of them is `language-*`.
    pub lang: String,
    /// `textContent` of the `<code>`.
    pub text: String,
    /// `data-code-language` of the `<pre>`.
    pub label: Option<String>,
    /// `(token text, class attribute)` of the `<span class>` children.
    pub spans: Vec<(String, String)>,
}

impl HtmlCodeBlock {
    /// Whether the original emitted Lezer token spans.
    pub fn highlighted(&self) -> bool {
        self.spans.iter().any(|(_, c)| c.split_whitespace().any(|x| x.starts_with("tok-")))
    }
}

/// Every `pre > code[class]` in document order (what the original inspects).
pub fn html_code_blocks(html: &str) -> Vec<HtmlCodeBlock> {
    let frag = Html::parse_fragment(html);
    let sel = scraper::Selector::parse("pre > code[class]").unwrap();
    frag.select(&sel)
        .map(|code| {
            let lang = code
                .value()
                .classes()
                .find_map(|c| c.strip_prefix("language-"))
                .unwrap_or("")
                .to_owned();
            let pre = code.parent().and_then(ElementRef::wrap).unwrap();
            let spans = code
                .children()
                .filter_map(ElementRef::wrap)
                .filter(|e| e.value().name() == "span")
                .map(|e| (e.text().collect::<String>(), e.value().attr("class").unwrap_or("").to_owned()))
                .collect();
            HtmlCodeBlock {
                lang,
                text: code.text().collect(),
                label: pre.value().attr("data-code-language").map(str::to_owned),
                spans,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Math verdicts from KaTeX output
// ---------------------------------------------------------------------------

/// What KaTeX made of a placeholder, read from its rendered HTML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KatexVerdict {
    /// Clean render.
    Ok,
    /// `katex-error` (ParseError, incl. too many expansions).
    Parse,
    /// Rendered, with these commands in the error color (unknown/untrusted).
    Unsupported(Vec<String>),
}

impl KatexVerdict {
    /// Does a `math::validate` result agree with this verdict?
    pub fn matches(&self, r: &Result<(), MathError>) -> bool {
        match (self, r) {
            (KatexVerdict::Ok, Ok(())) => true,
            (KatexVerdict::Parse, Err(MathError::Parse(_))) => true,
            (KatexVerdict::Unsupported(want), Err(MathError::Unsupported(got))) => want == got,
            _ => false,
        }
    }
}

/// Verdict for one rendered `.math-inline`/`.math-block` element.
pub fn katex_verdict(el: ElementRef<'_>) -> KatexVerdict {
    let mut red = Vec::new();
    for d in el.descendants().filter_map(ElementRef::wrap) {
        if d.value().classes().any(|c| c == "katex-error") {
            return KatexVerdict::Parse;
        }
        let st = d.value().attr("style").unwrap_or("").replace(' ', "");
        let is_red = st.contains("color:rgb(204,0,0)") || st.contains("color:#cc0000");
        let is_leaf_mord = d.value().classes().collect::<Vec<_>>() == ["mord"];
        if is_red && is_leaf_mord {
            let t: String = d.text().collect();
            if t.starts_with('\\') {
                red.push(t);
            }
        }
    }
    if red.is_empty() { KatexVerdict::Ok } else { KatexVerdict::Unsupported(red) }
}

/// `(style, placeholder text, verdict)` for every math placeholder in a final
/// (post-KaTeX) HTML string, document order. The TeX comes from `before_html`
/// (the same placeholders before KaTeX ran).
pub fn katex_verdicts(before_html: &str, after_html: &str) -> Vec<(MathStyle, String, KatexVerdict)> {
    let sel = scraper::Selector::parse(".math-inline, .math-block").unwrap();
    let before = Html::parse_fragment(before_html);
    let after = Html::parse_fragment(after_html);
    let texs: Vec<(MathStyle, String)> = before
        .select(&sel)
        .map(|e| {
            let st = if e.value().classes().any(|c| c == "math-block") { MathStyle::Display } else { MathStyle::Inline };
            (st, e.text().collect())
        })
        .collect();
    let verdicts: Vec<KatexVerdict> = after.select(&sel).map(katex_verdict).collect();
    texs.into_iter().zip(verdicts).map(|((s, t), v)| (s, t, v)).collect()
}
