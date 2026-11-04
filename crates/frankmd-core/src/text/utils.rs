//! Generic text helpers (port of `lib/text_utils.js`): HTML escaping, fuzzy
//! scoring, safe image URLs, Levenshtein distance.
//!
//! **[COMPAT]** `fuzzy_score` and `levenshtein_distance` compare and count in
//! UTF-16 code units, exactly like the JS (`str[i]`, `.length`, `charAt`), so
//! scores and distances match the original for non-BMP text (an emoji counts
//! as 2). Golden: `js_text_utils.json`.

/// Escape `& < > " '` for safe interpolation into HTML text *and* attributes.
/// `'` becomes `&#39;`. `None` (JS `null`/`undefined`/falsy) yields `""`.
pub fn escape_html(text: Option<&str>) -> String {
    todo!("text_utils.js escapeHtml")
}

/// DOM-free variant used by older call sites: same as [`escape_html`] but `'`
/// becomes `&#039;` (**[COMPAT]**, the two spellings are both observable).
pub fn escape_html_string(text: Option<&str>) -> String {
    todo!("text_utils.js escapeHtmlString")
}

/// Whether `url` may be used as an image `src`.
///
/// Rejects any whitespace or C0/DEL control character anywhere. Accepts
/// (case-insensitive) `http:`, `https:`, `data:image/`, `blob:` prefixes and
/// root-relative paths that are not protocol-relative (`//`, `/\`).
pub fn is_safe_image_url(url: &str) -> bool {
    todo!("text_utils.js isSafeImageUrl")
}

/// File-finder fuzzy match score; `0` = no match, higher = better.
///
/// Per query unit: +1, consecutive-run bonus (+2, +4, ... while consecutive),
/// +3 at string start or after `/`, `-`, `_`; finally
/// `+ max(0, 10 - (len(str) - len(query)))`. Case-sensitive. Lengths and
/// indices are UTF-16 code units (**[COMPAT]**).
pub fn fuzzy_score(s: &str, query: &str) -> u32 {
    todo!("text_utils.js fuzzyScore")
}

/// Levenshtein edit distance over UTF-16 code units (**[COMPAT]**).
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    todo!("text_utils.js levenshteinDistance")
}
