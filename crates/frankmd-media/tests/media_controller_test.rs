//! Port of test/controllers/media_controller_test.rb (`POST /media/upload`
//! → `MediaApi::media_upload`). Default config (local provider, default video
//! extensions) in a temp notes root.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use frankmd_media::api::{ApiMessage, ApiStatus, MediaApi};
use frankmd_media::config::MediaConfig;
use frankmd_media::param::Param;
use frankmd_media::upload_storage::Upload;
use frankmd_testkit::TempTree;
use uploads::*;

fn api(t: &TempTree) -> MediaApi {
    MediaApi::new(env_for(MediaConfig::new(t.path()), None).1)
}

fn video_file(name: &str) -> Option<Upload> {
    Some(Upload::new(name, "fake video bytes"))
}

// port: test/controllers/media_controller_test.rb:10 "upload returns error when no file provided"
#[tokio::test]
async fn upload_returns_error_when_no_file_provided() {
    let t = TempTree::empty();
    let err = api(&t).media_upload(None, &Param::Nil, None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    // t("errors.no_file_upload") — en text contains "No file".
    assert_eq!(err.message, ApiMessage::Key("errors.no_file_upload"));
}

// port: test/controllers/media_controller_test.rb:18 "upload saves an allowed video to notes/videos"
#[tokio::test]
async fn upload_saves_an_allowed_video_to_notes_videos() {
    let t = TempTree::empty();
    let url = api(&t).media_upload(video_file("clip.mp4"), &Param::Nil, None).await.unwrap();
    assert!(url.starts_with("videos/"));
    assert!(url.contains("clip"));
    assert!(t.path().join(&url).is_file());
}

// port: test/controllers/media_controller_test.rb:31 "upload rejects a disallowed extension with a reason"
#[tokio::test]
async fn upload_rejects_a_disallowed_extension_with_a_reason() {
    let t = TempTree::empty();
    let err = api(&t).media_upload(video_file("notes.txt"), &Param::Nil, None).await.unwrap_err();
    assert_eq!(err.status, ApiStatus::Unprocessable);
    assert!(err.message.as_str().contains("not an accepted video type"));
}
