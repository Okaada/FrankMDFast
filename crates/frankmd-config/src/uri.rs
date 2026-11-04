//! **[COMPAT]** the subset of Ruby `URI.parse` (RFC 3986) that
//! `UploadStorage.s3_configured?` relies on.

/// `URI.parse(s)` succeeds, the scheme is `http`/`https` (any case), the host
/// is non-empty and there is no userinfo.
pub(crate) fn is_plain_http_url(s: &str) -> bool {
    if !s.bytes().all(is_uri_byte) || !percent_escapes_ok(s) {
        return false; // URI::InvalidURIError
    }
    let Some((scheme, rest)) = split_scheme(s) else {
        return false; // relative reference: no scheme
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    let Some(after) = rest.strip_prefix("//") else {
        return false; // no authority, no host
    };
    let end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let (authority, tail) = after.split_at(end);
    if tail.contains(['[', ']']) || tail.split_once('#').is_some_and(|(_, frag)| frag.contains('#')) {
        return false;
    }
    if authority.contains('@') {
        return false; // userinfo present (even empty)
    }
    let bracketed = authority.starts_with('[');
    let (host, port) = if let Some(literal) = authority.strip_prefix('[') {
        let Some((inner, after_bracket)) = literal.split_once(']') else {
            return false;
        };
        if inner.is_empty() {
            return false;
        }
        match after_bracket {
            "" => (inner, ""),
            p => match p.strip_prefix(':') {
                Some(port) => (inner, port),
                None => return false,
            },
        }
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) => (h, p),
            None => (authority, ""),
        }
    };
    if (!bracketed && host.contains([':', '[', ']'])) || !port.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    !host.is_empty()
}

/// `scheme ":" rest` when `s` starts with an RFC 3986 scheme.
fn split_scheme(s: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = s.split_once(':')?;
    let mut bytes = scheme.bytes();
    let first_ok = bytes.next().is_some_and(|b| b.is_ascii_alphabetic());
    (first_ok && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))).then_some((scheme, rest))
}

/// Unreserved, reserved and `%`.
fn is_uri_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"-._~:/?#[]@!$&'()*+,;=%".contains(&b)
}

/// Every `%` starts a `%HH` escape.
fn percent_escapes_ok(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.iter().enumerate().filter(|(_, b)| **b == b'%').all(|(i, _)| {
        bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit) && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit)
    })
}

#[cfg(test)]
mod tests {
    use super::is_plain_http_url as ok;

    #[test]
    fn matches_upload_storage_rules() {
        assert!(ok("http://localhost:9000"));
        assert!(ok("https://s3.example.com/path?q=1#f"));
        assert!(ok("HTTP://minio"));
        assert!(ok("http://[::1]:9000"));
        assert!(!ok("ftp://x"));
        assert!(!ok("http://u:p@minio:9000"));
        assert!(!ok("minio:9000"));
        assert!(!ok("http://exa mple"));
        assert!(!ok("localhost"));
        assert!(!ok("http://"));
        assert!(!ok("http://host:port"));
        assert!(!ok("http://h/%zz"));
    }
}
