//! Port of `test/controllers/folders_controller_test.rb` through the controller
//! emulation in `common::http`. Turbo-stream HTML is Rails plumbing; the
//! ported turbo tests assert the rendered tree *data* (rows, expanded set)
//! that `FoldersController#load_tree_for_turbo_stream` computes.
#![allow(
    missing_docs,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::disallowed_methods
)]

mod common;

use common::Sandbox;
use common::http::{dispatch, dispatch_turbo};
use frankmd_core::tree_view::{RowKind, TreeView};
use frankmd_testkit::serde_json::{Value, json};

const DEFAULT_NOTE: &str = "# Test\n\nContent";

fn rows(r: &common::http::HttpResponse) -> Vec<frankmd_core::tree_view::TreeRow> {
    match &r.tree.as_ref().expect("tree rendered").view {
        TreeView::Rows(rows) => rows.clone(),
        TreeView::Empty => vec![],
    }
}

fn expanded_paths(r: &common::http::HttpResponse) -> Vec<String> {
    rows(r)
        .into_iter()
        .filter(|row| row.kind == RowKind::Folder && row.expanded)
        .map(|row| row.path)
        .collect()
}

// port: test/controllers/folders_controller_test.rb:16 "create makes new folder"
#[test]
fn create_makes_new_folder() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(&sb, "POST", "/folders/new_folder", &Value::Null).status,
        201
    );
    assert!(sb.abs("new_folder").is_dir());
}

// port: test/controllers/folders_controller_test.rb:23 "create makes nested folder"
#[test]
fn create_makes_nested_folder() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(&sb, "POST", "/folders/parent/child", &Value::Null).status,
        201
    );
    assert!(sb.abs("parent/child").is_dir());
}

// port: test/controllers/folders_controller_test.rb:30 "create returns error if folder exists"
#[test]
fn create_returns_error_if_folder_exists() {
    let sb = Sandbox::empty();
    sb.mkdir("existing");
    assert_eq!(
        dispatch(&sb, "POST", "/folders/existing", &Value::Null).status,
        422
    );
}

// port: test/controllers/folders_controller_test.rb:39 "destroy removes empty folder"
#[test]
fn destroy_removes_empty_folder() {
    let sb = Sandbox::empty();
    sb.mkdir("empty_folder");
    assert_eq!(
        dispatch(&sb, "DELETE", "/folders/empty_folder", &Value::Null).status,
        200
    );
    assert!(!sb.exists("empty_folder"));
}

// port: test/controllers/folders_controller_test.rb:48 "destroy returns error for non-empty folder"
#[test]
fn destroy_returns_error_for_non_empty_folder() {
    let sb = Sandbox::empty();
    sb.mkdir("folder");
    sb.write("folder/note.md", DEFAULT_NOTE);
    assert_eq!(
        dispatch(&sb, "DELETE", "/folders/folder", &Value::Null).status,
        422
    );
    assert!(sb.exists("folder"));
}

// port: test/controllers/folders_controller_test.rb:59 "destroy returns 404 for missing folder"
#[test]
fn destroy_returns_404_for_missing_folder() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(&sb, "DELETE", "/folders/nonexistent", &Value::Null).status,
        404
    );
}

// port: test/controllers/folders_controller_test.rb:66 "rename moves folder"
#[test]
fn rename_moves_folder() {
    let sb = Sandbox::empty();
    sb.mkdir("old_name");
    sb.write("old_name/note.md", "Content");
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/folders/old_name/rename",
            &json!({"new_path": "new_name"})
        )
        .status,
        200
    );
    assert!(!sb.exists("old_name"));
    assert!(sb.abs("new_name").is_dir());
    assert!(sb.exists("new_name/note.md"));
}

// port: test/controllers/folders_controller_test.rb:78 "rename moves folder to different parent"
#[test]
fn rename_moves_folder_to_different_parent() {
    let sb = Sandbox::empty();
    sb.mkdir("source");
    sb.mkdir("target");
    sb.write("source/note.md", DEFAULT_NOTE);
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/folders/source/rename",
            &json!({"new_path": "target/source"})
        )
        .status,
        200
    );
    assert!(!sb.exists("source"));
    assert!(sb.abs("target/source").is_dir());
    assert!(sb.exists("target/source/note.md"));
}

// port: test/controllers/folders_controller_test.rb:91 "rename returns 404 for missing folder"
#[test]
fn rename_returns_404_for_missing_folder() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/folders/nonexistent/rename",
            &json!({"new_path": "new"})
        )
        .status,
        404
    );
}

// port: test/controllers/folders_controller_test.rb:98 "create responds with turbo stream when requested"
#[test]
fn create_turbo_tree_contains_the_new_folder() {
    let sb = Sandbox::empty();
    let r = dispatch_turbo(&sb, "POST", "/folders/turbo_folder", &Value::Null);
    assert_eq!(r.status, 201);
    assert!(
        rows(&r)
            .iter()
            .any(|row| row.path == "turbo_folder" && row.kind == RowKind::Folder)
    );
}

// port: test/controllers/folders_controller_test.rb:110 "create turbo stream includes expanded folder state"
#[test]
fn create_turbo_tree_keeps_expanded_folder_state() {
    let sb = Sandbox::empty();
    sb.mkdir("parent");
    sb.write("parent/note.md", DEFAULT_NOTE);
    let r = dispatch_turbo(
        &sb,
        "POST",
        "/folders/new_folder",
        &json!({"expanded": "parent"}),
    );
    assert_eq!(r.status, 201);
    assert_eq!(expanded_paths(&r), vec!["parent".to_owned()]);
}

// port: test/controllers/folders_controller_test.rb:122 "destroy responds with turbo stream when requested"
#[test]
fn destroy_turbo_tree_omits_the_deleted_folder() {
    let sb = Sandbox::empty();
    sb.mkdir("empty_folder");
    let r = dispatch_turbo(&sb, "DELETE", "/folders/empty_folder", &Value::Null);
    assert_eq!(r.status, 200);
    assert!(!r.tree_paths().contains(&"empty_folder".to_owned()));
}

// port: test/controllers/folders_controller_test.rb:135 "rename responds with turbo stream when requested"
#[test]
fn rename_turbo_tree_shows_new_path_only() {
    let sb = Sandbox::empty();
    sb.mkdir("old_folder");
    sb.write("old_folder/note.md", DEFAULT_NOTE);
    let r = dispatch_turbo(
        &sb,
        "POST",
        "/folders/old_folder/rename",
        &json!({"new_path": "new_folder", "expanded": ""}),
    );
    assert_eq!(r.status, 200);
    let paths = r.tree_paths();
    assert!(paths.contains(&"new_folder".to_owned()));
    assert!(
        !paths
            .iter()
            .any(|p| p == "old_folder" || p.starts_with("old_folder/"))
    );
}

// port: test/controllers/folders_controller_test.rb:151 "rename turbo stream remaps expanded folders from old to new path"
#[test]
fn rename_turbo_remaps_expanded_folders_from_old_to_new_path() {
    let sb = Sandbox::empty();
    sb.mkdir("project/src");
    sb.write("project/src/main.md", DEFAULT_NOTE);
    sb.mkdir("other");
    sb.write("other/note.md", DEFAULT_NOTE);
    let r = dispatch_turbo(
        &sb,
        "POST",
        "/folders/project/rename",
        &json!({"new_path": "app", "expanded": "project,project/src,other"}),
    );
    assert_eq!(r.status, 200);
    assert_eq!(
        r.tree.as_ref().unwrap().expanded,
        vec!["app", "app/src", "other"]
    );
    let mut expanded = expanded_paths(&r);
    expanded.sort();
    assert_eq!(expanded, vec!["app", "app/src", "other"]);
}

// port: test/controllers/folders_controller_test.rb:172 "rename turbo stream keeps unrelated expanded folders unchanged"
#[test]
fn rename_turbo_keeps_unrelated_expanded_folders_unchanged() {
    let sb = Sandbox::empty();
    sb.mkdir("alpha");
    sb.write("alpha/note.md", DEFAULT_NOTE);
    sb.mkdir("beta");
    sb.write("beta/note.md", DEFAULT_NOTE);
    let r = dispatch_turbo(
        &sb,
        "POST",
        "/folders/alpha/rename",
        &json!({"new_path": "gamma", "expanded": "alpha,beta"}),
    );
    assert_eq!(r.status, 200);
    let paths = r.tree_paths();
    assert!(paths.contains(&"beta".to_owned()));
    assert!(paths.contains(&"gamma".to_owned()));
    assert!(
        !paths
            .iter()
            .any(|p| p == "alpha" || p.starts_with("alpha/"))
    );
    let mut expanded = expanded_paths(&r);
    expanded.sort();
    assert_eq!(expanded, vec!["beta", "gamma"]);
}
