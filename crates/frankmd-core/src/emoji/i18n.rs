//! Localized emoji search index (port of lib/emoji_i18n.js).
//!
//! The original fetched `/emoji-data/<locale>.json` (emojibase data) at
//! runtime and kept a mutable module-level "current locale". Here the same
//! files are embedded (`assets/emoji/names/<locale>.json`, byte-identical to
//! FrankMD's `public/emoji-data/`) and lookups are synchronous and stateless:
//! the caller keeps the app locale and passes it in.
//! Golden: `js_emoji_i18n.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

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
    app_locale
        .and_then(|code| LOCALE_MAP.iter().find(|(app, _)| *app == code))
        .map_or("en", |(_, emojibase)| emojibase)
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

/// Emojibase locales with embedded data (see [`super::localized_names_json`]).
const DATASET_LOCALES: [&str; 5] = ["en", "es", "ja", "ko", "pt"];

/// Parsed dataset for an emojibase locale (`en`, `es`, `ja`, `ko`, `pt`),
/// from [`super::localized_names_json`], cached per locale. `None` for a
/// locale without data.
pub fn dataset(emojibase_locale: &str) -> Option<&'static [EmojibaseEntry]> {
    static CACHE: [OnceLock<Option<Vec<EmojibaseEntry>>>; DATASET_LOCALES.len()] =
        [const { OnceLock::new() }; DATASET_LOCALES.len()];
    let slot = DATASET_LOCALES
        .iter()
        .position(|l| *l == emojibase_locale)?;
    CACHE[slot]
        .get_or_init(|| {
            let json = super::localized_names_json(emojibase_locale)?;
            // Embedded and covered by tests; unparsable data counts as missing
            // (the JS treated a failed load the same way).
            serde_json::from_str(json).ok()
        })
        .as_deref()
}

/// **[COMPAT]** `getTranslatedEmojis()` for an app locale: the dataset of
/// [`emojibase_locale`]`(app_locale)`, falling back to English when that
/// locale has no data (the JS fallback after a failed fetch).
pub fn translated_emojis(app_locale: Option<&str>) -> &'static [EmojibaseEntry] {
    dataset(emojibase_locale(app_locale))
        .or_else(|| dataset("en"))
        .unwrap_or_default()
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
    /// emoji -> index into `entries`, for O(1) lookups and overwrites.
    positions: HashMap<String, usize>,
}

impl SearchIndex {
    /// Entry for an emoji.
    pub fn get(&self, emoji: &str) -> Option<&SearchEntry> {
        self.positions.get(emoji).map(|&pos| &self.entries[pos].1)
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
    let mut index = SearchIndex::default();
    for entry in entries {
        let Some(emoji) = non_empty(&entry.unicode).or_else(|| non_empty(&entry.emoji)) else {
            continue;
        };
        let label = entry.label.clone().unwrap_or_default();
        let tags = entry.tags.clone().unwrap_or_default();
        let mut terms = Vec::with_capacity(tags.len() + 1);
        terms.push(label.as_str());
        terms.extend(tags.iter().map(String::as_str));
        let search_terms = terms.join(" ").to_lowercase();
        let value = SearchEntry {
            label,
            tags,
            search_terms,
        };
        match index.positions.get(emoji) {
            Some(&pos) => index.entries[pos].1 = value,
            None => {
                index
                    .positions
                    .insert(emoji.to_owned(), index.entries.len());
                index.entries.push((emoji.to_owned(), value));
            }
        }
    }
    index
}

/// JS truthiness for an optional string: `Some` only when non-empty.
fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|s| !s.is_empty())
}

/// Picker search over the built-in shortcode table (**[COMPAT]**
/// `emoji_picker_controller#searchEmojisWithI18n`): `query` is split on
/// whitespace; a row matches if every term is a substring of
/// `"<shortcode> <keywords>"`, or — when `index` has the row's emoji — every
/// term is a substring of its `search_terms`. The query is trimmed and
/// lowercased first, as the picker's `onInput` did. Tests for this live with
/// the emoji-picker port (UI agent).
pub fn search_emojis(query: &str, index: Option<&SearchIndex>) -> Vec<&'static super::Emoji> {
    let query = query.trim().to_lowercase();
    let terms: Vec<&str> = query.split_whitespace().collect();
    super::shortcodes()
        .iter()
        .filter(|row| {
            let english = format!("{} {}", row.shortcode, row.keywords).to_lowercase();
            if terms.iter().all(|t| english.contains(t)) {
                return true;
            }
            index
                .and_then(|idx| idx.get(&row.emoji))
                .is_some_and(|entry| terms.iter().all(|t| entry.search_terms.contains(t)))
        })
        .collect()
}
