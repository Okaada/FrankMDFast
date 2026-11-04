//! `Vault::open_foreign`: a folder that is not a notes root (the user's
//! `images_path`) keeps every app file — including locks — in the state dir.
#![allow(clippy::disallowed_methods, clippy::unwrap_used, missing_docs)]

use frankmd_fs::Vault;

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn foreign_vault_mutations_create_no_files_in_the_folder() {
    let folder = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("photo.png"), b"x").unwrap();
    std::fs::write(folder.path().join("keep.png"), b"y").unwrap();
    let vault = Vault::open_foreign(folder.path(), state.path()).unwrap();
    let target = vault.safe_path("photo.png", true).unwrap();
    vault.delete_note(&target, None).unwrap();
    assert_eq!(names(folder.path()), vec!["keep.png".to_owned()]);
    // The lock went to the state dir instead.
    assert!(state.path().join("locks").is_dir());
}

#[test]
fn regular_vault_still_uses_the_shared_compat_lock() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path(), state.path()).unwrap();
    let guard = vault
        .lock(
            frankmd_fs::lock::FILESYSTEM_LOCK,
            frankmd_fs::lock::LockKind::Exclusive,
        )
        .unwrap();
    assert_eq!(guard.path(), vault.filesystem_lock_path());
}
