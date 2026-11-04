//! Port of `test/javascript/lib/new_note_utils.test.js` (New Note dialog preview).
#![allow(clippy::unwrap_used, missing_docs)]

use frankmd_core::hugo::HugoPathStyle;
use frankmd_core::new_note::{
    NotePreview, NoteTemplate, PreviewRequest, can_create_note, hugo_date_path, is_valid_note_name,
    preview_note_path, preview_slugify as slugify,
};

const T: &str = "test/javascript/lib/new_note_utils.test.js";

/// `new Date(2026, 7, 18)` (local = UTC in the fixtures).
fn date() -> jiff::civil::Date {
    jiff::civil::date(2026, 8, 18)
}

/// `previewNotePath({name, template, date})` with JS defaults for the rest.
fn req(name: &str, template: NoteTemplate) -> PreviewRequest<'_> {
    PreviewRequest {
        name: Some(name),
        parent: "",
        template,
        style: HugoPathStyle::Dated,
        date: date(),
        name_placeholder: "name",
        root_segment: "root",
    }
}

fn sl(s: &str) -> String {
    slugify(Some(s))
}

// --- slugify ---

// port: test/javascript/lib/new_note_utils.test.js:16 "lowercases and hyphenates spaces"
#[test]
fn slugify_lowercases_and_hyphenates_spaces() {
    assert_eq!(sl("My First Post"), "my-first-post", "{T}");
}

// port: test/javascript/lib/new_note_utils.test.js:20 "collapses runs of non-alphanumeric characters into one hyphen"
#[test]
fn slugify_collapses_runs_of_non_alphanumeric() {
    assert_eq!(sl("Hello, World! (v2)"), "hello-world-v2");
}

// port: test/javascript/lib/new_note_utils.test.js:24 "strips leading and trailing hyphens"
#[test]
fn slugify_strips_leading_and_trailing_hyphens() {
    assert_eq!(sl("--hello--"), "hello");
}

// port: test/javascript/lib/new_note_utils.test.js:28 "maps accented characters to ASCII (matching HugoService)"
#[test]
fn slugify_maps_accented_characters_to_ascii() {
    assert_eq!(sl("Olá, você está bem?"), "ola-voce-esta-bem");
    assert_eq!(sl("Grüße aus München"), "grusse-aus-munchen");
}

// port: test/javascript/lib/new_note_utils.test.js:33 "returns an empty string when nothing survives"
#[test]
fn slugify_returns_empty_when_nothing_survives() {
    assert_eq!(sl("???"), "");
    assert_eq!(sl(""), "");
}

// port: test/javascript/lib/new_note_utils.test.js:38 "strips a trailing .md before slugifying is handled by caller"
#[test]
fn slugify_does_not_know_about_extensions() {
    assert_eq!(sl("my-post.md"), "my-post-md");
}

// port: test/javascript/lib/new_note_utils.test.js:44 "previews flat Hugo posts within their explicit parent"
#[test]
fn previews_flat_hugo_posts_within_their_explicit_parent() {
    let r = PreviewRequest {
        parent: "blog",
        style: HugoPathStyle::Flat,
        ..req("My Post", NoteTemplate::Hugo)
    };
    assert_eq!(preview_note_path(&r).path, "blog/my-post.md");
}

// --- hugoDatePath ---

// port: test/javascript/lib/new_note_utils.test.js:49 "formats YYYY/MM/DD with zero-padding"
#[test]
fn hugo_date_path_formats_with_zero_padding() {
    assert_eq!(hugo_date_path(jiff::civil::date(2026, 8, 18)), "2026/08/18");
    assert_eq!(hugo_date_path(jiff::civil::date(2026, 1, 3)), "2026/01/03");
}

// port: test/javascript/lib/new_note_utils.test.js:54 "defaults to the current date when none is given"
#[test]
fn hugo_date_path_of_today_matches_shape() {
    let p = hugo_date_path(jiff::Zoned::now().date());
    let parts: Vec<&str> = p.split('/').collect();
    assert_eq!(
        parts.iter().map(|s| s.len()).collect::<Vec<_>>(),
        vec![4, 2, 2],
        "{p}"
    );
    assert!(p.chars().all(|c| c.is_ascii_digit() || c == '/'), "{p}");
}

// --- isValidNoteName ---

// port: test/javascript/lib/new_note_utils.test.js:60 "accepts a plain name"
#[test]
fn is_valid_note_name_accepts_a_plain_name() {
    assert!(is_valid_note_name(Some("my-note")));
    assert!(is_valid_note_name(Some("my note.md")));
}

// port: test/javascript/lib/new_note_utils.test.js:65 "rejects empty or whitespace-only names"
#[test]
fn is_valid_note_name_rejects_empty_or_whitespace() {
    assert!(!is_valid_note_name(Some("")));
    assert!(!is_valid_note_name(Some("   ")));
    assert!(!is_valid_note_name(None));
}

// port: test/javascript/lib/new_note_utils.test.js:71 "rejects names containing slashes"
#[test]
fn is_valid_note_name_rejects_slashes() {
    assert!(!is_valid_note_name(Some("nested/path")));
    assert!(!is_valid_note_name(Some("/absolute")));
    assert!(!is_valid_note_name(Some("trailing/")));
}

// port: test/javascript/lib/new_note_utils.test.js:77 "rejects dot-only names"
#[test]
fn is_valid_note_name_rejects_dot_only_names() {
    assert!(!is_valid_note_name(Some(".")));
    assert!(!is_valid_note_name(Some("..")));
}

// --- canCreateNote ---

// port: test/javascript/lib/new_note_utils.test.js:84 "allows valid names for the empty template"
#[test]
fn can_create_note_allows_valid_names_for_empty_template() {
    assert!(can_create_note(Some("my note"), NoteTemplate::Empty));
}

// port: test/javascript/lib/new_note_utils.test.js:88 "blocks invalid names"
#[test]
fn can_create_note_blocks_invalid_names() {
    assert!(!can_create_note(Some(""), NoteTemplate::Empty));
    assert!(!can_create_note(Some("a/b"), NoteTemplate::Empty));
}

// port: test/javascript/lib/new_note_utils.test.js:93 "blocks hugo names whose slug would be empty"
#[test]
fn can_create_note_blocks_hugo_names_with_empty_slug() {
    assert!(!can_create_note(Some("!!!"), NoteTemplate::Hugo));
}

// port: test/javascript/lib/new_note_utils.test.js:97 "allows hugo names that slugify to something"
#[test]
fn can_create_note_allows_hugo_names_that_slugify() {
    assert!(can_create_note(Some("My Post.md"), NoteTemplate::Hugo));
}

// --- previewNotePath ---

// port: test/javascript/lib/new_note_utils.test.js:103 "previews an empty-document note at the root"
#[test]
fn preview_empty_document_note_at_root() {
    let r = preview_note_path(&req("my-note", NoteTemplate::Empty));
    assert_eq!(
        r,
        NotePreview {
            valid: true,
            empty: false,
            path: "root/my-note.md".into()
        }
    );
}

// port: test/javascript/lib/new_note_utils.test.js:108 "appends .md only once"
#[test]
fn preview_appends_md_only_once() {
    assert_eq!(
        preview_note_path(&req("note.md", NoteTemplate::Empty)).path,
        "root/note.md"
    );
}

// port: test/javascript/lib/new_note_utils.test.js:113 "shows the parent folder when one is set"
#[test]
fn preview_shows_the_parent_folder() {
    let r = PreviewRequest {
        parent: "docs/guides",
        ..req("my-note", NoteTemplate::Empty)
    };
    assert_eq!(preview_note_path(&r).path, "docs/guides/my-note.md");
}

// port: test/javascript/lib/new_note_utils.test.js:118 "previews a dated hugo page bundle for today"
#[test]
fn preview_dated_hugo_page_bundle() {
    let r = preview_note_path(&req("My Post", NoteTemplate::Hugo));
    assert_eq!(r.path, "root/2026/08/18/my-post/index.md");
    assert!(r.valid);
}

// port: test/javascript/lib/new_note_utils.test.js:124 "slugifies and strips .md for hugo previews"
#[test]
fn preview_slugifies_and_strips_md_for_hugo() {
    assert_eq!(
        preview_note_path(&req("Olá Mundo.md", NoteTemplate::Hugo)).path,
        "root/2026/08/18/ola-mundo/index.md"
    );
}

// port: test/javascript/lib/new_note_utils.test.js:129 "previews hugo posts inside a parent folder"
#[test]
fn preview_hugo_posts_inside_a_parent_folder() {
    let r = PreviewRequest {
        parent: "blog",
        ..req("My Post", NoteTemplate::Hugo)
    };
    assert_eq!(
        preview_note_path(&r).path,
        "blog/2026/08/18/my-post/index.md"
    );
}

// port: test/javascript/lib/new_note_utils.test.js:134 "uses the localized root segment"
#[test]
fn preview_uses_the_localized_root_segment() {
    let r = PreviewRequest {
        root_segment: "raiz",
        ..req("x", NoteTemplate::Empty)
    };
    assert_eq!(preview_note_path(&r).path, "raiz/x.md");
}

// port: test/javascript/lib/new_note_utils.test.js:139 "shows the placeholder slot and flags empty while the name is blank"
#[test]
fn preview_placeholder_slot_while_name_is_blank() {
    let r = PreviewRequest {
        name_placeholder: "nome-da-nota",
        ..req("", NoteTemplate::Empty)
    };
    assert_eq!(
        preview_note_path(&r),
        NotePreview {
            valid: false,
            empty: true,
            path: "root/nome-da-nota.md".into()
        }
    );
}

// port: test/javascript/lib/new_note_utils.test.js:144 "flags slash-containing names as invalid but still shows the typed path"
#[test]
fn preview_flags_slash_names_invalid_but_shows_path() {
    let r = preview_note_path(&req("a/b", NoteTemplate::Empty));
    assert!(!r.valid);
    assert!(!r.empty);
    assert_eq!(r.path, "root/a/b.md");
}

// port: test/javascript/lib/new_note_utils.test.js:151 "flags hugo names with empty slugs as invalid"
#[test]
fn preview_flags_hugo_names_with_empty_slugs_invalid() {
    let r = preview_note_path(&req("!!!", NoteTemplate::Hugo));
    assert!(!r.valid);
    assert!(!r.empty);
    assert_eq!(r.path, "root/2026/08/18/!!!/index.md");
}

// port: test/javascript/lib/new_note_utils.test.js:158 "uses the placeholder slot for a blank hugo name"
#[test]
fn preview_placeholder_slot_for_blank_hugo_name() {
    let r = PreviewRequest {
        name_placeholder: "note-name",
        ..req("  ", NoteTemplate::Hugo)
    };
    let r = preview_note_path(&r);
    assert!(r.empty);
    assert_eq!(r.path, "root/2026/08/18/note-name/index.md");
}

// port: test/javascript/lib/new_note_utils.test.js:164 "trims whitespace from the name"
#[test]
fn preview_trims_whitespace_from_the_name() {
    assert_eq!(
        preview_note_path(&req("  my-note  ", NoteTemplate::Empty)).path,
        "root/my-note.md"
    );
}
