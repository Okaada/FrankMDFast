//! The store.

use crate::{Command, Effect, Event, clock::Clock};
use frankmd_core::registry::DocRegistry;
use frankmd_core::{DocId, Generation, RelPath, Revision};
use std::collections::HashMap;
use std::sync::Arc;

/// An open document.
#[derive(Debug, Clone)]
pub struct OpenDoc {
    /// Stable id.
    pub id: DocId,
    /// Revision of the bytes last read/written on disk.
    pub disk_revision: Revision,
    /// Generation of the latest edit (renders/saves compare against it).
    pub edit_generation: Generation,
    /// Unsaved changes present.
    pub dirty: bool,
}

/// Immutable view handed to the UI.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Current tree.
    pub tree: Arc<Vec<frankmd_core::tree::TreeNode>>,
    /// Active document and its path.
    pub active: Option<(DocId, RelPath)>,
    /// Resolved config.
    pub config: Arc<frankmd_config::ResolvedConfig>,
}

/// The workspace for one vault.
pub struct Workspace {
    vault: frankmd_fs::Vault,
    config: frankmd_config::store::ConfigStore,
    registry: DocRegistry,
    docs: HashMap<DocId, OpenDoc>,
    tree_generation: Generation,
    clock: Box<dyn Clock>,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace").field("vault", &self.vault).finish_non_exhaustive()
    }
}

impl Workspace {
    /// Create for an opened vault and config store.
    pub fn new(
        vault: frankmd_fs::Vault,
        config: frankmd_config::store::ConfigStore,
        clock: Box<dyn Clock>,
    ) -> Self {
        Self {
            vault,
            config,
            registry: DocRegistry::default(),
            docs: HashMap::new(),
            tree_generation: Generation::default(),
            clock,
        }
    }

    /// Handle one command. Returns events for views and effects for the runtime.
    pub fn handle(&mut self, cmd: Command) -> (Vec<Event>, Vec<Effect>) {
        todo!()
    }

    /// Current snapshot.
    pub fn snapshot(&self) -> Snapshot {
        todo!()
    }
}
