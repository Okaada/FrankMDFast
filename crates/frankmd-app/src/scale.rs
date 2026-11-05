//! The UI scale (Settings → Appearance → Interface size).
//!
//! Every design size in this crate goes through [`px`], which multiplies by
//! the scale; text follows through gpui-kit's rem size (`theme.font_size`,
//! set in `bridge::theme`). Sizes measured from the window or the mouse
//! (splitter widths, menu positions) use `gpui_kit::px` and stay unscaled.

use std::sync::atomic::{AtomicU32, Ordering};

/// The steps offered in Settings, in percent.
pub const STEPS: &[i64] = &[80, 90, 100, 110, 120, 135, 150, 175, 200];

/// Default scale in percent.
pub const DEFAULT: u16 = 100;

static PERCENT: AtomicU32 = AtomicU32::new(DEFAULT as u32);

/// Set the scale (clamped to the offered range).
pub fn set(percent: u16) {
    let clamped = percent.clamp(STEPS[0] as u16, STEPS[STEPS.len() - 1] as u16);
    PERCENT.store(u32::from(clamped), Ordering::Relaxed);
}

/// Current scale in percent.
pub fn percent() -> u16 {
    PERCENT.load(Ordering::Relaxed) as u16
}

/// Current scale as a factor (`1.0` = 100%).
pub fn factor() -> f32 {
    f32::from(percent()) / 100.0
}

/// A design size in logical pixels, scaled.
pub fn px(value: f32) -> gpui_kit::Pixels {
    gpui_kit::px(value * factor())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_clamps_to_the_offered_range() {
        set(500);
        assert_eq!(percent(), 200);
        set(10);
        assert_eq!(percent(), 80);
        set(DEFAULT);
        assert!((factor() - 1.0).abs() < f32::EPSILON);
    }
}
