//! Atomic replace and no-clobber create primitives.
//!
//! Every helper keeps its temporary file in the target's own directory so
//! the final `rename(2)`/`link(2)` never crosses filesystems, and removes it
//! on every failure path.

use crate::FsError;
use crate::error::libc_errno::{
    EEXIST, EINVAL, EISDIR, ENOSYS, ENOTEMPTY, EOPNOTSUPP, EPERM, EXDEV,
};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// What permission bits the replaced file gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModePolicy {
    /// Keep the existing file's `mode & 0o777` (notes, **[COMPAT]**).
    Preserve,
    /// Force a mode (`.fed` is always `0o600`, **[COMPAT]**).
    Fixed(u32),
}

/// Atomically replace `target` with `bytes` (temp in same dir + fsync + rename).
///
/// A failed write never truncates the existing file: it stays intact until
/// the rename succeeds. A directory at `target` is refused (`EISDIR`) before
/// any temp file is created.
pub fn atomic_replace(target: &Path, bytes: &[u8], mode: ModePolicy) -> Result<(), FsError> {
    if fs::symlink_metadata(target).is_ok_and(|m| m.is_dir()) {
        return Err(io::Error::from_raw_os_error(EISDIR).into());
    }
    let (dir, tmp) = temp_path_for(target)?;
    let result = (|| -> io::Result<()> {
        write_new_file(&tmp, bytes)?;
        let bits = match mode {
            ModePolicy::Preserve => fs::metadata(target)
                .ok()
                .map(|m| m.permissions().mode() & 0o777),
            ModePolicy::Fixed(bits) => Some(bits),
        };
        if let Some(bits) = bits {
            fs::set_permissions(&tmp, fs::Permissions::from_mode(bits))?;
        }
        crate::fault_check!(crate::fault::FaultPoint::AtomicRename)?;
        fs::rename(&tmp, target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result?;
    sync_dir(&dir);
    Ok(())
}

/// Create `target` with `bytes`, failing with `AlreadyExists` if anything
/// (including a dangling symlink) is already there.
///
/// The content is written to an `O_EXCL` temp file first and then published
/// with `link(2)`, which fails atomically when the destination exists. On
/// filesystems without hard links it falls back to
/// `renameat2(RENAME_NOREPLACE)`.
pub fn create_new(target: &Path, bytes: &[u8]) -> Result<(), FsError> {
    let already = || FsError::AlreadyExists(target.display().to_string());
    if fs::symlink_metadata(target).is_ok() {
        return Err(already());
    }
    let (dir, tmp) = temp_path_for(target)?;
    let result = (|| -> Result<(), FsError> {
        write_new_file(&tmp, bytes)?;
        let linked = crate::fault_check!(crate::fault::FaultPoint::HardLink)
            .and_then(|()| fs::hard_link(&tmp, target));
        match linked {
            Ok(()) => Ok(()),
            Err(e) if e.raw_os_error() == Some(EEXIST) => Err(already()),
            Err(e) if matches!(e.raw_os_error(), Some(EPERM | EOPNOTSUPP | EXDEV)) => {
                rename_noreplace(&tmp, target).map_err(|e| map_exists(e, target))
            }
            Err(e) => Err(e.into()),
        }
    })();
    let _ = fs::remove_file(&tmp);
    result?;
    sync_dir(&dir);
    Ok(())
}

/// Rename that never replaces an existing destination
/// (`renameat2(RENAME_NOREPLACE)`, EXDEV → copy + delete).
///
/// An occupied destination is [`FsError::AlreadyExists`]; moving a folder
/// into itself is `EINVAL` (as `rename(2)` reports it).
pub fn rename_noclobber(from: &Path, to: &Path) -> Result<(), FsError> {
    let renamed = crate::fault_check!(crate::fault::FaultPoint::Rename)
        .and_then(|()| rename_noreplace(from, to));
    match renamed {
        Ok(()) => {}
        Err(e) if e.raw_os_error() == Some(EXDEV) => {
            copy_tree_noclobber(from, to).map_err(|e| map_exists(e, to))?;
            remove_tree(from)?;
        }
        Err(e) => return Err(map_exists(e, to)),
    }
    if let Some(parent) = to.parent() {
        sync_dir(parent);
    }
    Ok(())
}

/// **[COMPAT]** temp file name for `basename`: `.<basename>.<12 hex>.tmp`.
pub fn temp_name(basename: &str) -> String {
    format!(".{basename}.{}.tmp", random_hex(6))
}

/// `2 * n` lowercase hex chars of OS randomness (`SecureRandom.hex(n)`).
fn random_hex(n: usize) -> String {
    let mut buf = vec![0u8; n];
    let mut filled = 0;
    while filled < n {
        match rustix::rand::getrandom(&mut buf[filled..], rustix::rand::GetRandomFlags::empty()) {
            Ok(0) | Err(_) => break,
            Ok(read) => filled += read,
        }
    }
    if filled < n {
        // getrandom(2) is unavailable: fall back to a time/pid-derived value.
        // Uniqueness (not secrecy) is what matters: the temp is `O_EXCL`.
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
            ^ u128::from(std::process::id());
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = (seed >> ((i % 16) * 8)) as u8;
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// The directory of `target` and a fresh temp path inside it.
fn temp_path_for(target: &Path) -> io::Result<(PathBuf, PathBuf)> {
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::from_raw_os_error(EISDIR))?;
    let dir = match target.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let tmp = dir.join(temp_name(&name.to_string_lossy()));
    Ok((dir, tmp))
}

/// Write `bytes` to a new (`O_EXCL`) file and flush it to disk.
fn write_new_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// `renameat2(RENAME_NOREPLACE)`, with a check-then-rename fallback on
/// filesystems that do not support the flag (callers hold the vault lock).
fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    match renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE) {
        Ok(()) => Ok(()),
        Err(e) => {
            let err = io::Error::from(e);
            let unsupported = match err.raw_os_error() {
                Some(ENOSYS | EOPNOTSUPP) => true,
                // EINVAL is also "move a folder into itself"; only retry otherwise.
                Some(EINVAL) => !to.starts_with(from),
                _ => false,
            };
            if !unsupported {
                return Err(err);
            }
            if fs::symlink_metadata(to).is_ok() {
                return Err(io::Error::from_raw_os_error(EEXIST));
            }
            fs::rename(from, to)
        }
    }
}

/// Map "destination occupied" OS errors to [`FsError::AlreadyExists`].
fn map_exists(err: impl Into<FsError>, target: &Path) -> FsError {
    match err.into() {
        FsError::Io(e) if matches!(e.raw_os_error(), Some(EEXIST | ENOTEMPTY | EISDIR)) => {
            FsError::AlreadyExists(target.display().to_string())
        }
        other => other,
    }
}

/// Cross-device move, first half: copy `from` to `to` without replacing
/// anything (files, directories and symlinks; permission bits kept).
fn copy_tree_noclobber(from: &Path, to: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.file_type().is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(from)?, to)
    } else if meta.is_dir() {
        fs::create_dir(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree_noclobber(&entry.path(), &to.join(entry.file_name()))?;
        }
        fs::set_permissions(to, meta.permissions())
    } else {
        let mut src = File::open(from)?;
        let mut dst = OpenOptions::new().write(true).create_new(true).open(to)?;
        io::copy(&mut src, &mut dst)?;
        dst.set_permissions(meta.permissions())?;
        dst.sync_all()
    }
}

/// Cross-device move, second half: remove the source.
fn remove_tree(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Best-effort `fsync` of a directory so a completed rename is durable.
fn sync_dir(dir: &Path) {
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
}
