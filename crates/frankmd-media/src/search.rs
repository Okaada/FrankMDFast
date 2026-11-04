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
pub const DDG_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
/// Max DuckDuckGo results returned (`first(20)`).
pub const DDG_MAX_RESULTS: usize = 20;
/// vqd token patterns, tried in order.
pub const VQD_PATTERNS: [&str; 3] = [
    r#"vqd=["']?([\d-]+)["']?"#,
    r"vqd=([\d-]+)",
    r#""vqd":"([\d-]+)""#,
];

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
        Self {
            duckduckgo: "https://duckduckgo.com".into(),
            google: "https://www.googleapis.com".into(),
        }
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
    present(config.google_api_key.as_deref()) && present(config.google_cse_id.as_deref())
}

/// Flags for the config endpoint.
pub fn search_flags(config: &MediaConfig) -> SearchFlags {
    SearchFlags {
        web_search_enabled: true,
        google_enabled: google_configured(config),
        pinterest_enabled: true,
    }
}

/// Rails `present?` on an optional string.
pub(crate) fn present(value: Option<&str>) -> bool {
    value.is_some_and(|v| !v.trim().is_empty())
}

/// Ruby `String#strip` (whitespace and NUL).
pub(crate) fn ruby_strip(s: &str) -> &str {
    s.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\0'))
}

/// Ruby `URI.encode_www_form_component` (`application/x-www-form-urlencoded`).
pub(crate) fn form_component(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// Ruby `URI.encode_www_form` over `(key, value)` pairs.
pub(crate) fn form_query(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", form_component(k), form_component(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Ruby `String#to_i` on a JSON value (numbers kept, strings parsed by
/// their leading digits, anything else 0).
fn json_to_i(value: Option<&serde_json::Value>) -> u64 {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
        Some(serde_json::Value::String(s)) => {
            let digits: String = s
                .trim_start()
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse().unwrap_or(0)
        }
        _ => 0,
    }
}

/// A JSON string field.
fn str_field(item: &serde_json::Value, key: &str) -> Option<String> {
    item.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

/// Why a DuckDuckGo search failed (mapped to the original's messages).
enum DdgFailure {
    /// `JSON::ParserError` → "Failed to parse results".
    Parse,
    /// Anything else → "Search failed".
    Other,
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
        let query = ruby_strip(q);
        if query.is_empty() {
            return Err(SearchError::QueryRequired);
        }
        Ok(self.search_duckduckgo(query, None).await)
    }

    /// `GET /images/search_pinterest?q=`: DuckDuckGo with `"<q> site:pinterest.com"`.
    pub async fn search_pinterest(&self, q: &str) -> Result<SearchResults, SearchError> {
        let query = ruby_strip(q);
        if query.is_empty() {
            return Err(SearchError::QueryRequired);
        }
        Ok(self.search_duckduckgo(query, Some("pinterest.com")).await)
    }

    /// `GET /images/search_google?q=&start=`: blank → 400, unconfigured → 503;
    /// `key cx q searchType=image num=10 start=max(start,1) safe=active`.
    pub async fn search_google(
        &self,
        config: &MediaConfig,
        q: &str,
        start: i64,
    ) -> Result<SearchResults, SearchError> {
        let query = ruby_strip(q);
        if query.is_empty() {
            return Err(SearchError::QueryRequired);
        }
        if !google_configured(config) {
            return Err(SearchError::NotConfigured("errors.google_not_configured"));
        }
        let page = start.max(1).to_string();
        let query_string = form_query(&[
            ("key", config.google_api_key.as_deref().unwrap_or_default()),
            ("cx", config.google_cse_id.as_deref().unwrap_or_default()),
            ("q", query),
            ("searchType", "image"),
            ("num", "10"),
            ("start", &page),
            ("safe", "active"),
        ]);
        let url = format!(
            "{}/customsearch/v1?{query_string}",
            self.endpoints.google.trim_end_matches('/')
        );
        let response = self
            .http
            .reqwest()
            .get(url)
            .send()
            .await
            .map_err(|_| SearchError::Failed)?;
        let status = response.status();
        let body = response.text().await.map_err(|_| SearchError::Failed)?;
        if !status.is_success() {
            let fallback = format!("Google API error ({})", status.as_u16());
            let message = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|data| {
                    data.pointer("/error/message")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or(fallback);
            return Ok(SearchResults {
                error: Some(message),
                ..SearchResults::default()
            });
        }
        let data: serde_json::Value =
            serde_json::from_str(&body).map_err(|_| SearchError::Failed)?;
        let images = data
            .get("items")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        let link = str_field(item, "link");
                        let image = item.get("image").unwrap_or(&serde_json::Value::Null);
                        WebImage {
                            thumbnail: str_field(image, "thumbnailLink").or_else(|| link.clone()),
                            url: link.unwrap_or_default(),
                            title: str_field(item, "title"),
                            source: str_field(item, "displayLink"),
                            width: image.get("width").and_then(serde_json::Value::as_u64),
                            height: image.get("height").and_then(serde_json::Value::as_u64),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(SearchResults {
            images,
            total: Some(json_to_i(data.pointer("/searchInformation/totalResults"))),
            next_start: Some(start + 10),
            ..SearchResults::default()
        })
    }

    /// `search_duckduckgo_images(query, site_filter)`: never fails; problems
    /// come back as `note` / `error` on an empty result.
    async fn search_duckduckgo(&self, query: &str, site_filter: Option<&str>) -> SearchResults {
        let search_query = match site_filter {
            Some(site) => format!("{query} site:{site}"),
            None => query.to_owned(),
        };
        match self.duckduckgo_images(&search_query).await {
            Ok(results) => results,
            Err(DdgFailure::Parse) => SearchResults {
                error: Some("Failed to parse results".into()),
                ..SearchResults::default()
            },
            Err(DdgFailure::Other) => SearchResults {
                error: Some("Search failed".into()),
                ..SearchResults::default()
            },
        }
    }

    /// The two DuckDuckGo requests (vqd page, then `i.js`).
    async fn duckduckgo_images(&self, search_query: &str) -> Result<SearchResults, DdgFailure> {
        let base = self.endpoints.duckduckgo.trim_end_matches('/');
        let page_url = format!(
            "{base}/?q={}&iax=images&ia=images",
            form_component(search_query)
        );
        let page = self
            .http
            .reqwest()
            .get(page_url)
            .header("User-Agent", DDG_USER_AGENT)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|_| DdgFailure::Other)?
            .text()
            .await
            .map_err(|_| DdgFailure::Other)?;
        let Some(vqd) = extract_vqd(&page) else {
            return Ok(SearchResults {
                note: Some("DuckDuckGo search temporarily unavailable".into()),
                ..SearchResults::default()
            });
        };
        let query_string = form_query(&[
            ("l", "us-en"),
            ("o", "json"),
            ("q", search_query),
            ("vqd", &vqd),
            ("f", ",,,,,"),
            ("p", "1"),
        ]);
        let response = self
            .http
            .reqwest()
            .get(format!("{base}/i.js?{query_string}"))
            .header("User-Agent", DDG_USER_AGENT)
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .header("Referer", "https://duckduckgo.com/")
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|_| DdgFailure::Other)?;
        if !response.status().is_success() {
            return Ok(SearchResults {
                error: Some("Failed to fetch images".into()),
                ..SearchResults::default()
            });
        }
        let body = response.text().await.map_err(|_| DdgFailure::Other)?;
        let data: serde_json::Value = serde_json::from_str(&body).map_err(|_| DdgFailure::Parse)?;
        let images = data
            .get("results")
            .and_then(serde_json::Value::as_array)
            .map(|results| {
                results
                    .iter()
                    .take(DDG_MAX_RESULTS)
                    .map(|item| WebImage {
                        url: str_field(item, "image").unwrap_or_default(),
                        thumbnail: str_field(item, "thumbnail"),
                        title: str_field(item, "title"),
                        source: str_field(item, "source"),
                        width: item.get("width").and_then(serde_json::Value::as_u64),
                        height: item.get("height").and_then(serde_json::Value::as_u64),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(SearchResults {
            images,
            ..SearchResults::default()
        })
    }
}

/// First [`VQD_PATTERNS`] match in the DuckDuckGo page.
fn extract_vqd(page: &str) -> Option<String> {
    VQD_PATTERNS.iter().find_map(|pattern| {
        let re = regex::Regex::new(pattern).ok()?;
        re.captures(page)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_owned())
    })
}
