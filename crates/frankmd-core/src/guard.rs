//! Content-loss protection ("large deletion" guard, frontend.md B.5).
//!
//! **[COMPAT]** `autosave_controller.js#isLargeDeletion`:
//! ```js
//! if (baseline.length === 0) return false
//! const lostChars = baseline.length - content.length
//! const lostPercent = lostChars / baseline.length
//! return lostPercent > 0.2 && lostChars > 50
//! ```
//! Lengths are JS `.length`, i.e. **UTF-16 code units** (an emoji counts 2).
//! Both comparisons are strict: losing exactly 20 % or exactly 50 units is fine.

use crate::text::utf16_len;

/// Outcome of comparing the last saved content with a pending save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeletionVerdict {
    /// Save normally.
    Ok,
    /// The edit removes a large share of the note; ask before saving.
    LargeDeletion {
        /// UTF-16 code units removed (`baseline.length - content.length`).
        removed_chars: usize,
    },
}

/// Minimum share of the baseline that must be lost (strictly greater).
pub const LOST_FRACTION_THRESHOLD: f64 = 0.2;
/// Minimum UTF-16 units that must be lost (strictly greater).
pub const LOST_CHARS_THRESHOLD: usize = 50;

/// **[COMPAT]** thresholds from the autosave controller. `previous` is the
/// last saved server text; `None` (fresh file, `_lastSavedContent === null`)
/// never warns.
pub fn check_deletion(previous: Option<&str>, next: &str) -> DeletionVerdict {
    let Some(previous) = previous else {
        return DeletionVerdict::Ok;
    };
    let baseline = utf16_len(previous);
    if baseline == 0 {
        return DeletionVerdict::Ok;
    }
    let remaining = utf16_len(next);
    let Some(lost) = baseline.checked_sub(remaining) else {
        return DeletionVerdict::Ok;
    };
    let lost_fraction = lost as f64 / baseline as f64;
    if lost_fraction > LOST_FRACTION_THRESHOLD && lost > LOST_CHARS_THRESHOLD {
        DeletionVerdict::LargeDeletion {
            removed_chars: lost,
        }
    } else {
        DeletionVerdict::Ok
    }
}

/// Boolean form (`isLargeDeletion`).
pub fn is_large_deletion(previous: Option<&str>, next: &str) -> bool {
    matches!(
        check_deletion(previous, next),
        DeletionVerdict::LargeDeletion { .. }
    )
}
