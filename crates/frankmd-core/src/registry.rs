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
        if let Some(id) = self.by_path.get(path) {
            return *id;
        }
        let id = DocId::new();
        self.by_path.insert(path.clone(), id);
        self.by_id.insert(id, path.clone());
        id
    }

    /// Current path of `id`, if the document still exists.
    pub fn path_of(&self, id: DocId) -> Option<&RelPath> {
        self.by_id.get(&id)
    }

    /// Apply a file *or folder* rename/move: every id at or below `from` is
    /// rebased onto `to`. Returns the remaps in a deterministic order.
    pub fn rename(&mut self, from: &RelPath, to: &RelPath) -> Vec<Remap> {
        let mut remaps: Vec<Remap> = self
            .by_path
            .iter()
            .filter_map(|(path, id)| {
                path.rebase(from, to).map(|new| Remap {
                    id: *id,
                    from: path.clone(),
                    to: new,
                })
            })
            .collect();
        remaps.sort_by(|a, b| a.from.cmp(&b.from));
        for remap in &remaps {
            self.by_path.remove(&remap.from);
        }
        for remap in &remaps {
            // A document already registered at the destination is displaced:
            // the moved document owns that path now.
            if let Some(displaced) = self.by_path.insert(remap.to.clone(), remap.id) {
                self.by_id.remove(&displaced);
            }
            self.by_id.insert(remap.id, remap.to.clone());
        }
        remaps
    }

    /// Forget every id at or below `path` (delete). Returns the removed ids.
    pub fn remove(&mut self, path: &RelPath) -> Vec<DocId> {
        let mut removed: Vec<(RelPath, DocId)> = self
            .by_path
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, id)| (p.clone(), *id))
            .collect();
        removed.sort();
        for (p, id) in &removed {
            self.by_path.remove(p);
            self.by_id.remove(id);
        }
        removed.into_iter().map(|(_, id)| id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RelPath {
        RelPath::parse(s).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn folder_rename_keeps_ids_and_rebases_segment_wise() {
        let mut reg = DocRegistry::default();
        let a = reg.id_for(&p("blog/a.md"));
        let b = reg.id_for(&p("blog/sub/b.md"));
        let other = reg.id_for(&p("blog-old/c.md"));
        assert_eq!(reg.id_for(&p("blog/a.md")), a);

        let remaps = reg.rename(&p("blog"), &p("posts"));
        assert_eq!(remaps.len(), 2);
        assert_eq!(reg.path_of(a), Some(&p("posts/a.md")));
        assert_eq!(reg.path_of(b), Some(&p("posts/sub/b.md")));
        assert_eq!(reg.path_of(other), Some(&p("blog-old/c.md")));
        assert_eq!(reg.id_for(&p("posts/a.md")), a);
    }

    #[test]
    fn remove_forgets_the_subtree() {
        let mut reg = DocRegistry::default();
        let a = reg.id_for(&p("x/a.md"));
        let keep = reg.id_for(&p("xy.md"));
        assert_eq!(reg.remove(&p("x")), vec![a]);
        assert_eq!(reg.path_of(a), None);
        assert_eq!(reg.path_of(keep), Some(&p("xy.md")));
    }
}
