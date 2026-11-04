//! Port of `test/controllers/translations_controller_test.rb` (22 tests). `GET /translations` → [`served_translations`]
//! (+ [`resolve_locale`] for the locale choice). The locale precedence tests build
//! the configured value through frankmd-config (`.fed locale` > non-blank
//! `FRANKMD_LOCALE` > nil) exactly as `ApplicationController#set_locale` read it.
//! The three `PATCH /config {locale}` write-back tests are owned by the config
//! port (ConfigStore::update); here only their read-back half is asserted.

use frankmd_config::{EnvSnapshot, Key, ResolvedConfig, Value};
use frankmd_i18n::{Locale, Node, merged_section, resolve_locale, served_translations};
use serde_json::Value as Json;

/// `GET /translations` with a given `.fed` and `FRANKMD_LOCALE` (None = unset).
fn get_translations(fed: Option<&str>, env_locale: Option<&str>) -> Json {
    let env = EnvSnapshot::from_pairs(env_locale.map(|v| ("FRANKMD_LOCALE", v)));
    let cfg = ResolvedConfig::new(fed, env);
    let configured = match cfg.value(Key::Locale) {
        Some(Value::Str(s)) => Some(s),
        _ => None,
    };
    let locale = resolve_locale(None, configured.as_deref());
    served_translations(locale).expect("served translations")
}

fn s<'a>(v: &'a Json, path: &str) -> &'a str {
    path.split('.')
        .fold(v, |n, seg| &n[seg])
        .as_str()
        .unwrap_or_else(|| panic!("{path} is not a string"))
}

const ALL: [&str; 7] = ["en", "pt-BR", "pt-PT", "es", "he", "ja", "ko"];

// port: test/controllers/translations_controller_test.rb:20 "show returns translations for default locale (en)"
#[test]
fn default_locale_is_en_with_sections() {
    let data = get_translations(None, None);
    assert_eq!(data["locale"], "en");
    let tr = &data["translations"];
    for section in [
        "common", "dialogs", "status", "errors", "success", "editor", "sidebar", "library",
    ] {
        assert!(tr.get(section).is_some(), "missing section {section}");
    }
}

// port: test/controllers/translations_controller_test.rb:40 "show returns correct English translations"
#[test]
fn english_values() {
    let tr = &get_translations(None, None)["translations"];
    assert_eq!(s(tr, "common.cancel"), "Cancel");
    assert_eq!(s(tr, "common.save"), "Save");
    assert_eq!(s(tr, "status.saved"), "Saved");
    assert_eq!(s(tr, "errors.note_not_found"), "Note not found");
    assert_eq!(s(tr, "library.title"), "Media Library");
    assert_eq!(s(tr, "library.images"), "Images");
}

// port: test/controllers/translations_controller_test.rb:57 "show returns Portuguese translations when ENV locale is pt-BR"
#[test]
fn pt_br_via_env() {
    let data = get_translations(None, Some("pt-BR"));
    assert_eq!(data["locale"], "pt-BR");
    let tr = &data["translations"];
    assert_eq!(s(tr, "common.cancel"), "Cancelar");
    assert_eq!(s(tr, "common.save"), "Salvar");
    assert_eq!(s(tr, "status.saved"), "Salvo");
    assert_eq!(s(tr, "library.title"), "Biblioteca de mídia");
    assert_eq!(s(tr, "library.videos"), "Vídeos");
}

// port: test/controllers/translations_controller_test.rb:74 "show returns Spanish translations when ENV locale is es"
#[test]
fn es_via_env() {
    let data = get_translations(None, Some("es"));
    assert_eq!(data["locale"], "es");
    let tr = &data["translations"];
    assert_eq!(s(tr, "common.cancel"), "Cancelar");
    assert_eq!(s(tr, "common.save"), "Guardar");
    assert_eq!(s(tr, "status.saved"), "Guardado");
}

// port: test/controllers/translations_controller_test.rb:89 "show fills missing locale keys from English translations"
#[test]
fn missing_keys_filled_from_english() {
    // The original stubbed I18n.t("common", locale: :es) to {save: "Guardar"}.
    let english = Node::from_json(&serde_json::json!({"save": "Save", "cancel": "Cancel"}));
    let localized = Node::from_json(&serde_json::json!({"save": "Guardar"}));
    let merged = merged_section(Some(&english), Some(&localized)).to_json();
    assert_eq!(merged["save"], "Guardar");
    assert_eq!(merged["cancel"], "Cancel");
}

// port: test/controllers/translations_controller_test.rb:109 "show returns Japanese translations when ENV locale is ja"
#[test]
fn ja_via_env() {
    let data = get_translations(None, Some("ja"));
    assert_eq!(data["locale"], "ja");
    let tr = &data["translations"];
    assert_eq!(s(tr, "common.cancel"), "キャンセル");
    assert_eq!(s(tr, "common.save"), "保存");
    assert_eq!(s(tr, "status.saved"), "保存済み");
}

// port: test/controllers/translations_controller_test.rb:126 "locale can be set via config file when ENV not set"
#[test]
fn locale_from_fed() {
    assert_eq!(
        get_translations(Some("locale = es\n"), None)["locale"],
        "es"
    );
}

// port: test/controllers/translations_controller_test.rb:137 "locale can be saved via config endpoint"
#[test]
fn locale_saved_via_config_reads_back() {
    // Write half (PATCH /config writes `locale = ja`) is ported with the config store tests.
    assert_eq!(
        get_translations(Some("locale = ja\n"), None)["locale"],
        "ja"
    );
}

// port: test/controllers/translations_controller_test.rb:152 "locale persists across requests after config save"
#[test]
fn locale_persists_across_requests() {
    let fed = "locale = pt-BR\n";
    assert_eq!(get_translations(Some(fed), None)["locale"], "pt-BR");
    assert_eq!(get_translations(Some(fed), None)["locale"], "pt-BR");
}

// port: test/controllers/translations_controller_test.rb:174 "all locales have complete common translations"
#[test]
fn common_complete_in_all_locales() {
    for locale in ALL {
        let data = get_translations(None, Some(locale));
        for key in [
            "cancel", "apply", "save", "create", "delete", "rename", "close", "search", "insert",
            "edit", "ok",
        ] {
            let v = &data["translations"]["common"][key];
            assert!(
                v.as_str().is_some_and(|v| !v.is_empty()),
                "Locale {locale} is missing or has empty common.{key}"
            );
        }
    }
}

// port: test/controllers/translations_controller_test.rb:191 "all locales have complete status translations"
#[test]
fn status_complete_in_all_locales() {
    for locale in ALL {
        let data = get_translations(None, Some(locale));
        for key in [
            "saved",
            "unsaved",
            "error_saving",
            "error_loading",
            "searching",
            "no_matches",
        ] {
            let v = &data["translations"]["status"][key];
            assert!(
                v.as_str().is_some_and(|v| !v.is_empty()),
                "Locale {locale} is missing or has empty status.{key}"
            );
        }
    }
}

// port: test/controllers/translations_controller_test.rb:208 "all locales have complete error translations"
#[test]
fn errors_complete_in_all_locales() {
    for locale in ALL {
        let data = get_translations(None, Some(locale));
        for key in ["note_not_found", "folder_not_found", "file_not_found"] {
            let v = &data["translations"]["errors"][key];
            assert!(
                v.as_str().is_some_and(|v| !v.is_empty()),
                "Locale {locale} is missing or has empty errors.{key}"
            );
        }
    }
}

// port: test/controllers/translations_controller_test.rb:225 "all supported locales have translations for issue 187 dialog copy"
#[test]
fn issue_187_dialog_copy_in_all_locales() {
    let required_paths = [
        "dialogs.log_viewer.title_logs",
        "dialogs.log_viewer.title_config",
        "dialogs.log_viewer.refresh",
        "dialogs.log_viewer.close",
        "dialogs.log_viewer.logs",
        "dialogs.log_viewer.config",
        "dialogs.log_viewer.loading",
        "dialogs.log_viewer.empty_logs",
        "dialogs.log_viewer.line_count_one",
        "dialogs.log_viewer.line_count",
        "dialogs.log_viewer.error_loading_logs",
        "dialogs.log_viewer.no_config_file",
        "dialogs.log_viewer.key_count_one",
        "dialogs.log_viewer.key_count",
        "dialogs.log_viewer.error_loading_config",
        "dialogs.log_viewer.config_file_label",
        "dialogs.log_viewer.file_exists_label",
        "dialogs.log_viewer.ai_configured_label",
        "dialogs.log_viewer.yes",
        "dialogs.log_viewer.no",
        "dialogs.log_viewer.ai_configured_yes",
        "dialogs.log_viewer.ai_configured_no",
        "dialogs.log_viewer.key",
        "dialogs.log_viewer.value",
        "dialogs.log_viewer.source",
        "dialogs.log_viewer.env_var",
        "dialogs.log_viewer.source_default",
        "dialogs.log_viewer.null_value",
        "dialogs.log_viewer.empty_config",
        "dialogs.file_finder.preview_error",
        "dialogs.image_picker.processing_image",
        "dialogs.image_picker.search_keywords_required",
        "dialogs.image_picker.search_failed",
        "dialogs.image_picker.search_result_one",
        "dialogs.image_picker.search_result_many",
        "dialogs.image_picker.no_images_found",
        "dialogs.image_picker.no_images_in_folder",
        "dialogs.image_picker.folder_showing_recent",
        "dialogs.image_picker.folder_images_found_one",
        "dialogs.image_picker.folder_images_found_many",
        "dialogs.image_picker.folder_api_not_supported",
        "dialogs.image_picker.folder_access_failed",
        "dialogs.image_picker.folder_read_failed",
        "dialogs.image_picker.image_load_failed",
        "dialogs.image_picker.image_delete_failed",
        "dialogs.image_picker.image_upload_failed",
        "dialogs.image_picker.ai_prompt_required",
        "dialogs.image_picker.ai_not_configured",
        "dialogs.image_picker.ai_generation_failed",
        "dialogs.image_picker.ai_no_generated_data",
        "dialogs.image_picker.ai_save_failed",
        "dialogs.image_picker.no_file_extension",
        "dialogs.ai_diff.get_key_from",
        "dialogs.text_format.bold",
        "dialogs.text_format.italic",
        "dialogs.text_format.strikethrough",
        "dialogs.text_format.highlight",
        "dialogs.text_format.subscript",
        "dialogs.text_format.superscript",
        "dialogs.text_format.link",
        "dialogs.video.url_hint_youtube",
        "dialogs.video.url_hint_file",
    ];
    for locale in ALL {
        let data = get_translations(None, Some(locale));
        let tr = &data["translations"];
        for path in required_paths {
            let v = path.split('.').fold(tr, |n, seg| &n[seg]);
            assert!(
                v.as_str().is_some_and(|v| !v.is_empty()),
                "Locale {locale} is missing {path}"
            );
        }
        let youtube_hint = s(tr, "dialogs.video.url_hint_youtube");
        let file_hint = s(tr, "dialogs.video.url_hint_file");
        assert!(
            youtube_hint.starts_with("http://") || youtube_hint.starts_with("https://"),
            "Locale {locale} should start the YouTube hint with its URL example"
        );
        assert!(
            file_hint.starts_with('/'),
            "Locale {locale} should start the video file hint with its path example"
        );
    }
}

// port: test/controllers/translations_controller_test.rb:310 "invalid locale falls back to default"
#[test]
fn invalid_locale_falls_back_to_en() {
    assert_eq!(
        get_translations(None, Some("invalid_locale"))["locale"],
        "en"
    );
}

// port: test/controllers/translations_controller_test.rb:322 "completed Spanish entries remain localized in the endpoint"
#[test]
fn spanish_entries_localized() {
    let tr = &get_translations(None, Some("es"))["translations"];
    assert_eq!(s(tr, "errors.title_required"), "El título es obligatorio");
    assert_eq!(
        s(tr, "errors.permission_denied"),
        "Permiso denegado. Es posible que no tengas acceso a este archivo o carpeta."
    );
    assert_eq!(
        s(tr, "connection.offline_message"),
        "Se perdió la conexión. La edición está desactivada hasta que se restablezca."
    );
}

// port: test/controllers/translations_controller_test.rb:337 "locale values win over English for shared keys"
#[test]
fn locale_values_win() {
    let tr = &get_translations(None, Some("es"))["translations"];
    assert_eq!(s(tr, "common.save"), "Guardar");
    assert_eq!(s(tr, "common.cancel"), "Cancelar");
}

// port: test/controllers/translations_controller_test.rb:348 "requesting English returns the English tree unchanged"
#[test]
fn english_tree_unchanged() {
    let tr = &get_translations(None, Some("en"))["translations"];
    assert_eq!(s(tr, "errors.title_required"), "Title is required");
    assert_eq!(s(tr, "common.save"), "Save");
}

// port: test/controllers/translations_controller_test.rb:361 "config file takes precedence over ENV variable"
#[test]
fn fed_beats_env() {
    assert_eq!(
        get_translations(Some("locale = ja\n"), Some("es"))["locale"],
        "ja"
    );
}

// port: test/controllers/translations_controller_test.rb:376 "ENV variable is used as fallback when config file has no locale"
#[test]
fn env_used_when_fed_has_no_locale() {
    assert_eq!(
        get_translations(Some("theme = dark\n"), Some("es"))["locale"],
        "es"
    );
}

// port: test/controllers/translations_controller_test.rb:389 "empty ENV FRANKMD_LOCALE does not override config file locale"
#[test]
fn empty_env_does_not_override_fed() {
    assert_eq!(
        get_translations(Some("locale = pt-BR\n"), Some(""))["locale"],
        "pt-BR"
    );
    // and an empty ENV alone is ignored (→ en)
    assert_eq!(resolve_locale(None, None), Locale::En);
}

// port: test/controllers/translations_controller_test.rb:405 "updating locale via config endpoint updates config file"
#[test]
fn updating_locale_reads_back_latest() {
    // Write half (single `locale = ja` line, no `locale = en` left) is ported with the config store tests.
    assert_eq!(
        get_translations(Some("locale = ja\n"), None)["locale"],
        "ja"
    );
}

// port: test/controllers/translations_controller_test.rb:423 "translations include all sections needed by JavaScript"
#[test]
fn all_js_sections_present() {
    let data = get_translations(None, None);
    for section in [
        "common",
        "dialogs",
        "status",
        "errors",
        "success",
        "editor",
        "sidebar",
        "preview",
        "context_menu",
    ] {
        assert!(
            data["translations"].get(section).is_some(),
            "Missing section: {section}"
        );
    }
}
