//! Golden conformance: `tests/fixtures/golden/path_safety.json`, filesystem
//! half (`PathSafety.contain` on a real tree, `NotesService#safe_path`,
//! `NotesService#exists?/file?/directory?`).
//!
//! Not here: `http` cases (`golden_http.rs`) and the pure lexical functions
//! (`PathSafety.contain` with `base_exists: false`, `PathSafety.within?`,
//! `Note.normalize_path`, Note/Folder attrs), which are frankmd-core goldens.
//!
//! Quirk: Ruby raises `ArgumentError("pathname contains null byte")` for a NUL
//! in the path. The Rust API has no "raise": `contain` must return `None`, and
//! `safe_path`/predicates must fail with an `InvalidPath` error.
#![allow(clippy::disallowed_methods, clippy::unwrap_used, missing_docs, unreachable_pub)]

mod common;

use common::{Sandbox, check_eq, fs_error_json, spec_orders};
use frankmd_fs::NotesService;
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};
use std::path::Path;

const FIXTURE: &str = "path_safety";

fn sandbox(case: &Case, orders: &std::collections::HashMap<String, common::SpecOrder>) -> Sandbox {
    let i = &case.input;
    Sandbox::from_specs(&i["tree"], &i["outside"], orders.get(&case.name))
}

/// Ruby path argument → Rust: nil → None, Symbol/Integer → their `to_s`.
fn path_arg(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Object(o) if o.contains_key("symbol") => o["symbol"].as_str().map(str::to_owned),
        other => Some(other.to_string()),
    }
}

fn subst(sb: &Sandbox, s: &str) -> String {
    s.replace("<ROOT>", &sb.root.display().to_string())
        .replace("<OUTSIDE>", &sb.outside.display().to_string())
}

fn is_null_byte_error(expected: &Value) -> bool {
    expected.get("error").and_then(Value::as_str) == Some("ArgumentError")
}

/// Compare a `Result<Value, FsError>` against the fixture, mapping the NUL quirk.
fn check_result(sb: &Sandbox, actual: Result<Value, frankmd_fs::FsError>, expected: &Value) -> Result<(), String> {
    match actual {
        Ok(v) => check_eq("result", &sb.norm_value(&v), expected),
        Err(e) if is_null_byte_error(expected) => {
            if e.is_invalid_path() {
                Ok(())
            } else {
                Err(format!("NUL path: expected an InvalidPath error, got {}", fs_error_json(&e)))
            }
        }
        Err(e) => check_eq("error", &sb.norm_value(&fs_error_json(&e)), expected),
    }
}

#[test]
fn golden_contain_on_real_tree() {
    let g = golden(FIXTURE);
    let orders = spec_orders(FIXTURE);
    let cases = g.cases_for("PathSafety.contain").filter(|c| c.input["base_exists"] == json!(true));
    assert_cases("path_safety PathSafety.contain (fs)", cases, |case| {
        let sb = sandbox(case, &orders);
        let base = subst(&sb, case.input["base"].as_str().unwrap());
        let path = path_arg(&case.input["path"]).map(|p| subst(&sb, &p));
        let actual = frankmd_fs::path_safety::contain(Path::new(&base), path.as_deref());
        let actual = actual.map_or(Value::Null, |p| Value::String(sb.norm(&p.display().to_string())));
        let expected = if is_null_byte_error(&case.expected) { Value::Null } else { case.expected.clone() };
        // Rejections must never create anything (escape/new.md cases).
        check_eq("contain", &actual, &expected)?;
        check_eq(
            "outside untouched",
            &json!(std::fs::read_dir(&sb.outside).unwrap().count()),
            &json!(case.input["outside"].as_object().map_or(0, |o| {
                o.keys().map(|k| k.split('/').next().unwrap().to_owned()).collect::<std::collections::BTreeSet<_>>().len()
            })),
        )
    });
}

#[test]
fn golden_safe_path() {
    let g = golden(FIXTURE);
    let orders = spec_orders(FIXTURE);
    assert_cases("path_safety NotesService#safe_path", g.cases_for("NotesService#safe_path"), |case| {
        let sb = sandbox(case, &orders);
        let vault = sb.vault();
        let path = path_arg(&case.input["path"]).unwrap_or_default();
        let must_exist = case.input["must_exist"].as_bool().unwrap_or(false);
        let actual = vault
            .safe_path(&subst(&sb, &path), must_exist)
            .map(|c| Value::String(c.abs().display().to_string()));
        check_result(&sb, actual, &case.expected)
    });
}

#[test]
fn golden_existence_predicates() {
    let g = golden(FIXTURE);
    let orders = spec_orders(FIXTURE);
    for func in ["NotesService#exists?", "NotesService#file?", "NotesService#directory?"] {
        assert_cases(&format!("path_safety {func}"), g.cases_for(func), |case| {
            let sb = sandbox(case, &orders);
            let vault = sb.vault();
            let svc = NotesService::new(&vault);
            let path = subst(&sb, &path_arg(&case.input["path"]).unwrap_or_default());
            let actual = match func {
                "NotesService#exists?" => svc.exists(&path),
                "NotesService#file?" => svc.is_file(&path),
                _ => svc.is_dir(&path),
            }
            .map(Value::Bool);
            check_result(&sb, actual, &case.expected)
        });
    }
}
