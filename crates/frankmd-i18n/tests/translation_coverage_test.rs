//! Port of `test/services/translation_coverage_test.rb` (2 tests).

use frankmd_i18n::{Locale, Node, locale_tree, translation_failures};

// port: test/services/translation_coverage_test.rb:11 "all supported locales cover every English translation leaf"
#[test]
fn all_supported_locales_cover_every_english_translation_leaf() {
    // LOCALES = available_locales - en, sorted.
    let mut others: Vec<Locale> = Locale::ALL.into_iter().filter(|l| *l != Locale::En).collect();
    others.sort_by_key(|l| l.code());
    let english = locale_tree(Locale::En).expect("en.yml");
    let trees: Vec<(Locale, Node)> = others.iter().map(|l| (*l, locale_tree(*l).expect("locale yaml"))).collect();
    let refs: Vec<(&str, &Node)> = trees.iter().map(|(l, t)| (l.code(), t)).collect();
    let failures = translation_failures(&english, &refs);
    let details: Vec<String> = failures.iter().map(|f| format!("  - {f}")).collect();
    assert!(failures.is_empty(), "Missing or invalid locale translations:\n{}", details.join("\n"));
}

// port: test/services/translation_coverage_test.rb:20 "reports every nested missing, blank, and non-string translation"
#[test]
fn reports_every_nested_missing_blank_and_non_string_translation() {
    let english = Node::from_json(&serde_json::json!({
        "dialogs": {"help": {"find": "Find"}},
        "errors": {"required": "Required"}
    }));
    let pt_br = Node::from_json(&serde_json::json!({
        "dialogs": {"help": {}},
        "errors": {"required": "   "}
    }));
    let ja = Node::from_json(&serde_json::json!({
        "dialogs": {"help": {"find": false}},
        "errors": {"required": "必須"}
    }));
    assert_eq!(
        translation_failures(&english, &[("pt-BR", &pt_br), ("ja", &ja)]),
        vec![
            "pt-BR: missing key dialogs.help.find".to_owned(),
            "pt-BR: blank or non-string value for errors.required (got String)".to_owned(),
            "ja: blank or non-string value for dialogs.help.find (got FalseClass)".to_owned(),
        ]
    );
}
