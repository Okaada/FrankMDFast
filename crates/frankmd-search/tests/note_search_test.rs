//! Port of the `Note.search` test in FrankMD's `test/models/note_test.rb`
//! (the rest of that file belongs to the notes-fs port). `Note.search(q)` is
//! `NotesService#search_content(q)` with the service defaults → [`Query::new`].
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

mod common;

use common::{BASE_MTIME, Sandbox};
use frankmd_search::content::{self, Query};

// port: test/models/note_test.rb:93 "Note.search returns matching results"
#[test]
fn note_search_returns_matching_results() {
    let sb = Sandbox::new();
    sb.note(
        "searchable.md",
        "Hello world findme content",
        BASE_MTIME as f64,
    );
    let r = content::search(&sb.root, &Query::new("findme"));
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.hits[0].path, "searchable.md");
}
