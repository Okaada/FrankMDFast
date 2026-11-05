//! Inline IR → styled text runs.
//!
//! A run of inlines becomes a list of [`Segment`]s: rich text (one string
//! plus non-overlapping style runs, monospace ranges and clickable link
//! ranges) interleaved with things text cannot hold (math images, icons,
//! images). Raw inline HTML is shown as literal text (security: never
//! interpreted).

use crate::scale::px;
use std::ops::Range;

use frankmd_markdown::ir::Inline;
use gpui_kit::{FontStyle, FontWeight, HighlightStyle, StrikethroughStyle, UnderlineStyle};

use super::style::PreviewColors;

/// What a clickable text range does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkRef {
    /// A URL that passed the IR allowlist (classified on click).
    Url(String),
    /// `[[target]]`.
    Wiki(String),
}

/// Text with styles.
#[derive(Debug, Clone, Default)]
pub struct Rich {
    /// The string.
    pub text: String,
    /// Sorted, non-overlapping style runs.
    pub highlights: Vec<(Range<usize>, HighlightStyle)>,
    /// Sorted, non-overlapping monospace ranges.
    pub mono: Vec<Range<usize>>,
    /// Clickable ranges.
    pub links: Vec<(Range<usize>, LinkRef)>,
}

/// One piece of an inline flow.
#[derive(Debug, Clone)]
pub enum Segment {
    /// Styled text.
    Text(Rich),
    /// `$…$` math.
    Math(String),
    /// `:ph-name:` icon.
    Icon(String),
    /// An image: its document ordinal, src, alt, and the link wrapping it.
    Image {
        /// Document ordinal (focus slot, element id).
        ordinal: usize,
        /// URL as written (`None` = rejected by the IR allowlist).
        src: Option<String>,
        /// Alt text.
        alt: String,
        /// The link wrapping the image, if any.
        link: Option<LinkRef>,
    },
}

#[derive(Clone, Copy, Default)]
struct Style {
    bold: bool,
    italic: bool,
    strike: bool,
    link: bool,
}

/// Builds segments from inlines.
pub struct Builder<'a> {
    colors: &'a PreviewColors,
    segments: Vec<Segment>,
    current: Rich,
    next_image: usize,
}

impl<'a> Builder<'a> {
    /// Start a flow whose first image has document ordinal `first_image`.
    pub fn new(colors: &'a PreviewColors, first_image: usize) -> Self {
        Self {
            colors,
            segments: Vec::new(),
            current: Rich::default(),
            next_image: first_image,
        }
    }

    /// Ordinal the next image would get (to continue numbering in the next flow).
    pub fn next_image(&self) -> usize {
        self.next_image
    }

    /// Finish and return the segments.
    pub fn finish(mut self) -> Vec<Segment> {
        self.flush();
        self.segments
    }

    /// Add inlines.
    pub fn push(&mut self, inlines: &[Inline]) {
        self.inlines(inlines, Style::default(), None);
    }

    fn flush(&mut self) {
        if !self.current.text.is_empty() {
            self.segments
                .push(Segment::Text(std::mem::take(&mut self.current)));
        }
    }

    fn inlines(&mut self, inlines: &[Inline], style: Style, link: Option<&LinkRef>) {
        for inline in inlines {
            self.inline(inline, style, link);
        }
    }

    fn inline(&mut self, inline: &Inline, style: Style, link: Option<&LinkRef>) {
        let c = self.colors;
        match inline {
            Inline::Text(t) => self.text(t, style, None, link),
            Inline::Code(t) => {
                let hl = HighlightStyle {
                    background_color: Some(c.code_bg),
                    ..self.base(style)
                };
                self.mono_text(t, hl, link);
            }
            Inline::Emph(children) => self.inlines(
                children,
                Style {
                    italic: true,
                    ..style
                },
                link,
            ),
            Inline::Strong(children) => self.inlines(
                children,
                Style {
                    bold: true,
                    ..style
                },
                link,
            ),
            Inline::Strike(children) => self.inlines(
                children,
                Style {
                    strike: true,
                    ..style
                },
                link,
            ),
            Inline::Math(tex) => {
                self.flush();
                self.segments.push(Segment::Math(tex.clone()));
            }
            Inline::Superscript(t) | Inline::Subscript(t) => self.text(
                t,
                style,
                Some(HighlightStyle {
                    color: Some(c.text_secondary),
                    ..Default::default()
                }),
                link,
            ),
            Inline::Highlight(t) => self.text(
                t,
                style,
                Some(HighlightStyle {
                    background_color: Some(c.mark),
                    ..Default::default()
                }),
                link,
            ),
            Inline::Emoji { emoji, .. } => self.text(emoji, style, None, link),
            Inline::Icon { name, .. } => {
                self.flush();
                self.segments.push(Segment::Icon(name.clone()));
            }
            Inline::Link { url, children, .. } => match url {
                Some(url) => {
                    let target = LinkRef::Url(url.0.clone());
                    self.inlines(
                        children,
                        Style {
                            link: true,
                            ..style
                        },
                        Some(&target),
                    );
                }
                // DOMPurify dropped the href: keep the text, not clickable.
                None => self.inlines(children, style, link),
            },
            Inline::WikiLink { target, label } => {
                let shown = label.clone().unwrap_or_else(|| wiki_basename(target));
                let target = LinkRef::Wiki(target.clone());
                self.text(
                    &shown,
                    Style {
                        link: true,
                        ..style
                    },
                    None,
                    Some(&target),
                );
            }
            Inline::Image { src, alt, .. } => {
                self.flush();
                let ordinal = self.next_image;
                self.next_image += 1;
                self.segments.push(Segment::Image {
                    ordinal,
                    src: src.as_ref().map(|s| s.0.clone()),
                    alt: alt.clone(),
                    link: link.cloned(),
                });
            }
            Inline::Html(raw) => {
                let hl = HighlightStyle {
                    color: Some(c.muted),
                    ..self.base(style)
                };
                self.mono_text(raw, hl, link);
            }
            Inline::FootnoteRef(label) => self.text(&format!("[^{label}]"), style, None, link),
            Inline::SoftBreak => self.text(" ", style, None, link),
            Inline::HardBreak => self.text("\n", style, None, link),
        }
    }

    fn base(&self, style: Style) -> HighlightStyle {
        let c = self.colors;
        HighlightStyle {
            color: style.link.then_some(c.accent),
            font_weight: style.bold.then_some(FontWeight::BOLD),
            font_style: style.italic.then_some(FontStyle::Italic),
            underline: style.link.then_some(UnderlineStyle {
                thickness: px(1.),
                color: Some(c.accent),
                wavy: false,
            }),
            strikethrough: style.strike.then_some(StrikethroughStyle {
                thickness: px(1.),
                color: Some(c.text),
            }),
            ..Default::default()
        }
    }

    fn text(
        &mut self,
        t: &str,
        style: Style,
        extra: Option<HighlightStyle>,
        link: Option<&LinkRef>,
    ) {
        let mut hl = self.base(style);
        if let Some(extra) = extra {
            hl = hl.highlight(extra);
        }
        self.append(t, hl, false, link);
    }

    fn mono_text(&mut self, t: &str, hl: HighlightStyle, link: Option<&LinkRef>) {
        self.append(t, hl, true, link);
    }

    fn append(&mut self, t: &str, hl: HighlightStyle, mono: bool, link: Option<&LinkRef>) {
        if t.is_empty() {
            return;
        }
        let rich = &mut self.current;
        let start = rich.text.len();
        rich.text.push_str(t);
        let range = start..rich.text.len();
        if hl != HighlightStyle::default() {
            rich.highlights.push((range.clone(), hl));
        }
        if mono {
            rich.mono.push(range.clone());
        }
        if let Some(link) = link {
            match rich.links.last_mut() {
                Some((r, l)) if l == link && r.end == start => r.end = range.end,
                _ => rich.links.push((range, link.clone())),
            }
        }
    }
}

/// Wikilink display text without a label: the target's basename.
pub fn wiki_basename(target: &str) -> String {
    target.rsplit('/').next().unwrap_or(target).to_owned()
}

/// Plain text of inlines (alt text, a11y labels, table cells in tests).
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    fn walk(v: &[Inline], out: &mut String) {
        for i in v {
            match i {
                Inline::Text(t)
                | Inline::Code(t)
                | Inline::Math(t)
                | Inline::Superscript(t)
                | Inline::Subscript(t)
                | Inline::Highlight(t)
                | Inline::Html(t) => out.push_str(t),
                Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) => walk(c, out),
                Inline::Link { children, .. } => walk(children, out),
                Inline::Emoji { emoji, .. } => out.push_str(emoji),
                Inline::WikiLink { target, label } => {
                    out.push_str(label.as_deref().unwrap_or(&wiki_basename(target)))
                }
                Inline::Image { alt, .. } => out.push_str(alt),
                Inline::Icon { .. } => {}
                Inline::FootnoteRef(l) => {
                    out.push_str("[^");
                    out.push_str(l);
                    out.push(']');
                }
                Inline::SoftBreak => out.push(' '),
                Inline::HardBreak => out.push('\n'),
            }
        }
    }
    walk(inlines, &mut out);
    out
}

/// Split a rich text at word boundaries (after each run of spaces) into
/// pieces, re-basing every range, so mixed flows (text + math) can wrap.
pub fn split_words(rich: &Rich) -> Vec<Rich> {
    let mut cuts = vec![0];
    let bytes = rich.text.as_bytes();
    for i in 1..bytes.len() {
        if (bytes[i - 1] == b' ' && bytes[i] != b' ') || bytes[i] == b'\n' || bytes[i - 1] == b'\n'
        {
            cuts.push(i);
        }
    }
    cuts.push(bytes.len());
    cuts.windows(2).map(|w| slice(rich, w[0]..w[1])).collect()
}

/// The part of `rich` inside `range`, ranges re-based to 0.
pub fn slice(rich: &Rich, range: Range<usize>) -> Rich {
    let clip = |r: &Range<usize>| {
        let s = r.start.max(range.start);
        let e = r.end.min(range.end);
        (s < e).then(|| s - range.start..e - range.start)
    };
    Rich {
        text: rich.text[range.clone()].to_owned(),
        highlights: rich
            .highlights
            .iter()
            .filter_map(|(r, h)| clip(r).map(|r| (r, *h)))
            .collect(),
        mono: rich.mono.iter().filter_map(clip).collect(),
        links: rich
            .links
            .iter()
            .filter_map(|(r, l)| clip(r).map(|r| (r, l.clone())))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frankmd_markdown::parse_inline;

    fn colors() -> PreviewColors {
        PreviewColors::builtin(true)
    }

    #[test]
    fn styles_and_links_become_ranges() {
        let c = colors();
        let mut b = Builder::new(&c, 0);
        b.push(&parse_inline(
            "a **bold** [link](https://x.org) and [[dir/Note]] `code`",
        ));
        let segs = b.finish();
        let [Segment::Text(rich)] = segs.as_slice() else {
            panic!("one text segment: {segs:?}")
        };
        assert_eq!(rich.text, "a bold link and Note code");
        assert_eq!(rich.links.len(), 2);
        assert_eq!(&rich.text[rich.links[0].0.clone()], "link");
        assert_eq!(rich.links[1].1, LinkRef::Wiki("dir/Note".into()));
        assert_eq!(rich.mono, vec![21..25]);
        assert!(
            rich.highlights
                .windows(2)
                .all(|w| w[0].0.end <= w[1].0.start)
        );
    }

    #[test]
    fn raw_html_is_literal_text() {
        let c = colors();
        let mut b = Builder::new(&c, 0);
        b.push(&parse_inline("x <img src=x onerror=alert(1)> y"));
        let segs = b.finish();
        let [Segment::Text(rich)] = segs.as_slice() else {
            panic!("{segs:?}")
        };
        assert!(rich.text.contains("<img src=x onerror=alert(1)>"));
    }

    #[test]
    fn math_and_images_split_the_flow() {
        let c = colors();
        let mut b = Builder::new(&c, 3);
        b.push(&parse_inline("see $x^2$ and ![pic](p.png)"));
        let segs = b.finish();
        assert!(matches!(segs[1], Segment::Math(ref t) if t == "x^2"));
        assert!(matches!(segs[3], Segment::Image { ordinal: 3, .. }));
    }

    #[test]
    fn word_split_rebases_ranges() {
        let c = colors();
        let mut b = Builder::new(&c, 0);
        b.push(&parse_inline("one [two three](https://x.org) four"));
        let Segment::Text(rich) = &b.finish()[0] else {
            panic!()
        };
        let words = split_words(rich);
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["one ", "two ", "three ", "four"]
        );
        assert_eq!(words[1].links[0].0, 0..4);
        assert_eq!(words[2].links[0].0, 0..5);
        assert!(words[3].links.is_empty());
    }
}
