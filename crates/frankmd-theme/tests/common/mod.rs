//! Shared helpers for frankmd-theme tests (filesystem setup is test-only).
#![allow(dead_code, unreachable_pub, clippy::disallowed_methods, clippy::unwrap_used, clippy::missing_panics_doc)]

use frankmd_theme::{Rgb, contrast_ratio};
use serde_json::Value as J;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub fn rgb(hex: &str) -> Rgb {
    Rgb::parse_hex(hex).unwrap_or_else(|| panic!("bad hex {hex}"))
}

/// Relative float comparison for values the original computed in f64.
pub fn approx(got: f64, want: f64) -> bool {
    (got - want).abs() <= 1e-9 * want.abs().max(1.0)
}

pub fn eq(what: &str, got: &J, want: &J) -> Result<(), String> {
    if got == want { Ok(()) } else { Err(format!("{what}:\n    got  {got}\n    want {want}")) }
}

/// FrankMD's own test helper (`assert_syntax_contrast`): every syntax role ≥ 4.5 vs code-bg.
pub fn syntax_contrast_failures(get: impl Fn(&str) -> Rgb) -> Vec<String> {
    let code_bg = get("code-bg");
    frankmd_theme::SYNTAX_ROLES
        .iter()
        .filter_map(|role| {
            let fg = get(&format!("syntax-{role}"));
            let r = contrast_ratio(fg, code_bg);
            (r < 4.5).then(|| format!("syntax-{role} {fg:?} on {code_bg:?} = {r:.3}"))
        })
        .collect()
}

/// Minimal base64 decoder for the binary fixture inputs.
pub fn base64_decode(s: &str) -> Vec<u8> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let (mut out, mut buf, mut bits) = (Vec::new(), 0u32, 0);
    for c in s.bytes().filter(|c| *c != b'=') {
        buf = (buf << 6) | T.iter().position(|t| *t == c).unwrap() as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

/// A fake `$HOME` with an Omarchy `current` dir and a sibling themes dir.
pub struct OmarchyHome {
    pub dir: tempfile::TempDir,
}

impl OmarchyHome {
    pub fn new() -> Self {
        Self { dir: tempfile::tempdir().unwrap() }
    }
    pub fn home(&self) -> &Path {
        self.dir.path()
    }
    pub fn themes(&self) -> PathBuf {
        self.home().join("themes")
    }

    /// Materialize an `omarchy_theme.json` file spec under `base`:
    /// `"path": "text"`, `"dir/": null`, `{"symlink": "<THEMES>/x"}`,
    /// `{"mode": 0, "content": ..}`, `{"base64": ..}`.
    pub fn put_spec(&self, base: &Path, spec: &J) {
        for (rel, v) in spec.as_object().into_iter().flatten() {
            let path = base.join(rel.trim_end_matches('/'));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            match v {
                J::Null => std::fs::create_dir_all(&path).unwrap(),
                J::String(s) => std::fs::write(&path, s).unwrap(),
                J::Object(o) if o.contains_key("symlink") => {
                    let target = o["symlink"].as_str().unwrap().replace("<THEMES>", &self.themes().to_string_lossy());
                    std::os::unix::fs::symlink(target, &path).unwrap();
                }
                J::Object(o) if o.contains_key("base64") => {
                    std::fs::write(&path, base64_decode(o["base64"].as_str().unwrap())).unwrap();
                }
                J::Object(o) if o.contains_key("mode") => {
                    std::fs::write(&path, o["content"].as_str().unwrap_or_default()).unwrap();
                    let mode = o["mode"].as_u64().unwrap() as u32;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
                }
                other => panic!("unsupported spec value for {rel}: {other}"),
            }
        }
    }

    pub fn write(&self, rel: &str, text: &str) {
        let path = self.home().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    pub fn mkdir(&self, rel: &str) {
        std::fs::create_dir_all(self.home().join(rel)).unwrap();
    }
}

/// `true` when running as root (EACCES cases cannot be exercised).
pub fn is_root() -> bool {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("probe");
    std::fs::write(&p, "x").unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();
    std::fs::read(&p).is_ok()
}
