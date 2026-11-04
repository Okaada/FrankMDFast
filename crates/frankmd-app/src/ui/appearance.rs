//! Which palette the window shows (ui-spec §5.17, OM-005): the `.fed`
//! `theme` value resolved against Omarchy's live `colors.toml`, the 18
//! built-ins and the `.fed` custom theme. Pure resolution; the bridge applies it.

use frankmd_config::{Key, ResolvedConfig, Value};
use frankmd_theme::omarchy::load_current;
use frankmd_theme::{OmarchyPaths, UiTokens, builtin, from_omarchy};

/// `.fed` `theme` value that follows Omarchy (also the default when Omarchy is present).
pub const OMARCHY: &str = "omarchy";
/// `.fed` `theme` value of the custom theme.
pub const CUSTOM: &str = "custom";
/// Built-in used when nothing else resolves.
pub const FALLBACK: &str = "dark";

/// A resolved palette.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTheme {
    /// Colours.
    pub tokens: UiTokens,
    /// The theme id actually shown (`omarchy`, a built-in id, `custom`).
    pub id: String,
    /// Omarchy theme name when following Omarchy.
    pub omarchy_name: Option<String>,
}

/// The current Omarchy palette, `None` when Omarchy is absent or mid-swap (HP-024).
pub fn omarchy_tokens(paths: Option<&OmarchyPaths>) -> Option<(String, UiTokens)> {
    let paths = paths?;
    match load_current(paths) {
        Ok(Some((name, palette))) => Some((name, from_omarchy(&palette))),
        Ok(None) => None,
        Err(err) => {
            tracing::warn!(%err, "omarchy theme unreadable");
            None
        }
    }
}

/// Resolve theme `id` (a `.fed` `theme` value; `None` = default).
pub fn resolve_id(
    id: Option<&str>,
    config: &ResolvedConfig,
    omarchy: Option<&OmarchyPaths>,
) -> ResolvedTheme {
    let id = id.map(str::trim).filter(|s| !s.is_empty());
    match id {
        None | Some(OMARCHY) => {
            if let Some((name, tokens)) = omarchy_tokens(omarchy) {
                return ResolvedTheme {
                    tokens,
                    id: OMARCHY.to_owned(),
                    omarchy_name: Some(name),
                };
            }
        }
        Some(CUSTOM) => {
            if let Some(Value::CustomTheme(custom)) = config.value(Key::CustomTheme)
                && let Some(base) = builtin(&custom.base)
            {
                return ResolvedTheme {
                    tokens: base.with_overrides(&custom),
                    id: CUSTOM.to_owned(),
                    omarchy_name: None,
                };
            }
        }
        Some(other) => {
            if let Some(tokens) = builtin(other) {
                return ResolvedTheme {
                    tokens,
                    id: other.to_owned(),
                    omarchy_name: None,
                };
            }
        }
    }
    fallback()
}

/// Resolve the configured theme.
pub fn resolve(config: &ResolvedConfig, omarchy: Option<&OmarchyPaths>) -> ResolvedTheme {
    let id = config
        .value(Key::Theme)
        .and_then(|v| v.as_str().map(str::to_owned));
    resolve_id(id.as_deref(), config, omarchy)
}

fn fallback() -> ResolvedTheme {
    let tokens = builtin(FALLBACK)
        .or_else(|| {
            frankmd_theme::builtin_ids()
                .first()
                .and_then(|id| builtin(id))
        })
        .unwrap_or_else(|| UiTokens {
            mode: frankmd_theme::Mode::Dark,
            colors: Default::default(),
        });
    ResolvedTheme {
        tokens,
        id: FALLBACK.to_owned(),
        omarchy_name: None,
    }
}

/// Hyprland's `decoration:rounding` (OM-015), `None` without Hyprland.
/// Runs `hyprctl`; call it off the UI thread.
pub fn hyprland_rounding() -> Option<f32> {
    let out = std::process::Command::new("hyprctl")
        .args(["-j", "getoption", "decoration:rounding"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    #[allow(clippy::cast_possible_truncation)]
    json.get("int")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(0, 32) as f32)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_methods)]
mod tests {
    use super::*;
    use frankmd_config::EnvSnapshot;

    fn config(fed: &str) -> ResolvedConfig {
        ResolvedConfig::new(Some(fed), EnvSnapshot::default())
    }

    #[test]
    fn builtin_ids_resolve_and_unknown_falls_back() {
        let cfg = config("");
        assert_eq!(resolve_id(Some("nord"), &cfg, None).id, "nord");
        assert_eq!(resolve_id(Some("no-such-theme"), &cfg, None).id, FALLBACK);
    }

    #[test]
    fn omarchy_without_omarchy_falls_back() {
        let cfg = config("theme = omarchy\n");
        let r = resolve(&cfg, None);
        assert_eq!(r.id, FALLBACK);
        assert_eq!(r.omarchy_name, None);
    }

    #[test]
    fn omarchy_palette_is_followed_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let paths = OmarchyPaths::from_home(dir.path());
        std::fs::create_dir_all(paths.colors_toml().parent().unwrap()).unwrap();
        std::fs::write(
            paths.colors_toml(),
            "background = \"#1a1b26\"\nforeground = \"#c0caf5\"\naccent = \"#7aa2f7\"\n",
        )
        .unwrap();
        let r = resolve(&config(""), Some(&paths));
        assert_eq!(r.id, OMARCHY);
    }
}
