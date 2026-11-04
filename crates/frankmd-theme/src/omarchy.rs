//! Omarchy 4.5 integration (docs/research/omarchy-4.5.md §2): the active
//! palette lives in `~/.local/state/omarchy/current/theme/colors.toml`
//! (FrankMD still reads the dead `~/.config/omarchy/current`, see
//! [`crate::legacy`]). The theme id stays `omarchy` (**[COMPAT]** `.fed`).

use crate::{Mode, Rgb, ThemeError, UiTokens};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where the active Omarchy theme lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmarchyPaths {
    /// `~/.local/state/omarchy/current` (watch this dir, not the files: the
    /// `theme/` dir is replaced by rename on every switch).
    pub current: PathBuf,
}

impl OmarchyPaths {
    /// `<home>/.local/state/omarchy/current`.
    pub fn from_home(home: &Path) -> Self {
        todo!()
    }
    /// `<current>/theme/colors.toml`.
    pub fn colors_toml(&self) -> PathBuf {
        todo!()
    }
    /// `<current>/theme/light.mode` (presence = light, legacy signal).
    pub fn light_mode_file(&self) -> PathBuf {
        todo!()
    }
    /// `<current>/theme.name`.
    pub fn theme_name(&self) -> PathBuf {
        todo!()
    }
    /// `<current>/theme.generation` (opaque token rewritten on every switch, 4.5).
    pub fn generation(&self) -> PathBuf {
        todo!()
    }
}

/// Omarchy palette after the alias cascade.
#[derive(Debug, Clone, PartialEq)]
pub struct OmarchyPalette {
    /// Resolved mode (`mode` → `theme_type` → `light.mode` → luminance `R+G+B > 382`).
    pub mode: Mode,
    /// Canonical + derived + alias keys (`background`, `accent`, `orange`,
    /// `bright_red`, `color0`..`color15`, short names, ...).
    pub colors: BTreeMap<String, Rgb>,
}

impl OmarchyPalette {
    /// Parse `colors.toml` text; `light_mode_file` = presence of `light.mode`.
    /// Applies the full `resolve_theme_colors` cascade (§2.2).
    pub fn parse(colors_toml: &str, light_mode_file: bool) -> Result<Self, ThemeError> {
        todo!("omarchy-4.5.md §2.2-2.3")
    }

    /// One resolved color.
    pub fn get(&self, key: &str) -> Option<Rgb> {
        todo!()
    }
}

/// Map an Omarchy palette to UI tokens (§2.6 table), contrast-correcting
/// syntax/text tokens to ≥ 4.5:1 against `code-bg` (OM-004).
pub fn from_omarchy(p: &OmarchyPalette) -> UiTokens {
    todo!("omarchy-4.5.md §2.6")
}

/// Read the active theme. `Ok(None)` when Omarchy is not installed (the
/// `current` dir does not exist). `Err` when `current` exists but
/// `theme/colors.toml` is missing (mid-swap: `rm -rf theme && mv next-theme
/// theme`), unreadable or unparsable — the caller keeps the last good theme
/// and retries on the next watcher event (HP-024). Returns `(theme name,
/// palette)`; the name is `theme.name` stripped (empty if absent).
pub fn load_current(paths: &OmarchyPaths) -> Result<Option<(String, OmarchyPalette)>, ThemeError> {
    todo!()
}
