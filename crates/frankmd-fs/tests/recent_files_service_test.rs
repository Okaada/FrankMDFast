//! Port of `test/services/recent_files_service_test.rb` (7 tests) onto
//! `RecentFilesService` + a `.fed`-backed fake [`RecentStore`]
//! (`common::FedRecentStore`; the real one is `frankmd_config::ConfigStore`).
#![allow(clippy::unwrap_used, clippy::disallowed_methods)]

mod common;

use common::{FedRecentStore, Sandbox};
use frankmd_fs::fault::{FaultPoint, inject};
use frankmd_fs::{NotesService, RecentFilesService};

const NOTE: &str = "# Test\n\nContent";

fn now() -> jiff::Timestamp {
    jiff::Timestamp::now()
}

fn paths(entries: &[frankmd_core::recent::RecentFile]) -> Vec<String> {
    entries.iter().map(|e| e.path.clone()).collect()
}

// port: test/services/recent_files_service_test.rb:14 "shares bounded deduplicated opening order across instances"
#[test]
fn shares_bounded_deduplicated_opening_order_across_instances() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let store = FedRecentStore::new(&sb.root);
    let history = RecentFilesService::new(&vault, &store);
    for i in 0..12 {
        sb.write(&format!("{i}.md"), NOTE);
        assert!(history.record(Some(&format!("{i}.md")), now()).unwrap());
    }
    assert!(history.record(Some("4.md"), now()).unwrap());

    let other_store = FedRecentStore::new(&sb.root);
    let entries = RecentFilesService::new(&vault, &other_store).entries();
    assert_eq!(entries.len(), 10);
    assert_eq!(entries[0].path, "4.md");
    let mut uniq = paths(&entries);
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), entries.len());
    assert!(frankmd_core::recent::parse_opened_at(&entries[0].opened_at).is_some());
    assert!(!sb.read(".fed").contains("# Test"));
}

// port: test/services/recent_files_service_test.rb:28 "external deletion is filtered without removing unrelated entries"
#[test]
fn external_deletion_is_filtered_without_removing_unrelated_entries() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let store = FedRecentStore::new(&sb.root);
    let history = RecentFilesService::new(&vault, &store);
    sb.write("gone.md", NOTE);
    sb.write("keep.md", NOTE);
    history.record(Some("gone.md"), now()).unwrap();
    history.record(Some("keep.md"), now()).unwrap();
    std::fs::remove_file(sb.abs("gone.md")).unwrap();
    assert_eq!(paths(&history.entries()), ["keep.md"]);
    let err = history.record(Some("gone.md"), now()).unwrap_err();
    assert_eq!(err.compat_class(), "NotesService::NotFoundError");
    assert!(!sb.exists("gone.md"));
}

// port: test/services/recent_files_service_test.rb:39 "successful file and folder mutations update shared history"
#[test]
fn successful_file_and_folder_mutations_update_shared_history() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let store = FedRecentStore::new(&sb.root);
    let history = RecentFilesService::new(&vault, &store);
    sb.write("docs/first.md", NOTE);
    sb.write("docs-other/other.md", NOTE);
    history.record(Some("docs/first.md"), now()).unwrap();
    history.record(Some("docs-other/other.md"), now()).unwrap();

    let notes = NotesService::new(&vault).with_recents(&store);
    notes.rename("docs", "renamed").unwrap();
    assert_eq!(paths(&history.entries()), ["docs-other/other.md", "renamed/first.md"]);
    notes.rename("renamed/first.md", "renamed/second.md").unwrap();
    notes.delete("renamed/second.md", None).unwrap();
    assert_eq!(paths(&history.entries()), ["docs-other/other.md"]);
}

// port: test/services/recent_files_service_test.rb:52 "rejects traversal hidden files outside symlinks and non-markdown paths"
#[test]
fn rejects_traversal_hidden_files_outside_symlinks_and_non_markdown_paths() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let store = FedRecentStore::new(&sb.root);
    let history = RecentFilesService::new(&vault, &store);
    sb.write("safe.md", NOTE);
    std::fs::write(sb.outside.join("README.md"), "outside").unwrap();
    std::os::unix::fs::symlink(sb.outside.join("README.md"), sb.abs("outside.md")).unwrap();
    for path in [
        Some("../README.md"),
        Some("/tmp/secret.md"),
        Some(".secret.md"),
        Some("docs/../safe.md"),
        Some("outside.md"),
        Some("image.png"),
        None,
    ] {
        let err = history.record(path, now()).unwrap_err();
        assert_eq!(err.compat_class(), "NotesService::NotFoundError", "{path:?}");
    }
}

// port: test/services/recent_files_service_test.rb:60 "permission errors do not erase an existing entry"
#[test]
fn permission_errors_do_not_erase_an_existing_entry() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let store = FedRecentStore::new(&sb.root);
    let history = RecentFilesService::new(&vault, &store);
    sb.write("safe.md", NOTE);
    history.record(Some("safe.md"), now()).unwrap();
    let _fault = inject(
        FaultPoint::Stat("safe.md".into()),
        std::io::ErrorKind::PermissionDenied,
        Some(13), // EACCES
        false,
    );
    assert_eq!(history.entries()[0].path, "safe.md");
}

// port: test/services/recent_files_service_test.rb:67 "concurrent config modifications preserve every opening and settings"
#[test]
#[ignore = "needs frankmd-config ConfigStore (cross-crate): the guarantee is ConfigStore::modify's \
            read-modify-write under .frankmd-config.lock, which the in-test fake store does not model"]
fn concurrent_config_modifications_preserve_every_opening_and_settings() {}

// port: test/services/recent_files_service_test.rb:79 "malformed history does not discard other configuration"
// (the `theme == "nord"` half is Config parsing, ported in frankmd-config's tests.)
#[test]
fn malformed_history_does_not_discard_other_configuration() {
    let sb = Sandbox::empty();
    sb.write(".fed", "theme = nord\nrecent_files = [broken\n");
    let vault = sb.vault();
    let store = FedRecentStore::new(&sb.root);
    let history = RecentFilesService::new(&vault, &store);
    assert!(history.entries().is_empty());
    assert!(history.clear().unwrap());
    assert!(RecentFilesService::new(&vault, &FedRecentStore::new(&sb.root)).entries().is_empty());
    assert!(sb.read(".fed").contains("theme = nord"));
}
