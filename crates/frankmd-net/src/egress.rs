//! Egress policy (**[COMPAT]** port of `app/services/egress_policy.rb`;
//! golden `tests/fixtures/golden/egress_policy.json`).
//!
//! DNS is behind the [`HostResolver`] trait so tests (and the golden
//! "resolve stubbed" cases) can inject answers; the production client uses
//! [`SystemResolver`] wrapped by [`GuardedResolver`].
//!
//! One deliberate deviation from the original (security decision): URLs that
//! carry userinfo (`http://user:pass@host/`) are refused, see
//! [`USERINFO_MESSAGE`].

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex, PoisonError};

use ipnet::{IpNet, Ipv4Net, Ipv6Net};

use crate::NetError;
pub use crate::ruby_ip::{ParsedIp, ruby_to_s};
use crate::ruby_uri;

/// `EgressPolicy::ALLOWED_SCHEMES`.
pub const ALLOWED_SCHEMES: [&str; 2] = ["http", "https"];

/// Message for a URL with embedded credentials. Not in the original, which
/// accepted them; refusing them keeps credentials out of requests and logs and
/// closes the `http://trusted.com@evil.com/` confusion trick.
pub const USERINFO_MESSAGE: &str = "URLs with embedded credentials cannot be fetched";

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
///
/// Blocking: call it off the async executor (the client uses
/// `spawn_blocking`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemResolver;

impl HostResolver for SystemResolver {
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, NetError> {
        // Ruby: `Addrinfo.getaddrinfo(host, nil, nil, :STREAM)`, and any
        // SocketError means "no addresses". std asks for SOCK_STREAM too.
        let addrs = match (host, 0u16).to_socket_addrs() {
            Ok(it) => it.map(|sa| sa.ip()).collect(),
            Err(_) => Vec::new(),
        };
        Ok(addrs)
    }
}

/// Fixed host → addresses table (tests, golden "resolve stubbed" cases).
/// Unknown hosts resolve to nothing. Records every host it was asked for.
#[derive(Debug, Default)]
pub struct StaticResolver {
    table: HashMap<String, Vec<IpAddr>>,
    calls: Mutex<Vec<String>>,
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

// --- address classification ----------------------------------------------

/// `EgressPolicy::EXTRA_BLOCKED`, IPv4 part.
const EXTRA_BLOCKED_V4: [(Ipv4Addr, u8); 8] = [
    (Ipv4Addr::new(0, 0, 0, 0), 8),       // "this" network
    (Ipv4Addr::new(100, 64, 0, 0), 10),   // CGNAT
    (Ipv4Addr::new(192, 0, 0, 0), 24),    // IETF protocol assignments
    (Ipv4Addr::new(192, 0, 2, 0), 24),    // TEST-NET-1
    (Ipv4Addr::new(198, 18, 0, 0), 15),   // benchmarking
    (Ipv4Addr::new(198, 51, 100, 0), 24), // TEST-NET-2
    (Ipv4Addr::new(203, 0, 113, 0), 24),  // TEST-NET-3
    (Ipv4Addr::new(240, 0, 0, 0), 4),     // reserved
];

/// `EgressPolicy::EXTRA_BLOCKED`, IPv6 part.
const EXTRA_BLOCKED_V6: [(Ipv6Addr, u8); 2] = [
    (Ipv6Addr::UNSPECIFIED, 128),                    // unspecified
    (Ipv6Addr::new(0x100, 0, 0, 0, 0, 0, 0, 0), 64), // discard-only
];

/// `true` iff `ip` is a public unicast address under the original's rules
/// (rejects loopback, RFC1918, link-local, CGNAT, TEST-NETs, 240/4, multicast,
/// IPv4-mapped forms of those, ULA, ...).
pub fn is_public(ip: IpAddr) -> bool {
    !is_blocked_net(&IpNet::from(ip))
}

/// **[COMPAT]** `EgressPolicy.blocked_ip?` (`!is_public`).
pub fn is_blocked(ip: IpAddr) -> bool {
    !is_public(ip)
}

/// Parse an address string the way Ruby's `IPAddr.new` does (golden
/// `blocked_ip?` cases): plain v4/v6, `[v6]`, `%zone`, and `addr/prefix`
/// (masked to the network address). Rejects glibc shorthand (`127.1`,
/// `0x7f.0.0.1`, `2130706433`) and zero-filled octets. Errors are
/// [`NetError::InvalidAddress`] with Ruby's message prefix (`"invalid address: "`,
/// `"zero-filled number in IPv4 address is ambiguous: "`).
pub fn parse_ip(s: &str) -> Result<ParsedIp, NetError> {
    crate::ruby_ip::parse(s)
}

/// `blocked_ip?` on a parsed `IPAddr` that may carry a prefix
/// (**[COMPAT]** quirk: `::/0` is *not* blocked, `0.0.0.0/0` is).
///
/// Ruby's predicates (`loopback?`, `private?`, `link_local?`, `to_i == 0`,
/// the multicast nibble) look at the network address only, while
/// `range.include?(net)` requires the whole network to lie inside `range`.
pub fn is_blocked_net(net: &IpNet) -> bool {
    match net {
        IpNet::V4(v4) => is_blocked_v4(v4),
        IpNet::V6(v6) => match v6.addr().to_ipv4_mapped() {
            // IPv4-mapped IPv6 (::ffff:127.0.0.1) is judged on the IPv4 value.
            Some(mapped) => {
                let len = v6.prefix_len().saturating_sub(96);
                Ipv4Net::new(mapped, len).is_ok_and(|n| is_blocked_v4(&n))
            }
            None => is_blocked_v6(v6),
        },
    }
}

fn is_blocked_v4(net: &Ipv4Net) -> bool {
    let a = net.addr();
    let o = a.octets();
    let loopback = o[0] == 127;
    let private =
        o[0] == 10 || (o[0] == 172 && (o[1] & 0xf0) == 16) || (o[0] == 192 && o[1] == 168);
    let link_local = o[0] == 169 && o[1] == 254;
    let zero = u32::from(a) == 0;
    let extra = EXTRA_BLOCKED_V4
        .iter()
        .any(|&(addr, len)| Ipv4Net::new(addr, len).is_ok_and(|r| r.contains(net)));
    let multicast = (u32::from(a) >> 28) == 0b1110;
    loopback || private || link_local || zero || extra || multicast
}

fn is_blocked_v6(net: &Ipv6Net) -> bool {
    let a = net.addr();
    let first = a.segments()[0];
    let loopback = a == Ipv6Addr::LOCALHOST;
    let private = (first & 0xfe00) == 0xfc00; // fc00::/7
    let link_local = (first & 0xffc0) == 0xfe80; // fe80::/10
    let extra = EXTRA_BLOCKED_V6
        .iter()
        .any(|&(addr, len)| Ipv6Net::new(addr, len).is_ok_and(|r| r.contains(net)));
    let multicast =
        Ipv6Net::new(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 0), 8).is_ok_and(|r| r.contains(net));
    loopback || private || link_local || extra || multicast
}

// --- URL checks -------------------------------------------------------------

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
    /// `uri.userinfo`. Always `None` on success: the policy refuses
    /// userinfo (security decision; the original accepted it).
    pub userinfo: Option<String>,
    /// `uri.fragment`.
    pub fragment: Option<String>,
    /// First resolved (vetted) address, for pinning.
    pub ip: IpAddr,
}

fn blocked(msg: impl Into<String>) -> NetError {
    NetError::Blocked(msg.into())
}

fn default_port(scheme: &str) -> u32 {
    if scheme == "https" { 443 } else { 80 }
}

/// **[COMPAT]** exact port of `EgressPolicy.checked_target`, with Ruby
/// `URI.parse` (RFC 3986, no IDN, no spaces) semantics, plus the userinfo
/// refusal. Messages: `"Invalid URL"`, `"Only http and https URLs can be
/// fetched"`, `"URL has no host"`, [`USERINFO_MESSAGE`], `"Could not resolve
/// <host>"`, `"Refusing to fetch a non-public address (<ip>)"`.
pub fn check_target(url: &str, resolver: &dyn HostResolver) -> Result<Target, NetError> {
    let parts = ruby_uri::parse(url).ok_or_else(|| blocked("Invalid URL"))?;

    let scheme = parts
        .scheme
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !ALLOWED_SCHEMES.contains(&scheme.as_str()) {
        return Err(blocked("Only http and https URLs can be fetched"));
    }
    let host = parts.host.clone().unwrap_or_default();
    if host.is_empty() {
        return Err(blocked("URL has no host"));
    }
    // security: the original accepted userinfo; it is refused before any DNS.
    if parts.userinfo.is_some() {
        return Err(blocked(USERINFO_MESSAGE));
    }
    let port = match parts.port.as_deref() {
        None | Some("") => default_port(&scheme),
        Some(digits) => digits.parse().map_err(|_| blocked("Invalid URL"))?,
    };

    let addresses = resolver.resolve(&host).unwrap_or_default();
    let Some(&first) = addresses.first() else {
        return Err(blocked(format!("Could not resolve {host}")));
    };
    if let Some(&bad) = addresses.iter().find(|&&ip| is_blocked(ip)) {
        return Err(blocked(format!(
            "Refusing to fetch a non-public address ({})",
            ruby_to_s(bad)
        )));
    }

    let path = parts.path.clone().unwrap_or_default();
    let hostname = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(&host)
        .to_owned();
    Ok(Target {
        uri: ruby_to_s_uri(
            &scheme,
            &host,
            port,
            &path,
            parts.query.as_deref(),
            parts.fragment.as_deref(),
        ),
        scheme,
        host,
        hostname,
        port: Some(port),
        path,
        query: parts.query,
        userinfo: None,
        fragment: parts.fragment,
        ip: first,
    })
}

/// `URI::Generic#to_s` for a hierarchical http(s) URI without userinfo.
fn ruby_to_s_uri(
    scheme: &str,
    host: &str,
    port: u32,
    path: &str,
    query: Option<&str>,
    fragment: Option<&str>,
) -> String {
    let mut s = format!("{scheme}://{host}");
    if port != default_port(scheme) {
        s.push_str(&format!(":{port}"));
    }
    s.push_str(path);
    if let Some(q) = query {
        s.push('?');
        s.push_str(q);
    }
    if let Some(f) = fragment {
        s.push('#');
        s.push_str(f);
    }
    s
}

/// Validate an already-parsed URL before any I/O: http/https only, no
/// userinfo, host present.
pub fn check_url(url: &url::Url) -> Result<(), NetError> {
    if !ALLOWED_SCHEMES.contains(&url.scheme()) {
        return Err(blocked("Only http and https URLs can be fetched"));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(blocked("URL has no host"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(blocked(USERINFO_MESSAGE));
    }
    Ok(())
}

/// **[COMPAT]** `EgressPolicy.checked_target(url)`: parse, check scheme/host,
/// resolve through `resolver`, refuse if **any** address is blocked
/// (split horizon), and return the URL plus the first vetted address so the
/// caller can pin the connection. Errors are [`NetError::Blocked`] carrying
/// the original's exact messages.
///
/// Defense in depth: the URL is re-parsed with the WHATWG parser that reqwest
/// uses; it must pass [`check_url`], and if that parser reads the host as an
/// IP literal (reqwest then skips DNS) that address must be public too.
pub fn checked_target(
    url: &str,
    resolver: &dyn HostResolver,
) -> Result<(url::Url, IpAddr), NetError> {
    let target = check_target(url, resolver)?;
    let parsed = url::Url::parse(&target.uri).map_err(|_| blocked("Invalid URL"))?;
    check_url(&parsed)?;
    let literal = match parsed.host() {
        Some(url::Host::Ipv4(v4)) => Some(IpAddr::V4(v4)),
        Some(url::Host::Ipv6(v6)) => Some(IpAddr::V6(v6)),
        _ => None,
    };
    if let Some(ip) = literal.filter(|ip| is_blocked(*ip)) {
        return Err(blocked(format!(
            "Refusing to fetch a non-public address ({})",
            ruby_to_s(ip)
        )));
    }
    Ok((parsed, target.ip))
}

// --- the resolver reqwest uses ---------------------------------------------

/// A pinned answer for one host, shared by the in-flight requests to it.
#[derive(Debug, Clone, Copy)]
struct Pin {
    addr: SocketAddr,
    users: usize,
}

/// A `reqwest::dns::Resolve` implementation for [`crate::EgressClient`].
///
/// Before each request the client vets the URL with [`checked_target`] and
/// *pins* the host to the vetted address, so reqwest connects to exactly that
/// address (no second lookup, no DNS rebinding). A host that was not pinned
/// is resolved through `inner` and refused unless every answer is public.
#[derive(Debug, Clone)]
pub struct GuardedResolver {
    inner: Arc<dyn HostResolver>,
    pins: Arc<Mutex<HashMap<String, Pin>>>,
}

impl GuardedResolver {
    /// Guard `inner`.
    pub fn new(inner: Arc<dyn HostResolver>) -> Self {
        Self {
            inner,
            pins: Arc::default(),
        }
    }

    /// Pin `host` to `addr` until the returned guard is dropped.
    pub(crate) fn pin(&self, host: &str, addr: SocketAddr) -> PinGuard {
        let key = host.to_ascii_lowercase();
        let mut pins = self.pins.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = pins.entry(key.clone()).or_insert(Pin { addr, users: 0 });
        // Concurrent fetches may vet different addresses; both are public.
        entry.addr = addr;
        entry.users += 1;
        PinGuard {
            pins: Arc::clone(&self.pins),
            key,
        }
    }

    fn pinned(&self, host: &str) -> Option<SocketAddr> {
        let pins = self.pins.lock().unwrap_or_else(PoisonError::into_inner);
        pins.get(&host.to_ascii_lowercase()).map(|p| p.addr)
    }
}

impl Default for GuardedResolver {
    fn default() -> Self {
        Self::new(Arc::new(SystemResolver))
    }
}

/// Releases a [`GuardedResolver`] pin when dropped.
#[derive(Debug)]
pub(crate) struct PinGuard {
    pins: Arc<Mutex<HashMap<String, Pin>>>,
    key: String,
}

impl Drop for PinGuard {
    fn drop(&mut self) {
        let mut pins = self.pins.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(pin) = pins.get_mut(&self.key) {
            pin.users = pin.users.saturating_sub(1);
            if pin.users == 0 {
                pins.remove(&self.key);
            }
        }
    }
}

impl reqwest::dns::Resolve for GuardedResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        if let Some(addr) = self.pinned(&host) {
            return Box::pin(
                async move { Ok(Box::new(std::iter::once(addr)) as reqwest::dns::Addrs) },
            );
        }
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let addrs =
                tokio::task::spawn_blocking(move || inner.resolve(&host).unwrap_or_default())
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            if addrs.is_empty() || addrs.iter().any(|ip| is_blocked(*ip)) {
                return Err(Box::new(blocked("Refusing to fetch a non-public address"))
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            let it = addrs.into_iter().map(|ip| SocketAddr::new(ip, 0));
            Ok(Box::new(it) as reqwest::dns::Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn userinfo_is_refused_before_resolving() {
        let r = StaticResolver::new().with("example.com", &["93.184.216.34".parse().expect("ip")]);
        let err = check_target("http://user:pass@example.com/", &r).expect_err("refused");
        assert!(
            matches!(err, NetError::Blocked(ref m) if m == USERINFO_MESSAGE),
            "{err:?}"
        );
        assert!(r.calls().is_empty());
    }

    #[test]
    fn check_url_refuses_userinfo_and_other_schemes() {
        let ok = url::Url::parse("https://example.com/x").expect("url");
        assert!(check_url(&ok).is_ok());
        for bad in [
            "https://u@example.com/",
            "https://:p@example.com/",
            "ftp://example.com/",
        ] {
            let u = url::Url::parse(bad).expect("url");
            assert!(check_url(&u).is_err(), "{bad}");
        }
    }

    #[test]
    fn pins_are_released_when_the_last_guard_drops() {
        let g = GuardedResolver::default();
        let addr: SocketAddr = "93.184.216.34:0".parse().expect("addr");
        let a = g.pin("Example.com", addr);
        let b = g.pin("example.com", addr);
        drop(a);
        assert_eq!(g.pinned("example.com"), Some(addr));
        drop(b);
        assert_eq!(g.pinned("example.com"), None);
    }
}
