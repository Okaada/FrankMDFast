//! Port of `test/models/folder_test.rb` (the `Folder` model: `NotesService` +
//! `frankmd_fs::model` + `frankmd_core::note`).
//!
//! Skipped: `folder.to_param` (Rails routing).
#![allow(clippy::disallowed_methods, clippy::unwrap_used, missing_docs, unreachable_pub)]

mod common;

use common::{Sandbox, en};
use frankmd_core::note::{ValidationError, folder_name, folder_parent_path, validate_folder_path};
use frankmd_fs::model::{self, ErrorField, ModelError};
use frankmd_fs::{FsError, NotesService};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const DEFAULT_CONTENT: &str = "# Test\n\nContent";

fn running_as_root() -> bool {
    let t = tempfile::tempdir().unwrap();
    std::fs::metadata(t.path()).unwrap().uid() == 0
}

fn chmod(p: &std::path::Path, mode: u32) {
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn base_errors(errs: &[ModelError]) -> Vec<String> {
    errs.iter().filter(|e| e.field == ErrorField::Base).map(|e| e.full_message(&en)).collect()
}

/// `Folder.find(path)`: raises NotFound unless a directory is there.
fn find(svc: &NotesService<'_>, path: &str) -> Result<(String, String, Option<String>), FsError> {
    if !model::folder_exists(svc, path)? {
        return Err(FsError::NotFound(format!("Folder not found: {path}")));
    }
    Ok((path.to_owned(), folder_name(path), folder_parent_path(path)))
}

fn is_dir(sb: &Sandbox, rel: &str) -> bool {
    sb.abs(rel).is_dir()
}

// port: test/models/folder_test.rb:16 "Folder.find returns folder"
#[test]
fn find_returns_folder() {
    let sb = Sandbox::empty();
    sb.mkdir("my_folder");
    let vault = sb.vault();
    let (path, name, _) = find(&NotesService::new(&vault), "my_folder").unwrap();
    assert_eq!(path, "my_folder");
    assert_eq!(name, "my_folder");
}

// port: test/models/folder_test.rb:24 "Folder.find raises for missing folder"
#[test]
fn find_raises_for_missing_folder() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let err = find(&NotesService::new(&vault), "nonexistent").unwrap_err();
    assert!(matches!(err, FsError::NotFound(_)), "{err:?}");
}

// port: test/models/folder_test.rb:30 "Folder.find works with nested paths"
#[test]
fn find_works_with_nested_paths() {
    let sb = Sandbox::empty();
    sb.mkdir("deep/nested/folder");
    let vault = sb.vault();
    let (path, name, parent) = find(&NotesService::new(&vault), "deep/nested/folder").unwrap();
    assert_eq!(path, "deep/nested/folder");
    assert_eq!(name, "folder");
    assert_eq!(parent.as_deref(), Some("deep/nested"));
}

// port: test/models/folder_test.rb:41 "folder.create makes directory"
#[test]
fn create_makes_directory() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    model::folder_create(&NotesService::new(&vault), "new_folder").unwrap().unwrap();
    assert!(is_dir(&sb, "new_folder"));
}

// port: test/models/folder_test.rb:47 "folder.create makes nested directories"
#[test]
fn create_makes_nested_directories() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    model::folder_create(&NotesService::new(&vault), "deep/nested/new_folder").unwrap().unwrap();
    assert!(is_dir(&sb, "deep/nested/new_folder"));
}

// port: test/models/folder_test.rb:53 "folder.create returns false if folder exists"
#[test]
fn create_returns_false_if_folder_exists() {
    let sb = Sandbox::empty();
    sb.mkdir("existing_folder");
    let vault = sb.vault();
    let errs = model::folder_create(&NotesService::new(&vault), "existing_folder").unwrap().unwrap_err();
    assert!(!base_errors(&errs).is_empty(), "{errs:?}");
}

// port: test/models/folder_test.rb:61 "folder.create validates path format"
#[test]
fn create_validates_path_format() {
    assert_eq!(validate_folder_path("../escape"), vec![ValidationError::Traversal]);
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let errs = model::folder_create(&NotesService::new(&vault), "../escape").unwrap().unwrap_err();
    assert!(errs.iter().any(|e| e.field == ErrorField::Path), "{errs:?}");
}

// port: test/models/folder_test.rb:67 "folder.create validates presence of path"
#[test]
fn create_validates_presence_of_path() {
    assert!(validate_folder_path("").contains(&ValidationError::Blank));
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let errs = model::folder_create(&NotesService::new(&vault), "").unwrap().unwrap_err();
    assert!(errs.iter().any(|e| e.field == ErrorField::Path), "{errs:?}");
}

// port: test/models/folder_test.rb:75 "folder.destroy removes empty directory"
#[test]
fn destroy_removes_empty_directory() {
    let sb = Sandbox::empty();
    sb.mkdir("empty_folder");
    let vault = sb.vault();
    model::folder_destroy(&NotesService::new(&vault), "empty_folder").unwrap().unwrap();
    assert!(!sb.exists("empty_folder"));
}

// port: test/models/folder_test.rb:83 "folder.destroy fails for non-empty directory"
#[test]
fn destroy_fails_for_non_empty_directory() {
    let sb = Sandbox::empty();
    sb.mkdir("folder_with_content");
    sb.write("folder_with_content/note.md", DEFAULT_CONTENT);
    let vault = sb.vault();
    let errs = model::folder_destroy(&NotesService::new(&vault), "folder_with_content").unwrap().unwrap_err();
    assert!(!base_errors(&errs).is_empty(), "{errs:?}");
    assert!(sb.exists("folder_with_content"));
}

// port: test/models/folder_test.rb:93 "folder.destroy returns false for missing folder"
#[test]
fn destroy_returns_false_for_missing_folder() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let errs = model::folder_destroy(&NotesService::new(&vault), "nonexistent").unwrap().unwrap_err();
    assert!(!base_errors(&errs).is_empty(), "{errs:?}");
}

// port: test/models/folder_test.rb:101 "folder.rename moves folder"
#[test]
fn rename_moves_folder() {
    let sb = Sandbox::empty();
    sb.mkdir("old_folder");
    let vault = sb.vault();
    let path = model::folder_rename(&NotesService::new(&vault), "old_folder", "new_folder").unwrap().unwrap();
    assert_eq!(path, "new_folder");
    assert!(!sb.exists("old_folder"));
    assert!(sb.exists("new_folder"));
}

// port: test/models/folder_test.rb:111 "folder.rename moves folder with contents"
#[test]
fn rename_moves_folder_with_contents() {
    let sb = Sandbox::empty();
    sb.mkdir("old_folder");
    sb.write("old_folder/note.md", "content");
    let vault = sb.vault();
    model::folder_rename(&NotesService::new(&vault), "old_folder", "new_folder").unwrap().unwrap();
    assert!(!sb.exists("old_folder"));
    assert!(sb.exists("new_folder"));
    assert!(sb.exists("new_folder/note.md"));
}

// port: test/models/folder_test.rb:122 "folder.rename updates Hugo frontmatter slug in index.md"
#[test]
fn rename_updates_hugo_frontmatter_slug() {
    let sb = Sandbox::empty();
    sb.mkdir("old-slug");
    let hugo = "---\ntitle: \"My Blog Post\"\nslug: \"old-slug\"\ndate: 2026-02-01T10:00:00-03:00\ndraft: true\ntags:\n-\n---\n\nContent here\n";
    sb.write("old-slug/index.md", hugo);
    let vault = sb.vault();
    model::folder_rename(&NotesService::new(&vault), "old-slug", "new-slug").unwrap().unwrap();
    let updated = sb.read("new-slug/index.md");
    assert!(updated.contains("slug: \"new-slug\""), "{updated}");
    assert!(!updated.contains("slug: \"old-slug\""), "{updated}");
    assert!(updated.contains("title: \"My Blog Post\""), "{updated}");
    assert!(updated.contains("Content here"), "{updated}");
}

// port: test/models/folder_test.rb:148 "folder.rename updates slug with accented folder name"
#[test]
fn rename_updates_slug_with_accented_folder_name() {
    let sb = Sandbox::empty();
    sb.mkdir("old-post");
    sb.write("old-post/index.md", "---\ntitle: \"Test\"\nslug: \"old-post\"\n---\n\nBody\n");
    let vault = sb.vault();
    model::folder_rename(&NotesService::new(&vault), "old-post", "café-açaí").unwrap().unwrap();
    let updated = sb.read("café-açaí/index.md");
    assert!(updated.contains("slug: \"cafe-acai\""), "{updated}");
}

// port: test/models/folder_test.rb:167 "folder.rename does not modify non-Hugo index.md"
#[test]
fn rename_does_not_modify_non_hugo_index() {
    let sb = Sandbox::empty();
    sb.mkdir("regular-folder");
    let regular = "# Just a regular note\n\nNo frontmatter here";
    sb.write("regular-folder/index.md", regular);
    let vault = sb.vault();
    model::folder_rename(&NotesService::new(&vault), "regular-folder", "renamed-folder").unwrap().unwrap();
    assert_eq!(sb.read("renamed-folder/index.md"), regular);
}

// port: test/models/folder_test.rb:179 "folder.rename does not modify frontmatter without slug field"
#[test]
fn rename_does_not_modify_frontmatter_without_slug() {
    let sb = Sandbox::empty();
    sb.mkdir("no-slug-folder");
    let fm = "---\ntitle: \"No Slug\"\ndate: 2026-02-01\n---\n\nContent\n";
    sb.write("no-slug-folder/index.md", fm);
    let vault = sb.vault();
    model::folder_rename(&NotesService::new(&vault), "no-slug-folder", "renamed-no-slug").unwrap().unwrap();
    assert_eq!(sb.read("renamed-no-slug/index.md"), fm);
}

// port: test/models/folder_test.rb:200 "folder.exists? returns true for existing directory"
#[test]
fn exists_true_for_existing_directory() {
    let sb = Sandbox::empty();
    sb.mkdir("exists");
    let vault = sb.vault();
    assert!(model::folder_exists(&NotesService::new(&vault), "exists").unwrap());
}

// port: test/models/folder_test.rb:206 "folder.exists? returns false for missing directory"
#[test]
fn exists_false_for_missing_directory() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    assert!(!model::folder_exists(&NotesService::new(&vault), "missing").unwrap());
}

// port: test/models/folder_test.rb:211 "folder.exists? returns false for files"
#[test]
fn exists_false_for_files() {
    let sb = Sandbox::empty();
    sb.write("file.md", DEFAULT_CONTENT);
    let vault = sb.vault();
    assert!(!model::folder_exists(&NotesService::new(&vault), "file.md").unwrap());
}

// port: test/models/folder_test.rb:219 "folder.name returns folder name"
#[test]
fn name_returns_folder_name() {
    assert_eq!(folder_name("my_folder"), "my_folder");
}

// port: test/models/folder_test.rb:224 "folder.name returns last component of nested path"
#[test]
fn name_returns_last_component() {
    assert_eq!(folder_name("deep/nested/folder"), "folder");
}

// port: test/models/folder_test.rb:229 "folder.parent_path returns parent directory"
#[test]
fn parent_path_returns_parent_directory() {
    assert_eq!(folder_parent_path("parent/child").as_deref(), Some("parent"));
}

// port: test/models/folder_test.rb:234 "folder.parent_path returns nil for root folders"
#[test]
fn parent_path_none_for_root_folders() {
    assert_eq!(folder_parent_path("root_folder"), None);
}

// port: test/models/folder_test.rb:239 "folder.persisted? returns true when directory exists"
#[test]
fn persisted_true_when_directory_exists() {
    let sb = Sandbox::empty();
    sb.mkdir("persisted");
    let vault = sb.vault();
    assert!(model::folder_exists(&NotesService::new(&vault), "persisted").unwrap());
}

// port: test/models/folder_test.rb:245 "folder.persisted? returns false when directory does not exist"
#[test]
fn persisted_false_when_directory_missing() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    assert!(!model::folder_exists(&NotesService::new(&vault), "not_persisted").unwrap());
}

// port: test/models/folder_test.rb:257 "folder.children returns children items"
#[test]
fn children_returns_children_items() {
    let sb = Sandbox::empty();
    sb.mkdir("parent");
    sb.write("parent/child1.md", DEFAULT_CONTENT);
    sb.write("parent/child2.md", DEFAULT_CONTENT);
    sb.mkdir("parent/subfolder");
    let vault = sb.vault();
    assert_eq!(NotesService::new(&vault).folder_children("parent").unwrap().len(), 3);
}

// port: test/models/folder_test.rb:270 "folder.children returns empty array for empty folder"
#[test]
fn children_empty_for_empty_folder() {
    let sb = Sandbox::empty();
    sb.mkdir("empty");
    let vault = sb.vault();
    assert!(NotesService::new(&vault).folder_children("empty").unwrap().is_empty());
}

// port: test/models/folder_test.rb:277 "folder.children returns empty array for nonexistent folder"
#[test]
fn children_empty_for_nonexistent_folder() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    assert!(NotesService::new(&vault).folder_children("nonexistent").unwrap().is_empty());
}

// port: test/models/folder_test.rb:284 "folder.destroy handles folder that disappeared"
#[test]
fn destroy_handles_folder_that_disappeared() {
    let sb = Sandbox::empty();
    sb.mkdir("disappearing");
    let vault = sb.vault();
    std::fs::remove_dir_all(sb.abs("disappearing")).unwrap();
    let errs = model::folder_destroy(&NotesService::new(&vault), "disappearing").unwrap().unwrap_err();
    let base = base_errors(&errs);
    assert!(base.first().is_some_and(|m| m.contains("not found")), "{base:?}");
}

// port: test/models/folder_test.rb:296 "folder.rename handles source folder that disappeared"
#[test]
fn rename_handles_source_folder_that_disappeared() {
    let sb = Sandbox::empty();
    sb.mkdir("source_folder");
    let vault = sb.vault();
    std::fs::remove_dir_all(sb.abs("source_folder")).unwrap();
    let errs = model::folder_rename(&NotesService::new(&vault), "source_folder", "destination_folder")
        .unwrap()
        .unwrap_err();
    assert!(!base_errors(&errs).is_empty(), "{errs:?}");
}

// port: test/models/folder_test.rb:307 "folder.create handles permission denied"
#[test]
fn create_handles_permission_denied() {
    if running_as_root() {
        eprintln!("skipped: chmod has no effect for root");
        return;
    }
    let sb = Sandbox::empty();
    let vault = sb.vault();
    chmod(&sb.root, 0o555);
    let result = model::folder_create(&NotesService::new(&vault), "cannot_create");
    chmod(&sb.root, 0o755);
    let errs = result.unwrap().unwrap_err();
    let base = base_errors(&errs);
    assert!(base.first().is_some_and(|m| m.contains("Permission denied")), "{base:?}");
}

// port: test/models/folder_test.rb:324 "folder.destroy handles permission denied"
#[test]
fn destroy_handles_permission_denied() {
    if running_as_root() {
        eprintln!("skipped: chmod has no effect for root");
        return;
    }
    let sb = Sandbox::empty();
    sb.mkdir("protected_folder");
    let vault = sb.vault();
    chmod(&sb.root, 0o555);
    let result = model::folder_destroy(&NotesService::new(&vault), "protected_folder");
    chmod(&sb.root, 0o755);
    let errs = result.unwrap().unwrap_err();
    let base = base_errors(&errs);
    assert!(base.first().is_some_and(|m| m.contains("Permission denied")), "{base:?}");
}
