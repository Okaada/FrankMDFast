//! Port of test/services/upload_storage_test.rb.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use frankmd_media::config::MediaConfig;
use frankmd_media::error::MediaError;
use frankmd_media::param::Param;
use frankmd_media::s3::{S3Client, S3Error};
use frankmd_media::upload_storage::{self as us, UploadKind};
use frankmd_media::StorageProvider;
use frankmd_net::TrustedClient;
use uploads::*;

/// setup: provider local, image extensions `.png .jpg`, everything else nil.
fn config() -> MediaConfig {
    let mut c = MediaConfig::new("/tmp/notes");
    c.media_storage_provider = Some("local".into());
    c.image_upload_extensions = exts(&[".png", ".jpg"]);
    c
}

fn rejected(r: Result<impl std::fmt::Debug, MediaError>) -> String {
    match r {
        Err(MediaError::Rejected(m)) => m,
        other => panic!("expected RejectedError, got {other:?}"),
    }
}

fn service_error(code: &str) -> S3Error {
    S3Error::Service { code: code.into(), status: 400 }
}

// port: test/services/upload_storage_test.rb:21 "provider uses the saved preference and rejects conflicting legacy requests"
#[test]
fn provider_uses_the_saved_preference_and_rejects_conflicting_legacy_requests() {
    let mut c = config();
    assert_eq!(us::provider(&c, &Param::Nil).unwrap(), StorageProvider::Local);
    assert_eq!(us::provider(&c, &Param::Bool(false)).unwrap(), StorageProvider::Local);
    rejected(us::provider(&c, &Param::str("true")));
    c.media_storage_provider = Some("s3".into());
    rejected(us::provider(&c, &Param::Nil));
    c.s3.access_key_id = Some("configured".into());
    c.s3.secret_access_key = Some("configured".into());
    c.s3.bucket = Some("configured".into());
    assert_eq!(us::provider(&c, &Param::Nil).unwrap(), StorageProvider::S3);
    rejected(us::provider(&c, &Param::str("false")));
    c.media_storage_provider = Some("invalid".into());
    rejected(us::provider(&c, &Param::Nil));
}

// port: test/services/upload_storage_test.rb:36 "only definitive S3 configuration errors trigger recovery"
#[test]
fn only_definitive_s3_configuration_errors_trigger_recovery() {
    for code in [
        "InvalidAccessKeyId",
        "SignatureDoesNotMatch",
        "InvalidToken",
        "ExpiredToken",
        "AccessDenied",
        "NoSuchBucket",
        "AuthorizationHeaderMalformed",
    ] {
        assert!(us::invalid_s3_configuration_error(&service_error(code)), "{code}");
    }
    for code in ["SlowDown", "InternalError", "ServiceUnavailable", "PreconditionFailed"] {
        assert!(!us::invalid_s3_configuration_error(&service_error(code)), "{code}");
    }
    assert!(us::invalid_s3_configuration_error(&S3Error::InvalidRegion));
    assert!(!us::invalid_s3_configuration_error(&S3Error::Network("timeout".into())));
}

// port: test/services/upload_storage_test.rb:47 "validate_extension! returns the normalized extension for an allowed file"
#[test]
fn validate_extension_returns_the_normalized_extension_for_an_allowed_file() {
    assert_eq!(us::validate_filename(&config(), "a.PNG", UploadKind::Image).unwrap(), ".png");
}

// port: test/services/upload_storage_test.rb:52 "validate_extension! raises RejectedError for a disallowed extension"
#[test]
fn validate_extension_raises_for_a_disallowed_extension() {
    let msg = rejected(us::validate_filename(&config(), "evil.html", UploadKind::Image));
    assert!(msg.contains(".html"));
    assert!(msg.contains("not an accepted image type"));
}

// port: test/services/upload_storage_test.rb:60 "validate_extension! raises for a file with no extension"
#[test]
fn validate_extension_raises_for_a_file_with_no_extension() {
    let msg = rejected(us::validate_filename(&config(), "README", UploadKind::Image));
    assert!(msg.contains("no extension"));
}

// port: test/services/upload_storage_test.rb:67 "validate_extension! is not fooled by a double extension"
#[test]
fn validate_extension_is_not_fooled_by_a_double_extension() {
    let msg = rejected(us::validate_filename(&config(), "evil.png.html", UploadKind::Image));
    assert!(msg.contains(".html"));
}

// port: test/services/upload_storage_test.rb:76 "enforce_size! raises when the file exceeds the cap"
#[test]
fn enforce_size_raises_when_the_file_exceeds_the_cap() {
    let msg = rejected(us::enforce_bytes(Some(us::MAX_UPLOAD_BYTES as i64 + 1), us::MAX_UPLOAD_BYTES));
    assert!(msg.contains("too large"));
}

// port: test/services/upload_storage_test.rb:82 "enforce_size! allows a file at the cap"
#[test]
fn enforce_size_allows_a_file_at_the_cap() {
    assert!(us::enforce_bytes(Some(us::MAX_UPLOAD_BYTES as i64), us::MAX_UPLOAD_BYTES).is_ok());
}

// port: test/services/upload_storage_test.rb:89 "dest_filename strips path separators and traversal, keeping a timestamp prefix"
#[test]
fn dest_filename_strips_path_separators_and_traversal() {
    let name = us::dest_filename(&frankmd_media::MediaCtx::system(), Some("../../etc/pa ss!.mp4"));
    assert!(!name.contains('/'));
    assert!(!name.contains(' '));
    assert!(!name.contains('!'));
    assert_match(r"\A\d{8}_\d{6}_", &name);
    assert!(name.ends_with(".mp4"));
}

// port: test/services/upload_storage_test.rb:98 "dest_filename is unique for uploads with the same name in the same second"
#[test]
fn dest_filename_is_unique_for_uploads_with_the_same_name_in_the_same_second() {
    // Fixed clock, real randomness (the original stubbed only Time.now).
    let ctx = frankmd_media::MediaCtx {
        clock: pinned_ctx().clock,
        random: std::sync::Arc::new(frankmd_media::ctx::OsRandom),
    };
    assert_ne!(us::dest_filename(&ctx, Some("video.mp4")), us::dest_filename(&ctx, Some("video.mp4")));
}

// port: test/services/upload_storage_test.rb:106 "sanitize_s3_key keeps nested prefixes"
#[test]
fn sanitize_s3_key_keeps_nested_prefixes() {
    assert_eq!(us::sanitize_s3_key(Some("blog/2026/hero")), "blog/2026/hero");
}

// port: test/services/upload_storage_test.rb:110 "sanitize_s3_key strips traversal, leading and doubled slashes"
#[test]
fn sanitize_s3_key_strips_traversal_leading_and_doubled_slashes() {
    assert_eq!(us::sanitize_s3_key(Some("../../etc/passwd")), "etc/passwd");
    assert_eq!(us::sanitize_s3_key(Some("/a//b/")), "a/b");
    assert_eq!(us::sanitize_s3_key(Some("a/./b")), "a/b");
}

// port: test/services/upload_storage_test.rb:116 "sanitize_s3_key replaces unsafe characters per segment"
#[test]
fn sanitize_s3_key_replaces_unsafe_characters_per_segment() {
    assert_eq!(us::sanitize_s3_key(Some("my folder/a b")), "my_folder/a_b");
}

// port: test/services/upload_storage_test.rb:120 "sanitize_s3_key returns empty string when nothing survives"
#[test]
fn sanitize_s3_key_returns_empty_string_when_nothing_survives() {
    assert_eq!(us::sanitize_s3_key(Some("../..")), "");
    assert_eq!(us::sanitize_s3_key(None), "");
}

fn sys() -> frankmd_media::MediaCtx {
    frankmd_media::MediaCtx::system()
}

// port: test/services/upload_storage_test.rb:127 "s3_key builds the default frankmd/YYYY/MM/<filename> when no custom value"
#[test]
fn s3_key_builds_the_default_path_when_no_custom_value() {
    assert_match(r"\Afrankmd/\d{4}/\d{2}/photo-[0-9a-f]{16}\.png\z", &us::s3_key(&sys(), Some("photo.png"), None, None));
}

// port: test/services/upload_storage_test.rb:131 "s3_key sanitizes the filename in the default path"
#[test]
fn s3_key_sanitizes_the_filename_in_the_default_path() {
    let key = us::s3_key(&sys(), Some("../ev il.png"), None, None);
    assert_match(r"\Afrankmd/\d{4}/\d{2}/", &key);
    assert_match(r"/.._ev_il-[0-9a-f]{16}\.png\z", &key);
}

// port: test/services/upload_storage_test.rb:137 "s3_key makes a custom key unique while preserving its prefix and extension"
#[test]
fn s3_key_makes_a_custom_key_unique() {
    let key = us::s3_key(&sys(), Some("photo.png"), Some("blog/2026/hero.png"), Some("ignored"));
    assert_ne!(key, "blog/2026/hero.png");
    assert_match(r"\Ablog/2026/hero-[0-9a-f]{16}\.png\z", &key);
}

// port: test/services/upload_storage_test.rb:143 "s3_key appends the safe filename to a custom_prefix"
#[test]
fn s3_key_appends_the_safe_filename_to_a_custom_prefix() {
    assert_match(
        r"\Ablog/2026/photo-[0-9a-f]{16}\.png\z",
        &us::s3_key(&sys(), Some("photo.png"), None, Some("blog/2026")),
    );
}

// port: test/services/upload_storage_test.rb:147 "s3_key blocks traversal in a custom_key"
#[test]
fn s3_key_blocks_traversal_in_a_custom_key() {
    assert_match(
        r"\Aetc/passwd-[0-9a-f]{16}\.png\z",
        &us::s3_key(&sys(), Some("photo.png"), Some("../../etc/passwd"), None),
    );
}

// port: test/services/upload_storage_test.rb:152 "s3_key falls back to the default when a custom value sanitizes to empty"
#[test]
fn s3_key_falls_back_to_the_default_when_a_custom_value_sanitizes_to_empty() {
    let re = r"\Afrankmd/\d{4}/\d{2}/photo-[0-9a-f]{16}\.png\z";
    assert_match(re, &us::s3_key(&sys(), Some("photo.png"), Some("../.."), None));
    assert_match(re, &us::s3_key(&sys(), Some("photo.png"), None, Some("///")));
}

// port: test/services/upload_storage_test.rb:159 "s3_client passes a custom endpoint and force_path_style when configured"
#[test]
fn s3_client_passes_a_custom_endpoint_and_force_path_style_when_configured() {
    let mut c = config();
    c.s3.region = Some("us-east-1".into());
    c.s3.endpoint = Some("http://localhost:9000".into());
    c.s3.force_path_style = true;
    let client = S3Client::new(&c.s3, &TrustedClient::new().unwrap()).unwrap();
    assert_eq!(client.endpoint(), Some("http://localhost:9000"));
    assert!(client.force_path_style());
    assert_eq!(client.region(), "us-east-1");
}

// port: test/services/upload_storage_test.rb:175 "s3_client omits endpoint/force_path_style for plain AWS"
#[test]
fn s3_client_omits_endpoint_and_force_path_style_for_plain_aws() {
    let c = config();
    let client = S3Client::new(&c.s3, &TrustedClient::new().unwrap()).unwrap();
    assert_eq!(client.endpoint(), None);
    // Ruby: no :force_path_style key at all; Rust: virtual-host addressing.
    assert!(!client.force_path_style());
    assert_eq!(client.region(), "us-east-1"); // DEFAULT_S3_REGION
}

// port: test/services/upload_storage_test.rb:190 "s3_url builds the AWS virtual-host URL by default"
#[test]
fn s3_url_builds_the_aws_virtual_host_url_by_default() {
    assert_eq!(
        us::s3_url(&config(), "mybucket", "us-west-2", "frankmd/2026/09/a b.png"),
        "https://mybucket.s3.us-west-2.amazonaws.com/frankmd/2026/09/a%20b.png"
    );
}

// port: test/services/upload_storage_test.rb:195 "s3_url uses path-style against a custom endpoint"
#[test]
fn s3_url_uses_path_style_against_a_custom_endpoint() {
    let mut c = config();
    c.s3.endpoint = Some("http://localhost:9000/".into());
    c.s3.force_path_style = true;
    assert_eq!(us::s3_url(&c, "mybucket", "us-east-1", "frankmd/x.png"), "http://localhost:9000/mybucket/frankmd/x.png");
}

// port: test/services/upload_storage_test.rb:202 "s3_url uses virtual-host style against a custom endpoint when path-style is off"
#[test]
fn s3_url_uses_virtual_host_style_against_a_custom_endpoint() {
    let mut c = config();
    c.s3.endpoint = Some("https://s3.example.com".into());
    c.s3.force_path_style = false;
    assert_eq!(us::s3_url(&c, "mybucket", "us-east-1", "k.png"), "https://mybucket.s3.example.com/k.png");
}

// port: test/services/upload_storage_test.rb:209 "s3_url keeps a non-default port for a custom endpoint"
#[test]
fn s3_url_keeps_a_non_default_port_for_a_custom_endpoint() {
    let mut c = config();
    c.s3.endpoint = Some("http://minio.local:9000".into());
    c.s3.force_path_style = false;
    assert_eq!(us::s3_url(&c, "b", "us-east-1", "k.png"), "http://b.minio.local:9000/k.png");
}

// port: test/services/upload_storage_test.rb:218 "with_temp_copy writes to a random name (not the client filename) and cleans up"
// SKIPPED (adapted away): the Rust pipeline never stages client bytes in a temp
// file — uploads are validated and written from memory straight to their
// no-clobber destination, so there is no temp name to leak. The security intent
// (client names never become paths) is covered by dest_filename / save tests.
