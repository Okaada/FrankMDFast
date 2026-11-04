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
    ruby_basename(path, Some(".md")).to_owned()
}

/// **[COMPAT]** `Note#directory`: parent dir, `""` at the root.
pub fn note_directory(path: &str) -> String {
    match ruby_dirname(path) {
        "." => String::new(),
        dir => dir.to_owned(),
    }
}

/// **[COMPAT]** `Note#valid?` on an (already normalized) path: blank, then the
/// `..` substring rule. Returns every failing rule in Rails order.
pub fn validate_note_path(path: &str) -> Vec<ValidationError> {
    if is_blank(path) {
        vec![ValidationError::Blank]
    } else if path.contains("..") {
        vec![ValidationError::Traversal]
    } else {
        Vec::new()
    }
}

/// **[COMPAT]** `Folder#name`: last path segment.
pub fn folder_name(path: &str) -> String {
    ruby_basename(path, None).to_owned()
}

/// **[COMPAT]** `Folder#parent_path`: `None` at the root level.
pub fn folder_parent_path(path: &str) -> Option<String> {
    match ruby_dirname(path) {
        "." => None,
        dir => Some(dir.to_owned()),
    }
}

/// **[COMPAT]** `Folder#valid?` (same rules as notes).
pub fn validate_folder_path(path: &str) -> Vec<ValidationError> {
    validate_note_path(path)
}

/// ActiveSupport `String#blank?`: empty or only (Unicode) whitespace.
fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// Ruby `File.basename(path[, suffix])` on Unix: trailing `/` are ignored, an
/// all-slash path is `"/"`, and `suffix` is removed unless it is the whole name.
pub fn ruby_basename<'a>(path: &'a str, suffix: Option<&str>) -> &'a str {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return if path.is_empty() { "" } else { "/" };
    }
    let base = trimmed.rsplit('/').next().unwrap_or(trimmed);
    match suffix.and_then(|sfx| base.strip_suffix(sfx)) {
        Some(stem) if !stem.is_empty() => stem,
        _ => base,
    }
}

/// Ruby `File.dirname(path)` on Unix: `"."` without a directory part,
/// `"/"` for paths directly under the root; repeated slashes collapse.
pub fn ruby_dirname(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return if path.is_empty() { "." } else { "/" };
    }
    match trimmed.rfind('/') {
        None => ".",
        Some(i) => {
            let dir = trimmed[..i].trim_end_matches('/');
            if dir.is_empty() { "/" } else { dir }
        }
    }
}
