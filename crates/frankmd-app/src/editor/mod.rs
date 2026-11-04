//! Editor pane: gpui-kit's code editor (tree-sitter markdown highlighting) with the
//! `frankmd-vim` layer, typewriter mode, the inline find bar and the markdown
//! formatting commands.
//!
//! - [`EditorPane`] ([`pane`]): the view the shell embeds, and its [`EditorEvent`]s.
//! - [`EditorSettings`] ([`settings`]): what the shell resolves from `.fed`.
//! - [`format`], [`find`], [`edit`]: GUI-free text transforms and find state.
//! - [`insert`]: slash menu, `[[`/link completion, the insert dialogs and Ctrl+.
//! - [`vim`]: keystroke conversion and the `VimBuffer` over gpui-kit's `EditorState`
//!   (one vim command = one undo step; see that module for how).

pub mod edit;
pub mod find;
pub mod format;
pub mod insert;
pub mod pane;
pub mod settings;
pub(crate) mod vim;

pub use edit::Edit;
pub use insert::{ContextAction, EditorRequest};
pub use pane::{EditorEvent, EditorPane, OWNED_COMMANDS};
pub use settings::EditorSettings;
