//! Port of `test/helpers/tree_helper_test.rb` — the data part only.
//!
//! Rails asserted on rendered HTML; the GPUI Explorer renders rows from
//! [`tree_rows`]. Each test asserts the row flags the HTML encoded
//! (`data-type`, `selected`, `expanded`, `hidden`, `draggable`, context menu).
//! Pure-markup assertions (CSS classes like `text-sm`, `tabindex="0"`, the
//! exact Stimulus `data-action` strings) are skipped: they are web wiring with
//! no GPUI equivalent; focus/keyboard rename+delete on every row kind is a UI
//! (frankmd-app) contract noted per test.
#![allow(missing_docs, clippy::unwrap_used)]

use frankmd_core::tree::TreeNode;
use frankmd_core::tree_view::{RowKind, TreeRow, TreeView, tree_rows};
use frankmd_i18n::{Catalog, Locale};
use frankmd_testkit::serde_json::{self, Value, json};
use std::collections::BTreeSet;

fn nodes(v: Value) -> Vec<TreeNode> {
    serde_json::from_value(v).unwrap()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn rows(view: TreeView) -> Vec<TreeRow> {
    match view {
        TreeView::Rows(r) => r,
        TreeView::Empty => panic!("expected rows, got Empty"),
    }
}

fn row<'a>(rows: &'a [TreeRow], path: &str) -> &'a TreeRow {
    rows.iter()
        .find(|r| r.path == path)
        .unwrap_or_else(|| panic!("no row {path}"))
}

// port: test/helpers/tree_helper_test.rb:8 "renders empty state when no items"
#[test]
fn renders_empty_state_when_no_items() {
    assert_eq!(tree_rows(&[], &set(&[]), ""), TreeView::Empty);
    // The empty state shows the localized sidebar.no_notes_yet text.
    let text = Catalog::load(Locale::En)
        .unwrap()
        .t("sidebar.no_notes_yet", &[]);
    assert_eq!(text, "No notes yet");
}

// port: test/helpers/tree_helper_test.rb:14 "renders file item with correct attributes"
#[test]
fn renders_file_item_with_correct_attributes() {
    let items = nodes(
        json!([{"name": "test", "path": "test.md", "type": "file", "file_type": "markdown"}]),
    );
    let r = rows(tree_rows(&items, &set(&[]), ""));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].path, "test.md");
    assert_eq!(r[0].kind, RowKind::Note);
    assert_eq!(r[0].name, "test");
    assert_eq!(r[0].depth, 0);
    assert!(r[0].visible);
}

// port: test/helpers/tree_helper_test.rb:25 "renders selected file with selected class"
#[test]
fn renders_selected_file_with_selected_class() {
    let items = nodes(
        json!([{"name": "test", "path": "test.md", "type": "file", "file_type": "markdown"}]),
    );
    assert!(rows(tree_rows(&items, &set(&[]), "test.md"))[0].selected);
    assert!(!rows(tree_rows(&items, &set(&[]), ""))[0].selected);
}

// port: test/helpers/tree_helper_test.rb:32 "renders folders as focusable explorer items with selection and keyboard actions"
// Focus / keyboard delete+rename are frankmd-app behaviors; the data part is a Folder row.
#[test]
fn renders_folders_as_focusable_explorer_items() {
    let items =
        nodes(json!([{"name": "folder1", "path": "folder1", "type": "folder", "children": []}]));
    let r = rows(tree_rows(&items, &set(&[]), ""));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, RowKind::Folder);
    assert_eq!(r[0].path, "folder1");
}

// port: test/helpers/tree_helper_test.rb:42 "renders folder with children"
#[test]
fn renders_folder_with_children() {
    let items = nodes(
        json!([{"name": "folder1", "path": "folder1", "type": "folder",
        "children": [{"name": "note", "path": "folder1/note.md", "type": "file", "file_type": "markdown"}]}]),
    );
    let r = rows(tree_rows(&items, &set(&[]), ""));
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].kind, RowKind::Folder);
    assert!(!r[0].expanded);
    let child = row(&r, "folder1/note.md");
    assert_eq!(child.depth, 1);
    assert!(!child.visible, "collapsed folder hides its children");
}

// port: test/helpers/tree_helper_test.rb:56 "renders expanded folder without hidden class"
#[test]
fn renders_expanded_folder_without_hidden_class() {
    let items = nodes(
        json!([{"name": "folder1", "path": "folder1", "type": "folder",
        "children": [{"name": "note", "path": "folder1/note.md", "type": "file", "file_type": "markdown"}]}]),
    );
    let r = rows(tree_rows(&items, &set(&["folder1"]), ""));
    assert!(row(&r, "folder1").expanded);
    assert!(row(&r, "folder1/note.md").visible);
}

// port: test/helpers/tree_helper_test.rb:68 "renders config file without drag attributes"
#[test]
fn renders_config_file_without_drag_attributes() {
    let items =
        nodes(json!([{"name": ".fed", "path": ".fed", "type": "file", "file_type": "config"}]));
    let r = rows(tree_rows(&items, &set(&[]), ""));
    assert_eq!(r[0].kind, RowKind::Config);
    assert!(!r[0].draggable);
    assert!(!r[0].context_menu);
    // Still selectable.
    assert!(rows(tree_rows(&items, &set(&[]), ".fed"))[0].selected);
}

// port: test/helpers/tree_helper_test.rb:83 "renders regular files with drag and context menu"
#[test]
fn renders_regular_files_with_drag_and_context_menu() {
    let items = nodes(
        json!([{"name": "test", "path": "test.md", "type": "file", "file_type": "markdown"}]),
    );
    let r = rows(tree_rows(&items, &set(&[]), ""));
    assert!(r[0].draggable);
    assert!(r[0].context_menu);
}

// port: test/helpers/tree_helper_test.rb:93 "renders folders with drag-drop actions"
// Drop-target wiring is UI; the data part: folders are draggable with a context menu.
#[test]
fn renders_folders_with_drag_drop_actions() {
    let items =
        nodes(json!([{"name": "folder1", "path": "folder1", "type": "folder", "children": []}]));
    let r = rows(tree_rows(&items, &set(&[]), ""));
    assert!(r[0].draggable);
    assert!(r[0].context_menu);
}

// port: test/helpers/tree_helper_test.rb:103 "handles string keys in items"
// Ruby symbol vs string keys has no Rust analogue; the JSON item shape deserializes into TreeNode.
#[test]
fn handles_string_keys_in_items() {
    let items = nodes(
        json!([{"name": "test", "path": "test.md", "type": "file", "file_type": "markdown"}]),
    );
    let r = rows(tree_rows(&items, &set(&[]), ""));
    assert_eq!(r[0].path, "test.md");
    assert_eq!(r[0].name, "test");
}

// port: test/helpers/tree_helper_test.rb:111 "returns empty string for empty children at non-root depth"
#[test]
fn returns_empty_string_for_empty_children_at_non_root_depth() {
    let items =
        nodes(json!([{"name": "empty", "path": "empty", "type": "folder", "children": []}]));
    let r = rows(tree_rows(&items, &set(&[]), ""));
    // Only the folder row: no "no notes yet" placeholder for the empty nested list.
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, RowKind::Folder);
}
