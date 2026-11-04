//! Golden conformance: `tests/fixtures/golden/recent_files.json`
//! (`RecentFilesService` + its NotesService side effects).
//!
//! Each op runs at `PINNED_NOW + at_offset` s. `travel_to` truncates the
//! microseconds, so the effective base is `2026-01-30T12:34:56Z` (hence the
//! `.000000Z` in every recorded `opened_at`). `entries_after` was taken at
//! now+3600 s; `entries()` takes no clock because normalization is
//! time-independent. Cases that contain `"http"` ops are in `golden_http.rs`.
//! Persistence uses a `.fed`-backed fake store with case-insensitive key
//! matching like Rails `Config`; `.fed` is compared through its
//! `recent_files` lines/value only.
//!
//! **[QUIRK]** `record(42)` / `record(["safe.md"])` (non-string paths) are
//! passed as `None`: the Rust API only takes strings, and Rails rejected them
//! with the same NotFoundError.
#![allow(clippy::unwrap_used, clippy::disallowed_methods)]

mod common;
mod support_d;

use common::Sandbox;
use frankmd_fs::{NotesService, RecentFilesService};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};
use support_d::CiFedStore;

fn base_now() -> jiff::Timestamp {
    jiff::Timestamp::from_second(common::pinned_now().as_second()).unwrap()
}

fn has_http(case: &Case) -> bool {
    case.input["ops"]
        .as_array()
        .unwrap()
        .iter()
        .any(|o| o["op"] == "http")
}

fn run_case(case: &Case) -> Result<(), String> {
    let input = &case.input;
    let sb = Sandbox::from_specs(&input["tree"], &Value::Null, None);
    let vault = sb.vault();
    let store = CiFedStore::new(&sb.root);
    let s = |v: &Value| v.as_str().unwrap_or_default().to_owned();

    let ops = input["ops"].as_array().unwrap();
    let results = case.expected["results"].as_array().unwrap();
    let mut errs = Vec::new();
    for (i, (op, exp)) in ops.iter().zip(results).enumerate() {
        let what = format!("op #{i} {}", op["op"]);
        let at =
            base_now() + jiff::SignedDuration::from_secs(op["at_offset"].as_i64().unwrap_or(0));
        let history = RecentFilesService::new(&vault, &store);
        let notes = NotesService::new(&vault).with_recents(&store);
        let r = match op["op"].as_str().unwrap() {
            "record" => {
                support_d::check_bool(&what, history.record(op["path"].as_str(), at), exp, &sb)
            }
            "entries" => common::check_eq(&what, &support_d::entries_json(&history.entries()), exp),
            "clear" => support_d::check_bool(&what, history.clear(), exp, &sb),
            "remove" => support_d::check_bool(&what, history.remove(&s(&op["path"])), exp, &sb),
            "remap" => support_d::check_bool(
                &what,
                history.remap(&s(&op["old_path"]), &s(&op["new_path"])),
                exp,
                &sb,
            ),
            "fs_delete" => {
                std::fs::remove_file(sb.abs(&s(&op["path"]))).unwrap();
                Ok(())
            }
            "fs_write" => {
                sb.write(&s(&op["path"]), &s(&op["content"]));
                Ok(())
            }
            "fs_chmod" => {
                sb.chmod(&s(&op["path"]), op["mode"].as_u64().unwrap() as u32);
                Ok(())
            }
            "service_rename" => support_d::check_unit(
                &what,
                notes.rename(&s(&op["old_path"]), &s(&op["new_path"])),
                exp,
                &sb,
            ),
            "service_delete" => {
                support_d::check_unit(&what, notes.delete(&s(&op["path"]), None), exp, &sb)
            }
            "service_delete_folder" => {
                support_d::check_unit(&what, notes.delete_folder(&s(&op["path"])), exp, &sb)
            }
            other => Err(format!("unknown op {other}")),
        };
        if let Err(m) = r {
            errs.push(m);
        }
    }
    support_d::unlock_dirs(&sb.root);

    // The generator read `entries_after` through a fresh Rails `Config.new`,
    // which creates `.fed` from the template (and its lock file) when missing.
    // That is a frankmd-config side effect, reproduced here so `tree_after`
    // compares only what this crate is responsible for.
    if !sb.exists(".fed") {
        sb.write(".fed", &common::fed_template());
        sb.write(".frankmd-config.lock", "");
    }

    let history = RecentFilesService::new(&vault, &store);
    if let Err(m) = common::check_eq(
        "entries_after",
        &support_d::entries_json(&history.entries()),
        &case.expected["entries_after"],
    ) {
        errs.push(m);
    }
    let lines = support_d::fed_recent_lines(&sb.root).map_or(Value::Null, |l| json!(l));
    if let Err(m) = common::check_eq(
        "fed_recent_files_lines",
        &lines,
        &case.expected["fed_recent_files_lines"],
    ) {
        errs.push(m);
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

#[test]
fn golden_recent_files() {
    let g = golden("recent_files");
    let cases: Vec<&Case> = g
        .cases_for("RecentFilesService")
        .filter(|c| !has_http(c))
        .collect();
    assert_cases("recent_files", cases, run_case);
}
