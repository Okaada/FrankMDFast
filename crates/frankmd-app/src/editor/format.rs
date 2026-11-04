//! Markdown formatting commands: re-exported from [`frankmd_core::text::format`]
//! (GUI-free; the pane only applies the returned [`super::Edit`]s).

pub use frankmd_core::text::format::{
    FORMATS, TextFormat, apply_menu_format, format_by_hotkey, format_by_id, insert_link,
    toggle_tasks, wrap_selection,
};
