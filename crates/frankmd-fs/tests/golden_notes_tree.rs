//! Golden conformance: `tests/fixtures/golden/notes_tree.json`
//! (`NotesService#list_tree` incl. `tree_after`, `Folder#children`,
//! `Note.all`, `Folder.find`). `http` cases live in `golden_http.rs`.
//!
//! Normalizations (fixture notes): sibling runs with an equal sort key (equal
//! integer mtime for files, equal lowercased name for folders) are not ordered
//! deterministically by the original, so each such run is sorted by path on
//! both sides before comparing; everything else must match in order.
#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    missing_docs,
    unreachable_pub
)]

mod common;

use common::{Sandbox, SpecOrder, check_eq, expected_tree, fs_error_json, spec_orders, tree_json};
use frankmd_core::note::{folder_name, folder_parent_path};
use frankmd_fs::{FsError, NotesService, model};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

const FIXTURE: &str = "notes_tree";

/// `common::Sandbox` pins only non-negative integer mtimes; fractional and
/// negative ones (the truncation case) are stripped from the spec and applied
/// here afterwards.
fn build(case: &Case, orders: &HashMap<String, SpecOrder>) -> Sandbox {
    let mut tree = case.input["tree"].clone();
    let mut late: Vec<(String, f64)> = Vec::new();
    if let Some(map) = tree.as_object_mut() {
        for (k, v) in map.iter_mut() {
            if let Some(o) = v.as_object_mut()
                && let Some(m) = o.get("mtime").cloned()
                && m.as_i64().filter(|i| *i >= 0).is_none()
            {
                late.push((k.clone(), m.as_f64().unwrap()));
                o.remove("mtime");
            }
        }
    }
    let sb = Sandbox::from_specs(&tree, &case.input["outside"], orders.get(&case.name));
    for (rel, secs) in late {
        set_mtime_f64(&sb.abs(&rel), secs);
    }
    sb
}

fn set_mtime_f64(path: &Path, secs: f64) {
    use std::time::{Duration, UNIX_EPOCH};
    let t = if secs >= 0.0 {
        UNIX_EPOCH + Duration::from_secs_f64(secs)
    } else {
        UNIX_EPOCH - Duration::from_secs_f64(-secs)
    };
    let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_times(std::fs::FileTimes::new().set_accessed(t).set_modified(t))
        .unwrap();
}

fn sort_key(node: &Value) -> (u8, String) {
    if node["type"] == "folder" {
        (0, node["name"].as_str().unwrap_or("").to_lowercase())
    } else {
        (1, node["mtime"].to_string())
    }
}

/// Sort each run of consecutive siblings sharing a sort key by path (recursively).
fn canon_ties(v: &Value) -> Value {
    let Some(items) = v.as_array() else {
        return v.clone();
    };
    let mut items: Vec<Value> = items
        .iter()
        .map(|n| {
            let mut n = n.clone();
            if let Some(ch) = n.get("children").cloned() {
                n["children"] = canon_ties(&ch);
            }
            n
        })
        .collect();
    let mut start = 0;
    while start < items.len() {
        let key = sort_key(&items[start]);
        let mut end = start + 1;
        while end < items.len() && sort_key(&items[end]) == key {
            end += 1;
        }
        items[start..end].sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        start = end;
    }
    Value::Array(items)
}

/// Expected `tree_after`, with `{base64}` file entries decoded to the lossy
/// UTF-8 text `Sandbox::snapshot` produces.
fn expected_after(v: &Value) -> BTreeMap<String, Value> {
    expected_tree(v)
        .into_iter()
        .map(|(k, v)| match v.get("base64").and_then(Value::as_str) {
            Some(b) => (
                k,
                Value::String(String::from_utf8_lossy(&common::base64_decode(b)).into_owned()),
            ),
            None => (k, v),
        })
        .collect()
}

#[test]
fn golden_list_tree() {
    let g = golden(FIXTURE);
    let orders = spec_orders(FIXTURE);
    assert_cases(
        "notes_tree list_tree",
        g.cases_for("NotesService#list_tree"),
        |case| {
            let sb = build(case, &orders);
            let vault = sb.vault();
            let tree = NotesService::new(&vault)
                .list_tree()
                .map_err(|e| format!("list_tree failed: {e}"))?;
            check_eq(
                "tree",
                &canon_ties(&tree_json(&tree)),
                &canon_ties(&case.expected["tree"]),
            )?;
            let after = sb.snapshot();
            let want = expected_after(&case.expected["tree_after"]);
            if after != want {
                return Err(format!(
                    "tree_after:\n    expected: {want:?}\n    actual:   {after:?}"
                ));
            }
            Ok(())
        },
    );
}

#[test]
fn golden_folder_children() {
    let g = golden(FIXTURE);
    let orders = spec_orders(FIXTURE);
    assert_cases(
        "notes_tree Folder#children",
        g.cases_for("Folder#children"),
        |case| {
            let sb = build(case, &orders);
            let vault = sb.vault();
            let path = case.input["path"].as_str().unwrap_or("");
            let children = NotesService::new(&vault)
                .folder_children(path)
                .map_err(|e| format!("folder_children failed: {e}"))?;
            check_eq(
                "children",
                &canon_ties(&tree_json(&children)),
                &canon_ties(&case.expected),
            )
        },
    );
}

#[test]
fn golden_note_all() {
    let g = golden(FIXTURE);
    let orders = spec_orders(FIXTURE);
    assert_cases("notes_tree Note.all", g.cases_for("Note.all"), |case| {
        let sb = build(case, &orders);
        let vault = sb.vault();
        let tree = NotesService::new(&vault)
            .list_tree()
            .map_err(|e| format!("list_tree failed: {e}"))?;
        check_eq(
            "Note.all",
            &canon_ties(&tree_json(&tree)),
            &canon_ties(&case.expected),
        )
    });
}

/// `Folder.find(path)`: `safe_path` (invalid paths raise), then
/// `NotFound("Folder not found: <path>")` unless it is a directory.
fn folder_find(svc: &NotesService<'_>, path: &str) -> Result<Value, FsError> {
    if !model::folder_exists(svc, path)? {
        return Err(FsError::NotFound(format!("Folder not found: {path}")));
    }
    Ok(json!({
        "path": path,
        "name": folder_name(path),
        "parent_path": folder_parent_path(path),
        "persisted": model::folder_exists(svc, path)?,
    }))
}

#[test]
fn golden_folder_find() {
    let g = golden(FIXTURE);
    let orders = spec_orders(FIXTURE);
    assert_cases(
        "notes_tree Folder.find",
        g.cases_for("Folder.find"),
        |case| {
            let sb = build(case, &orders);
            let vault = sb.vault();
            let svc = NotesService::new(&vault);
            let path = case.input["path"].as_str().unwrap_or("");
            let actual = folder_find(&svc, path).unwrap_or_else(|e| fs_error_json(&e));
            check_eq("Folder.find", &actual, &case.expected)
        },
    );
}
