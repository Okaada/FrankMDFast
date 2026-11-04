//! Conformance against `tests/fixtures/golden/js_s3_key.json` (20 cases; TZ=UTC so
//! the `$date` instant's UTC calendar date is the local date).

use frankmd_media::s3_key::{default_s3_key, default_s3_prefix};
use frankmd_testkit::{assert_cases, decode_js, golden};

fn date_arg(v: &frankmd_testkit::serde_json::Value) -> Result<jiff::civil::Date, String> {
    let s = decode_js(v);
    let ts: jiff::Timestamp = s
        .as_str()
        .ok_or("date arg")?
        .parse()
        .map_err(|e| format!("{e}"))?;
    Ok(ts.to_zoned(jiff::tz::TimeZone::UTC).date())
}

#[test]
fn js_s3_key() {
    let g = golden("js_s3_key");
    assert_cases("js_s3_key", &g.cases, |c| {
        let args = c.input.as_array().ok_or("args")?;
        let got = match c.func.as_deref() {
            Some("defaultS3Prefix") => default_s3_prefix(date_arg(&args[0])?),
            Some("defaultS3Key") => {
                let name = decode_js(&args[0]);
                default_s3_key(name.as_str().ok_or("filename")?, date_arg(&args[1])?)
            }
            other => return Err(format!("unknown fn {other:?}")),
        };
        let want = decode_js(&c.expected);
        if want.as_str() == Some(got.as_str()) {
            Ok(())
        } else {
            Err(format!("expected {want}, got {got:?}"))
        }
    });
}
