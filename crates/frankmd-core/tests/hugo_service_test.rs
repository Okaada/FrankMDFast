//! Port of `test/services/hugo_service_test.rb` (pure parts: slugify,
//! generate_blog_post, update_frontmatter_slug).
//!
//! Rails resolved the path style from `Config.new.get("hugo_path_style")` and
//! the template from `NOTES_PATH/.hugo_template.md`; here both are inputs
//! (`HugoPathStyle::from_config`, `NewPost::template`).
#![allow(clippy::unwrap_used, missing_docs)]

use frankmd_core::hugo::{self, GeneratedPost, HugoPathStyle, NewPost, generate_post, update_frontmatter_slug};
use frankmd_core::slug::slugify;

/// A UTC "now" (Rails `Time.current` with the unset → UTC Rails zone).
fn utc(y: i16, mo: i8, d: i8, h: i8, mi: i8) -> jiff::Zoned {
    jiff::civil::date(y, mo, d).at(h, mi, 0, 0).in_tz("UTC").unwrap()
}

fn now() -> jiff::Zoned {
    utc(2026, 11, 4, 15, 30)
}

fn post(title: &str, parent: Option<&str>, style: HugoPathStyle, template: Option<&str>, now: jiff::Zoned) -> GeneratedPost {
    generate_post(&NewPost { title, parent, now, style, template })
}

fn default_post(title: &str) -> GeneratedPost {
    post(title, None, HugoPathStyle::default(), None, now())
}

/// `%r{\d{4}/\d{2}/\d{2}/<slug>/index\.md}` (unanchored).
fn assert_dated(path: &str, prefix: &str, slug: &str) {
    let rest = path.strip_prefix(prefix).unwrap_or_else(|| panic!("{path} lacks prefix {prefix}"));
    let parts: Vec<&str> = rest.split('/').collect();
    assert_eq!(parts.len(), 5, "{path}");
    assert!(parts[0].len() == 4 && parts[0].chars().all(|c| c.is_ascii_digit()), "{path}");
    assert!(parts[1].len() == 2 && parts[1].chars().all(|c| c.is_ascii_digit()), "{path}");
    assert!(parts[2].len() == 2 && parts[2].chars().all(|c| c.is_ascii_digit()), "{path}");
    assert_eq!(parts[3], slug, "{path}");
    assert_eq!(parts[4], "index.md", "{path}");
}

// === slugify ===

// port: test/services/hugo_service_test.rb:16 "slugify converts text to lowercase"
#[test]
fn slugify_converts_text_to_lowercase() {
    assert_eq!(slugify("Hello World"), "hello-world");
}

// port: test/services/hugo_service_test.rb:20 "slugify replaces spaces with hyphens"
#[test]
fn slugify_replaces_spaces_with_hyphens() {
    assert_eq!(slugify("hello world"), "hello-world");
}

// port: test/services/hugo_service_test.rb:24 "slugify handles accented characters"
#[test]
fn slugify_handles_accented_characters() {
    assert_eq!(slugify("café açaí"), "cafe-acai");
}

// port: test/services/hugo_service_test.rb:28 "slugify handles special characters"
#[test]
fn slugify_handles_special_characters() {
    assert_eq!(slugify("hello! @world#"), "hello-world");
}

// port: test/services/hugo_service_test.rb:32 "slugify collapses multiple hyphens"
#[test]
fn slugify_collapses_multiple_hyphens() {
    assert_eq!(slugify("hello---world"), "hello-world");
}

// port: test/services/hugo_service_test.rb:36 "slugify removes leading and trailing hyphens"
#[test]
fn slugify_removes_leading_and_trailing_hyphens() {
    assert_eq!(slugify("--hello-world--"), "hello-world");
}

// port: test/services/hugo_service_test.rb:40 "slugify handles Portuguese characters"
#[test]
fn slugify_handles_portuguese_characters() {
    assert_eq!(
        slugify("Vibe Code: Fiz um Editor de Markdown do zero com Claude Code (FrankMD)"),
        "vibe-code-fiz-um-editor-de-markdown-do-zero-com-claude-code-frankmd"
    );
}

// === generate_blog_post ===

// port: test/services/hugo_service_test.rb:47 "generate_blog_post returns path with date structure"
#[test]
fn generate_blog_post_returns_path_with_date_structure() {
    assert_dated(&default_post("My Blog Post").path, "", "my-blog-post");
}

// port: test/services/hugo_service_test.rb:54 "generate_blog_post returns content with frontmatter"
#[test]
fn generate_blog_post_returns_content_with_frontmatter() {
    let c = default_post("My Blog Post").content;
    assert!(c.starts_with("---"));
    assert!(c.contains(r#"title: "My Blog Post""#));
    assert!(c.contains(r#"slug: "my-blog-post""#));
    assert!(c.contains("draft: true"));
}

// port: test/services/hugo_service_test.rb:63 "generate_blog_post uses current date"
#[test]
fn generate_blog_post_uses_current_date() {
    let r = post("Test Post", None, HugoPathStyle::Dated, None, utc(2026, 2, 1, 10, 30));
    assert!(r.path.starts_with("2026/02/01/"), "{}", r.path);
    assert!(r.content.contains("date: 2026-02-01T10:30:00"), "{}", r.content);
}

// port: test/services/hugo_service_test.rb:72 "generate_blog_post prepends parent path when provided"
#[test]
fn generate_blog_post_prepends_parent_path_when_provided() {
    let r = post("My Post", Some("content/posts"), HugoPathStyle::Dated, None, now());
    assert!(r.path.starts_with("content/posts/"));
    assert_dated(&r.path, "content/posts/", "my-post");
}

// port: test/services/hugo_service_test.rb:79 "generate_blog_post with flat path style returns slug.md"
#[test]
fn generate_blog_post_with_flat_path_style_returns_slug_md() {
    assert_eq!(post("My Blog Post", None, HugoPathStyle::Flat, None, now()).path, "my-blog-post.md");
}

// port: test/services/hugo_service_test.rb:85 "generate_blog_post with flat path style prepends parent path"
#[test]
fn generate_blog_post_with_flat_path_style_prepends_parent_path() {
    let r = post("My Post", Some("content/posts"), HugoPathStyle::Flat, None, now());
    assert_eq!(r.path, "content/posts/my-post.md");
}

// port: test/services/hugo_service_test.rb:91 "generate_blog_post with flat path style includes frontmatter"
#[test]
fn generate_blog_post_with_flat_path_style_includes_frontmatter() {
    let c = post("My Post", None, HugoPathStyle::Flat, None, now()).content;
    assert!(c.starts_with("---"));
    assert!(c.contains(r#"title: "My Post""#));
    assert!(c.contains(r#"slug: "my-post""#));
    assert!(c.contains("draft: true"));
}

// port: test/services/hugo_service_test.rb:100 "generate_blog_post with dated path style returns date structure"
#[test]
fn generate_blog_post_with_dated_path_style_returns_date_structure() {
    let r = post("My Post", None, HugoPathStyle::Dated, None, now());
    assert_dated(&r.path, "", "my-post");
}

// port: test/services/hugo_service_test.rb:106 "generate_blog_post reads path_style from config"
#[test]
fn generate_blog_post_reads_path_style_from_config() {
    let style = HugoPathStyle::from_config(Some("flat"));
    assert_eq!(post("Config Test", None, style, None, now()).path, "config-test.md");
}

// port: test/services/hugo_service_test.rb:117 "generate_blog_post defaults to dated when config not set"
#[test]
fn generate_blog_post_defaults_to_dated_when_config_not_set() {
    let style = HugoPathStyle::from_config(None);
    assert_eq!(style, HugoPathStyle::Dated);
    assert_dated(&post("Config Test", None, style, None, now()).path, "", "config-test");
}

// port: test/services/hugo_service_test.rb:128 "generate_blog_post escapes quotes in title"
#[test]
fn generate_blog_post_escapes_quotes_in_title() {
    let c = default_post(r#"Say "Hello" World"#).content;
    assert!(c.contains(r#"title: "Say \"Hello\" World""#), "{c}");
}

// port: test/services/hugo_service_test.rb:134 "generate_blog_post emits a valid empty tags list, not a bare hyphen"
#[test]
fn generate_blog_post_emits_a_valid_empty_tags_list_not_a_bare_hyphen() {
    let c = default_post("My Post").content;
    assert!(c.contains("tags: []"));
    // A bare `-` under `tags:` creates a null list entry that breaks Hugo (issue #69):
    // refute /tags:\s*\n\s*-\s*\n/.
    let lines: Vec<&str> = c.lines().collect();
    for w in lines.windows(2) {
        let bare_tags = w[0].trim_start().starts_with("tags:") && w[0].trim_end().ends_with(':');
        assert!(!(bare_tags && w[1].trim() == "-"), "bare hyphen list item in {c:?}");
    }
}

// === custom template (.hugo_template.md) ===

// port: test/services/hugo_service_test.rb:144 "generate_blog_post uses .hugo_template.md when present"
#[test]
fn generate_blog_post_uses_hugo_template_md_when_present() {
    assert_eq!(hugo::TEMPLATE_FILE, ".hugo_template.md");
    let tpl = "---\ntitle: \"{{title}}\"\nslug: \"{{slug}}\"\ndate: {{date}}\ndraft: false\nauthor: \"Jane\"\ntags:\n  - uncategorized\n---\n";
    let c = post("Custom Post", None, HugoPathStyle::Dated, Some(tpl), utc(2026, 2, 1, 10, 30)).content;
    assert!(c.contains(r#"title: "Custom Post""#));
    assert!(c.contains(r#"slug: "custom-post""#));
    assert!(c.contains("date: 2026-02-01T10:30:00"));
    assert!(c.contains(r#"author: "Jane""#));
    assert!(c.contains("- uncategorized"));
    assert!(c.contains("draft: false"));
}

// port: test/services/hugo_service_test.rb:169 "generate_blog_post escapes quotes in title within custom template"
#[test]
fn generate_blog_post_escapes_quotes_in_title_within_custom_template() {
    let c = post(r#"Say "Hi""#, None, HugoPathStyle::Dated, Some("title: \"{{title}}\"\n"), now()).content;
    assert!(c.contains(r#"title: "Say \"Hi\"""#), "{c}");
}

// port: test/services/hugo_service_test.rb:177 "generate_blog_post falls back to default template when none present"
#[test]
fn generate_blog_post_falls_back_to_default_template_when_none_present() {
    let c = default_post("No Template").content;
    assert!(c.contains("draft: true"));
    assert!(c.contains("tags: []"));
}

// === update_frontmatter_slug ===

// port: test/services/hugo_service_test.rb:186 "update_frontmatter_slug updates slug in frontmatter"
#[test]
fn update_frontmatter_slug_updates_slug_in_frontmatter() {
    let content = "---\ntitle: \"Test\"\nslug: \"old-slug\"\ndate: 2026-02-01\n---\n\nContent\n";
    let r = update_frontmatter_slug(content, "new-slug").unwrap();
    assert!(r.contains(r#"slug: "new-slug""#));
    assert!(!r.contains(r#"slug: "old-slug""#));
    assert!(r.contains("Content"));
}

// port: test/services/hugo_service_test.rb:204 "update_frontmatter_slug returns nil for content without frontmatter"
#[test]
fn update_frontmatter_slug_returns_nil_for_content_without_frontmatter() {
    assert_eq!(update_frontmatter_slug("Just some regular content", "new-slug"), None);
}

// port: test/services/hugo_service_test.rb:211 "update_frontmatter_slug returns nil for frontmatter without slug"
#[test]
fn update_frontmatter_slug_returns_nil_for_frontmatter_without_slug() {
    let content = "---\ntitle: \"Test\"\ndate: 2026-02-01\n---\n\nContent\n";
    assert_eq!(update_frontmatter_slug(content, "new-slug"), None);
}

// port: test/services/hugo_service_test.rb:226 "update_frontmatter_slug returns nil when slug unchanged"
#[test]
fn update_frontmatter_slug_returns_nil_when_slug_unchanged() {
    let content = "---\ntitle: \"Test\"\nslug: \"same-slug\"\n---\n\nContent\n";
    assert_eq!(update_frontmatter_slug(content, "same-slug"), None);
}

// port: test/services/hugo_service_test.rb:241 "update_frontmatter_slug handles unquoted slugs"
#[test]
fn update_frontmatter_slug_handles_unquoted_slugs() {
    let content = "---\ntitle: \"Test\"\nslug: old-slug\n---\n\nContent\n";
    assert!(update_frontmatter_slug(content, "new-slug").unwrap().contains(r#"slug: "new-slug""#));
}

// port: test/services/hugo_service_test.rb:256 "update_frontmatter_slug handles single-quoted slugs"
#[test]
fn update_frontmatter_slug_handles_single_quoted_slugs() {
    let content = "---\ntitle: \"Test\"\nslug: 'old-slug'\n---\n\nContent\n";
    assert!(update_frontmatter_slug(content, "new-slug").unwrap().contains(r#"slug: "new-slug""#));
}
