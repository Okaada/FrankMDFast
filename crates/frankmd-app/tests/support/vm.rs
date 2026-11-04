//! Proposed GUI-free view-model types (STUBS).
//!
//! Each type names its intended home. They live here only so the example UI
//! tests compile before those crates grow the API; the implementation phase
//! moves them (real logic, plain `#[test]`s) and deletes this file. Bodies are
//! `todo!()` on purpose: the tests that use them are acceptance criteria.

use std::time::Duration;

// ---------------------------------------------------------------------------
// frankmd_workspace::layout (ui-spec §3.1, §4.1, §4.2)
// ---------------------------------------------------------------------------

/// Window width class with 24 px hysteresis (§4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthClass {
    /// < 720 px.
    Narrow,
    /// 720–1099 px.
    Medium,
    /// ≥ 1100 px.
    Wide,
}

/// Persisted view mode (`view_mode` in `.fed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Editor and preview side by side.
    Split,
    /// Editor ⇄ preview swap.
    Single,
}

/// How the sidebar is shown for the current width and state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    /// Docked at inline-start.
    Docked,
    /// Not shown.
    Hidden,
    /// Narrow overlay drawer (Sheet) — only while opened.
    Drawer,
}

/// How the preview is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    /// Not shown.
    Hidden,
    /// Beside the editor.
    Split,
    /// Full width, editor hidden (single mode / narrow).
    Swapped,
}

/// Result of resolving the layout for a width: what each region shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneArrangement {
    /// Sidebar presentation.
    pub sidebar: SidebarMode,
    /// Editor pane visible.
    pub editor: bool,
    /// Preview presentation.
    pub preview: PreviewMode,
    /// Pane header visible (false in zen).
    pub pane_header: bool,
    /// Status line shows only mode + save + message (zen, narrow).
    pub status_compact: bool,
}

/// Layout state of one window (per-vault persisted parts noted on fields).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutState {
    /// Last computed width class.
    pub width_class: WidthClass,
    /// User's sidebar preference (persisted per vault).
    pub sidebar_visible: bool,
    /// Narrow drawer currently open.
    pub drawer_open: bool,
    /// User's preview preference (persisted per vault, FE-172 changed).
    pub preview_visible: bool,
    /// Split or single.
    pub view_mode: ViewMode,
    /// In single mode: preview is the visible pane.
    pub single_shows_preview: bool,
    /// Zen mode (F11).
    pub zen: bool,
    /// Media library replaces editor+preview.
    pub media_library: bool,
    /// Typewriter mode.
    pub typewriter: bool,
}

impl LayoutState {
    /// Defaults: wide, sidebar and preview visible, split.
    pub fn new() -> Self {
        todo!("frankmd_workspace::layout::LayoutState::new")
    }
    /// Re-evaluate the width class with 24 px hysteresis; returns the new class.
    pub fn on_resize(&mut self, _logical_width: f32) -> WidthClass {
        todo!("frankmd_workspace::layout::LayoutState::on_resize")
    }
    /// Ctrl+E (narrow: opens/closes the drawer instead).
    pub fn toggle_sidebar(&mut self) {
        todo!()
    }
    /// Ctrl+Shift+V (split: show/hide; single/narrow: swap).
    pub fn toggle_preview(&mut self) {
        todo!()
    }
    /// F11.
    pub fn toggle_zen(&mut self) {
        todo!()
    }
    /// Ctrl+Shift+M; leaving restores the previous preview visibility.
    pub fn toggle_media_library(&mut self) {
        todo!()
    }
    /// What each region shows at `logical_width`.
    pub fn arrangement(&self, _logical_width: f32) -> PaneArrangement {
        todo!("frankmd_workspace::layout::LayoutState::arrangement")
    }
}

// ---------------------------------------------------------------------------
// frankmd_workspace::focus (§4.3)
// ---------------------------------------------------------------------------

/// Focus regions cycled by F6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRegion {
    /// Explorer.
    Sidebar,
    /// Editor text.
    Editor,
    /// Preview pane.
    Preview,
    /// Ex command line (vim) only.
    StatusLine,
}

impl FocusRegion {
    /// F6: next visible region in reading order, wrapping.
    pub fn next(self, _arrangement: &PaneArrangement) -> FocusRegion {
        todo!("frankmd_workspace::focus::FocusRegion::next")
    }
    /// Shift+F6.
    pub fn prev(self, _arrangement: &PaneArrangement) -> FocusRegion {
        todo!("frankmd_workspace::focus::FocusRegion::prev")
    }
}

// ---------------------------------------------------------------------------
// frankmd_workspace::overlay (§2, §5 intro, §6.1 Escape chain)
// ---------------------------------------------------------------------------

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
    /// A modal dialog.
    Dialog(&'static str),
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

/// One-at-a-time overlay with focus restore.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayStack {
    /// Open overlay and the region to give focus back to.
    pub open: Option<(OverlayKind, FocusRegion)>,
}

impl OverlayStack {
    /// Opens `kind`, replacing any open overlay (keeps the original return focus).
    pub fn open(&mut self, _kind: OverlayKind, _return_to: FocusRegion) {
        todo!("frankmd_workspace::overlay::OverlayStack::open")
    }
    /// Closes; returns where focus goes back.
    pub fn close(&mut self) -> Option<FocusRegion> {
        todo!("frankmd_workspace::overlay::OverlayStack::close")
    }
}

// ---------------------------------------------------------------------------
// frankmd_workspace::commands (§5.1, HP-018)
// ---------------------------------------------------------------------------

/// What the registry needs to know about the current UI to filter commands.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommandContext {
    /// A markdown note is open.
    pub markdown_open: bool,
    /// Caret is inside a table.
    pub caret_in_table: bool,
    /// Vim mode on (toggle state).
    pub vim: bool,
}

/// One palette row.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteRow {
    /// Stable id, e.g. `view.toggle_zen`.
    pub id: &'static str,
    /// `Category: Name` as displayed.
    pub title: String,
    /// Printed shortcut, e.g. `F11`.
    pub shortcut: Option<String>,
    /// Toggle commands: current state (✓).
    pub checked: Option<bool>,
    /// `Some(reason)` for the trailing "Unavailable" section.
    pub unavailable: Option<String>,
}

/// The single command registry.
#[derive(Debug, Default)]
pub struct CommandRegistry;

impl CommandRegistry {
    /// Every built-in command.
    pub fn builtin() -> Self {
        todo!("frankmd_workspace::commands::CommandRegistry::builtin")
    }
    /// Fuzzy palette query; empty query → recently used first.
    pub fn query(&self, _query: &str, _cx: &CommandContext, _recent: &[&str]) -> Vec<PaletteRow> {
        todo!("frankmd_workspace::commands::CommandRegistry::query")
    }
}

// ---------------------------------------------------------------------------
// frankmd_workspace::quick_open (§5.2)
// ---------------------------------------------------------------------------

/// Note entry fed to quick open.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteEntry {
    /// Vault-relative path with `.md`.
    pub path: String,
    /// Modification time (seconds).
    pub mtime: i64,
}

/// Quick-open list model.
#[derive(Debug, Default)]
pub struct QuickOpenModel;

impl QuickOpenModel {
    /// Recents (shared `.fed` recents) first, then the rest by mtime desc.
    pub fn new(_notes: Vec<NoteEntry>, _recents: Vec<String>) -> Self {
        todo!("frankmd_workspace::quick_open::QuickOpenModel::new")
    }
    /// Updates the query (fuzzy via frankmd-search, top 50).
    pub fn set_query(&mut self, _q: &str) {
        todo!()
    }
    /// Visible rows as paths.
    pub fn rows(&self) -> Vec<String> {
        todo!()
    }
    /// ↓ with wrap.
    pub fn select_next(&mut self) {
        todo!()
    }
    /// ↑ with wrap.
    pub fn select_prev(&mut self) {
        todo!()
    }
    /// Selected row index.
    pub fn selected(&self) -> Option<usize> {
        todo!()
    }
    /// Empty result offers "Ctrl+N creates `<query>.md`".
    pub fn create_suggestion(&self) -> Option<String> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// frankmd_workspace::banners (§5.12)
// ---------------------------------------------------------------------------

/// Editor-pane banners, declared in display priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BannerKind {
    /// danger: "Can't reach ~/notes. Edits are kept as drafts." [Retry]
    VaultUnreachable,
    /// info: read-only note or grammar check running.
    ReadOnly,
    /// warning: [Reload, Compare…]
    ChangedOnDisk,
    /// warning: [Save again, Close]
    DeletedOnDisk,
    /// warning: "Saving would remove N characters (P %)." [Undo, Save anyway]
    ContentLoss,
    /// danger: [Details]
    DraftStorageFailed,
    /// info: [Settings, Dismiss]
    StorageFallback,
}

/// Banner stack of the editor pane.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BannerStack {
    raised: Vec<BannerKind>,
}

impl BannerStack {
    /// Raise (idempotent).
    pub fn raise(&mut self, _kind: BannerKind) {
        todo!("frankmd_workspace::banners::BannerStack::raise")
    }
    /// Clear one.
    pub fn clear(&mut self, _kind: BannerKind) {
        todo!()
    }
    /// Visible banners in priority order.
    pub fn visible(&self) -> Vec<BannerKind> {
        todo!()
    }
    /// Button labels (i18n keys) for a banner.
    pub fn buttons(_kind: BannerKind) -> Vec<&'static str> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// frankmd_workspace::status (§3.6)
// ---------------------------------------------------------------------------

/// Save-state slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveState {
    /// ● Unsaved.
    Unsaved,
    /// ↻ Saving.
    Saving,
    /// ✓ Saved.
    Saved,
    /// ✕ Save failed.
    Failed,
    /// ⚠ Draft not stored.
    DraftNotStored,
}

/// Status-line message slot with TTL (2 s info, 5 s error), injected clock.
#[derive(Debug, Default)]
pub struct MessageSlot {
    current: Option<(String, bool, Duration)>,
}

impl MessageSlot {
    /// Show a message now (replaces the previous one).
    pub fn show(&mut self, _text: &str, _error: bool, _now: Duration) {
        todo!("frankmd_workspace::status::MessageSlot::show")
    }
    /// Current text at `now`, if not expired.
    pub fn current(&self, _now: Duration) -> Option<String> {
        todo!()
    }
}
