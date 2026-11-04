//! Port of the category-A parts of `test/controllers/recent_files_controller_test.rb`
//! through the controller emulation in `common::http`. `GET /recent_files`
//! returns the entries as data instead of the HTML panel (panel rendering is
//! category C, owned by the app's Home view).
#![allow(missing_docs, unreachable_pub, clippy::unwrap_used, clippy::disallowed_methods)]

mod common;

use common::http::{App, dispatch};
use common::{FedRecentStore, Sandbox};
use frankmd_fs::{FsError, NotesService, RecentStore};
use frankmd_testkit::serde_json::{Value, json};

const DEFAULT_NOTE: &str = "# Test\n\nContent";

// port: test/controllers/recent_files_controller_test.rb:9 "shared history records opens renders escaped paths and clears"
#[test]
fn shared_history_records_opens_and_clears() {
    let sb = Sandbox::empty();
    sb.write("docs/<sample>.md", DEFAULT_NOTE);
    assert_eq!(dispatch(&sb, "POST", "/recent_files", &json!({"path": "docs/<sample>.md"})).status, 204);
    let r = dispatch(&sb, "GET", "/recent_files", &Value::Null);
    assert_eq!(r.status, 200);
    let paths: Vec<&str> = r.body.as_array().unwrap().iter().map(|e| e["path"].as_str().unwrap()).collect();
    assert_eq!(paths, vec!["docs/<sample>.md"]);
    assert_eq!(dispatch(&sb, "DELETE", "/recent_files", &Value::Null).status, 204);
    let vault = sb.vault();
    let store = FedRecentStore::new(&sb.root);
    assert!(frankmd_fs::RecentFilesService::new(&vault, &store).entries().is_empty());
    let r = dispatch(&sb, "GET", "/recent_files", &Value::Null);
    assert_eq!(r.status, 200);
    assert_eq!(r.body, json!([]));
}

// port: test/controllers/recent_files_controller_test.rb:75 "missing notes cannot be recorded and config state is not user-writable"
#[test]
fn missing_notes_cannot_be_recorded() {
    // The `PATCH /config {recent_files}` → 422 half is frankmd-config's
    // contract (recent_files is not a UI key) and is ported there.
    let sb = Sandbox::empty();
    assert_eq!(dispatch(&sb, "POST", "/recent_files", &json!({"path": "missing.md"})).status, 404);
    assert!(!sb.exists("missing.md"));
}

/// A store whose writes fail (`Config#set` → false).
struct FailingStore;

impl RecentStore for FailingStore {
    fn load(&self) -> Option<String> {
        None
    }
    fn modify(&self, f: &mut dyn FnMut(Option<String>) -> Result<String, FsError>) -> Result<bool, FsError> {
        f(None)?;
        Ok(false)
    }
    fn is_present(&self) -> bool {
        true
    }
}

// port: test/controllers/recent_files_controller_test.rb:83 "history failure reports unavailable without a successful response"
#[test]
fn history_failure_reports_unavailable() {
    // Rails stubs `RecentFilesService#record` → false; here the store's write
    // fails instead (the note exists so validation passes).
    let sb = Sandbox::empty();
    sb.write("safe.md", DEFAULT_NOTE);
    let store = FailingStore;
    let r = App::new(&sb, &store).call("POST", "/recent_files", &json!({"path": "safe.md"}), false);
    assert_eq!(r.status, 503);
}

// port: test/controllers/recent_files_controller_test.rb:89 "config lock cannot be downloaded or changed as a note"
#[test]
fn config_lock_cannot_be_downloaded_or_changed_as_a_note() {
    let sb = Sandbox::empty();
    sb.write(frankmd_fs::lock::CONFIG_LOCK, "");
    let r = dispatch(&sb, "GET", &format!("/notes/{}", frankmd_fs::lock::CONFIG_LOCK), &Value::Null);
    assert_eq!(r.status, 404);
    let vault = sb.vault();
    let err = NotesService::new(&vault).write(frankmd_fs::lock::CONFIG_LOCK, b"unsafe").unwrap_err();
    assert!(matches!(err, FsError::InvalidPath(_)), "expected InvalidPath, got {err:?}");
}
