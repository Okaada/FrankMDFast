//! Port of the `test/services/hugo_service_test.rb` tests that need a real
//! `<NOTES_PATH>/.hugo_template.md` (the IO half: `NotesService::hugo_template`
//! feeding `frankmd_core::hugo::generate_post`, plus `create_hugo_post`).
//! The pure slugify / path / front-matter tests are ported in frankmd-core.
#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    missing_docs,
    unreachable_pub
)]

mod common;

use common::Sandbox;
use frankmd_core::hugo::{HugoPathStyle, NewPost, TEMPLATE_FILE, generate_post};
use frankmd_fs::NotesService;

fn at_2026_02_01_1030() -> jiff::Zoned {
    "2026-02-01T10:30:00[UTC]".parse().unwrap()
}

/// `HugoService.generate_blog_post(title)` with the vault's template.
fn generate(sb: &Sandbox, title: &str) -> String {
    let vault = sb.vault();
    let tpl = NotesService::new(&vault)
        .hugo_template()
        .map(|b| String::from_utf8(b).unwrap());
    let post = generate_post(&NewPost {
        title,
        parent: None,
        now: at_2026_02_01_1030(),
        style: HugoPathStyle::Dated,
        template: tpl.as_deref(),
    });
    post.content
}

// port: test/services/hugo_service_test.rb:144 "generate_blog_post uses .hugo_template.md when present"
#[test]
fn uses_hugo_template_when_present() {
    let sb = Sandbox::empty();
    sb.write(
        TEMPLATE_FILE,
        "---\ntitle: \"{{title}}\"\nslug: \"{{slug}}\"\ndate: {{date}}\ndraft: false\nauthor: \"Jane\"\ntags:\n  - uncategorized\n---\n",
    );
    let content = generate(&sb, "Custom Post");
    for needle in [
        "title: \"Custom Post\"",
        "slug: \"custom-post\"",
        "date: 2026-02-01T10:30:00",
        "author: \"Jane\"",
        "- uncategorized",
        "draft: false",
    ] {
        assert!(
            content.contains(needle),
            "missing {needle:?} in {content:?}"
        );
    }

    // Same through the creating entry point: the file lands with that content.
    let vault = sb.vault();
    let post = NotesService::new(&vault)
        .create_hugo_post(
            "Custom Post",
            None,
            HugoPathStyle::Dated,
            at_2026_02_01_1030(),
        )
        .unwrap();
    assert_eq!(post.path, "2026/02/01/custom-post/index.md");
    assert_eq!(sb.read(&post.path), post.content);
    assert!(post.content.contains("author: \"Jane\""));
}

// port: test/services/hugo_service_test.rb:169 "generate_blog_post escapes quotes in title within custom template"
#[test]
fn escapes_quotes_in_title_within_custom_template() {
    let sb = Sandbox::empty();
    sb.write(TEMPLATE_FILE, "title: \"{{title}}\"\n");
    let content = generate(&sb, "Say \"Hi\"");
    assert!(content.contains("title: \"Say \\\"Hi\\\"\""), "{content:?}");
}

// port: test/services/hugo_service_test.rb:177 "generate_blog_post falls back to default template when none present"
#[test]
fn falls_back_to_default_template_when_none_present() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    assert_eq!(NotesService::new(&vault).hugo_template(), None);
    let content = generate(&sb, "No Template");
    assert!(content.contains("draft: true"));
    assert!(content.contains("tags: []"));
}
