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
#![allow(unused_variables, dead_code)] // skeleton

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
        Locale::En, Locale::PtBr, Locale::PtPt, Locale::Es, Locale::He, Locale::Ja, Locale::Ko,
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
        todo!()
    }

    /// Map an Omarchy language code / POSIX `LANG` to a FrankMD locale.
    pub fn from_system(code: &str) -> Option<Self> {
        todo!("omarchy-4.5.md §4.2")
    }

    /// Right-to-left script?
    pub fn is_rtl(self) -> bool {
        matches!(self, Locale::He)
    }

    /// `I18n.fallbacks[locale]` as recorded (`pt-BR → [pt-BR, pt, en]`, `es → [es, en]`).
    pub fn fallback_chain(self) -> Vec<&'static str> {
        todo!()
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
    todo!()
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
    todo!()
}

/// [`parse_locale_yaml`] for a shipped locale.
pub fn locale_tree(locale: Locale) -> Result<Node, I18nError> {
    parse_locale_yaml(locale.source_yaml(), locale.code())
}

/// Keys Rails served from its gems' `en` locale files (activemodel /
/// activesupport `errors.*` etc.), as a tree to deep-merge under the app's.
pub fn rails_supplement_en() -> Node {
    todo!("embed the 31 golden en_fill_keys (translations.json) values")
}

/// **[COMPAT]** `flatten_leaves` (translation_coverage_test.rb): dotted path →
/// leaf, in document order. Empty maps produce no leaves.
pub fn flatten_leaves(tree: &Node) -> Vec<(String, Leaf)> {
    todo!()
}

/// **[COMPAT]** `translation_failures(english, translations_by_locale)`:
/// for each locale (given order) and each English leaf (document order):
/// `"<loc>: missing key <path>"` or
/// `"<loc>: blank or non-string value for <path> (got <RubyClass>)"`.
pub fn translation_failures(english: &Node, translations: &[(&str, &Node)]) -> Vec<String> {
    todo!()
}

/// `%{name}` tokens of every leaf that has any, sorted and de-duplicated per key.
pub fn interpolation_vars(tree: &Node) -> BTreeMap<String, Vec<String>> {
    todo!()
}

/// Rails `present?` lookup of a dotted key in a tree (non-blank string leaf).
pub fn key_present(tree: &Node, dotted: &str) -> bool {
    todo!()
}

/// Sections served to the UI (`TranslationsController` `js_keys`, in order).
pub const SERVED_SECTIONS: [&str; 11] =
    ["common", "dialogs", "status", "errors", "success", "editor", "sidebar", "preview", "context_menu", "connection", "library"];

/// **[COMPAT]** `deep_merge_over(base, overrides)`: overrides win, nested maps
/// merge so sibling keys from both sides survive; a non-map side replaces.
pub fn deep_merge_over(base: &Node, overrides: &Node) -> Node {
    todo!()
}

/// **[COMPAT]** `TranslationsController#merged_section`: the locale's section
/// deep-merged over the English one (if either is not a map, the localized
/// value is returned as is; missing → empty map).
pub fn merged_section(english: Option<&Node>, localized: Option<&Node>) -> Node {
    todo!()
}

/// **[COMPAT]** `GET /translations` body: `{locale, translations: {section: merged}}`
/// for [`SERVED_SECTIONS`], including [`rails_supplement_en`] keys.
pub fn served_translations(locale: Locale) -> Result<serde_json::Value, I18nError> {
    todo!()
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
        todo!()
    }

    /// Build from explicit trees (tests).
    pub fn from_trees(locale: Locale, active: &Node, english: &Node) -> Self {
        todo!()
    }

    /// The active locale.
    pub fn locale(&self) -> Locale {
        self.locale.unwrap_or(Locale::En)
    }

    /// Translate `key` with `%{var}` interpolation. Lookup: active locale, then
    /// `en`; a key missing everywhere returns the key itself (what the JS
    /// `window.t` did). Unknown `%{var}` with no arg is left verbatim.
    pub fn t(&self, key: &str, args: Args<'_>) -> String {
        todo!()
    }

    /// Translate with Rails default pluralization on `count` (`key.zero` only if
    /// present and count == 0, `key.one` for 1, else `key.other`); `%{count}` is
    /// interpolated automatically.
    pub fn t_count(&self, key: &str, count: i64, args: Args<'_>) -> String {
        todo!()
    }
}
