//! Shared helpers for the `editor_*` headless tests: a window hosting one
//! [`EditorPane`], an event recorder, and key/text drivers.
#![allow(dead_code, unreachable_pub, missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;

use frankmd_app::editor::{EditorEvent, EditorPane, EditorSettings};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Bounds, Entity, Point, TestAppContext, Window,
    WindowBounds, WindowOptions, px, size,
};

/// A window with an editor pane and the events it emitted.
pub struct Fixture {
    pub window: AnyWindowHandle,
    pub pane: Entity<EditorPane>,
    pub events: Rc<RefCell<Vec<EditorEvent>>>,
}

/// Open a 1280×800 window with an editor showing `text`, focused, caret at 0.
pub fn open(cx: &mut TestAppContext, text: &str, settings: EditorSettings) -> Fixture {
    cx.update(gpui_kit::init);
    let events: Rc<RefCell<Vec<EditorEvent>>> = Rc::default();
    let recorder = Rc::clone(&events);
    let text = text.to_owned();
    let (window, pane) = cx.update(|cx| {
        let bounds = Bounds {
            origin: Point::default(),
            size: size(px(1280.), px(800.)),
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };
        let (window, pane) = gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| EditorPane::new(window, cx, settings))
        })
        .expect("open test window");
        cx.subscribe(&pane, move |_, event: &EditorEvent, _| {
            recorder.borrow_mut().push(event.clone())
        })
        .detach();
        (window, pane)
    });
    let fixture = Fixture {
        window,
        pane,
        events,
    };
    fixture.with(cx, |pane, window, cx| {
        pane.update(cx, |pane, cx| {
            pane.set_document(None, &text, window, cx);
            pane.focus(window, cx);
        });
    });
    fixture
}

impl Fixture {
    /// Run `f` in the window after a fresh frame.
    pub fn with<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&Entity<EditorPane>, &mut Window, &mut App) -> R,
    ) -> R {
        let pane = self.pane.clone();
        cx.update_window(self.window, |_, window, cx| {
            window.render_frame(cx);
            let out = f(&pane, window, cx);
            window.render_frame(cx);
            out
        })
        .expect("window open")
    }

    /// Press space-separated keystrokes (`"ctrl-b escape"`).
    pub fn press(&self, cx: &mut TestAppContext, keys: &str) {
        cx.update_window(self.window, |_, window, cx| {
            for key in keys.split_whitespace() {
                window.press(key, cx);
            }
        })
        .expect("window open");
        cx.run_until_parked();
    }

    /// Type text into whatever has focus.
    pub fn type_text(&self, cx: &mut TestAppContext, text: &str) {
        cx.update_window(self.window, |_, window, cx| window.input(text, cx))
            .expect("window open");
        cx.run_until_parked();
    }

    pub fn text(&self, cx: &mut TestAppContext) -> String {
        self.pane.read_with(cx, |pane, cx| pane.text(cx))
    }

    pub fn selection(&self, cx: &mut TestAppContext) -> std::ops::Range<usize> {
        self.pane.read_with(cx, |pane, cx| pane.selection(cx))
    }

    pub fn select(&self, cx: &mut TestAppContext, range: std::ops::Range<usize>) {
        self.pane
            .update(cx, |pane, cx| pane.set_selection(range, cx));
        cx.run_until_parked();
    }

    pub fn is_shown(&self, cx: &mut TestAppContext, id: &'static str) -> bool {
        self.with(cx, |_, window, _| {
            window.try_find(id).is_some_and(|e| e.visible())
        })
    }

    pub fn has_focus(&self, cx: &mut TestAppContext, id: &'static str) -> bool {
        self.with(cx, |_, window, _| {
            window.try_find(id).and_then(|e| e.focused()) == Some(true)
        })
    }

    pub fn take_events(&self) -> Vec<EditorEvent> {
        std::mem::take(&mut *self.events.borrow_mut())
    }

    pub fn apply_settings(&self, cx: &mut TestAppContext, settings: EditorSettings) {
        self.with(cx, |pane, window, cx| {
            pane.update(cx, |pane, cx| pane.apply_settings(settings, window, cx))
        });
        cx.run_until_parked();
    }
}

/// Default settings with vim on.
pub fn vim() -> EditorSettings {
    EditorSettings {
        vim: true,
        macos: false,
        ..EditorSettings::default()
    }
}
