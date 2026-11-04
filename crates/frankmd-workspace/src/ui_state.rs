//! Per-vault window state that is not a `.fed` setting (ui-spec §3.1, §3.3,
//! §5.14): sidebar width, preview share, expanded folders and the collapsed
//! state of the "Linked from" section. Stored as `<vault state>/ui_state.json`,
//! outside the notes folder, written atomically through `frankmd-fs`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::layout::SIDEBAR_WIDTH;

/// File name inside the per-vault state dir.
pub const UI_STATE_FILE: &str = "ui_state.json";

/// Sidebar width bounds (ui-spec §3.1).
pub const SIDEBAR_MIN: f32 = 180.0;
/// Sidebar width upper bound.
pub const SIDEBAR_MAX: f32 = 480.0;
/// Default preview share of the editor + preview width.
pub const PREVIEW_RATIO_DEFAULT: f32 = 0.45;
/// Keyboard splitter step in px (ui-spec §3.1, FE-021 fixed).
pub const SPLITTER_STEP: f32 = 16.0;

/// The persisted values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    /// Docked sidebar width in logical px.
    pub sidebar_width: f32,
    /// Preview width as a share of editor + preview.
    pub preview_ratio: f32,
    /// Expanded explorer folders.
    pub expanded: Vec<String>,
    /// "Linked from" section collapsed.
    pub backlinks_collapsed: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sidebar_width: SIDEBAR_WIDTH,
            preview_ratio: PREVIEW_RATIO_DEFAULT,
            expanded: Vec::new(),
            backlinks_collapsed: false,
        }
    }
}

impl UiState {
    /// Clamp a sidebar width to its bounds.
    pub fn clamp_sidebar(width: f32) -> f32 {
        if width.is_finite() {
            width.clamp(SIDEBAR_MIN, SIDEBAR_MAX)
        } else {
            SIDEBAR_WIDTH
        }
    }

    /// The preview share that keeps both panes at their minimum widths
    /// inside `total` px (editor ≥ `editor_min`, preview ≥ `preview_min`).
    pub fn clamp_ratio(ratio: f32, total: f32, editor_min: f32, preview_min: f32) -> f32 {
        if !ratio.is_finite() || total <= 0.0 {
            return PREVIEW_RATIO_DEFAULT;
        }
        let lo = (preview_min / total).min(0.9);
        let hi = (1.0 - editor_min / total).max(lo);
        ratio.clamp(lo, hi)
    }
}

/// Loads and saves [`UiState`] for one vault.
#[derive(Debug, Default)]
pub struct UiStateStore {
    file: Option<PathBuf>,
    state: UiState,
}

impl UiStateStore {
    /// Never persisted (tests, unavailable state dir).
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Load `dir/ui_state.json`; missing or malformed → defaults. Values are
    /// clamped on load (a hand-edited file can't break the layout).
    pub fn open(dir: &Path) -> Self {
        let file = dir.join(UI_STATE_FILE);
        let mut state = std::fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<UiState>(&bytes).ok())
            .unwrap_or_default();
        state.sidebar_width = UiState::clamp_sidebar(state.sidebar_width);
        if !state.preview_ratio.is_finite() || !(0.05..=0.95).contains(&state.preview_ratio) {
            state.preview_ratio = PREVIEW_RATIO_DEFAULT;
        }
        Self {
            file: Some(file),
            state,
        }
    }

    /// Current values.
    pub fn get(&self) -> &UiState {
        &self.state
    }

    /// Change and persist (only when something changed).
    pub fn update(&mut self, f: impl FnOnce(&mut UiState)) {
        let before = self.state.clone();
        f(&mut self.state);
        self.state.sidebar_width = UiState::clamp_sidebar(self.state.sidebar_width);
        if self.state != before {
            self.persist();
        }
    }

    /// Change in memory only (splitter drags); [`Self::flush`] writes it.
    pub fn update_transient(&mut self, f: impl FnOnce(&mut UiState)) {
        f(&mut self.state);
        self.state.sidebar_width = UiState::clamp_sidebar(self.state.sidebar_width);
    }

    /// Write the current values.
    pub fn flush(&self) {
        self.persist();
    }

    /// Best effort: losing it only costs layout preferences.
    fn persist(&self) {
        let Some(file) = &self.file else {
            return;
        };
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.state) {
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
        let mut store = UiStateStore::open(dir.path());
        assert_eq!(store.get(), &UiState::default());
        store.update(|s| {
            s.sidebar_width = 9999.0;
            s.preview_ratio = 0.3;
            s.expanded = vec!["blog".into()];
            s.backlinks_collapsed = true;
        });
        let again = UiStateStore::open(dir.path());
        assert!((again.get().sidebar_width - SIDEBAR_MAX).abs() < f32::EPSILON);
        assert!((again.get().preview_ratio - 0.3).abs() < f32::EPSILON);
        assert_eq!(again.get().expanded, vec!["blog".to_owned()]);
        assert!(again.get().backlinks_collapsed);
    }

    #[test]
    #[allow(clippy::disallowed_methods)] // test fixture setup
    fn malformed_file_gives_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(UI_STATE_FILE), b"{not json").expect("write");
        assert_eq!(UiStateStore::open(dir.path()).get(), &UiState::default());
    }

    #[test]
    fn ratio_respects_pane_minimums() {
        assert!((UiState::clamp_ratio(0.9, 1000.0, 320.0, 280.0) - 0.68).abs() < 1e-4);
        assert!((UiState::clamp_ratio(0.1, 1000.0, 320.0, 280.0) - 0.28).abs() < 1e-4);
        assert!(
            (UiState::clamp_ratio(f32::NAN, 1000.0, 320.0, 280.0) - PREVIEW_RATIO_DEFAULT).abs()
                < 1e-4
        );
    }
}
