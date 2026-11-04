//! DOMPurify-equivalent sanitizer (see [`super::sanitize`]).
//!
//! **[COMPAT]** A port of the parts of DOMPurify 3.4.17 that FrankMD's
//! `sanitizeHtml` exercises (`html_sanitizer.js`): default HTML + SVG +
//! MathML profiles, `SAFE_FOR_XML`, `SANITIZE_DOM`, `KEEP_CONTENT`, plus the
//! config (`ADD_TAGS: iframe`, `ADD_ATTR`, `FORBID_TAGS`, `FORBID_ATTR`) and
//! the `uponSanitizeElement` hook (checkbox-only `<input>`, iframe host
//! allowlist). Golden: `js_html_sanitizer.json`.
//!
//! Why not ammonia: DOMPurify's results depend on its exact algorithm
//! (document parse with scripting disabled, head content dropped, children
//! of removed elements hoisted and re-checked against their new parent's
//! namespace, element removal decided by attribute values). ammonia parses
//! a `<div>` fragment with scripting enabled and cannot drop an element based
//! on an attribute, so it cannot reproduce those outputs. The tree work runs
//! on html5ever, the parser ammonia itself uses.

mod dom;
mod tables;

use std::collections::HashSet;
use std::sync::LazyLock;

use dom::{Dom, Id, Kind, Ns};

/// Hosts allowed as `<iframe src>` (`ALLOWED_EMBED_HOSTS`).
const ALLOWED_EMBED_HOSTS: &[&str] =
    &["www.youtube.com", "youtube.com", "www.youtube-nocookie.com", "youtube-nocookie.com", "player.vimeo.com"];

/// `FORBID_TAGS` from FrankMD's config.
const FORBID_TAGS: &[&str] = &["form", "button", "textarea", "select", "option", "style"];

/// `ADD_ATTR` from FrankMD's config.
const ADD_ATTR: &[&str] = &["allow", "allowfullscreen", "frameborder", "referrerpolicy"];

/// Base URL relative iframe sources resolve against (`window.location.href`).
const BASE_URL: &str = "http://localhost:3000/";

/// DOMPurify `LITERAL_TEXT_ELEMENTS`.
const LITERAL_TEXT_ELEMENTS: &[&str] =
    &["style", "script", "xmp", "iframe", "noembed", "noframes", "plaintext", "noscript"];

/// DOMPurify `MATHML_TEXT_INTEGRATION_POINTS`.
const MATHML_TEXT_INTEGRATION_POINTS: &[&str] = &["mi", "mo", "mn", "ms", "mtext"];

/// DOMPurify `COMMON_SVG_AND_HTML_ELEMENTS`.
const COMMON_SVG_AND_HTML_ELEMENTS: &[&str] = &["title", "style", "font", "a", "script"];

type Set = HashSet<&'static str>;

fn set(lists: &[&[&'static str]]) -> Set {
    lists.iter().flat_map(|l| l.iter().copied()).collect()
}

/// `ALLOWED_TAGS`: the default profiles + `#text`, `ADD_TAGS` and `tbody`.
static ALLOWED_TAGS: LazyLock<Set> = LazyLock::new(|| {
    set(&[tables::HTML_TAGS, tables::SVG_TAGS, tables::SVG_FILTERS, tables::MATHML_TAGS, &["#text", "iframe", "tbody"]])
});

/// `ALLOWED_ATTR`: the default attribute profiles + `ADD_ATTR`.
static ALLOWED_ATTR: LazyLock<Set> = LazyLock::new(|| {
    set(&[tables::HTML_ATTRS, tables::SVG_ATTRS, tables::MATHML_ATTRS, tables::XML_ATTRS, ADD_ATTR])
});

static ALL_SVG_TAGS: LazyLock<Set> =
    LazyLock::new(|| set(&[tables::SVG_TAGS, tables::SVG_FILTERS, tables::SVG_DISALLOWED]));

static ALL_MATHML_TAGS: LazyLock<Set> = LazyLock::new(|| set(&[tables::MATHML_TAGS, tables::MATHML_DISALLOWED]));

pub(super) fn sanitize(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }
    // DOMPurify returns markup-free strings untouched (no parse, no escaping).
    if !html.contains('<') {
        return html.to_owned();
    }
    let mut dom = dom::parse_document(html);
    let Some(body) = dom.body() else { return String::new() };
    // The document parser drops leading whitespace; DOMPurify puts it back.
    let rest = html.trim_start_matches(['\r', '\n', '\t', ' ']);
    let leading = &html[..html.len() - rest.len()];
    if !leading.is_empty() {
        let text = dom.new_text(leading);
        dom.prepend(body, text);
    }
    walk(&mut dom, body);
    dom.inner_html(body)
}

/// DOMPurify's `NodeIterator` loop: visit every node under `root` in tree
/// order, sanitizing the node and then its attributes. A removed node's
/// hoisted children (inserted where it was) are visited next.
fn walk(dom: &mut Dom, root: Id) {
    let mut current = dom.first_child(root);
    while let Some(node) = current {
        let prev = dom.prev_sibling(node);
        let parent = dom.parent(node).unwrap_or(root);
        if sanitize_node(dom, node) {
            // Resume after the node that preceded the removed one.
            current = match prev {
                Some(p) => next_skipping_children(dom, p, root),
                None => dom.first_child(parent).or_else(|| next_skipping_children(dom, parent, root)),
            };
        } else {
            sanitize_attributes(dom, node);
            current = dom.first_child(node).or_else(|| next_skipping_children(dom, node, root));
        }
    }
}

/// The node after `node`'s subtree in tree order, staying inside `root`.
fn next_skipping_children(dom: &Dom, mut node: Id, root: Id) -> Option<Id> {
    loop {
        if node == root {
            return None;
        }
        if let Some(next) = dom.next_sibling(node) {
            return Some(next);
        }
        node = dom.parent(node)?;
    }
}

/// `nodeName` lowercased (`transformCaseFunc`), `None` for nodes that are not
/// text, comment or element.
fn node_name(dom: &Dom, id: Id) -> Option<String> {
    match dom.kind(id) {
        Kind::Text(_) => Some("#text".to_owned()),
        Kind::Comment(_) => Some("#comment".to_owned()),
        Kind::Element(e) => Some(e.name.local.to_string().to_ascii_lowercase()),
        Kind::Document => None,
    }
}

/// `_sanitizeElements`. Returns true when the node was removed.
fn sanitize_node(dom: &mut Dom, id: Id) -> bool {
    let Some(tag) = node_name(dom, id) else {
        dom.detach(id);
        return true;
    };
    if frankmd_element_hook(dom, id, &tag) {
        dom.detach(id);
        return true;
    }
    if is_unsafe_node(dom, id, &tag) {
        dom.detach(id);
        return true;
    }
    if FORBID_TAGS.contains(&tag.as_str()) || !ALLOWED_TAGS.contains(tag.as_str()) {
        // KEEP_CONTENT: hoist the children into the node's place.
        if !tables::FORBID_CONTENTS.contains(&tag.as_str()) {
            for child in dom.children(id) {
                dom.insert_before(id, child);
            }
        }
        dom.detach(id);
        return true;
    }
    if dom.elem(id).is_some() && !has_valid_namespace(dom, id, &tag) {
        dom.detach(id);
        return true;
    }
    if matches!(tag.as_str(), "noscript" | "noembed" | "noframes") {
        let inner = dom.inner_html(id).to_ascii_lowercase();
        if ["</noscript", "</noembed", "</noframes"].iter().any(|t| inner.contains(t)) {
            dom.detach(id);
            return true;
        }
    }
    false
}

/// FrankMD's `uponSanitizeElement` hook. Returns true when it removes the node.
fn frankmd_element_hook(dom: &mut Dom, id: Id, tag: &str) -> bool {
    match tag {
        "input" => {
            if get_attribute(dom, id, "type") != Some("checkbox") {
                return true;
            }
            // A kept checkbox keeps only `type` and `checked` (#203).
            if let Some(elem) = dom.elem_mut(id) {
                elem.attrs.retain(|a| {
                    let name = dom::attr_name(a);
                    name == "type" || name == "checked"
                });
            }
            false
        }
        "iframe" => !is_allowed_embed(get_attribute(dom, id, "src").unwrap_or("")),
        _ => false,
    }
}

/// `new URL(src, location.href)` is https and its host is allowlisted.
fn is_allowed_embed(src: &str) -> bool {
    let Ok(base) = url::Url::parse(BASE_URL) else { return false };
    let Ok(url) = base.join(src) else { return false };
    url.scheme() == "https" && url.host_str().is_some_and(|h| ALLOWED_EMBED_HOSTS.contains(&h))
}

/// `element.getAttribute(name)` (first attribute with that qualified name).
fn get_attribute<'a>(dom: &'a Dom, id: Id, name: &str) -> Option<&'a str> {
    let elem = dom.elem(id)?;
    elem.attrs.iter().find(|a| dom::attr_name(a).eq_ignore_ascii_case(name)).map(|a| &*a.value)
}

/// DOMPurify `ELEMENT_MARKUP_PROBE` `/<[/\w!]/`.
fn has_element_markup(s: &str) -> bool {
    markup_probe(s, true)
}

/// `<` followed by `/`, an ASCII word char, or (when `bang`) `!`.
fn markup_probe(s: &str, bang: bool) -> bool {
    s.as_bytes()
        .windows(2)
        .any(|w| w[0] == b'<' && (w[1] == b'/' || w[1] == b'_' || w[1].is_ascii_alphanumeric() || (bang && w[1] == b'!')))
}

/// `_isUnsafeNode`.
fn is_unsafe_node(dom: &Dom, id: Id, tag: &str) -> bool {
    match dom.kind(id) {
        Kind::Comment(data) => markup_probe(data, false),
        Kind::Element(elem) => {
            let has_children = dom.first_child(id).is_some();
            let has_element_child = dom.has_element_child(id);
            if has_children
                && !has_element_child
                && has_element_markup(&dom.text_content(id))
                && has_element_markup(&dom.inner_html(id))
            {
                return true;
            }
            LITERAL_TEXT_ELEMENTS.contains(&tag)
                && Ns::of(&elem.name.ns) == Ns::Html
                && (has_element_child || has_element_markup(&dom.text_content(id)))
        }
        _ => false,
    }
}

/// `_checkValidNamespace`: could a spec-compliant parser have produced this
/// element under its current parent?
fn has_valid_namespace(dom: &Dom, id: Id, tag: &str) -> bool {
    let Some(elem) = dom.elem(id) else { return true };
    let (parent_ns, parent_tag) = match dom.parent(id).and_then(|p| dom.elem(p)) {
        Some(parent) => (Ns::of(&parent.name.ns), parent.name.local.to_string().to_ascii_lowercase()),
        None => (Ns::Html, "template".to_owned()),
    };
    let parent_tag = parent_tag.as_str();
    let html_integration_point = parent_tag == "annotation-xml";
    let mathml_text_point = MATHML_TEXT_INTEGRATION_POINTS.contains(&parent_tag);
    match Ns::of(&elem.name.ns) {
        Ns::Svg => match parent_ns {
            Ns::Html => tag == "svg",
            Ns::MathMl => tag == "svg" && (html_integration_point || mathml_text_point),
            _ => ALL_SVG_TAGS.contains(tag),
        },
        Ns::MathMl => match parent_ns {
            Ns::Html => tag == "math",
            Ns::Svg => tag == "math" && html_integration_point,
            _ => ALL_MATHML_TAGS.contains(tag),
        },
        Ns::Html => {
            if parent_ns == Ns::Svg && !html_integration_point {
                return false;
            }
            if parent_ns == Ns::MathMl && !mathml_text_point {
                return false;
            }
            !ALL_MATHML_TAGS.contains(tag)
                && (COMMON_SVG_AND_HTML_ELEMENTS.contains(&tag) || !ALL_SVG_TAGS.contains(tag))
        }
        Ns::Other => false,
    }
}

/// `_sanitizeAttributes`.
fn sanitize_attributes(dom: &mut Dom, id: Id) {
    let Some(elem) = dom.elem_mut(id) else { return };
    let lc_tag = elem.name.local.to_string().to_ascii_lowercase();
    let mut kept = Vec::with_capacity(elem.attrs.len());
    for mut attr in std::mem::take(&mut elem.attrs) {
        let name = dom::attr_name(&attr).into_owned();
        let lc_name = name.to_ascii_lowercase();
        let value = if name == "value" { attr.value.to_string() } else { js_trim(&attr.value).to_owned() };
        let keep = !is_unsafe_attribute_value(&name, &value) && is_valid_attribute(&lc_tag, &lc_name, &value);
        if keep {
            if *attr.value != *value {
                attr.value = value.into();
            }
            kept.push(attr);
        } else if name == "is" {
            // `_removeAttribute` re-adds a removed `is` as `is=""`.
            attr.value = "".into();
            kept.push(attr);
        }
    }
    elem.attrs = kept;
}

/// `_isUnsafeAttributeValue`: comment/CDATA closers or raw-text end tags
/// (`SAFE_FOR_XML`), and SVG animation of `href`.
fn is_unsafe_attribute_value(name: &str, value: &str) -> bool {
    if value.contains("-->") || value.contains("--!>") || value.contains("]>") {
        return true;
    }
    let lower = value.to_ascii_lowercase();
    const RAW_TEXT_END: &[&str] =
        &["style", "script", "title", "xmp", "textarea", "noscript", "iframe", "noembed", "noframes"];
    if lower.match_indices("</").any(|(i, _)| RAW_TEXT_END.iter().any(|t| lower[i + 2..].starts_with(t))) {
        return true;
    }
    name.eq_ignore_ascii_case("attributename") && value.contains("href")
}

/// `_isValidAttribute` (lowercased tag and attribute names).
fn is_valid_attribute(lc_tag: &str, lc_name: &str, value: &str) -> bool {
    if lc_name == "style" {
        return false; // FORBID_ATTR
    }
    // `_isPatchLinkageAttribute`.
    if lc_name == "patchsrc" || (lc_name == "for" && lc_tag != "label" && lc_tag != "output") {
        return false;
    }
    // SANITIZE_DOM: no DOM clobbering through id/name.
    if (lc_name == "id" || lc_name == "name") && tables::DOM_PROPERTY_NAMES.binary_search(&value).is_ok() {
        return false;
    }
    if is_data_attr(lc_name) || is_aria_attr(lc_name) {
        return true;
    }
    if !ALLOWED_ATTR.contains(lc_name) {
        return false;
    }
    if tables::URI_SAFE_ATTRS.contains(&lc_name) {
        return true;
    }
    let compact: String = value.chars().filter(|&c| !is_attr_whitespace(c)).collect();
    if is_allowed_uri(&compact) {
        return true;
    }
    if matches!(lc_name, "src" | "xlink:href" | "href")
        && lc_tag != "script"
        && value.starts_with("data:")
        && tables::DATA_URI_TAGS.contains(&lc_tag)
    {
        return true;
    }
    value.is_empty()
}

/// `DATA_ATTR` `/^data-[\-\w.·-￿]+$/` (astral chars are surrogate
/// pairs inside that range in JS).
fn is_data_attr(name: &str) -> bool {
    name.strip_prefix("data-").is_some_and(|rest| {
        !rest.is_empty()
            && rest.chars().all(|c| c == '-' || c == '.' || c == '_' || c.is_ascii_alphanumeric() || c as u32 >= 0xB7)
    })
}

/// `ARIA_ATTR` `/^aria-[\-\w]+$/`.
fn is_aria_attr(name: &str) -> bool {
    name.strip_prefix("aria-").is_some_and(|rest| {
        !rest.is_empty() && rest.chars().all(|c| c == '-' || c == '_' || c.is_ascii_alphanumeric())
    })
}

/// `IS_ALLOWED_URI`:
/// `/^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|matrix):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i`.
fn is_allowed_uri(value: &str) -> bool {
    const SCHEMES: &[&str] =
        &["http:", "https:", "ftp:", "ftps:", "mailto:", "tel:", "callto:", "sms:", "cid:", "xmpp:", "matrix:"];
    let lower = value.to_ascii_lowercase();
    if SCHEMES.iter().any(|s| lower.starts_with(s)) {
        return true;
    }
    let Some(first) = value.chars().next() else { return false };
    if !first.is_ascii_alphabetic() {
        return true;
    }
    // A scheme-like run of `[a-z+.-]` must not end in `:` (a non-allowlisted scheme).
    let is_scheme_char = |c: char| c.is_ascii_alphabetic() || matches!(c, '+' | '.' | '-');
    value.chars().find(|&c| !is_scheme_char(c)) != Some(':')
}

/// `ATTR_WHITESPACE`.
fn is_attr_whitespace(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{20}' | '\u{a0}' | '\u{1680}' | '\u{180e}' | '\u{2000}'..='\u{2029}' | '\u{205f}' | '\u{3000}')
}

/// JS `String.prototype.trim` (WhiteSpace + LineTerminator, incl. U+FEFF).
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n'
                | '\u{b}'
                | '\u{c}'
                | '\r'
                | ' '
                | '\u{a0}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_uri_matches_dompurify_regex() {
        assert!(is_allowed_uri("https://x"));
        assert!(is_allowed_uri("/rel"));
        assert!(is_allowed_uri("#frag"));
        assert!(is_allowed_uri("currentColor"));
        assert!(is_allowed_uri("a.png"));
        assert!(!is_allowed_uri("javascript:alert(1)"));
        assert!(!is_allowed_uri("vbscript:x"));
        assert!(!is_allowed_uri("data:text/html,x"));
        assert!(!is_allowed_uri(""));
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        // 5000 levels: deep enough to overflow a recursive walk on a test thread
        // (html5ever parsing is quadratic in depth, so more is too slow in debug).
        let html = "<div>".repeat(5_000) + "x";
        assert!(sanitize(&html).contains('x'));
    }
}
