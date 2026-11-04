//! Round-4 file operations through the store: delete to the Trash (non-empty
//! folders included), Hugo posts with an edited slug, recents recorded on
//! open, the Recovery dialog's "Keep both", and the global settings scope.
#![allow(clippy::unwrap_used)]

use frankmd_config::store::ConfigStore;
use frankmd_config::{EnvSnapshot, Key, Provenance, Value};
use frankmd_core::{FakeClock, RelPath};
use frankmd_fs::service::NotesService;
use frankmd_fs::{TrashBackend, Vault};
use frankmd_workspace::autosave::{RecoveryRequest, RecoverySource};
use frankmd_workspace::doc_index::DocIndex;
use frankmd_workspace::drafts::{FileDraftBackend, open_draft_storage};
use frankmd_workspace::{Command, Event, Workspace};

struct Fixture {
    _dir: tempfile::TempDir,
    _state_dir: tempfile::TempDir,
    root: std::path::PathBuf,
    state: std::path::PathBuf,
    clock: FakeClock,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        let state_dir = tempfile::tempdir().unwrap();
        let state = state_dir.path().to_path_buf();
        let f = Self {
            root,
            state,
            _dir: dir,
            _state_dir: state_dir,
            clock: FakeClock::default(),
        };
        f.vault();
        f
    }
    fn vault(&self) -> Vault {
        Vault::open(&self.root, &self.state).unwrap()
    }
    fn write(&self, path: &str, text: &str) {
        NotesService::new(&self.vault())
            .write(path, text.as_bytes())
            .unwrap();
    }
    fn read(&self, path: &str) -> Option<String> {
        NotesService::new(&self.vault())
            .read(path)
            .ok()
            .map(|f| String::from_utf8(f.bytes).unwrap())
    }
    fn trash(&self) -> std::path::PathBuf {
        self.state.join("trash")
    }
    fn workspace(&self) -> Workspace<FileDraftBackend> {
        let vault = self
            .vault()
            .with_trash(TrashBackend::Directory(self.trash()));
        let config = ConfigStore::open(vault.clone(), EnvSnapshot::default())
            .with_global(self.state.join("config.toml"));
        let drafts = open_draft_storage(&self.state).unwrap();
        Workspace::new(
            vault,
            config,
            drafts,
            DocIndex::open(&self.state),
            Box::new(self.clock.clone()),
        )
    }
}

fn p(path: &str) -> RelPath {
    RelPath::parse(path).unwrap()
}

fn errors(events: &[Event]) -> Vec<&'static str> {
    events
        .iter()
        .filter_map(|e| {
            if let Event::Error(err) = e {
                Some(err.key)
            } else {
                None
            }
        })
        .collect()
}

// ui-spec §5.13: delete moves to the Trash, so a non-empty folder can be deleted
// (FE-267 fixed: FrankMD refused "Folder not empty" despite its confirm text).
#[test]
fn deleting_a_non_empty_folder_moves_it_to_the_trash() {
    let f = Fixture::new();
    f.write("docs/a.md", "a");
    f.write("docs/sub/b.md", "b");
    let mut ws = f.workspace();
    let (events, _) = ws.handle(Command::Delete { path: p("docs") });
    assert!(errors(&events).is_empty(), "{events:?}");
    assert_eq!(f.read("docs/a.md"), None);
    assert!(
        f.trash().join("docs").join("sub").join("b.md").is_file(),
        "the folder is in the trash"
    );
}

// adapts: test/models/note_test.rb "destroy" — a note goes to the Trash, not unlinked.
#[test]
fn deleting_a_note_keeps_a_copy_in_the_trash() {
    let f = Fixture::new();
    f.write("gone.md", "keep me");
    let mut ws = f.workspace();
    ws.handle(Command::Delete { path: p("gone.md") });
    assert_eq!(f.read("gone.md"), None);
    assert_eq!(
        std::fs::read_to_string(f.trash().join("gone.md")).unwrap(),
        "keep me"
    );
}

// ui-spec §5.13: the New note dialog's editable slug (CJK titles slugify to empty).
#[test]
fn hugo_post_uses_the_edited_slug() {
    let f = Fixture::new();
    let mut ws = f.workspace();
    let (events, _) = ws.handle(Command::CreateHugoPost {
        parent: p("blog"),
        title: "日本語の記事".into(),
        slug: "japanese-post".into(),
    });
    assert!(errors(&events).is_empty(), "{events:?}");
    let path = ws.open_doc().unwrap().path.as_str().to_owned();
    assert!(
        path.starts_with("blog/") && path.ends_with("/japanese-post/index.md"),
        "{path}"
    );
    let text = f.read(&path).unwrap();
    assert!(text.contains("title: \"日本語の記事\""), "{text}");
    assert!(text.contains("slug: \"japanese-post\""), "{text}");
}

#[test]
fn hugo_post_blank_slug_falls_back_to_the_title() {
    let f = Fixture::new();
    let mut ws = f.workspace();
    ws.handle(Command::UpdateSettings {
        changes: vec![(Key::HugoPathStyle, Some(Value::Str("flat".into())))],
    });
    ws.handle(Command::CreateHugoPost {
        parent: RelPath::root(),
        title: "My Post".into(),
        slug: " ".into(),
    });
    assert_eq!(ws.open_doc().unwrap().path.as_str(), "my-post.md");
}

// FE-298…301: the shared recents are recorded by the workspace on every open.
#[test]
fn opening_notes_records_shared_recents_newest_first() {
    let f = Fixture::new();
    f.write("a.md", "a");
    f.write("b.md", "b");
    let mut ws = f.workspace();
    ws.handle(Command::OpenNote { path: p("a.md") });
    f.clock.advance_ms(60_000);
    ws.handle(Command::OpenNote { path: p("b.md") });
    assert_eq!(
        ws.recent_paths(),
        vec!["b.md".to_owned(), "a.md".to_owned()]
    );
    assert!(ws.clear_recents());
    assert!(ws.recent_paths().is_empty());
}

#[test]
fn renaming_a_note_remaps_its_recent_entry() {
    let f = Fixture::new();
    f.write("a.md", "a");
    let mut ws = f.workspace();
    ws.handle(Command::OpenNote { path: p("a.md") });
    ws.handle(Command::Rename {
        from: p("a.md"),
        to: p("z.md"),
    });
    assert_eq!(ws.recent_paths(), vec!["z.md".to_owned()]);
}

// ui-spec §5.11 "Keep both": the draft is written next to the note, the file wins.
#[test]
fn keep_both_writes_the_draft_as_a_sibling_copy() {
    let f = Fixture::new();
    f.write("blog/Brief.md", "disk text");
    f.write("blog/Brief (draft 2026-11-04 14-02).md", "older copy");
    let mut ws = f.workspace();
    let request = RecoveryRequest {
        path: "blog/Brief.md".into(),
        server_content: "disk text".into(),
        backup_content: "draft text".into(),
        // 2026-11-04T14:02:00Z; FakeClock's zone renders the name.
        backup_timestamp: 1_793_800_920_000.0,
        source: RecoverySource::Draft,
        draft_revision: None,
        conflict_id: None,
    };
    let (events, _) = ws.handle(Command::RecoveryKeepBoth(request));
    assert!(errors(&events).is_empty(), "{events:?}");
    let revealed = events
        .iter()
        .find_map(|e| {
            if let Event::Reveal { path } = e {
                Some(path.as_str().to_owned())
            } else {
                None
            }
        })
        .expect("copy revealed");
    assert!(revealed.starts_with("blog/Brief (draft "), "{revealed}");
    assert_ne!(
        revealed, "blog/Brief (draft 2026-11-04 14-02).md",
        "an existing file is never overwritten"
    );
    assert_eq!(f.read(&revealed).as_deref(), Some("draft text"));
    assert_eq!(f.read("blog/Brief.md").as_deref(), Some("disk text"));
}

// ui-spec §5.16 scope toggle: "All vaults" writes the global file; `.fed` still wins.
#[test]
fn global_settings_apply_below_the_vault_file() {
    let f = Fixture::new();
    let mut ws = f.workspace();
    let (events, _) = ws.handle(Command::UpdateGlobalSettings {
        changes: vec![(Key::EditorFontSize, Some(Value::Int(18)))],
    });
    assert!(events.iter().any(|e| matches!(e, Event::SettingsChanged)));
    let config = ws.snapshot().config;
    assert_eq!(config.get(Key::EditorFontSize).source, Provenance::Global);
    assert_eq!(config.value(Key::EditorFontSize), Some(Value::Int(18)));
    assert!(
        std::fs::read_to_string(f.state.join("config.toml"))
            .unwrap()
            .contains("editor_font_size")
    );
    ws.handle(Command::UpdateSettings {
        changes: vec![(Key::EditorFontSize, Some(Value::Int(12)))],
    });
    assert_eq!(
        ws.snapshot().config.value(Key::EditorFontSize),
        Some(Value::Int(12))
    );
    assert!(
        !std::fs::read_to_string(f.root.join(".fed"))
            .unwrap_or_default()
            .contains("18")
    );
}
