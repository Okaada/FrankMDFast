//! **[COMPAT]** port of `app/services/path_safety.rb` (backend.md §2.4).
//! Golden: `path_safety.json` (`PathSafety.contain`, `PathSafety.within?`).

use frankmd_core::path::{is_within, lexical_contain};
use std::path::{Path, PathBuf};

/// `PathSafety.contain(base, path)`: the lexical check of
/// [`frankmd_core::path::lexical_contain`], then — only when `base` exists —
/// walk up from the candidate to the nearest existing ancestor (`exist?`
/// follows symlinks, so a dangling link counts as missing) and require
/// `realpath(ancestor)` to be within `realpath(base)`. Returns the cleaned
/// **lexical** candidate (not the realpath), or `None` when rejected.
/// Never creates anything.
pub fn contain(base: &Path, path: Option<&str>) -> Option<PathBuf> {
    let base_str = base.to_str()?;
    let candidate = PathBuf::from(lexical_contain(base_str, path)?);
    if !base.exists() {
        return Some(candidate);
    }
    let real_base = base.canonicalize().ok()?;
    let real_ancestor = nearest_existing_ancestor(&candidate).canonicalize().ok()?;
    let (ancestor, base) = (real_ancestor.to_str()?, real_base.to_str()?);
    is_within(ancestor, base).then_some(candidate)
}

/// The candidate itself or its closest parent that exists (symlinks
/// followed); `"."` for a relative path with no existing component.
fn nearest_existing_ancestor(candidate: &Path) -> &Path {
    let mut ancestor = candidate;
    while !ancestor.exists() {
        match ancestor.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => ancestor = parent,
            _ => return Path::new("."),
        }
    }
    ancestor
}
