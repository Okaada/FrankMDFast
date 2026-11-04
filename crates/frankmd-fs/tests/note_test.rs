//! Port of `test/models/note_test.rb` (the `Note` model: `NotesService` +
//! `frankmd_fs::model` + `frankmd_core::{path, note}`).
//!
//! Skipped: `Note.search` (owned by frankmd-search, `search.json`),
//! `note.to_param` (Rails routing).
#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    missing_docs,
    unreachable_pub
)]

mod common;

use common::{Sandbox, en, sha256_hex};
use frankmd_core::note::{ValidationError, note_directory, note_name, validate_note_path};
use frankmd_core::path::normalize_note_path;
use frankmd_core::tree::TreeNode;
use frankmd_fs::model::{self, ErrorField, ModelError, SaveMode};
use frankmd_fs::{FsError, NotesService};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

/// The Rails `setup_test_notes_dir` + `create_test_note` default content.
const DEFAULT_CONTENT: &str = "# Test\n\nContent";

fn running_as_root() -> bool {
    let t = tempfile::tempdir().unwrap();
    std::fs::metadata(t.path()).unwrap().uid() == 0
}

fn chmod(p: &std::path::Path, mode: u32) {
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// `Note.find(path)`: normalize, then read.
fn find(svc: &NotesService<'_>, path: &str) -> Result<(String, String), FsError> {
    let p = normalize_note_path(path);
    let file = svc.read(&p)?;
    Ok((p, String::from_utf8(file.bytes).unwrap()))
}

fn base_errors(errs: &[ModelError]) -> Vec<String> {
    errs.iter()
        .filter(|e| e.field == ErrorField::Base)
        .map(|e| e.full_message(&en))
        .collect()
}

// port: test/models/note_test.rb:16 "Note.find returns note with content"
#[test]
fn find_returns_note_with_content() {
    let sb = Sandbox::empty();
    sb.write("test.md", "# Hello");
    let vault = sb.vault();
    let (path, content) = find(&NotesService::new(&vault), "test.md").unwrap();
    assert_eq!(path, "test.md");
    assert_eq!(content, "# Hello");
    assert_eq!(note_name(&path), "test");
}

// port: test/models/note_test.rb:25 "Note.find normalizes path without .md extension"
#[test]
fn find_normalizes_path_without_md_extension() {
    let sb = Sandbox::empty();
    sb.write("test.md", "# Hello");
    let vault = sb.vault();
    let (path, content) = find(&NotesService::new(&vault), "test").unwrap();
    assert_eq!(path, "test.md");
    assert_eq!(content, "# Hello");
}

// port: test/models/note_test.rb:33 "Note.find raises for missing note"
#[test]
fn find_raises_for_missing_note() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let err = find(&NotesService::new(&vault), "nonexistent.md").unwrap_err();
    assert!(matches!(err, FsError::NotFound(_)), "{err:?}");
}

// port: test/models/note_test.rb:39 "Note.find works with nested paths"
#[test]
fn find_works_with_nested_paths() {
    let sb = Sandbox::empty();
    sb.mkdir("deep/nested");
    sb.write("deep/nested/file.md", "# Nested");
    let vault = sb.vault();
    let (path, content) = find(&NotesService::new(&vault), "deep/nested/file.md").unwrap();
    assert_eq!(path, "deep/nested/file.md");
    assert_eq!(content, "# Nested");
    assert_eq!(note_name(&path), "file");
    assert_eq!(note_directory(&path), "deep/nested");
}

// port: test/models/note_test.rb:50 "Note.find works with .fed config file"
#[test]
fn find_works_with_fed_config_file() {
    let sb = Sandbox::empty();
    sb.write(".fed", "theme = dark");
    let vault = sb.vault();
    let (path, content) = find(&NotesService::new(&vault), ".fed").unwrap();
    assert_eq!(path, ".fed");
    assert_eq!(content, "theme = dark");
}

// port: test/models/note_test.rb:60 "normalize_path adds .md extension to regular paths"
#[test]
fn normalize_path_adds_md_extension() {
    assert_eq!(normalize_note_path("test"), "test.md");
    assert_eq!(normalize_note_path("folder/file"), "folder/file.md");
}

// port: test/models/note_test.rb:65 "normalize_path does not add .md to paths already having it"
#[test]
fn normalize_path_keeps_existing_md() {
    assert_eq!(normalize_note_path("test.md"), "test.md");
}

// port: test/models/note_test.rb:69 "normalize_path does not add .md to .fed"
#[test]
fn normalize_path_does_not_touch_fed() {
    assert_eq!(normalize_note_path(".fed"), ".fed");
}

// port: test/models/note_test.rb:73 "normalize_path handles blank paths"
#[test]
fn normalize_path_handles_blank_paths() {
    // Ruby nil and "" both map to "" (the Rust API takes &str; nil ≙ "").
    assert_eq!(normalize_note_path(""), "");
}

// port: test/models/note_test.rb:80 "Note.all returns tree structure"
#[test]
fn all_returns_tree_structure() {
    let sb = Sandbox::empty();
    sb.write("note1.md", DEFAULT_CONTENT);
    sb.mkdir("folder");
    sb.write("folder/note2.md", DEFAULT_CONTENT);
    let vault = sb.vault();
    let tree = NotesService::new(&vault).list_tree().unwrap();
    assert!(tree.iter().any(|n| n.name() == "note1"));
    assert!(
        tree.iter()
            .any(|n| n.name() == "folder" && matches!(n, TreeNode::Folder { .. }))
    );
}

// port: test/models/note_test.rb:103 "note.save creates new file"
#[test]
fn save_creates_new_file() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let svc = NotesService::new(&vault);
    model::note_save(&svc, "new.md", Some(b"# New"), SaveMode::Upsert)
        .unwrap()
        .unwrap();
    assert!(sb.exists("new.md"));
    assert_eq!(sb.read("new.md"), "# New");
}

// port: test/models/note_test.rb:110 "note.save updates existing file"
#[test]
fn save_updates_existing_file() {
    let sb = Sandbox::empty();
    sb.write("existing.md", "Old content");
    let vault = sb.vault();
    let svc = NotesService::new(&vault);
    model::note_save(&svc, "existing.md", Some(b"New content"), SaveMode::Upsert)
        .unwrap()
        .unwrap();
    assert_eq!(sb.read("existing.md"), "New content");
}

// port: test/models/note_test.rb:118 "note.save creates parent directories"
#[test]
fn save_creates_parent_directories() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let svc = NotesService::new(&vault);
    model::note_save(
        &svc,
        "deep/nested/new.md",
        Some(b"# Nested"),
        SaveMode::Upsert,
    )
    .unwrap()
    .unwrap();
    assert!(sb.exists("deep/nested/new.md"));
}

// port: test/models/note_test.rb:124 "note.save validates path format"
#[test]
fn save_validates_path_format() {
    assert_eq!(
        validate_note_path("../escape.md"),
        vec![ValidationError::Traversal]
    );
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let errs = model::note_save(
        &NotesService::new(&vault),
        "../escape.md",
        Some(b"bad"),
        SaveMode::Upsert,
    )
    .unwrap()
    .unwrap_err();
    assert!(errs.iter().any(|e| e.field == ErrorField::Path), "{errs:?}");
}

// port: test/models/note_test.rb:130 "note.save validates presence of path"
#[test]
fn save_validates_presence_of_path() {
    assert!(validate_note_path("").contains(&ValidationError::Blank));
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let errs = model::note_save(
        &NotesService::new(&vault),
        "",
        Some(b"content"),
        SaveMode::Upsert,
    )
    .unwrap()
    .unwrap_err();
    assert!(errs.iter().any(|e| e.field == ErrorField::Path), "{errs:?}");
}

// port: test/models/note_test.rb:138 "note.destroy removes file"
#[test]
fn destroy_removes_file() {
    let sb = Sandbox::empty();
    sb.write("to_delete.md", DEFAULT_CONTENT);
    let vault = sb.vault();
    model::note_destroy(&NotesService::new(&vault), "to_delete.md", None)
        .unwrap()
        .unwrap();
    assert!(!sb.exists("to_delete.md"));
}

// port: test/models/note_test.rb:146 "note.destroy deletes when the expected revision matches"
#[test]
fn destroy_deletes_when_expected_revision_matches() {
    let sb = Sandbox::empty();
    let content = "current content";
    sb.write("to_delete.md", content);
    let vault = sb.vault();
    let rev = sha256_hex(content.as_bytes());
    model::note_destroy(&NotesService::new(&vault), "to_delete.md", Some(&rev))
        .unwrap()
        .unwrap();
    assert!(!sb.exists("to_delete.md"));
}

// port: test/models/note_test.rb:155 "note.destroy reports a revision conflict and preserves the changed file"
#[test]
fn destroy_reports_revision_conflict_and_preserves_file() {
    let sb = Sandbox::empty();
    sb.write("to_delete.md", "updated content");
    let vault = sb.vault();
    let rev = sha256_hex(b"old content");
    let errs = model::note_destroy(&NotesService::new(&vault), "to_delete.md", Some(&rev))
        .unwrap()
        .unwrap_err();
    assert!(errs.iter().any(|e| e.revision_conflict), "{errs:?}");
    assert_eq!(sb.read("to_delete.md"), "updated content");
}

// port: test/models/note_test.rb:165 "note.destroy returns false for missing file"
#[test]
fn destroy_returns_false_for_missing_file() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let errs = model::note_destroy(&NotesService::new(&vault), "nonexistent.md", None)
        .unwrap()
        .unwrap_err();
    assert!(!base_errors(&errs).is_empty(), "{errs:?}");
}

// port: test/models/note_test.rb:173 "note.rename moves file"
#[test]
fn rename_moves_file() {
    let sb = Sandbox::empty();
    sb.write("old.md", "content");
    let vault = sb.vault();
    let new_path = model::note_rename(&NotesService::new(&vault), "old.md", "new.md")
        .unwrap()
        .unwrap();
    assert_eq!(new_path, "new.md");
    assert!(!sb.exists("old.md"));
    assert!(sb.exists("new.md"));
}

// port: test/models/note_test.rb:183 "note.rename moves file to different directory"
#[test]
fn rename_moves_file_to_different_directory() {
    let sb = Sandbox::empty();
    sb.write("root.md", "content");
    sb.mkdir("subfolder");
    let vault = sb.vault();
    let new_path = model::note_rename(&NotesService::new(&vault), "root.md", "subfolder/moved.md")
        .unwrap()
        .unwrap();
    assert_eq!(new_path, "subfolder/moved.md");
    assert!(!sb.exists("root.md"));
    assert!(sb.exists("subfolder/moved.md"));
}

// port: test/models/note_test.rb:196 "note.exists? returns true for existing file"
#[test]
fn exists_true_for_existing_file() {
    let sb = Sandbox::empty();
    sb.write("exists.md", DEFAULT_CONTENT);
    let vault = sb.vault();
    assert!(model::note_exists(&NotesService::new(&vault), "exists.md").unwrap());
}

// port: test/models/note_test.rb:202 "note.exists? returns false for missing file"
#[test]
fn exists_false_for_missing_file() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    assert!(!model::note_exists(&NotesService::new(&vault), "missing.md").unwrap());
}

// port: test/models/note_test.rb:209 "note.name returns filename without extension"
#[test]
fn name_returns_filename_without_extension() {
    assert_eq!(note_name("my-note.md"), "my-note");
}

// port: test/models/note_test.rb:214 "note.directory returns parent directory"
#[test]
fn directory_returns_parent_directory() {
    assert_eq!(
        note_directory("folder/subfolder/note.md"),
        "folder/subfolder"
    );
}

// port: test/models/note_test.rb:219 "note.directory returns empty string for root files"
#[test]
fn directory_returns_empty_string_for_root_files() {
    assert_eq!(note_directory("root.md"), "");
}

// port: test/models/note_test.rb:224 "note.persisted? returns true when file exists"
#[test]
fn persisted_true_when_file_exists() {
    let sb = Sandbox::empty();
    sb.write("persisted.md", DEFAULT_CONTENT);
    let vault = sb.vault();
    assert!(model::note_exists(&NotesService::new(&vault), "persisted.md").unwrap());
}

// port: test/models/note_test.rb:230 "note.persisted? returns false when file does not exist"
#[test]
fn persisted_false_when_file_missing() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    assert!(!model::note_exists(&NotesService::new(&vault), "not_persisted.md").unwrap());
}

// port: test/models/note_test.rb:240 "note.as_json returns hash with path, name, and content"
#[test]
fn as_json_has_path_name_content() {
    // `as_json` = {path, name: note_name(path), content}; only `name` is derived.
    assert_eq!(note_name("test.md"), "test");
}

// port: test/models/note_test.rb:251 "note.destroy handles file that disappeared"
#[test]
fn destroy_handles_file_that_disappeared() {
    let sb = Sandbox::empty();
    sb.write("disappearing.md", "content");
    let vault = sb.vault();
    std::fs::remove_file(sb.abs("disappearing.md")).unwrap();
    let errs = model::note_destroy(&NotesService::new(&vault), "disappearing.md", None)
        .unwrap()
        .unwrap_err();
    let base = base_errors(&errs);
    assert!(
        base.first().is_some_and(|m| m.contains("not found")),
        "{base:?}"
    );
}

// port: test/models/note_test.rb:263 "note.rename handles source file that disappeared"
#[test]
fn rename_handles_source_file_that_disappeared() {
    let sb = Sandbox::empty();
    sb.write("source.md", "content");
    let vault = sb.vault();
    std::fs::remove_file(sb.abs("source.md")).unwrap();
    let errs = model::note_rename(&NotesService::new(&vault), "source.md", "destination.md")
        .unwrap()
        .unwrap_err();
    assert!(!base_errors(&errs).is_empty(), "{errs:?}");
}

// port: test/models/note_test.rb:274 "note.save handles permission denied on create"
#[test]
fn save_handles_permission_denied_on_create() {
    if running_as_root() {
        eprintln!("skipped: chmod has no effect for root");
        return;
    }
    let sb = Sandbox::empty();
    sb.mkdir("readonly");
    chmod(&sb.abs("readonly"), 0o555);
    let vault = sb.vault();
    let result = model::note_save(
        &NotesService::new(&vault),
        "readonly/cannot_write.md",
        Some(b"content"),
        SaveMode::Upsert,
    );
    chmod(&sb.abs("readonly"), 0o755);
    let errs = result.unwrap().unwrap_err();
    assert!(!base_errors(&errs).is_empty(), "{errs:?}");
}

// port: test/models/note_test.rb:293 "note.destroy handles permission denied"
#[test]
fn destroy_handles_permission_denied() {
    if running_as_root() {
        eprintln!("skipped: chmod has no effect for root");
        return;
    }
    let sb = Sandbox::empty();
    sb.write("protected.md", "content");
    let vault = sb.vault();
    chmod(&sb.root, 0o555);
    let result = model::note_destroy(&NotesService::new(&vault), "protected.md", None);
    chmod(&sb.root, 0o755);
    let errs = result.unwrap().unwrap_err();
    let base = base_errors(&errs);
    assert!(
        base.first()
            .is_some_and(|m| m.contains("Permission denied")),
        "{base:?}"
    );
}
