//! marked's token shapes (only the fields the renderer and the IR use).

use crate::ir::Align;

/// A block token: marked's `raw` (needed for marked's merge/re-lex rules),
/// its 0-based line in the lexed source, and the payload.
#[derive(Debug, Clone)]
pub(crate) struct BlockTok {
    pub(crate) raw: String,
    pub(crate) line: u32,
    pub(crate) kind: BlockKind,
}

/// Inline content of a block: marked queues `text` and lexes it after the
/// block pass (so every link definition is known).
#[derive(Debug, Clone, Default)]
pub(crate) struct Inl {
    pub(crate) text: String,
    pub(crate) tokens: Vec<InlineTok>,
}

impl Inl {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tokens: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum BlockKind {
    Space,
    /// `lang` is marked's processed info string (`None` when absent/empty).
    Code {
        lang: Option<String>,
        text: String,
    },
    Heading {
        depth: u8,
        inl: Inl,
    },
    Hr,
    Blockquote {
        text: String,
        children: Vec<BlockTok>,
    },
    List {
        ordered: bool,
        start: u64,
        loose: bool,
        items: Vec<ListItemTok>,
    },
    Html {
        text: String,
    },
    Table {
        align: Vec<Align>,
        header: Vec<Inl>,
        rows: Vec<Vec<Inl>>,
    },
    Paragraph {
        inl: Inl,
    },
    /// marked's block `text` token (list items: `top == false`).
    Text {
        inl: Inl,
    },
    /// FrankMD `mathBlock`.
    MathBlock {
        text: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct ListItemTok {
    pub(crate) raw: String,
    pub(crate) line: u32,
    pub(crate) task: Option<bool>,
    pub(crate) text: String,
    pub(crate) children: Vec<BlockTok>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InlineTok {
    /// `escape`: one literal character.
    Escape(String),
    /// Inline raw HTML (`tag`).
    Html(String),
    Link {
        href: String,
        title: Option<String>,
        children: Vec<InlineTok>,
    },
    Image {
        href: String,
        title: Option<String>,
        text: String,
    },
    Strong(Vec<InlineTok>),
    Em(Vec<InlineTok>),
    Del(Vec<InlineTok>),
    Codespan(String),
    Br,
    /// `escaped` = marked's `inRawBlock` flag: printed without escaping.
    Text {
        raw: String,
        text: String,
        escaped: bool,
    },
    Superscript(String),
    Subscript(String),
    Highlight(String),
    Emoji {
        shortcode: String,
        emoji: &'static str,
    },
    Icon {
        name: String,
        path: String,
        view_box: String,
    },
    Wikilink {
        target: String,
        display: Option<String>,
    },
    Math(String),
}
