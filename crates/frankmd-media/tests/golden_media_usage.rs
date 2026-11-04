//! Conformance against `tests/fixtures/golden/media_usage.json` (233 cases).
//!
//! - `build_index`: tree materialized at a temp root (`<OUTSIDE>/x` symlinks point
//!   into a sibling dir built from `outside_tree`); also checks `usage_counts`,
//!   `library_paths` (MediaLibrary with DEFAULT extensions) and
//!   `unreferenced_library_paths`.
//! - Private helpers (`references_in`, `canonical_media_path`, ...) are public fns.
//! - **Decision:** `canonical_media_path("images/%FF.png")` decodes to invalid UTF-8
//!   in Ruby (`{invalid_utf8_base64}`); Rust keys are `String`, so the expected
//!   value is the lossy-decoded string (U+FFFD), consistent with notes being scrubbed.
#![allow(clippy::disallowed_methods)]

#[path = "common/library.rs"]
mod library;

use frankmd_media::config::MediaConfig;
use frankmd_media::library::MediaLibrary;
use frankmd_media::usage::{
    self, HTML_SOURCE, MARKDOWN_DESTINATION, MAX_CACHED_ROOTS, MEDIA_ROOTS, MediaUsage,
    REFERENCE_DEFINITION, REFERENCE_USE,
};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, golden};
use library::{Notes, unbase64};

fn eq(actual: &Value, expected: &Value) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected}\n     got {actual}"))
    }
}

fn s<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str()
        .ok_or_else(|| format!("input.{k} not a string"))
}

#[test]
fn constants() {
    let g = golden("media_usage");
    assert_cases("media_usage constants", g.cases_for("constants"), |c| {
        eq(
            &json!({
                "MEDIA_ROOTS": MEDIA_ROOTS, "MAX_CACHED_ROOTS": MAX_CACHED_ROOTS,
                "MARKDOWN_DESTINATION": MARKDOWN_DESTINATION, "REFERENCE_DEFINITION": REFERENCE_DEFINITION,
                "REFERENCE_USE": REFERENCE_USE, "HTML_SOURCE": HTML_SOURCE,
            }),
            &c.expected,
        )
    });
}

#[test]
fn build_index() {
    let g = golden("media_usage");
    assert_cases("media_usage build_index", g.cases_for("build_index"), |c| {
        let n = Notes::from_golden(&c.input["tree"], c.input.get("outside_tree"));
        pin_unpinned_mtimes(&n, &c.input["tree"]);
        let index = MediaUsage::new(n.root())
            .build_index()
            .map_err(|e| format!("error: {e}"))?;
        eq(&json!(*index), &c.expected["index"])?;
        eq(
            &json!(usage::usage_counts(&index)),
            &c.expected["usage_counts"],
        )?;
        let vault = n.vault();
        let listing = MediaLibrary::from_config(&vault, &MediaConfig::new(n.root())).list();
        let mut library_paths: Vec<String> = listing
            .images
            .iter()
            .chain(listing.videos.iter())
            .map(|i| i.path.clone())
            .collect();
        // Ruby: `list[:images].map(path) + list[:videos].map(path)` in listing order.
        eq(&json!(library_paths), &c.expected["library_paths"])?;
        library_paths.retain(|p| !index.contains_key(p));
        eq(
            &json!(library_paths),
            &c.expected["unreferenced_library_paths"],
        )
    });
}

/// Mechanical: the fixture trees carry no mtimes, so Ruby's listing order
/// (mtime desc) came from its write order, which a JSON object (sorted keys
/// here) cannot reproduce. Pinning every plain-content file to one mtime makes
/// the order deterministic (the `path.downcase` tie-break); every recorded
/// listing in this fixture is consistent with that order.
fn pin_unpinned_mtimes(n: &Notes, tree: &Value) {
    for (rel, v) in tree.as_object().into_iter().flatten() {
        if v.is_string() && !rel.ends_with('/') {
            library::set_mtime(&n.root().join(rel), 1_767_268_800);
        }
    }
}

#[test]
fn canonical_media_path() {
    let g = golden("media_usage");
    assert_cases(
        "media_usage canonical_media_path",
        g.cases_for("canonical_media_path"),
        |c| {
            let got =
                usage::canonical_media_path(s(&c.input, "reference")?, s(&c.input, "note_path")?);
            let expected = match c
                .expected
                .get("invalid_utf8_base64")
                .and_then(Value::as_str)
            {
                Some(b64) => json!(String::from_utf8_lossy(&unbase64(b64))),
                None => c.expected.clone(),
            };
            eq(&json!(got), &expected)
        },
    );
}

#[test]
fn markdown_without_code() {
    let g = golden("media_usage");
    assert_cases(
        "media_usage markdown_without_code",
        g.cases_for("markdown_without_code"),
        |c| {
            eq(
                &json!(usage::markdown_without_code(s(&c.input, "content")?)),
                &c.expected,
            )
        },
    );
}

#[test]
fn normalize_reference_label() {
    let g = golden("media_usage");
    assert_cases(
        "media_usage normalize_reference_label",
        g.cases_for("normalize_reference_label"),
        |c| {
            eq(
                &json!(usage::normalize_reference_label(s(&c.input, "label")?)),
                &c.expected,
            )
        },
    );
}

#[test]
fn references_in() {
    let g = golden("media_usage");
    assert_cases(
        "media_usage references_in",
        g.cases_for("references_in"),
        |c| {
            eq(
                &json!(usage::references_in(s(&c.input, "content")?)),
                &c.expected,
            )
        },
    );
}

#[test]
fn strip_unescaped_url_suffix() {
    let g = golden("media_usage");
    assert_cases(
        "media_usage strip_unescaped_url_suffix",
        g.cases_for("strip_unescaped_url_suffix"),
        |c| {
            eq(
                &json!(usage::strip_unescaped_url_suffix(s(&c.input, "path")?)),
                &c.expected,
            )
        },
    );
}

#[test]
fn every_case_is_wired() {
    let g = golden("media_usage");
    let wired = [
        "constants",
        "build_index",
        "canonical_media_path",
        "markdown_without_code",
        "normalize_reference_label",
        "references_in",
        "strip_unescaped_url_suffix",
    ];
    let unwired: Vec<_> = g
        .cases
        .iter()
        .filter(|c| !wired.contains(&c.func.as_deref().unwrap_or("")))
        .map(|c| &c.name)
        .collect();
    assert!(unwired.is_empty(), "unwired cases: {unwired:?}");
    assert_eq!(g.cases.len(), 233);
}
