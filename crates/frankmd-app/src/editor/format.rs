//! Markdown formatting commands as pure text transforms.
//!
//! **[COMPAT]** ports of FrankMD's editor commands:
//! - [`wrap_selection`]: `codemirror_extensions.js` `wrapSelection` (Ctrl+B, Ctrl+I,
//!   Ctrl+Shift+S, Ctrl+\`), a toggle that unwraps markers around or inside the selection;
//! - [`insert_link`]: the `Mod-k` command;
//! - [`apply_menu_format`]: `text_format_controller.js` `applyFormatToEditor` (Ctrl+M menu);
//! - [`toggle_tasks`]: `codemirror_controller.js` `toggleTask` (Ctrl+Enter) over
//!   `frankmd_core::text::task`.
//!
//! These are GUI-free; they live here only until a `frankmd-core` home exists for them
//! (ui-test-plan "workspace::text_format"). Offsets are UTF-8 bytes.

use std::ops::Range;

use frankmd_core::text::task::{apply_task_toggle, task_marker_range};

use super::edit::{Edit, map_offset, minimal_replacement};

/// One row of the text format menu (`TextFormatController.formats`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFormat {
    /// Stable id (`bold`, `italic`, ...); the menu row id is `text-format-row-<id>`.
    pub id: &'static str,
    /// Mnemonic letter (upper case).
    pub hotkey: char,
    /// Opening marker.
    pub prefix: &'static str,
    /// Closing marker.
    pub suffix: &'static str,
}

impl TextFormat {
    /// i18n key of the row label.
    pub fn label_key(&self) -> String {
        format!("dialogs.text_format.{}", self.id)
    }

    /// Symmetric markers can be toggled off.
    pub fn is_toggleable(&self) -> bool {
        self.prefix == self.suffix
    }
}

/// The seven formats, in menu order.
pub const FORMATS: [TextFormat; 7] = [
    TextFormat {
        id: "bold",
        hotkey: 'B',
        prefix: "**",
        suffix: "**",
    },
    TextFormat {
        id: "italic",
        hotkey: 'I',
        prefix: "*",
        suffix: "*",
    },
    TextFormat {
        id: "strikethrough",
        hotkey: 'S',
        prefix: "~~",
        suffix: "~~",
    },
    TextFormat {
        id: "highlight",
        hotkey: 'H',
        prefix: "==",
        suffix: "==",
    },
    TextFormat {
        id: "subscript",
        hotkey: 'U',
        prefix: "~",
        suffix: "~",
    },
    TextFormat {
        id: "superscript",
        hotkey: 'P',
        prefix: "^",
        suffix: "^",
    },
    TextFormat {
        id: "link",
        hotkey: 'L',
        prefix: "[",
        suffix: "](url)",
    },
];

/// Look a format up by id.
pub fn format_by_id(id: &str) -> Option<&'static TextFormat> {
    FORMATS.iter().find(|f| f.id == id)
}

/// Look a format up by its mnemonic (case-insensitive).
pub fn format_by_hotkey(key: char) -> Option<&'static TextFormat> {
    let key = key.to_ascii_uppercase();
    FORMATS.iter().find(|f| f.hotkey == key)
}

/// `text[range]` when both ends are char boundaries inside the text.
fn slice(text: &str, range: Range<usize>) -> Option<&str> {
    text.get(range)
}

/// **[COMPAT]** `wrapSelection(view, prefix, suffix)`:
/// 1. markers just outside the selection → remove them, keep the text selected;
/// 2. the selection itself starts/ends with the markers → unwrap it;
/// 3. otherwise wrap and keep the inner text selected (an empty selection leaves the
///    caret between the markers).
pub fn wrap_selection(text: &str, sel: Range<usize>, prefix: &str, suffix: &str) -> Edit {
    let Range {
        start: from,
        end: to,
    } = sel;
    let selected = &text[from..to];
    let before_from = from.saturating_sub(prefix.len());
    let after_to = (to + suffix.len()).min(text.len());
    if slice(text, before_from..from) == Some(prefix) && slice(text, to..after_to) == Some(suffix) {
        return Edit {
            range: before_from..after_to,
            text: selected.to_owned(),
            selection: before_from..before_from + selected.len(),
        };
    }
    if selected.len() >= prefix.len() + suffix.len()
        && selected.starts_with(prefix)
        && selected.ends_with(suffix)
    {
        let unwrapped = &selected[prefix.len()..selected.len() - suffix.len()];
        return Edit {
            range: from..to,
            text: unwrapped.to_owned(),
            selection: from..from + unwrapped.len(),
        };
    }
    let inner = from + prefix.len();
    Edit {
        range: from..to,
        text: format!("{prefix}{selected}{suffix}"),
        selection: inner..inner + selected.len(),
    }
}

/// **[COMPAT]** `Mod-k`: `[sel](url)` with `url` selected, or `[](url)` with the caret
/// inside the brackets.
pub fn insert_link(text: &str, sel: Range<usize>) -> Edit {
    let selected = &text[sel.clone()];
    if selected.is_empty() {
        let caret = sel.start + 1;
        return Edit {
            range: sel,
            text: "[](url)".to_owned(),
            selection: caret..caret,
        };
    }
    let url = sel.start + selected.len() + 3;
    Edit {
        range: sel,
        text: format!("[{selected}](url)"),
        selection: url..url + 3,
    }
}

/// **[COMPAT]** `applyFormatToEditor`: the text format menu's action for a non-empty
/// selection. Symmetric formats toggle off (markers inside the selection first, then
/// just outside it); the link format selects `url`; other formats put the caret after
/// the formatted text.
pub fn apply_menu_format(text: &str, sel: Range<usize>, format: &TextFormat) -> Edit {
    let Range { start, end } = sel;
    let selected = &text[start..end];
    let (prefix, suffix) = (format.prefix, format.suffix);
    if format.is_toggleable() {
        if selected.starts_with(prefix)
            && selected.ends_with(suffix)
            && selected.len() >= prefix.len() + suffix.len()
        {
            let unwrapped = &selected[prefix.len()..selected.len() - suffix.len()];
            return Edit {
                range: start..end,
                text: unwrapped.to_owned(),
                selection: start..start + unwrapped.len(),
            };
        }
        if let (Some(before_start), after_end) =
            (start.checked_sub(prefix.len()), end + suffix.len())
            && after_end <= text.len()
            && slice(text, before_start..start) == Some(prefix)
            && slice(text, end..after_end) == Some(suffix)
        {
            return Edit {
                range: before_start..after_end,
                text: selected.to_owned(),
                selection: before_start..before_start + selected.len(),
            };
        }
    }
    let formatted = format!("{prefix}{selected}{suffix}");
    let selection = if prefix == "[" && suffix == "](url)" {
        let url = start + prefix.len() + selected.len() + 2;
        url..url + 3
    } else {
        let caret = start + formatted.len();
        caret..caret
    };
    Edit {
        range: start..end,
        text: formatted,
        selection,
    }
}

/// Byte range of the line containing `pos` (without its `\n`).
fn line_at(text: &str, pos: usize) -> Range<usize> {
    let start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    let end = text[pos..].find('\n').map_or(text.len(), |i| pos + i);
    start..end
}

/// **[COMPAT]** `toggleTask`: every line touched by the selection gets the shortcut
/// rule (toggle an existing marker in place, else turn the line into `- [ ] …`).
/// Returns `None` when nothing changes. The caret/selection is mapped through the
/// per-line changes.
pub fn toggle_tasks(text: &str, sel: Range<usize>) -> Option<Edit> {
    let first = line_at(text, sel.start);
    let last = line_at(text, sel.end);
    let span = first.start..last.end;
    let mut new_text = String::with_capacity(span.len() + 8);
    let mut changes: Vec<(Range<usize>, usize)> = Vec::new();
    let mut line_start = span.start;
    for (i, line) in text[span.clone()].split('\n').enumerate() {
        if i > 0 {
            new_text.push('\n');
        }
        let replacement = match task_marker_range(line) {
            Some(marker) => format!(
                "{}{}{}",
                &line[..marker.from],
                marker.insert,
                &line[marker.to..]
            ),
            None => apply_task_toggle(line),
        };
        if replacement != line {
            let (range, inserted) = minimal_replacement(line, line_start, &replacement);
            changes.push((range, inserted.len()));
        }
        new_text.push_str(&replacement);
        line_start += line.len() + 1;
    }
    if changes.is_empty() {
        return None;
    }
    // Changes are ascending and expressed in old offsets; map through them from the
    // last to the first so earlier ranges stay valid while a later one is applied.
    let map = |offset: usize| {
        changes.iter().rev().fold(offset, |pos, (range, new_len)| {
            map_offset(pos, range, *new_len)
        })
    };
    let selection = map(sel.start)..map(sel.end);
    Some(Edit {
        range: span,
        text: new_text,
        selection,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, edit: &Edit) -> String {
        let mut s = text.to_owned();
        edit.apply_to(&mut s);
        s
    }

    #[test]
    fn wrap_toggles() {
        let e = wrap_selection("a word b", 2..6, "**", "**");
        assert_eq!(apply("a word b", &e), "a **word** b");
        assert_eq!(e.selection, 4..8);
        let t = apply("a word b", &e);
        let u = wrap_selection(&t, 4..8, "**", "**");
        assert_eq!(apply(&t, &u), "a word b");
        assert_eq!(u.selection, 2..6);
    }

    #[test]
    fn task_toggle_maps_caret() {
        let e = toggle_tasks("x", 1..1).expect("changes");
        assert_eq!(apply("x", &e), "- [ ] x");
        assert_eq!(e.selection, 7..7);
    }
}
