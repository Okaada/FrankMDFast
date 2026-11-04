//! Headless window helpers over `gpui_kit::test`.
//!
//! Verified against gpui-kit 0.9.0 / gpui-pre 0.3.11 (see harness_selftest.rs).

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Bounds, Entity, Pixels, Point, Render, Size,
    TestAppContext, Window, WindowBounds, WindowHandle, WindowOptions, base::Root, px, size,
};

/// Window sizes for the three width classes (ui-spec §4.1).
pub const WIDE: Size<Pixels> = Size { width: px(1280.), height: px(800.) };
/// 720–1099 px.
pub const MEDIUM: Size<Pixels> = Size { width: px(900.), height: px(800.) };
/// < 720 px.
pub const NARROW: Size<Pixels> = Size { width: px(640.), height: px(800.) };

/// Opens a window through the production entry point (`gpui_kit::open_window`,
/// which wraps the view in a Base `Root`) with deterministic bounds.
pub fn open_window<V: Render>(
    cx: &mut TestAppContext,
    window_size: Size<Pixels>,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> (WindowHandle<Root>, Entity<V>) {
    cx.update(|cx| {
        let bounds = Bounds { origin: Point::default(), size: window_size };
        let (window, content) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            build,
        )
        .expect("open test window");
        (window.downcast::<Root>().expect("Base Root"), content)
    })
}

/// Runs `f` inside the window after completing a fresh frame.
pub fn in_window<R>(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    f: impl FnOnce(&mut Window, &mut App) -> R,
) -> R {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        f(window, cx)
    })
    .expect("test window closed")
}

/// Presses a sequence of space-separated keystrokes (e.g. `"ctrl-shift-p escape"`).
pub fn press(cx: &mut TestAppContext, handle: AnyWindowHandle, keys: &str) {
    in_window(cx, handle, |window, cx| {
        for key in keys.split_whitespace() {
            window.press(key, cx);
        }
    });
}

/// Types text into whatever has focus.
pub fn type_text(cx: &mut TestAppContext, handle: AnyWindowHandle, text: &str) {
    in_window(cx, handle, |window, cx| window.input(text, cx));
}

/// Is an observed element present AND visible in the last frame?
pub fn is_shown(cx: &mut TestAppContext, handle: AnyWindowHandle, id: &'static str) -> bool {
    in_window(cx, handle, |window, _| window.try_find(id).is_some_and(|e| e.visible()))
}

/// Does the observed element report keyboard focus?
pub fn has_focus(cx: &mut TestAppContext, handle: AnyWindowHandle, id: &'static str) -> bool {
    in_window(cx, handle, |window, _| window.try_find(id).and_then(|e| e.focused()) == Some(true))
}

/// Resizes and re-renders (hysteresis tests resize in small steps).
pub fn resize(cx: &mut TestAppContext, handle: AnyWindowHandle, width: f32) {
    cx.simulate_window_resize(handle, size(px(width), px(800.)));
    in_window(cx, handle, |_, _| ());
}
