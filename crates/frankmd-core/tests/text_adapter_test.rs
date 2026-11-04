//! Port of test/javascript/lib/codemirror_adapter.test.js.
#![allow(clippy::unwrap_used)]

#[path = "text_common/fake_editor.rs"]
mod fake_editor;

use std::cell::RefCell;
use std::rc::Rc;

use fake_editor::{Call, FakeEditor};
use frankmd_core::text::insertion::{
    CursorInfo, CursorPosition, EditorSurface, InputEvent, Selection, TextareaAdapter,
    get_cursor_info, get_cursor_position, get_editor_content, get_editor_selection,
};

fn adapter(ed: &mut FakeEditor) -> TextareaAdapter<'_> {
    TextareaAdapter::new(Some(ed as &mut dyn EditorSurface)).expect("adapter for an editor")
}

fn sel(from: usize, to: usize, text: &str) -> Selection {
    Selection {
        from,
        to,
        text: text.to_owned(),
    }
}

// ---- createTextareaAdapter ----------------------------------------------------

// port: test/javascript/lib/codemirror_adapter.test.js:12 "returns null when controller is null"
#[test]
fn adapter_none_when_controller_is_null() {
    assert!(TextareaAdapter::new(None).is_none());
}

// port: test/javascript/lib/codemirror_adapter.test.js:16 "returns null when controller is undefined"
// (null and undefined both map to `None` in Rust.)
#[test]
fn adapter_none_when_controller_is_undefined() {
    let missing: Option<&mut dyn EditorSurface> = Option::default();
    assert!(TextareaAdapter::new(missing).is_none());
}

// port: test/javascript/lib/codemirror_adapter.test.js:20 "creates adapter with value getter"
#[test]
fn adapter_value_getter() {
    let mut ed = FakeEditor::with_text("test content");
    {
        let a = adapter(&mut ed);
        assert_eq!(a.value(), "test content");
    }
    assert!(ed.text_reads.get() > 0, "getValue called");
}

// port: test/javascript/lib/codemirror_adapter.test.js:30 "creates adapter with value setter"
#[test]
fn adapter_value_setter() {
    let mut ed = FakeEditor::default();
    adapter(&mut ed).set_value("new content");
    assert_eq!(ed.calls, vec![Call::SetText("new content".into())]);
}

// port: test/javascript/lib/codemirror_adapter.test.js:41 "creates adapter with selectionStart getter"
#[test]
fn adapter_selection_start_getter() {
    let mut ed = FakeEditor {
        selection: sel(5, 10, "hello"),
        ..FakeEditor::default()
    };
    assert_eq!(adapter(&mut ed).selection_start(), 5);
}

// port: test/javascript/lib/codemirror_adapter.test.js:51 "creates adapter with selectionEnd getter"
#[test]
fn adapter_selection_end_getter() {
    let mut ed = FakeEditor {
        selection: sel(5, 10, "hello"),
        ..FakeEditor::default()
    };
    assert_eq!(adapter(&mut ed).selection_end(), 10);
}

// port: test/javascript/lib/codemirror_adapter.test.js:61 "creates adapter with setSelectionRange method"
#[test]
fn adapter_set_selection_range() {
    let mut ed = FakeEditor::default();
    adapter(&mut ed).set_selection_range(3, 7);
    assert_eq!(ed.calls, vec![Call::SetSelection(3, 7)]);
}

// port: test/javascript/lib/codemirror_adapter.test.js:72 "creates adapter with focus method"
#[test]
fn adapter_focus() {
    let mut ed = FakeEditor::default();
    adapter(&mut ed).focus();
    assert_eq!(ed.calls, vec![Call::Focus]);
}

// port: test/javascript/lib/codemirror_adapter.test.js:83 "handles addEventListener for input events"
#[test]
fn adapter_add_input_listener() {
    let mut ed = FakeEditor::default();
    let mut a = adapter(&mut ed);
    a.add_input_listener(Box::new(|_| {}));
    assert!(a.has_input_listener());
}

// port: test/javascript/lib/codemirror_adapter.test.js:92 "handles removeEventListener for input events"
#[test]
fn adapter_remove_input_listener() {
    let mut ed = FakeEditor::default();
    let mut a = adapter(&mut ed);
    a.add_input_listener(Box::new(|_| {}));
    a.remove_input_listener();
    assert!(!a.has_input_listener());
}

// port: test/javascript/lib/codemirror_adapter.test.js:102 "dispatches input events to handler"
#[test]
fn adapter_dispatches_input_events_to_handler() {
    let seen: Rc<RefCell<Vec<InputEvent>>> = Rc::default();
    let sink = Rc::clone(&seen);
    let mut ed = FakeEditor::default();
    let mut a = adapter(&mut ed);
    a.add_input_listener(Box::new(move |e| sink.borrow_mut().push(e.clone())));
    a.dispatch_input(&InputEvent);
    assert_eq!(*seen.borrow(), vec![InputEvent]);
}

// port: test/javascript/lib/codemirror_adapter.test.js:114 "ignores dispatch when no handler set"
#[test]
fn adapter_ignores_dispatch_without_handler() {
    let mut ed = FakeEditor::default();
    adapter(&mut ed).dispatch_input(&InputEvent); // must not panic
}

// ---- getEditorContent -----------------------------------------------------------

// port: test/javascript/lib/codemirror_adapter.test.js:124 "returns content from controller when available"
#[test]
fn content_from_controller() {
    let ed = FakeEditor::with_text("controller content");
    assert_eq!(
        get_editor_content(Some(&ed as &dyn EditorSurface), None),
        "controller content"
    );
}

// port: test/javascript/lib/codemirror_adapter.test.js:132 "returns content from fallback textarea when controller is null"
#[test]
fn content_from_fallback_textarea() {
    assert_eq!(
        get_editor_content(None, Some("textarea content")),
        "textarea content"
    );
}

// port: test/javascript/lib/codemirror_adapter.test.js:137 "returns empty string when both are null"
#[test]
fn content_empty_when_both_missing() {
    assert_eq!(get_editor_content(None, None), "");
}

// port: test/javascript/lib/codemirror_adapter.test.js:141 "prefers controller over fallback"
#[test]
fn content_prefers_controller_over_fallback() {
    let ed = FakeEditor::with_text("controller content");
    assert_eq!(
        get_editor_content(Some(&ed as &dyn EditorSurface), Some("textarea content")),
        "controller content"
    );
}

// ---- getEditorSelection ----------------------------------------------------------

// port: test/javascript/lib/codemirror_adapter.test.js:152 "returns selection from controller"
#[test]
fn selection_from_controller() {
    let ed = FakeEditor {
        selection: sel(10, 20, "selected"),
        ..FakeEditor::default()
    };
    assert_eq!(
        get_editor_selection(Some(&ed as &dyn EditorSurface)),
        sel(10, 20, "selected")
    );
}

// port: test/javascript/lib/codemirror_adapter.test.js:164 "returns default selection when controller is null"
#[test]
fn selection_default_without_controller() {
    assert_eq!(get_editor_selection(None), sel(0, 0, ""));
}

// ---- getCursorPosition -------------------------------------------------------------

// port: test/javascript/lib/codemirror_adapter.test.js:170 "returns position from controller"
#[test]
fn cursor_position_from_controller() {
    let pos = CursorPosition {
        line: 5,
        column: 10,
        offset: 45,
    };
    let ed = FakeEditor {
        cursor: pos,
        ..FakeEditor::default()
    };
    assert_eq!(get_cursor_position(Some(&ed as &dyn EditorSurface)), pos);
}

// port: test/javascript/lib/codemirror_adapter.test.js:182 "returns default position when controller is null"
#[test]
fn cursor_position_default_without_controller() {
    assert_eq!(
        get_cursor_position(None),
        CursorPosition {
            line: 1,
            column: 1,
            offset: 0
        }
    );
}

// ---- getCursorInfo -------------------------------------------------------------------

// port: test/javascript/lib/codemirror_adapter.test.js:188 "returns cursor info from controller"
#[test]
fn cursor_info_from_controller() {
    let info = CursorInfo {
        current_line: 7,
        total_lines: 100,
    };
    let ed = FakeEditor {
        info,
        ..FakeEditor::default()
    };
    assert_eq!(get_cursor_info(Some(&ed as &dyn EditorSurface)), info);
}

// port: test/javascript/lib/codemirror_adapter.test.js:199 "returns default info when controller is null"
#[test]
fn cursor_info_default_without_controller() {
    assert_eq!(
        get_cursor_info(None),
        CursorInfo {
            current_line: 1,
            total_lines: 1
        }
    );
}
