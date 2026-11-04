//! Golden conformance: `tests/fixtures/golden/hugo.json` (HugoService).
//!
//! Skipped on purpose:
//! - `template_source` (reads `.hugo_template.md` from disk: frankmd-fs, see
//!   `NotesService::hugo_template`) and `notes_root` (Rails `NOTES_PATH`
//!   resolution: frankmd-config). Their presence is asserted so the count shows.
//! - cases whose expected value is a Ruby `NoMethodError` (nil / Integer passed
//!   where a String is required): unrepresentable with `&str` signatures.
#![allow(clippy::unwrap_used, missing_docs)]

use frankmd_core::hugo::{self, HugoPathStyle, NewPost};
use frankmd_core::slug::{self, slugify};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, golden};

const FIXTURE: &str = "hugo";

/// Ruby type errors (`{error: NoMethodError}`) cannot happen with `&str`.
fn is_type_error(c: &Case) -> bool {
    c.expected.get("error").and_then(Value::as_str) == Some("NoMethodError")
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str()
        .unwrap_or_else(|| panic!("input.{k} is not a string: {v}"))
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
    assert_cases(
        FIXTURE,
        g.cases_for("slugify").filter(|c| !is_type_error(c)),
        |c| eq(&json!(slugify(s(&c.input, "text"))), &c.expected),
    );
}

#[test]
fn render_template_cases() {
    let g = golden(FIXTURE);
    assert_cases(
        FIXTURE,
        g.cases_for("render_template").filter(|c| !is_type_error(c)),
        |c| {
            let i = &c.input;
            let out =
                hugo::render_template(s(i, "template"), s(i, "title"), s(i, "slug"), s(i, "date"));
            eq(&json!(out), &c.expected)
        },
    );
}

#[test]
fn update_frontmatter_slug_cases() {
    let g = golden(FIXTURE);
    assert_cases(
        FIXTURE,
        g.cases_for("update_frontmatter_slug")
            .filter(|c| !is_type_error(c)),
        |c| {
            let out =
                hugo::update_frontmatter_slug(s(&c.input, "content"), s(&c.input, "new_slug"));
            eq(&json!(out), &c.expected)
        },
    );
}

#[test]
fn constants() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("constants"), |c| {
        let e = &c.expected;
        eq(&json!(hugo::TEMPLATE_FILE), &e["TEMPLATE_FILE"])?;
        eq(&json!(hugo::DEFAULT_TEMPLATE), &e["DEFAULT_TEMPLATE"])?;
        // JSON objects lose source order; compare as a map and check no duplicates.
        let map: frankmd_testkit::serde_json::Map<String, Value> = slug::ACCENT_MAP
            .iter()
            .map(|(k, v)| (k.to_string(), json!(v)))
            .collect();
        if map.len() != slug::ACCENT_MAP.len() {
            return Err("ACCENT_MAP has duplicate keys".into());
        }
        eq(&Value::Object(map), &e["ACCENT_MAP"])?;
        // Config schema: default "dated" (the key itself lives in frankmd-config).
        let schema = &e["config_schema_hugo_path_style"];
        eq(
            &json!(schema["default"].as_str() == Some("dated")),
            &json!(true),
        )?;
        eq(
            &json!(format!("{:?}", HugoPathStyle::default())),
            &json!("Dated"),
        )
    });
}

/// `path_style ||= Config.new.get("hugo_path_style") || "dated"` resolved from
/// the fixture: explicit kwarg (even `""`) wins; else the `.fed` line (an empty
/// value counts as unset); else `ENV["HUGO_PATH_STYLE"]` (raw, unstripped).
fn resolve_style(i: &Value) -> HugoPathStyle {
    let given = i["kwargs_given"]
        .as_array()
        .is_some_and(|a| a.iter().any(|k| k == "path_style"));
    if given {
        return HugoPathStyle::from_config(i["path_style"].as_str());
    }
    let from_fed = i["tree"][".fed"].as_str().and_then(|fed| {
        fed.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "hugo_path_style").then(|| v.trim().trim_matches(['"', '\'']).to_owned())
        })
    });
    let value = from_fed
        .filter(|v| !v.is_empty())
        .or_else(|| i["env"]["HUGO_PATH_STYLE"].as_str().map(str::to_owned));
    HugoPathStyle::from_config(value.as_deref())
}

#[test]
fn generate_blog_post_cases() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, g.cases_for("generate_blog_post"), |c| {
        let i = &c.input;
        let zone = s(&i["now"], "zone");
        let civil: jiff::civil::DateTime = s(&i["now"], "time").parse().unwrap();
        let now = civil.in_tz(zone).unwrap();
        let template = i["tree"][hugo::TEMPLATE_FILE].as_str();
        let post = hugo::generate_post(&NewPost {
            title: s(i, "title"),
            parent: i["parent"].as_str(),
            now,
            style: resolve_style(i),
            template,
        });
        eq(
            &json!({"path": post.path, "content": post.content}),
            &c.expected["result"],
        )
    });
}

#[test]
fn skipped_fns_are_present() {
    let g = golden(FIXTURE);
    assert!(
        g.cases_for("template_source").count() > 0,
        "template_source cases (frankmd-fs golden)"
    );
    assert!(
        g.cases_for("notes_root").count() > 0,
        "notes_root cases (Rails ENV, skipped)"
    );
}
