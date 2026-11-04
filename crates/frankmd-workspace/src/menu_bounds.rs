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
    let available = (viewport_height - top - padding).floor();
    // `as` saturates for out-of-range floats, which is the clamp we want.
    (available as i64).max(MIN_MENU_HEIGHT)
}

/// Max height plus a horizontal shift that brings an overhanging menu back
/// inside `[padding, viewport_width - padding]` (left edge wins).
pub fn clamp_menu_to_viewport(rect: &MenuRect, viewport_height: f64, viewport_width: f64) -> MenuPlacement {
    let max_height = menu_max_height(rect.top, viewport_height, VIEWPORT_PADDING);
    let shift_x = match (rect.left, rect.right) {
        (Some(left), Some(right)) if left.is_finite() && right.is_finite() => {
            let shift = if left < VIEWPORT_PADDING {
                VIEWPORT_PADDING - left
            } else if right > viewport_width - VIEWPORT_PADDING {
                viewport_width - VIEWPORT_PADDING - right
            } else {
                0.0
            };
            // **[COMPAT]** the shift is tested before rounding, so 0.3 px
            // still yields `translateX(0px)`.
            (shift != 0.0).then(|| js_round(shift))
        }
        _ => None,
    };
    MenuPlacement { max_height, shift_x }
}

/// JS `Math.round` (halves round towards +∞).
fn js_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
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
    let will_show = !force_hidden && currently_hidden;
    MenuToggle {
        visible: will_show,
        placement: will_show.then(|| clamp_menu_to_viewport(rect, viewport_height, viewport_width)),
    }
}
