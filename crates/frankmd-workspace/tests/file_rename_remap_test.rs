//! Port of the category-A tests of test/javascript/controllers/app_file_rename.test.js.
//! The original re-implements app_controller's path remapping rules; here they
//! are the shared `draft_storage::remap_path` (path-segment boundary).

use frankmd_workspace::draft_storage::{ItemKind, remap_path};
use std::collections::BTreeSet;

fn remap_expanded(set: &[&str], old: &str, new: &str) -> BTreeSet<String> {
    set.iter().map(|p| remap_path(p, old, new, ItemKind::Folder)).collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// `remapCurrentFile`: only files *inside* the renamed folder move.
fn remap_current(current: Option<&str>, old: &str, new: &str) -> Option<String> {
    current.map(|c| remap_path(c, old, new, ItemKind::Folder))
}

// port: test/javascript/controllers/app_file_rename.test.js:30 "remaps the renamed folder itself"
#[test]
fn remaps_the_renamed_folder_itself() {
    assert_eq!(remap_expanded(&["project"], "project", "app"), set(&["app"]));
}

// port: test/javascript/controllers/app_file_rename.test.js:36 "remaps nested children of renamed folder"
#[test]
fn remaps_nested_children_of_renamed_folder() {
    assert_eq!(
        remap_expanded(&["project", "project/src", "project/src/lib"], "project", "app"),
        set(&["app", "app/src", "app/src/lib"])
    );
}

// port: test/javascript/controllers/app_file_rename.test.js:42 "leaves unrelated folders unchanged"
#[test]
fn leaves_unrelated_folders_unchanged() {
    assert_eq!(remap_expanded(&["project", "other", "docs/api"], "project", "app"), set(&["app", "other", "docs/api"]));
}

// port: test/javascript/controllers/app_file_rename.test.js:48 "does not remap folders that merely share a prefix"
#[test]
fn does_not_remap_folders_that_merely_share_a_prefix() {
    assert_eq!(
        remap_expanded(&["project", "project-old", "project-old/src"], "project", "app"),
        set(&["app", "project-old", "project-old/src"])
    );
}

// port: test/javascript/controllers/app_file_rename.test.js:55 "handles empty expanded set"
#[test]
fn handles_empty_expanded_set() {
    assert_eq!(remap_expanded(&[], "old", "new"), set(&[]));
}

// port: test/javascript/controllers/app_file_rename.test.js:60 "handles deep nesting rename"
#[test]
fn handles_deep_nesting_rename() {
    assert_eq!(
        remap_expanded(&["a/b/c", "a/b/c/d", "a/b/c/d/e"], "a/b/c", "a/b/renamed"),
        set(&["a/b/renamed", "a/b/renamed/d", "a/b/renamed/d/e"])
    );
}

// port: test/javascript/controllers/app_file_rename.test.js:68 "remaps file inside renamed folder"
#[test]
fn remaps_file_inside_renamed_folder() {
    assert_eq!(remap_current(Some("project/src/main.md"), "project", "app").as_deref(), Some("app/src/main.md"));
}

// port: test/javascript/controllers/app_file_rename.test.js:73 "remaps file directly in renamed folder"
#[test]
fn remaps_file_directly_in_renamed_folder() {
    assert_eq!(
        remap_current(Some("docs/readme.md"), "docs", "documentation").as_deref(),
        Some("documentation/readme.md")
    );
}

// port: test/javascript/controllers/app_file_rename.test.js:78 "does not remap file outside renamed folder"
#[test]
fn does_not_remap_file_outside_renamed_folder() {
    assert_eq!(remap_current(Some("other/file.md"), "project", "app").as_deref(), Some("other/file.md"));
}

// port: test/javascript/controllers/app_file_rename.test.js:83 "does not remap file with shared prefix but not in folder"
#[test]
fn does_not_remap_file_with_shared_prefix() {
    assert_eq!(remap_current(Some("project-v2/file.md"), "project", "app").as_deref(), Some("project-v2/file.md"));
}

// port: test/javascript/controllers/app_file_rename.test.js:89 "handles null currentFile"
#[test]
fn handles_null_current_file() {
    assert_eq!(remap_current(None, "project", "app"), None);
}

// port: test/javascript/controllers/app_file_rename.test.js:94 "handles undefined currentFile"
// (JS null and undefined both map to `None`.)
#[test]
fn handles_undefined_current_file() {
    assert_eq!(remap_current(None, "project", "app"), None);
}

// contract: file renames match exactly; reverse remap is the inverse
#[test]
fn file_remap_is_exact_and_reverse_remap_inverts() {
    use frankmd_workspace::draft_storage::reverse_remap_path;
    assert_eq!(remap_path("a.md", "a.md", "b.md", ItemKind::File), "b.md");
    assert_eq!(remap_path("a.md/child.md", "a.md", "b.md", ItemKind::File), "a.md/child.md");
    assert_eq!(reverse_remap_path("archive/x/y.md", "docs", "archive", ItemKind::Folder).as_deref(), Some("docs/x/y.md"));
    assert_eq!(reverse_remap_path("archive-old/y.md", "docs", "archive", ItemKind::Folder), None);
    assert_eq!(reverse_remap_path("b.md", "a.md", "b.md", ItemKind::File).as_deref(), Some("a.md"));
}
