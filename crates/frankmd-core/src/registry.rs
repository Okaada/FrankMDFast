//! The single registry mapping [`DocId`] ⇄ current [`RelPath`] (HP-001).
//!
//! Every subsystem (autosave, drafts, undo cache, recents, expansion state)
//! keys by `DocId`. A rename/move is one transactional call here that returns
//! the list of affected ids; subscribers react to that list instead of
//! re-implementing `startsWith(old + "/")` remapping.

use crate::{DocId, RelPath};
use std::collections::HashMap;

/// A path change applied to one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remap {
    /// The document.
    pub id: DocId,
    /// Path before the operation.
    pub from: RelPath,
    /// Path after the operation.
    pub to: RelPath,
}

/// DocId ⇄ path registry. Owned by the workspace store; never cloned into UI.
#[derive(Debug, Default)]
pub struct DocRegistry {
    by_id: HashMap<DocId, RelPath>,
    by_path: HashMap<RelPath, DocId>,
}

impl DocRegistry {
    /// Id for `path`, minting one if the path is not known yet.
    pub fn id_for(&mut self, path: &RelPath) -> DocId {
        todo!()
    }

    /// Current path of `id`, if the document still exists.
    pub fn path_of(&self, id: DocId) -> Option<&RelPath> {
        self.by_id.get(&id)
    }

    /// Apply a file *or folder* rename/move: every id at or below `from` is
    /// rebased onto `to`. Returns the remaps in a deterministic order.
    pub fn rename(&mut self, from: &RelPath, to: &RelPath) -> Vec<Remap> {
        todo!()
    }

    /// Forget every id at or below `path` (delete). Returns the removed ids.
    pub fn remove(&mut self, path: &RelPath) -> Vec<DocId> {
        todo!()
    }
}
