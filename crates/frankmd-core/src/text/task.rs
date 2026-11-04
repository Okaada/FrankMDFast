//! Task-list checkbox toggling (port of `lib/task_utils.js`, issue #203):
//! shared by the preview's click-to-toggle and the `Ctrl+Enter` shortcut.
//!
//! A task line matches `^(\s*(?:[-*+]|\d{1,9}[.)])\s+)\[([ xX])\](\s|$)`
//! (JS `\s`, i.e. Unicode whitespace).

/// The `[ ]`/`[x]` marker of a task line and its toggled replacement.
///
/// Replacing only this 3-byte range is length-preserving, so other cursors in
/// the document map through the edit unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMarkerRange {
    /// Byte offset of `[` within the line.
    pub from: usize,
    /// `from + 3`.
    pub to: usize,
    /// `"[x]"` when the marker was unchecked, else `"[ ]"` (`[X]` unchecks too).
    pub insert: &'static str,
}

/// Locate the marker in one line; `None` if the line is not a GFM task item.
pub fn task_marker_range(line: &str) -> Option<TaskMarkerRange> {
    todo!()
}

/// Toggle the marker of a task line; `None` for non-task lines.
pub fn toggle_task_marker(line: &str) -> Option<String> {
    todo!()
}

/// Shortcut rule: toggle an existing task, otherwise turn the line into an
/// unchecked task keeping its leading whitespace: `"  x"` → `"  - [ ] x"`,
/// `"   "` → `"   - [ ]"`, `""` → `"- [ ]"`. The body is JS-`trim()`med.
pub fn apply_task_toggle(line: &str) -> String {
    todo!()
}
