//! Golden conformance: `tests/fixtures/golden/hugo.json`, fn `template_source`
//! (`NotesService::hugo_template`; `None` ≙ `DEFAULT_TEMPLATE`). Non-UTF-8
//! templates are recorded as `{base64, encoding}` and compared as bytes.
//!
//! Skipped: fn `notes_root` (4 cases) — `NOTES_PATH` / `Rails.root/notes`
//! resolution belongs to frankmd-config / frankmd-app, not to the vault.
//! The pure fns (`slugify`, `render_template`, `update_frontmatter_slug`,
//! `generate_blog_post`, `constants`) are frankmd-core goldens.
#![allow(clippy::disallowed_methods, clippy::unwrap_used, missing_docs, unreachable_pub)]

mod common;

use common::{Sandbox, base64_decode, spec_orders};
use frankmd_core::hugo::DEFAULT_TEMPLATE;
use frankmd_fs::NotesService;
use frankmd_testkit::serde_json::Value;
use frankmd_testkit::{assert_cases, golden};

#[test]
fn golden_template_source() {
    let g = golden("hugo");
    let orders = spec_orders("hugo");
    assert_cases("hugo template_source", g.cases_for("template_source"), |case| {
        let i = &case.input;
        let sb = Sandbox::from_specs(&i["tree"], &Value::Null, orders.get(&case.name));
        // `base_path: "<OTHER>"` reads the template of another directory.
        let other = (i["base_path"].as_str() == Some("<OTHER>")).then(|| Sandbox::from_tree(&i["other_tree"]));
        let vault = other.as_ref().unwrap_or(&sb).vault();
        let actual = NotesService::new(&vault).hugo_template().unwrap_or_else(|| DEFAULT_TEMPLATE.as_bytes().to_vec());
        let expected = match &case.expected {
            Value::String(s) => s.as_bytes().to_vec(),
            Value::Object(o) => base64_decode(o["base64"].as_str().unwrap()),
            other => return Err(format!("unexpected expected value {other}")),
        };
        if actual == expected {
            Ok(())
        } else {
            Err(format!(
                "expected {:?}, got {:?}",
                String::from_utf8_lossy(&expected),
                String::from_utf8_lossy(&actual)
            ))
        }
    });
}
