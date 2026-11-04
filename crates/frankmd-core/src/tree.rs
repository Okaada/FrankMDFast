//! The Explorer tree model (what `NotesService#build_tree` returns).
//!
//! The serde form is exactly the Rails item hash (golden `notes_tree.json`):
//! `{name, path, type: "folder", children}` /
//! `{name, path, type: "file", file_type: "markdown", mtime}`.

use crate::RelPath;
use serde::{Deserialize, Serialize};

/// What kind of file a tree leaf is (`file_type` in the Rails hash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    /// A `*.md` note (the only kind `list_tree` produces).
    #[default]
    Markdown,
    /// The `.fed` config file, injected by the UI (selectable, not draggable).
    Config,
}

/// One node of the notes tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TreeNode {
    /// A directory (shown even when empty).
    Folder {
        /// Basename.
        name: String,
        /// Vault-relative path.
        path: RelPath,
        /// Children: folders first, then files.
        #[serde(default)]
        children: Vec<TreeNode>,
    },
    /// A markdown note (`*.md`, case-sensitive).
    File {
        /// Basename without `.md`.
        name: String,
        /// Vault-relative path.
        path: RelPath,
        /// `markdown` (or `config` for the injected `.fed` row).
        #[serde(default)]
        file_type: FileType,
        /// Modification time, whole epoch seconds (truncated).
        #[serde(default)]
        mtime: i64,
    },
}

impl TreeNode {
    /// Vault-relative path of the node.
    pub fn path(&self) -> &RelPath {
        match self {
            Self::Folder { path, .. } | Self::File { path, .. } => path,
        }
    }

    /// Display name of the node.
    pub fn name(&self) -> &str {
        match self {
            Self::Folder { name, .. } | Self::File { name, .. } => name,
        }
    }
}

/// Raw directory entry as read by `frankmd-fs` (already `lstat`ed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntryInfo {
    /// Basename.
    pub name: String,
    /// Entry kind.
    pub kind: EntryKind,
    /// mtime in whole seconds.
    pub mtime: i64,
}

/// Kind of a directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// Regular file.
    File,
    /// Directory.
    Dir,
    /// Symlink (always skipped by the tree, **[COMPAT]**).
    Symlink,
}

/// **[COMPAT]** visibility rules: dotfiles hidden, symlinks skipped, only
/// `.md` files, root-level `images`/`videos` hidden unless they contain notes.
/// `has_markdown_below` is supplied by the caller for the root special case.
pub fn is_visible(entry: &DirEntryInfo, at_root: bool, has_markdown_below: bool) -> bool {
    todo!("backend.md §2.2")
}

/// **[COMPAT]** ordering: dirs first by lowercase name (Unicode lowercase,
/// then bytewise), files by mtime desc.
pub fn sort_entries(entries: &mut [DirEntryInfo]) {
    todo!("backend.md §2.2")
}

/// **[COMPAT]** `tree_utils.js#flattenTree`: depth-first list of every file
/// node (folders contribute only their descendants). Feeds the file finder.
pub fn flatten_files(nodes: &[TreeNode]) -> Vec<&TreeNode> {
    todo!("frontend.md tree_utils")
}
