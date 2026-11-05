//! Windows path adapters: separator normalization at the lexical-containment
//! seam (issue #1). `Vault::open` canonicalizes the notes root, which yields a
//! verbatim `\\?\` path on Windows; verbatim paths reject `/` (reserved
//! character, parsing disabled), so a `/`-joined candidate fails every Win32
//! call with `ERROR_INVALID_NAME`. These tests run on Windows only.
#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

use frankmd_core::path::{RelPath, is_config_file, is_within, lexical_contain};

const VERBATIM_ROOT: &str = r"\\?\C:\Users\u\Documents\Notes";

#[test]
fn lexical_contain_joins_with_windows_separators() {
    assert_eq!(
        lexical_contain(VERBATIM_ROOT, Some("asdf.md")),
        Some(r"\\?\C:\Users\u\Documents\Notes\asdf.md".to_owned())
    );
    assert_eq!(
        lexical_contain(VERBATIM_ROOT, Some("sub/asdf.md")),
        Some(r"\\?\C:\Users\u\Documents\Notes\sub\asdf.md".to_owned())
    );
    // A `/`-spelled base is normalized too, so both spellings behave alike.
    assert_eq!(
        lexical_contain(r"C:\Users\u\Documents/Notes", Some("asdf.md")),
        Some(r"C:\Users\u\Documents\Notes\asdf.md".to_owned())
    );
}

#[test]
fn lexical_contain_still_rejects_traversal() {
    assert_eq!(lexical_contain(VERBATIM_ROOT, Some("../../windows")), None);
    assert_eq!(lexical_contain(VERBATIM_ROOT, Some(r"..\..\windows")), None);
    // Lexically-inside `..` is resolved, not rejected.
    assert_eq!(
        lexical_contain(VERBATIM_ROOT, Some("sub/../asdf.md")),
        Some(r"\\?\C:\Users\u\Documents\Notes\asdf.md".to_owned())
    );
}

#[test]
fn is_within_anchors_on_the_windows_separator() {
    assert!(is_within(r"C:\notes\a.md", r"C:\notes"));
    assert!(!is_within(r"C:\notes-backup\a.md", r"C:\notes"));
}

#[test]
fn rel_path_accepts_windows_separators() {
    assert_eq!(
        RelPath::parse(r"sub\asdf.md").unwrap().as_str(),
        "sub/asdf.md"
    );
    assert!(RelPath::parse(r"\\notes").is_err());
}

#[test]
fn config_file_guard_fires_with_windows_separators() {
    assert!(is_config_file(".fed"));
    assert!(is_config_file(r"sub\..\.fed"));
    assert!(!is_config_file("sub/.fed"));
    assert!(!is_config_file("regular.md"));
}
