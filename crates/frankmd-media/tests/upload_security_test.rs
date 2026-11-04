//! security: uploads accept only real media (user-approved decision; the
//! original trusted the extension alone). Every upload source goes through
//! `pipeline::store_media`, so each source is checked here once.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use frankmd_media::error::MediaError;
use frankmd_media::import::ImportRequest;
use frankmd_media::param::Param;
use frankmd_media::upload_storage::Upload;
use frankmd_media::{MediaKind, images, sniff, video};
use frankmd_testkit::TempTree;
use uploads::*;
use wiremock::MockServer;

const HTML: &[u8] =
    b"<!DOCTYPE html><html><body><script>alert(document.cookie)</script></body></html>";
const SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";

fn image_rejection() -> String {
    sniff::rejection_message(MediaKind::Image)
}

fn assert_rejected<T: std::fmt::Debug>(r: Result<T, MediaError>, want: &str) {
    match r {
        Err(MediaError::Rejected(m)) => assert_eq!(m, want),
        other => panic!("expected a content rejection, got {other:?}"),
    }
}

fn nothing_stored(t: &TempTree) {
    assert!(
        !t.path().join("images").exists(),
        "nothing may be written to images/"
    );
    assert!(
        !t.path().join("videos").exists(),
        "nothing may be written to videos/"
    );
}

/// A valid PNG with an HTML payload appended (a classic polyglot).
fn png_polyglot() -> Vec<u8> {
    let mut v = PNG_1X1.to_vec();
    v.extend_from_slice(b"<script>alert(1)</script>");
    v
}

/// A GIF header followed by markup in the browser's sniffing window.
fn gif_html_polyglot() -> Vec<u8> {
    let mut v = sample_media(".gif");
    v.splice(
        13..13,
        b"<html><script>alert(1)</script></html>".iter().copied(),
    );
    v
}

async fn upload(t: &TempTree, name: &str, bytes: &[u8]) -> Result<String, MediaError> {
    let (_, env) = env_for(local_config(t.path()), None);
    images::upload_file(
        &env,
        Some(Upload::new(name, bytes)),
        None,
        &Param::Nil,
        None,
    )
    .await
}

#[tokio::test]
async fn text_named_png_is_rejected() {
    let t = TempTree::empty();
    assert_rejected(
        upload(&t, "notes.png", b"just some text").await,
        &image_rejection(),
    );
    nothing_stored(&t);
}

#[tokio::test]
async fn html_named_jpg_is_rejected() {
    let t = TempTree::empty();
    assert_rejected(upload(&t, "page.jpg", HTML).await, &image_rejection());
    nothing_stored(&t);
}

#[tokio::test]
async fn svg_with_script_is_rejected_under_an_image_extension() {
    let t = TempTree::empty();
    assert_rejected(upload(&t, "logo.png", SVG).await, &image_rejection());
    nothing_stored(&t);
}

#[tokio::test]
async fn svg_is_rejected_even_when_svg_is_allow_listed() {
    let t = TempTree::empty();
    let mut c = local_config(t.path());
    c.image_upload_extensions.push(".svg".into());
    let (_, env) = env_for(c, None);
    let r = images::upload_file(
        &env,
        Some(Upload::new("logo.svg", SVG)),
        None,
        &Param::Nil,
        None,
    )
    .await;
    assert_rejected(r, &image_rejection());
    nothing_stored(&t);
}

#[tokio::test]
async fn polyglots_are_rejected() {
    for (name, bytes) in [
        ("poly.png", png_polyglot()),
        ("poly.gif", gif_html_polyglot()),
    ] {
        let t = TempTree::empty();
        assert_rejected(upload(&t, name, &bytes).await, &image_rejection());
        nothing_stored(&t);
    }
}

#[tokio::test]
async fn a_bare_signature_without_a_readable_header_is_rejected() {
    let t = TempTree::empty();
    assert_rejected(
        upload(&t, "fake.png", b"\x89PNG\r\n\x1a\nfake").await,
        &image_rejection(),
    );
    nothing_stored(&t);
}

#[tokio::test]
async fn content_type_must_be_in_the_allow_list() {
    // PNG bytes behind an allowed `.jpg` name, with PNG not allow-listed.
    let t = TempTree::empty();
    let mut c = local_config(t.path());
    c.image_upload_extensions = exts(&[".jpg"]);
    let (_, env) = env_for(c, None);
    let r = images::upload_file(
        &env,
        Some(Upload::new("photo.jpg", PNG_1X1)),
        None,
        &Param::Nil,
        None,
    )
    .await;
    assert_rejected(r, &image_rejection());
    nothing_stored(&t);
}

#[tokio::test]
async fn every_allowed_image_type_is_accepted() {
    for name in ["a.png", "a.jpg", "a.jpeg", "a.gif", "a.webp", "a.bmp"] {
        let t = TempTree::empty();
        let url = upload(&t, name, &sample_for(name)).await.unwrap();
        assert!(t.path().join(&url).is_file(), "{name}");
    }
}

#[tokio::test]
async fn base64_html_is_rejected() {
    let t = TempTree::empty();
    let (_, env) = env_for(local_config(t.path()), None);
    let r = images::upload_base64_data(
        &env,
        &b64(HTML),
        Some("image/png"),
        Some("ai.png"),
        &Param::Nil,
        None,
    )
    .await;
    assert_rejected(r, &image_rejection());
    nothing_stored(&t);
}

#[tokio::test]
async fn videos_must_be_real_video_containers() {
    let t = TempTree::empty();
    let (_, env) = env_for(local_config(t.path()), None);
    for (name, bytes) in [
        ("clip.mp4", b"fake video bytes".to_vec()),
        ("clip.webm", HTML.to_vec()),
        ("clip.mp4", PNG_1X1.to_vec()),
    ] {
        let r = video::save_upload(&env, Some(Upload::new(name, bytes)), &Param::Nil, None).await;
        assert_rejected(r, &sniff::rejection_message(MediaKind::Video));
    }
    nothing_stored(&t);
    for name in ["clip.mp4", "clip.webm"] {
        let url = video::save_upload(
            &env,
            Some(Upload::new(name, sample_for(name))),
            &Param::Nil,
            None,
        )
        .await
        .unwrap();
        assert!(t.path().join(&url).is_file(), "{name}");
    }
}

#[tokio::test]
async fn url_import_with_an_image_content_type_but_html_body_is_rejected() {
    let t = TempTree::empty();
    let mock = MockServer::start().await;
    external(&mock, "/cat.png", HTML, "image/png").await;
    let (_, env) = env_for(local_config(t.path()), Some(&mock));
    let req = ImportRequest {
        url: Param::str("http://example.com/cat.png"),
        ..Default::default()
    };
    assert_rejected(
        frankmd_media::import::import_image(&env, &req).await,
        &image_rejection(),
    );
    nothing_stored(&t);
}

#[tokio::test]
async fn server_folder_import_of_text_named_png_is_rejected() {
    let t = TempTree::empty();
    let server = t.path().join("server");
    write(&server, "fake.png", b"not an image");
    let mut c = local_config(t.path());
    c.images_path = Some(server);
    let (_, env) = env_for(c, None);
    let req = ImportRequest {
        path: Param::str("fake.png"),
        ..Default::default()
    };
    assert_rejected(
        frankmd_media::import::import_image(&env, &req).await,
        &image_rejection(),
    );
    nothing_stored(&t);
}

#[tokio::test]
async fn rejected_content_is_never_sent_to_s3() {
    let t = TempTree::empty();
    let mock = MockServer::start().await;
    s3_put_ok(&mock, "test-bucket").await;
    external(&mock, "/x.jpg", HTML, "image/jpeg").await;
    let (_, env) = env_for(s3_config(t.path(), "us-east-1"), Some(&mock));
    let r = images::upload_file(
        &env,
        Some(Upload::new("x.png", HTML)),
        None,
        &Param::Nil,
        None,
    )
    .await;
    assert_rejected(r, &image_rejection());
    let r = images::download_and_upload_to_s3(&env, "http://example.com/x.jpg", None, None).await;
    assert_rejected(r, &image_rejection());
    assert!(puts(&mock).await.is_empty(), "no PUT may reach S3");
    nothing_stored(&t);
}

#[test]
fn validate_bytes_reports_the_detected_kind() {
    let all = exts(&[".png", ".jpg", ".mp4"]);
    assert_eq!(
        frankmd_media::validate_bytes("a.png", &PNG_1X1, &all).unwrap(),
        MediaKind::Image
    );
    assert_eq!(
        frankmd_media::validate_bytes("a.mp4", &sample_media(".mp4"), &all).unwrap(),
        MediaKind::Video
    );
    assert!(frankmd_media::validate_bytes("a.png", HTML, &all).is_err());
    assert!(frankmd_media::validate_bytes("a.html", HTML, &all).is_err());
}
