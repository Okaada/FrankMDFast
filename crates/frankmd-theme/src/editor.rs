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
        todo!()
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
    todo!("codemirror_theme.js createSyntaxHighlighting")
}

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
    todo!()
}

/// Autocomplete list grows to this many `em` before scrolling.
pub const AUTOCOMPLETE_MAX_HEIGHT_EM: f32 = 16.0;
