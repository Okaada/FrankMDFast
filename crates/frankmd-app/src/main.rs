//! `frankmd` — FrankMD desktop binary.
//!
//! This crate is deliberately thin: it wires GUI-free crates to GPUI views.
//! Domain logic never lives here (see docs/architecture.md, "the GUI-free rule").
#![allow(dead_code)] // skeleton: bridges are wired up incrementally

mod assets;
mod bridge;
mod cli;
mod ui;

use clap::Parser as _;
use gpui_kit::*;

/// Wayland app_id / X11 WM_CLASS; must match `frankmd.desktop` (OM-006).
const APP_ID: &str = "frankmd";

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    init_tracing();
    tracing::info!(?cli, "starting FrankMD");

    gpui_kit::application()
        .with_assets(assets::FrankAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.activate(true);

            let options = WindowOptions {
                app_id: Some(APP_ID.into()),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1280.), px(800.)),
                    cx,
                ))),
                ..gpui_kit::component::TitleBar::window_options()
            };

            let opened = gpui_kit::open_window(options, cx, |window, cx| {
                window.set_window_title("FrankMD");
                cx.new(|cx| ui::workspace::WorkspaceView::new(window, cx))
            });
            if let Err(err) = opened {
                tracing::error!(%err, "failed to open window");
                cx.quit();
            }
        });
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter = EnvFilter::try_from_env("FRANKMD_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).try_init();
}
