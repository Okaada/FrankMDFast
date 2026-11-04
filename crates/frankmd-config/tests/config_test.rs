//! Port of `test/models/config_test.rb` (67 tests, all category A).
//!
//! `Config.new(base_path: dir)` ↦ `ConfigStore::open(Vault::open(dir, state), env)`;
//! `config.get(k)` ↦ `store.current().get(k).value`; `set`/`update` ↦
//! `store.update(..).is_ok()`. The Ruby setup deletes the AI/S3/YouTube/Google
//! ENV vars; here every test starts from an empty `EnvSnapshot` and adds only
//! what the original test sets.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_config::schema::KeyType;
use frankmd_config::{AiProviderId, Feature, Key, Value};
use serde_json::json;

const S3_ENV: [(&str, &str); 3] = [
    ("AWS_ACCESS_KEY_ID", "key"),
    ("AWS_SECRET_ACCESS_KEY", "secret"),
    ("AWS_S3_BUCKET", "bucket"),
];

// === Initialization ===

// port: test/models/config_test.rb:67 "media storage defaults to local even with S3 configured"
#[test]
fn media_storage_defaults_to_local_even_with_s3_configured() {
    let v = TestVault::new();
    let store = v.open(env(&S3_ENV));
    assert_eq!(
        get_str(&store.current(), Key::MediaStorageProvider).as_deref(),
        Some("local")
    );
}

// port: test/models/config_test.rb:75 "media storage rejects invalid or unavailable providers without changing settings"
#[test]
fn media_storage_rejects_invalid_or_unavailable_providers_without_changing_settings() {
    let v = TestVault::new();
    let mut store = v.open_empty_env();
    assert!(
        store
            .update(&[(Key::MediaStorageProvider, s("unknown"))])
            .is_err()
    );
    assert!(
        store
            .update(&[
                (Key::MediaStorageProvider, s("s3")),
                (Key::Theme, s("dark"))
            ])
            .is_err()
    );
    let fresh = v.open_empty_env().current();
    assert_eq!(
        get_str(&fresh, Key::MediaStorageProvider).as_deref(),
        Some("local")
    );
    assert_eq!(fresh.get(Key::Theme).value, None);
}

// port: test/models/config_test.rb:83 "media storage persists and retains S3 selection after credentials disappear"
#[test]
fn media_storage_persists_and_retains_s3_selection_after_credentials_disappear() {
    let v = TestVault::new();
    assert!(
        v.open(env(&S3_ENV))
            .update(&[(Key::MediaStorageProvider, s("s3"))])
            .is_ok()
    );
    let ui = ui_settings_json(&v.open(env(&S3_ENV)).current());
    assert_eq!(ui["media_storage_provider"], json!("s3"));

    let without_key = env(&S3_ENV[1..]);
    assert_eq!(
        get_str(
            &v.open(without_key.clone()).current(),
            Key::MediaStorageProvider
        )
        .as_deref(),
        Some("s3")
    );
    assert!(
        v.open(without_key)
            .update(&[(Key::MediaStorageProvider, s("local"))])
            .is_ok()
    );
}

// port: test/models/config_test.rb:95 "creates config file on initialization if it does not exist"
#[test]
fn creates_config_file_on_initialization_if_it_does_not_exist() {
    let v = TestVault::new();
    let _store = v.open_empty_env();
    assert!(v.fed_exists());
    assert_eq!(v.mode(FED), 0o600);
}

// port: test/models/config_test.rb:102 "creates config file and parent directory when neither exist"
#[test]
fn creates_config_file_and_parent_directory_when_neither_exist() {
    let v = TestVault::new().at("nested/deeply/path");
    assert!(!v.root.exists());
    let _store = v.open_empty_env();
    assert!(v.root.is_dir(), "Parent directory should be created");
    assert!(v.fed_exists(), ".fed file should be created");
}

// port: test/models/config_test.rb:113 "config file template contains all sections"
#[test]
fn config_file_template_contains_all_sections() {
    let v = TestVault::new();
    let _store = v.open_empty_env();
    let content = v.read_fed();
    for needle in [
        "# UI Settings",
        "# theme =",
        "# editor_font =",
        "# AWS S3",
        "# YouTube API",
        "# Google Custom Search",
        "# AI/LLM",
        "# ollama_api_base",
        "# anthropic_api_key",
        "# gemini_api_key",
    ] {
        assert!(content.contains(needle), "template is missing {needle:?}");
    }
}

// === Default Values ===

// port: test/models/config_test.rb:131 "returns default values when nothing is configured"
#[test]
fn returns_default_values_when_nothing_is_configured() {
    let rc = TestVault::new().open_empty_env().current();
    assert_eq!(
        get_str(&rc, Key::EditorFont).as_deref(),
        Some("cascadia-code")
    );
    assert_eq!(rc.get(Key::EditorFontSize).value, Some(Value::Int(14)));
    assert_eq!(get_int(&rc, Key::PreviewZoom), Some(100));
    assert_eq!(get_bool(&rc, Key::SidebarVisible), Some(true));
    assert_eq!(get_bool(&rc, Key::TypewriterMode), Some(false));
    assert_eq!(get_bool(&rc, Key::ShowStats), Some(false));
    assert_eq!(get_bool(&rc, Key::CursorRecenterOnClick), Some(false));
    assert_eq!(get_bool(&rc, Key::VimMode), Some(false));
    assert_eq!(rc.get(Key::Theme).value, None);
}

fn assert_ui_bool_default(k: Key, default: &str) {
    assert!(Key::ui_keys().contains(&k), "{k:?} must be in UI_KEYS");
    assert!(k.spec().ui);
    assert_eq!(k.spec().ty, KeyType::Boolean);
    assert_eq!(k.spec().default, Some(default));
}

// port: test/models/config_test.rb:145 "vim_mode is a UI setting persisted from the frontend"
#[test]
fn vim_mode_is_a_ui_setting_persisted_from_the_frontend() {
    assert_ui_bool_default(Key::VimMode, "false");
}

// port: test/models/config_test.rb:151 "show_stats is disabled by default and persists in the config file"
#[test]
fn show_stats_is_disabled_by_default_and_persists_in_the_config_file() {
    assert!(Key::ui_keys().contains(&Key::ShowStats));
    assert_eq!(Key::ShowStats.spec().ty, KeyType::Boolean);
    let v = TestVault::new();
    let mut store = v.open_empty_env();
    assert_eq!(get_bool(&store.current(), Key::ShowStats), Some(false));
    assert!(
        store
            .update(&[(Key::ShowStats, Some(Value::Bool(true)))])
            .is_ok()
    );
    assert_eq!(
        get_bool(&v.open_empty_env().current(), Key::ShowStats),
        Some(true)
    );
}

// port: test/models/config_test.rb:160 "reads vim_mode from the config file"
#[test]
fn reads_vim_mode_from_the_config_file() {
    let rc = TestVault::with_fed("vim_mode = true\n")
        .open_empty_env()
        .current();
    assert_eq!(get_bool(&rc, Key::VimMode), Some(true));
}

// port: test/models/config_test.rb:166 "cursor_recenter_on_click is a UI setting that defaults to false"
#[test]
fn cursor_recenter_on_click_is_a_ui_setting_that_defaults_to_false() {
    assert_ui_bool_default(Key::CursorRecenterOnClick, "false");
}

// port: test/models/config_test.rb:172 "reads cursor_recenter_on_click from the config file"
#[test]
fn reads_cursor_recenter_on_click_from_the_config_file() {
    let rc = TestVault::with_fed("cursor_recenter_on_click = true\n")
        .open_empty_env()
        .current();
    assert_eq!(get_bool(&rc, Key::CursorRecenterOnClick), Some(true));
}

// port: test/models/config_test.rb:178 "scroll_sync is a UI setting that defaults to true"
#[test]
fn scroll_sync_is_a_ui_setting_that_defaults_to_true() {
    assert_ui_bool_default(Key::ScrollSync, "true");
}

// port: test/models/config_test.rb:184 "reads scroll_sync from the config file"
#[test]
fn reads_scroll_sync_from_the_config_file() {
    let rc = TestVault::with_fed("scroll_sync = false\n")
        .open_empty_env()
        .current();
    assert_eq!(get_bool(&rc, Key::ScrollSync), Some(false));
}

// port: test/models/config_test.rb:190 "view_mode is a UI setting that defaults to split"
#[test]
fn view_mode_is_a_ui_setting_that_defaults_to_split() {
    assert!(Key::ui_keys().contains(&Key::ViewMode));
    assert_eq!(Key::ViewMode.spec().ty, KeyType::String);
    assert_eq!(Key::ViewMode.spec().default, Some("split"));
}

// port: test/models/config_test.rb:196 "reads view_mode from the config file"
#[test]
fn reads_view_mode_from_the_config_file() {
    let rc = TestVault::with_fed("view_mode = single\n")
        .open_empty_env()
        .current();
    assert_eq!(get_str(&rc, Key::ViewMode).as_deref(), Some("single"));
}

// === Upload extensions ===

// port: test/models/config_test.rb:204 "upload_extensions returns the default image list"
#[test]
fn upload_extensions_returns_the_default_image_list() {
    let exts = TestVault::new()
        .open_empty_env()
        .current()
        .upload_extensions(Key::ImageUploadExtensions);
    assert!(exts.contains(&".png".to_owned()));
    assert!(exts.contains(&".jpg".to_owned()));
}

// port: test/models/config_test.rb:212 "upload_extensions returns the default video list"
#[test]
fn upload_extensions_returns_the_default_video_list() {
    let exts = TestVault::new()
        .open_empty_env()
        .current()
        .upload_extensions(Key::VideoUploadExtensions);
    assert!(exts.contains(&".mp4".to_owned()));
    assert!(exts.contains(&".webm".to_owned()));
}

// port: test/models/config_test.rb:220 "upload_extensions normalizes whitespace and case"
#[test]
fn upload_extensions_normalizes_whitespace_and_case() {
    let store = TestVault::new().open(env(&[("VIDEO_UPLOAD_EXTENSIONS", " .MP4 , .MOV ")]));
    assert_eq!(
        store
            .current()
            .upload_extensions(Key::VideoUploadExtensions),
        vec![".mp4", ".mov"]
    );
}

// === Reading from File ===

// port: test/models/config_test.rb:231 "reads values from config file"
#[test]
fn reads_values_from_config_file() {
    let rc =
        TestVault::with_fed("theme = gruvbox\neditor_font = fira-code\neditor_font_size = 16\n")
            .open_empty_env()
            .current();
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("gruvbox"));
    assert_eq!(get_str(&rc, Key::EditorFont).as_deref(), Some("fira-code"));
    assert_eq!(get_int(&rc, Key::EditorFontSize), Some(16));
}

// port: test/models/config_test.rb:245 "reads boolean values correctly"
#[test]
fn reads_boolean_values_correctly() {
    let rc = TestVault::with_fed("typewriter_mode = true\nsidebar_visible = false\n")
        .open_empty_env()
        .current();
    assert_eq!(get_bool(&rc, Key::TypewriterMode), Some(true));
    assert_eq!(get_bool(&rc, Key::SidebarVisible), Some(false));
}

// port: test/models/config_test.rb:257 "handles quoted string values"
#[test]
fn handles_quoted_string_values() {
    let rc = TestVault::with_fed("theme = \"tokyo-night\"\neditor_font = 'jetbrains-mono'\n")
        .open_empty_env()
        .current();
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("tokyo-night"));
    assert_eq!(
        get_str(&rc, Key::EditorFont).as_deref(),
        Some("jetbrains-mono")
    );
}

// port: test/models/config_test.rb:269 "ignores comments"
#[test]
fn ignores_comments() {
    let rc = TestVault::with_fed(
        "# This is a comment\ntheme = dark\n# editor_font = should-be-ignored\n",
    )
    .open_empty_env()
    .current();
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("dark"));
    assert_eq!(
        get_str(&rc, Key::EditorFont).as_deref(),
        Some("cascadia-code")
    );
}

// port: test/models/config_test.rb:282 "ignores unknown keys"
#[test]
fn ignores_unknown_keys() {
    let rc = TestVault::with_fed("unknown_key = some_value\ntheme = light\n")
        .open_empty_env()
        .current();
    // `get(:unknown_key)` is unrepresentable with a typed Key; the equivalent
    // contract is that the name is not a key and the line is dropped.
    assert_eq!(Key::from_name("unknown_key"), None);
    assert_eq!(file_values_json(&rc), json!({"theme": "light"}));
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("light"));
}

// === ENV Fallback ===

// port: test/models/config_test.rb:296 "falls back to ENV for keys with ENV mapping"
#[test]
fn falls_back_to_env_for_keys_with_env_mapping() {
    let store = TestVault::new().open(env(&[("YOUTUBE_API_KEY", "test-youtube-key")]));
    assert_eq!(
        get_str(&store.current(), Key::YoutubeApiKey).as_deref(),
        Some("test-youtube-key")
    );
}

// port: test/models/config_test.rb:306 "file value overrides ENV value"
#[test]
fn file_value_overrides_env_value() {
    let store = TestVault::with_fed("youtube_api_key = file-key\n")
        .open(env(&[("YOUTUBE_API_KEY", "env-key")]));
    assert_eq!(
        get_str(&store.current(), Key::YoutubeApiKey).as_deref(),
        Some("file-key")
    );
}

// === Writing Values ===

// port: test/models/config_test.rb:321 "set saves a single value"
#[test]
fn set_saves_a_single_value() {
    let v = TestVault::new();
    v.open_empty_env()
        .update(&[(Key::Theme, s("gruvbox"))])
        .unwrap();
    assert_eq!(
        get_str(&v.open_empty_env().current(), Key::Theme).as_deref(),
        Some("gruvbox")
    );
}

// port: test/models/config_test.rb:331 "update saves multiple values"
#[test]
fn update_saves_multiple_values() {
    let v = TestVault::new();
    v.open_empty_env()
        .update(&[
            (Key::Theme, s("dark")),
            (Key::EditorFontSize, Some(Value::Int(18))),
        ])
        .unwrap();
    let rc = v.open_empty_env().current();
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("dark"));
    assert_eq!(get_int(&rc, Key::EditorFontSize), Some(18));
}

// port: test/models/config_test.rb:341 "update replaces commented line with actual value"
#[test]
fn update_replaces_commented_line_with_actual_value() {
    let v = TestVault::new();
    let mut store = v.open_empty_env();
    assert!(v.read_fed().contains("# theme ="));
    store.update(&[(Key::Theme, s("nord"))]).unwrap();
    let content = v.read_fed();
    assert!(content.contains("theme = nord"));
    assert!(!content.contains("# theme = nord"));
}

// port: test/models/config_test.rb:353 "update does not duplicate keys"
#[test]
fn update_does_not_duplicate_keys() {
    let v = TestVault::new();
    let mut store = v.open_empty_env();
    for t in ["dark", "light", "gruvbox"] {
        store.update(&[(Key::Theme, s(t))]).unwrap();
    }
    assert_eq!(
        v.read_fed()
            .lines()
            .filter(|l| l.starts_with("theme = "))
            .count(),
        1
    );
}

// port: test/models/config_test.rb:364 "preserves user-customized file structure when updating"
#[test]
fn preserves_user_customized_file_structure_when_updating() {
    let v = TestVault::with_fed(&format!(
        "editor_font = hack\ntheme = dark\neditor_font_size = 16\n{AI_MARKER}"
    ));
    v.open_empty_env()
        .update(&[(Key::Theme, s("gruvbox"))])
        .unwrap();
    assert_eq!(
        nonblank_lines(&v.read_fed()),
        [
            "editor_font = hack",
            "theme = gruvbox",
            "editor_font_size = 16",
            "# ai_model = placeholder"
        ]
    );
}

// port: test/models/config_test.rb:388 "does not re-add values user manually removed"
#[test]
fn does_not_re_add_values_user_manually_removed() {
    let v = TestVault::with_fed(&format!(
        "theme = dark\neditor_font = hack\neditor_font_size = 18\n{AI_MARKER}"
    ));
    let rc = v.open_empty_env().current();
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("dark"));
    assert_eq!(get_str(&rc, Key::EditorFont).as_deref(), Some("hack"));
    assert_eq!(get_int(&rc, Key::EditorFontSize), Some(18));

    v.write_fed(format!("theme = dark\neditor_font_size = 18\n{AI_MARKER}").as_bytes());
    v.open_empty_env()
        .update(&[(Key::Theme, s("nord"))])
        .unwrap();
    let content = v.read_fed();
    assert!(!content.contains("editor_font = hack"));
    assert!(!content.contains("editor_font = cascadia"));
    assert!(content.contains("theme = nord"));
    assert!(content.contains("editor_font_size = 18"));
}

// port: test/models/config_test.rb:422 "only modifies the specific key being changed"
#[test]
fn only_modifies_the_specific_key_being_changed() {
    let v = TestVault::with_fed(
        "# My custom comment\ntheme = dark\n\n# Another section\neditor_font = fira-code\neditor_font_size = 20\n",
    );
    v.open_empty_env()
        .update(&[(Key::EditorFontSize, Some(Value::Int(22)))])
        .unwrap();
    let content = v.read_fed();
    assert!(content.contains("# My custom comment"));
    assert!(content.contains("# Another section"));
    assert!(content.contains("theme = dark"));
    assert!(content.contains("editor_font = fira-code"));
    assert!(content.contains("editor_font_size = 22"));
    assert!(!content.contains("editor_font_size = 20"));
}

// port: test/models/config_test.rb:449 "appends new key at end if not present in file"
#[test]
fn appends_new_key_at_end_if_not_present_in_file() {
    let v = TestVault::with_fed(&format!("theme = dark\n{AI_MARKER}"));
    v.open_empty_env()
        .update(&[(Key::EditorFont, s("hack"))])
        .unwrap();
    assert_eq!(
        nonblank_lines(&v.read_fed()),
        [
            "theme = dark",
            "# ai_model = placeholder",
            "editor_font = hack"
        ]
    );
}

// port: test/models/config_test.rb:467 "update rejects string values that could inject config lines"
#[test]
fn update_rejects_string_values_that_could_inject_config_lines() {
    let v = TestVault::with_fed("theme = dark\n");
    let mut store = v.open_empty_env();
    let original = v.read_fed();
    assert!(
        store
            .update(&[(Key::Theme, s("light\"\nimages_path = /outside"))])
            .is_err()
    );
    assert_eq!(v.read_fed(), original);
    assert!(!v.read_fed().contains("images_path = /outside"));
}

// port: test/models/config_test.rb:478 "update atomically persists valid values"
#[test]
fn update_atomically_persists_valid_values() {
    let v = TestVault::new();
    assert!(
        v.open_empty_env()
            .update(&[(Key::Theme, s("dark"))])
            .is_ok()
    );
    let line = v
        .read_fed()
        .lines()
        .find(|l| l.starts_with("theme ="))
        .map(str::trim)
        .map(str::to_owned);
    assert_eq!(line.as_deref(), Some("theme = dark"));
}

// port: test/models/config_test.rb:485 "update preserves a 0600 config file mode"
#[test]
fn update_preserves_a_0600_config_file_mode() {
    let v = TestVault::with_fed("theme = light\n");
    v.chmod(FED, 0o600);
    assert!(
        v.open_empty_env()
            .update(&[(Key::Theme, s("dark"))])
            .is_ok()
    );
    assert_eq!(v.mode(FED), 0o600);
}

// port: test/models/config_test.rb:495 "update replaces config files with mode 0600"
#[test]
fn update_replaces_config_files_with_mode_0600() {
    let v = TestVault::new();
    assert!(
        v.open_empty_env()
            .update(&[(Key::Theme, s("dark"))])
            .is_ok()
    );
    assert_eq!(v.mode(FED), 0o600);
}

// === UI Settings ===

// port: test/models/config_test.rb:504 "ui_settings returns only UI keys"
#[test]
fn ui_settings_returns_only_ui_keys() {
    let rc = TestVault::with_fed("theme = dark\neditor_font = hack\nyoutube_api_key = secret\n")
        .open_empty_env()
        .current();
    let ui = ui_settings_json(&rc);
    assert_eq!(ui["theme"], json!("dark"));
    assert_eq!(ui["editor_font"], json!("hack"));
    assert!(!ui.contains_key("youtube_api_key"));
}

// === Feature Detection ===

// port: test/models/config_test.rb:521 "feature_available? detects S3 configuration"
#[test]
fn feature_available_detects_s3_configuration() {
    let v = TestVault::new();
    assert!(
        !v.open_empty_env()
            .current()
            .feature_available(Feature::S3Upload)
    );
    v.write_fed(
        b"aws_access_key_id = key\naws_secret_access_key = secret\naws_s3_bucket = bucket\n",
    );
    assert!(
        v.open_empty_env()
            .current()
            .feature_available(Feature::S3Upload)
    );
}

// port: test/models/config_test.rb:535 "feature_available? detects YouTube configuration"
#[test]
fn feature_available_detects_youtube_configuration() {
    let v = TestVault::new();
    assert!(
        !v.open_empty_env()
            .current()
            .feature_available(Feature::YoutubeSearch)
    );
    v.write_fed(b"youtube_api_key = test-key\n");
    assert!(
        v.open_empty_env()
            .current()
            .feature_available(Feature::YoutubeSearch)
    );
}

// port: test/models/config_test.rb:547 "feature_available? detects Google search configuration"
#[test]
fn feature_available_detects_google_search_configuration() {
    let v = TestVault::new();
    assert!(
        !v.open_empty_env()
            .current()
            .feature_available(Feature::GoogleSearch)
    );
    v.write_fed(b"google_api_key = api-key\ngoogle_cse_id = cse-id\n");
    assert!(
        v.open_empty_env()
            .current()
            .feature_available(Feature::GoogleSearch)
    );
}

// === Corrupted File Handling ===

// port: test/models/config_test.rb:562 "handles empty config file gracefully"
#[test]
fn handles_empty_config_file_gracefully() {
    let rc = TestVault::with_fed("").open_empty_env().current();
    assert_eq!(
        get_str(&rc, Key::EditorFont).as_deref(),
        Some("cascadia-code")
    );
}

// port: test/models/config_test.rb:570 "handles malformed lines gracefully"
#[test]
fn handles_malformed_lines_gracefully() {
    let rc = TestVault::with_fed(
        "this is not valid\n= no key\ntheme = dark\neditor_font_size not an int\n",
    )
    .open_empty_env()
    .current();
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("dark"));
    assert_eq!(get_int(&rc, Key::EditorFontSize), Some(14));
}

// port: test/models/config_test.rb:586 "handles binary garbage in file"
#[test]
fn handles_binary_garbage_in_file() {
    let v = TestVault::new();
    v.write_fed(b"\x00\xFF\xFE\x00theme = dark\n");
    let rc = v.open_empty_env().current();
    // "May or may not parse correctly, but should not crash."
    let _ = rc.get(Key::Theme);
}

// port: test/models/config_test.rb:596 "handles file read errors gracefully"
#[test]
fn handles_file_read_errors_gracefully() {
    let v = TestVault::from_spec(&json!({".fed/": null}));
    let rc = v.open_empty_env().current();
    assert_eq!(
        get_str(&rc, Key::EditorFont).as_deref(),
        Some("cascadia-code")
    );
}

// === Type Casting ===

// port: test/models/config_test.rb:610 "casts integer values"
#[test]
fn casts_integer_values() {
    let rc = TestVault::with_fed("editor_font_size = 20")
        .open_empty_env()
        .current();
    assert_eq!(rc.get(Key::EditorFontSize).value, Some(Value::Int(20)));
}

// port: test/models/config_test.rb:619 "casts boolean true values"
#[test]
fn casts_boolean_true_values() {
    for value in ["true", "1", "yes", "on", "TRUE", "Yes"] {
        let rc = TestVault::with_fed(&format!("typewriter_mode = {value}"))
            .open_empty_env()
            .current();
        assert_eq!(
            get_bool(&rc, Key::TypewriterMode),
            Some(true),
            "Expected '{value}' to be true"
        );
    }
}

// port: test/models/config_test.rb:627 "casts boolean false values"
#[test]
fn casts_boolean_false_values() {
    for value in ["false", "0", "no", "off", "FALSE", "anything"] {
        let rc = TestVault::with_fed(&format!("typewriter_mode = {value}"))
            .open_empty_env()
            .current();
        assert_eq!(
            get_bool(&rc, Key::TypewriterMode),
            Some(false),
            "Expected '{value}' to be false"
        );
    }
}

// === AI Provider Tests ===

// port: test/models/config_test.rb:637 "ai_providers_available returns empty when nothing configured"
#[test]
fn ai_providers_available_returns_empty_when_nothing_configured() {
    assert!(
        TestVault::new()
            .open_empty_env()
            .current()
            .ai_providers_available()
            .is_empty()
    );
}

// port: test/models/config_test.rb:642 "ai_providers_available returns configured providers"
#[test]
fn ai_providers_available_returns_configured_providers() {
    let store = TestVault::new().open(env(&[
        ("OPENAI_API_KEY", "sk-test"),
        ("ANTHROPIC_API_KEY", "sk-ant-test"),
    ]));
    let providers = store.current().ai_providers_available();
    assert!(providers.contains(&AiProviderId::Openai));
    assert!(providers.contains(&AiProviderId::Anthropic));
    assert!(!providers.contains(&AiProviderId::Ollama));
    assert!(!providers.contains(&AiProviderId::Gemini));
}

// port: test/models/config_test.rb:655 "effective_ai_provider follows priority order"
#[test]
fn effective_ai_provider_follows_priority_order() {
    let store = TestVault::new().open(env(&[
        ("OLLAMA_API_BASE", "http://localhost:11434"),
        ("OPENROUTER_API_KEY", "sk-or-test"),
        ("ANTHROPIC_API_KEY", "sk-ant-test"),
        ("OPENAI_API_KEY", "sk-test"),
    ]));
    assert_eq!(
        store.current().effective_ai_provider(),
        Some(AiProviderId::Openai)
    );
}

// port: test/models/config_test.rb:667 "effective_ai_provider respects explicit provider setting"
#[test]
fn effective_ai_provider_respects_explicit_provider_setting() {
    let store = TestVault::with_fed("ai_provider = openai").open(env(&[
        ("OLLAMA_API_BASE", "http://localhost:11434"),
        ("OPENAI_API_KEY", "sk-test"),
    ]));
    assert_eq!(
        store.current().effective_ai_provider(),
        Some(AiProviderId::Openai)
    );
}

// port: test/models/config_test.rb:677 "effective_ai_provider ignores unavailable provider"
#[test]
fn effective_ai_provider_ignores_unavailable_provider() {
    let store = TestVault::with_fed("ai_provider = openai")
        .open(env(&[("OLLAMA_API_BASE", "http://localhost:11434")]));
    assert_eq!(
        store.current().effective_ai_provider(),
        Some(AiProviderId::Ollama)
    );
}

// port: test/models/config_test.rb:686 "effective_ai_model returns provider-specific default"
#[test]
fn effective_ai_model_returns_provider_specific_default() {
    let store = TestVault::new().open(env(&[("ANTHROPIC_API_KEY", "sk-ant-test")]));
    assert_eq!(
        store.current().effective_ai_model().as_deref(),
        Some("claude-sonnet-4-20250514")
    );
}

// port: test/models/config_test.rb:693 "effective_ai_model respects global ai_model override"
#[test]
fn effective_ai_model_respects_global_ai_model_override() {
    let store =
        TestVault::with_fed("ai_model = gpt-4-turbo").open(env(&[("OPENAI_API_KEY", "sk-test")]));
    assert_eq!(
        store.current().effective_ai_model().as_deref(),
        Some("gpt-4-turbo")
    );
}

// port: test/models/config_test.rb:702 "effective_ai_model respects provider-specific model"
#[test]
fn effective_ai_model_respects_provider_specific_model() {
    let store = TestVault::with_fed("anthropic_model = claude-3-opus-20240229")
        .open(env(&[("ANTHROPIC_API_KEY", "sk-ant-test")]));
    assert_eq!(
        store.current().effective_ai_model().as_deref(),
        Some("claude-3-opus-20240229")
    );
}

// port: test/models/config_test.rb:711 "feature_available? returns true for ai when any provider configured"
#[test]
fn feature_available_returns_true_for_ai_when_any_provider_configured() {
    let store = TestVault::new().open(env(&[("GEMINI_API_KEY", "gemini-test")]));
    assert!(store.current().feature_available(Feature::Ai));
}

// port: test/models/config_test.rb:718 "feature_available? returns false for ai when no provider configured"
#[test]
fn feature_available_returns_false_for_ai_when_no_provider_configured() {
    assert!(
        !TestVault::new()
            .open_empty_env()
            .current()
            .feature_available(Feature::Ai)
    );
}

// port: test/models/config_test.rb:723 "ai_configured_in_file? returns true when AI credential in file"
#[test]
fn ai_configured_in_file_returns_true_when_ai_credential_in_file() {
    assert!(
        TestVault::with_fed("anthropic_api_key = sk-ant-test")
            .open_empty_env()
            .current()
            .ai_configured_in_file()
    );
}

// port: test/models/config_test.rb:730 "ai_configured_in_file? returns false when only ai_provider in file"
#[test]
fn ai_configured_in_file_returns_false_when_only_ai_provider_in_file() {
    assert!(
        !TestVault::with_fed("ai_provider = anthropic")
            .open_empty_env()
            .current()
            .ai_configured_in_file()
    );
}

// port: test/models/config_test.rb:737 "get_ai ignores ENV when AI credential set in file"
#[test]
fn get_ai_ignores_env_when_ai_credential_set_in_file() {
    let store = TestVault::with_fed("anthropic_api_key = sk-file-anthropic").open(env(&[
        ("OPENAI_API_KEY", "sk-env-openai"),
        ("OPENROUTER_API_KEY", "sk-env-openrouter"),
    ]));
    let rc = store.current();
    assert_eq!(
        rc.get_ai(Key::AnthropicApiKey).value,
        s("sk-file-anthropic")
    );
    assert_eq!(rc.get_ai(Key::OpenaiApiKey).value, None);
    assert_eq!(rc.get_ai(Key::OpenrouterApiKey).value, None);
}

// port: test/models/config_test.rb:753 "get_ai uses ENV when no AI credential in file"
#[test]
fn get_ai_uses_env_when_no_ai_credential_in_file() {
    let store =
        TestVault::with_fed("theme = dark").open(env(&[("OPENAI_API_KEY", "sk-env-openai")]));
    assert_eq!(
        store.current().get_ai(Key::OpenaiApiKey).value,
        s("sk-env-openai")
    );
}

// port: test/models/config_test.rb:764 "effective_ai_provider uses file credentials over ENV"
#[test]
fn effective_ai_provider_uses_file_credentials_over_env() {
    let store = TestVault::with_fed("anthropic_api_key = sk-file-anthropic").open(env(&[
        ("OPENAI_API_KEY", "sk-env-openai"),
        ("OPENROUTER_API_KEY", "sk-env-openrouter"),
    ]));
    let rc = store.current();
    assert_eq!(rc.effective_ai_provider(), Some(AiProviderId::Anthropic));
    assert_eq!(rc.ai_providers_available(), vec![AiProviderId::Anthropic]);
}

// === Config File Upgrade Tests ===

// port: test/models/config_test.rb:780 "upgrade adds missing AI section to existing config"
#[test]
fn upgrade_adds_missing_ai_section_to_existing_config() {
    let v = TestVault::with_fed(
        "# FrankMD Configuration\ntheme = gruvbox\neditor_font_size = 16\n\n# Local Images\n# images_path = /path/to/images\n",
    );
    let rc = v.open_empty_env().current();
    assert_eq!(get_str(&rc, Key::Theme).as_deref(), Some("gruvbox"));
    assert_eq!(get_int(&rc, Key::EditorFontSize), Some(16));
    let content = v.read_fed();
    for needle in [
        "# AI/LLM",
        "ollama_api_base",
        "anthropic_api_key",
        "gemini_api_key",
    ] {
        assert!(content.contains(needle), "missing {needle:?}");
    }
    assert_eq!(v.mode(FED), 0o600);
}

// port: test/models/config_test.rb:809 "upgrade preserves existing values and comments"
#[test]
fn upgrade_preserves_existing_values_and_comments() {
    let v = TestVault::with_fed(
        "# My custom header comment\ntheme = tokyo-night\n\n# My custom comment about images\nimages_path = /my/images\n",
    );
    let _store = v.open_empty_env();
    let content = v.read_fed();
    for needle in [
        "# My custom header comment",
        "theme = tokyo-night",
        "# My custom comment about images",
        "images_path = /my/images",
    ] {
        assert!(content.contains(needle), "missing {needle:?}");
    }
}

// port: test/models/config_test.rb:830 "upgrade does not duplicate existing AI section"
#[test]
fn upgrade_does_not_duplicate_existing_ai_section() {
    let v = TestVault::with_fed(
        "# FrankMD Configuration\ntheme = dark\n\n# AI/LLM (for grammar checking)\nopenai_api_key = sk-existing\n",
    );
    let rc = v.open_empty_env().current();
    assert_eq!(v.read_fed().matches("# AI/LLM").count(), 1);
    assert_eq!(
        get_str(&rc, Key::OpenaiApiKey).as_deref(),
        Some("sk-existing")
    );
}
