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
    pub fn parse(input: &str) -> Result<Self, CoreError> {
        todo!("lexical normalization per backend.md §2.4")
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
        todo!()
    }

    /// Append a validated child path.
    pub fn join(&self, child: &RelPath) -> RelPath {
        todo!()
    }

    /// `self` is `other` or lies below it (segment-wise, never string-prefix).
    pub fn starts_with(&self, other: &RelPath) -> bool {
        todo!()
    }

    /// Re-root `self` from `old` prefix onto `new` (used by rename/move remaps).
    pub fn rebase(&self, old: &RelPath, new: &RelPath) -> Option<RelPath> {
        todo!()
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
    todo!("backend.md §2.3")
}

/// **[COMPAT]** the lexical part of `PathSafety.contain(base, path)`
/// (backend.md §2.4 steps 1-3): `None` input → `None`; clean `base`; join
/// (`Pathname#join`: an absolute `path` *replaces* `base`); clean; accept only
/// `candidate == base` or `candidate` under `base + "/"` (separator-anchored, so
/// `/data/notes-backup` is outside `/data/notes`). Returns the cleaned absolute
/// candidate. The symlink/realpath half lives in `frankmd_fs::path_safety`.
pub fn lexical_contain(base: &str, path: Option<&str>) -> Option<String> {
    todo!("backend.md §2.4")
}

/// **[COMPAT]** `PathSafety.within?(path, base)`: `path == base` or `path`
/// starts with `base + "/"` (both taken as given, no cleaning).
pub fn is_within(path: &str, base: &str) -> bool {
    todo!("backend.md §2.4")
}

/// **[COMPAT]** `NotesController#reject_config_file`: does this raw request
/// path name the root `.fed` once lexically cleaned (`./.fed`, `foo/../.fed`,
/// `/.fed` …)? Note operations on it are refused (HTTP 403 in Rails).
/// `sub/.fed` is an ordinary (non-note) file and is **not** the config file.
pub fn is_config_file(raw: &str) -> bool {
    todo!("backend.md §2.9")
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
    todo!("backend.md §2.9")
}
