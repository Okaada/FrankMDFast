//! Helpers shared by the rename / backlinks / recents ports (fork D files).
#![allow(dead_code, unreachable_pub, clippy::disallowed_methods, clippy::unwrap_used)]

use crate::common::{self, Sandbox, fed_template};
use frankmd_fs::model::{ModelError, ModelResult};
use frankmd_fs::{FsError, RecentStore};
use frankmd_testkit::serde_json::{self, Value, json};
use std::fs;
use std::path::{Path, PathBuf};

/// Compare an actual error with a fixture `{error, message}`. For `Errno::*`
/// classes only the class is compared (Ruby's `"... @ rb_file_s_rename - (...)"`
/// text is libc/Ruby specific).
pub fn check_error(actual: &FsError, expected: &Value, sb: &Sandbox) -> Result<(), String> {
    let class = expected["error"].as_str().unwrap_or_default();
    let got = common::fs_error_json(actual);
    let got = sb.norm_value(&got);
    if class.starts_with("Errno::") {
        if got["error"] == expected["error"] {
            return Ok(());
        }
        return Err(format!("error class: expected {class}, got {got}"));
    }
    common::check_eq("error", &got, expected)
}

/// `true` / `{error, message}` result of a service op as fixture JSON.
pub fn unit_result<T>(r: Result<T, FsError>) -> Result<(), FsError> {
    r.map(|_| ())
}

/// Compare a service result (`true` or an error) with the fixture.
pub fn check_unit(what: &str, actual: Result<(), FsError>, expected: &Value, sb: &Sandbox) -> Result<(), String> {
    match actual {
        Ok(()) => common::check_eq(what, &Value::Bool(true), expected),
        Err(e) if expected.get("error").is_some() => check_error(&e, expected, sb).map_err(|m| format!("{what}: {m}")),
        Err(e) => Err(format!("{what}: expected {expected}, got error {e} ({})", e.compat_class())),
    }
}

/// Compare a `Result<bool, FsError>` (Config#set result or error).
pub fn check_bool(what: &str, actual: Result<bool, FsError>, expected: &Value, sb: &Sandbox) -> Result<(), String> {
    match actual {
        Ok(b) => common::check_eq(what, &Value::Bool(b), expected),
        Err(e) if expected.get("error").is_some() => check_error(&e, expected, sb).map_err(|m| format!("{what}: {m}")),
        Err(e) => Err(format!("{what}: expected {expected}, got error {e} ({})", e.compat_class())),
    }
}

/// `NotesFS.model_result(ok, model)` → `{ok, errors (full messages), path}`.
/// `orig_path` is the model's path when the op fails; on success the new path.
pub fn model_json(r: ModelResult<String>, orig_path: &str) -> Result<Value, FsError> {
    let r = r?;
    Ok(match r {
        Ok(new_path) => json!({"ok": true, "errors": [], "path": new_path}),
        Err(errs) => json!({"ok": false, "errors": full_messages(&errs), "path": orig_path}),
    })
}

/// English full messages of model errors.
pub fn full_messages(errs: &[ModelError]) -> Vec<String> {
    errs.iter().map(|e| e.full_message(&common::en)).collect()
}

/// `recent_files` JSON value from `.fed` text (Config key match is
/// case-insensitive in Rails).
pub fn fed_recent_value(text: &str) -> Option<Value> {
    let re = regex::Regex::new(r"(?mi)^recent_files\s*=\s*(.*)$").unwrap();
    re.captures(text).and_then(|c| serde_json::from_str(c[1].trim()).ok())
}

/// Lines of `.fed` matching `/\Arecent_files\s*=/` (the fixtures'
/// `fed_recent_files_lines`), `None` when `.fed` is absent.
pub fn fed_recent_lines(root: &Path) -> Option<Vec<String>> {
    let text = fs::read_to_string(root.join(".fed")).ok()?;
    let re = regex::Regex::new(r"^recent_files\s*=").unwrap();
    Some(text.lines().filter(|l| re.is_match(l)).map(str::to_owned).collect())
}

/// Compare `tree_after`: everything but `.fed` exactly; `.fed` present on
/// both sides or neither, and its `recent_files` value equal.
pub fn check_tree_after(sb: &Sandbox, expected: &Value) -> Result<(), String> {
    let mut actual = sb.snapshot();
    let mut exp = common::expected_tree(expected);
    let a_fed = actual.remove(".fed");
    let e_fed = exp.remove(".fed");
    let mut errs = Vec::new();
    if actual != exp {
        errs.push(format!(
            "tree_after (excluding .fed):\n    expected: {}\n    actual:   {}",
            serde_json::to_string(&exp).unwrap(),
            serde_json::to_string(&actual).unwrap()
        ));
    }
    match (a_fed, e_fed) {
        (None, None) => {}
        (Some(a), Some(e)) => {
            let a = fed_recent_value(a.as_str().unwrap_or_default());
            let e = fed_recent_value(&common::expand_fed(e.as_str().unwrap_or_default()));
            if a != e {
                errs.push(format!(".fed recent_files: expected {e:?}, actual {a:?}"));
            }
        }
        (a, e) => errs.push(format!(".fed presence: expected {}, actual {}", e.is_some(), a.is_some())),
    }
    if errs.is_empty() { Ok(()) } else { Err(errs.join("\n")) }
}

/// Like [`common::FedRecentStore`] but matching the key case-insensitively
/// (Rails `Config` parses `/^([a-z0-9_]+)\s*=/i`, so `RECENT_FILES = …` is
/// the same key and is replaced in place).
pub struct CiFedStore {
    /// `<ROOT>/.fed`.
    pub fed: PathBuf,
    /// `<ROOT>/.frankmd-config.lock`.
    pub lock: PathBuf,
}

impl CiFedStore {
    /// Store for a sandbox root.
    pub fn new(root: &Path) -> Self {
        Self { fed: root.join(".fed"), lock: root.join(".frankmd-config.lock") }
    }
}

impl RecentStore for CiFedStore {
    fn load(&self) -> Option<String> {
        let text = fs::read_to_string(&self.fed).ok()?;
        let re = regex::Regex::new(r"(?mi)^recent_files\s*=\s*(.*)$").unwrap();
        re.captures(&text).map(|c| c[1].trim().to_owned())
    }

    fn modify(&self, f: &mut dyn FnMut(Option<String>) -> Result<String, FsError>) -> Result<bool, FsError> {
        fs::OpenOptions::new().create(true).append(true).open(&self.lock)?;
        let text = fs::read_to_string(&self.fed).unwrap_or_else(|_| fed_template());
        let new_json = f(self.load())?;
        let line = format!("recent_files = {new_json}");
        let re = regex::Regex::new(r"(?mi)^#?\s*recent_files\s*=\s*(.*)$").unwrap();
        let out = if re.is_match(&text) {
            re.replace(&text, regex::NoExpand(&line)).into_owned()
        } else {
            let mut t = text.clone();
            if !t.is_empty() && !t.ends_with('\n') {
                t.push('\n');
            }
            t.push_str(&line);
            t.push('\n');
            t
        };
        fs::write(&self.fed, out)?;
        Ok(true)
    }

    fn is_present(&self) -> bool {
        fs::metadata(&self.fed).is_ok_and(|m| m.is_file())
    }
}

/// Recents entries as fixture JSON (`[{path, opened_at}]`, Rails key order
/// is irrelevant for `Value` equality).
pub fn entries_json(entries: &[frankmd_core::recent::RecentFile]) -> Value {
    serde_json::to_value(entries).unwrap()
}

/// Restore `0o755` on every real directory under the root (the generator
/// does this before snapshotting so a `chmod 000` case can be walked/removed).
pub fn unlock_dirs(root: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fn walk(p: &Path) {
        let Ok(md) = fs::symlink_metadata(p) else { return };
        if md.file_type().is_dir() {
            let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o755));
            if let Ok(rd) = fs::read_dir(p) {
                for e in rd.flatten() {
                    walk(&e.path());
                }
            }
        }
    }
    walk(root);
}
