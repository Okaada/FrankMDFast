//! Math. The IR keeps TeX source; the app renders it with RaTeX
//! (`ratex-parser` → `ratex-layout` → display list) into GPUI paint calls or a
//! cached raster. See docs/research/gpui-stack.md §Math.
//!
//! **[COMPAT]** FrankMD's KaTeX options (`math_renderer.js`):
//! `{throwOnError: false, trust: false, strict: "ignore", maxExpand: 1000}`.
//! Golden: `js_math_renderer.json`.

/// Inline vs display math.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MathStyle {
    /// `$...$` (KaTeX `displayMode: false`).
    Inline,
    /// `$$...$$` (KaTeX `displayMode: true`).
    Display,
}

/// The hardened math policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathPolicy {
    /// `trust`: `\href`, `\url`, `\htmlClass`, `\htmlId`, `\htmlStyle`,
    /// `\htmlData`, `\includegraphics` are never executed when false.
    pub trust: bool,
    /// Errors render inline (source shown) instead of failing the preview.
    pub throw_on_error: bool,
    /// Macro expansion bound (DoS guard).
    pub max_expand: u32,
}

impl MathPolicy {
    /// FrankMD's options.
    pub const FRANKMD: MathPolicy = MathPolicy { trust: false, throw_on_error: false, max_expand: 1000 };
}

/// Why a TeX snippet does not render cleanly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathError {
    /// KaTeX `ParseError` (incl. "Too many expansions"): the preview shows the
    /// raw source in the error color (`katex-error`).
    Parse(String),
    /// Parsed, but these commands are unknown or untrusted (KaTeX with
    /// `throwOnError: false` renders each as red text, `color: #cc0000`).
    Unsupported(Vec<String>),
}

/// Validate TeX cheaply under [`MathPolicy::FRANKMD`] so the preview can show
/// the source on errors (KaTeX `throwOnError: false` behaviour).
pub fn validate(tex: &str, style: MathStyle) -> Result<(), MathError> {
    todo!()
}
