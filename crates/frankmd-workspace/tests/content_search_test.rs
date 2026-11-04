//! Tests for the content_search_test view model.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant, SystemTime};

use frankmd_search::content::{search_lines, Hit, Results, SearchRegex, SEARCH_REGEX_TIMEOUT};
use frankmd_workspace::content_search::*;

fn hit(path: &str, line: u32) -> Hit {
    Hit {
        path: path.into(),
        name: path.rsplit('/').next().unwrap_or(path).into(),
        mtime: SystemTime::UNIX_EPOCH,
        line_number: line,
        match_text: "x".into(),
        context: vec![],
    }
}

fn results(hits: Vec<Hit>, truncated: bool) -> Results {
    Results { hits, skipped: vec![], truncated }
}

fn matches(pattern: &str, line: &str) -> bool {
    let re = SearchRegex::compile(pattern, SEARCH_REGEX_TIMEOUT).unwrap();
    !search_lines(&[line.to_owned()], &re, 0, 10).unwrap().is_empty()
}

fn loaded(hits: Vec<Hit>) -> ContentSearchModel {
    let mut m = ContentSearchModel::default();
    let t0 = Instant::now();
    m.set_query("x", t0);
    let req = m.tick(t0 + DEBOUNCE).unwrap();
    m.apply_results(req.generation, results(hits, false));
    m
}

// adapts: content_search_controller.test.js:59 "initializes empty state"
#[test]
fn starts_empty() {
    let m = ContentSearchModel::default();
    assert!(m.results().is_empty());
    assert_eq!(m.selected(), None);
    assert!(!m.searching());
    assert_eq!(m.status(), SearchStatus::Hint);
    assert_eq!(m.options(), SearchOptions { case_sensitive: false, regex: true });
}

// adapts: content_search_controller.test.js:74 "clears previous state"
#[test]
fn open_resets_but_keeps_options() {
    let mut m = loaded(vec![hit("a.md", 1), hit("b.md", 2), hit("c.md", 3)]);
    m.select(2);
    let opts = SearchOptions { case_sensitive: true, regex: false };
    m.set_options(opts, Instant::now());
    m.open();
    assert_eq!(m.query(), "");
    assert!(m.results().is_empty());
    assert_eq!(m.selected(), None);
    assert!(!m.searching());
    assert_eq!(m.status(), SearchStatus::Hint);
    assert_eq!(m.options(), opts);
}

// adapts: content_search_controller.test.js:108 "clears results when query is empty"
#[test]
fn blank_query_clears() {
    let mut m = loaded(vec![hit("a.md", 1)]);
    assert_eq!(m.set_query("   ", Instant::now()), None);
    assert!(m.results().is_empty());
    assert!(!m.searching());
    assert_eq!(m.status(), SearchStatus::Hint);
    assert!(m.search_now().is_none());
}

// adapts: content_search_controller.test.js:124 "sets up debounced search"
#[test]
fn debounce_fires_once_after_300ms() {
    let mut m = ContentSearchModel::default();
    let t0 = Instant::now();
    assert_eq!(m.set_query("hello", t0), Some(t0 + DEBOUNCE));
    assert!(m.searching());
    assert!(m.tick(t0 + Duration::from_millis(299)).is_none());
    let req = m.tick(t0 + Duration::from_millis(300)).unwrap();
    assert_eq!(req.query.pattern.as_deref(), Some("hello"));
    assert!(m.tick(t0 + Duration::from_millis(900)).is_none());
}

// adapts: content_search_controller.test.js:135 "cancels previous timeout on new input"
#[test]
fn typing_rearms_debounce() {
    let mut m = ContentSearchModel::default();
    let t0 = Instant::now();
    m.set_query("hel", t0);
    m.set_query("hello", t0 + Duration::from_millis(200));
    assert!(m.tick(t0 + Duration::from_millis(300)).is_none());
    let req = m.tick(t0 + Duration::from_millis(500)).unwrap();
    assert_eq!(req.query.pattern.as_deref(), Some("hello"));
    assert!(m.tick(t0 + Duration::from_millis(1000)).is_none());
}

#[test]
fn ui_query_parameters() {
    let mut m = ContentSearchModel::default();
    m.set_query("a", Instant::now());
    let q = m.search_now().unwrap().query;
    assert_eq!(q.max_results, 20);
    assert_eq!(q.context_lines, 3);
}

// adapts: content_search_controller.test.js:447 "hides spinner after performSearch completes"
#[test]
fn stale_generation_dropped() {
    let mut m = ContentSearchModel::default();
    let t0 = Instant::now();
    m.set_query("a", t0);
    let g1 = m.tick(t0 + DEBOUNCE).unwrap().generation;
    m.set_query("ab", t0 + DEBOUNCE);
    let g2 = m.tick(t0 + DEBOUNCE * 2).unwrap().generation;
    assert_ne!(g1, g2);
    m.apply_results(g1, results(vec![hit("old.md", 1)], false));
    assert!(m.results().is_empty());
    assert!(m.searching());
    m.apply_results(g2, results(vec![hit("new.md", 1)], false));
    assert_eq!(m.results().len(), 1);
    assert!(!m.searching());
    m.apply_error(g1);
    assert_ne!(m.status(), SearchStatus::Error);
}

// adapts: content_search_controller.test.js:194 "shows match count in status"
#[test]
fn status_counts() {
    let m = loaded(vec![hit("a.md", 1), hit("b.md", 2)]);
    assert_eq!(m.status(), SearchStatus::Count { matches: 2, notes: 2, truncated: false });

    let mut m = ContentSearchModel::default();
    m.set_query("x", Instant::now());
    let g = m.search_now().unwrap().generation;
    let hits = (0..20).map(|i| hit(&format!("n{}.md", i % 7), i + 1)).collect();
    m.apply_results(g, results(hits, true));
    assert_eq!(m.status(), SearchStatus::Count { matches: 20, notes: 7, truncated: true });
}

// adapts: content_search_controller.test.js:209 "shows no matches message"
#[test]
fn no_matches() {
    let m = loaded(vec![]);
    assert_eq!(m.status(), SearchStatus::NoMatches);
    assert_eq!(m.selected(), None);
}

// adapts: content_search_controller.test.js:220 "handles search errors"
#[test]
fn error_status() {
    let mut m = ContentSearchModel::default();
    m.set_query("x", Instant::now());
    let g = m.search_now().unwrap().generation;
    m.apply_error(g);
    assert_eq!(m.status(), SearchStatus::Error);
    assert!(!m.searching());
}

// adapts: content_search_controller.test.js:117 "shows searching status"
#[test]
fn searching_from_first_keystroke() {
    let mut m = ContentSearchModel::default();
    m.set_query("h", Instant::now());
    assert!(m.searching());
    assert_eq!(m.status(), SearchStatus::Searching);
}

#[test]
fn effective_pattern_modes() {
    let d = SearchOptions::default();
    assert_eq!(effective_pattern("a.b", d), ("a.b".to_owned(), false));
    assert!(matches("a.b", "axb"));

    let lit = SearchOptions { regex: false, ..d };
    let (p, fb) = effective_pattern("a.b", lit);
    assert!(!fb);
    assert!(!matches(&p, "axb"));
    assert!(matches(&p, "a.b"));

    assert_eq!(effective_pattern("  ", d), ("  ".to_owned(), false));
}

#[test]
fn case_sensitive_wrapper_changes_matching() {
    let insens = effective_pattern("Hello", SearchOptions::default()).0;
    assert!(matches(&insens, "hello"));
    let sens = effective_pattern("Hello", SearchOptions { case_sensitive: true, regex: true }).0;
    assert!(sens.starts_with("(?-i:"));
    assert!(!matches(&sens, "hello"));
    assert!(matches(&sens, "say Hello"));
}

#[test]
fn invalid_regex_falls_back_to_literal() {
    let (p, fb) = effective_pattern("a(", SearchOptions::default());
    assert!(fb);
    assert!(matches(&p, "call a( now"));

    let mut m = ContentSearchModel::default();
    m.set_query("a(", Instant::now());
    assert!(!m.invalid_pattern());
    let req = m.search_now().unwrap();
    assert!(m.invalid_pattern());
    assert_eq!(req.query.pattern.as_deref(), Some("a\\("));
    m.set_query("ok", Instant::now());
    m.search_now();
    assert!(!m.invalid_pattern());
}

#[test]
fn options_change_rearms() {
    let mut m = ContentSearchModel::default();
    let t0 = Instant::now();
    assert_eq!(m.set_options(SearchOptions { case_sensitive: true, regex: true }, t0), None);
    m.set_query("A", t0);
    let t1 = t0 + Duration::from_millis(500);
    assert_eq!(m.set_options(SearchOptions::default(), t1), Some(t1 + DEBOUNCE));
}

// adapts: content_search_controller.test.js:164 "updates results data from HTML response"
#[test]
fn groups_in_first_appearance_order() {
    let m = loaded(vec![hit("b.md", 1), hit("a.md", 2), hit("b.md", 9)]);
    let g = m.groups();
    assert_eq!(g.len(), 2);
    assert_eq!((g[0].path.as_str(), g[0].hits.as_slice()), ("b.md", [0usize, 2].as_slice()));
    assert_eq!((g[1].path.as_str(), g[1].hits.as_slice()), ("a.md", [1usize].as_slice()));
}

// adapts: content_search_controller.test.js:182 "resets selected index"
#[test]
fn results_select_first() {
    let mut m = loaded(vec![hit("a.md", 1), hit("b.md", 2), hit("c.md", 3), hit("d.md", 4)]);
    m.select(3);
    let g = m.search_now().unwrap().generation;
    m.apply_results(g, results(vec![hit("a.md", 1), hit("b.md", 2)], false));
    assert_eq!(m.selected(), Some(0));
}

// adapts: content_search_controller.test.js:259 "does not go below 0"
// adapts: content_search_controller.test.js:268 "does not exceed list length"
#[test]
fn selection_wraps() {
    let mut m = loaded(vec![hit("a.md", 1), hit("b.md", 2), hit("c.md", 3)]);
    m.select_prev();
    assert_eq!(m.selected(), Some(2));
    m.select_next();
    assert_eq!(m.selected(), Some(0));
    m.select_next();
    assert_eq!(m.selected(), Some(1));
}

// adapts: content_search_controller.test.js:299 "handles missing element gracefully"
#[test]
fn navigation_on_empty_is_noop() {
    let mut m = ContentSearchModel::default();
    m.select_next();
    m.select_prev();
    m.select(3);
    assert_eq!(m.selected(), None);
    assert_eq!(m.activate(), None);
}

// adapts: content_search_controller.test.js:481 "updates selectedIndex"
// adapts: content_search_controller.test.js:371 "dispatches selected event for current selection"
#[test]
fn activate_returns_open_at() {
    let mut m = loaded(vec![hit("notes/a.md", 12), hit("folder/note.md", 25)]);
    assert_eq!(m.activate(), Some(OpenAt { path: "notes/a.md".into(), line: 12 }));
    m.select(1);
    assert_eq!(m.activate(), Some(OpenAt { path: "folder/note.md".into(), line: 25 }));
}
