//! Model of the slash-command popover (`/` menu): filtered rows, selection
//! and the outcome of accepting a row. GUI-free; the editor view renders it.

use std::ops::Range;

use frankmd_core::text::edit::Edit;
use frankmd_core::text::slash::{
    SlashCommand, SlashTrigger, apply_command, dialog_range, slash_trigger,
};
use frankmd_core::text::utils::fuzzy_score;

/// Rows visible before the list scrolls.
pub const MAX_VISIBLE_ROWS: usize = 8;
/// Height of one row in pixels.
pub const ROW_HEIGHT_PX: f32 = 28.0;
/// Popover width in pixels.
pub const MENU_WIDTH_PX: f32 = 280.0;

/// One row of the menu.
#[derive(Debug, Clone, PartialEq)]
pub struct SlashRow {
    /// The command the row runs.
    pub command: SlashCommand,
    /// Localized label.
    pub label: String,
    /// Phosphor icon name.
    pub icon: &'static str,
}

/// What accepting a row does.
#[derive(Debug, Clone, PartialEq)]
pub enum SlashOutcome {
    /// Apply this single edit to the document.
    Edit(Edit),
    /// Open the command's dialog, bound to the `/query` range.
    OpenDialog {
        /// Dialog command (Table, Image, Video or Emoji).
        command: SlashCommand,
        /// Range of `/query` in the document.
        range: Range<usize>,
        /// Text of `/query` at open time.
        query: String,
    },
}

/// State of an open slash menu. Dismissing is dropping the model; the query
/// text stays in the document.
#[derive(Debug, Clone)]
pub struct SlashMenuModel {
    trigger: SlashTrigger,
    labels: Vec<(SlashCommand, String)>,
    rows: Vec<SlashRow>,
    selected: usize,
}

impl SlashMenuModel {
    /// Open the menu for `trigger`; `label` resolves a command to its
    /// localized label (its fallback when the translation is missing).
    pub fn open(trigger: SlashTrigger, label: impl Fn(SlashCommand) -> String) -> Self {
        let labels = SlashCommand::ALL.iter().map(|&c| (c, label(c))).collect();
        let mut model = Self {
            trigger,
            labels,
            rows: Vec::new(),
            selected: 0,
        };
        model.refilter();
        model
    }

    /// Re-run the trigger after the document changed. `false` means the
    /// trigger is gone and the caller must close the menu. The rows may be
    /// empty (nothing matches): the caller then hides the popover but may
    /// keep the model so backspacing brings the rows back.
    pub fn update(&mut self, text: &str, caret: usize) -> bool {
        let Some(trigger) = slash_trigger(text, caret) else {
            return false;
        };
        let previous = self.rows.get(self.selected).map(|r| r.command);
        self.trigger = trigger;
        self.refilter();
        self.selected = previous
            .and_then(|c| self.rows.iter().position(|r| r.command == c))
            .unwrap_or(0);
        true
    }

    /// The current trigger.
    pub fn trigger(&self) -> &SlashTrigger {
        &self.trigger
    }

    /// Rows matching the query, best match first.
    pub fn rows(&self) -> &[SlashRow] {
        &self.rows
    }

    /// Index of the selected row.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Select the next row, wrapping around.
    pub fn select_next(&mut self) {
        if !self.rows.is_empty() {
            self.selected = (self.selected + 1) % self.rows.len();
        }
    }

    /// Select the previous row, wrapping around.
    pub fn select_prev(&mut self) {
        if !self.rows.is_empty() {
            self.selected = (self.selected + self.rows.len() - 1) % self.rows.len();
        }
    }

    /// Select the row under the pointer (ignored when out of range).
    pub fn hover(&mut self, index: usize) {
        if index < self.rows.len() {
            self.selected = index;
        }
    }

    /// Number of rows shown without scrolling.
    pub fn visible_rows(&self) -> usize {
        self.rows.len().min(MAX_VISIBLE_ROWS)
    }

    /// Popover list height in pixels.
    pub fn height_px(&self) -> f32 {
        self.visible_rows() as f32 * ROW_HEIGHT_PX
    }

    /// Accept the selected row against the current `text`.
    pub fn accept(&self, text: &str) -> Option<SlashOutcome> {
        self.accept_at(self.selected, text)
    }

    /// Accept the row at `index`. `None` when out of range or when the
    /// command finds no editable block.
    pub fn accept_at(&self, index: usize, text: &str) -> Option<SlashOutcome> {
        let command = self.rows.get(index)?.command;
        if command.opens_dialog() {
            let (range, query) = dialog_range(&self.trigger);
            return Some(SlashOutcome::OpenDialog {
                command,
                range,
                query,
            });
        }
        apply_command(text, &self.trigger, command).map(SlashOutcome::Edit)
    }

    fn refilter(&mut self) {
        let query = normalize(&self.trigger.query);
        let mut scored: Vec<(u32, SlashRow)> = self
            .labels
            .iter()
            .filter_map(|(command, label)| {
                let score = if query.is_empty() {
                    1
                } else {
                    fuzzy_score(&normalize(&command.filter_text(label)), &query)
                };
                (score > 0).then(|| {
                    (
                        score,
                        SlashRow {
                            command: *command,
                            label: label.clone(),
                            icon: command.icon(),
                        },
                    )
                })
            })
            .collect();
        // Stable sort keeps the original order among equal scores.
        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        self.rows = scored.into_iter().map(|(_, row)| row).collect();
        self.selected = 0;
    }
}

/// Lowercase and turn blanks into `-` so word starts earn `fuzzy_score`'s
/// separator bonus.
fn normalize(s: &str) -> String {
    s.to_lowercase().replace(' ', "-")
}
