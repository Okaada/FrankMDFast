//! Generic text helpers (port of `lib/text_utils.js`): HTML escaping, fuzzy
//! scoring, safe image URLs, Levenshtein distance.
//!
//! **[COMPAT]** `fuzzy_score` and `levenshtein_distance` compare and count in
//! UTF-16 code units, exactly like the JS (`str[i]`, `.length`, `charAt`), so
//! scores and distances match the original for non-BMP text (an emoji counts
//! as 2). Golden: `js_text_utils.json`.

use super::is_js_whitespace;

/// Escape `& < > " '` for safe interpolation into HTML text *and* attributes.
/// `'` becomes `&#39;`. `None` (JS `null`/`undefined`/falsy) yields `""`.
pub fn escape_html(text: Option<&str>) -> String {
    escape_with(text, "&#39;")
}

/// DOM-free variant used by older call sites: same as [`escape_html`] but `'`
/// becomes `&#039;` (**[COMPAT]**, the two spellings are both observable).
pub fn escape_html_string(text: Option<&str>) -> String {
    escape_with(text, "&#039;")
}

/// Whether `url` may be used as an image `src`.
///
/// Rejects any whitespace or C0/DEL control character anywhere. Accepts
/// (case-insensitive) `http:`, `https:`, `data:image/`, `blob:` prefixes and
/// root-relative paths that are not protocol-relative (`//`, `/\`).
pub fn is_safe_image_url(url: &str) -> bool {
    if url
        .chars()
        .any(|c| is_js_whitespace(c) || c.is_ascii_control())
    {
        return false;
    }
    let lower_prefix = |p: &str| {
        url.get(..p.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(p))
    };
    ["http:", "https:", "data:image/", "blob:"]
        .into_iter()
        .any(lower_prefix)
        || (url.starts_with('/') && !url.starts_with("//") && !url.starts_with("/\\"))
}

/// File-finder fuzzy match score; `0` = no match, higher = better.
///
/// Per query unit: +1, consecutive-run bonus (+2, +4, ... while consecutive),
/// +3 at string start or after `/`, `-`, `_`; finally
/// `+ max(0, 10 - (len(str) - len(query)))`. Case-sensitive. Lengths and
/// indices are UTF-16 code units (**[COMPAT]**).
pub fn fuzzy_score(s: &str, query: &str) -> u32 {
    let hay: Vec<u16> = s.encode_utf16().collect();
    let needle: Vec<u16> = query.encode_utf16().collect();
    let mut score: i64 = 0;
    let mut from = 0usize;
    let mut prev_match: Option<usize> = None;
    let mut consecutive_bonus: i64 = 0;

    for &unit in &needle {
        let Some(found) = hay
            .get(from..)
            .and_then(|rest| rest.iter().position(|&u| u == unit))
            .map(|p| p + from)
        else {
            return 0;
        };
        score += 1;
        if prev_match.map_or(found == 0, |p| found == p + 1) {
            consecutive_bonus += 2;
            score += consecutive_bonus;
        } else {
            consecutive_bonus = 0;
        }
        let after_separator = found
            .checked_sub(1)
            .is_some_and(|i| b"/-_".iter().any(|&sep| hay[i] == u16::from(sep)));
        if found == 0 || after_separator {
            score += 3;
        }
        prev_match = Some(found);
        from = found + 1;
    }

    let length_gap = hay.len() as i64 - needle.len() as i64;
    score += (10 - length_gap).max(0);
    u32::try_from(score).unwrap_or(u32::MAX)
}

/// Levenshtein edit distance over UTF-16 code units (**[COMPAT]**).
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<u16> = a.encode_utf16().collect();
    let b: Vec<u16> = b.encode_utf16().collect();
    // Single-row DP over `a`, iterating `b` in the outer loop like the JS.
    let mut prev: Vec<usize> = (0..=a.len()).collect();
    for (i, &bu) in b.iter().enumerate() {
        let mut row = Vec::with_capacity(a.len() + 1);
        row.push(i + 1);
        for (j, &au) in a.iter().enumerate() {
            let value = if bu == au {
                prev[j]
            } else {
                1 + prev[j].min(row[j]).min(prev[j + 1])
            };
            row.push(value);
        }
        prev = row;
    }
    prev[a.len()]
}

/// Shared body of the two escape helpers; only the `'` spelling differs.
fn escape_with(text: Option<&str>, apostrophe: &str) -> String {
    let Some(text) = text.filter(|t| !t.is_empty()) else {
        return String::new();
    };
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str(apostrophe),
            other => out.push(other),
        }
    }
    out
}
