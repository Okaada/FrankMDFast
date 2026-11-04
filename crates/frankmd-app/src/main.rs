//! `frankmd` binary entry point; everything lives in the library.

// A GUI app: no console window on Windows (debug builds keep it for logs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> anyhow::Result<()> {
    frankmd_app::run()
}
