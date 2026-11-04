//! Port of test/javascript/lib/task_utils.test.js.

use frankmd_core::text::task::{apply_task_toggle, toggle_task_marker};

fn t(s: &str) -> Option<String> {
    toggle_task_marker(s)
}

// port: test/javascript/lib/task_utils.test.js:5 "checks an unchecked task"
#[test]
fn checks_an_unchecked_task() {
    assert_eq!(t("- [ ] buy milk").as_deref(), Some("- [x] buy milk"));
}

// port: test/javascript/lib/task_utils.test.js:9 "unchecks a checked task"
#[test]
fn unchecks_a_checked_task() {
    assert_eq!(t("- [x] buy milk").as_deref(), Some("- [ ] buy milk"));
}

// port: test/javascript/lib/task_utils.test.js:13 "handles * and + bullets"
#[test]
fn handles_star_and_plus_bullets() {
    assert_eq!(t("* [ ] a").as_deref(), Some("* [x] a"));
    assert_eq!(t("+ [x] a").as_deref(), Some("+ [ ] a"));
}

// port: test/javascript/lib/task_utils.test.js:18 "handles ordered task items"
#[test]
fn handles_ordered_task_items() {
    assert_eq!(t("3. [ ] a").as_deref(), Some("3. [x] a"));
    assert_eq!(t("1) [x] a").as_deref(), Some("1) [ ] a"));
}

// port: test/javascript/lib/task_utils.test.js:23 "handles an uppercase X marker and end-of-line task"
#[test]
fn handles_uppercase_x_and_end_of_line_task() {
    assert_eq!(t("- [X] done").as_deref(), Some("- [ ] done"));
    assert_eq!(t("- [x]").as_deref(), Some("- [ ]"));
}

// port: test/javascript/lib/task_utils.test.js:28 "preserves indentation"
#[test]
fn preserves_indentation() {
    assert_eq!(t("  - [ ] nested").as_deref(), Some("  - [x] nested"));
}

// port: test/javascript/lib/task_utils.test.js:32 "returns null for non-task lines"
#[test]
fn returns_none_for_non_task_lines() {
    for line in ["plain text", "- no marker", "- [y] not gfm", "[ ] no bullet", ""] {
        assert_eq!(t(line), None, "{line:?}");
    }
}

// port: test/javascript/lib/task_utils.test.js:42 "toggles an existing task"
#[test]
fn apply_toggles_an_existing_task() {
    assert_eq!(apply_task_toggle("- [ ] a"), "- [x] a");
    assert_eq!(apply_task_toggle("- [x] a"), "- [ ] a");
}

// port: test/javascript/lib/task_utils.test.js:47 "turns a plain line into an unchecked task"
#[test]
fn apply_turns_plain_line_into_task() {
    assert_eq!(apply_task_toggle("buy milk"), "- [ ] buy milk");
}

// port: test/javascript/lib/task_utils.test.js:51 "preserves indentation when creating a task"
#[test]
fn apply_preserves_indentation_when_creating() {
    assert_eq!(apply_task_toggle("  indented"), "  - [ ] indented");
}

// port: test/javascript/lib/task_utils.test.js:55 "creates a bare marker on an empty line"
#[test]
fn apply_creates_bare_marker_on_empty_line() {
    assert_eq!(apply_task_toggle(""), "- [ ]");
    assert_eq!(apply_task_toggle("   "), "   - [ ]");
}
