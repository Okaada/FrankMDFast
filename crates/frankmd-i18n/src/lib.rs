//! # frankmd-i18n
//!
//! Loads FrankMD's Rails locale YAML **unchanged** and reproduces Rails'
//! behaviour exactly: `%{var}` interpolation, the *default* pluralizer
//! (`zero` only if present and count == 0, `one` for 1, else `other`) and
//! fallback to `en`. Golden: `tests/fixtures/golden/translations/`.
//!
//! No process-global locale (HP-026): a [`Catalog`] is passed explicitly.
//!
//! Besides the app's own YAML, Rails served a few keys from its gems'
//! locale files (`errors.format`, `errors.messages.*`, `errors.connection_refused`
//! ...). [`rails_supplement_en`] embeds those so [`served_translations`]
//! reproduces `GET /translations` byte-for-byte (golden `en_fill_keys`).

mod interpolate;
pub mod tree;

pub use tree::{Leaf, Node};

use std::collections::{BTreeMap, HashMap};

/// The seven shipped UI locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    /// English (fallback).
    En,
    /// Brazilian Portuguese.
    PtBr,
    /// European Portuguese (Omarchy `pt` maps here).
    PtPt,
    /// Spanish (Omarchy `es-MX` maps here).
    Es,
    /// Hebrew (RTL).
    He,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
}

impl Locale {
    /// All shipped locales, in `config.i18n.available_locales` order
    /// (`en pt-BR pt-PT es he ja ko`).
    pub const ALL: [Locale; 7] = [
        Locale::En,
        Locale::PtBr,
        Locale::PtPt,
        Locale::Es,
        Locale::He,
        Locale::Ja,
        Locale::Ko,
    ];

    /// The Rails locale YAML for this locale, embedded **unchanged** from
    /// `assets/locales/` (extracted from FrankMD by `tools/assets/extract.mjs`).
    pub fn source_yaml(self) -> &'static str {
        match self {
            Locale::En => include_str!("../../../assets/locales/en.yml"),
            Locale::PtBr => include_str!("../../../assets/locales/pt-BR.yml"),
            Locale::PtPt => include_str!("../../../assets/locales/pt-PT.yml"),
            Locale::Es => include_str!("../../../assets/locales/es.yml"),
            Locale::He => include_str!("../../../assets/locales/he.yml"),
            Locale::Ja => include_str!("../../../assets/locales/ja.yml"),
            Locale::Ko => include_str!("../../../assets/locales/ko.yml"),
        }
    }

    /// The FrankMD code (`"en"`, `"pt-BR"`, ...), also the YAML root key.
    pub fn code(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::PtBr => "pt-BR",
            Locale::PtPt => "pt-PT",
            Locale::Es => "es",
            Locale::He => "he",
            Locale::Ja => "ja",
            Locale::Ko => "ko",
        }
    }

    /// Parse a FrankMD locale code. **[COMPAT]** exact and case-sensitive like
    /// `I18n.available_locales.include?(locale.to_sym)`: `"pt-br"`, `"pt_BR"` → `None`.
    pub fn from_code(code: &str) -> Option<Self> {
        Locale::ALL.into_iter().find(|l| l.code() == code)
    }

    /// Map an Omarchy language code / POSIX `LANG` to a FrankMD locale.
    ///
    /// Accepts Omarchy registry codes (`pt` is European Portuguese, `es-MX` is
    /// the only Spanish) and POSIX locales (`pt_BR.UTF-8`, `es_ES@euro`).
    /// Anything else (`de`, `C`, `POSIX`) → `None`, meaning "use `en`".
    pub fn from_system(code: &str) -> Option<Self> {
        let base = code.split(['.', '@']).next().unwrap_or_default().trim();
        let mut parts = base.split(['_', '-']);
        let lang = parts.next().unwrap_or_default().to_ascii_lowercase();
        let region = parts.next().unwrap_or_default().to_ascii_uppercase();
        match lang.as_str() {
            "en" => Some(Locale::En),
            "pt" if region == "BR" => Some(Locale::PtBr),
            "pt" => Some(Locale::PtPt),
            "es" => Some(Locale::Es),
            // `iw` is the legacy ISO 639 code for Hebrew, still seen in some LANGs.
            "he" | "iw" => Some(Locale::He),
            "ja" => Some(Locale::Ja),
            "ko" => Some(Locale::Ko),
            _ => None,
        }
    }

    /// Right-to-left script?
    pub fn is_rtl(self) -> bool {
        matches!(self, Locale::He)
    }

    /// `I18n.fallbacks[locale]` as recorded (`pt-BR → [pt-BR, pt, en]`, `es → [es, en]`).
    pub fn fallback_chain(self) -> Vec<&'static str> {
        let code = self.code();
        let mut chain = vec![code];
        if let Some((language, _region)) = code.split_once('-') {
            chain.push(language);
        }
        if self != Locale::En {
            chain.push(Locale::En.code());
        }
        chain
    }
}

/// **[COMPAT]** `ApplicationController#set_locale`:
/// `params[:locale] || Config.get("locale") || default`, applied only when it is
/// an available locale; otherwise the request keeps `en` (the locale is reset
/// per request). A present-but-invalid `param` does **not** fall through to
/// `configured` (`?locale=xx` with `FRANKMD_LOCALE=es` → `en`).
///
/// `configured` is `ResolvedConfig` `locale` (`.fed` > non-blank `FRANKMD_LOCALE`
/// > nil), resolved by frankmd-config.
pub fn resolve_locale(param: Option<&str>, configured: Option<&str>) -> Locale {
    // Ruby `||`: an empty-but-present param is still truthy and wins.
    param
        .or(configured)
        .and_then(Locale::from_code)
        .unwrap_or(Locale::En)
}

/// Interpolation arguments.
pub type Args<'a> = &'a [(&'a str, &'a str)];

/// Errors loading a catalog.
#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    /// YAML could not be parsed.
    #[error("locale yaml: {0}")]
    Yaml(String),
    /// The YAML has no `<locale>:` root.
    #[error("locale yaml has no root key {0}")]
    MissingRoot(String),
}

/// Parse a locale YAML (YAML 1.2: `yes`/`no` stay strings, HP-022) and return the
/// tree under its `<code>:` root, key order preserved (`YAML.safe_load_file(..).fetch(locale)`).
pub fn parse_locale_yaml(yaml: &str, code: &str) -> Result<Node, I18nError> {
    let options = serde_saphyr::options! {
        strict_booleans: true,
        duplicate_keys: serde_saphyr::DuplicateKeyPolicy::LastWins,
    };
    let doc: Node = serde_saphyr::from_str_with_options(yaml, options)
        .map_err(|e| I18nError::Yaml(e.to_string()))?;
    doc.get(code)
        .cloned()
        .ok_or_else(|| I18nError::MissingRoot(code.to_owned()))
}

/// [`parse_locale_yaml`] for a shipped locale.
pub fn locale_tree(locale: Locale) -> Result<Node, I18nError> {
    parse_locale_yaml(locale.source_yaml(), locale.code())
}

/// Keys Rails served from its gems' `en` locale files (activemodel /
/// activesupport `errors.*` etc.), as a tree to deep-merge under the app's.
pub fn rails_supplement_en() -> Node {
    const SUPPLEMENT: &str = include_str!("rails_supplement_en.json");
    // The embedded file is valid JSON (checked by the crate's tests); an
    // unparsable file degrades to an empty supplement rather than a panic.
    serde_json::from_str::<serde_json::Value>(SUPPLEMENT)
        .map_or_else(|_| Node::Map(Vec::new()), |v| Node::from_json(&v))
}

/// **[COMPAT]** `flatten_leaves` (translation_coverage_test.rb): dotted path →
/// leaf, in document order. Empty maps produce no leaves.
pub fn flatten_leaves(tree: &Node) -> Vec<(String, Leaf)> {
    fn walk(node: &Node, prefix: Option<&str>, out: &mut Vec<(String, Leaf)>) {
        let Node::Map(entries) = node else { return };
        for (key, value) in entries {
            let path = prefix.map_or_else(|| key.clone(), |p| format!("{p}.{key}"));
            match value {
                Node::Map(_) => walk(value, Some(&path), out),
                Node::Leaf(leaf) => out.push((path, leaf.clone())),
            }
        }
    }
    let mut out = Vec::new();
    walk(tree, None, &mut out);
    out
}

/// **[COMPAT]** `translation_failures(english, translations_by_locale)`:
/// for each locale (given order) and each English leaf (document order):
/// `"<loc>: missing key <path>"` or
/// `"<loc>: blank or non-string value for <path> (got <RubyClass>)"`.
pub fn translation_failures(english: &Node, translations: &[(&str, &Node)]) -> Vec<String> {
    let english_leaves = flatten_leaves(english);
    let mut failures = Vec::new();
    for (locale, tree) in translations {
        let translated: HashMap<String, Leaf> = flatten_leaves(tree).into_iter().collect();
        for (path, _) in &english_leaves {
            match translated.get(path) {
                None => failures.push(format!("{locale}: missing key {path}")),
                Some(Leaf::Str(s)) if !ruby_strip(s).is_empty() => {}
                Some(value) => failures.push(format!(
                    "{locale}: blank or non-string value for {path} (got {})",
                    value.ruby_class()
                )),
            }
        }
    }
    failures
}

/// Ruby `String#strip`: ASCII whitespace and NUL only.
fn ruby_strip(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\0'))
}

/// `%{name}` tokens of every leaf that has any, sorted and de-duplicated per key.
pub fn interpolation_vars(tree: &Node) -> BTreeMap<String, Vec<String>> {
    flatten_leaves(tree)
        .into_iter()
        .filter_map(|(path, leaf)| {
            let vars = interpolate::leaf_vars(&leaf);
            (!vars.is_empty()).then_some((path, vars))
        })
        .collect()
}

/// Rails `present?` lookup of a dotted key in a tree (non-blank string leaf).
pub fn key_present(tree: &Node, dotted: &str) -> bool {
    match tree.lookup(dotted) {
        Some(Node::Leaf(leaf)) => leaf.is_present(),
        Some(Node::Map(entries)) => !entries.is_empty(),
        None => false,
    }
}

/// Sections served to the UI (`TranslationsController` `js_keys`, in order).
pub const SERVED_SECTIONS: [&str; 11] = [
    "common",
    "dialogs",
    "status",
    "errors",
    "success",
    "editor",
    "sidebar",
    "preview",
    "context_menu",
    "connection",
    "library",
];

/// **[COMPAT]** `deep_merge_over(base, overrides)`: overrides win, nested maps
/// merge so sibling keys from both sides survive; a non-map side replaces.
pub fn deep_merge_over(base: &Node, overrides: &Node) -> Node {
    let Node::Map(override_entries) = overrides else {
        return overrides.clone();
    };
    let mut merged = match base {
        Node::Map(_) => base.clone(),
        Node::Leaf(_) => Node::Map(Vec::new()),
    };
    for (key, value) in override_entries {
        let next = match (merged.get(key), value) {
            (Some(existing @ Node::Map(_)), Node::Map(_)) => deep_merge_over(existing, value),
            _ => value.clone(),
        };
        merged.insert(key.clone(), next);
    }
    merged
}

/// **[COMPAT]** `TranslationsController#merged_section`: the locale's section
/// deep-merged over the English one (if either is not a map, the localized
/// value is returned as is; missing → empty map).
pub fn merged_section(english: Option<&Node>, localized: Option<&Node>) -> Node {
    let empty = Node::Map(Vec::new());
    let localized = localized.unwrap_or(&empty);
    let english = english.unwrap_or(&empty);
    match (english, localized) {
        (Node::Map(_), Node::Map(_)) => deep_merge_over(english, localized),
        _ => localized.clone(),
    }
}

/// **[COMPAT]** `GET /translations` body: `{locale, translations: {section: merged}}`
/// for [`SERVED_SECTIONS`], including [`rails_supplement_en`] keys.
pub fn served_translations(locale: Locale) -> Result<serde_json::Value, I18nError> {
    let english = english_with_supplement()?;
    let own = if locale == Locale::En {
        english.clone()
    } else {
        locale_tree(locale)?
    };
    let mut translations = serde_json::Map::new();
    for section in SERVED_SECTIONS {
        // `I18n.t(section, locale:)` with fallbacks: the first locale in the
        // chain that has the section (`pt` has no file, so it never does).
        let localized = own.get(section).or_else(|| english.get(section));
        translations.insert(
            section.to_owned(),
            merged_section(english.get(section), localized).to_json(),
        );
    }
    Ok(serde_json::json!({ "locale": locale.code(), "translations": translations }))
}

/// The `en` store as Rails had it: gem locale files first, the app's `en.yml`
/// deep-merged over them.
fn english_with_supplement() -> Result<Node, I18nError> {
    Ok(deep_merge_over(
        &rails_supplement_en(),
        &locale_tree(Locale::En)?,
    ))
}

/// Flattened `key → message` tables for the active locale plus `en` fallback.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    locale: Option<Locale>,
    active: HashMap<String, Leaf>,
    fallback: HashMap<String, Leaf>,
}

impl Catalog {
    /// Build from [`Locale::source_yaml`] (keys like `home.title`; YAML 1.2
    /// semantics so `yes`/`no` stay strings, HP-022).
    pub fn load(locale: Locale) -> Result<Self, I18nError> {
        let english = locale_tree(Locale::En)?;
        let active = if locale == Locale::En {
            english.clone()
        } else {
            locale_tree(locale)?
        };
        Ok(Self::from_trees(locale, &active, &english))
    }

    /// Build from explicit trees (tests).
    pub fn from_trees(locale: Locale, active: &Node, english: &Node) -> Self {
        Self {
            locale: Some(locale),
            active: flatten_leaves(active).into_iter().collect(),
            fallback: flatten_leaves(english).into_iter().collect(),
        }
    }

    /// The active table, then the `en` one.
    fn tables(&self) -> [&HashMap<String, Leaf>; 2] {
        [&self.active, &self.fallback]
    }

    /// The active locale.
    pub fn locale(&self) -> Locale {
        self.locale.unwrap_or(Locale::En)
    }

    /// Translate `key` with `%{var}` interpolation. Lookup: active locale, then
    /// `en`; a key missing everywhere returns the key itself (what the JS
    /// `window.t` did). Unknown `%{var}` with no arg is left verbatim.
    pub fn t(&self, key: &str, args: Args<'_>) -> String {
        self.tables()
            .into_iter()
            .find_map(|table| table.get(key).and_then(Leaf::as_str))
            .map_or_else(
                || key.to_owned(),
                |message| interpolate::interpolate(message, args),
            )
    }

    /// Translate with Rails default pluralization on `count` (`key.zero` only if
    /// present and count == 0, `key.one` for 1, else `key.other`); `%{count}` is
    /// interpolated automatically.
    pub fn t_count(&self, key: &str, count: i64, args: Args<'_>) -> String {
        let count_text = count.to_string();
        let mut all_args: Vec<(&str, &str)> = vec![("count", count_text.as_str())];
        all_args.extend(args.iter().filter(|(name, _)| *name != "count"));

        for table in self.tables() {
            // A plain string entry ignores the count (Rails returns it as is).
            if let Some(message) = table.get(key).and_then(Leaf::as_str) {
                return interpolate::interpolate(message, &all_args);
            }
            let form = |name: &str| table.get(&format!("{key}.{name}")).and_then(Leaf::as_str);
            let has_entry = ["zero", "one", "other"]
                .iter()
                .any(|name| form(name).is_some());
            if !has_entry {
                continue;
            }
            let zero = if count == 0 { form("zero") } else { None };
            let chosen = zero.or_else(|| {
                if count == 1 {
                    form("one")
                } else {
                    form("other")
                }
            });
            return chosen.map_or_else(
                || key.to_owned(),
                |message| interpolate::interpolate(message, &all_args),
            );
        }
        key.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `served_translations` against the recorded `GET /translations` bodies,
    /// without going through frankmd-config's locale resolution.
    #[test]
    fn served_bodies_match_the_recorded_ones() {
        for locale in Locale::ALL {
            let file = format!("translations/served_{}.json", locale.code());
            let expected: serde_json::Value =
                serde_json::from_slice(&frankmd_testkit::golden_bytes(&file)).expect("golden json");
            let actual = served_translations(locale).expect("served");
            assert!(actual == expected, "{} differs from {file}", locale.code());
        }
    }

    #[test]
    fn system_codes_map_like_omarchy() {
        assert_eq!(Locale::from_system("pt"), Some(Locale::PtPt));
        assert_eq!(Locale::from_system("pt_BR.UTF-8"), Some(Locale::PtBr));
        assert_eq!(Locale::from_system("es-MX"), Some(Locale::Es));
        assert_eq!(Locale::from_system("es_ES@euro"), Some(Locale::Es));
        assert_eq!(Locale::from_system("en_US.UTF-8"), Some(Locale::En));
        assert_eq!(Locale::from_system("de"), None);
        assert_eq!(Locale::from_system("C"), None);
    }

    #[test]
    fn invalid_param_does_not_fall_through() {
        assert_eq!(resolve_locale(Some("xx"), Some("es")), Locale::En);
        assert_eq!(resolve_locale(None, Some("es")), Locale::Es);
        assert_eq!(resolve_locale(Some("ja"), Some("es")), Locale::Ja);
    }
}
