//! Golden conformance: `tests/fixtures/golden/notes_crud.json` (service and
//! model ops). Cases containing an `"http"` op are driven by `golden_http.rs`.
//!
//! Each case runs `input.ops` sequentially in a fresh sandbox, like
//! `NotesOps.run` in `tools/golden/ruby/notes_fs_common.rb`, then compares
//! every op result and the full `tree_after` snapshot.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[allow(missing_docs)]
mod common;

use common::{Sandbox, check_eq, en, expected_tree, fs_error_json, sha256_hex, spec_orders};
use frankmd_core::note::note_name;
use frankmd_core::path::normalize_note_path;
use frankmd_fs::model::{self, ModelError, SaveMode};
use frankmd_fs::{FsError, NotesService};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, golden};

const FIXTURE: &str = "notes_crud";

fn has_http(case: &frankmd_testkit::Case) -> bool {
    case.input["ops"]
        .as_array()
        .is_some_and(|ops| ops.iter().any(|o| o["op"] == "http"))
}

fn s<'a>(op: &'a Value, key: &str) -> &'a str {
    op[key].as_str().unwrap_or("")
}

fn expected_revision(op: &Value) -> Option<String> {
    if let Some(src) = op.get("expected_revision_of").and_then(Value::as_str) {
        return Some(sha256_hex(src.as_bytes()));
    }
    op.get("expected_revision")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn content_bytes(op: &Value) -> Vec<u8> {
    op["content"].as_str().unwrap_or("").as_bytes().to_vec()
}

fn ok_or_err<T>(r: Result<T, FsError>) -> Value {
    match r {
        Ok(_) => Value::Bool(true),
        Err(e) => fs_error_json(&e),
    }
}

fn bool_or_err(r: Result<bool, FsError>) -> Value {
    match r {
        Ok(b) => Value::Bool(b),
        Err(e) => fs_error_json(&e),
    }
}

fn messages(errors: &[ModelError]) -> Value {
    let t = |k: &str| en(k);
    Value::Array(
        errors
            .iter()
            .map(|e| Value::String(e.full_message(&t)))
            .collect(),
    )
}

/// `NotesFS.model_result(ok, model)`. An unrescued exception was captured by
/// the generator as the value of `ok` (`{"ok": {error, message}, "errors": [], …}`).
/// `path_after` is the model's `path` attribute afterwards (`null` for a nil path).
fn model_result<T>(
    r: model::ModelResult<T>,
    path_after: impl FnOnce(Option<&T>) -> Value,
) -> Value {
    match r {
        Ok(Ok(v)) => json!({"ok": true, "errors": [], "path": path_after(Some(&v))}),
        Ok(Err(errs)) => json!({"ok": false, "errors": messages(&errs), "path": path_after(None)}),
        Err(e) => json!({"ok": fs_error_json(&e), "errors": [], "path": path_after(None)}),
    }
}

fn note_find(svc: &NotesService<'_>, raw: &str) -> Value {
    let path = normalize_note_path(raw);
    match model::note_exists(svc, &path) {
        Ok(true) => {}
        Ok(false) => {
            return json!({"error": "NotesService::NotFoundError", "message": format!("Note not found: {raw}")});
        }
        Err(e) => return fs_error_json(&e),
    }
    match svc.read(&path) {
        Ok(note) => {
            let content = String::from_utf8_lossy(&note.bytes).into_owned();
            let name = note_name(&path);
            json!({
                "path": path, "name": name, "content": content,
                "revision": note.revision.0,
                "as_json": {"path": path, "name": name, "content": content},
            })
        }
        Err(e) => fs_error_json(&e),
    }
}

fn stat_mode(sb: &Sandbox, rel: &str) -> Value {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::symlink_metadata(sb.abs(rel)) {
        Ok(m) => Value::String(format!("{:o}", m.permissions().mode() & 0o7777)),
        Err(e) => json!({"error": "Errno::ENOENT", "message": e.to_string()}),
    }
}

fn run_op(sb: &Sandbox, svc: &NotesService<'_>, op: &Value) -> Value {
    let path = s(op, "path");
    match s(op, "op") {
        "read" => match svc.read(path) {
            Ok(n) => Value::String(String::from_utf8_lossy(&n.bytes).into_owned()),
            Err(e) => fs_error_json(&e),
        },
        "write" => ok_or_err(svc.write(path, &content_bytes(op))),
        "create" => ok_or_err(svc.create(path, &content_bytes(op))),
        "update" => ok_or_err(svc.update(path, &content_bytes(op))),
        "delete" => ok_or_err(svc.delete(path, expected_revision(op).as_deref())),
        "rename" => ok_or_err(svc.rename(s(op, "old_path"), s(op, "new_path"))),
        "create_folder" => ok_or_err(svc.create_folder(path)),
        "delete_folder" => ok_or_err(svc.delete_folder(path)),
        "exists?" => bool_or_err(svc.exists(path)),
        "file?" => bool_or_err(svc.is_file(path)),
        "directory?" => bool_or_err(svc.is_dir(path)),
        "list_tree" => match svc.list_tree() {
            Ok(t) => common::mask_mtimes(&common::tree_json(&t)),
            Err(e) => fs_error_json(&e),
        },
        "stat_mode" => stat_mode(sb, path),
        "chmod" => {
            sb.chmod(path, op["mode"].as_u64().unwrap() as u32);
            Value::Bool(true)
        }
        "Note.find" => note_find(svc, path),
        "Note#save" => {
            let mode = match s(op, "mode") {
                "existing_only" => SaveMode::ExistingOnly,
                "create_only" => SaveMode::CreateOnly,
                _ => SaveMode::Upsert,
            };
            let content = op["content"].as_str().map(str::as_bytes);
            // The model's `path` attribute is not changed by save.
            model_result(model::note_save(svc, path, content, mode), |_| {
                op["path"].clone()
            })
        }
        "Note#destroy" => {
            let r = model::note_destroy(svc, path, expected_revision(op).as_deref());
            let conflict = matches!(&r, Ok(Err(errs)) if errs.iter().any(|e| e.revision_conflict));
            let mut v = model_result(r, |_| op["path"].clone());
            if let Some(o) = v.as_object_mut().filter(|o| o.contains_key("ok")) {
                o.insert("revision_conflict".into(), Value::Bool(conflict));
            }
            v
        }
        "Note#rename" => model_result(model::note_rename(svc, path, s(op, "new_path")), |new| {
            Value::String(new.cloned().unwrap_or_else(|| path.to_owned()))
        }),
        "Note#exists?" => match model::note_exists(svc, path) {
            Ok(b) => json!({"exists": b, "persisted": b}),
            Err(e) => fs_error_json(&e),
        },
        "Folder#create" => model_result(model::folder_create(svc, path), |_| op["path"].clone()),
        "Folder#destroy" => model_result(model::folder_destroy(svc, path), |_| op["path"].clone()),
        "Folder#rename" => {
            model_result(model::folder_rename(svc, path, s(op, "new_path")), |new| {
                Value::String(new.cloned().unwrap_or_else(|| path.to_owned()))
            })
        }
        other => panic!("unknown op {other}"),
    }
}

/// Ruby-runtime error messages (`Errno::*`, `ArgumentError`) are not
/// reproducible verbatim: compare the class only, and for `ArgumentError`
/// (NUL byte in a path) accept any error.
fn compare_result(i: usize, actual: &Value, expected: &Value) -> Result<(), String> {
    // An unrescued model exception sits under `ok`: compare it like a top-level error.
    if let (Some(exp_ok), Some(act_ok)) = (
        expected.get("ok").filter(|v| v.is_object()),
        actual.get("ok"),
    ) {
        compare_result(i, act_ok, exp_ok)?;
        return check_eq(
            &format!("op {i} errors/path"),
            &json!([actual["errors"], actual["path"]]),
            &json!([expected["errors"], expected["path"]]),
        );
    }
    if let Some(class) = expected.get("error").and_then(Value::as_str) {
        if class == "ArgumentError" {
            return if actual.get("error").is_some() {
                Ok(())
            } else {
                Err(format!(
                    "op {i}: expected an error ({expected}), got {actual}"
                ))
            };
        }
        if class.starts_with("Errno::") {
            return check_eq(
                &format!("op {i} error class"),
                &actual["error"],
                &expected["error"],
            );
        }
    }
    check_eq(&format!("op {i}"), actual, expected)
}

#[test]
fn golden_notes_crud_service_and_model_ops() {
    let g = golden(FIXTURE);
    let orders = spec_orders(FIXTURE);
    assert_cases(FIXTURE, g.cases.iter().filter(|c| !has_http(c)), |case| {
        let sb = Sandbox::from_specs(
            &case.input["tree"],
            &case.input["outside"],
            orders.get(&case.name),
        );
        let vault = sb.vault();
        // Rails NotesService always updates the `.fed` history (when `.fed` is
        // a regular file), so couple the recents store like production does.
        let store = common::FedRecentStore::new(&sb.root);
        let svc = NotesService::new(&vault).with_recents(&store);
        let ops = case.input["ops"].as_array().unwrap();
        let expected = case.expected["results"].as_array().unwrap();
        let mut errs = Vec::new();
        for (i, (op, exp)) in ops.iter().zip(expected).enumerate() {
            let actual = sb.norm_value(&run_op(&sb, &svc, op));
            if let Err(e) = compare_result(i, &actual, exp) {
                errs.push(e);
            }
        }
        let snap = sb.snapshot();
        let want = expected_tree(&case.expected["tree_after"]);
        if snap != want {
            errs.push(format!(
                "tree_after:\n    expected: {want:?}\n    actual:   {snap:?}"
            ));
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("\n  "))
        }
    });
}
