//! Commands in, events and effects out.

use frankmd_core::registry::Remap;
use frankmd_core::tree::TreeNode;
use frankmd_core::{DocId, Generation, RelPath, Revision};

use crate::autosave::{AutosaveEffect, RecoveryResolution};

pub use frankmd_core::new_note::NoteTemplate;

/// Everything that can change workspace state. Keyboard shortcuts, menus,
/// vim ex-commands and the CLI all produce these (one registry, HP-018/030).
#[derive(Debug, Clone)]
pub enum Command {
    /// Open (or switch to) the note at `path`, recovering its draft.
    OpenNote {
        /// Vault-relative path.
        path: RelPath,
    },
    /// Close the active note (flushes its draft first).
    CloseNote,
    /// Editor buffer changed (content is the full text; the editor keeps the rope).
    Edited {
        /// Document.
        doc: DocId,
        /// Full text.
        text: String,
    },
    /// Ctrl+S.
    SaveNow {
        /// Document.
        doc: DocId,
    },
    /// Create a note under `parent` and open it.
    CreateNote {
        /// Destination folder (root for the vault root).
        parent: RelPath,
        /// Name (`.md` appended when missing) or Hugo post title.
        name: String,
        /// How the note is seeded.
        template: NoteTemplate,
    },
    /// Create a Hugo post with an explicit slug (the New Note dialog lets the
    /// user edit it: CJK titles slugify to empty, ui-spec §5.13).
    CreateHugoPost {
        /// Destination folder (root for the vault root).
        parent: RelPath,
        /// Post title (front matter `title`).
        title: String,
        /// Slug (path segment and front matter `slug`); blank → slugified title.
        slug: String,
    },
    /// Create a folder.
    CreateFolder {
        /// Parent folder.
        parent: RelPath,
        /// Folder name.
        name: String,
    },
    /// File or folder rename/move; also used by inline title rename (HP-030).
    Rename {
        /// Current path.
        from: RelPath,
        /// New path.
        to: RelPath,
    },
    /// Multi-selection move with ancestor de-dup and stop-on-first-failure (HP-029).
    MoveMany {
        /// Selected items.
        items: Vec<RelPath>,
        /// Destination folder.
        dest: RelPath,
    },
    /// Delete a note or folder.
    Delete {
        /// Path.
        path: RelPath,
    },
    /// Write settings to `.fed`.
    UpdateSettings {
        /// Key changes (`None` removes the key).
        changes: Vec<(frankmd_config::Key, Option<frankmd_config::Value>)>,
    },
    /// Write settings to the global `config.toml` (Settings scope "All
    /// vaults", ui-spec §5.16); `.fed` values still win in this vault.
    UpdateGlobalSettings {
        /// Key changes (`None` removes the key).
        changes: Vec<(frankmd_config::Key, Option<frankmd_config::Value>)>,
    },
    /// The recovery dialog was answered.
    ResolveRecovery(RecoveryResolution),
    /// Recovery dialog "Keep both" (ui-spec §5.11): write the draft as
    /// `<name> (draft YYYY-MM-DD HH-MM).md` next to the note, then keep the
    /// file on disk in the editor.
    RecoveryKeepBoth(crate::autosave::RecoveryRequest),
    /// Content-loss banner: the app ran the editor's undo first.
    ContentLossUndo {
        /// Whether the editor's own undo changed anything.
        undid_editor_change: bool,
    },
    /// Content-loss banner: "Save anyway".
    ContentLossSaveAnyway,
    /// Window deactivated / app suspending: flush the active draft, no save.
    FlushDrafts,
    /// Timer tick from the app (drives debounced autosave with the injected clock).
    Tick,
    /// Result of an `Effect::ScanTree`.
    TreeScanned {
        /// Generation the scan was requested with.
        generation: Generation,
        /// The tree.
        tree: Vec<TreeNode>,
    },
    /// A watcher event.
    External(frankmd_watch::WatchEvent),
}

/// Notifications to views.
#[derive(Debug, Clone)]
pub enum Event {
    /// A note became the active document; `text` is what the editor shows
    /// (the recovered draft when one was restored).
    DocOpened {
        /// Document.
        doc: DocId,
        /// Its path.
        path: RelPath,
        /// Editor text.
        text: String,
    },
    /// The active note was closed.
    DocClosed {
        /// Document.
        doc: DocId,
    },
    /// A save reached the disk.
    DocSaved {
        /// Document.
        doc: DocId,
        /// New revision.
        revision: Revision,
    },
    /// Paths changed under live documents; subscribers update from this list only.
    Remapped(Vec<Remap>),
    /// The document was deleted through FrankMD.
    DocDeleted {
        /// Document.
        doc: DocId,
    },
    /// The file changed on disk under an open, *clean* document → reloaded.
    DocReloaded {
        /// Document.
        doc: DocId,
        /// New text.
        text: String,
    },
    /// The file changed on disk under a *dirty* document → "changed on disk" banner.
    DocConflict {
        /// Document.
        doc: DocId,
    },
    /// The file of an open document vanished → "deleted on disk" banner.
    DocDeletedOnDisk {
        /// Document.
        doc: DocId,
    },
    /// The tree snapshot changed.
    TreeChanged,
    /// Reveal (expand ancestors + select) before the next render (HP-029).
    Reveal {
        /// Path to reveal.
        path: RelPath,
    },
    /// `.fed` changed (ours or external).
    SettingsChanged,
    /// Autosave UI effects: status label, content-loss banner, recovery
    /// dialog, editor text replacement, unload guard (never `Save`, which the
    /// store performs itself).
    Autosave(AutosaveEffect),
    /// System watcher events the app reacts to (theme, language, fonts, errors).
    System(frankmd_watch::WatchEvent),
    /// Localizable, user-facing error.
    Error(UserError),
}

/// Work the app must run off the UI thread, then report back.
#[derive(Debug, Clone)]
pub enum Effect {
    /// Rescan the tree and answer with [`Command::TreeScanned`].
    ScanTree {
        /// Generation to tag the result with.
        generation: Generation,
    },
    /// Re-render the preview of `doc`.
    Render {
        /// Document.
        doc: DocId,
        /// Edit generation the render is for.
        generation: Generation,
    },
    /// Arm one timer for this instant and send [`Command::Tick`] then.
    WakeAt(std::time::Instant),
}

/// Error with an i18n key (never raw upstream text, HP-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserError {
    /// i18n key, e.g. `errors.note_already_exists`.
    pub key: &'static str,
    /// Interpolation values.
    pub args: Vec<(&'static str, String)>,
}

impl UserError {
    /// An error without interpolation values.
    pub fn new(key: &'static str) -> Self {
        Self {
            key,
            args: Vec::new(),
        }
    }
}
