//! Markdown pipe tables with column alignment and escaped pipes, for the table
//! editor dialog (ui-spec §5.8).
//!
//! [`super::table`] is the byte-exact port of FrankMD's `table_utils.js` (golden
//! tested) and keeps its quirks. The editor needs two of them fixed, so it uses
//! this module instead:
//! - **alignment is preserved**: the separator row's colons are read into
//!   [`Alignment`]s and written back (`:---`, `:---:`, `---:`); FrankMD dropped them;
//! - **escaped pipes**: `\|` inside a cell is part of the cell (loaded as `|` and
//!   written back as `\|`); FrankMD split the cell there;
//! - a row without a trailing pipe keeps its last cell.
//!
//! Everything else matches the original output (FE-388): every column padded to
//! `max(3, widest cell)` measured in UTF-16 code units, `| a | b |` rows, a
//! separator after the header, `\n` between lines and no trailing newline.

use super::{is_js_whitespace, js_trim, utf16_len};

/// Column alignment from the separator row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// `---` (renderer default).
    #[default]
    None,
    /// `:---`.
    Left,
    /// `:---:`.
    Center,
    /// `---:`.
    Right,
}

impl Alignment {
    /// The separator cell for a column of `width` (≥ 3) characters.
    pub fn separator(self, width: usize) -> String {
        let width = width.max(3);
        match self {
            Self::None => "-".repeat(width),
            Self::Left => format!(":{}", "-".repeat(width - 1)),
            Self::Center => format!(":{}:", "-".repeat(width - 2)),
            Self::Right => format!("{}:", "-".repeat(width - 1)),
        }
    }

    /// Parse one separator cell (`:--`, `-:`...); `None` if it isn't one.
    fn from_separator_cell(cell: &str) -> Option<Self> {
        let cell = js_trim(cell);
        let left = cell.starts_with(':');
        let right = cell.len() > 1 && cell.ends_with(':');
        let dashes = cell.trim_start_matches(':').trim_end_matches(':');
        if dashes.is_empty() || !dashes.chars().all(|c| c == '-') {
            return None;
        }
        Some(match (left, right) {
            (true, true) => Self::Center,
            (true, false) => Self::Left,
            (false, true) => Self::Right,
            (false, false) => Self::None,
        })
    }
}

/// A table: `rows[0]` is the header; `alignments[c]` applies to column `c`
/// (missing entries are [`Alignment::None`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlignedTable {
    /// Cell text, unescaped.
    pub rows: Vec<Vec<String>>,
    /// Per-column alignment.
    pub alignments: Vec<Alignment>,
}

impl AlignedTable {
    /// Number of columns (widest row or alignment list).
    pub fn column_count(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0).max(self.alignments.len())
    }

    /// Alignment of column `col`.
    pub fn alignment(&self, col: usize) -> Alignment {
        self.alignments.get(col).copied().unwrap_or_default()
    }
}

/// Split a table line into raw cells on unescaped `|`. The leading pipe (if
/// any) opens the row; a trailing unescaped pipe closes it.
fn split_cells(line: &str) -> Vec<&str> {
    let mut cells = Vec::new();
    let mut start = 0;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match c {
            '\\' if !escaped => escaped = true,
            '|' if !escaped => {
                cells.push(&line[start..i]);
                start = i + 1;
            }
            _ => escaped = false,
        }
        if c != '\\' {
            escaped = false;
        }
    }
    let tail = &line[start..];
    let closed = tail.trim_matches(is_js_whitespace).is_empty() && !cells.is_empty();
    if !closed {
        cells.push(tail);
    }
    if line.starts_with('|') && !cells.is_empty() {
        cells.remove(0);
    }
    cells
}

/// `\|` → `|` (other backslashes are kept).
fn unescape_cell(cell: &str) -> String {
    cell.replace("\\|", "|")
}

/// `|` → `\|` (an already escaped pipe is not escaped twice).
pub fn escape_cell(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    let mut prev_backslash = false;
    for c in cell.chars() {
        if c == '|' && !prev_backslash {
            out.push('\\');
        }
        out.push(c);
        prev_backslash = c == '\\' && !prev_backslash;
    }
    out
}

/// Parse table text (lines split on `\n`). Blank lines are skipped; the first
/// separator row sets the alignments and is not a data row.
pub fn parse_aligned_table(text: &str) -> AlignedTable {
    let mut table = AlignedTable::default();
    for line in text.split('\n') {
        let trimmed = js_trim(line);
        if trimmed.is_empty() {
            continue;
        }
        let cells = split_cells(trimmed);
        if cells.is_empty() {
            continue;
        }
        let separator: Option<Vec<Alignment>> =
            cells.iter().map(|c| Alignment::from_separator_cell(c)).collect();
        if let Some(alignments) = separator {
            if table.alignments.is_empty() {
                table.alignments = alignments;
            }
            continue;
        }
        table.rows.push(cells.iter().map(|c| unescape_cell(js_trim(c))).collect());
    }
    table
}

/// Render a table (see the module docs). No rows → `""`.
pub fn generate_aligned_table(table: &AlignedTable) -> String {
    if table.rows.is_empty() {
        return String::new();
    }
    let col_count = table.column_count();
    let escaped: Vec<Vec<String>> = table
        .rows
        .iter()
        .map(|row| (0..col_count).map(|c| row.get(c).map_or_else(String::new, |s| escape_cell(s))).collect())
        .collect();
    let widths: Vec<usize> = (0..col_count)
        .map(|c| escaped.iter().map(|row| utf16_len(&row[c])).max().unwrap_or(0).max(3))
        .collect();
    let render = |row: &[String]| {
        let cells: Vec<String> = row
            .iter()
            .zip(&widths)
            .map(|(cell, &w)| format!("{cell}{}", " ".repeat(w.saturating_sub(utf16_len(cell)))))
            .collect();
        format!("| {} |", cells.join(" | "))
    };
    let mut lines = Vec::with_capacity(escaped.len() + 1);
    lines.push(render(&escaped[0]));
    let separator: Vec<String> =
        widths.iter().enumerate().map(|(c, &w)| table.alignment(c).separator(w)).collect();
    lines.push(format!("| {} |", separator.join(" | ")));
    lines.extend(escaped[1..].iter().map(|row| render(row)));
    lines.join("\n")
}

/// **[COMPAT]** `openTableEditor` with a pending `/table` inside the table:
/// remove `query_range` (relative to `table_text`) plus one adjacent space or
/// tab, using the original's three rules (space on both sides → drop one after;
/// query at the start → drop one after; nothing after → trim spaces before).
pub fn strip_slash_query(table_text: &str, query_range: std::ops::Range<usize>) -> String {
    let (Some(before), Some(after)) = (table_text.get(..query_range.start), table_text.get(query_range.end..))
    else {
        return table_text.to_owned();
    };
    let is_blank = |c: char| c == ' ' || c == '\t';
    let mut before = before;
    let mut after = after;
    if after.starts_with(is_blank) && (before.ends_with(is_blank) || before.is_empty()) {
        after = &after[1..];
    } else if js_trim(after).is_empty() && before.ends_with(is_blank) {
        before = before.trim_end_matches(is_blank);
    }
    format!("{before}{after}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[&[&str]], alignments: &[Alignment]) -> AlignedTable {
        AlignedTable {
            rows: rows.iter().map(|r| r.iter().map(|c| (*c).to_owned()).collect()).collect(),
            alignments: alignments.to_vec(),
        }
    }

    #[test]
    fn default_table_matches_frankmd_output() {
        let t = table(&[&["Header 1", "Header 2", "Header 3"], &["", "", ""], &["", "", ""]], &[]);
        assert_eq!(
            generate_aligned_table(&t),
            "| Header 1 | Header 2 | Header 3 |\n| -------- | -------- | -------- |\n|          |          |          |\n|          |          |          |"
        );
    }

    #[test]
    fn alignment_round_trips() {
        let src = "| Name | Qty | Price |\n| :--- | :-: | ----: |\n| a | 1 | 2 |";
        let t = parse_aligned_table(src);
        assert_eq!(t.alignments, vec![Alignment::Left, Alignment::Center, Alignment::Right]);
        assert_eq!(t.rows, vec![vec!["Name", "Qty", "Price"], vec!["a", "1", "2"]]);
        let out = generate_aligned_table(&t);
        assert_eq!(out, "| Name | Qty | Price |\n| :--- | :-: | ----: |\n| a    | 1   | 2     |");
        assert_eq!(parse_aligned_table(&out), t);
    }

    #[test]
    fn escaped_pipes_stay_in_their_cell() {
        let t = parse_aligned_table("| a \\| b | c |\n| --- | --- |\n| x | y|z |");
        assert_eq!(t.rows[0], vec!["a | b", "c"]);
        assert_eq!(t.rows[1], vec!["x", "y", "z"]);
        let out = generate_aligned_table(&table(&[&["a|b"], &["1"]], &[]));
        assert_eq!(out, "| a\\|b |\n| ---- |\n| 1    |");
        assert_eq!(parse_aligned_table(&out).rows[0], vec!["a|b"]);
    }

    #[test]
    fn row_without_trailing_pipe_keeps_last_cell() {
        let t = parse_aligned_table("| a | b\n|---|---|\n| 1 | 2");
        assert_eq!(t.rows, vec![vec!["a", "b"], vec!["1", "2"]]);
    }

    #[test]
    fn quotes_are_kept_verbatim() {
        // adapts: table_editor_controller.test.js:351 "does not allow a quoted cell value to inject attributes"
        let cell = "x\" onfocus=\"y";
        let out = generate_aligned_table(&table(&[&[cell]], &[]));
        assert!(out.contains(cell));
        assert_eq!(parse_aligned_table(&out).rows[0][0], cell);
    }

    #[test]
    fn separator_cells_parse_and_render() {
        assert_eq!(Alignment::from_separator_cell(" :--: "), Some(Alignment::Center));
        assert_eq!(Alignment::from_separator_cell("-"), Some(Alignment::None));
        assert_eq!(Alignment::from_separator_cell(":"), None);
        assert_eq!(Alignment::from_separator_cell("a"), None);
        assert_eq!(Alignment::Center.separator(3), ":-:");
        assert_eq!(Alignment::Right.separator(5), "----:");
    }

    #[test]
    fn slash_query_is_removed_with_one_blank() {
        // adapts: app_slash_commands.test.js:117 "removes a table slash query only from the editor draft"
        let text = "| /table | value |";
        assert_eq!(strip_slash_query(text, 2..8), "| | value |");
        assert_eq!(strip_slash_query("/table x", 0..6), "x");
        assert_eq!(strip_slash_query("| a /table", 4..10), "| a");
        let t = parse_aligned_table(&strip_slash_query("| /table | value |\n|---|---|", 2..8));
        assert_eq!(t.rows[0], vec!["", "value"]);
    }

    #[test]
    fn escape_cell_does_not_double_escape() {
        assert_eq!(escape_cell("a|b"), "a\\|b");
        assert_eq!(escape_cell("a\\|b"), "a\\|b");
        assert_eq!(escape_cell("a\\\\|b"), "a\\\\\\|b");
    }
}
