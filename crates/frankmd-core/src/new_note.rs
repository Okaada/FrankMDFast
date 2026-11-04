//! The New Note dialog's path preview and gating
//! (`app/javascript/lib/new_note_utils.js`). Golden: `js_new_note_utils.json`.
//!
//! The preview must agree with what creation will do ([`crate::hugo`],
//! [`crate::path::normalize_note_path`]).

/// Template picked in the New Note dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoteTemplate {
    /// Plain markdown note (`"empty"`).
    #[default]
    Empty,
    /// Hugo page bundle (`"hugo"`).
    Hugo,
}

impl NoteTemplate {
    /// **[COMPAT]** JS compares `template === "hugo"`; anything else is Empty.
    pub fn from_js(value: &str) -> Self {
        if value == "hugo" { Self::Hugo } else { Self::Empty }
    }
}

/// Options for [`preview_note_path`] (JS defaults in field docs).
#[derive(Debug, Clone)]
pub struct PreviewRequest<'a> {
    /// Raw name input (`None` ≙ JS null/undefined).
    pub name: Option<&'a str>,
    /// Parent folder (`""` = root).
    pub parent: &'a str,
    /// Template.
    pub template: NoteTemplate,
    /// Path style (`"flat"` vs default dated).
    pub style: crate::hugo::HugoPathStyle,
    /// Date for the Hugo segment.
    pub date: jiff::civil::Date,
    /// Localized slot shown while the name is empty (default `"name"`).
    pub name_placeholder: &'a str,
    /// Localized label for the root (default `"root"`).
    pub root_segment: &'a str,
}

/// Result of [`preview_note_path`] (`{valid, empty, path}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotePreview {
    /// The name can be used to create a note.
    pub valid: bool,
    /// The name is blank (the path shows the placeholder slot).
    pub empty: bool,
    /// Path to display.
    pub path: String,
}

/// **[COMPAT]** JS `slugify` (`toLowerCase`, then the same ACCENT_MAP). Kept
/// separate from [`crate::slug::slugify`] because JS and Ruby lowercase a few
/// code points differently; the golden fixtures pin both.
pub fn preview_slugify(text: Option<&str>) -> String {
    todo!("new_note_utils.js")
}

/// **[COMPAT]** `hugoDatePath(date)`: `YYYY/MM/DD`, zero-padded.
pub fn hugo_date_path(date: jiff::civil::Date) -> String {
    todo!()
}

/// **[COMPAT]** `isValidNoteName`: trimmed non-empty, no `/`, not `.`/`..`.
pub fn is_valid_note_name(name: Option<&str>) -> bool {
    todo!()
}

/// **[COMPAT]** `canCreateNote(name, template)`: a valid name and, for Hugo,
/// a non-empty slug of the name without a trailing `.md`.
pub fn can_create_note(name: Option<&str>, template: NoteTemplate) -> bool {
    todo!()
}

/// **[COMPAT]** `previewNotePath({...})`.
pub fn preview_note_path(req: &PreviewRequest<'_>) -> NotePreview {
    todo!()
}
