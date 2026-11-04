//! Persisted `path → DocId` index (`<vault state>/doc_index.json`), so a
//! note keeps its identity — and therefore its draft — across restarts.
//!
//! The in-memory [`DocRegistry`](frankmd_core::registry::DocRegistry) mints
//! ids per run and cannot be seeded with known ones yet, so the store
//! reconciles on open: when the id persisted for a path differs from the id
//! the registry minted, the drafts filed under the old id are re-keyed to the
//! new one ([`crate::draft_storage::DraftStorage::remap_drafts`], which never
//! overwrites a different draft). Renames and deletes keep the index in step.
//! The file is written atomically through `frankmd-fs`; it lives outside the
//! notes folder and never enters front matter (compatibility).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use frankmd_core::DocId;
use frankmd_core::registry::Remap;

/// File name inside the per-vault state dir.
pub const DOC_INDEX_FILE: &str = "doc_index.json";

/// The persisted index.
#[derive(Debug, Default)]
pub struct DocIndex {
    file: Option<PathBuf>,
    by_path: BTreeMap<String, DocId>,
}

impl DocIndex {
    /// An index that is never persisted (tests, read-only state dirs).
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Load `dir/doc_index.json`; a missing or unreadable file starts empty
    /// (identity is then re-established as notes are opened).
    pub fn open(dir: &Path) -> Self {
        let file = dir.join(DOC_INDEX_FILE);
        let by_path = std::fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BTreeMap<String, DocId>>(&bytes).ok())
            .unwrap_or_default();
        Self {
            file: Some(file),
            by_path,
        }
    }

    /// The id last recorded for `path`.
    pub fn get(&self, path: &str) -> Option<DocId> {
        self.by_path.get(path).copied()
    }

    /// Record that `path` is `doc`. Returns the different id it had before.
    pub fn record(&mut self, path: &str, doc: DocId) -> Option<DocId> {
        let old = self.by_path.insert(path.to_owned(), doc);
        if old != Some(doc) {
            self.persist();
        }
        old.filter(|previous| *previous != doc)
    }

    /// Follow a rename/move.
    pub fn apply_remaps(&mut self, remaps: &[Remap]) {
        if remaps.is_empty() {
            return;
        }
        for remap in remaps {
            self.by_path.remove(remap.from.as_str());
        }
        for remap in remaps {
            self.by_path.insert(remap.to.as_str().to_owned(), remap.id);
        }
        self.persist();
    }

    /// Forget deleted documents.
    pub fn remove(&mut self, docs: &[DocId]) {
        let before = self.by_path.len();
        self.by_path.retain(|_, id| !docs.contains(id));
        if self.by_path.len() != before {
            self.persist();
        }
    }

    /// Best effort: a failed write only costs identity (and thus automatic
    /// draft matching) after the next restart, never note content.
    fn persist(&self) {
        let Some(file) = &self.file else {
            return;
        };
        if let Ok(bytes) = serde_json::to_vec(&self.by_path) {
            let _ = frankmd_fs::atomic::atomic_replace(
                file,
                &bytes,
                frankmd_fs::atomic::ModePolicy::Fixed(0o600),
            );
        }
    }
}
