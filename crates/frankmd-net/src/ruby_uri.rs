//! A port of Ruby 3.4's `URI::RFC3986_Parser#split` (the matcher behind
//! `URI.parse`), limited to what `EgressPolicy.checked_target` reads.
//!
//! The Ruby parser is two anchored regexes with possessive quantifiers
//! (`RFC3986_URI`, then `RFC3986_relative_ref`). Possessive repetition never
//! gives characters back, so each piece below is a greedy scan, and the only
//! backtracking left is between the `hier-part` alternatives.

/// The components `URI.parse` produced (strings exactly as matched).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UriParts {
    /// `uri.scheme`, as written (Ruby lowercases it afterwards).
    pub(crate) scheme: Option<String>,
    /// `uri.userinfo`.
    pub(crate) userinfo: Option<String>,
    /// `uri.host` (IP literals keep their brackets).
    pub(crate) host: Option<String>,
    /// The raw port digits (`""` when the URL has a bare `:`).
    pub(crate) port: Option<String>,
    /// `uri.path` (`None` for opaque URIs).
    pub(crate) path: Option<String>,
    /// `uri.query`.
    pub(crate) query: Option<String>,
    /// `uri.fragment`.
    pub(crate) fragment: Option<String>,
}

/// `URI.parse(s)`, or `None` where Ruby raises `URI::InvalidURIError`.
pub(crate) fn parse(s: &str) -> Option<UriParts> {
    if !s.is_ascii() {
        return None;
    }
    let b = s.as_bytes();
    absolute(b).or_else(|| relative(b))
}

// --- character classes (straight from the Ruby regex sources) -------------

fn is_unreserved_or_sub_delim(c: u8) -> bool {
    // `[!$&-.0-9;=A-Z_a-z~]` (`&-.` is the range `&'()*+,-.`)
    matches!(c, b'!' | b'$' | b'&'..=b'.' | b'0'..=b'9' | b';' | b'=' | b'A'..=b'Z' | b'_' | b'a'..=b'z' | b'~')
}

fn is_reg_name(c: u8) -> bool {
    is_unreserved_or_sub_delim(c)
}

fn is_userinfo(c: u8) -> bool {
    is_unreserved_or_sub_delim(c) || c == b':'
}

/// `SEG`: pchar plus `/`.
fn is_seg(c: u8) -> bool {
    is_unreserved_or_sub_delim(c) || matches!(c, b':' | b'@' | b'/')
}

/// `SEG_NC`: a segment character that is neither `:` nor `/`.
fn is_seg_nc(c: u8) -> bool {
    is_unreserved_or_sub_delim(c) || c == b'@'
}

fn is_fragment(c: u8) -> bool {
    is_seg(c) || c == b'?'
}

fn is_scheme_tail(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.')
}

/// Possessively match `(?:%\h\h|[class])*` from `start`; returns the end index.
fn scan(b: &[u8], start: usize, class: fn(u8) -> bool) -> usize {
    let mut i = start;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = b.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                && b.get(i + 2).is_some_and(u8::is_ascii_hexdigit);
            if !hex {
                break;
            }
            i += 3;
        } else if class(b[i]) {
            i += 1;
        } else {
            break;
        }
    }
    i
}

fn text(b: &[u8], from: usize, to: usize) -> String {
    String::from_utf8_lossy(&b[from..to]).into_owned()
}

// --- shared pieces --------------------------------------------------------

/// `(?:\?(?<query>[^#]*+))?(?:\#(?<fragment>FRAGMENT))?\z` from `i`.
fn tail(b: &[u8], mut i: usize) -> Option<(Option<String>, Option<String>)> {
    let mut query = None;
    if b.get(i) == Some(&b'?') {
        let start = i + 1;
        let end = b[start..]
            .iter()
            .position(|&c| c == b'#')
            .map_or(b.len(), |p| start + p);
        query = Some(text(b, start, end));
        i = end;
    }
    let mut fragment = None;
    if b.get(i) == Some(&b'#') {
        let end = scan(b, i + 1, is_fragment);
        fragment = Some(text(b, i + 1, end));
        i = end;
    }
    (i == b.len()).then_some((query, fragment))
}

struct Authority {
    userinfo: Option<String>,
    host: String,
    port: Option<String>,
    end: usize,
}

/// `(?:(?<userinfo>USERINFO)@)?(?<host>HOST)(?::(?<port>\d*+))?` from `start`.
fn authority(b: &[u8], start: usize) -> Option<Authority> {
    let mut i = start;
    let mut userinfo = None;
    let ui_end = scan(b, i, is_userinfo);
    if b.get(ui_end) == Some(&b'@') {
        userinfo = Some(text(b, i, ui_end));
        i = ui_end + 1;
    }

    // IP-literal first; when it does not match, reg-name (which also covers
    // IPv4address, see module docs) takes over.
    let host_end = ip_literal(b, i).unwrap_or_else(|| scan(b, i, is_reg_name));
    let host = text(b, i, host_end);
    i = host_end;

    let mut port = None;
    if b.get(i) == Some(&b':') {
        let digits_end = i + 1 + b[i + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
        port = Some(text(b, i + 1, digits_end));
        i = digits_end;
    }
    Some(Authority {
        userinfo,
        host,
        port,
        end: i,
    })
}

/// `\[(IPv6address|IPvFuture)\]` from `start`; returns the index after `]`.
fn ip_literal(b: &[u8], start: usize) -> Option<usize> {
    if b.get(start) != Some(&b'[') {
        return None;
    }
    let close = start + b[start..].iter().position(|&c| c == b']')?;
    let inner = std::str::from_utf8(&b[start + 1..close]).ok()?;
    let ok = inner.parse::<std::net::Ipv6Addr>().is_ok() || is_ipv_future(inner);
    ok.then_some(close + 1)
}

/// `v\h++\.[!$&-.0-9:;=A-Z_a-z~]++`
fn is_ipv_future(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('v') else {
        return false;
    };
    let Some((hex, tail)) = rest.split_once('.') else {
        return false;
    };
    !hex.is_empty()
        && hex.bytes().all(|c| c.is_ascii_hexdigit())
        && !tail.is_empty()
        && tail
            .bytes()
            .all(|c| is_unreserved_or_sub_delim(c) || c == b':')
}

// --- the two top-level grammars ------------------------------------------

/// `RFC3986_URI`.
fn absolute(b: &[u8]) -> Option<UriParts> {
    if !b.first().is_some_and(u8::is_ascii_alphabetic) {
        return None;
    }
    let scheme_end = 1 + b[1..].iter().take_while(|&&c| is_scheme_tail(c)).count();
    if b.get(scheme_end) != Some(&b':') {
        return None;
    }
    let scheme = Some(text(b, 0, scheme_end));
    let r = scheme_end + 1;

    // hier-part alternative 1: "//" authority path-abempty
    if b[r..].starts_with(b"//")
        && let Some(auth) = authority(b, r + 2)
    {
        let path_end = path_abempty(b, auth.end);
        if let Some((query, fragment)) = tail(b, path_end) {
            return Some(UriParts {
                scheme,
                userinfo: auth.userinfo,
                host: Some(auth.host),
                port: auth.port,
                path: Some(text(b, auth.end, path_end)),
                query,
                fragment,
            });
        }
    }

    // alternative 2: path-absolute = "/" ((?!/) seg++)?
    if b.get(r) == Some(&b'/') {
        let end = if b.get(r + 1) == Some(&b'/') {
            r + 1
        } else {
            scan(b, r + 1, is_seg)
        };
        if let Some((query, fragment)) = tail(b, end) {
            return Some(UriParts {
                scheme,
                path: Some(text(b, r, end)),
                query,
                fragment,
                ..UriParts::default()
            });
        }
    }

    // alternative 3: path-rootless (opaque) = (?!/) seg++
    if b.get(r) != Some(&b'/') {
        let end = scan(b, r, is_seg);
        if end > r
            && let Some((_query, fragment)) = tail(b, end)
        {
            // Opaque: no host and no path (Ruby folds the query into `opaque`).
            return Some(UriParts {
                scheme,
                fragment,
                ..UriParts::default()
            });
        }
    }

    // alternative 4: path-empty
    let (query, fragment) = tail(b, r)?;
    Some(UriParts {
        scheme,
        path: Some(String::new()),
        query,
        fragment,
        ..UriParts::default()
    })
}

/// `(?:/\g<seg>*+)?` from `i`.
fn path_abempty(b: &[u8], i: usize) -> usize {
    if b.get(i) == Some(&b'/') {
        scan(b, i + 1, is_seg)
    } else {
        i
    }
}

/// `RFC3986_relative_ref`. Only validity matters to the egress policy (a
/// relative reference has no scheme), so only the host/userinfo are kept.
fn relative(b: &[u8]) -> Option<UriParts> {
    if b.starts_with(b"//")
        && let Some(auth) = authority(b, 2)
    {
        let path_end = path_abempty(b, auth.end);
        if let Some((query, fragment)) = tail(b, path_end) {
            return Some(UriParts {
                userinfo: auth.userinfo,
                host: Some(auth.host),
                port: auth.port,
                path: Some(text(b, auth.end, path_end)),
                query,
                fragment,
                ..UriParts::default()
            });
        }
    }
    let end = if b.first() == Some(&b'/') {
        // path-absolute = "/" seg*
        scan(b, 1, is_seg)
    } else {
        // path-noscheme = SEG_NC++ (?:/ seg*)?  |  path-empty
        let nc_end = scan(b, 0, is_seg_nc);
        if nc_end > 0 {
            path_abempty(b, nc_end)
        } else {
            0
        }
    };
    let (query, fragment) = tail(b, end)?;
    Some(UriParts {
        path: Some(text(b, 0, end)),
        query,
        fragment,
        ..UriParts::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_full_url() {
        let p = parse("https://u:p@example.com:8443/a/b?c=d#e").expect("valid");
        assert_eq!(p.scheme.as_deref(), Some("https"));
        assert_eq!(p.userinfo.as_deref(), Some("u:p"));
        assert_eq!(p.host.as_deref(), Some("example.com"));
        assert_eq!(p.port.as_deref(), Some("8443"));
        assert_eq!(p.path.as_deref(), Some("/a/b"));
        assert_eq!(p.query.as_deref(), Some("c=d"));
        assert_eq!(p.fragment.as_deref(), Some("e"));
    }

    #[test]
    fn opaque_and_relative_forms_have_no_host() {
        assert_eq!(parse("http:example.com").expect("opaque").host, None);
        assert_eq!(parse("example.com/x.png").expect("relative").scheme, None);
        assert_eq!(parse("").expect("empty").scheme, None);
    }

    #[test]
    fn rejects_what_ruby_rejects() {
        for bad in [
            "   ",
            "http://example.com:abc/",
            "http://exa mple.com/",
            "http://ex\u{e4}mple.com/",
            "https://example.com/\n",
        ] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn ip_literals_keep_brackets() {
        assert_eq!(
            parse("http://[::1]/").expect("v6").host.as_deref(),
            Some("[::1]")
        );
        assert_eq!(
            parse("http://[v1.x]/").expect("future").host.as_deref(),
            Some("[v1.x]")
        );
    }
}
