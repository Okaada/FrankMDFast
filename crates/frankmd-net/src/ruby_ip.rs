//! A faithful port of the parts of Ruby's `IPAddr` that `EgressPolicy` relies
//! on: `IPAddr.new(string)` parsing and `IPAddr#to_s` formatting.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ipnet::IpNet;

use crate::NetError;

/// Ruby's `IPAddr::InvalidAddressError` message for anything that is not an address.
const INVALID_ADDRESS: &str = "invalid address";
/// Ruby's message for a zero-filled IPv4 octet (`01.2.3.4`).
const ZERO_FILLED: &str = "zero-filled number in IPv4 address is ambiguous";
/// Ruby's `IPAddr::InvalidPrefixError` message for an out-of-range prefix.
const INVALID_LENGTH: &str = "invalid length";

/// An address parsed like Ruby's `IPAddr.new`: a network (the address masked by
/// its prefix; a plain address has a full-length prefix) plus an optional IPv6
/// zone id (`%eth0`), which Ruby keeps and prints back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedIp {
    net: IpNet,
    zone: Option<String>,
}

impl ParsedIp {
    /// The (masked) network.
    pub fn net(&self) -> &IpNet {
        &self.net
    }

    /// The (masked) address, i.e. Ruby's `IPAddr#to_i` as an address.
    pub fn addr(&self) -> IpAddr {
        self.net.addr()
    }

    /// The IPv6 zone id including its `%`, if any.
    pub fn zone(&self) -> Option<&str> {
        self.zone.as_deref()
    }
}

/// `IPAddr#to_s` (the address only; Ruby does not print the prefix).
impl fmt::Display for ParsedIp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&ruby_to_s(self.addr()))?;
        if let Some(zone) = &self.zone {
            f.write_str(zone)?;
        }
        Ok(())
    }
}

/// Ruby's `IPAddr#to_s` for a plain address.
///
/// It matches Rust's `Display` (RFC 5952 compression) except that Ruby also
/// prints IPv4-compatible addresses with a dotted tail (`::127.0.0.1`), not
/// only IPv4-mapped ones.
pub fn ruby_to_s(ip: IpAddr) -> String {
    let IpAddr::V6(v6) = ip else {
        return ip.to_string();
    };
    let rust = v6.to_string();
    let seg = v6.segments();
    // Ruby: /\A::(ffff:)?([\da-f]{1,4}):([\da-f]{1,4})\z/ → dotted quad.
    // Rust already prints the `::ffff:` (mapped) case dotted; handle `::x:y`.
    let compat = seg[..6].iter().all(|s| *s == 0) && seg[6] != 0;
    if compat {
        let [a, b] = seg[6].to_be_bytes();
        let [c, d] = seg[7].to_be_bytes();
        return format!("::{a}.{b}.{c}.{d}");
    }
    rust
}

/// Port of `IPAddr.new(string)`: plain v4/v6, `[v6]`, a `%zone` suffix and an
/// `addr/prefix` suffix (masked to the network address).
pub(crate) fn parse(input: &str) -> Result<ParsedIp, NetError> {
    let (mut addr, prefix) = match input.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (input, None),
    };
    let mut force_v6 = false;
    if let Some(inner) = addr.strip_prefix('[').and_then(|a| a.strip_suffix(']')) {
        addr = inner;
        force_v6 = true;
    }
    let mut zone = None;
    if let Some((rest, z)) = split_zone(addr) {
        addr = rest;
        zone = Some(z.to_owned());
        force_v6 = true;
    }

    let v4 = if force_v6 { None } else { parse_v4(addr)? };
    let ip = match v4 {
        Some(v4) => IpAddr::V4(v4),
        None => IpAddr::V6(parse_v6(addr)?),
    };

    let max = if ip.is_ipv4() { 32 } else { 128 };
    let len = match prefix {
        None => max,
        Some(p) => parse_prefix(p, max)?,
    };
    let net = IpNet::new(ip, len)
        .map_err(|_| invalid(INVALID_LENGTH))?
        .trunc();
    Ok(ParsedIp { net, zone })
}

/// Ruby: `/\A(.*)(%\w+)\z/` (greedy, so the last `%` starts the zone).
fn split_zone(addr: &str) -> Option<(&str, &str)> {
    let idx = addr.rfind('%')?;
    let zone = &addr[idx..];
    let word = &zone[1..];
    let is_word = !word.is_empty() && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    is_word.then(|| (&addr[..idx], zone))
}

/// Ruby's `in_addr`: `None` when the string is not v4-shaped
/// (`/\A(\d+)\.(\d+)\.(\d+)\.(\d+)\z/`), an error for a bad octet.
fn parse_v4(addr: &str) -> Result<Option<Ipv4Addr>, NetError> {
    let parts: Vec<&str> = addr.split('.').collect();
    let shaped = parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    if !shaped {
        return Ok(None);
    }
    let mut octets = [0u8; 4];
    for (slot, part) in octets.iter_mut().zip(&parts) {
        // `s.to_i < 256` is checked before the zero-fill rule.
        let n: u8 = part.parse().map_err(|_| invalid(INVALID_ADDRESS))?;
        if *part != "0" && part.starts_with('0') {
            return Err(invalid(ZERO_FILLED));
        }
        *slot = n;
    }
    Ok(Some(Ipv4Addr::from(octets)))
}

/// Ruby's `in6_addr`. Rust's parser accepts the same RFC 4291 text forms.
fn parse_v6(addr: &str) -> Result<Ipv6Addr, NetError> {
    addr.parse().map_err(|_| invalid(INVALID_ADDRESS))
}

/// Ruby's `mask!` for a numeric prefix length.
fn parse_prefix(prefix: &str, max: u8) -> Result<u8, NetError> {
    if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(INVALID_ADDRESS));
    }
    match prefix.parse::<u8>() {
        Ok(len) if len <= max => Ok(len),
        _ => Err(invalid(INVALID_LENGTH)),
    }
}

/// Ruby re-raises as `"#{message}: #{input}"`; the golden fixture records only
/// the prefix (the input is never echoed, so it cannot leak into logs).
fn invalid(message: &str) -> NetError {
    NetError::InvalidAddress(format!("{message}: "))
}
