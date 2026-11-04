//! Editor presentation settings (the `.fed` editor keys, already resolved by the shell).

use frankmd_markdown::line_numbers::LineNumberMode;
use gpui_kit::SharedString;

/// Line height of the editor text (ui-spec §7.2: editor text `/ 1.6`).
pub const LINE_HEIGHT: f32 = 1.6;

/// What [`super::EditorPane::apply_settings`] takes. The shell maps `.fed` values
/// (`editor_font`, `editor_font_size`, `vim_mode`, `typewriter_mode`,
/// `editor_line_numbers`, `editor_width`, `editor_indent`) into this.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorSettings {
    /// Resolved font family (fontconfig); `None` uses the theme's monospace font.
    pub font_family: Option<SharedString>,
    /// Font size in px (`editor_font_size`, default 14).
    pub font_size: f32,
    /// Vim mode (`vim_mode`).
    pub vim: bool,
    /// Typewriter mode (`typewriter_mode`).
    pub typewriter: bool,
    /// Gutter mode (`editor_line_numbers`). gpui-kit has no relative numbering, so
    /// `Relative` renders absolute numbers (documented gap).
    pub line_numbers: LineNumberMode,
    /// Soft wrap (always on in FrankMD).
    pub soft_wrap: bool,
    /// Text column width in `ch` (`editor_width`, default 72).
    pub width_ch: u16,
    /// Indent width in spaces (`editor_indent`, default 2); 0 means hard tabs.
    pub indent: usize,
    /// Vim keeps its whole Ctrl keymap on macOS (no passthrough list).
    pub macos: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: None,
            font_size: 14.0,
            vim: false,
            typewriter: false,
            line_numbers: LineNumberMode::Off,
            soft_wrap: true,
            width_ch: 72,
            indent: 2,
            macos: cfg!(target_os = "macos"),
        }
    }
}
