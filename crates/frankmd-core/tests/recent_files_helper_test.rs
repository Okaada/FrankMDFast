//! Port of `test/helpers/recent_files_helper_test.rb` (Home panel time labels).
#![allow(missing_docs, clippy::unwrap_used)]

use frankmd_core::recent::{OpenedAgo, opened_ago, parse_opened_at};
use frankmd_i18n::{Catalog, Locale};
use jiff::{SignedDuration, Timestamp};

fn label(cat: &Catalog, ago: OpenedAgo) -> String {
    match ago.i18n() {
        (key, Some(n)) => cat.t_count(key, n, &[]),
        (key, None) => cat.t(key, &[]),
    }
}

fn now() -> Timestamp {
    "2026-01-30T12:34:56Z".parse().unwrap()
}

// port: test/helpers/recent_files_helper_test.rb:8 "opening times use one unit with exact unit and week boundaries"
#[test]
fn opening_times_use_one_unit_with_exact_unit_and_week_boundaries() {
    let cat = Catalog::load(Locale::En).unwrap();
    let now = now();
    let table: &[(i64, &str)] = &[
        (0, "Just now"),
        (1, "1 second ago"),
        (59, "59 seconds ago"),
        (60, "1 minute ago"),
        (119, "1 minute ago"),
        (600, "10 minutes ago"),
        (3599, "59 minutes ago"),
        (3600, "1 hour ago"),
        (7200, "2 hours ago"),
        (86399, "23 hours ago"),
        (86400, "1 day ago"),
        (172800, "2 days ago"),
        (604800, "7 days ago"),
        (604801, "Over a week ago"),
    ];
    for (seconds, expected) in table {
        let opened = now - SignedDuration::from_secs(*seconds);
        assert_eq!(
            label(&cat, opened_ago(now, opened)),
            *expected,
            "Age: {seconds} seconds"
        );
    }
    // A future time (1 minute from now) is "Just now".
    let future = now + SignedDuration::from_secs(60);
    assert_eq!(opened_ago(now, future), OpenedAgo::JustNow);
    assert_eq!(label(&cat, opened_ago(now, future)), "Just now");
}

// port: test/helpers/recent_files_helper_test.rb:33 "opening times use the current locale"
#[test]
fn opening_times_use_the_current_locale() {
    let cat = Catalog::load(Locale::PtBr).unwrap();
    let now = now();
    let day = 86_400;
    assert_eq!(
        label(&cat, opened_ago(now, now - SignedDuration::from_secs(day))),
        "há 1 dia"
    );
    assert_eq!(
        label(
            &cat,
            opened_ago(now, now - SignedDuration::from_secs(2 * day))
        ),
        "há 2 dias"
    );
    assert_eq!(
        label(
            &cat,
            opened_ago(now, now - SignedDuration::from_secs(8 * day))
        ),
        "Há mais de uma semana"
    );
}

// port: test/helpers/recent_files_helper_test.rb:43 "opening timestamps preserve timezone offsets and tolerate malformed history"
#[test]
fn opening_timestamps_preserve_timezone_offsets_and_tolerate_malformed_history() {
    let parsed = parse_opened_at("2026-10-26T09:30:00-03:00").unwrap();
    assert_eq!(
        parsed.timestamp,
        "2026-10-26T12:30:00Z".parse::<Timestamp>().unwrap()
    );
    assert_eq!(parsed.offset.seconds(), -10800);
    assert_eq!(parse_opened_at("not a timestamp"), None);
    assert_eq!(parse_opened_at(""), None);
}
