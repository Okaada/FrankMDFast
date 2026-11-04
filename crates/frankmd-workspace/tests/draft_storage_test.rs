//! Port of test/javascript/lib/draft_storage.test.js (22 tests) plus
//! test/javascript/controllers/offline_backup_controller.test.js (8 tests).
#![allow(clippy::unwrap_used)]

#[path = "draft_common/mod.rs"]
mod draft_common;

use draft_common::{Fault, Method, drafts, drafts_seeded};
use frankmd_workspace::draft_storage::{
    DraftErrorKind, DraftRecord, ItemKind, LegacyBackup, backup_key, draft_key,
};

const T: &str = "test/javascript/lib/draft_storage.test.js";

fn backup_json(content: &str, ts: u64) -> String {
    format!(r#"{{"content":{},"timestamp":{ts}}}"#, frankmd_testkit::serde_json::to_string(content).unwrap())
}

fn with_path(d: &DraftRecord, path: &str) -> DraftRecord {
    DraftRecord { path: path.to_owned(), ..d.clone() }
}

// port: test/javascript/lib/draft_storage.test.js:17 "stores and reads an empty draft with its server baseline"
#[test]
fn stores_and_reads_an_empty_draft_with_its_server_baseline() {
    let mut d = drafts();
    let write = d.write_draft("folder/note.md", "", "server-revision").unwrap();
    let read = d.read_draft("folder/note.md").unwrap();

    assert_eq!(read.draft.as_ref(), Some(&write.draft), "{T}");
    assert!(!read.malformed);
    assert_eq!(write.draft.path, "folder/note.md");
    assert_eq!(write.draft.content, "");
    assert_eq!(write.draft.base_revision, "server-revision");
    assert!(!write.draft.draft_revision.is_empty());
    assert!(write.draft.updated_at.is_finite());
}

// port: test/javascript/lib/draft_storage.test.js:36 "skips an unchanged snapshot and keeps its exact draft revision"
#[test]
fn skips_an_unchanged_snapshot_and_keeps_its_exact_draft_revision() {
    let mut d = drafts();
    let first = d.write_draft("note.md", "same", "baseline").unwrap();
    let second = d.write_draft("note.md", "same", "baseline").unwrap();

    assert_eq!(d.backend().set_calls, 1);
    assert!(second.unchanged);
    assert_eq!(second.draft.draft_revision, first.draft.draft_revision);
}

// port: test/javascript/lib/draft_storage.test.js:49 "removes only the matching draft revision"
#[test]
fn removes_only_the_matching_draft_revision() {
    let mut d = drafts();
    let first = d.write_draft("note.md", "older", "baseline").unwrap().draft;
    let second = d.write_draft("note.md", "newer", "baseline").unwrap().draft;

    assert_eq!(d.remove_draft_if_revision("note.md", &first.draft_revision), Ok(false));
    assert_eq!(d.read_draft("note.md").unwrap().draft.unwrap().draft_revision, second.draft_revision);
    assert_eq!(d.remove_draft_if_revision("note.md", &second.draft_revision), Ok(true));
    assert_eq!(d.read_draft("note.md").unwrap().draft, None);
}

// port: test/javascript/lib/draft_storage.test.js:61 "remaps a file draft and preserves its revision metadata"
#[test]
fn remaps_a_file_draft_and_preserves_its_revision_metadata() {
    let mut d = drafts();
    let original = d.write_draft("notes/old.md", "local edit", "server-revision").unwrap().draft;

    assert_eq!(d.remap_drafts("notes/old.md", "archive/new.md", ItemKind::File), Ok(1));
    assert_eq!(d.read_draft("notes/old.md").unwrap().draft, None);
    assert_eq!(d.read_draft("archive/new.md").unwrap().draft, Some(with_path(&original, "archive/new.md")));
}

// port: test/javascript/lib/draft_storage.test.js:71 "remaps only a folder path and its descendants"
#[test]
fn remaps_only_a_folder_path_and_its_descendants() {
    let mut d = drafts();
    d.write_draft("docs/index.md", "index", "rev-1").unwrap();
    d.write_draft("docs/guides/setup.md", "guide", "rev-2").unwrap();
    d.write_draft("docs-old/keep.md", "keep", "rev-3").unwrap();

    assert_eq!(d.remap_drafts("docs", "archive", ItemKind::Folder), Ok(2));
    assert_eq!(d.read_draft("archive/index.md").unwrap().draft.unwrap().content, "index");
    assert_eq!(d.read_draft("archive/guides/setup.md").unwrap().draft.unwrap().content, "guide");
    assert_eq!(d.read_draft("docs-old/keep.md").unwrap().draft.unwrap().content, "keep");
    assert_eq!(d.read_draft("docs/index.md").unwrap().draft, None);
}

// port: test/javascript/lib/draft_storage.test.js:85 "remaps a legacy backup with a renamed file"
#[test]
fn remaps_a_legacy_backup_with_a_renamed_file() {
    let backup = backup_json("unsaved legacy edit", 123);
    let mut d = drafts_seeded(&[(&backup_key("notes/old.md"), &backup)]);

    assert_eq!(d.remap_drafts("notes/old.md", "archive/new.md", ItemKind::File), Ok(1));
    assert_eq!(d.backend().raw(&backup_key("notes/old.md")), None);
    assert_eq!(d.backend().raw(&backup_key("archive/new.md")), Some(backup.as_str()));
}

// port: test/javascript/lib/draft_storage.test.js:96 "remaps legacy backups in a moved folder without touching neighboring paths"
#[test]
fn remaps_legacy_backups_in_a_moved_folder_without_touching_neighboring_paths() {
    let index = backup_json("index edit", 123);
    let nested = backup_json("nested edit", 456);
    let unrelated = backup_json("keep", 789);
    let mut d = drafts_seeded(&[
        (&backup_key("docs/index.md"), &index),
        (&backup_key("docs/guides/setup.md"), &nested),
        (&backup_key("docs-old/keep.md"), &unrelated),
    ]);

    assert_eq!(d.remap_drafts("docs", "archive", ItemKind::Folder), Ok(2));
    let b = d.backend();
    assert_eq!(b.raw(&backup_key("docs/index.md")), None);
    assert_eq!(b.raw(&backup_key("docs/guides/setup.md")), None);
    assert_eq!(b.raw(&backup_key("archive/index.md")), Some(index.as_str()));
    assert_eq!(b.raw(&backup_key("archive/guides/setup.md")), Some(nested.as_str()));
    assert_eq!(b.raw(&backup_key("docs-old/keep.md")), Some(unrelated.as_str()));
}

// port: test/javascript/lib/draft_storage.test.js:114 "preserves both legacy backups and reports a destination collision"
#[test]
fn preserves_both_legacy_backups_and_reports_a_destination_collision() {
    let src = backup_json("source edit", 123);
    let dst = backup_json("other edit", 456);
    let mut d = drafts_seeded(&[(&backup_key("old.md"), &src), (&backup_key("new.md"), &dst)]);

    let err = d.remap_drafts("old.md", "new.md", ItemKind::File).unwrap_err();

    assert!(err.backup_collision);
    assert_eq!(err.source_path.as_deref(), Some("old.md"));
    assert_eq!(err.destination_path.as_deref(), Some("new.md"));
    assert_eq!(d.backend().raw(&backup_key("old.md")), Some(src.as_str()));
    assert_eq!(d.backend().raw(&backup_key("new.md")), Some(dst.as_str()));
}

// port: test/javascript/lib/draft_storage.test.js:134 "retains the source legacy backup when writing the destination fails"
#[test]
fn retains_the_source_legacy_backup_when_writing_the_destination_fails() {
    let backup = backup_json("unsaved edit", 123);
    let mut d = drafts_seeded(&[(&backup_key("old.md"), &backup)]);
    d.backend_mut().add_fault(Fault::throw(Method::SetItem).on_key(&backup_key("new.md")).message("quota exceeded"));

    let err = d.remap_drafts("old.md", "new.md", ItemKind::File).unwrap_err();
    assert_eq!(err.source_path.as_deref(), Some("old.md"));
    assert_eq!(d.backend().raw(&backup_key("old.md")), Some(backup.as_str()));
    assert_eq!(d.backend().raw(&backup_key("new.md")), None);
}

// port: test/javascript/lib/draft_storage.test.js:149 "keeps both legacy backup copies if removing the verified source fails"
#[test]
fn keeps_both_legacy_backup_copies_if_removing_the_verified_source_fails() {
    let backup = backup_json("unsaved edit", 123);
    let mut d = drafts_seeded(&[(&backup_key("old.md"), &backup)]);
    d.backend_mut().add_fault(Fault::throw(Method::RemoveItem).on_key(&backup_key("old.md")));

    let err = d.remap_drafts("old.md", "new.md", ItemKind::File).unwrap_err();
    assert_eq!(err.source_path.as_deref(), Some("old.md"));
    assert_eq!(d.backend().raw(&backup_key("old.md")), Some(backup.as_str()));
    assert_eq!(d.backend().raw(&backup_key("new.md")), Some(backup.as_str()));

    d.backend_mut().clear_faults();
    assert_eq!(d.remap_drafts("old.md", "new.md", ItemKind::File), Ok(1));
    assert_eq!(d.backend().raw(&backup_key("old.md")), None);
    assert_eq!(d.backend().raw(&backup_key("new.md")), Some(backup.as_str()));
}

// port: test/javascript/lib/draft_storage.test.js:169 "preserves both source and destination drafts when a remap collides"
#[test]
fn preserves_both_source_and_destination_drafts_when_a_remap_collides() {
    let mut d = drafts();
    let source = d.write_draft("old.md", "source version", "source-revision").unwrap().draft;
    let destination = d.write_draft("new.md", "destination version", "destination-revision").unwrap().draft;

    let err = d.remap_drafts("old.md", "new.md", ItemKind::File).unwrap_err();

    assert!(err.collision);
    assert_eq!(err.source_path.as_deref(), Some("old.md"));
    assert_eq!(err.destination_path.as_deref(), Some("new.md"));
    assert_eq!(d.read_draft("old.md").unwrap().draft, None);
    assert_eq!(d.read_draft("new.md").unwrap().draft, Some(destination));
    let conflicts = d.list_draft_conflicts("new.md").unwrap();
    assert_eq!(conflicts.len(), 1);
    let c = &conflicts[0];
    assert_eq!(Some(&c.conflict_id), err.conflict_id.as_ref());
    assert_eq!(c.path, "new.md");
    assert_eq!(c.source_path, "old.md");
    assert_eq!(c.content, "source version");
    assert_eq!(c.draft_revision, source.draft_revision);
}

// port: test/javascript/lib/draft_storage.test.js:191 "keeps every nested draft recoverable when a folder remap has one collision"
#[test]
fn keeps_every_nested_draft_recoverable_when_a_folder_remap_has_one_collision() {
    let mut d = drafts();
    d.write_draft("docs/index.md", "index draft", "index-revision").unwrap();
    let source = d.write_draft("docs/guides/setup.md", "source guide", "source-revision").unwrap().draft;
    let destination =
        d.write_draft("archive/guides/setup.md", "destination guide", "destination-revision").unwrap().draft;
    d.write_draft("docs-old/keep.md", "unrelated", "keep-revision").unwrap();

    let err = d.remap_drafts("docs", "archive", ItemKind::Folder).unwrap_err();

    assert!(err.collision);
    assert_eq!(err.source_path.as_deref(), Some("docs/guides/setup.md"));
    assert_eq!(err.destination_path.as_deref(), Some("archive/guides/setup.md"));
    assert_eq!(d.read_draft("docs/index.md").unwrap().draft, None);
    assert_eq!(d.read_draft("archive/index.md").unwrap().draft, None);
    let idx = d.list_draft_conflicts("archive/index.md").unwrap();
    assert_eq!(idx.len(), 1);
    assert_eq!((idx[0].source_path.as_str(), idx[0].content.as_str()), ("docs/index.md", "index draft"));
    assert_eq!(d.read_draft("docs/guides/setup.md").unwrap().draft, None);
    assert_eq!(d.read_draft("archive/guides/setup.md").unwrap().draft, Some(destination));
    let setup = d.list_draft_conflicts("archive/guides/setup.md").unwrap();
    assert_eq!(setup.len(), 1);
    assert_eq!((setup[0].source_path.as_str(), setup[0].content.as_str()), (source.path.as_str(), source.content.as_str()));
    assert_eq!(d.read_draft("docs-old/keep.md").unwrap().draft.unwrap().content, "unrelated");
}

// port: test/javascript/lib/draft_storage.test.js:220 "retains the source draft when a destination write fails"
#[test]
fn retains_the_source_draft_when_a_destination_write_fails() {
    let mut d = drafts();
    let source = d.write_draft("old.md", "local edit", "revision").unwrap().draft;
    d.backend_mut().add_fault(Fault::throw(Method::SetItem).on_key(&draft_key("new.md")));

    let err = d.remap_drafts("old.md", "new.md", ItemKind::File).unwrap_err();
    assert_eq!(err.source_path.as_deref(), Some("old.md"));
    assert_eq!(d.read_draft("old.md").unwrap().draft, Some(source));
    assert_eq!(d.read_draft("new.md").unwrap().draft, None);
}

// port: test/javascript/lib/draft_storage.test.js:235 "does not remove a source draft until the verified copy exists"
#[test]
fn does_not_remove_a_source_draft_until_the_verified_copy_exists() {
    let mut d = drafts();
    let source = d.write_draft("old.md", "local edit", "revision").unwrap().draft;
    d.backend_mut().add_fault(Fault::throw(Method::SetItem).on_key(&draft_key("new.md")).noop());

    let err = d.remap_drafts("old.md", "new.md", ItemKind::File).unwrap_err();
    assert_eq!(err.source_path.as_deref(), Some("old.md"));
    assert_eq!(d.read_draft("old.md").unwrap().draft, Some(source));
    assert_eq!(d.read_draft("new.md").unwrap().draft, None);
}

// port: test/javascript/lib/draft_storage.test.js:250 "retains both copies if removing the verified source fails"
#[test]
fn retains_both_copies_if_removing_the_verified_source_fails() {
    let mut d = drafts();
    let source = d.write_draft("old.md", "local edit", "revision").unwrap().draft;
    d.backend_mut().add_fault(Fault::throw(Method::RemoveItem).on_key(&draft_key("old.md")));

    let err = d.remap_drafts("old.md", "new.md", ItemKind::File).unwrap_err();
    assert_eq!(err.source_path.as_deref(), Some("old.md"));
    assert_eq!(d.read_draft("old.md").unwrap().draft, Some(source.clone()));
    assert_eq!(d.read_draft("new.md").unwrap().draft, Some(with_path(&source, "new.md")));
}

// port: test/javascript/lib/draft_storage.test.js:265 "removes deleted file or folder drafts and backups by path boundary"
#[test]
fn removes_deleted_file_or_folder_drafts_and_backups_by_path_boundary() {
    let mut d = drafts();
    d.write_draft("docs/index.md", "index", "rev-1").unwrap();
    d.write_draft("docs/guides/setup.md", "guide", "rev-2").unwrap();
    d.write_draft("docs-old/keep.md", "keep", "rev-3").unwrap();
    d.write_backup("docs/index.md", "legacy").unwrap();
    d.write_backup("docs-old/keep.md", "keep legacy").unwrap();

    assert_eq!(d.remove_drafts("docs", ItemKind::Folder), Ok(3));
    assert_eq!(d.read_draft("docs/index.md").unwrap().draft, None);
    assert_eq!(d.read_draft("docs/guides/setup.md").unwrap().draft, None);
    assert_eq!(d.read_draft("docs-old/keep.md").unwrap().draft.unwrap().content, "keep");
    assert_eq!(d.read_backup("docs/index.md").unwrap().backup, None);
    assert_eq!(d.read_backup("docs-old/keep.md").unwrap().backup.unwrap().content, "keep legacy");
}

// port: test/javascript/lib/draft_storage.test.js:282 "removes recovery conflict copies when their file or folder is deleted"
#[test]
fn removes_recovery_conflict_copies_when_their_file_or_folder_is_deleted() {
    let mut d = drafts();
    d.write_draft("docs/old.md", "source", "revision").unwrap();
    d.write_draft("docs/new.md", "destination", "other-revision").unwrap();
    let _ = d.remap_drafts("docs/old.md", "docs/new.md", ItemKind::File);

    assert_eq!(d.list_draft_conflicts("docs/new.md").unwrap().len(), 1);
    assert!(d.remove_drafts("docs", ItemKind::Folder).is_ok());
    assert_eq!(d.list_draft_conflicts("docs/new.md").unwrap(), vec![]);
    assert_eq!(d.read_draft("docs/old.md").unwrap().draft, None);
    assert_eq!(d.read_draft("docs/new.md").unwrap().draft, None);
}

// port: test/javascript/lib/draft_storage.test.js:296 "reports draft cleanup errors without throwing"
#[test]
fn reports_draft_cleanup_errors_without_throwing() {
    let mut d = drafts();
    d.write_draft("note.md", "local edit", "revision").unwrap();
    d.backend_mut().add_fault(Fault::throw(Method::RemoveItem));

    let err = d.remove_drafts("note.md", ItemKind::File).unwrap_err();
    assert_eq!(err.removed, Some(0));
}

// port: test/javascript/lib/draft_storage.test.js:305 "discards malformed records without accepting unrelated local storage keys"
#[test]
fn discards_malformed_records_without_accepting_unrelated_local_storage_keys() {
    let wrong = r#"{"schemaVersion":1,"path":"another.md","content":"untrusted","baseRevision":"baseline","draftRevision":"revision","updatedAt":1}"#;
    let mut d = drafts_seeded(&[
        ("frankmd:draft:broken.md", "{bad json"),
        ("frankmd:draft:wrong-path", wrong),
        ("another-app:key", "leave me"),
    ]);

    let read = d.read_draft("broken.md").unwrap();
    assert_eq!(read.draft, None);
    assert!(read.malformed);
    assert_eq!(d.list_drafts(), Ok(vec![]));
    assert_eq!(d.remove_all_drafts(), Ok(()));
    assert_eq!(d.backend().raw("another-app:key"), Some("leave me"));
    assert_eq!(d.backend().raw("frankmd:draft:broken.md"), None);
}

// port: test/javascript/lib/draft_storage.test.js:326 "preserves draft records from a newer schema it cannot read"
#[test]
fn preserves_draft_records_from_a_newer_schema_it_cannot_read() {
    let raw = r#"{"schemaVersion":2,"content":"future draft"}"#;
    let mut d = drafts_seeded(&[("frankmd:draft:note.md", raw)]);

    let err = d.read_draft("note.md").unwrap_err();

    assert!(err.message.contains("Unsupported draft schema version"), "{}", err.message);
    assert_eq!(d.backend().raw("frankmd:draft:note.md"), Some(raw));
}

// port: test/javascript/lib/draft_storage.test.js:339 "reports storage read, write, and removal failures to callers"
#[test]
fn reports_storage_read_write_and_removal_failures_to_callers() {
    let mut d = drafts();
    d.backend_mut().add_fault(Fault::throw(Method::GetItem).message("QuotaExceededError"));
    let e = d.read_draft("note.md").unwrap_err();
    assert_eq!((e.kind, e.message.as_str()), (DraftErrorKind::Storage, "QuotaExceededError"));
    let e = d.write_draft("note.md", "content", "baseline").unwrap_err();
    assert_eq!(e.message, "QuotaExceededError");

    let mut w = drafts();
    w.backend_mut().add_fault(Fault::throw(Method::SetItem).message("QuotaExceededError"));
    assert_eq!(w.write_draft("note.md", "content", "baseline").unwrap_err().message, "QuotaExceededError");

    let mut r = drafts();
    let record = r.write_draft("note.md", "content", "baseline").unwrap().draft;
    r.backend_mut().add_fault(Fault::throw(Method::RemoveItem).message("QuotaExceededError"));
    assert_eq!(r.remove_draft_if_revision("note.md", &record.draft_revision).unwrap_err().message, "QuotaExceededError");
}

// port: test/javascript/lib/draft_storage.test.js:361 "continues to read and remove legacy offline backup records"
#[test]
fn continues_to_read_and_remove_legacy_offline_backup_records() {
    let mut d = drafts();
    assert!(d.write_backup("folder/note.md", "offline").is_ok());
    assert_eq!(d.read_backup("folder/note.md").unwrap().backup.unwrap().content, "offline");
    assert_eq!(d.remove_backup("folder/note.md"), Ok(()));
    assert_eq!(d.read_backup("folder/note.md").unwrap().backup, None);
}

// ---------------------------------------------------------------------------
// test/javascript/controllers/offline_backup_controller.test.js
// ---------------------------------------------------------------------------

// port: test/javascript/controllers/offline_backup_controller.test.js:53 "writes content and timestamp to localStorage"
#[test]
fn offline_backup_save_writes_content_and_timestamp() {
    let mut d = drafts();
    d.backup_save("notes/test.md", "hello world").unwrap();
    let raw = d.backend().raw("frankmd:backup:notes/test.md").unwrap().to_owned();
    let v: frankmd_testkit::serde_json::Value = frankmd_testkit::serde_json::from_str(&raw).unwrap();
    assert_eq!(v["content"], "hello world");
    // SeqEnv pins Date.now() to 1700000000000 for the first call.
    assert_eq!(v["timestamp"].as_f64(), Some(1_700_000_000_000.0));
}

// port: test/javascript/controllers/offline_backup_controller.test.js:67 "handles localStorage quota exceeded gracefully"
#[test]
fn offline_backup_save_handles_quota_exceeded_gracefully() {
    let mut d = drafts();
    d.backend_mut().add_fault(Fault::throw(Method::SetItem).message("QuotaExceededError"));
    // Must not panic; the failure is reported (and logged as a warning by the app).
    let e = d.backup_save("test.md", "content").unwrap_err();
    assert_eq!(e.message, "QuotaExceededError");
}

// port: test/javascript/controllers/offline_backup_controller.test.js:93 "returns null when no backup exists"
#[test]
fn offline_backup_check_returns_none_when_no_backup_exists() {
    let mut d = drafts();
    assert_eq!(d.backup_check("test.md", "server content"), None);
}

// port: test/javascript/controllers/offline_backup_controller.test.js:98 "returns data when backup differs from server content"
#[test]
fn offline_backup_check_returns_data_when_backup_differs() {
    let mut d = drafts();
    d.backup_save("test.md", "local backup content").unwrap();
    let b = d.backup_check("test.md", "server content").unwrap();
    assert_eq!(b, LegacyBackup { content: "local backup content".into(), timestamp: 1_700_000_000_000.0 });
}

// port: test/javascript/controllers/offline_backup_controller.test.js:107 "returns null and auto-clears when backup matches server content"
#[test]
fn offline_backup_check_auto_clears_matching_backup() {
    let mut d = drafts();
    d.backup_save("test.md", "same content").unwrap();
    assert_eq!(d.backup_check("test.md", "same content"), None);
    assert_eq!(d.backend().raw("frankmd:backup:test.md"), None);
}

// port: test/javascript/controllers/offline_backup_controller.test.js:117 "returns null and auto-clears when backup is corrupt JSON"
#[test]
fn offline_backup_check_auto_clears_corrupt_json() {
    let mut d = drafts_seeded(&[("frankmd:backup:test.md", "not valid json{{{")]);
    assert_eq!(d.backup_check("test.md", "server content"), None);
    assert_eq!(d.backend().raw("frankmd:backup:test.md"), None);
}

// port: test/javascript/controllers/offline_backup_controller.test.js:129 "removes backup for a specific path"
#[test]
fn offline_backup_clear_removes_only_that_path() {
    let mut d = drafts();
    d.backup_save("test.md", "content").unwrap();
    d.backup_save("other.md", "other content").unwrap();
    d.remove_backup("test.md").unwrap();
    assert_eq!(d.backend().raw("frankmd:backup:test.md"), None);
    assert!(d.backend().raw("frankmd:backup:other.md").is_some());
}

// port: test/javascript/controllers/offline_backup_controller.test.js:141 "removes all frankmd backup keys"
#[test]
fn offline_backup_clear_all_removes_all_backup_keys() {
    let mut d = drafts();
    d.backup_save("test.md", "content 1").unwrap();
    d.backup_save("folder/note.md", "content 2").unwrap();
    d.backend_mut().put_raw("unrelated-key", "keep me");
    d.remove_all_backups().unwrap();
    assert_eq!(d.backend().raw("frankmd:backup:test.md"), None);
    assert_eq!(d.backend().raw("frankmd:backup:folder/note.md"), None);
    assert_eq!(d.backend().raw("unrelated-key"), Some("keep me"));
}
