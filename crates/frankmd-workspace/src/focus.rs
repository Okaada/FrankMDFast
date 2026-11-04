//! Focus regions (ui-spec §4.3): F6 / Shift+F6 cycle through the visible
//! regions in reading order, Ctrl+1/2/3 jump directly.

use crate::layout::{PaneArrangement, PreviewMode, SidebarMode};

/// Focus regions cycled by F6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRegion {
    /// Explorer.
    Sidebar,
    /// Editor text (or the media library that replaces it).
    Editor,
    /// Preview pane.
    Preview,
    /// Ex command line (vim) only.
    StatusLine,
}

/// Reading order.
const ORDER: [FocusRegion; 4] = [
    FocusRegion::Sidebar,
    FocusRegion::Editor,
    FocusRegion::Preview,
    FocusRegion::StatusLine,
];

impl FocusRegion {
    /// Whether the region can take focus in `arrangement`. The status line
    /// only can while it shows the ex command line (`ex_line`).
    pub fn is_visible(self, arrangement: &PaneArrangement, ex_line: bool) -> bool {
        match self {
            Self::Sidebar => arrangement.sidebar != SidebarMode::Hidden,
            Self::Editor => arrangement.editor || arrangement.media_library,
            Self::Preview => arrangement.preview != PreviewMode::Hidden,
            Self::StatusLine => ex_line,
        }
    }

    /// F6: next visible region in reading order, wrapping.
    pub fn next(self, arrangement: &PaneArrangement) -> FocusRegion {
        self.step(arrangement, false, true)
    }

    /// Shift+F6.
    pub fn prev(self, arrangement: &PaneArrangement) -> FocusRegion {
        self.step(arrangement, false, false)
    }

    /// One cycling step; `ex_line` includes the vim command line.
    pub fn step(self, arrangement: &PaneArrangement, ex_line: bool, forward: bool) -> FocusRegion {
        let start = ORDER.iter().position(|r| *r == self).unwrap_or(0);
        let len = ORDER.len();
        (1..=len)
            .map(|offset| {
                if forward {
                    (start + offset) % len
                } else {
                    (start + len - offset) % len
                }
            })
            .map(|i| ORDER[i])
            .find(|region| region.is_visible(arrangement, ex_line))
            .unwrap_or(self)
    }

    /// Where focus goes when `self` disappears (e.g. the preview was hidden):
    /// the editor, or the first visible region.
    pub fn fallback(self, arrangement: &PaneArrangement) -> FocusRegion {
        if self.is_visible(arrangement, false) {
            return self;
        }
        if Self::Editor.is_visible(arrangement, false) {
            return Self::Editor;
        }
        Self::Editor.next(arrangement)
    }
}
