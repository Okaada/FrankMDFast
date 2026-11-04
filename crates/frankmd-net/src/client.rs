//! HTTP clients.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{CONTENT_TYPE, LOCATION, USER_AGENT};

use crate::egress::{GuardedResolver, HostResolver, checked_target};

/// **[COMPAT]** `User-Agent` the original sent when fetching external images.
pub const EGRESS_USER_AGENT: &str = "Mozilla/5.0 (compatible; FrankMD/1.0)";
/// `User-Agent` for configured API endpoints.
pub const TRUSTED_USER_AGENT: &str = concat!("FrankMD/", env!("CARGO_PKG_VERSION"));
/// **[COMPAT]** `http.open_timeout = 10`.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// **[COMPAT]** `http.read_timeout = 30` (external fetches).
const EGRESS_READ_TIMEOUT: Duration = Duration::from_secs(30);
/// Read timeout for configured endpoints (AI completions can be slow).
const TRUSTED_READ_TIMEOUT: Duration = Duration::from_secs(120);
/// Redirect hops a configured endpoint may take.
const TRUSTED_MAX_REDIRECTS: usize = 5;

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
    pub const EXTERNAL_IMAGE: Limits = Limits {
        max_bytes: 25 * 1024 * 1024,
        timeout: Duration::from_secs(30),
        max_redirects: 5,
    };
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
    guard: GuardedResolver,
    dial_override: Option<SocketAddr>,
}

impl NetError {
    /// Map a reqwest error without its URL (it may carry user data).
    fn from_reqwest(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            return NetError::Timeout;
        }
        NetError::Transport(e.without_url().to_string())
    }
}

impl TrustedClient {
    /// Build (rustls, gzip, no proxy, default timeouts).
    pub fn new() -> Result<Self, NetError> {
        let inner = reqwest::Client::builder()
            .no_proxy()
            .user_agent(TRUSTED_USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(TRUSTED_READ_TIMEOUT)
            .redirect(reqwest::redirect::Policy::limited(TRUSTED_MAX_REDIRECTS))
            .build()
            .map_err(NetError::from_reqwest)?;
        Ok(Self { inner })
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
    pub fn with_resolver(
        limits: Limits,
        resolver: Arc<dyn HostResolver>,
    ) -> Result<Self, NetError> {
        let guard = GuardedResolver::new(Arc::clone(&resolver));
        let inner = reqwest::Client::builder()
            .no_proxy()
            // Redirects are followed by hand so every hop is re-vetted.
            .redirect(reqwest::redirect::Policy::none())
            .dns_resolver(Arc::new(guard.clone()))
            .user_agent(EGRESS_USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(EGRESS_READ_TIMEOUT)
            .build()
            .map_err(NetError::from_reqwest)?;
        Ok(Self {
            inner,
            limits,
            resolver,
            guard,
            dial_override: None,
        })
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
        match tokio::time::timeout(self.limits.timeout, self.fetch_hops(url)).await {
            Ok(result) => result,
            Err(_elapsed) => Err(NetError::Timeout),
        }
    }

    /// The redirect loop (`(MAX_EXTERNAL_IMAGE_REDIRECTS + 1).times`).
    async fn fetch_hops(&self, url: &str) -> Result<Fetched, NetError> {
        let mut current = url.to_owned();
        let mut redirects = 0;
        loop {
            let (target, ip) = self.vet(&current).await?;
            match self.fetch_one(&target, ip).await? {
                Hop::Done(fetched) => return Ok(fetched),
                Hop::Redirect(next) => {
                    if redirects == self.limits.max_redirects {
                        return Err(NetError::TooManyRedirects);
                    }
                    redirects += 1;
                    current = next;
                }
            }
        }
    }

    /// `EgressPolicy.checked_target`, off the async executor (DNS blocks).
    async fn vet(&self, url: &str) -> Result<(url::Url, IpAddr), NetError> {
        let resolver = Arc::clone(&self.resolver);
        let url = url.to_owned();
        tokio::task::spawn_blocking(move || checked_target(&url, resolver.as_ref()))
            .await
            .map_err(|e| NetError::Transport(e.to_string()))?
    }

    /// One request to a vetted target, connected to the vetted address.
    async fn fetch_one(&self, target: &url::Url, ip: IpAddr) -> Result<Hop, NetError> {
        let _pin = target.host_str().map(|host| {
            let addr = self.dial_override.unwrap_or(SocketAddr::new(ip, 0));
            self.guard.pin(host, addr)
        });
        let mut response = self
            .inner
            .get(target.clone())
            .header(USER_AGENT, EGRESS_USER_AGENT)
            .send()
            .await
            .map_err(NetError::from_reqwest)?;
        let status = response.status();

        if status.is_redirection() {
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .filter(|v| !v.is_empty());
            if let Some(location) = location {
                // `URI.join(uri, response["location"])`
                let next = target
                    .join(location)
                    .map_err(|_| NetError::Blocked("Invalid URL".to_owned()))?;
                return Ok(Hop::Redirect(next.to_string()));
            }
        }
        if !status.is_success() {
            return Err(NetError::Status(status.as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|len| len > self.limits.max_bytes)
        {
            return Err(NetError::TooLarge);
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(NetError::from_reqwest)? {
            bytes.extend_from_slice(&chunk);
            if bytes.len() as u64 > self.limits.max_bytes {
                return Err(NetError::TooLarge);
            }
        }
        Ok(Hop::Done(Fetched {
            url: target.clone(),
            content_type,
            bytes,
        }))
    }
}

/// Outcome of one request in the redirect loop.
enum Hop {
    Done(Fetched),
    Redirect(String),
}
