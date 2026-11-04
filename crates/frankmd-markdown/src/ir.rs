//! The render IR: blocks and inlines with source spans.
//!
//! The IR mirrors what FrankMD's preview pipeline (marked 15 with
//! `breaks: true, gfm: true` + FrankMD's extensions + DOMPurify) can express,
//! and nothing more. Golden conformance: `js_preview_pipeline.json`,
//! `js_marked_extensions.json`, `js_markdown_line_mapper.json`.

use std::sync::Arc;

/// 1-based inclusive source line span of a block (true lines from the lexer), in
/// lines of the **whole note** (front matter counted), so task toggling and
/// scroll sync address the editor buffer directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// First line.
    pub start_line: u32,
    /// Last line.
    pub end_line: u32,
}

/// A rendered document: top-level blocks (the preview virtualizes over these).
#[derive(Debug, Clone, Default)]
pub struct RenderDoc {
    /// Top-level blocks in order.
    pub blocks: Vec<Block>,
    /// Footnote definitions, rendered at the end. **[COMPAT]** marked has no
    /// footnote support, so FrankMD-compatible rendering leaves this empty and
    /// renders `[^1]` literally (see `js_preview_pipeline.json` "footnotes").
    pub footnotes: Vec<Footnote>,
    /// Raw front matter text (YAML `---` or TOML `+++`), if the note had one.
    /// Never rendered as markdown.
    pub front_matter: Option<String>,
    /// Total source lines of the whole note: JS `content.split("\n").length`
    /// (so `""` is 1 line). Used for ratio fallbacks.
    pub total_lines: u32,
}

/// Block-level node.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Block {
    /// `#`..`######` (and setext). `anchor` is the heading id slug.
    Heading { level: u8, inlines: Vec<Inline>, anchor: String, span: SourceSpan },
    /// Paragraph.
    Paragraph { inlines: Vec<Inline>, span: SourceSpan },
    /// Fenced or indented code. `lang` is the first word of the info string
    /// exactly as written (marked emits `class="language-<lang>"`); resolve it
    /// with [`crate::highlight::language_for_fence`]. `lines` are
    /// pre-highlighted within the render budget (HP-017), plain otherwise.
    Code { lang: Option<String>, lines: Arc<Vec<HighlightedLine>>, span: SourceSpan },
    /// `$$...$$` display math (FrankMD `mathBlock` extension). `tex` is trimmed.
    Math { tex: String, span: SourceSpan },
    /// Blockquote / GFM alert.
    Quote { alert: Option<AlertKind>, children: Vec<Block>, span: SourceSpan },
    /// Ordered (`Some(start)`) or unordered (`None`) list. `loose` lists wrap
    /// item paragraphs in `<p>` (marked's loose/tight distinction).
    List { ordered: Option<u64>, loose: bool, items: Vec<ListItem>, span: SourceSpan },
    /// GFM table.
    Table { align: Vec<Align>, header: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>>, span: SourceSpan },
    /// `---`.
    Rule { span: SourceSpan },
    /// YouTube (and future allowlisted) embeds rendered as thumbnail cards.
    /// `html` is the note's raw `<iframe>` for HTML export.
    Embed { embed: crate::embed::Embed, html: String, span: SourceSpan },
    /// Raw HTML block **as written in the note** (untrusted). HTML export
    /// sanitizes the assembled document with [`crate::html::sanitize`] like
    /// FrankMD (raw HTML can wrap later markdown, e.g. `<details>`); a native
    /// renderer must pass it through `sanitize` or show it as text.
    Html { text: String, span: SourceSpan },
}

#[allow(missing_docs)]
impl Block {
    pub fn span(&self) -> SourceSpan {
        match self {
            Block::Heading { span, .. }
            | Block::Paragraph { span, .. }
            | Block::Code { span, .. }
            | Block::Math { span, .. }
            | Block::Quote { span, .. }
            | Block::List { span, .. }
            | Block::Table { span, .. }
            | Block::Rule { span }
            | Block::Embed { span, .. }
            | Block::Html { span, .. } => *span,
        }
    }
}

/// List item (task items carry their source line for interactive toggling).
#[derive(Debug, Clone)]
pub struct ListItem {
    /// `Some(checked)` for GFM task items (`[ ]`, `[x]`, `[X]`).
    pub task: Option<bool>,
    /// Item content.
    pub children: Vec<Block>,
    /// Source span (toggle rewrites `[ ]`/`[x]` on `span.start_line` via
    /// `frankmd_core::text::task::task_marker_range`).
    pub span: SourceSpan,
}

/// Inline node.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum Inline {
    Text(String),
    Code(String),
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    /// `$...$` (FrankMD `mathInline` rules: no currency, no `$$`).
    Math(String),
    /// `^text^` → `<sup>`; content is literal text (not re-parsed).
    Superscript(String),
    /// `~text~` (single tilde) → `<sub>`; content is literal text.
    Subscript(String),
    /// `==text==` → `<mark>`; content is literal text.
    Highlight(String),
    /// `:shortcode:` resolved through `frankmd_core::emoji::lookup`.
    Emoji { shortcode: String, emoji: String },
    /// `:ph-name:` Phosphor icon (inline SVG path data).
    Icon { name: String, path: String, view_box: String },
    /// `url` is `None` when the href failed the URL allowlist (DOMPurify
    /// drops the attribute and keeps the text).
    Link { url: Option<SafeUrl>, title: Option<String>, children: Vec<Inline> },
    /// `[[target]]` / `[[target|label]]`, both trimmed. Display text is
    /// `label` or the basename of `target`.
    WikiLink { target: String, label: Option<String> },
    /// `src` is `None` when the URL failed the allowlist.
    Image { src: Option<SafeUrl>, alt: String, title: Option<String> },
    /// Raw inline HTML as written (untrusted, see [`Block::Html`]).
    Html(String),
    FootnoteRef(String),
    SoftBreak,
    /// With marked's `breaks: true`, every newline inside a paragraph is a
    /// hard break (`<br>`).
    HardBreak,
}

/// A URL that passed DOMPurify's URI allowlist (http(s), ftp, mailto, tel,
/// relative, vault-local...; `data:` only for images). `javascript:` and
/// other script schemes never reach the IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeUrl(pub String);

/// One highlighted code line: `(byte range, token style)` runs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HighlightedLine {
    /// Line text (without the trailing newline).
    pub text: String,
    /// Token runs over `text`; uncovered bytes are plain.
    pub runs: Vec<(std::ops::Range<usize>, TokenStyle)>,
}

/// Highlight token roles. These are the theme's `--theme-syntax-*` roles that
/// FrankMD's `prose.css` maps the Lezer `tok-*` classes onto:
///
/// | Lezer class(es) | role |
/// |---|---|
/// | `tok-keyword`, `tok-modifier` | Keyword |
/// | `tok-function` | Function |
/// | `tok-operator`, `tok-derefOperator` | Operator |
/// | `tok-string`, `tok-string2`, `tok-character`, `tok-attributeValue` | String |
/// | `tok-number`, `tok-unit` | Number |
/// | `tok-constant`, `tok-bool`, `tok-null`, `tok-atom` | Constant |
/// | `tok-comment`, `tok-docComment` | Comment |
/// | `tok-typeName`, `tok-className`, `tok-namespace`, `tok-tagName` | Type |
/// | `tok-propertyName`, `tok-attributeName` | Property |
/// | `tok-variableName` | Identifier |
/// | `tok-punctuation`, `tok-bracket`, `tok-separator` | Punctuation |
/// | `tok-invalid` | Invalid |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum TokenClass {
    Keyword, Function, Operator, String, Number, Constant, Comment, Type, Property,
    Identifier, Punctuation, Invalid, Plain,
}

/// A token's role plus the `tok-definition` modifier (rendered bold).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenStyle {
    /// Color role.
    pub class: TokenClass,
    /// Lezer `tok-definition` (a binding site: `fn main`, `{ name: ... }`).
    pub definition: bool,
}

/// Table column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Align { None, Left, Center, Right }

/// GFM alert kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum AlertKind { Note, Tip, Important, Warning, Caution }

/// Footnote definition.
#[derive(Debug, Clone)]
pub struct Footnote {
    /// Label.
    pub label: String,
    /// Content.
    pub children: Vec<Block>,
}
