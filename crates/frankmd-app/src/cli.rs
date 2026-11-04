//! Command line (`frankmd open [path]`, headless subcommands later, OM-021).

use std::path::PathBuf;

/// FrankMD command line.
#[derive(Debug, clap::Parser)]
#[command(name = "frankmd", version, about = "Fast Markdown notes and blog editor")]
pub(crate) struct Cli {
    /// Notes folder to open (falls back to `NOTES_PATH`, like `fed <path>`).
    pub(crate) path: Option<PathBuf>,
    /// Override the UI locale (debugging/testing).
    #[arg(long)]
    pub(crate) locale: Option<String>,
}
