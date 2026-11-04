//! GUI-free form models for the New Note, New Folder, Rename/Move, inline
//! rename and Delete dialogs (frontend inventory F.7-F.11, ui-spec 5.13).
//!
//! Labels are i18n keys; the GUI resolves them.

use frankmd_core::RelPath;
use frankmd_core::hugo::HugoPathStyle;
use frankmd_core::new_note::{
    NotePreview, PreviewRequest, can_create_note, is_valid_note_name, preview_note_path,
    preview_slugify,
};
use frankmd_core::path::{is_config_file, is_within};

use crate::command::{Command, NoteTemplate, UserError};

/// Whether an item is a note/file or a folder (shared with the explorer).
pub use crate::explorer::ItemKind;

/// Folder a Hugo post goes to when New Note is opened globally (F.7).
const HUGO_GLOBAL_FOLDER: &str = "blog";

/// Where a New Note request came from (F.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreationOrigin {
    /// Toolbar / shortcut / quick open; carries the selected folder (`""` = root).
    Global {
        /// Currently selected folder.
        selected_folder: String,
    },
    /// The folder context menu on this folder.
    FolderContext(String),
}

fn rel(path: &str) -> Option<RelPath> {
    RelPath::parse(path).ok()
}

fn auto_slug(name: &str) -> String {
    let trimmed = name.trim();
    preview_slugify(Some(trimmed.strip_suffix(".md").unwrap_or(trimmed)))
}

/// New Note dialog state.
#[derive(Debug, Clone)]
pub struct NewNoteForm {
    origin: CreationOrigin,
    style: HugoPathStyle,
    template: NoteTemplate,
    name: String,
    slug: String,
    slug_edited: bool,
    error: Option<UserError>,
}

impl NewNoteForm {
    /// Empty form: template Empty, blank name and slug.
    pub fn new(origin: CreationOrigin, style: HugoPathStyle) -> Self {
        Self {
            origin,
            style,
            template: NoteTemplate::Empty,
            name: String::new(),
            slug: String::new(),
            slug_edited: false,
            error: None,
        }
    }

    /// Prefill the name (quick open "Ctrl+N creates <query>").
    #[must_use]
    pub fn with_name(mut self, name: &str) -> Self {
        self.set_name(name);
        self
    }

    /// Current name input.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name; the slug follows it unless the user edited the slug.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
        if !self.slug_edited {
            self.slug = auto_slug(name);
        }
    }

    /// Current slug (auto-derived or user-edited).
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// Set the slug by hand. An empty value reverts to the automatic slug.
    pub fn set_slug(&mut self, slug: &str) {
        if slug.is_empty() {
            self.slug_edited = false;
            self.slug = auto_slug(&self.name);
        } else {
            self.slug_edited = true;
            self.slug = slug.to_owned();
        }
    }

    /// Selected template.
    pub fn template(&self) -> NoteTemplate {
        self.template
    }

    /// Select a template.
    pub fn set_template(&mut self, t: NoteTemplate) {
        self.template = t;
    }

    /// Destination folder (`""` = root). Hugo from the global entry points
    /// always targets `blog` (F.7).
    pub fn destination(&self) -> String {
        match (&self.origin, self.template) {
            (CreationOrigin::Global { .. }, NoteTemplate::Hugo) => HUGO_GLOBAL_FOLDER.to_owned(),
            (CreationOrigin::Global { selected_folder }, _) => selected_folder.clone(),
            (CreationOrigin::FolderContext(p), _) => p.clone(),
        }
    }

    fn effective_slug(&self) -> String {
        preview_slugify(Some(&self.slug))
    }

    /// Path preview for the dialog. `root_label` and `name_slot` are the
    /// localized root label and empty-name placeholder.
    pub fn preview(
        &self,
        date: jiff::civil::Date,
        root_label: &str,
        name_slot: &str,
    ) -> NotePreview {
        let parent = self.destination();
        let request = |name| PreviewRequest {
            name: Some(name),
            parent: &parent,
            template: self.template,
            style: self.style,
            date,
            name_placeholder: name_slot,
            root_segment: root_label,
        };
        let mut preview = preview_note_path(&request(self.name.as_str()));
        if self.template == NoteTemplate::Hugo && self.slug_edited {
            let slug = self.effective_slug();
            if !slug.is_empty() {
                preview.path = preview_note_path(&request(slug.as_str())).path;
                preview.valid = is_valid_note_name(Some(&self.name));
            }
        }
        preview
    }

    /// Whether Create is enabled. For Hugo a typed slug rescues titles whose
    /// automatic slug is empty (CJK).
    pub fn can_submit(&self) -> bool {
        match self.template {
            NoteTemplate::Empty => can_create_note(Some(&self.name), NoteTemplate::Empty),
            NoteTemplate::Hugo => {
                is_valid_note_name(Some(&self.name)) && !self.effective_slug().is_empty()
            }
        }
    }

    /// The command the Create button sends, or `None` when not submittable.
    pub fn submit(&self) -> Option<Command> {
        if !self.can_submit() {
            return None;
        }
        let parent = rel(&self.destination())?;
        let name = self.name.trim();
        Some(match self.template {
            NoteTemplate::Empty => Command::CreateNote {
                parent,
                name: name.to_owned(),
                template: NoteTemplate::Empty,
            },
            NoteTemplate::Hugo => Command::CreateHugoPost {
                parent,
                title: name.strip_suffix(".md").unwrap_or(name).to_owned(),
                slug: self.effective_slug(),
            },
        })
    }

    /// Error shown in the dialog.
    pub fn error(&self) -> Option<&UserError> {
        self.error.as_ref()
    }

    /// Set or clear the dialog error.
    pub fn set_error(&mut self, e: Option<UserError>) {
        self.error = e;
    }
}

/// New Folder dialog state.
#[derive(Debug, Clone)]
pub struct NewFolderForm {
    /// Parent folder (`""` = root).
    pub parent: String,
    name: String,
    error: Option<UserError>,
}

impl NewFolderForm {
    /// Form creating inside `parent`.
    pub fn new(parent: &str) -> Self {
        Self {
            parent: parent.to_owned(),
            name: String::new(),
            error: None,
        }
    }

    /// Set the folder name (nested `a/b` allowed).
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    /// Current name input.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether Create is enabled (trimmed non-empty).
    pub fn can_submit(&self) -> bool {
        !self.name.trim().is_empty()
    }

    /// The `CreateFolder` command, if submittable.
    pub fn submit(&self) -> Option<Command> {
        if !self.can_submit() {
            return None;
        }
        Some(Command::CreateFolder {
            parent: rel(&self.parent)?,
            name: self.name.trim().to_owned(),
        })
    }

    /// `true` when the parent is the root: show `dialogs.new_item.root_location`.
    pub fn subtitle_root(&self) -> bool {
        self.parent.is_empty()
    }

    /// Error shown in the dialog.
    pub fn error(&self) -> Option<&UserError> {
        self.error.as_ref()
    }

    /// Set or clear the dialog error.
    pub fn set_error(&mut self, e: Option<UserError>) {
        self.error = e;
    }
}

fn split_parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else {
        format!("{folder}/{name}")
    }
}

fn display_name(name: &str, kind: ItemKind) -> String {
    match kind {
        ItemKind::File => name.strip_suffix(".md").unwrap_or(name).to_owned(),
        ItemKind::Folder => name.to_owned(),
    }
}

/// Builds `folder/name` (appending `.md` to files) and a `Rename` command.
fn rename_command(
    from: &str,
    kind: ItemKind,
    folder: &str,
    name: &str,
) -> Result<Option<Command>, UserError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(UserError::new("errors.title_required"));
    }
    let leaf = if kind == ItemKind::File && !name.ends_with(".md") {
        format!("{name}.md")
    } else {
        name.to_owned()
    };
    let to = join(folder, &leaf);
    if to == from {
        return Ok(None);
    }
    let invalid = || UserError::new("errors.invalid_path");
    Ok(Some(Command::Rename {
        from: RelPath::parse(from).map_err(|_| invalid())?,
        to: RelPath::parse(&to).map_err(|_| invalid())?,
    }))
}

/// Rename/Move dialog: folder picker plus name field (ui-spec 5.13, F.10).
#[derive(Debug, Clone)]
pub struct MoveForm {
    from: String,
    kind: ItemKind,
    /// Destination folder (`""` = root).
    pub folder: String,
    name: String,
    error: Option<UserError>,
}

impl MoveForm {
    /// Prefilled from the current location; files show their name without `.md`.
    pub fn new(from: &str, kind: ItemKind) -> Self {
        let (folder, leaf) = split_parent(from);
        Self {
            from: from.to_owned(),
            kind,
            folder: folder.to_owned(),
            name: display_name(leaf, kind),
            error: None,
        }
    }

    /// Current name input.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name (clears the error).
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
        self.error = None;
    }

    /// Chosen destination folder.
    pub fn folder(&self) -> &str {
        &self.folder
    }

    /// Choose the destination folder (clears the error).
    pub fn set_folder(&mut self, folder: &str) {
        self.folder = folder.to_owned();
        self.error = None;
    }

    /// The resulting path. A name containing `/` moves into sub-paths (F.10).
    pub fn target(&self) -> Result<String, UserError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(UserError::new("errors.title_required"));
        }
        let leaf = if self.kind == ItemKind::File && !name.ends_with(".md") {
            format!("{name}.md")
        } else {
            name.to_owned()
        };
        Ok(join(&self.folder, &leaf))
    }

    /// `Ok(None)` when the target equals the source (no-op success).
    pub fn submit(&self) -> Result<Option<Command>, UserError> {
        rename_command(&self.from, self.kind, &self.folder, &self.name)
    }

    /// Error shown in the dialog.
    pub fn error(&self) -> Option<&UserError> {
        self.error.as_ref()
    }

    /// Set or clear the dialog error.
    pub fn set_error(&mut self, e: Option<UserError>) {
        self.error = e;
    }
}

/// Where an inline rename is happening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameTarget {
    /// The pane title.
    Title,
    /// An explorer row.
    Row,
}

/// Inline rename editor state (HP-030).
#[derive(Debug, Clone)]
pub struct InlineRename {
    /// Which surface hosts the editor.
    pub target: RenameTarget,
    /// Path being renamed.
    pub path: String,
    kind: ItemKind,
    value: String,
    error: Option<UserError>,
}

impl InlineRename {
    /// Begin editing; `None` for the config file, which cannot be renamed.
    pub fn start(target: RenameTarget, path: &str, kind: ItemKind) -> Option<Self> {
        if is_config_file(path) {
            return None;
        }
        let (_, leaf) = split_parent(path);
        Some(Self {
            target,
            path: path.to_owned(),
            kind,
            value: display_name(leaf, kind),
            error: None,
        })
    }

    /// Current text.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Replace the text (clears the error).
    pub fn set_value(&mut self, v: &str) {
        self.value = v.to_owned();
        self.error = None;
    }

    /// Error shown next to the editor.
    pub fn error(&self) -> Option<&UserError> {
        self.error.as_ref()
    }

    /// Set or clear the error.
    pub fn set_error(&mut self, e: Option<UserError>) {
        self.error = e;
    }

    /// Enter: `Ok(None)` when unchanged, `Err` for a blank name.
    pub fn commit(&self) -> Result<Option<Command>, UserError> {
        let (folder, _) = split_parent(&self.path);
        rename_command(&self.path, self.kind, folder, &self.value)
    }
}

/// Delete confirmation (F.11): items go to the Trash.
#[derive(Debug, Clone)]
pub struct DeleteConfirm {
    /// Captured targets, in selection order.
    pub items: Vec<(String, ItemKind)>,
}

impl DeleteConfirm {
    /// Drops config files, duplicates and descendants of other selected
    /// folders; `None` when nothing is left.
    pub fn new(items: Vec<(String, ItemKind)>) -> Option<Self> {
        let folders: Vec<&str> = items
            .iter()
            .filter(|(_, k)| *k == ItemKind::Folder)
            .map(|(p, _)| p.as_str())
            .collect();
        let mut kept: Vec<(String, ItemKind)> = Vec::new();
        for (path, kind) in &items {
            let covered = folders.iter().any(|f| *f != path && is_within(path, f));
            if is_config_file(path) || covered || kept.iter().any(|(p, _)| p == path) {
                continue;
            }
            kept.push((path.clone(), *kind));
        }
        if kept.is_empty() {
            None
        } else {
            Some(Self { items: kept })
        }
    }

    /// Message i18n key and its interpolation args.
    pub fn message(&self) -> (&'static str, Vec<(&'static str, String)>) {
        match self.items.as_slice() {
            [(path, kind)] => {
                let key = match kind {
                    ItemKind::File => "dialogs.delete.trash_file",
                    ItemKind::Folder => "dialogs.delete.trash_folder",
                };
                (key, vec![("name", split_parent(path).1.to_owned())])
            }
            many => (
                "dialogs.delete.trash_multiple",
                vec![("count", many.len().to_string())],
            ),
        }
    }

    /// One `Delete` per item, in order. The executor must stop at the first
    /// error and not run the remaining commands.
    pub fn commands(&self) -> Vec<Command> {
        self.items
            .iter()
            .filter_map(|(p, _)| rel(p).map(|path| Command::Delete { path }))
            .collect()
    }

    /// Deletion is destructive (red primary button).
    pub const fn destructive() -> bool {
        true
    }

    /// Focus starts on Cancel.
    pub const fn initial_focus_cancel() -> bool {
        true
    }
}
