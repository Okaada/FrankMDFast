//! **[COMPAT]** port of the web image search actions of
//! `app/controllers/images_controller.rb` (`search_web`, `search_google`,
//! `search_pinterest`) plus the search flags of `GET /images/config`.
//!
//! Configured endpoints (Google CSE, DuckDuckGo) go through
//! `frankmd_net::TrustedClient`; base URLs are injectable through
//! [`SearchEndpoints`]. Result URLs are only *listed* here; they are fetched
//! later through the import pipeline (egress-guarded), as in the original.

use crate::config::MediaConfig;

/// DuckDuckGo browser `User-Agent` the original sent on both requests.
pub const DDG_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
/// Max DuckDuckGo results returned (`first(20)`).
pub const DDG_MAX_RESULTS: usize = 20;
/// vqd token patterns, tried in order.
pub const VQD_PATTERNS: [&str; 3] = [r#"vqd=["']?([\d-]+)["']?"#, r"vqd=([\d-]+)", r#""vqd":"([\d-]+)""#];

/// Base URLs (tests point them at wiremock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchEndpoints {
    /// `https://duckduckgo.com` (`/?q=..&iax=images&ia=images`, then `/i.js`).
    pub duckduckgo: String,
    /// `https://www.googleapis.com` (`/customsearch/v1`).
    pub google: String,
}

impl Default for SearchEndpoints {
    fn default() -> Self {
        Self { duckduckgo: "https://duckduckgo.com".into(), google: "https://www.googleapis.com".into() }
    }
}

/// One result image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebImage {
    /// Full image URL (DDG `image` / Google `link`).
    pub url: String,
    /// Thumbnail (Google: `image.thumbnailLink` or `link`).
    pub thumbnail: Option<String>,
    /// Title.
    pub title: Option<String>,
    /// Source host (DDG `source` / Google `displayLink`).
    pub source: Option<String>,
    /// Width.
    pub width: Option<u64>,
    /// Height.
    pub height: Option<u64>,
}

/// A 200 response body: `{images, total?, next_start?, error?, note?}`.
/// Upstream failures are *not* errors: they come back as empty `images` plus
/// `error` (Google's own message, e.g. `quotaExceeded`) or `note`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchResults {
    /// Results.
    pub images: Vec<WebImage>,
    /// Google: `searchInformation.totalResults.to_i`.
    pub total: Option<u64>,
    /// Google: `start + 10`.
    pub next_start: Option<i64>,
    /// `"Failed to fetch images"`, `"Failed to parse results"`, `"Search failed"`, Google's message.
    pub error: Option<String>,
    /// `"DuckDuckGo search temporarily unavailable"` (no vqd token).
    pub note: Option<String>,
}

/// Request-level errors (HTTP status in the original), carrying i18n keys.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SearchError {
    /// 400 `errors.query_required` (blank after strip).
    #[error("errors.query_required")]
    QueryRequired,
    /// 503: `errors.google_not_configured` / `errors.youtube_not_configured`.
    #[error("{0}")]
    NotConfigured(&'static str),
    /// 500 `errors.search_failed`.
    #[error("errors.search_failed")]
    Failed,
}

impl SearchError {
    /// i18n key of the user-visible message.
    pub fn i18n_key(&self) -> &'static str {
        match self {
            SearchError::QueryRequired => "errors.query_required",
            SearchError::NotConfigured(k) => k,
            SearchError::Failed => "errors.search_failed",
        }
    }
}

/// Search flags of `GET /images/config`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchFlags {
    /// Always `true` (DuckDuckGo, no key).
    pub web_search_enabled: bool,
    /// `google_api_key` and `google_cse_id` present.
    pub google_enabled: bool,
    /// Always `true` (DuckDuckGo + `site:pinterest.com`).
    pub pinterest_enabled: bool,
}

/// `google_api_configured?`.
pub fn google_configured(config: &MediaConfig) -> bool {
    todo!()
}

/// Flags for the config endpoint.
pub fn search_flags(config: &MediaConfig) -> SearchFlags {
    todo!()
}

/// Web image search client.
#[derive(Debug, Clone)]
pub struct ImageSearch {
    http: frankmd_net::TrustedClient,
    endpoints: SearchEndpoints,
}

impl ImageSearch {
    /// Bind a client to endpoints.
    pub fn new(http: frankmd_net::TrustedClient, endpoints: SearchEndpoints) -> Self {
        Self { http, endpoints }
    }

    /// `GET /images/search_web?q=`: DuckDuckGo (vqd page, then `i.js` with
    /// `l=us-en o=json q vqd f=,,,,, p=1`).
    pub async fn search_web(&self, q: &str) -> Result<SearchResults, SearchError> {
        todo!()
    }

    /// `GET /images/search_pinterest?q=`: DuckDuckGo with `"<q> site:pinterest.com"`.
    pub async fn search_pinterest(&self, q: &str) -> Result<SearchResults, SearchError> {
        todo!()
    }

    /// `GET /images/search_google?q=&start=`: blank → 400, unconfigured → 503;
    /// `key cx q searchType=image num=10 start=max(start,1) safe=active`.
    pub async fn search_google(&self, config: &MediaConfig, q: &str, start: i64) -> Result<SearchResults, SearchError> {
        todo!()
    }
}
