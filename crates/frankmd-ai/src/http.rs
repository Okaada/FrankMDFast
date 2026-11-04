//! JSON POST over `frankmd-net`'s trusted client, with ruby_llm's retry policy.
//!
//! Failures are reduced to a [`PostError`] that carries no upstream body and no
//! request data, so it is always safe to log.

use std::time::Duration;

use frankmd_net::TrustedClient;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use secrecy::{ExposeSecret, SecretString};

/// **[COMPAT]** ruby_llm `request_timeout` (300 s), applied per attempt.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
/// **[COMPAT]** ruby_llm `max_retries`.
const MAX_RETRIES: u32 = 3;
/// **[COMPAT]** ruby_llm `retry_interval` (doubled per attempt, `retry_backoff_factor` 2).
const RETRY_INTERVAL: Duration = Duration::from_millis(100);
/// **[COMPAT]** ruby_llm retries these statuses (plus transport errors).
const RETRY_STATUSES: [u16; 6] = [429, 500, 502, 503, 504, 529];
/// Response size cap. Generated images arrive base64-encoded inside the JSON.
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

/// One request header. Secret values are marked sensitive so reqwest never
/// prints them in `Debug` output.
pub(crate) enum Header {
    /// A plain header.
    Plain(&'static str, &'static str),
    /// A credential header (`Authorization: Bearer …`, `x-api-key`, …).
    Secret(&'static str, SecretString),
}

/// Why a POST failed. Never contains bodies, URLs or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PostError {
    /// Upstream answered with a non-success status.
    #[error("upstream status {0}")]
    Status(u16),
    /// Connection/TLS/read failure.
    #[error("transport error")]
    Transport,
    /// The request timed out.
    #[error("timed out")]
    Timeout,
    /// The response exceeded [`MAX_RESPONSE_BYTES`].
    #[error("response too large")]
    TooLarge,
    /// The response was not JSON.
    #[error("invalid JSON response")]
    Decode,
    /// A header value could not be built (e.g. a key with a newline).
    #[error("invalid request header")]
    BadHeader,
}

impl PostError {
    fn is_retryable(self) -> bool {
        match self {
            PostError::Status(code) => RETRY_STATUSES.contains(&code),
            PostError::Transport | PostError::Timeout => true,
            PostError::TooLarge | PostError::Decode | PostError::BadHeader => false,
        }
    }
}

/// POST `body` as JSON to `url` and parse the JSON answer, retrying transient
/// failures with exponential backoff.
pub(crate) async fn post_json(
    client: &TrustedClient,
    url: &str,
    headers: &[Header],
    body: &serde_json::Value,
) -> Result<serde_json::Value, PostError> {
    let headers = build_headers(headers)?;
    let payload = serde_json::to_vec(body).map_err(|_| PostError::Decode)?;
    let mut attempt = 0;
    loop {
        match post_once(client, url, &headers, &payload).await {
            Err(err) if err.is_retryable() && attempt < MAX_RETRIES => {
                tracing::debug!(attempt, error = %err, "AI request failed; retrying");
                tokio::time::sleep(RETRY_INTERVAL * 2u32.pow(attempt)).await;
                attempt += 1;
            }
            result => return result,
        }
    }
}

fn build_headers(headers: &[Header]) -> Result<HeaderMap, PostError> {
    let mut map = HeaderMap::new();
    map.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    for header in headers {
        let (name, value) = match header {
            Header::Plain(name, value) => (*name, HeaderValue::from_static(value)),
            Header::Secret(name, secret) => {
                let mut value = HeaderValue::from_str(secret.expose_secret())
                    .map_err(|_| PostError::BadHeader)?;
                value.set_sensitive(true);
                (*name, value)
            }
        };
        map.insert(HeaderName::from_static(name), value);
    }
    Ok(map)
}

async fn post_once(
    client: &TrustedClient,
    url: &str,
    headers: &HeaderMap,
    payload: &[u8],
) -> Result<serde_json::Value, PostError> {
    let mut response = client
        .reqwest()
        .post(url)
        .headers(headers.clone())
        .timeout(REQUEST_TIMEOUT)
        .body(payload.to_vec())
        .send()
        .await
        .map_err(classify)?;
    let status = response.status();
    if !status.is_success() {
        // The body is dropped unread: upstream errors may echo credentials (HP-011).
        return Err(PostError::Status(status.as_u16()));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(classify)? {
        if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(PostError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| PostError::Decode)
}

fn classify(err: reqwest::Error) -> PostError {
    if err.is_timeout() {
        PostError::Timeout
    } else {
        PostError::Transport
    }
}
