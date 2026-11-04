//! Port of test/javascript/table_utils.test.js (lib/table_utils.js).

use frankmd_core::text::table::{
    TableData, add_column, add_row, delete_column, delete_row, generate_markdown_table,
    parse_markdown_table, parse_markdown_table_lines, swap_columns, swap_rows,
};

fn t(rows: &[&[&str]]) -> TableData {
    rows.iter().map(|r| r.iter().map(|c| (*c).to_owned()).collect()).collect()
}

// port: test/javascript/table_utils.test.js:14 "parses a simple markdown table"
#[test]
fn parses_a_simple_markdown_table() {
    let md = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 25 |";
    assert_eq!(parse_markdown_table(md), t(&[&["Name", "Age"], &["Alice", "30"], &["Bob", "25"]]));
}

// port: test/javascript/table_utils.test.js:29 "handles table with extra whitespace"
#[test]
fn handles_table_with_extra_whitespace() {
    let md = "|  Name  |  Age  |\n|--------|-------|\n|  Alice |  30   |";
    assert_eq!(parse_markdown_table(md), t(&[&["Name", "Age"], &["Alice", "30"]]));
}

// port: test/javascript/table_utils.test.js:42 "skips empty lines"
#[test]
fn skips_empty_lines() {
    let md = "| Header |\n\n| --- |\n\n| Value |";
    assert_eq!(parse_markdown_table(md), t(&[&["Header"], &["Value"]]));
}

// port: test/javascript/table_utils.test.js:57 "handles array input"
#[test]
fn handles_array_input() {
    let lines = ["| A | B |", "| --- | --- |", "| 1 | 2 |"];
    assert_eq!(parse_markdown_table_lines(&lines), t(&[&["A", "B"], &["1", "2"]]));
}

// port: test/javascript/table_utils.test.js:72 "skips separator rows with colons (alignment)"
#[test]
fn skips_separator_rows_with_colons() {
    let md = "| Left | Center | Right |\n|:-----|:------:|------:|\n| L    | C      | R     |";
    assert_eq!(
        parse_markdown_table(md),
        t(&[&["Left", "Center", "Right"], &["L", "C", "R"]])
    );
}

// port: test/javascript/table_utils.test.js:85 "returns empty array for empty input"
#[test]
fn returns_empty_for_empty_input() {
    assert_eq!(parse_markdown_table(""), TableData::new());
    let none: [&str; 0] = [];
    assert_eq!(parse_markdown_table_lines(&none), TableData::new());
}

// port: test/javascript/table_utils.test.js:90 "handles cells with special characters"
#[test]
fn handles_cells_with_special_characters() {
    let md = "| Code | Description |\n| --- | --- |\n| `foo` | A *bold* statement |";
    assert_eq!(
        parse_markdown_table(md),
        t(&[&["Code", "Description"], &["`foo`", "A *bold* statement"]])
    );
}

// port: test/javascript/table_utils.test.js:105 "generates a properly formatted table"
#[test]
fn generates_a_properly_formatted_table() {
    let data = t(&[&["Name", "Age"], &["Alice", "30"], &["Bob", "25"]]);
    assert_eq!(
        generate_markdown_table(&data),
        "| Name  | Age |\n| ----- | --- |\n| Alice | 30  |\n| Bob   | 25  |"
    );
}

// port: test/javascript/table_utils.test.js:120 "pads columns to consistent width"
#[test]
fn pads_columns_to_consistent_width() {
    let out = generate_markdown_table(&t(&[&["A", "LongerHeader"], &["Short", "X"]]));
    let lines: Vec<&str> = out.split('\n').collect();
    assert_eq!(lines[0].len(), lines[1].len());
    assert_eq!(lines[1].len(), lines[2].len());
}

// port: test/javascript/table_utils.test.js:134 "handles empty cells"
#[test]
fn handles_empty_cells() {
    let out = generate_markdown_table(&t(&[&["A", "B", "C"], &["1", "", "3"]]));
    assert!(out.contains("|     |"), "{out}");
}

// port: test/javascript/table_utils.test.js:146 "returns empty string for empty data"
#[test]
fn returns_empty_string_for_empty_data() {
    // JS also passed null/undefined; the Rust API takes a slice, so "no data" is `&[]`.
    assert_eq!(generate_markdown_table(&[]), "");
}

// port: test/javascript/table_utils.test.js:152 "normalizes uneven row lengths"
#[test]
fn normalizes_uneven_row_lengths() {
    let out = generate_markdown_table(&t(&[&["A", "B", "C"], &["1", "2"]]));
    let lines: Vec<&str> = out.split('\n').collect();
    assert_eq!(lines[2].split('|').count(), 5);
}

// port: test/javascript/table_utils.test.js:165 "minimum column width is 3 characters"
#[test]
fn minimum_column_width_is_3() {
    let out = generate_markdown_table(&t(&[&["A", "B"], &["1", "2"]]));
    assert!(out.contains("| --- | --- |"), "{out}");
}

// port: test/javascript/table_utils.test.js:179 "reconstructs table after parsing"
#[test]
fn roundtrip_reconstructs_table_after_parsing() {
    let data = t(&[
        &["Product", "Price", "Quantity"],
        &["Apple", "$1.50", "10"],
        &["Banana", "$0.75", "25"],
    ]);
    assert_eq!(parse_markdown_table(&generate_markdown_table(&data)), data);
}

// port: test/javascript/table_utils.test.js:194 "swaps two columns"
#[test]
fn swaps_two_columns() {
    let data = t(&[&["A", "B", "C"], &["1", "2", "3"]]);
    assert_eq!(swap_columns(&data, 0, 2), t(&[&["C", "B", "A"], &["3", "2", "1"]]));
}

// port: test/javascript/table_utils.test.js:208 "does not mutate original array"
#[test]
fn swap_columns_does_not_mutate_original() {
    // Rust enforces this through `&[..]`; still check the input is intact and the output differs.
    let data = t(&[&["A", "B"]]);
    let out = swap_columns(&data, 0, 1);
    assert_eq!(data, t(&[&["A", "B"]]));
    assert_eq!(out, t(&[&["B", "A"]]));
}

// port: test/javascript/table_utils.test.js:216 "swaps two rows"
#[test]
fn swaps_two_rows() {
    let data = t(&[&["Header"], &["Row1"], &["Row2"]]);
    assert_eq!(swap_rows(&data, 1, 2), t(&[&["Header"], &["Row2"], &["Row1"]]));
}

// port: test/javascript/table_utils.test.js:234 "removes a column"
#[test]
fn removes_a_column() {
    let data = t(&[&["A", "B", "C"], &["1", "2", "3"]]);
    assert_eq!(delete_column(&data, 1), t(&[&["A", "C"], &["1", "3"]]));
}

// port: test/javascript/table_utils.test.js:250 "removes a row"
#[test]
fn removes_a_row() {
    let data = t(&[&["Header"], &["Row1"], &["Row2"]]);
    assert_eq!(delete_row(&data, 1), t(&[&["Header"], &["Row2"]]));
}

// port: test/javascript/table_utils.test.js:267 "adds a column with header"
#[test]
fn adds_a_column_with_header() {
    let data = t(&[&["A", "B"], &["1", "2"]]);
    assert_eq!(add_column(&data, "New"), t(&[&["A", "B", "New"], &["1", "2", ""]]));
}

// port: test/javascript/table_utils.test.js:283 "adds an empty row"
#[test]
fn adds_an_empty_row() {
    let data = t(&[&["A", "B", "C"], &["1", "2", "3"]]);
    assert_eq!(
        add_row(&data),
        t(&[&["A", "B", "C"], &["1", "2", "3"], &["", "", ""]])
    );
}

// Rust-side decision (no JS original): out-of-range swaps produced sparse JS arrays
// (golden cases skipped); the port leaves the table unchanged instead.
#[test]
fn out_of_range_swaps_leave_table_unchanged() {
    let data = t(&[&["Name", "Age"], &["Alice", "30"]]);
    assert_eq!(swap_columns(&data, 0, 5), data);
    assert_eq!(swap_rows(&data, 0, 9), data);
}
