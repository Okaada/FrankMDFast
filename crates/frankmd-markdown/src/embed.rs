//! Allowlisted embeds. YouTube links/iframes become thumbnail cards (no
//! webview on Wayland); clicking opens the browser.

use std::sync::LazyLock;

use regex::Regex;

/// An embed the preview knows how to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Embed {
    /// YouTube video by id; thumbnail from `i.ytimg.com/vi/<id>/hqdefault.jpg`.
    YouTube {
        /// 11-char video id.
        id: String,
        /// Start offset in seconds.
        start: Option<u32>,
    },
}

/// **[COMPAT]** recognize the YouTube URL forms FrankMD's marked extension accepts.
/// (`extractYouTubeId`: `watch?v=`, `youtu.be/`, `/embed/`, `/v/` or a bare
/// 11-char id), plus a `start=`/`t=` offset in seconds when present.
pub fn parse_youtube(url: &str) -> Option<Embed> {
    let id = YOUTUBE_URL
        .captures(url)
        .and_then(|c| c.get(1))
        .or_else(|| BARE_ID.captures(url).and_then(|c| c.get(1)))?
        .as_str()
        .to_owned();
    let start = START.captures(url).and_then(|c| c.get(1)).and_then(|m| m.as_str().parse().ok());
    Some(Embed::YouTube { id, start })
}

static YOUTUBE_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/embed/|youtube\.com/v/)([a-zA-Z0-9_-]{11})")
        .expect("valid regex")
});
static BARE_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9_-]{11})$").expect("valid regex"));
static START: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&](?:start|t)=(\d+)").expect("valid regex"));
static IFRAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)^\s*<iframe\s[^>]*?\bsrc\s*=\s*["']([^"']+)["'][^>]*>\s*</iframe>\s*$"#).expect("valid regex")
});

/// A raw HTML block that is exactly one `<iframe>` whose `src` is an
/// `https://` YouTube embed on a host the sanitizer allows.
pub(crate) fn from_html_block(html: &str) -> Option<Embed> {
    let src = IFRAME.captures(html)?.get(1)?.as_str();
    let rest = src.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let allowed = ["www.youtube.com", "youtube.com", "www.youtube-nocookie.com", "youtube-nocookie.com"];
    if !allowed.contains(&host.to_ascii_lowercase().as_str()) {
        return None;
    }
    let normalized = src.replace("youtube-nocookie.com", "youtube.com");
    parse_youtube(&normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_forms() {
        let e = |id: &str, start| Some(Embed::YouTube { id: id.into(), start });
        assert_eq!(parse_youtube("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42"), e("dQw4w9WgXcQ", Some(42)));
        assert_eq!(parse_youtube("https://youtu.be/dQw4w9WgXcQ"), e("dQw4w9WgXcQ", None));
        assert_eq!(parse_youtube("dQw4w9WgXcQ"), e("dQw4w9WgXcQ", None));
        assert_eq!(parse_youtube("https://example.com"), None);
        let iframe = r#"<iframe src="https://www.youtube.com/embed/dQw4w9WgXcQ" allowfullscreen></iframe>"#;
        assert_eq!(from_html_block(iframe), e("dQw4w9WgXcQ", None));
        assert_eq!(from_html_block(&iframe.replace("https://www.youtube.com", "https://evil.com/youtube.com")), None);
    }
}
