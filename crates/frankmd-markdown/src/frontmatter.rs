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
    match locate(content) {
        Some(Located { body_start: Some(start), .. }) => Stripped {
            content: crate::marked::js_trim_start(&content[start..]),
            frontmatter_lines: line_count(&content[..start]),
        },
        Some(Located { body_start: None, .. }) => Stripped { content: "", frontmatter_lines: line_count(content) },
        None => Stripped { content, frontmatter_lines: 0 },
    }
}

/// Where `stripFrontmatter` finds the front matter.
struct Located {
    /// Index of the `\n` that starts the closing delimiter line.
    close: usize,
    /// Start of the body (after the closing line's `\n`); `None` when the
    /// closing delimiter is the last line.
    body_start: Option<usize>,
}

fn locate(content: &str) -> Option<Located> {
    for delim in ["---", "+++"] {
        if !content.starts_with(delim) {
            continue;
        }
        let needle = format!("\n{delim}");
        let Some(close) = content.get(3..).and_then(|s| s.find(&needle)).map(|i| i + 3) else { continue };
        let after = content.get(close + 4..).and_then(|s| s.find('\n')).map(|i| i + close + 4);
        return Some(Located { close, body_start: after.map(|a| a + 1) });
    }
    None
}

/// JS `s.split("\n").length`.
fn line_count(s: &str) -> u32 {
    crate::marked::newlines(s) + 1
}

/// Split `---`/`+++` front matter (located like [`strip`]) from the body.
/// Returns `(front matter text between the delimiter lines, untrimmed body,
/// number of lines before the body)`.
pub fn split(source: &str) -> (Option<&str>, &str, u32) {
    let Some(loc) = locate(source) else { return (None, source, 0) };
    let inner_start = source.find('\n').map_or(source.len(), |i| i + 1).min(loc.close);
    let inner = &source[inner_start..loc.close];
    match loc.body_start {
        Some(start) => (Some(inner), &source[start..], crate::marked::newlines(&source[..start])),
        None => (Some(inner), "", crate::marked::newlines(source) + 1),
    }
}

/// The front matter block itself (delimiters included), if the note has one.
pub(crate) fn block(source: &str) -> Option<&str> {
    let loc = locate(source)?;
    Some(match loc.body_start {
        Some(start) => &source[..start],
        None => source,
    })
}
