//! On-disk draft store, DocId index and startup target resolution
//! (HP-007: crash-safe drafts under the state dir, never in the vault).
#![allow(clippy::unwrap_used)]

use std::os::unix::fs::PermissionsExt;

use frankmd_core::registry::DocRegistry;
use frankmd_core::{DocId, RelPath};
use frankmd_workspace::doc_index::DocIndex;
use frankmd_workspace::draft_storage::DraftBackend;
use frankmd_workspace::drafts::{DRAFTS_FILE, FileDraftBackend, draft_subject, open_draft_storage};
use frankmd_workspace::startup::{StartupTarget, resolve_initial_note};

fn p(path: &str) -> RelPath {
    RelPath::parse(path).unwrap()
}

#[test]
fn drafts_persist_across_reopen_with_private_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let doc = DocId::new();
    let written = open_draft_storage(dir.path()).unwrap().write_draft(&draft_subject(doc), "text", "rev").unwrap();

    let mut reopened = open_draft_storage(dir.path()).unwrap();
    assert_eq!(reopened.read_draft(&draft_subject(doc)).unwrap().draft, Some(written.draft));
    let mode = std::fs::metadata(dir.path().join(DRAFTS_FILE)).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "drafts hold private text");

    reopened.remove_draft(&draft_subject(doc)).unwrap();
    assert!(open_draft_storage(dir.path()).unwrap().backend().is_empty().unwrap());
}

#[test]
fn a_corrupt_store_is_reported_and_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(DRAFTS_FILE);
    frankmd_fs::atomic::atomic_replace(&file, b"{not json", frankmd_fs::atomic::ModePolicy::Fixed(0o600)).unwrap();

    assert!(FileDraftBackend::open(dir.path()).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), b"{not json");
}

#[test]
fn a_failed_write_leaves_the_previous_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut backend = FileDraftBackend::open(dir.path()).unwrap();
    backend.set_item("k", "v1").unwrap();
    let missing = dir.path().join("gone");
    let mut orphan = FileDraftBackend::open(&missing).unwrap();

    assert!(orphan.set_item("k", "v").is_err(), "directory missing → error, not a silent drop");
    assert_eq!(orphan.len().unwrap(), 0, "memory reflects only what reached the disk");
    assert_eq!(backend.get_item("k").unwrap().as_deref(), Some("v1"));
}

#[test]
fn the_doc_index_keeps_identity_across_runs_and_follows_renames() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = DocRegistry::default();
    let doc = registry.id_for(&p("docs/a.md"));
    let mut index = DocIndex::open(dir.path());
    assert_eq!(index.record("docs/a.md", doc), None);

    let remaps = registry.rename(&p("docs"), &p("archive"));
    index.apply_remaps(&remaps);

    let reloaded = DocIndex::open(dir.path());
    assert_eq!(reloaded.get("archive/a.md"), Some(doc));
    assert_eq!(reloaded.get("docs/a.md"), None);

    let mut index = reloaded;
    let next_run = DocId::new();
    assert_eq!(index.record("archive/a.md", next_run), Some(doc), "previous id reported for draft re-keying");
    index.remove(&[next_run]);
    assert_eq!(DocIndex::open(dir.path()).get("archive/a.md"), None);
}

#[test]
fn startup_resolves_relative_and_in_vault_absolute_paths() {
    let root = std::path::Path::new("/home/u/notes");
    let exists = |path: &RelPath| path.as_str() == "blog/post.md";

    assert_eq!(resolve_initial_note(&[root], None, exists), StartupTarget::None);
    assert_eq!(resolve_initial_note(&[root], Some("blog/post"), exists), StartupTarget::Note(p("blog/post.md")));
    assert_eq!(
        resolve_initial_note(&[root], Some("/home/u/notes/blog/post.md"), exists),
        StartupTarget::Note(p("blog/post.md"))
    );
    assert_eq!(resolve_initial_note(&[root], Some("todo.md"), exists), StartupTarget::Missing(p("todo.md")));
    assert_eq!(resolve_initial_note(&[root], Some("/etc/passwd"), exists), StartupTarget::None, "outside the vault");
    assert_eq!(resolve_initial_note(&[root], Some("../escape.md"), exists), StartupTarget::None);
    // security: in-app `.fed` editing stays blocked.
    assert_eq!(resolve_initial_note(&[root], Some(".fed"), |_| true), StartupTarget::None);
}
