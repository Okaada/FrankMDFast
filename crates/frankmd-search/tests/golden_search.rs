//! Conformance with `tests/fixtures/golden/search.json` (FrankMD
//! `NotesService#search_content` + `GET /notes/search`).
//!
//! Comparison rules (from the fixture notes):
//! - results are compared as `render json:` would emit them (see `common::hit_json`);
//! - Ruby's final `sort_by { -mtime.to_i }` gives no order among equal whole-second
//!   mtimes, so within one mtime group the results are compared after a *stable*
//!   sort by path (which keeps per-file line order);
//! - the "readdir order" case only contracts the count and mtime-desc order;
//! - file mtimes: explicit `{content, mtime}` values, otherwise the mtime the
//!   expected output reports for that path, otherwise `BASE_MTIME - 100*i`
//!   (`serde_json` maps are not guaranteed to keep spec order, and the
//!   mtime of a file that does not match is unobservable).
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

mod common;

use std::collections::BTreeMap;

use common::{BASE_MTIME, Sandbox, hit_json, parse_iso_millis_utc, set_len, set_mtime};
use frankmd_search::content::{self, MAX_SEARCH_FILE_BYTES, Query};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};

fn mtime_key(v: &Value) -> i64 {
    parse_iso_millis_utc(v["mtime"].as_str().unwrap()).floor() as i64
}

/// Compare result arrays modulo the order of equal-mtime groups.
fn compare(actual: &[Value], expected: &[Value]) -> Result<(), String> {
    fn groups(v: &[Value]) -> Vec<Vec<Value>> {
        let mut out: Vec<Vec<Value>> = Vec::new();
        for item in v {
            match out.last_mut() {
                Some(g) if mtime_key(&g[0]) == mtime_key(item) => g.push(item.clone()),
                _ => out.push(vec![item.clone()]),
            }
        }
        for g in &mut out {
            g.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        }
        out
    }
    if groups(actual) == groups(expected) {
        Ok(())
    } else {
        Err(format!(
            "\n  expected: {}\n  actual:   {}",
            Value::Array(expected.to_vec()),
            Value::Array(actual.to_vec())
        ))
    }
}

fn run_service_case(case: &Case) -> Result<(), String> {
    let input = &case.input;
    let expected = case.expected.as_array().unwrap();
    let sb = Sandbox::new();
    sb.build(&sb.outside, &input["outside"], &|_| None);
    let reported: BTreeMap<String, f64> = expected
        .iter()
        .map(|h| (h["path"].as_str().unwrap().to_owned(), parse_iso_millis_utc(h["mtime"].as_str().unwrap())))
        .collect();
    sb.build(&sb.root, &input["tree"], &|rel| {
        // explicit mtimes in the spec win (applied inside build when this is None)
        let explicit = input["tree"][rel].get("mtime").is_some();
        if explicit { None } else { reported.get(rel).copied() }
    });

    // Per-case setup done by the Ruby generator's block.
    match case.name.as_str() {
        "file over 5MB skipped" => {
            let p = sb.root.join("large.md");
            set_len(&p, MAX_SEARCH_FILE_BYTES + 1);
            set_mtime(&p, (BASE_MTIME - 1) as f64);
        }
        n if n.starts_with("file exactly 5MB searched") => {
            let p = sb.root.join("edge.md");
            let mut body = b"findme\n".to_vec();
            body.resize(MAX_SEARCH_FILE_BYTES as usize, b'\n');
            std::fs::write(&p, body).unwrap();
            set_mtime(&p, BASE_MTIME as f64);
        }
        _ => {}
    }

    let opts = &input["options"];
    let mut q = Query::new("");
    q.pattern = input["query"].as_str().map(str::to_owned);
    if let Some(c) = opts.get("context_lines").and_then(Value::as_i64) {
        q.context_lines = c;
    }
    if let Some(m) = opts.get("max_results").and_then(Value::as_u64) {
        q.max_results = m as usize;
    }
    let results = content::search(&sb.root, &q);
    let actual: Vec<Value> = results.hits.iter().map(hit_json).collect();

    if case.name.contains("readdir order") {
        // Only the count and the mtime-desc ordering are contractual.
        if actual.len() != expected.len() {
            return Err(format!("expected {} results, got {}", expected.len(), actual.len()));
        }
        let keys: Vec<i64> = actual.iter().map(mtime_key).collect();
        if keys.windows(2).any(|w| w[0] < w[1]) {
            return Err(format!("results not sorted by mtime desc: {keys:?}"));
        }
        return Ok(());
    }
    compare(&actual, expected)
}

#[test]
fn golden_search_content() {
    let g = golden("search");
    assert_cases("search.json", g.cases_for("NotesService#search_content"), run_service_case);
}

/// `GET /notes/search` = `search_content(q, context_lines: 3, max_results: 20)`.
/// The `text/html` case renders the Rails `_search_results` partial (UI markup,
/// re-expressed by the GPUI view) and is not checked here.
#[test]
fn golden_search_http() {
    let g = golden("search");
    let cases = g.cases_for("http").filter(|c| c.input["accept"] == "application/json");
    assert_cases("search.json#http", cases, |case| {
        let sb = Sandbox::new();
        for i in 0..25 {
            sb.note(&format!("n{i:02}.md"), format!("findme {i}"), (BASE_MTIME - i) as f64);
        }
        sb.note("one.md", "single <b>hit</b> & \"q\"", (BASE_MTIME + 100) as f64);
        let r = content::search(&sb.root, &Query::ui(case.input["query"].as_str().unwrap()));
        let actual: Vec<Value> = r.hits.iter().map(hit_json).collect();
        let exp = &case.expected;
        if exp["status"] != json!(200) {
            return Err(format!("unexpected fixture status {}", exp["status"]));
        }
        if let Some(count) = exp.get("count").and_then(Value::as_u64) {
            return if actual.len() as u64 == count {
                Ok(())
            } else {
                Err(format!("expected {count} results, got {}", actual.len()))
            };
        }
        compare(&actual, exp["body"].as_array().unwrap())
    });
}
