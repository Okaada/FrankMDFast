//! `frankmd` binary entry point; everything lives in the library.

fn main() -> anyhow::Result<()> {
    frankmd_app::run()
}
