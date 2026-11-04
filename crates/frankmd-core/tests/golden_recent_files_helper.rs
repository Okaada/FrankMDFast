//! Golden conformance: `tests/fixtures/golden/recent_files_helper.json`
//! (`RecentFilesHelper#recent_file_opened_ago` / `#recent_file_opened_at`).
//!
//! Fixture notes: `now` is frozen by `travel_to`, which truncates usec, so the
//! effective now is the fixture `now` with sub-seconds dropped; `opened_at =
//! now - seconds_ago`. Labels are localized for every shipped locale.
#![allow(missing_docs, clippy::unwrap_used)]

use frankmd_core::recent::{opened_ago, parse_opened_at};
use frankmd_i18n::{Catalog, Locale};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, golden};
use jiff::{SignedDuration, Timestamp, tz::TimeZone};
use std::collections::HashMap;

fn locale(code: &str) -> Locale {
    match code {
        "en" => Locale::En,
        "pt-BR" => Locale::PtBr,
        "pt-PT" => Locale::PtPt,
        "es" => Locale::Es,
        "he" => Locale::He,
        "ja" => Locale::Ja,
        "ko" => Locale::Ko,
        other => panic!("unknown locale {other}"),
    }
}

#[test]
fn golden_recent_file_opened_ago() {
    let g = golden("recent_files_helper");
    let mut catalogs: HashMap<String, Catalog> = HashMap::new();
    assert_cases(
        "recent_files_helper",
        g.cases_for("recent_file_opened_ago"),
        |c| {
            let now: Timestamp = c.input["now"].as_str().unwrap().parse().unwrap();
            // travel_to truncates sub-second precision.
            let now = Timestamp::from_second(now.as_second()).unwrap();
            // seconds_ago may be fractional (0.4 s, 1.5 s …).
            let ago = c.input["seconds_ago"].as_f64().unwrap();
            let opened = now - SignedDuration::from_secs_f64(ago);
            let code = c.input["locale"].as_str().unwrap().to_owned();
            let cat = catalogs
                .entry(code.clone())
                .or_insert_with(|| Catalog::load(locale(&code)).unwrap());
            let got = match opened_ago(now, opened).i18n() {
                (key, Some(n)) => cat.t_count(key, n, &[]),
                (key, None) => cat.t(key, &[]),
            };
            let want = c.expected.as_str().unwrap();
            if got == want {
                Ok(())
            } else {
                Err(format!("expected {want:?}, got {got:?}"))
            }
        },
    );
}

fn fmt6(ts: Timestamp, offset: jiff::tz::Offset) -> String {
    let z = ts.to_zoned(TimeZone::fixed(offset));
    z.strftime("%Y-%m-%dT%H:%M:%S%.6f").to_string()
}

#[test]
fn golden_recent_file_opened_at() {
    let g = golden("recent_files_helper");
    assert_cases(
        "recent_files_helper",
        g.cases_for("recent_file_opened_at"),
        |c| {
            let input = c.input.as_str().unwrap();
            let got = parse_opened_at(input).map(|p| {
                // Ruby renders a UTC-parsed ("Z") time with "Z" and an explicit
                // "+00:00" offset as "+00:00"; compare the instant + offset, and the
                // iso8601 text up to the zone designator.
                json!({
                    "local": fmt6(p.timestamp, p.offset),
                    "utc": format!("{}Z", fmt6(p.timestamp, jiff::tz::Offset::UTC)),
                    "utc_offset": p.offset.seconds(),
                })
            });
            let want = match &c.expected {
                Value::Null => None,
                e => {
                    let iso = e["iso8601"].as_str().unwrap();
                    // strip trailing "Z" or "+hh:mm"/"-hh:mm"
                    let local = iso
                        .strip_suffix('Z')
                        .unwrap_or_else(|| &iso[..iso.len() - 6]);
                    Some(json!({"local": local, "utc": e["utc"], "utc_offset": e["utc_offset"]}))
                }
            };
            if got == want {
                Ok(())
            } else {
                Err(format!("expected {want:?}, got {got:?}"))
            }
        },
    );
}
