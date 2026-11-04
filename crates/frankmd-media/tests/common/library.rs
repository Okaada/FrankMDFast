//! Helpers for the library / usage / S3-library ports (media-library port).
#![allow(dead_code, unreachable_pub, clippy::disallowed_methods, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use frankmd_media::config::MediaConfig;
use frankmd_media::ctx::MediaCtx;
use frankmd_media::env::{MediaEnv, MemorySettings};
use frankmd_net::{EgressClient, Limits, StaticResolver, TrustedClient};
use frankmd_testkit::TempTree;
use frankmd_testkit::serde_json::Value;

/// A temp notes root plus a sibling "outside" dir (for symlink escapes).
pub struct Notes {
    pub tree: TempTree,
    pub outside: TempTree,
    state: TempTree,
}

impl Notes {
    pub fn new() -> Self {
        Self { tree: TempTree::empty(), outside: TempTree::empty(), state: TempTree::empty() }
    }

    pub fn root(&self) -> &Path {
        self.tree.path()
    }

    pub fn vault(&self) -> frankmd_fs::Vault {
        frankmd_fs::Vault::open(self.root(), self.state.path()).expect("vault")
    }

    /// `write_media` / `create_test_note`.
    pub fn write(&self, rel: &str, contents: &str) -> PathBuf {
        write_file(self.root(), rel, contents.as_bytes())
    }

    /// A file outside the notes root.
    pub fn write_outside(&self, rel: &str, contents: &str) -> PathBuf {
        write_file(self.outside.path(), rel, contents.as_bytes())
    }

    pub fn symlink(&self, rel: &str, target: &Path) {
        let link = self.root().join(rel);
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    /// Materialize a golden tree spec: `"path": "content"`, `{content, mtime}`,
    /// `{symlink: target}` (`<OUTSIDE>/x` → the outside dir), `"dir/": null`.
    pub fn from_golden(tree: &Value, outside_tree: Option<&Value>) -> Self {
        let n = Self::new();
        if let Some(o) = outside_tree.and_then(Value::as_object) {
            for (rel, v) in o {
                materialize(n.outside.path(), rel, v, n.outside.path());
            }
        }
        if let Some(o) = tree.as_object() {
            for (rel, v) in o {
                materialize(n.root(), rel, v, n.outside.path());
            }
        }
        n
    }

    /// Snapshot in the fixtures' `tree_after` encoding.
    pub fn snapshot(&self) -> Value {
        Value::Object(self.tree.snapshot().into_iter().collect())
    }

    pub fn normalize(&self, s: &str) -> String {
        self.tree.normalize(s)
    }
}

pub fn write_file(root: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, bytes).unwrap();
    p
}

pub fn set_mtime(path: &Path, unix_secs: u64) {
    let f = std::fs::File::options().write(true).open(path).unwrap();
    f.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(unix_secs)).unwrap();
}

fn materialize(root: &Path, rel: &str, v: &Value, outside: &Path) {
    if rel.ends_with('/') {
        std::fs::create_dir_all(root.join(rel.trim_end_matches('/'))).unwrap();
        return;
    }
    match v {
        Value::String(s) => {
            write_file(root, rel, s.as_bytes());
        }
        Value::Null => {
            write_file(root, rel, b"");
        }
        Value::Object(o) if o.contains_key("base64") => {
            write_file(root, rel, &unbase64(o["base64"].as_str().unwrap()));
        }
        Value::Object(o) if o.contains_key("symlink") => {
            let target = o["symlink"].as_str().unwrap().replace("<OUTSIDE>", &outside.display().to_string());
            let link = root.join(rel);
            std::fs::create_dir_all(link.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(target, link).unwrap();
        }
        Value::Object(o) => {
            let p = write_file(root, rel, o["content"].as_str().unwrap_or_default().as_bytes());
            if let Some(m) = o.get("mtime").and_then(Value::as_u64) {
                set_mtime(&p, m);
            }
        }
        other => panic!("unsupported tree value for {rel}: {other}"),
    }
}

/// `chmod` helper for EACCES simulations (the original stubbed `File.delete` /
/// `Pathname#read` to raise). Returns a guard that restores 0755 on drop.
pub struct ModeGuard(PathBuf);

impl Drop for ModeGuard {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

pub fn chmod(path: &Path, mode: u32) -> ModeGuard {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    ModeGuard(path.to_owned())
}

/// Running as root defeats permission-based EACCES simulations.
pub fn is_root() -> bool {
    rustix_like_euid() == 0
}

fn rustix_like_euid() -> u32 {
    use std::os::unix::fs::MetadataExt;
    // A file we just created is owned by our euid.
    let t = TempTree::empty();
    let p = write_file(t.path(), "probe", b"");
    std::fs::metadata(p).unwrap().uid()
}

/// `@config.stubs(:upload_extensions)` from the original setups: images `.png .jpg`, videos `.mp4 .webm`.
pub fn config(root: &Path) -> MediaConfig {
    let mut c = MediaConfig::new(root);
    c.image_upload_extensions = vec![".png".into(), ".jpg".into()];
    c.video_upload_extensions = vec![".mp4".into(), ".webm".into()];
    c
}

/// S3 settings of `S3MediaLibraryServiceTest` (bucket `bucket`, us-east-1, no endpoint).
pub fn s3_config(root: &Path) -> MediaConfig {
    let mut c = config(root);
    c.media_storage_provider = Some("s3".into());
    c.s3.access_key_id = Some("AKIATEST".into());
    c.s3.secret_access_key = Some("secret".into());
    c.s3.bucket = Some("bucket".into());
    c.s3.region = Some("us-east-1".into());
    c
}

/// Media services with S3 requests routed to `s3_mock` (path style) while URLs
/// keep the AWS virtual-host form.
pub fn env(config: MediaConfig, s3_mock: Option<&str>) -> MediaEnv {
    let mut env = MediaEnv::new(
        MemorySettings::new(config),
        MediaCtx::pinned("2026-01-01T12:00:00+00:00[UTC]".parse().unwrap()),
        TrustedClient::new().expect("client"),
        EgressClient::with_resolver(Limits::EXTERNAL_IMAGE, Arc::new(StaticResolver::new())).expect("egress"),
    );
    env.s3_transport = s3_mock.map(|u| u.parse().unwrap());
    env
}

/// `ListObjectsV2` XML page.
pub fn list_objects_xml(bucket: &str, objects: &[(&str, u64)], next_token: Option<&str>) -> String {
    let contents: String = objects
        .iter()
        .map(|(k, s)| format!("<Contents><Key>{k}</Key><LastModified>2026-10-16T00:00:00.000Z</LastModified><Size>{s}</Size></Contents>"))
        .collect();
    let trunc = next_token.map_or_else(
        || "<IsTruncated>false</IsTruncated>".to_owned(),
        |t| format!("<IsTruncated>true</IsTruncated><NextContinuationToken>{t}</NextContinuationToken>"),
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>{bucket}</Name><Prefix>frankmd/</Prefix>{trunc}{contents}</ListBucketResult>"#
    )
}

/// Standard base64 decode (fixtures encode non-UTF-8 strings as `{base64}`).
pub fn unbase64(s: &str) -> Vec<u8> {
    let val = |c: u8| -> u32 {
        match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    };
    let clean: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let mut out = Vec::new();
    for chunk in clean.chunks(4) {
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        let n = chunk.iter().fold(0u32, |acc, &c| (acc << 6) | val(c)) << (6 * (4 - chunk.len() as u32));
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..3usize.saturating_sub(pad).min(chunk.len().saturating_sub(1))]);
    }
    out
}
