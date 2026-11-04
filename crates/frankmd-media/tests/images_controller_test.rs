//! Port of test/controllers/images_controller_test.rb (everything except the
//! web-search endpoints `search_web` / `search_google` / `search_pinterest`
//! and `SearchActionsTest`, which are ported by the media-client tests).
//! Endpoints → `frankmd_media::api::MediaApi`; `t("errors.x")` → `ApiMessage::Key`.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use std::path::PathBuf;

use frankmd_media::api::{ApiMessage, ApiStatus, Base64Upload, MediaApi};
use frankmd_media::config::MediaConfig;
use frankmd_media::images::parse_resize_ratio;
use frankmd_media::param::Param;
use frankmd_media::upload_storage::Upload;
use frankmd_testkit::TempTree;
use uploads::*;
use wiremock::MockServer;

fn nil() -> Param {
    Param::Nil
}

// ---- top-level ImagesControllerTest (default config) ----------------------

// port: test/controllers/images_controller_test.rb:8 "config returns enabled and s3 status"
#[test]
fn config_returns_enabled_and_s3_status() {
    let t = TempTree::empty();
    let api = MediaApi::new(env_for(MediaConfig::new(t.path()), None).1);
    let info = api.images_config();
    // Booleans by construction; the defaults are: no server folder, no S3,
    // web search on, Google off, Pinterest on.
    assert!(!info.enabled);
    assert!(!info.s3_enabled);
    assert!(info.web_search_enabled);
    assert!(!info.google_enabled);
    assert!(info.pinterest_enabled);
}

// port: test/controllers/images_controller_test.rb:20 "config exposes accepted upload extensions"
#[test]
fn config_exposes_accepted_upload_extensions() {
    let t = TempTree::empty();
    let info = MediaApi::new(env_for(MediaConfig::new(t.path()), None).1).images_config();
    assert!(info.image_extensions.contains(&".png".to_owned()));
    assert!(info.video_extensions.contains(&".mp4".to_owned()));
}

// port: test/controllers/images_controller_test.rb:31 "search_web returns error when query is blank" — ported by media-client (search)
// port: test/controllers/images_controller_test.rb:41 "search_google returns error when no API key" — ported by media-client (search)
// port: test/controllers/images_controller_test.rb:52 "search_google returns error when query is blank" — ported by media-client (search)
// port: test/controllers/images_controller_test.rb:62 "search_pinterest returns error when query is blank" — ported by media-client (search)

// port: test/controllers/images_controller_test.rb:72 "upload returns error when no file provided"
#[tokio::test]
async fn upload_returns_error_when_no_file_provided() {
    let t = TempTree::empty();
    let api = MediaApi::new(env_for(MediaConfig::new(t.path()), None).1);
    let err = api.upload(None, &nil(), &nil(), None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert_eq!(err.message, ApiMessage::Key("errors.no_file_upload"));
}

// ---- WithImagesConfigured --------------------------------------------------

struct Configured {
    notes: TempTree,
    server_dir: PathBuf,
    api: MediaApi,
}

/// images_path = temp dir, local provider, extensions `.jpg .jpeg .png .gif .webp .bmp`
/// for both lists (the stub returned one list for every key).
fn configured() -> Configured {
    let notes = TempTree::empty();
    let server_dir = notes.path().join("server_images");
    std::fs::create_dir_all(&server_dir).unwrap();
    let mut c = local_config(notes.path());
    c.images_path = Some(server_dir.clone());
    c.video_upload_extensions = c.image_upload_extensions.clone();
    let api = MediaApi::new(env_for(c, None).1);
    Configured { notes, server_dir, api }
}

impl Configured {
    fn image(&self, name: &str) -> PathBuf {
        write(&self.server_dir, name, &PNG_1X1)
    }
}

// port: test/controllers/images_controller_test.rb:118 "config returns enabled when configured"
#[test]
fn config_returns_enabled_when_configured() {
    assert!(configured().api.images_config().enabled);
}

// port: test/controllers/images_controller_test.rb:126 "index returns list of images with pagination metadata"
#[test]
fn index_returns_list_of_images_with_pagination_metadata() {
    let f = configured();
    f.image("photo1.jpg");
    f.image("photo2.png");
    let idx = f.api.index(None, &nil(), &nil()).unwrap();
    assert_eq!(idx.images.len(), 2);
    assert_eq!(idx.total, 2);
    assert_eq!(idx.limit, 10);
    assert_eq!(idx.offset, 0);
}

// port: test/controllers/images_controller_test.rb:140 "index filters by search"
#[test]
fn index_filters_by_search() {
    let f = configured();
    f.image("cat.jpg");
    f.image("dog.jpg");
    let idx = f.api.index(Some("cat"), &nil(), &nil()).unwrap();
    assert_eq!(idx.images.len(), 1);
    assert_eq!(idx.images[0].name, "cat.jpg");
    assert_eq!(idx.total, 1);
}

// port: test/controllers/images_controller_test.rb:153 "index honors limit and offset params"
#[test]
fn index_honors_limit_and_offset_params() {
    let f = configured();
    for i in 0..5 {
        f.image(&format!("photo{i}.jpg"));
    }
    let idx = f.api.index(None, &Param::Int(2), &Param::Int(2)).unwrap();
    assert_eq!(idx.images.len(), 2);
    assert_eq!(idx.total, 5);
    assert_eq!(idx.limit, 2);
    assert_eq!(idx.offset, 2);
}

// port: test/controllers/images_controller_test.rb:166 "index clamps an oversized limit and a negative offset"
#[test]
fn index_clamps_an_oversized_limit_and_a_negative_offset() {
    let f = configured();
    f.image("photo.jpg");
    let idx = f.api.index(None, &Param::Int(500), &Param::Int(-10)).unwrap();
    assert_eq!(idx.limit, 100);
    assert_eq!(idx.offset, 0);
}

// port: test/controllers/images_controller_test.rb:177 "preview serves image file"
#[test]
fn preview_serves_image_file() {
    let f = configured();
    let p = f.image("test.png");
    assert_eq!(f.api.preview("test.png").unwrap().canonicalize().unwrap(), p.canonicalize().unwrap());
}

// port: test/controllers/images_controller_test.rb:184 "preview returns 404 for non-existent image"
#[test]
fn preview_returns_404_for_non_existent_image() {
    assert_eq!(configured().api.preview("nonexistent.jpg").unwrap_err().status, ApiStatus::NotFound);
}

// port: test/controllers/images_controller_test.rb:189 "destroy deletes an image and returns success"
#[test]
fn destroy_deletes_an_image_and_returns_success() {
    let f = configured();
    let p = f.image("delete_me.png");
    f.api.destroy("delete_me.png").unwrap();
    assert!(!p.exists());
}

// port: test/controllers/images_controller_test.rb:198 "destroy returns 422 for a non-existent image"
#[test]
fn destroy_returns_422_for_a_non_existent_image() {
    let err = configured().api.destroy("nope.png").unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(!err.message.as_str().is_empty());
}

// port: test/controllers/images_controller_test.rb:204 "upload saves file to notes/images directory"
#[tokio::test]
async fn upload_saves_file_to_notes_images_directory() {
    let f = configured();
    let url = f.api.upload(Some(Upload::new("test_upload.png", PNG_1X1)), &nil(), &nil(), None).await.unwrap();
    assert!(url.starts_with("images/"));
    assert!(url.contains("test_upload"));
    assert!(f.notes.path().join(&url).is_file());
    assert!(!f.server_dir.join(&url).exists(), "saved to <notes>/images, not images_path");
}

// port: test/controllers/images_controller_test.rb:237 "upload rejects a disallowed extension and stores nothing (stored-XSS guard)"
#[tokio::test]
async fn upload_rejects_a_disallowed_extension_and_stores_nothing() {
    let f = configured();
    let err = f
        .api
        .upload(Some(Upload::new("evil.html", "<script>alert(1)</script>")), &nil(), &nil(), None)
        .await
        .unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(err.message.as_str().contains("not an accepted image type"));
    let html: Vec<_> = std::fs::read_dir(f.notes.path().join("images"))
        .map(|rd| rd.filter_map(Result::ok).filter(|e| e.path().extension().is_some_and(|x| x == "html")).collect())
        .unwrap_or_default();
    assert!(html.is_empty(), "the rejected HTML upload must not be stored");
}

// port: test/controllers/images_controller_test.rb:263 "upload returns error for S3 when not configured"
#[tokio::test]
async fn upload_returns_error_for_s3_when_not_configured() {
    let f = configured();
    let err = f.api.upload(Some(Upload::new("test.png", PNG_1X1)), &nil(), &Param::str("true"), None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(err.message.as_str().contains("conflicts"));
}

fn b64_req(data: String, mime: &str, filename: &str) -> Base64Upload {
    Base64Upload { data, mime_type: mime.into(), filename: filename.into(), ..Default::default() }
}

// port: test/controllers/images_controller_test.rb:286 "upload_base64 saves base64 image data to notes/images"
#[tokio::test]
async fn upload_base64_saves_base64_image_data_to_notes_images() {
    let f = configured();
    let url = f.api.upload_base64(&b64_req(b64(&PNG_1X1), "image/png", "ai_test.png")).await.unwrap();
    assert!(url.starts_with("images/"));
    assert!(url.contains("ai_test"));
}

// port: test/controllers/images_controller_test.rb:316 "upload_base64 returns error when no data provided"
#[tokio::test]
async fn upload_base64_returns_error_when_no_data_provided() {
    let err = configured().api.upload_base64(&b64_req(String::new(), "", "")).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::BadRequest);
    assert_eq!(err.message, ApiMessage::Key("errors.no_image_data"));
}

// port: test/controllers/images_controller_test.rb:324 "upload_base64 returns error for invalid base64"
#[tokio::test]
async fn upload_base64_returns_error_for_invalid_base64() {
    let err = configured().api.upload_base64(&b64_req("not valid base64!!!".into(), "image/png", "")).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(err.message.as_str().contains("Invalid base64"));
}

// port: test/controllers/images_controller_test.rb:335 "upload_base64 generates filename when not provided"
#[tokio::test]
async fn upload_base64_generates_filename_when_not_provided() {
    let url = configured().api.upload_base64(&b64_req(b64(&PNG_1X1), "image/png", "")).await.unwrap();
    assert!(url.starts_with("images/"));
    assert!(url.contains("ai_generated_"));
}

// ---- WithS3Configured ------------------------------------------------------

struct WithS3 {
    _notes: TempTree,
    server_dir: PathBuf,
    mock: MockServer,
}

impl WithS3 {
    async fn new() -> Self {
        let notes = TempTree::empty();
        let server_dir = notes.path().join("server_images");
        std::fs::create_dir_all(&server_dir).unwrap();
        let mock = MockServer::start().await;
        s3_put_ok(&mock, "test-bucket").await;
        Self { _notes: notes, server_dir, mock }
    }

    /// provider s3, `test-key`/`test-secret`, bucket test-bucket, region us-west-2.
    fn config(&self) -> MediaConfig {
        let mut c = s3_config(self._notes.path(), "us-west-2");
        c.images_path = Some(self.server_dir.clone());
        c
    }

    fn api(&self, c: MediaConfig) -> MediaApi {
        MediaApi::new(env_for(c, Some(&self.mock)).1)
    }

    /// The original stubbed `feature_available?(:s3_upload)` to true while the
    /// key was nil, so recovery never ran. Equivalent here: recovery cannot
    /// persist `local`, so the endpoint's own S3 gate answers.
    fn api_without_recovery(&self, c: MediaConfig) -> MediaApi {
        let (settings, env) = env_for(c, Some(&self.mock));
        settings.fail_writes(true);
        MediaApi::new(env)
    }
}

// port: test/controllers/images_controller_test.rb:415 "upload_to_s3 uploads local image and returns S3 URL"
#[tokio::test]
async fn upload_to_s3_uploads_local_image_and_returns_s3_url() {
    let f = WithS3::new().await;
    write(&f.server_dir, "s3_test.png", &PNG_1X1);
    let url = f.api(f.config()).upload_to_s3("s3_test.png", &nil(), None).await.unwrap();
    assert_match(r"^https://test-bucket\.s3\.us-west-2\.amazonaws\.com/frankmd/", &url);
}

// port: test/controllers/images_controller_test.rb:425 "upload_to_s3 returns error when S3 not configured"
#[tokio::test]
async fn upload_to_s3_returns_error_when_s3_not_configured() {
    let f = WithS3::new().await;
    let mut c = f.config();
    c.s3.access_key_id = None;
    let err = f.api_without_recovery(c).upload_to_s3("test.png", &nil(), None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert_eq!(err.message, ApiMessage::Key("errors.s3_not_configured")); // en: "S3 not configured"
    assert!(!err.media_storage_fallback);
}

// port: test/controllers/images_controller_test.rb:435 "upload_external_to_s3 downloads image and uploads to S3"
#[tokio::test]
async fn upload_external_to_s3_downloads_image_and_uploads_to_s3() {
    let f = WithS3::new().await;
    external(&f.mock, "/photo.jpg", &sample_media(".jpg"), "image/jpeg").await; // security: real JPEG bytes
    let url = f.api(f.config()).upload_external_to_s3("http://example.com/photo.jpg", &nil(), None).await.unwrap();
    assert_match(r"^https://test-bucket\.s3\.us-west-2\.amazonaws\.com/frankmd/", &url);
}

// port: test/controllers/images_controller_test.rb:450 "upload_external_to_s3 returns error when URL is blank"
#[tokio::test]
async fn upload_external_to_s3_returns_error_when_url_is_blank() {
    let f = WithS3::new().await;
    let err = f.api(f.config()).upload_external_to_s3("", &nil(), None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::BadRequest);
    assert_eq!(err.message, ApiMessage::Key("errors.url_required"));
}

// port: test/controllers/images_controller_test.rb:458 "upload_external_to_s3 returns error when S3 not configured"
#[tokio::test]
async fn upload_external_to_s3_returns_error_when_s3_not_configured() {
    let f = WithS3::new().await;
    let mut c = f.config();
    c.s3.access_key_id = None;
    let err = f.api_without_recovery(c).upload_external_to_s3("http://example.com/photo.jpg", &nil(), None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert_eq!(err.message, ApiMessage::Key("errors.s3_not_configured"));
    assert!(!err.media_storage_fallback);
}

// ---- SearchActionsTest: ported by media-client ----------------------------
// port: test/controllers/images_controller_test.rb:496 "search_web returns images from DuckDuckGo" — media-client
// port: test/controllers/images_controller_test.rb:529 "search_web handles missing vqd token gracefully" — media-client
// port: test/controllers/images_controller_test.rb:540 "search_google returns images from Google CSE API" — media-client
// port: test/controllers/images_controller_test.rb:572 "search_google handles API error response" — media-client
// port: test/controllers/images_controller_test.rb:587 "search_pinterest uses DuckDuckGo with site filter" — media-client

// ---- ParseResizeRatioTest --------------------------------------------------

// port: test/controllers/images_controller_test.rb:622 "parse_resize_ratio returns 0.5 for legacy true value"
#[test]
fn parse_resize_ratio_returns_half_for_legacy_true_value() {
    assert_eq!(parse_resize_ratio(&Param::Bool(true)), Some(0.5));
    assert_eq!(parse_resize_ratio(&Param::str("true")), Some(0.5));
}

// port: test/controllers/images_controller_test.rb:627 "parse_resize_ratio returns nil for blank or false"
#[test]
fn parse_resize_ratio_returns_nil_for_blank_or_false() {
    assert_eq!(parse_resize_ratio(&Param::Nil), None);
    assert_eq!(parse_resize_ratio(&Param::str("")), None);
    assert_eq!(parse_resize_ratio(&Param::str("false")), None);
    assert_eq!(parse_resize_ratio(&Param::Bool(false)), None);
}

// port: test/controllers/images_controller_test.rb:634 "parse_resize_ratio parses valid ratio values"
#[test]
fn parse_resize_ratio_parses_valid_ratio_values() {
    assert_eq!(parse_resize_ratio(&Param::str("0.25")), Some(0.25));
    assert_eq!(parse_resize_ratio(&Param::str("0.5")), Some(0.5));
    assert_eq!(parse_resize_ratio(&Param::str("0.67")), Some(0.67));
    assert_eq!(parse_resize_ratio(&Param::str("1.0")), Some(1.0));
}

// port: test/controllers/images_controller_test.rb:641 "parse_resize_ratio rejects out-of-range values"
#[test]
fn parse_resize_ratio_rejects_out_of_range_values() {
    assert_eq!(parse_resize_ratio(&Param::str("0")), None);
    assert_eq!(parse_resize_ratio(&Param::str("-0.5")), None);
    assert_eq!(parse_resize_ratio(&Param::str("1.5")), None);
}

// ---- WithImagesDisabled ----------------------------------------------------

fn disabled() -> (TempTree, MediaApi) {
    let t = TempTree::empty();
    let api = MediaApi::new(env_for(local_config(t.path()), None).1);
    (t, api)
}

// port: test/controllers/images_controller_test.rb:661 "config returns disabled"
#[test]
fn config_returns_disabled() {
    assert!(!disabled().1.images_config().enabled);
}

// port: test/controllers/images_controller_test.rb:669 "index returns 404 when images disabled"
#[test]
fn index_returns_404_when_images_disabled() {
    let err = disabled().1.index(None, &nil(), &nil()).unwrap_err();
    assert_eq!(err.status, ApiStatus::NotFound);
    assert_eq!(err.message, ApiMessage::Key("errors.images_not_configured"));
}

// port: test/controllers/images_controller_test.rb:674 "preview returns 404 when images disabled"
#[test]
fn preview_returns_404_when_images_disabled() {
    assert_eq!(disabled().1.preview("test.jpg").unwrap_err().status, ApiStatus::NotFound);
}
