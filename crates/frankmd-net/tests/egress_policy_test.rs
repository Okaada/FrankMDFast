//! Port of `test/services/egress_policy_test.rb` (11 tests, all category A).
//! `EgressPolicy.stubs(:resolve)` becomes an injected [`StaticResolver`] /
//! [`FixedResolver`].

use std::net::IpAddr;

use frankmd_net::NetError;
use frankmd_net::egress::{HostResolver, StaticResolver, checked_target, is_blocked};

/// `EgressPolicy.stubs(:resolve).returns([...])`: same answer for every host.
#[derive(Debug)]
struct FixedResolver(Vec<IpAddr>);

impl HostResolver for FixedResolver {
    fn resolve(&self, _host: &str) -> Result<Vec<IpAddr>, NetError> {
        Ok(self.0.clone())
    }
}

fn ip(s: &str) -> IpAddr {
    s.parse().expect("test address")
}

fn blocked_message(r: Result<(url::Url, IpAddr), NetError>) -> String {
    match r {
        Err(NetError::Blocked(msg)) => msg,
        other => panic!("expected BlockedError, got {other:?}"),
    }
}

// port: test/services/egress_policy_test.rb:8 "rejects a non-http scheme"
#[test]
fn rejects_a_non_http_scheme() {
    let msg = blocked_message(checked_target("file:///etc/passwd", &StaticResolver::new()));
    assert!(msg.contains("http"), "{msg}");
}

// port: test/services/egress_policy_test.rb:15 "rejects a URL with no host"
#[test]
fn rejects_a_url_with_no_host() {
    blocked_message(checked_target("http://", &StaticResolver::new()));
}

// port: test/services/egress_policy_test.rb:19 "rejects a host that does not resolve"
#[test]
fn rejects_a_host_that_does_not_resolve() {
    blocked_message(checked_target("https://does-not-exist.invalid/x.png", &FixedResolver(vec![])));
}

// port: test/services/egress_policy_test.rb:29 "blocks the cloud metadata address"
#[test]
fn blocks_the_cloud_metadata_address() {
    assert!(is_blocked(ip("169.254.169.254")));
}

// port: test/services/egress_policy_test.rb:33 "blocks loopback, private and link-local addresses"
#[test]
fn blocks_loopback_private_and_link_local_addresses() {
    for addr in ["127.0.0.1", "10.0.0.5", "172.16.0.1", "192.168.1.10", "169.254.1.1", "::1", "fc00::1", "fe80::1"] {
        assert!(is_blocked(ip(addr)), "{addr} should be blocked");
    }
}

// port: test/services/egress_policy_test.rb:39 "blocks reserved, unspecified and multicast ranges"
#[test]
fn blocks_reserved_unspecified_and_multicast_ranges() {
    for addr in
        ["0.0.0.0", "100.64.0.1", "192.0.2.1", "198.18.0.1", "203.0.113.1", "240.0.0.1", "224.0.0.1", "ff02::1"]
    {
        assert!(is_blocked(ip(addr)), "{addr} should be blocked");
    }
}

// port: test/services/egress_policy_test.rb:45 "blocks an IPv4-mapped IPv6 loopback"
#[test]
fn blocks_an_ipv4_mapped_ipv6_loopback() {
    // ::ffff:127.0.0.1 must be judged on its IPv4 value, not as a plain v6 address
    assert!(is_blocked(ip("::ffff:127.0.0.1")));
}

// port: test/services/egress_policy_test.rb:50 "allows a public address"
#[test]
fn allows_a_public_address() {
    assert!(!is_blocked(ip("93.184.216.34")));
    assert!(!is_blocked(ip("2606:2800:220:1:248:1893:25c8:1946")));
}

// port: test/services/egress_policy_test.rb:57 "rejects a host that resolves to a private address"
#[test]
fn rejects_a_host_that_resolves_to_a_private_address() {
    let msg = blocked_message(checked_target(
        "http://metadata.example.com/latest/meta-data/",
        &FixedResolver(vec![ip("169.254.169.254")]),
    ));
    assert!(msg.contains("non-public"), "{msg}");
}

// port: test/services/egress_policy_test.rb:66 "rejects when any resolved address is non-public"
#[test]
fn rejects_when_any_resolved_address_is_non_public() {
    // A host resolving to both a public and a private address must be refused.
    blocked_message(checked_target(
        "https://split-horizon.example.com/x.png",
        &FixedResolver(vec![ip("93.184.216.34"), ip("127.0.0.1")]),
    ));
}

// port: test/services/egress_policy_test.rb:75 "returns the uri and the resolved ip for a public host"
#[test]
fn returns_the_uri_and_the_resolved_ip_for_a_public_host() {
    let resolver = StaticResolver::new().with("example.com", &[ip("93.184.216.34")]);
    let (uri, addr) = checked_target("https://example.com/image.png", &resolver).expect("public host");
    assert_eq!(uri.host_str(), Some("example.com"));
    assert_eq!(uri.path(), "/image.png");
    assert_eq!(addr.to_string(), "93.184.216.34");
    assert_eq!(resolver.calls(), vec!["example.com".to_owned()]);
}
