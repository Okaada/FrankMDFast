//! Front matter (shown as a metadata table, not as text).

/// Result of FrankMD's preview front-matter strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stripped<'a> {
    /// Body handed to the markdown renderer (JS `trimStart()`ed).
    pub content: &'a str,
    /// JS `frontmatterLines`: `frontmatter.split("\n").length` where the
    /// front matter includes the closing line's `\n`.
    pub frontmatter_lines: u32,
}

/// **[COMPAT]** `stripFrontmatter` from `preview_controller.js`: YAML (`---`)
/// or TOML (`+++`) at the very start, closed by the next `\n---` / `\n+++`.
/// Golden: `js_preview_pipeline.json` (`fn: stripFrontmatter`).
///
/// QUIRK: `frontmatter_lines` is one more than the front matter's line count,
/// and the body is `trimStart()`ed, so FrankMD's `data-source-line` is only
/// right when exactly one blank line follows the front matter. The IR does not
/// inherit that: [`crate::render`] computes spans from the body's true line.
pub fn strip(content: &str) -> Stripped<'_> {
    todo!()
}

/// Split `---\n...\n---\n` front matter from the body. Returns `(yaml, body, body_line_offset)`.
pub fn split(source: &str) -> (Option<&str>, &str, u32) {
    todo!()
}
