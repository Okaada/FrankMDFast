//! GPUI views. Each view renders from the workspace's view models and sends
//! registry commands; none owns domain state.

pub mod actions;
pub mod appearance;
mod commands;
mod editor_host;
mod editor_pane;
mod explorer;
mod file_dialogs;
pub mod fonts;
mod help;
pub mod i18n;
mod media_host;
mod navigation;
mod overlay_view;
mod overlays;
mod palette;
mod prelude;
mod preview_region;
mod recovery;
mod render;
mod search;
mod settings;
mod shell;
mod sidebar;
mod status_line;
mod tasks;

pub use shell::{Shell, ShellConfig};
