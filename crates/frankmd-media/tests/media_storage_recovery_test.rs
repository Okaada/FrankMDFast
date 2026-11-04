//! Port of test/controllers/media_storage_recovery_test.rb.
//!
//! Rails pieces → Rust: `GET /config` / page loads → `MediaApi::storage_status`
//! (consumes the warning); `GET /images/config` → `images_config` (keeps it);
//! the four upload endpoints → `MediaApi::{upload, upload_base64, import, media_upload}`;
//! `Config.new.get("media_storage_provider")` → `MemorySettings::current()`;
//! `Config#set(...)` returning false → `MemorySettings::fail_writes(true)`;
//! mocked `Aws::S3::Client` errors → S3 error documents from wiremock.
//! Loops over keys/endpoints are expanded into one Rust test per row.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use std::path::PathBuf;
use std::sync::Arc;

use frankmd_media::api::{ApiError, ApiStatus, Base64Upload, MediaApi};
use frankmd_media::config::MediaConfig;
use frankmd_media::env::{MemorySettings, SettingsStore};
use frankmd_media::import::ImportRequest;
use frankmd_media::param::Param;
use frankmd_media::upload_storage::Upload;
use frankmd_testkit::TempTree;
use uploads::*;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Clone, Copy, Debug)]
enum Endpoint {
    ImagesUpload,
    ImagesUploadBase64,
    ImagesImport,
    MediaUpload,
}

struct Fixture {
    notes: TempTree,
    _server_dir: PathBuf,
    mock: MockServer,
    settings: Arc<MemorySettings>,
    api: MediaApi,
}

impl Fixture {
    /// setup: server folder with photo.png, `images_path` set, `select_s3`.
    async fn new() -> Self {
        let notes = TempTree::empty();
        let server_dir = notes.path().join("server");
        // security: real media bytes throughout (placeholder text is rejected
        // by content sniffing before any provider is contacted).
        write(&server_dir, "photo.png", &PNG_1X1);
        let mock = MockServer::start().await;
        let mut c = MediaConfig::new(notes.path());
        c.images_path = Some(server_dir.clone());
        select_s3(&mut c);
        let (settings, env) = env_for(c, Some(&mock));
        Self { notes, _server_dir: server_dir, mock, settings, api: MediaApi::new(env) }
    }

    fn set(&self, f: impl FnOnce(&mut MediaConfig)) {
        self.settings.update(f);
    }

    fn select_s3(&self) {
        self.set(select_s3);
    }

    fn provider(&self) -> Option<String> {
        self.settings.current().media_storage_provider
    }

    fn no_local_media(&self) {
        assert!(!self.notes.path().join("images").exists());
        assert!(!self.notes.path().join("videos").exists());
    }

    async fn post(&self, ep: Endpoint) -> Result<String, ApiError> {
        let nil = Param::Nil;
        match ep {
            Endpoint::ImagesUpload => self.api.upload(Some(Upload::new("photo.png", PNG_1X1)), &nil, &nil, None).await,
            Endpoint::ImagesUploadBase64 => {
                let req = Base64Upload {
                    data: b64(&PNG_1X1),
                    mime_type: "image/png".into(),
                    filename: "photo.png".into(),
                    ..Default::default()
                };
                self.api.upload_base64(&req).await
            }
            Endpoint::ImagesImport => {
                self.api.import(&ImportRequest { path: Param::str("photo.png"), ..Default::default() }).await
            }
            Endpoint::MediaUpload => self.api.media_upload(Some(Upload::new("clip.mp4", sample_media(".mp4"))), &nil, None).await,
        }
    }
}

/// `select_s3`: key/secret/bucket, endpoint blank, provider s3.
fn select_s3(c: &mut MediaConfig) {
    c.s3.access_key_id = Some("key".into());
    c.s3.secret_access_key = Some("secret".into());
    c.s3.bucket = Some("bucket".into());
    c.s3.endpoint = None;
    c.media_storage_provider = Some("s3".into());
}

fn unprocessable_with_fallback(r: Result<String, ApiError>) {
    let err = r.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(err.media_storage_fallback, "expected media_storage_fallback: true, got {err:?}");
}

async fn configuration_refresh_persists_local_after_removal(key: &str) {
    let f = Fixture::new().await;
    f.set(|c| match key {
        "aws_access_key_id" => c.s3.access_key_id = None,
        "aws_secret_access_key" => c.s3.secret_access_key = None,
        _ => c.s3.bucket = None,
    });
    let s = f.api.storage_status();
    assert_eq!(s.media_storage_provider.as_deref(), Some("local"));
    assert!(s.media_storage_fallback);
    assert_eq!(f.provider().as_deref(), Some("local"));
    assert!(!f.api.storage_status().media_storage_fallback);
}

// port: test/controllers/media_storage_recovery_test.rb:40 "configuration refresh persists Local after #{key} is removed and warns once" [aws_access_key_id]
#[tokio::test]
async fn configuration_refresh_persists_local_after_aws_access_key_id_is_removed() {
    configuration_refresh_persists_local_after_removal("aws_access_key_id").await;
}

// port: test/controllers/media_storage_recovery_test.rb:40 "configuration refresh persists Local after #{key} is removed and warns once" [aws_secret_access_key]
#[tokio::test]
async fn configuration_refresh_persists_local_after_aws_secret_access_key_is_removed() {
    configuration_refresh_persists_local_after_removal("aws_secret_access_key").await;
}

// port: test/controllers/media_storage_recovery_test.rb:40 "configuration refresh persists Local after #{key} is removed and warns once" [aws_s3_bucket]
#[tokio::test]
async fn configuration_refresh_persists_local_after_aws_s3_bucket_is_removed() {
    configuration_refresh_persists_local_after_removal("aws_s3_bucket").await;
}

// port: test/controllers/media_storage_recovery_test.rb:52 "invalid endpoints recover but S3-compatible endpoints remain valid"
#[tokio::test]
async fn invalid_endpoints_recover_but_s3_compatible_endpoints_remain_valid() {
    let f = Fixture::new().await;
    for endpoint in ["not a URL", "ftp://s3.example.com", "https:///"] {
        f.select_s3();
        f.set(|c| c.s3.endpoint = Some(endpoint.into()));
        let s = f.api.storage_status();
        assert_eq!(s.media_storage_provider.as_deref(), Some("local"), "{endpoint}");
        assert!(s.media_storage_fallback, "{endpoint}");
    }
    f.select_s3();
    f.set(|c| c.s3.endpoint = Some("http://minio.local:9000".into()));
    let s = f.api.storage_status();
    assert_eq!(s.media_storage_provider.as_deref(), Some("s3"));
    assert!(!s.media_storage_fallback);
}

// port: test/controllers/media_storage_recovery_test.rb:67 "page loads warn after recovery and image config preserves the warning for shared controls"
// The HTML assertions (data-media-storage-* attributes, the alertdialog) are UI
// and belong to the app's tests; the state contract is asserted here.
#[tokio::test]
async fn page_loads_warn_after_recovery_and_image_config_preserves_the_warning() {
    let f = Fixture::new().await;
    f.set(|c| c.s3.bucket = None);
    let info = f.api.images_config();
    assert_eq!(info.media_storage_provider.as_deref(), Some("local"));
    assert!(info.media_storage_fallback);
    // first page load shows the warning ...
    let page = f.api.storage_status();
    assert_eq!(page.media_storage_provider.as_deref(), Some("local"));
    assert!(page.media_storage_fallback);
    // ... the next one does not.
    assert!(!f.api.storage_status().media_storage_fallback);
}

async fn recovers_missing_settings_before_writing(ep: Endpoint) {
    let f = Fixture::new().await;
    f.set(|c| c.s3.access_key_id = None);
    unprocessable_with_fallback(f.post(ep).await);
    assert_eq!(f.provider().as_deref(), Some("local"));
    f.no_local_media();
    let url = f.post(ep).await.unwrap();
    assert!(f.notes.path().join(&url).is_file(), "{ep:?} retry must store locally");
    assert!(puts(&f.mock).await.is_empty());
}

// port: test/controllers/media_storage_recovery_test.rb:81 "#{endpoint} recovers missing settings before writing and can be retried locally" [/images/upload]
#[tokio::test]
async fn images_upload_recovers_missing_settings_before_writing() {
    recovers_missing_settings_before_writing(Endpoint::ImagesUpload).await;
}

// port: test/controllers/media_storage_recovery_test.rb:81 "#{endpoint} recovers missing settings before writing and can be retried locally" [/images/upload_base64]
#[tokio::test]
async fn images_upload_base64_recovers_missing_settings_before_writing() {
    recovers_missing_settings_before_writing(Endpoint::ImagesUploadBase64).await;
}

// port: test/controllers/media_storage_recovery_test.rb:81 "#{endpoint} recovers missing settings before writing and can be retried locally" [/images/import]
#[tokio::test]
async fn images_import_recovers_missing_settings_before_writing() {
    recovers_missing_settings_before_writing(Endpoint::ImagesImport).await;
}

// port: test/controllers/media_storage_recovery_test.rb:81 "#{endpoint} recovers missing settings before writing and can be retried locally" [/media/upload]
#[tokio::test]
async fn media_upload_recovers_missing_settings_before_writing() {
    recovers_missing_settings_before_writing(Endpoint::MediaUpload).await;
}

async fn recovers_an_invalid_sdk_region(ep: Endpoint) {
    let f = Fixture::new().await;
    f.set(|c| c.s3.region = Some("bad_region".into()));
    unprocessable_with_fallback(f.post(ep).await);
    assert_eq!(f.provider().as_deref(), Some("local"));
    f.no_local_media();
    assert!(puts(&f.mock).await.is_empty(), "the upload is not retried / never sent");
    assert!(f.api.storage_status().media_storage_fallback, "warning still pending");
    let url = f.post(ep).await.unwrap();
    assert!(f.notes.path().join(&url).is_file());
}

// port: test/controllers/media_storage_recovery_test.rb:94 "#{endpoint} recovers an invalid SDK region without retrying the upload" [/images/upload]
#[tokio::test]
async fn images_upload_recovers_an_invalid_sdk_region() {
    recovers_an_invalid_sdk_region(Endpoint::ImagesUpload).await;
}

// port: test/controllers/media_storage_recovery_test.rb:94 "#{endpoint} recovers an invalid SDK region without retrying the upload" [/images/upload_base64]
#[tokio::test]
async fn images_upload_base64_recovers_an_invalid_sdk_region() {
    recovers_an_invalid_sdk_region(Endpoint::ImagesUploadBase64).await;
}

// port: test/controllers/media_storage_recovery_test.rb:94 "#{endpoint} recovers an invalid SDK region without retrying the upload" [/images/import]
#[tokio::test]
async fn images_import_recovers_an_invalid_sdk_region() {
    recovers_an_invalid_sdk_region(Endpoint::ImagesImport).await;
}

// port: test/controllers/media_storage_recovery_test.rb:94 "#{endpoint} recovers an invalid SDK region without retrying the upload" [/media/upload]
#[tokio::test]
async fn media_upload_recovers_an_invalid_sdk_region() {
    recovers_an_invalid_sdk_region(Endpoint::MediaUpload).await;
}

async fn recovers_rejected_s3_credentials(ep: Endpoint) {
    let f = Fixture::new().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(403).set_body_raw(s3_error_body("InvalidAccessKeyId"), "application/xml"))
        .expect(1)
        .mount(&f.mock)
        .await;
    unprocessable_with_fallback(f.post(ep).await);
    assert_eq!(f.provider().as_deref(), Some("local"));
    f.no_local_media();
    assert!(f.api.storage_status().media_storage_fallback);
    assert!(!f.api.storage_status().media_storage_fallback);
    f.mock.verify().await;
}

// port: test/controllers/media_storage_recovery_test.rb:109 "#{endpoint} recovers rejected S3 credentials without retrying the upload" [/images/upload]
#[tokio::test]
async fn images_upload_recovers_rejected_s3_credentials() {
    recovers_rejected_s3_credentials(Endpoint::ImagesUpload).await;
}

// port: test/controllers/media_storage_recovery_test.rb:109 "#{endpoint} recovers rejected S3 credentials without retrying the upload" [/images/upload_base64]
#[tokio::test]
async fn images_upload_base64_recovers_rejected_s3_credentials() {
    recovers_rejected_s3_credentials(Endpoint::ImagesUploadBase64).await;
}

// port: test/controllers/media_storage_recovery_test.rb:109 "#{endpoint} recovers rejected S3 credentials without retrying the upload" [/images/import]
#[tokio::test]
async fn images_import_recovers_rejected_s3_credentials() {
    recovers_rejected_s3_credentials(Endpoint::ImagesImport).await;
}

// port: test/controllers/media_storage_recovery_test.rb:109 "#{endpoint} recovers rejected S3 credentials without retrying the upload" [/media/upload]
#[tokio::test]
async fn media_upload_recovers_rejected_s3_credentials() {
    recovers_rejected_s3_credentials(Endpoint::MediaUpload).await;
}

// port: test/controllers/media_storage_recovery_test.rb:126 "legacy S3 endpoints also recover missing settings without importing"
#[tokio::test]
async fn legacy_s3_endpoints_also_recover_missing_settings_without_importing() {
    let f = Fixture::new().await;
    for legacy in ["upload_to_s3", "upload_external_to_s3"] {
        f.select_s3();
        f.set(|c| c.s3.secret_access_key = None);
        let r = if legacy == "upload_to_s3" {
            f.api.upload_to_s3("photo.png", &Param::Nil, None).await
        } else {
            f.api.upload_external_to_s3("http://example.com/photo.png", &Param::Nil, None).await
        };
        unprocessable_with_fallback(r);
        assert_eq!(f.provider().as_deref(), Some("local"), "{legacy}");
        assert!(!f.notes.path().join("images").exists());
    }
    assert!(requests(&f.mock).await.is_empty(), "nothing fetched or uploaded");
}

// port: test/controllers/media_storage_recovery_test.rb:138 "temporary failures do not change the provider or claim recovery"
#[tokio::test]
async fn temporary_failures_do_not_change_the_provider_or_claim_recovery() {
    // SlowDown, InternalError, and Seahorse::Client::NetworkingError (here: the
    // S3 transport points at a closed port → connection refused).
    for (code, status) in [("SlowDown", 503), ("InternalError", 500), ("NetworkingError", 0)] {
        let mut f = Fixture::new().await;
        if status == 0 {
            f.api.env.s3_transport = Some(url::Url::parse("http://127.0.0.1:1/").unwrap());
        } else {
            s3_put_error(&f.mock, code, status).await;
        }
        let err = f.post(Endpoint::MediaUpload).await.unwrap_err();
        assert_eq!(err.status, ApiStatus::Unprocessable, "{code}");
        assert_eq!(f.provider().as_deref(), Some("s3"), "{code}");
        assert!(!err.media_storage_fallback, "{code}");
        assert!(!f.notes.path().join("videos").exists());
    }
}

// port: test/controllers/media_storage_recovery_test.rb:152 "restoring S3 before a pending warning is displayed does not claim Local is active"
#[tokio::test]
async fn restoring_s3_before_a_pending_warning_is_displayed_does_not_claim_local_is_active() {
    let f = Fixture::new().await;
    f.set(|c| c.s3.bucket = None);
    assert!(f.api.images_config().media_storage_fallback);
    f.select_s3();
    let s = f.api.storage_status();
    assert_eq!(s.media_storage_provider.as_deref(), Some("s3"));
    assert!(!s.media_storage_fallback);
}

// port: test/controllers/media_storage_recovery_test.rb:162 "failed preference persistence leaves S3 selected and never claims recovery"
#[tokio::test]
async fn failed_preference_persistence_leaves_s3_selected_and_never_claims_recovery() {
    let f = Fixture::new().await;
    f.set(|c| c.s3.bucket = None);
    f.settings.fail_writes(true);
    let s = f.api.storage_status();
    assert_eq!(s.media_storage_provider.as_deref(), Some("s3"));
    assert!(!s.media_storage_fallback);
    let err = f.post(Endpoint::MediaUpload).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(!err.media_storage_fallback);
    assert!(!f.notes.path().join("videos").exists());
}

// port: test/controllers/media_storage_recovery_test.rb:174 "a failed recovery save after credential rejection retains S3"
#[tokio::test]
async fn a_failed_recovery_save_after_credential_rejection_retains_s3() {
    let f = Fixture::new().await;
    f.settings.fail_writes(true);
    s3_put_error(&f.mock, "AccessDenied", 403).await;
    let err = f.post(Endpoint::MediaUpload).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert_eq!(f.provider().as_deref(), Some("s3"));
    assert!(!err.media_storage_fallback);
}
