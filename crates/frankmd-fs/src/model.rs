//! **[COMPAT]** the Rails `Note` / `Folder` model operations
//! (`app/models/note.rb`, `app/models/folder.rb`): validation, the
//! [`NotesService`] call, and the per-operation mapping of failures to the
//! user-facing error (an i18n key under `errors.*` or a raw service message,
//! attached to `:base` or `:path`).
//!
//! The UI renders [`ModelError::full_message`] with the active locale; in
//! English this reproduces ActiveModel's `errors.full_messages` (a `:path`
//! error is prefixed with `"Path "`).

use crate::{FsError, NotesService};
use frankmd_core::note::ValidationError;

/// Which attribute an error is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorField {
    /// `errors.add(:base, …)`: shown as is.
    Base,
    /// `errors.add(:path, …)`: shown as `"Path <text>"`.
    Path,
}

/// The text of a model error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorText {
    /// An i18n key (`"errors.note_already_exists"`, …).
    Key(&'static str),
    /// A raw service message (`"Invalid path: x"`, `"Folder not empty: x"`)
    /// or a fixed validation text (`"can't be blank"`).
    Raw(String),
}

/// One model error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelError {
    /// Attribute.
    pub field: ErrorField,
    /// Text.
    pub text: ErrorText,
    /// `Note#revision_conflict?` marker (`errors.add(:base, :revision_conflict)`).
    pub revision_conflict: bool,
}

impl ModelError {
    /// ActiveModel full message; `t` translates i18n keys.
    pub fn full_message(&self, t: &dyn Fn(&str) -> String) -> String {
        todo!()
    }
}

impl From<ValidationError> for ModelError {
    fn from(e: ValidationError) -> Self {
        todo!()
    }
}

/// `Note#save` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaveMode {
    /// `write` (upsert).
    #[default]
    Upsert,
    /// `existing_only: true` → `update`.
    ExistingOnly,
    /// `create_only: true` → `create`.
    CreateOnly,
}

/// Model operation, selecting the rescue table of the matching Rails method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelOp {
    /// `Note#save`.
    NoteSave,
    /// `Note#destroy`.
    NoteDestroy,
    /// `Note#rename`.
    NoteRename,
    /// `Folder#create`.
    FolderCreate,
    /// `Folder#destroy`.
    FolderDestroy,
    /// `Folder#rename`.
    FolderRename,
}

/// **[COMPAT]** the `rescue` table of `op` applied to `err`
/// (AlreadyExists → `errors.note_already_exists` / `errors.folder_already_exists`,
/// NotFound → `errors.note_not_found` / `errors.folder_not_found`,
/// RevisionConflict → `errors.note_changed_since_loaded` (+ marker),
/// other InvalidPath → raw message on `:path` or `:base`,
/// EACCES/EPERM → `errors.permission_denied`,
/// ENOENT → `errors.parent_folder_not_found` / `errors.file_no_longer_exists` /
/// `errors.folder_no_longer_exists`). Other errors are not rescued in Rails
/// and are returned as `Err(err)` by the operations below.
pub fn model_error(op: ModelOp, err: &FsError) -> Option<ModelError> {
    todo!("note.rb / folder.rb rescue clauses")
}

/// Result of a model operation: `Ok` (Rails `true`), `Err(errors)` (Rails
/// `false` + `errors`), or an unrescued failure.
pub type ModelResult<T = ()> = Result<Result<T, Vec<ModelError>>, FsError>;

/// `Note#save(path, content)`: normalize (`.md`), validate, then
/// write/update/create. `None` content saves `""`.
pub fn note_save(svc: &NotesService<'_>, path: &str, content: Option<&[u8]>, mode: SaveMode) -> ModelResult<String> {
    todo!("note.rb#save")
}

/// `Note#destroy(expected_revision:)`.
pub fn note_destroy(svc: &NotesService<'_>, path: &str, expected_revision: Option<&str>) -> ModelResult {
    todo!("note.rb#destroy")
}

/// `Note#rename(new_path)`: normalizes `new_path` (`.md` added); returns the
/// new path on success.
pub fn note_rename(svc: &NotesService<'_>, path: &str, new_path: &str) -> ModelResult<String> {
    todo!("note.rb#rename")
}

/// `Note#exists?` / `persisted?`: `false` for a blank path; `file?` of the
/// normalized path otherwise (escaping paths raise `InvalidPath`).
pub fn note_exists(svc: &NotesService<'_>, path: &str) -> Result<bool, FsError> {
    todo!("note.rb#exists?")
}

/// `Folder#create`: validate, `errors.folder_already_exists` when it exists,
/// then `create_folder`.
pub fn folder_create(svc: &NotesService<'_>, path: &str) -> ModelResult {
    todo!("folder.rb#create")
}

/// `Folder#destroy`.
pub fn folder_destroy(svc: &NotesService<'_>, path: &str) -> ModelResult {
    todo!("folder.rb#destroy")
}

/// `Folder#rename(new_path)` (+ Hugo `index.md` slug rewrite).
pub fn folder_rename(svc: &NotesService<'_>, path: &str, new_path: &str) -> ModelResult<String> {
    todo!("folder.rb#rename")
}

/// `Folder#exists?` / `persisted?`: a directory (not a file) at `path`.
pub fn folder_exists(svc: &NotesService<'_>, path: &str) -> Result<bool, FsError> {
    todo!("folder.rb#exists?")
}
