//! Port of `test/controllers/config_controller_test.rb` (16 tests: 15 ported,
//! 1 skipped as Rails view plumbing).
//!
//! `GET /config` ↦ `ResolvedConfig::ui_settings()` + `feature_available`;
//! `PATCH /config` ↦ `ConfigStore::update_ui_settings` (UI_KEYS filter +
//! "No valid settings" error). The `/config/editor` HTML partial's data
//! attributes are just resolved config values, so those tests assert the values.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_config::{ConfigError, Feature, Key, Value};
use serde_json::json;

// === show ===

// port: test/controllers/config_controller_test.rb:16 "show returns UI settings and features"
#[test]
fn show_returns_ui_settings_and_features() {
    let rc = TestVault::new().open_empty_env().current();
    let settings = ui_settings_json(&rc);
    for k in [
        "theme", "editor_font", "editor_font_size", "preview_zoom", "sidebar_visible", "typewriter_mode",
        "show_stats", "cursor_recenter_on_click",
    ] {
        assert!(settings.contains_key(k), "settings missing {k}");
    }
    // features: s3_upload, youtube_search, google_search, local_images
    for f in [Feature::S3Upload, Feature::YoutubeSearch, Feature::GoogleSearch, Feature::LocalImages] {
        assert!(!rc.feature_available(f), "{f:?} must be off with nothing configured");
    }
}

// port: test/controllers/config_controller_test.rb:43 "show returns default values"
#[test]
fn show_returns_default_values() {
    let s = ui_settings_json(&TestVault::new().open_empty_env().current());
    assert_eq!(s["editor_font"], json!("cascadia-code"));
    assert_eq!(s["editor_font_size"], json!(14));
    assert_eq!(s["preview_zoom"], json!(100));
    assert_eq!(s["sidebar_visible"], json!(true));
    assert_eq!(s["typewriter_mode"], json!(false));
    assert_eq!(s["show_stats"], json!(false));
    assert_eq!(s["cursor_recenter_on_click"], json!(false));
}

// port: test/controllers/config_controller_test.rb:59 "show returns configured values from file"
#[test]
fn show_returns_configured_values_from_file() {
    let v = TestVault::with_fed(
        "theme = gruvbox\neditor_font = hack\ntypewriter_mode = true\nshow_stats = true\ncursor_recenter_on_click = true\n",
    );
    let s = ui_settings_json(&v.open_empty_env().current());
    assert_eq!(s["theme"], json!("gruvbox"));
    assert_eq!(s["editor_font"], json!("hack"));
    assert_eq!(s["typewriter_mode"], json!(true));
    assert_eq!(s["show_stats"], json!(true));
    assert_eq!(s["cursor_recenter_on_click"], json!(true));
}

// === update ===

// port: test/controllers/config_controller_test.rb:83 "media provider is exposed and persisted through configuration endpoints"
#[test]
fn media_provider_is_exposed_and_persisted_through_configuration_endpoints() {
    let v = TestVault::new();
    let mut store = v.open_empty_env();
    assert_eq!(ui_settings_json(&store.current())["media_storage_provider"], json!("local"));
    assert!(store.update_ui_settings(&[(Key::MediaStorageProvider, s("local"))]).is_ok());
    // `GET /images/config` exposes the same resolved value.
    assert_eq!(get_str(&v.open_empty_env().current(), Key::MediaStorageProvider).as_deref(), Some("local"));
}

// port: test/controllers/config_controller_test.rb:93 "media provider rejects unknown providers and unconfigured S3"
#[test]
fn media_provider_rejects_unknown_providers_and_unconfigured_s3() {
    let v = TestVault::new();
    let mut store = v.open_empty_env();
    let err = store.update_ui_settings(&[(Key::MediaStorageProvider, s("unknown"))]).unwrap_err();
    assert!(matches!(err, ConfigError::UnknownMediaProvider), "{err:?}");
    assert_eq!(get_str(&v.open_empty_env().current(), Key::MediaStorageProvider).as_deref(), Some("local"));
    let err = store.update_ui_settings(&[(Key::MediaStorageProvider, s("s3"))]).unwrap_err();
    assert!(matches!(err, ConfigError::S3NotConfigured), "{err:?}");
    assert_eq!(get_str(&v.open_empty_env().current(), Key::MediaStorageProvider).as_deref(), Some("local"));
}

// port: test/controllers/config_controller_test.rb:102 "update saves UI settings"
#[test]
fn update_saves_ui_settings() {
    let v = TestVault::new();
    let rc = v
        .open_empty_env()
        .update_ui_settings(&[(Key::Theme, s("dark")), (Key::EditorFontSize, Some(Value::Int(18)))])
        .unwrap();
    let s1 = ui_settings_json(&rc);
    assert_eq!(s1["theme"], json!("dark"));
    assert_eq!(s1["editor_font_size"], json!(18));
    let s2 = ui_settings_json(&v.open_empty_env().current());
    assert_eq!(s2["theme"], json!("dark"));
    assert_eq!(s2["editor_font_size"], json!(18));
}

// port: test/controllers/config_controller_test.rb:117 "update rejects non-UI settings"
#[test]
fn update_rejects_non_ui_settings() {
    let v = TestVault::new();
    let mut store = v.open_empty_env();
    let err = store.update_ui_settings(&[(Key::AwsAccessKeyId, s("hack-attempt"))]).unwrap_err();
    assert!(matches!(err, ConfigError::NoValidSettings), "{err:?}");
    assert!(!v.read_fed().contains("hack-attempt"));
}

// port: test/controllers/config_controller_test.rb:126 "update handles partial updates"
#[test]
fn update_handles_partial_updates() {
    let v = TestVault::new();
    v.open_empty_env().update_ui_settings(&[(Key::Theme, s("dark"))]).unwrap();
    v.open_empty_env().update_ui_settings(&[(Key::EditorFont, s("hack"))]).unwrap();
    let s = ui_settings_json(&v.open_empty_env().current());
    assert_eq!(s["theme"], json!("dark"));
    assert_eq!(s["editor_font"], json!("hack"));
}

// port: test/controllers/config_controller_test.rb:141 "update saves locale setting and returns updated settings"
#[test]
fn update_saves_locale_setting_and_returns_updated_settings() {
    let v = TestVault::new();
    let rc = v.open_empty_env().update_ui_settings(&[(Key::Locale, s("pt-BR"))]).unwrap();
    assert_eq!(ui_settings_json(&rc)["locale"], json!("pt-BR"));
    assert_eq!(ui_settings_json(&v.open_empty_env().current())["locale"], json!("pt-BR"));
}

// port: test/controllers/config_controller_test.rb:154 "update returns error for empty params"
#[test]
fn update_returns_error_for_empty_params() {
    let err = TestVault::new().open_empty_env().update_ui_settings(&[]).unwrap_err();
    assert!(matches!(err, ConfigError::NoValidSettings));
    assert!(err.to_string().contains("No valid settings"));
}

// port: test/controllers/config_controller_test.rb:162 "update handles boolean values"
#[test]
fn update_handles_boolean_values() {
    let rc = TestVault::new()
        .open_empty_env()
        .update_ui_settings(&[
            (Key::TypewriterMode, Some(Value::Bool(true))),
            (Key::SidebarVisible, Some(Value::Bool(false))),
            (Key::CursorRecenterOnClick, Some(Value::Bool(true))),
            (Key::ShowStats, Some(Value::Bool(true))),
        ])
        .unwrap();
    let s = ui_settings_json(&rc);
    assert_eq!(s["typewriter_mode"], json!(true));
    assert_eq!(s["sidebar_visible"], json!(false));
    assert_eq!(s["cursor_recenter_on_click"], json!(true));
    assert_eq!(s["show_stats"], json!(true));
}

// port: test/controllers/config_controller_test.rb:173 "update writes font settings to file"
#[test]
fn update_writes_font_settings_to_file() {
    let v = TestVault::new();
    v.open_empty_env()
        .update_ui_settings(&[(Key::EditorFont, s("hack")), (Key::EditorFontSize, Some(Value::Int(20)))])
        .unwrap();
    let content = v.read_fed();
    assert!(content.contains("editor_font = hack"));
    assert!(content.contains("editor_font_size = 20"));
}

// port: test/controllers/config_controller_test.rb:183 "update preserves existing file content when adding new settings"
#[test]
fn update_preserves_existing_file_content_when_adding_new_settings() {
    let v = TestVault::with_fed("theme = tokyo-night\n");
    v.open_empty_env().update_ui_settings(&[(Key::EditorFont, s("fira-code"))]).unwrap();
    let content = v.read_fed();
    assert!(content.contains("theme = tokyo-night"));
    assert!(content.contains("editor_font = fira-code"));
}

// === editor partial ===

// skip: test/controllers/config_controller_test.rb:200 "editor returns HTML partial with config data attributes"
//       [P] Rails HTML partial markup (`#editor-config`, `data-controller`); no native equivalent.

// port: test/controllers/config_controller_test.rb:213 "editor partial reflects configured values"
#[test]
fn editor_partial_reflects_configured_values() {
    let rc = TestVault::with_fed("editor_font = hack\neditor_font_size = 20\ntheme = gruvbox\n").open_empty_env().current();
    assert_eq!(get_str(&rc, Key::EditorFont).as_deref(), Some("hack"));
    assert_eq!(get_int(&rc, Key::EditorFontSize), Some(20));
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("gruvbox"));
}

// port: test/controllers/config_controller_test.rb:228 "editor partial uses defaults when no config file"
#[test]
fn editor_partial_uses_defaults_when_no_config_file() {
    let rc = TestVault::new().open_empty_env().current();
    assert_eq!(get_str(&rc, Key::EditorFont).as_deref(), Some("cascadia-code"));
    assert_eq!(get_int(&rc, Key::EditorFontSize), Some(14));
    assert_eq!(get_int(&rc, Key::EditorWidth), Some(72));
    assert_eq!(get_int(&rc, Key::PreviewZoom), Some(100));
}
