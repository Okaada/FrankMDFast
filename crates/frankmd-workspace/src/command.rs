//! Commands in, events and effects out.

use frankmd_core::{DocId, Generation, RelPath, Revision};
use frankmd_core::registry::Remap;
use frankmd_core::tree::TreeNode;

/// Everything that can change workspace state. Keyboard shortcuts, menus,
/// vim ex-commands and the CLI all produce these (one registry, HP-018/030).
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Command {
    OpenNote { path: RelPath },
    /// Editor buffer changed (content is the full text; the editor keeps the rope).
    Edited { doc: DocId, text: String },
    SaveNow { doc: DocId },
    CreateNote { parent: RelPath, name: String, template: NoteTemplate },
    CreateFolder { parent: RelPath, name: String },
    /// File or folder rename/move; also used by inline title rename (HP-030).
    Rename { from: RelPath, to: RelPath },
    /// Multi-selection move with ancestor de-dup and stop-on-first-failure (HP-029).
    MoveMany { items: Vec<RelPath>, dest: RelPath },
    Delete { path: RelPath },
    UpdateSettings { changes: Vec<(frankmd_config::Key, Option<frankmd_config::Value>)> },
    /// Timer tick from the app (drives debounced autosave with the injected clock).
    Tick,
    /// Result of an `Effect::ScanTree`.
    TreeScanned { generation: Generation, tree: Vec<TreeNode> },
    /// A watcher event.
    External(frankmd_watch::WatchEvent),
}

/// How a new note is seeded.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum NoteTemplate {
    Empty,
    HugoPost,
}

/// Notifications to views.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Event {
    DocOpened { doc: DocId },
    DocSaved { doc: DocId, revision: Revision },
    /// Paths changed under live documents; subscribers update from this list only.
    Remapped(Vec<Remap>),
    DocDeleted { doc: DocId },
    /// The file changed on disk under an open, *clean* document → reloaded.
    DocReloaded { doc: DocId },
    /// The file changed on disk under a *dirty* document → user must choose.
    DocConflict { doc: DocId },
    TreeChanged,
    /// Reveal (expand ancestors + select) before the next render (HP-029).
    Reveal { path: RelPath },
    SettingsChanged,
    /// Localizable, user-facing error.
    Error(UserError),
}

/// Work the app must run off the UI thread, then report back.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Effect {
    ScanTree { generation: Generation },
    Render { doc: DocId, generation: Generation },
    PersistDraft { doc: DocId },
}

/// Error with an i18n key (never raw upstream text, HP-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserError {
    /// i18n key, e.g. `errors.note_already_exists`.
    pub key: &'static str,
    /// Interpolation values.
    pub args: Vec<(&'static str, String)>,
}
