//! **[COMPAT]** port of `app/javascript/lib/s3_key.js`: the default S3 key the
//! picker pre-fills (mirrors `UploadStorage.s3_key`'s `frankmd/YYYY/MM` layout).
//! Dates are *local* calendar dates (JS `getFullYear`/`getMonth`).

/// `PREFIX`.
pub const PREFIX: &str = "frankmd";

/// `defaultS3Prefix(date)` → `"frankmd/2026/07"` (zero-padded month).
pub fn default_s3_prefix(date: jiff::civil::Date) -> String {
    todo!()
}

/// `defaultS3Key(filename, date)` → `"frankmd/2026/07/<filename>"` (filename verbatim).
pub fn default_s3_key(filename: &str, date: jiff::civil::Date) -> String {
    todo!()
}

/// `defaultS3Prefix()` with "now" from the clock (local date).
pub fn default_s3_prefix_now(clock: &dyn crate::ctx::Clock) -> String {
    todo!()
}

/// `defaultS3Key(filename)` with "now" from the clock.
pub fn default_s3_key_now(filename: &str, clock: &dyn crate::ctx::Clock) -> String {
    todo!()
}
