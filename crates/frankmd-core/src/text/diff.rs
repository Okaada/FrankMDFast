//! Word diff for the recovery dialog and the AI grammar review (port of
//! `lib/diff_utils.js`). Pure, offset-free: works on token strings.

use super::is_js_whitespace;

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
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut current: Option<bool> = None;
    for (i, c) in text.char_indices() {
        let space = is_js_whitespace(c);
        if current.is_some_and(|kind| kind != space) {
            tokens.push(&text[start..i]);
            start = i;
        }
        current = Some(space);
    }
    if start < text.len() {
        tokens.push(&text[start..]);
    }
    tokens
}

/// Word-level diff of `original` → `corrected`: `lcs_word_diff(tokenize(a), tokenize(b))`.
pub fn compute_word_diff(original: &str, corrected: &str) -> Vec<DiffOp> {
    lcs_word_diff(&tokenize(original), &tokenize(corrected))
}

/// LCS diff over tokens with the original's backtracking tie-break
/// (**[COMPAT]**: on ties an *insert* is preferred over a delete while
/// walking backwards, so a changed word comes out as `delete` then `insert`),
/// then same-kind runs merged.
pub fn lcs_word_diff(old_tokens: &[&str], new_tokens: &[&str]) -> Vec<DiffOp> {
    let (m, n) = (old_tokens.len(), new_tokens.len());
    let width = n + 1;
    let mut dp = vec![0usize; (m + 1) * width];
    for i in 1..=m {
        for j in 1..=n {
            dp[i * width + j] = if old_tokens[i - 1] == new_tokens[j - 1] {
                dp[(i - 1) * width + j - 1] + 1
            } else {
                dp[(i - 1) * width + j].max(dp[i * width + j - 1])
            };
        }
    }

    // Backtrack from the end, collecting operations in reverse.
    let mut reversed: Vec<(DiffKind, &str)> = Vec::with_capacity(m + n);
    let (mut i, mut j) = (m, n);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_tokens[i - 1] == new_tokens[j - 1] {
            reversed.push((DiffKind::Equal, old_tokens[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i * width + j - 1] >= dp[(i - 1) * width + j]) {
            reversed.push((DiffKind::Insert, new_tokens[j - 1]));
            j -= 1;
        } else {
            reversed.push((DiffKind::Delete, old_tokens[i - 1]));
            i -= 1;
        }
    }

    let mut merged: Vec<DiffOp> = Vec::new();
    for (kind, value) in reversed.into_iter().rev() {
        match merged.last_mut() {
            Some(last) if last.kind == kind => last.value.push_str(value),
            _ => merged.push(DiffOp {
                kind,
                value: value.to_owned(),
            }),
        }
    }
    merged
}
