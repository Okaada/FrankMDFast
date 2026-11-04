//! Task-list checkbox toggling (port of `lib/task_utils.js`, issue #203):
//! shared by the preview's click-to-toggle and the `Ctrl+Enter` shortcut.
//!
//! A task line matches `^(\s*(?:[-*+]|\d{1,9}[.)])\s+)\[([ xX])\](\s|$)`
//! (JS `\s`, i.e. Unicode whitespace).

use super::{is_js_whitespace, js_trim};

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
    let from = task_prefix_len(line)?;
    let marker = line.as_bytes()[from + 1];
    let rest = &line[from + 3..];
    if !(rest.is_empty() || rest.starts_with(is_js_whitespace)) {
        return None;
    }
    Some(TaskMarkerRange {
        from,
        to: from + 3,
        insert: if marker == b' ' { "[x]" } else { "[ ]" },
    })
}

/// Toggle the marker of a task line; `None` for non-task lines.
pub fn toggle_task_marker(line: &str) -> Option<String> {
    let range = task_marker_range(line)?;
    Some(format!(
        "{}{}{}",
        &line[..range.from],
        range.insert,
        &line[range.to..]
    ))
}

/// Shortcut rule: toggle an existing task, otherwise turn the line into an
/// unchecked task keeping its leading whitespace: `"  x"` → `"  - [ ] x"`,
/// `"   "` → `"   - [ ]"`, `""` → `"- [ ]"`. The body is JS-`trim()`med.
pub fn apply_task_toggle(line: &str) -> String {
    if let Some(toggled) = toggle_task_marker(line) {
        return toggled;
    }
    let body = js_trim(line);
    let indent = &line[..line.len() - line.trim_start_matches(is_js_whitespace).len()];
    if body.is_empty() {
        format!("{indent}- [ ]")
    } else {
        format!("{indent}- [ ] {body}")
    }
}

/// Byte length of `^\s*(?:[-*+]|\d{1,9}[.)])\s+` when it is followed by
/// `[` + one of ` xX` + `]`; the offset of that `[`.
fn task_prefix_len(line: &str) -> Option<usize> {
    let after_indent = line.trim_start_matches(is_js_whitespace);
    let bytes = after_indent.as_bytes();
    let bullet_len = match bytes.first()? {
        b'-' | b'*' | b'+' => 1,
        _ => {
            let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            if !(1..=9).contains(&digits) || !matches!(bytes.get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
    };
    let after_bullet = &after_indent[bullet_len..];
    let after_space = after_bullet.trim_start_matches(is_js_whitespace);
    if after_space.len() == after_bullet.len() {
        return None;
    }
    let marker = after_space.as_bytes();
    if marker.len() < 3
        || marker[0] != b'['
        || !matches!(marker[1], b' ' | b'x' | b'X')
        || marker[2] != b']'
    {
        return None;
    }
    Some(line.len() - after_space.len())
}
