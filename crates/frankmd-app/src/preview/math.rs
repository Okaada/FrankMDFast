//! Math: TeX → RaTeX layout → self-contained SVG (glyph outlines embedded)
//! → GPUI `Image` (rasterized by GPUI's resvg at 2× for crispness).
//!
//! **[COMPAT]** FrankMD's KaTeX runs with `throwOnError: false, trust: false`:
//! anything that does not validate under [`MathPolicy::FRANKMD`] (parse
//! errors, unknown commands, `\href`/`\url`/`\html*`/`\includegraphics`)
//! shows its TeX source in the error colour instead of rendering.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use frankmd_markdown::math::{self, MathStyle};
use frankmd_theme::Rgb;
use gpui_kit::{Image, ImageFormat};

/// SVG user units per em. The image is laid out at `em × (text px / this)`,
/// so this only sets the rasterization detail (GPUI rasterizes SVG at 2×).
const SVG_EM: f64 = 32.0;
/// Padding around the formula, in SVG units.
const SVG_PAD: f64 = 1.0;
/// Upper bound on cached formulas per pane (past it the cache restarts).
const CACHE_LIMIT: usize = 4096;

/// A laid-out formula, in em, plus its SVG (painted black; recoloured per theme).
#[derive(Debug, Clone)]
pub struct Formula {
    /// SVG document with `rgba(0,0,0,1)` paint.
    svg: Arc<str>,
    /// Box width in em (padding included).
    pub width_em: f32,
    /// Height above the baseline in em (padding included).
    pub ascent_em: f32,
    /// Depth below the baseline in em (padding included).
    pub descent_em: f32,
}

/// Lay out `tex`. `Err` carries a short reason; the caller shows the source.
pub fn layout(tex: &str, style: MathStyle) -> Result<Formula, String> {
    math::validate(tex, style).map_err(|e| format!("{e:?}"))?;
    let ast = ratex_parser::parse(tex).map_err(|e| e.to_string())?;
    let mut opts = ratex_layout::LayoutOptions::default();
    if style == MathStyle::Inline {
        // Display → Text without naming ratex-types (numerator of display is text).
        opts.style = opts.style.numerator();
    }
    let lbox = ratex_layout::layout(&ast, &opts);
    ratex_layout::validate_layout(&lbox)?;
    let list = ratex_layout::to_display_list(&lbox);
    let svg_opts = ratex_svg::SvgOptions {
        font_size: SVG_EM,
        padding: SVG_PAD,
        stroke_width: 1.5 * SVG_EM / 40.0,
        embed_glyphs: true,
        font_dir: String::new(),
    };
    let svg = ratex_svg::render_to_svg(&list, &svg_opts);
    let pad = (SVG_PAD / SVG_EM) as f32;
    Ok(Formula {
        svg: svg.into(),
        width_em: list.width as f32 + 2.0 * pad,
        ascent_em: list.height as f32 + pad,
        descent_em: list.depth as f32 + pad,
    })
}

impl Formula {
    /// The SVG painted in `color` (black paint is the formula's default
    /// colour; explicit `\color{…}` runs keep their colour).
    pub fn image(&self, color: Rgb) -> Arc<Image> {
        let paint = format!("rgba({},{},{},1)", color.0, color.1, color.2);
        let svg = self.svg.replace("rgba(0,0,0,1)", &paint);
        Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
    }
}

/// Layouts shared between the main thread and background render jobs.
pub type SharedLayouts = Arc<Mutex<HashMap<(String, bool), Result<Formula, String>>>>;

/// Per-pane cache: `(tex, style)` → layout (shared with render jobs), and
/// `(tex, style, colour)` → themed image (main thread only).
#[derive(Default)]
pub struct MathCache {
    layouts: SharedLayouts,
    images: HashMap<(String, bool, Rgb), Arc<Image>>,
}

/// Lay out every formula of `doc` that is not cached yet (runs off the main thread).
pub fn prepare(doc: &frankmd_markdown::RenderDoc, layouts: &SharedLayouts) {
    for (tex, style) in collect(doc) {
        let key = (tex, style == MathStyle::Display);
        if lock(layouts).contains_key(&key) {
            continue;
        }
        let result = layout(&key.0, style);
        let mut map = lock(layouts);
        if map.len() >= CACHE_LIMIT {
            map.clear();
        }
        map.insert(key, result);
    }
}

fn lock(
    layouts: &SharedLayouts,
) -> std::sync::MutexGuard<'_, HashMap<(String, bool), Result<Formula, String>>> {
    layouts
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl MathCache {
    /// The layout table, for background render jobs.
    pub fn shared(&self) -> SharedLayouts {
        self.layouts.clone()
    }

    /// Layout and themed image of a formula, computing it on a miss.
    pub fn get(
        &mut self,
        tex: &str,
        style: MathStyle,
        color: Rgb,
    ) -> Result<(Formula, Arc<Image>), String> {
        let display = style == MathStyle::Display;
        let key = (tex.to_owned(), display);
        let cached = lock(&self.layouts).get(&key).cloned();
        let formula = match cached {
            Some(result) => result?,
            None => {
                let result = layout(tex, style);
                lock(&self.layouts).insert(key, result.clone());
                result?
            }
        };
        if self.images.len() >= CACHE_LIMIT {
            self.images.clear();
        }
        let image = self
            .images
            .entry((tex.to_owned(), display, color))
            .or_insert_with(|| formula.image(color))
            .clone();
        Ok((formula, image))
    }

    /// Whether a layout is cached (tests).
    pub fn contains(&self, tex: &str, style: MathStyle) -> bool {
        lock(&self.layouts).contains_key(&(tex.to_owned(), style == MathStyle::Display))
    }

    /// Drop themed images (after a theme change).
    pub fn clear_images(&mut self) {
        self.images.clear();
    }
}

/// Every formula in a document, for precomputing layouts off the main thread.
pub fn collect(doc: &frankmd_markdown::RenderDoc) -> Vec<(String, MathStyle)> {
    use frankmd_markdown::ir::{Block, Inline};
    fn inl(v: &[Inline], out: &mut Vec<(String, MathStyle)>) {
        for i in v {
            match i {
                Inline::Math(t) => out.push((t.clone(), MathStyle::Inline)),
                Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) => inl(c, out),
                Inline::Link { children, .. } => inl(children, out),
                _ => {}
            }
        }
    }
    fn blk(b: &Block, out: &mut Vec<(String, MathStyle)>) {
        match b {
            Block::Math { tex, .. } => out.push((tex.clone(), MathStyle::Display)),
            Block::Heading { inlines, .. } | Block::Paragraph { inlines, .. } => inl(inlines, out),
            Block::Quote { children, .. } => children.iter().for_each(|c| blk(c, out)),
            Block::List { items, .. } => items
                .iter()
                .flat_map(|i| &i.children)
                .for_each(|c| blk(c, out)),
            Block::Table { header, rows, .. } => {
                header.iter().for_each(|c| inl(c, out));
                rows.iter().flatten().for_each(|c| inl(c, out));
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    doc.blocks.iter().for_each(|b| blk(b, &mut out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_renders_to_path_svg() {
        let f = layout(r"\frac{a}{b} + \sqrt{x^2}", MathStyle::Display).expect("layout");
        assert!(f.width_em > 1.0 && f.ascent_em > 0.5);
        assert!(
            f.svg.contains("<path"),
            "glyphs must be outlines, not <text>"
        );
        assert!(!f.svg.contains("<text"));
        let img = f.image(Rgb(255, 0, 0));
        assert_eq!(img.format(), ImageFormat::Svg);
    }

    #[test]
    fn inline_is_smaller_than_display() {
        let d = layout(r"\sum_{i=0}^n i", MathStyle::Display).expect("display");
        let i = layout(r"\sum_{i=0}^n i", MathStyle::Inline).expect("inline");
        assert!(i.ascent_em + i.descent_em < d.ascent_em + d.descent_em);
    }

    #[test]
    fn untrusted_and_broken_tex_is_rejected() {
        assert!(layout(r"\href{javascript:alert(1)}{x}", MathStyle::Inline).is_err());
        assert!(layout(r"\frac{a", MathStyle::Inline).is_err());
        assert!(layout(r"\nosuchcommand", MathStyle::Inline).is_err());
    }
}
