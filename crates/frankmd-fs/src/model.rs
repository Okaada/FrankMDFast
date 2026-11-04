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
use frankmd_core::note::{ValidationError, validate_folder_path, validate_note_path};
use frankmd_core::path::normalize_note_path;

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
        let text = match &self.text {
            ErrorText::Key(key) => t(key),
            ErrorText::Raw(raw) => raw.clone(),
        };
        match self.field {
            ErrorField::Base => text,
            ErrorField::Path => format!("Path {text}"),
        }
    }

    fn base(key: &'static str) -> Self {
        Self {
            field: ErrorField::Base,
            text: ErrorText::Key(key),
            revision_conflict: false,
        }
    }

    fn path(key: &'static str) -> Self {
        Self {
            field: ErrorField::Path,
            text: ErrorText::Key(key),
            revision_conflict: false,
        }
    }

    fn raw(field: ErrorField, message: &str) -> Self {
        Self {
            field,
            text: ErrorText::Raw(message.to_owned()),
            revision_conflict: false,
        }
    }
}

impl From<ValidationError> for ModelError {
    fn from(e: ValidationError) -> Self {
        // `Display` is the full message ("Path can't be blank"); the
        // attribute text is what follows the "Path " prefix.
        let text = match e {
            ValidationError::Blank => "can't be blank",
            ValidationError::Traversal => "cannot contain directory traversal",
        };
        Self::raw(ErrorField::Path, text)
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
    use ModelOp::*;
    // Rescue clauses in Rails source order. AlreadyExists and NotEmpty are
    // InvalidPathError subclasses, so an InvalidPath clause catches them
    // when no AlreadyExists clause comes first.
    let invalid_message = match err {
        FsError::AlreadyExists(m) | FsError::NotEmpty(m) | FsError::InvalidPath(m) => {
            Some(m.as_str())
        }
        _ => None,
    };
    let rescued = match (op, err) {
        (NoteSave, FsError::AlreadyExists(_)) => ModelError::base("errors.note_already_exists"),
        (NoteRename, FsError::AlreadyExists(_)) => ModelError::path("errors.note_already_exists"),
        (FolderCreate | FolderRename, FsError::AlreadyExists(_)) => {
            ModelError::base("errors.folder_already_exists")
        }
        (NoteDestroy, FsError::RevisionConflict(_)) => ModelError {
            revision_conflict: true,
            ..ModelError::base("errors.note_changed_since_loaded")
        },
        (NoteSave | NoteDestroy | NoteRename, FsError::NotFound(_)) => {
            ModelError::base("errors.note_not_found")
        }
        (FolderDestroy | FolderRename, FsError::NotFound(_)) => {
            ModelError::base("errors.folder_not_found")
        }
        (NoteSave | NoteRename | FolderCreate | FolderRename, _) if invalid_message.is_some() => {
            ModelError::raw(ErrorField::Path, invalid_message.unwrap_or_default())
        }
        (NoteDestroy | FolderDestroy, _) if invalid_message.is_some() => {
            ModelError::raw(ErrorField::Base, invalid_message.unwrap_or_default())
        }
        (_, e) if e.is_permission_denied() => ModelError::base("errors.permission_denied"),
        (NoteSave | FolderCreate, e) if e.is_enoent() => {
            ModelError::base("errors.parent_folder_not_found")
        }
        (NoteDestroy | NoteRename, e) if e.is_enoent() => {
            ModelError::base("errors.file_no_longer_exists")
        }
        (FolderDestroy | FolderRename, e) if e.is_enoent() => {
            ModelError::base("errors.folder_no_longer_exists")
        }
        _ => return None,
    };
    Some(rescued)
}

/// Run a service call under the rescue table of `op`.
fn rescue<T>(op: ModelOp, result: Result<T, FsError>) -> ModelResult<T> {
    match result {
        Ok(value) => Ok(Ok(value)),
        Err(err) => match model_error(op, &err) {
            Some(rescued) => Ok(Err(vec![rescued])),
            None => Err(err),
        },
    }
}

/// `valid?` errors as model errors, `None` when the path is valid.
fn invalid(errors: Vec<ValidationError>) -> Option<Vec<ModelError>> {
    (!errors.is_empty()).then(|| errors.into_iter().map(ModelError::from).collect())
}

/// Result of a model operation: `Ok` (Rails `true`), `Err(errors)` (Rails
/// `false` + `errors`), or an unrescued failure.
pub type ModelResult<T = ()> = Result<Result<T, Vec<ModelError>>, FsError>;

/// `Note#save(path, content)`: normalize (`.md`), validate, then
/// write/update/create. `None` content saves `""`.
pub fn note_save(
    svc: &NotesService<'_>,
    path: &str,
    content: Option<&[u8]>,
    mode: SaveMode,
) -> ModelResult<String> {
    if let Some(errors) = invalid(validate_note_path(path)) {
        return Ok(Err(errors));
    }
    let normalized = normalize_note_path(path);
    let content = content.unwrap_or_default();
    let saved = match mode {
        SaveMode::Upsert => svc.write(&normalized, content),
        SaveMode::ExistingOnly => svc.update(&normalized, content),
        SaveMode::CreateOnly => svc.create(&normalized, content),
    };
    rescue(ModelOp::NoteSave, saved.map(|_| normalized))
}

/// `Note#destroy(expected_revision:)`.
pub fn note_destroy(
    svc: &NotesService<'_>,
    path: &str,
    expected_revision: Option<&str>,
) -> ModelResult {
    rescue(
        ModelOp::NoteDestroy,
        svc.delete(&normalize_note_path(path), expected_revision),
    )
}

/// `Note#rename(new_path)`: normalizes `new_path` (`.md` added); returns the
/// new path on success.
pub fn note_rename(svc: &NotesService<'_>, path: &str, new_path: &str) -> ModelResult<String> {
    let new_path = normalize_note_path(new_path);
    let renamed = svc.rename(&normalize_note_path(path), &new_path);
    rescue(ModelOp::NoteRename, renamed.map(|()| new_path))
}

/// `Note#exists?` / `persisted?`: `false` for a blank path; `file?` of the
/// normalized path otherwise (escaping paths raise `InvalidPath`).
pub fn note_exists(svc: &NotesService<'_>, path: &str) -> Result<bool, FsError> {
    if is_blank(path) {
        return Ok(false);
    }
    svc.is_file(&normalize_note_path(path))
}

/// `Folder#create`: validate, `errors.folder_already_exists` when it exists,
/// then `create_folder`.
pub fn folder_create(svc: &NotesService<'_>, path: &str) -> ModelResult {
    if let Some(errors) = invalid(validate_folder_path(path)) {
        return Ok(Err(errors));
    }
    let created = folder_exists(svc, path).and_then(|exists| {
        if exists {
            return Err(FsError::AlreadyExists(String::new()));
        }
        svc.create_folder(path)
    });
    rescue(ModelOp::FolderCreate, created)
}

/// `Folder#destroy`.
pub fn folder_destroy(svc: &NotesService<'_>, path: &str) -> ModelResult {
    rescue(ModelOp::FolderDestroy, svc.delete_folder(path))
}

/// `Folder#rename(new_path)` (+ Hugo `index.md` slug rewrite).
pub fn folder_rename(svc: &NotesService<'_>, path: &str, new_path: &str) -> ModelResult<String> {
    rescue(
        ModelOp::FolderRename,
        svc.rename_folder(path, new_path)
            .map(|()| new_path.to_owned()),
    )
}

/// `Folder#exists?` / `persisted?`: a directory (not a file) at `path`.
pub fn folder_exists(svc: &NotesService<'_>, path: &str) -> Result<bool, FsError> {
    if is_blank(path) {
        return Ok(false);
    }
    svc.is_dir(path)
}

/// Ruby `blank?` on a string: empty or whitespace only.
fn is_blank(path: &str) -> bool {
    path.trim().is_empty()
}
