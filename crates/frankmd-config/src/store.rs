//! The config store: owns `.fed` for one vault, serializes writes under the
//! **[COMPAT]** config lock, and publishes a fresh [`ResolvedConfig`] after
//! each load/write/external change.

use crate::{ConfigError, EnvSnapshot, Key, ResolvedConfig, Value, fed};
use frankmd_fs::atomic::{ModePolicy, atomic_replace};
use frankmd_fs::lock::{CONFIG_LOCK, LockGuard, LockKind};
use frankmd_fs::{FsError, RecentStore, Vault};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// `.fed` file name at the notes root (**[COMPAT]** `Config::CONFIG_FILE`).
pub const CONFIG_FILE: &str = ".fed";

/// **[COMPAT]** `.fed` is always left with mode 0600 after a write.
const FED_MODE: ModePolicy = ModePolicy::Fixed(0o600);

/// Owner of `.fed` for one vault.
///
/// The snapshot sits behind a mutex so the `&self` [`RecentStore`] methods
/// can publish a new one; cross-process (and cross-instance) exclusion is the
/// `flock` on [`CONFIG_LOCK`].
#[derive(Debug)]
pub struct ConfigStore {
    vault: Vault,
    env: EnvSnapshot,
    global: Option<PathBuf>,
    current: Mutex<Arc<ResolvedConfig>>,
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
        let store = Self {
            vault,
            env,
            global: None,
            current: Mutex::new(Arc::new(ResolvedConfig::new(None, EnvSnapshot::default()))),
        };
        store.load_from_disk();
        store
    }

    /// Attach the global settings layer (`config.toml`, see
    /// [`crate::global`]) and refresh [`Self::current`]. A missing file is fine;
    /// problems become [`ResolvedConfig::warnings`].
    #[must_use]
    pub fn with_global(mut self, path: PathBuf) -> Self {
        self.global = Some(path);
        let rc = self.read_snapshot();
        self.publish(rc);
        self
    }

    /// Path of the attached global config, if any.
    pub fn global_path(&self) -> Option<&Path> {
        self.global.as_deref()
    }

    /// Apply `changes` to the global config file (`None` removes a key) and
    /// refresh [`Self::current`]. `.fed` is not touched.
    pub fn update_global(
        &mut self,
        changes: &[(Key, Option<Value>)],
    ) -> Result<Arc<ResolvedConfig>, ConfigError> {
        let path = self.global.clone().ok_or(ConfigError::NoGlobalConfig)?;
        crate::global::write_global(&path, changes)?;
        let rc = self.read_snapshot();
        Ok(self.publish(rc))
    }

    /// Absolute path of `.fed`.
    pub fn path(&self) -> PathBuf {
        self.vault.root().join(CONFIG_FILE)
    }

    /// Current immutable snapshot (cheap clone).
    pub fn current(&self) -> Arc<ResolvedConfig> {
        self.current
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// `Config#update` / `#set`: format (validate) **all** changes first, then
    /// under the lock re-run `ensure_config_file`, re-read `.fed`, apply
    /// [`crate::fed::apply_changes`] and replace it atomically with mode 0600.
    /// Nothing is written if any change is invalid. An empty `changes` writes
    /// nothing. Returns the new snapshot (re-parsed from what was written).
    pub fn update(
        &mut self,
        changes: &[(Key, Option<Value>)],
    ) -> Result<Arc<ResolvedConfig>, ConfigError> {
        let current = self.current();
        let lines = changes
            .iter()
            .map(|(key, value)| Ok((*key, current.format_line(*key, value.as_ref())?)))
            .collect::<Result<Vec<_>, ConfigError>>()?;
        if lines.is_empty() {
            return Ok(current);
        }
        let _guard = self.lock()?;
        let _ = self.ensure_config_file();
        let text = self.read_text()?;
        self.write(&fed::apply_changes(&text, &lines))
    }

    /// **[COMPAT]** `PATCH /config`: keep only `UI_KEYS` (others are dropped
    /// silently, e.g. `aws_access_key_id`, `recent_files`), fail with
    /// [`ConfigError::NoValidSettings`] if nothing remains, else [`Self::update`].
    pub fn update_ui_settings(
        &mut self,
        changes: &[(Key, Option<Value>)],
    ) -> Result<Arc<ResolvedConfig>, ConfigError> {
        let ui: Vec<(Key, Option<Value>)> = changes
            .iter()
            .filter(|(k, _)| k.spec().ui)
            .cloned()
            .collect();
        if ui.is_empty() {
            return Err(ConfigError::NoValidSettings);
        }
        self.update(&ui)
    }

    /// Read-modify-write one key under the lock (`Config#modify`, used by
    /// recents): `f` sees the value resolved from the **fresh disk state**.
    pub fn modify(
        &mut self,
        key: Key,
        f: impl FnOnce(Option<Value>) -> Option<Value>,
    ) -> Result<Arc<ResolvedConfig>, ConfigError> {
        let _guard = self.lock()?;
        let _ = self.ensure_config_file();
        let text = self.read_text()?;
        let fresh = self.with_global_layer(ResolvedConfig::new(Some(&text), self.env.clone()));
        let new = recast(f(fresh.get(key).value));
        let line = fresh.format_line(key, new.as_ref())?;
        self.write(&fed::apply_changes(&text, &[(key, line)]))
    }

    /// Re-read after the watcher reports an external change to `.fed` or the
    /// global config.
    pub fn reload(&mut self) -> Result<Arc<ResolvedConfig>, ConfigError> {
        Ok(self.load_from_disk())
    }

    // ------------------------------------------------------------ internals

    /// `Config.new`: `ensure_config_file`, then parse whatever is on disk.
    fn load_from_disk(&self) -> Arc<ResolvedConfig> {
        let mut warnings = Vec::new();
        match self.lock() {
            Ok(_guard) => {
                if let Err(e) = self.ensure_config_file() {
                    warnings.push(format!("Cannot prepare .fed config: {e}"));
                }
            }
            Err(e) => warnings.push(format!("Cannot prepare .fed config: {e}")),
        }
        let mut rc = self.read_snapshot();
        warnings.append(&mut rc.warnings);
        rc.warnings = warnings;
        self.publish(rc)
    }

    /// Parse `.fed` as it is on disk now (no template/upgrade step).
    fn read_snapshot(&self) -> ResolvedConfig {
        let path = self.path();
        let rc = if !path.exists() {
            ResolvedConfig::new(None, self.env.clone())
        } else {
            match std::fs::read(&path) {
                Ok(bytes) => ResolvedConfig::from_bytes(Some(&bytes), self.env.clone()),
                Err(e) => ResolvedConfig::failed(
                    self.env.clone(),
                    format!("Failed to load .fed config: {e}"),
                ),
            }
        };
        self.with_global_layer(rc)
    }

    /// Add the global layer (and its load warnings) to a `.fed` snapshot.
    fn with_global_layer(&self, rc: ResolvedConfig) -> ResolvedConfig {
        let Some(path) = &self.global else {
            return rc;
        };
        let (values, mut warnings) = crate::global::load_global(path);
        let mut rc = rc.with_global(values);
        rc.warnings.append(&mut warnings);
        rc
    }

    fn publish(&self, rc: ResolvedConfig) -> Arc<ResolvedConfig> {
        let rc = Arc::new(rc);
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = rc.clone();
        rc
    }

    /// The exclusive config lock. **[COMPAT]** a symlinked lock file is an
    /// error (Ruby opens it `O_NOFOLLOW` → ELOOP) rather than a fallback.
    fn lock(&self) -> Result<LockGuard, FsError> {
        let shared = self.vault.real_root().join(CONFIG_LOCK);
        if std::fs::symlink_metadata(&shared).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(FsError::Io(frankmd_fs::errno::error(
                frankmd_fs::errno::ELOOP,
            )));
        }
        self.vault.lock(CONFIG_LOCK, LockKind::Exclusive)
    }

    /// **[COMPAT]** `ensure_config_file` (caller holds the lock): create the
    /// template when `.fed` is missing (a dangling symlink counts as
    /// missing), else append the AI section when it is absent.
    fn ensure_config_file(&self) -> Result<(), FsError> {
        let path = self.path();
        if !path.exists() {
            return atomic_replace(&path, fed::template().as_bytes(), FED_MODE);
        }
        let text = self.read_text()?;
        match fed::upgrade_ai_section(&text) {
            Some(upgraded) => atomic_replace(&path, upgraded.as_bytes(), FED_MODE),
            None => Ok(()),
        }
    }

    /// `.fed` as UTF-8 text.
    fn read_text(&self) -> Result<String, FsError> {
        let bytes = std::fs::read(self.path())?;
        String::from_utf8(bytes).map_err(|_| {
            FsError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid byte sequence in UTF-8",
            ))
        })
    }

    /// Replace `.fed` with `text` (0600) and publish its parse.
    fn write(&self, text: &str) -> Result<Arc<ResolvedConfig>, ConfigError> {
        atomic_replace(&self.path(), text.as_bytes(), FED_MODE)?;
        let rc = self.with_global_layer(ResolvedConfig::new(Some(text), self.env.clone()));
        Ok(self.publish(rc))
    }

    /// `modify` for [`RecentStore`]: the closure's compact JSON text is
    /// written verbatim (keeps `{path, opened_at}` key order).
    fn modify_recent(
        &self,
        f: &mut dyn FnMut(Option<String>) -> Result<String, FsError>,
    ) -> Result<bool, FsError> {
        let Ok(_guard) = self.lock() else {
            return Ok(false); // Ruby `modify` rescues SystemCallError → false
        };
        let _ = self.ensure_config_file();
        let Ok(text) = self.read_text() else {
            return Ok(false);
        };
        let fresh = ResolvedConfig::new(Some(&text), self.env.clone());
        let json = f(recent_raw(&fresh))?;
        // Compact JSON never contains a raw line break; anything else is not
        // ours to write into `.fed`.
        if json.contains(['\n', '\r']) || serde_json::from_str::<serde_json::Value>(&json).is_err()
        {
            return Ok(false);
        }
        let line = format!("{} = {json}", Key::RecentFiles.name());
        Ok(self
            .write(&fed::apply_changes(&text, &[(Key::RecentFiles, line)]))
            .is_ok())
    }
}

/// `set` re-casts the block's result: a blank string becomes nil.
fn recast(value: Option<Value>) -> Option<Value> {
    match value {
        Some(Value::Str(s)) if crate::value::ruby_strip(&s).is_empty() => None,
        other => other,
    }
}

/// The `recent_files` value present in `.fed`, as JSON text.
fn recent_raw(rc: &ResolvedConfig) -> Option<String> {
    rc.file_values()
        .iter()
        .find(|(k, _)| *k == Key::RecentFiles)
        .and_then(|(_, v)| v.as_ref())
        .map(|v| v.to_json().to_string())
}

impl RecentStore for ConfigStore {
    fn load(&self) -> Option<String> {
        recent_raw(&self.publish(self.read_snapshot()))
    }

    fn modify(
        &self,
        f: &mut dyn FnMut(Option<String>) -> Result<String, FsError>,
    ) -> Result<bool, FsError> {
        self.modify_recent(f)
    }

    fn is_present(&self) -> bool {
        std::fs::metadata(self.path()).is_ok_and(|m| m.is_file())
    }
}
