//! Word diff for the recovery dialog and the AI grammar review (port of
//! `lib/diff_utils.js`). Pure, offset-free: works on token strings.

/// Kind of a diff operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    /// Present in both texts.
    Equal,
    /// Only in the new text.
    Insert,
    /// Only in the old text.
    Delete,
}

/// One diff operation; consecutive operations of the same kind are merged,
/// so `value` is the concatenation of their tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffOp {
    /// Operation kind.
    pub kind: DiffKind,
    /// Concatenated token text.
    pub value: String,
}

/// Split `text` into maximal runs of whitespace and non-whitespace
/// (`/(\s+|\S+)/g`). Concatenating the tokens yields `text` again.
///
/// **[COMPAT]** "whitespace" is the JS `\s` class (Unicode `White_Space` plus
/// U+FEFF), not just ASCII.
pub fn tokenize(text: &str) -> Vec<&str> {
    todo!()
}

/// Word-level diff of `original` → `corrected`: `lcs_word_diff(tokenize(a), tokenize(b))`.
pub fn compute_word_diff(original: &str, corrected: &str) -> Vec<DiffOp> {
    todo!()
}

/// LCS diff over tokens with the original's backtracking tie-break
/// (**[COMPAT]**: on ties an *insert* is preferred over a delete while
/// walking backwards, so a changed word comes out as `delete` then `insert`),
/// then same-kind runs merged.
pub fn lcs_word_diff(old_tokens: &[&str], new_tokens: &[&str]) -> Vec<DiffOp> {
    todo!()
}
