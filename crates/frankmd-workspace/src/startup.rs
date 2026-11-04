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
