//! Port of `test/javascript/image_sources/drop_images.test.js` (6 tests): the
//! Drop tab reuses `FolderImageSource::ingest`.

#[path = "common/client_picker.rs"]
mod client_picker;

use client_picker::file;
use frankmd_media::picker::{DEFAULT_IMAGE_EXTENSIONS, FolderImageSource, ext_of};

fn defaults() -> Vec<String> {
    DEFAULT_IMAGE_EXTENSIONS
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

fn f(name: &str) -> frankmd_media::picker::PickedFile {
    let mut f = file(name, 1, 10);
    f.dimensions = Some((100, 100));
    f
}

// port: test/javascript/image_sources/drop_images.test.js:35 "accepts files whose extension is in the allowed list"
#[test]
fn accepts_allowed_extensions() {
    let r = FolderImageSource::default()
        .ingest(None, vec![f("a.png"), f("b.jpg")], Some(&defaults()))
        .expect("ingest");
    assert_eq!(r["count"], 2);
    assert_eq!(r["rejected"], serde_json::json!([]));
}

// port: test/javascript/image_sources/drop_images.test.js:42 "rejects disallowed files and reports the extension as the reason"
#[test]
fn rejects_disallowed_with_extension_reason() {
    let r = FolderImageSource::default()
        .ingest(None, vec![f("a.png"), f("notes.txt")], Some(&defaults()))
        .expect("ingest");
    assert_eq!(r["count"], 1);
    assert_eq!(r["rejected"].as_array().map(Vec::len), Some(1));
    assert_eq!(r["rejected"][0]["name"], "notes.txt");
    assert_eq!(r["rejected"][0]["ext"], ".txt");
}

// port: test/javascript/image_sources/drop_images.test.js:51 "labels files with no extension"
#[test]
fn labels_files_without_extension() {
    let r = FolderImageSource::default()
        .ingest(None, vec![f("README")], Some(&defaults()))
        .expect("ingest");
    assert_eq!(r["count"], 0);
    assert_eq!(r["rejected"][0]["ext"], "no extension");
}

// port: test/javascript/image_sources/drop_images.test.js:58 "respects a custom (config-overridden) extension list"
#[test]
fn respects_custom_extension_list() {
    let r = FolderImageSource::default()
        .ingest(
            None,
            vec![f("a.png"), f("b.webp")],
            Some(&[".webp".to_owned()]),
        )
        .expect("ingest");
    assert_eq!(r["count"], 1);
    assert_eq!(r["rejected"].as_array().map(Vec::len), Some(1));
    assert_eq!(r["rejected"][0]["name"], "a.png");
}

// port: test/javascript/image_sources/drop_images.test.js:69 "returns the trailing extension"
#[test]
fn ext_of_returns_trailing_extension() {
    assert_eq!(ext_of(None, "video.final.MP4"), ".mp4");
}

// port: test/javascript/image_sources/drop_images.test.js:73 "returns a readable label when there is no extension"
#[test]
fn ext_of_labels_missing_extension() {
    assert_eq!(ext_of(None, "README"), "no extension");
}
