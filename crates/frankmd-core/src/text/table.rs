//! Markdown table model and edits (port of `lib/table_utils.js`), used by the
//! table editor dialog. A table is a list of rows of cell strings; row 0 is
//! the header. All edit functions return a new table and never mutate input.
//! Golden: `js_table_utils.json`.

/// A parsed table: `rows[0]` is the header row.
pub type TableData = Vec<Vec<String>>;

/// Header used by [`add_column`] when the caller gives none (**[COMPAT]**).
pub const DEFAULT_COLUMN_HEADER: &str = "Header";

/// Parse markdown table text (split on `\n`) into rows. See [`parse_markdown_table_lines`].
pub fn parse_markdown_table(text: &str) -> TableData {
    todo!("table_utils.js parseMarkdownTable (string input)")
}

/// Parse already-split lines. Each line is trimmed; blank lines and separator
/// rows (`^\|[\s\-:]+\|$` or `^\|(\s*:?-+:?\s*\|)+$`) are skipped; the rest is
/// split on every `|` (escaped `\|` is **not** special, **[COMPAT]**), the
/// first and last pieces dropped, cells trimmed; rows with no cells are skipped.
pub fn parse_markdown_table_lines<S: AsRef<str>>(lines: &[S]) -> TableData {
    todo!("table_utils.js parseMarkdownTable (array input)")
}

/// Render a table: rows padded to the widest row, each column padded to
/// `max(3, longest cell)`, a `---` separator after the header, lines joined
/// with `\n` (no trailing newline). Empty input → `""`.
///
/// **[COMPAT]** widths and padding are measured in UTF-16 code units (JS
/// `.length`/`padEnd`), not display columns.
pub fn generate_markdown_table(table: &[Vec<String>]) -> String {
    todo!("table_utils.js generateMarkdownTable")
}

/// Swap columns `a` and `b` in every row. Indices must be in range for the
/// rows (out-of-range swaps produced sparse JS arrays; the Rust version leaves
/// such a row unchanged).
pub fn swap_columns(table: &[Vec<String>], a: usize, b: usize) -> TableData {
    todo!("table_utils.js swapColumns")
}

/// Swap rows `a` and `b` (out of range → table unchanged).
pub fn swap_rows(table: &[Vec<String>], a: usize, b: usize) -> TableData {
    todo!("table_utils.js swapRows")
}

/// Remove column `col` from every row (rows shorter than `col` unchanged).
pub fn delete_column(table: &[Vec<String>], col: usize) -> TableData {
    todo!("table_utils.js deleteColumn")
}

/// Remove row `row` (out of range → unchanged).
pub fn delete_row(table: &[Vec<String>], row: usize) -> TableData {
    todo!("table_utils.js deleteRow")
}

/// Append a column: `header` on row 0, `""` on the others. Empty table stays empty.
pub fn add_column(table: &[Vec<String>], header: &str) -> TableData {
    todo!("table_utils.js addColumn")
}

/// Append an empty row with as many cells as the header row (`[]` → `[[]]`).
pub fn add_row(table: &[Vec<String>]) -> TableData {
    todo!("table_utils.js addRow")
}
