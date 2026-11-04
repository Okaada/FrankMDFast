//! Golden conformance: `tests/fixtures/golden/js_tree_utils.json`
//! (`flattenTree`) through [`frankmd_core::tree::flatten_files`].
//!
//! Inputs are decoded with `decode_js` (`undefined` → null) and deserialized
//! leniently: `null`/`undefined` tree → `[]`; nodes of unknown `type` are dropped; extra JS properties (`size`)
//! are ignored by serde, so outputs are compared on `{type, name, path}`.
#![allow(missing_docs, clippy::unwrap_used)]

use frankmd_core::tree::{TreeNode, flatten_files};
use frankmd_testkit::serde_json::{self, Value, json};
use frankmd_testkit::{assert_cases, decode_js, golden};

/// JS skips nodes whose `type` is neither "file" nor "folder" (e.g. "image");
/// TreeNode cannot represent them, so drop them before deserializing.
fn known_types(v: &Value) -> Value {
    match v {
        Value::Array(a) => Value::Array(
            a.iter()
                .filter(|n| matches!(n["type"].as_str(), Some("file" | "folder")))
                .map(|n| {
                    let mut n = n.clone();
                    if let Some(kids) = n.get("children").cloned() {
                        n["children"] = known_types(&kids);
                    }
                    n
                })
                .collect(),
        ),
        Value::Null => Value::Array(vec![]),
        other => other.clone(),
    }
}

#[test]
fn golden_flatten_tree() {
    let g = golden("js_tree_utils");
    assert_cases("js_tree_utils", g.cases_for("flattenTree"), |c| {
        let arg = decode_js(&c.input)[0].clone();
        let nodes: Vec<TreeNode> = match arg {
            Value::Null => vec![],
            v => serde_json::from_value(known_types(&v)).map_err(|e| e.to_string())?,
        };
        // JS `flattenTree(items, result)` appends to the optional accumulator;
        // the Rust API returns a fresh list, so the test prepends the accumulator.
        let shape = |e: &Value| json!({"type": e["type"], "name": e["name"], "path": e["path"]});
        let acc: Vec<Value> = decode_js(&c.input)
            .get(1)
            .and_then(Value::as_array)
            .map(|a| a.iter().map(shape).collect())
            .unwrap_or_default();
        let got: Vec<Value> = acc
            .into_iter()
            .chain(
                flatten_files(&nodes)
                    .into_iter()
                    .map(|n| json!({"type": "file", "name": n.name(), "path": n.path().as_str()})),
            )
            .collect();
        let want: Vec<Value> = decode_js(&c.expected)
            .as_array()
            .unwrap()
            .iter()
            .map(shape)
            .collect();
        if got == want {
            Ok(())
        } else {
            Err(format!("expected {want:?}, got {got:?}"))
        }
    });
}
