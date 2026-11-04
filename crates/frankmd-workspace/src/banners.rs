//! Editor-pane banners (ui-spec §5.12): a stack shown in a fixed priority
//! order regardless of raise order. Labels are i18n keys (D-3).

use std::collections::BTreeSet;

/// Banner kinds, declared in display priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BannerKind {
    /// danger: "Can't reach ~/notes. Edits are kept as drafts." \[Retry\]
    VaultUnreachable,
    /// info: read-only note or grammar check running.
    ReadOnly,
    /// warning: \[Reload, Compare…\]
    ChangedOnDisk,
    /// warning: \[Save again, Close\]
    DeletedOnDisk,
    /// warning: "Saving would remove N characters (P %)." \[Undo, Save anyway\]
    ContentLoss,
    /// danger: \[Details\]
    DraftStorageFailed,
    /// info: \[Settings, Dismiss\]
    StorageFallback,
}

/// Colour role of a banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerRole {
    /// `danger`.
    Danger,
    /// `warning`.
    Warning,
    /// `info`.
    Info,
}

impl BannerKind {
    /// Kebab-case name used in element ids (`banner-<kind>`).
    pub fn slug(self) -> &'static str {
        match self {
            Self::VaultUnreachable => "vault-unreachable",
            Self::ReadOnly => "read-only",
            Self::ChangedOnDisk => "changed-on-disk",
            Self::DeletedOnDisk => "deleted-on-disk",
            Self::ContentLoss => "content-loss",
            Self::DraftStorageFailed => "draft-storage-failed",
            Self::StorageFallback => "storage-fallback",
        }
    }

    /// Element id of the banner.
    pub fn element_id(self) -> String {
        format!("banner-{}", self.slug())
    }

    /// Element id of one of its buttons (`banner-<kind>-<button>`), where
    /// `button` is the last segment of the button's i18n key, kebab-cased.
    pub fn button_id(self, button_key: &str) -> String {
        let name = button_key.rsplit('.').next().unwrap_or(button_key).replace('_', "-");
        format!("banner-{}-{name}", self.slug())
    }

    /// Colour role.
    pub fn role(self) -> BannerRole {
        match self {
            Self::VaultUnreachable | Self::DraftStorageFailed => BannerRole::Danger,
            Self::ChangedOnDisk | Self::DeletedOnDisk | Self::ContentLoss => BannerRole::Warning,
            Self::ReadOnly | Self::StorageFallback => BannerRole::Info,
        }
    }

    /// Sentence i18n key.
    pub fn message_key(self) -> &'static str {
        match self {
            Self::VaultUnreachable => "banners.offline.message",
            Self::ReadOnly => "banners.read_only.message",
            Self::ChangedOnDisk => "banners.changed_on_disk.message",
            Self::DeletedOnDisk => "banners.deleted_on_disk.message",
            Self::ContentLoss => "banners.content_loss.message",
            Self::DraftStorageFailed => "banners.draft_storage.message",
            Self::StorageFallback => "banners.storage_fallback.message",
        }
    }
}

/// Banner stack of the editor pane.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BannerStack {
    raised: BTreeSet<BannerKind>,
    dismissed_for_session: BTreeSet<BannerKind>,
}

impl BannerStack {
    /// Raise (idempotent). A storage-fallback banner the user dismissed stays
    /// dismissed for the session.
    pub fn raise(&mut self, kind: BannerKind) {
        if !self.dismissed_for_session.contains(&kind) {
            self.raised.insert(kind);
        }
    }

    /// Clear one.
    pub fn clear(&mut self, kind: BannerKind) {
        self.raised.remove(&kind);
    }

    /// Raise or clear.
    pub fn set(&mut self, kind: BannerKind, raised: bool) {
        if raised { self.raise(kind) } else { self.clear(kind) }
    }

    /// The user dismissed `kind`; [`BannerKind::StorageFallback`] then never
    /// comes back this session.
    pub fn dismiss(&mut self, kind: BannerKind) {
        self.clear(kind);
        if kind == BannerKind::StorageFallback {
            self.dismissed_for_session.insert(kind);
        }
    }

    /// Whether `kind` is shown.
    pub fn is_raised(&self, kind: BannerKind) -> bool {
        self.raised.contains(&kind)
    }

    /// Visible banners in priority order.
    pub fn visible(&self) -> Vec<BannerKind> {
        self.raised.iter().copied().collect()
    }

    /// Button labels (i18n keys) for a banner, in display order.
    pub fn buttons(kind: BannerKind) -> Vec<&'static str> {
        match kind {
            BannerKind::VaultUnreachable => vec!["banners.offline.retry"],
            BannerKind::ReadOnly => vec![],
            BannerKind::ChangedOnDisk => vec!["banners.changed_on_disk.reload", "banners.changed_on_disk.compare"],
            BannerKind::DeletedOnDisk => vec!["banners.deleted_on_disk.save_again", "banners.deleted_on_disk.close"],
            BannerKind::ContentLoss => vec!["banners.content_loss.undo", "banners.content_loss.save_anyway"],
            BannerKind::DraftStorageFailed => vec!["banners.draft_storage.details"],
            BannerKind::StorageFallback => vec!["banners.storage_fallback.settings", "banners.storage_fallback.dismiss"],
        }
    }
}
