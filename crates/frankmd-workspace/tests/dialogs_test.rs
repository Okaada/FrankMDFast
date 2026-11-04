//! Dialog form models (ui-test-plan workspace::dialogs / inline_rename).
#![allow(clippy::unwrap_used)]

use frankmd_core::RelPath;
use frankmd_core::hugo::HugoPathStyle;
use frankmd_core::new_note::NoteTemplate;
use frankmd_workspace::Command;
use frankmd_workspace::dialogs::{
    CreationOrigin, DeleteConfirm, InlineRename, ItemKind, MoveForm, NewFolderForm, NewNoteForm,
    RenameTarget,
};

fn date() -> jiff::civil::Date {
    jiff::civil::date(2026, 3, 7)
}

fn global(sel: &str, style: HugoPathStyle) -> NewNoteForm {
    NewNoteForm::new(CreationOrigin::Global { selected_folder: sel.into() }, style)
}

fn dbg<T: std::fmt::Debug>(v: &T) -> String {
    format!("{v:?}")
}

fn p(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}

#[test]
fn empty_destination_follows_selection_or_context() {
    // ui-spec §5.13
    let mut f = global("docs", HugoPathStyle::Dated);
    assert_eq!(f.destination(), "docs");
    f.set_name("a");
    assert_eq!(f.preview(date(), "root", "name").path, "docs/a.md");
    let f = NewNoteForm::new(CreationOrigin::FolderContext("x/y".into()), HugoPathStyle::Dated);
    assert_eq!(f.destination(), "x/y");
}

#[test]
fn hugo_global_goes_to_blog_context_goes_to_folder() {
    // adapts: test/system/creation_destination_test.rb "hugo from toolbar goes to blog"
    let mut f = global("docs", HugoPathStyle::Dated);
    f.set_template(NoteTemplate::Hugo);
    assert_eq!(f.destination(), "blog");
    let mut f = global("", HugoPathStyle::Flat);
    f.set_template(NoteTemplate::Hugo);
    assert_eq!(f.destination(), "blog");
    let mut f = NewNoteForm::new(CreationOrigin::FolderContext("docs".into()), HugoPathStyle::Dated);
    f.set_template(NoteTemplate::Hugo);
    assert_eq!(f.destination(), "docs");
}

#[test]
fn preview_paths_dated_and_flat() {
    // ui-spec §5.13
    let mut f = global("", HugoPathStyle::Dated).with_name("Sample");
    f.set_template(NoteTemplate::Hugo);
    assert_eq!(f.preview(date(), "root", "name").path, "blog/2026/03/07/sample/index.md");
    let mut f = global("", HugoPathStyle::Flat).with_name("My Post");
    f.set_template(NoteTemplate::Hugo);
    assert_eq!(f.preview(date(), "root", "name").path, "blog/my-post.md");
    let mut f = NewNoteForm::new(CreationOrigin::FolderContext("docs".into()), HugoPathStyle::Flat)
        .with_name("Explicit Post");
    f.set_template(NoteTemplate::Hugo);
    assert_eq!(f.preview(date(), "root", "name").path, "docs/explicit-post.md");
}

#[test]
fn preview_root_placeholder_and_nested() {
    // ui-spec §5.13
    let f = global("", HugoPathStyle::Dated);
    let pv = f.preview(date(), "root", "note-name");
    assert_eq!(pv.path, "root/note-name.md");
    assert!(pv.empty && !pv.valid);
    let f = global("docs/nested", HugoPathStyle::Dated).with_name("Sample");
    assert_eq!(f.preview(date(), "root", "name").path, "docs/nested/Sample.md");
}

#[test]
fn slug_follows_name_until_edited() {
    // ui-spec §5.13
    let mut f = global("", HugoPathStyle::Dated);
    f.set_name("Hello World");
    assert_eq!(f.slug(), "hello-world");
    f.set_slug("custom");
    f.set_name("Other Title");
    assert_eq!(f.slug(), "custom");
    f.set_slug("");
    assert_eq!(f.slug(), "other-title");
    f.set_name("Third");
    assert_eq!(f.slug(), "third");
}

#[test]
fn slug_strips_md_suffix() {
    let f = global("", HugoPathStyle::Dated).with_name("note.md");
    assert_eq!(f.slug(), "note");
}

#[test]
fn cjk_title_needs_typed_slug() {
    // ui-spec §5.13
    let mut f = global("", HugoPathStyle::Dated).with_name("日本語");
    f.set_template(NoteTemplate::Hugo);
    assert_eq!(f.slug(), "");
    assert!(!f.can_submit());
    assert!(f.submit().is_none());
    f.set_slug("nihongo");
    assert!(f.can_submit());
    assert_eq!(f.preview(date(), "root", "name").path, "blog/2026/03/07/nihongo/index.md");
    assert!(f.preview(date(), "root", "name").valid);
    assert_eq!(dbg(&f.submit()), dbg(&Some(Command::CreateHugoPost { parent: p("blog"), title: "日本語".into(), slug: "nihongo".into() })));
}

#[test]
fn slashes_and_blank_invalid() {
    // ui-spec §5.13
    let mut f = global("", HugoPathStyle::Dated);
    assert!(!f.can_submit());
    f.set_name("a/b");
    assert!(!f.can_submit());
    assert!(f.submit().is_none());
    f.set_template(NoteTemplate::Hugo);
    f.set_slug("ok");
    assert!(!f.can_submit());
    f.set_name("  ");
    assert!(!f.can_submit());
}

#[test]
fn submit_empty_and_root() {
    // adapts: test/system/creation_destination_test.rb "new note in root"
    let f = global("", HugoPathStyle::Dated).with_name("  Idea  ");
    assert_eq!(dbg(&f.submit()), dbg(&Some(Command::CreateNote { parent: p(""), name: "Idea".into(), template: NoteTemplate::Empty })));
    let f = global("docs", HugoPathStyle::Dated).with_name("x.md");
    assert_eq!(dbg(&f.submit()), dbg(&Some(Command::CreateNote { parent: p("docs"), name: "x.md".into(), template: NoteTemplate::Empty })));
}

#[test]
fn submit_hugo_auto_slug_and_error() {
    let mut f = global("", HugoPathStyle::Dated).with_name("My Post.md");
    f.set_template(NoteTemplate::Hugo);
    assert_eq!(dbg(&f.submit()), dbg(&Some(Command::CreateHugoPost { parent: p("blog"), title: "My Post".into(), slug: "my-post".into() })));
    assert!(f.error().is_none());
    f.set_error(Some(frankmd_workspace::command::UserError::new("errors.x")));
    assert!(f.error().is_some());
    f.set_error(None);
    assert!(f.error().is_none());
}

#[test]
fn new_folder_nested() {
    // ui-spec §5.13
    let mut f = NewFolderForm::new("");
    assert!(f.subtitle_root());
    assert!(!f.can_submit());
    f.set_name("  ");
    assert!(f.submit().is_none());
    f.set_name(" a/b ");
    assert_eq!(dbg(&f.submit()), dbg(&Some(Command::CreateFolder { parent: p(""), name: "a/b".into() })));
    let f = NewFolderForm::new("docs");
    assert!(!f.subtitle_root());
}

#[test]
fn move_form_prefill_and_target() {
    // ui-spec §5.13
    let mut f = MoveForm::new("docs/a.md", ItemKind::File);
    assert_eq!((f.folder(), f.name()), ("docs", "a"));
    assert_eq!(f.target().unwrap(), "docs/a.md");
    assert!(f.submit().unwrap().is_none());
    f.set_name("b");
    assert_eq!(f.target().unwrap(), "docs/b.md");
    assert_eq!(dbg(&f.submit().unwrap()), dbg(&Some(Command::Rename { from: p("docs/a.md"), to: p("docs/b.md") })));
    f.set_folder("");
    assert_eq!(f.target().unwrap(), "b.md");
    f.set_name("sub/c.md");
    assert_eq!(f.target().unwrap(), "sub/c.md");
}

#[test]
fn move_form_blank_and_folder() {
    // ui-spec §5.13
    let mut f = MoveForm::new("docs", ItemKind::Folder);
    assert_eq!((f.folder(), f.name()), ("", "docs"));
    f.set_name("  ");
    assert_eq!(f.submit().unwrap_err().key, "errors.title_required");
    assert_eq!(f.target().unwrap_err().key, "errors.title_required");
    f.set_name("archive");
    assert_eq!(f.target().unwrap(), "archive");
    f.set_error(Some(frankmd_workspace::command::UserError::new("errors.x")));
    assert!(f.error().is_some());
    f.set_name("z");
    assert!(f.error().is_none());
}

#[test]
fn inline_rename_commit() {
    // adapts: test/system/notes_test.rb "rename note via title"
    let mut r = InlineRename::start(RenameTarget::Title, "docs/a.md", ItemKind::File).unwrap();
    assert_eq!(r.value(), "a");
    assert!(r.commit().unwrap().is_none());
    r.set_value("b");
    assert_eq!(dbg(&r.commit().unwrap()), dbg(&Some(Command::Rename { from: p("docs/a.md"), to: p("docs/b.md") })));
    r.set_value("   ");
    assert_eq!(r.commit().unwrap_err().key, "errors.title_required");
    r.set_error(Some(frankmd_workspace::command::UserError::new("errors.x")));
    r.set_value("c");
    assert!(r.error().is_none());
}

#[test]
fn inline_rename_row_folder_and_config() {
    // ui-spec §5.13
    let mut r = InlineRename::start(RenameTarget::Row, "x/dir", ItemKind::Folder).unwrap();
    assert_eq!(r.value(), "dir");
    r.set_value("new");
    assert_eq!(dbg(&r.commit().unwrap()), dbg(&Some(Command::Rename { from: p("x/dir"), to: p("x/new") })));
    assert!(InlineRename::start(RenameTarget::Row, ".fed", ItemKind::File).is_none());
}

#[test]
fn delete_message_keys() {
    // adapts: test/system/notes_test.rb "delete note"
    let d = DeleteConfirm::new(vec![("docs/a.md".into(), ItemKind::File)]).unwrap();
    assert_eq!(d.message(), ("dialogs.delete.trash_file", vec![("name", "a.md".to_owned())]));
    let d = DeleteConfirm::new(vec![("docs/sub".into(), ItemKind::Folder)]).unwrap();
    assert_eq!(d.message(), ("dialogs.delete.trash_folder", vec![("name", "sub".to_owned())]));
    let d = DeleteConfirm::new(vec![("a.md".into(), ItemKind::File), ("b.md".into(), ItemKind::File)]).unwrap();
    assert_eq!(d.message(), ("dialogs.delete.trash_multiple", vec![("count", "2".to_owned())]));
    assert!(DeleteConfirm::destructive());
    assert!(DeleteConfirm::initial_focus_cancel());
}

#[test]
fn delete_dedupes_and_drops() {
    // ui-spec §5.13
    let d = DeleteConfirm::new(vec![
        ("docs".into(), ItemKind::Folder),
        ("docs/a.md".into(), ItemKind::File),
        ("docs".into(), ItemKind::Folder),
        (".fed".into(), ItemKind::File),
        ("docsx/b.md".into(), ItemKind::File),
        ("docsx/b.md".into(), ItemKind::File),
    ])
    .unwrap();
    assert_eq!(d.items.len(), 2);
    assert_eq!(dbg(&d.commands()), dbg(&vec![Command::Delete { path: p("docs") }, Command::Delete { path: p("docsx/b.md") }]));
    assert!(DeleteConfirm::new(vec![(".fed".into(), ItemKind::File)]).is_none());
    assert!(DeleteConfirm::new(vec![]).is_none());
}
