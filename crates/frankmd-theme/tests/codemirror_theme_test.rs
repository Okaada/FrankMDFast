//! `test/javascript/lib/codemirror_theme.test.js` (8 tests, all category C:
//! CodeMirror extension objects). Three carry a portable contract (defaults,
//! autocomplete height, highlight rules) and are adapted to
//! `frankmd_theme::editor`; the other five only check JS extension shape.

use frankmd_theme::editor::{AUTOCOMPLETE_MAX_HEIGHT_EM, EditorFont, markdown_highlights};

// skip: test/javascript/lib/codemirror_theme.test.js:12 "returns an extension"                      [C] CodeMirror Extension shape
// skip: test/javascript/lib/codemirror_theme.test.js:17 "accepts font options"                      [C] options are a plain struct here
// skip: test/javascript/lib/codemirror_theme.test.js:59 "returns an array of extensions"            [C]
// skip: test/javascript/lib/codemirror_theme.test.js:65 "includes both editor theme and syntax highlighting" [C]
// skip: test/javascript/lib/codemirror_theme.test.js:70 "accepts and passes options to editor theme" [C]

// port: test/javascript/lib/codemirror_theme.test.js:26 "uses default values when no options provided"
#[test]
fn uses_default_values_when_no_options_provided() {
    let f = EditorFont::default();
    assert_eq!(f.family, "'Cascadia Code', monospace");
    assert!((f.size_px - 14.0).abs() < f32::EPSILON);
    assert!((f.line_height - 1.6).abs() < f32::EPSILON);
}

// port: test/javascript/lib/codemirror_theme.test.js:31 "lets autocomplete results grow to 16em before scrolling"
#[test]
fn lets_autocomplete_results_grow_to_16em_before_scrolling() {
    assert!((AUTOCOMPLETE_MAX_HEIGHT_EM - 16.0).abs() < f32::EPSILON);
}

// port: test/javascript/lib/codemirror_theme.test.js:52 "returns a syntax highlighting extension"
#[test]
fn returns_a_syntax_highlighting_extension() {
    let rules = markdown_highlights();
    assert!(!rules.is_empty());
    for r in rules {
        assert!(
            frankmd_theme::COLOR_KEYS.contains(&r.color),
            "{} uses unknown token {}",
            r.tag,
            r.color
        );
    }
}
