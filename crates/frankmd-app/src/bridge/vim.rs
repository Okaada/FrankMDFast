//! `frankmd_vim::VimBuffer` implemented over gpui-kit's `EditorState`.
//!
//! Key flow: the editor pane wraps `Editor` in a `div().capture_key_down(..)`
//! so keystrokes reach the vim engine *before* the editor's own bindings
//! (capture phase runs root → focused). `Outcome::Handled` →
//! `cx.stop_propagation()`; `PassThrough` → let the editor/app keymap run.
//! While IME preedit is active every key passes through.
