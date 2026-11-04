//! Port of `test/javascript/lib/marked_extensions.test.js` (44 tests).
//! `start()`/`tokenizer()` map to `parse::ext`, `renderer()` to
//! `html::render_inline` / `html::render_math_block`.
#![allow(clippy::unwrap_used)]

use frankmd_markdown::html::{render_inline, render_math_block};
use frankmd_markdown::ir::Inline;
use frankmd_markdown::parse::ext::{self, Extension};

fn wikilink(target: &str, label: Option<&str>) -> String {
    render_inline(&Inline::WikiLink { target: target.into(), label: label.map(Into::into) })
}

// ---- wikilinkExtension ----------------------------------------------------

// port: test/javascript/lib/marked_extensions.test.js:9 "returns index of [[ in source"
#[test]
fn wikilink_start_returns_index_of_brackets() {
    assert_eq!(ext::start(Extension::Wikilink, "hello [[world]]"), Some(6));
}

// port: test/javascript/lib/marked_extensions.test.js:13 "returns -1 when no [[ found"
#[test]
fn wikilink_start_none_without_brackets() {
    assert_eq!(ext::start(Extension::Wikilink, "no wikilinks here"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:19 "matches simple wikilink [[Note Name]]"
#[test]
fn wikilink_tokenizer_simple() {
    let t = ext::scan_wikilink("[[My Note]] rest").expect("token");
    assert_eq!(&"[[My Note]] rest"[..t.raw_len], "[[My Note]]");
    assert_eq!(t.value.target, "My Note");
    assert_eq!(t.value.display, None);
}

// port: test/javascript/lib/marked_extensions.test.js:28 "matches wikilink with display text [[Note|Display]]"
#[test]
fn wikilink_tokenizer_display_text() {
    let t = ext::scan_wikilink("[[My Note|Custom Label]] rest").expect("token");
    assert_eq!(t.value.target, "My Note");
    assert_eq!(t.value.display.as_deref(), Some("Custom Label"));
}

// port: test/javascript/lib/marked_extensions.test.js:35 "matches wikilink with path [[folder/Note]]"
#[test]
fn wikilink_tokenizer_path() {
    let t = ext::scan_wikilink("[[projects/My Note]] rest").expect("token");
    assert_eq!(t.value.target, "projects/My Note");
    assert_eq!(t.value.display, None);
}

// port: test/javascript/lib/marked_extensions.test.js:42 "trims whitespace from target and display text"
#[test]
fn wikilink_tokenizer_trims() {
    let t = ext::scan_wikilink("[[  My Note  |  Label  ]] rest").expect("token");
    assert_eq!(t.value.target, "My Note");
    assert_eq!(t.value.display.as_deref(), Some("Label"));
}

// port: test/javascript/lib/marked_extensions.test.js:48 "returns undefined for non-matching input"
#[test]
fn wikilink_tokenizer_non_matching() {
    assert_eq!(ext::scan_wikilink("not a wikilink"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:52 "returns undefined for unclosed brackets"
#[test]
fn wikilink_tokenizer_unclosed() {
    assert_eq!(ext::scan_wikilink("[[unclosed"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:58 "renders simple wikilink as anchor tag"
#[test]
fn wikilink_renderer_simple() {
    let html = wikilink("My Note", None);
    assert!(html.contains(r#"class="wikilink""#), "{html}");
    assert!(html.contains(r#"data-wikilink-path="My Note""#), "{html}");
    assert!(html.contains(r#"data-action="click->app#openWikilink""#), "{html}");
    assert!(html.contains(">My Note</a>"), "{html}");
}

// port: test/javascript/lib/marked_extensions.test.js:66 "renders wikilink with custom display text"
#[test]
fn wikilink_renderer_display_text() {
    let html = wikilink("My Note", Some("Click here"));
    assert!(html.contains(r#"data-wikilink-path="My Note""#), "{html}");
    assert!(html.contains(">Click here</a>"), "{html}");
}

// port: test/javascript/lib/marked_extensions.test.js:72 "renders path-based wikilink showing only basename"
#[test]
fn wikilink_renderer_basename() {
    let html = wikilink("projects/My Note", None);
    assert!(html.contains(r#"data-wikilink-path="projects/My Note""#), "{html}");
    assert!(html.contains(">My Note</a>"), "{html}");
}

// port: test/javascript/lib/marked_extensions.test.js:78 "escapes HTML in display text"
#[test]
fn wikilink_renderer_escapes_display() {
    let html = wikilink("Note", Some("<script>alert(1)</script>"));
    assert!(!html.contains("<script>"), "{html}");
    assert!(html.contains("&lt;script&gt;"), "{html}");
}

// port: test/javascript/lib/marked_extensions.test.js:84 "escapes ampersands in target and display text"
#[test]
fn wikilink_renderer_escapes_ampersands() {
    let html = wikilink("Notes & Ideas", Some("R&D Notes"));
    assert!(html.contains(r#"data-wikilink-path="Notes &amp; Ideas""#), "{html}");
    assert!(html.contains(">R&amp;D Notes</a>"), "{html}");
}

// port: test/javascript/lib/marked_extensions.test.js:90 "escapes quotes in target path"
#[test]
fn wikilink_renderer_escapes_quotes() {
    let html = wikilink(r#"Note "with" quotes"#, None);
    assert!(html.contains("&quot;"), "{html}");
    assert!(!html.contains(r#"path="Note "with""#), "{html}");
}

// ---- iconExtension --------------------------------------------------------

// port: test/javascript/lib/marked_extensions.test.js:100 "returns index of :ph- in source"
#[test]
fn icon_start_returns_index() {
    assert_eq!(ext::start(Extension::Icon, "hello :ph-heart: world"), Some(6));
}

// port: test/javascript/lib/marked_extensions.test.js:104 "returns -1 when no :ph- prefix found"
#[test]
fn icon_start_none_without_prefix() {
    assert_eq!(ext::start(Extension::Icon, "no icons here, just :smile:"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:110 "matches a known icon shortcode"
#[test]
fn icon_tokenizer_known_icon() {
    let t = ext::scan_icon(":ph-heart: rest").expect("token");
    assert_eq!(&":ph-heart: rest"[..t.raw_len], ":ph-heart:");
    assert!(!t.value.path.is_empty());
    assert_eq!(t.value.view_box, "0 0 256 256");
}

// port: test/javascript/lib/marked_extensions.test.js:120 "returns undefined for an unknown icon name"
#[test]
fn icon_tokenizer_unknown_icon() {
    assert_eq!(ext::scan_icon(":ph-not-a-real-icon:"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:124 "returns undefined for a plain emoji shortcode"
#[test]
fn icon_tokenizer_plain_emoji() {
    assert_eq!(ext::scan_icon(":smile:"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:128 "returns undefined for non-matching input"
#[test]
fn icon_tokenizer_non_matching() {
    assert_eq!(ext::scan_icon("not an icon"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:134 "renders an inline SVG using the token's path and viewBox"
#[test]
fn icon_renderer_inline_svg() {
    let html =
        render_inline(&Inline::Icon { name: "x".into(), path: "M1 2 3".into(), view_box: "0 0 256 256".into() });
    for needle in [
        "<svg",
        r#"viewBox="0 0 256 256""#,
        r#"fill="currentColor""#,
        r#"width="1em""#,
        r#"height="1em""#,
        r#"<path d="M1 2 3"/>"#,
        r#"class="frankmd-inline-icon""#,
    ] {
        assert!(html.contains(needle), "{needle} not in {html}");
    }
    assert!(!html.contains(r#"style=""#), "{html}");
}

// ---- mathInlineExtension --------------------------------------------------

// port: test/javascript/lib/marked_extensions.test.js:151 "returns the index of the first unescaped $"
#[test]
fn math_inline_start_first_unescaped_dollar() {
    assert_eq!(ext::start(Extension::MathInline, "The equation is $E=mc^2$"), Some(16));
}

// port: test/javascript/lib/marked_extensions.test.js:155 "ignores an escaped \\$ (currency stays literal)"
#[test]
fn math_inline_start_ignores_escaped_dollar() {
    assert_eq!(ext::start(Extension::MathInline, r"costs \$5 today"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:159 "returns undefined when there is no $"
#[test]
fn math_inline_start_none_without_dollar() {
    assert_eq!(ext::start(Extension::MathInline, "no dollars here"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:165 "matches simple inline math"
#[test]
fn math_inline_tokenizer_simple() {
    let t = ext::scan_math_inline("$E = mc^2$ rest").expect("token");
    assert_eq!(t.value, "E = mc^2");
    assert_eq!(&"$E = mc^2$ rest"[..t.raw_len], "$E = mc^2$");
}

// port: test/javascript/lib/marked_extensions.test.js:173 "matches a single-token expression"
#[test]
fn math_inline_tokenizer_single_token() {
    assert_eq!(ext::scan_math_inline("$x$").expect("token").value, "x");
}

// port: test/javascript/lib/marked_extensions.test.js:177 "does NOT match a lone currency value ($10.)"
#[test]
fn math_inline_rejects_lone_currency() {
    assert_eq!(ext::scan_math_inline("$10."), None);
}

// port: test/javascript/lib/marked_extensions.test.js:181 "does NOT match two currency values in a sentence"
#[test]
fn math_inline_rejects_two_currency_values() {
    assert_eq!(ext::scan_math_inline("$10 and that costs $20."), None);
}

// port: test/javascript/lib/marked_extensions.test.js:185 "does NOT match when the closing $ is followed by a digit"
#[test]
fn math_inline_rejects_closing_dollar_before_digit() {
    assert_eq!(ext::scan_math_inline("$100 to $200"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:189 "rejects a space right after the opening $"
#[test]
fn math_inline_rejects_space_after_open() {
    assert_eq!(ext::scan_math_inline("$ x$"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:193 "rejects a space right before the closing $"
#[test]
fn math_inline_rejects_space_before_close() {
    assert_eq!(ext::scan_math_inline("$x $"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:197 "does NOT swallow a $$ block delimiter"
#[test]
fn math_inline_rejects_block_delimiter() {
    assert_eq!(ext::scan_math_inline("$$block$$"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:201 "stops at the first valid closing $ so adjacent expressions are independent"
#[test]
fn math_inline_stops_at_first_close() {
    let t = ext::scan_math_inline("$a$ and $b$").expect("token");
    assert_eq!(t.raw_len, "$a$".len());
    assert_eq!(t.value, "a");
}

// port: test/javascript/lib/marked_extensions.test.js:207 "keeps escaped \\$ inside the expression"
#[test]
fn math_inline_keeps_escaped_dollar() {
    let t = ext::scan_math_inline(r"$a \$ b$ rest").expect("token");
    assert_eq!(t.value, r"a \$ b");
}

// port: test/javascript/lib/marked_extensions.test.js:215 "emits a class-tagged span with the TeX as text, no style"
#[test]
fn math_inline_renderer_span() {
    let html = render_inline(&Inline::Math("E = mc^2".into()));
    assert_eq!(html, r#"<span class="math-inline">E = mc^2</span>"#);
    assert!(!html.contains("style"));
}

// port: test/javascript/lib/marked_extensions.test.js:221 "HTML-escapes the TeX so it cannot inject markup"
#[test]
fn math_inline_renderer_escapes() {
    let html = render_inline(&Inline::Math("<img src=x onerror=alert(1)>".into()));
    assert!(!html.contains("<img"), "{html}");
    assert!(html.contains("&lt;img"), "{html}");
}

// ---- mathBlockExtension ---------------------------------------------------

// port: test/javascript/lib/marked_extensions.test.js:231 "returns the index of $$"
#[test]
fn math_block_start_index() {
    assert_eq!(ext::start(Extension::MathBlock, "text\n$$\nx=y\n$$"), Some(5));
}

// port: test/javascript/lib/marked_extensions.test.js:235 "returns undefined without $$"
#[test]
fn math_block_start_none() {
    assert_eq!(ext::start(Extension::MathBlock, "no block here"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:241 "matches a multiline display equation"
#[test]
fn math_block_tokenizer_multiline() {
    let t = ext::scan_math_block("$$\nx = \\frac{-b}{2a}\n$$\nmore").expect("token");
    assert_eq!(t.value, r"x = \frac{-b}{2a}");
}

// port: test/javascript/lib/marked_extensions.test.js:248 "matches a single-line $$...$$"
#[test]
fn math_block_tokenizer_single_line() {
    assert_eq!(ext::scan_math_block("$$E=mc^2$$").expect("token").value, "E=mc^2");
}

// port: test/javascript/lib/marked_extensions.test.js:252 "does NOT match an empty/whitespace block (malformed)"
#[test]
fn math_block_rejects_whitespace_block() {
    assert_eq!(ext::scan_math_block("$$   $$"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:256 "does NOT match an unterminated block"
#[test]
fn math_block_rejects_unterminated() {
    assert_eq!(ext::scan_math_block("$$\nx = y\n"), None);
}

// port: test/javascript/lib/marked_extensions.test.js:262 "emits a class-tagged div with the TeX as text, no style"
#[test]
fn math_block_renderer_div() {
    let html = render_math_block("x = y");
    assert_eq!(html, "<div class=\"math-block\">x = y</div>\n");
    assert!(!html.contains("style"));
}

// port: test/javascript/lib/marked_extensions.test.js:268 "HTML-escapes the TeX"
#[test]
fn math_block_renderer_escapes() {
    let html = render_math_block("</div><script>alert(1)</script>");
    assert!(!html.contains("<script>"), "{html}");
    assert!(html.contains("&lt;script&gt;"), "{html}");
}
