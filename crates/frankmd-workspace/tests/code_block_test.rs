//! Tests for the code block dialog view model.

use frankmd_workspace::code_block::*;

fn insert(markdown: &str, cursor_offset: usize) -> CodeBlockOutcome {
    CodeBlockOutcome::Insert { markdown: markdown.into(), cursor_offset, mode: CodeBlockMode::Insert }
}

// adapts: code_dialog_controller.test.js:54 "initializes code state"
#[test]
fn fresh_form_is_insert_mode() {
    let f = CodeBlockForm::new_insert();
    assert_eq!(f.mode(), &CodeBlockMode::Insert);
    assert_eq!(f.language(), "");
    assert_eq!(f.content(), "");
    assert_eq!(f.primary_label_key(), "dialogs.code.insert_code");
}

// adapts: code_dialog_controller.test.js:60 "initializes code languages array"
#[test]
fn languages_list() {
    let f = CodeBlockForm::new_insert();
    let l = f.languages();
    assert_eq!(l.len(), 44);
    assert_eq!(l[0], "javascript");
    assert_eq!(l[43], "plaintext");
    for x in ["javascript", "python", "ruby"] {
        assert!(l.contains(&x));
    }
}

// adapts: code_dialog_controller.test.js:77 "opens dialog with existing code block for edit mode"
#[test]
fn caret_in_fence_is_update_mode() {
    let text = "intro\n```javascript\nconst x = 1;\n```\nafter";
    let f = CodeBlockForm::from_document(text, 25);
    assert!(matches!(f.mode(), CodeBlockMode::Update(r) if r.start == 6));
    assert_eq!(f.language(), "javascript");
    assert_eq!(f.content().trim_end(), "const x = 1;");
    assert_eq!(f.primary_label_key(), "dialogs.code.update_code");
    assert_eq!(CodeBlockForm::from_document(text, 2).mode(), &CodeBlockMode::Insert);
}

#[test]
fn unchanged_edit_round_trips() {
    let text = "a\n```rust\nfn main() {}\n```\nb";
    let mut f = CodeBlockForm::from_document(text, 8);
    let CodeBlockMode::Update(range) = f.mode().clone() else { panic!() };
    let CodeBlockOutcome::Insert { markdown, .. } = f.confirm() else { panic!() };
    assert_eq!(&text[range], markdown);
}

// adapts: code_dialog_controller.test.js:128 "selects first suggestion on Tab when suggestions visible"
#[test]
fn tab_takes_first_suggestion() {
    let mut f = CodeBlockForm::new_insert();
    f.set_language("pyth");
    assert!(f.suggestions_visible());
    assert!(f.key_tab());
    assert_eq!(f.language(), "python");
    assert!(!f.suggestions_visible());
    assert!(!f.key_tab());
}

// adapts: code_dialog_controller.test.js:141 "hides suggestions on Escape"
#[test]
fn hide_suggestions_keeps_rows() {
    let mut f = CodeBlockForm::new_insert();
    f.set_language("java");
    f.hide_suggestions();
    assert!(!f.suggestions_visible());
    assert!(!f.suggestions().is_empty());
}

// adapts: code_dialog_controller.test.js:171 "inserts spaces on Tab"
#[test]
fn tab_inserts_two_spaces() {
    let mut f = CodeBlockForm::new_insert();
    f.set_content("line1");
    assert_eq!(f.indent_at_caret(5), 7);
    assert_eq!(f.content(), "line1  ");
}

// adapts: code_dialog_controller.test.js:184 "inserts tabs when useTabs is checked"
#[test]
fn tab_inserts_tab_when_enabled() {
    let mut f = CodeBlockForm::new_insert();
    f.set_use_tabs(true);
    f.set_content("line1");
    assert_eq!(f.indent_at_caret(5), 6);
    assert_eq!(f.content(), "line1\t");
}

// adapts: code_dialog_controller.test.js:199 "populates suggestions container"
#[test]
fn suggestions_for_java() {
    let mut f = CodeBlockForm::new_insert();
    f.set_language("java");
    assert_eq!(f.suggestions(), &["javascript", "java"]);
    assert!(f.suggestions_visible());
}

#[test]
fn suggestions_hidden_on_exact_or_empty_and_capped() {
    let mut f = CodeBlockForm::new_insert();
    f.set_language("python");
    assert!(!f.suggestions_visible());
    f.set_language("  ");
    assert!(!f.suggestions_visible());
    f.set_language("s");
    assert_eq!(f.suggestions().len(), MAX_SUGGESTIONS);
}

// adapts: code_dialog_controller.test.js:217 "sets language value from clicked suggestion"
#[test]
fn select_suggestion_sets_language() {
    let mut f = CodeBlockForm::new_insert();
    f.set_language("py");
    f.select_suggestion("python");
    assert_eq!(f.language(), "python");
    assert!(!f.suggestions_visible());
}

// adapts: code_dialog_controller.test.js:280 "shows confirmation for unrecognized language"
#[test]
fn unknown_language_warns() {
    let mut f = CodeBlockForm::new_insert();
    f.set_language("pyhton");
    f.set_content("code");
    let out = f.confirm();
    assert_eq!(
        out,
        CodeBlockOutcome::UnknownLanguage { typed: "pyhton".into(), did_you_mean: Some("python") }
    );
    assert_eq!(f.warning(), Some(&out));
    f.set_language("python");
    assert!(f.warning().is_none());
}

#[test]
fn unknown_without_close_match() {
    let mut f = CodeBlockForm::new_insert();
    f.set_language("zzzzzzzz");
    assert!(matches!(f.confirm(), CodeBlockOutcome::UnknownLanguage { did_you_mean: None, .. }));
    f.set_language("rubyx");
    assert!(matches!(f.confirm(), CodeBlockOutcome::UnknownLanguage { did_you_mean: Some("ruby"), .. }));
    f.set_language("jav");
    assert!(matches!(f.confirm(), CodeBlockOutcome::UnknownLanguage { did_you_mean: Some("javascript"), .. }));
}

// adapts: code_dialog_controller.test.js:289 "does not insert when confirmation is cancelled"
#[test]
fn warning_choices() {
    let mk = || {
        let mut f = CodeBlockForm::new_insert();
        f.set_language("pyhton");
        f.set_content("code");
        f.confirm();
        f
    };
    assert!(mk().warning().is_some());
    assert_eq!(mk().use_suggested(), Some(insert("```python\ncode\n```", 10)));
    let mut f = mk();
    assert_eq!(f.keep_typed(), Some(insert("```pyhton\ncode\n```", 10)));
    assert!(f.warning().is_none());
    assert_eq!(CodeBlockForm::new_insert().keep_typed(), None);
}

// adapts: code_dialog_controller.test.js:300 "closes dialog after insert"
#[test]
fn known_language_inserts() {
    let mut f = CodeBlockForm::new_insert();
    f.set_language(" JavaScript ");
    f.set_content("x");
    assert_eq!(f.confirm(), insert("```JavaScript\nx\n```", 14));
    assert!(f.warning().is_none());
}

#[test]
fn fence_building() {
    assert_eq!(build_fence("", ""), "```\n\n```");
    assert_eq!(build_fence("rs", "a\n"), "```rs\na\n```");
    assert_eq!(build_fence("rs", "a"), "```rs\na\n```");
}

#[test]
fn indent_and_outdent_lines() {
    let mut f = CodeBlockForm::new_insert();
    f.set_content("a\nb\nc");
    assert_eq!(f.indent_lines(1..3), 0..7);
    assert_eq!(f.content(), "  a\n  b\nc");
    assert_eq!(f.outdent(1..6), 0..3);
    assert_eq!(f.content(), "a\nb\nc");
}

#[test]
fn outdent_caret_line() {
    let mut f = CodeBlockForm::new_insert();
    f.set_content("x\n    y");
    assert_eq!(f.outdent(7..7), 5..5);
    assert_eq!(f.content(), "x\n  y");
    f.set_content("x\n y");
    assert_eq!(f.outdent(4..4), 3..3);
    assert_eq!(f.content(), "x\ny");
}
