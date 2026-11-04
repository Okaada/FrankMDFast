//! Overlays (ui-spec §2, §5): one at a time, focus trapped, and focus
//! returns to the region the user was in before the *first* overlay opened.

use crate::focus::FocusRegion;

/// The one overlay that can be open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayKind {
    /// Ctrl+Shift+P, with sub-mode.
    Palette(PaletteMode),
    /// Ctrl+P.
    QuickOpen,
    /// Ctrl+Shift+F.
    SearchInFiles,
    /// Ctrl+,.
    Settings,
    /// F1 / Ctrl+/.
    Help,
    /// A modal dialog, by its element id suffix (`dialog-<kind>`).
    Dialog(&'static str),
}

impl OverlayKind {
    /// Esc closes it. Only the Recovery dialog refuses: the user must choose.
    pub fn closes_on_escape(&self) -> bool {
        !matches!(self, Self::Dialog("recovery"))
    }
}

/// Palette sub-modes (§5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteMode {
    /// `>` prompt.
    Commands,
    /// `:` prompt.
    GoToLine,
    /// Theme picker.
    Theme,
    /// Language picker.
    Language,
    /// Vault switcher.
    Vault,
}

impl PaletteMode {
    /// Mode selected by the first character typed in the palette input.
    pub fn from_prefix(input: &str) -> Option<Self> {
        match input.chars().next() {
            Some('>') => Some(Self::Commands),
            Some(':') => Some(Self::GoToLine),
            _ => None,
        }
    }
}

/// One-at-a-time overlay with focus restore.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayStack {
    /// Open overlay and the region to give focus back to.
    pub open: Option<(OverlayKind, FocusRegion)>,
}

impl OverlayStack {
    /// Open `kind`, replacing any open overlay (keeps the original return focus).
    pub fn open(&mut self, kind: OverlayKind, return_to: FocusRegion) {
        let return_to = self.open.take().map_or(return_to, |(_, original)| original);
        self.open = Some((kind, return_to));
    }

    /// Close; returns where focus goes back.
    pub fn close(&mut self) -> Option<FocusRegion> {
        self.open.take().map(|(_, region)| region)
    }

    /// Esc: closes the overlay unless it refuses ([`OverlayKind::closes_on_escape`]).
    /// `None` when nothing closed (the Esc goes further down the chain).
    pub fn escape(&mut self) -> Option<FocusRegion> {
        if self.open.as_ref().is_some_and(|(kind, _)| kind.closes_on_escape()) { self.close() } else { None }
    }

    /// The open overlay.
    pub fn current(&self) -> Option<&OverlayKind> {
        self.open.as_ref().map(|(kind, _)| kind)
    }

    /// Whether `kind` is open.
    pub fn is_open(&self, kind: &OverlayKind) -> bool {
        self.current() == Some(kind)
    }
}
