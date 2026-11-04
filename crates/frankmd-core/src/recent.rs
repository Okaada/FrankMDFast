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
        let Some(items) = value.as_array() else {
            return Self::default();
        };
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for item in items.iter().take(MAX_INPUT) {
            let Some(path) = item.get("path").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(opened_at) = item.get("opened_at").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !item.is_object() || !is_valid_path(path) || !present(path) {
                continue;
            }
            if seen.insert(path.to_owned()) {
                out.push(RecentFile {
                    path: path.to_owned(),
                    opened_at: opened_at.to_owned(),
                });
            }
        }
        out.truncate(LIMIT);
        Self(out)
    }

    /// Record that `path` was opened at `opened_at` (prepend, dedupe, cap).
    pub fn touch(&mut self, path: &RelPath, opened_at: &str) {
        let path = path.as_str();
        self.0.retain(|item| item.path != path);
        self.0.insert(
            0,
            RecentFile {
                path: path.to_owned(),
                opened_at: opened_at.to_owned(),
            },
        );
        self.0.truncate(LIMIT);
    }

    /// **[COMPAT]** `remap(old, new)` on the *raw* stored value (the old path
    /// no longer exists, so this runs before normalization): entries equal to
    /// `old` or under `old + "/"` get the prefix replaced; non-objects and
    /// entries without a string path are dropped; then [`RecentFiles::normalize`].
    pub fn remap_raw(
        value: &serde_json::Value,
        from: &str,
        to: &str,
        present: impl Fn(&str) -> bool,
    ) -> Self {
        let Some(items) = value.as_array() else {
            return Self::default();
        };
        let remapped: Vec<serde_json::Value> = items
            .iter()
            .take(MAX_INPUT)
            .filter_map(|item| {
                let path = item.as_object()?.get("path")?.as_str()?;
                let mut candidate = item.clone();
                if let Some(new_path) = rebase(path, from, to) {
                    candidate["path"] = serde_json::Value::String(new_path);
                }
                Some(candidate)
            })
            .collect();
        Self::normalize(&serde_json::Value::Array(remapped), present)
    }

    /// Apply a rename/move (file or folder), segment-wise.
    pub fn rename(&mut self, from: &RelPath, to: &RelPath) {
        for item in &mut self.0 {
            if let Some(new_path) = rebase(&item.path, from.as_str(), to.as_str()) {
                item.path = new_path;
            }
        }
    }

    /// Drop entries equal to `path` or under `path + "/"`.
    pub fn remove(&mut self, path: &RelPath) {
        self.0.retain(|item| !scoped(&item.path, path.as_str()));
    }

    /// **[COMPAT]** the compact JSON persisted in `.fed`, keys in Rails order:
    /// `[{"path":"a.md","opened_at":"2026-…Z"}]`.
    pub fn to_json(&self) -> String {
        // Serializing plain strings into a Vec cannot fail; fall back to `[]`
        // rather than panicking.
        serde_json::to_string(&self.0).unwrap_or_else(|_| "[]".to_owned())
    }
}

/// **[COMPAT]** `valid_path?` (lexical half): a string of at most
/// [`MAX_PATH_BYTES`] bytes ending in `.md`, no `\` or NUL, no empty segment
/// and no segment starting with `.` (rules out `/x`, `a//b`, `.hidden.md`,
/// `..`). The caller additionally applies symlink-aware containment.
pub fn is_valid_path(path: &str) -> bool {
    path.len() <= MAX_PATH_BYTES
        && path.ends_with(".md")
        && !path.contains('\\')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.'))
}

/// **[COMPAT]** `scoped?(candidate, path)`: equal, or below `path + "/"`.
fn scoped(candidate: &str, path: &str) -> bool {
    candidate == path
        || candidate
            .strip_prefix(path)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// `"#{new}#{candidate.delete_prefix(old)}"` when `candidate` is scoped under `old`.
fn rebase(candidate: &str, old: &str, new: &str) -> Option<String> {
    if !scoped(candidate, old) {
        return None;
    }
    let rest = candidate.strip_prefix(old).unwrap_or(candidate);
    Some(format!("{new}{rest}"))
}

/// **[COMPAT]** `Time.current.utc.iso8601(6)`: UTC, 6 fractional digits, `Z`.
pub fn format_opened_at(ts: jiff::Timestamp) -> String {
    let utc = ts.to_zoned(jiff::tz::TimeZone::UTC);
    let micros = utc.subsec_nanosecond() / 1_000;
    format!("{}.{micros:06}Z", utc.strftime("%Y-%m-%dT%H:%M:%S"))
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
    iso8601::parse(value)
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
    const MINUTE: i64 = 60;
    const HOUR: i64 = 3_600;
    const DAY: i64 = 86_400;
    const WEEK: i64 = 604_800;
    let seconds = now.duration_since(opened_at).as_secs().max(0);
    match seconds {
        0 => OpenedAgo::JustNow,
        s if s > WEEK => OpenedAgo::OverAWeek,
        s if s < MINUTE => OpenedAgo::Seconds(s),
        s if s < HOUR => OpenedAgo::Minutes(s / MINUTE),
        s if s < DAY => OpenedAgo::Hours(s / HOUR),
        s => OpenedAgo::Days(s / DAY),
    }
}

/// Ruby `Time.iso8601` / `Time.xmlschema` (`lib/time.rb`):
/// `\A\s*(-?\d+)-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)(\.\d+)?(Z|[+-]\d\d(?::?\d\d)?)?\s*\z`
/// (case-insensitive), then `Time.utc`/`Time.new` range checks with their
/// rollovers (Feb 30 → Mar 2, `24:00:00` → next day, `:60` → next minute).
mod iso8601 {
    use super::OpenedAt;
    use jiff::SignedDuration;
    use jiff::tz::Offset;

    /// A tiny cursor over the ASCII grammar.
    struct Cursor<'a> {
        bytes: &'a [u8],
        pos: usize,
    }

    impl Cursor<'_> {
        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.pos).copied()
        }

        fn eat(&mut self, want: u8) -> bool {
            if self.peek().is_some_and(|b| b.eq_ignore_ascii_case(&want)) {
                self.pos += 1;
                true
            } else {
                false
            }
        }

        /// Exactly two ASCII digits.
        fn two_digits(&mut self) -> Option<i64> {
            let d = self.bytes.get(self.pos..self.pos + 2)?;
            if !d.iter().all(u8::is_ascii_digit) {
                return None;
            }
            self.pos += 2;
            Some(i64::from(d[0] - b'0') * 10 + i64::from(d[1] - b'0'))
        }

        /// One or more ASCII digits, returned as the raw slice.
        fn digits(&mut self) -> Option<&[u8]> {
            let start = self.pos;
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
            (self.pos > start).then(|| &self.bytes[start..self.pos])
        }

        fn skip_space(&mut self) {
            while self
                .peek()
                .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'))
            {
                self.pos += 1;
            }
        }
    }

    fn number(digits: &[u8]) -> Option<i64> {
        std::str::from_utf8(digits).ok()?.parse().ok()
    }

    pub(super) fn parse(value: &str) -> Option<OpenedAt> {
        let mut c = Cursor {
            bytes: value.as_bytes(),
            pos: 0,
        };
        c.skip_space();
        let negative = c.eat(b'-');
        let year = number(c.digits()?)?;
        let year = if negative { -year } else { year };
        let fields = (|| {
            if !c.eat(b'-') {
                return None;
            }
            let mon = c.two_digits()?;
            if !c.eat(b'-') {
                return None;
            }
            let day = c.two_digits()?;
            if !c.eat(b'T') {
                return None;
            }
            let hour = c.two_digits()?;
            if !c.eat(b':') {
                return None;
            }
            let min = c.two_digits()?;
            if !c.eat(b':') {
                return None;
            }
            let sec = c.two_digits()?;
            Some((mon, day, hour, min, sec))
        })();
        let (mon, day, hour, min, sec) = fields?;

        let mut nanos = 0i64;
        if c.peek() == Some(b'.') {
            c.pos += 1;
            let frac = c.digits()?;
            for (i, d) in frac.iter().take(9).enumerate() {
                let exp = u32::try_from(8 - i).ok()?;
                nanos += i64::from(d - b'0') * 10i64.pow(exp);
            }
        }

        let offset_seconds = if c.eat(b'Z') {
            0
        } else if let Some(sign @ (b'+' | b'-')) = c.peek() {
            c.pos += 1;
            let hours = c.two_digits()?;
            let save = c.pos;
            let colon = c.eat(b':');
            let minutes = match c.two_digits() {
                Some(m) => m,
                None if colon => return None,
                None => {
                    c.pos = save;
                    0
                }
            };
            let total = hours * 3_600 + minutes * 60;
            if sign == b'-' { -total } else { total }
        } else {
            0
        };
        c.skip_space();
        if c.pos != c.bytes.len() {
            return None;
        }

        // `Time.utc` / `Time.new` argument checks.
        if !(1..=12).contains(&mon) || !(1..=31).contains(&day) || !(0..=59).contains(&min) {
            return None;
        }
        if !(0..=24).contains(&hour) || !(0..=60).contains(&sec) {
            return None;
        }
        if hour == 24 && (min != 0 || sec != 0) {
            return None;
        }
        if offset_seconds.abs() >= 86_400 {
            return None;
        }

        let first =
            jiff::civil::Date::new(i16::try_from(year).ok()?, i8::try_from(mon).ok()?, 1).ok()?;
        let date = first.checked_add(jiff::Span::new().days(day - 1)).ok()?;
        let midnight = date.to_zoned(jiff::tz::TimeZone::UTC).ok()?.timestamp();
        let wall = SignedDuration::from_secs(hour * 3_600 + min * 60 + sec)
            + SignedDuration::from_nanos(nanos);
        let timestamp = midnight
            .checked_add(wall)
            .ok()?
            .checked_sub(SignedDuration::from_secs(offset_seconds))
            .ok()?;
        let offset = Offset::from_seconds(i32::try_from(offset_seconds).ok()?).ok()?;
        Some(OpenedAt { timestamp, offset })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::{SignedDuration, Timestamp};

    #[test]
    fn opened_ago_buckets_have_exact_boundaries() {
        let now: Timestamp = "2026-01-30T12:34:56Z".parse().unwrap_or_default();
        let table = [
            (0, OpenedAgo::JustNow),
            (59, OpenedAgo::Seconds(59)),
            (60, OpenedAgo::Minutes(1)),
            (3599, OpenedAgo::Minutes(59)),
            (3600, OpenedAgo::Hours(1)),
            (86399, OpenedAgo::Hours(23)),
            (86400, OpenedAgo::Days(1)),
            (604_800, OpenedAgo::Days(7)),
            (604_801, OpenedAgo::OverAWeek),
            (-60, OpenedAgo::JustNow),
        ];
        for (secs, want) in table {
            assert_eq!(
                opened_ago(now, now - SignedDuration::from_secs(secs)),
                want,
                "{secs}"
            );
        }
        // Fractional ages truncate toward zero.
        assert_eq!(
            opened_ago(now, now - SignedDuration::from_millis(1500)),
            OpenedAgo::Seconds(1)
        );
        assert_eq!(
            opened_ago(now, now - SignedDuration::from_millis(400)),
            OpenedAgo::JustNow
        );
    }

    #[test]
    fn opened_at_round_trips_with_six_fraction_digits() {
        let ts: Timestamp = "2026-01-30T12:34:56.789012345Z".parse().unwrap_or_default();
        assert_eq!(format_opened_at(ts), "2026-01-30T12:34:56.789012Z");
    }

    #[test]
    fn remap_and_remove_are_segment_wise() {
        let raw = serde_json::json!([
            {"path": "a/x.md", "opened_at": "t"},
            {"path": "ab/y.md", "opened_at": "t"},
            {"path": 42, "opened_at": "t"},
        ]);
        let remapped = RecentFiles::remap_raw(&raw, "a", "b", |_| true);
        let paths: Vec<&str> = remapped.0.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["b/x.md", "ab/y.md"]);
        assert_eq!(
            remapped.to_json(),
            r#"[{"path":"b/x.md","opened_at":"t"},{"path":"ab/y.md","opened_at":"t"}]"#
        );
    }
}
