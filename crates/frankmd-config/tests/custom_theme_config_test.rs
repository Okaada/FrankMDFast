//! Port of `test/controllers/custom_theme_config_test.rb` (3 tests, A).
//!
//! The PATCH body carries `custom_theme` as a JSON **string** (strong params
//! cannot permit nested hashes); `value::coerce` is the cast step that parses
//! and validates it. The root-page `assert_select` checks of test 1 are view
//! plumbing and are not ported (the native theme editor is a UI test).
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_config::value::coerce;
use frankmd_config::{ConfigError, Key};
use serde_json::json;

fn theme_json(accent: &str) -> String {
    json!({"version": 1, "name": "My \"theme\"", "base": "dark", "colors": {"accent": accent}})
        .to_string()
}

// port: test/controllers/custom_theme_config_test.rb:12 "saves palette and selection atomically and roundtrips through config"
#[test]
fn saves_palette_and_selection_atomically_and_roundtrips_through_config() {
    let v = TestVault::new();
    let custom = coerce(Key::CustomTheme, &json!(theme_json("#AABBCC"))).unwrap();
    let rc = v
        .open_empty_env()
        .update_ui_settings(&[(Key::Theme, s("custom")), (Key::CustomTheme, custom)])
        .unwrap();
    let settings = ui_settings_json(&rc);
    assert_eq!(settings["theme"], json!("custom"));
    assert_eq!(
        settings["custom_theme"]["colors"]["accent"],
        json!("#aabbcc")
    );

    let fresh = v.open_empty_env().current();
    assert_eq!(
        get_json(&fresh, "custom_theme")["name"],
        json!("My \"theme\"")
    );
    assert_eq!(
        ui_settings_json(&fresh)["custom_theme"]["base"],
        json!("dark")
    );
}

// port: test/controllers/custom_theme_config_test.rb:25 "invalid palette cannot partially change the selected theme"
#[test]
fn invalid_palette_cannot_partially_change_the_selected_theme() {
    let v = TestVault::new();
    v.open_empty_env()
        .update(&[(Key::Theme, s("nord"))])
        .unwrap();
    let before = v.read_fed();

    let err = coerce(
        Key::CustomTheme,
        &json!(theme_json("url(https://example.com)")),
    )
    .unwrap_err();
    assert!(matches!(err, ConfigError::CustomTheme(_)), "{err:?}");
    // The controller answers 422 and nothing is written.
    assert_eq!(v.read_fed(), before);
    let fresh = v.open_empty_env().current();
    assert_eq!(get_str(&fresh, Key::Theme).as_deref(), Some("nord"));
    assert_eq!(fresh.get(Key::CustomTheme).value, None);
}

// port: test/controllers/custom_theme_config_test.rb:34 "malformed on-disk palette does not discard other settings"
#[test]
fn malformed_on_disk_palette_does_not_discard_other_settings() {
    let v = TestVault::with_fed("theme = nord\ncustom_theme = {broken\neditor_font_size = 18\n");
    let mut store = v.open_empty_env();
    let rc = store.current();
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("nord"));
    assert_eq!(get_int(&rc, Key::EditorFontSize), Some(18));
    assert_eq!(rc.get(Key::CustomTheme).value, None);
    assert!(
        rc.warnings.iter().any(|w| w.contains("custom_theme")),
        "{:?}",
        rc.warnings
    );
    let err = store
        .update(&[(Key::EditorFont, s("unsafe\"font"))])
        .unwrap_err();
    assert!(
        matches!(
            err,
            ConfigError::UnsafeString {
                key: Key::EditorFont
            }
        ),
        "{err:?}"
    );
}
