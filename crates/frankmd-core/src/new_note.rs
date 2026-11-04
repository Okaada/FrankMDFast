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
        if value == "hugo" {
            Self::Hugo
        } else {
            Self::Empty
        }
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
    let mut mapped = String::new();
    for c in text.unwrap_or("").to_lowercase().chars() {
        match crate::slug::accent(c) {
            Some(ascii) => mapped.push_str(ascii),
            None => mapped.push(c),
        }
    }
    crate::slug::collapse_to_slug(&mapped)
}

/// JS `String.prototype.trim`: strips ECMAScript whitespace and line
/// terminators (which, unlike Rust's `trim`, include U+FEFF but not U+0085).
fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{9}'
            | '\u{a}'
            | '\u{b}'
            | '\u{c}'
            | '\u{d}'
            | ' '
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    ) || ('\u{2000}'..='\u{200a}').contains(&c)
}

/// The Hugo slug of a trimmed name: JS `slugify(trimmed.replace(/\.md$/, ""))`.
fn hugo_slug(trimmed: &str) -> String {
    preview_slugify(Some(trimmed.strip_suffix(".md").unwrap_or(trimmed)))
}

/// **[COMPAT]** `hugoDatePath(date)`: `YYYY/MM/DD`, zero-padded.
pub fn hugo_date_path(date: jiff::civil::Date) -> String {
    format!("{:04}/{:02}/{:02}", date.year(), date.month(), date.day())
}

/// **[COMPAT]** `isValidNoteName`: trimmed non-empty, no `/`, not `.`/`..`.
pub fn is_valid_note_name(name: Option<&str>) -> bool {
    let trimmed = js_trim(name.unwrap_or(""));
    !trimmed.is_empty() && !trimmed.contains('/') && trimmed != "." && trimmed != ".."
}

/// **[COMPAT]** `canCreateNote(name, template)`: a valid name and, for Hugo,
/// a non-empty slug of the name without a trailing `.md`.
pub fn can_create_note(name: Option<&str>, template: NoteTemplate) -> bool {
    let trimmed = js_trim(name.unwrap_or(""));
    if !is_valid_note_name(Some(trimmed)) {
        return false;
    }
    match template {
        NoteTemplate::Hugo => !hugo_slug(trimmed).is_empty(),
        NoteTemplate::Empty => true,
    }
}

/// **[COMPAT]** `previewNotePath({...})`.
pub fn preview_note_path(req: &PreviewRequest<'_>) -> NotePreview {
    let trimmed = js_trim(req.name.unwrap_or(""));
    let empty = trimmed.is_empty();
    let place = if req.parent.is_empty() {
        req.root_segment
    } else {
        req.parent
    };

    if req.template == NoteTemplate::Hugo {
        let slug = hugo_slug(trimmed);
        let slot = if !slug.is_empty() {
            slug.as_str()
        } else if !trimmed.is_empty() {
            trimmed
        } else {
            req.name_placeholder
        };
        let path = match req.style {
            crate::hugo::HugoPathStyle::Flat => format!("{place}/{slot}.md"),
            crate::hugo::HugoPathStyle::Dated => {
                format!("{place}/{}/{slot}/index.md", hugo_date_path(req.date))
            }
        };
        return NotePreview {
            valid: can_create_note(Some(trimmed), NoteTemplate::Hugo),
            empty,
            path,
        };
    }

    let file_name = if empty {
        format!("{}.md", req.name_placeholder)
    } else if trimmed.ends_with(".md") {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.md")
    };
    NotePreview {
        valid: can_create_note(Some(trimmed), NoteTemplate::Empty),
        empty,
        path: format!("{place}/{file_name}"),
    }
}
