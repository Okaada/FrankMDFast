//! **[COMPAT]** port of `app/services/path_safety.rb` (backend.md §2.4).
//! Golden: `path_safety.json` (`PathSafety.contain`, `PathSafety.within?`).

use std::path::{Path, PathBuf};

/// `PathSafety.contain(base, path)`: the lexical check of
/// [`frankmd_core::path::lexical_contain`], then — only when `base` exists —
/// walk up from the candidate to the nearest existing ancestor (`exist?`
/// follows symlinks, so a dangling link counts as missing) and require
/// `realpath(ancestor)` to be within `realpath(base)`. Returns the cleaned
/// **lexical** candidate (not the realpath), or `None` when rejected.
/// Never creates anything.
pub fn contain(base: &Path, path: Option<&str>) -> Option<PathBuf> {
    todo!("backend.md §2.4")
}
