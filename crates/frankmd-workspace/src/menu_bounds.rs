//! Dropdown menu viewport clamping math (logic of `lib/menu_bounds.js`,
//! issue #141): menus stay anchored to their trigger but never extend past
//! the viewport; they scroll internally instead. Pure geometry — the app
//! applies the resulting placement to its GPUI element.

/// Distance kept from the viewport edges, in px.
pub const VIEWPORT_PADDING: f64 = 8.0;
/// Minimum usable menu height, in px.
pub const MIN_MENU_HEIGHT: i64 = 96;

/// The menu's measured bounds (`getBoundingClientRect`), measured at its CSS
/// anchor (i.e. *without* any previous horizontal shift).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuRect {
    /// Top edge.
    pub top: f64,
    /// Left edge; `None` (or non-finite) disables horizontal clamping.
    pub left: Option<f64>,
    /// Right edge; `None` (or non-finite) disables horizontal clamping.
    pub right: Option<f64>,
}

/// Where to draw the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuPlacement {
    /// Max height in px (`floor(viewport - top - padding)`, at least
    /// [`MIN_MENU_HEIGHT`]).
    pub max_height: i64,
    /// Horizontal shift in px (`Math.round`), `None` when no shift is needed
    /// (JS leaves `transform` empty).
    pub shift_x: Option<i64>,
}

/// Result of [`toggle_menu`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuToggle {
    /// Whether the menu is now visible.
    pub visible: bool,
    /// Placement to apply when it became visible; `None` when hidden.
    pub placement: Option<MenuPlacement>,
}

/// Max height so the menu fits between its top edge and the viewport bottom
/// minus `padding` (JS `menuMaxHeight`, which returns `"<n>px"`).
pub fn menu_max_height(top: f64, viewport_height: f64, padding: f64) -> i64 {
    todo!()
}

/// Max height plus a horizontal shift that brings an overhanging menu back
/// inside `[padding, viewport_width - padding]` (left edge wins).
pub fn clamp_menu_to_viewport(rect: &MenuRect, viewport_height: f64, viewport_width: f64) -> MenuPlacement {
    todo!()
}

/// Show a hidden menu (computing its placement) or hide it; `force_hidden`
/// only hides. Returns whether it is now visible.
pub fn toggle_menu(
    currently_hidden: bool,
    force_hidden: bool,
    rect: &MenuRect,
    viewport_height: f64,
    viewport_width: f64,
) -> MenuToggle {
    todo!()
}
