//! Port of test/controllers/image_import_test.rb (`POST /images/import` →
//! `MediaApi::import`, plus the legacy S3 endpoints).
//!
//! setup: temp notes root, `images_path = <notes>/server`, default upload
//! extensions, local provider; example.com resolves to a public IP and every
//! connection is dialled to the wiremock server (also the S3 endpoint).
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use std::net::IpAddr;
use std::path::PathBuf;

use frankmd_media::api::{ApiStatus, MediaApi};
use frankmd_media::config::MediaConfig;
use frankmd_media::env::MemorySettings;
use frankmd_media::import::ImportRequest;
use frankmd_media::param::Param;
use frankmd_net::{Limits, StaticResolver};
use frankmd_testkit::TempTree;
use std::sync::Arc;
use uploads::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Fixture {
    notes: TempTree,
    server_dir: PathBuf,
    mock: MockServer,
}

impl Fixture {
    async fn new() -> Self {
        let notes = TempTree::empty();
        let server_dir = notes.path().join("server");
        std::fs::create_dir_all(&server_dir).unwrap();
        Self { notes, server_dir, mock: MockServer::start().await }
    }

    fn config(&self) -> MediaConfig {
        let mut c = MediaConfig::new(self.notes.path());
        c.images_path = Some(self.server_dir.clone());
        c
    }

    /// `use_s3`: key/secret/bucket "bucket", provider s3 (default region).
    fn s3(&self) -> MediaConfig {
        let mut c = self.config();
        c.s3.access_key_id = Some("key".into());
        c.s3.secret_access_key = Some("secret".into());
        c.s3.bucket = Some("bucket".into());
        c.media_storage_provider = Some("s3".into());
        c
    }

    fn api_with(&self, c: MediaConfig, resolver: StaticResolver) -> (Arc<MemorySettings>, MediaApi) {
        let settings = MemorySettings::new(c);
        let env = env_with(settings.clone(), pinned_ctx(), Some(&self.mock), resolver, Limits::EXTERNAL_IMAGE);
        (settings, MediaApi::new(env))
    }

    fn api(&self, c: MediaConfig) -> MediaApi {
        self.api_with(c, default_resolver()).1
    }

    fn local_images_exist(&self) -> bool {
        self.notes.path().join("images").exists()
    }

    async fn external_image(&self) {
        // security: real PNG bytes (placeholder text is rejected by content sniffing).
        external(&self.mock, "/image.png", &PNG_1X1, "image/png").await;
    }
}

fn url(u: &str) -> ImportRequest {
    ImportRequest { url: Param::str(u), ..Default::default() }
}

fn server_path(p: &str) -> ImportRequest {
    ImportRequest { path: Param::str(p), ..Default::default() }
}

// port: test/controllers/image_import_test.rb:32 "external URL is saved locally without requiring a server image folder"
#[tokio::test]
async fn external_url_is_saved_locally_without_requiring_a_server_image_folder() {
    let f = Fixture::new().await;
    f.external_image().await;
    let mut c = f.config();
    c.images_path = None;
    let u = f.api(c).import(&url("http://example.com/image.png")).await.unwrap();
    assert_match(r"\Aimages/", &u);
    assert_eq!(std::fs::read(f.notes.path().join(&u)).unwrap(), PNG_1X1);
}

// port: test/controllers/image_import_test.rb:41 "external and server images are saved to S3 without local media copies"
#[tokio::test]
async fn external_and_server_images_are_saved_to_s3_without_local_media_copies() {
    let f = Fixture::new().await;
    f.external_image().await;
    s3_put_ok(&f.mock, "bucket").await;
    write(&f.server_dir, "photo.png", &PNG_1X1); // security: real PNG bytes
    let api = f.api(f.s3());
    for mut req in [url("http://example.com/image.png"), server_path("photo.png")] {
        req.s3_prefix = Some("chosen".into());
        let u = api.import(&req).await.unwrap();
        assert_match(r"\Ahttps://bucket", &u);
    }
    let puts = puts(&f.mock).await;
    assert_eq!(puts.len(), 2);
    for p in &puts {
        assert!(p.url.path().starts_with("/bucket/chosen/"), "{}", p.url.path());
        assert_eq!(header(p, "if-none-match").as_deref(), Some("*"));
    }
    assert!(!f.local_images_exist());
}

async fn bmp_imports_locally(content_type: &str) {
    let f = Fixture::new().await;
    let body = unb64(BMP_1X1_B64);
    external(&f.mock, "/photo.bmp", &body, content_type).await;
    let mut c = f.config();
    c.image_upload_extensions = exts(&[".bmp"]);
    let u = f.api(c).import(&url("http://example.com/photo.bmp")).await.unwrap();
    assert_eq!(std::path::Path::new(&u).extension().unwrap(), "bmp");
    assert_eq!(std::fs::read(f.notes.path().join(&u)).unwrap(), body);
}

// port: test/controllers/image_import_test.rb:57 "#{content_type} imports locally with a BMP-only allowlist" [image/bmp]
#[tokio::test]
async fn image_bmp_imports_locally_with_a_bmp_only_allowlist() {
    bmp_imports_locally("image/bmp").await;
}

// port: test/controllers/image_import_test.rb:57 "#{content_type} imports locally with a BMP-only allowlist" [image/x-ms-bmp]
#[tokio::test]
async fn image_x_ms_bmp_imports_locally_with_a_bmp_only_allowlist() {
    bmp_imports_locally("image/x-ms-bmp").await;
}

async fn bmp_imports_to_s3(content_type: &str) {
    let f = Fixture::new().await;
    let body = unb64(BMP_1X1_B64);
    external(&f.mock, "/photo.bmp", &body, content_type).await;
    s3_put_ok(&f.mock, "bucket").await;
    let mut c = f.s3();
    c.image_upload_extensions = exts(&[".bmp"]);
    let u = f.api(c).import(&url("http://example.com/photo.bmp")).await.unwrap();
    assert_eq!(std::path::Path::new(&u).extension().unwrap(), "bmp");
    let puts = puts(&f.mock).await;
    assert_eq!(puts.len(), 1);
    assert_eq!(header(&puts[0], "content-type").as_deref(), Some("image/bmp"));
    assert!(puts[0].url.path().ends_with(".bmp"));
    assert_eq!(puts[0].body, body);
    assert!(!f.local_images_exist());
}

// port: test/controllers/image_import_test.rb:69 "#{content_type} imports to S3 with BMP extension and content type" [image/bmp]
#[tokio::test]
async fn image_bmp_imports_to_s3_with_bmp_extension_and_content_type() {
    bmp_imports_to_s3("image/bmp").await;
}

// port: test/controllers/image_import_test.rb:69 "#{content_type} imports to S3 with BMP extension and content type" [image/x-ms-bmp]
#[tokio::test]
async fn image_x_ms_bmp_imports_to_s3_with_bmp_extension_and_content_type() {
    bmp_imports_to_s3("image/x-ms-bmp").await;
}

// port: test/controllers/image_import_test.rb:88 "server images can be copied locally without changing the original"
#[tokio::test]
async fn server_images_can_be_copied_locally_without_changing_the_original() {
    let f = Fixture::new().await;
    write(&f.server_dir, "photo.png", &PNG_1X1); // security: real PNG bytes
    let u = f.api(f.config()).import(&server_path("photo.png")).await.unwrap();
    assert_eq!(std::fs::read(f.notes.path().join(&u)).unwrap(), PNG_1X1);
    assert_eq!(std::fs::read(f.server_dir.join("photo.png")).unwrap(), PNG_1X1);
}

// port: test/controllers/image_import_test.rb:96 "exactly one string source is required"
#[tokio::test]
async fn exactly_one_string_source_is_required() {
    let f = Fixture::new().await;
    let api = f.api(f.config());
    let both = ImportRequest { url: Param::str("http://example.com/a.png"), path: Param::str("a.png"), ..Default::default() };
    let array = ImportRequest { url: Param::List(vec![Param::str("http://example.com/a.png")]), ..Default::default() };
    for req in [ImportRequest::default(), both, array] {
        let err = api.import(&req).await.unwrap_err();
        assert_eq!(err.status, ApiStatus::Unprocessable, "{req:?}");
    }
}

// port: test/controllers/image_import_test.rb:103 "server import rejects traversal symlinks and nonimages"
#[tokio::test]
async fn server_import_rejects_traversal_symlinks_and_nonimages() {
    let f = Fixture::new().await;
    let outside = write(f.notes.path(), "outside.png", b"outside");
    std::os::unix::fs::symlink(&outside, f.server_dir.join("link.png")).unwrap();
    write(&f.server_dir, "script.html", b"<script>alert(1)</script>");
    let api = f.api(f.config());
    for p in ["../outside.png", "link.png", "script.html"] {
        let err = api.import(&server_path(p)).await.unwrap_err();
        assert_eq!(err.status, ApiStatus::Unprocessable, "{p}");
    }
    assert!(!f.local_images_exist());
}

// port: test/controllers/image_import_test.rb:115 "external import rejects HTML and SVG content"
#[tokio::test]
async fn external_import_rejects_html_and_svg_content() {
    for ct in ["text/html", "image/svg+xml"] {
        let f = Fixture::new().await;
        external(&f.mock, "/image.png", b"unsafe", ct).await;
        let err = f.api(f.config()).import(&url("http://example.com/image.png")).await.unwrap_err();
        assert_eq!(err.status, ApiStatus::Unprocessable, "{ct}");
        assert!(!f.local_images_exist());
    }
}

// port: test/controllers/image_import_test.rb:124 "external import rejects oversized responses"
#[tokio::test]
async fn external_import_rejects_oversized_responses() {
    let f = Fixture::new().await;
    let body = vec![b'x'; (frankmd_media::images::MAX_EXTERNAL_IMAGE_BYTES + 1) as usize];
    external(&f.mock, "/image.png", &body, "image/png").await;
    let err = f.api(f.config()).import(&url("http://example.com/image.png")).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(!f.local_images_exist());
}

// port: test/controllers/image_import_test.rb:131 "external import follows redirects and rejects private targets"
#[tokio::test]
async fn external_import_follows_redirects_and_rejects_private_targets() {
    let f = Fixture::new().await;
    Mock::given(method("GET"))
        .and(path("/start"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/image.png"))
        .mount(&f.mock)
        .await;
    f.external_image().await;
    assert!(f.api(f.config()).import(&url("http://example.com/start")).await.is_ok());

    let loopback: IpAddr = "127.0.0.1".parse().unwrap();
    let (_, api) = f.api_with(f.config(), StaticResolver::new().with("example.com", &[loopback]));
    let err = api.import(&url("http://example.com/image.png")).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
}

// port: test/controllers/image_import_test.rb:142 "S3 failures and missing credentials never create local media"
#[tokio::test]
async fn s3_failures_and_missing_credentials_never_create_local_media() {
    let f = Fixture::new().await;
    f.external_image().await;
    s3_put_error(&f.mock, "InternalError", 500).await;
    let (settings, api) = f.api_with(f.s3(), default_resolver());
    let err = api.import(&url("http://example.com/image.png")).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(!err.message.as_str().contains("private provider details"));
    settings.update(|c| c.s3.access_key_id = Some(String::new()));
    let err = api.import(&url("http://example.com/image.png")).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(!f.local_images_exist());
}

// port: test/controllers/image_import_test.rb:157 "legacy S3 endpoints reject an active local provider"
#[tokio::test]
async fn legacy_s3_endpoints_reject_an_active_local_provider() {
    let f = Fixture::new().await;
    let mut c = f.config();
    c.s3.access_key_id = Some("key".into());
    c.s3.secret_access_key = Some("secret".into());
    c.s3.bucket = Some("bucket".into());
    let api = f.api(c);
    let err = api.upload_to_s3("photo.png", &Param::Nil, None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    let err = api.upload_external_to_s3("http://example.com/image.png", &Param::Nil, None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(puts(&f.mock).await.is_empty());
}
