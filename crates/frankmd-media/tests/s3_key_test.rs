//! Port of `test/javascript/lib/s3_key.test.js` (3 tests).

use frankmd_media::ctx::FakeClock; // mechanical: canonical frankmd-core clock
use frankmd_media::s3_key::{default_s3_key, default_s3_key_now, default_s3_prefix, default_s3_prefix_now};
use jiff::civil::date;

// port: test/javascript/lib/s3_key.test.js:9 "defaultS3Prefix builds frankmd/YYYY/MM with a zero-padded month"
#[test]
fn default_prefix_zero_pads_month() {
    assert_eq!(default_s3_prefix(date(2026, 7, 9)), "frankmd/2026/07");
}

// port: test/javascript/lib/s3_key.test.js:13 "defaultS3Key appends the filename to the prefix"
#[test]
fn default_key_appends_filename() {
    assert_eq!(default_s3_key("photo.png", date(2026, 7, 9)), "frankmd/2026/07/photo.png");
}

// port: test/javascript/lib/s3_key.test.js:17 "defaults to the current date when none is given"
#[test]
fn defaults_to_current_date() {
    let clock = FakeClock::new("2031-11-05T10:00:00+00:00[UTC]".parse().expect("zoned"));
    assert_eq!(default_s3_prefix_now(&clock), "frankmd/2031/11");
    assert_eq!(default_s3_key_now("a.png", &clock), "frankmd/2031/11/a.png");
    let sys = frankmd_media::ctx::SystemClock;
    let p = default_s3_prefix_now(&sys);
    assert!(p.len() == "frankmd/2026/07".len() && p.starts_with("frankmd/"), "{p}");
}
