//! Tests for the navigation_test view model.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use frankmd_workspace::navigation::{History, RecentSwitcher};

fn s(x: &str) -> Option<String> {
    Some(x.to_owned())
}

#[test]
fn history_visit_back_forward() {
    let mut h = History::default();
    assert_eq!(h.current(), None);
    assert!(!h.can_back() && !h.can_forward());
    h.visit(Some("a.md"));
    h.visit(Some("a.md"));
    h.visit(Some("b.md"));
    assert_eq!(h.current(), Some("b.md"));
    assert_eq!(h.back(), Some(s("a.md")));
    assert!(h.can_forward());
    assert_eq!(h.forward(), Some(s("b.md")));
    assert_eq!(h.forward(), None);
}

#[test]
fn history_back_past_first_is_empty_state() {
    let mut h = History::default();
    h.visit(Some("a.md"));
    assert_eq!(h.back(), Some(None));
    assert_eq!(h.current(), None);
    assert_eq!(h.back(), None);
    assert_eq!(h.forward(), Some(s("a.md")));
}

#[test]
fn history_visit_clears_forward() {
    let mut h = History::default();
    h.visit(Some("a.md"));
    h.visit(Some("b.md"));
    h.back();
    h.visit(Some("c.md"));
    assert!(!h.can_forward());
    assert_eq!(h.back(), Some(s("a.md")));
}

#[test]
fn history_visit_none_is_an_entry() {
    let mut h = History::default();
    h.visit(Some("a.md"));
    h.visit(None);
    assert_eq!(h.back(), Some(s("a.md")));
}

#[test]
fn history_remap_is_segment_wise() {
    let mut h = History::default();
    h.visit(Some("docs/a.md"));
    h.visit(Some("docs2/b.md"));
    h.visit(Some("docs"));
    h.visit(Some("x.md"));
    h.remap("docs", "notes");
    assert_eq!(h.back(), Some(s("notes")));
    assert_eq!(h.back(), Some(s("docs2/b.md")));
    assert_eq!(h.back(), Some(s("notes/a.md")));
}

#[test]
fn history_remove_collapses() {
    let mut h = History::default();
    h.visit(Some("a.md"));
    h.visit(Some("dir/x.md"));
    h.visit(Some("b.md"));
    h.visit(Some("dir/y.md"));
    h.visit(Some("a.md"));
    h.remove("dir");
    assert_eq!(h.back(), Some(s("b.md")));
    assert_eq!(h.back(), Some(s("a.md")));
    assert_eq!(h.back(), Some(None));
}

#[test]
fn history_remove_current_becomes_empty() {
    let mut h = History::default();
    h.visit(Some("a.md"));
    h.visit(Some("dir/x.md"));
    h.remove("dir/x.md");
    assert_eq!(h.current(), None);
    assert_eq!(h.back(), Some(s("a.md")));
}

fn sw(paths: &[&str]) -> RecentSwitcher {
    let mut r = RecentSwitcher::default();
    for p in paths {
        r.touch(p);
    }
    r
}

#[test]
fn switcher_touch_dedupes_and_caps() {
    let mut r = sw(&["a", "b", "c", "a"]);
    assert_eq!(r.mru(), ["a", "c", "b"]);
    for i in 0..60 {
        r.touch(&format!("n{i}"));
    }
    assert_eq!(r.mru().len(), 50);
    assert_eq!(r.mru()[0], "n59");
}

#[test]
fn switcher_start_and_step_wrap() {
    let mut r = sw(&["a", "b", "c"]); // mru: c b a
    r.start(true);
    assert_eq!(r.selected(), Some("b"));
    r.step(true);
    r.step(true);
    assert_eq!(r.selected(), Some("c"));
    r.step(false);
    assert_eq!(r.selected(), Some("a"));
    r.cancel();
    assert!(!r.is_active());
    r.start(false);
    assert_eq!(r.selected(), Some("a"));
}

#[test]
fn switcher_single_and_empty() {
    let mut r = RecentSwitcher::default();
    r.start(true);
    assert!(!r.is_active());
    assert_eq!(r.commit(), None);
    r.touch("only");
    r.start(true);
    assert_eq!(r.selected(), Some("only"));
}

#[test]
fn switcher_step_starts_and_commit_ends() {
    let mut r = sw(&["a", "b"]);
    r.step(true);
    assert!(r.is_active());
    assert_eq!(r.commit(), Some("a".to_owned()));
    assert!(!r.is_active());
    assert_eq!(r.commit(), None);
}

#[test]
fn switcher_remap_and_remove() {
    let mut r = sw(&["d/a", "d/b", "z"]); // z d/b d/a
    r.start(true);
    assert_eq!(r.selected(), Some("d/b"));
    r.remap("d", "e");
    assert_eq!(r.mru(), ["z", "e/b", "e/a"]);
    assert_eq!(r.selected(), Some("e/b"));
    r.remove("e");
    assert_eq!(r.mru(), ["z"]);
    assert_eq!(r.selected(), Some("z"));
    r.remove("z");
    assert!(!r.is_active());
}
