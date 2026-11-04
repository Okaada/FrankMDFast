//! Port of `test/controllers/library_controller_test.rb` (13 tests) against
//! [`frankmd_media::library::Library`] (the provider-routed controller logic).
//! Config stub: images `.png .jpg`, videos `.mp4 .webm`, everything else nil.
#![allow(clippy::disallowed_methods)]

#[path = "common/library.rs"]
mod library;

use std::collections::BTreeMap;

use frankmd_media::MediaKind;
use frankmd_media::library::{Library, LibraryError, UsageResponse};
use frankmd_media::usage::MediaUsage;
use library::{Notes, chmod, config, env, is_root, list_objects_xml, s3_config};
use wiremock::matchers::{header, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn counts(r: UsageResponse) -> BTreeMap<String, usize> {
    match r {
        UsageResponse::Counts(c) => c,
        UsageResponse::Notes(n) => panic!("expected usage_counts only, got usage_notes {n:?}"),
    }
}

// port: test/controllers/library_controller_test.rb:19 "index returns supported image and video groups with metadata"
#[tokio::test]
async fn index_returns_supported_groups_with_metadata() {
    let n = Notes::new();
    n.write("images/Photo.png", "image");
    n.write("videos/clip.mp4", "video");
    n.write("images/ignored.svg", "not allowed");
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let data = Library::new(&vault, &e).list().await.expect("list");
    assert_eq!(data.images.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), vec!["Photo.png"]);
    assert_eq!(data.videos.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), vec!["clip.mp4"]);
    assert_eq!(data.images[0].path, "images/Photo.png");
    assert_eq!(data.images[0].preview_url, "/notes/images/Photo.png");
    assert_eq!(data.images[0].kind, MediaKind::Image);
}

// port: test/controllers/library_controller_test.rb:35 "index excludes symlinked media without exposing their targets"
#[tokio::test]
async fn index_excludes_symlinked_media() {
    let n = Notes::new();
    let outside = n.write_outside("library_outside.png", "outside");
    n.symlink("images/linked.png", &outside);
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let data = Library::new(&vault, &e).list().await.expect("list");
    assert!(data.images.is_empty());
    assert!(outside.exists());
}

// port: test/controllers/library_controller_test.rb:50 "usage returns distinct counts without sending the reverse index"
#[test]
fn usage_returns_distinct_counts_only() {
    let n = Notes::new();
    n.write("first.md", "![first](images/photo.png)\n[again](images/photo.png)\n[clip](videos/clip.mp4)");
    n.write("nested/second.md", "![photo](../images/photo.png)");
    n.write("images/unused.png", "unused");
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let c = counts(Library::new(&vault, &e).usage(&MediaUsage::new(n.root()), None).expect("usage"));
    assert_eq!(c.get("images/photo.png"), Some(&2));
    assert_eq!(c.get("videos/clip.mp4"), Some(&1));
    assert!(!c.contains_key("images/unused.png"));
}

// port: test/controllers/library_controller_test.rb:69 "usage returns note paths only for the requested media item"
#[test]
fn usage_returns_note_paths_for_requested_item() {
    let n = Notes::new();
    n.write("first.md", "![photo](images/photo.png)\n[clip](videos/clip.mp4)");
    n.write("second.md", "![photo](images/photo.png)");
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let r = Library::new(&vault, &e).usage(&MediaUsage::new(n.root()), Some("images/photo.png")).expect("usage");
    assert_eq!(r, UsageResponse::Notes(vec!["first.md".into(), "second.md".into()]));
}

// port: test/controllers/library_controller_test.rb:81 "usage counts repeated references from one note once"
#[test]
fn usage_counts_repeated_references_once() {
    let n = Notes::new();
    n.write("repeated.md", "![first](images/photo.png)\n![second](images/photo.png)");
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let c = counts(Library::new(&vault, &e).usage(&MediaUsage::new(n.root()), None).expect("usage"));
    assert_eq!(c, BTreeMap::from([("images/photo.png".to_owned(), 1)]));
}

// port: test/controllers/library_controller_test.rb:92 "usage builds the index once per request"
#[test]
fn usage_builds_the_index_once_per_request() {
    let n = Notes::new();
    n.write("note.md", "![photo](images/photo.png)");
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let usage = MediaUsage::new(n.root());
    let c = counts(Library::new(&vault, &e).usage(&usage, None).expect("usage"));
    assert_eq!(c, BTreeMap::from([("images/photo.png".to_owned(), 1)]));
    assert_eq!(usage.build_count(), 1);
}

// port: test/controllers/library_controller_test.rb:103 "usage returns no partial map and a localized error when the note scan fails"
#[test]
fn usage_scan_failure_is_localized_error_without_partial_map() {
    if is_root() {
        eprintln!("skipped: running as root");
        return;
    }
    let n = Notes::new();
    n.write("ok.md", "![photo](images/photo.png)");
    let bad = n.write("bad.md", "![x](images/x.png)");
    let _g = chmod(&bad, 0o000); // ScanError "read failed"
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let err = Library::new(&vault, &e).usage(&MediaUsage::new(n.root()), None).expect_err("scan failed");
    assert!(matches!(err, LibraryError::UsageScanFailed), "{err:?}");
    assert_eq!(err.i18n_key(), Some("library.usage_scan_failed"));
}

// port: test/controllers/library_controller_test.rb:115 "usage returns a localized error when a notes directory cannot be scanned"
#[test]
fn usage_directory_eacces_is_localized_error() {
    if is_root() {
        eprintln!("skipped: running as root");
        return;
    }
    let n = Notes::new();
    n.write("locked/note.md", "![photo](images/photo.png)");
    let _g = chmod(&n.root().join("locked"), 0o000); // Errno::EACCES while walking
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let err = Library::new(&vault, &e).usage(&MediaUsage::new(n.root()), None).expect_err("eacces");
    assert_eq!(err.i18n_key(), Some("library.usage_scan_failed"));
}

// port: test/controllers/library_controller_test.rb:124 "destroy removes a supported media file"
#[tokio::test]
async fn destroy_removes_a_supported_media_file() {
    let n = Notes::new();
    let p = n.write("images/photo.png", "image");
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    Library::new(&vault, &e).delete("images/photo.png").await.expect("204");
    assert!(!p.exists());
}

// port: test/controllers/library_controller_test.rb:133 "destroy rejects traversal, unrelated roots, and unsupported files"
#[tokio::test]
async fn destroy_rejects_traversal_unrelated_roots_and_unsupported() {
    let n = Notes::new();
    let outside = n.root().parent().expect("parent").join(format!(
        "library_controller_outside_{}.png",
        n.root().file_name().expect("name").to_string_lossy()
    ));
    std::fs::write(&outside, "outside").expect("write");
    let unrelated = n.write("attachments/photo.png", "attachment");
    let unsupported = n.write("images/readme.md", "note");
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let lib = Library::new(&vault, &e);
    let base = outside.file_name().expect("name").to_string_lossy().into_owned();
    assert!(matches!(lib.delete(&format!("images/../{base}")).await, Err(LibraryError::NotFound)));
    assert!(outside.exists());
    assert!(matches!(lib.delete("attachments/photo.png").await, Err(LibraryError::NotFound)));
    assert!(unrelated.exists());
    assert!(matches!(lib.delete("images/readme.md").await, Err(LibraryError::NotFound)));
    assert!(unsupported.exists());
    let _ = std::fs::remove_file(outside);
}

// port: test/controllers/library_controller_test.rb:154 "destroy returns a clear error when the filesystem rejects deletion"
#[tokio::test]
async fn destroy_reports_filesystem_rejection() {
    if is_root() {
        eprintln!("skipped: running as root");
        return;
    }
    let n = Notes::new();
    n.write("images/protected.png", "image");
    let _g = chmod(&n.root().join("images"), 0o555);
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let err = Library::new(&vault, &e).delete("images/protected.png").await.expect_err("422");
    assert!(matches!(err, LibraryError::DeleteFailed), "{err:?}");
    assert_eq!(err.i18n_key(), Some("library.delete_failed"));
}

// port: test/controllers/library_controller_test.rb:164 "S3 provider routes listing, deletion, and rename through the scoped S3 library"
#[tokio::test]
async fn s3_provider_routes_list_delete_and_rename_through_s3_library() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex("^/bucket/?$")) // mechanical: aws-sdk-s3 (Rust) sends `/bucket/` for path-style ListObjectsV2
        .and(query_param("prefix", "frankmd/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_objects_xml("bucket", &[], None)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("HEAD")).and(path("/bucket/frankmd/2026/renamed.png")).respond_with(ResponseTemplate::new(404)).mount(&server).await;
    Mock::given(method("PUT"))
        .and(path("/bucket/frankmd/2026/renamed.png"))
        .and(header("x-amz-copy-source", "bucket/frankmd/2026/photo.png"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<CopyObjectResult/>"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/bucket/frankmd/2026/photo.png"))
        .respond_with(ResponseTemplate::new(204))
        .expect(2) // destroy + rename's source delete
        .mount(&server)
        .await;
    let n = Notes::new();
    n.write("images/local.png", "local media must not be listed under s3");
    let (vault, e) = (n.vault(), env(s3_config(n.root()), Some(&server.uri())));
    let lib = Library::new(&vault, &e);
    let listing = lib.list().await.expect("list");
    assert!(listing.images.is_empty() && listing.videos.is_empty());
    lib.delete("frankmd/2026/photo.png").await.expect("204");
    let renamed = lib.rename("frankmd/2026/photo.png", "renamed.png").await.expect("200");
    assert_eq!(renamed.path, "frankmd/2026/renamed.png");
}

// port: test/controllers/library_controller_test.rb:183 "local provider rejects rename without changing the file"
#[tokio::test]
async fn local_provider_rejects_rename() {
    let n = Notes::new();
    let p = n.write("images/photo.png", "image");
    let (vault, e) = (n.vault(), env(config(n.root()), None));
    let err = Library::new(&vault, &e).rename("images/photo.png", "renamed.png").await.expect_err("422");
    assert!(matches!(err, LibraryError::RenameFailed), "{err:?}");
    assert!(p.exists());
    assert!(!n.root().join("images/renamed.png").exists());
}
