//! The issue #1 flow on a real Windows filesystem: every vault operation goes
//! through `safe_path`, whose base is the canonicalized (verbatim `\\?\`)
//! notes root. Before the separator normalization in `frankmd-core`, each of
//! these calls failed with `ERROR_INVALID_NAME` surfaced as the generic
//! "Failed to create" / "Not found" errors.
//!
//! Self-contained on purpose: the shared `common` fixtures exercise Unix file
//! modes and symlinks and do not compile on Windows.
#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

use frankmd_fs::{NotesService, Vault};

#[test]
fn create_read_rename_delete_work_from_a_verbatim_root() {
    let tmp = tempfile::tempdir().unwrap();
    let vault = Vault::open(tmp.path(), tmp.path().join("state")).unwrap();
    let svc = NotesService::new(&vault);

    // The reporter's steps: a folder and a note in the notes root.
    svc.create_folder("asdf").unwrap();
    svc.create("asdf.md", b"").unwrap();
    // The same seam with a nested path (`RelPath` is `/`-separated inside).
    svc.create_folder("sub").unwrap();
    svc.create("sub/asdf.md", b"hello").unwrap();

    assert_eq!(svc.read("asdf.md").unwrap().bytes, b"");
    assert_eq!(svc.read("sub/asdf.md").unwrap().bytes, b"hello");

    svc.rename("asdf.md", "renamed.md").unwrap();
    assert_eq!(svc.read("renamed.md").unwrap().bytes, b"");

    svc.delete("renamed.md", None).unwrap();
    svc.delete("sub/asdf.md", None).unwrap();
    svc.delete_folder("sub").unwrap();
    svc.delete_folder("asdf").unwrap();
}
