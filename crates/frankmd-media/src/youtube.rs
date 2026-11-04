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
    crate::search::present(config.youtube_api_key.as_deref())
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
        let query = crate::search::ruby_strip(q);
        if query.is_empty() {
            return Err(SearchError::QueryRequired);
        }
        let key = match config.youtube_api_key.as_deref() {
            Some(key) if enabled(config) => key,
            _ => return Err(SearchError::NotConfigured("errors.youtube_not_configured")),
        };
        let max = MAX_RESULTS.to_string();
        let query_string = crate::search::form_query(&[
            ("part", "snippet"),
            ("q", query),
            ("type", "video"),
            ("maxResults", &max),
            ("key", key),
        ]);
        let url = format!("{}/youtube/v3/search?{query_string}", self.base.trim_end_matches('/'));
        let response = self.http.reqwest().get(url).send().await.map_err(|_| SearchError::Failed)?;
        if !response.status().is_success() {
            return Ok(VideoResults { videos: Vec::new(), error: Some("YouTube API error".into()) });
        }
        let body = response.text().await.map_err(|_| SearchError::Failed)?;
        let data: serde_json::Value = serde_json::from_str(&body).map_err(|_| SearchError::Failed)?;
        let text = |item: &serde_json::Value, pointer: &str| {
            item.pointer(pointer).and_then(serde_json::Value::as_str).map(str::to_owned)
        };
        let videos = data
            .get("items")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| Video {
                        id: text(item, "/id/videoId"),
                        title: text(item, "/snippet/title"),
                        channel: text(item, "/snippet/channelTitle"),
                        thumbnail: text(item, "/snippet/thumbnails/medium/url")
                            .or_else(|| text(item, "/snippet/thumbnails/default/url")),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(VideoResults { videos, error: None })
    }
}
