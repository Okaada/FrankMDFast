//! Port of test/services/media_service_test.rb (`MediaServiceTest` +
//! `MediaServiceS3Test`). Video extensions: the defaults
//! `.mp4 .webm .mkv .mov .avi .m4v .ogv`. The original wrote into the ambient
//! NOTES_PATH; here each test has a temp notes root.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use frankmd_media::config::{MediaConfig, DEFAULT_VIDEO_EXTENSIONS};
use frankmd_media::error::MediaError;
use frankmd_media::param::Param;
use frankmd_media::upload_storage::Upload;
use frankmd_media::video;
use frankmd_testkit::TempTree;
use uploads::*;
use wiremock::MockServer;

fn config(root: &std::path::Path) -> MediaConfig {
    let mut c = local_config(root);
    c.video_upload_extensions = exts(&DEFAULT_VIDEO_EXTENSIONS);
    c
}

fn uploaded(name: &str) -> Option<Upload> {
    // security: a real container of the claimed type (placeholder text is
    // rejected by content sniffing); disallowed names keep the placeholder.
    let bytes = if name.contains('.') && !name.ends_with(".txt") { sample_for(name) } else { b"fake video bytes".to_vec() };
    Some(Upload::new(name, bytes))
}

async fn save(root: &std::path::Path, name: &str) -> Result<String, MediaError> {
    let (_, env) = env_for(config(root), None);
    video::save_upload(&env, uploaded(name), &Param::Nil, None).await
}

// port: test/services/media_service_test.rb:22 "save_upload returns error when no file provided"
#[tokio::test]
async fn save_upload_returns_error_when_no_file_provided() {
    let t = TempTree::empty();
    let (_, env) = env_for(config(t.path()), None);
    assert!(video::save_upload(&env, None, &Param::Nil, None).await.is_err());
}

// port: test/services/media_service_test.rb:27 "save_upload writes an allowed video to notes/videos"
#[tokio::test]
async fn save_upload_writes_an_allowed_video_to_notes_videos() {
    let t = TempTree::empty();
    let url = save(t.path(), "clip.mp4").await.unwrap();
    assert!(url.starts_with("videos/"));
    assert!(url.contains("clip"));
    assert!(t.path().join(&url).exists());
}

// port: test/services/media_service_test.rb:39 "save_upload rejects a disallowed extension with a reason"
#[tokio::test]
async fn save_upload_rejects_a_disallowed_extension_with_a_reason() {
    let t = TempTree::empty();
    let msg = save(t.path(), "notes.txt").await.unwrap_err().to_string();
    assert!(msg.contains(".txt"));
    assert!(msg.contains("not an accepted video type"));
}

// port: test/services/media_service_test.rb:47 "save_upload rejects a file with no extension"
#[tokio::test]
async fn save_upload_rejects_a_file_with_no_extension() {
    let t = TempTree::empty();
    assert!(save(t.path(), "README").await.unwrap_err().to_string().contains("no extension"));
}

// port: test/services/media_service_test.rb:54 "save_upload sanitizes the stored filename"
#[tokio::test]
async fn save_upload_sanitizes_the_stored_filename() {
    let t = TempTree::empty();
    let url = save(t.path(), "my cool clip!.mp4").await.unwrap();
    assert!(!url.contains(' '));
    assert!(!url.contains('!'));
}

// port: test/services/media_service_test.rb:64 "save_upload neutralizes path traversal in the filename"
#[tokio::test]
async fn save_upload_neutralizes_path_traversal_in_the_filename() {
    let t = TempTree::empty();
    let url = save(t.path(), "../../../etc/evil.mp4").await.unwrap();
    assert!(url.starts_with("videos/"));
    assert_eq!(std::path::Path::new(&url).parent().unwrap(), std::path::Path::new("videos"), "flat name under videos/");
    let saved = t.path().join(&url);
    assert!(saved.canonicalize().unwrap().starts_with(t.path().join("videos").canonicalize().unwrap()));
    assert!(saved.exists());
}

fn s3(root: &std::path::Path) -> MediaConfig {
    let mut c = s3_config(root, "us-east-1");
    c.video_upload_extensions = exts(&DEFAULT_VIDEO_EXTENSIONS);
    c
}

// port: test/services/media_service_test.rb:109 "save_upload uploads to S3 with the correct video mime type"
#[tokio::test]
async fn save_upload_uploads_to_s3_with_the_correct_video_mime_type() {
    let t = TempTree::empty();
    let mock = MockServer::start().await;
    s3_put_ok(&mock, "test-bucket").await;
    let (_, env) = env_for(s3(t.path()), Some(&mock));
    let url = video::save_upload(&env, uploaded("movie.webm"), &Param::Nil, None).await.unwrap();
    assert_match(
        r"^https://test-bucket\.s3\.us-east-1\.amazonaws\.com/frankmd/\d{4}/\d{2}/movie-[0-9a-f]{16}\.webm$",
        &url,
    );
    let puts = puts(&mock).await;
    assert_eq!(puts.len(), 1);
    assert_eq!(header(&puts[0], "content-type").as_deref(), Some("video/webm"));
    assert_eq!(header(&puts[0], "if-none-match").as_deref(), Some("*"));
    assert!(!t.path().join("videos").exists(), "store_local must never run");
}

// port: test/services/media_service_test.rb:121 "save_upload handles S3 collisions without exposing provider details"
#[tokio::test]
async fn save_upload_handles_s3_collisions_without_exposing_provider_details() {
    let t = TempTree::empty();
    let mock = MockServer::start().await;
    s3_put_error(&mock, "PreconditionFailed", 412).await;
    let (_, env) = env_for(s3(t.path()), Some(&mock));
    let err = video::save_upload(&env, uploaded("movie.webm"), &Param::Bool(true), None).await.unwrap_err();
    assert_eq!(err.to_string(), "Video upload failed");
    assert!(!err.to_string().contains("private provider details"));
}

// port: test/services/media_service_test.rb:132 "save_upload rejects unavailable S3 without writing locally"
#[tokio::test]
async fn save_upload_rejects_unavailable_s3_without_writing_locally() {
    let t = TempTree::empty();
    let mut c = s3(t.path());
    c.s3.access_key_id = None;
    let (_, env) = env_for(c, None);
    let err = video::save_upload(&env, uploaded("local.mp4"), &Param::Bool(true), None).await.unwrap_err();
    assert_eq!(err.to_string(), "S3 is not configured");
    assert!(!t.path().join("videos").exists());
}
