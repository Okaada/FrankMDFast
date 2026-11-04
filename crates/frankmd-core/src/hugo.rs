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
    let slug = crate::slug::slugify(input.title);
    let relative = match input.style {
        HugoPathStyle::Flat => format!("{slug}.md"),
        HugoPathStyle::Dated => format!("{}/{slug}/index.md", date_path(&input.now)),
    };
    // Ruby `parent.present?`: nil, "" and whitespace-only are absent.
    let path = match input.parent {
        Some(parent) if !parent.trim().is_empty() => format!("{parent}/{relative}"),
        _ => relative,
    };
    let template = input.template.unwrap_or(DEFAULT_TEMPLATE);
    let content = render_template(template, input.title, &slug, &format_date(&input.now));
    GeneratedPost { path, content }
}

/// **[COMPAT]** `strftime("%Y-%m-%dT%H:%M:%S%:z")` (e.g. `2026-02-01T10:30:00+00:00`).
pub fn format_date(now: &jiff::Zoned) -> String {
    now.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// **[COMPAT]** `strftime("%Y/%m/%d")` of the same instant.
pub fn date_path(now: &jiff::Zoned) -> String {
    now.strftime("%Y/%m/%d").to_string()
}

/// **[COMPAT]** `HugoService.render_template`: literal replacement of every
/// `{{title}}` (with `"` → `\"`, backslashes untouched), `{{slug}}` and `{{date}}`.
pub fn render_template(template: &str, title: &str, slug: &str, date: &str) -> String {
    let escaped_title = title.replace('"', "\\\"");
    template
        .replace("{{title}}", &escaped_title)
        .replace("{{slug}}", slug)
        .replace("{{date}}", date)
}

/// **[COMPAT]** rewrite the `slug:` of `index.md` after a folder rename.
/// Returns `None` when the content has no front matter, no slug line, or the
/// slug is already `new_slug`.
pub fn update_frontmatter_slug(content: &str, new_slug: &str) -> Option<String> {
    if !content.starts_with("---") {
        return None;
    }
    let parts = ruby_split(&patterns().fence, content, 3);
    let [_, frontmatter, body] = parts.as_slice() else {
        return None;
    };
    if !frontmatter.contains("slug:") {
        return None;
    }
    let replacement = format!("slug: \"{new_slug}\"");
    let p = patterns();
    let mut updated = ruby_gsub(&p.double_quoted, frontmatter, &replacement);
    updated = ruby_gsub(&p.single_quoted, &updated, &replacement);
    updated = ruby_gsub(&p.bare, &updated, &replacement);
    if updated == *frontmatter {
        return None;
    }
    Some(format!("---{updated}---{body}"))
}

/// The front-matter regexes, in Ruby multiline-anchor form (`^`/`$` are
/// line anchors in Ruby; Ruby `\s` is ASCII-only and spans newlines).
struct Patterns {
    fence: regex::Regex,
    double_quoted: regex::Regex,
    single_quoted: regex::Regex,
    bare: regex::Regex,
}

fn patterns() -> &'static Patterns {
    static PATTERNS: std::sync::OnceLock<Patterns> = std::sync::OnceLock::new();
    PATTERNS.get_or_init(|| {
        // The patterns are literals; failing to compile would be a programming error.
        let re = |src: &str| {
            regex::Regex::new(src).unwrap_or_else(|e| panic!("invalid built-in regex {src}: {e}"))
        };
        Patterns {
            fence: re(r"(?m)^---[ \t\r\n\f\v]*$"),
            double_quoted: re(r#"(?m)^slug:[ \t\r\n\f\v]*"[^"]*""#),
            single_quoted: re(r"(?m)^slug:[ \t\r\n\f\v]*'[^']*'"),
            bare: re(r"(?m)^slug:[ \t\r\n\f\v]*[^ \t\r\n\f\v]+"),
        }
    })
}

/// Ruby `String#split(regex, limit)` for a positive `limit` (trailing empty
/// fields are kept).
fn ruby_split<'a>(re: &regex::Regex, text: &'a str, limit: usize) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut start = 0;
    for m in re.find_iter(text) {
        if parts.len() + 1 >= limit {
            break;
        }
        parts.push(&text[start..m.start()]);
        start = m.end();
    }
    parts.push(&text[start..]);
    parts
}

/// Ruby `String#gsub(regex, replacement_string)`, including the backslash
/// escapes Ruby expands in a replacement string (`\\0`, `\\&`, `\\1`…, `` \\` ``,
/// `\\'`, `\\\\`).
fn ruby_gsub(re: &regex::Regex, text: &str, replacement: &str) -> String {
    re.replace_all(text, |caps: &regex::Captures<'_>| {
        expand_ruby_replacement(caps, text, replacement)
    })
    .into_owned()
}

fn expand_ruby_replacement(caps: &regex::Captures<'_>, text: &str, replacement: &str) -> String {
    let whole = caps.get(0).map_or(0..0, |m| m.range());
    let mut out = String::with_capacity(replacement.len());
    let mut chars = replacement.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some(d @ '0'..='9') => {
                chars.next();
                let index = d as usize - '0' as usize;
                out.push_str(caps.get(index).map_or("", |m| m.as_str()));
            }
            Some('&') => {
                chars.next();
                out.push_str(&text[whole.clone()]);
            }
            Some('`') => {
                chars.next();
                out.push_str(&text[..whole.start]);
            }
            Some('\'') => {
                chars.next();
                out.push_str(&text[whole.end..]);
            }
            Some('\\') => {
                chars.next();
                out.push('\\');
            }
            _ => out.push('\\'),
        }
    }
    out
}
