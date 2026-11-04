//! Port of `test/services/custom_theme_test.rb` (3 tests, A).
#![allow(clippy::unwrap_used)]

use frankmd_theme::custom::MAX_BYTES;
use frankmd_theme::{BASE_THEMES, COLOR_KEYS, CustomTheme, CustomThemeError};
use serde_json::{Value, json};

fn theme() -> Value {
    json!({"version": 1, "name": " My colors ", "base": "nord", "colors": {"accent": "#AABBCC"}})
}

fn merged(changes: Value) -> Value {
    let mut t = theme();
    for (k, v) in changes.as_object().unwrap() {
        t[k] = v.clone();
    }
    t
}

// port: test/services/custom_theme_test.rb:10 "normalizes a bounded palette with builtin base and no executable CSS"
#[test]
fn normalizes_a_bounded_palette_with_builtin_base_and_no_executable_css() {
    let result = CustomTheme::parse(&theme().to_string()).unwrap().unwrap();
    assert_eq!(result.name, "My colors");
    assert_eq!(
        result.colors,
        vec![("accent".to_owned(), "#aabbcc".to_owned())]
    );
    assert_eq!(result.base, "nord");
    assert_eq!(CustomTheme::parse("null").unwrap(), None);
}

// port: test/services/custom_theme_test.rb:18 "catalog and color allowlist match shipped theme files"
#[test]
fn catalog_and_color_allowlist_match_shipped_theme_files() {
    // The shipped theme files are `assets/themes/builtin.json` (extracted
    // unchanged from app/assets/tailwind/themes/*.css).
    let shipped: Value = serde_json::from_str(frankmd_theme::BUILTIN_THEMES_JSON).unwrap();
    let mut ids: Vec<&str> = shipped["themes"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let mut base: Vec<&str> = BASE_THEMES.to_vec();
    ids.sort_unstable();
    base.sort_unstable();
    assert_eq!(ids, base);

    let mut keys: Vec<&str> = shipped["themes"]["dark"]["colors"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let mut color_keys = COLOR_KEYS.to_vec();
    keys.sort_unstable();
    color_keys.sort_unstable();
    assert_eq!(keys, color_keys);
}

// port: test/services/custom_theme_test.rb:25 "rejects unsafe colors keys names versions and oversized values"
#[test]
fn rejects_unsafe_colors_keys_names_versions_and_oversized_values() {
    for value in [
        "red",
        "#123",
        "url(https://example.com)",
        "#aabbcc;display:none",
        "var(--secret)",
    ] {
        let r = CustomTheme::from_value(&merged(json!({"colors": {"accent": value}})));
        assert_eq!(r, Err(CustomThemeError::Colors), "{value}");
    }
    for changes in [
        json!({"colors": {"font-family": "#aabbcc"}}),
        json!({"base": "omarchy"}),
        json!({"version": 2}),
        json!({"name": ""}),
        json!({"name": "a".repeat(61)}),
        json!({"name": "bad\nname"}),
        json!({"css": "body{}"}),
    ] {
        assert!(
            CustomTheme::from_value(&merged(changes.clone())).is_err(),
            "{changes}"
        );
    }
    assert_eq!(
        CustomTheme::parse(&" ".repeat(MAX_BYTES + 1)),
        Err(CustomThemeError::TooLarge)
    );
}
