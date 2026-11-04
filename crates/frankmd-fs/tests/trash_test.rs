//! `NotesService::trash` with the directory trash backend (never the real
//! system trash).
#![allow(clippy::disallowed_methods, clippy::unwrap_used, missing_docs)]

mod common;

use frankmd_core::Revision;
use frankmd_fs::{FsError, NotesService, RecentFilesService, RecentStore, TrashBackend, Vault};
use std::fs;

struct Env {
    root: tempfile::TempDir,
    _state: tempfile::TempDir,
    bin: tempfile::TempDir,
    vault: Vault,
}

fn env() -> Env {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path(), state.path())
        .unwrap()
        .with_trash(TrashBackend::Directory(bin.path().join("trash")));
    Env { root, _state: state, bin, vault }
}

fn trashed(e: &Env, name: &str) -> std::path::PathBuf {
    e.bin.path().join("trash").join(name)
}

#[test]
fn default_backend_is_system() {
    let state = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path(), state.path()).unwrap();
    assert_eq!(vault.trash_backend(), &TrashBackend::System);
}

#[test]
fn file_is_trashed_with_content() {
    let e = env();
    fs::write(e.root.path().join("a.md"), "hello").unwrap();
    NotesService::new(&e.vault).trash("a.md", None).unwrap();
    assert!(!e.root.path().join("a.md").exists());
    assert_eq!(fs::read_to_string(trashed(&e, "a.md")).unwrap(), "hello");
}

#[test]
fn non_empty_folder_is_trashed() {
    let e = env();
    fs::create_dir_all(e.root.path().join("dir/sub")).unwrap();
    fs::write(e.root.path().join("dir/x.md"), "x").unwrap();
    fs::write(e.root.path().join("dir/sub/y.md"), "y").unwrap();
    NotesService::new(&e.vault).trash("dir", None).unwrap();
    assert!(!e.root.path().join("dir").exists());
    assert_eq!(fs::read_to_string(trashed(&e, "dir/sub/y.md")).unwrap(), "y");
    assert_eq!(fs::read_to_string(trashed(&e, "dir/x.md")).unwrap(), "x");
}

#[test]
fn same_names_get_unique_trash_names() {
    let e = env();
    let svc = NotesService::new(&e.vault);
    for (dir, body) in [("one", "1"), ("two", "2"), ("three", "3")] {
        fs::create_dir_all(e.root.path().join(dir)).unwrap();
        fs::write(e.root.path().join(dir).join("n.md"), body).unwrap();
        svc.trash(&format!("{dir}/n.md"), None).unwrap();
    }
    assert_eq!(fs::read_to_string(trashed(&e, "n.md")).unwrap(), "1");
    assert_eq!(fs::read_to_string(trashed(&e, "n.md.1")).unwrap(), "2");
    assert_eq!(fs::read_to_string(trashed(&e, "n.md.2")).unwrap(), "3");
}

#[test]
fn revision_mismatch_keeps_the_file() {
    let e = env();
    fs::write(e.root.path().join("a.md"), "hello").unwrap();
    let svc = NotesService::new(&e.vault);
    let err = svc.trash("a.md", Some("deadbeef")).unwrap_err();
    assert!(matches!(err, FsError::RevisionConflict(_)));
    assert!(e.root.path().join("a.md").exists());
    let rev = Revision::of(b"hello");
    svc.trash("a.md", Some(&rev.0)).unwrap();
    assert!(!e.root.path().join("a.md").exists());
}

#[test]
fn missing_path_is_not_found() {
    let e = env();
    let err = NotesService::new(&e.vault).trash("nope.md", None).unwrap_err();
    assert!(matches!(err, FsError::NotFound(_)), "{err:?}");
}

#[test]
fn vault_root_is_refused() {
    let e = env();
    fs::write(e.root.path().join("a.md"), "x").unwrap();
    for raw in ["", ".", "/"] {
        let err = NotesService::new(&e.vault).trash(raw, None).unwrap_err();
        assert!(matches!(err, FsError::InvalidPath(_)), "{raw:?}: {err:?}");
    }
    assert!(e.root.path().join("a.md").exists());
}

#[test]
fn config_file_is_refused() {
    let e = env();
    fs::write(e.root.path().join(".fed"), "x").unwrap();
    let err = NotesService::new(&e.vault).trash(".fed", None).unwrap_err();
    assert!(matches!(err, FsError::PermissionDenied(_)), "{err:?}");
    assert!(e.root.path().join(".fed").exists());
}

#[cfg(unix)]
#[test]
fn symlink_moves_only_the_link() {
    let e = env();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("secret.md");
    fs::write(&target, "secret").unwrap();
    std::os::unix::fs::symlink(&target, e.root.path().join("link.md")).unwrap();
    NotesService::new(&e.vault).trash("link.md", None).unwrap();
    assert!(fs::symlink_metadata(e.root.path().join("link.md")).is_err());
    assert!(fs::symlink_metadata(trashed(&e, "link.md")).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(&target).unwrap(), "secret");
}

#[test]
fn recents_entry_is_removed() {
    let e = env();
    fs::write(e.root.path().join("a.md"), "x").unwrap();
    fs::write(e.root.path().join("b.md"), "y").unwrap();
    let store = common::FedRecentStore::new(e.root.path());
    let recents = RecentFilesService::new(&e.vault, &store as &dyn RecentStore);
    recents.record(Some("a.md"), jiff::Timestamp::now()).unwrap();
    recents.record(Some("b.md"), jiff::Timestamp::now()).unwrap();
    NotesService::new(&e.vault)
        .with_recents(&store as &dyn RecentStore)
        .trash("a.md", None)
        .unwrap();
    let paths: Vec<String> = recents.entries().into_iter().map(|r| r.path).collect();
    assert_eq!(paths, vec!["b.md".to_owned()]);
}
