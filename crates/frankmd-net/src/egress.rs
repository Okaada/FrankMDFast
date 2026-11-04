//! Egress policy (**[COMPAT]** port of `app/services/egress_policy.rb`;
//! golden `tests/fixtures/golden/egress_policy.json`).
//!
//! DNS is behind the [`HostResolver`] trait so tests (and the golden
//! "resolve stubbed" cases) can inject answers; the production client uses
//! [`SystemResolver`] wrapped by [`GuardedResolver`].

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use crate::NetError;

/// Resolves a host name to addresses. Mirrors `EgressPolicy.resolve(host)`.
///
/// `host` is passed exactly like Ruby's `uri.host` (IPv6 literals keep their
/// brackets, e.g. `"[::1]"`).
pub trait HostResolver: Send + Sync + std::fmt::Debug {
    /// Every address the host resolves to (empty = does not resolve).
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, NetError>;
}

/// The OS resolver (`getaddrinfo`; numeric hosts follow glibc `inet_aton`
/// forms such as `127.1`, `0x7f.0.0.1`, `2130706433`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemResolver;

impl HostResolver for SystemResolver {
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, NetError> {
        todo!("backend.md §8")
    }
}

/// Fixed host → addresses table (tests, golden "resolve stubbed" cases).
/// Unknown hosts resolve to nothing. Records every host it was asked for.
#[derive(Debug, Default)]
pub struct StaticResolver {
    table: HashMap<String, Vec<IpAddr>>,
    calls: std::sync::Mutex<Vec<String>>,
}

impl StaticResolver {
    /// Empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add (or replace) an entry.
    pub fn with(mut self, host: &str, addrs: &[IpAddr]) -> Self {
        self.table.insert(host.to_owned(), addrs.to_vec());
        self
    }

    /// Hosts passed to [`HostResolver::resolve`], in call order.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().map(|c| c.clone()).unwrap_or_default()
    }
}

impl HostResolver for StaticResolver {
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, NetError> {
        if let Ok(mut c) = self.calls.lock() {
            c.push(host.to_owned());
        }
        Ok(self.table.get(host).cloned().unwrap_or_default())
    }
}

/// `true` iff `ip` is a public unicast address (rejects loopback, RFC1918,
/// link-local, CGNAT, TEST-NETs, 240/4, multicast, IPv4-mapped/compat IPv6
/// forms of those, ULA, ...).
pub fn is_public(ip: IpAddr) -> bool {
    todo!("backend.md §8")
}

/// **[COMPAT]** `EgressPolicy.blocked_ip?` (`!is_public`).
pub fn is_blocked(ip: IpAddr) -> bool {
    !is_public(ip)
}

/// Parse an address string the way Ruby's `IPAddr.new` does (golden
/// `blocked_ip?` cases): plain v4/v6, `[v6]`, and `addr/prefix` (masked to the
/// network address). Rejects glibc shorthand (`127.1`, `0x7f.0.0.1`,
/// `2130706433`) and zero-filled octets. Errors are
/// [`NetError::InvalidAddress`] with Ruby's message prefix (`"invalid address: "`,
/// `"zero-filled number in IPv4 address is ambiguous: "`).
pub fn parse_ip(s: &str) -> Result<ipnet::IpNet, NetError> {
    todo!()
}

/// `blocked_ip?` on a parsed `IPAddr` that may carry a prefix
/// (**[COMPAT]** quirk: `::/0` is *not* blocked, `0.0.0.0/0` is).
pub fn is_blocked_net(net: &ipnet::IpNet) -> bool {
    todo!()
}

/// What `checked_target` hands back, mirroring Ruby's `URI` accessors so the
/// golden fixture can be compared field by field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// `uri.to_s` (scheme lowercased, everything else verbatim).
    pub uri: String,
    /// `uri.scheme` (lowercased).
    pub scheme: String,
    /// `uri.host` (IPv6 keeps brackets; case preserved).
    pub host: String,
    /// `uri.hostname` (brackets stripped).
    pub hostname: String,
    /// `uri.port` (default port when absent; Ruby accepts > 65535).
    pub port: Option<u32>,
    /// `uri.path` (`""` when absent).
    pub path: String,
    /// `uri.query`.
    pub query: Option<String>,
    /// `uri.userinfo` (**[COMPAT]** accepted by the original policy).
    pub userinfo: Option<String>,
    /// `uri.fragment`.
    pub fragment: Option<String>,
    /// First resolved (vetted) address, for pinning.
    pub ip: IpAddr,
}

/// **[COMPAT]** exact port of `EgressPolicy.checked_target`, with Ruby
/// `URI.parse` (RFC 3986, no IDN, no spaces) semantics. Messages:
/// `"Invalid URL"`, `"Only http and https URLs can be fetched"`,
/// `"URL has no host"`, `"Could not resolve <host>"`,
/// `"Refusing to fetch a non-public address (<ip>)"`.
pub fn check_target(url: &str, resolver: &dyn HostResolver) -> Result<Target, NetError> {
    todo!("backend.md §8")
}

/// Validate a user URL before any I/O: http/https only, no userinfo, host present.
pub fn check_url(url: &url::Url) -> Result<(), NetError> {
    todo!()
}

/// **[COMPAT]** `EgressPolicy.checked_target(url)`: parse, check scheme/host,
/// resolve through `resolver`, refuse if **any** address is blocked
/// (split horizon), and return the URL plus the first vetted address so the
/// caller can pin the connection. Errors are [`NetError::Blocked`] carrying
/// the original's exact messages.
pub fn checked_target(url: &str, resolver: &dyn HostResolver) -> Result<(url::Url, IpAddr), NetError> {
    todo!("backend.md §8 (thin wrapper over check_target)")
}

/// A `reqwest::dns::Resolve` implementation that filters every resolved
/// address through [`is_public`] (wired in when `EgressClient` is built).
#[derive(Debug, Clone)]
pub struct GuardedResolver {
    inner: Arc<dyn HostResolver>,
}

impl GuardedResolver {
    /// Guard `inner`.
    pub fn new(inner: Arc<dyn HostResolver>) -> Self {
        Self { inner }
    }
}

impl Default for GuardedResolver {
    fn default() -> Self {
        Self::new(Arc::new(SystemResolver))
    }
}
