//! Content sniffing for uploads (security decision: uploads accept only real
//! media). The original trusted the file extension alone; FrankMDFast also
//! requires the bytes to *be* an allowed image/video:
//!
//! 1. the magic bytes (via `infer`, plus the ISO-BMFF `ftyp` box) identify an
//!    image or video container of the upload's kind;
//! 2. that type maps to an extension in the configured allow-list
//!    (`image_upload_extensions` / `video_upload_extensions`);
//! 3. images must also have a parseable header (`image` crate), so a bare
//!    signature followed by junk is not accepted;
//! 4. nothing that a browser could sniff as markup (HTML/SVG/XML, `<script`)
//!    may appear in the leading bytes, and no `<script` anywhere in an image
//!    (polyglot files).
//!
//! SVG is never accepted: it is XML and can carry script.

use std::io::Cursor;

use crate::MediaKind;
use crate::error::MediaError;

/// Bytes a browser inspects when sniffing a resource as HTML.
const SNIFF_WINDOW: usize = 1024;

/// Markup that must not appear in the sniff window (lowercase).
const MARKUP_MARKERS: [&[u8]; 12] = [
    b"<!doctype",
    b"<html",
    b"<head",
    b"<body",
    b"<script",
    b"<svg",
    b"<?xml",
    b"<iframe",
    b"<object",
    b"<embed",
    b"<meta",
    b"<!--",
];

/// The rejection message (`kind` is "image" / "video").
pub fn rejection_message(kind: MediaKind) -> String {
    format!("File content is not a valid {}.", kind_word(kind))
}

fn kind_word(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "image",
        MediaKind::Video => "video",
    }
}

/// A detected media type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detected {
    /// Kind.
    pub kind: MediaKind,
    /// Canonical MIME type.
    pub mime: &'static str,
    /// Extensions this content may legitimately carry.
    pub extensions: &'static [&'static str],
}

/// Identify `bytes` by magic number; `None` when it is no supported media container.
pub fn detect(bytes: &[u8]) -> Option<Detected> {
    let image = |mime, extensions| {
        Some(Detected {
            kind: MediaKind::Image,
            mime,
            extensions,
        })
    };
    let video = |mime, extensions| {
        Some(Detected {
            kind: MediaKind::Video,
            mime,
            extensions,
        })
    };
    const ISO: &[&str] = &[".mp4", ".m4v", ".mov"];
    const EBML: &[&str] = &[".webm", ".mkv"];
    if let Some(t) = infer::get(bytes) {
        match t.mime_type() {
            "image/png" => return image("image/png", &[".png"]),
            "image/jpeg" => return image("image/jpeg", &[".jpg", ".jpeg"]),
            "image/gif" => return image("image/gif", &[".gif"]),
            "image/webp" => return image("image/webp", &[".webp"]),
            "image/bmp" => return image("image/bmp", &[".bmp"]),
            "video/mp4" => return video("video/mp4", ISO),
            "video/x-m4v" => return video("video/x-m4v", ISO),
            "video/quicktime" => return video("video/quicktime", ISO),
            "video/webm" => return video("video/webm", EBML),
            "video/x-matroska" => return video("video/x-matroska", EBML),
            "video/x-msvideo" => return video("video/x-msvideo", &[".avi"]),
            "video/x-ms-wmv" => return video("video/x-ms-wmv", &[".wmv"]),
            "video/mpeg" => return video("video/mpeg", &[".mpg", ".mpeg"]),
            "video/x-flv" => return video("video/x-flv", &[".flv"]),
            "audio/ogg" => return video("video/ogg", &[".ogv", ".ogg"]),
            _ => {}
        }
    }
    // Other ISO-BMFF video brands (`ftyp` box) infer does not list; still images
    // in the same container family (HEIF/AVIF) are excluded.
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        let still = [
            b"heic", b"heix", b"heim", b"heis", b"mif1", b"msf1", b"avif", b"avis",
        ];
        if !still.iter().any(|b| brand == *b) {
            return video("video/mp4", ISO);
        }
    }
    // EBML (Matroska/WebM) whose DocType sits beyond infer's window.
    if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        let head = &bytes[..bytes.len().min(4096)];
        if contains(head, b"\x42\x82\x84webm") {
            return video("video/webm", EBML);
        }
        if contains(head, b"\x42\x82\x88matroska") {
            return video("video/x-matroska", EBML);
        }
    }
    None
}

/// Validate that `bytes` is real media of `kind`, allowed by `allowed`
/// (the configured, normalized extension list). Returns the detected type.
pub fn check(bytes: &[u8], kind: MediaKind, allowed: &[String]) -> Result<Detected, MediaError> {
    let reject = || MediaError::Rejected(rejection_message(kind));
    let detected = detect(bytes)
        .filter(|d| d.kind == kind)
        .ok_or_else(reject)?;
    if !detected
        .extensions
        .iter()
        .any(|ext| allowed.iter().any(|a| a == ext))
    {
        return Err(reject());
    }
    if has_markup(bytes, kind) {
        return Err(reject());
    }
    if kind == MediaKind::Image && !image_header_parses(bytes) {
        return Err(reject());
    }
    Ok(detected)
}

/// Markup in the sniff window, or a `<script` anywhere in an image.
fn has_markup(bytes: &[u8], kind: MediaKind) -> bool {
    let window = bytes[..bytes.len().min(SNIFF_WINDOW)].to_ascii_lowercase();
    if MARKUP_MARKERS.iter().any(|m| contains(&window, m)) {
        return true;
    }
    kind == MediaKind::Image && contains_ignore_case(bytes, b"<script")
}

/// The `image` crate can read the dimensions from the header.
fn image_header_parses(bytes: &[u8]) -> bool {
    image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_dimensions().ok())
        .is_some_and(|(w, h)| w > 0 && h > 0)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn contains_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}
