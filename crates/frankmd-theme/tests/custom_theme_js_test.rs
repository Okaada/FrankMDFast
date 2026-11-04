//! Port of `test/javascript/lib/custom_theme.test.js` (4 tests, A).
//! DOM style mutation (`applyCustomColors`/`clearCustomColors`) becomes the
//! pure override filter plus `UiTokens::with_overrides`: clearing a custom
//! palette is just going back to the base tokens, so "leaves other
//! properties alone" holds by construction.
#![allow(clippy::unwrap_used)]

use frankmd_theme::custom::{
    applicable_overrides, contrast_ratio_hex, normalize_color, theme_color,
};
use serde_json::json;

// port: test/javascript/lib/custom_theme.test.js:10 "accepts hex colors and rejects CSS expressions"
#[test]
fn accepts_hex_colors_and_rejects_css_expressions() {
    assert_eq!(normalize_color(" #AABBCC ").as_deref(), Some("#aabbcc"));
    // `null` is unrepresentable as &str (the Rust API takes a string).
    for color in [
        "#abc",
        "red",
        "url(https://example.com)",
        "#ffffff;display:none",
    ] {
        assert_eq!(normalize_color(color), None, "{color}");
    }
}

// port: test/javascript/lib/custom_theme.test.js:17 "applies only known color tokens and cleans up without touching layout"
#[test]
fn applies_only_known_color_tokens_and_cleans_up_without_touching_layout() {
    let applied = applicable_overrides(
        &json!({"accent": "#112233", "bg-primary": "url(secret)", "font-family": "#abcdef"}),
    );
    assert_eq!(applied, vec![("accent", "#112233".to_owned())]);

    let base = frankmd_theme::builtin("dark").unwrap();
    let custom = frankmd_theme::CustomTheme::from_value(&json!({
        "version": 1, "name": "x", "base": "dark", "colors": {"accent": "#112233"}
    }))
    .unwrap()
    .unwrap();
    let themed = base.with_overrides(&custom);
    assert_eq!(themed.get("accent").unwrap().to_hex(), "#112233");
    // Every other token keeps the base value ("clear" = base tokens again).
    for key in frankmd_theme::COLOR_KEYS.iter().filter(|k| **k != "accent") {
        assert_eq!(themed.get(key), base.get(key), "{key}");
    }
}

// port: test/javascript/lib/custom_theme.test.js:28 "computes contrast using linearized color channels"
#[test]
fn computes_contrast_using_linearized_color_channels() {
    assert_eq!(contrast_ratio_hex("#ffffff", "#000000"), Some(21.0));
    assert_eq!(contrast_ratio_hex("#123456", "#123456"), Some(1.0));
    let r = contrast_ratio_hex("#777777", "#ffffff").unwrap();
    assert!((r - 4.478).abs() < 0.005, "{r}");
    assert_eq!(contrast_ratio_hex("", "#ffffff"), None);
}

// port: test/javascript/lib/custom_theme.test.js:35 "expands minified base colors without allowing incomplete user hex input"
#[test]
fn expands_minified_base_colors_without_allowing_incomplete_user_hex_input() {
    assert_eq!(theme_color(" #fff ").as_deref(), Some("#ffffff"));
    assert_eq!(normalize_color("#fff"), None);
}
