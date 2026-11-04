//! Helpers for the media-client ports (search, YouTube, picker sources).
#![allow(dead_code, unreachable_pub, clippy::disallowed_methods, clippy::unwrap_used)]

use std::path::PathBuf;

use frankmd_media::config::MediaConfig;

/// `crates/frankmd-media/tests/data/<name>` contents.
pub fn data(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

pub fn http() -> frankmd_net::TrustedClient {
    frankmd_net::TrustedClient::new().expect("client")
}

/// Config with no keys (the original's `ENV.delete(...)`).
pub fn bare_config() -> MediaConfig {
    MediaConfig::new("/nonexistent-notes-root")
}

/// `SearchActionsTest` config stub: Google key + CSE id set.
pub fn google_config() -> MediaConfig {
    let mut c = bare_config();
    c.google_api_key = Some("test-google-key".into());
    c.google_cse_id = Some("test-cse-id".into());
    c
}

pub fn youtube_config(key: &str) -> MediaConfig {
    let mut c = bare_config();
    c.youtube_api_key = Some(key.into());
    c
}
