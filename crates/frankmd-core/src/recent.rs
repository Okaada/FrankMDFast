//! Recent files (**[COMPAT]** stored as JSON in `.fed` `recent_files`, so they
//! sync with the vault across devices, issue #245), plus the Home panel's
//! time formatting (`app/helpers/recent_files_helper.rb`).
//!
//! Golden: `recent_files.json` (driven through `frankmd_fs::RecentFilesService`),
//! `recent_files_helper.json`.

use crate::RelPath;
use serde::{Deserialize, Serialize};

/// **[COMPAT]** `RecentFilesService::LIMIT`.
pub const LIMIT: usize = 10;

/// **[COMPAT]** inputs are truncated to this many items before normalizing.
pub const MAX_INPUT: usize = 100;

/// **[COMPAT]** max path length in bytes accepted by `valid_path?`.
pub const MAX_PATH_BYTES: usize = 4096;

/// One entry of `recent_files` (`{path, opened_at}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentFile {
    /// Vault-relative note path.
    pub path: String,
    /// ISO-8601 timestamp, as written by Rails (`2026-01-30T12:34:56.789012Z`).
    pub opened_at: String,
}

/// Pure operations over the recent-files list (the caller persists it
/// through `frankmd-config` under the config lock).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentFiles(pub Vec<RecentFile>);

impl RecentFiles {
    /// **[COMPAT]** `normalize(value)`: non-array → empty; take the first
    /// [`MAX_INPUT`]; keep objects with a [`is_valid_path`] `path`, a string
    /// `opened_at` and `present(path)`; dedupe by path keeping the first; take
    /// [`LIMIT`]; drop extra keys.
    pub fn normalize(value: &serde_json::Value, present: impl Fn(&str) -> bool) -> Self {
        todo!("recent_files_service.rb#normalize")
    }

    /// Record that `path` was opened at `opened_at` (prepend, dedupe, cap).
    pub fn touch(&mut self, path: &RelPath, opened_at: &str) {
        todo!("backend.md §6")
    }

    /// **[COMPAT]** `remap(old, new)` on the *raw* stored value (the old path
    /// no longer exists, so this runs before normalization): entries equal to
    /// `old` or under `old + "/"` get the prefix replaced; non-objects and
    /// entries without a string path are dropped; then [`RecentFiles::normalize`].
    pub fn remap_raw(value: &serde_json::Value, from: &str, to: &str, present: impl Fn(&str) -> bool) -> Self {
        todo!("recent_files_service.rb#remap")
    }

    /// Apply a rename/move (file or folder), segment-wise.
    pub fn rename(&mut self, from: &RelPath, to: &RelPath) {
        todo!()
    }

    /// Drop entries equal to `path` or under `path + "/"`.
    pub fn remove(&mut self, path: &RelPath) {
        todo!()
    }

    /// **[COMPAT]** the compact JSON persisted in `.fed`, keys in Rails order:
    /// `[{"path":"a.md","opened_at":"2026-…Z"}]`.
    pub fn to_json(&self) -> String {
        todo!()
    }
}

/// **[COMPAT]** `valid_path?` (lexical half): a string of at most
/// [`MAX_PATH_BYTES`] bytes ending in `.md`, no `\` or NUL, no empty segment
/// and no segment starting with `.` (rules out `/x`, `a//b`, `.hidden.md`,
/// `..`). The caller additionally applies symlink-aware containment.
pub fn is_valid_path(path: &str) -> bool {
    todo!("recent_files_service.rb#valid_path?")
}

/// **[COMPAT]** `Time.current.utc.iso8601(6)`: UTC, 6 fractional digits, `Z`.
pub fn format_opened_at(ts: jiff::Timestamp) -> String {
    todo!()
}

/// A parsed `opened_at` keeping the original offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenedAt {
    /// The instant.
    pub timestamp: jiff::Timestamp,
    /// The offset it was written with (`utc_offset`).
    pub offset: jiff::tz::Offset,
}

/// **[COMPAT]** `recent_file_opened_at(value)` = Ruby `Time.iso8601(value)`;
/// `None` on anything it would reject (→ "Unknown time" in the UI).
pub fn parse_opened_at(value: &str) -> Option<OpenedAt> {
    todo!("recent_files_helper.rb")
}

/// Relative-time bucket for the Home list (`home.opened_ago.*` i18n keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenedAgo {
    /// 0 s (or in the future): `home.opened_ago.just_now`.
    JustNow,
    /// 1..59 s: `home.opened_ago.seconds` (count).
    Seconds(i64),
    /// 60..3599 s: `home.opened_ago.minutes` (count = s / 60).
    Minutes(i64),
    /// 1 h..<1 day: `home.opened_ago.hours`.
    Hours(i64),
    /// 1 day..=1 week: `home.opened_ago.days`.
    Days(i64),
    /// More than 604800 s: `home.opened_ago.over_a_week`.
    OverAWeek,
}

impl OpenedAgo {
    /// i18n key and optional `count` for this bucket.
    pub fn i18n(self) -> (&'static str, Option<i64>) {
        match self {
            Self::JustNow => ("home.opened_ago.just_now", None),
            Self::Seconds(n) => ("home.opened_ago.seconds", Some(n)),
            Self::Minutes(n) => ("home.opened_ago.minutes", Some(n)),
            Self::Hours(n) => ("home.opened_ago.hours", Some(n)),
            Self::Days(n) => ("home.opened_ago.days", Some(n)),
            Self::OverAWeek => ("home.opened_ago.over_a_week", None),
        }
    }
}

/// **[COMPAT]** `recent_file_opened_ago`: `seconds = max(0, (now - opened_at)
/// truncated toward zero)`, single unit, exact boundaries.
pub fn opened_ago(now: jiff::Timestamp, opened_at: jiff::Timestamp) -> OpenedAgo {
    todo!("recent_files_helper.rb")
}
