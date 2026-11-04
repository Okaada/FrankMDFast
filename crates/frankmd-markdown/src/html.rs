//! HTML export of the IR (CLI `render --html`) and the **conformance oracle**
//! against FrankMD's marked + DOMPurify output (`js_preview_pipeline.json`
//! `sanitized_html`, `js_markdown_line_mapper.json`). The native preview never
//! goes through HTML; this path exists so the IR can be compared with the
//! original renderer byte-for-byte modulo the normalization documented in
//! `docs/testing/port-markdown-search.md`.

mod sanitizer;

use crate::RenderDoc;
use crate::ir::{Align, Block, Inline, ListItem, SourceSpan};

/// Serialize the IR as sanitized HTML equivalent to FrankMD's
/// `parseWithLineNumbers` output: marked's tag shapes (`<pre><code
/// class="language-x">`, `<input type="checkbox">` task items with `disabled`
/// stripped, `<span class="math-inline">`, `<div class="math-block">`,
/// `<a class="wikilink" data-wikilink-path=.. data-action="click->app#openWikilink">`),
/// with `data-source-line` on every top-level block and on every list item.
/// Like FrankMD, the assembled document goes through [`sanitize`] last (raw
/// HTML in a note can wrap the markdown that follows it).
pub fn to_html(doc: &RenderDoc) -> String {
    let mut out = String::new();
    for b in &doc.blocks {
        write_block(b, true, &mut out);
    }
    sanitize(&out)
}

/// Serialize inlines (marked `parseInline` equivalent; not sanitized).
pub fn inlines_to_html(inlines: &[Inline]) -> String {
    let mut out = String::new();
    write_inlines(inlines, &mut out);
    out
}

/// One inline exactly as FrankMD's marked extension renderers print it, e.g.
/// `<sup>x</sup>`, the bare emoji, `<svg xmlns=... class="frankmd-inline-icon"
/// aria-hidden="true"><path d=".."/></svg>`, `<span class="math-inline">..</span>`,
/// and the wikilink anchor (label = display text or target basename).
/// Text is escaped with the extensions' `escapeHtml` (`& " < >`).
pub fn render_inline(inline: &Inline) -> String {
    let mut out = String::new();
    write_inline(inline, &mut out);
    out
}

/// `<div class="math-block">TEX</div>\n` exactly as the mathBlock renderer prints it.
pub fn render_math_block(tex: &str) -> String {
    format!("<div class=\"math-block\">{}</div>\n", escape(tex))
}

/// `textContent`-like plain text of inlines (heading anchors).
pub(crate) fn plain_text(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for i in inlines {
        match i {
            Inline::Text(t)
            | Inline::Code(t)
            | Inline::Math(t)
            | Inline::Superscript(t)
            | Inline::Subscript(t)
            | Inline::Highlight(t) => s.push_str(t),
            Inline::Emph(c) | Inline::Strong(c) | Inline::Strike(c) => s.push_str(&plain_text(c)),
            Inline::Link { children, .. } => s.push_str(&plain_text(children)),
            Inline::WikiLink { target, label } => {
                s.push_str(&wikilink_display(target, label.as_deref()))
            }
            Inline::Emoji { emoji, .. } => s.push_str(emoji),
            Inline::FootnoteRef(l) => s.push_str(l),
            Inline::SoftBreak => s.push(' '),
            Inline::Icon { .. } | Inline::Image { .. } | Inline::Html(_) | Inline::HardBreak => {}
        }
    }
    s
}

/// FrankMD's extension `escapeHtml`: `& " < >`.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Display text: the label, or the target's basename (an empty label counts as absent).
fn wikilink_display(target: &str, label: Option<&str>) -> String {
    match label {
        Some(l) if !l.is_empty() => l.to_owned(),
        _ => target.rsplit('/').next().unwrap_or(target).to_owned(),
    }
}

fn line_attr(span: SourceSpan) -> String {
    format!(" data-source-line=\"{}\"", span.start_line)
}

fn write_block(b: &Block, top: bool, out: &mut String) {
    let attr = |span: SourceSpan| if top { line_attr(span) } else { String::new() };
    match b {
        Block::Heading {
            level,
            inlines,
            span,
            ..
        } => {
            out.push_str(&format!("<h{level}{}>", attr(*span)));
            write_inlines(inlines, out);
            out.push_str(&format!("</h{level}>\n"));
        }
        Block::Paragraph { inlines, span } => {
            out.push_str(&format!("<p{}>", attr(*span)));
            write_inlines(inlines, out);
            out.push_str("</p>\n");
        }
        Block::Code { lang, lines, span } => {
            let code: String = lines.iter().map(|l| format!("{}\n", l.text)).collect();
            match lang {
                Some(l) => out.push_str(&format!(
                    "<pre{}><code class=\"language-{}\">",
                    attr(*span),
                    escape(l)
                )),
                None => out.push_str(&format!("<pre{}><code>", attr(*span))),
            }
            out.push_str(&escape(&code));
            out.push_str("</code></pre>\n");
        }
        Block::Math { tex, span } => {
            out.push_str(&format!(
                "<div class=\"math-block\"{}>{}</div>\n",
                attr(*span),
                escape(tex)
            ));
        }
        Block::Quote { children, span, .. } => {
            out.push_str(&format!("<blockquote{}>\n", attr(*span)));
            for c in children {
                write_block(c, false, out);
            }
            out.push_str("</blockquote>\n");
        }
        Block::List {
            ordered,
            loose,
            items,
            span,
        } => {
            let tag = if ordered.is_some() { "ol" } else { "ul" };
            let start = match ordered {
                Some(n) if *n != 1 => format!(" start=\"{n}\""),
                _ => String::new(),
            };
            out.push_str(&format!("<{tag}{start}{}>\n", attr(*span)));
            for it in items {
                write_item(it, *loose, out);
            }
            out.push_str(&format!("</{tag}>\n"));
        }
        Block::Table {
            align,
            header,
            rows,
            span,
        } => {
            out.push_str(&format!("<table{}>\n<thead>\n<tr>\n", attr(*span)));
            for (i, cell) in header.iter().enumerate() {
                write_cell("th", align.get(i).copied(), cell, out);
            }
            out.push_str("</tr>\n</thead>\n");
            if !rows.is_empty() {
                out.push_str("<tbody>");
                for row in rows {
                    out.push_str("<tr>\n");
                    for (i, cell) in row.iter().enumerate() {
                        write_cell("td", align.get(i).copied(), cell, out);
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>");
            }
            out.push_str("</table>\n");
        }
        Block::Rule { span } => out.push_str(&format!("<hr{}>\n", attr(*span))),
        Block::Embed { html, .. } => out.push_str(html),
        Block::Html { text, .. } => out.push_str(text),
    }
}

fn write_cell(tag: &str, align: Option<Align>, cell: &[Inline], out: &mut String) {
    let align = match align {
        Some(Align::Left) => " align=\"left\"",
        Some(Align::Center) => " align=\"center\"",
        Some(Align::Right) => " align=\"right\"",
        _ => "",
    };
    out.push_str(&format!("<{tag}{align}>"));
    write_inlines(cell, out);
    out.push_str(&format!("</{tag}>\n"));
}

/// marked's `listitem`: tight items print their text bare, loose ones in `<p>`;
/// a task checkbox (clickable: no `disabled`) leads the first text.
fn write_item(it: &ListItem, loose: bool, out: &mut String) {
    out.push_str(&format!("<li{}>", line_attr(it.span)));
    let checkbox = it.task.map(|checked| {
        if checked {
            "<input checked=\"\" type=\"checkbox\">"
        } else {
            "<input type=\"checkbox\">"
        }
    });
    let mut children = it.children.iter().peekable();
    if let Some(cb) = checkbox {
        if loose {
            out.push_str("<p>");
            out.push_str(cb);
            out.push(' ');
            if let Some(Block::Paragraph { inlines, .. }) = children.peek() {
                out.push('\n');
                write_inlines(inlines, out);
                children.next();
            }
            out.push_str("</p>\n");
        } else {
            out.push_str(cb);
            out.push(' ');
        }
    }
    for c in children {
        match c {
            Block::Paragraph { inlines, .. } if !loose => write_inlines(inlines, out),
            other => write_block(other, false, out),
        }
    }
    out.push_str("</li>\n");
}

fn write_inlines(inlines: &[Inline], out: &mut String) {
    for i in inlines {
        write_inline(i, out);
    }
}

fn write_inline(inline: &Inline, out: &mut String) {
    match inline {
        Inline::Text(t) => out.push_str(&escape(t)),
        Inline::Code(t) => out.push_str(&format!("<code>{}</code>", escape(t))),
        Inline::Emph(c) => wrap("em", c, out),
        Inline::Strong(c) => wrap("strong", c, out),
        Inline::Strike(c) => wrap("del", c, out),
        Inline::Math(t) => out.push_str(&format!("<span class=\"math-inline\">{}</span>", escape(t))),
        Inline::Superscript(t) => out.push_str(&format!("<sup>{}</sup>", escape(t))),
        Inline::Subscript(t) => out.push_str(&format!("<sub>{}</sub>", escape(t))),
        Inline::Highlight(t) => out.push_str(&format!("<mark>{}</mark>", escape(t))),
        Inline::Emoji { emoji, .. } => out.push_str(emoji),
        Inline::Icon { path, view_box, .. } => out.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{}\" fill=\"currentColor\" width=\"1em\" height=\"1em\" class=\"frankmd-inline-icon\" aria-hidden=\"true\"><path d=\"{}\"/></svg>",
            escape(view_box),
            escape(path)
        )),
        Inline::Link { url, title, children } => {
            out.push_str("<a");
            if let Some(u) = url {
                out.push_str(&format!(" href=\"{}\"", escape(&u.0)));
            }
            if let Some(t) = title {
                out.push_str(&format!(" title=\"{}\"", escape(t)));
            }
            out.push('>');
            write_inlines(children, out);
            out.push_str("</a>");
        }
        Inline::WikiLink { target, label } => out.push_str(&format!(
            "<a class=\"wikilink\" data-wikilink-path=\"{}\" data-action=\"click->app#openWikilink\">{}</a>",
            escape(target),
            escape(&wikilink_display(target, label.as_deref()))
        )),
        Inline::Image { src, alt, title } => {
            out.push_str("<img");
            if let Some(s) = src {
                out.push_str(&format!(" src=\"{}\"", escape(&s.0)));
            }
            out.push_str(&format!(" alt=\"{}\"", escape(alt)));
            if let Some(t) = title {
                out.push_str(&format!(" title=\"{}\"", escape(t)));
            }
            out.push('>');
        }
        Inline::Html(h) => out.push_str(h),
        Inline::FootnoteRef(l) => out.push_str(&format!("<sup>{}</sup>", escape(l))),
        Inline::SoftBreak => out.push('\n'),
        Inline::HardBreak => out.push_str("<br>"),
    }
}

fn wrap(tag: &str, children: &[Inline], out: &mut String) {
    out.push_str(&format!("<{tag}>"));
    write_inlines(children, out);
    out.push_str(&format!("</{tag}>"));
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
    sanitizer::sanitize(html)
}
