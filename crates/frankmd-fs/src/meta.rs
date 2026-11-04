//! File metadata the vault needs, read the same way on every OS.

use std::fs::Metadata;
use std::path::Path;

/// Modification time in whole seconds since the epoch (`File.mtime.to_i`).
#[cfg(unix)]
pub fn mtime_secs(meta: &Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    meta.mtime()
}

/// Modification time in whole seconds since the epoch (`File.mtime.to_i`).
#[cfg(not(unix))]
pub fn mtime_secs(meta: &Metadata) -> i64 {
    use std::time::UNIX_EPOCH;
    match meta.modified() {
        Ok(t) => match t.duration_since(UNIX_EPOCH) {
            Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
            Err(e) => -i64::try_from(e.duration().as_secs()).unwrap_or(i64::MAX),
        },
        Err(_) => 0,
    }
}

/// Identity of a file for cycle detection: `(dev, inode)` on Unix.
#[cfg(unix)]
pub type FileKey = (u64, u64);

/// Identity of a file for cycle detection: its canonical path off Unix.
#[cfg(not(unix))]
pub type FileKey = std::path::PathBuf;

/// The identity of `path`, whose metadata (symlinks followed) is `meta`.
#[cfg(unix)]
pub fn file_key(_path: &Path, meta: &Metadata) -> FileKey {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino())
}

/// The identity of `path`, whose metadata (symlinks followed) is `meta`.
#[cfg(not(unix))]
pub fn file_key(path: &Path, _meta: &Metadata) -> FileKey {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
