//! App-wide preferences that belong to this machine, not to a notes folder
//! (so they are neither `.fed` keys nor per-vault UI state): the UI scale.
//! Stored as `<app state dir>/prefs.json`, written atomically.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// File name inside the app state dir.
pub const APP_PREFS_FILE: &str = "prefs.json";

/// UI scale bounds in percent.
pub const UI_SCALE_MIN: u16 = 80;
/// See [`UI_SCALE_MIN`].
pub const UI_SCALE_MAX: u16 = 200;

/// The persisted values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPrefs {
    /// Interface size in percent (100 = gpui-kit's default sizes).
    pub ui_scale: u16,
}

impl Default for AppPrefs {
    fn default() -> Self {
        Self { ui_scale: 100 }
    }
}

/// Loads and saves [`AppPrefs`].
#[derive(Debug, Default)]
pub struct AppPrefsStore {
    file: Option<PathBuf>,
    prefs: AppPrefs,
}

impl AppPrefsStore {
    /// Never persisted (tests).
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Load `file`; missing or malformed → defaults, values clamped.
    pub fn open(file: &Path) -> Self {
        let mut prefs = std::fs::read(file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<AppPrefs>(&bytes).ok())
            .unwrap_or_default();
        prefs.ui_scale = prefs.ui_scale.clamp(UI_SCALE_MIN, UI_SCALE_MAX);
        Self {
            file: Some(file.to_path_buf()),
            prefs,
        }
    }

    /// Current values.
    pub fn get(&self) -> &AppPrefs {
        &self.prefs
    }

    /// Set the UI scale (clamped) and persist it when it changed.
    pub fn set_ui_scale(&mut self, percent: u16) {
        let percent = percent.clamp(UI_SCALE_MIN, UI_SCALE_MAX);
        if percent != self.prefs.ui_scale {
            self.prefs.ui_scale = percent;
            self.persist();
        }
    }

    /// Best effort: losing it only costs the preference.
    fn persist(&self) {
        let Some(file) = &self.file else {
            return;
        };
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.prefs) {
            let _ = frankmd_fs::atomic::atomic_replace(
                file,
                &bytes,
                frankmd_fs::atomic::ModePolicy::Fixed(0o600),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_clamps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join(APP_PREFS_FILE);
        let mut store = AppPrefsStore::open(&file);
        assert_eq!(store.get().ui_scale, 100);
        store.set_ui_scale(150);
        assert_eq!(AppPrefsStore::open(&file).get().ui_scale, 150);
        store.set_ui_scale(999);
        assert_eq!(AppPrefsStore::open(&file).get().ui_scale, UI_SCALE_MAX);
    }

    #[test]
    #[allow(clippy::disallowed_methods)] // test fixture setup
    fn malformed_file_gives_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join(APP_PREFS_FILE);
        std::fs::write(&file, b"{not json").expect("write");
        assert_eq!(AppPrefsStore::open(&file).get(), &AppPrefs::default());
    }
}
