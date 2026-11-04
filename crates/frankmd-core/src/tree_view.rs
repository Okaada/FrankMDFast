//! The data part of the Explorer rendering (`app/helpers/tree_helper.rb`,
//! `FoldersController#load_tree_for_turbo_stream`).
//!
//! Rails rendered HTML; the GPUI Explorer renders rows. What must stay the same
//! is *which* rows exist, in which order, at which depth, and their flags
//! (expanded, selected, draggable, context menu, config vs note). Golden:
//! `tree_helper.json` (the HTML is parsed back into rows by the test).

use crate::tree::{FileType, TreeNode};
use std::collections::BTreeSet;

/// Kind of an Explorer row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A folder header (focusable, toggles, drop target, draggable).
    Folder,
    /// A markdown note (draggable, has a context menu).
    Note,
    /// The `.fed` config file: selectable, renamable/deletable by keyboard,
    /// **not** draggable and **no** context menu (**[COMPAT]**).
    Config,
}

/// One rendered Explorer row, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    /// Vault-relative path (`data-path`).
    pub path: String,
    /// Display name.
    pub name: String,
    /// Nesting depth (0 = root level).
    pub depth: usize,
    /// Row kind.
    pub kind: RowKind,
    /// Folder rows: path is in the expanded set (chevron `expanded`).
    pub expanded: bool,
    /// File rows: path equals the selected file (`selected` class). A folder
    /// whose path equals the selection is **not** marked.
    pub selected: bool,
    /// `draggable="true"` (folders and notes, not config).
    pub draggable: bool,
    /// Has `contextmenu->app#showContextMenu` (folders and notes, not config).
    pub context_menu: bool,
    /// `false` when an ancestor folder is collapsed (row is in a `hidden`
    /// `tree-children` container).
    pub visible: bool,
    /// File rows: `data-mtime`, when known.
    pub mtime: Option<i64>,
}

/// Result of rendering a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeView {
    /// Top-level list is empty: show `sidebar.no_notes_yet`.
    Empty,
    /// Rows in document order (collapsed descendants included, `visible: false`).
    Rows(Vec<TreeRow>),
}

/// **[COMPAT]** `render_tree_items(items, expanded_folders:, selected_file:)`
/// as data. Empty nested folders render nothing (no "no notes" message).
pub fn tree_rows(items: &[TreeNode], expanded: &BTreeSet<String>, selected_file: &str) -> TreeView {
    todo!("tree_helper.rb")
}

/// Row kind of a file leaf.
pub fn file_row_kind(file_type: FileType) -> RowKind {
    match file_type {
        FileType::Markdown => RowKind::Note,
        FileType::Config => RowKind::Config,
    }
}

/// **[COMPAT]** expanded-folder remap after a folder rename
/// (`FoldersController#load_tree_for_turbo_stream`): every entry equal to
/// `from` or under `from + "/"` is re-rooted onto `to`; others are kept.
/// Input order is kept and duplicates collapse (Ruby `Set`). Parsing of the
/// comma list (`""` → empty) is [`parse_expanded`].
pub fn remap_expanded(expanded: &[String], from: &str, to: &str) -> Vec<String> {
    todo!("folders_controller.rb:64")
}

/// **[COMPAT]** `params[:expanded].to_s.split(",").to_set`, in insertion order.
pub fn parse_expanded(param: &str) -> Vec<String> {
    todo!()
}
