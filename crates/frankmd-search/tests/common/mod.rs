//! Shared helpers for frankmd-search's integration tests.
//!
//! The golden search trees need features `TempTree` does not have (pinned
//! mtimes, base64 bytes, `<OUTSIDE>` symlink targets), so they are built here.
#![allow(
    dead_code,
    unreachable_pub,
    clippy::disallowed_methods,
    clippy::unwrap_used
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use frankmd_search::content::Hit;
use frankmd_testkit::serde_json::{Map, Value, json};

/// `NotesFS::BASE_MTIME` of the Ruby generators.
pub const BASE_MTIME: i64 = 1_700_000_000;

/// A sandbox like `NotesFS.sandbox`: `<tmp>/notes` (the vault) and `<tmp>/outside`.
pub struct Sandbox {
    _tmp: tempfile::TempDir,
    pub root: PathBuf,
    pub outside: PathBuf,
}

impl Sandbox {
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let root = base.join("notes");
        let outside = base.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        Self {
            _tmp: tmp,
            root,
            outside,
        }
    }

    /// Build a fixture tree spec under the vault. Values: `"content"`,
    /// `{"content","mtime"}`, `{"base64"}`, `{"symlink": "<OUTSIDE>|<ROOT>|rel"}`,
    /// `"dir/": null`. `mtime_for(rel)` overrides the default mtime.
    pub fn build(&self, base: &Path, spec: &Value, mtime_for: &dyn Fn(&str) -> Option<f64>) {
        let Some(map) = spec.as_object() else { return };
        for (i, (rel, val)) in map.iter().enumerate() {
            let path = base.join(rel.trim_end_matches('/'));
            if rel.ends_with('/') {
                std::fs::create_dir_all(&path).unwrap();
                continue;
            }
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            if let Some(target) = val.get("symlink").and_then(Value::as_str) {
                let target = self.subst(target);
                std::os::unix::fs::symlink(target, &path).unwrap();
                continue;
            }
            let bytes = if let Some(b64) = val.get("base64").and_then(Value::as_str) {
                base64_decode(b64)
            } else if let Some(c) = val.get("content") {
                c.as_str().unwrap_or_default().as_bytes().to_vec()
            } else {
                val.as_str().unwrap_or_default().as_bytes().to_vec()
            };
            std::fs::write(&path, bytes).unwrap();
            let mtime = mtime_for(rel)
                .or_else(|| val.get("mtime").and_then(Value::as_f64))
                .unwrap_or((BASE_MTIME - 100 * i as i64) as f64);
            set_mtime(&path, mtime);
        }
    }

    pub fn subst(&self, s: &str) -> String {
        s.replace("<ROOT>", &self.root.display().to_string())
            .replace("<OUTSIDE>", &self.outside.display().to_string())
    }

    /// Write a vault file with a pinned mtime.
    pub fn note(&self, rel: &str, content: impl AsRef<[u8]>, mtime: f64) -> PathBuf {
        let path = self.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        set_mtime(&path, mtime);
        path
    }
}

/// Seconds since the epoch (fractional, may be negative) → `SystemTime`.
pub fn system_time(secs: f64) -> SystemTime {
    if secs >= 0.0 {
        UNIX_EPOCH + Duration::from_secs_f64(secs)
    } else {
        UNIX_EPOCH - Duration::from_secs_f64(-secs)
    }
}

/// `File.utime(m, m, path)`.
pub fn set_mtime(path: &Path, secs: f64) {
    let f = std::fs::File::options().write(true).open(path).unwrap();
    let t = system_time(secs);
    f.set_times(std::fs::FileTimes::new().set_accessed(t).set_modified(t))
        .unwrap();
}

/// Truncate/extend a file (NUL padded), like `File.truncate`.
pub fn set_len(path: &Path, len: u64) {
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_len(len)
        .unwrap();
}

/// Milliseconds since the epoch, floored (Ruby `Time#xmlschema(3)` truncates).
fn millis(t: SystemTime) -> i64 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(e) => {
            let d = e.duration();
            let ms = d.as_millis() as i64;
            if d.as_nanos() % 1_000_000 == 0 {
                -ms
            } else {
                -ms - 1
            }
        }
    }
}

/// ActiveSupport JSON for a `Time` in UTC: `2023-11-14T22:13:20.000+00:00`.
pub fn iso_millis_utc(t: SystemTime) -> String {
    let ms = millis(t);
    let secs = ms.div_euclid(1000);
    let frac = ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{frac:03}+00:00",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// Parse [`iso_millis_utc`] output back to fractional seconds.
pub fn parse_iso_millis_utc(s: &str) -> f64 {
    let n = |r: std::ops::Range<usize>| s[r].parse::<i64>().unwrap();
    let days = days_from_civil(n(0..4), n(5..7), n(8..10));
    let secs = days * 86_400 + n(11..13) * 3600 + n(14..16) * 60 + n(17..19);
    secs as f64 + n(20..23) as f64 / 1000.0
}

// Howard Hinnant's civil-date algorithms.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// A hit as `render json:` serializes it.
pub fn hit_json(h: &Hit) -> Value {
    let context: Vec<Value> = h
        .context
        .iter()
        .map(
            |c| json!({"line_number": c.line_number, "content": c.content, "is_match": c.is_match}),
        )
        .collect();
    let mut m = Map::new();
    m.insert("line_number".into(), json!(h.line_number));
    m.insert("match_text".into(), json!(h.match_text));
    m.insert("context".into(), Value::Array(context));
    m.insert("path".into(), json!(h.path));
    m.insert("name".into(), json!(h.name));
    m.insert("mtime".into(), json!(iso_millis_utc(h.mtime)));
    Value::Object(m)
}

/// Minimal standard base64 decoder (fixture `{"base64": ...}` file bodies).
pub fn base64_decode(s: &str) -> Vec<u8> {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes().filter(|c| *c != b'=' && !c.is_ascii_whitespace()) {
        let v = A.iter().position(|a| *a == c).unwrap() as u32;
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

/// Vault-relative paths of hits, in result order.
pub fn paths(hits: &[Hit]) -> Vec<&str> {
    hits.iter().map(|h| h.path.as_str()).collect()
}
