//! Golden conformance: tests/fixtures/golden/js_draft_storage.json (44 scenarios)
//! driven through `frankmd_workspace::draft_storage::DraftStorage`.
//!
//! Each scenario seeds an insertion-ordered memory backend, runs the steps
//! (method calls, fault installs, provider failure toggles, raw writes) and
//! compares every step result — re-encoded in the original's `{ok, …}` JS
//! shape — plus the final raw storage entries (exact key order and bytes).
//!
//! Arguments a Rust `&str` API cannot receive (JS `null`/numbers where a string
//! is required, or an object that is not a draft record) are not called: the
//! original rejects all of them with a `TypeError` before touching storage, so
//! the expected result must be that `TypeError` and the step is checked as such.
#![allow(clippy::unwrap_used)]

#[path = "draft_common/mod.rs"]
mod draft_common;

use draft_common::{Fault, FaultMode, Method, drafts_seeded};
use frankmd_testkit::serde_json::{self, Map, Value, json};
use frankmd_testkit::{assert_cases, golden};
use frankmd_workspace::draft_storage::{
    DraftConflict, DraftError, DraftErrorKind, DraftRecord, ItemKind, LegacyBackup, RemapError,
    RemoveDraftsError, backup_key, draft_conflict_key, draft_conflict_path_prefix, draft_key,
    path_matches,
};

/// JS number encoding: integral values compare as integers.
fn num(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < 9.0e15 {
        json!(f as i64)
    } else {
        json!(f)
    }
}

fn err_js(e: &DraftError) -> Value {
    let name = match e.kind {
        DraftErrorKind::InvalidArgument => "TypeError",
        DraftErrorKind::Storage => "Error",
    };
    json!({"$error": {"name": name, "message": e.message}})
}

fn fail(e: &DraftError) -> Value {
    json!({"ok": false, "error": err_js(e)})
}

fn draft_js(d: &DraftRecord) -> Value {
    let mut m = Map::new();
    m.insert("schemaVersion".into(), json!(1));
    m.insert("path".into(), json!(d.path));
    m.insert("content".into(), json!(d.content));
    m.insert("baseRevision".into(), json!(d.base_revision));
    m.insert("draftRevision".into(), json!(d.draft_revision));
    m.insert("updatedAt".into(), num(d.updated_at));
    for (k, raw) in &d.extra {
        m.insert(k.clone(), serde_json::from_str(raw).unwrap());
    }
    Value::Object(m)
}

fn conflict_js(c: &DraftConflict) -> Value {
    json!({
        "schemaVersion": 1, "path": c.path, "sourcePath": c.source_path, "conflictId": c.conflict_id,
        "content": c.content, "baseRevision": c.base_revision, "draftRevision": c.draft_revision,
        "updatedAt": num(c.updated_at),
    })
}

fn backup_js(b: &LegacyBackup) -> Value {
    json!({"content": b.content, "timestamp": num(b.timestamp)})
}

fn remap_err_js(e: &RemapError) -> Value {
    let mut m = Map::new();
    m.insert("ok".into(), json!(false));
    m.insert("error".into(), err_js(&e.error));
    if let Some(a) = &e.affected_paths {
        m.insert("affectedPaths".into(), json!(a));
    }
    if let Some(s) = &e.source_path {
        m.insert("sourcePath".into(), json!(s));
    }
    if let Some(s) = &e.destination_path {
        m.insert("destinationPath".into(), json!(s));
    }
    if e.backup_collision {
        m.insert("backupCollision".into(), json!(true));
    }
    if e.collision {
        m.insert("collision".into(), json!(true));
    }
    if let Some(c) = &e.conflict_id {
        m.insert("conflictId".into(), json!(c));
    }
    if let Some(cs) = &e.collisions {
        let v: Vec<Value> =
            cs.iter().map(|p| json!({"sourcePath": p.source_path, "destinationPath": p.destination_path})).collect();
        m.insert("collisions".into(), Value::Array(v));
    }
    Value::Object(m)
}

fn remove_err_js(e: &RemoveDraftsError) -> Value {
    let mut v = fail(&e.error);
    if let Some(n) = e.removed {
        v["removed"] = json!(n);
    }
    v
}

fn kind(v: Option<&Value>) -> ItemKind {
    match v.and_then(Value::as_str) {
        Some("folder") => ItemKind::Folder,
        _ => ItemKind::File,
    }
}

/// Parse a JS draft object argument; `None` if it is not representable as a record.
fn draft_arg(v: &Value) -> Option<DraftRecord> {
    let o = v.as_object()?;
    if o.get("schemaVersion")? != &json!(1) {
        return None;
    }
    Some(DraftRecord {
        path: o.get("path")?.as_str()?.to_owned(),
        content: o.get("content")?.as_str()?.to_owned(),
        base_revision: o.get("baseRevision")?.as_str()?.to_owned(),
        draft_revision: o.get("draftRevision")?.as_str()?.to_owned(),
        updated_at: o.get("updatedAt")?.as_f64()?,
        extra: vec![],
    })
}

/// Outcome of one call step.
enum Step {
    Value(Value),
    /// Arguments not representable in the Rust signature: must be a TypeError.
    Unrepresentable,
}

fn run_call(d: &mut draft_common::Drafts, call: &str, args: &[Value]) -> Step {
    use Step::{Unrepresentable as U, Value as V};
    let s = |i: usize| args.get(i).and_then(Value::as_str);
    // Fetch N string args, or `None` when any is not a JS string.
    let s1 = || s(0);
    let s2 = || Some((s(0)?, s(1)?));
    let s3 = || Some((s(0)?, s(1)?, s(2)?));
    match call {
        "readDraft" => {
            let Some(p) = s1() else { return U };
            V(match d.read_draft(p) {
                Ok(r) => {
                    let mut v = json!({"ok": true, "draft": r.draft.as_ref().map(draft_js)});
                    if r.malformed {
                        v["malformed"] = json!(true);
                    }
                    v
                }
                Err(e) => fail(&e),
            })
        }
        "writeDraft" => {
            let Some((p, c, r)) = s3() else { return U };
            V(match d.write_draft(p, c, r) {
                Ok(w) => json!({"ok": true, "draft": draft_js(&w.draft), "unchanged": w.unchanged}),
                Err(e) => fail(&e),
            })
        }
        "removeDraftIfRevision" => {
            let Some((p, r)) = s2() else { return U };
            V(match d.remove_draft_if_revision(p, r) {
                Ok(removed) => json!({"ok": true, "removed": removed}),
                Err(e) => fail(&e),
            })
        }
        "removeDraft" => {
            let Some(p) = s1() else { return U };
            V(match d.remove_draft(p) {
                Ok(()) => json!({"ok": true}),
                Err(e) => fail(&e),
            })
        }
        "remapDrafts" => {
            let Some((o, n)) = s2() else { return U };
            V(match d.remap_drafts(o, n, kind(args.get(2))) {
                Ok(count) => json!({"ok": true, "remapped": count}),
                Err(e) => remap_err_js(&e),
            })
        }
        "removeDrafts" => {
            let Some(p) = s1() else { return U };
            V(match d.remove_drafts(p, kind(args.get(1))) {
                Ok(n) => json!({"ok": true, "removed": n}),
                Err(e) => remove_err_js(&e),
            })
        }
        "listDraftConflicts" => {
            let Some(p) = s1() else { return U };
            V(match d.list_draft_conflicts(p) {
                Ok(cs) => json!({"ok": true, "conflicts": cs.iter().map(conflict_js).collect::<Vec<_>>()}),
                Err(e) => fail(&e),
            })
        }
        "preserveDraftConflict" => {
            let (Some(p), Some(draft)) = (s1(), args.get(1).and_then(draft_arg)) else { return U };
            V(match d.preserve_draft_conflict(p, &draft) {
                Ok(c) => json!({"ok": true, "conflictId": c.conflict_id, "conflict": conflict_js(&c)}),
                Err(e) => fail(&e),
            })
        }
        "removeDraftConflictIfRevision" => {
            let Some((p, id, r)) = s3() else { return U };
            V(match d.remove_draft_conflict_if_revision(p, id, r) {
                Ok(removed) => json!({"ok": true, "removed": removed}),
                Err(e) => fail(&e),
            })
        }
        "listDrafts" => V(match d.list_drafts() {
            Ok(ds) => json!({"ok": true, "drafts": ds.iter().map(draft_js).collect::<Vec<_>>()}),
            Err(e) => fail(&e),
        }),
        "removeAllDrafts" => V(match d.remove_all_drafts() {
            Ok(()) => json!({"ok": true}),
            Err(e) => fail(&e),
        }),
        "readBackup" => {
            let Some(p) = s1() else { return U };
            V(match d.read_backup(p) {
                Ok(r) => {
                    let mut v = json!({"ok": true, "backup": r.backup.as_ref().map(backup_js)});
                    if r.malformed {
                        v["malformed"] = json!(true);
                    }
                    v
                }
                Err(e) => fail(&e),
            })
        }
        "writeBackup" => {
            let Some((p, c)) = s2() else { return U };
            V(match d.write_backup(p, c) {
                Ok(()) => json!({"ok": true}),
                Err(e) => fail(&e),
            })
        }
        "removeBackup" => {
            let Some(p) = s1() else { return U };
            V(match d.remove_backup(p) {
                Ok(()) => json!({"ok": true}),
                Err(e) => fail(&e),
            })
        }
        "removeBackupIfSnapshot" => {
            let (Some((p, c)), Some(ts)) = (s2(), args.get(2).and_then(Value::as_f64)) else { return U };
            V(match d.remove_backup_if_snapshot(p, c, ts) {
                Ok(removed) => json!({"ok": true, "removed": removed}),
                Err(e) => fail(&e),
            })
        }
        "removeAllBackups" => V(match d.remove_all_backups() {
            Ok(()) => json!({"ok": true}),
            Err(e) => fail(&e),
        }),
        "draftKey" => V(json!(draft_key(s1().unwrap()))),
        "backupKey" => V(json!(backup_key(s1().unwrap()))),
        "draftConflictKey" => {
            let (p, id) = s2().unwrap();
            V(json!(draft_conflict_key(p, id)))
        }
        "draftConflictPathPrefix" => V(json!(draft_conflict_path_prefix(s1().unwrap()))),
        "pathMatches" => {
            let (p, t) = s2().unwrap();
            V(json!(path_matches(p, t, kind(args.get(2)))))
        }
        other => panic!("unknown DraftStorage call {other}"),
    }
}

fn fault_from(v: &Value) -> Fault {
    let method = match v["method"].as_str().unwrap() {
        "length" => Method::Length,
        "key" => Method::Key,
        "getItem" => Method::GetItem,
        "setItem" => Method::SetItem,
        "removeItem" => Method::RemoveItem,
        m => panic!("unknown fault method {m}"),
    };
    Fault {
        method,
        key: v.get("key").and_then(Value::as_str).map(str::to_owned),
        prefix: v.get("prefix").and_then(Value::as_str).map(str::to_owned),
        mode: if v.get("mode").and_then(Value::as_str) == Some("noop") { FaultMode::Noop } else { FaultMode::Throw },
        message: v.get("message").and_then(Value::as_str).map(str::to_owned),
        times: None,
    }
}

fn is_type_error(v: &Value) -> bool {
    v["ok"] == json!(false) && v["error"]["$error"]["name"] == json!("TypeError")
}

#[test]
fn golden_draft_storage_scenarios() {
    let g = golden("js_draft_storage");
    assert_cases("js_draft_storage", g.cases_for("scenario"), |case| {
        let initial: Vec<(String, String)> = case.input["initial"]
            .as_array()
            .map(|a| a.iter().map(|e| (e[0].as_str().unwrap().to_owned(), e[1].as_str().unwrap().to_owned())).collect())
            .unwrap_or_default();
        let seed: Vec<(&str, &str)> = initial.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let mut d = drafts_seeded(&seed);
        let expected = case.expected["results"].as_array().unwrap();
        let steps = case.input["steps"].as_array().unwrap();
        let mut errs = Vec::new();

        for (i, step) in steps.iter().enumerate() {
            let want = &expected[i];
            if let Some(f) = step.get("fault") {
                d.backend_mut().add_fault(fault_from(f));
                continue;
            }
            if step.get("clearFaults").is_some() {
                d.backend_mut().clear_faults();
                continue;
            }
            if let Some(p) = step.get("providerFails") {
                d.backend_mut().provider_fails = p.as_bool().unwrap();
                continue;
            }
            if let Some(raw) = step.get("raw") {
                d.backend_mut().put_raw(raw[0].as_str().unwrap(), raw[1].as_str().unwrap());
                continue;
            }
            let call = step["call"].as_str().unwrap();
            let args: Vec<Value> = step.get("args").and_then(Value::as_array).cloned().unwrap_or_default();
            match run_call(&mut d, call, &args) {
                Step::Value(got) => {
                    if &got != want {
                        errs.push(format!("step {i} {call}{args:?}: got {got}, want {want}"));
                    }
                }
                Step::Unrepresentable => {
                    if !is_type_error(want) {
                        errs.push(format!("step {i} {call}{args:?}: unrepresentable args but original returned {want}"));
                    }
                }
            }
        }

        let want_after: Vec<(String, String)> = case.expected["storage_after"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| (e[0].as_str().unwrap().to_owned(), e[1].as_str().unwrap().to_owned()))
            .collect();
        if d.backend().entries != want_after {
            errs.push(format!("storage_after: got {:?}, want {want_after:?}", d.backend().entries));
        }
        if errs.is_empty() { Ok(()) } else { Err(errs.join("\n    ")) }
    });
}
