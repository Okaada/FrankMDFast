//! **[COMPAT]** port of `app/javascript/lib/url_utils.js`. Golden: `js_url_utils.json`.

/// `encodePath(path)`: `encodeURIComponent` on every `/`-separated segment
/// (UTF-8 percent-encoding, unreserved set `A-Z a-z 0-9 - _ . ! ~ * ' ( )`);
/// `None`/empty → `""`.
pub fn encode_path(path: Option<&str>) -> String {
    match path {
        None | Some("") => String::new(),
        Some(path) => path.split('/').map(encode_uri_component).collect::<Vec<_>>().join("/"),
    }
}

/// JS `encodeURIComponent` (UTF-8 bytes, unreserved `A-Z a-z 0-9 - _ . ! ~ * ' ( )`).
pub fn encode_uri_component(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `extractYouTubeId(url)`: first match of
/// `(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/embed/|youtube\.com/v/)([a-zA-Z0-9_-]{11})`
/// (unanchored), else the whole input if it is exactly an 11-char id; else `None`.
pub fn extract_youtube_id(url: Option<&str>) -> Option<String> {
    static PATTERNS: std::sync::LazyLock<[Option<regex::Regex>; 2]> = std::sync::LazyLock::new(|| {
        [
            regex::Regex::new(
                r"(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/embed/|youtube\.com/v/)([a-zA-Z0-9_-]{11})",
            )
            .ok(),
            regex::Regex::new(r"^([a-zA-Z0-9_-]{11})$").ok(),
        ]
    });
    let url = url.filter(|u| !u.is_empty())?;
    PATTERNS
        .iter()
        .flatten()
        .find_map(|re| re.captures(url).and_then(|c| c.get(1)).map(|m| m.as_str().to_owned()))
}
