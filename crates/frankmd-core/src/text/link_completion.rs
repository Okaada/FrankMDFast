//! Link completion after Ctrl+K (ui-spec §5.5 "Link completion", new on top
//! of FE-058's `[sel](url)` template).
//!
//! While the caret is inside the destination of a markdown link (`[text](…)`),
//! the editor offers: the URL on the clipboard first (when it is one), then the
//! vault's notes as paths relative to the open note. Typing filters; Esc leaves
//! the `url` placeholder alone, so the original template behaviour still works.

use std::ops::Range;

/// What a suggestion is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkSuggestionKind {
    /// The URL on the clipboard.
    Clipboard,
    /// A note of the vault.
    Note,
}

/// One row of the link completion popover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkSuggestion {
    /// Row label (the URL, or the note name without `.md`).
    pub label: String,
    /// Dim detail (the note's folder), if any.
    pub detail: Option<String>,
    /// Text that replaces the destination.
    pub insert: String,
    /// Clipboard URL or note.
    pub kind: LinkSuggestionKind,
}

/// Most rows offered.
pub const MAX_SUGGESTIONS: usize = 20;

/// The destination range of the markdown link around `caret`: the caret must
/// sit between `](` and the closing `)` (or the end of the line) on one line.
/// Returns `start..end` of the destination text.
pub fn link_destination_at(text: &str, caret: usize) -> Option<Range<usize>> {
    let caret = caret.min(text.len());
    let before = text.get(..caret)?;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let open = before[line_start..].rfind("](")? + line_start + 2;
    let typed = &text[open..caret];
    if typed.contains([')', ' ', '\t', '(']) {
        return None;
    }
    let rest = &text[caret..];
    let end = caret + rest.find([')', '\n', ' ', '\t']).unwrap_or(rest.len());
    Some(open..end)
}

/// The clipboard text, when it is a single http(s) or mailto URL.
pub fn clipboard_url(clipboard: &str) -> Option<&str> {
    let url = clipboard.trim();
    let lower = url.to_ascii_lowercase();
    let scheme_ok = ["http://", "https://", "mailto:"].iter().any(|s| lower.starts_with(s));
    let well_formed = url.len() > 8 && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    (scheme_ok && well_formed).then_some(url)
}

/// Path of vault note `target` relative to the folder of `from_note` (both
/// vault-relative, `/`-separated). Spaces and parentheses are percent-encoded
/// so the result is a valid markdown destination.
pub fn relative_note_path(from_note: Option<&str>, target: &str) -> String {
    let from_dir: Vec<&str> = from_note
        .map(|p| {
            let mut parts: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
            parts.pop();
            parts
        })
        .unwrap_or_default();
    let target_parts: Vec<&str> = target.split('/').filter(|s| !s.is_empty()).collect();
    let common = from_dir
        .iter()
        .zip(&target_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut out: Vec<&str> = vec![".."; from_dir.len() - common];
    out.extend(&target_parts[common..]);
    encode_destination(&out.join("/"))
}

fn encode_destination(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '(' => out.push_str("%28"),
            ')' => out.push_str("%29"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            _ => out.push(c),
        }
    }
    out
}

/// The rows for `query` (the destination typed so far; the `url` placeholder
/// counts as empty). `notes` are vault-relative paths; only `.md` files are
/// offered and the open note itself is skipped. Ranking: name prefix, then
/// name substring, then path substring (case-insensitive), ties by path.
pub fn link_suggestions(
    query: &str,
    clipboard: Option<&str>,
    notes: &[String],
    current_note: Option<&str>,
) -> Vec<LinkSuggestion> {
    let query = if query == "url" { "" } else { query };
    let needle = query.to_lowercase();
    let mut rows = Vec::new();
    if let Some(url) = clipboard.and_then(clipboard_url)
        && (needle.is_empty() || url.to_lowercase().contains(&needle))
    {
        rows.push(LinkSuggestion {
            label: url.to_owned(),
            detail: None,
            insert: url.to_owned(),
            kind: LinkSuggestionKind::Clipboard,
        });
    }
    let mut scored: Vec<(u8, &String)> = notes
        .iter()
        .filter(|p| p.ends_with(".md") && Some(p.as_str()) != current_note)
        .filter_map(|path| note_score(path, &needle).map(|s| (s, path)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    for (_, path) in scored.into_iter().take(MAX_SUGGESTIONS.saturating_sub(rows.len())) {
        let stem = path.strip_suffix(".md").unwrap_or(path);
        let (folder, name) = match stem.rsplit_once('/') {
            Some((folder, name)) => (Some(folder.to_owned()), name),
            None => (None, stem),
        };
        rows.push(LinkSuggestion {
            label: name.to_owned(),
            detail: folder,
            insert: relative_note_path(current_note, path),
            kind: LinkSuggestionKind::Note,
        });
    }
    rows
}

fn note_score(path: &str, needle: &str) -> Option<u8> {
    if needle.is_empty() {
        return Some(1);
    }
    let stem = path.strip_suffix(".md").unwrap_or(path);
    let name = stem.rsplit('/').next().unwrap_or(stem).to_lowercase();
    if name.starts_with(needle) {
        Some(3)
    } else if name.contains(needle) {
        Some(2)
    } else if path.to_lowercase().contains(needle) {
        Some(1)
    } else {
        None
    }
}

/// What sits under the caret for the Ctrl+. context action (ui-spec §6.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaretLink {
    /// `[text](dest)`, `<https://…>` or a bare `https://…` URL: the destination.
    Url(String),
    /// `[[target]]` / `[[target|label]]`: the target.
    Wiki(String),
    /// `![alt](src)`: the image source.
    Image(String),
}

/// The link, wikilink or image whose markup contains `caret` (same line only).
pub fn link_at_caret(text: &str, caret: usize) -> Option<CaretLink> {
    let caret = caret.min(text.len());
    let line_start = text.get(..caret)?.rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[caret..].find('\n').map_or(text.len(), |i| caret + i);
    let line = &text[line_start..line_end];
    let at = caret - line_start;
    wikilink_at(line, at)
        .or_else(|| inline_link_at(line, at))
        .or_else(|| autolink_at(line, at))
}

fn wikilink_at(line: &str, at: usize) -> Option<CaretLink> {
    let mut from = 0;
    while let Some(open) = line[from..].find("[[").map(|i| i + from) {
        let close = line[open + 2..].find("]]").map(|i| open + 2 + i)?;
        if open <= at && at <= close + 2 {
            let inner = &line[open + 2..close];
            let target = inner.split('|').next().unwrap_or(inner).trim();
            return (!target.is_empty()).then(|| CaretLink::Wiki(target.to_owned()));
        }
        from = close + 2;
    }
    None
}

fn inline_link_at(line: &str, at: usize) -> Option<CaretLink> {
    let mut from = 0;
    while let Some(mid) = line[from..].find("](").map(|i| i + from) {
        let close = line[mid + 2..].find(')').map(|i| mid + 2 + i)?;
        let open = line[..mid].rfind('[')?;
        let image = open > 0 && line.as_bytes()[open - 1] == b'!';
        let start = if image { open - 1 } else { open };
        if start <= at && at <= close + 1 {
            let dest = line[mid + 2..close].trim();
            let dest = dest.split_whitespace().next().unwrap_or("");
            let dest = dest.trim_start_matches('<').trim_end_matches('>').to_owned();
            if dest.is_empty() {
                return None;
            }
            return Some(if image { CaretLink::Image(dest) } else { CaretLink::Url(dest) });
        }
        from = close + 1;
    }
    None
}

fn autolink_at(line: &str, at: usize) -> Option<CaretLink> {
    for scheme in ["https://", "http://", "mailto:"] {
        let mut from = 0;
        while let Some(start) = line[from..].find(scheme).map(|i| i + from) {
            let end = line[start..]
                .find(|c: char| c.is_whitespace() || c == '>' || c == ')')
                .map_or(line.len(), |i| start + i);
            if start <= at && at <= end {
                let url = line[start..end].trim_end_matches(['.', ',', ';']);
                return Some(CaretLink::Url(url.to_owned()));
            }
            from = end.max(start + 1);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn caret_links_are_recognised() {
        let text = "a [doc](https://x.dev/a) and [[Some Note|alias]] ![p](img/p.png) see https://b.dev.";
        let at = |needle: &str| text.find(needle).expect("needle");
        assert_eq!(link_at_caret(text, at("doc")), Some(CaretLink::Url("https://x.dev/a".into())));
        assert_eq!(link_at_caret(text, at("Some")), Some(CaretLink::Wiki("Some Note".into())));
        assert_eq!(link_at_caret(text, at("[p]")), Some(CaretLink::Image("img/p.png".into())));
        assert_eq!(link_at_caret(text, at("b.dev")), Some(CaretLink::Url("https://b.dev".into())));
        assert_eq!(link_at_caret(text, at("and")), None);
        assert_eq!(link_at_caret("x\n[a](b)", 0), None);
    }

    #[test]
    fn destination_is_found_inside_the_parentheses() {
        let text = "see [docs](url) now";
        assert_eq!(link_destination_at(text, 11), Some(11..14));
        assert_eq!(link_destination_at(text, 14), Some(11..14));
        assert_eq!(link_destination_at(text, 4), None);
        assert_eq!(link_destination_at("[a](b) c", 7), None);
        assert_eq!(link_destination_at("[a](", 4), Some(4..4));
    }

    #[test]
    fn clipboard_url_must_be_a_single_web_url() {
        assert_eq!(clipboard_url(" https://example.com/a \n"), Some("https://example.com/a"));
        assert_eq!(clipboard_url("mailto:me@example.com"), Some("mailto:me@example.com"));
        assert_eq!(clipboard_url("file:///etc/passwd"), None);
        assert_eq!(clipboard_url("https://a b"), None);
        assert_eq!(clipboard_url("hello"), None);
    }

    #[test]
    fn relative_paths_walk_up_and_encode() {
        assert_eq!(relative_note_path(Some("a/b/note.md"), "a/c/other.md"), "../c/other.md");
        assert_eq!(relative_note_path(Some("note.md"), "dir/My Note.md"), "dir/My%20Note.md");
        assert_eq!(relative_note_path(None, "x.md"), "x.md");
        assert_eq!(relative_note_path(Some("a/n.md"), "a/m.md"), "m.md");
    }

    #[test]
    fn clipboard_first_then_ranked_notes() {
        let all = notes(&["work/meeting.md", "ideas.md", "projects/rust-ideas.md", "img.png", "cur.md"]);
        let rows = link_suggestions("url", Some("https://x.dev"), &all, Some("cur.md"));
        assert_eq!(rows[0].kind, LinkSuggestionKind::Clipboard);
        assert_eq!(rows.len(), 4);
        let rows = link_suggestions("ide", None, &all, Some("cur.md"));
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["ideas", "rust-ideas"]);
        assert_eq!(rows[1].detail.as_deref(), Some("projects"));
        assert_eq!(rows[1].insert, "projects/rust-ideas.md");
        assert!(link_suggestions("zzz", Some("https://x.dev"), &all, None).is_empty());
    }
}
