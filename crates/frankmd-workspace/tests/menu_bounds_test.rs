//! Port of test/javascript/lib/menu_bounds.test.js. The DOM element becomes
//! a [`MenuRect`] (its measured bounds) plus a `hidden` flag; the CSS strings
//! the JS writes are rebuilt from the returned placement.

use frankmd_workspace::menu_bounds::{
    MenuPlacement, MenuRect, VIEWPORT_PADDING, clamp_menu_to_viewport, menu_max_height, toggle_menu,
};

fn top_only(top: f64) -> MenuRect {
    MenuRect {
        top,
        left: None,
        right: None,
    }
}

fn transform(p: &MenuPlacement) -> String {
    p.shift_x
        .map(|x| format!("translateX({x}px)"))
        .unwrap_or_default()
}

// port: test/javascript/lib/menu_bounds.test.js:16 "bounds the menu to the remaining viewport below its top edge"
#[test]
fn max_height_bounds_to_remaining_viewport() {
    assert_eq!(
        menu_max_height(400.0, 900.0, VIEWPORT_PADDING),
        900 - 400 - 8
    );
}

// port: test/javascript/lib/menu_bounds.test.js:21 "keeps a usable minimum when opened near the bottom"
#[test]
fn max_height_keeps_usable_minimum() {
    assert_eq!(menu_max_height(890.0, 900.0, VIEWPORT_PADDING), 96);
}

// port: test/javascript/lib/menu_bounds.test.js:26 "honors a custom padding"
#[test]
fn max_height_honors_custom_padding() {
    assert_eq!(menu_max_height(100.0, 900.0, 20.0), 780);
}

// port: test/javascript/lib/menu_bounds.test.js:35 "sets an inline max-height sized to the viewport"
#[test]
fn clamp_sets_max_height_sized_to_viewport() {
    let p = clamp_menu_to_viewport(&top_only(300.0), 900.0, 1024.0);
    assert_eq!(p.max_height, 592);
    assert_eq!(p.shift_x, None, "no left/right measured → no shift");
}

// port: test/javascript/lib/menu_bounds.test.js:42 "ignores missing menu elements"
// SKIPPED: a null element cannot be passed to the Rust API (no Option at this layer).

// port: test/javascript/lib/menu_bounds.test.js:47 "shifts an overhanging menu into a 320px viewport" (it.each ×2)
#[test]
fn clamp_shifts_overhanging_menu_into_320px_viewport() {
    for (left, right, expected) in [
        (177.0, 449.0, "translateX(-137px)"),
        (-20.0, 252.0, "translateX(28px)"),
    ] {
        let rect = MenuRect {
            top: 80.0,
            left: Some(left),
            right: Some(right),
        };
        let p = clamp_menu_to_viewport(&rect, 720.0, 320.0);
        assert_eq!(transform(&p), expected, "left={left} right={right}");
    }
}

// port: test/javascript/lib/menu_bounds.test.js:56 "resets an old horizontal shift before measuring after a resize"
// The pure API measures from the CSS anchor (no previous shift is an input), so
// the contract reduces to: an in-bounds rect yields no transform at all.
#[test]
fn clamp_in_bounds_after_resize_has_no_shift() {
    let rect = MenuRect {
        top: 80.0,
        left: Some(8.0),
        right: Some(280.0),
    };
    let p = clamp_menu_to_viewport(&rect, 720.0, 320.0);
    assert_eq!(transform(&p), "");
}

// port: test/javascript/lib/menu_bounds.test.js:71 "shows a hidden menu and applies the viewport clamp"
#[test]
fn toggle_shows_hidden_menu_and_clamps() {
    let t = toggle_menu(true, false, &top_only(500.0), 900.0, 1024.0);
    assert!(t.visible);
    assert_eq!(t.placement.map(|p| p.max_height), Some(900 - 500 - 8));
}

// port: test/javascript/lib/menu_bounds.test.js:81 "hides a visible menu"
#[test]
fn toggle_hides_visible_menu() {
    let t = toggle_menu(false, true, &top_only(0.0), 768.0, 1024.0);
    assert!(!t.visible);
    assert_eq!(t.placement, None);
}
