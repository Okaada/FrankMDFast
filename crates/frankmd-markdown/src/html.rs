//! HTML export of the IR (CLI `render --html`) and the **conformance oracle**
//! against FrankMD's marked + DOMPurify output (`js_preview_pipeline.json`
//! `sanitized_html`, `js_markdown_line_mapper.json`). The native preview never
//! goes through HTML; this path exists so the IR can be compared with the
//! original renderer byte-for-byte modulo the normalization documented in
//! `docs/testing/port-markdown-search.md`.

use crate::RenderDoc;
use crate::ir::Inline;

/// Serialize the IR as sanitized HTML equivalent to FrankMD's
/// `parseWithLineNumbers` output: marked's tag shapes (`<pre><code
/// class="language-x">`, `<input type="checkbox">` task items with `disabled`
/// stripped, `<span class="math-inline">`, `<div class="math-block">`,
/// `<a class="wikilink" data-wikilink-path=.. data-action="click->app#openWikilink">`),
/// with `data-source-line` on every top-level block and on every list item.
pub fn to_html(doc: &RenderDoc) -> String {
    todo!()
}

/// Serialize inlines (marked `parseInline` equivalent).
pub fn inlines_to_html(inlines: &[Inline]) -> String {
    todo!()
}

/// One inline exactly as FrankMD's marked extension renderers print it, e.g.
/// `<sup>x</sup>`, the bare emoji, `<svg xmlns=... class="frankmd-inline-icon"
/// aria-hidden="true"><path d=".."/></svg>`, `<span class="math-inline">..</span>`,
/// and the wikilink anchor (label = display text or target basename).
/// Text is escaped with the extensions' `escapeHtml` (`& " < >`).
pub fn render_inline(inline: &Inline) -> String {
    todo!()
}

/// `<div class="math-block">TEX</div>\n` exactly as the mathBlock renderer prints it.
pub fn render_math_block(tex: &str) -> String {
    todo!()
}

/// **[COMPAT]** FrankMD's `sanitizeHtml` (DOMPurify 3 defaults plus
/// `html_sanitizer.js`): iframes only from `https://` `www.youtube.com`,
/// `youtube.com`, `www.youtube-nocookie.com`, `youtube-nocookie.com`,
/// `player.vimeo.com`; `allow`/`allowfullscreen`/`frameborder`/`referrerpolicy`
/// allowed, `srcdoc` never; `form button textarea select option style` and
/// every `style` attribute removed; `<input>` kept only as `type="checkbox"`
/// with just `type`/`checked`; no `on*` handlers or `javascript:` URLs.
/// Empty input → `""`. Used for raw HTML in notes ([`crate::ir::Block::Html`],
/// [`Inline::Html`]).
pub fn sanitize(html: &str) -> String {
    todo!()
}
