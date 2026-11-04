//! Table editor model (ui-spec §5.8). Adapted from FrankMD's
//! `table_editor_controller.test.js` and the table rows of
//! `test/system/notes_test.rb` (docs/testing/ui-test-plan.md §7 rows 23–33, §13).
#![allow(clippy::unwrap_used)]

use frankmd_workspace::table_editor::{
    Alignment, CellAction, DEFAULT_HEADER_TEMPLATE, TableEditorModel, TableMode,
};

fn model() -> TableEditorModel {
    TableEditorModel::new(DEFAULT_HEADER_TEMPLATE)
}

fn from(md: &str) -> TableEditorModel {
    TableEditorModel::from_markdown(md, 0..md.len(), DEFAULT_HEADER_TEMPLATE)
}

#[test]
fn default_grid_is_three_by_three() {
    // adapts: notes_test.rb:469 "table editor dialog opens and shows grid"
    let m = model();
    assert_eq!(m.dimensions(), (3, 3));
    assert_eq!(m.cell_count(), 9);
    assert_eq!(m.cells()[0], vec!["Header 1", "Header 2", "Header 3"]);
    assert_eq!(*m.mode(), TableMode::Insert);
    assert_eq!(m.primary_label_key(), "dialogs.table_editor.insert_table");
}

#[test]
fn add_and_remove_columns() {
    // adapts: notes_test.rb:484 "table editor can add and remove columns"
    let mut m = model();
    m.add_column();
    assert_eq!(m.cell_count(), 12);
    assert_eq!(m.cell(0, 3), "Header 4");
    m.remove_column();
    assert_eq!(m.cell_count(), 9);
}

#[test]
fn add_and_remove_rows() {
    // adapts: notes_test.rb:505 "table editor can add and remove rows"
    let mut m = model();
    m.add_row();
    assert_eq!(m.cell_count(), 12);
    m.remove_row();
    assert_eq!(m.cell_count(), 9);
}

#[test]
fn insert_writes_header_and_separator() {
    // adapts: notes_test.rb:526 "inserting a table adds markdown to content"
    let mut m = model();
    m.set_cell(0, 0, "Name");
    let out = m.confirm().unwrap();
    let mut lines = out.markdown.lines();
    assert!(lines.next().unwrap().starts_with("| Name"));
    assert!(lines.next().unwrap().starts_with("| ----"));
    assert_eq!(out.mode, TableMode::Insert);
}

#[test]
fn caret_in_table_loads_it_for_update() {
    // adapts: notes_test.rb:554 "editing existing table in content"
    let text = "intro\n\n| Col1 | Col2 |\n|------|------|\n| A | B |\n\nafter";
    let caret = text.find("A |").unwrap();
    let m = TableEditorModel::from_document(text, caret, None, DEFAULT_HEADER_TEMPLATE);
    assert_eq!(m.cells()[0], vec!["Col1", "Col2"]);
    assert_eq!(m.cells()[1], vec!["A", "B"]);
    let start = text.find("| Col1").unwrap();
    let end = text.find(" B |").unwrap() + 4;
    assert_eq!(*m.mode(), TableMode::Update(start..end));
    assert_eq!(m.primary_label_key(), "dialogs.table_editor.update_table");
}

#[test]
fn caret_outside_a_table_starts_a_new_one() {
    let m = TableEditorModel::from_document("just text", 3, None, "Cabeçalho %{n}");
    assert_eq!(m.cell(0, 1), "Cabeçalho 2");
    assert_eq!(*m.mode(), TableMode::Insert);
}

#[test]
fn cell_menu_lists_actions_with_shortcuts() {
    // adapts: notes_test.rb:593 "table cell context menu appears on right-click"
    let m = model();
    let menu = m.cell_menu(0, 0);
    let actions: Vec<_> = menu.iter().map(|r| r.action).collect();
    assert_eq!(actions, CellAction::ALL.to_vec());
    assert!(menu.iter().all(|r| !r.action.shortcut().is_empty()));
    assert_eq!(CellAction::MoveDown.shortcut(), "Alt+↓");
    assert_eq!(CellAction::DeleteRow.shortcut(), "Ctrl+Shift+Backspace");
}

#[test]
fn move_column_right_swaps_headers() {
    // adapts: notes_test.rb:617 "move column right via context menu"
    let mut m = model();
    assert!(m.move_column_right(0));
    assert_eq!(&m.cells()[0][..2], &["Header 2", "Header 1"]);
}

#[test]
fn move_row_down_moves_the_row() {
    // adapts: notes_test.rb:647 "move row down via context menu"
    let mut m = model();
    m.set_cell(1, 0, "Row2Col1");
    assert!(m.move_row_down(1));
    assert_eq!(m.cell(2, 0), "Row2Col1");
}

#[test]
fn delete_column_and_row() {
    // adapts: notes_test.rb:678 "delete column via context menu" / :702 "delete row via context menu"
    let mut m = model();
    assert!(m.delete_column(1));
    assert_eq!(m.cell_count(), 6);
    let mut m = model();
    assert!(m.delete_row(1));
    assert_eq!(m.cell_count(), 6);
}

#[test]
fn header_row_cannot_be_deleted_or_moved() {
    // adapts: notes_test.rb:726 "cannot delete header row"
    let mut m = model();
    let menu = m.cell_menu(0, 0);
    let enabled = |a: CellAction| menu.iter().find(|r| r.action == a).unwrap().enabled;
    assert!(!enabled(CellAction::DeleteRow));
    assert!(!enabled(CellAction::MoveUp));
    assert!(!enabled(CellAction::MoveDown));
    assert!(!enabled(CellAction::MoveLeft));
    assert!(enabled(CellAction::MoveRight));
    assert!(!m.delete_row(0));
    assert_eq!(m.rows(), 3);
    // The first body row can't move up into the header.
    assert!(!m.move_row_up(1));
    assert!(m.move_row_up(2));
}

#[test]
fn dimensions_follow_the_loaded_table() {
    // adapts: table_editor_controller.test.js:93 "updates input values to match table data"
    let m = from("| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |");
    assert_eq!(m.dimensions(), (3, 2));
}

#[test]
fn set_size_accepts_explicit_values() {
    // adapts: table_editor_controller.test.js:101 "accepts explicit values"
    let mut m = model();
    m.set_size(5, 4);
    assert_eq!(m.dimensions(), (5, 4));
    m.set_size(0, 0);
    assert_eq!(m.dimensions(), (1, 1));
}

#[test]
fn same_counts_are_no_ops() {
    // adapts: table_editor_controller.test.js:147 / :223 "no-ops when count is same as current"
    let mut m = model();
    m.set_cols(3);
    m.set_rows(3);
    assert!(!m.is_dirty());
    assert_eq!(m.revision(), 0);
    m.set_cell(0, 0, "Header 1");
    assert!(!m.is_dirty());
}

#[test]
fn steppers_add_and_remove() {
    // adapts: table_editor_controller.test.js:268/274/280/286 add/remove column/row
    let mut m = from("| a | b |\n|---|---|\n| 1 | 2 |");
    m.add_column();
    assert_eq!(m.cols(), 3);
    assert_eq!(m.cell(0, 2), "Header 3");
    m.remove_column();
    assert_eq!(m.cols(), 2);
    m.add_row();
    m.add_row();
    assert_eq!(m.rows(), 4);
    m.remove_row();
    assert_eq!(m.rows(), 3);
    assert_eq!(m.cell(0, 0), "a");
}

#[test]
fn steppers_follow_grid_edits() {
    // adapts: table_editor_controller.test.js:326 "syncs dimension inputs after rendering"
    let mut m = model();
    let before = m.revision();
    assert_eq!(m.apply(CellAction::DeleteRow, 2, 0), Some((1, 0)));
    assert_eq!(m.dimensions(), (2, 3));
    assert!(m.revision() > before);
}

#[test]
fn quoted_cells_are_kept_verbatim() {
    // adapts: table_editor_controller.test.js:351 "does not allow a quoted cell value to inject attributes"
    let mut m = model();
    m.set_cell(1, 0, "x\" onfocus=\"y | z");
    assert_eq!(m.cell(1, 0), "x\" onfocus=\"y | z");
    assert!(m.to_markdown().contains("x\" onfocus=\"y \\| z"));
}

#[test]
fn update_replaces_the_original_range() {
    // adapts: table_editor_controller.test.js:386 "dispatches frankmd:insert-table event with markdown"
    let md = "| A | B |\n|---|---|\n| 1 | 2 |";
    let m = TableEditorModel::from_markdown(md, 10..40, DEFAULT_HEADER_TEMPLATE);
    let out = m.confirm().unwrap();
    assert!(out.markdown.starts_with("| A   | B   |"));
    assert_eq!(out.mode, TableMode::Update(10..40));
}

#[test]
fn unparsable_table_falls_back_to_one_header_row() {
    // adapts: table_editor_controller.test.js:401 "closes dialog without dispatching when tableData is empty"
    // An edit-mode table that parses to nothing becomes one header row (FrankMD), so
    // confirm always has markdown for a real table; an emptied model never does.
    let m = from("|---|---|");
    assert_eq!(m.dimensions(), (1, 3));
    assert!(m.confirm().is_some());
}

#[test]
fn alignment_is_preserved_and_editable() {
    // ui-spec §5.8 (new): the original dropped alignment colons.
    let mut m = from("| L | C | R |\n| :-- | :-: | --: |\n| 1 | 2 | 3 |");
    assert_eq!(m.alignment(0), Alignment::Left);
    assert_eq!(m.alignment(1), Alignment::Center);
    assert_eq!(m.alignment(2), Alignment::Right);
    assert!(!m.is_dirty());
    assert_eq!(
        m.to_markdown(),
        "| L   | C   | R   |\n| :-- | :-: | --: |\n| 1   | 2   | 3   |"
    );
    m.set_alignment(0, Alignment::None);
    assert!(m.to_markdown().contains("| --- | :-: | --: |"));
}

#[test]
fn slash_query_inside_the_table_is_stripped_from_the_copy() {
    // adapts: app_slash_commands.test.js:117 "removes a table slash query only from the editor draft"
    let text = "| /table | value |\n|---|---|";
    let m =
        TableEditorModel::from_document(text, 4, Some((2..8, "/table")), DEFAULT_HEADER_TEMPLATE);
    assert_eq!(m.cells()[0], vec!["", "value"]);
    assert_eq!(*m.mode(), TableMode::Update(0..text.len()));
    // A stale query (document changed) is not stripped.
    let m =
        TableEditorModel::from_document(text, 4, Some((2..8, "/tabXe")), DEFAULT_HEADER_TEMPLATE);
    assert_eq!(m.cell(0, 0), "/table");
}
