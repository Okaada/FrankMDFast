//! Golden conformance: `notes_rename.json` cases
//! `FoldersController#load_tree_for_turbo_stream` (expanded-folder remap after
//! a folder rename; the renamed folder becomes the selection).
#![allow(missing_docs, clippy::unwrap_used)]

use frankmd_core::tree_view::{parse_expanded, remap_expanded};
use frankmd_testkit::serde_json::json;
use frankmd_testkit::{assert_cases, golden};

#[test]
fn golden_expanded_folder_remap() {
    let g = golden("notes_rename");
    assert_cases(
        "notes_rename",
        g.cases_for("FoldersController#load_tree_for_turbo_stream"),
        |c| {
            let from = c.input["remap_from"].as_str().unwrap();
            let to = c.input["remap_to"].as_str().unwrap();
            let expanded = parse_expanded(c.input["expanded"].as_str().unwrap());
            let remapped = remap_expanded(&expanded, from, to);
            // The controller selects the renamed folder's new path.
            let got = json!({"expanded_folders": remapped, "selected_file": to});
            if got == c.expected {
                Ok(())
            } else {
                Err(format!("expected {}, got {got}", c.expected))
            }
        },
    );
}

#[test]
fn parse_expanded_empty_param_is_empty() {
    assert!(parse_expanded("").is_empty());
}
