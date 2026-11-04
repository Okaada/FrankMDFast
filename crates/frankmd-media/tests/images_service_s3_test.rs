//! Port of test/services/images_service_test.rb, class `ImagesServiceS3Test`.
//!
//! The mocked `Aws::S3::Client` becomes a wiremock S3-compatible endpoint
//! reached through `MediaEnv::s3_transport` (path style), so the public URLs
//! keep the original AWS form (`https://test-bucket.s3.us-east-1.amazonaws.com/...`).
//! WebMock + the stubbed `EgressPolicy.resolve` become the same wiremock server
//! behind `EgressClient::with_dial_override` with a `StaticResolver`
//! (example.com / cdn.example.com → 93.184.216.34). External URLs use
//! `http://` because the mock speaks plain HTTP.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

use frankmd_media::config::MediaConfig;
use frankmd_media::error::MediaError;
use frankmd_media::env::MemorySettings;
use frankmd_media::images;
use frankmd_media::param::Param;
use frankmd_media::upload_storage::Upload;
use frankmd_net::{Limits, StaticResolver};
use frankmd_testkit::TempTree;
use uploads::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AWS_URL: &str = r"^https://test-bucket\.s3\.us-east-1\.amazonaws\.com/frankmd/\d{4}/\d{2}/";

struct Fixture {
    notes: TempTree,
    server_dir: PathBuf,
    mock: MockServer,
}

impl Fixture {
    async fn new() -> Self {
        let notes = TempTree::empty();
        let server_dir = notes.path().join("server_images");
        std::fs::create_dir_all(&server_dir).unwrap();
        Self { notes, server_dir, mock: MockServer::start().await }
    }

    fn config(&self) -> MediaConfig {
        let mut c = s3_config(self.notes.path(), "us-east-1");
        c.images_path = Some(self.server_dir.clone());
        c
    }

    fn env_with_config(&self, c: MediaConfig) -> frankmd_media::MediaEnv {
        env_for(c, Some(&self.mock)).1
    }

    fn env(&self) -> frankmd_media::MediaEnv {
        self.env_with_config(self.config())
    }

    fn image(&self, name: &str, content: &[u8]) {
        write(&self.server_dir, name, content);
    }

    fn no_local_images(&self) {
        assert!(!self.notes.path().join("images").exists(), "must not save locally");
    }
}

fn no_creds(mut c: MediaConfig) -> MediaConfig {
    c.s3.access_key_id = None;
    c
}

// port: test/services/images_service_test.rb:561 "saved S3 provider handles multipart and base64 images without request flags"
#[tokio::test]
async fn saved_s3_provider_handles_multipart_and_base64_images_without_request_flags() {
    let f = Fixture::new().await;
    s3_put_ok(&f.mock, "test-bucket").await;
    let env = f.env();
    let url = images::upload_file(&env, Some(Upload::new("image.png", "image")), None, &Param::Nil, None)
        .await
        .unwrap();
    assert_match("https://test-bucket", &url);
    let url = images::upload_base64_data(&env, &b64(b"image"), Some("image/png"), None, &Param::Nil, None)
        .await
        .unwrap();
    assert_match("https://test-bucket", &url);
    let puts = puts(&f.mock).await;
    assert_eq!(puts.len(), 2);
    for p in &puts {
        assert!(p.url.path().starts_with("/test-bucket/"));
        assert_eq!(header(p, "if-none-match").as_deref(), Some("*"));
    }
    f.no_local_images();
}

// port: test/services/images_service_test.rb:571 "unavailable S3 rejects every new image without local fallback"
#[tokio::test]
async fn unavailable_s3_rejects_every_new_image_without_local_fallback() {
    let f = Fixture::new().await;
    let env = f.env_with_config(no_creds(f.config()));
    let err = images::upload_file(&env, Some(Upload::new("image.png", "image")), None, &Param::Nil, None)
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "S3 is not configured");
    let err = images::upload_base64_data(&env, "aW1hZ2U=", None, None, &Param::Nil, None).await.unwrap_err();
    assert_eq!(err.to_string(), "S3 is not configured");
    f.no_local_images();
    assert!(requests(&f.mock).await.is_empty());
}

// port: test/services/images_service_test.rb:579 "S3 upload failures never write a local image"
#[tokio::test]
async fn s3_upload_failures_never_write_a_local_image() {
    let f = Fixture::new().await;
    s3_put_error(&f.mock, "InternalError", 500).await;
    let err = images::upload_base64_data(&f.env(), "aW1hZ2U=", Some("image/png"), None, &Param::Nil, None)
        .await
        .unwrap_err();
    assert!(matches!(err, MediaError::S3(_)), "service error must reach the caller, got {err:?}");
    f.no_local_images();
}

// port: test/services/images_service_test.rb:589 "s3_enabled? returns true when configured"
#[tokio::test]
async fn s3_enabled_returns_true_when_configured() {
    assert!(images::s3_enabled(&Fixture::new().await.config()));
}

// port: test/services/images_service_test.rb:593 "s3_enabled? returns false when not configured"
#[tokio::test]
async fn s3_enabled_returns_false_when_not_configured() {
    assert!(!images::s3_enabled(&no_creds(Fixture::new().await.config())));
}

// port: test/services/images_service_test.rb:598 "upload_to_s3 returns nil when s3 disabled"
#[tokio::test]
async fn upload_to_s3_fails_when_s3_disabled() {
    let f = Fixture::new().await;
    f.image("test.jpg", b"fake image data");
    let env = f.env_with_config(no_creds(f.config()));
    assert!(images::upload_to_s3(&env, "test.jpg", None, None).await.is_err());
    assert!(requests(&f.mock).await.is_empty());
}

// port: test/services/images_service_test.rb:607 "upload_to_s3 returns nil for non-existent image"
#[tokio::test]
async fn upload_to_s3_fails_for_non_existent_image() {
    let f = Fixture::new().await;
    let err = images::upload_to_s3(&f.env(), "nonexistent.jpg", None, None).await.unwrap_err();
    assert!(matches!(err, MediaError::NotFound), "{err:?}");
}

// port: test/services/images_service_test.rb:613 "upload_to_s3 calls S3 client with correct parameters"
#[tokio::test]
async fn upload_to_s3_calls_s3_with_correct_parameters() {
    let f = Fixture::new().await;
    f.image("upload_test.jpg", b"image content");
    s3_put_ok(&f.mock, "test-bucket").await;
    let url = images::upload_to_s3(&f.env(), "upload_test.jpg", None, None).await.unwrap();
    assert_match(&format!("{AWS_URL}upload_test-[0-9a-f]{{16}}\\.jpg$"), &url);
    let puts = puts(&f.mock).await;
    assert_eq!(puts.len(), 1);
    assert_eq!(header(&puts[0], "if-none-match").as_deref(), Some("*"));
    assert_eq!(header(&puts[0], "content-type").as_deref(), Some("image/jpeg"));
    assert_eq!(puts[0].body, b"image content");
}

// port: test/services/images_service_test.rb:627 "upload_to_s3 sanitizes special characters in the object key"
#[tokio::test]
async fn upload_to_s3_sanitizes_special_characters_in_the_object_key() {
    let f = Fixture::new().await;
    f.image("my photo (1).jpg", b"image content");
    s3_put_ok(&f.mock, "test-bucket").await;
    let url = images::upload_to_s3(&f.env(), "my photo (1).jpg", None, None).await.unwrap();
    assert_match(r"my_photo__1_-[0-9a-f]{16}\.jpg", &url);
    assert!(!url.contains(' '));
}

// port: test/services/images_service_test.rb:645 "upload_to_s3 handles ACL not supported error gracefully"
#[tokio::test]
async fn upload_to_s3_handles_acl_not_supported_error_gracefully() {
    let f = Fixture::new().await;
    f.image("acl_test.jpg", b"image data");
    s3_put_error(&f.mock, "AccessControlListNotSupported", 400).await;
    let url = images::upload_to_s3(&f.env(), "acl_test.jpg", None, None).await.unwrap();
    assert!(!url.is_empty());
}

// port: test/services/images_service_test.rb:658 "upload_to_s3 with resize option returns S3 URL"
#[tokio::test]
async fn upload_to_s3_with_resize_option_returns_s3_url() {
    let f = Fixture::new().await;
    f.image("resize_s3.png", &PNG_1X1);
    s3_put_ok(&f.mock, "test-bucket").await;
    let url = images::upload_to_s3(&f.env(), "resize_s3.png", Some(0.5), None).await.unwrap();
    assert_match(r"resize_s3-[0-9a-f]{16}\.(jpg|png)", &url);
    assert_match(r"^https://test-bucket\.s3\.us-east-1\.amazonaws\.com", &url);
}

// port: test/services/images_service_test.rb:683 "download_and_upload_to_s3 returns nil when s3 disabled"
#[tokio::test]
async fn download_and_upload_to_s3_fails_when_s3_disabled() {
    let f = Fixture::new().await;
    let env = f.env_with_config(no_creds(f.config()));
    assert!(images::download_and_upload_to_s3(&env, "http://example.com/image.jpg", None, None).await.is_err());
    assert!(requests(&f.mock).await.is_empty(), "nothing fetched, nothing uploaded");
}

// port: test/services/images_service_test.rb:691 "fetch_external_image stops when its overall deadline expires"
// Adapted: the monotonic-clock stub becomes a tiny overall deadline against a slow response.
#[tokio::test]
async fn fetch_external_image_stops_when_its_overall_deadline_expires() {
    let f = Fixture::new().await;
    Mock::given(method("GET"))
        .and(path("/image.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(b"x".to_vec(), "image/jpeg").set_delay(Duration::from_secs(5)))
        .mount(&f.mock)
        .await;
    let limits = Limits { timeout: Duration::from_millis(100), ..Limits::EXTERNAL_IMAGE };
    let env = env_with(MemorySettings::new(f.config()), pinned_ctx(), Some(&f.mock), default_resolver(), limits);
    let started = std::time::Instant::now();
    assert!(images::fetch_external_image(&env, "http://example.com/image.jpg").await.is_err());
    assert!(started.elapsed() < Duration::from_secs(4), "deadline must cut the fetch short");
}

// port: test/services/images_service_test.rb:697 "download_and_upload_to_s3 downloads image and uploads to S3"
#[tokio::test]
async fn download_and_upload_to_s3_downloads_image_and_uploads_to_s3() {
    let f = Fixture::new().await;
    external(&f.mock, "/photo.jpg", b"fake image content", "image/jpeg").await;
    s3_put_ok(&f.mock, "test-bucket").await;
    let url = images::download_and_upload_to_s3(&f.env(), "http://example.com/photo.jpg", None, None).await.unwrap();
    assert_match(&format!("{AWS_URL}photo-[0-9a-f]{{16}}\\.jpg$"), &url);
    let puts = puts(&f.mock).await;
    assert_eq!(puts.len(), 1);
    assert_eq!(puts[0].body, b"fake image content");
}

// port: test/services/images_service_test.rb:715 "download_and_upload_to_s3 generates filename when URL has no extension"
#[tokio::test]
async fn download_and_upload_to_s3_generates_filename_when_url_has_no_extension() {
    let f = Fixture::new().await;
    external(&f.mock, "/images/", b"fake image content", "image/png").await;
    s3_put_ok(&f.mock, "test-bucket").await;
    let url = images::download_and_upload_to_s3(&f.env(), "http://example.com/images/", None, None).await.unwrap();
    assert_match(r"\.png$", &url);
}

// port: test/services/images_service_test.rb:733 "download_and_upload_to_s3 returns nil on download failure"
#[tokio::test]
async fn download_and_upload_to_s3_fails_on_download_failure() {
    let f = Fixture::new().await;
    Mock::given(method("GET"))
        .and(path("/missing.jpg"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&f.mock)
        .await;
    assert!(images::download_and_upload_to_s3(&f.env(), "http://example.com/missing.jpg", None, None).await.is_err());
    assert!(puts(&f.mock).await.is_empty());
}

// port: test/services/images_service_test.rb:742 "download_and_upload_to_s3 rejects a body over the size cap (Content-Length)"
#[tokio::test]
async fn download_and_upload_to_s3_rejects_a_body_over_the_size_cap() {
    let f = Fixture::new().await;
    // A real body one byte over the cap (wiremock sets an honest Content-Length).
    let body = vec![b'x'; (images::MAX_EXTERNAL_IMAGE_BYTES + 1) as usize];
    external(&f.mock, "/huge.jpg", &body, "image/jpeg").await;
    assert!(images::download_and_upload_to_s3(&f.env(), "http://example.com/huge.jpg", None, None).await.is_err());
    assert!(puts(&f.mock).await.is_empty());
}

// port: test/services/images_service_test.rb:756 "download_and_upload_to_s3 follows a redirect, re-validating the target"
#[tokio::test]
async fn download_and_upload_to_s3_follows_a_redirect_re_validating_the_target() {
    let f = Fixture::new().await;
    Mock::given(method("GET"))
        .and(path("/go.jpg"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "http://cdn.example.com/real.jpg"))
        .mount(&f.mock)
        .await;
    external(&f.mock, "/real.jpg", b"fake image content", "image/jpeg").await;
    s3_put_ok(&f.mock, "test-bucket").await;
    let resolver = default_resolver();
    let env = env_with(MemorySettings::new(f.config()), pinned_ctx(), Some(&f.mock), resolver, Limits::EXTERNAL_IMAGE);
    let url = images::download_and_upload_to_s3(&env, "http://example.com/go.jpg", None, None).await;
    assert!(url.is_ok(), "expected the redirect to be followed to a public target: {url:?}");
    assert!(requests(&f.mock).await.contains(&("GET".into(), "/real.jpg".into())));
}

// port: test/services/images_service_test.rb:769 "download_and_upload_to_s3 blocks a redirect to a non-public address (SSRF via redirect)"
#[tokio::test]
async fn download_and_upload_to_s3_blocks_a_redirect_to_a_non_public_address() {
    let f = Fixture::new().await;
    Mock::given(method("GET"))
        .and(path("/rebind.jpg"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("Location", "http://169.254.169.254/latest/meta-data/"),
        )
        .mount(&f.mock)
        .await;
    assert!(images::download_and_upload_to_s3(&f.env(), "http://example.com/rebind.jpg", None, None).await.is_err());
    let reqs = requests(&f.mock).await;
    assert!(!reqs.iter().any(|(_, p)| p == "/latest/meta-data/"), "metadata host must never be requested: {reqs:?}");
}

// port: test/services/images_service_test.rb:784 "download_and_upload_to_s3 refuses a host resolving to a non-public address (SSRF)"
#[tokio::test]
async fn download_and_upload_to_s3_refuses_a_host_resolving_to_a_non_public_address() {
    let f = Fixture::new().await;
    let metadata: IpAddr = "169.254.169.254".parse().unwrap();
    let resolver = StaticResolver::new().with("example.com", &[metadata]).with("169.254.169.254", &[metadata]);
    let env = env_with(MemorySettings::new(f.config()), pinned_ctx(), Some(&f.mock), resolver, Limits::EXTERNAL_IMAGE);
    assert!(
        images::download_and_upload_to_s3(&env, "http://169.254.169.254/latest/meta-data/iam/", None, None).await.is_err()
    );
    assert!(images::download_and_upload_to_s3(&env, "http://example.com/latest/meta-data/iam/", None, None).await.is_err());
    assert!(requests(&f.mock).await.is_empty(), "nothing may be requested");
}

// port: test/services/images_service_test.rb:793 "download_and_upload_to_s3 refuses a non-http scheme (SSRF)"
#[tokio::test]
async fn download_and_upload_to_s3_refuses_a_non_http_scheme() {
    let f = Fixture::new().await;
    assert!(images::download_and_upload_to_s3(&f.env(), "file:///etc/passwd", None, None).await.is_err());
    assert!(requests(&f.mock).await.is_empty());
}
