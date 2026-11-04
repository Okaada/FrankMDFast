//! Window layout view model (ui-spec §3.1, §4.1, §4.2): width classes with
//! hysteresis, sidebar/preview presentation, view modes, zen and the media
//! library. Pure state; the app renders [`LayoutState::arrangement`].

/// Narrow/medium boundary, logical px.
pub const MEDIUM_MIN_WIDTH: f32 = 720.0;
/// Medium/wide boundary, logical px.
pub const WIDE_MIN_WIDTH: f32 = 1100.0;
/// A width must cross a boundary by this much before the class changes.
pub const HYSTERESIS: f32 = 24.0;
/// Default docked sidebar width.
pub const SIDEBAR_WIDTH: f32 = 248.0;
/// Narrowest usable editor pane.
pub const EDITOR_MIN_WIDTH: f32 = 320.0;
/// Narrowest usable preview pane.
pub const PREVIEW_MIN_WIDTH: f32 = 280.0;
/// Inline padding of the editor text column.
pub const TEXT_COLUMN_INLINE_PADDING: f32 = 32.0;

/// Window width class (§4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WidthClass {
    /// < 720 px.
    Narrow,
    /// 720–1099 px.
    Medium,
    /// ≥ 1100 px.
    Wide,
}

impl WidthClass {
    /// The class `width` falls into when coming from `current`, with
    /// [`HYSTERESIS`] around both boundaries so a tile on a boundary doesn't flicker.
    pub fn with_hysteresis(current: WidthClass, width: f32) -> WidthClass {
        let threshold = |boundary: f32, currently_above: bool| {
            if currently_above { boundary - HYSTERESIS } else { boundary + HYSTERESIS }
        };
        let above_medium = width >= threshold(MEDIUM_MIN_WIDTH, current >= WidthClass::Medium);
        let above_wide = width >= threshold(WIDE_MIN_WIDTH, current == WidthClass::Wide);
        match (above_medium, above_wide) {
            (false, _) => WidthClass::Narrow,
            (true, false) => WidthClass::Medium,
            (true, true) => WidthClass::Wide,
        }
    }
}

/// Persisted view mode (`view_mode` in `.fed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Editor and preview side by side.
    #[default]
    Split,
    /// Editor ⇄ preview swap.
    Single,
}

impl ViewMode {
    /// `.fed` value; anything unknown falls back to Split.
    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("single") => Self::Single,
            _ => Self::Split,
        }
    }
}

/// How the sidebar is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    /// Docked at inline-start.
    Docked,
    /// Not shown.
    Hidden,
    /// Narrow overlay drawer, only while opened.
    Drawer,
}

/// How the preview is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    /// Not shown.
    Hidden,
    /// Beside the editor.
    Split,
    /// Full width, editor hidden (single mode / narrow).
    Swapped,
}

/// What each region shows for one width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneArrangement {
    /// Sidebar presentation.
    pub sidebar: SidebarMode,
    /// Editor pane visible.
    pub editor: bool,
    /// Preview presentation.
    pub preview: PreviewMode,
    /// Media library replaces editor and preview.
    pub media_library: bool,
    /// Pane header visible (false in zen).
    pub pane_header: bool,
    /// Status line shows only mode + save + message (zen, narrow).
    pub status_compact: bool,
}

/// Layout state of one window.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutState {
    /// Last computed width class.
    pub width_class: WidthClass,
    /// User's sidebar preference (persisted per vault).
    pub sidebar_visible: bool,
    /// Narrow drawer currently open.
    pub drawer_open: bool,
    /// User's preview preference in split mode (persisted per vault).
    pub preview_visible: bool,
    /// Split or single.
    pub view_mode: ViewMode,
    /// Single mode / narrow: the preview is the visible pane.
    pub single_shows_preview: bool,
    /// Zen mode (F11).
    pub zen: bool,
    /// Media library replaces editor+preview.
    pub media_library: bool,
    /// Typewriter mode preference.
    pub typewriter: bool,
    /// Preview visibility saved when the media library opened.
    preview_before_library: Option<bool>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutState {
    /// Defaults: wide, sidebar and preview visible, split.
    pub fn new() -> Self {
        Self {
            width_class: WidthClass::Wide,
            sidebar_visible: true,
            drawer_open: false,
            preview_visible: true,
            view_mode: ViewMode::Split,
            single_shows_preview: false,
            zen: false,
            media_library: false,
            typewriter: false,
            preview_before_library: None,
        }
    }

    /// Re-evaluate the width class with hysteresis; returns the new class.
    /// Leaving narrow closes the drawer (the docked sidebar takes over).
    pub fn on_resize(&mut self, logical_width: f32) -> WidthClass {
        let class = WidthClass::with_hysteresis(self.width_class, logical_width);
        if class != WidthClass::Narrow {
            self.drawer_open = false;
        }
        self.width_class = class;
        class
    }

    /// Ctrl+E (narrow: opens/closes the drawer instead).
    pub fn toggle_sidebar(&mut self) {
        if self.width_class == WidthClass::Narrow {
            self.drawer_open = !self.drawer_open;
        } else {
            self.sidebar_visible = !self.sidebar_visible;
        }
    }

    /// Esc / scrim click on the narrow drawer. Returns whether it was open.
    pub fn close_drawer(&mut self) -> bool {
        std::mem::take(&mut self.drawer_open)
    }

    /// Whether the editor and preview swap instead of splitting.
    fn swaps(&self) -> bool {
        self.width_class == WidthClass::Narrow || self.view_mode == ViewMode::Single
    }

    /// Ctrl+Shift+V (split: show/hide; single/narrow: swap).
    pub fn toggle_preview(&mut self) {
        if self.swaps() {
            self.single_shows_preview = !self.single_shows_preview;
        } else {
            self.preview_visible = !self.preview_visible;
        }
    }

    /// F11. Leaving restores the previous arrangement (nothing else changes).
    pub fn toggle_zen(&mut self) {
        self.zen = !self.zen;
    }

    /// Ctrl+Shift+M; leaving restores the previous preview visibility.
    pub fn toggle_media_library(&mut self) {
        if self.media_library {
            self.close_media_library();
        } else {
            self.media_library = true;
            self.preview_before_library = Some(self.preview_visible);
        }
    }

    /// Esc in the library or opening a note from it.
    pub fn close_media_library(&mut self) {
        if !self.media_library {
            return;
        }
        self.media_library = false;
        if let Some(visible) = self.preview_before_library.take() {
            self.preview_visible = visible;
        }
    }

    /// A note was opened: the narrow drawer and the media library close.
    pub fn note_opened(&mut self) {
        self.drawer_open = false;
        self.close_media_library();
    }

    /// What each region shows at `logical_width` (the class is the
    /// hysteresis-stable [`Self::width_class`]).
    pub fn arrangement(&self, logical_width: f32) -> PaneArrangement {
        let narrow = self.width_class == WidthClass::Narrow;
        let mut arrangement = PaneArrangement {
            sidebar: SidebarMode::Hidden,
            editor: true,
            preview: PreviewMode::Hidden,
            media_library: self.media_library,
            pane_header: !self.zen,
            status_compact: self.zen || narrow,
        };
        if self.zen {
            return arrangement;
        }
        if narrow {
            arrangement.sidebar = if self.drawer_open { SidebarMode::Drawer } else { SidebarMode::Hidden };
        } else if self.sidebar_visible {
            arrangement.sidebar = SidebarMode::Docked;
        }
        if self.media_library {
            arrangement.editor = false;
            return arrangement;
        }
        if self.swaps() {
            if self.single_shows_preview {
                arrangement.editor = false;
                arrangement.preview = PreviewMode::Swapped;
            }
        } else if self.preview_visible {
            arrangement.preview = PreviewMode::Split;
            // Medium: the docked sidebar yields to the preview when the editor
            // would drop below its minimum (the preference is kept).
            let needed = EDITOR_MIN_WIDTH + PREVIEW_MIN_WIDTH;
            if self.width_class == WidthClass::Medium
                && arrangement.sidebar == SidebarMode::Docked
                && logical_width - SIDEBAR_WIDTH < needed
            {
                arrangement.sidebar = SidebarMode::Hidden;
            }
        }
        arrangement
    }
}

/// Text column of the editor pane: `(inline_margin, column_width)` for a
/// pane `pane_width` px wide, `editor_width` characters of `char_width` px,
/// centred, never narrower than the pane minus its inline padding allows.
pub fn text_column(pane_width: f32, editor_width: u32, char_width: f32) -> (f32, f32) {
    let available = (pane_width - 2.0 * TEXT_COLUMN_INLINE_PADDING).max(0.0);
    #[allow(clippy::cast_precision_loss)] // editor_width is 40..=200
    let wanted = editor_width as f32 * char_width;
    let column = wanted.min(available);
    ((pane_width - column) / 2.0, column)
}
