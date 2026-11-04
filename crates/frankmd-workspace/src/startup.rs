//! Explicit, typed startup phases (HP-016). The app advances these in order
//! and nothing reads state belonging to a later phase.

/// Startup phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Phase {
    /// CLI parsed, logging up, XDG dirs resolved.
    Boot,
    /// Vault opened, `.fed` loaded/created, locale + theme resolved.
    ConfigLoaded,
    /// First tree scan complete, DocId index loaded, watchers armed.
    IndexReady,
    /// Window + views mounted.
    UiMounted,
    /// Deep-linked / last note opened (drafts recovered).
    DocumentOpened,
}

/// What the app opens once the workspace is ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupTarget {
    /// Nothing requested: restore the last note or show the empty state.
    None,
    /// An existing note.
    Note(frankmd_core::RelPath),
    /// A requested note that does not exist (note-not-found flow, never created).
    Missing(frankmd_core::RelPath),
}

/// Resolve a CLI/IPC note argument against the vault.
///
/// Relative paths are vault-relative; absolute paths are accepted only inside
/// the vault (`vault_root` as given or its realpath). `.md` is appended like
/// `Note.normalize_path`. Paths escaping the vault and the `.fed` config file
/// (never edited in-app) resolve to [`StartupTarget::None`]. `exists` reports
/// whether a vault-relative note exists (a `frankmd-fs` lookup in the app).
pub fn resolve_initial_note(
    vault_roots: &[&std::path::Path],
    arg: Option<&str>,
    exists: impl Fn(&frankmd_core::RelPath) -> bool,
) -> StartupTarget {
    let Some(arg) = arg.map(str::trim).filter(|a| !a.is_empty()) else {
        return StartupTarget::None;
    };
    let raw = std::path::Path::new(arg);
    let relative = if raw.is_absolute() {
        let inside = vault_roots
            .iter()
            .find_map(|root| raw.strip_prefix(root).ok());
        match inside.and_then(std::path::Path::to_str) {
            Some(rel) => rel.to_owned(),
            None => return StartupTarget::None,
        }
    } else {
        arg.to_owned()
    };
    let normalized = frankmd_core::path::normalize_note_path(&relative);
    if frankmd_core::path::is_config_file(&normalized) {
        return StartupTarget::None;
    }
    match frankmd_core::RelPath::parse(&normalized) {
        Ok(path) if path.is_root() => StartupTarget::None,
        Ok(path) if exists(&path) => StartupTarget::Note(path),
        Ok(path) => StartupTarget::Missing(path),
        Err(_) => StartupTarget::None,
    }
}
