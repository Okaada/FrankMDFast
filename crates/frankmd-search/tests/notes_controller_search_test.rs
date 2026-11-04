//! Port of the `GET /notes/search` tests in FrankMD's
//! `test/controllers/notes_controller_test.rb`. The endpoint is
//! `Note.search(q, context_lines: 3, max_results: 20)` → [`Query::ui`]; the HTTP
//! layer itself is Rails plumbing.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

mod common;

use common::{BASE_MTIME, Sandbox};
use frankmd_search::content::{self, Query};

const M: f64 = BASE_MTIME as f64;

// port: test/controllers/notes_controller_test.rb:582 "search returns matching results"
#[test]
fn search_returns_matching_results() {
    let sb = Sandbox::new();
    sb.note("test.md", "Hello world\nThis is searchable content", M);
    let r = content::search(&sb.root, &Query::ui("searchable"));
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.hits[0].path, "test.md");
}

// port: test/controllers/notes_controller_test.rb:593 "search returns empty array for no matches"
#[test]
fn search_returns_empty_array_for_no_matches() {
    let sb = Sandbox::new();
    sb.note("test.md", "Hello world", M);
    assert!(
        content::search(&sb.root, &Query::ui("nonexistent"))
            .hits
            .is_empty()
    );
}

// port: test/controllers/notes_controller_test.rb:603 "search supports regex patterns"
#[test]
fn search_supports_regex_patterns() {
    let sb = Sandbox::new();
    sb.note("test.md", "foo123bar", M);
    assert_eq!(
        content::search(&sb.root, &Query::ui(r"foo\d+bar"))
            .hits
            .len(),
        1
    );
}

// port: test/controllers/notes_controller_test.rb:613 "search returns context lines"
#[test]
fn search_returns_context_lines() {
    let sb = Sandbox::new();
    sb.note("test.md", "line1\nline2\nmatch\nline4\nline5", M);
    let r = content::search(&sb.root, &Query::ui("match"));
    assert!(r.hits[0].context.len() > 1);
}
