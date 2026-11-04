//! HTTP clients.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::egress::HostResolver;

/// Per-request caps (HP-017).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Max response body bytes (external image fetch: 25 MB, **[COMPAT]**).
    pub max_bytes: u64,
    /// Total timeout (deadline over all hops, monotonic clock).
    pub timeout: Duration,
    /// Max redirect hops (each re-validated).
    pub max_redirects: usize,
}

impl Limits {
    /// **[COMPAT]** external image fetch: `MAX_EXTERNAL_IMAGE_BYTES` = 25 MiB,
    /// `MAX_EXTERNAL_IMAGE_SECONDS` = 30, 5 redirects.
    pub const EXTERNAL_IMAGE: Limits =
        Limits { max_bytes: 25 * 1024 * 1024, timeout: Duration::from_secs(30), max_redirects: 5 };
}

/// Network errors, already safe to log (no secrets, no upstream bodies).
#[derive(Debug, thiserror::Error)]
pub enum NetError {
    /// URL rejected by policy (scheme, host, non-public address). Carries the
    /// original `EgressPolicy::BlockedError` message.
    #[error("{0}")]
    Blocked(String),
    /// Not an IP address (`IPAddr::InvalidAddressError` message).
    #[error("{0}")]
    InvalidAddress(String),
    /// Body exceeded `Limits::max_bytes`.
    #[error("response too large")]
    TooLarge,
    /// Timeout.
    #[error("timed out")]
    Timeout,
    /// Too many redirects.
    #[error("too many redirects")]
    TooManyRedirects,
    /// Upstream returned an error status (body intentionally dropped).
    #[error("upstream status {0}")]
    Status(u16),
    /// Transport error.
    #[error("transport: {0}")]
    Transport(String),
}

/// A fetched response (body already capped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    /// Final URL after redirects.
    pub url: url::Url,
    /// `Content-Type` header, verbatim.
    pub content_type: Option<String>,
    /// Body bytes.
    pub bytes: Vec<u8>,
}

/// Client for configured, trusted endpoints.
#[derive(Debug, Clone)]
pub struct TrustedClient {
    inner: reqwest::Client,
}

/// Client for user-supplied URLs (SSRF-guarded).
#[derive(Debug, Clone)]
pub struct EgressClient {
    inner: reqwest::Client,
    limits: Limits,
    resolver: Arc<dyn HostResolver>,
    dial_override: Option<SocketAddr>,
}

impl TrustedClient {
    /// Build (rustls, gzip, no proxy, default timeouts).
    pub fn new() -> Result<Self, NetError> {
        todo!()
    }
    /// Escape hatch for SDK-style crates (genai/aws) that accept a client.
    pub fn reqwest(&self) -> &reqwest::Client {
        &self.inner
    }
}

impl EgressClient {
    /// Build with the system resolver behind the guard, custom redirect policy and `.no_proxy()`.
    pub fn new(limits: Limits) -> Result<Self, NetError> {
        Self::with_resolver(limits, Arc::new(crate::egress::SystemResolver))
    }

    /// Build with an injected resolver (tests: [`crate::egress::StaticResolver`]).
    /// Policy is applied to whatever `resolver` returns.
    pub fn with_resolver(limits: Limits, resolver: Arc<dyn HostResolver>) -> Result<Self, NetError> {
        todo!()
    }

    /// **Test seam.** After the policy has vetted the resolved address, connect to
    /// `addr` instead (e.g. a wiremock server on 127.0.0.1). The request keeps the
    /// original Host header / URL; policy decisions are unchanged.
    pub fn with_dial_override(mut self, addr: SocketAddr) -> Self {
        self.dial_override = Some(addr);
        self
    }

    /// The limits in force.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// GET a user URL: [`crate::egress::checked_target`] on the URL and on every
    /// redirect hop, `Content-Length` pre-check and streamed byte cap, overall
    /// deadline. Non-2xx → [`NetError::Status`].
    pub async fn fetch(&self, url: &str) -> Result<Fetched, NetError> {
        todo!()
    }
}
