//! Media library view model. Adapted rows of docs/testing/ui-test-plan.md
//! (library_controller.test.js, app_library.test.js).
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use frankmd_media::MediaKind;
use frankmd_media::library::{LibraryListing, MediaItem, RenamedItem};
use frankmd_media::vm::Text;
use frankmd_media::vm::library::*;
use frankmd_media::vm::markup::MediaInsert;
use frankmd_media::vm::picker::Thumb;

fn item(path: &str, size: u64, kind: MediaKind) -> MediaItem {
    MediaItem {
        name: path.rsplit('/').next().unwrap_or(path).into(),
        path: path.into(),
        size,
        mtime: "2026-07-04T10:15:00+00:00".into(),
        preview_url: String::new(),
        kind,
        s3_url: None,
    }
}

fn s3_item(key: &str) -> MediaItem {
    MediaItem {
        s3_url: Some(format!("https://b.s3.amazonaws.com/{key}")),
        ..item(key, 10, MediaKind::Image)
    }
}

fn opened(listing: LibraryListing) -> MediaLibraryView {
    let mut lib = MediaLibraryView::new(PathBuf::from("/notes"));
    let effects = lib.open();
    let [LibraryEffect::Load { generation }] = effects.as_slice() else {
        panic!()
    };
    lib.load_result(*generation, Ok(listing));
    lib
}

fn listing() -> LibraryListing {
    LibraryListing {
        images: vec![
            item("images/zebra.png", 2048, MediaKind::Image),
            item("images/photo one.png", 2048, MediaKind::Image),
        ],
        videos: vec![item("videos/clip.mp4", 4096, MediaKind::Video)],
    }
}

// library rows 1, app_library row 3: once per open
#[test]
fn listed_only_when_opened_once_per_open() {
    let mut lib = MediaLibraryView::new(PathBuf::from("/notes"));
    assert_eq!(lib.load_state(), &LoadState::Idle);
    assert_eq!(lib.open().len(), 1);
    assert!(lib.open().is_empty(), "already open: no reload");
    lib.close();
    assert_eq!(lib.open().len(), 1);
    assert_eq!(lib.open_count(), 2);
}

// library rows 2, 10
#[test]
fn cards_in_received_order_with_sizes_and_tabs() {
    let mut lib = opened(listing());
    let cards = lib.cards();
    assert_eq!(
        cards.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["zebra.png", "photo one.png"]
    );
    assert_eq!(cards[1].size, "2 KB");
    assert_eq!(
        cards[1].thumb,
        Thumb::File(PathBuf::from("/notes/images/photo one.png"))
    );
    assert!(!cards[0].can_rename);
    assert_eq!(lib.counts(), (2, 1));
    lib.set_category(Category::Videos);
    let cards = lib.cards();
    assert_eq!((cards.len(), cards[0].size.as_str()), (1, "4 KB"));
    lib.set_filter("nothing");
    assert_eq!(lib.empty_text(), Some(Text::key("media_library.empty")));
}

// library rows 3-5
#[test]
fn usage_is_lazy_shared_and_reset_on_reopen() {
    let mut lib = opened(listing());
    assert_eq!(lib.cards()[0].usage, UsageLabel::Checking);
    let effects = lib.mark_visible("images/zebra.png");
    let [LibraryEffect::LoadUsage { generation }] = effects.as_slice() else {
        panic!()
    };
    assert!(
        lib.mark_visible("images/photo one.png").is_empty(),
        "one shared scan"
    );
    let mut counts = BTreeMap::new();
    counts.insert("images/zebra.png".to_owned(), 2);
    lib.usage_result(*generation, Ok(counts));
    let cards = lib.cards();
    assert_eq!(cards[0].usage, UsageLabel::Used(2));
    assert_eq!(cards[1].usage, UsageLabel::NotUsed);
    assert_eq!(
        UsageLabel::Used(2).text(),
        Text::with("library.usage_used", &[("count", "2")])
    );
    assert_eq!(
        UsageLabel::NotUsed.text(),
        Text::key("media_library.not_used")
    );
    lib.close();
    lib.open();
    assert_eq!(lib.cards()[0].usage, UsageLabel::Checking);
    let effects = lib.mark_visible("images/zebra.png");
    let [LibraryEffect::LoadUsage { generation }] = effects.as_slice() else {
        panic!()
    };
    lib.usage_result(*generation, Err(Text::key("library.usage_scan_failed")));
    assert_eq!(lib.cards()[0].usage, UsageLabel::Unknown);
}

// library row 6 (detail "Used in": filtered, deduped, opens the note)
#[test]
fn detail_used_in_list_and_open_note() {
    let mut lib = opened(listing());
    let effects = lib.mark_visible("images/zebra.png");
    let [LibraryEffect::LoadUsage { generation }] = effects.as_slice() else {
        panic!()
    };
    lib.usage_result(
        *generation,
        Ok(BTreeMap::from([("images/zebra.png".to_owned(), 1)])),
    );
    let effects = lib.open_detail("images/zebra.png");
    assert_eq!(
        effects,
        vec![LibraryEffect::LoadUsageNotes {
            path: "images/zebra.png".into()
        }]
    );
    lib.usage_notes_result(
        "images/zebra.png",
        Ok(vec![
            "a.md".into(),
            "../outside.md".into(),
            "a.md".into(),
            "a\" b.md".into(),
        ]),
    );
    assert_eq!(
        lib.detail_notes(),
        Some(&["a.md".to_owned(), "a\" b.md".to_owned()][..])
    );
    assert_eq!(
        lib.open_note("a\" b.md"),
        vec![LibraryEffect::OpenNote("a\" b.md".into())]
    );
    assert!(!lib.is_open(), "the library closes");
}

// library row 7 (metadata with aspect ratio and format)
#[test]
fn detail_metadata() {
    let mut lib = opened(LibraryListing {
        images: vec![item("images/photo.png", 2048, MediaKind::Image)],
        videos: vec![],
    });
    lib.open_detail("images/photo.png");
    let meta = lib.detail_metadata(Some((1920, 1080))).unwrap();
    assert!(meta.starts_with("2 KB · "), "{meta}");
    assert!(meta.ends_with(" · 16:9 · PNG"), "{meta}");
}

// library row 9 (states)
#[test]
fn load_failure_state() {
    let mut lib = MediaLibraryView::new(PathBuf::from("/n"));
    let effects = lib.open();
    let [LibraryEffect::Load { generation }] = effects.as_slice() else {
        panic!()
    };
    assert_eq!(lib.load_state(), &LoadState::Loading);
    lib.load_result(*generation, Err(Text::Raw("offline".into())));
    assert_eq!(
        lib.load_state(),
        &LoadState::Failed(Text::Raw("offline".into()))
    );
}

// library row 11 (copy path)
#[test]
fn copy_path_copies_the_library_path_or_s3_url() {
    let lib = opened(LibraryListing {
        images: vec![
            item("images/photo one.png", 1, MediaKind::Image),
            s3_item("frankmd/a.png"),
        ],
        videos: vec![],
    });
    assert_eq!(
        lib.copy_path("images/photo one.png"),
        vec![LibraryEffect::WriteClipboard("images/photo one.png".into())]
    );
    assert_eq!(
        lib.copy_path("frankmd/a.png"),
        vec![LibraryEffect::WriteClipboard(
            "https://b.s3.amazonaws.com/frankmd/a.png".into()
        )]
    );
    assert_eq!(
        lib.copy_result(true),
        vec![LibraryEffect::Status(Text::key("library.copied"), false)]
    );
    assert_eq!(
        lib.copy_result(false),
        vec![LibraryEffect::Status(
            Text::key("library.copy_failed"),
            true
        )]
    );
}

// library rows 12-13
#[test]
fn delete_confirms_removes_and_reports() {
    let mut lib = opened(listing());
    let text = lib.request_delete("images/zebra.png").unwrap();
    assert_eq!(
        text,
        Text::with("library.delete_confirm", &[("name", "zebra.png")])
    );
    lib.cancel_prompt();
    assert!(lib.accept_prompt("").is_empty(), "cancel deletes nothing");
    lib.request_delete("images/zebra.png");
    assert_eq!(
        lib.accept_prompt(""),
        vec![LibraryEffect::Delete {
            path: "images/zebra.png".into()
        }]
    );
    assert_eq!(
        lib.delete_result("images/zebra.png", Ok(())),
        vec![LibraryEffect::Status(Text::key("library.deleted"), false)]
    );
    assert_eq!(lib.cards().len(), 1);
    let effects = lib.delete_result("images/photo one.png", Err(Text::Raw("in use".into())));
    assert_eq!(
        effects,
        vec![LibraryEffect::Status(Text::Raw("in use".into()), true)]
    );
    assert_eq!(lib.cards().len(), 1, "card kept on failure");
}

// library row 14 (S3: remote URL, usage unknown, rename only S3)
#[test]
fn s3_items_rename_and_usage_unknown() {
    let mut lib = opened(LibraryListing {
        images: vec![
            s3_item("frankmd/a.png"),
            item("images/b.png", 1, MediaKind::Image),
        ],
        videos: vec![],
    });
    let cards = lib.cards();
    assert_eq!(
        cards[0].thumb,
        Thumb::Url("https://b.s3.amazonaws.com/frankmd/a.png".into())
    );
    assert_eq!(cards[0].usage, UsageLabel::Unknown);
    assert!(cards[0].can_rename && !cards[1].can_rename);
    assert!(
        lib.mark_visible("frankmd/a.png").is_empty(),
        "S3 cards never trigger a scan"
    );
    assert_eq!(lib.request_rename("images/b.png"), None);
    assert_eq!(
        lib.request_rename("frankmd/a.png").as_deref(),
        Some("a.png")
    );
    assert_eq!(
        lib.accept_prompt("renamed.png"),
        vec![LibraryEffect::Rename {
            path: "frankmd/a.png".into(),
            name: "renamed.png".into()
        }]
    );
    let renamed = RenamedItem {
        name: "renamed.png".into(),
        path: "frankmd/renamed.png".into(),
        url: "https://b/renamed.png".into(),
    };
    assert_eq!(
        lib.rename_result("frankmd/a.png", Ok(renamed)),
        vec![LibraryEffect::Status(Text::key("library.renamed"), false)]
    );
    assert_eq!(lib.cards()[0].name, "renamed.png");
}

// library row 15, app_library rows 5-6
#[test]
fn insert_closes_on_success_and_explains_refusals() {
    let mut lib = opened(LibraryListing {
        images: vec![item("images/photo (one).png", 1, MediaKind::Image)],
        videos: vec![],
    });
    let refused = lib.insert("images/photo (one).png", None);
    assert_eq!(
        refused,
        vec![LibraryEffect::Status(
            Text::key("library.no_open_note"),
            true
        )]
    );
    assert!(lib.is_open());
    let refused = lib.insert("images/photo (one).png", Some("x.txt"));
    assert_eq!(
        refused,
        vec![LibraryEffect::Status(
            Text::key("library.markdown_note_required"),
            true
        )]
    );
    let effects = lib.insert("images/photo (one).png", Some("folder/sub/note.md"));
    assert_eq!(
        effects,
        vec![LibraryEffect::Insert(MediaInsert::Image(
            "![photo (one).png](../../images/photo%20%28one%29.png)".into()
        ))]
    );
    assert!(!lib.is_open());
}

// library rows 16-18: odd names act literally
#[test]
fn quoted_names_are_literal() {
    let name = "photo\" onerror=\"alert(1).png";
    let lib = opened(LibraryListing {
        images: vec![item(&format!("images/{name}"), 1, MediaKind::Image)],
        videos: vec![],
    });
    let card = &lib.cards()[0];
    assert_eq!(card.name, name);
    assert_eq!(card.path, format!("images/{name}"));
}

#[test]
fn date_format() {
    let d = format_date("not a date");
    assert_eq!(d, "not a date");
    assert_eq!(
        format_date("2026-07-04T10:15:00+00:00").len(),
        "2026-07-04 10:15".len()
    );
}
