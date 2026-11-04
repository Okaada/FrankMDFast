//! Editor gutter line-number mode. **[COMPAT]** `app/javascript/lib/line_numbers.js`;
//! the numeric values are what `.fed` `editor_line_numbers` persists. Golden
//! `js_line_numbers.json`.
//!
//! Lives here (next to the line mapping) only because it was assigned with the
//! preview/line-mapping port; the editor/workspace may re-export or move it.

/// `LINE_NUMBER_MODES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineNumberMode {
    /// 0.
    Off,
    /// 1 (FrankMD's default fallback).
    #[default]
    Absolute,
    /// 2.
    Relative,
}

impl LineNumberMode {
    /// Persisted numeric value (0/1/2).
    pub fn value(self) -> u8 {
        todo!()
    }

    /// Inverse of [`Self::value`].
    pub fn from_value(value: i64) -> Option<Self> {
        todo!()
    }

    /// `nextLineNumberMode`: Off → Absolute → Relative → Off.
    pub fn next(self) -> Self {
        todo!()
    }
}

/// `normalizeLineNumberMode(value, fallback)` over the value's string form
/// (`None` = `undefined`/missing): JS `parseInt(value, 10)` semantics (leading
/// whitespace skipped, optional sign, leading digits only: `"2px"` → 2,
/// `" 1"` → 1, `"1.9"` → 1); anything that does not parse to 0/1/2 → `fallback`.
pub fn normalize_line_number_mode(value: Option<&str>, fallback: LineNumberMode) -> LineNumberMode {
    todo!()
}
