//! Markdown → IR rendering, plus FrankMD's custom inline/block syntax.
//!
//! The lexer is a port of marked (see [`crate::marked`] for why not comrak).
//!
//! FrankMD renders with marked 15 (`breaks: true, gfm: true`) and eight
//! extensions (`app/javascript/lib/marked_extensions.js`, in this priority
//! order): superscript `^x^`, subscript `~x~`, highlight `==x==`, emoji
//! `:code:`, icon `:ph-name:`, wikilink `[[t|l]]`, block math `$$..$$`, inline
//! math `$..$`. The extensions are the scanners in [`ext`], whose contracts
//! are the original tokenizers'.

use std::fmt;
use std::sync::Arc;

use crate::RenderDoc;
use crate::highlight::{CodeHighlighter, HighlightBudget};
use crate::ir::Inline;

/// Rendering options derived from config.
#[derive(Clone, Default)]
pub struct RenderOptions {
    /// Per-render highlighting limits (HP-017). Default = FrankMD's.
    pub highlight_budget: HighlightBudget,
    /// Highlighter for fenced code. `None` leaves every block plain.
    pub highlighter: Option<Arc<dyn CodeHighlighter>>,
}

impl fmt::Debug for RenderOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderOptions")
            .field("highlight_budget", &self.highlight_budget)
            .field(
                "highlighter",
                &self.highlighter.as_ref().map(|_| "dyn CodeHighlighter"),
            )
            .finish()
    }
}

/// Render a whole note: strips front matter exactly like FrankMD's preview
/// (`crate::frontmatter::strip`), then renders the body. Spans are lines of
/// the whole note. Pure and deterministic; run on a background thread by the app.
pub fn render(source: &str, opts: &RenderOptions) -> RenderDoc {
    let stripped = crate::frontmatter::strip(source);
    // `content` is a suffix of `source`: everything before it (front matter
    // and the blank lines `trimStart` dropped) shifts the body's lines.
    let body_offset = crate::marked::newlines(&source[..source.len() - stripped.content.len()]);
    let mut doc = render_body(stripped.content, body_offset, opts);
    doc.total_lines = js_line_count(source);
    doc.front_matter = crate::frontmatter::block(source).map(str::to_owned);
    doc
}

/// Render markdown without front-matter detection; every span is shifted by
/// `line_offset` (FrankMD `parseWithLineNumbers(markdown, lineOffset)`).
/// `total_lines` is the line count of `markdown`.
pub fn render_body(markdown: &str, line_offset: u32, opts: &RenderOptions) -> RenderDoc {
    let (tokens, line_map) = crate::marked::lex(markdown);
    RenderDoc {
        blocks: crate::lower::blocks(&tokens, &line_map, line_offset, opts),
        footnotes: Vec::new(),
        front_matter: None,
        total_lines: js_line_count(markdown),
    }
}

/// Parse inline markdown only (marked `parseInline`).
pub fn parse_inline(src: &str) -> Vec<Inline> {
    crate::lower::inlines(&crate::marked::lex_inline(src))
}

/// JS `content.split("\n").length`.
fn js_line_count(s: &str) -> u32 {
    crate::marked::newlines(s) + 1
}

/// FrankMD's marked extensions as pure scanners. Each `scan_*` mirrors the
/// extension's `tokenizer(src)`: it only matches at the very start of `src`
/// and returns the consumed byte length (`raw`) plus the token payload, or
/// `None` where the tokenizer returned `undefined`. Each [`start`] mirrors the
/// extension's `start(src)` (marked's "earliest possible start" hint).
pub mod ext {
    use std::sync::LazyLock;

    use fancy_regex::Regex;

    use crate::marked::{js_trim, rules};

    /// The eight extensions, in FrankMD's registration (= priority) order.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[allow(missing_docs)]
    pub enum Extension {
        Superscript,
        Subscript,
        Highlight,
        Emoji,
        Icon,
        Wikilink,
        MathBlock,
        MathInline,
    }

    /// marked extension level.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[allow(missing_docs)]
    pub enum Level {
        Inline,
        Block,
    }

    impl Extension {
        /// All extensions in registration order (`allExtensions`).
        pub const ALL: [Extension; 8] = [
            Extension::Superscript,
            Extension::Subscript,
            Extension::Highlight,
            Extension::Emoji,
            Extension::Icon,
            Extension::Wikilink,
            Extension::MathBlock,
            Extension::MathInline,
        ];

        /// marked extension `name`.
        pub fn name(self) -> &'static str {
            match self {
                Extension::Superscript => "superscript",
                Extension::Subscript => "subscript",
                Extension::Highlight => "highlight",
                Extension::Emoji => "emoji",
                Extension::Icon => "icon",
                Extension::Wikilink => "wikilink",
                Extension::MathBlock => "mathBlock",
                Extension::MathInline => "mathInline",
            }
        }

        /// marked extension `level`.
        pub fn level(self) -> Level {
            match self {
                Extension::MathBlock => Level::Block,
                _ => Level::Inline,
            }
        }
    }

    /// A successful scan: `raw_len` bytes of `src` were consumed.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Scanned<T> {
        /// Byte length of the matched raw text (`token.raw`).
        pub raw_len: usize,
        /// Token payload.
        pub value: T,
    }

    /// Wikilink payload (`token.target`, `token.displayText`).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WikiTok {
        /// Trimmed target (`folder/Note`).
        pub target: String,
        /// Trimmed display text, `None` when absent.
        pub display: Option<String>,
    }

    /// Icon payload (`token.path`, `token.viewBox`).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct IconTok {
        /// Phosphor icon name (without `ph-`).
        pub name: String,
        /// SVG path data.
        pub path: String,
        /// SVG viewBox (`"0 0 256 256"`).
        pub view_box: String,
    }

    /// `start(src)`: byte offset of the earliest possible start, `None` for
    /// JS `-1`/`undefined`. MathInline only offers an **unescaped** `$`.
    pub fn start(ext: Extension, src: &str) -> Option<usize> {
        match ext {
            Extension::Superscript => src.find('^'),
            Extension::Subscript => src.find('~'),
            Extension::Highlight => src.find("=="),
            Extension::Emoji => src.find(':'),
            Extension::Icon => src.find(":ph-"),
            Extension::Wikilink => src.find("[["),
            Extension::MathBlock => src.find("$$"),
            Extension::MathInline => {
                let b = src.as_bytes();
                (0..b.len()).find(|&i| b[i] == b'$' && (i == 0 || b[i - 1] != b'\\'))
            }
        }
    }

    fn scan(re: &Regex, src: &str) -> Option<(usize, Vec<Option<String>>)> {
        let cap = rules::exec(re, src)?;
        let len = cap.get(0)?.end();
        let groups = (1..cap.len())
            .map(|i| cap.get(i).map(|m| m.as_str().to_owned()))
            .collect();
        Some((len, groups))
    }

    fn scan_text(re: &Regex, src: &str) -> Option<Scanned<String>> {
        let (raw_len, mut g) = scan(re, src)?;
        Some(Scanned {
            raw_len,
            value: g.swap_remove(0).unwrap_or_default(),
        })
    }

    static SUPERSCRIPT: LazyLock<Regex> = LazyLock::new(|| rules::js(r"^\^([^\^]+)\^", ""));
    static SUBSCRIPT: LazyLock<Regex> = LazyLock::new(|| rules::js(r"^~([^~]+)~(?!~)", ""));
    static HIGHLIGHT: LazyLock<Regex> = LazyLock::new(|| rules::js(r"^==([^=]+)==", ""));
    static EMOJI: LazyLock<Regex> = LazyLock::new(|| rules::js(r"^:([a-z0-9_+-]+):", ""));
    static ICON: LazyLock<Regex> = LazyLock::new(|| rules::js(r"^:ph-([a-z0-9-]+):", ""));
    static WIKILINK: LazyLock<Regex> =
        LazyLock::new(|| rules::js(r"^\[\[([^\]|]+)(?:\|([^\]]+))?\]\]", ""));
    static MATH_BLOCK: LazyLock<Regex> =
        LazyLock::new(|| rules::js(r"^\$\$([\s\S]+?)\$\$(?:\n+|$)", ""));
    static MATH_INLINE: LazyLock<Regex> =
        LazyLock::new(|| rules::js(r"^\$(?!\$)(?!\s)((?:\\.|[^\\$])+?)(?<!\s)\$(?!\d)", ""));

    /// `^([^^]+)^` → text.
    pub fn scan_superscript(src: &str) -> Option<Scanned<String>> {
        if !src.starts_with('^') {
            return None;
        }
        scan_text(&SUPERSCRIPT, src)
    }

    /// `~([^~]+)~(?!~)` → text.
    pub fn scan_subscript(src: &str) -> Option<Scanned<String>> {
        if !src.starts_with('~') {
            return None;
        }
        scan_text(&SUBSCRIPT, src)
    }

    /// `==([^=]+)==` → text.
    pub fn scan_highlight(src: &str) -> Option<Scanned<String>> {
        if !src.starts_with("==") {
            return None;
        }
        scan_text(&HIGHLIGHT, src)
    }

    /// `:([a-z0-9_+-]+):` for a known shortcode → the emoji (last row wins on duplicates).
    pub fn scan_emoji(src: &str) -> Option<Scanned<&'static str>> {
        if !src.starts_with(':') {
            return None;
        }
        let s = scan_text(&EMOJI, src)?;
        let emoji = frankmd_core::emoji::lookup(&s.value)?;
        Some(Scanned {
            raw_len: s.raw_len,
            value: emoji,
        })
    }

    /// `:ph-([a-z0-9-]+):` for a known Phosphor icon.
    pub fn scan_icon(src: &str) -> Option<Scanned<IconTok>> {
        if !src.starts_with(":ph-") {
            return None;
        }
        let s = scan_text(&ICON, src)?;
        let glyph = frankmd_core::emoji::icons::lookup(&s.value)?;
        Some(Scanned {
            raw_len: s.raw_len,
            value: IconTok {
                name: s.value,
                path: glyph.path.to_owned(),
                view_box: glyph.view_box.to_owned(),
            },
        })
    }

    /// `\[\[([^\]|]+)(?:\|([^\]]+))?\]\]`, both parts trimmed.
    pub fn scan_wikilink(src: &str) -> Option<Scanned<WikiTok>> {
        if !src.starts_with("[[") {
            return None;
        }
        let (raw_len, g) = scan(&WIKILINK, src)?;
        let target = js_trim(g[0].as_deref().unwrap_or("")).to_owned();
        let display = g[1]
            .as_deref()
            .filter(|d| !d.is_empty())
            .map(|d| js_trim(d).to_owned());
        Some(Scanned {
            raw_len,
            value: WikiTok { target, display },
        })
    }

    /// `^\$\$([\s\S]+?)\$\$(?:\n+|$)` with non-blank content → trimmed TeX.
    pub fn scan_math_block(src: &str) -> Option<Scanned<String>> {
        if !src.starts_with("$$") {
            return None;
        }
        let s = scan_text(&MATH_BLOCK, src)?;
        let tex = js_trim(&s.value);
        (!tex.is_empty()).then(|| Scanned {
            raw_len: s.raw_len,
            value: tex.to_owned(),
        })
    }

    /// `^\$(?!\$)(?!\s)((?:\\.|[^\\$])+?)(?<!\s)\$(?!\d)` → TeX (escapes kept).
    pub fn scan_math_inline(src: &str) -> Option<Scanned<String>> {
        if !src.starts_with('$') {
            return None;
        }
        scan_text(&MATH_INLINE, src)
    }
}
