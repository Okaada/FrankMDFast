//! Port of the category-A parts of `test/controllers/notes_controller_test.rb`,
//! driven through the controller emulation in `common::http` (the HTTP layer
//! itself is Rails plumbing; what is under test is the note behavior behind
//! each endpoint). HTML-only (C) and search (frankmd-search) tests are not
//! ported here; see `docs/testing/port-notes-fs.md`.
#![allow(
    missing_docs,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::disallowed_methods
)]

mod common;

use common::http::{dispatch, dispatch_turbo};
use common::{Sandbox, sha256_hex};
use frankmd_core::tree_view::TreeView;
use frankmd_testkit::serde_json::{Value, json};

const DEFAULT_NOTE: &str = "# Test\n\nContent";

fn none() -> Value {
    Value::Null
}

// port: test/controllers/notes_controller_test.rb:166 "index refuses a canonical alias for the root config file"
#[test]
fn index_refuses_a_canonical_alias_for_the_root_config_file() {
    let sb = Sandbox::empty();
    let r = dispatch(&sb, "GET", "/", &json!({"file": "./.fed"}));
    assert_eq!(r.status, 403);
}

// port: test/controllers/notes_controller_test.rb:184 "tree returns HTML file tree"
#[test]
fn tree_returns_html_file_tree() {
    let sb = Sandbox::empty();
    sb.write("note1.md", DEFAULT_NOTE);
    sb.mkdir("folder1");
    sb.write("folder1/note2.md", DEFAULT_NOTE);
    let r = dispatch(&sb, "GET", "/notes/tree", &none());
    assert_eq!(r.status, 200);
    let TreeView::Rows(rows) = &r.tree.as_ref().unwrap().view else {
        panic!("expected rows")
    };
    assert!(rows.iter().any(|row| row.path == "note1.md" && row.kind == frankmd_core::tree_view::RowKind::Note));
    assert!(
        rows.iter().any(
            |row| row.path == "folder1" && row.kind == frankmd_core::tree_view::RowKind::Folder
        )
    );
}

// port: test/controllers/notes_controller_test.rb:198 "tree accepts expanded and selected params"
#[test]
fn tree_accepts_expanded_and_selected_params() {
    let sb = Sandbox::empty();
    sb.mkdir("folder1");
    sb.write("folder1/note1.md", DEFAULT_NOTE);
    let r = dispatch(
        &sb,
        "GET",
        "/notes/tree",
        &json!({"expanded": "folder1", "selected": "folder1/note1.md"}),
    );
    assert_eq!(r.status, 200);
    let TreeView::Rows(rows) = &r.tree.as_ref().unwrap().view else {
        panic!("expected rows")
    };
    assert!(rows.iter().any(|row| row.path == "folder1" && row.expanded));
    assert!(
        rows.iter()
            .any(|row| row.path == "folder1/note1.md" && row.selected && row.visible)
    );
}

// port: test/controllers/notes_controller_test.rb:213 "show returns note content"
#[test]
fn show_returns_note_content() {
    let sb = Sandbox::empty();
    sb.write("test.md", "# Hello\n\nWorld");
    let r = dispatch(&sb, "GET", "/notes/test.md", &none());
    assert_eq!(r.status, 200);
    assert_eq!(r.body["path"], "test.md");
    assert_eq!(r.body["content"], "# Hello\n\nWorld");
    assert_eq!(r.body["revision"], sha256_hex(b"# Hello\n\nWorld"));
}

// port: test/controllers/notes_controller_test.rb:225 "show returns 404 for missing note"
#[test]
fn show_returns_404_for_missing_note() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(&sb, "GET", "/notes/nonexistent.md", &none()).status,
        404
    );
}

// port: test/controllers/notes_controller_test.rb:230 "show refuses the root config file"
#[test]
fn show_refuses_the_root_config_file() {
    let sb = Sandbox::empty();
    sb.write(".fed", "secret-token");
    let r = dispatch(&sb, "GET", "/notes/.fed", &none());
    assert_eq!(r.status, 403);
    assert!(!r.body.to_string().contains("secret-token"));
}

// port: test/controllers/notes_controller_test.rb:239 "show refuses a percent-decoded config file alias"
#[test]
fn show_refuses_a_percent_decoded_config_file_alias() {
    let sb = Sandbox::empty();
    sb.write(".fed", "secret-token");
    let r = dispatch(&sb, "GET", "/notes/foo/%2E%2E/.fed", &none());
    assert_eq!(r.status, 403);
    assert!(!r.body.to_string().contains("secret-token"));
}

// port: test/controllers/notes_controller_test.rb:250 "show serves image files from notes directory"
#[test]
fn show_serves_image_files_from_notes_directory() {
    let sb = Sandbox::empty();
    sb.mkdir("images");
    std::fs::write(sb.abs("images/test.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    let r = dispatch(&sb, "GET", "/notes/images/test.png", &none());
    assert_eq!(r.status, 200);
    assert_eq!(r.content_type.as_deref(), Some("image/png"));
}

// port: test/controllers/notes_controller_test.rb:260 "show returns 404 for missing asset files"
#[test]
fn show_returns_404_for_missing_asset_files() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(&sb, "GET", "/notes/images/nonexistent.png", &none()).status,
        404
    );
}

// port: test/controllers/notes_controller_test.rb:265 "show returns 404 for the internal filesystem lock file"
#[test]
fn show_returns_404_for_the_internal_filesystem_lock_file() {
    let sb = Sandbox::empty();
    sb.write(frankmd_fs::lock::FILESYSTEM_LOCK, "internal lock");
    let r = dispatch(
        &sb,
        "GET",
        &format!("/notes/{}", frankmd_fs::lock::FILESYSTEM_LOCK),
        &none(),
    );
    assert_eq!(r.status, 404);
    assert!(!r.body.to_string().contains("internal lock"));
}

// port: test/controllers/notes_controller_test.rb:274 "show blocks path traversal for assets"
#[test]
fn show_blocks_path_traversal_for_assets() {
    let sb = Sandbox::empty();
    // note_url(path: "../../etc/passwd") escapes each segment's dots literally.
    assert_eq!(
        dispatch(&sb, "GET", "/notes/..%2F..%2Fetc%2Fpasswd", &none()).status,
        403
    );
}

// port: test/controllers/notes_controller_test.rb:281 "create makes new note"
#[test]
fn create_makes_new_note() {
    let sb = Sandbox::empty();
    let r = dispatch(
        &sb,
        "POST",
        "/notes/new_note",
        &json!({"content": "# New Note"}),
    );
    assert_eq!(r.status, 201);
    assert_eq!(sb.read("new_note.md"), "# New Note");
}

// port: test/controllers/notes_controller_test.rb:289 "create adds .md extension if missing"
#[test]
fn create_adds_md_extension_if_missing() {
    let sb = Sandbox::empty();
    let r = dispatch(
        &sb,
        "POST",
        "/notes/no_extension",
        &json!({"content": "Content"}),
    );
    assert_eq!(r.status, 201);
    assert!(sb.exists("no_extension.md"));
}

// port: test/controllers/notes_controller_test.rb:296 "create refuses the root config file"
#[test]
fn create_refuses_the_root_config_file() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(&sb, "POST", "/notes/.fed", &json!({"content": "secret"})).status,
        403
    );
}

// port: test/controllers/notes_controller_test.rb:302 "create in subfolder works"
#[test]
fn create_in_subfolder_works() {
    let sb = Sandbox::empty();
    sb.mkdir("subfolder");
    let r = dispatch(
        &sb,
        "POST",
        "/notes/subfolder/nested.md",
        &json!({"content": "Nested"}),
    );
    assert_eq!(r.status, 201);
    assert!(sb.exists("subfolder/nested.md"));
}

// port: test/controllers/notes_controller_test.rb:311 "create with nested directories creates parent folders"
#[test]
fn create_with_nested_directories_creates_parent_folders() {
    let sb = Sandbox::empty();
    let hugo_path = "2026/01/30/my-blog-post/index.md";
    let hugo_content = "---\ntitle: \"My Blog Post\"\nslug: \"my-blog-post\"\ndate: 2026-01-30T14:30:00-0300\ndraft: true\ntags:\n-\n---\n\n";
    let r = dispatch(
        &sb,
        "POST",
        &format!("/notes/{hugo_path}"),
        &json!({"content": hugo_content}),
    );
    assert_eq!(r.status, 201);
    assert_eq!(sb.read(hugo_path), hugo_content);
}

// port: test/controllers/notes_controller_test.rb:334 "create returns error if note exists"
#[test]
fn create_returns_error_if_note_exists() {
    let sb = Sandbox::empty();
    sb.write("existing.md", DEFAULT_NOTE);
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/notes/existing.md",
            &json!({"content": "Content"})
        )
        .status,
        422
    );
}

// port: test/controllers/notes_controller_test.rb:341 "create preserves a note created after the availability check"
#[test]
fn create_preserves_a_note_created_after_the_availability_check() {
    // Rails stubs `Note#exists?` → false so the exclusive create itself must
    // refuse; here the controller's pre-check is skipped by calling the model
    // operation the controller performs next.
    let sb = Sandbox::empty();
    sb.write("raced.md", "Original content");
    let vault = sb.vault();
    let svc = frankmd_fs::NotesService::new(&vault);
    let res = frankmd_fs::model::note_save(
        &svc,
        "raced.md",
        Some(b"replacement"),
        frankmd_fs::model::SaveMode::CreateOnly,
    )
    .unwrap();
    let errors = res.expect_err("create_only must refuse an existing note (→ 422)");
    assert_eq!(
        errors
            .iter()
            .map(|e| e.full_message(&|k| common::en(k)))
            .collect::<Vec<_>>(),
        vec![common::en("errors.note_already_exists")]
    );
    assert_eq!(sb.read("raced.md"), "Original content");
}

// port: test/controllers/notes_controller_test.rb:353 "create with hugo template generates date-based path"
#[test]
fn create_with_hugo_template_generates_date_based_path() {
    // Rails travels to 2026-02-01 10:30; the emulation pins 2026-01-30 12:34:56 UTC.
    let sb = Sandbox::empty();
    let r = dispatch(
        &sb,
        "POST",
        "/notes",
        &json!({"template": "hugo", "title": "My Blog Post"}),
    );
    assert_eq!(r.status, 201);
    assert_eq!(r.body["path"], "2026/01/30/my-blog-post/index.md");
}

// port: test/controllers/notes_controller_test.rb:363 "create with hugo template generates frontmatter content"
#[test]
fn create_with_hugo_template_generates_frontmatter_content() {
    let sb = Sandbox::empty();
    let r = dispatch(
        &sb,
        "POST",
        "/notes",
        &json!({"template": "hugo", "title": "My Blog Post"}),
    );
    assert_eq!(r.status, 201);
    let content = sb.read(r.body["path"].as_str().unwrap());
    assert!(content.starts_with("---"));
    assert!(content.contains("title: \"My Blog Post\""));
    assert!(content.contains("slug: \"my-blog-post\""));
    assert!(content.contains("draft: true"));
}

// port: test/controllers/notes_controller_test.rb:378 "create with hugo template respects parent folder"
#[test]
fn create_with_hugo_template_respects_parent_folder() {
    let sb = Sandbox::empty();
    let r = dispatch(
        &sb,
        "POST",
        "/notes",
        &json!({"template": "hugo", "title": "My Post", "parent": "blog"}),
    );
    assert_eq!(r.status, 201);
    assert!(
        r.body["path"]
            .as_str()
            .unwrap()
            .starts_with("blog/2026/01/30/")
    );
}

// port: test/controllers/notes_controller_test.rb:388 "create with hugo template requires title"
#[test]
fn create_with_hugo_template_requires_title() {
    let sb = Sandbox::empty();
    let r = dispatch(
        &sb,
        "POST",
        "/notes",
        &json!({"template": "hugo", "title": ""}),
    );
    assert_eq!(r.status, 422);
    assert!(r.body["error"].as_str().unwrap().contains("required"));
}

// port: test/controllers/notes_controller_test.rb:396 "create with hugo template handles accented characters in title"
#[test]
fn create_with_hugo_template_handles_accented_characters_in_title() {
    let sb = Sandbox::empty();
    let r = dispatch(
        &sb,
        "POST",
        "/notes",
        &json!({"template": "hugo", "title": "Café Açaí"}),
    );
    assert_eq!(r.status, 201);
    assert!(r.body["path"].as_str().unwrap().contains("cafe-acai"));
}

// port: test/controllers/notes_controller_test.rb:406 "update saves note content"
#[test]
fn update_saves_note_content() {
    let sb = Sandbox::empty();
    sb.write("test.md", "Old content");
    let r = dispatch(
        &sb,
        "PATCH",
        "/notes/test.md",
        &json!({"content": "New content"}),
    );
    assert_eq!(r.status, 200);
    assert_eq!(sb.read("test.md"), "New content");
    assert_eq!(r.body["revision"], sha256_hex(b"New content"));
}

// port: test/controllers/notes_controller_test.rb:416 "update returns 404 without creating a missing note"
#[test]
fn update_returns_404_without_creating_a_missing_note() {
    let sb = Sandbox::empty();
    let r = dispatch(
        &sb,
        "PATCH",
        "/notes/missing.md",
        &json!({"content": "stale autosave"}),
    );
    assert_eq!(r.status, 404);
    assert_eq!(r.body["error"], "Note not found");
    assert!(!sb.exists("missing.md"));
}

// port: test/controllers/notes_controller_test.rb:424 "stale update cannot recreate a note after rename"
#[test]
fn stale_update_cannot_recreate_a_note_after_rename() {
    let sb = Sandbox::empty();
    sb.write("foo.md", "Original content");
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/notes/foo.md/rename",
            &json!({"new_path": "bar.md"})
        )
        .status,
        200
    );
    assert_eq!(
        dispatch(
            &sb,
            "PATCH",
            "/notes/foo.md",
            &json!({"content": "stale autosave"})
        )
        .status,
        404
    );
    assert!(!sb.exists("foo.md"));
    assert_eq!(sb.read("bar.md"), "Original content");
}

// port: test/controllers/notes_controller_test.rb:437 "stale update cannot recreate a deleted note"
#[test]
fn stale_update_cannot_recreate_a_deleted_note() {
    let sb = Sandbox::empty();
    sb.write("foo.md", DEFAULT_NOTE);
    assert_eq!(
        dispatch(&sb, "DELETE", "/notes/foo.md", &none()).status,
        200
    );
    assert_eq!(
        dispatch(
            &sb,
            "PATCH",
            "/notes/foo.md",
            &json!({"content": "stale autosave"})
        )
        .status,
        404
    );
    assert!(!sb.exists("foo.md"));
}

// port: test/controllers/notes_controller_test.rb:449 "stale nested update cannot recreate a renamed parent folder"
#[test]
fn stale_nested_update_cannot_recreate_a_renamed_parent_folder() {
    let sb = Sandbox::empty();
    sb.mkdir("docs");
    sb.write("docs/foo.md", "Original content");
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/folders/docs/rename",
            &json!({"new_path": "archive"})
        )
        .status,
        200
    );
    assert_eq!(
        dispatch(
            &sb,
            "PATCH",
            "/notes/docs/foo.md",
            &json!({"content": "stale autosave"})
        )
        .status,
        404
    );
    assert!(!sb.exists("docs"));
    assert_eq!(sb.read("archive/foo.md"), "Original content");
}

// port: test/controllers/notes_controller_test.rb:463 "update refuses the root config file"
#[test]
fn update_refuses_the_root_config_file() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(&sb, "PATCH", "/notes/.fed", &json!({"content": "secret"})).status,
        403
    );
}

// port: test/controllers/notes_controller_test.rb:469 "update refuses a canonical config file alias without mutation"
#[test]
fn update_refuses_a_canonical_config_file_alias_without_mutation() {
    let sb = Sandbox::empty();
    let original = "# AI/LLM\noriginal\n";
    sb.write(".fed", original);
    assert_eq!(
        dispatch(
            &sb,
            "PATCH",
            "/notes/%2E/.fed",
            &json!({"content": "changed"})
        )
        .status,
        403
    );
    assert_eq!(sb.read(".fed"), original);
}

// port: test/controllers/notes_controller_test.rb:481 "destroy removes note"
#[test]
fn destroy_removes_note() {
    let sb = Sandbox::empty();
    sb.write("to_delete.md", DEFAULT_NOTE);
    assert_eq!(
        dispatch(&sb, "DELETE", "/notes/to_delete.md", &none()).status,
        200
    );
    assert!(!sb.exists("to_delete.md"));
}

// port: test/controllers/notes_controller_test.rb:490 "destroy deletes note when expected revision matches"
#[test]
fn destroy_deletes_note_when_expected_revision_matches() {
    let sb = Sandbox::empty();
    sb.write("to_delete.md", "current content");
    let r = dispatch(
        &sb,
        "DELETE",
        "/notes/to_delete.md",
        &json!({"expected_revision": sha256_hex(b"current content")}),
    );
    assert_eq!(r.status, 200);
    assert!(!sb.exists("to_delete.md"));
}

// port: test/controllers/notes_controller_test.rb:502 "destroy returns a conflict and preserves note when expected revision is stale"
#[test]
fn destroy_returns_a_conflict_and_preserves_note_when_expected_revision_is_stale() {
    let sb = Sandbox::empty();
    sb.write("to_delete.md", "updated content");
    let r = dispatch(
        &sb,
        "DELETE",
        "/notes/to_delete.md",
        &json!({"expected_revision": sha256_hex(b"old content")}),
    );
    assert_eq!(r.status, 409);
    assert_eq!(r.body["code"], "revision_conflict");
    assert!(
        r.body["error"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("changed since it was loaded")
    );
    assert_eq!(sb.read("to_delete.md"), "updated content");
}

// port: test/controllers/notes_controller_test.rb:515 "destroy returns 404 for missing note"
#[test]
fn destroy_returns_404_for_missing_note() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(&sb, "DELETE", "/notes/nonexistent.md", &none()).status,
        404
    );
}

// port: test/controllers/notes_controller_test.rb:520 "destroy refuses the root config file"
#[test]
fn destroy_refuses_the_root_config_file() {
    let sb = Sandbox::empty();
    assert_eq!(dispatch(&sb, "DELETE", "/notes/.fed", &none()).status, 403);
}

// port: test/controllers/notes_controller_test.rb:528 "rename moves note to new path"
#[test]
fn rename_moves_note_to_new_path() {
    let sb = Sandbox::empty();
    sb.write("old.md", "Content");
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/notes/old.md/rename",
            &json!({"new_path": "new.md"})
        )
        .status,
        200
    );
    assert!(!sb.exists("old.md"));
    assert!(sb.exists("new.md"));
}

// port: test/controllers/notes_controller_test.rb:538 "rename moves note to subfolder"
#[test]
fn rename_moves_note_to_subfolder() {
    let sb = Sandbox::empty();
    sb.write("root.md", "Content");
    sb.mkdir("subfolder");
    let r = dispatch(
        &sb,
        "POST",
        "/notes/root.md/rename",
        &json!({"new_path": "subfolder/moved.md"}),
    );
    assert_eq!(r.status, 200);
    assert!(!sb.exists("root.md"));
    assert!(sb.exists("subfolder/moved.md"));
}

// port: test/controllers/notes_controller_test.rb:549 "rename conflict preserves both source and destination contents"
#[test]
fn rename_conflict_preserves_both_source_and_destination_contents() {
    let sb = Sandbox::empty();
    sb.write("source.md", "Source content");
    sb.write("destination.md", "Destination content");
    let r = dispatch(
        &sb,
        "POST",
        "/notes/source.md/rename",
        &json!({"new_path": "destination.md"}),
    );
    assert_eq!(r.status, 422);
    assert_eq!(sb.read("source.md"), "Source content");
    assert_eq!(sb.read("destination.md"), "Destination content");
}

// port: test/controllers/notes_controller_test.rb:560 "rename returns 404 for missing note"
#[test]
fn rename_returns_404_for_missing_note() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/notes/nonexistent.md/rename",
            &json!({"new_path": "new.md"})
        )
        .status,
        404
    );
}

// port: test/controllers/notes_controller_test.rb:565 "rename refuses the root config file"
#[test]
fn rename_refuses_the_root_config_file() {
    let sb = Sandbox::empty();
    assert_eq!(
        dispatch(
            &sb,
            "POST",
            "/notes/.fed/rename",
            &json!({"new_path": "renamed.md"})
        )
        .status,
        403
    );
}

// port: test/controllers/notes_controller_test.rb:571 "rename refuses a canonical config destination alias"
#[test]
fn rename_refuses_a_canonical_config_destination_alias() {
    let sb = Sandbox::empty();
    sb.write("source.md", "original");
    let r = dispatch(
        &sb,
        "POST",
        "/notes/source.md/rename",
        &json!({"new_path": "foo/../.fed"}),
    );
    assert_eq!(r.status, 403);
    assert_eq!(sb.read("source.md"), "original");
}

// port: test/controllers/notes_controller_test.rb:626 "backlinks returns matching results"
#[test]
fn backlinks_returns_matching_results() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target");
    sb.write("source.md", "Links to [[target]] here");
    let r = dispatch(&sb, "GET", "/notes/target.md/backlinks", &none());
    assert_eq!(r.status, 200);
    assert_eq!(r.body["count"], 1);
    assert_eq!(r.body["backlinks"].as_array().unwrap().len(), 1);
    assert_eq!(r.body["backlinks"][0]["path"], "source.md");
}

// port: test/controllers/notes_controller_test.rb:639 "backlinks returns empty array when no links exist"
#[test]
fn backlinks_returns_empty_array_when_no_links_exist() {
    let sb = Sandbox::empty();
    sb.write("lonely.md", "# No one links to me");
    let r = dispatch(&sb, "GET", "/notes/lonely.md/backlinks", &none());
    assert_eq!(r.status, 200);
    assert_eq!(r.body["count"], 0);
    assert_eq!(r.body["backlinks"], json!([]));
}

// port: test/controllers/notes_controller_test.rb:745 "create responds with turbo stream when requested"
#[test]
fn create_turbo_tree_contains_the_new_note() {
    let sb = Sandbox::empty();
    let r = dispatch_turbo(
        &sb,
        "POST",
        "/notes/turbo_note.md",
        &json!({"content": "# Turbo", "expanded": "folder1"}),
    );
    assert_eq!(r.status, 201);
    assert_eq!(r.created_path.as_deref(), Some("turbo_note.md"));
    assert!(r.tree_paths().contains(&"turbo_note.md".to_owned()));
}

// port: test/controllers/notes_controller_test.rb:759 "create turbo stream includes expanded folder state"
#[test]
fn create_turbo_tree_keeps_expanded_folder_state() {
    let sb = Sandbox::empty();
    sb.mkdir("myfolder");
    sb.write("myfolder/existing.md", DEFAULT_NOTE);
    let r = dispatch_turbo(
        &sb,
        "POST",
        "/notes/new_note.md",
        &json!({"content": "", "expanded": "myfolder"}),
    );
    assert_eq!(r.status, 201);
    let TreeView::Rows(rows) = &r.tree.as_ref().unwrap().view else {
        panic!("expected rows")
    };
    assert!(
        rows.iter()
            .any(|row| row.path == "myfolder" && row.expanded)
    );
}

// port: test/controllers/notes_controller_test.rb:772 "destroy responds with turbo stream when requested"
#[test]
fn destroy_turbo_tree_omits_the_deleted_note() {
    let sb = Sandbox::empty();
    sb.write("to_delete.md", DEFAULT_NOTE);
    let r = dispatch_turbo(&sb, "DELETE", "/notes/to_delete.md", &none());
    assert_eq!(r.status, 200);
    assert!(!r.tree_paths().contains(&"to_delete.md".to_owned()));
}

// port: test/controllers/notes_controller_test.rb:786 "rename responds with turbo stream when requested"
#[test]
fn rename_turbo_tree_shows_new_path_only() {
    let sb = Sandbox::empty();
    sb.write("old_name.md", "Content");
    let r = dispatch_turbo(
        &sb,
        "POST",
        "/notes/old_name.md/rename",
        &json!({"new_path": "new_name.md", "expanded": ""}),
    );
    assert_eq!(r.status, 200);
    let paths = r.tree_paths();
    assert!(paths.contains(&"new_name.md".to_owned()));
    assert!(!paths.contains(&"old_name.md".to_owned()));
}
