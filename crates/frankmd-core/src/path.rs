//! Vault-relative paths.
//!
//! [`RelPath`] is a *lexically* validated, `/`-separated path relative to the
//! notes root. Symlink containment needs the filesystem and is enforced by
//! `frankmd_fs::Vault::contain`, which is the only way to turn a `RelPath`
//! into something that can be opened.
//!
//! The free functions below are the pure halves of the Rails path rules
//! (`PathSafety`, `Note.normalize_path`, `reject_config_file`), kept with the
//! original semantics so the golden fixtures stay the spec.

use crate::CoreError;
use serde::{Deserialize, Serialize};

/// **[COMPAT]** the config file at the vault root (`Note::SPECIAL_FILES`).
pub const CONFIG_FILE: &str = ".fed";

/// **[COMPAT]** the note extension; compared case-sensitively (`.MD` is not a note).
pub const NOTE_EXT: &str = ".md";

/// A validated vault-relative path (`"blog/2026/post.md"`, `""` = root).
///
/// Invariants: no leading `/`, no NUL, no empty or `.` segments, and `..`
/// never escapes the root (it is resolved lexically on construction).
/// Legit names such as `notes..v2.md` are accepted (the Rails `..` substring
/// rule is a [QUIRK-DROP]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelPath(String);

impl RelPath {
    /// The vault root.
    pub fn root() -> Self {
        Self(String::new())
    }

    /// Validate and lexically normalize user- or disk-supplied input.
    ///
    /// Same lexical rules as `PathSafety.contain` (backend.md §2.4 steps 1-3):
    /// `.`/`//` collapse, `..` resolves against earlier segments, an absolute
    /// path or a `..` that climbs above the root is [`CoreError::InvalidPath`],
    /// and NUL is rejected. `""` and `"."` give the root.
    ///
    /// On Windows a `\` can only be a separator (`\` is reserved in names),
    /// so `\`-separated input (e.g. stripped from a `\\?\`-prefixed absolute
    /// path) is accepted and stored `/`-separated like everywhere else.
    pub fn parse(input: &str) -> Result<Self, CoreError> {
        let invalid = || CoreError::InvalidPath(input.to_owned());
        #[cfg(target_os = "windows")]
        let owned = input.replace('\\', "/");
        #[cfg(target_os = "windows")]
        let input = owned.as_str();
        if input.contains('\0') || input.starts_with('/') {
            return Err(invalid());
        }
        let mut segments: Vec<&str> = Vec::new();
        for segment in input.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    segments.pop().ok_or_else(invalid)?;
                }
                name => segments.push(name),
            }
        }
        Ok(Self(segments.join("/")))
    }

    /// The path as a `/`-separated string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `true` for the vault root (`""`).
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Final segment, if any.
    pub fn file_name(&self) -> Option<&str> {
        self.0.rsplit('/').next().filter(|s| !s.is_empty())
    }

    /// Parent directory (`None` for the root).
    pub fn parent(&self) -> Option<RelPath> {
        if self.is_root() {
            return None;
        }
        Some(match self.0.rfind('/') {
            Some(i) => Self(self.0[..i].to_owned()),
            None => Self::root(),
        })
    }

    /// Append a validated child path.
    pub fn join(&self, child: &RelPath) -> RelPath {
        if self.is_root() {
            child.clone()
        } else if child.is_root() {
            self.clone()
        } else {
            Self(format!("{}/{}", self.0, child.0))
        }
    }

    /// `self` is `other` or lies below it (segment-wise, never string-prefix).
    pub fn starts_with(&self, other: &RelPath) -> bool {
        other.is_root() || is_within(&self.0, &other.0)
    }

    /// Re-root `self` from `old` prefix onto `new` (used by rename/move remaps).
    pub fn rebase(&self, old: &RelPath, new: &RelPath) -> Option<RelPath> {
        if !self.starts_with(old) {
            return None;
        }
        let rest = self.0[old.0.len()..].trim_start_matches('/');
        if rest.is_empty() {
            return Some(new.clone());
        }
        Some(new.join(&Self(rest.to_owned())))
    }
}

impl std::fmt::Display for RelPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// **[COMPAT]** `Note.normalize_path`: blank → `""`; a path whose basename is a
/// special file (`.fed`) is returned unchanged; otherwise `.md` is appended
/// unless the path already ends with `.md` (case-sensitive), so `foo.txt` →
/// `foo.txt.md`.
pub fn normalize_note_path(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }
    if ruby_basename(input) == CONFIG_FILE || input.ends_with(NOTE_EXT) {
        return input.to_owned();
    }
    format!("{input}{NOTE_EXT}")
}

/// Ruby `File.basename(path)`: the last non-empty segment (`"a/b/"` → `"b"`,
/// `"/"` → `"/"`).
fn ruby_basename(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return if path.is_empty() { "" } else { "/" };
    }
    trimmed.rsplit('/').next().unwrap_or(trimmed)
}

/// Ruby `Pathname#cleanpath` (the aggressive, non-conservative form): `.`
/// and repeated `/` removed, `..` resolved against the previous segment,
/// `..` above `/` dropped, a leading relative `..` kept; empty → `"."`.
pub fn clean_path(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => match segments.last() {
                Some(&last) if last != ".." => {
                    segments.pop();
                }
                _ if absolute => {}
                _ => segments.push(".."),
            },
            name => segments.push(name),
        }
    }
    let joined = segments.join("/");
    if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_owned()
    } else {
        joined
    }
}

/// **[COMPAT]** the lexical part of `PathSafety.contain(base, path)`
/// (backend.md §2.4 steps 1-3): `None` input → `None`; clean `base`; join
/// (`Pathname#join`: an absolute `path` *replaces* `base`); clean; accept only
/// `candidate == base` or `candidate` under `base + "/"` (separator-anchored, so
/// `/data/notes-backup` is outside `/data/notes`). Returns the cleaned absolute
/// candidate. The symlink/realpath half lives in `frankmd_fs::path_safety`.
///
/// On Windows the candidate is returned with `\` separators: `base` usually
/// comes from `canonicalize()` (a verbatim `\\?\` path), and verbatim paths
/// reject `/` outright (`/` is reserved), so a `/`-joined candidate would fail
/// every filesystem call with `ERROR_INVALID_NAME`.
pub fn lexical_contain(base: &str, path: Option<&str>) -> Option<String> {
    let path = path?;
    if path.contains('\0') || base.contains('\0') {
        return None;
    }
    #[cfg(target_os = "windows")]
    let path = path.replace('\\', "/");
    let base = clean_path(base);
    #[cfg(target_os = "windows")]
    let base = base.replace('/', "\\");
    let joined = if path.starts_with('/') {
        path.to_owned()
    } else if path.is_empty() {
        base.clone()
    } else {
        format!("{base}/{path}")
    };
    let candidate = clean_path(&joined);
    #[cfg(target_os = "windows")]
    let candidate = candidate.replace('/', "\\");
    is_within(&candidate, &base).then_some(candidate)
}

/// **[COMPAT]** `PathSafety.within?(path, base)`: `path == base` or `path`
/// starts with `base` + the platform separator (both taken as given, no
/// cleaning).
pub fn is_within(path: &str, base: &str) -> bool {
    path == base
        || path
            .strip_prefix(base)
            .is_some_and(|rest| rest.starts_with(SEPARATOR))
}

/// [`is_within`]'s separator anchor: `/` as in Ruby, `\` on Windows where
/// [`lexical_contain`] normalizes candidates to platform separators.
#[cfg(target_os = "windows")]
const SEPARATOR: char = '\\';
#[cfg(not(target_os = "windows"))]
const SEPARATOR: char = '/';

/// **[COMPAT]** `NotesController#reject_config_file`: does this raw request
/// path name the root `.fed` once lexically cleaned (`./.fed`, `foo/../.fed`,
/// `/.fed` …)? Note operations on it are refused (HTTP 403 in Rails).
/// `sub/.fed` is an ordinary (non-note) file and is **not** the config file.
pub fn is_config_file(raw: &str) -> bool {
    // Any absolute base works: the check is lexical and relative to it.
    const BASE: &str = "/notes-root";
    // The expected value goes through `lexical_contain` too, so both sides
    // carry the same platform separators.
    let expected = lexical_contain(BASE, Some(CONFIG_FILE));
    lexical_contain(BASE, Some(raw))
        .zip(expected)
        .is_some_and(|(path, want)| path == want)
}

/// How the Rails `GET /notes/*path` route classified a raw path
/// (backend.md §2.9): a note when it ends with `.md` **or contains no `.`
/// at all**, otherwise a raw asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    /// Treated as a markdown note (`.md` is appended by [`normalize_note_path`]).
    Note,
    /// Served as a raw file (images, videos, …).
    Asset,
}

/// **[COMPAT]** classify a raw request path (see [`PathKind`]).
pub fn classify(raw: &str) -> PathKind {
    if raw.ends_with(NOTE_EXT) || !raw.contains('.') {
        PathKind::Note
    } else {
        PathKind::Asset
    }
}
