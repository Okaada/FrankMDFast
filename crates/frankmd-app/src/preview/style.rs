//! Preview colours and typography (ui-spec §5.21, §7.2), from FrankMD's 38
//! theme tokens.

use crate::scale::px;
use frankmd_markdown::ir::{AlertKind, TokenClass};
use frankmd_theme::{Rgb, UiTokens};
use gpui_kit::{Hsla, Pixels, Rgba};

/// Default preview zoom (percent).
pub const DEFAULT_ZOOM: u16 = 100;
/// Zoom bounds (Settings › Preview, ui-spec §9).
pub const MIN_ZOOM: u16 = 50;
/// Upper zoom bound.
pub const MAX_ZOOM: u16 = 200;
/// Zoom step for zoom in / out.
pub const ZOOM_STEP: u16 = 10;

/// Clamp a zoom percentage into the allowed range.
pub fn clamp_zoom(percent: u16) -> u16 {
    percent.clamp(MIN_ZOOM, MAX_ZOOM)
}

/// Body text size at `zoom` percent: `zoom × 15 / 100` px.
pub fn text_px(zoom: u16) -> f32 {
    f32::from(zoom) * 15.0 / 100.0
}

/// Heading size multipliers (em) for h1..h6.
pub(crate) const HEADING_EM: [f32; 6] = [2.0, 1.5, 1.25, 1.1, 1.0, 1.0];

/// Resolved colours the preview paints with.
#[derive(Debug, Clone)]
pub struct PreviewColors {
    /// `bg-primary`.
    pub background: Hsla,
    /// `text-primary`.
    pub text: Hsla,
    /// `text-secondary`.
    pub text_secondary: Hsla,
    /// `text-muted`.
    pub muted: Hsla,
    /// `text-faint`.
    pub faint: Hsla,
    /// `border`.
    pub border: Hsla,
    /// `accent`.
    pub accent: Hsla,
    /// `accent-text` (text on an accent fill).
    pub accent_text: Hsla,
    /// `code-bg`.
    pub code_bg: Hsla,
    /// `bg-secondary`.
    pub surface: Hsla,
    /// `bg-hover`.
    pub hover: Hsla,
    /// `selection` (used for `==mark==`).
    pub mark: Hsla,
    /// `error`.
    pub danger: Hsla,
    /// `warning`.
    pub warning: Hsla,
    /// `success`.
    pub success: Hsla,
    /// `heading-1`..`heading-3`.
    pub headings: [Hsla; 3],
    /// `syntax-*` roles in [`TokenClass`] order.
    syntax: [Hsla; 12],
    /// The raw `text-primary` value, for math SVG paint.
    pub text_rgb: Rgb,
}

impl PreviewColors {
    /// Build from the theme tokens (any missing token falls back to `text-primary`).
    pub fn from_tokens(tokens: &UiTokens) -> Self {
        let text_rgb = tokens.get("text-primary").unwrap_or(Rgb(0xcc, 0xcc, 0xcc));
        let c = |key: &str| hsla(tokens.get(key).unwrap_or(text_rgb));
        let syntax = [
            "syntax-keyword",
            "syntax-function",
            "syntax-operator",
            "syntax-string",
            "syntax-number",
            "syntax-constant",
            "syntax-comment",
            "syntax-type",
            "syntax-property",
            "syntax-identifier",
            "syntax-punctuation",
            "syntax-invalid",
        ]
        .map(c);
        Self {
            background: c("bg-primary"),
            text: c("text-primary"),
            text_secondary: c("text-secondary"),
            muted: c("text-muted"),
            faint: c("text-faint"),
            border: c("border"),
            accent: c("accent"),
            accent_text: c("accent-text"),
            code_bg: c("code-bg"),
            surface: c("bg-secondary"),
            hover: c("bg-hover"),
            mark: c("selection"),
            danger: c("error"),
            warning: c("warning"),
            success: c("success"),
            headings: [c("heading-1"), c("heading-2"), c("heading-3")],
            syntax,
            text_rgb,
        }
    }

    /// The built-in theme matching a light/dark mode (startup default before
    /// the shell applies the real theme).
    pub fn builtin(dark: bool) -> Self {
        let id = if dark { "dark" } else { "light" };
        frankmd_theme::builtin(id)
            .map(|t| Self::from_tokens(&t))
            .unwrap_or_else(|| {
                Self::from_tokens(&UiTokens {
                    mode: if dark {
                        frankmd_theme::Mode::Dark
                    } else {
                        frankmd_theme::Mode::Light
                    },
                    colors: Default::default(),
                })
            })
    }

    /// Colour of a highlight token role (`Plain` = body text).
    pub fn syntax(&self, class: TokenClass) -> Hsla {
        let ix = match class {
            TokenClass::Keyword => 0,
            TokenClass::Function => 1,
            TokenClass::Operator => 2,
            TokenClass::String => 3,
            TokenClass::Number => 4,
            TokenClass::Constant => 5,
            TokenClass::Comment => 6,
            TokenClass::Type => 7,
            TokenClass::Property => 8,
            TokenClass::Identifier => 9,
            TokenClass::Punctuation => 10,
            TokenClass::Invalid => 11,
            TokenClass::Plain => return self.text,
        };
        self.syntax[ix]
    }

    /// Heading colour for level 1..=6 (h4–h6 use the accent, FE-220).
    pub fn heading(&self, level: u8) -> Hsla {
        match level {
            1..=3 => self.headings[usize::from(level - 1)],
            _ => self.accent,
        }
    }

    /// Alert border/title colour.
    pub fn alert(&self, kind: AlertKind) -> Hsla {
        match kind {
            AlertKind::Note => self.accent,
            AlertKind::Tip => self.success,
            AlertKind::Important => self.headings[0],
            AlertKind::Warning => self.warning,
            AlertKind::Caution => self.danger,
        }
    }
}

/// Typography metrics at a zoom level.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Metrics {
    /// Body size.
    pub text: Pixels,
    /// Body line height (1.6).
    pub line_height: Pixels,
    /// Code size (0.9 em).
    pub code: Pixels,
    /// Vertical gap between blocks.
    pub gap: Pixels,
}

impl Metrics {
    pub(crate) fn at(zoom: u16) -> Self {
        let t = text_px(zoom);
        Self {
            text: px(t),
            line_height: px(t * 1.6),
            code: px(t * 0.9),
            gap: px(t * 0.8),
        }
    }

    pub(crate) fn heading(&self, level: u8) -> Pixels {
        let em = HEADING_EM[usize::from(level.clamp(1, 6) - 1)];
        self.text * em
    }
}

fn hsla(c: Rgb) -> Hsla {
    Rgba {
        r: f32::from(c.0) / 255.0,
        g: f32::from(c.1) / 255.0,
        b: f32::from(c.2) / 255.0,
        a: 1.0,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_scales_text_and_clamps() {
        assert_eq!(text_px(100), 15.0);
        assert_eq!(text_px(200), 30.0);
        assert_eq!(clamp_zoom(10), 50);
        assert_eq!(clamp_zoom(500), 200);
    }

    #[test]
    fn syntax_roles_come_from_tokens() {
        let tokens = frankmd_theme::builtin("dark").expect("dark theme");
        let colors = PreviewColors::from_tokens(&tokens);
        let keyword = hsla(tokens.get("syntax-keyword").expect("token"));
        assert_eq!(colors.syntax(TokenClass::Keyword), keyword);
        assert_eq!(colors.syntax(TokenClass::Plain), colors.text);
        assert_eq!(colors.heading(5), colors.accent);
    }
}
