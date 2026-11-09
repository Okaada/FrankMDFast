//! The table editor dialog's model (ui-spec §5.8; port of FrankMD's
//! `table_editor_controller.js`).
//!
//! **[COMPAT]** kept: the default 3×3 grid with "Header N" headers, the dimension
//! steppers (min 1 row, 1 column; new columns get "Header N"), the cell menu
//! (move left/right/up/down, delete column/row; the header row never moves or is
//! deleted, the first body row can't move up), padded output (FE-388) and "an empty
//! table inserts nothing".
//!
//! **Changed** (ui-spec §5.8): column alignment is kept and editable
//! ([`Alignment`]), pipes in cells are escaped, the primary button reads "Update"
//! when editing an existing table, and every action has a keyboard shortcut
//! (shown in the cell menu).

use std::ops::Range;

pub use frankmd_core::text::aligned_table::Alignment;
use frankmd_core::text::aligned_table::{
    AlignedTable, generate_aligned_table, parse_aligned_table, strip_slash_query,
};
use frankmd_core::text::markdown::find_table_at_position;

/// Default header template (`dialogs.table_editor.header_n`); `%{n}` is the
/// 1-based column number.
pub const DEFAULT_HEADER_TEMPLATE: &str = "Header %{n}";

/// Insert a new table or update the one under the caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableMode {
    /// New table at the caret (or over a pending `/table`).
    Insert,
    /// Replace this byte range of the document (the table being edited).
    Update(Range<usize>),
}

/// The actions of the cell context menu, in menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellAction {
    /// Swap the column with its left neighbour (Alt+←).
    MoveLeft,
    /// Swap the column with its right neighbour (Alt+→).
    MoveRight,
    /// Delete the column (Ctrl+Alt+Backspace).
    DeleteColumn,
    /// Swap the row with the one above (Alt+↑).
    MoveUp,
    /// Swap the row with the one below (Alt+↓).
    MoveDown,
    /// Delete the row (Ctrl+Shift+Backspace).
    DeleteRow,
}

impl CellAction {
    /// All actions in menu order (column group, then row group).
    pub const ALL: [CellAction; 6] = [
        Self::MoveLeft,
        Self::MoveRight,
        Self::DeleteColumn,
        Self::MoveUp,
        Self::MoveDown,
        Self::DeleteRow,
    ];

    /// i18n key of the label (`dialogs.table_editor.*`).
    pub fn label_key(self) -> &'static str {
        match self {
            Self::MoveLeft => "dialogs.table_editor.move_left",
            Self::MoveRight => "dialogs.table_editor.move_right",
            Self::DeleteColumn => "dialogs.table_editor.delete_column",
            Self::MoveUp => "dialogs.table_editor.move_up",
            Self::MoveDown => "dialogs.table_editor.move_down",
            Self::DeleteRow => "dialogs.table_editor.delete_row",
        }
    }

    /// The shortcut shown next to the label (`mac`: `⌘⇧Backspace`).
    pub fn shortcut(self, mac: bool) -> String {
        let keystroke = match self {
            Self::MoveLeft => "alt-left",
            Self::MoveRight => "alt-right",
            Self::DeleteColumn => "ctrl-alt-backspace",
            Self::MoveUp => "alt-up",
            Self::MoveDown => "alt-down",
            Self::DeleteRow => "ctrl-shift-backspace",
        };
        crate::commands::display_keystroke(keystroke, mac)
    }

    /// Destructive actions use the danger colour.
    pub fn is_destructive(self) -> bool {
        matches!(self, Self::DeleteColumn | Self::DeleteRow)
    }
}

/// One cell-menu row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellMenuRow {
    /// The action.
    pub action: CellAction,
    /// Whether it applies to the cell.
    pub enabled: bool,
}

/// What Insert/Update produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableConfirm {
    /// The padded markdown table (no trailing newline).
    pub markdown: String,
    /// Insert or replace.
    pub mode: TableMode,
}

/// The table editor state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableEditorModel {
    table: AlignedTable,
    mode: TableMode,
    header_template: String,
    dirty: bool,
    revision: u64,
}

impl Default for TableEditorModel {
    fn default() -> Self {
        Self::new(DEFAULT_HEADER_TEMPLATE)
    }
}

impl TableEditorModel {
    /// A new 3×3 table: headers from `header_template` ("Header %{n}"), two empty rows.
    pub fn new(header_template: &str) -> Self {
        let header = (1..=3).map(|n| header_label(header_template, n)).collect();
        Self {
            table: AlignedTable {
                rows: vec![header, vec![String::new(); 3], vec![String::new(); 3]],
                alignments: Vec::new(),
            },
            mode: TableMode::Insert,
            header_template: header_template.to_owned(),
            dirty: false,
            revision: 0,
        }
    }

    /// Edit `markdown` (a table's lines) in place of `range`. A table that parses to
    /// no rows falls back to a single header row of three cells (**[COMPAT]**).
    pub fn from_markdown(markdown: &str, range: Range<usize>, header_template: &str) -> Self {
        let mut table = parse_aligned_table(markdown);
        if table.rows.is_empty() {
            table.rows = vec![(1..=3).map(|n| header_label(header_template, n)).collect()];
        }
        let cols = table.column_count();
        for row in &mut table.rows {
            row.resize(cols, String::new());
        }
        Self {
            table,
            mode: TableMode::Update(range),
            header_template: header_template.to_owned(),
            dirty: false,
            revision: 0,
        }
    }

    /// **[COMPAT]** `openTableEditor`: edit the table under `caret`, else a new table.
    /// `slash` is a pending `/table` (range + query): when it lies inside the table
    /// it is removed from the copy being edited (the document is untouched until
    /// Update).
    pub fn from_document(
        text: &str,
        caret: usize,
        slash: Option<(Range<usize>, &str)>,
        header_template: &str,
    ) -> Self {
        let Some(found) = find_table_at_position(text, caret) else {
            return Self::new(header_template);
        };
        let mut existing = found.lines.join("\n");
        if let Some((range, query)) = slash
            && range.start >= found.start_pos
            && range.end <= found.end_pos
            && text.get(range.clone()) == Some(query)
        {
            existing = strip_slash_query(
                &existing,
                range.start - found.start_pos..range.end - found.start_pos,
            );
        }
        Self::from_markdown(&existing, found.start_pos..found.end_pos, header_template)
    }

    /// Insert or update.
    pub fn mode(&self) -> &TableMode {
        &self.mode
    }

    /// i18n key of the primary button.
    pub fn primary_label_key(&self) -> &'static str {
        match self.mode {
            TableMode::Insert => "dialogs.table_editor.insert_table",
            TableMode::Update(_) => "dialogs.table_editor.update_table",
        }
    }

    /// Edited since opening.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Bumped on every change (views re-sync their inputs when it moves).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// `(rows, columns)`; rows include the header.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.rows(), self.cols())
    }

    /// Row count (header included).
    pub fn rows(&self) -> usize {
        self.table.rows.len()
    }

    /// Column count.
    pub fn cols(&self) -> usize {
        self.table.column_count()
    }

    /// Total cells.
    pub fn cell_count(&self) -> usize {
        self.rows() * self.cols()
    }

    /// The table rows (row 0 = header).
    pub fn cells(&self) -> &[Vec<String>] {
        &self.table.rows
    }

    /// Text of cell `(row, col)` (`""` when out of range).
    pub fn cell(&self, row: usize, col: usize) -> &str {
        self.table
            .rows
            .get(row)
            .and_then(|r| r.get(col))
            .map_or("", String::as_str)
    }

    /// Set a cell's text verbatim.
    pub fn set_cell(&mut self, row: usize, col: usize, value: &str) {
        let Some(cell) = self.table.rows.get_mut(row).and_then(|r| r.get_mut(col)) else {
            return;
        };
        if cell != value {
            *cell = value.to_owned();
            self.touch();
        }
    }

    /// Alignment of `col`.
    pub fn alignment(&self, col: usize) -> Alignment {
        self.table.alignment(col)
    }

    /// Set the alignment of `col` (the separator colons on output).
    pub fn set_alignment(&mut self, col: usize, alignment: Alignment) {
        if col >= self.cols() || self.alignment(col) == alignment {
            return;
        }
        let cols = self.cols();
        self.table.alignments.resize(cols, Alignment::None);
        self.table.alignments[col] = alignment;
        self.touch();
    }

    /// Rows stepper: set the row count (min 1; adds empty rows / drops from the bottom).
    pub fn set_rows(&mut self, rows: usize) {
        let rows = rows.max(1);
        if rows == self.rows() {
            return;
        }
        let cols = self.cols();
        self.table.rows.resize(rows, vec![String::new(); cols]);
        self.touch();
    }

    /// Columns stepper: set the column count (min 1; new columns get "Header N" on
    /// row 0 and `""` below; removing truncates from the right).
    pub fn set_cols(&mut self, cols: usize) {
        let cols = cols.max(1);
        let current = self.cols();
        if cols == current {
            return;
        }
        for (r, row) in self.table.rows.iter_mut().enumerate() {
            while row.len() < cols {
                let n = row.len() + 1;
                row.push(if r == 0 {
                    header_label(&self.header_template, n)
                } else {
                    String::new()
                });
            }
            row.truncate(cols);
        }
        self.table.alignments.truncate(cols);
        self.touch();
    }

    /// Both steppers.
    pub fn set_size(&mut self, rows: usize, cols: usize) {
        self.set_rows(rows);
        self.set_cols(cols);
    }

    /// Rows `+` (also Tab in the last cell).
    pub fn add_row(&mut self) {
        self.set_rows(self.rows() + 1);
    }

    /// Rows `−`.
    pub fn remove_row(&mut self) {
        self.set_rows(self.rows().saturating_sub(1));
    }

    /// Columns `+`.
    pub fn add_column(&mut self) {
        self.set_cols(self.cols() + 1);
    }

    /// Columns `−`.
    pub fn remove_column(&mut self) {
        self.set_cols(self.cols().saturating_sub(1));
    }

    /// Whether `action` applies to cell `(row, col)` (**[COMPAT]** enabled states).
    pub fn can(&self, action: CellAction, row: usize, col: usize) -> bool {
        let (rows, cols) = self.dimensions();
        match action {
            CellAction::MoveLeft => col > 0 && col < cols,
            CellAction::MoveRight => col + 1 < cols,
            CellAction::DeleteColumn => cols > 1 && col < cols,
            CellAction::MoveUp => row > 1 && row < rows,
            CellAction::MoveDown => row > 0 && row + 1 < rows,
            CellAction::DeleteRow => row > 0 && rows > 1 && row < rows,
        }
    }

    /// The cell menu for `(row, col)`.
    pub fn cell_menu(&self, row: usize, col: usize) -> Vec<CellMenuRow> {
        CellAction::ALL
            .iter()
            .map(|&action| CellMenuRow {
                action,
                enabled: self.can(action, row, col),
            })
            .collect()
    }

    /// Run `action` on `(row, col)`. Returns the cell that should keep focus
    /// (it follows the moved row/column), or `None` when the action didn't apply.
    pub fn apply(&mut self, action: CellAction, row: usize, col: usize) -> Option<(usize, usize)> {
        if !self.can(action, row, col) {
            return None;
        }
        let focus = match action {
            CellAction::MoveLeft => {
                self.swap_columns(col, col - 1);
                (row, col - 1)
            }
            CellAction::MoveRight => {
                self.swap_columns(col, col + 1);
                (row, col + 1)
            }
            CellAction::DeleteColumn => {
                for r in &mut self.table.rows {
                    if col < r.len() {
                        r.remove(col);
                    }
                }
                if col < self.table.alignments.len() {
                    self.table.alignments.remove(col);
                }
                (row, col.min(self.cols().saturating_sub(1)))
            }
            CellAction::MoveUp => {
                self.table.rows.swap(row, row - 1);
                (row - 1, col)
            }
            CellAction::MoveDown => {
                self.table.rows.swap(row, row + 1);
                (row + 1, col)
            }
            CellAction::DeleteRow => {
                self.table.rows.remove(row);
                (row.min(self.rows() - 1), col)
            }
        };
        self.touch();
        Some(focus)
    }

    /// Move Right on column `col` (headers swap with their neighbour).
    pub fn move_column_right(&mut self, col: usize) -> bool {
        self.apply(CellAction::MoveRight, 0, col).is_some()
    }

    /// Move Left on column `col`.
    pub fn move_column_left(&mut self, col: usize) -> bool {
        self.apply(CellAction::MoveLeft, 0, col).is_some()
    }

    /// Move Down on row `row` (not the header).
    pub fn move_row_down(&mut self, row: usize) -> bool {
        self.apply(CellAction::MoveDown, row, 0).is_some()
    }

    /// Move Up on row `row` (rows ≥ 2).
    pub fn move_row_up(&mut self, row: usize) -> bool {
        self.apply(CellAction::MoveUp, row, 0).is_some()
    }

    /// Delete column `col` (keeps at least one).
    pub fn delete_column(&mut self, col: usize) -> bool {
        self.apply(CellAction::DeleteColumn, 0, col).is_some()
    }

    /// Delete row `row` (never the header; keeps at least one).
    pub fn delete_row(&mut self, row: usize) -> bool {
        self.apply(CellAction::DeleteRow, row, 0).is_some()
    }

    /// The markdown for the current grid (padded, aligned, pipes escaped).
    pub fn to_markdown(&self) -> String {
        generate_aligned_table(&self.table)
    }

    /// Insert/Update: `None` when the table is empty (the dialog just closes).
    pub fn confirm(&self) -> Option<TableConfirm> {
        let markdown = self.to_markdown();
        (!markdown.is_empty()).then(|| TableConfirm {
            markdown,
            mode: self.mode.clone(),
        })
    }

    fn swap_columns(&mut self, a: usize, b: usize) {
        for r in &mut self.table.rows {
            if a < r.len() && b < r.len() {
                r.swap(a, b);
            }
        }
        let cols = self.cols();
        if self.table.alignments.iter().any(|&x| x != Alignment::None) {
            self.table.alignments.resize(cols, Alignment::None);
            self.table.alignments.swap(a, b);
        }
    }

    fn touch(&mut self) {
        self.dirty = true;
        self.revision += 1;
    }
}

fn header_label(template: &str, n: usize) -> String {
    template.replace("%{n}", &n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_follows_its_column() {
        let mut m = TableEditorModel::from_markdown(
            "| a | b |\n| :-- | --: |\n| 1 | 2 |",
            0..10,
            DEFAULT_HEADER_TEMPLATE,
        );
        assert!(m.move_column_right(0));
        assert_eq!(m.alignment(0), Alignment::Right);
        assert_eq!(m.alignment(1), Alignment::Left);
        assert!(m.delete_column(0));
        assert_eq!(m.alignment(0), Alignment::Left);
    }
}
