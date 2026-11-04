//! **[COMPAT]** port of `app/javascript/lib/url_utils.js`. Golden: `js_url_utils.json`.

/// `encodePath(path)`: `encodeURIComponent` on every `/`-separated segment
/// (UTF-8 percent-encoding, unreserved set `A-Z a-z 0-9 - _ . ! ~ * ' ( )`);
/// `None`/empty → `""`.
pub fn encode_path(path: Option<&str>) -> String {
    todo!()
}

/// `extractYouTubeId(url)`: first match of
/// `(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/embed/|youtube\.com/v/)([a-zA-Z0-9_-]{11})`
/// (unanchored), else the whole input if it is exactly an 11-char id; else `None`.
pub fn extract_youtube_id(url: Option<&str>) -> Option<String> {
    todo!()
}
