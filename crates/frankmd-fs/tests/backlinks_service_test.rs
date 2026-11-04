//! Port of `test/services/backlinks_service_test.rb` (13 tests) onto
//! `NotesService::backlinks`.
#![allow(clippy::unwrap_used)]

mod common;

use common::Sandbox;
use frankmd_fs::NotesService;

fn backlinks(sb: &Sandbox, target: &str, ctx: usize) -> Vec<frankmd_core::wikilink::Backlink> {
    let vault = sb.vault();
    NotesService::new(&vault).backlinks(target, ctx).unwrap()
}

// port: test/services/backlinks_service_test.rb:15 "returns empty array when no backlinks exist"
#[test]
fn returns_empty_array_when_no_backlinks_exist() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target note");
    sb.write("other.md", "# Unrelated note");
    assert!(backlinks(&sb, "target.md", 3).is_empty());
}

// port: test/services/backlinks_service_test.rb:23 "finds wikilink backlinks by name"
#[test]
fn finds_wikilink_backlinks_by_name() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target note");
    sb.write("source.md", "Links to [[target]] here");
    let r = backlinks(&sb, "target.md", 3);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].path, "source.md");
    assert!(r[0].match_text.contains("[[target]]"));
}

// port: test/services/backlinks_service_test.rb:33 "finds wikilink backlinks with .md extension"
#[test]
fn finds_wikilink_backlinks_with_md_extension() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target");
    sb.write("source.md", "Links to [[target.md]] here");
    assert_eq!(backlinks(&sb, "target.md", 3).len(), 1);
}

// port: test/services/backlinks_service_test.rb:41 "finds wikilink backlinks case-insensitively"
#[test]
fn finds_wikilink_backlinks_case_insensitively() {
    let sb = Sandbox::empty();
    sb.write("Target.md", "# Target");
    sb.write("source.md", "Links to [[target]] here");
    assert_eq!(backlinks(&sb, "Target.md", 3).len(), 1);
}

// port: test/services/backlinks_service_test.rb:49 "finds wikilink backlinks by full path"
#[test]
fn finds_wikilink_backlinks_by_full_path() {
    let sb = Sandbox::empty();
    sb.mkdir("projects");
    sb.write("projects/myproject.md", "# My Project");
    sb.write("index.md", "See [[projects/myproject]] for details");
    let r = backlinks(&sb, "projects/myproject.md", 3);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].path, "index.md");
}

// port: test/services/backlinks_service_test.rb:59 "finds standard markdown link backlinks"
#[test]
fn finds_standard_markdown_link_backlinks() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target");
    sb.write("source.md", "Check [this link](target.md) out");
    let r = backlinks(&sb, "target.md", 3);
    assert_eq!(r.len(), 1);
    assert!(r[0].match_text.contains("[this link](target.md)"));
}

// port: test/services/backlinks_service_test.rb:68 "finds markdown link backlinks with path"
#[test]
fn finds_markdown_link_backlinks_with_path() {
    let sb = Sandbox::empty();
    sb.mkdir("docs");
    sb.write("docs/guide.md", "# Guide");
    sb.write("readme.md", "See [the guide](docs/guide.md)");
    assert_eq!(backlinks(&sb, "docs/guide.md", 3).len(), 1);
}

// port: test/services/backlinks_service_test.rb:77 "skips the target file itself"
#[test]
fn skips_the_target_file_itself() {
    let sb = Sandbox::empty();
    sb.write("self.md", "I link to [[self]] in my own content");
    assert!(backlinks(&sb, "self.md", 3).is_empty());
}

// port: test/services/backlinks_service_test.rb:84 "finds multiple backlinks from different files"
#[test]
fn finds_multiple_backlinks_from_different_files() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target");
    sb.write("source1.md", "See [[target]]");
    sb.write("source2.md", "Also [[target]]");
    sb.write("source3.md", "No link here");
    let r = backlinks(&sb, "target.md", 3);
    assert_eq!(r.len(), 2);
    let mut paths: Vec<_> = r.iter().map(|b| b.path.as_str()).collect();
    paths.sort_unstable();
    assert_eq!(paths, ["source1.md", "source2.md"]);
}

// port: test/services/backlinks_service_test.rb:96 "finds multiple backlinks within same file"
#[test]
fn finds_multiple_backlinks_within_same_file() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target");
    sb.write("source.md", "First [[target]] and\nlater [[target]] again");
    let r = backlinks(&sb, "target.md", 3);
    assert_eq!(r.len(), 2);
    assert_eq!(r.iter().map(|b| b.line_number).collect::<Vec<_>>(), [1, 2]);
}

// port: test/services/backlinks_service_test.rb:105 "returns context lines around matches"
#[test]
fn returns_context_lines_around_matches() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target");
    sb.write("source.md", "Line 1\nLine 2\nLink to [[target]] here\nLine 4\nLine 5");
    let r = backlinks(&sb, "target.md", 1);
    assert_eq!(r.len(), 1);
    let ctx = &r[0].context;
    assert_eq!(ctx.len(), 3);
    assert_eq!(ctx[0].content, "Line 2");
    assert!(ctx[1].is_match);
    assert_eq!(ctx[2].content, "Line 4");
}

// port: test/services/backlinks_service_test.rb:120 "returns correct result structure"
#[test]
fn returns_correct_result_structure() {
    let sb = Sandbox::empty();
    sb.write("target.md", "# Target");
    sb.write("source.md", "See [[target]]");
    let r = backlinks(&sb, "target.md", 3);
    let v = frankmd_testkit::serde_json::to_value(&r[0]).unwrap();
    for key in ["path", "name", "line_number", "match_text", "context"] {
        assert!(v.get(key).is_some(), "missing key {key}");
    }
    assert_eq!(r[0].name, "source");
    assert_eq!(r[0].line_number, 1);
}

// port: test/services/backlinks_service_test.rb:136 "rejects path traversal attempts"
#[test]
fn rejects_path_traversal_attempts() {
    let sb = Sandbox::empty();
    let vault = sb.vault();
    let svc = NotesService::new(&vault);
    for t in ["../etc/passwd", "folder/../../secret.md"] {
        let e = svc.backlinks(t, 3).unwrap_err();
        assert_eq!(e.compat_class(), "ArgumentError", "{t}");
    }
}
