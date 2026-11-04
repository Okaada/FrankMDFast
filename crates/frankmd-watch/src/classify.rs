//! Pure classification of raw vault event paths.

use frankmd_core::RelPath;
use frankmd_core::path::CONFIG_FILE;
use std::path::{Component, Path};

/// **[COMPAT]** lock file `frankmd-fs` takes around `.fed` writes.
pub const CONFIG_LOCK_FILE: &str = ".frankmd-config.lock";
/// **[COMPAT]** lock file `frankmd-fs` takes around vault mutations.
pub const FILESYSTEM_LOCK_FILE: &str = ".frankmd-filesystem.lock";

/// What a raw event path means for the vault watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathClass {
    /// Not interesting: outside the vault, the root itself, a dotfile or
    /// dot-dir anywhere in the path, a FrankMD temp/lock file, or a name
    /// that is not valid UTF-8.
    Ignore,
    /// The root `.fed` config file.
    Config,
    /// A visible vault entry (note, folder, image, …).
    Vault(RelPath),
}

/// Classify absolute event path `path` against the vault root(s).
///
/// `roots` lists equivalent spellings of the vault root (e.g. as configured
/// and canonicalized); the first one that is a prefix of `path` is used.
/// Only the root-level `.fed` is [`PathClass::Config`]; `.fed` in a
/// subfolder is an ordinary dotfile and ignored.
pub fn classify(roots: &[&Path], path: &Path) -> PathClass {
    let Some(rel) = roots.iter().find_map(|root| path.strip_prefix(root).ok()) else {
        return PathClass::Ignore;
    };
    let mut segments: Vec<&str> = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(name) => match name.to_str() {
                Some(name) => segments.push(name),
                None => return PathClass::Ignore,
            },
            _ => return PathClass::Ignore,
        }
    }
    match segments.as_slice() {
        [] => PathClass::Ignore,
        [name] if *name == CONFIG_FILE => PathClass::Config,
        [name] if *name == CONFIG_LOCK_FILE || *name == FILESYSTEM_LOCK_FILE => PathClass::Ignore,
        // Dotfiles cover FrankMD temp files (`.<name>.<12hex>.tmp`) too.
        _ if segments.iter().any(|s| s.starts_with('.')) => PathClass::Ignore,
        _ => match RelPath::parse(&segments.join("/")) {
            Ok(rel) => PathClass::Vault(rel),
            Err(_) => PathClass::Ignore,
        },
    }
}

/// `true` when `rel` (relative to the Omarchy `current/` dir) touches the
/// theme: the `theme` entry itself, anything under it, or a sibling such as
/// `theme.name`. `background` and other entries are not theme changes.
pub fn is_omarchy_theme_entry(rel: &Path) -> bool {
    match rel.components().next() {
        Some(Component::Normal(first)) => first
            .to_str()
            .is_some_and(|name| name == "theme" || name.starts_with("theme.")),
        _ => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn vault(path: &str) -> PathClass {
        classify(&[Path::new("/v")], &Path::new("/v").join(path))
    }

    #[test]
    fn visible_paths_become_rel_paths() {
        assert_eq!(
            vault("a.md"),
            PathClass::Vault(RelPath::parse("a.md").unwrap())
        );
        assert_eq!(
            vault("blog/2026/post.md"),
            PathClass::Vault(RelPath::parse("blog/2026/post.md").unwrap())
        );
        assert_eq!(
            vault("notes..v2.md"),
            PathClass::Vault(RelPath::parse("notes..v2.md").unwrap())
        );
    }

    #[test]
    fn root_fed_is_config_but_nested_fed_is_ignored() {
        assert_eq!(vault(".fed"), PathClass::Config);
        assert_eq!(vault("sub/.fed"), PathClass::Ignore);
    }

    #[test]
    fn dotfiles_temp_and_lock_files_are_ignored() {
        assert_eq!(vault(".hidden.md"), PathClass::Ignore);
        assert_eq!(vault(".git/config"), PathClass::Ignore);
        assert_eq!(vault("a/.obsidian/x.md"), PathClass::Ignore);
        assert_eq!(vault(".post.md.0123456789ab.tmp"), PathClass::Ignore);
        assert_eq!(vault("blog/.post.md.0123456789ab.tmp"), PathClass::Ignore);
        assert_eq!(vault(CONFIG_LOCK_FILE), PathClass::Ignore);
        assert_eq!(vault(FILESYSTEM_LOCK_FILE), PathClass::Ignore);
    }

    #[test]
    fn root_and_outside_paths_are_ignored() {
        assert_eq!(
            classify(&[Path::new("/v")], Path::new("/v")),
            PathClass::Ignore
        );
        assert_eq!(
            classify(&[Path::new("/v")], Path::new("/other/a.md")),
            PathClass::Ignore
        );
        assert_eq!(
            classify(&[Path::new("/v")], Path::new("/vault/a.md")),
            PathClass::Ignore
        );
    }

    #[test]
    fn any_listed_root_spelling_matches() {
        let roots = [Path::new("/link/v"), Path::new("/real/v")];
        assert_eq!(
            classify(&roots, Path::new("/real/v/a.md")),
            PathClass::Vault(RelPath::parse("a.md").unwrap())
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_names_are_ignored() {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::OsStr::from_bytes(b"bad\xff.md");
        assert_eq!(
            classify(&[Path::new("/v")], &Path::new("/v").join(name)),
            PathClass::Ignore
        );
    }

    #[test]
    fn omarchy_theme_entries() {
        assert!(is_omarchy_theme_entry(Path::new("theme")));
        assert!(is_omarchy_theme_entry(Path::new("theme/colors.toml")));
        assert!(is_omarchy_theme_entry(Path::new("theme.name")));
        assert!(!is_omarchy_theme_entry(Path::new("background")));
        assert!(!is_omarchy_theme_entry(Path::new("themes")));
        assert!(!is_omarchy_theme_entry(Path::new("")));
    }
}
