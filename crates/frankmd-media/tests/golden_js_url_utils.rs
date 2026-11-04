//! Conformance against `tests/fixtures/golden/js_url_utils.json` (95 cases).
//! `$undefined`/`null` args map to `None`; strings are compared after
//! `decode_js` (UTF-16 inputs decoded to Rust strings).

use frankmd_media::url_utils::{encode_path, extract_youtube_id};
use frankmd_testkit::serde_json::Value;
use frankmd_testkit::{assert_cases, decode_js, golden};

#[test]
fn js_url_utils() {
    let g = golden("js_url_utils");
    assert_cases("js_url_utils", &g.cases, |c| {
        let arg = decode_js(c.input.as_array().and_then(|a| a.first()).unwrap_or(&Value::Null));
        let arg = arg.as_str();
        let got = match c.func.as_deref() {
            Some("encodePath") => Value::String(encode_path(arg)),
            Some("extractYouTubeId") => extract_youtube_id(arg).map_or(Value::Null, Value::String),
            other => return Err(format!("unknown fn {other:?}")),
        };
        let want = decode_js(&c.expected);
        if got == want { Ok(()) } else { Err(format!("input {arg:?}: expected {want}, got {got}")) }
    });
}
