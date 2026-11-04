//! Hugo blog-post generation (`app/services/hugo_service.rb`).
//! Golden: `tests/fixtures/golden/hugo.json`.

/// **[COMPAT]** custom template file at the vault root.
pub const TEMPLATE_FILE: &str = ".hugo_template.md";

/// **[COMPAT]** `HugoService::DEFAULT_TEMPLATE` (note the blank line after `---`).
pub const DEFAULT_TEMPLATE: &str =
    "---\ntitle: \"{{title}}\"\nslug: \"{{slug}}\"\ndate: {{date}}\ndraft: true\ntags: []\n---\n\n";

/// `hugo_path_style` config value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HugoPathStyle {
    /// `YYYY/MM/DD/slug/index.md` (default; any unknown value maps here).
    #[default]
    Dated,
    /// `slug.md` in the current folder.
    Flat,
}

impl HugoPathStyle {
    /// **[COMPAT]** `"flat"` → Flat, anything else → Dated.
    pub fn from_config(value: Option<&str>) -> Self {
        match value {
            Some("flat") => Self::Flat,
            _ => Self::Dated,
        }
    }
}

/// Inputs for a new post.
#[derive(Debug, Clone)]
pub struct NewPost<'a> {
    /// Post title as typed by the user.
    pub title: &'a str,
    /// Parent folder as given (`None`/`""` = vault root; resolved by the
    /// workspace, HP-014). Joined verbatim as `<parent>/<relative>`.
    pub parent: Option<&'a str>,
    /// Creation instant in the zone the date is rendered in.
    /// **[COMPAT]** Rails used `Time.current` in the (unset → UTC) Rails zone;
    /// the caller decides which zone to pass.
    pub now: jiff::Zoned,
    /// Path style.
    pub style: HugoPathStyle,
    /// Contents of `.hugo_template.md` if present at the vault root
    /// (`None` → [`DEFAULT_TEMPLATE`]; `Some("")` is an empty template).
    pub template: Option<&'a str>,
}

/// Generated post: where it goes and what it contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedPost {
    /// Destination path exactly as Rails returned it. **[QUIRK]** an empty slug
    /// yields `YYYY/MM/DD//index.md`; [`crate::RelPath::parse`] cleans it on
    /// creation.
    pub path: String,
    /// Full file contents including front matter.
    pub content: String,
}

/// Build the path and front matter for a new Hugo post.
pub fn generate_post(input: &NewPost<'_>) -> GeneratedPost {
    todo!("backend.md §4.2-4.3")
}

/// **[COMPAT]** `strftime("%Y-%m-%dT%H:%M:%S%:z")` (e.g. `2026-02-01T10:30:00+00:00`).
pub fn format_date(now: &jiff::Zoned) -> String {
    todo!("backend.md §4.3")
}

/// **[COMPAT]** `strftime("%Y/%m/%d")` of the same instant.
pub fn date_path(now: &jiff::Zoned) -> String {
    todo!("backend.md §4.2")
}

/// **[COMPAT]** `HugoService.render_template`: literal replacement of every
/// `{{title}}` (with `"` → `\"`, backslashes untouched), `{{slug}}` and `{{date}}`.
pub fn render_template(template: &str, title: &str, slug: &str, date: &str) -> String {
    todo!("backend.md §4.3")
}

/// **[COMPAT]** rewrite the `slug:` of `index.md` after a folder rename.
/// Returns `None` when the content has no front matter, no slug line, or the
/// slug is already `new_slug`.
pub fn update_frontmatter_slug(content: &str, new_slug: &str) -> Option<String> {
    todo!("backend.md §4.4")
}
