//! Insert dialogs (ui-spec §5.5, §5.8, §5.9): the table editor, the code
//! block dialog and the emoji / emoticon / icon picker.
//!
//! Each is a GPUI view opened with its `open(...)` function. They only edit
//! their own GUI-free model (`frankmd_workspace::table_editor`,
//! `frankmd_workspace::code_block`, `frankmd_core::emoji::picker`) and report
//! the result as an [`InsertDialogEvent`]; the host (today the
//! [`crate::editor::EditorPane`], which owns the text and any pending `/query`)
//! turns it into one undoable edit with `frankmd_core::text::pending`.
//!
//! Every dialog registers its own keystroke interceptor while it is open, so its
//! keys (Esc, Ctrl+Enter, Tab between cells, Alt+arrows...) work whichever view
//! hosts it, before the app keymap and gpui-kit's input bindings.

pub mod code;
pub mod emoji;
pub mod table;

use crate::scale::px;
use std::ops::Range;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

pub use code::CodeDialog;
pub use emoji::EmojiPopover;
pub use table::TableDialog;

/// What an insert dialog reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertDialogEvent {
    /// Insert block markdown (table, fence). `replace` is the range being edited
    /// (Update mode); `None` inserts at the caret or over the pending `/query`,
    /// padded with blank lines. `cursor_offset` is the caret position inside the
    /// content (`None` = after it).
    Block {
        /// The markdown.
        content: String,
        /// Update mode: the document range to replace.
        replace: Option<Range<usize>>,
        /// Caret offset into `content`.
        cursor_offset: Option<usize>,
    },
    /// Insert inline text (`:smile:`, `:ph-star:`, a kaomoji) over the selection
    /// or the pending `/query`.
    Inline {
        /// The text.
        content: String,
    },
    /// Closed without inserting.
    Cancel,
}

/// One of the open insert dialogs (what a host keeps).
#[derive(Clone)]
pub enum InsertDialog {
    /// Table editor.
    Table(Entity<TableDialog>),
    /// Code block.
    Code(Entity<CodeDialog>),
    /// Emoji picker.
    Emoji(Entity<EmojiPopover>),
}

impl InsertDialog {
    /// The element id of the dialog root (`dialog-table`, `dialog-code-block`,
    /// `emoji-picker`), also its `OverlayKind::Dialog` name.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Table(_) => "table",
            Self::Code(_) => "code-block",
            Self::Emoji(_) => "emoji-picker",
        }
    }

    /// The view to render.
    pub fn view(&self) -> AnyView {
        match self {
            Self::Table(v) => v.clone().into(),
            Self::Code(v) => v.clone().into(),
            Self::Emoji(v) => v.clone().into(),
        }
    }

    /// Whether this is a modal (table, code) rather than a caret popover (emoji).
    pub fn is_modal(&self) -> bool {
        !matches!(self, Self::Emoji(_))
    }

    /// Move focus into the dialog (its first field).
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        match self {
            Self::Table(v) => v.update(cx, |d, cx| d.focus(window, cx)),
            Self::Code(v) => v.update(cx, |d, cx| d.focus(window, cx)),
            Self::Emoji(v) => v.update(cx, |d, cx| d.focus(window, cx)),
        }
    }
}

/// Keystroke as `ctrl-alt-shift-key` text (modifiers in a fixed order).
pub(crate) fn key_text(k: &Keystroke) -> String {
    let m = k.modifiers;
    let mut s = String::new();
    for (on, name) in [
        (m.control || m.platform, "ctrl-"),
        (m.alt, "alt-"),
        (m.shift, "shift-"),
    ] {
        if on {
            s.push_str(name);
        }
    }
    s.push_str(&k.key.to_ascii_lowercase());
    s
}

/// The shared modal frame: title row, body, footer, plus `extra` layers (the
/// table's cell menu). `id` is the element id (`dialog-<kind>`); `width` the
/// preferred width (clamped to the window).
#[allow(clippy::too_many_arguments)]
pub(crate) fn modal_frame(
    id: &'static str,
    focus: &FocusHandle,
    width: Pixels,
    title: SharedString,
    body: impl IntoElement,
    footer: impl IntoElement,
    extra: Option<AnyElement>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    div()
        .id(id)
        .test_support()
        .track_focus(focus)
        .flex()
        .flex_col()
        .w(width)
        .max_w(relative(1.))
        .max_h(relative(1.))
        .p_4()
        .gap_3()
        .bg(theme.popover)
        .text_color(theme.popover_foreground)
        .border_1()
        .border_color(theme.border)
        .rounded(theme.radius_lg)
        .shadow_lg()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(body)
        .child(footer)
        .children(extra)
        .into_any_element()
}

/// The full-window scrim a host puts behind a modal insert dialog; clicks on
/// it do nothing (FrankMD's table/code dialogs ignore backdrop clicks).
pub fn modal_scrim(dialog: AnyView, window: &Window, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let size = window.viewport_size();
    deferred(
        anchored()
            .position_mode(AnchoredPositionMode::Window)
            .position(point(px(0.), px(0.)))
            .child(
                div()
                    .id("insert-dialog-scrim")
                    .w(size.width)
                    .h(size.height)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .p_4()
                    .bg(theme.background.opacity(0.6))
                    .occlude()
                    .child(dialog),
            ),
    )
    .with_priority(1)
    .into_any_element()
}
