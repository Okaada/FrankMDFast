//! Wikilinks (`[[target]]`), backlinks (`app/services/backlinks_service.rb`)
//! and the `[[` completion source (`app/javascript/lib/codemirror_wikilink.js`).
//!
//! Golden: `backlinks.json` (scan driven by `frankmd_fs::NotesService::backlinks`),
//! `js_codemirror_wikilink.json` (completions).

use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// One `[[...]]` occurrence in a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// Link target as written.
    pub target: String,
    /// Optional label after `|`.
    pub label: Option<String>,
    /// UTF-8 byte range of the whole `[[...]]` in the source.
    pub range: Range<usize>,
}

/// Extract wikilinks from markdown source.
pub fn extract(source: &str) -> Vec<WikiLink> {
    let mut links = Vec::new();
    let mut from = 0;
    while let Some(open) = source[from..].find("[[").map(|i| from + i) {
        let inner_start = open + 2;
        let Some(close) = source[inner_start..].find("]]").map(|i| inner_start + i) else {
            break;
        };
        let inner = &source[inner_start..close];
        if inner.contains('\n') || inner.contains("[[") || inner.trim().is_empty() {
            // Not a link on its own line segment; resume after this `[[`.
            from = inner_start;
            continue;
        }
        let (target, label) = match inner.split_once('|') {
            Some((target, label)) => (target, Some(label.to_owned())),
            None => (inner, None),
        };
        links.push(WikiLink {
            target: target.to_owned(),
            label,
            range: open..close + 2,
        });
        from = close + 2;
    }
    links
}

// ---------------------------------------------------------------------------
// Backlinks
// ---------------------------------------------------------------------------

/// **[COMPAT]** the three case-insensitive patterns of `BacklinksService`
/// for one target (backend.md §5):
/// `\[\[NAME(\.md)?\]\]`, `\[\[PATH_NO_EXT(\.md)?\]\]`,
/// `\[[^\]]*\]\([^)]*PATH_NO_EXT(\.md)?[^)]*\)` (NAME/PATH regex-escaped).
#[derive(Debug, Clone)]
pub struct BacklinkQuery {
    /// Target path as given (normalized with `.md` by the caller).
    pub target: String,
    /// Basename without `.md`.
    pub name: String,
    /// Target path without `.md`.
    pub path_no_ext: String,
    /// The union of the three patterns.
    pattern: regex::Regex,
}

impl BacklinkQuery {
    /// Build the query. **[COMPAT]** any `..` substring in `target` is
    /// `CoreError::InvalidPath("Invalid path")` (Rails `ArgumentError`).
    pub fn new(target: &str) -> Result<Self, CoreError> {
        if target.contains("..") {
            return Err(CoreError::InvalidPath("Invalid path".to_owned()));
        }
        let name = crate::note::ruby_basename(target, Some(".md")).to_owned();
        let path_no_ext = target.strip_suffix(".md").unwrap_or(target).to_owned();
        let name_re = regex::escape(&name);
        let path_re = regex::escape(&path_no_ext);
        let source = format!(
            r"\[\[{name_re}(\.md)?\]\]|\[\[{path_re}(\.md)?\]\]|\[[^\]]*\]\([^)]*{path_re}(\.md)?[^)]*\)"
        );
        let pattern = regex::RegexBuilder::new(&source)
            .case_insensitive(true)
            .build()
            .map_err(|e| CoreError::InvalidPath(e.to_string()))?;
        Ok(Self {
            target: target.to_owned(),
            name,
            path_no_ext,
            pattern,
        })
    }

    /// Does one line contain a link to the target?
    pub fn matches_line(&self, line: &str) -> bool {
        self.pattern.is_match(line)
    }

    /// **[COMPAT]** the target itself is skipped: `rel == target` or
    /// `rel == path_no_ext + ".md"`.
    pub fn is_target(&self, rel_path: &str) -> bool {
        rel_path == self.target || rel_path.strip_suffix(".md") == Some(self.path_no_ext.as_str())
    }
}

/// Ruby `readlines(chomp: true)`: split after each `\n`, then drop one
/// trailing `\r\n`, `\n` or `\r` from every line.
pub fn ruby_lines(content: &str) -> Vec<&str> {
    content
        .split_inclusive('\n')
        .map(|line| {
            line.strip_suffix("\r\n")
                .or_else(|| line.strip_suffix('\n'))
                .or_else(|| line.strip_suffix('\r'))
                .unwrap_or(line)
        })
        .collect()
}

/// One context line around a match (same shape as search results).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextLine {
    /// 1-based line number.
    pub line_number: usize,
    /// Line text (no line terminator).
    pub content: String,
    /// This is the matching line.
    pub is_match: bool,
}

/// One backlink hit: `{path, name, line_number, match_text, context}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backlink {
    /// Vault-relative path of the linking note.
    pub path: String,
    /// Its basename without `.md`.
    pub name: String,
    /// 1-based line number of the match.
    pub line_number: usize,
    /// The whole matching line.
    pub match_text: String,
    /// `±context_lines` lines around it, clamped to the file.
    pub context: Vec<ContextLine>,
}

/// **[COMPAT]** `search_file`: every matching line of one note, in line order
/// (lines split like Ruby `readlines(chomp: true)`).
pub fn scan_note(
    query: &BacklinkQuery,
    rel_path: &str,
    content: &str,
    context_lines: usize,
) -> Vec<Backlink> {
    let lines = ruby_lines(content);
    let name = crate::note::ruby_basename(rel_path, Some(".md")).to_owned();
    let mut hits = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !query.matches_line(line) {
            continue;
        }
        let start = index.saturating_sub(context_lines);
        let end = (index + context_lines).min(lines.len() - 1);
        let context = (start..=end)
            .map(|i| ContextLine {
                line_number: i + 1,
                content: lines[i].to_owned(),
                is_match: i == index,
            })
            .collect();
        hits.push(Backlink {
            path: rel_path.to_owned(),
            name: name.clone(),
            line_number: index + 1,
            match_text: (*line).to_owned(),
            context,
        });
    }
    hits
}

/// Does `link` resolve to `note`? (wikilink click-through, by name or path,
/// case-insensitively and with an optional `.md`, like the backlink patterns.)
pub fn resolves_to(link: &WikiLink, note: &crate::RelPath) -> bool {
    let target = link.target.trim().to_lowercase();
    let target = target.strip_suffix(".md").unwrap_or(&target);
    let path = note.as_str().to_lowercase();
    let path_no_ext = path.strip_suffix(".md").unwrap_or(&path);
    let name = crate::note::ruby_basename(path_no_ext, None);
    !target.is_empty() && (target == path_no_ext || target == name)
}

// ---------------------------------------------------------------------------
// `[[` completions
// ---------------------------------------------------------------------------

/// A file offered to the completion source (`{path, name}` from the tree;
/// either may be missing, as in the JS provider).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct WikiFile {
    /// Vault-relative path; only paths ending in lowercase `.md` are offered.
    #[serde(default)]
    pub path: Option<String>,
    /// Display name override (default: basename without `.md`).
    #[serde(default)]
    pub name: Option<String>,
}

/// One completion option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiCompletion {
    /// Display label (name).
    pub label: String,
    /// Path without `.md`, only when it contains a `/`.
    pub detail: Option<String>,
    /// 3 = name prefix, 2 = name substring, 1 = path substring or empty query.
    pub boost: i32,
    /// Text that replaces `from..cursor`: `<path without .md>]]`.
    pub insert: String,
}

/// Completion result for the cursor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiCompletions {
    /// UTF-8 byte offset just after the open `[[`.
    pub from: usize,
    /// UTF-8 byte offset of the cursor (end of the replaced range).
    pub to: usize,
    /// Options in provider order (ranking by `boost` is the UI's job).
    pub options: Vec<WikiCompletion>,
}

impl WikiCompletion {
    /// Caret position after applying (UTF-8 byte offset): `from + insert.len()`.
    pub fn caret_after(&self, from: usize) -> usize {
        from + self.insert.len()
    }
}

/// **[COMPAT]** `wikilinkCompletions`: look back on the cursor's line for the
/// last unclosed `[[`, filter `files` by the lowercased query, `None` when
/// nothing applies. `pos` is a UTF-8 byte offset into `doc`.
pub fn complete(doc: &str, pos: usize, files: &[WikiFile]) -> Option<WikiCompletions> {
    let pos = pos.min(doc.len());
    let before_cursor = doc.get(..pos)?;
    let line_start = before_cursor.rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let text_before = &before_cursor[line_start..];

    let open = text_before.rfind("[[")?;
    let after_open = &text_before[open + 2..];
    if after_open.contains("]]") {
        return None;
    }
    let query = after_open.to_lowercase();
    let from = line_start + open + 2;

    let options: Vec<WikiCompletion> = files
        .iter()
        .filter_map(|file| completion_for(file, &query))
        .collect();
    if options.is_empty() {
        return None;
    }
    Some(WikiCompletions {
        from,
        to: pos,
        options,
    })
}

/// Score one provider file against the lowercased query (`None` = filtered out).
fn completion_for(file: &WikiFile, query: &str) -> Option<WikiCompletion> {
    let full_path = file.path.as_deref().filter(|p| p.ends_with(".md"))?;
    let path = full_path.strip_suffix(".md").unwrap_or(full_path);
    let name = match file.name.as_deref() {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => path.rsplit('/').next().unwrap_or(path).to_owned(),
    };
    let lower_name = name.to_lowercase();
    let boost = if query.is_empty() {
        1
    } else if lower_name.starts_with(query) {
        3
    } else if lower_name.contains(query) {
        2
    } else if path.to_lowercase().contains(query) {
        1
    } else {
        return None;
    };
    Some(WikiCompletion {
        label: name,
        detail: path.contains('/').then(|| path.to_owned()),
        boost,
        insert: format!("{path}]]"),
    })
}
