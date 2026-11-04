//! A single text edit plus the selection to restore afterwards.
//!
//! Every editor command (formatting, task toggle, find/replace, the vim
//! layer, dialog insertions) computes an [`Edit`] from the plain text and the
//! editor applies it as **one** replace, so each command is exactly one undo step. Offsets are
//! UTF-8 bytes on char boundaries.

use std::ops::Range;

/// Replace `range` with `text`, then select `selection` (in the new text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Byte range of the old text to replace.
    pub range: Range<usize>,
    /// Replacement text.
    pub text: String,
    /// Selection after the edit (`start == end` for a caret).
    pub selection: Range<usize>,
}

impl Edit {
    /// Apply to a `String` (tests and the vim shadow buffer).
    pub fn apply_to(&self, text: &mut String) {
        text.replace_range(self.range.clone(), &self.text);
    }

    /// The same edit shrunk to the bytes that really change (common prefix
    /// and suffix removed), so undo/redo and decorations touch as little as
    /// possible. `old` is the full current text.
    pub fn minimized(self, old: &str) -> Self {
        let (range, text) =
            minimal_replacement(&old[self.range.clone()], self.range.start, &self.text);
        Self {
            range,
            text: text.to_owned(),
            selection: self.selection,
        }
    }
}

/// The smallest `(old range, new text)` turning `old` (which starts at byte
/// `offset` of the document) into `new`. Both ends snap to char boundaries.
pub fn minimal_replacement<'a>(old: &str, offset: usize, new: &'a str) -> (Range<usize>, &'a str) {
    let prefix = common_prefix(old, new);
    let suffix = common_suffix(&old[prefix..], &new[prefix..]);
    let range = offset + prefix..offset + old.len() - suffix;
    (range, &new[prefix..new.len() - suffix])
}

/// Byte length of the longest common prefix, on a char boundary of both.
pub fn common_prefix(a: &str, b: &str) -> usize {
    let mut n = a
        .as_bytes()
        .iter()
        .zip(b.as_bytes())
        .take_while(|(x, y)| x == y)
        .count();
    while !(a.is_char_boundary(n) && b.is_char_boundary(n)) {
        n -= 1;
    }
    n
}

/// Byte length of the longest common suffix, on a char boundary of both.
pub fn common_suffix(a: &str, b: &str) -> usize {
    let mut n = a
        .as_bytes()
        .iter()
        .rev()
        .zip(b.as_bytes().iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    while !(a.is_char_boundary(a.len() - n) && b.is_char_boundary(b.len() - n)) {
        n -= 1;
    }
    n
}

/// Map a byte offset through `range → new_len` (a caret inside the replaced
/// range moves to its end, like CodeMirror's default mapping for a change
/// that covers it).
pub fn map_offset(offset: usize, range: &Range<usize>, new_len: usize) -> usize {
    if offset <= range.start {
        offset
    } else if offset >= range.end {
        offset - range.len() + new_len
    } else {
        range.start + new_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_replacement_trims_both_ends() {
        assert_eq!(
            minimal_replacement("hello world", 10, "hello there world"),
            (16..16, "there ")
        );
        assert_eq!(minimal_replacement("aXb", 0, "ab"), (1..2, ""));
        assert_eq!(minimal_replacement("same", 0, "same"), (4..4, ""));
    }

    #[test]
    fn minimal_replacement_respects_char_boundaries() {
        // "é" (c3 a9) vs "è" (c3 a8) share their first byte.
        assert_eq!(minimal_replacement("é", 0, "è"), (0..2, "è"));
    }

    #[test]
    fn map_offset_shifts_after_and_clamps_inside() {
        assert_eq!(map_offset(2, &(5..7), 4), 2);
        assert_eq!(map_offset(9, &(5..7), 4), 11);
        assert_eq!(map_offset(6, &(5..7), 4), 9);
    }
}
