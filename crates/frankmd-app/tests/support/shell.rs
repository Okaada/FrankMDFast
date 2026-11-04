//! Entry point for level-b tests: the real app shell in a headless window.
//!
//! STUB. `frankmd-app` is a binary-only crate today, so integration tests can't
//! reach its views. Requirement for the implementation phase: add
//! `crates/frankmd-app/src/lib.rs` exposing `ui::Shell::new(window, cx, ShellConfig)`
//! (vault path, injected clock/env) and `keymap::install(cx)`, keep `main.rs` as
//! a thin binary over it, then replace the `todo!()`s below with calls into it.
//!
//! The window is real (see `harness`); what's stubbed is the root view and its
//! state accessors. Accessors read the GUI-free view-models (see `vm`) out of
//! the shell's `WorkspaceModel` entity, so assertions are on state, plus
//! element ids / focus for wiring (gpui-kit can't assert rendered text).

use gpui_kit::{AnyWindowHandle, Pixels, Size, TestAppContext};

use super::vm::{BannerKind, FocusRegion, LayoutState, OverlayKind, SaveState};

/// A vault to build in a temp dir before the shell opens it.
#[derive(Debug, Clone, Default)]
pub struct VaultFixture {
    /// `(relative path, content)`; folders are implied by paths.
    pub notes: Vec<(&'static str, &'static str)>,
    /// Shared recents (newest first), written to `.fed` `recent_files`.
    pub recents: Vec<&'static str>,
    /// Note to open at startup (`frankmd open <path>`), if any.
    pub open: Option<&'static str>,
}

impl VaultFixture {
    /// A small vault: two folders, four notes, one open.
    pub fn sample() -> Self {
        Self {
            notes: vec![
                ("Inbox.md", "# Inbox\n"),
                ("blog/2026/Project brief.md", "# Project brief\n\nBody.\n"),
                ("projects/Launch list.md", "# Launch list\n\n- [ ] ship\n"),
                ("reference/Glossary.md", "# Glossary\n"),
            ],
            recents: vec!["projects/Launch list.md", "Inbox.md"],
            open: Some("blog/2026/Project brief.md"),
        }
    }

    /// Same vault, nothing open (empty state with docked quick open, §5.2).
    pub fn sample_no_note() -> Self {
        Self { open: None, ..Self::sample() }
    }
}

/// Handle to an opened shell window.
pub struct Shell {
    /// The test window.
    pub window: AnyWindowHandle,
}

/// Opens the app shell on `fixture` at `size`, with the default keymap bound.
pub fn open_shell(cx: &mut TestAppContext, _size: Size<Pixels>, _fixture: VaultFixture) -> Shell {
    cx.update(gpui_kit::init);
    cx.update(|cx| cx.bind_keys(super::actions::default_keymap()));
    todo!("open frankmd_app::ui::Shell in a headless window via support::harness::open_window")
}

impl Shell {
    /// Layout view-model.
    pub fn layout(&self, _cx: &mut TestAppContext) -> LayoutState {
        todo!("read WorkspaceModel → layout")
    }
    /// Focused region (from the shell's focus handles).
    pub fn focus_region(&self, _cx: &mut TestAppContext) -> Option<FocusRegion> {
        todo!("read focused region")
    }
    /// Open overlay.
    pub fn overlay(&self, _cx: &mut TestAppContext) -> Option<OverlayKind> {
        todo!("read OverlayStack")
    }
    /// Visible banners, top to bottom.
    pub fn banners(&self, _cx: &mut TestAppContext) -> Vec<BannerKind> {
        todo!("read BannerStack")
    }
    /// Save-state slot.
    pub fn save_state(&self, _cx: &mut TestAppContext) -> SaveState {
        todo!("read status line")
    }
    /// Active note path.
    pub fn active_note(&self, _cx: &mut TestAppContext) -> Option<String> {
        todo!("read Snapshot.active")
    }
    /// Test hook: push a workspace condition that raises a banner
    /// (e.g. vault unreachable, content-loss guard) through the real command path.
    pub fn simulate(&self, _cx: &mut TestAppContext, _condition: BannerKind) {
        todo!("send the Command that raises this banner")
    }
}
