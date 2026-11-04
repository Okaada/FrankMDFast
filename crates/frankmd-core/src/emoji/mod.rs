//! Emoji and emoticon data reused from FrankMD (`assets/emoji/`, extracted
//! from `app/javascript/lib/emoji_data.js` and the localized name tables).
//! Feeds the emoji/emoticon pickers and `:shortcode:` rendering in the
//! preview. Golden: `js_emoji_data.json`, `js_emoji_i18n.json`.

pub mod i18n;
pub mod icons;

/// `{source, format: "[shortcode, emoji, keywords]", data: [...]}`.
pub const SHORTCODES_JSON: &str = include_str!("../../../../assets/emoji/shortcodes.json");
/// Emoticon / kaomoji list.
pub const EMOTICONS_JSON: &str = include_str!("../../../../assets/emoji/emoticons.json");

/// Localized emoji labels for picker search, by FrankMD language
/// (`en`, `es`, `ja`, `ko`, `pt`; pt-BR and pt-PT share `pt`).
pub fn localized_names_json(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "en" => include_str!("../../../../assets/emoji/names/en.json"),
        "es" => include_str!("../../../../assets/emoji/names/es.json"),
        "ja" => include_str!("../../../../assets/emoji/names/ja.json"),
        "ko" => include_str!("../../../../assets/emoji/names/ko.json"),
        "pt" => include_str!("../../../../assets/emoji/names/pt.json"),
        _ => return None,
    })
}

use std::collections::HashMap;

/// One emoji row of `EMOJI_DATA` (`[shortcode, emoji, keywords]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emoji {
    /// Shortcode without colons (`grinning`).
    pub shortcode: String,
    /// The emoji itself.
    pub emoji: String,
    /// Space-separated search keywords.
    pub keywords: String,
}

/// One emoticon / kaomoji row (`[name, emoticon, keywords]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emoticon {
    /// Name (`happy`).
    pub name: String,
    /// The emoticon text (`(◕‿◕)`), inserted verbatim.
    pub text: String,
    /// Space-separated search keywords.
    pub keywords: String,
}

/// The shortcode table in source order (**[COMPAT]** `EMOJI_DATA`, 675 rows).
/// Parsed once from [`SHORTCODES_JSON`] and cached for the process lifetime.
pub fn shortcodes() -> &'static [Emoji] {
    todo!("parse SHORTCODES_JSON once (OnceLock); needs serde_json as a normal dependency")
}

/// The emoticon table in source order, parsed once from [`EMOTICONS_JSON`].
pub fn emoticons() -> &'static [Emoticon] {
    todo!("parse EMOTICONS_JSON once")
}

/// `shortcode -> emoji` (**[COMPAT]** `getEmojiMap()`): for duplicate
/// shortcodes the LAST row wins. Built once; every call returns the same map.
pub fn emoji_map() -> &'static HashMap<&'static str, &'static str> {
    todo!()
}

/// Resolve `:shortcode:` → emoji (**[COMPAT]** preview behaviour). Same
/// semantics as [`emoji_map`]`.get(shortcode)`; `shortcode` has no colons.
pub fn lookup(shortcode: &str) -> Option<&'static str> {
    todo!()
}
