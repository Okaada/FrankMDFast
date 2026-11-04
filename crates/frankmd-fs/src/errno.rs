//! The POSIX `errno` values the compat layer distinguishes, on every OS.
//!
//! On Unix the numbers come from the platform (Linux and macOS differ for
//! several of them). On Windows, Win32 error codes are translated to the
//! same names, and errors this crate raises itself carry their POSIX code
//! as the error payload, so [`of`] reads the same on every platform.

use std::io;

#[cfg(unix)]
#[allow(missing_docs)] // the names are the documentation
mod codes {
    use rustix::io::Errno;
    pub const EPERM: i32 = Errno::PERM.raw_os_error();
    pub const ENOENT: i32 = Errno::NOENT.raw_os_error();
    pub const EIO: i32 = Errno::IO.raw_os_error();
    pub const EACCES: i32 = Errno::ACCESS.raw_os_error();
    pub const EBUSY: i32 = Errno::BUSY.raw_os_error();
    pub const EEXIST: i32 = Errno::EXIST.raw_os_error();
    pub const EXDEV: i32 = Errno::XDEV.raw_os_error();
    pub const ENOTDIR: i32 = Errno::NOTDIR.raw_os_error();
    pub const EISDIR: i32 = Errno::ISDIR.raw_os_error();
    pub const EINVAL: i32 = Errno::INVAL.raw_os_error();
    pub const ENOSPC: i32 = Errno::NOSPC.raw_os_error();
    pub const EROFS: i32 = Errno::ROFS.raw_os_error();
    pub const EMLINK: i32 = Errno::MLINK.raw_os_error();
    pub const ENAMETOOLONG: i32 = Errno::NAMETOOLONG.raw_os_error();
    pub const ENOSYS: i32 = Errno::NOSYS.raw_os_error();
    pub const ENOTEMPTY: i32 = Errno::NOTEMPTY.raw_os_error();
    pub const ELOOP: i32 = Errno::LOOP.raw_os_error();
    pub const EOPNOTSUPP: i32 = Errno::OPNOTSUPP.raw_os_error();
}

/// Off Unix there is no errno: the Linux numbers serve as stable names.
#[cfg(not(unix))]
#[allow(missing_docs)]
mod codes {
    pub const EPERM: i32 = 1;
    pub const ENOENT: i32 = 2;
    pub const EIO: i32 = 5;
    pub const EACCES: i32 = 13;
    pub const EBUSY: i32 = 16;
    pub const EEXIST: i32 = 17;
    pub const EXDEV: i32 = 18;
    pub const ENOTDIR: i32 = 20;
    pub const EISDIR: i32 = 21;
    pub const EINVAL: i32 = 22;
    pub const ENOSPC: i32 = 28;
    pub const EROFS: i32 = 30;
    pub const EMLINK: i32 = 31;
    pub const ENAMETOOLONG: i32 = 36;
    pub const ENOSYS: i32 = 38;
    pub const ENOTEMPTY: i32 = 39;
    pub const ELOOP: i32 = 40;
    pub const EOPNOTSUPP: i32 = 95;
}

pub use codes::*;

/// The POSIX errno behind `err`, if it is an OS error.
#[cfg(unix)]
pub fn of(err: &io::Error) -> Option<i32> {
    err.raw_os_error()
}

/// An OS error with POSIX errno `code`.
#[cfg(unix)]
pub fn error(code: i32) -> io::Error {
    io::Error::from_raw_os_error(code)
}

/// The POSIX errno behind `err`, if it is an OS error.
#[cfg(not(unix))]
pub fn of(err: &io::Error) -> Option<i32> {
    if let Some(posix) = err.get_ref().and_then(|e| e.downcast_ref::<Posix>()) {
        return Some(posix.0);
    }
    err.raw_os_error().map(from_win32)
}

/// An error carrying POSIX errno `code` (read back by [`of`]).
#[cfg(not(unix))]
pub fn error(code: i32) -> io::Error {
    let kind = match code {
        ENOENT => io::ErrorKind::NotFound,
        EACCES | EPERM => io::ErrorKind::PermissionDenied,
        EEXIST => io::ErrorKind::AlreadyExists,
        EISDIR => io::ErrorKind::IsADirectory,
        ENOTDIR => io::ErrorKind::NotADirectory,
        ENOTEMPTY => io::ErrorKind::DirectoryNotEmpty,
        EXDEV => io::ErrorKind::CrossesDevices,
        EINVAL => io::ErrorKind::InvalidInput,
        EOPNOTSUPP | ENOSYS => io::ErrorKind::Unsupported,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, Posix(code))
}

#[cfg(not(unix))]
#[derive(Debug)]
struct Posix(i32);

#[cfg(not(unix))]
impl std::fmt::Display for Posix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self.0 {
            EEXIST => "File exists",
            EISDIR => "Is a directory",
            ENOTEMPTY => "Directory not empty",
            ELOOP => "Too many levels of symbolic links",
            EINVAL => "Invalid argument",
            _ => "Operating system error",
        };
        f.write_str(text)
    }
}

#[cfg(not(unix))]
impl std::error::Error for Posix {}

/// Win32 error code → the closest POSIX errno (unknown codes pass through
/// offset by 10000 so they never collide with a POSIX value).
#[cfg(not(unix))]
fn from_win32(code: i32) -> i32 {
    match code {
        1 | 50 => EOPNOTSUPP,         // ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED
        2 | 3 | 123 => ENOENT,        // FILE/PATH_NOT_FOUND, INVALID_NAME
        5 => EACCES,                  // ERROR_ACCESS_DENIED
        17 => EXDEV,                  // ERROR_NOT_SAME_DEVICE
        19 => EROFS,                  // ERROR_WRITE_PROTECT
        32 | 33 => EBUSY,             // SHARING / LOCK_VIOLATION
        80 | 183 => EEXIST,           // FILE_EXISTS, ALREADY_EXISTS
        87 => EINVAL,                 // ERROR_INVALID_PARAMETER
        112 => ENOSPC,                // ERROR_DISK_FULL
        145 => ENOTEMPTY,             // ERROR_DIR_NOT_EMPTY
        206 => ENAMETOOLONG,          // ERROR_FILENAME_EXCED_RANGE
        267 => ENOTDIR,               // ERROR_DIRECTORY
        1142 => EMLINK,               // ERROR_TOO_MANY_LINKS
        1314 => EPERM,                // ERROR_PRIVILEGE_NOT_HELD
        1921 => ELOOP,                // ERROR_CANT_RESOLVE_FILENAME
        other => 10_000 + other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_round_trip_their_code() {
        for code in [EEXIST, EISDIR, ENOTEMPTY, EXDEV, ELOOP] {
            assert_eq!(of(&error(code)), Some(code));
        }
    }
}
