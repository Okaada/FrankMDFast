//! Document statistics for the stats panel (port of `lib/stats_utils.js`).

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
    todo!()
}

/// `"< 1 min"` for `minutes <= 1`, else `"<minutes> min"` with the number
/// formatted like JS `Number#toString` (`1.5` → `"1.5 min"`, `2.0` → `"2 min"`).
pub fn format_read_time(minutes: f64) -> String {
    todo!()
}

/// Human-readable size: `"0 B"`, `"1023 B"`, `"1.5 KB"` (one decimal when
/// the scaled size is `< 10`, else none — JS `toFixed`, so `1048575` →
/// `"1024 KB"`), `"1.0 MB"`, `"1.0 GB"`.
///
/// Deliberate deviation: the JS unit table stops at GB and prints
/// `"1.0 undefined"` for ≥ 1 TiB; Rust continues with `TB`, `PB`.
/// Negative/fractional inputs (JS `"NaN undefined"`) are unrepresentable.
pub fn format_file_size(bytes: u64) -> String {
    todo!()
}
