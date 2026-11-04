//! Port of `test/services/media_library_service_test.rb` (9 tests).
//! Config stub: images `.png .jpg`, videos `.mp4 .webm`.
#![allow(clippy::disallowed_methods)]

#[path = "common/library.rs"]
mod library;

use library::{Notes, chmod, config, is_root};
use frankmd_media::MediaKind;
use frankmd_media::error::MediaError;
use frankmd_media::library::MediaLibrary;

fn paths(items: &[frankmd_media::library::MediaItem]) -> Vec<String> {
    let mut v: Vec<String> = items.iter().map(|i| i.path.clone()).collect();
    v.sort();
    v
}

// port: test/services/media_library_service_test.rb:18 "list returns supported images and videos with note asset preview paths"
#[test]
fn list_returns_supported_images_and_videos_with_preview_paths() {
    let n = Notes::new();
    n.write("images/photo.png", "image");
    n.write("images/not-supported.svg", "unsupported image");
    n.write("videos/clip.mp4", "video");
    n.write("videos/nested/clip.webm", "nested video");
    n.write("videos/readme.txt", "unsupported video");
    let vault = n.vault();
    let result = MediaLibrary::from_config(&vault, &config(n.root())).list();

    assert_eq!(paths(&result.images), vec!["images/photo.png"]);
    assert_eq!(paths(&result.videos), vec!["videos/clip.mp4", "videos/nested/clip.webm"]);
    let image = &result.images[0];
    assert_eq!(image.name, "photo.png");
    assert_eq!(image.kind, MediaKind::Image);
    assert_eq!(result.videos[0].kind, MediaKind::Video);
    assert_eq!(image.preview_url, "/notes/images/photo.png");
    assert_eq!(image.size, "image".len() as u64);
    assert!(!image.mtime.is_empty());
}

// port: test/services/media_library_service_test.rb:38 "list ignores symlinked media files and directories"
#[test]
fn list_ignores_symlinked_files_and_directories() {
    let n = Notes::new();
    let outside = n.write_outside("outside_media.png", "outside");
    n.write("images/real.png", "real");
    n.symlink("images/linked.png", &outside);
    n.symlink("images/external", n.outside.path());
    let vault = n.vault();
    assert_eq!(paths(&MediaLibrary::from_config(&vault, &config(n.root())).list().images), vec!["images/real.png"]);
}

// port: test/services/media_library_service_test.rb:50 "delete removes a supported file from a managed media directory"
#[test]
fn delete_removes_supported_file() {
    let n = Notes::new();
    let path = n.write("images/nested/photo.png", "image");
    let vault = n.vault();
    assert!(MediaLibrary::from_config(&vault, &config(n.root())).delete("images/nested/photo.png").expect("delete"));
    assert!(!path.exists());
}

// port: test/services/media_library_service_test.rb:57 "delete rejects traversal, absolute paths, and paths outside managed roots"
#[test]
fn delete_rejects_traversal_absolute_and_outside_roots() {
    let n = Notes::new();
    // The original put the outside file in the notes dir's parent; mirror with a sibling of the root.
    let outside = n.root().parent().expect("parent").join(format!(
        "library_delete_outside_{}.png",
        n.root().file_name().expect("name").to_string_lossy()
    ));
    std::fs::write(&outside, "outside").expect("write");
    n.write("notes.png", "note");
    n.write("attachments/photo.png", "attachment");
    let vault = n.vault();
    let lib = MediaLibrary::from_config(&vault, &config(n.root()));

    let base = outside.file_name().expect("name").to_string_lossy().into_owned();
    assert!(!lib.delete(&format!("images/../{base}")).expect("no error"));
    assert!(!lib.delete(&outside.display().to_string()).expect("no error"));
    assert!(!lib.delete("notes.png").expect("no error"));
    assert!(!lib.delete("attachments/photo.png").expect("no error"));
    assert!(outside.exists());
    assert!(n.root().join("notes.png").exists());
    assert!(n.root().join("attachments/photo.png").exists());
    let _ = std::fs::remove_file(outside);
}

// port: test/services/media_library_service_test.rb:74 "delete rejects unsupported extensions"
#[test]
fn delete_rejects_unsupported_extensions() {
    let n = Notes::new();
    let path = n.write("images/readme.md", "note content");
    let vault = n.vault();
    assert!(!MediaLibrary::from_config(&vault, &config(n.root())).delete("images/readme.md").expect("no error"));
    assert!(path.exists());
}

// port: test/services/media_library_service_test.rb:81 "delete resolves paths containing null bytes to not-found instead of raising"
#[test]
fn delete_with_nul_bytes_is_not_found() {
    let n = Notes::new();
    n.write("images/photo.png", "image");
    let vault = n.vault();
    let lib = MediaLibrary::from_config(&vault, &config(n.root()));
    assert!(!lib.delete("images/a\0b.png").expect("no error"));
    assert!(!lib.delete("images/nested/a\0b.png").expect("no error"));
    assert!(!lib.delete("images/\0").expect("no error"));
    assert!(n.root().join("images/photo.png").exists());
}

// port: test/services/media_library_service_test.rb:90 "delete rejects symlinks in nested directories and leaves their targets intact"
#[test]
fn delete_rejects_symlinked_nested_directory() {
    let n = Notes::new();
    let outside = n.write_outside("library_symlink_target.png", "outside");
    n.symlink("images/external", n.outside.path());
    let vault = n.vault();
    let lib = MediaLibrary::from_config(&vault, &config(n.root()));
    assert!(!lib.delete("images/external/library_symlink_target.png").expect("no error"));
    assert!(outside.exists());
    assert!(n.root().join("images/external").is_symlink());
}

// port: test/services/media_library_service_test.rb:103 "delete rejects a symlinked leaf file and leaves its target intact"
#[test]
fn delete_rejects_symlinked_leaf() {
    let n = Notes::new();
    let outside = n.write_outside("library_symlink_leaf_target.png", "outside");
    n.symlink("images/linked.png", &outside);
    let vault = n.vault();
    assert!(!MediaLibrary::from_config(&vault, &config(n.root())).delete("images/linked.png").expect("no error"));
    assert!(outside.exists());
    assert!(n.root().join("images/linked.png").is_symlink());
}

// port: test/services/media_library_service_test.rb:116 "delete reports filesystem errors to its caller"
#[test]
fn delete_reports_filesystem_errors() {
    if is_root() {
        eprintln!("skipped: running as root, chmod cannot produce EACCES");
        return;
    }
    let n = Notes::new();
    let path = n.write("images/protected.png", "image");
    // `File.stubs(:delete).raises(Errno::EACCES)` → a read-only parent directory.
    let _guard = chmod(&n.root().join("images"), 0o555);
    let vault = n.vault();
    let err = MediaLibrary::from_config(&vault, &config(n.root())).delete("images/protected.png").expect_err("EACCES");
    assert!(matches!(err, MediaError::Io(std::io::ErrorKind::PermissionDenied) | MediaError::Fs(_)), "{err:?}");
    assert!(path.exists());
}
