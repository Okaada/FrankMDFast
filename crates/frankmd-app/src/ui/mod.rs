//! GPUI views. Each view renders from the workspace's view models and sends
//! registry commands; none owns domain state.

pub mod actions;
pub mod appearance;
mod editor_host;
pub mod fonts;
pub mod i18n;
mod palette;
mod render;
mod settings;
mod shell;

pub use shell::{Shell, ShellConfig};
