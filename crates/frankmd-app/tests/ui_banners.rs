//! Adapted UI tests: editor-pane banners and the status-line message slot.
//!
//! See docs/testing/ui-test-plan.md (ui-spec §5.12, §3.6).
#![allow(missing_docs, clippy::unwrap_used)]

extern crate gpui_kit as gpui;

mod support;

use std::time::Duration;

use gpui_kit::TestAppContext;
use support::harness::{self, WIDE};
use support::shell::{VaultFixture, open_shell};
use support::vm::{BannerKind, BannerStack, MessageSlot};

// ---- level a: frankmd_workspace::banners / status ---------------------------

// §5.12: banners stack in a fixed priority order regardless of raise order; raising twice is a no-op.
#[test]
fn banners_stack_in_priority_order() {
    let mut stack = BannerStack::default();
    stack.raise(BannerKind::ContentLoss);
    stack.raise(BannerKind::VaultUnreachable);
    stack.raise(BannerKind::ContentLoss);
    stack.raise(BannerKind::ChangedOnDisk);
    assert_eq!(
        stack.visible(),
        vec![BannerKind::VaultUnreachable, BannerKind::ChangedOnDisk, BannerKind::ContentLoss]
    );
    stack.clear(BannerKind::VaultUnreachable);
    assert_eq!(stack.visible().first(), Some(&BannerKind::ChangedOnDisk));
}

// adapts: test/javascript/controllers/autosave_controller.test.js:869
// "sets override, dismisses banner, and calls saveNow" — content-loss offers Undo / Save anyway.
#[test]
fn content_loss_banner_offers_undo_and_save_anyway() {
    assert_eq!(
        BannerStack::buttons(BannerKind::ContentLoss),
        vec!["banners.content_loss.undo", "banners.content_loss.save_anyway"]
    );
    assert_eq!(BannerStack::buttons(BannerKind::VaultUnreachable), vec!["banners.offline.retry"]);
}

// changed (§2.1, §3.6): toasts → status-line message slot; 2 s info / 5 s errors, single instance.
#[test]
fn status_messages_expire_on_the_injected_clock() {
    let mut slot = MessageSlot::default();
    slot.show("Saved", false, Duration::ZERO);
    assert_eq!(slot.current(Duration::from_millis(1999)).as_deref(), Some("Saved"));
    assert_eq!(slot.current(Duration::from_secs(2)), None);
    slot.show("Save failed", true, Duration::from_secs(10));
    slot.show("Copied", false, Duration::from_secs(11));
    assert_eq!(slot.current(Duration::from_secs(11)).as_deref(), Some("Copied"), "single instance");
}

// ---- level b: real shell ------------------------------------------------------

// adapts: test/javascript/controllers/connection_monitor_controller.test.js:101 "shows banner when going offline",
// :110 "disables textarea when going offline" (changed §5.12/§11: editing continues as drafts),
// :139 "hides banner when coming online".
#[gpui_kit::test]
fn vault_unreachable_banner_keeps_editing_enabled(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    let w = shell.window;
    shell.simulate(cx, BannerKind::VaultUnreachable);
    assert!(harness::is_shown(cx, w, "banner-vault-unreachable"));
    harness::press(cx, w, "ctrl-2");
    harness::type_text(cx, w, "still typing");
    assert_eq!(
        shell.save_state(cx),
        support::vm::SaveState::Unsaved,
        "the edit landed; it will be kept as a draft"
    );
}

// adapts: autosave_controller.test.js:850 "hides banner and resets flags" — Undo restores the text
// and clears the banner; focus goes back to the editor (banner buttons are keyboard reachable, §9).
#[gpui_kit::test]
fn content_loss_banner_undo_clears_it(cx: &mut TestAppContext) {
    let shell = open_shell(cx, WIDE, VaultFixture::sample());
    let w = shell.window;
    shell.simulate(cx, BannerKind::ContentLoss);
    assert_eq!(shell.banners(cx), vec![BannerKind::ContentLoss]);
    harness::in_window(cx, w, |window, cx| {
        use gpui_kit::test::TestWindowExt as _;
        window.click("banner-content-loss-undo", cx);
    });
    assert!(shell.banners(cx).is_empty());
    assert!(!harness::is_shown(cx, w, "banner-content-loss"));
}
