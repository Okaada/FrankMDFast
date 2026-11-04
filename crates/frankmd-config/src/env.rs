//! ENV as an explicit, injectable snapshot (HP-026: no process-global reads
//! inside logic; tests pass their own map).

use std::collections::BTreeMap;

/// Immutable copy of the environment variables FrankMD cares about.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvSnapshot(BTreeMap<String, String>);

impl EnvSnapshot {
    /// Capture from the real process environment (called once at startup,
    /// after optional `.env` loading via dotenvy in the app crate).
    pub fn capture() -> Self {
        Self(std::env::vars().collect())
    }

    /// Build from pairs (tests).
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self(pairs.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }

    /// **[COMPAT]** Rails `present?`: `Some` only if non-empty after trim.
    pub fn present(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str).filter(|v| !v.trim().is_empty())
    }

    /// Raw lookup (an empty string counts as *set*, e.g. `IMAGES_PATH` boot rule).
    pub fn raw(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// **[COMPAT]** `config/initializers/images.rb` boot rule, as a new
    /// snapshot instead of an ENV mutation: if `IMAGES_PATH` is unset (an
    /// empty string counts as set) use `XDG_PICTURES_DIR` verbatim if set,
    /// else `$HOME/Pictures` when `dir_exists` says it is a directory.
    pub fn with_images_path_default(&self, dir_exists: impl Fn(&std::path::Path) -> bool) -> Self {
        todo!()
    }
}
