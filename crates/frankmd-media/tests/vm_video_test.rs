//! Insert video dialog view model. Adapted rows of docs/testing/ui-test-plan.md
//! §13 (video_dialog_controller.test.js).
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use frankmd_media::vm::Text;
use frankmd_media::vm::markup::MediaInsert;
use frankmd_media::vm::picker::{LocalFile, StoreError};
use frankmd_media::vm::storage::Provider;
use frankmd_media::vm::video::*;
use frankmd_media::youtube::{Video, VideoResults};

fn dialog(youtube: bool) -> VideoDialog {
    let mut d = VideoDialog::default();
    d.open(
        Provider::Local,
        true,
        youtube,
        Vec::new(),
        jiff::civil::date(2026, 7, 4),
    );
    d
}

fn video(id: &str, title: &str) -> Video {
    Video {
        id: Some(id.into()),
        title: Some(title.into()),
        channel: Some("chan".into()),
        thumbnail: None,
    }
}

fn with_results(d: &mut VideoDialog, videos: Vec<Video>) {
    d.set_query("test query");
    let effects = d.search();
    let [VideoEffect::Search { generation, query }] = effects.as_slice() else {
        panic!("{effects:?}")
    };
    assert_eq!(query, "test query");
    d.search_result(
        *generation,
        Ok(VideoResults {
            videos,
            error: None,
        }),
    );
}

fn clip(name: &str) -> LocalFile {
    LocalFile {
        name: name.into(),
        path: Some(PathBuf::from(format!("/tmp/{name}"))),
        bytes: None,
        size: 10,
        modified_ms: 1,
        dims: None,
    }
}

// rows 1, 2, 5-8 (reset on open), 6 (insert disabled)
#[test]
fn opens_clean_on_upload() {
    let mut d = dialog(true);
    assert!(d.results().is_empty());
    assert_eq!(d.selected(), None);
    d.set_tab(VideoTab::Url);
    d.set_url("https://youtu.be/dQw4w9WgXcQ");
    with_results(&mut d, vec![video("a", "A")]);
    d.select(0);
    d.open(
        Provider::Local,
        true,
        true,
        Vec::new(),
        jiff::civil::date(2026, 7, 4),
    );
    assert_eq!(d.tab(), VideoTab::Upload);
    assert_eq!(d.url(), "");
    assert_eq!(d.detected(), None);
    assert!(d.results().is_empty());
    assert_eq!(d.selected(), None);
    assert!(!d.can_insert());
}

// FE-340/341, rows 12-16
#[test]
fn url_detection_and_enter() {
    let mut d = dialog(true);
    d.set_tab(VideoTab::Url);
    assert_eq!(d.url_hint(), Some(Text::key("dialogs.video.preview_hint")));
    d.set_url("page.html");
    assert_eq!(
        d.url_hint(),
        Some(Text::key("dialogs.video.unknown_format"))
    );
    assert!(d.url_enter().is_empty(), "invalid URL: nothing");
    d.set_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1");
    assert_eq!(
        d.detected(),
        Some(&Detected::YouTube {
            id: "dQw4w9WgXcQ".into()
        })
    );
    let effects = d.url_enter();
    let [VideoEffect::Insert(MediaInsert::Block(embed))] = effects.as_slice() else {
        panic!()
    };
    assert!(embed.contains("src=\"https://www.youtube.com/embed/dQw4w9WgXcQ\""));
    assert!(embed.contains("title=\"YouTube video player\""));
    d.set_url("/media/clip.MOV");
    assert_eq!(
        d.detected(),
        Some(&Detected::File {
            url: "/media/clip.MOV".into(),
            filename: "clip.MOV".into()
        })
    );
    let effects = d.confirm();
    let [VideoEffect::Insert(MediaInsert::Block(embed))] = effects.as_slice() else {
        panic!()
    };
    assert!(embed.contains("type=\"video/quicktime\""));
    d.set_url("/media/clip.mp4?x=1");
    assert_eq!(
        d.detected(),
        None,
        "a query string defeats the extension check"
    );
    d.set_url("");
    assert_eq!(
        d.confirm(),
        vec![VideoEffect::Close],
        "nothing detected just closes"
    );
}

// row 14: thumbnail fallback chain
#[test]
fn youtube_thumbnail_falls_back_once_then_hides() {
    let mut d = dialog(true);
    d.set_url("abcdefghijk");
    assert_eq!(
        d.url_thumbnail().as_deref(),
        Some("https://img.youtube.com/vi/abcdefghijk/mqdefault.jpg")
    );
    d.url_thumbnail_failed();
    assert_eq!(
        d.url_thumbnail().as_deref(),
        Some("https://img.youtube.com/vi/abcdefghijk/default.jpg")
    );
    d.url_thumbnail_failed();
    assert_eq!(d.url_thumbnail(), None);
}

// rows 3, 17, 18
#[test]
fn youtube_search_guards() {
    let mut no_key = dialog(false);
    no_key.set_query("x");
    assert!(no_key.search().is_empty());
    assert!(!no_key.youtube_enabled());
    let mut d = dialog(true);
    assert!(d.search().is_empty());
    assert_eq!(
        d.status().0,
        Some(&Text::key("status.please_enter_keywords"))
    );
}

// rows 19-25, 37 (selection, embed, Hugo, empty id, grid keys)
#[test]
fn youtube_results_select_navigate_and_insert() {
    let mut d = dialog(true);
    d.select(0);
    assert_eq!(d.selected(), None, "no results → no-op");
    with_results(
        &mut d,
        vec![
            video("xyz789", "Test Title"),
            video("b", "B"),
            video("c", "C"),
            video("d", "D"),
        ],
    );
    assert_eq!(
        d.status().0,
        Some(&Text::with("status.found_videos", &[("count", "4")]))
    );
    assert!(
        d.grid_key(GridKey::Down),
        "down from the field enters the grid"
    );
    assert_eq!(d.selected(), Some(0));
    d.grid_key(GridKey::Right);
    assert_eq!(d.selected(), Some(1));
    d.grid_key(GridKey::Down);
    assert_eq!(d.selected(), Some(3));
    d.grid_key(GridKey::Up);
    assert_eq!(d.selected(), Some(1));
    assert!(
        !d.grid_key(GridKey::Up),
        "up from the first row returns to the field"
    );
    assert_eq!(d.selected(), None);
    let effects = d.choose_result(0);
    let [VideoEffect::Insert(MediaInsert::Block(embed))] = effects.as_slice() else {
        panic!()
    };
    assert!(embed.contains("youtube.com/embed/xyz789") && embed.contains("title=\"Test Title\""));
    d.set_hugo(true);
    assert_eq!(
        d.choose_result(0),
        vec![VideoEffect::Insert(MediaInsert::Block(
            "{{< youtube id=\"xyz789\" title=\"Test Title\" >}}".into()
        ))]
    );
    d.open(
        Provider::Local,
        true,
        true,
        Vec::new(),
        jiff::civil::date(2026, 7, 4),
    );
    assert!(d.hugo(), "the Hugo checkbox persists across opens");
    with_results(
        &mut d,
        vec![Video {
            id: Some(String::new()),
            title: None,
            channel: None,
            thumbnail: None,
        }],
    );
    assert!(d.choose_result(0).is_empty(), "result without id ignored");
    with_results(&mut d, Vec::new());
    assert_eq!(d.status().0, Some(&Text::key("status.no_videos_found")));
}

// rows 26-30, 35 (upload)
#[test]
fn upload_validation_single_flight_and_result() {
    let mut d = dialog(true);
    assert!(d.drop_files(vec![clip("notes.txt")]).is_empty());
    let Some(Text::Key(key, _)) = d.upload_feedback() else {
        panic!()
    };
    assert_eq!(*key, "dialogs.video.drop_rejected");
    let effects = d.drop_files(vec![clip("clip.mp4"), clip("other.mp4")]);
    let [VideoEffect::Upload { file, s3_prefix }] = effects.as_slice() else {
        panic!("{effects:?}")
    };
    assert_eq!(file.name, "clip.mp4");
    assert_eq!(s3_prefix, &None);
    assert_eq!(
        d.upload_state(),
        &UploadState::Uploading {
            name: "clip.mp4".into()
        }
    );
    assert!(
        d.drop_files(vec![clip("again.mp4")]).is_empty(),
        "no concurrent uploads"
    );
    assert!(d.choose_file().is_empty());
    d.upload_result(Ok("videos/20260704_clip.mp4".into()));
    assert!(d.can_insert());
    let effects = d.confirm();
    let [VideoEffect::Insert(MediaInsert::Block(embed))] = effects.as_slice() else {
        panic!()
    };
    assert!(
        embed.contains("src=\"videos/20260704_clip.mp4\""),
        "uploaded URL inserted as returned"
    );
    let mut s3 = VideoDialog::default();
    s3.open(
        Provider::S3,
        true,
        true,
        Vec::new(),
        jiff::civil::date(2026, 7, 4),
    );
    let effects = s3.drop_files(vec![clip("clip.mp4")]);
    let [VideoEffect::Upload { s3_prefix, .. }] = effects.as_slice() else {
        panic!()
    };
    assert_eq!(s3_prefix.as_deref(), Some("frankmd/2026/07"));
    let effects = s3.upload_result(Err(StoreError {
        message: "boom".into(),
        fallback: false,
    }));
    assert_eq!(
        effects,
        vec![VideoEffect::RefreshStorage],
        "storage refreshed once"
    );
    assert_eq!(s3.upload_feedback(), Some(&Text::Raw("boom".into())));
}

#[test]
fn tabs_wrap() {
    let mut d = dialog(true);
    d.step_tab(false);
    assert_eq!(d.tab(), VideoTab::YouTube);
    d.step_tab(true);
    assert_eq!(d.tab(), VideoTab::Upload);
    d.step_tab(true);
    assert_eq!(d.tab(), VideoTab::Url);
}
