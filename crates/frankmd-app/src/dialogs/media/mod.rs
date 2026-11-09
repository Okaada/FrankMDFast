//! Media dialogs: Insert image (ui-spec §5.6), Insert video (§5.7) and the
//! media library workspace (§5.15), plus paste / drop of images into the
//! editor and the shared "Storage for new media" model.
//!
//! State lives in the GUI-free view models of `frankmd_media::vm`; these
//! views render them and run their effects: file IO on the background
//! executor, network on the tokio bridge ([`task::spawn_net`], aborted when
//! the dialog closes). Results reach the shell as [`MediaEvent`]s.
//!
//! Wiring (done by the shell, see docs/testing/wiring-media.md):
//! [`init`] once at startup, a [`MediaHub`] per window, [`apply_insert`] for
//! [`MediaEvent::Insert`].

pub mod files;
pub mod library;
pub mod picker;
pub mod services;
pub mod storage;
pub mod task;
pub mod video;
pub(crate) mod widgets;

use std::ops::Range;
use std::sync::Arc;

pub use frankmd_media::vm::markup::MediaInsert;
use gpui_kit::*;

pub use library::MediaLibraryPane;
pub use picker::ImagePickerDialog;
pub use services::{MediaContext, MediaServices};
pub use storage::{StorageEvent, StorageModel};
pub use video::VideoDialogView;

/// Key context of the media dialogs.
pub const CONTEXT: &str = "MediaDialog";

actions!(
    media_dialog,
    [
        /// Ctrl+Enter: Insert.
        Confirm,
        /// Esc: cancel (or abort a running generation first).
        Cancel,
        /// Ctrl+Tab / Down on the rail: next source or tab.
        NextSource,
        /// Ctrl+Shift+Tab / Up on the rail: previous source or tab.
        PrevSource,
        /// Ctrl+1.
        Source1,
        /// Ctrl+2.
        Source2,
        /// Ctrl+3.
        Source3,
        /// Ctrl+4.
        Source4,
        /// Ctrl+5.
        Source5,
        /// Ctrl+6.
        Source6,
        /// Ctrl+7.
        Source7,
        /// Enter / Space on a dropzone or grid card.
        Activate,
    ]
);

/// Bind the dialogs' keys (ui-spec §6.4). Call once at startup.
pub fn init(cx: &mut App) {
    let ctx = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("ctrl-enter", Confirm, ctx),
        KeyBinding::new("escape", Cancel, ctx),
        KeyBinding::new("ctrl-tab", NextSource, ctx),
        KeyBinding::new("ctrl-shift-tab", PrevSource, ctx),
        KeyBinding::new("ctrl-1", Source1, ctx),
        KeyBinding::new("ctrl-2", Source2, ctx),
        KeyBinding::new("ctrl-3", Source3, ctx),
        KeyBinding::new("ctrl-4", Source4, ctx),
        KeyBinding::new("ctrl-5", Source5, ctx),
        KeyBinding::new("ctrl-6", Source6, ctx),
        KeyBinding::new("ctrl-7", Source7, ctx),
        KeyBinding::new("down", NextSource, Some("MediaRail")),
        KeyBinding::new("up", PrevSource, Some("MediaRail")),
        KeyBinding::new("enter", Activate, Some("MediaActivate")),
        KeyBinding::new("space", Activate, Some("MediaActivate")),
    ]);
    if cfg!(target_os = "macos") {
        // `ctrl-` is the physical Ctrl key; macOS users expect ⌘ too. No
        // ⌘Tab twins: the OS app switcher owns it.
        cx.bind_keys([
            KeyBinding::new("cmd-enter", Confirm, ctx),
            KeyBinding::new("cmd-1", Source1, ctx),
            KeyBinding::new("cmd-2", Source2, ctx),
            KeyBinding::new("cmd-3", Source3, ctx),
            KeyBinding::new("cmd-4", Source4, ctx),
            KeyBinding::new("cmd-5", Source5, ctx),
            KeyBinding::new("cmd-6", Source6, ctx),
            KeyBinding::new("cmd-7", Source7, ctx),
        ]);
    }
    library::init(cx);
    crate::dialogs::ai::init(cx);
}

/// What the media views ask of the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaEvent {
    /// Insert into the open note (use [`apply_insert`] with the pending slash
    /// range), mark it dirty, focus the editor. The dialog has closed itself.
    Insert(MediaInsert),
    /// The dialog closed without inserting: pop the overlay, restore focus.
    Closed,
    /// A status-line message.
    Status {
        /// Text.
        text: SharedString,
        /// Error (5 s, `danger`) or info (2 s).
        error: bool,
    },
    /// Raise `BannerKind::StorageFallback` (once per session).
    StorageFallback,
    /// Open Settings › Integrations.
    OpenSettings,
    /// Open (and reveal) a note; close the media library.
    OpenNote(String),
    /// Close the media library and return to the editor.
    CloseLibrary,
}

/// The media surfaces of one window.
pub struct MediaHub {
    /// Vault services.
    pub services: Arc<MediaServices>,
    /// "Storage for new media" (also bind the Settings row to it).
    pub storage: Entity<StorageModel>,
    /// Insert image.
    pub picker: Entity<ImagePickerDialog>,
    /// Insert video.
    pub video: Entity<VideoDialogView>,
    /// Media library pane.
    pub library: Entity<MediaLibraryPane>,
}

impl MediaHub {
    /// Build the surfaces for a vault (no IO on this thread besides opening `.fed`'s store).
    pub fn new(ctx: &MediaContext, window: &mut Window, cx: &mut App) -> anyhow::Result<Self> {
        let services = MediaServices::new(ctx)?;
        let storage = cx.new(|cx| StorageModel::new(Arc::clone(&services), cx));
        let picker =
            cx.new(|cx| ImagePickerDialog::new(Arc::clone(&services), storage.clone(), window, cx));
        let video =
            cx.new(|cx| VideoDialogView::new(Arc::clone(&services), storage.clone(), window, cx));
        let library = cx.new(|cx| MediaLibraryPane::new(Arc::clone(&services), window, cx));
        Ok(Self {
            services,
            storage,
            picker,
            video,
            library,
        })
    }
}

/// Apply a [`MediaInsert`] to the editor as one undoable edit, with
/// FrankMD's placement rules (`insertImage` / `insertBlockContent`).
/// `slash` is the pending `/image` / `/video` query range. Returns `false`
/// when nothing was inserted.
pub fn apply_insert(
    pane: &Entity<crate::editor::EditorPane>,
    insert: &MediaInsert,
    slash: Option<Range<usize>>,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let text = pane.read(cx).text(cx);
    let selection = pane.read(cx).selection(cx);
    let Some(edit) = frankmd_media::vm::markup::insertion_edit(&text, selection, slash, insert)
    else {
        return false;
    };
    let edit = crate::editor::Edit {
        range: edit.range,
        text: edit.text,
        selection: edit.caret..edit.caret,
    };
    pane.update(cx, |pane, cx| {
        pane.apply_edit(edit, window, cx);
        pane.focus(window, cx);
    });
    true
}

/// Compute the edit [`apply_insert`] makes (GUI-free; for tests and callers
/// holding the text themselves).
pub fn insertion_edit(
    text: &str,
    selection: Range<usize>,
    slash: Option<Range<usize>>,
    insert: &MediaInsert,
) -> Option<crate::editor::Edit> {
    frankmd_media::vm::markup::insertion_edit(text, selection, slash, insert).map(|e| {
        crate::editor::Edit {
            range: e.range,
            text: e.text,
            selection: e.caret..e.caret,
        }
    })
}
