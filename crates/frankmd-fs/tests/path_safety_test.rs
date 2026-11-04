//! Port of `test/services/path_safety_test.rb`.
#![allow(clippy::disallowed_methods, clippy::unwrap_used, missing_docs, unreachable_pub)]

use frankmd_core::path::lexical_contain;
use frankmd_fs::path_safety::contain;
use std::fs;
use std::path::{Path, PathBuf};

const BASE: &str = "/data/notes";

/// BASE does not exist, so `contain` is purely lexical; both halves must agree.
fn lex(path: Option<&str>) -> Option<PathBuf> {
    let fs_result = contain(Path::new(BASE), path);
    let core_result = lexical_contain(BASE, path).map(PathBuf::from);
    assert_eq!(fs_result, core_result, "fs::contain and core::lexical_contain disagree for {path:?}");
    fs_result
}

/// `<tmp>/notes` + `<tmp>/outside` (realpath'd, like the Ruby tests).
fn dirs() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let raw = tempfile::tempdir().unwrap();
    let tmp = raw.path().canonicalize().unwrap();
    let base = tmp.join("notes");
    let outside = tmp.join("outside");
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(&outside).unwrap();
    (raw, base, outside)
}

// port: test/services/path_safety_test.rb:8 "contains a plain path inside base"
#[test]
fn contains_a_plain_path_inside_base() {
    assert_eq!(lex(Some("note.md")), Some(PathBuf::from("/data/notes/note.md")));
}

// port: test/services/path_safety_test.rb:12 "contains a nested path inside base"
#[test]
fn contains_a_nested_path_inside_base() {
    assert_eq!(lex(Some("a/b.md")), Some(PathBuf::from("/data/notes/a/b.md")));
}

// port: test/services/path_safety_test.rb:16 "rejects ../ traversal"
#[test]
fn rejects_dotdot_traversal() {
    assert_eq!(lex(Some("../../etc/passwd")), None);
}

// port: test/services/path_safety_test.rb:20 "rejects an absolute path outside base"
#[test]
fn rejects_an_absolute_path_outside_base() {
    assert_eq!(lex(Some("/etc/passwd")), None);
}

// port: test/services/path_safety_test.rb:24 "rejects a sibling directory that shares the name prefix"
#[test]
fn rejects_a_sibling_directory_that_shares_the_name_prefix() {
    assert_eq!(lex(Some("/data/notes-backup/secret.md")), None);
    assert_eq!(lex(Some("/data/notesX")), None);
}

// port: test/services/path_safety_test.rb:31 "allows a filename that legitimately contains dot-dot"
#[test]
fn allows_a_filename_that_legitimately_contains_dot_dot() {
    assert_eq!(lex(Some("notes..v2.md")), Some(PathBuf::from("/data/notes/notes..v2.md")));
}

// port: test/services/path_safety_test.rb:36 "returns nil for a nil path"
#[test]
fn returns_none_for_a_nil_path() {
    assert_eq!(lex(None), None);
}

// port: test/services/path_safety_test.rb:40 "rejects a symlink that resolves outside base"
#[test]
fn rejects_a_symlink_that_resolves_outside_base() {
    let (_t, base, outside) = dirs();
    fs::write(outside.join("secret.txt"), "x").unwrap();
    std::os::unix::fs::symlink(outside.join("secret.txt"), base.join("link.txt")).unwrap();
    assert_eq!(contain(&base, Some("link.txt")), None);
}

// port: test/services/path_safety_test.rb:54 "rejects a path through a symlinked directory that points outside base"
#[test]
fn rejects_a_path_through_a_symlinked_directory_that_points_outside_base() {
    let (_t, base, outside) = dirs();
    fs::write(outside.join("secret.txt"), "x").unwrap();
    std::os::unix::fs::symlink(&outside, base.join("escape")).unwrap();
    assert_eq!(contain(&base, Some("escape/secret.txt")), None);
}

// port: test/services/path_safety_test.rb:70 "rejects a nonexistent leaf through a symlinked directory outside base"
#[test]
fn rejects_a_nonexistent_leaf_through_a_symlinked_directory_outside_base() {
    let (_t, base, outside) = dirs();
    std::os::unix::fs::symlink(&outside, base.join("escape")).unwrap();
    assert_eq!(contain(&base, Some("escape/new.md")), None);
    assert!(!outside.join("new.md").exists());
}

// port: test/services/path_safety_test.rb:84 "allows a real file inside base"
#[test]
fn allows_a_real_file_inside_base() {
    let (_t, base, _outside) = dirs();
    fs::write(base.join("ok.md"), "x").unwrap();
    assert_eq!(contain(&base, Some("ok.md")), Some(base.join("ok.md")));
}
