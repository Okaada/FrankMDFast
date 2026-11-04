//! Port of the `search_content` tests in FrankMD's `test/services/notes_service_test.rb`
//! (the rest of that file belongs to frankmd-fs/frankmd-core).
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

mod common;

use std::time::Duration;

use common::{BASE_MTIME, Sandbox, paths, set_len};
use frankmd_search::content::{
    self, MAX_SEARCH_FILE_BYTES, MatchTimeout, Query, SEARCH_REGEX_TIMEOUT, SearchRegex, SkipReason,
};

const M: f64 = BASE_MTIME as f64;

fn search(sb: &Sandbox, q: &str) -> content::Results {
    content::search(&sb.root, &Query::new(q))
}

// port: test/services/notes_service_test.rb:118 "list_tree and search skip symlinked directories" (search half; list_tree half is frankmd-fs)
#[test]
fn search_skips_symlinked_directories() {
    let sb = Sandbox::new();
    std::fs::write(sb.outside.join("external.md"), "findme").unwrap();
    std::os::unix::fs::symlink(&sb.outside, sb.root.join("external")).unwrap();

    assert!(search(&sb, "findme").hits.is_empty());
}

// port: test/services/notes_service_test.rb:573 "search_content returns empty array for blank query"
#[test]
fn search_content_returns_empty_array_for_blank_query() {
    let sb = Sandbox::new();
    sb.note("note.md", "Some content", M);
    assert!(search(&sb, "").hits.is_empty());
    let nil = Query { pattern: None, ..Query::new("") };
    assert!(content::search(&sb.root, &nil).hits.is_empty());
}

// port: test/services/notes_service_test.rb:579 "search_content finds text in files"
#[test]
fn search_content_finds_text_in_files() {
    let sb = Sandbox::new();
    sb.note("note1.md", "Hello world\nThis is a test", M);
    sb.note("note2.md", "Another file\nWith different content", M - 100.0);

    let r = search(&sb, "world");
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.hits[0].path, "note1.md");
    assert_eq!(r.hits[0].line_number, 1);
    assert!(r.hits[0].match_text.contains("world"));
}

// port: test/services/notes_service_test.rb:590 "search_content is case insensitive"
#[test]
fn search_content_is_case_insensitive() {
    let sb = Sandbox::new();
    sb.note("note.md", "Hello World", M);
    assert_eq!(search(&sb, "WORLD").hits.len(), 1);
}

// port: test/services/notes_service_test.rb:597 "search_content does not fail on a file with invalid UTF-8 bytes"
#[test]
fn search_content_does_not_fail_on_invalid_utf8() {
    let sb = Sandbox::new();
    sb.note("bad.md", b"hello \xFF\xFE world\n", M);
    sb.note("good.md", "hello world", M - 100.0);

    let r = search(&sb, "world");
    assert!(paths(&r.hits).contains(&"good.md"));
}

// port: test/services/notes_service_test.rb:607 "search_content matches text in a file with invalid UTF-8, scrubbing bad bytes"
#[test]
fn search_content_matches_scrubbed_invalid_utf8() {
    let sb = Sandbox::new();
    sb.note("bad.md", b"target_word \xFF here\n", M);

    let r = search(&sb, "target_word");
    assert!(paths(&r.hits).contains(&"bad.md"));
    assert_eq!(r.hits[0].match_text, "target_word \u{FFFD} here");
}

// port: test/services/notes_service_test.rb:614 "search_content skips a file it cannot read and keeps searching the rest"
// (the Ruby test stubs IO.readlines to raise EACCES; here the file is chmod 000)
#[test]
fn search_content_skips_unreadable_file() {
    use std::os::unix::fs::PermissionsExt;
    let sb = Sandbox::new();
    sb.note("good.md", "findme here", M);
    let locked = sb.note("locked.md", "findme too", M - 100.0);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&locked).is_ok() {
        eprintln!("running with CAP_DAC_OVERRIDE (root?): cannot simulate EACCES, skipping");
        return;
    }

    let r = search(&sb, "findme");
    let p = paths(&r.hits);
    assert!(p.contains(&"good.md"));
    assert!(!p.contains(&"locked.md"));
    assert!(
        r.skipped.iter().any(|s| s.path == "locked.md" && matches!(s.reason, SkipReason::Io(_))),
        "unreadable file must be reported as skipped: {:?}",
        r.skipped
    );
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o600)).unwrap();
}

// port: test/services/notes_service_test.rb:631 "search_content supports regex patterns"
#[test]
fn search_content_supports_regex_patterns() {
    let sb = Sandbox::new();
    sb.note("note.md", "foo123bar\nfoo456bar\nhello world", M);
    assert_eq!(search(&sb, r"foo\d+bar").hits.len(), 2);
}

// port: test/services/notes_service_test.rb:638 "search_content skips files over the size limit"
#[test]
fn search_content_skips_files_over_the_size_limit() {
    let sb = Sandbox::new();
    sb.note("small.md", "findme", M);
    let large = sb.note("large.md", "findme", M - 1.0);
    set_len(&large, MAX_SEARCH_FILE_BYTES + 1);

    let r = search(&sb, "findme");
    assert_eq!(paths(&r.hits), vec!["small.md"]);
    assert!(r.skipped.iter().any(|s| s.path == "large.md" && s.reason == SkipReason::TooLarge));
}

// port: test/services/notes_service_test.rb:646 "search_content skips a regex timeout"
// (Ruby stubs `match?` to raise Regexp::TimeoutError; here a catastrophic pattern
// with a tiny budget must time out, and `search_lines` reports it)
#[test]
fn search_content_skips_a_regex_timeout() {
    let re = SearchRegex::compile("(a|a)+$", Duration::from_millis(1)).unwrap();
    let lines = vec![format!("{}!", "a".repeat(40))];
    assert_eq!(content::search_lines(&lines, &re, 3, 50), Err(MatchTimeout));

    let sb = Sandbox::new();
    sb.note("note.md", format!("{}!", "a".repeat(40)), M);
    let q = Query { timeout: Duration::from_millis(1), ..Query::new("(a|a)+$") };
    let r = content::search(&sb.root, &q);
    assert!(r.hits.is_empty());
    assert!(r.skipped.iter().any(|s| s.path == "note.md" && s.reason == SkipReason::Timeout));
    assert_eq!(SEARCH_REGEX_TIMEOUT, Duration::from_millis(500));
}

// port: test/services/notes_service_test.rb:654 "search_content includes context lines"
#[test]
fn search_content_includes_context_lines() {
    let sb = Sandbox::new();
    sb.note("note.md", "line1\nline2\nMATCH HERE\nline4\nline5", M);

    let q = Query { context_lines: 2, ..Query::new("MATCH") };
    let r = content::search(&sb.root, &q);
    assert_eq!(r.hits.len(), 1);
    let ctx = &r.hits[0].context;
    assert_eq!(ctx.len(), 5);
    assert_eq!(ctx.first().unwrap().line_number, 1);
    assert_eq!(ctx.last().unwrap().line_number, 5);
    assert!(ctx.iter().find(|c| c.is_match).unwrap().content.contains("MATCH"));
}

// port: test/services/notes_service_test.rb:668 "search_content respects max_results"
#[test]
fn search_content_respects_max_results() {
    let sb = Sandbox::new();
    for i in 0..5 {
        sb.note(&format!("note{i}.md"), "findme", M - f64::from(i) * 100.0);
    }
    let q = Query { max_results: 3, ..Query::new("findme") };
    let r = content::search(&sb.root, &q);
    assert_eq!(r.hits.len(), 3);
    assert!(r.truncated);
}

// port: test/services/notes_service_test.rb:677 "search_content searches nested folders"
#[test]
fn search_content_searches_nested_folders() {
    let sb = Sandbox::new();
    sb.note("folder/nested.md", "find this text", M);

    let r = search(&sb, "find this");
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.hits[0].path, "folder/nested.md");
    assert_eq!(r.hits[0].name, "nested");
}

// port: test/services/notes_service_test.rb:686 "search_content handles invalid regex by escaping"
#[test]
fn search_content_handles_invalid_regex_by_escaping() {
    let sb = Sandbox::new();
    sb.note("note.md", "test [brackets]", M);
    assert_eq!(search(&sb, "[brackets").hits.len(), 1);
    assert!(SearchRegex::compile("[brackets", SEARCH_REGEX_TIMEOUT).unwrap().is_literal_fallback());
}

// security: symlinks escaping the vault (to a folder, to a file, dangling) never
// leak outside content into results.
#[test]
fn search_never_returns_content_outside_the_vault() {
    let sb = Sandbox::new();
    std::fs::write(sb.outside.join("secret.md"), "findme outside").unwrap();
    std::fs::create_dir_all(sb.outside.join("deep")).unwrap();
    std::fs::write(sb.outside.join("deep/more.md"), "findme deep").unwrap();
    sb.note("inside.md", "findme inside", M);
    std::os::unix::fs::symlink(&sb.outside, sb.root.join("outdir")).unwrap();
    std::os::unix::fs::symlink(sb.outside.join("secret.md"), sb.root.join("secret.md")).unwrap();
    std::os::unix::fs::symlink(sb.outside.join("missing.md"), sb.root.join("dangling.md")).unwrap();
    std::fs::create_dir_all(sb.root.join("sub")).unwrap();
    std::os::unix::fs::symlink("../../outside", sb.root.join("sub/rel")).unwrap();

    let r = search(&sb, "findme");
    assert_eq!(paths(&r.hits), vec!["inside.md"]);
    assert!(r.hits.iter().all(|h| !h.match_text.contains("outside") && !h.match_text.contains("deep")));
}
