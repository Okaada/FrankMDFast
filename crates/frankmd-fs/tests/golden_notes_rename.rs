//! Golden conformance: `tests/fixtures/golden/notes_rename.json`
//! (`NotesService#rename`, `Note#rename`, `Folder#rename` + Hugo slug, recents remap).
//!
//! Not here: `"http"` cases (`golden_http.rs`) and
//! `FoldersController#load_tree_for_turbo_stream` (frankmd-core `tree_view`).
//! Recents are always coupled through a `.fed`-backed store; the service must
//! only touch it when `.fed` is a regular file (`RecentStore::is_present`).
//! `.fed` is compared through its `recent_files` value only (line surgery is
//! frankmd-config's contract); `Errno::*` errors are compared by class only.
#![allow(clippy::unwrap_used, clippy::disallowed_methods)]

mod common;
mod support_d;

use common::{FedRecentStore, Sandbox};
use frankmd_fs::{NotesService, model};
use frankmd_testkit::serde_json::Value;
use frankmd_testkit::{Case, assert_cases, golden};

const FNS: &[&str] = &["rename", "rename,delete", "Note#rename", "Folder#rename"];

fn run_case(case: &Case) -> Result<(), String> {
    let input = &case.input;
    let sb = Sandbox::from_specs(&input["tree"], &input["outside"], None);
    let vault = sb.vault();
    let store = FedRecentStore::new(&sb.root);
    let svc = NotesService::new(&vault).with_recents(&store);
    let s = |v: &Value| v.as_str().unwrap_or_default().to_owned();

    let ops = input["ops"].as_array().unwrap();
    let results = case.expected["results"].as_array().unwrap();
    let mut errs = Vec::new();
    for (i, (op, exp)) in ops.iter().zip(results).enumerate() {
        let what = format!("op #{i} {}", op["op"]);
        let r = match op["op"].as_str().unwrap() {
            "rename" => support_d::check_unit(
                &what,
                svc.rename(&s(&op["old_path"]), &s(&op["new_path"])),
                exp,
                &sb,
            ),
            "delete" => support_d::check_unit(&what, svc.delete(&s(&op["path"]), None), exp, &sb),
            "Note#rename" => {
                let path = s(&op["path"]);
                compare_model(
                    &what,
                    model::note_rename(&svc, &path, &s(&op["new_path"])),
                    &path,
                    exp,
                    &sb,
                )
            }
            "Folder#rename" => {
                let path = s(&op["path"]);
                compare_model(
                    &what,
                    model::folder_rename(&svc, &path, &s(&op["new_path"])),
                    &path,
                    exp,
                    &sb,
                )
            }
            other => Err(format!("unknown op {other}")),
        };
        if let Err(m) = r {
            errs.push(m);
        }
    }
    if let Err(m) = support_d::check_tree_after(&sb, &case.expected["tree_after"]) {
        errs.push(m);
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("\n  "))
    }
}

fn compare_model(
    what: &str,
    r: model::ModelResult<String>,
    orig: &str,
    exp: &Value,
    sb: &Sandbox,
) -> Result<(), String> {
    match support_d::model_json(r, orig) {
        Ok(v) => common::check_eq(what, &v, exp),
        Err(e) if exp.get("error").is_some() => support_d::check_error(&e, exp, sb),
        Err(e) => Err(format!(
            "{what}: unrescued error {e} ({}), expected {exp}",
            e.compat_class()
        )),
    }
}

#[test]
fn golden_notes_rename() {
    let g = golden("notes_rename");
    let cases: Vec<&Case> = g
        .cases
        .iter()
        .filter(|c| c.func.as_deref().is_some_and(|f| FNS.contains(&f)))
        .collect();
    assert_cases("notes_rename", cases, run_case);
}
