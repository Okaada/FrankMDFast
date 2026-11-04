//! Injectable watcher configuration.

use crate::SharedSelfWrites;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default debounce for the vault watcher.
pub const VAULT_DEBOUNCE: Duration = Duration::from_millis(200);
/// Default debounce for the Omarchy / language / fontconfig watchers.
pub const SYSTEM_DEBOUNCE: Duration = Duration::from_millis(100);

/// What [`crate::start_with`] watches. `None` disables an optional watcher.
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Notes root, watched recursively. Must exist.
    pub vault_root: PathBuf,
    /// Omarchy `current/` dir (`~/.local/state/omarchy/current`) →
    /// [`crate::WatchEvent::OmarchyThemeChanged`].
    pub omarchy_current_dir: Option<PathBuf>,
    /// Omarchy language file (`~/.config/omarchy/language`) →
    /// [`crate::WatchEvent::SystemLanguageChanged`].
    pub omarchy_language_file: Option<PathBuf>,
    /// fontconfig dir (`~/.config/fontconfig`) → [`crate::WatchEvent::FontsChanged`].
    pub fontconfig_dir: Option<PathBuf>,
    /// Token store to use; `None` creates a fresh one (see [`crate::Watchers::self_writes`]).
    pub self_writes: Option<SharedSelfWrites>,
    /// Vault debounce window.
    pub vault_debounce: Duration,
    /// Debounce window of the optional system watchers.
    pub system_debounce: Duration,
}

impl WatchConfig {
    /// Vault only; every optional watcher disabled.
    pub fn new(vault_root: PathBuf) -> Self {
        Self {
            vault_root,
            omarchy_current_dir: None,
            omarchy_language_file: None,
            fontconfig_dir: None,
            self_writes: None,
            vault_debounce: VAULT_DEBOUNCE,
            system_debounce: SYSTEM_DEBOUNCE,
        }
    }

    /// Vault plus the system watchers at their default locations under
    /// `$HOME` (fontconfig honours `$XDG_CONFIG_HOME`). Without `$HOME`
    /// the system watchers stay disabled.
    pub fn with_system_defaults(vault_root: PathBuf) -> Self {
        let home = std::env::var_os("HOME")
            .filter(|h| !h.is_empty())
            .map(PathBuf::from);
        let xdg_config = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|p| Path::new(p).is_absolute())
            .map(PathBuf::from);
        Self::with_home(vault_root, home.as_deref(), xdg_config.as_deref())
    }

    fn with_home(vault_root: PathBuf, home: Option<&Path>, xdg_config: Option<&Path>) -> Self {
        let mut config = Self::new(vault_root);
        let Some(home) = home else {
            return config;
        };
        config.omarchy_current_dir = Some(home.join(".local/state/omarchy/current"));
        config.omarchy_language_file = Some(home.join(".config/omarchy/language"));
        let config_home = xdg_config.map_or_else(|| home.join(".config"), Path::to_path_buf);
        config.fontconfig_dir = Some(config_home.join("fontconfig"));
        config
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn defaults_derive_from_home() {
        let c = WatchConfig::with_home("/v".into(), Some(Path::new("/home/u")), None);
        assert_eq!(
            c.omarchy_current_dir.unwrap(),
            Path::new("/home/u/.local/state/omarchy/current")
        );
        assert_eq!(
            c.omarchy_language_file.unwrap(),
            Path::new("/home/u/.config/omarchy/language")
        );
        assert_eq!(
            c.fontconfig_dir.unwrap(),
            Path::new("/home/u/.config/fontconfig")
        );
        assert_eq!(c.vault_debounce, VAULT_DEBOUNCE);
    }

    #[test]
    fn xdg_config_home_moves_fontconfig_only() {
        let c = WatchConfig::with_home(
            "/v".into(),
            Some(Path::new("/home/u")),
            Some(Path::new("/cfg")),
        );
        assert_eq!(c.fontconfig_dir.unwrap(), Path::new("/cfg/fontconfig"));
        assert_eq!(
            c.omarchy_language_file.unwrap(),
            Path::new("/home/u/.config/omarchy/language")
        );
    }

    #[test]
    fn no_home_disables_system_watchers() {
        let c = WatchConfig::with_home("/v".into(), None, None);
        assert!(c.omarchy_current_dir.is_none() && c.omarchy_language_file.is_none());
        assert!(c.fontconfig_dir.is_none());
    }
}
