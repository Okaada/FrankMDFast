//! App-owned locations **outside** the notes folder (HP-012, OM-010).
//! The only app files inside a vault are the compat-required `.fed` and the
//! two lock files.

use crate::EnvSnapshot;
use std::path::{Path, PathBuf};

/// Resolved XDG directories for FrankMD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirs {
    /// `$XDG_CONFIG_HOME/frankmd` (global settings, user themes).
    pub config: PathBuf,
    /// `$XDG_STATE_HOME/frankmd` (drafts, DocId index, window state, logs, fallback locks).
    pub state: PathBuf,
    /// `$XDG_CACHE_HOME/frankmd` (thumbnails, math raster cache, search index).
    pub cache: PathBuf,
}

impl AppDirs {
    /// XDG strategy on every OS (also macOS, so `~/.config/frankmd` stays put).
    pub fn discover() -> Option<Self> {
        use etcetera::BaseStrategy as _;
        let s = etcetera::choose_base_strategy().ok()?;
        Some(Self {
            config: s.config_dir().join("frankmd"),
            // Windows has no state dir: keep state with the app data there.
            state: s
                .state_dir()
                .unwrap_or_else(|| s.data_dir())
                .join("frankmd"),
            cache: s.cache_dir().join("frankmd"),
        })
    }

    /// Per-vault state directory, keyed by a hash of the vault's realpath.
    pub fn vault_state(&self, vault_realpath: &Path) -> PathBuf {
        let id = frankmd_core::Revision::of(vault_realpath.as_os_str().as_encoded_bytes()).0;
        self.state.join("vaults").join(id)
    }
}

/// **[COMPAT]** notes root resolution of `Config.new(base_path:)`: an explicit
/// path wins, else `NOTES_PATH` verbatim (a trailing `/` is kept). `None` when
/// neither is given (Rails then used `Rails.root/notes`; the app decides) or
/// the value is empty.
pub fn notes_root(explicit: Option<&Path>, env: &EnvSnapshot) -> Option<PathBuf> {
    match explicit {
        Some(path) => Some(path.to_path_buf()),
        None => env.raw("NOTES_PATH").map(PathBuf::from),
    }
    .filter(|p| !p.as_os_str().is_empty())
}

/// `<root>/.fed`.
pub fn config_file_path(notes_root: &Path) -> PathBuf {
    notes_root.join(crate::store::CONFIG_FILE)
}
