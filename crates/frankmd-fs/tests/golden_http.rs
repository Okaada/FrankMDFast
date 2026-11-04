//! Golden conformance for every recorded Rails HTTP request (`fn == "http"`
//! or an `"http"` op) in the notes-filesystem fixtures, driven through the
//! controller emulation in `common::http` on top of the real Rust API.
//!
//! Normalizations: `.fed` / `.frankmd-config.lock` side effects of Rails
//! `Config.new` are dropped from both `tree_after`s (frankmd-config's
//! contract); tree-partial HTML is compared as its `data-path` sequence;
//! `/notes/search` requests are skipped (frankmd-search).
#![allow(
    missing_docs,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::disallowed_methods
)]

mod common;

use common::http::{HttpResponse, dispatch};
use common::{
    Sandbox, check_eq, expected_tree, fs_error_json, spec_orders, without_config_side_effects,
};
use frankmd_fs::{NotesService, RecentStore};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};

/// The ops of a case, or `None` if it records no HTTP request.
fn http_ops(case: &Case) -> Option<Vec<Value>> {
    let input = &case.input;
    let ops: Vec<Value> = if let Some(ops) = input.get("ops").and_then(Value::as_array) {
        ops.clone()
    } else if case.func.as_deref() == Some("http") {
        vec![json!({
            "op": "http",
            "method": input.get("method").and_then(Value::as_str).unwrap_or("GET"),
            "url": input.get("url").and_then(Value::as_str).unwrap_or("/notes/tree"),
            "params": input.get("params").cloned().unwrap_or(Value::Null),
        })]
    } else {
        return None;
    };
    ops.iter().any(|o| o["op"] == "http").then_some(ops)
}

/// `data-path` values of a rendered tree partial, in order, deduplicated
/// (a folder carries it on both its wrapper and its header).
fn html_tree_paths(html: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"data-path="([^"]*)""#).unwrap();
    let mut out: Vec<String> = Vec::new();
    for c in re.captures_iter(html) {
        let p = c[1]
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    out
}

fn run_other(sb: &Sandbox, op: &Value) -> Value {
    let vault = sb.vault();
    let store = common::FedRecentStore::new(&sb.root);
    let svc = NotesService::new(&vault).with_recents(&store as &dyn RecentStore);
    let s = |k: &str| op[k].as_str().unwrap_or("").to_owned();
    let r = match op["op"].as_str().unwrap() {
        "rename" => svc.rename(&s("old_path"), &s("new_path")),
        "delete" => svc.delete(&s("path"), op["expected_revision"].as_str()),
        "write" => svc.write(&s("path"), s("content").as_bytes()).map(|_| ()),
        other => panic!("unsupported non-http op {other}"),
    };
    match r {
        Ok(()) => Value::Bool(true),
        Err(e) => sb.norm_value(&fs_error_json(&e)),
    }
}

fn check_response(
    sb: &Sandbox,
    i: usize,
    resp: &HttpResponse,
    expected: &Value,
) -> Result<(), String> {
    if let Some(Value::String(html)) = expected.get("body")
        && (html.contains("data-path=") || html.starts_with("<div"))
    {
        check_eq(
            &format!("op {i} status"),
            &json!(resp.status),
            &expected["status"],
        )?;
        let want = html_tree_paths(html);
        return check_eq(
            &format!("op {i} tree data-paths"),
            &json!(resp.tree_paths()),
            &json!(want),
        );
    }
    check_eq(
        &format!("op {i}"),
        &sb.norm_value(&resp.to_fixture()),
        expected,
    )
}

fn run_fixture(fixture: &str) {
    let g = golden(fixture);
    let orders = spec_orders(fixture);
    let cases: Vec<&Case> = g
        .cases
        .iter()
        .filter(|c| {
            http_ops(c).is_some_and(|ops| {
                !ops.iter().any(|o| {
                    o["url"]
                        .as_str()
                        .is_some_and(|u| u.starts_with("/notes/search"))
                })
            })
        })
        .collect();
    assert_cases(fixture, cases, |case| {
        let ops = http_ops(case).unwrap();
        let sb = Sandbox::from_specs(
            &case.input["tree"],
            &case.input["outside"],
            orders.get(&case.name),
        );
        let expected_results: Vec<Value> = match case.expected.get("results") {
            Some(Value::Array(a)) => a.clone(),
            // Single-request cases: the response sits next to `tree_after`,
            // which is compared separately below.
            _ => {
                let mut single = case.expected.clone();
                if let Some(o) = single.as_object_mut() {
                    o.remove("tree_after");
                }
                vec![single]
            }
        };
        let mut errs = Vec::new();
        for (i, (op, want)) in ops.iter().zip(&expected_results).enumerate() {
            let res = if op["op"] == "http" {
                let resp = dispatch(
                    &sb,
                    op["method"].as_str().unwrap(),
                    op["url"].as_str().unwrap(),
                    &op["params"],
                );
                check_response(&sb, i, &resp, want)
            } else {
                check_eq(&format!("op {i} ({})", op["op"]), &run_other(&sb, op), want)
            };
            if let Err(e) = res {
                errs.push(e);
            }
        }
        if let Some(t) = case.expected.get("tree_after") {
            let actual = without_config_side_effects(sb.snapshot());
            let want = without_config_side_effects(expected_tree(t));
            if actual != want {
                errs.push(format!(
                    "tree_after:\n    expected: {want:?}\n    actual:   {actual:?}"
                ));
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("\n  "))
        }
    });
}

#[test]
fn golden_http_path_safety() {
    run_fixture("path_safety");
}

#[test]
fn golden_http_notes_tree() {
    run_fixture("notes_tree");
}

#[test]
fn golden_http_notes_crud() {
    run_fixture("notes_crud");
}

#[test]
fn golden_http_notes_rename() {
    run_fixture("notes_rename");
}

#[test]
fn golden_http_backlinks() {
    run_fixture("backlinks");
}
