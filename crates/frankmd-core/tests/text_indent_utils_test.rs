//! Port of test/javascript/indent_utils.test.js (lib/indent_utils.js).

use frankmd_core::text::indent::{IndentValue, indent_lines, parse_indent_setting, unindent_lines};

fn num(n: f64) -> String {
    parse_indent_setting(Some(&IndentValue::Number(n))).to_owned()
}
fn text(s: &str) -> String {
    parse_indent_setting(Some(&IndentValue::Text(s.to_owned()))).to_owned()
}

// port: test/javascript/indent_utils.test.js:8 "returns 2 spaces by default"
#[test]
fn returns_2_spaces_by_default() {
    // JS undefined and null both map to `None`.
    assert_eq!(parse_indent_setting(None), "  ");
    assert_eq!(text(""), "  ");
}

// port: test/javascript/indent_utils.test.js:14 "returns tab character for 0"
#[test]
fn returns_tab_for_0() {
    assert_eq!(num(0.0), "\t");
    assert_eq!(text("0"), "\t");
}

// port: test/javascript/indent_utils.test.js:19 "returns 1 space for 1"
#[test]
fn returns_1_space_for_1() {
    assert_eq!(num(1.0), " ");
    assert_eq!(text("1"), " ");
}

// port: test/javascript/indent_utils.test.js:24 "returns 2 spaces for 2"
#[test]
fn returns_2_spaces_for_2() {
    assert_eq!(num(2.0), "  ");
    assert_eq!(text("2"), "  ");
}

// port: test/javascript/indent_utils.test.js:29 "returns 4 spaces for 4"
#[test]
fn returns_4_spaces_for_4() {
    assert_eq!(num(4.0), "    ");
    assert_eq!(text("4"), "    ");
}

// port: test/javascript/indent_utils.test.js:34 "returns 6 spaces for 6"
#[test]
fn returns_6_spaces_for_6() {
    assert_eq!(num(6.0), "      ");
    assert_eq!(text("6"), "      ");
}

// port: test/javascript/indent_utils.test.js:39 "clamps to 6 spaces for values > 6"
#[test]
fn clamps_to_6_spaces() {
    assert_eq!(num(7.0), "      ");
    assert_eq!(num(10.0), "      ");
    assert_eq!(num(100.0), "      ");
}

// port: test/javascript/indent_utils.test.js:45 "returns default for negative values"
#[test]
fn returns_default_for_negative_values() {
    assert_eq!(num(-1.0), "  ");
    assert_eq!(num(-5.0), "  ");
}

// port: test/javascript/indent_utils.test.js:50 "returns default for non-numeric strings"
#[test]
fn returns_default_for_non_numeric_strings() {
    assert_eq!(text("abc"), "  ");
    assert_eq!(text("foo"), "  ");
}

// port: test/javascript/indent_utils.test.js:57 "adds indent to single line"
#[test]
fn adds_indent_to_single_line() {
    assert_eq!(indent_lines("hello", "  "), "  hello");
}

// port: test/javascript/indent_utils.test.js:61 "adds indent to multiple lines"
#[test]
fn adds_indent_to_multiple_lines() {
    assert_eq!(
        indent_lines("line1\nline2\nline3", "  "),
        "  line1\n  line2\n  line3"
    );
}

// port: test/javascript/indent_utils.test.js:65 "adds tab indent"
#[test]
fn adds_tab_indent() {
    assert_eq!(indent_lines("hello\nworld", "\t"), "\thello\n\tworld");
}

// port: test/javascript/indent_utils.test.js:69 "adds 4 space indent"
#[test]
fn adds_4_space_indent() {
    assert_eq!(indent_lines("code", "    "), "    code");
}

// port: test/javascript/indent_utils.test.js:73 "handles empty lines"
#[test]
fn indent_handles_empty_lines() {
    assert_eq!(indent_lines("line1\n\nline3", "  "), "  line1\n  \n  line3");
}

// port: test/javascript/indent_utils.test.js:79 "removes exact indent from single line"
#[test]
fn removes_exact_indent_from_single_line() {
    assert_eq!(unindent_lines("  hello", "  "), "hello");
}

// port: test/javascript/indent_utils.test.js:83 "removes exact indent from multiple lines"
#[test]
fn removes_exact_indent_from_multiple_lines() {
    assert_eq!(
        unindent_lines("  line1\n  line2\n  line3", "  "),
        "line1\nline2\nline3"
    );
}

// port: test/javascript/indent_utils.test.js:87 "removes tab indent"
#[test]
fn removes_tab_indent() {
    assert_eq!(unindent_lines("\thello\n\tworld", "\t"), "hello\nworld");
}

// port: test/javascript/indent_utils.test.js:91 "removes partial indent when line has fewer spaces"
#[test]
fn removes_partial_indent_when_line_has_fewer_spaces() {
    assert_eq!(unindent_lines(" hello", "    "), "hello");
    assert_eq!(unindent_lines("  hello", "    "), "hello");
    assert_eq!(unindent_lines("   hello", "    "), "hello");
}

// port: test/javascript/indent_utils.test.js:98 "does not remove non-space characters"
#[test]
fn does_not_remove_non_space_characters() {
    assert_eq!(unindent_lines("hello", "  "), "hello");
    assert_eq!(unindent_lines("xhello", "  "), "xhello");
}

// port: test/javascript/indent_utils.test.js:103 "removes tab when spaces expected but tab found"
#[test]
fn removes_tab_when_spaces_expected() {
    assert_eq!(unindent_lines("\thello", "  "), "hello");
}

// port: test/javascript/indent_utils.test.js:107 "handles mixed indentation"
#[test]
fn handles_mixed_indentation() {
    assert_eq!(
        unindent_lines("  line1\n    line2\nline3", "  "),
        "line1\n  line2\nline3"
    );
}

// port: test/javascript/indent_utils.test.js:112 "handles empty lines"
#[test]
fn unindent_handles_empty_lines() {
    assert_eq!(unindent_lines("  line1\n\n  line3", "  "), "line1\n\nline3");
}

// port: test/javascript/indent_utils.test.js:118 "indent then unindent returns original"
#[test]
fn roundtrip_indent_then_unindent() {
    let original = "line1\nline2\nline3";
    assert_eq!(
        unindent_lines(&indent_lines(original, "  "), "  "),
        original
    );
}

// port: test/javascript/indent_utils.test.js:126 "works with tabs"
#[test]
fn roundtrip_works_with_tabs() {
    let original = "code\nmore code";
    assert_eq!(
        unindent_lines(&indent_lines(original, "\t"), "\t"),
        original
    );
}

// port: test/javascript/indent_utils.test.js:134 "preserves existing indentation"
#[test]
fn roundtrip_preserves_existing_indentation() {
    let original = "  already indented\n    more indented";
    let indented = indent_lines(original, "  ");
    assert_eq!(indented, "    already indented\n      more indented");
    assert_eq!(unindent_lines(&indented, "  "), original);
}
