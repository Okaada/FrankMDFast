//! Localized emoji search index (port of lib/emoji_i18n.js).
//!
//! The original fetched `/emoji-data/<locale>.json` (emojibase data) at
//! runtime and kept a mutable module-level "current locale". Here the same
//! files are embedded (`assets/emoji/names/<locale>.json`, byte-identical to
//! FrankMD's `public/emoji-data/`) and lookups are synchronous and stateless:
//! the caller keeps the app locale and passes it in.
//! Golden: `js_emoji_i18n.json`.

use serde::{Deserialize, Serialize};

/// **[COMPAT]** app locale → emojibase locale (`LOCALE_MAP`). Anything not
/// listed falls back to `"en"` (Hebrew is listed explicitly as `"en"`).
pub const LOCALE_MAP: [(&str, &str); 7] = [
    ("en", "en"),
    ("pt-BR", "pt"),
    ("pt-PT", "pt"),
    ("es", "es"),
    ("ja", "ja"),
    ("ko", "ko"),
    ("he", "en"),
];

/// Emojibase locale for an app locale (**[COMPAT]** `setEmojiLocale` +
/// `getEmojibaseLocale`). Exact, case-sensitive match on [`LOCALE_MAP`];
/// `None`, `""`, unknown codes (`"fr"`, `"EN"`, bare `"pt"`) → `"en"`.
///
/// Deliberate deviation: the JS looked the key up on a plain object, so
/// `"toString"` / `"__proto__"` returned inherited prototype values (a function,
/// `{}`) instead of a locale. Here they fall back to `"en"` like any unknown code.
pub fn emojibase_locale(app_locale: Option<&str>) -> &'static str {
    todo!()
}

/// An emojibase `emoticon` field: one string or several.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(missing_docs)]
pub enum EmojibaseEmoticon {
    One(String),
    Many(Vec<String>),
}

/// One emojibase row as stored in `assets/emoji/names/<locale>.json`.
/// Every field is optional because `buildSearchIndex` accepts partial rows
/// (`{emoji, label}`, `{label}` only, ...). Serializes back with absent fields
/// omitted, so a row round-trips to the original JSON object.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EmojibaseEntry {
    /// `1F600`, `1F3F4-E0067-...`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hexcode: Option<String>,
    /// Localized label (`grinning face`, `rosto sorridente`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The emoji (preferred key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode: Option<String>,
    /// Alternative emoji key, used when `unicode` is missing or empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// Emojibase group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<u32>,
    /// Emojibase order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
    /// Localized tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// ASCII emoticon(s) (`:D`, `["xD","XD"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoticon: Option<EmojibaseEmoticon>,
    /// Skin-tone variants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skins: Option<Vec<EmojibaseEntry>>,
}

/// Parsed dataset for an emojibase locale (`en`, `es`, `ja`, `ko`, `pt`),
/// from [`super::localized_names_json`], cached per locale. `None` for a
/// locale without data.
pub fn dataset(emojibase_locale: &str) -> Option<&'static [EmojibaseEntry]> {
    todo!("parse once per locale; needs serde_json as a normal dependency")
}

/// **[COMPAT]** `getTranslatedEmojis()` for an app locale: the dataset of
/// [`emojibase_locale`]`(app_locale)`, falling back to English when that
/// locale has no data (the JS fallback after a failed fetch).
pub fn translated_emojis(app_locale: Option<&str>) -> &'static [EmojibaseEntry] {
    todo!()
}

/// One search-index value (**[COMPAT]** `{label, tags, searchTerms}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchEntry {
    /// Label, `""` when missing.
    pub label: String,
    /// Tags, empty when missing.
    pub tags: Vec<String>,
    /// `[label, ...tags].join(" ").to_lowercase()` — note a missing label
    /// leaves a leading space (`" love"`), kept for compatibility.
    pub search_terms: String,
}

/// Insertion-ordered `emoji -> SearchEntry` map (**[COMPAT]** JS `Map`
/// semantics: a duplicate emoji overwrites the value but keeps the position
/// of its first insertion).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchIndex {
    entries: Vec<(String, SearchEntry)>,
}

impl SearchIndex {
    /// Entry for an emoji.
    pub fn get(&self, emoji: &str) -> Option<&SearchEntry> {
        todo!()
    }
    /// Number of distinct emoji.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &SearchEntry)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// **[COMPAT]** `buildSearchIndex(emojibaseData)`. The key is `unicode`, or
/// `emoji` when `unicode` is missing/empty (JS truthiness); rows with neither
/// are skipped.
pub fn build_search_index(entries: &[EmojibaseEntry]) -> SearchIndex {
    todo!()
}

/// Picker search over the built-in shortcode table (**[COMPAT]**
/// `emoji_picker_controller#searchEmojisWithI18n`): `query` is split on
/// whitespace; a row matches if every term is a substring of
/// `"<shortcode> <keywords>"`, or — when `index` has the row's emoji — every
/// term is a substring of its `search_terms`. Tests for this live with the
/// emoji-picker port (UI agent).
pub fn search_emojis(query: &str, index: Option<&SearchIndex>) -> Vec<&'static super::Emoji> {
    todo!()
}
