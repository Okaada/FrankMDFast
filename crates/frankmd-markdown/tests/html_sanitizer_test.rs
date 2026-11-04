//! Port of `test/javascript/lib/html_sanitizer.test.js` (23 tests) against
//! [`html::sanitize`] (raw HTML inside notes; the IR itself is allowlisted by
//! construction).
#![allow(clippy::unwrap_used)]

use frankmd_markdown::html::sanitize;

/// The `<input ...>` start tags of `html`.
fn input_tags(html: &str) -> Vec<&str> {
    html.match_indices("<input")
        .map(|(i, _)| {
            let rest = &html[i..];
            &rest[..rest.find('>').map_or(rest.len(), |e| e + 1)]
        })
        .collect()
}

fn has_input_with(html: &str, needle: &str) -> bool {
    input_tags(html).iter().any(|t| t.contains(needle))
}

const TASK_LIST: &str = "<ul>\n<li><input disabled=\"\" type=\"checkbox\"> open task</li>\n<li><input checked=\"\" disabled=\"\" type=\"checkbox\"> done task</li>\n</ul>\n";

// port: test/javascript/lib/html_sanitizer.test.js:11 "keeps task checkboxes inside list items, rendered enabled for interaction"
#[test]
fn keeps_task_checkboxes_enabled() {
    let out = sanitize(TASK_LIST);
    assert!(out.contains("<li>"), "{out}");
    assert!(has_input_with(&out, r#"type="checkbox""#), "{out}");
    assert!(!has_input_with(&out, "disabled"), "{out}");
}

// port: test/javascript/lib/html_sanitizer.test.js:20 "preserves the checked state of completed tasks"
#[test]
fn preserves_checked_state() {
    assert!(has_input_with(&sanitize(TASK_LIST), "checked"));
}

// port: test/javascript/lib/html_sanitizer.test.js:25 "keeps a non-disabled checkbox with whitelisted attributes only"
#[test]
fn checkbox_keeps_only_whitelisted_attributes() {
    let out = sanitize(
        r#"<input checked="" type="checkbox" name="evil" value="1" class="x" onclick="alert(1)">"#,
    );
    assert!(has_input_with(&out, r#"type="checkbox""#), "{out}");
    assert!(has_input_with(&out, "checked"), "{out}");
    for banned in ["name=", "value=", "class=", "onclick", "disabled"] {
        assert!(!out.contains(banned), "{banned} in {out}");
    }
}

// port: test/javascript/lib/html_sanitizer.test.js:36 "removes inputs that are not checkboxes"
#[test]
fn removes_non_checkbox_inputs() {
    assert!(!sanitize(r#"<input type="text" value="x">"#).contains("<input"));
    assert!(!sanitize(r#"<input type="submit">"#).contains("<input"));
    assert!(!sanitize("<input>").contains("<input"));
}

// port: test/javascript/lib/html_sanitizer.test.js:42 "strips smuggled attributes from a kept checkbox"
#[test]
fn strips_smuggled_checkbox_attributes() {
    let out = sanitize(
        r#"<input checked="" disabled="" type="checkbox" name="evil" value="1" class="x">"#,
    );
    assert!(has_input_with(&out, r#"type="checkbox""#), "{out}");
    for banned in ["name=", "value=", "class="] {
        assert!(!out.contains(banned), "{banned} in {out}");
    }
}

// port: test/javascript/lib/html_sanitizer.test.js:50 "still forbids every other form control"
#[test]
fn forbids_other_form_controls() {
    assert!(!sanitize("<form><button>go</button></form>").contains("<form"));
    assert!(!sanitize("<button>go</button>").contains("<button"));
    assert!(!sanitize("<textarea>x</textarea>").contains("<textarea"));
    assert!(!sanitize("<select><option>1</option></select>").contains("<select"));
}

// port: test/javascript/lib/html_sanitizer.test.js:59 "removes script tags"
#[test]
fn removes_script_tags() {
    let out = sanitize("<p>hi</p><script>alert(1)</script>");
    assert!(out.contains("hi"), "{out}");
    assert!(!out.contains("<script"), "{out}");
}

// port: test/javascript/lib/html_sanitizer.test.js:65 "removes inline event handlers"
#[test]
fn removes_inline_event_handlers() {
    assert!(!sanitize(r#"<img src="x" onerror="alert(1)">"#).contains("onerror"));
    assert!(!sanitize(r#"<div onclick="alert(1)">x</div>"#).contains("onclick"));
    assert!(!sanitize(r#"<svg onload="alert(1)"></svg>"#).contains("onload"));
}

// port: test/javascript/lib/html_sanitizer.test.js:71 "removes inline styles and style elements"
#[test]
fn removes_styles() {
    assert!(!sanitize(r#"<p style="position:fixed">x</p>"#).contains("style="));
    assert!(!sanitize("<style>body { display: none }</style><p>x</p>").contains("<style"));
}

// port: test/javascript/lib/html_sanitizer.test.js:76 "removes javascript: URLs"
#[test]
fn removes_javascript_urls() {
    assert!(!sanitize(r#"<a href="javascript:alert(1)">x</a>"#).contains("javascript:"));
}

// port: test/javascript/lib/html_sanitizer.test.js:80 "removes iframe srcdoc"
#[test]
fn removes_iframe_srcdoc() {
    let out = sanitize(
        r#"<iframe src="https://www.youtube.com/embed/a" srcdoc="<script>alert(1)</script>"></iframe>"#,
    );
    assert!(!out.contains("srcdoc"), "{out}");
}

// port: test/javascript/lib/html_sanitizer.test.js:87 "keeps the YouTube embed the video dialog inserts"
#[test]
fn keeps_youtube_embed() {
    let embed = r#"<div class="embed-container"><iframe src="https://www.youtube.com/embed/dQw4w9WgXcQ" title="t" frameborder="0" allow="autoplay" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe></div>"#;
    let out = sanitize(embed);
    assert!(
        out.contains("https://www.youtube.com/embed/dQw4w9WgXcQ"),
        "{out}"
    );
    assert!(out.contains("allowfullscreen"), "{out}");
}

// port: test/javascript/lib/html_sanitizer.test.js:95 "keeps a local video embed"
#[test]
fn keeps_local_video_embed() {
    let out = sanitize(
        r#"<video controls class="video-player"><source src="/notes/videos/a.mp4" type="video/mp4"></video>"#,
    );
    assert!(out.contains("<video"), "{out}");
    assert!(out.contains("a.mp4"), "{out}");
}

// port: test/javascript/lib/html_sanitizer.test.js:103 "keeps data-source-line annotations used by scroll sync"
#[test]
fn keeps_data_source_line() {
    assert!(sanitize(r#"<p data-source-line="7">x</p>"#).contains(r#"data-source-line="7""#));
}

// port: test/javascript/lib/html_sanitizer.test.js:107 "keeps wikilink attributes used by the app controller"
#[test]
fn keeps_wikilink_attributes() {
    let out = sanitize(
        r#"<a class="wikilink" data-wikilink-path="Note.md" data-action="click->app#openWikilink">Note</a>"#,
    );
    assert!(out.contains(r#"data-wikilink-path="Note.md""#), "{out}");
    assert!(out.contains("openWikilink"), "{out}");
}

// port: test/javascript/lib/html_sanitizer.test.js:115 "keeps ordinary markdown output"
#[test]
fn keeps_ordinary_markdown_output() {
    let out = sanitize(
        r#"<h1>T</h1><pre><code class="language-js">const a = 1</code></pre><table><tr><td>c</td></tr></table>"#,
    );
    assert!(out.contains("<h1>"), "{out}");
    assert!(out.contains("language-js"), "{out}");
    assert!(out.contains("<table"), "{out}");
}

// port: test/javascript/lib/html_sanitizer.test.js:126 "drops an iframe from a host that is not allow-listed"
#[test]
fn drops_iframe_from_unlisted_host() {
    assert!(
        !sanitize(r#"<iframe src="https://evil.example/x"></iframe>"#).contains("evil.example")
    );
}

// port: test/javascript/lib/html_sanitizer.test.js:131 "drops an http (non-https) iframe"
#[test]
fn drops_http_iframe() {
    assert!(
        !sanitize(r#"<iframe src="http://www.youtube.com/embed/a"></iframe>"#)
            .contains("youtube.com")
    );
}

// port: test/javascript/lib/html_sanitizer.test.js:136 "drops an iframe with no src"
#[test]
fn drops_iframe_without_src() {
    assert!(!sanitize("<iframe></iframe>").contains("<iframe"));
}

// port: test/javascript/lib/html_sanitizer.test.js:140 "allows youtube-nocookie and vimeo players"
#[test]
fn allows_nocookie_and_vimeo() {
    assert!(
        sanitize(r#"<iframe src="https://www.youtube-nocookie.com/embed/a"></iframe>"#)
            .contains("youtube-nocookie")
    );
    assert!(
        sanitize(r#"<iframe src="https://player.vimeo.com/video/1"></iframe>"#)
            .contains("player.vimeo.com")
    );
}

// port: test/javascript/lib/html_sanitizer.test.js:145 "defeats host-confusion tricks on the iframe allowlist"
#[test]
fn defeats_host_confusion() {
    assert!(
        !sanitize(r#"<iframe src="https://www.youtube.com@evil.com/embed/a"></iframe>"#)
            .contains("evil.com")
    );
    assert!(
        !sanitize(r#"<iframe src="https://www.youtube.com.evil.com/embed/a"></iframe>"#)
            .contains("evil.com")
    );
    assert!(!sanitize(r#"<iframe src="//evil.com/embed/a"></iframe>"#).contains("evil.com"));
}

// port: test/javascript/lib/html_sanitizer.test.js:156 "removes form and input tags but keeps their text content"
#[test]
fn strips_form_input_button() {
    let out =
        sanitize(r#"<form action="https://evil.com"><input name="p"><button>Go</button></form>"#);
    for banned in ["<form", "<input", "<button"] {
        assert!(!out.contains(banned), "{banned} in {out}");
    }
}

// port: test/javascript/lib/html_sanitizer.test.js:164 "returns an empty string for empty input"
// (null/undefined have no Rust equivalent; "" covers the falsy branch.)
#[test]
fn empty_input_returns_empty() {
    assert_eq!(sanitize(""), "");
}
