//! The config store: owns `.fed` for one vault, serializes writes under the
//! **[COMPAT]** config lock, and publishes a fresh [`ResolvedConfig`] after
//! each load/write/external change.

use crate::{ConfigError, EnvSnapshot, Key, ResolvedConfig, Value};
use frankmd_fs::Vault;
use std::path::PathBuf;
use std::sync::Arc;

/// `.fed` file name at the notes root (**[COMPAT]** `Config::CONFIG_FILE`).
pub const CONFIG_FILE: &str = ".fed";

/// Owner of `.fed` for one vault.
#[derive(Debug)]
pub struct ConfigStore {
    vault: Vault,
    env: EnvSnapshot,
    current: Arc<ResolvedConfig>,
}

impl ConfigStore {
    /// Load exactly like `Config.new` → `ensure_config_file`: under the config
    /// lock (`.frankmd-config.lock` in the notes root, created 0600), create
    /// `.fed` from the template if missing (atomic, 0600, `mkdir -p` the root)
    /// or append the AI section if [`crate::fed::needs_ai_section`]; then parse.
    ///
    /// **Never fails** (**[COMPAT]**): a read-only mount, a `.fed` directory,
    /// a symlinked lock file (`O_NOFOLLOW` → ELOOP) or undecodable bytes are
    /// logged into [`ResolvedConfig::warnings`] and resolution falls back to
    /// ENV and defaults. A `.fed` symlink is read through; a rewrite replaces
    /// it with a regular file.
    pub fn open(vault: Vault, env: EnvSnapshot) -> Self {
        todo!("backend.md §1.6")
    }

    /// Absolute path of `.fed`.
    pub fn path(&self) -> PathBuf {
        todo!()
    }

    /// Current immutable snapshot (cheap clone).
    pub fn current(&self) -> Arc<ResolvedConfig> {
        self.current.clone()
    }

    /// `Config#update` / `#set`: format (validate) **all** changes first, then
    /// under the lock re-run `ensure_config_file`, re-read `.fed`, apply
    /// [`crate::fed::apply_changes`] and replace it atomically with mode 0600.
    /// Nothing is written if any change is invalid. An empty `changes` writes
    /// nothing. Returns the new snapshot (re-parsed from what was written).
    pub fn update(&mut self, changes: &[(Key, Option<Value>)]) -> Result<Arc<ResolvedConfig>, ConfigError> {
        todo!()
    }

    /// **[COMPAT]** `PATCH /config`: keep only `UI_KEYS` (others are dropped
    /// silently, e.g. `aws_access_key_id`, `recent_files`), fail with
    /// [`ConfigError::NoValidSettings`] if nothing remains, else [`Self::update`].
    pub fn update_ui_settings(
        &mut self,
        changes: &[(Key, Option<Value>)],
    ) -> Result<Arc<ResolvedConfig>, ConfigError> {
        todo!()
    }

    /// Read-modify-write one key under the lock (`Config#modify`, used by
    /// recents): `f` sees the value resolved from the **fresh disk state**.
    pub fn modify(
        &mut self,
        key: Key,
        f: impl FnOnce(Option<Value>) -> Option<Value>,
    ) -> Result<Arc<ResolvedConfig>, ConfigError> {
        todo!()
    }

    /// Re-read after the watcher reports an external change to `.fed`.
    pub fn reload(&mut self) -> Result<Arc<ResolvedConfig>, ConfigError> {
        todo!()
    }
}
