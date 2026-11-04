//! Port of test/javascript/diff_utils.test.js.

use frankmd_core::text::diff::{DiffKind, DiffOp, compute_word_diff, lcs_word_diff, tokenize};

fn op(kind: DiffKind, value: &str) -> DiffOp {
    DiffOp { kind, value: value.to_owned() }
}

fn has(diff: &[DiffOp], kind: DiffKind, needle: &str) -> bool {
    diff.iter().any(|d| d.kind == kind && d.value.contains(needle))
}

// port: test/javascript/diff_utils.test.js:9 "splits text into words"
#[test]
fn tokenize_splits_text_into_words() {
    assert_eq!(tokenize("hello world"), ["hello", " ", "world"]);
}

// port: test/javascript/diff_utils.test.js:14 "preserves multiple spaces"
#[test]
fn tokenize_preserves_multiple_spaces() {
    assert_eq!(tokenize("hello  world"), ["hello", "  ", "world"]);
}

// port: test/javascript/diff_utils.test.js:19 "handles newlines"
#[test]
fn tokenize_handles_newlines() {
    assert_eq!(tokenize("hello\nworld"), ["hello", "\n", "world"]);
}

// port: test/javascript/diff_utils.test.js:24 "handles empty string"
#[test]
fn tokenize_handles_empty_string() {
    assert!(tokenize("").is_empty());
}

// port: test/javascript/diff_utils.test.js:29 "handles leading/trailing spaces"
#[test]
fn tokenize_handles_leading_trailing_spaces() {
    assert_eq!(tokenize("  hello  "), ["  ", "hello", "  "]);
}

// port: test/javascript/diff_utils.test.js:36 "returns equal for identical text"
#[test]
fn compute_word_diff_returns_equal_for_identical_text() {
    assert_eq!(compute_word_diff("hello world", "hello world"), [op(DiffKind::Equal, "hello world")]);
}

// port: test/javascript/diff_utils.test.js:41 "detects inserted word"
#[test]
fn compute_word_diff_detects_inserted_word() {
    let diff = compute_word_diff("hello world", "hello beautiful world");
    assert!(has(&diff, DiffKind::Insert, "beautiful"), "{diff:?}");
}

// port: test/javascript/diff_utils.test.js:46 "detects deleted word"
#[test]
fn compute_word_diff_detects_deleted_word() {
    let diff = compute_word_diff("hello beautiful world", "hello world");
    assert!(has(&diff, DiffKind::Delete, "beautiful"), "{diff:?}");
}

// port: test/javascript/diff_utils.test.js:51 "detects changed word"
#[test]
fn compute_word_diff_detects_changed_word() {
    let diff = compute_word_diff("hello world", "hello universe");
    assert!(has(&diff, DiffKind::Delete, "world"), "{diff:?}");
    assert!(has(&diff, DiffKind::Insert, "universe"), "{diff:?}");
}

// port: test/javascript/diff_utils.test.js:57 "handles empty strings"
#[test]
fn compute_word_diff_handles_empty_strings() {
    assert!(compute_word_diff("", "").is_empty());
    assert_eq!(compute_word_diff("hello", ""), [op(DiffKind::Delete, "hello")]);
    assert_eq!(compute_word_diff("", "hello"), [op(DiffKind::Insert, "hello")]);
}

// port: test/javascript/diff_utils.test.js:65 "returns equal for identical tokens"
#[test]
fn lcs_returns_equal_for_identical_tokens() {
    assert_eq!(lcs_word_diff(&["a", "b", "c"], &["a", "b", "c"]), [op(DiffKind::Equal, "abc")]);
}

// port: test/javascript/diff_utils.test.js:70 "detects insertions"
#[test]
fn lcs_detects_insertions() {
    let diff = lcs_word_diff(&["a", "c"], &["a", "b", "c"]);
    assert!(diff.contains(&op(DiffKind::Insert, "b")), "{diff:?}");
}

// port: test/javascript/diff_utils.test.js:75 "detects deletions"
#[test]
fn lcs_detects_deletions() {
    let diff = lcs_word_diff(&["a", "b", "c"], &["a", "c"]);
    assert!(diff.contains(&op(DiffKind::Delete, "b")), "{diff:?}");
}

// port: test/javascript/diff_utils.test.js:80 "handles empty arrays"
#[test]
fn lcs_handles_empty_arrays() {
    assert!(lcs_word_diff(&[], &[]).is_empty());
}

// port: test/javascript/diff_utils.test.js:84 "handles all new tokens"
#[test]
fn lcs_handles_all_new_tokens() {
    assert_eq!(lcs_word_diff(&[], &["a", "b"]), [op(DiffKind::Insert, "ab")]);
}

// port: test/javascript/diff_utils.test.js:89 "handles all deleted tokens"
#[test]
fn lcs_handles_all_deleted_tokens() {
    assert_eq!(lcs_word_diff(&["a", "b"], &[]), [op(DiffKind::Delete, "ab")]);
}

// port: test/javascript/diff_utils.test.js:94 "merges consecutive operations"
#[test]
fn lcs_merges_consecutive_operations() {
    let diff = lcs_word_diff(&["a", "x", "y", "b"], &["a", "b"]);
    let deletes: Vec<_> = diff.iter().filter(|d| d.kind == DiffKind::Delete).collect();
    assert_eq!(deletes.len(), 1, "{diff:?}");
    assert_eq!(deletes[0].value, "xy");
}
