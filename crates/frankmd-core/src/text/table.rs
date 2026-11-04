//! Markdown table model and edits (port of `lib/table_utils.js`), used by the
//! table editor dialog. A table is a list of rows of cell strings; row 0 is
//! the header. All edit functions return a new table and never mutate input.
//! Golden: `js_table_utils.json`.

use super::{is_js_whitespace, js_trim, utf16_len};

/// A parsed table: `rows[0]` is the header row.
pub type TableData = Vec<Vec<String>>;

/// Header used by [`add_column`] when the caller gives none (**[COMPAT]**).
pub const DEFAULT_COLUMN_HEADER: &str = "Header";

/// Parse markdown table text (split on `\n`) into rows. See [`parse_markdown_table_lines`].
pub fn parse_markdown_table(text: &str) -> TableData {
    let lines: Vec<&str> = text.split('\n').collect();
    parse_markdown_table_lines(&lines)
}

/// Parse already-split lines. Each line is trimmed; blank lines and separator
/// rows (`^\|[\s\-:]+\|$` or `^\|(\s*:?-+:?\s*\|)+$`) are skipped; the rest is
/// split on every `|` (escaped `\|` is **not** special, **[COMPAT]**), the
/// first and last pieces dropped, cells trimmed; rows with no cells are skipped.
pub fn parse_markdown_table_lines<S: AsRef<str>>(lines: &[S]) -> TableData {
    let mut rows = Vec::new();
    for line in lines {
        let trimmed = js_trim(line.as_ref());
        if trimmed.is_empty() || is_separator_row(trimmed) {
            continue;
        }
        let pieces: Vec<&str> = trimmed.split('|').collect();
        if pieces.len() < 3 {
            // `split("|").slice(1, -1)` leaves nothing.
            continue;
        }
        rows.push(
            pieces[1..pieces.len() - 1]
                .iter()
                .map(|cell| js_trim(cell).to_owned())
                .collect(),
        );
    }
    rows
}

/// Render a table: rows padded to the widest row, each column padded to
/// `max(3, longest cell)`, a `---` separator after the header, lines joined
/// with `\n` (no trailing newline). Empty input → `""`.
///
/// **[COMPAT]** widths and padding are measured in UTF-16 code units (JS
/// `.length`/`padEnd`), not display columns.
pub fn generate_markdown_table(table: &[Vec<String>]) -> String {
    if table.is_empty() {
        return String::new();
    }
    let col_count = table.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..col_count)
        .map(|col| {
            table
                .iter()
                .map(|row| utf16_len(cell(row, col)))
                .max()
                .unwrap_or(0)
                .max(3)
        })
        .collect();

    let render_row = |row: &[String]| {
        let cells: Vec<String> = widths
            .iter()
            .enumerate()
            .map(|(col, &width)| pad_end(cell(row, col), width))
            .collect();
        format!("| {} |", cells.join(" | "))
    };

    let mut lines = Vec::with_capacity(table.len() + 1);
    lines.push(render_row(&table[0]));
    let separator: Vec<String> = widths.iter().map(|&w| "-".repeat(w)).collect();
    lines.push(format!("| {} |", separator.join(" | ")));
    lines.extend(table[1..].iter().map(|row| render_row(row)));
    lines.join("\n")
}

/// Swap columns `a` and `b` in every row. Indices must be in range for the
/// rows (out-of-range swaps produced sparse JS arrays; the Rust version leaves
/// such a row unchanged).
pub fn swap_columns(table: &[Vec<String>], a: usize, b: usize) -> TableData {
    table
        .iter()
        .map(|row| {
            let mut row = row.clone();
            if a < row.len() && b < row.len() {
                row.swap(a, b);
            }
            row
        })
        .collect()
}

/// Swap rows `a` and `b` (out of range → table unchanged).
pub fn swap_rows(table: &[Vec<String>], a: usize, b: usize) -> TableData {
    let mut rows = table.to_vec();
    if a < rows.len() && b < rows.len() {
        rows.swap(a, b);
    }
    rows
}

/// Remove column `col` from every row (rows shorter than `col` unchanged).
pub fn delete_column(table: &[Vec<String>], col: usize) -> TableData {
    table
        .iter()
        .map(|row| {
            let mut row = row.clone();
            if col < row.len() {
                row.remove(col);
            }
            row
        })
        .collect()
}

/// Remove row `row` (out of range → unchanged).
pub fn delete_row(table: &[Vec<String>], row: usize) -> TableData {
    let mut rows = table.to_vec();
    if row < rows.len() {
        rows.remove(row);
    }
    rows
}

/// Append a column: `header` on row 0, `""` on the others. Empty table stays empty.
pub fn add_column(table: &[Vec<String>], header: &str) -> TableData {
    table
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut row = row.clone();
            row.push(if i == 0 {
                header.to_owned()
            } else {
                String::new()
            });
            row
        })
        .collect()
}

/// Append an empty row with as many cells as the header row (`[]` → `[[]]`).
pub fn add_row(table: &[Vec<String>]) -> TableData {
    let col_count = table.first().map_or(0, Vec::len);
    let mut rows = table.to_vec();
    rows.push(vec![String::new(); col_count]);
    rows
}

/// Cell `col` of `row`, `""` when the row is short (JS `row[col] || ""`).
fn cell(row: &[String], col: usize) -> &str {
    row.get(col).map_or("", String::as_str)
}

/// JS `padEnd(width)`: append spaces until the UTF-16 length reaches `width`.
fn pad_end(cell: &str, width: usize) -> String {
    let missing = width.saturating_sub(utf16_len(cell));
    format!("{cell}{}", " ".repeat(missing))
}

/// Separator rows skipped by the parser: `^\|[\s\-:]+\|$` or
/// `^\|(\s*:?-+:?\s*\|)+$`.
fn is_separator_row(line: &str) -> bool {
    let Some(inner) = line.strip_prefix('|') else {
        return false;
    };
    let simple = inner.strip_suffix('|').is_some_and(|mid| {
        !mid.is_empty()
            && mid
                .chars()
                .all(|c| c == '-' || c == ':' || is_js_whitespace(c))
    });
    simple || is_aligned_cells(inner)
}

/// `(\s*:?-+:?\s*\|)+$` over the text after the leading `|`.
fn is_aligned_cells(mut rest: &str) -> bool {
    let mut cells = 0;
    while !rest.is_empty() {
        rest = rest.trim_start_matches(is_js_whitespace);
        rest = rest.strip_prefix(':').unwrap_or(rest);
        let after_dashes = rest.trim_start_matches('-');
        if after_dashes.len() == rest.len() {
            return false;
        }
        rest = after_dashes.strip_prefix(':').unwrap_or(after_dashes);
        rest = rest.trim_start_matches(is_js_whitespace);
        match rest.strip_prefix('|') {
            Some(next) => rest = next,
            None => return false,
        }
        cells += 1;
    }
    cells > 0
}
