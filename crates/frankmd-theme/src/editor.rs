//! Editor styling roles ported from `app/javascript/lib/codemirror_theme.js`.
//! GUI-free: everything is expressed as [`crate::COLOR_KEYS`] token names;
//! the app crate resolves them against the active [`crate::UiTokens`].

/// Editor font options (`createEditorTheme({fontFamily, fontSize, lineHeight})`).
#[derive(Debug, Clone, PartialEq)]
pub struct EditorFont {
    /// CSS-ish family list (default `'Cascadia Code', monospace`).
    pub family: String,
    /// Pixels (default 14).
    pub size_px: f32,
    /// Line height multiplier (default 1.6).
    pub line_height: f32,
}

impl Default for EditorFont {
    fn default() -> Self {
        Self {
            family: "'Cascadia Code', monospace".to_owned(),
            size_px: 14.0,
            line_height: 1.6,
        }
    }
}

/// Text decoration of a highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Decoration {
    Underline,
    LineThrough,
}

/// One markdown highlight rule (`createSyntaxHighlighting()` spec).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpec {
    /// `@lezer/highlight` tag name (`heading1`, `strong`, `monospace`, ...).
    pub tag: &'static str,
    /// Foreground token (`heading-1`, `accent`, ...).
    pub color: &'static str,
    /// Background token, if any (`code-bg` for inline code).
    pub background: Option<&'static str>,
    /// `fontWeight: bold`.
    pub bold: bool,
    /// `fontStyle: italic`.
    pub italic: bool,
    /// `textDecoration`.
    pub decoration: Option<Decoration>,
}

/// The 26 markdown highlight rules, in the original order.
pub fn markdown_highlights() -> &'static [HighlightSpec] {
    &MARKDOWN_HIGHLIGHTS
}

/// A rule with only a foreground color.
const fn color(tag: &'static str, color: &'static str) -> HighlightSpec {
    HighlightSpec {
        tag,
        color,
        background: None,
        bold: false,
        italic: false,
        decoration: None,
    }
}

/// `createSyntaxHighlighting()` in `codemirror_theme.js`, rule for rule.
static MARKDOWN_HIGHLIGHTS: [HighlightSpec; 26] = [
    color("heading1", "heading-1"),
    color("heading2", "heading-2"),
    color("heading3", "heading-3"),
    color("heading4", "accent"),
    color("heading5", "accent"),
    color("heading6", "accent"),
    HighlightSpec {
        bold: true,
        ..color("strong", "text-primary")
    },
    HighlightSpec {
        italic: true,
        ..color("emphasis", "text-muted")
    },
    HighlightSpec {
        decoration: Some(Decoration::LineThrough),
        ..color("strikethrough", "text-muted")
    },
    HighlightSpec {
        background: Some("code-bg"),
        ..color("monospace", "accent")
    },
    HighlightSpec {
        decoration: Some(Decoration::Underline),
        ..color("link", "accent")
    },
    color("url", "accent"),
    color("quote", "text-muted"),
    color("list", "text-muted"),
    color("processingInstruction", "text-muted"),
    color("meta", "text-muted"),
    color("contentSeparator", "border"),
    color("angleBracket", "text-muted"),
    color("tagName", "accent"),
    color("attributeName", "heading-2"),
    color("attributeValue", "heading-3"),
    HighlightSpec {
        italic: true,
        ..color("comment", "text-faint")
    },
    color("labelName", "accent"),
    color("inserted", "accent"),
    HighlightSpec {
        decoration: Some(Decoration::LineThrough),
        ..color("deleted", "text-muted")
    },
    color("escape", "text-muted"),
];

/// Editor chrome → token mapping (`createEditorTheme` rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorChrome {
    /// Editor background (`bg-primary`).
    pub background: &'static str,
    /// Text and caret (`text-primary`).
    pub text: &'static str,
    /// Selection background (`selection`).
    pub selection: &'static str,
    /// Gutter background (`bg-secondary`).
    pub gutter_background: &'static str,
    /// Gutter text / line numbers (`text-faint`).
    pub gutter_text: &'static str,
    /// Gutter right border (`border`).
    pub gutter_border: &'static str,
    /// Active line number (`text-muted`).
    pub active_gutter_text: &'static str,
    /// Placeholder (`text-faint`, italic).
    pub placeholder: &'static str,
    /// Search match background (`selection`) / outline (`accent`).
    pub search_match: &'static str,
    /// Search match outline.
    pub search_match_outline: &'static str,
    /// Selected search match background (`accent`).
    pub search_match_selected: &'static str,
    /// Panels / tooltips / autocomplete background (`bg-secondary`).
    pub panel_background: &'static str,
}

/// The editor chrome mapping.
pub fn editor_chrome() -> EditorChrome {
    EditorChrome {
        background: "bg-primary",
        text: "text-primary",
        selection: "selection",
        gutter_background: "bg-secondary",
        gutter_text: "text-faint",
        gutter_border: "border",
        active_gutter_text: "text-muted",
        placeholder: "text-faint",
        search_match: "selection",
        search_match_outline: "accent",
        search_match_selected: "accent",
        panel_background: "bg-secondary",
    }
}

/// Autocomplete list grows to this many `em` before scrolling.
pub const AUTOCOMPLETE_MAX_HEIGHT_EM: f32 = 16.0;
