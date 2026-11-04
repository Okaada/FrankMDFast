//! The GUI-free view models the UI tests use. They used to be stubs here; the
//! real types live in `frankmd-workspace` (ui-test-plan §4) and are re-exported
//! under the names the tests were written against.

pub use frankmd_workspace::banners::{BannerKind, BannerStack};
pub use frankmd_workspace::commands::{CommandContext, CommandRegistry, PaletteRow};
pub use frankmd_workspace::focus::FocusRegion;
pub use frankmd_workspace::layout::{
    LayoutState, PaneArrangement, PreviewMode, SidebarMode, ViewMode, WidthClass,
};
pub use frankmd_workspace::overlay::{OverlayKind, OverlayStack, PaletteMode};
pub use frankmd_workspace::quick_open::{NoteEntry, QuickOpenModel};
pub use frankmd_workspace::status::{MessageSlot, SaveState};
