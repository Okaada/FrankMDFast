//! Catalog behaviour the original relied on implicitly (no dedicated Ruby test):
//! Rails `%{var}` interpolation, the default pluralizer and `en` fallback.
//! Sources cited per test; values come from the shipped `assets/locales/*.yml`.

use frankmd_i18n::{Catalog, Locale, Node};

fn catalog(active: serde_json::Value, english: serde_json::Value) -> Catalog {
    Catalog::from_trees(
        Locale::PtBr,
        &Node::from_json(&active),
        &Node::from_json(&english),
    )
}

// derived from: i18n gem `I18n::Backend::Base#interpolate` (`%{name}`), used e.g. by dialogs.confirm.delete_file
#[test]
fn interpolates_named_variables() {
    let c = Catalog::load(Locale::En).expect("en");
    let out = c.t("dialogs.confirm.delete_file", &[("name", "todo.md")]);
    assert!(out.contains("todo.md"), "{out}");
    assert!(!out.contains("%{name}"), "{out}");
}

// derived from: i18n gem interpolation — every occurrence is replaced; unknown vars stay verbatim (we do not raise MissingInterpolationArgument)
#[test]
fn interpolation_replaces_all_occurrences_and_keeps_unknown() {
    let c = catalog(
        serde_json::json!({"k": "%{a}-%{a}-%{b}"}),
        serde_json::json!({}),
    );
    assert_eq!(c.t("k", &[("a", "x")]), "x-x-%{b}");
}

// derived from: config.i18n.fallbacks = true (config/application.rb:46); pt-BR chain is [pt-BR, pt, en]
#[test]
fn missing_key_falls_back_to_english() {
    let c = catalog(
        serde_json::json!({"common": {"save": "Salvar"}}),
        serde_json::json!({"common": {"save": "Save", "cancel": "Cancel"}}),
    );
    assert_eq!(c.t("common.save", &[]), "Salvar");
    assert_eq!(c.t("common.cancel", &[]), "Cancel");
}

// derived from: app/javascript `window.t` returned the key on a miss (i18n_media_upload_keys_test.rb comment)
#[test]
fn key_missing_everywhere_returns_the_key() {
    let c = catalog(serde_json::json!({}), serde_json::json!({}));
    assert_eq!(c.t("nope.missing", &[]), "nope.missing");
}

// derived from: app/helpers/recent_files_helper.rb:25 t("home.opened_ago.#{unit}", count:) + Rails default pluralizer
#[test]
fn default_pluralizer_one_and_other() {
    let c = Catalog::load(Locale::En).expect("en");
    assert_eq!(c.t_count("home.opened_ago.seconds", 1, &[]), "1 second ago");
    assert_eq!(
        c.t_count("home.opened_ago.seconds", 5, &[]),
        "5 seconds ago"
    );
    // count 0 has no `zero:` entry → `other`
    assert_eq!(
        c.t_count("home.opened_ago.minutes", 0, &[]),
        "0 minutes ago"
    );
}

// derived from: i18n gem Pluralization default rule: `:zero` only when present and count == 0
#[test]
fn zero_form_used_only_when_present() {
    let c = catalog(
        serde_json::json!({"n": {"zero": "none", "one": "one", "other": "%{count} many"}}),
        serde_json::json!({}),
    );
    assert_eq!(c.t_count("n", 0, &[]), "none");
    assert_eq!(c.t_count("n", 1, &[]), "one");
    assert_eq!(c.t_count("n", 2, &[]), "2 many");
    assert_eq!(c.t_count("n", -1, &[]), "-1 many");
}

// derived from: HP-022 (YAML 1.2): dialogs.log_viewer.yes/no must stay strings
#[test]
fn yes_no_values_stay_strings() {
    let c = Catalog::load(Locale::En).expect("en");
    // en.yml: `"yes": "yes"` / `"no": "no"` (quoted keys; YAML 1.1 would read bare yes/no as booleans)
    assert_eq!(c.t("dialogs.log_viewer.yes", &[]), "yes");
    assert_eq!(c.t("dialogs.log_viewer.no", &[]), "no");
}

// derived from: config/application.rb available_locales; Locale::from_code is exact (I18n.available_locales.include?)
#[test]
fn locale_codes_round_trip_and_are_case_sensitive() {
    for l in Locale::ALL {
        assert_eq!(Locale::from_code(l.code()), Some(l));
    }
    assert_eq!(Locale::from_code("pt-br"), None);
    assert_eq!(Locale::from_code("pt_BR"), None);
    assert_eq!(Locale::from_code(""), None);
    assert!(Locale::He.is_rtl());
    assert!(!Locale::Ja.is_rtl());
}
