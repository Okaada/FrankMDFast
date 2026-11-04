//! Shared helpers for the upload / import / recovery ports (owned by the
//! media-uploads port; other test files have their own helpers).
#![allow(
    dead_code,
    unreachable_pub,
    clippy::disallowed_methods,
    clippy::unwrap_used
)]

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use frankmd_media::config::MediaConfig;
use frankmd_media::ctx::MediaCtx;
use frankmd_media::env::{MediaEnv, MemorySettings};
use frankmd_media::param::Param;
use frankmd_net::{EgressClient, Limits, StaticResolver, TrustedClient};
use frankmd_testkit::serde_json::Value;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A valid 1x1 RGB PNG. The port's first copy of the original tests' PNG had
/// a corrupted IDAT chunk (CRC mismatch, so no strict decoder could read it);
/// these are the same IHDR/IEND chunks with a correct IDAT.
pub const PNG_1X1: [u8; 69] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0xC9, 0xFE, 0x92, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// security: uploads are content-sniffed, so tests that stored placeholder
/// text ("img", "fake video bytes") now store a real minimal file of the type
/// the name claims. `ext` is the (case-insensitive) extension incl. the dot.
pub fn sample_media(ext: &str) -> Vec<u8> {
    use image::ImageFormat;
    let image = |format: ImageFormat| {
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(1, 1)
            .write_to(&mut out, format)
            .unwrap();
        out.into_inner()
    };
    match ext.to_ascii_lowercase().as_str() {
        ".png" => PNG_1X1.to_vec(),
        ".jpg" | ".jpeg" => image(ImageFormat::Jpeg),
        ".gif" => image(ImageFormat::Gif),
        ".webp" => image(ImageFormat::WebP),
        ".bmp" => unb64(BMP_1X1_B64),
        ".mp4" => iso_bmff(b"isom"),
        ".m4v" => iso_bmff(b"M4V "),
        ".mov" => iso_bmff(b"qt  "),
        ".webm" => ebml(b"webm"),
        ".mkv" => ebml(b"matroska"),
        ".avi" => {
            let mut v = b"RIFF\x24\x00\x00\x00AVI LIST".to_vec();
            v.resize(64, 0);
            v
        }
        ".ogv" => {
            let mut v = b"OggS\x00\x02".to_vec();
            v.resize(64, 0);
            v
        }
        other => panic!("no sample for {other}"),
    }
}

/// `ftyp` box with `brand` followed by an empty `mdat`.
fn iso_bmff(brand: &[u8; 4]) -> Vec<u8> {
    let mut v = vec![0, 0, 0, 0x18];
    v.extend_from_slice(b"ftyp");
    v.extend_from_slice(brand);
    v.extend_from_slice(&[0, 0, 2, 0]);
    v.extend_from_slice(brand);
    v.extend_from_slice(b"mp41");
    v.extend_from_slice(&[0, 0, 0, 8]);
    v.extend_from_slice(b"mdat");
    v
}

/// EBML header with the given DocType, padded past infer's 256-byte window.
fn ebml(doctype: &[u8]) -> Vec<u8> {
    let mut v = vec![
        0x1A, 0x45, 0xDF, 0xA3, 0x9F, 0x42, 0x86, 0x81, 0x01, 0x42, 0x82,
    ];
    v.push(0x80 | doctype.len() as u8);
    v.extend_from_slice(doctype);
    v.resize(300, 0);
    v
}

/// [`sample_media`] for the extension of `name`.
pub fn sample_for(name: &str) -> Vec<u8> {
    let ext = name.rfind('.').map_or("", |i| &name[i..]);
    sample_media(ext)
}

/// 1x1 24-bit BMP from image_import_test.rb (base64 decoded).
pub const BMP_1X1_B64: &str =
    "Qk06AAAAAAAAADYAAAAoAAAAAQAAAAEAAAABABgAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAA////AA==";

/// The public IP every test host resolves to (`EgressPolicy.resolve` stub).
pub const PUBLIC_IP: &str = "93.184.216.34";

/// `Time.utc(2026, 1, 1, 12, 0, 0)` + pinned hex.
pub fn pinned_ctx() -> MediaCtx {
    pinned_at("2026-01-01T12:00:00Z", "UTC")
}

/// Pinned clock at `time` (RFC 3339 UTC) viewed in `tz`.
pub fn pinned_at(time: &str, tz: &str) -> MediaCtx {
    let ts: jiff::Timestamp = time.parse().unwrap();
    MediaCtx::pinned(ts.in_tz(tz).unwrap())
}

/// The golden `input.pinned` block → ctx (default 2026-01-01T12:00:00Z UTC).
pub fn ctx_from_pinned(input: &Value) -> MediaCtx {
    let p = &input["pinned"];
    let time = p["time"].as_str().unwrap_or("2026-01-01T12:00:00Z");
    let tz = p["process_TZ"].as_str().unwrap_or("UTC");
    pinned_at(time, tz)
}

/// Strings are slices of the hex alphabet in pinned mode; real mode needs a
/// regex check. `true` if `s` is `n` lowercase hex chars.
pub fn is_hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// JSON fixture value → [`Param`].
pub fn param(v: &Value) -> Param {
    match v {
        Value::Null => Param::Nil,
        Value::Bool(b) => Param::Bool(*b),
        Value::Number(n) if n.is_i64() => Param::Int(n.as_i64().unwrap()),
        Value::Number(n) => Param::Float(n.as_f64().unwrap()),
        Value::String(s) => Param::Str(s.clone()),
        Value::Array(a) => Param::List(a.iter().map(param).collect()),
        Value::Object(_) => panic!("unsupported param {v}"),
    }
}

/// Local provider; images `.jpg .jpeg .png .gif .webp .bmp`; videos `.mp4 .webm`
/// (the stubs most service tests used).
pub fn local_config(notes_root: &Path) -> MediaConfig {
    let mut c = MediaConfig::new(notes_root);
    c.media_storage_provider = Some("local".into());
    c.image_upload_extensions = exts(&[".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"]);
    c.video_upload_extensions = exts(&[".mp4", ".webm"]);
    c
}

/// S3 provider fully configured (`test-key-id`/`test-secret`, `test-bucket`, `region`).
pub fn s3_config(notes_root: &Path, region: &str) -> MediaConfig {
    let mut c = local_config(notes_root);
    c.media_storage_provider = Some("s3".into());
    c.s3.access_key_id = Some("test-key-id".into());
    c.s3.secret_access_key = Some("test-secret".into());
    c.s3.bucket = Some("test-bucket".into());
    c.s3.region = Some(region.into());
    c
}

/// Owned extension list.
pub fn exts(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// Resolver used by most tests: `example.com`, `cdn.example.com` → [`PUBLIC_IP`],
/// `169.254.169.254` → itself.
pub fn default_resolver() -> StaticResolver {
    let public: IpAddr = PUBLIC_IP.parse().unwrap();
    let metadata: IpAddr = "169.254.169.254".parse().unwrap();
    StaticResolver::new()
        .with("example.com", &[public])
        .with("cdn.example.com", &[public])
        .with("169.254.169.254", &[metadata])
}

/// Build an env: settings, pinned ctx, egress pinned to `mock` (if any) via
/// `resolver`, S3 transport to `mock` (if any).
pub fn env_with(
    settings: Arc<MemorySettings>,
    ctx: MediaCtx,
    mock: Option<&MockServer>,
    resolver: StaticResolver,
    limits: Limits,
) -> MediaEnv {
    let mut egress = EgressClient::with_resolver(limits, Arc::new(resolver)).unwrap();
    if let Some(m) = mock {
        egress = egress.with_dial_override(*m.address());
    }
    let mut env = MediaEnv::new(settings, ctx, TrustedClient::new().unwrap(), egress);
    env.s3_transport = mock.map(|m| url::Url::parse(&m.uri()).unwrap());
    env
}

/// Default env for a config (pinned ctx, default resolver, external-image limits).
pub fn env_for(config: MediaConfig, mock: Option<&MockServer>) -> (Arc<MemorySettings>, MediaEnv) {
    let settings = MemorySettings::new(config);
    let env = env_with(
        settings.clone(),
        pinned_ctx(),
        mock,
        default_resolver(),
        Limits::EXTERNAL_IMAGE,
    );
    (settings, env)
}

/// Any S3 PUT under `/test-bucket/` (or `bucket`) succeeds.
pub async fn s3_put_ok(server: &MockServer, bucket: &str) {
    Mock::given(method("PUT"))
        .and(path_regex(format!("^/{bucket}/")))
        .respond_with(ResponseTemplate::new(200))
        .mount(server)
        .await;
}

/// S3 error document body.
pub fn s3_error_body(code: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Error><Code>{code}</Code><Message>private provider details</Message><RequestId>1</RequestId></Error>"
    )
}

/// Every S3 PUT fails with `code` / `status`.
pub async fn s3_put_error(server: &MockServer, code: &str, status: u16) {
    Mock::given(method("PUT"))
        .respond_with(
            ResponseTemplate::new(status).set_body_raw(s3_error_body(code), "application/xml"),
        )
        .mount(server)
        .await;
}

/// GET `path` → 200 with `body` and `content_type`.
pub async fn external(server: &MockServer, path: &str, body: &[u8], content_type: &str) {
    Mock::given(method("GET"))
        .and(wiremock::matchers::path(path))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.to_vec(), content_type))
        .mount(server)
        .await;
}

/// Requests received by `server` (method, path).
pub async fn requests(server: &MockServer) -> Vec<(String, String)> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .map(|r| (r.method.to_string(), r.url.path().to_owned()))
        .collect()
}

/// All PUT requests (path, headers lookup, body).
pub async fn puts(server: &MockServer) -> Vec<wiremock::Request> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.method.as_str() == "PUT")
        .collect()
}

/// Header value of a recorded request.
pub fn header(req: &wiremock::Request, name: &str) -> Option<String> {
    req.headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Write a file (creating parents) and return its path.
pub fn write(root: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, bytes).unwrap();
    p
}

/// Set a file's mtime (ordering tests; the original slept 10 ms between writes).
pub fn set_mtime(path: &Path, secs_ago: u64) {
    let t = std::time::SystemTime::now() - Duration::from_secs(secs_ago);
    let f = std::fs::File::options().write(true).open(path).unwrap();
    f.set_modified(t).unwrap();
}

/// Simple regex-free matcher for `^prefix<16 hex>suffix$`.
pub fn matches_hex_key(s: &str, prefix: &str, suffix: &str) -> bool {
    s.strip_prefix(prefix)
        .and_then(|r| r.strip_suffix(suffix))
        .is_some_and(|hex| is_hex(hex, 16))
}

/// `frankmd/YYYY/MM/` prefix check.
pub fn has_dated_prefix(key: &str) -> bool {
    let parts: Vec<&str> = key.split('/').collect();
    parts.len() >= 4
        && parts[0] == "frankmd"
        && parts[1].len() == 4
        && parts[1].bytes().all(|b| b.is_ascii_digit())
        && parts[2].len() == 2
        && parts[2].bytes().all(|b| b.is_ascii_digit())
}

/// `Regexp#match?` for the original `assert_match` patterns.
pub fn assert_match(pattern: &str, s: &str) {
    let re = regex::Regex::new(pattern).unwrap();
    assert!(re.is_match(s), "expected {s:?} to match /{pattern}/");
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `Base64.strict_encode64`.
pub fn b64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `Base64.strict_decode64` (input assumed valid).
pub fn unb64(s: &str) -> Vec<u8> {
    let vals: Vec<u32> = s
        .bytes()
        .filter(|&c| c != b'=')
        .map(|c| B64.iter().position(|&x| x == c).unwrap() as u32)
        .collect();
    let mut out = Vec::new();
    for chunk in vals.chunks(4) {
        let mut n = 0u32;
        for (i, v) in chunk.iter().enumerate() {
            n |= v << (18 - 6 * i);
        }
        for i in 0..chunk.len().saturating_sub(1) {
            out.push((n >> (16 - 8 * i)) as u8);
        }
    }
    out
}
