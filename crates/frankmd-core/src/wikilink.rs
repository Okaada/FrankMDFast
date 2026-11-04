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
    todo!("backend.md §5")
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
}

impl BacklinkQuery {
    /// Build the query. **[COMPAT]** any `..` substring in `target` is
    /// `CoreError::InvalidPath("Invalid path")` (Rails `ArgumentError`).
    pub fn new(target: &str) -> Result<Self, CoreError> {
        todo!("backlinks_service.rb:13")
    }

    /// Does one line contain a link to the target?
    pub fn matches_line(&self, line: &str) -> bool {
        todo!()
    }

    /// **[COMPAT]** the target itself is skipped: `rel == target` or
    /// `rel == path_no_ext + ".md"`.
    pub fn is_target(&self, rel_path: &str) -> bool {
        todo!()
    }
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
pub fn scan_note(query: &BacklinkQuery, rel_path: &str, content: &str, context_lines: usize) -> Vec<Backlink> {
    todo!("backlinks_service.rb#search_file")
}

/// Does `link` resolve to `note`? (wikilink click-through, by name or path.)
pub fn resolves_to(link: &WikiLink, note: &crate::RelPath) -> bool {
    todo!()
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
    todo!("codemirror_wikilink.js")
}
