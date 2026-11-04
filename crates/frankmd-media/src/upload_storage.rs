//! **[COMPAT]** port of `app/services/upload_storage.rb`: the shared,
//! security-sensitive upload rules (extension allow-list, size caps, naming,
//! provider resolution, S3 keys and URLs). Golden: `media.json` `UploadStorage.*`.

use crate::config::{MediaConfig, StorageProvider};
use crate::ctx::MediaCtx;
use crate::error::MediaError;
use crate::param::Param;
use crate::s3::S3Error;

/// `SANITIZE_RE` source: every char outside this class becomes `_`
/// (one `_` per *character*, not per byte).
pub const SANITIZE_RE: &str = "[^a-zA-Z0-9._-]";
/// `MAX_UPLOAD_BYTES` = 512 MiB.
pub const MAX_UPLOAD_BYTES: u64 = 512 * 1024 * 1024;
/// `S3_KEY_PREFIX`.
pub const S3_KEY_PREFIX: &str = "frankmd";
/// `DEFAULT_S3_REGION`.
pub const DEFAULT_S3_REGION: &str = crate::s3::DEFAULT_S3_REGION;

/// Message of the size-cap rejection.
pub const TOO_LARGE_MESSAGE: &str = "File is too large (max 512 MB).";

/// An uploaded file as received (multipart / drop / paste).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upload {
    /// Client-supplied name (untrusted; only sanitized forms are ever used).
    pub original_filename: String,
    /// Bytes.
    pub bytes: Vec<u8>,
}

impl Upload {
    /// Build.
    pub fn new(original_filename: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            original_filename: original_filename.into(),
            bytes: bytes.into(),
        }
    }
}

/// Which allow-list (`image_upload_extensions` / `video_upload_extensions`)
/// and which word goes into the message (`"image"` / `"video"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadKind {
    /// `image_upload_extensions`, "image".
    Image,
    /// `video_upload_extensions`, "video".
    Video,
}

/// `UploadStorage.provider(legacy_s3:)`: the persisted provider, never a
/// UI-supplied one. Errors ([`MediaError::Rejected`], exact messages):
/// - not `local`/`s3` (case-sensitive) → "Unknown media storage provider";
/// - `legacy_s3` given and its Rails boolean cast disagrees (note `""` casts to
///   nil, which disagrees with both) → "Upload destination conflicts with the
///   active media storage provider";
/// - `s3` but not [`s3_configured`] → "S3 is not configured".
pub fn provider(config: &MediaConfig, legacy_s3: &Param) -> Result<StorageProvider, MediaError> {
    let selected = config.media_storage_provider.as_deref();
    let provider = match selected {
        Some("local") => StorageProvider::Local,
        Some("s3") => StorageProvider::S3,
        _ => {
            return Err(MediaError::Rejected(
                "Unknown media storage provider".to_owned(),
            ));
        }
    };
    if !legacy_s3.is_nil()
        && legacy_s3.rails_boolean_cast() != Some(provider == StorageProvider::S3)
    {
        return Err(MediaError::Rejected(
            "Upload destination conflicts with the active media storage provider".to_owned(),
        ));
    }
    if provider == StorageProvider::S3 && !s3_configured(config) {
        return Err(MediaError::Rejected("S3 is not configured".to_owned()));
    }
    Ok(provider)
}

/// `UploadStorage.s3_configured?`: key id, secret and bucket present; an
/// endpoint, if set, must parse as http(s) with a host and no userinfo.
pub fn s3_configured(config: &MediaConfig) -> bool {
    let s3 = &config.s3;
    let present = |v: &Option<String>| v.as_deref().is_some_and(|s| !s.trim().is_empty());
    if !(present(&s3.access_key_id) && present(&s3.secret_access_key) && present(&s3.bucket)) {
        return false;
    }
    match s3.endpoint.as_deref().filter(|e| !e.trim().is_empty()) {
        None => true,
        Some(endpoint) => endpoint_is_plain_http(endpoint),
    }
}

/// `UploadStorage.invalid_s3_configuration_error?`: only definitive
/// configuration/auth failures (InvalidRegion, or a service error whose code is
/// one of InvalidAccessKeyId SignatureDoesNotMatch InvalidToken ExpiredToken
/// TokenRefreshRequired AccessDenied NoSuchBucket InvalidBucketName
/// AuthorizationHeaderMalformed) trigger recovery to local.
pub fn invalid_s3_configuration_error(error: &S3Error) -> bool {
    const CODES: [&str; 9] = [
        "InvalidAccessKeyId",
        "SignatureDoesNotMatch",
        "InvalidToken",
        "ExpiredToken",
        "TokenRefreshRequired",
        "AccessDenied",
        "NoSuchBucket",
        "InvalidBucketName",
        "AuthorizationHeaderMalformed",
    ];
    match error {
        S3Error::InvalidRegion => true,
        S3Error::Service { code, .. } => CODES.contains(&code.as_str()),
        S3Error::Network(_) => false,
    }
}

/// `validate_filename!` / `validate_extension!`: `File.extname(name).downcase`
/// must be in the configured list. Returns the normalized extension; else
/// `Rejected("<.ext|no extension> is not an accepted <kind> type. Accepted: .a, .b")`.
pub fn validate_filename(
    config: &MediaConfig,
    filename: &str,
    kind: UploadKind,
) -> Result<String, MediaError> {
    let extension = ruby_extname(filename).to_lowercase();
    let (allowed, word) = match kind {
        UploadKind::Image => (&config.image_upload_extensions, "image"),
        UploadKind::Video => (&config.video_upload_extensions, "video"),
    };
    if allowed.contains(&extension) {
        return Ok(extension);
    }
    let shown = if extension.trim().is_empty() {
        "no extension"
    } else {
        extension.as_str()
    };
    Err(MediaError::Rejected(format!(
        "{shown} is not an accepted {word} type. Accepted: {}",
        allowed.join(", ")
    )))
}

/// `enforce_size!` / `enforce_bytes!`: `None` (unknown size) passes; above
/// `max` → `Rejected(TOO_LARGE_MESSAGE)`. (Ruby accepted negative sizes.)
pub fn enforce_bytes(size: Option<i64>, max: u64) -> Result<(), MediaError> {
    match size {
        Some(size) if size > 0 && size.unsigned_abs() > max => {
            Err(MediaError::Rejected(TOO_LARGE_MESSAGE.to_owned()))
        }
        _ => Ok(()),
    }
}

/// `max_base64_encoded_bytes` = `((MAX+2)/3)*4 + 1024` = 715828908.
pub const fn max_base64_encoded_bytes() -> u64 {
    MAX_UPLOAD_BYTES.div_ceil(3) * 4 + 1024
}

/// `enforce_base64_size!`: encoded length above `max` → `Rejected(TOO_LARGE_MESSAGE)`.
pub fn enforce_base64_size(size: u64, max: u64) -> Result<(), MediaError> {
    if size > max {
        Err(MediaError::Rejected(TOO_LARGE_MESSAGE.to_owned()))
    } else {
        Ok(())
    }
}

/// `SANITIZE_RE` replacement (`nil` → `""`).
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// `dest_filename`: `"<YYYYMMDD>_<HHMMSS>_<hex(4)>_<sanitized>"`, time in the
/// clock's zone (the original used the *process* local zone).
pub fn dest_filename(ctx: &MediaCtx, original_filename: Option<&str>) -> String {
    let now = ctx.clock.now();
    let safe = sanitize_filename(original_filename.unwrap_or_default());
    format!(
        "{}_{}_{safe}",
        now.strftime("%Y%m%d_%H%M%S"),
        ctx.random.hex(4)
    )
}

/// `sanitize_s3_key`: split on `/`, drop empty/`.`/`..` segments, sanitize
/// each, rejoin (`None` → `""`).
pub fn sanitize_s3_key(value: Option<&str>) -> String {
    value
        .unwrap_or_default()
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .map(sanitize_filename)
        .collect::<Vec<_>>()
        .join("/")
}

/// `unique_s3_key(key, original_filename)`: insert `-<hex(8)>` before the
/// key's extension (or the filename's when the key has none).
pub fn unique_s3_key(ctx: &MediaCtx, key: &str, original_filename: Option<&str>) -> String {
    let mut extension = ruby_extname(key);
    if extension.trim().is_empty() {
        extension = ruby_extname(original_filename.unwrap_or_default());
    }
    let base = if !extension.trim().is_empty() {
        key.strip_suffix(extension).unwrap_or(key)
    } else {
        key
    };
    format!("{base}-{}{extension}", ctx.random.hex(8))
}

/// `s3_key(filename, custom_key:, custom_prefix:)`: custom key (sanitized,
/// made unique) wins; else `<sanitized prefix | frankmd/YYYY/MM>/<safe name>`
/// made unique. `YYYY/MM` uses **UTC** (`Time.current` in the default Rails zone).
pub fn s3_key(
    ctx: &MediaCtx,
    original_filename: Option<&str>,
    custom_key: Option<&str>,
    custom_prefix: Option<&str>,
) -> String {
    let key = sanitize_s3_key(custom_key);
    if !key.trim().is_empty() {
        return unique_s3_key(ctx, &key, original_filename);
    }
    let safe = sanitize_filename(original_filename.unwrap_or_default());
    let mut prefix = sanitize_s3_key(custom_prefix);
    if prefix.trim().is_empty() {
        // `Time.current` in the default Rails zone (UTC).
        let utc = ctx.clock.now().with_time_zone(jiff::tz::TimeZone::UTC);
        prefix = format!("{S3_KEY_PREFIX}/{}", utc.strftime("%Y/%m"));
    }
    unique_s3_key(ctx, &format!("{prefix}/{safe}"), original_filename)
}

/// `s3_region`: `aws_region` if present, else `DEFAULT_S3_REGION`.
pub fn s3_region(config: &MediaConfig) -> String {
    config
        .s3
        .region
        .as_deref()
        .filter(|r| !r.trim().is_empty())
        .unwrap_or(DEFAULT_S3_REGION)
        .to_owned()
}

/// `s3_url(bucket, region, key)`: segments `ERB::Util.url_encode`d. No endpoint →
/// `https://<bucket>.s3.<region>.amazonaws.com/<key>`; endpoint + path style →
/// `<endpoint without trailing />/<bucket>/<key>`; endpoint virtual-host →
/// `<scheme>://<bucket>.<host>[:<non-default port>]/<key>` (endpoint path dropped).
pub fn s3_url(config: &MediaConfig, bucket: &str, region: &str, key: &str) -> String {
    let encoded = key
        .split('/')
        .map(erb_url_encode)
        .collect::<Vec<_>>()
        .join("/");
    let Some(endpoint) = config
        .s3
        .endpoint
        .as_deref()
        .filter(|e| !e.trim().is_empty())
    else {
        return format!("https://{bucket}.s3.{region}.amazonaws.com/{encoded}");
    };
    let base = endpoint.strip_suffix('/').unwrap_or(endpoint);
    if config.s3.force_path_style {
        return format!("{base}/{bucket}/{encoded}");
    }
    match url::Url::parse(base) {
        Ok(uri) => {
            let port = uri.port().map(|p| format!(":{p}")).unwrap_or_default();
            format!(
                "{}://{bucket}.{}{port}/{encoded}",
                uri.scheme(),
                uri.host_str().unwrap_or_default()
            )
        }
        // Unreachable for a configured provider (`s3_configured` validated the endpoint).
        Err(_) => format!("{base}/{bucket}/{encoded}"),
    }
}

/// Ruby `File.extname`: extension of the basename, leading dots ignored
/// (`".png"` → `""`), a trailing dot kept (`"x."` → `"."`).
pub fn ruby_extname(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    let base = trimmed.rsplit('/').next().unwrap_or(trimmed);
    let name = base.trim_start_matches('.');
    name.rfind('.').map_or("", |i| &name[i..])
}

/// Ruby `File.basename(name, ".*")`: the basename without its extension.
pub fn ruby_basename_without_ext(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    let base = trimmed.rsplit('/').next().unwrap_or(trimmed);
    let ext = ruby_extname(base);
    if ext.is_empty() {
        base
    } else {
        &base[..base.len() - ext.len()]
    }
}

/// `ERB::Util.url_encode`: percent-encode everything but `[A-Za-z0-9_.~-]`.
pub(crate) fn erb_url_encode(part: &str) -> String {
    let mut out = String::with_capacity(part.len());
    for b in part.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'~' | b'-') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `URI.parse(endpoint)` is http(s) with a host and no userinfo.
fn endpoint_is_plain_http(endpoint: &str) -> bool {
    match url::Url::parse(endpoint) {
        Ok(uri) => {
            matches!(uri.scheme(), "http" | "https")
                && uri.host_str().is_some_and(|h| !h.is_empty())
                && uri.username().is_empty()
                && uri.password().is_none()
                && !endpoint.contains('@')
        }
        Err(_) => false,
    }
}
