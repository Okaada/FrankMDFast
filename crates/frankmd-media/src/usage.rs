//! **[COMPAT]** port of `app/services/media_usage_service.rb`: which notes
//! reference which managed media. Golden: `media_usage.json`.
//!
//! Regexes are kept as their Onigmo sources ([`MARKDOWN_DESTINATION`] etc.,
//! golden `constants`); they need look-behind, so the implementation compiles
//! them with `fancy-regex`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::MediaError;

/// `MEDIA_ROOTS`.
pub const MEDIA_ROOTS: [&str; 2] = ["images", "videos"];
/// `MAX_CACHED_ROOTS` (process-wide LRU of indexes in the original).
pub const MAX_CACHED_ROOTS: usize = 8;
/// `MARKDOWN_DESTINATION` (Onigmo source, `/m`).
pub const MARKDOWN_DESTINATION: &str = r"(?<!\\)\[(?:\\.|[^\]\r\n])*\]\([ \t]*(?:<([^>\r\n]*)>|((?:\\.|[^)\s])+))";
/// `REFERENCE_DEFINITION` (Onigmo source).
pub const REFERENCE_DEFINITION: &str = r"\A {0,3}\[([^\]\r\n]+)\]:[ \t]*(?:<([^>\r\n]*)>|([^\s]+))";
/// `REFERENCE_USE` (Onigmo source).
pub const REFERENCE_USE: &str =
    r"(?<!\\)!\[((?:\\.|[^\]\r\n])*)\](?:\[([^\]\r\n]*)\])?|(?<![\\!])\[((?:\\.|[^\]\r\n])*)\](?:\[([^\]\r\n]*)\])?";
/// `HTML_SOURCE` (Onigmo source, `/im`).
pub const HTML_SOURCE: &str = r#"<(?:img|video|audio|source|track|embed|object)\b[^>]*?\s+src\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#;

/// `{ "images/photo.png" => ["notes/a.md", ...] }`: sorted keys, sorted distinct notes.
pub type UsageIndex = BTreeMap<String, Vec<String>>;

/// `usage_counts = index.transform_values(&:length)` (`GET /library/usage`).
pub fn usage_counts(index: &UsageIndex) -> BTreeMap<String, usize> {
    index.iter().map(|(k, v)| (k.clone(), v.len())).collect()
}

/// `MediaUsageService` for one notes root, with the original's fingerprint cache
/// (relative path, dev, ino, size, mtime, ctime of every visible `.md`).
#[derive(Debug)]
pub struct MediaUsage {
    base: PathBuf,
    cache: std::sync::Mutex<Option<(Vec<u8>, Arc<UsageIndex>)>>,
    builds: std::sync::atomic::AtomicUsize,
}

impl MediaUsage {
    /// `MediaUsageService.new(base_path:)`.
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into(), cache: std::sync::Mutex::new(None), builds: std::sync::atomic::AtomicUsize::new(0) }
    }

    /// `build_index`: scans visible, non-symlinked `.md` notes (dot dirs skipped);
    /// an unchanged fingerprint returns the cached index without re-reading.
    /// Any read failure → [`MediaError::Scan`] (`"Could not read Markdown note …"` /
    /// `"Could not inspect Markdown notes: …"`), never a partial index.
    pub fn build_index(&self) -> Result<Arc<UsageIndex>, MediaError> {
        todo!("backend.md §7.9")
    }

    /// **Test seam** (stands in for `expects(:build_index_from).once`): how many
    /// times the notes were actually read and indexed.
    pub fn build_count(&self) -> usize {
        self.builds.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// `references_in(content)`: every destination (markdown inline, used
/// reference definitions, HTML `src`) after code/comments are masked, in order.
pub fn references_in(content: &str) -> Vec<String> {
    todo!()
}

/// `markdown_without_code(content)`: blank out fenced blocks, inline code
/// spans and HTML comments (same length/lines, so offsets stay stable).
pub fn markdown_without_code(content: &str) -> String {
    todo!()
}

/// `normalize_reference_label`: downcase + collapse whitespace + strip.
pub fn normalize_reference_label(label: &str) -> String {
    todo!()
}

/// `canonical_media_path(reference, note_path)`: decode, strip `#`/`?` suffix
/// and Markdown escapes, resolve note-relative / `/images` / `/notes/images`,
/// reject anything escaping the root or outside [`MEDIA_ROOTS`].
pub fn canonical_media_path(reference: &str, note_path: &str) -> Option<String> {
    todo!()
}

/// `strip_unescaped_url_suffix(path)`: drop an unescaped `?query` / `#fragment`.
pub fn strip_unescaped_url_suffix(path: &str) -> String {
    todo!()
}
