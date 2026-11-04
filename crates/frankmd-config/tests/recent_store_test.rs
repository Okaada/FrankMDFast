//! `ConfigStore` as the real `frankmd_fs::RecentStore` (`recent_files` lives
//! in `.fed`). Hosts the cross-crate port that frankmd-fs keeps `#[ignore]`d
//! because it cannot depend on this crate.
#![allow(clippy::unwrap_used, clippy::disallowed_methods)]

mod common;

use common::*;
use frankmd_config::Key;
use frankmd_fs::{RecentFilesService, RecentStore};

fn now() -> jiff::Timestamp {
    jiff::Timestamp::now()
}

// port: test/services/recent_files_service_test.rb:67 "concurrent config modifications preserve every opening and settings"
#[test]
fn concurrent_config_modifications_preserve_every_opening_and_settings() {
    let v = TestVault::new();
    for i in 0..6 {
        write_file(&v.root.join(format!("{i}.md")), b"# Note\n");
    }
    let _ = v.open_empty_env(); // create the template first, like the Ruby setup
    std::thread::scope(|scope| {
        for i in 0..6 {
            let v = &v;
            scope.spawn(move || {
                let vault = v.vault();
                let store = v.open_empty_env();
                assert!(RecentFilesService::new(&vault, &store).record(Some(&format!("{i}.md")), now()).unwrap());
            });
        }
        scope.spawn(|| {
            v.open_empty_env().update(&[(Key::Theme, s("nord"))]).unwrap();
        });
    });
    let vault = v.vault();
    let store = v.open_empty_env();
    assert_eq!(RecentFilesService::new(&vault, &store).entries().len(), 6);
    assert_eq!(get_str(&store.current(), Key::Theme).as_deref(), Some("nord"));
}

#[test]
fn recent_files_round_trip_keeps_entry_key_order_and_comments() {
    let v = TestVault::with_fed("# keep me\ntheme = dark\n# ai_model = placeholder\n");
    write_file(&v.root.join("a.md"), b"a");
    let vault = v.vault();
    let store = v.open_empty_env();
    assert!(store.is_present());
    assert_eq!(store.load(), None);
    assert!(RecentFilesService::new(&vault, &store).record(Some("a.md"), now()).unwrap());
    let text = v.read_fed();
    assert!(text.starts_with("# keep me\ntheme = dark\n# ai_model = placeholder\nrecent_files = [{\"path\":\"a.md\",\"opened_at\":"), "{text}");
    assert_eq!(v.mode(FED), 0o600);
    // The snapshot follows the write.
    assert!(store.current().get(Key::RecentFiles).value.is_some());
    assert_eq!(RecentFilesService::new(&vault, &store).entries()[0].path, "a.md");
}

#[test]
fn recents_missing_note_is_an_error_and_writes_nothing() {
    let v = TestVault::with_fed("theme = dark\n# ai_model = placeholder\n");
    let vault = v.vault();
    let store = v.open_empty_env();
    assert!(RecentFilesService::new(&vault, &store).record(Some("missing.md"), now()).is_err());
    assert_eq!(v.read_fed(), "theme = dark\n# ai_model = placeholder\n");
}

#[test]
fn recents_write_fails_softly_on_symlinked_lock() {
    let v = TestVault::with_fed("theme = dark\n# ai_model = placeholder\n");
    write_file(&v.root.join("a.md"), b"a");
    write_file(&v.root.join("x.lock"), b"");
    std::os::unix::fs::symlink("x.lock", v.root.join(LOCK)).unwrap();
    let vault = v.vault();
    let store = v.open_empty_env();
    assert!(!RecentFilesService::new(&vault, &store).record(Some("a.md"), now()).unwrap());
    assert_eq!(v.read_fed(), "theme = dark\n# ai_model = placeholder\n");
}

#[test]
fn debug_output_never_shows_secrets() {
    let v = TestVault::with_fed("openai_api_key = sk-file-secret\n");
    let store = v.open(env(&[("ANTHROPIC_API_KEY", "sk-env-secret")]));
    let dbg = format!("{store:?} {:?}", store.current());
    assert!(!dbg.contains("sk-file-secret") && !dbg.contains("sk-env-secret"), "{dbg}");
}
