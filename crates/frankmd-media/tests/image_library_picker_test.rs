//! Port of `test/controllers/image_library_picker_test.rb` (8 tests).
//! `GET /library/picker` rendered a turbo-stream partial; the Rust contract is
//! the [`PickerPage`] data (the view escapes `name` and uses `preview_url`/`url`
//! verbatim). Setup: `images_path` = `<notes>/server`.
#![allow(clippy::disallowed_methods)]

#[path = "common/library.rs"]
mod library;

use frankmd_media::config::MediaConfig;
use frankmd_media::library::{LibraryError, PickerCollection, PickerPage, PickerQuery, picker_list};
use frankmd_media::param::Param;
use library::{Notes, chmod, is_root};

fn setup() -> (Notes, MediaConfig) {
    let n = Notes::new();
    std::fs::create_dir_all(n.root().join("server")).expect("server dir");
    let mut cfg = MediaConfig::new(n.root());
    cfg.images_path = Some(n.root().join("server"));
    (n, cfg)
}

fn picker(n: &Notes, cfg: &MediaConfig, q: PickerQuery) -> Result<PickerPage, LibraryError> {
    picker_list(&n.vault(), cfg, &q)
}

fn all_text(page: &PickerPage) -> String {
    page.images.iter().map(|i| format!("{} {} {} {}\n", i.name, i.path, i.url, i.preview_url)).collect()
}

// port: test/controllers/image_library_picker_test.rb:22 "picker defaults to managed images and renders escaped markup with local references"
#[test]
fn defaults_to_managed_images_with_local_references() {
    let (n, cfg) = setup();
    n.write("images/a <photo>.png", "image");
    n.write("videos/clip.mp4", "video");
    n.write("server/server.png", "server");
    let page = picker(&n, &cfg, PickerQuery::default()).expect("page");
    assert_eq!(page.collection, PickerCollection::Managed);
    let item = &page.images[0];
    assert_eq!(item.path, "images/a <photo>.png"); // the view HTML-escapes it to `images/a &lt;photo&gt;.png`
    assert_eq!(item.preview_url, "/notes/images/a%20%3Cphoto%3E.png");
    let text = all_text(&page);
    assert!(!text.contains("server.png"));
    assert!(!text.contains("clip.mp4"));
}

// port: test/controllers/image_library_picker_test.rb:36 "server collection uses existing preview URLs without exposing filesystem paths"
#[test]
fn server_collection_uses_preview_urls_without_filesystem_paths() {
    let (n, cfg) = setup();
    n.write("server/server photo.png", "server");
    let page = picker(&n, &cfg, PickerQuery { collection: Param::str("server"), ..Default::default() }).expect("page");
    assert_eq!(page.collection, PickerCollection::Server); // data-collection="server"
    assert_eq!(page.images[0].url, "/images/preview/server%20photo.png");
    assert_eq!(page.images[0].preview_url, "/images/preview/server%20photo.png");
    assert!(!all_text(&page).contains(&n.root().join("server").display().to_string()));
}

// port: test/controllers/image_library_picker_test.rb:45 "managed images support search and bounded pagination"
#[test]
fn managed_images_support_search_and_pagination() {
    let (n, cfg) = setup();
    for i in 0..3 {
        n.write(&format!("images/photo{i}.png"), "image");
    }
    n.write("images/other.png", "image");
    let q = |offset: i64| PickerQuery {
        search: Param::str("PHOTO"),
        limit: Param::Int(2),
        offset: Param::Int(offset),
        ..Default::default()
    };
    let first = picker(&n, &cfg, q(0)).expect("page");
    assert_eq!(first.total, 3); // data-total="3"
    assert_eq!(first.images.len(), 2);
    assert_eq!(picker(&n, &cfg, q(2)).expect("page").images.len(), 1);
}

// port: test/controllers/image_library_picker_test.rb:56 "managed images remain browsable with S3 selected and its credentials missing"
#[test]
fn managed_images_browsable_with_s3_selected_but_unconfigured() {
    let (n, mut cfg) = setup();
    n.write("images/photo.png", "image");
    cfg.media_storage_provider = Some("s3".into());
    let page = picker(&n, &cfg, PickerQuery::default()).expect("page");
    assert_eq!(page.images[0].url, "images/photo.png"); // data-url="images/photo.png"
}

// port: test/controllers/image_library_picker_test.rb:64 "unconfigured or missing server folders produce a useful empty state"
#[test]
fn missing_or_unconfigured_server_folder_is_an_empty_state() {
    let (n, mut cfg) = setup();
    cfg.images_path = Some(n.root().join("missing"));
    let page = picker(&n, &cfg, PickerQuery { collection: Param::str("server"), ..Default::default() }).expect("200");
    // the view shows `dialogs.image_picker.local_path_required` for this state
    assert!(!page.server_available);
    assert!(page.images.is_empty());
    cfg.images_path = None; // `images_path = ""`
    let page = picker(&n, &cfg, PickerQuery { collection: Param::str("server"), ..Default::default() }).expect("200");
    assert!(!page.server_available);
}

// port: test/controllers/image_library_picker_test.rb:74 "managed collection omits symlinks and unsupported files"
#[test]
fn managed_collection_omits_symlinks_and_unsupported_files() {
    let (n, cfg) = setup();
    n.write("images/photo.png", "image");
    n.write("images/unsafe.svg", "svg");
    n.symlink("images/link.png", &n.root().join("images/photo.png"));
    let text = all_text(&picker(&n, &cfg, PickerQuery::default()).expect("page"));
    assert!(!text.contains("unsafe.svg"));
    assert!(!text.contains("link.png"));
}

// port: test/controllers/image_library_picker_test.rb:84 "unknown collections are rejected without accessing S3"
#[test]
fn unknown_collections_are_rejected() {
    let (n, cfg) = setup();
    let err = picker(&n, &cfg, PickerQuery { collection: Param::str("s3"), ..Default::default() }).expect_err("400");
    assert!(matches!(err, LibraryError::BadRequest(_)), "{err:?}");
}

// port: test/controllers/image_library_picker_test.rb:89 "filesystem access failures are reported without exposing paths"
#[test]
fn filesystem_failures_are_reported_without_paths() {
    if is_root() {
        eprintln!("skipped: running as root");
        return;
    }
    let (n, cfg) = setup();
    n.write("images/photo.png", "image");
    let _g = chmod(&n.root().join("images"), 0o000); // `stubs(:list).raises(Errno::EACCES)`
    let err = picker(&n, &cfg, PickerQuery::default()).expect_err("422");
    assert!(matches!(err, LibraryError::LoadFailed), "{err:?}");
    assert!(!err.to_string().contains(&n.root().display().to_string()));
}
