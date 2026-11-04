//! Pure attributes and validation of the Rails `Note` / `Folder` models
//! (`app/models/note.rb`, `app/models/folder.rb`).
//!
//! The models' IO (find/save/destroy/rename) is `frankmd_fs::NotesService`;
//! this module holds the derived attributes and the validation rules whose
//! English messages are the ActiveModel `full_messages` the UI showed.

/// A model validation failure.
///
/// `Display` is the **[COMPAT]** English full message (`errors.full_messages`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    /// `validates :path, presence: true`.
    #[error("Path can't be blank")]
    Blank,
    /// The `..` substring rule (note.rb:182-187, folder.rb:341-346).
    #[error("Path cannot contain directory traversal")]
    Traversal,
}

/// **[COMPAT]** `Note#name`: basename without a trailing `.md`.
pub fn note_name(path: &str) -> String {
    todo!("note.rb")
}

/// **[COMPAT]** `Note#directory`: parent dir, `""` at the root.
pub fn note_directory(path: &str) -> String {
    todo!("note.rb")
}

/// **[COMPAT]** `Note#valid?` on an (already normalized) path: blank, then the
/// `..` substring rule. Returns every failing rule in Rails order.
pub fn validate_note_path(path: &str) -> Vec<ValidationError> {
    todo!("note.rb:182-187")
}

/// **[COMPAT]** `Folder#name`: last path segment.
pub fn folder_name(path: &str) -> String {
    todo!("folder.rb")
}

/// **[COMPAT]** `Folder#parent_path`: `None` at the root level.
pub fn folder_parent_path(path: &str) -> Option<String> {
    todo!("folder.rb")
}

/// **[COMPAT]** `Folder#valid?` (same rules as notes).
pub fn validate_folder_path(path: &str) -> Vec<ValidationError> {
    todo!("folder.rb:341-346")
}
