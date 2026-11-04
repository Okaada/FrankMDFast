//! **[COMPAT]** port of `app/services/images_service.rb`: the user's server
//! image folder (`images_path`), image uploads (multipart, base64), legacy
//! S3 uploads and external-URL downloads. Golden: `media.json` `ImagesService.*`.
//!
//! Resizing uses the `image` crate instead of ImageMagick; the ratio whitelist
//! ([`resize_arg`]) and the "fall back to the original bytes" rule are kept.

use std::io::Cursor;
use std::path::Path;

use base64::Engine as _;

use crate::config::MediaConfig;
use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::param::Param;
use crate::pipeline::{UploadRequest, store_media};
use crate::upload_storage::{self as us, Upload};
use crate::{MediaKind, UploadOrigin};

pub use crate::server_folder::{
    Dimensions, MAX_RESULTS, SUPPORTED_EXTENSIONS, ServerImage, clamp_limit, clamp_offset, count,
    delete, enabled, find_image, image_dimensions, list,
};
/// `MAX_EXTERNAL_IMAGE_BYTES` = 25 MiB.
pub const MAX_EXTERNAL_IMAGE_BYTES: u64 = 25 * 1024 * 1024;
/// `MAX_EXTERNAL_IMAGE_REDIRECTS`.
pub const MAX_EXTERNAL_IMAGE_REDIRECTS: usize = 5;
/// `MAX_EXTERNAL_IMAGE_SECONDS`.
pub const MAX_EXTERNAL_IMAGE_SECONDS: u64 = 30;

/// Result of [`resize_and_compress`]: `[content, content_type, filename]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resized {
    /// Bytes (re-encoded JPEG, or the original on failure).
    pub bytes: Vec<u8>,
    /// `image/jpeg`, or [`content_type_for`] of the source on failure.
    pub content_type: String,
    /// `<stem>.jpg`, or the original name on failure.
    pub filename: String,
}

/// `s3_enabled?` = `UploadStorage.s3_configured?`.
pub fn s3_enabled(config: &MediaConfig) -> bool {
    us::s3_configured(config)
}

/// `imagemagick_resize_arg(ratio)`: whitelist on `ratio.to_s` →
/// `"25%" | "33%" | "50%" | "67%" | "75%" | "100%"`, default `"50%"`
/// (so injection strings can never reach the encoder).
pub fn resize_arg(ratio: &Param) -> &'static str {
    match ratio.ruby_to_s().as_str() {
        "0.25" | "0.250" | "25" | "25%" => "25%",
        "0.33" | "0.333" | "33" | "33%" => "33%",
        "0.5" | "0.50" | "50" | "50%" | "true" => "50%",
        "0.67" | "0.666" | "0.667" | "67" | "67%" => "67%",
        "0.75" | "0.750" | "75" | "75%" => "75%",
        "1" | "1.0" | "1.00" | "100" | "100%" => "100%",
        _ => "50%",
    }
}

/// `resize_and_compress(source, original_name, ratio)`: scale by
/// [`resize_arg`], encode JPEG q95, strip metadata, name `<stem>.jpg`.
/// Any decode/encode failure returns the original bytes,
/// `content_type_for(source_name)` and `original_name` (no error).
pub fn resize_and_compress(
    bytes: &[u8],
    source_name: &str,
    original_name: Option<&str>,
    ratio: &Param,
) -> Resized {
    let original_name =
        original_name.map_or_else(|| basename(source_name).to_owned(), str::to_owned);
    let percent: u32 = resize_arg(ratio)
        .trim_end_matches('%')
        .parse()
        .unwrap_or(50);
    match reencode_jpeg(bytes, percent) {
        Some(jpeg) => Resized {
            bytes: jpeg,
            content_type: "image/jpeg".to_owned(),
            filename: format!("{}.jpg", us::ruby_basename_without_ext(&original_name)),
        },
        None => Resized {
            bytes: bytes.to_vec(),
            content_type: content_type_for(source_name).to_owned(),
            filename: original_name,
        },
    }
}

/// `content_type_for(path)` by extension (case-insensitive); unknown →
/// `application/octet-stream`.
pub fn content_type_for(path: &str) -> &'static str {
    match us::ruby_extname(path).to_lowercase().as_str() {
        ".jpg" | ".jpeg" => "image/jpeg",
        ".png" => "image/png",
        ".gif" => "image/gif",
        ".webp" => "image/webp",
        ".svg" => "image/svg+xml",
        ".bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// `extension_for_content_type`: first `;` part, stripped, downcased;
/// unknown → `.jpg`. `None` where the original raised `NoMethodError`
/// (`nil` or `""` input).
pub fn extension_for_content_type(content_type: Option<&str>) -> Option<&'static str> {
    // Ruby `String#split(";")` drops trailing empty fields, so `""` and `";"`
    // have no first element and `.strip` raised NoMethodError.
    let mut parts: Vec<&str> = content_type?.split(';').collect();
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    let first = parts.first()?;
    Some(match first.trim().to_lowercase().as_str() {
        "image/jpeg" => ".jpg",
        "image/png" => ".png",
        "image/gif" => ".gif",
        "image/webp" => ".webp",
        "image/svg+xml" => ".svg",
        "image/bmp" | "image/x-ms-bmp" => ".bmp",
        _ => ".jpg",
    })
}

/// `ImagesController#parse_resize_ratio`: `true`/`"true"` → 0.5; blank,
/// `"false"`, `false` → `None`; else `to_f` within `(0, 1]`.
pub fn parse_resize_ratio(value: &Param) -> Option<f64> {
    match value {
        Param::Bool(true) => return Some(0.5),
        Param::Str(s) if s == "true" => return Some(0.5),
        Param::Str(s) if s == "false" => return None,
        v if !v.is_present() => return None,
        _ => {}
    }
    let ratio = match value {
        Param::Int(i) => *i as f64,
        Param::Float(f) => *f,
        Param::Str(s) => ruby_str_to_f(s),
        _ => return None,
    };
    (ratio > 0.0 && ratio <= 1.0).then_some(ratio)
}

/// `upload_file(file, resize:, upload_to_s3:, s3_key:)`: provider first
/// (rejections win), size cap, extension allow-list, then local
/// (`images/<dest_filename>`) or S3 (`s3_key` = custom key). Returns the `url`.
/// No file → `Invalid("No file provided")`. S3 collision → `Upstream("Image upload failed")`;
/// other S3 errors propagate as [`MediaError::S3`] (recovery classifies them).
pub async fn upload_file(
    env: &MediaEnv,
    file: Option<Upload>,
    resize: Option<f64>,
    upload_to_s3: &Param,
    s3_key: Option<&str>,
) -> Result<String, MediaError> {
    let Some(file) = file else {
        return Err(MediaError::Invalid("No file provided".to_owned()));
    };
    let config = env.settings.current();
    let provider = us::provider(&config, upload_to_s3)?;
    let mut req = UploadRequest::new(
        UploadOrigin::Picker,
        MediaKind::Image,
        file.original_filename,
        file.bytes,
    );
    req.resize = resize;
    req.s3_key = s3_key.map(str::to_owned);
    Ok(store_media(env, &config, provider, req)
        .await?
        .url()
        .to_owned())
}

/// `upload_base64_data(data, mime_type:, filename:, upload_to_s3:, s3_prefix:)`:
/// provider; encoded-size cap **before** decoding; strict base64 (else
/// `Invalid("Invalid base64 data: invalid base64")`); mime default `image/png`;
/// generated name `ai_generated_<hex(8)><ext>`; append ext if the name has no
/// `\.\w+$`; decoded cap; allow-list; store locally or to S3 (`s3_prefix`).
pub async fn upload_base64_data(
    env: &MediaEnv,
    data: &str,
    mime_type: Option<&str>,
    filename: Option<&str>,
    upload_to_s3: &Param,
    s3_prefix: Option<&str>,
) -> Result<String, MediaError> {
    let config = env.settings.current();
    let provider = us::provider(&config, upload_to_s3)?;
    us::enforce_base64_size(data.len() as u64, env.limits.max_base64_encoded_bytes)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| MediaError::Invalid("Invalid base64 data: invalid base64".to_owned()))?;
    let mime_type = mime_type
        .filter(|m| !m.trim().is_empty())
        .unwrap_or("image/png");
    let extension = extension_for_content_type(Some(mime_type)).unwrap_or(".jpg");
    let mut filename = match filename.filter(|f| !f.trim().is_empty()) {
        Some(name) => name.to_owned(),
        None => format!("ai_generated_{}{extension}", env.ctx.random.hex(8)),
    };
    if !has_word_extension(&filename) {
        filename.push_str(extension);
    }
    let mut req = UploadRequest::new(UploadOrigin::AiGenerated, MediaKind::Image, filename, bytes);
    // [QUIRK] the S3 content type follows the mime-derived extension, not the filename.
    req.type_name = format!("upload{extension}");
    req.s3_prefix = s3_prefix.map(str::to_owned);
    Ok(store_media(env, &config, provider, req)
        .await?
        .url()
        .to_owned())
}

/// `save_to_notes_directory(temp, original_filename, resize:)`: writes
/// `<notes>/images/<dest_filename(sanitized)>` no-clobber (resized → `<stem>.jpg`),
/// returns `images/<name>`.
pub fn save_to_notes_directory(
    env: &MediaEnv,
    bytes: &[u8],
    original_filename: &str,
    resize: Option<f64>,
) -> Result<String, MediaError> {
    let config = env.settings.current();
    let mut req = UploadRequest::new(
        UploadOrigin::Picker,
        MediaKind::Image,
        original_filename,
        bytes.to_vec(),
    );
    req.resize = resize;
    Ok(crate::pipeline::store_media_local(env, &config, req)?
        .url()
        .to_owned())
}

/// `upload_to_s3(path, resize:, custom_key:)` (legacy `/images/upload_to_s3`):
/// S3 not configured → `Rejected("S3 is not configured")`; provider must be s3
/// (`provider(legacy_s3: true)`); image via [`find_image`] (else `NotFound`);
/// `AccessControlListNotSupported` tolerated; collision → `Upstream("Image upload failed")`.
pub async fn upload_to_s3(
    env: &MediaEnv,
    rel: &str,
    resize: Option<f64>,
    custom_key: Option<&str>,
) -> Result<String, MediaError> {
    let config = env.settings.current();
    if !s3_enabled(&config) {
        return Err(MediaError::Rejected("S3 is not configured".to_owned()));
    }
    let provider = us::provider(&config, &Param::Bool(true))?;
    let path = find_image(&config, rel).ok_or(MediaError::NotFound)?;
    let bytes = read_file(&path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut req = UploadRequest::new(UploadOrigin::Picker, MediaKind::Image, name, bytes);
    req.resize = resize;
    req.s3_key = custom_key.map(str::to_owned);
    Ok(store_media(env, &config, provider, req)
        .await?
        .url()
        .to_owned())
}

/// `download_and_upload_to_s3(url, resize:, custom_prefix:)` (legacy
/// `/images/upload_external_to_s3`): same S3/provider gates, then
/// [`fetch_external_image`]; name from the URL path basename (sanitized) or
/// `<hex(8)><ext from Content-Type>`; S3 key with `custom_prefix`.
pub async fn download_and_upload_to_s3(
    env: &MediaEnv,
    url: &str,
    resize: Option<f64>,
    custom_prefix: Option<&str>,
) -> Result<String, MediaError> {
    let config = env.settings.current();
    if !s3_enabled(&config) {
        return Err(MediaError::Rejected("S3 is not configured".to_owned()));
    }
    let provider = us::provider(&config, &Param::Bool(true))?;
    let fetched = fetch_external_image(env, url).await?;
    let content_type = fetched.content_type.as_deref().unwrap_or("image/jpeg");
    let extension = extension_for_content_type(Some(content_type)).unwrap_or(".jpg");
    let mut name = us::sanitize_filename(basename(fetched.url.path()));
    if name.trim().is_empty() || name == "_" || !has_word_extension(&name) {
        name = format!("{}{extension}", env.ctx.random.hex(8));
    }
    let mut req = UploadRequest::new(
        UploadOrigin::UrlImport,
        MediaKind::Image,
        name,
        fetched.bytes,
    );
    req.resize = resize;
    req.s3_prefix = custom_prefix.map(str::to_owned);
    // security: the object's Content-Type follows the validated name, never the
    // remote server's header (the original stored that header verbatim).
    Ok(store_media(env, &config, provider, req)
        .await?
        .url()
        .to_owned())
}

/// `fetch_external_image(url)`: through [`MediaEnv::egress`] (EgressPolicy on
/// every hop, ≤ 5 redirects, 25 MiB cap on Content-Length and stream, 30 s
/// overall deadline). Missing `Content-Type` defaults to `image/jpeg`.
pub async fn fetch_external_image(
    env: &MediaEnv,
    url: &str,
) -> Result<frankmd_net::Fetched, MediaError> {
    let mut fetched = env.egress.fetch(url).await?;
    if fetched.content_type.as_deref().is_none_or(|c| c.is_empty()) {
        fetched.content_type = Some("image/jpeg".to_owned());
    }
    Ok(fetched)
}

/// JPEG quality of re-encoded images (ImageMagick `-quality 95`).
const JPEG_QUALITY: u8 = 95;

/// Decode, scale to `percent` (never below 1 px), re-encode as baseline JPEG.
/// Re-encoding drops every metadata block (`-strip`). `None` on any failure.
fn reencode_jpeg(bytes: &[u8], percent: u32) -> Option<Vec<u8>> {
    let img = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let scale = |v: u32| {
        ((u64::from(v) * u64::from(percent) + 50) / 100).clamp(1, u64::from(u32::MAX)) as u32
    };
    let (w, h) = (scale(img.width()), scale(img.height()));
    let img = if (w, h) == (img.width(), img.height()) {
        img
    } else {
        img.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
    };
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    rgb.write_with_encoder(encoder).ok()?;
    Some(out)
}

/// Last path segment (`File.basename`).
pub(crate) fn basename(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    trimmed.rsplit('/').next().unwrap_or(trimmed)
}

/// Ruby `/\.\w+$/`: the name ends in a dot plus word characters.
pub(crate) fn has_word_extension(name: &str) -> bool {
    name.split('\n').any(|line| {
        line.rfind('.').is_some_and(|i| {
            let tail = &line[i + 1..];
            !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
    })
}

/// Ruby `String#to_f` (leading float, else 0.0).
fn ruby_str_to_f(s: &str) -> f64 {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut end = 0;
    let digits = |from: usize| {
        let mut i = from;
        while i < bytes.len()
            && (bytes[i].is_ascii_digit()
                || (bytes[i] == b'_'
                    && i > from
                    && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)))
        {
            i += 1;
        }
        i
    };
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    let int_end = digits(end);
    if int_end == end {
        return 0.0;
    }
    end = int_end;
    if bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
        end = digits(end + 1);
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut exp = end + 1;
        if matches!(bytes.get(exp), Some(b'+' | b'-')) {
            exp += 1;
        }
        let exp_end = digits(exp);
        if exp_end > exp {
            end = exp_end;
        }
    }
    s[..end].replace('_', "").parse().unwrap_or(0.0)
}

/// Read a file the server folder lookup accepted (`binread`).
fn read_file(path: &Path) -> Result<Vec<u8>, MediaError> {
    std::fs::read(path).map_err(|e| MediaError::Io(e.kind()))
}
