//! Find/replace state for the inline find bar (ui-spec §5.4), GUI-free.
//!
//! Matching is `frankmd_search::find` (JS regex semantics, **[COMPAT]**
//! `find_replace_controller.js`); this module converts its UTF-16 offsets to the
//! editor's UTF-8 bytes and keeps the bar's state: query, options, matches, the
//! current match, the regex error and the transient "Replaced N" feedback.

use std::ops::Range;
use std::time::{Duration, Instant};

pub use frankmd_search::find::FindOptions;
use frankmd_search::find::{
    Direction, Match, find_all_matches, find_closest_match_index, replace_matches, validate_regex,
};

use super::edit::Edit;

/// How long "Replaced N" stays in place of the counter.
pub const FEEDBACK_TTL: Duration = Duration::from_secs(2);

/// What the counter slot shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Counter {
    /// Empty query: nothing.
    Empty,
    /// No matches (`dialogs.find_replace.no_results`).
    NoResults,
    /// `current of total` (`current` is 0 when no match is selected yet).
    Matches {
        /// 1-based current match, 0 if none.
        current: usize,
        /// Match count.
        total: usize,
    },
    /// Invalid regex (danger role).
    Error(String),
    /// "Replaced N" (success role) for [`FEEDBACK_TTL`].
    Replaced(usize),
}

/// The bar's state.
#[derive(Debug, Clone, Default)]
pub struct FindState {
    /// Bar visible.
    pub open: bool,
    /// Replace row visible.
    pub replace_visible: bool,
    /// Find query.
    pub query: String,
    /// Replacement text.
    pub replacement: String,
    /// Toggles (Aa, .*, whole word): the search crate's options, shared with every
    /// other find surface.
    pub options: FindOptions,
    matches: Vec<Match>,
    ranges: Vec<Range<usize>>,
    current: Option<usize>,
    error: Option<String>,
    feedback: Option<(usize, Instant)>,
}

impl FindState {
    /// Match byte ranges, ascending.
    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }

    /// Index of the current match.
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    /// Byte range of the current match.
    pub fn current_range(&self) -> Option<Range<usize>> {
        self.current.and_then(|i| self.ranges.get(i).cloned())
    }

    /// Regex error, if the query is an invalid regex.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The counter to show at `now`.
    pub fn counter(&self, now: Instant) -> Counter {
        if let Some((count, at)) = self.feedback
            && now.duration_since(at) < FEEDBACK_TTL
        {
            return Counter::Replaced(count);
        }
        if let Some(error) = &self.error {
            return Counter::Error(error.clone());
        }
        if self.query.is_empty() {
            return Counter::Empty;
        }
        if self.ranges.is_empty() {
            return Counter::NoResults;
        }
        Counter::Matches {
            current: self.current.map_or(0, |i| i + 1),
            total: self.ranges.len(),
        }
    }

    /// Whether a "Replaced N" message is still showing at `now`.
    pub fn feedback_active(&self, now: Instant) -> bool {
        self.feedback
            .is_some_and(|(_, at)| now.duration_since(at) < FEEDBACK_TTL)
    }

    /// **[COMPAT]** `findAll`: recompute matches; the current match becomes the closest
    /// one at/after `caret` (byte offset). Doesn't move the editor selection.
    pub fn recompute(&mut self, text: &str, caret: usize) {
        self.error = None;
        self.matches.clear();
        self.ranges.clear();
        self.current = None;
        if self.query.is_empty() {
            return;
        }
        if self.options.use_regex
            && let Err(error) = validate_regex(&self.query)
        {
            self.error = Some(error);
            return;
        }
        self.matches = find_all_matches(text, &self.query, self.options);
        self.ranges = utf16_to_byte_ranges(text, &self.matches);
        self.current = self.closest(text, caret, Direction::Next);
    }

    fn closest(&self, text: &str, pos: usize, direction: Direction) -> Option<usize> {
        let utf16 = frankmd_core::text::byte_to_utf16(text, pos);
        find_closest_match_index(
            &self.matches,
            i64::try_from(utf16).unwrap_or(i64::MAX),
            direction,
        )
    }

    /// **[COMPAT]** `findNext` (from the selection end) / `findPrevious` (from the
    /// selection start): the match to select, which becomes current.
    pub fn step(
        &mut self,
        text: &str,
        selection: Range<usize>,
        forward: bool,
    ) -> Option<Range<usize>> {
        if self.ranges.is_empty() {
            self.recompute(text, selection.start);
        }
        let (pos, direction) = if forward {
            (selection.end, Direction::Next)
        } else {
            (selection.start, Direction::Previous)
        };
        self.current = self.closest(text, pos, direction);
        self.current_range()
    }

    /// Replace the current match (regex mode expands `$1`, `$&`, ...); the caret goes
    /// after the replacement. The caller recomputes and, for "replace and next", steps
    /// forward from there.
    pub fn replace_current(&self) -> Option<Edit> {
        let index = self.current?;
        let m = self.matches.get(index)?;
        let range = self.ranges.get(index)?.clone();
        let replacement = resolve_replacement(m, &self.replacement, self.options.use_regex);
        let caret = range.start + replacement.len();
        Some(Edit {
            range,
            text: replacement,
            selection: caret..caret,
        })
    }

    /// **[COMPAT]** `replaceAll`: one edit over the whole text and the number of
    /// replaced matches; arms the "Replaced N" feedback at `now`.
    pub fn replace_all(&mut self, text: &str, caret: usize, now: Instant) -> Option<(Edit, usize)> {
        if self.matches.is_empty() {
            return None;
        }
        let count = self.matches.len();
        let updated =
            replace_matches(text, &self.matches, &self.replacement, self.options.use_regex);
        let caret = caret.min(updated.len());
        let caret = (0..=caret)
            .rev()
            .find(|&i| updated.is_char_boundary(i))
            .unwrap_or(0);
        self.feedback = Some((count, now));
        Some((
            Edit {
                range: 0..text.len(),
                text: updated,
                selection: caret..caret,
            },
            count,
        ))
    }

    /// Clear matches and the current match (bar closed).
    pub fn clear_matches(&mut self) {
        self.matches.clear();
        self.ranges.clear();
        self.current = None;
        self.error = None;
    }
}

/// The replacement for one match, with JS `$` expansion in regex mode.
fn resolve_replacement(m: &Match, replacement: &str, regex: bool) -> String {
    if !regex {
        return replacement.to_owned();
    }
    let local = Match {
        start: 0,
        end: m.text.encode_utf16().count(),
        text: m.text.clone(),
        groups: m.groups.clone(),
    };
    replace_matches(&m.text, &[local], replacement, true)
}

/// Convert ascending UTF-16 match offsets to UTF-8 byte ranges in one pass.
pub fn utf16_to_byte_ranges(text: &str, matches: &[Match]) -> Vec<Range<usize>> {
    let mut out = Vec::with_capacity(matches.len());
    let mut chars = text.char_indices().peekable();
    let mut unit = 0usize;
    let mut byte_at =
        |target: usize, chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>| -> usize {
            while unit < target {
                match chars.next() {
                    Some((_, c)) => unit += c.len_utf16(),
                    None => return text.len(),
                }
            }
            chars.peek().map_or(text.len(), |(b, _)| *b)
        };
    for m in matches {
        let start = byte_at(m.start, &mut chars);
        let end = byte_at(m.end, &mut chars);
        out.push(start..end);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_ranges_follow_multibyte_text() {
        let mut s = FindState {
            query: "a".into(),
            ..Default::default()
        };
        s.recompute("é a 😀 a", 0);
        assert_eq!(s.ranges(), &[3..4, 10..11]);
    }

    #[test]
    fn whole_word_literal_replacement_keeps_dollar_signs() {
        let mut s = FindState {
            query: "a".into(),
            replacement: "$&!".into(),
            options: FindOptions {
                whole_word: true,
                ..FindOptions::default()
            },
            ..Default::default()
        };
        s.recompute("a ab a", 0);
        assert_eq!(s.ranges(), &[0..1, 5..6]);
        let (edit, count) = s.replace_all("a ab a", 0, Instant::now()).expect("matches");
        assert_eq!(count, 2);
        assert_eq!(edit.text, "$&! ab $&!");
    }
}
