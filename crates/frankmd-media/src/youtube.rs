//! **[COMPAT]** port of `app/controllers/youtube_controller.rb` (JSON side; the
//! HTML partial is Rails view plumbing).

use crate::config::MediaConfig;
use crate::search::SearchError;

/// `maxResults`.
pub const MAX_RESULTS: u32 = 6;

/// One video result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    /// `id.videoId`.
    pub id: Option<String>,
    /// `snippet.title`.
    pub title: Option<String>,
    /// `snippet.channelTitle`.
    pub channel: Option<String>,
    /// `thumbnails.medium.url` or `thumbnails.default.url`.
    pub thumbnail: Option<String>,
}

/// `{videos, error?}` (an API failure is still a 200 with `error: "YouTube API error"`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VideoResults {
    /// Videos.
    pub videos: Vec<Video>,
    /// Set when the API answered non-2xx.
    pub error: Option<String>,
}

/// `GET /youtube/config` → `{enabled: youtube_api_key present}`.
pub fn enabled(config: &MediaConfig) -> bool {
    todo!()
}

/// YouTube Data API client.
#[derive(Debug, Clone)]
pub struct YoutubeSearch {
    http: frankmd_net::TrustedClient,
    /// `https://www.googleapis.com` (`/youtube/v3/search`).
    base: String,
}

impl YoutubeSearch {
    /// Production base URL.
    pub fn new(http: frankmd_net::TrustedClient) -> Self {
        Self::with_base(http, "https://www.googleapis.com")
    }

    /// Injected base URL (tests).
    pub fn with_base(http: frankmd_net::TrustedClient, base: impl Into<String>) -> Self {
        Self { http, base: base.into() }
    }

    /// `GET /youtube/search?q=`: strip, blank → [`SearchError::QueryRequired`];
    /// no key → `NotConfigured("errors.youtube_not_configured")`; request
    /// `part=snippet q type=video maxResults=6 key`; transport/parse failure →
    /// [`SearchError::Failed`].
    pub async fn search(&self, config: &MediaConfig, q: &str) -> Result<VideoResults, SearchError> {
        todo!()
    }
}
