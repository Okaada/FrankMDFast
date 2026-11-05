//! The **global** settings layer: `~/.config/frankmdfast/config.toml`, a flat TOML
//! table using the `.fed` key names. It sits between a vault's `.fed` and ENV
//! (`.fed > global > ENV > default`). The `.fed` format itself is untouched.

use crate::dirs::AppDirs;
use crate::value::{Value, cast};
use crate::{ConfigError, Key, KeyType};
use frankmd_fs::FsError;
use frankmd_fs::atomic::{ModePolicy, atomic_replace};
use std::path::{Path, PathBuf};

/// File name of the global settings inside the app config dir.
pub const GLOBAL_CONFIG_FILE: &str = "config.toml";

/// Global files are private: they may hold API keys.
const GLOBAL_MODE: ModePolicy = ModePolicy::Fixed(0o600);

/// `<config dir>/config.toml`.
pub fn global_config_path(dirs: &AppDirs) -> PathBuf {
    dirs.config.join(GLOBAL_CONFIG_FILE)
}

/// Keys that never live in the global file: vault-specific history and the
/// per-vault auth token.
pub fn global_forbidden(key: Key) -> bool {
    matches!(key, Key::RecentFiles | Key::AuthToken)
}

/// Load the global file. A missing file is empty with no warning; an
/// unreadable or malformed file is empty with one warning. Unknown keys,
/// forbidden keys and mistyped values are skipped with a warning naming the
/// key (never the value).
pub fn load_global(path: &Path) -> (Vec<(Key, Option<Value>)>, Vec<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Vec::new(), Vec::new()),
        Err(e) => {
            return (
                Vec::new(),
                vec![format!("Failed to load global config: {e}")],
            );
        }
    };
    let table = match text.parse::<toml::Table>() {
        Ok(t) => t,
        Err(e) => {
            return (
                Vec::new(),
                vec![format!("Failed to parse global config: {}", e.message())],
            );
        }
    };
    let mut values = Vec::new();
    let mut warnings = Vec::new();
    for (name, raw) in &table {
        let Some(key) = Key::from_name(name) else {
            warnings.push(format!("Global config: unknown key {name}"));
            continue;
        };
        if global_forbidden(key) {
            warnings.push(format!(
                "Global config: {name} is not allowed in the global config"
            ));
            continue;
        }
        match from_toml(key, raw) {
            Ok(v) => values.push((key, v)),
            Err(reason) => {
                warnings.push(format!("Global config: invalid value for {name}: {reason}"))
            }
        }
    }
    (values, warnings)
}

fn from_toml(key: Key, raw: &toml::Value) -> Result<Option<Value>, String> {
    use toml::Value as T;
    let ty = key.spec().ty;
    match (ty, raw) {
        (_, T::String(s)) if s.trim().is_empty() => Ok(None),
        (KeyType::String, T::String(s)) => Ok(Some(Value::Str(s.clone()))),
        (KeyType::Integer, T::Integer(i)) => Ok(Some(Value::Int(i128::from(*i)))),
        (KeyType::Boolean, T::Boolean(b)) => Ok(Some(Value::Bool(*b))),
        (KeyType::Json | KeyType::CustomTheme, T::String(s)) => cast(s, ty),
        (KeyType::String, _) => Err("expected a string".to_owned()),
        (KeyType::Integer, _) => Err("expected an integer".to_owned()),
        (KeyType::Boolean, _) => Err("expected a boolean".to_owned()),
        (KeyType::Json | KeyType::CustomTheme, _) => {
            Err("expected a string holding JSON".to_owned())
        }
    }
}

fn to_toml(value: &Value) -> Result<toml::Value, String> {
    Ok(match value {
        Value::Str(s) => toml::Value::String(s.clone()),
        Value::Int(i) => {
            toml::Value::Integer(i64::try_from(*i).map_err(|_| "integer out of range".to_owned())?)
        }
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Json(j) => toml::Value::String(j.to_string()),
        Value::CustomTheme(t) => toml::Value::String(t.to_json_string()),
    })
}

/// Apply `changes` to the global file (`None` removes the key), keeping every
/// other key. Atomic, mode 0600, parent directory created. Refuses
/// `recent_files` and `auth_token` before touching the disk.
pub fn write_global(path: &Path, changes: &[(Key, Option<Value>)]) -> Result<(), ConfigError> {
    if let Some((key, _)) = changes.iter().find(|(k, _)| global_forbidden(*k)) {
        return Err(ConfigError::GlobalKeyNotAllowed { key: *key });
    }
    let mut table = match std::fs::read_to_string(path) {
        Ok(text) => text
            .parse::<toml::Table>()
            .map_err(|e| ConfigError::GlobalFile(e.message().to_owned()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => toml::Table::new(),
        Err(e) => return Err(FsError::Io(e).into()),
    };
    for (key, value) in changes {
        match value {
            None => {
                table.remove(key.name());
            }
            Some(v) => {
                let t =
                    to_toml(v).map_err(|reason| ConfigError::InvalidValue { key: *key, reason })?;
                table.insert(key.name().to_owned(), t);
            }
        }
    }
    let text = toml::to_string(&table).map_err(|e| ConfigError::GlobalFile(e.to_string()))?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        // frankmd-fs has no mkdir helper; the global dir is outside any vault.
        #[allow(clippy::disallowed_methods)]
        std::fs::create_dir_all(parent).map_err(FsError::Io)?;
    }
    atomic_replace(path, text.as_bytes(), GLOBAL_MODE)?;
    Ok(())
}
