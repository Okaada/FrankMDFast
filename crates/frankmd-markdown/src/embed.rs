//! Allowlisted embeds. YouTube links/iframes become thumbnail cards (no
//! webview on Wayland); clicking opens the browser.

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
pub fn parse_youtube(url: &str) -> Option<Embed> {
    todo!()
}
