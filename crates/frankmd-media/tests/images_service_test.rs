//! Port of test/services/images_service_test.rb, class `ImagesServiceTest`
//! (local provider). The S3 class lives in `images_service_s3_test.rs`.
//!
//! setup: `images_path` = a temp "server" folder, provider local, image
//! extensions `.jpg .jpeg .png .gif .webp .bmp`, videos `.mp4 .webm`. The
//! original wrote base64 uploads into the ambient NOTES_PATH; here every test
//! gets its own temp notes root.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use std::path::{Path, PathBuf};

use frankmd_media::config::MediaConfig;
use frankmd_media::images::{self, Dimensions};
use frankmd_media::param::Param;
use frankmd_testkit::TempTree;
use uploads::*;

struct Fixture {
    notes: TempTree,
    server: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let notes = TempTree::empty();
        let server = notes.path().join("server_images");
        std::fs::create_dir_all(&server).unwrap();
        Self { notes, server }
    }

    fn config(&self) -> MediaConfig {
        let mut c = local_config(self.notes.path());
        c.images_path = Some(self.server.clone());
        c
    }

    /// `create_test_image(name, content)`; `age` seconds in the past orders by mtime.
    fn image(&self, name: &str, content: &[u8], age: u64) -> PathBuf {
        let p = write(&self.server, name, content);
        set_mtime(&p, age);
        p
    }

    fn env(&self) -> frankmd_media::MediaEnv {
        env_for(self.config(), None).1
    }
}

fn nil() -> Param {
    Param::Nil
}

fn names(list: &[images::ServerImage]) -> Vec<String> {
    list.iter().map(|i| i.name.clone()).collect()
}

fn list_default(c: &MediaConfig) -> Vec<images::ServerImage> {
    images::list(c, None, &Param::Int(images::MAX_RESULTS as i64), &Param::Int(0))
}

// port: test/services/images_service_test.rb:36 "enabled? returns true when path is set"
#[test]
fn enabled_returns_true_when_path_is_set() {
    assert!(images::enabled(&Fixture::new().config()));
}

// port: test/services/images_service_test.rb:40 "enabled? returns false when path is not set"
#[test]
fn enabled_returns_false_when_path_is_not_set() {
    let mut c = Fixture::new().config();
    c.images_path = None;
    assert!(!images::enabled(&c));
}

// port: test/services/images_service_test.rb:47 "list returns empty array when disabled"
#[test]
fn list_returns_empty_array_when_disabled() {
    let mut c = Fixture::new().config();
    c.images_path = None;
    assert!(list_default(&c).is_empty());
}

// port: test/services/images_service_test.rb:52 "list returns empty array when directory doesn't exist"
#[test]
fn list_returns_empty_array_when_directory_doesnt_exist() {
    let f = Fixture::new();
    std::fs::remove_dir_all(&f.server).unwrap();
    assert!(list_default(&f.config()).is_empty());
}

// port: test/services/images_service_test.rb:57 "list returns images sorted by most recent"
#[test]
fn list_returns_images_sorted_by_most_recent() {
    let f = Fixture::new();
    f.image("old.jpg", b"fake image data", 20);
    f.image("new.jpg", b"fake image data", 10);
    let list = list_default(&f.config());
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name, "new.jpg");
    assert_eq!(list[1].name, "old.jpg");
}

// port: test/services/images_service_test.rb:69 "list only returns supported image extensions"
#[test]
fn list_only_returns_supported_image_extensions() {
    let f = Fixture::new();
    for n in ["image.jpg", "image.png", "image.gif", "document.txt", "script.js"] {
        f.image(n, b"fake image data", 0);
    }
    let n = names(&list_default(&f.config()));
    assert!(n.contains(&"image.jpg".into()));
    assert!(n.contains(&"image.png".into()));
    assert!(n.contains(&"image.gif".into()));
    assert!(!n.contains(&"document.txt".into()));
    assert!(!n.contains(&"script.js".into()));
}

// port: test/services/images_service_test.rb:86 "list limits to 10 results"
#[test]
fn list_limits_to_10_results() {
    let f = Fixture::new();
    for i in 0..15 {
        f.image(&format!("image_{i}.jpg"), b"x", 0);
    }
    assert_eq!(list_default(&f.config()).len(), 10);
}

// port: test/services/images_service_test.rb:93 "list honors offset and limit for paging"
#[test]
fn list_honors_offset_and_limit_for_paging() {
    let f = Fixture::new();
    for i in 0..15 {
        f.image(&format!("image_{i}.jpg"), b"x", i);
    }
    let c = f.config();
    let first = images::list(&c, None, &Param::Int(10), &Param::Int(0));
    let second = images::list(&c, None, &Param::Int(10), &Param::Int(10));
    assert_eq!(first.len(), 10);
    assert_eq!(second.len(), 5);
    assert_ne!(names(&first), names(&second));
}

// port: test/services/images_service_test.rb:104 "list clamps an oversized limit to 100"
#[test]
fn list_clamps_an_oversized_limit_to_100() {
    let f = Fixture::new();
    for i in 0..5 {
        f.image(&format!("image_{i}.jpg"), b"x", 0);
    }
    assert_eq!(images::list(&f.config(), None, &Param::Int(1000), &Param::Int(0)).len(), 5);
    assert_eq!(images::clamp_limit(&Param::Int(1000)), 100);
}

// port: test/services/images_service_test.rb:111 "list clamps a negative offset to 0"
#[test]
fn list_clamps_a_negative_offset_to_0() {
    let f = Fixture::new();
    f.image("old.jpg", b"x", 20);
    f.image("new.jpg", b"x", 10);
    let list = images::list(&f.config(), None, &Param::Int(10), &Param::Int(-5));
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name, "new.jpg");
}

// port: test/services/images_service_test.rb:121 "list coerces garbage limit/offset instead of raising"
#[test]
fn list_coerces_garbage_limit_and_offset_instead_of_raising() {
    let f = Fixture::new();
    f.image("image.jpg", b"x", 0);
    let list = images::list(&f.config(), None, &Param::str("not a number"), &Param::str("also not a number"));
    assert_eq!(list.len(), 1);
}

// port: test/services/images_service_test.rb:130 "count returns 0 when disabled"
#[test]
fn count_returns_0_when_disabled() {
    let mut c = Fixture::new().config();
    c.images_path = None;
    assert_eq!(images::count(&c, None), 0);
}

// port: test/services/images_service_test.rb:135 "count returns 0 when directory doesn't exist"
#[test]
fn count_returns_0_when_directory_doesnt_exist() {
    let f = Fixture::new();
    std::fs::remove_dir_all(&f.server).unwrap();
    assert_eq!(images::count(&f.config(), None), 0);
}

// port: test/services/images_service_test.rb:140 "count returns the full total regardless of limit"
#[test]
fn count_returns_the_full_total_regardless_of_limit() {
    let f = Fixture::new();
    for i in 0..15 {
        f.image(&format!("image_{i}.jpg"), b"x", 0);
    }
    assert_eq!(images::count(&f.config(), None), 15);
    assert_eq!(list_default(&f.config()).len(), 10);
}

// port: test/services/images_service_test.rb:147 "count respects search"
#[test]
fn count_respects_search() {
    let f = Fixture::new();
    for n in ["cat.jpg", "dog.png", "category.gif"] {
        f.image(n, b"x", 0);
    }
    assert_eq!(images::count(&f.config(), Some("cat")), 2);
}

// port: test/services/images_service_test.rb:155 "list filters by search term"
#[test]
fn list_filters_by_search_term() {
    let f = Fixture::new();
    for n in ["cat.jpg", "dog.png", "category.gif"] {
        f.image(n, b"x", 0);
    }
    let n = names(&images::list(&f.config(), Some("cat"), &Param::Int(10), &Param::Int(0)));
    assert!(n.contains(&"cat.jpg".into()));
    assert!(n.contains(&"category.gif".into()));
    assert!(!n.contains(&"dog.png".into()));
}

// port: test/services/images_service_test.rb:168 "list search is case insensitive"
#[test]
fn list_search_is_case_insensitive() {
    let f = Fixture::new();
    f.image("MyPhoto.JPG", b"x", 0);
    let list = images::list(&f.config(), Some("myphoto"), &Param::Int(10), &Param::Int(0));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "MyPhoto.JPG");
}

// port: test/services/images_service_test.rb:176 "list includes images in subdirectories"
#[test]
fn list_includes_images_in_subdirectories() {
    let f = Fixture::new();
    f.image("root.jpg", b"x", 0);
    f.image("subfolder/nested.png", b"x", 0);
    let paths: Vec<String> = list_default(&f.config()).into_iter().map(|i| i.path).collect();
    assert!(paths.contains(&"root.jpg".into()));
    assert!(paths.contains(&"subfolder/nested.png".into()));
}

// port: test/services/images_service_test.rb:189 "delete removes an existing image and returns true"
#[test]
fn delete_removes_an_existing_image_and_returns_true() {
    let f = Fixture::new();
    let p = f.image("gone.jpg", b"x", 0);
    assert!(p.exists());
    assert!(images::delete(&f.config(), "gone.jpg"));
    assert!(!p.exists());
}

// security: deleting from the user's picture folder must not leave app files
// (lock files) behind — only the deleted image disappears.
#[test]
fn delete_leaves_no_app_files_in_images_path() {
    let f = Fixture::new();
    f.image("gone.jpg", b"x", 0);
    f.image("keep.jpg", b"x", 0);
    let root = f.config().images_path.unwrap();
    assert!(images::delete(&f.config(), "gone.jpg"));
    let mut left: Vec<String> =
        std::fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    left.sort();
    assert_eq!(left, vec!["keep.jpg".to_owned()]);
}

// port: test/services/images_service_test.rb:197 "delete works with subdirectories"
#[test]
fn delete_works_with_subdirectories() {
    let f = Fixture::new();
    let p = f.image("photos/old.jpg", b"x", 0);
    assert!(images::delete(&f.config(), "photos/old.jpg"));
    assert!(!p.exists());
}

// port: test/services/images_service_test.rb:204 "delete returns false for a non-existent image"
#[test]
fn delete_returns_false_for_a_non_existent_image() {
    assert!(!images::delete(&Fixture::new().config(), "nope.jpg"));
}

// port: test/services/images_service_test.rb:208 "delete refuses path traversal and leaves the target untouched"
#[test]
fn delete_refuses_path_traversal_and_leaves_the_target_untouched() {
    let f = Fixture::new();
    let outside = write(f.notes.path(), "delete_traversal.txt", b"keep me");
    assert!(!images::delete(&f.config(), "../delete_traversal.txt"));
    assert!(outside.exists(), "traversal must not delete files outside the images dir");
}

// port: test/services/images_service_test.rb:220 "delete refuses non-image files inside the images dir"
#[test]
fn delete_refuses_non_image_files_inside_the_images_dir() {
    let f = Fixture::new();
    let p = f.image("secret.txt", b"x", 0);
    assert!(!images::delete(&f.config(), "secret.txt"));
    assert!(p.exists());
}

// port: test/services/images_service_test.rb:229 "find_image returns full path for existing image"
#[test]
fn find_image_returns_full_path_for_existing_image() {
    let f = Fixture::new();
    f.image("test.jpg", b"x", 0);
    assert_eq!(
        images::find_image(&f.config(), "test.jpg").map(|p| p.canonicalize().unwrap()),
        Some(f.server.join("test.jpg").canonicalize().unwrap())
    );
}

// port: test/services/images_service_test.rb:236 "find_image returns nil for non-existent image"
#[test]
fn find_image_returns_nil_for_non_existent_image() {
    assert_eq!(images::find_image(&Fixture::new().config(), "nonexistent.jpg"), None);
}

// port: test/services/images_service_test.rb:241 "find_image prevents path traversal"
#[test]
fn find_image_prevents_path_traversal() {
    assert_eq!(images::find_image(&Fixture::new().config(), "../../../etc/passwd"), None);
}

// port: test/services/images_service_test.rb:246 "find_image works with subdirectories"
#[test]
fn find_image_works_with_subdirectories() {
    let f = Fixture::new();
    f.image("photos/vacation.jpg", b"x", 0);
    assert_eq!(
        images::find_image(&f.config(), "photos/vacation.jpg").map(|p| p.canonicalize().unwrap()),
        Some(f.server.join("photos/vacation.jpg").canonicalize().unwrap())
    );
}

// port: test/services/images_service_test.rb:253 "find_image rejects non-media files"
#[test]
fn find_image_rejects_non_media_files() {
    let f = Fixture::new();
    f.image("secret.txt", b"x", 0);
    f.image(".fed", b"x", 0);
    assert_eq!(images::find_image(&f.config(), "secret.txt"), None);
    assert_eq!(images::find_image(&f.config(), ".fed"), None);
}

fn images_dir_files(notes: &Path) -> Vec<String> {
    std::fs::read_dir(notes.join("images"))
        .map(|rd| rd.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default()
}

// port: test/services/images_service_test.rb:263 "upload_base64_data saves base64 image to notes directory"
#[tokio::test]
async fn upload_base64_data_saves_base64_image_to_notes_directory() {
    let f = Fixture::new();
    let env = f.env();
    let url = images::upload_base64_data(&env, &b64(&PNG_1X1), Some("image/png"), Some("test_ai.png"), &nil(), None)
        .await
        .unwrap();
    assert!(url.starts_with("images/"));
    assert!(url.contains("test_ai"));
    assert_eq!(std::fs::read(f.notes.path().join(&url)).unwrap(), PNG_1X1);
}

// port: test/services/images_service_test.rb:286 "upload_base64_data returns error for invalid base64"
#[tokio::test]
async fn upload_base64_data_returns_error_for_invalid_base64() {
    let f = Fixture::new();
    let err = images::upload_base64_data(&f.env(), "not valid base64!!!", Some("image/png"), None, &nil(), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Invalid base64"), "{err}");
}

// port: test/services/images_service_test.rb:292 "upload_base64_data rejects oversized encoded input before decoding"
#[tokio::test]
async fn upload_base64_data_rejects_oversized_encoded_input_before_decoding() {
    let f = Fixture::new();
    let mut env = f.env();
    env.limits.max_base64_encoded_bytes = 10;
    // 11 x "a" is not valid strict base64 either: getting "too large" (not
    // "Invalid base64") proves the cap is checked before decoding.
    let err = images::upload_base64_data(&env, &"a".repeat(11), None, Some("large.png"), &nil(), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("too large"), "{err}");
}

// port: test/services/images_service_test.rb:301 "upload_base64_data rejects an SVG mime type (stored-XSS vector)"
#[tokio::test]
async fn upload_base64_data_rejects_an_svg_mime_type() {
    let f = Fixture::new();
    let svg = b64(b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>");
    let err = images::upload_base64_data(&f.env(), &svg, Some("image/svg+xml"), Some("xss_1234.svg"), &nil(), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not an accepted"), "{err}");
    assert!(images_dir_files(f.notes.path()).iter().all(|n| !n.ends_with("xss_1234.svg")));
}

// port: test/services/images_service_test.rb:316 "upload_base64_data rejects an html filename (stored-XSS vector)"
#[tokio::test]
async fn upload_base64_data_rejects_an_html_filename() {
    let f = Fixture::new();
    let data = b64(b"<html><script>alert(1)</script></html>");
    let err = images::upload_base64_data(&f.env(), &data, Some("image/png"), Some("evil_1234.html"), &nil(), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not an accepted"), "{err}");
    assert!(images_dir_files(f.notes.path()).iter().all(|n| !n.ends_with(".html")));
}

// port: test/services/images_service_test.rb:329 "upload_base64_data generates filename when not provided"
#[tokio::test]
async fn upload_base64_data_generates_filename_when_not_provided() {
    let f = Fixture::new();
    let url = images::upload_base64_data(&f.env(), &b64(&PNG_1X1), Some("image/png"), None, &nil(), None)
        .await
        .unwrap();
    assert!(url.contains("ai_generated_"));
}

// port: test/services/images_service_test.rb:350 "local image saves use unique filenames for same-second uploads"
#[test]
fn local_image_saves_use_unique_filenames_for_same_second_uploads() {
    let f = Fixture::new();
    let mut env = f.env();
    env.ctx.random = std::sync::Arc::new(frankmd_media::ctx::OsRandom); // fixed clock, real randomness
    // security: real PNG bytes (placeholder text is rejected by content sniffing).
    let first = images::save_to_notes_directory(&env, &PNG_1X1, "photo.png", None).unwrap();
    let second = images::save_to_notes_directory(&env, &PNG_1X1, "photo.png", None).unwrap();
    assert_ne!(first, second);
}

// port: test/services/images_service_test.rb:368 "get_image_dimensions returns hash with width and height keys"
#[test]
fn get_image_dimensions_returns_width_and_height() {
    let f = Fixture::new();
    let p = f.image("real.png", &PNG_1X1, 0);
    // The original only asserted the keys exist (ImageMagick may be missing);
    // the `image` crate always decodes this PNG, so assert the values.
    assert_eq!(images::image_dimensions(&p), Dimensions { width: Some(1), height: Some(1) });
}

// port: test/services/images_service_test.rb:388 "get_image_dimensions returns nil for invalid file"
#[test]
fn get_image_dimensions_returns_nil_for_invalid_file() {
    let f = Fixture::new();
    let p = f.image("not_image.txt", b"just text", 0);
    assert_eq!(images::image_dimensions(&p), Dimensions::default());
}

// port: test/services/images_service_test.rb:397 "get_image_dimensions handles non-existent file"
#[test]
fn get_image_dimensions_handles_non_existent_file() {
    assert_eq!(images::image_dimensions(Path::new("/nonexistent/file.png")), Dimensions::default());
}

// port: test/services/images_service_test.rb:406 "resize_and_compress returns content and changes filename to jpg"
#[test]
fn resize_and_compress_returns_content_and_changes_filename_to_jpg() {
    let r = images::resize_and_compress(&PNG_1X1, "source.png", Some("source.png"), &Param::Float(1.0));
    assert!(!r.bytes.is_empty());
    assert!(!r.filename.is_empty());
    assert!(["image/jpeg", "image/png"].contains(&r.content_type.as_str()));
    // With the `image` crate the re-encode always succeeds for a valid PNG.
    assert_eq!(r.content_type, "image/jpeg");
    assert_eq!(r.filename, "source.jpg");
}

// port: test/services/images_service_test.rb:428 "resize_and_compress falls back to the original file when ImageMagick is missing"
// Adapted: there is no external binary to be missing; the equivalent failure
// is an undecodable source, which must degrade to the original bytes.
#[test]
fn resize_and_compress_falls_back_to_the_original_file_on_failure() {
    let broken = b"\x89PNG\r\n\x1a\nnot really a png".to_vec();
    let r = images::resize_and_compress(&broken, "source.png", Some("source.png"), &Param::Float(0.5));
    assert_eq!(r.bytes, broken);
    assert_eq!(r.content_type, "image/png");
    assert_eq!(r.filename, "source.png");
}

// port: test/services/images_service_test.rb:450 "imagemagick_cmd uses the magick entrypoint when ImageMagick 7 is installed"
// SKIPPED: ImageMagick is not used by the Rust port (resizing is in-process
// with the `image` crate), so there is no IM6/IM7 binary selection to test.

// port: test/services/images_service_test.rb:457 "imagemagick_cmd falls back to the legacy binaries without ImageMagick 7"
// SKIPPED: same reason as above.

// port: test/services/images_service_test.rb:466 "imagemagick_resize_arg returns correct percentage for known ratios"
#[test]
fn resize_arg_returns_correct_percentage_for_known_ratios() {
    assert_eq!(images::resize_arg(&Param::Float(0.25)), "25%");
    assert_eq!(images::resize_arg(&Param::Float(0.5)), "50%");
    assert_eq!(images::resize_arg(&Param::Float(0.75)), "75%");
    assert_eq!(images::resize_arg(&Param::Int(1)), "100%");
    assert_eq!(images::resize_arg(&Param::str("1.0")), "100%");
    assert_eq!(images::resize_arg(&Param::str("0.33")), "33%");
    assert_eq!(images::resize_arg(&Param::str("0.67")), "67%");
}

// port: test/services/images_service_test.rb:476 "imagemagick_resize_arg defaults to 50% for unknown values"
#[test]
fn resize_arg_defaults_to_50_percent_for_unknown_values() {
    assert_eq!(images::resize_arg(&Param::Float(0.42)), "50%");
    assert_eq!(images::resize_arg(&Param::str("arbitrary")), "50%");
    assert_eq!(images::resize_arg(&Param::Nil), "50%");
}

// port: test/services/images_service_test.rb:482 "imagemagick_resize_arg rejects shell injection attempts"
#[test]
fn resize_arg_rejects_shell_injection_attempts() {
    assert_eq!(images::resize_arg(&Param::str("50%; rm -rf /")), "50%");
    assert_eq!(images::resize_arg(&Param::str("100%\nmalicious")), "50%");
    assert_eq!(images::resize_arg(&Param::str("`whoami`")), "50%");
}

// port: test/services/images_service_test.rb:491 "content_type_for returns correct mime types"
#[test]
fn content_type_for_returns_correct_mime_types() {
    assert_eq!(images::content_type_for("test.jpg"), "image/jpeg");
    assert_eq!(images::content_type_for("test.jpeg"), "image/jpeg");
    assert_eq!(images::content_type_for("test.png"), "image/png");
    assert_eq!(images::content_type_for("test.gif"), "image/gif");
    assert_eq!(images::content_type_for("test.webp"), "image/webp");
    assert_eq!(images::content_type_for("test.svg"), "image/svg+xml");
    assert_eq!(images::content_type_for("test.bmp"), "image/bmp");
    assert_eq!(images::content_type_for("test.unknown"), "application/octet-stream");
}

// port: test/services/images_service_test.rb:504 "extension_for_content_type returns correct extensions"
#[test]
fn extension_for_content_type_returns_correct_extensions() {
    let e = |ct| images::extension_for_content_type(Some(ct));
    assert_eq!(e("image/jpeg"), Some(".jpg"));
    assert_eq!(e("image/png"), Some(".png"));
    assert_eq!(e("image/gif"), Some(".gif"));
    assert_eq!(e("image/webp"), Some(".webp"));
    assert_eq!(e("image/svg+xml"), Some(".svg"));
    assert_eq!(e("image/bmp"), Some(".bmp"));
    assert_eq!(e("unknown/type"), Some(".jpg"));
}

// port: test/services/images_service_test.rb:514 "extension_for_content_type handles content-type with charset"
#[test]
fn extension_for_content_type_handles_content_type_with_charset() {
    assert_eq!(images::extension_for_content_type(Some("image/png; charset=utf-8")), Some(".png"));
}
