//! Golden conformance for js_text_utils, js_markdown_utils, js_table_utils and
//! js_indent_utils (recorded from FrankMD's lib/*.js). JS offsets are UTF-16
//! code units; the Rust API uses UTF-8 byte offsets, so positions are
//! converted on the way in and results converted back before comparing.
//!
//! Skipped cases (counted, printed with `--nocapture`):
//! - inputs carrying `$utf16` lone surrogates (not representable in `&str`);
//! - non-string arguments where the Rust API is typed (`escapeHtml(42)`,
//!   `isSafeImageUrl({})`, ...): JS-dynamic-typing artifacts;
//! - negative indices/positions (Rust takes `usize`) and swaps whose JS result is a
//!   sparse array (out-of-range indices);
//! - positions that fall inside a surrogate pair.
#![allow(clippy::unwrap_used)]

#[path = "text_common/mod.rs"]
mod text_common;

use frankmd_core::text::{indent, markdown, table, utils};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, decode_js, golden};
use std::cell::Cell;
use text_common::{arg, arg_str, byte_to_u16, has_lone_surrogates, js_len};

/// Skip counter that reports at the end of the test.
struct Skips(&'static str, Cell<usize>);
impl Skips {
    fn new(name: &'static str) -> Self {
        Self(name, Cell::new(0))
    }
    fn skip(&self) -> Result<(), String> {
        self.1.set(self.1.get() + 1);
        Ok(())
    }
}
impl Drop for Skips {
    fn drop(&mut self) {
        eprintln!("{}: skipped {} case(s)", self.0, self.1.get());
    }
}

fn eq<T: std::fmt::Debug + PartialEq>(got: T, want: T, case: &Case) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("input={} got={got:?} want={want:?}", case.input))
    }
}

fn expected(case: &Case) -> Value {
    decode_js(&case.expected)
}

fn usize_arg(input: &Value, i: usize) -> Option<usize> {
    arg(input, i).as_i64().and_then(|n| usize::try_from(n).ok())
}

/// JS UTF-16 position -> byte offset, allowing positions past the end
/// (mapped to `len + overshoot`). `None` if it splits a surrogate pair or is negative.
fn js_pos_to_byte(s: &str, pos: i64) -> Option<usize> {
    let pos = usize::try_from(pos).ok()?;
    let total = js_len(s);
    if pos >= total {
        return Some(s.len() + (pos - total));
    }
    let mut units = 0;
    for (b, ch) in s.char_indices() {
        if units == pos {
            return Some(b);
        }
        if units > pos {
            return None;
        }
        units += ch.len_utf16();
    }
    None
}

// ---------------------------------------------------------------- text_utils

#[test]
fn golden_text_utils() {
    let g = golden("js_text_utils");
    let skips = Skips::new("js_text_utils");
    assert_cases("js_text_utils", &g.cases, |case| {
        if has_lone_surrogates(&case.input) {
            return skips.skip();
        }
        let a0 = arg(&case.input, 0);
        match case.func.as_deref().unwrap() {
            f @ ("escapeHtml" | "escapeHtmlString") => {
                let text = match &a0 {
                    Value::Null => None,
                    Value::String(s) => Some(s.as_str()),
                    _ => return skips.skip(), // escapeHtml(42/0/false)
                };
                let got = if f == "escapeHtml" {
                    utils::escape_html(text)
                } else {
                    utils::escape_html_string(text)
                };
                eq(got, expected(case).as_str().unwrap().to_owned(), case)
            }
            "isSafeImageUrl" => {
                let Some(url) = a0.as_str() else {
                    return skips.skip();
                };
                eq(
                    utils::is_safe_image_url(url),
                    expected(case).as_bool().unwrap(),
                    case,
                )
            }
            "fuzzyScore" => {
                let got = utils::fuzzy_score(&arg_str(&case.input, 0), &arg_str(&case.input, 1));
                eq(u64::from(got), expected(case).as_u64().unwrap(), case)
            }
            "levenshteinDistance" => {
                let got =
                    utils::levenshtein_distance(&arg_str(&case.input, 0), &arg_str(&case.input, 1));
                eq(got as u64, expected(case).as_u64().unwrap(), case)
            }
            other => Err(format!("unknown fn {other}")),
        }
    });
}

// ------------------------------------------------------------ markdown_utils

fn table_to_json(text: &str, t: &markdown::TableAtPosition) -> Value {
    json!({
        "startLine": t.start_line,
        "endLine": t.end_line,
        "startPos": byte_to_u16(text, t.start_pos),
        "endPos": byte_to_u16(text, t.end_pos),
        "lines": t.lines,
    })
}

fn block_to_json(text: &str, b: &markdown::CodeBlockAtPosition) -> Value {
    json!({
        "startPos": byte_to_u16(text, b.start_pos),
        "endPos": byte_to_u16(text, b.end_pos),
        "language": b.language,
        "content": b.content,
    })
}

#[test]
fn golden_markdown_utils() {
    let g = golden("js_markdown_utils");
    let skips = Skips::new("js_markdown_utils");
    assert_cases("js_markdown_utils", &g.cases, |case| {
        if has_lone_surrogates(&case.input) {
            return skips.skip();
        }
        let text = arg_str(&case.input, 0);
        let Some(pos) = js_pos_to_byte(&text, arg(&case.input, 1).as_i64().unwrap()) else {
            return skips.skip(); // negative or mid-surrogate position
        };
        let got = match case.func.as_deref().unwrap() {
            "findTableAtPosition" => markdown::find_table_at_position(&text, pos)
                .map_or(Value::Null, |t| table_to_json(&text, &t)),
            "findCodeBlockAtPosition" => markdown::find_code_block_at_position(&text, pos)
                .map_or(Value::Null, |b| block_to_json(&text, &b)),
            other => return Err(format!("unknown fn {other}")),
        };
        eq(got, expected(case), case)
    });
}

// --------------------------------------------------------------- table_utils

/// A JS table argument. `None` if it is not a plain array of string arrays.
fn to_table(v: &Value) -> Option<table::TableData> {
    v.as_array()?
        .iter()
        .map(|row| {
            row.as_array()?
                .iter()
                .map(|c| c.as_str().map(str::to_owned))
                .collect()
        })
        .collect()
}

fn table_json(t: &table::TableData) -> Value {
    json!(t)
}

#[test]
fn golden_table_utils() {
    let g = golden("js_table_utils");
    let skips = Skips::new("js_table_utils");
    assert_cases("js_table_utils", &g.cases, |case| {
        if has_lone_surrogates(&case.input) {
            return skips.skip();
        }
        let a0 = arg(&case.input, 0);
        let want = expected(case);
        let f = case.func.as_deref().unwrap();
        if f == "parseMarkdownTable" {
            let got = match &a0 {
                Value::String(s) => table::parse_markdown_table(s),
                Value::Array(lines) => {
                    let lines: Vec<&str> = lines.iter().map(|l| l.as_str().unwrap()).collect();
                    table::parse_markdown_table_lines(&lines)
                }
                _ => return skips.skip(),
            };
            return eq(table_json(&got), want, case);
        }
        if f == "generateMarkdownTable" {
            // JS null/undefined -> no data.
            let data = if a0.is_null() {
                Vec::new()
            } else {
                to_table(&a0).unwrap()
            };
            return eq(
                table::generate_markdown_table(&data),
                want.as_str().unwrap().to_owned(),
                case,
            );
        }
        let data = to_table(&a0).unwrap();
        // Sparse-array / negative-index results are JS artifacts with no Rust equivalent.
        if to_table(&want).is_none() {
            return skips.skip();
        }
        let got = match f {
            "swapColumns" | "swapRows" => {
                let (Some(a), Some(b)) = (usize_arg(&case.input, 1), usize_arg(&case.input, 2))
                else {
                    return skips.skip();
                };
                if f == "swapColumns" {
                    table::swap_columns(&data, a, b)
                } else {
                    table::swap_rows(&data, a, b)
                }
            }
            "deleteColumn" | "deleteRow" => {
                let Some(i) = usize_arg(&case.input, 1) else {
                    return skips.skip();
                };
                if f == "deleteColumn" {
                    table::delete_column(&data, i)
                } else {
                    table::delete_row(&data, i)
                }
            }
            "addColumn" => {
                let header = arg(&case.input, 1);
                table::add_column(
                    &data,
                    header.as_str().unwrap_or(table::DEFAULT_COLUMN_HEADER),
                )
            }
            "addRow" => table::add_row(&data),
            other => return Err(format!("unknown fn {other}")),
        };
        eq(table_json(&got), want, case)
    });
}

// -------------------------------------------------------------- indent_utils

fn indent_value(v: &Value) -> Option<indent::IndentValue> {
    match v {
        Value::Null => None,
        Value::Bool(b) => Some(indent::IndentValue::Bool(*b)),
        Value::Number(n) => Some(indent::IndentValue::Number(n.as_f64().unwrap())),
        Value::String(s) => Some(indent::IndentValue::Text(s.clone())),
        other => panic!("unexpected indent value {other}"),
    }
}

#[test]
fn golden_indent_utils() {
    let g = golden("js_indent_utils");
    let skips = Skips::new("js_indent_utils");
    assert_cases("js_indent_utils", &g.cases, |case| {
        if has_lone_surrogates(&case.input) {
            return skips.skip();
        }
        let want = expected(case).as_str().unwrap().to_owned();
        match case.func.as_deref().unwrap() {
            "parseIndentSetting" => {
                let raw = case.input.get(0).unwrap();
                let value = if raw.get("$number").is_some() {
                    let s = raw["$number"].as_str().unwrap();
                    Some(indent::IndentValue::Number(match s {
                        "NaN" => f64::NAN,
                        "Infinity" => f64::INFINITY,
                        "-Infinity" => f64::NEG_INFINITY,
                        _ => -0.0,
                    }))
                } else {
                    indent_value(&decode_js(raw))
                };
                eq(
                    indent::parse_indent_setting(value.as_ref()).to_owned(),
                    want,
                    case,
                )
            }
            "indentLines" => eq(
                indent::indent_lines(&arg_str(&case.input, 0), &arg_str(&case.input, 1)),
                want,
                case,
            ),
            "unindentLines" => eq(
                indent::unindent_lines(&arg_str(&case.input, 0), &arg_str(&case.input, 1)),
                want,
                case,
            ),
            other => Err(format!("unknown fn {other}")),
        }
    });
}
