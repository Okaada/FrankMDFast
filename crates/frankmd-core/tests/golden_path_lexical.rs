//! Golden conformance: the pure functions of `tests/fixtures/golden/path_safety.json`.
//!
//! - `PathSafety.contain` with `base_exists: false` → [`lexical_contain`]
//!   (the realpath half and the `base_exists: true` cases are frankmd-fs's).
//!   Non-string Ruby paths: `{symbol: x}` and integers are stringified as Ruby
//!   `Pathname#join` does; `nil` → `None`. **Decision:** a NUL byte made Ruby
//!   raise `ArgumentError("pathname contains null byte")`; the Rust API rejects
//!   it with `None` instead.
//! - `PathSafety.within?` → [`is_within`].
//! - `Note.normalize_path` → [`normalize_note_path`] (`nil` → `""`).
//! - `Note#name/directory/valid?` → `note_name`, `note_directory`,
//!   `validate_note_path` on the **raw** path (the generator does
//!   `Note.new(path: p)`, no normalization); `to_param` is the path itself
//!   (`nil` stays `null`).
//! - `Folder#name/parent_path/valid?` → `folder_name`, `folder_parent_path`,
//!   `validate_folder_path` (raw path; `nil` → `""`).
#![allow(missing_docs, clippy::unwrap_used)]

use frankmd_core::note::{
    folder_name, folder_parent_path, note_directory, note_name, validate_folder_path, validate_note_path,
};
use frankmd_core::path::{is_within, lexical_contain, normalize_note_path};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};

fn check(actual: Value, c: &Case) -> Result<(), String> {
    if actual == c.expected { Ok(()) } else { Err(format!("expected {}, got {actual}", c.expected)) }
}

fn ruby_path(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Object(o) if o.contains_key("symbol") => Some(o["symbol"].as_str().unwrap().to_owned()),
        other => panic!("unsupported path input {other}"),
    }
}

#[test]
fn golden_lexical_contain() {
    let g = golden("path_safety");
    let cases = g.cases_for("PathSafety.contain").filter(|c| c.input["base_exists"] == json!(false));
    assert_cases("path_safety", cases, |c| {
        let base = c.input["base"].as_str().unwrap();
        let path = ruby_path(&c.input["path"]);
        let got = lexical_contain(base, path.as_deref());
        let want = if c.expects_error() { None } else { c.expected.as_str().map(str::to_owned) };
        if got == want { Ok(()) } else { Err(format!("expected {want:?}, got {got:?}")) }
    });
}

#[test]
fn golden_within() {
    let g = golden("path_safety");
    assert_cases("path_safety", g.cases_for("PathSafety.within?"), |c| {
        let got = is_within(c.input["path"].as_str().unwrap(), c.input["base"].as_str().unwrap());
        check(json!(got), c)
    });
}

#[test]
fn golden_normalize_note_path() {
    let g = golden("path_safety");
    assert_cases("path_safety", g.cases_for("Note.normalize_path"), |c| {
        check(json!(normalize_note_path(c.input.as_str().unwrap_or(""))), c)
    });
}

#[test]
fn golden_note_attrs() {
    let g = golden("path_safety");
    assert_cases("path_safety", g.cases_for("Note#name/directory/valid?"), |c| {
        let p = c.input.as_str().unwrap_or("");
        let errors: Vec<String> = validate_note_path(p).iter().map(ToString::to_string).collect();
        check(
            json!({
                "name": note_name(p),
                "directory": note_directory(p),
                "valid": errors.is_empty(),
                "errors": errors,
                "to_param": c.input,
            }),
            c,
        )
    });
}

#[test]
fn golden_folder_attrs() {
    let g = golden("path_safety");
    assert_cases("path_safety", g.cases_for("Folder#name/parent_path/valid?"), |c| {
        let p = c.input.as_str().unwrap_or("");
        let errors: Vec<String> = validate_folder_path(p).iter().map(ToString::to_string).collect();
        check(
            json!({
                "name": folder_name(p),
                "parent_path": folder_parent_path(p),
                "valid": errors.is_empty(),
                "errors": errors,
            }),
            c,
        )
    });
}
