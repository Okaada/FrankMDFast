//! Golden conformance: `tests/fixtures/golden/js_new_note_utils.json`
//! (`app/javascript/lib/new_note_utils.js`). Dates are `{$date}` in UTC
//! (fixture TZ pinned to UTC, so local date = UTC date).
#![allow(clippy::unwrap_used, missing_docs)]

use frankmd_core::hugo::HugoPathStyle;
use frankmd_core::new_note::{
    NoteTemplate, PreviewRequest, can_create_note, hugo_date_path, is_valid_note_name,
    preview_note_path, preview_slugify,
};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, decode_js, golden};

const FIXTURE: &str = "js_new_note_utils";

fn args(c: &frankmd_testkit::Case) -> Vec<Value> {
    decode_js(&c.input).as_array().unwrap().clone()
}

/// `{$date: ISO}` (already decoded to the ISO string) → UTC civil date.
fn utc_date(v: &Value) -> jiff::civil::Date {
    let ts: jiff::Timestamp = v.as_str().unwrap().parse().unwrap();
    ts.to_zoned(jiff::tz::TimeZone::UTC).date()
}

fn eq(actual: &Value, expected: &Value) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "\n    expected: {expected}\n    actual:   {actual}"
        ))
    }
}

#[test]
fn slugify_cases() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("slugify"), |c| {
        let a = args(c);
        eq(
            &json!(preview_slugify(a[0].as_str())),
            &decode_js(&c.expected),
        )
    });
}

#[test]
fn hugo_date_path_cases() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("hugoDatePath"), |c| {
        let a = args(c);
        eq(
            &json!(hugo_date_path(utc_date(&a[0]))),
            &decode_js(&c.expected),
        )
    });
}

#[test]
fn is_valid_note_name_cases() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("isValidNoteName"), |c| {
        let a = args(c);
        eq(
            &json!(is_valid_note_name(a[0].as_str())),
            &decode_js(&c.expected),
        )
    });
}

#[test]
fn can_create_note_cases() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("canCreateNote"), |c| {
        let a = args(c);
        // JS default `template = "empty"`.
        let tpl = a
            .get(1)
            .and_then(Value::as_str)
            .map_or(NoteTemplate::Empty, NoteTemplate::from_js);
        eq(
            &json!(can_create_note(a[0].as_str(), tpl)),
            &decode_js(&c.expected),
        )
    });
}

#[test]
fn preview_note_path_cases() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("previewNotePath"), |c| {
        let a = args(c);
        let o = &a[0];
        let str_or =
            |k: &str, d: &'static str| o.get(k).and_then(Value::as_str).unwrap_or(d).to_owned();
        let (parent, placeholder, root) = (
            str_or("parent", ""),
            str_or("namePlaceholder", "name"),
            str_or("rootSegment", "root"),
        );
        let req = PreviewRequest {
            name: o.get("name").and_then(Value::as_str),
            parent: &parent,
            template: o
                .get("template")
                .and_then(Value::as_str)
                .map_or(NoteTemplate::Empty, NoteTemplate::from_js),
            // JS: `pathStyle === "flat"`; same rule as HugoPathStyle::from_config.
            style: HugoPathStyle::from_config(o.get("pathStyle").and_then(Value::as_str)),
            date: utc_date(&o["date"]),
            name_placeholder: &placeholder,
            root_segment: &root,
        };
        let r = preview_note_path(&req);
        eq(
            &json!({"valid": r.valid, "empty": r.empty, "path": r.path}),
            &decode_js(&c.expected),
        )
    });
}
