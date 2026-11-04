//! Document statistics for the stats panel (port of `lib/stats_utils.js`).

use super::{is_js_whitespace, utf16_len};

/// Statistics of one document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DocStats {
    /// Number of maximal non-whitespace runs (JS `/\S+/g`, Unicode `\s`).
    pub word_count: usize,
    /// **[COMPAT]** JS `text.length`: UTF-16 code units, *not* chars or bytes
    /// (an emoji like 👋 counts 2). Kept so the panel shows the same numbers.
    pub char_count_utf16: usize,
    /// UTF-8 byte length (`TextEncoder`), i.e. the size on disk.
    pub byte_size: usize,
    /// `ceil(word_count / 200)`; 0 for an empty document.
    pub read_time_minutes: usize,
}

/// Words per minute used for the read-time estimate.
pub const WORDS_PER_MINUTE: usize = 200;

/// Compute stats. `None` (JS `null`/`undefined`) and `""` give all zeros.
pub fn calculate_stats(text: Option<&str>) -> DocStats {
    let Some(text) = text.filter(|t| !t.is_empty()) else {
        return DocStats::default();
    };
    let word_count = text
        .split(is_js_whitespace)
        .filter(|word| !word.is_empty())
        .count();
    DocStats {
        word_count,
        char_count_utf16: utf16_len(text),
        byte_size: text.len(),
        read_time_minutes: word_count.div_ceil(WORDS_PER_MINUTE),
    }
}

/// `"< 1 min"` for `minutes <= 1`, else `"<minutes> min"` with the number
/// formatted like JS `Number#toString` (`1.5` → `"1.5 min"`, `2.0` → `"2 min"`).
pub fn format_read_time(minutes: f64) -> String {
    if minutes <= 1.0 {
        return "< 1 min".to_owned();
    }
    if minutes.is_infinite() {
        return "Infinity min".to_owned();
    }
    // Rust's `Display` for f64 prints the shortest round-trip form without a
    // trailing `.0`, like JS `Number#toString` for these magnitudes.
    format!("{minutes} min")
}

/// Human-readable size: `"0 B"`, `"1023 B"`, `"1.5 KB"` (one decimal when
/// the scaled size is `< 10`, else none — JS `toFixed`, so `1048575` →
/// `"1024 KB"`), `"1.0 MB"`, `"1.0 GB"`.
///
/// Deliberate deviation: the JS unit table stops at GB and prints
/// `"1.0 undefined"` for ≥ 1 TiB; Rust continues with `TB`, `PB`.
/// Negative/fractional inputs (JS `"NaN undefined"`) are unrepresentable.
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    // `Math.floor(Math.log(bytes) / Math.log(1024))`, computed exactly.
    let exponent = bytes.ilog2() / 10;
    let divisor = 1u128 << (10 * exponent);
    let bytes = u128::from(bytes);
    // JS `toFixed` rounds the exact quotient half-up; do the same in integers.
    let rounded = |scale: u128| (2 * bytes * scale + divisor) / (2 * divisor);
    let unit = UNITS[exponent as usize];
    if bytes < 10 * divisor {
        let tenths = rounded(10);
        format!("{}.{} {unit}", tenths / 10, tenths % 10)
    } else {
        format!("{} {unit}", rounded(1))
    }
}
