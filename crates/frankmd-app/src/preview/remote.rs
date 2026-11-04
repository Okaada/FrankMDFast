//! Remote (`https`) images for the preview, fetched only through
//! frankmd-net's [`EgressClient`] (public addresses only, redirects re-vetted,
//! 25 MB cap) on the tokio bridge, never on the GPUI main thread.
//!
//! Without a network runtime (headless tests, or before the shell installs
//! [`NetRuntimeGlobal`]) remote images stay [`RemoteState::Unavailable`].

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use frankmd_net::{EgressClient, Limits};
use gpui_kit::{Image, ImageFormat};

/// Load state of one remote image.
#[derive(Clone)]
pub enum RemoteState {
    /// Fetch in flight.
    Loading,
    /// Decoded bytes ready for `img()`.
    Ready(Arc<Image>),
    /// Fetch refused by policy, failed, or not an image.
    Unavailable,
}

/// Per-pane cache of remote images keyed by URL.
#[derive(Default)]
pub struct RemoteImages {
    states: HashMap<String, RemoteState>,
}

impl RemoteImages {
    /// Current state, if the URL was ever requested.
    pub fn get(&self, url: &str) -> Option<RemoteState> {
        self.states.get(url).cloned()
    }

    /// Mark a URL as requested; returns `true` when a fetch must start.
    pub fn begin(&mut self, url: &str) -> bool {
        if self.states.contains_key(url) {
            return false;
        }
        self.states.insert(url.to_owned(), RemoteState::Loading);
        true
    }

    /// Record a fetch result.
    pub fn finish(&mut self, url: &str, state: RemoteState) {
        self.states.insert(url.to_owned(), state);
    }
}

/// The process's egress client for preview images (one client, HP-010).
pub fn client() -> Option<&'static EgressClient> {
    static CLIENT: OnceLock<Option<EgressClient>> = OnceLock::new();
    CLIENT
        .get_or_init(|| match EgressClient::new(Limits::EXTERNAL_IMAGE) {
            Ok(c) => Some(c),
            Err(err) => {
                tracing::warn!(%err, "preview: no egress client");
                None
            }
        })
        .as_ref()
}

/// Turn fetched bytes into an image, sniffing the format from the bytes
/// (the `Content-Type` header is not trusted).
pub fn decode(bytes: Vec<u8>) -> RemoteState {
    match sniff(&bytes) {
        Some(format) => RemoteState::Ready(Arc::new(Image::from_bytes(format, bytes))),
        None => RemoteState::Unavailable,
    }
}

/// Magic-byte sniffing for the formats GPUI decodes.
pub fn sniff(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(ImageFormat::Jpeg)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(ImageFormat::Gif)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(ImageFormat::Webp)
    } else if bytes.starts_with(b"BM") {
        Some(ImageFormat::Bmp)
    } else {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).to_ascii_lowercase();
        head.contains("<svg").then_some(ImageFormat::Svg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffing_ignores_extensions_and_rejects_html() {
        assert_eq!(
            sniff(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0]),
            Some(ImageFormat::Png)
        );
        assert_eq!(sniff(b"GIF89a...."), Some(ImageFormat::Gif));
        assert_eq!(
            sniff(b"<?xml version='1.0'?><svg xmlns='x'/>"),
            Some(ImageFormat::Svg)
        );
        assert_eq!(sniff(b"<html><script>alert(1)</script>"), None);
        assert!(matches!(
            decode(b"not an image".to_vec()),
            RemoteState::Unavailable
        ));
    }

    #[test]
    fn begin_starts_one_fetch_per_url() {
        let mut cache = RemoteImages::default();
        assert!(cache.begin("https://a/x.png"));
        assert!(!cache.begin("https://a/x.png"));
        cache.finish("https://a/x.png", RemoteState::Unavailable);
        assert!(matches!(
            cache.get("https://a/x.png"),
            Some(RemoteState::Unavailable)
        ));
    }
}
