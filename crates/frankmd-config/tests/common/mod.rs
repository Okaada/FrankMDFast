//! Shared helpers for the frankmd-config ports and golden tests.
//!
//! Test-only code may touch the filesystem directly (setting up vaults,
//! simulating external writers, chmod), hence the lint opt-out.
#![allow(
    dead_code,
    unreachable_pub,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::missing_panics_doc
)]

use frankmd_config::{ConfigStore, EnvSnapshot, Key, ResolvedConfig, Value};
use frankmd_fs::Vault;
use frankmd_testkit::TempTree;
use serde_json::{Map, Value as J, json};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const FED: &str = ".fed";
pub const LOCK: &str = ".frankmd-config.lock";
/// Line the original tests add so `Config.new` does not append the AI section.
pub const AI_MARKER: &str = "# ai_model = placeholder\n";

// ---------------------------------------------------------------- env

/// ENV snapshot from pairs. Every test starts from an *empty* snapshot, which
/// is what the Ruby `setup` blocks achieve by deleting the AI/S3/... vars.
pub fn env(pairs: &[(&str, &str)]) -> EnvSnapshot {
    EnvSnapshot::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
}

/// ENV snapshot from a fixture object, replacing `<ROOT>` with `root`.
pub fn env_json(v: &J, root: &str) -> EnvSnapshot {
    let pairs: Vec<(String, String)> = v
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.as_str().unwrap_or_default().replace("<ROOT>", root),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    EnvSnapshot::from_pairs(pairs)
}

// ---------------------------------------------------------------- values

pub fn key(name: &str) -> Key {
    Key::from_name(name).unwrap_or_else(|| panic!("unknown key {name}"))
}

pub fn opt_json(v: Option<&Value>) -> J {
    v.map_or(J::Null, Value::to_json)
}

pub fn pairs_json(vals: &[(Key, Option<Value>)]) -> J {
    J::Object(
        vals.iter()
            .map(|(k, v)| (k.name().to_owned(), opt_json(v.as_ref())))
            .collect(),
    )
}

pub fn file_values_json(rc: &ResolvedConfig) -> J {
    pairs_json(rc.file_values())
}

pub fn resolved_json(rc: &ResolvedConfig) -> J {
    J::Object(
        Key::ALL
            .iter()
            .map(|k| (k.name().to_owned(), opt_json(rc.get(*k).value.as_ref())))
            .collect(),
    )
}

/// `get(key)` as JSON.
pub fn get_json(rc: &ResolvedConfig, name: &str) -> J {
    opt_json(rc.get(key(name)).value.as_ref())
}

pub fn get_str(rc: &ResolvedConfig, k: Key) -> Option<String> {
    rc.get(k).value.and_then(|v| v.as_str().map(str::to_owned))
}

pub fn get_int(rc: &ResolvedConfig, k: Key) -> Option<i64> {
    rc.get(k).value.and_then(|v| v.as_int())
}

pub fn get_bool(rc: &ResolvedConfig, k: Key) -> Option<bool> {
    rc.get(k).value.and_then(|v| v.as_bool())
}

pub fn ui_settings_json(rc: &ResolvedConfig) -> Map<String, J> {
    match pairs_json(&rc.ui_settings()) {
        J::Object(m) => m,
        _ => unreachable!(),
    }
}

/// Compare two JSON values, reporting a readable diff.
pub fn eq(what: &str, got: &J, want: &J) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{what}:\n    got  {got}\n    want {want}"))
    }
}

/// Typed value from JSON via the public coercion (what the UI hands to `update`).
pub fn val(k: Key, input: J) -> Option<Value> {
    frankmd_config::value::coerce(k, &input).unwrap()
}

pub fn s(v: &str) -> Option<Value> {
    Some(Value::Str(v.to_owned()))
}

// ---------------------------------------------------------------- vaults

/// A notes root plus an app state dir outside it (fallback locks, HP-012).
pub struct TestVault {
    pub notes: TempTree,
    pub state: tempfile::TempDir,
    /// Notes root actually opened (may be a nested, not-yet-existing path).
    pub root: PathBuf,
}

impl TestVault {
    pub fn new() -> Self {
        Self::from_spec(&J::Null)
    }

    /// Notes root built from a testkit tree spec.
    pub fn from_spec(spec: &J) -> Self {
        let notes = TempTree::from_spec(spec);
        let root = notes.path().to_path_buf();
        Self {
            notes,
            state: tempfile::tempdir().unwrap(),
            root,
        }
    }

    /// Notes root with `.fed` = `text`.
    pub fn with_fed(text: &str) -> Self {
        let v = Self::new();
        v.write_fed(text.as_bytes());
        v
    }

    /// Open `<tmp>/<rel>` instead of the temp root itself.
    pub fn at(mut self, rel: &str) -> Self {
        self.root = self.notes.path().join(rel);
        self
    }

    pub fn vault(&self) -> Vault {
        Vault::open(&self.root, self.state.path()).unwrap()
    }

    /// `Config.new(base_path: root)` with `env`.
    pub fn open(&self, env: EnvSnapshot) -> ConfigStore {
        ConfigStore::open(self.vault(), env)
    }

    pub fn open_empty_env(&self) -> ConfigStore {
        self.open(EnvSnapshot::default())
    }

    pub fn fed_path(&self) -> PathBuf {
        self.root.join(FED)
    }

    /// External writer (another FrankMD instance, a sync tool, the user).
    pub fn write_fed(&self, bytes: &[u8]) {
        std::fs::create_dir_all(&self.root).unwrap();
        std::fs::write(self.fed_path(), bytes).unwrap();
    }

    pub fn read_fed(&self) -> String {
        std::fs::read_to_string(self.fed_path()).unwrap()
    }

    pub fn read_fed_bytes(&self) -> Option<Vec<u8>> {
        std::fs::read(self.fed_path()).ok()
    }

    pub fn fed_exists(&self) -> bool {
        self.fed_path().is_file()
    }

    /// Permission bits of `<root>/<rel>` (no symlink follow).
    pub fn mode(&self, rel: &str) -> u32 {
        std::fs::symlink_metadata(self.root.join(rel))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    }

    pub fn chmod(&self, rel: &str, mode: u32) {
        std::fs::set_permissions(self.root.join(rel), std::fs::Permissions::from_mode(mode))
            .unwrap();
    }

    /// Snapshot of the notes temp dir (relative to the temp root, not `root`).
    pub fn snapshot(&self) -> J {
        J::Object(self.notes.snapshot().into_iter().collect())
    }

    pub fn normalize(&self, s: &str) -> String {
        self.notes.normalize(s)
    }
}

/// Make `dir` read-only; returns `false` (and restores) when we run as root,
/// where the permission check cannot be exercised.
pub fn make_readonly(dir: &Path) -> bool {
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    if tempfile::NamedTempFile::new_in(dir).is_ok() {
        restore_writable(dir);
        return false;
    }
    true
}

pub fn restore_writable(dir: &Path) {
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755));
}

pub fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).unwrap();
    }
    std::fs::write(path, bytes).unwrap();
}

pub fn chmod(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// Lines of `text`, stripped, blanks dropped (`content.lines.map(&:strip).reject(&:empty?)`).
pub fn nonblank_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Minimal RFC 4648 base64 decoder for the two binary fixtures.
pub fn base64_decode(s: &str) -> Vec<u8> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes().filter(|c| *c != b'=' && !c.is_ascii_whitespace()) {
        let v = T.iter().position(|t| *t == c).unwrap() as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

/// `"<TEMPLATE>"` placeholder → the golden template text.
pub fn expand_template(v: &J) -> J {
    match v {
        J::String(s) if s == "<TEMPLATE>" => J::String(template_golden()),
        J::Object(o) => J::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), expand_template(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

pub fn template_golden() -> String {
    String::from_utf8(frankmd_testkit::golden_bytes("fed_template.fed")).unwrap()
}

pub fn j(v: impl serde::Serialize) -> J {
    json!(v)
}
