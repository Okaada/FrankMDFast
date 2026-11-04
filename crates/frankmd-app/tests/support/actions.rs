//! Proposed GPUI actions and the default keymap (ui-spec §6.1/§6.2).
//!
//! Real data, not stubs: the implementation should move the table into the
//! command registry (`frankmd_workspace::commands`, HP-018) and generate the
//! GPUI bindings from it; the tests then import it from there.

use gpui_kit::{KeyBinding, actions};

actions!(
    frankmd,
    [
        ToggleCommandPalette,
        QuickOpen,
        RecentSwitcherNext,
        RecentSwitcherPrev,
        SearchInFiles,
        NewNote,
        NewFolder,
        SaveNow,
        CloseNote,
        Quit,
        NavigateBack,
        NavigateForward,
        ToggleSidebar,
        TogglePreview,
        ToggleTypewriter,
        ToggleScrollSync,
        ToggleZen,
        ToggleMediaLibrary,
        OpenSettings,
        ToggleHelp,
        ShowKeyboardShortcuts,
        OpenDiagnostics,
        FocusSidebar,
        FocusEditor,
        FocusPreview,
        FocusNextRegion,
        FocusPrevRegion,
        Escape,
        Confirm,
        SelectNext,
        SelectPrev,
        Find,
        Replace,
        FindNext,
        FindPrev,
        GoToLine,
        ToggleLineNumbers,
        EditorWidthIncrease,
        EditorWidthDecrease,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        InsertImage,
        InsertMenu,
        EmojiPicker,
        InsertLink,
        TextFormatMenu,
        ToggleTask,
        ContextAction,
        GrammarCheck,
        RenameNote,
        Bold,
        Italic,
        Strikethrough,
        InlineCode,
    ]
);

/// Key context of the workspace root (globals).
pub const CX_WORKSPACE: &str = "Workspace";
/// Key context of the editor pane.
pub const CX_EDITOR: &str = "Editor";
/// Key context of any overlay list (palette, quick open, search).
pub const CX_OVERLAY: &str = "Overlay";

/// The default keymap, one row per binding. Both shifted forms are bound where
/// US layouts differ (§6 intro, FE-042).
pub fn default_keymap() -> Vec<KeyBinding> {
    let ws = Some(CX_WORKSPACE);
    let ed = Some(CX_EDITOR);
    let ov = Some(CX_OVERLAY);
    vec![
        // §6.1 global
        KeyBinding::new("ctrl-shift-p", ToggleCommandPalette, ws),
        KeyBinding::new("ctrl-p", QuickOpen, ws),
        KeyBinding::new("ctrl-tab", RecentSwitcherNext, ws),
        KeyBinding::new("ctrl-shift-tab", RecentSwitcherPrev, ws),
        KeyBinding::new("ctrl-shift-f", SearchInFiles, ws),
        KeyBinding::new("ctrl-n", NewNote, ws),
        KeyBinding::new("ctrl-shift-n", NewFolder, ws),
        KeyBinding::new("ctrl-s", SaveNow, ws),
        KeyBinding::new("ctrl-w", CloseNote, ws),
        KeyBinding::new("ctrl-q", Quit, ws),
        KeyBinding::new("ctrl-alt-left", NavigateBack, ws),
        KeyBinding::new("ctrl-alt-right", NavigateForward, ws),
        KeyBinding::new("ctrl-e", ToggleSidebar, ws),
        KeyBinding::new("ctrl-shift-v", TogglePreview, ws),
        KeyBinding::new("ctrl-\\", ToggleTypewriter, ws),
        KeyBinding::new("ctrl-shift-\\", ToggleScrollSync, ws),
        KeyBinding::new("ctrl-|", ToggleScrollSync, ws),
        KeyBinding::new("f11", ToggleZen, ws),
        KeyBinding::new("ctrl-shift-m", ToggleMediaLibrary, ws),
        KeyBinding::new("ctrl-,", OpenSettings, ws),
        KeyBinding::new("f1", ToggleHelp, ws),
        KeyBinding::new("ctrl-/", ShowKeyboardShortcuts, ws),
        KeyBinding::new("ctrl-shift-o", OpenDiagnostics, ws),
        KeyBinding::new("ctrl-1", FocusSidebar, ws),
        KeyBinding::new("ctrl-2", FocusEditor, ws),
        KeyBinding::new("ctrl-3", FocusPreview, ws),
        KeyBinding::new("f6", FocusNextRegion, ws),
        KeyBinding::new("shift-f6", FocusPrevRegion, ws),
        KeyBinding::new("escape", Escape, ws),
        // §6.2 editor
        KeyBinding::new("ctrl-b", Bold, ed),
        KeyBinding::new("ctrl-i", Italic, ed),
        KeyBinding::new("ctrl-shift-s", Strikethrough, ed),
        KeyBinding::new("ctrl-`", InlineCode, ed),
        KeyBinding::new("ctrl-k", InsertLink, ed),
        KeyBinding::new("ctrl-m", TextFormatMenu, ed),
        KeyBinding::new("alt-i", InsertMenu, ed),
        KeyBinding::new("ctrl-shift-i", InsertImage, ed),
        KeyBinding::new("ctrl-shift-e", EmojiPicker, ed),
        KeyBinding::new("ctrl-.", ContextAction, ed),
        KeyBinding::new("ctrl-enter", ToggleTask, ed),
        KeyBinding::new("ctrl-f", Find, ed),
        KeyBinding::new("ctrl-h", Replace, ed),
        KeyBinding::new("f3", FindNext, ed),
        KeyBinding::new("shift-f3", FindPrev, ed),
        KeyBinding::new("ctrl-g", GoToLine, ed),
        KeyBinding::new("ctrl-l", ToggleLineNumbers, ed),
        KeyBinding::new("ctrl-shift-=", EditorWidthIncrease, ed),
        KeyBinding::new("ctrl-+", EditorWidthIncrease, ed),
        KeyBinding::new("ctrl-shift--", EditorWidthDecrease, ed),
        KeyBinding::new("ctrl-_", EditorWidthDecrease, ed),
        KeyBinding::new("ctrl-=", ZoomIn, ed),
        KeyBinding::new("ctrl--", ZoomOut, ed),
        KeyBinding::new("ctrl-0", ZoomReset, ed),
        KeyBinding::new("ctrl-shift-g", GrammarCheck, ed),
        KeyBinding::new("f2", RenameNote, ed),
        // §6.4 overlay lists
        KeyBinding::new("up", SelectPrev, ov),
        KeyBinding::new("down", SelectNext, ov),
        KeyBinding::new("enter", Confirm, ov),
        KeyBinding::new("escape", Escape, ov),
    ]
}
