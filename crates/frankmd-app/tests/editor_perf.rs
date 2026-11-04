//! Large-note performance probe (gpui-stack.md risk R5 / spike S4). Ignored by default;
//! run in release with real text shaping:
//!
//! ```sh
//! cargo test --release -p frankmd-app --test editor_perf -- --ignored --nocapture
//! ```
//!
//! It opens a ~5 MB markdown note in a headless window backed by the Linux text
//! system (cosmic-text + fontconfig, real glyph shaping; no GPU), then measures
//! `set_document`, the first frame, keystroke → next frame while typing at the end and
//! in the middle, vim motions, a find over the whole note and a replace-all.
//! `FRANKMD_PERF_MB` overrides the size.
#![allow(missing_docs, clippy::unwrap_used)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use frankmd_app::editor::{EditorPane, EditorSettings};
use gpui_kit::{
    AnyWindowHandle, AppContext as _, Bounds, Entity, HeadlessAppContext, Keystroke, Point, Window,
    WindowBounds, WindowOptions, px, size,
};

/// A realistic note: headings, prose with inline markup, lists, tasks, code, tables.
fn note(bytes: usize) -> String {
    let block = "## Section heading\n\n\
        The document is the **centre** of the workspace, with *calm* typography and a [link](https://example.com) \
        or two, some `inline code`, and a footnote-free paragraph that wraps across the column.\n\n\
        - [ ] first task item\n- [x] done item with ~~strike~~\n- plain bullet\n\n\
        ```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n\
        | a | b |\n|---|---|\n| 1 | 2 |\n\n";
    let mut s = String::with_capacity(bytes + block.len());
    while s.len() < bytes {
        s.push_str(block);
    }
    s
}

fn stats(label: &str, mut samples: Vec<Duration>) {
    samples.sort();
    let pct = |p: f64| samples[((samples.len() as f64 - 1.0) * p).round() as usize];
    let mean = samples.iter().sum::<Duration>() / samples.len() as u32;
    println!(
        "{label:<38} n={:<4} mean={:>8.2?} p50={:>8.2?} p95={:>8.2?} max={:>8.2?}",
        samples.len(),
        mean,
        pct(0.5),
        pct(0.95),
        samples[samples.len() - 1]
    );
}

fn frame(window: &mut Window, cx: &mut gpui_kit::App) {
    window.refresh();
    window.draw(cx).clear(cx);
}

/// Dispatch `key` and draw the next frame; the elapsed time is keystroke → frame.
fn key_to_frame(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    key: &'static str,
) -> Duration {
    let keystroke = Keystroke::parse(key).unwrap();
    let start = Instant::now();
    cx.update_window(handle, |_, window, cx| {
        window.dispatch_keystroke(keystroke.with_simulated_ime(), cx);
        frame(window, cx);
    })
    .unwrap();
    start.elapsed()
}

fn with_pane<R>(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    pane: &Entity<EditorPane>,
    f: impl FnOnce(&mut EditorPane, &mut Window, &mut gpui_kit::Context<EditorPane>) -> R,
) -> R {
    cx.update_window(handle, |_, window, cx| {
        pane.update(cx, |p, cx| f(p, window, cx))
    })
    .unwrap()
}

#[test]
#[ignore = "performance probe; run in release with --ignored --nocapture"]
fn five_megabyte_note() {
    let mb: f64 = std::env::var("FRANKMD_PERF_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5.0);
    let text = note((mb * 1024.0 * 1024.0) as usize);
    println!(
        "note: {:.2} MB, {} lines",
        text.len() as f64 / 1048576.0,
        text.lines().count()
    );

    let text_system = gpui_kit::platform::current_platform(true).text_system();
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || Ok(None));
    cx.update(gpui_kit::init);
    let (handle, pane) = cx.update(|cx| {
        let bounds = Bounds {
            origin: Point::default(),
            size: size(px(1280.), px(800.)),
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| EditorPane::new(window, cx, EditorSettings::default()))
        })
        .unwrap()
    });
    cx.update_window(handle, |_, window, cx| frame(window, cx))
        .unwrap();

    let start = Instant::now();
    with_pane(&mut cx, handle, &pane, |p, window, cx| {
        p.set_document(None, &text, window, cx);
        p.focus(window, cx);
    });
    let set = start.elapsed();
    let start = Instant::now();
    cx.update_window(handle, |_, window, cx| frame(window, cx))
        .unwrap();
    let first = start.elapsed();
    let start = Instant::now();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| frame(window, cx))
        .unwrap();
    let settle = start.elapsed();
    println!(
        "set_document {set:.2?}; first frame {first:.2?}; background work + frame {settle:.2?}"
    );

    // Steady-state frame with nothing changed.
    let idle: Vec<_> = (0..20)
        .map(|_| {
            let s = Instant::now();
            cx.update_window(handle, |_, window, cx| frame(window, cx))
                .unwrap();
            s.elapsed()
        })
        .collect();
    stats("idle frame", idle);

    // Typing at the end of the note.
    with_pane(&mut cx, handle, &pane, |p, _, cx| {
        let end = p.text(cx).len();
        p.set_selection(end..end, cx);
    });
    cx.update_window(handle, |_, window, cx| frame(window, cx))
        .unwrap();
    let typing: Vec<_> = (0..120)
        .map(|i| key_to_frame(&mut cx, handle, if i % 8 == 7 { "space" } else { "a" }))
        .collect();
    cx.run_until_parked();
    stats("type at end (key -> frame)", typing);

    // Typing in the middle (re-highlight and re-wrap of the edited region).
    with_pane(&mut cx, handle, &pane, |p, _, cx| {
        let mid = p.text(cx).len() / 2;
        let mid = p.text(cx)[..mid].rfind('\n').unwrap_or(0);
        p.set_selection(mid..mid, cx);
    });
    cx.update_window(handle, |_, window, cx| frame(window, cx))
        .unwrap();
    let typing: Vec<_> = (0..120)
        .map(|_| key_to_frame(&mut cx, handle, "b"))
        .collect();
    stats("type in middle (key -> frame)", typing);
    let enter: Vec<_> = (0..30)
        .map(|_| key_to_frame(&mut cx, handle, "enter"))
        .collect();
    stats("enter in middle (key -> frame)", enter);
    let back: Vec<_> = (0..30)
        .map(|_| key_to_frame(&mut cx, handle, "backspace"))
        .collect();
    stats("backspace in middle (key -> frame)", back);
    let arrows: Vec<_> = (0..60)
        .map(|_| key_to_frame(&mut cx, handle, "down"))
        .collect();
    stats("arrow down (key -> frame)", arrows);
    cx.run_until_parked();

    // Vim: motions and an edit (each key reads the text through the shadow buffer).
    with_pane(&mut cx, handle, &pane, |p, window, cx| {
        p.apply_settings(
            EditorSettings {
                vim: true,
                ..EditorSettings::default()
            },
            window,
            cx,
        );
    });
    let vim_j: Vec<_> = (0..60)
        .map(|_| key_to_frame(&mut cx, handle, "j"))
        .collect();
    stats("vim j (key -> frame)", vim_j);
    let vim_w: Vec<_> = (0..60)
        .map(|_| key_to_frame(&mut cx, handle, "w"))
        .collect();
    stats("vim w (key -> frame)", vim_w);
    let vim_x: Vec<_> = (0..30)
        .map(|_| key_to_frame(&mut cx, handle, "x"))
        .collect();
    stats("vim x (edit, key -> frame)", vim_x);
    let vim_u: Vec<_> = (0..10)
        .map(|_| key_to_frame(&mut cx, handle, "u"))
        .collect();
    stats("vim u (key -> frame)", vim_u);
    with_pane(&mut cx, handle, &pane, |p, window, cx| {
        p.apply_settings(EditorSettings::default(), window, cx)
    });

    // Find over the whole note, and replace-all as one edit.
    let start = Instant::now();
    with_pane(&mut cx, handle, &pane, |p, window, cx| {
        p.run_command("find.find", window, cx)
    });
    cx.update_window(handle, |_, window, cx| {
        window.dispatch_keystroke(Keystroke::parse("t").unwrap().with_simulated_ime(), cx);
        window.dispatch_keystroke(Keystroke::parse("h").unwrap().with_simulated_ime(), cx);
        window.dispatch_keystroke(Keystroke::parse("e").unwrap().with_simulated_ime(), cx);
        frame(window, cx);
    })
    .unwrap();
    let matches = pane.read_with(&cx, |p, _| p.find_state().ranges().len());
    println!(
        "find 'the' (3 keystrokes, live count + frame): {:.2?}, {matches} matches",
        start.elapsed()
    );
    // Whole-text edit with the find bar open (57k match decorations) and closed.
    let replace = |cx: &mut HeadlessAppContext, from: &'static str, to: &'static str| {
        let start = Instant::now();
        with_pane(cx, handle, &pane, |p, window, cx| {
            let edit = frankmd_app::editor::Edit {
                range: 0..p.text(cx).len(),
                text: p.text(cx).replace(from, to),
                selection: 0..0,
            };
            p.apply_edit(edit, window, cx);
        });
        cx.update_window(handle, |_, window, cx| frame(window, cx))
            .unwrap();
        start.elapsed()
    };
    println!(
        "whole-text edit + frame, find bar open: {:.2?}",
        replace(&mut cx, "Section", "Part")
    );
    with_pane(&mut cx, handle, &pane, |p, window, cx| {
        p.close_find(window, cx)
    });
    println!(
        "whole-text edit + frame, find bar closed: {:.2?}",
        replace(&mut cx, "Part", "Section")
    );
    let start = Instant::now();
    let total = with_pane(&mut cx, handle, &pane, |p, window, cx| {
        let len = p.text(cx).len();
        let edit = frankmd_app::editor::Edit {
            range: 0..len,
            text: p.text(cx).replace("Section", "Chapter"),
            selection: 0..0,
        };
        p.apply_edit(edit, window, cx);
        p.text(cx).len()
    });
    cx.update_window(handle, |_, window, cx| frame(window, cx))
        .unwrap();
    println!(
        "replace-all style whole-text edit + frame: {:.2?} ({total} bytes)",
        start.elapsed()
    );
}

#[test]
#[ignore = "performance probe; run in release with --ignored --nocapture"]
fn whole_text_replacement_paths() {
    let text = note(5 * 1024 * 1024);
    let text_system = gpui_kit::platform::current_platform(true).text_system();
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || Ok(None));
    cx.update(gpui_kit::init);
    let (handle, pane) = cx.update(|cx| {
        let bounds = Bounds {
            origin: Point::default(),
            size: size(px(1280.), px(800.)),
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| EditorPane::new(window, cx, EditorSettings::default()))
        })
        .unwrap()
    });
    with_pane(&mut cx, handle, &pane, |p, window, cx| {
        p.set_document(None, &text, window, cx)
    });
    cx.update_window(handle, |_, window, cx| frame(window, cx))
        .unwrap();
    let replaced = text.replace("Section", "Chapter");
    let state = pane.read_with(&cx, |p, _| p.editor_state().clone());
    let start = Instant::now();
    cx.update_window(handle, |_, window, cx| {
        state.update(cx, |s, cx| s.replace_all(replaced.clone(), window, cx));
    })
    .unwrap();
    println!("EditorState::replace_all: {:.2?}", start.elapsed());
    let start = Instant::now();
    cx.update_window(handle, |_, window, cx| {
        state.update(cx, |s, cx| {
            s.set_selected_range(0..s.text().len(), cx);
            s.replace(text.clone(), window, cx);
        });
    })
    .unwrap();
    println!("select-all + replace: {:.2?}", start.elapsed());
    let start = Instant::now();
    cx.update_window(handle, |_, window, cx| frame(window, cx))
        .unwrap();
    println!("next frame: {:.2?}", start.elapsed());
}
