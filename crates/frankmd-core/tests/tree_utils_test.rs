//! Port of `test/javascript/tree_utils.test.js` (`flattenTree`) through
//! [`frankmd_core::tree::flatten_files`].
//!
//! JS-only shapes: `null`/`undefined` input has no typed analogue (the empty
//! slice covers it); "preserves file metadata" maps to the typed fields
//! (`file_type`) — the arbitrary `size` property has no TreeNode field.
#![allow(missing_docs, clippy::unwrap_used)]

use frankmd_core::tree::{FileType, TreeNode, flatten_files};
use frankmd_testkit::serde_json::{self, Value, json};

fn nodes(v: Value) -> Vec<TreeNode> {
    serde_json::from_value(v).unwrap()
}

fn paths(v: &[&TreeNode]) -> Vec<String> {
    v.iter().map(|n| n.path().as_str().to_owned()).collect()
}

// port: test/javascript/tree_utils.test.js:5 "returns empty array for empty input"
#[test]
fn returns_empty_array_for_empty_input() {
    assert!(flatten_files(&[]).is_empty());
}

// port: test/javascript/tree_utils.test.js:11 "returns files from flat list"
#[test]
fn returns_files_from_flat_list() {
    let t = nodes(json!([
        {"type": "file", "name": "a.md", "path": "a.md"},
        {"type": "file", "name": "b.md", "path": "b.md"}
    ]));
    let r = flatten_files(&t);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].name(), "a.md");
    assert_eq!(r[1].name(), "b.md");
}

// port: test/javascript/tree_utils.test.js:22 "extracts files from nested folders"
#[test]
fn extracts_files_from_nested_folders() {
    let t = nodes(json!([
        {"type": "folder", "name": "docs", "path": "docs", "children": [
            {"type": "file", "name": "readme.md", "path": "docs/readme.md"}
        ]},
        {"type": "file", "name": "root.md", "path": "root.md"}
    ]));
    let r = flatten_files(&t);
    assert_eq!(r.len(), 2);
    let p = paths(&r);
    assert!(p.contains(&"docs/readme.md".to_owned()));
    assert!(p.contains(&"root.md".to_owned()));
}

// port: test/javascript/tree_utils.test.js:40 "handles deeply nested folders"
#[test]
fn handles_deeply_nested_folders() {
    let t = nodes(json!([
        {"type": "folder", "name": "a", "path": "a", "children": [
            {"type": "folder", "name": "b", "path": "a/b", "children": [
                {"type": "folder", "name": "c", "path": "a/b/c", "children": [
                    {"type": "file", "name": "deep.md", "path": "a/b/c/deep.md"}
                ]}
            ]}
        ]}
    ]));
    let r = flatten_files(&t);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].path().as_str(), "a/b/c/deep.md");
}

// port: test/javascript/tree_utils.test.js:70 "ignores empty folders"
#[test]
fn ignores_empty_folders() {
    let t = nodes(json!([
        {"type": "folder", "name": "empty", "path": "empty", "children": []},
        {"type": "file", "name": "file.md", "path": "file.md"}
    ]));
    let r = flatten_files(&t);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name(), "file.md");
}

// port: test/javascript/tree_utils.test.js:85 "handles folder without children property"
#[test]
fn handles_folder_without_children_property() {
    let t = nodes(json!([
        {"type": "folder", "name": "no-children", "path": "no-children"},
        {"type": "file", "name": "file.md", "path": "file.md"}
    ]));
    let r = flatten_files(&t);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name(), "file.md");
}

// port: test/javascript/tree_utils.test.js:100 "preserves file metadata"
#[test]
fn preserves_file_metadata() {
    let t = nodes(json!([
        {"type": "file", "name": "test.md", "path": "test.md", "file_type": "markdown", "mtime": 1024}
    ]));
    let r = flatten_files(&t);
    match r[0] {
        TreeNode::File {
            file_type, mtime, ..
        } => {
            assert_eq!(*file_type, FileType::Markdown);
            assert_eq!(*mtime, 1024);
        }
        TreeNode::Folder { .. } => panic!("expected a file"),
    }
}
