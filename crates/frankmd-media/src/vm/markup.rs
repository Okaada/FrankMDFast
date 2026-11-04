//! **[COMPAT]** The exact text each media dialog inserts, and where.
//!
//! Ports of `relativeMediaPath`, `encodeRelativeMediaPath`, `safeRemoteMediaUrl`,
//! `escapeMarkdownAlt`, `onImageSelected`, `insertLibraryMedia`
//! (`app_controller.js`), `youtubeEmbedCode` / `insertVideo`
//! (`video_dialog_controller.js`) and the `insertImage` / `insertBlockContent`
//! placement rules (`frankmd_core::text::insertion`). See
//! docs/inventory/frontend.md D1.1, D1.2 and D1.4.

use std::ops::Range;

use frankmd_core::text::cursor::{CursorInfo, CursorPosition, Selection};
use frankmd_core::text::insertion::{BlockInsert, EditorSurface, insert_block_content, insert_image};

use crate::url_utils::encode_uri_component;

/// What a dialog asks the editor to insert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaInsert {
    /// Image Markdown: replaces the selection (or slash range), padded with
    /// single newlines (`insertImage`).
    Image(String),
    /// A block (video embed, HTML): blank-line padded at the caret
    /// (`insertBlockContent`).
    Block(String),
}

impl MediaInsert {
    /// The inserted text without padding.
    pub fn text(&self) -> &str {
        match self {
            Self::Image(text) | Self::Block(text) => text,
        }
    }
}

/// The result of applying a [`MediaInsert`] to a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertEdit {
    /// Byte range of the old text that is replaced.
    pub range: Range<usize>,
    /// Replacement.
    pub text: String,
    /// Caret after the insertion (byte offset in the new text).
    pub caret: usize,
}

/// Where `insert` lands in `text` given the editor `selection` and an optional
/// pending slash-command range (`/image`, `/video`), exactly like FrankMD.
pub fn insertion_edit(
    text: &str,
    selection: Range<usize>,
    slash: Option<Range<usize>>,
    insert: &MediaInsert,
) -> Option<InsertEdit> {
    let mut surface = StringSurface { text: text.to_owned(), selection, last: None };
    match insert {
        MediaInsert::Image(markdown) => insert_image(Some(&mut surface), markdown, slash),
        MediaInsert::Block(content) => {
            let opts = BlockInsert { replace: slash, ..BlockInsert::default() };
            insert_block_content(Some(&mut surface), content, &opts)
        }
    };
    let (range, replacement) = surface.last?;
    Some(InsertEdit { range, text: replacement, caret: surface.selection.start })
}

/// A plain-string [`EditorSurface`] that records the one replace it receives.
struct StringSurface {
    text: String,
    selection: Range<usize>,
    last: Option<(Range<usize>, String)>,
}

impl EditorSurface for StringSurface {
    fn text(&self) -> String {
        self.text.clone()
    }
    fn set_text(&mut self, text: &str) {
        self.last = Some((0..self.text.len(), text.to_owned()));
        self.text = text.to_owned();
    }
    fn selection(&self) -> Selection {
        let text = self.text.get(self.selection.clone()).unwrap_or("").to_owned();
        Selection { from: self.selection.start, to: self.selection.end, text }
    }
    fn set_selection(&mut self, from: usize, to: usize) {
        self.selection = from..to;
    }
    fn cursor_position(&self) -> CursorPosition {
        let offset = self.selection.end;
        let before = &self.text[..offset.min(self.text.len())];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, str::len) + 1;
        CursorPosition { offset, line, column }
    }
    fn cursor_info(&self) -> CursorInfo {
        let position = self.cursor_position();
        CursorInfo { current_line: position.line, total_lines: self.text.matches('\n').count() + 1 }
    }
    fn replace_range(&mut self, text: &str, from: usize, to: usize) {
        self.text.replace_range(from..to, text);
        self.last = Some((from..to, text.to_owned()));
    }
    fn insert_at(&mut self, pos: usize, text: &str) {
        self.replace_range(text, pos, pos);
    }
    fn focus(&mut self) {}
}

// ---------------------------------------------------------------------------
// Paths and escaping
// ---------------------------------------------------------------------------

/// A path segment `relativeMediaPath` refuses: empty, `.`, `..` or containing `\`.
fn bad_segment(segment: &str) -> bool {
    segment.is_empty() || segment == "." || segment == ".." || segment.contains('\\')
}

/// `relativeMediaPath(notePath, mediaPath)`: the media path relative to the
/// note's folder (`blog/post.md` + `images/a.png` → `../images/a.png`).
/// `None` for unsafe paths or media outside `images/` / `videos/`.
pub fn relative_media_path(note_path: &str, media_path: &str) -> Option<String> {
    let note: Vec<&str> = note_path.split('/').collect();
    let media: Vec<&str> = media_path.split('/').collect();
    if note.iter().any(|s| bad_segment(s)) {
        return None;
    }
    if media.len() < 2 || media.iter().any(|s| bad_segment(s)) {
        return None;
    }
    if media[0] != "images" && media[0] != "videos" {
        return None;
    }
    let parents = &note[..note.len() - 1];
    let shared = parents.iter().zip(&media).take_while(|(a, b)| a == b).count();
    let mut out: Vec<&str> = std::iter::repeat_n("..", parents.len() - shared).collect();
    out.extend_from_slice(&media[shared..]);
    Some(out.join("/"))
}

/// `encodeRelativeMediaPath`: `encodeURIComponent` per segment, plus `! ' ( ) *`.
pub fn encode_relative_media_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            let mut out = String::new();
            for c in encode_uri_component(segment).chars() {
                match c {
                    '!' | '\'' | '(' | ')' | '*' => out.push_str(&format!("%{:X}", c as u32)),
                    _ => out.push(c),
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// `safeRemoteMediaUrl`: an http(s) URL without credentials, normalized
/// (WHATWG `href`) and with `(` `)` percent-encoded; `None` otherwise.
pub fn safe_remote_media_url(value: &str) -> Option<String> {
    let url = url::Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") || !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    Some(url.as_str().replace('(', "%28").replace(')', "%29"))
}

/// `escapeMarkdownAlt`: runs of `\r`/`\n` become one space; `\ [ ]` are escaped.
pub fn escape_markdown_alt(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_break = false;
    for c in value.chars() {
        if c == '\r' || c == '\n' {
            if !in_break {
                out.push(' ');
            }
            in_break = true;
            continue;
        }
        in_break = false;
        if matches!(c, '\\' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// JS `String.prototype.substring(0, n)` (UTF-16 code units, never splitting a char).
pub fn js_prefix(text: &str, units: usize) -> &str {
    let mut count = 0;
    for (index, c) in text.char_indices() {
        count += c.len_utf16();
        if count > units {
            return &text[..index];
        }
    }
    text
}

// ---------------------------------------------------------------------------
// Image picker
// ---------------------------------------------------------------------------

/// Upload/Folder/Library default alt: the name without its last extension,
/// `-` and `_` turned into spaces (`name.replace(/\.[^.]+$/, "").replace(/[-_]/g, " ")`).
pub fn file_alt(name: &str) -> String {
    let stem = match name.rfind('.') {
        Some(dot) if dot + 1 < name.len() => &name[..dot],
        _ => name,
    };
    stem.replace(['-', '_'], " ")
}

/// Web/Google/Pinterest default alt: the title with `-`/`_` as spaces, cut to 100 units.
pub fn web_alt(title: &str) -> String {
    js_prefix(&title.replace(['-', '_'], " "), 100).to_owned()
}

/// AI default alt: the prompt, or its first 100 units plus `...`.
pub fn ai_alt(prompt: &str) -> String {
    if prompt.encode_utf16().count() > 100 { format!("{}...", js_prefix(prompt, 100)) } else { prompt.to_owned() }
}

/// The picker's Markdown (`insertImage` in `image_picker_controller.js`):
/// `![alt](url)`, or `[![alt](url)](link)`; the alt is **not** escaped here.
pub fn picker_markdown(alt: &str, url: &str, link: &str) -> String {
    if link.is_empty() { format!("![{alt}]({url})") } else { format!("[![{alt}]({url})]({link})") }
}

/// The full Insert-image result: `alt = alt_input.trim() || name || "Image"`,
/// `link = link_input.trim()`, then the app-side rewrite of local `images/…`
/// uploads to a note-relative, percent-encoded path with an escaped alt.
pub fn final_image_markdown(note_path: Option<&str>, url: &str, alt_input: &str, name: &str, link_input: &str) -> ImageMarkdown {
    let alt = match alt_input.trim() {
        "" if name.is_empty() => "Image".to_owned(),
        "" => name.to_owned(),
        trimmed => trimmed.to_owned(),
    };
    let link = link_input.trim().to_owned();
    let mut markdown = picker_markdown(&alt, url, &link);
    if url.starts_with("images/")
        && let Some(relative) = note_path.and_then(|note| relative_media_path(note, url))
    {
        let safe_alt = escape_markdown_alt(if alt.is_empty() { "Image" } else { &alt });
        markdown = format!("![{safe_alt}]({})", encode_relative_media_path(&relative));
        if !link.is_empty() {
            markdown = format!("[{markdown}]({link})");
        }
    }
    ImageMarkdown { markdown, url: url.to_owned(), alt, link }
}

/// `image-picker:image-selected` detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMarkdown {
    /// What is inserted.
    pub markdown: String,
    /// The stored image URL (before the note-relative rewrite).
    pub url: String,
    /// Alt text used.
    pub alt: String,
    /// Link URL (may be empty).
    pub link: String,
}

// ---------------------------------------------------------------------------
// Video
// ---------------------------------------------------------------------------

/// The default iframe title.
pub const YOUTUBE_DEFAULT_TITLE: &str = "YouTube video player";

/// The title a search result without one gets (`selectYoutubeVideo`).
pub const YOUTUBE_SEARCH_FALLBACK_TITLE: &str = "YouTube video";

/// `youtubeEmbedCode(videoId, title)`: the iframe block, or with Hugo the
/// `{{< youtube … >}}` shortcode (title only when not the default). The
/// title is **not** escaped (**[COMPAT]**, quirk D1.7).
pub fn youtube_embed(video_id: &str, title: Option<&str>, hugo: bool) -> String {
    let title = title.unwrap_or(YOUTUBE_DEFAULT_TITLE);
    if hugo {
        if !title.is_empty() && title != YOUTUBE_DEFAULT_TITLE {
            return format!("{{{{< youtube id=\"{video_id}\" title=\"{title}\" >}}}}");
        }
        return format!("{{{{< youtube id=\"{video_id}\" >}}}}");
    }
    format!(
        "<div class=\"embed-container\">\n  <iframe\n    src=\"https://www.youtube.com/embed/{video_id}\"\n    title=\"{title}\"\n    frameborder=\"0\"\n    allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share\"\n    referrerpolicy=\"strict-origin-when-cross-origin\"\n    allowfullscreen>\n  </iframe>\n</div>"
    )
}

/// Video MIME by extension (lowercase, no dot), `None` when unknown.
pub fn video_mime(extension: &str) -> Option<&'static str> {
    Some(match extension {
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "m4v" => "video/x-m4v",
        "ogv" => "video/ogg",
        _ => return None,
    })
}

/// `url.split(".").pop().toLowerCase()`.
fn last_dot_part(text: &str) -> String {
    text.rsplit('.').next().unwrap_or("").to_lowercase()
}

/// The video dialog's file embed (URL HTML-escaped, MIME defaults to `video/mp4`).
pub fn video_file_embed(url: &str) -> String {
    let mime = video_mime(&last_dot_part(url)).unwrap_or("video/mp4");
    let src = frankmd_core::text::utils::escape_html(Some(url));
    format!("<video controls class=\"video-player\">\n  <source src=\"{src}\" type=\"{mime}\">\n  Your browser does not support the video tag.\n</video>")
}

// ---------------------------------------------------------------------------
// Library insert
// ---------------------------------------------------------------------------

/// A library item as the insert path sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryEntry {
    /// Basename.
    pub name: String,
    /// `images/…`, `videos/…` or an S3 key `frankmd/…`.
    pub path: String,
    /// Image or video.
    pub kind: crate::MediaKind,
    /// S3 object URL (S3 items only).
    pub s3_url: Option<String>,
}

impl From<&crate::library::MediaItem> for LibraryEntry {
    fn from(item: &crate::library::MediaItem) -> Self {
        Self { name: item.name.clone(), path: item.path.clone(), kind: item.kind, s3_url: item.s3_url.clone() }
    }
}

/// Why a library insert did not happen (`event.detail.status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryInsertError {
    /// `library.no_open_note`.
    NoOpenNote,
    /// `library.markdown_note_required`.
    MarkdownNoteRequired,
    /// `library.insertion_failed`.
    InvalidMedia,
}

impl LibraryInsertError {
    /// The locale key shown.
    pub fn key(self) -> &'static str {
        match self {
            Self::NoOpenNote => "library.no_open_note",
            Self::MarkdownNoteRequired => "library.markdown_note_required",
            Self::InvalidMedia => "library.insertion_failed",
        }
    }
}

/// `isValidLibraryMediaItem`.
pub fn is_valid_library_entry(entry: &LibraryEntry) -> bool {
    let segments: Vec<&str> = entry.path.split('/').collect();
    if segments.len() < 2 || segments.iter().any(|s| bad_segment(s)) {
        return false;
    }
    match &entry.s3_url {
        Some(url) => segments[0] == "frankmd" && safe_remote_media_url(url).is_some(),
        None => {
            let root = if entry.kind == crate::MediaKind::Image { "images" } else { "videos" };
            segments[0] == root
        }
    }
}

/// `insertLibraryMedia`: `![escaped name](note-relative or S3 url)` for
/// images, a `<video>` block (no fallback text, `type` only when known) for videos.
pub fn library_insert(entry: &LibraryEntry, open_note: Option<&str>) -> Result<MediaInsert, LibraryInsertError> {
    let note = open_note.ok_or(LibraryInsertError::NoOpenNote)?;
    if !note.to_lowercase().ends_with(".md") {
        return Err(LibraryInsertError::MarkdownNoteRequired);
    }
    if !is_valid_library_entry(entry) {
        return Err(LibraryInsertError::InvalidMedia);
    }
    let url = match &entry.s3_url {
        Some(url) => safe_remote_media_url(url),
        None => Some(encode_relative_media_path(&relative_media_path(note, &entry.path).unwrap_or_default())),
    }
    .filter(|u| !u.is_empty())
    .ok_or(LibraryInsertError::InvalidMedia)?;
    Ok(match entry.kind {
        crate::MediaKind::Image => MediaInsert::Image(format!("![{}]({url})", escape_markdown_alt(&entry.name))),
        crate::MediaKind::Video => {
            let file = entry.path.rsplit('/').next().unwrap_or("");
            let typ = video_mime(&last_dot_part(file)).map(|m| format!(" type=\"{m}\"")).unwrap_or_default();
            MediaInsert::Block(format!("<video controls class=\"video-player\">\n  <source src=\"{url}\"{typ}>\n</video>"))
        }
    })
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

/// `formatBytes`: `N B`, then KB/MB/GB/TB with one decimal below 10 (none for whole numbers).
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    let mut amount = bytes as f64 / 1024.0;
    let mut unit = 0;
    while amount >= 1024.0 && unit < UNITS.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    if amount.fract() == 0.0 || amount >= 10.0 {
        format!("{amount:.0} {}", UNITS[unit])
    } else {
        format!("{amount:.1} {}", UNITS[unit])
    }
}

/// `WxH` badge (ASCII `x`, OQ-65) when both sides are known and non-zero.
pub fn dims_badge(dims: Option<(u32, u32)>) -> Option<String> {
    dims.filter(|(w, h)| *w > 0 && *h > 0).map(|(w, h)| format!("{w}x{h}"))
}

/// `imageAspectRatio`: the reduced `w:h` (e.g. `16:9`).
pub fn aspect_ratio(width: u32, height: u32) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }
    let (mut a, mut b) = (width, height);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    Some(format!("{}:{}", width / a, height / a))
}

/// The uppercased extension of a name (`photo.png` → `PNG`).
pub fn format_label(name: &str) -> Option<String> {
    let (_, ext) = name.rsplit_once('.')?;
    (!ext.is_empty() && ext.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| ext.to_uppercase())
}
