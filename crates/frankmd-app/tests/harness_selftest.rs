//! Self-test of the GPUI headless test harness (no FrankMD code involved).
//!
//! Each test exercises one gpui / gpui-kit test facility that the UI test plan
//! (docs/testing/ui-test-plan.md) relies on, against a tiny probe view. These
//! tests are expected to PASS today; they document what works headless on Linux
//! with `gpui-kit =0.9.0` / `gpui-pre =0.3.11` and `features = ["test-support"]`.
#![allow(missing_docs, clippy::unwrap_used)]

// `#[gpui_kit::test]` expands to `gpui::…` paths; alias the kit as `gpui`.
extern crate gpui_kit as gpui;

mod support;

use std::time::Duration;

use gpui_kit::test::{TestAppContextExt as _, TestSupportExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, ClipboardItem, Context, FocusHandle, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _,
    TestAppContext, Window, actions, div, px, size,
};

actions!(probe, [Toggle, FocusB]);

/// Two focusable panes, a counter bound to an action, and a timer-driven flag.
struct Probe {
    a: FocusHandle,
    b: FocusHandle,
    toggles: u32,
    clicks: u32,
    ticked: bool,
    width_seen: f32,
}

impl Probe {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            a: cx.focus_handle(),
            b: cx.focus_handle(),
            toggles: 0,
            clicks: 0,
            ticked: false,
            width_seen: 0.0,
        }
    }
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.width_seen = f32::from(window.viewport_size().width);
        div()
            .id("root")
            .key_context("Probe")
            .size_full()
            .on_action(cx.listener(|this, _: &Toggle, _, cx| {
                this.toggles += 1;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusB, window, cx| {
                this.b.focus(window, cx);
            }))
            .child(
                div()
                    .id("pane-a")
                    .test_support()
                    .track_focus(&self.a)
                    .w(px(100.))
                    .h(px(40.))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("pane-b")
                    .test_support()
                    .track_focus(&self.b)
                    .w(px(100.))
                    .h(px(40.)),
            )
            .when(self.toggles % 2 == 1, |el| {
                el.child(div().id("overlay").test_support().w(px(50.)).h(px(20.)))
            })
    }
}

use gpui_kit::prelude::FluentBuilder as _;

fn open_probe(cx: &mut TestAppContext) -> (gpui_kit::AnyWindowHandle, gpui_kit::Entity<Probe>) {
    cx.update(gpui_kit::init);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("ctrl-shift-p", Toggle, None),
            KeyBinding::new("f6", FocusB, Some("Probe")),
        ]);
    });
    let (handle, probe) = support::harness::open_window(cx, size(px(1280.), px(800.)), |_, cx| {
        cx.new(Probe::new)
    });
    (handle.into(), probe)
}

// Entities and the deterministic executor work without a window.
#[gpui_kit::test]
fn entity_update_and_read(cx: &mut TestAppContext) {
    let probe = cx.new(Probe::new);
    probe.update(cx, |p, _| p.toggles = 3);
    assert_eq!(probe.read_with(cx, |p, _| p.toggles), 3);
}

// A headless window opens and renders a frame; observed elements are queryable.
#[gpui_kit::test]
fn window_renders_and_elements_are_observable(cx: &mut TestAppContext) {
    let (handle, _probe) = open_probe(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("pane-a").visible());
        assert!(window.find("pane-a").bounds().size.width > px(0.));
        assert!(window.try_find("overlay").is_none());
    })
    .unwrap();
}

// Keystrokes go through the real keymap (bind_keys) to `on_action` handlers.
#[gpui_kit::test]
fn keystroke_dispatches_bound_action(cx: &mut TestAppContext) {
    let (handle, probe) = open_probe(cx);
    cx.update_window(handle, |_, window, cx| {
        window.click("pane-a", cx); // gives the window a focused element
        window.press("ctrl-shift-p", cx);
        assert!(window.find("overlay").visible());
        window.press("ctrl-shift-p", cx);
        assert!(window.try_find("overlay").is_none());
    })
    .unwrap();
    assert_eq!(probe.read_with(cx, |p, _| p.toggles), 2);
}

// The TestAppContext-level keystroke and action APIs work too.
#[gpui_kit::test]
fn simulate_keystrokes_and_dispatch_action(cx: &mut TestAppContext) {
    let (handle, probe) = open_probe(cx);
    cx.update_window(handle, |_, window, cx| window.click("pane-a", cx)).unwrap();
    cx.simulate_keystrokes(handle, "ctrl-shift-p");
    cx.dispatch_action(handle, Toggle);
    cx.run_until_parked();
    assert_eq!(probe.read_with(cx, |p, _| p.toggles), 2);
}

// Focus is tracked per FocusHandle and visible through snapshots; context-scoped
// bindings (`Some("Probe")`) fire when focus is inside that key context.
#[gpui_kit::test]
fn focus_moves_between_regions(cx: &mut TestAppContext) {
    let (handle, _probe) = open_probe(cx);
    cx.update_window(handle, |_, window, cx| {
        window.click("pane-a", cx);
        window.render_frame(cx);
        assert_eq!(window.find("pane-a").focused(), Some(true));
        window.press("f6", cx);
        assert_eq!(window.find("pane-b").focused(), Some(true));
        assert_eq!(window.find("pane-a").focused(), Some(false));
    })
    .unwrap();
}

// Mouse clicks use real hit testing.
#[gpui_kit::test]
fn click_hits_target(cx: &mut TestAppContext) {
    let (handle, probe) = open_probe(cx);
    cx.update_window(handle, |_, window, cx| {
        window.click("pane-a", cx);
        window.double_click("pane-a", cx);
    })
    .unwrap();
    assert_eq!(probe.read_with(cx, |p, _| p.clicks), 3);
}

// Window resizes reach render (width classes, §4.1).
#[gpui_kit::test]
fn window_resize_is_seen_by_render(cx: &mut TestAppContext) {
    let (handle, probe) = open_probe(cx);
    cx.simulate_window_resize(handle, size(px(700.), px(600.)));
    cx.update_window(handle, |_, window, cx| window.render_frame(cx)).unwrap();
    assert_eq!(probe.read_with(cx, |p, _| p.width_seen), 700.0);
}

// Timers run on the deterministic test clock: `advance_clock` drives debounces.
#[gpui_kit::test]
async fn timers_advance_on_test_clock(cx: &mut TestAppContext) {
    let (handle, probe) = open_probe(cx);
    probe.update(cx, |_, cx| {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            this.update(cx, |p, cx| {
                p.ticked = true;
                cx.notify();
            })
            .ok();
        })
        .detach();
    });
    cx.executor().advance_clock(Duration::from_millis(1999));
    assert!(!probe.read_with(cx, |p, _| p.ticked));
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert!(probe.read_with(cx, |p, _| p.ticked));
    // wait_for polls frames on the test clock.
    cx.wait_for(handle, Duration::from_millis(50), |_, _| true).await;
}

// The test platform clipboard round-trips (Copy path / Copy contents).
#[gpui_kit::test]
fn clipboard_round_trip(cx: &mut TestAppContext) {
    cx.write_to_clipboard(ClipboardItem::new_string("notes/a.md".into()));
    assert_eq!(
        cx.read_from_clipboard().and_then(|c| c.text()).as_deref(),
        Some("notes/a.md")
    );
}

// Opening an external URL is recorded by the test platform (preview links §5.21).
#[gpui_kit::test]
fn opened_url_is_recorded(cx: &mut TestAppContext) {
    cx.update(|cx| cx.open_url("https://example.com"));
    assert_eq!(cx.opened_url().as_deref(), Some("https://example.com"));
}

// GOTCHA (verified): with NO focused element, keystrokes do not reach the
// root view's `on_action` handlers. The shell must focus a handle on open
// (editor or empty-state input) or global shortcuts are dead after launch.
#[gpui_kit::test]
fn keystroke_without_focus_is_not_dispatched(cx: &mut TestAppContext) {
    let (handle, probe) = open_probe(cx);
    cx.update_window(handle, |_, window, cx| window.press("ctrl-shift-p", cx)).unwrap();
    assert_eq!(probe.read_with(cx, |p, _| p.toggles), 0);
}

/// Text recorded with `.test_text(...)` is queryable (gpui_base::test_support::texts).
struct Label;
impl Render for Label {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::base::TestTextExt as _;
        div().size_full().child(div().child("✓ Saved").test_text("✓ Saved"))
    }
}

#[gpui_kit::test]
fn rendered_text_is_observable_when_recorded(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, _) = support::harness::open_window(cx, size(px(400.), px(200.)), |_, cx| cx.new(|_| Label));
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let texts = gpui_kit::base::test_support::texts(window);
        assert!(texts.iter().any(|t| t.text() == "✓ Saved" && t.visible()));
    })
    .unwrap();
}
