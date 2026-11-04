//! **[COMPAT]** port of `app/services/media_usage_service.rb`: which notes
//! reference which managed media. Golden: `media_usage.json`.
//!
//! Regexes are kept as their Onigmo sources ([`MARKDOWN_DESTINATION`] etc.,
//! golden `constants`); they need look-behind, so the implementation compiles
//! them with `fancy-regex`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, PoisonError};

use fancy_regex::Regex;

use crate::error::MediaError;

/// `MEDIA_ROOTS`.
pub const MEDIA_ROOTS: [&str; 2] = ["images", "videos"];
/// `MAX_CACHED_ROOTS` (process-wide LRU of indexes in the original).
pub const MAX_CACHED_ROOTS: usize = 8;
/// `MARKDOWN_DESTINATION` (Onigmo source, `/m`).
pub const MARKDOWN_DESTINATION: &str =
    r"(?<!\\)\[(?:\\.|[^\]\r\n])*\]\([ \t]*(?:<([^>\r\n]*)>|((?:\\.|[^)\s])+))";
/// `REFERENCE_DEFINITION` (Onigmo source).
pub const REFERENCE_DEFINITION: &str = r"\A {0,3}\[([^\]\r\n]+)\]:[ \t]*(?:<([^>\r\n]*)>|([^\s]+))";
/// `REFERENCE_USE` (Onigmo source).
pub const REFERENCE_USE: &str = r"(?<!\\)!\[((?:\\.|[^\]\r\n])*)\](?:\[([^\]\r\n]*)\])?|(?<![\\!])\[((?:\\.|[^\]\r\n])*)\](?:\[([^\]\r\n]*)\])?";
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
        Self {
            base: base.into(),
            cache: std::sync::Mutex::new(None),
            builds: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// `build_index`: scans visible, non-symlinked `.md` notes (dot dirs skipped);
    /// an unchanged fingerprint returns the cached index without re-reading.
    /// Any read failure → [`MediaError::Scan`] (`"Could not read Markdown note …"` /
    /// `"Could not inspect Markdown notes: …"`), never a partial index.
    pub fn build_index(&self) -> Result<Arc<UsageIndex>, MediaError> {
        let inspect =
            |e: std::io::Error| MediaError::Scan(format!("Could not inspect Markdown notes: {e}"));
        let mut files = Vec::new();
        collect_markdown_files(&self.base, &mut files).map_err(inspect)?;
        let fingerprint = self.fingerprint(&files).map_err(inspect)?;

        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((cached, index)) = cache.as_ref()
            && *cached == fingerprint
        {
            return Ok(Arc::clone(index));
        }
        let index = Arc::new(self.build_index_from(&files)?);
        *cache = Some((fingerprint, Arc::clone(&index)));
        Ok(index)
    }

    /// **Test seam** (stands in for `expects(:build_index_from).once`): how many
    /// times the notes were actually read and indexed.
    pub fn build_count(&self) -> usize {
        self.builds.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// `[relpath, dev, ino, size, mtime, ctime]` of every note, serialized.
    fn fingerprint(&self, files: &[PathBuf]) -> std::io::Result<Vec<u8>> {
        let mut out = Vec::new();
        for file in files {
            let meta = std::fs::metadata(file)?;
            out.extend_from_slice(self.relative(file).as_bytes());
            out.push(0);
            out.extend_from_slice(&stat_fingerprint(&meta));
        }
        Ok(out)
    }

    fn relative(&self, file: &Path) -> String {
        file.strip_prefix(&self.base)
            .unwrap_or(file)
            .to_string_lossy()
            .into_owned()
    }

    /// `build_index_from(files)`.
    fn build_index_from(&self, files: &[PathBuf]) -> Result<UsageIndex, MediaError> {
        self.builds
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut usage: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for file in files {
            let note_path = self.relative(file);
            let bytes = std::fs::read(file).map_err(|e| {
                MediaError::Scan(format!("Could not read Markdown note {note_path}: {e}"))
            })?;
            let content = String::from_utf8_lossy(&bytes);
            for reference in references_in(&content) {
                if let Some(media) = canonical_media_path(&reference, &note_path) {
                    usage.entry(media).or_default().insert(note_path.clone());
                }
            }
        }
        Ok(usage
            .into_iter()
            .map(|(media, notes)| (media, notes.into_iter().collect()))
            .collect())
    }
}

/// `collect_markdown_files`: pre-order walk sorted by path; dot entries and
/// **every symlink** skipped (so nothing outside the notes root is ever read);
/// regular `.md` files collected. A directory that cannot be read fails the scan.
fn collect_markdown_files(directory: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_markdown_files(&path, out)?;
        } else if file_type.is_file()
            && crate::server_folder::extname(&entry.file_name().to_string_lossy()) == ".md"
        {
            out.push(path);
        }
    }
    Ok(())
}

fn regex(source: &str) -> Regex {
    // The sources are constants verified by the test suite.
    #[allow(clippy::expect_used)]
    Regex::new(source).expect("valid media usage regex")
}

static MARKDOWN_DESTINATION_RE: LazyLock<Regex> =
    LazyLock::new(|| regex(&format!("(?s){MARKDOWN_DESTINATION}")));
static REFERENCE_DEFINITION_RE: LazyLock<Regex> = LazyLock::new(|| regex(REFERENCE_DEFINITION));
static REFERENCE_USE_RE: LazyLock<Regex> = LazyLock::new(|| regex(REFERENCE_USE));
static HTML_SOURCE_RE: LazyLock<Regex> = LazyLock::new(|| regex(&format!("(?is){HTML_SOURCE}")));
static FOLLOWED_BY_PAREN: LazyLock<Regex> = LazyLock::new(|| regex(r"\A[ \t]*\("));
static FENCE_OPEN: LazyLock<Regex> = LazyLock::new(|| regex(r"\A {0,3}(`{3,}|~{3,})"));
static FENCE_CLOSE: LazyLock<Regex> = LazyLock::new(|| regex(r"\A {0,3}(`+|~+)[ \t]*(?:\r?\n)?\z"));
static HTML_COMMENT: LazyLock<Regex> = LazyLock::new(|| regex(r"<!--[\s\S]*?-->"));
static INLINE_CODE: LazyLock<Regex> = LazyLock::new(|| regex(r"(?s)(?<!\\)(`+).*?\1"));
/// Onigmo `[[:punct:]]` on UTF-8 strings: Unicode punctuation plus the ASCII symbols.
static ESCAPED_PUNCT: LazyLock<Regex> = LazyLock::new(|| regex(r"\\(\p{P}|[$+<=>^`|~])"));
static SPACES: LazyLock<Regex> = LazyLock::new(|| regex(r"\s+"));
static SCHEME_OR_NETWORK: LazyLock<Regex> =
    LazyLock::new(|| regex(r"(?i)\A(?:[a-z][a-z0-9+.-]*:|//)"));

/// Ruby `String#lines`: split after each `\n`, keeping it.
fn lines(content: &str) -> impl Iterator<Item = &str> {
    content.split_inclusive('\n')
}

/// `mask_line`: every char but `\r`/`\n` becomes a space.
fn mask(text: &str) -> String {
    text.chars()
        .map(|c| if c == '\r' || c == '\n' { c } else { ' ' })
        .collect()
}

/// `gsub(re) { mask }` (fancy-regex errors such as backtrack limits leave text as is).
fn mask_matches(re: &Regex, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in re.find_iter(text).flatten() {
        out.push_str(&text[last..m.start()]);
        out.push_str(&mask(m.as_str()));
        last = m.end();
    }
    out.push_str(&text[last..]);
    out
}

/// `gsub(/\\([[:punct:]])/) { $1 }`.
fn unescape_punctuation(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for caps in ESCAPED_PUNCT.captures_iter(text).flatten() {
        let (Some(whole), Some(punct)) = (caps.get(0), caps.get(1)) else {
            continue;
        };
        out.push_str(&text[last..whole.start()]);
        out.push_str(punct.as_str());
        last = whole.end();
    }
    out.push_str(&text[last..]);
    out
}

/// `CGI.unescapeHTML`: `&amp; &quot; &apos; &gt; &lt;` and numeric references.
fn unescape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let decoded = tail.find(';').filter(|&end| end > 1).and_then(|end| {
            let entity = &tail[1..end];
            let ch = match entity {
                "amp" => Some('&'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "gt" => Some('>'),
                "lt" => Some('<'),
                _ => numeric_reference(entity),
            };
            ch.map(|c| (c, end + 1))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &tail[len..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// `#123` / `#x7B` → char.
fn numeric_reference(entity: &str) -> Option<char> {
    let digits = entity.strip_prefix('#')?;
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) if !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()) => {
            u32::from_str_radix(hex, 16).ok()?
        }
        Some(_) => return None,
        None if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) => {
            digits.parse().ok()?
        }
        None => return None,
    };
    char::from_u32(code)
}

/// `references_in(content)`: every destination (markdown inline, used
/// reference definitions, HTML `src`) after code/comments are masked, in order.
pub fn references_in(content: &str) -> Vec<String> {
    let source = markdown_without_code(content);
    let (definitions, source) = extract_reference_definitions(&source);
    let mut out: Vec<String> = MARKDOWN_DESTINATION_RE
        .captures_iter(&source)
        .flatten()
        .filter_map(|caps| {
            caps.get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str().to_owned())
        })
        .collect();
    out.extend(reference_destinations(&source, &definitions));
    out.extend(
        HTML_SOURCE_RE
            .captures_iter(&source)
            .flatten()
            .filter_map(|caps| {
                caps.get(1)
                    .or_else(|| caps.get(2))
                    .or_else(|| caps.get(3))
                    .map(|m| m.as_str().to_owned())
            }),
    );
    out
}

/// `extract_reference_definitions`: first definition per normalized label
/// wins; definition lines are masked.
fn extract_reference_definitions(content: &str) -> (BTreeMap<String, String>, String) {
    let mut definitions = BTreeMap::new();
    let mut rest = String::with_capacity(content.len());
    for line in lines(content) {
        match REFERENCE_DEFINITION_RE.captures(line).ok().flatten() {
            Some(caps) => {
                let label = normalize_reference_label(caps.get(1).map_or("", |m| m.as_str()));
                if let Some(dest) = caps.get(2).or_else(|| caps.get(3)) {
                    definitions
                        .entry(label)
                        .or_insert_with(|| dest.as_str().to_owned());
                }
                rest.push_str(&mask(line));
            }
            None => rest.push_str(line),
        }
    }
    (definitions, rest)
}

/// `reference_destinations`: overlapping scan (cursor = match start + 1 char).
fn reference_destinations(content: &str, definitions: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while cursor <= content.len() {
        let Some(caps) = REFERENCE_USE_RE
            .captures_from_pos(content, cursor)
            .ok()
            .flatten()
        else {
            break;
        };
        let Some(whole) = caps.get(0) else {
            break;
        };
        cursor = next_char_boundary(content, whole.start());
        let first = caps
            .get(1)
            .or_else(|| caps.get(3))
            .map_or("", |m| m.as_str());
        let second = caps.get(2).or_else(|| caps.get(4)).map(|m| m.as_str());
        if second.is_none()
            && FOLLOWED_BY_PAREN
                .is_match(&content[whole.end()..])
                .unwrap_or(false)
        {
            continue;
        }
        let label = match second {
            Some(s) if !s.is_empty() => s,
            _ => first,
        };
        if let Some(dest) = definitions.get(&normalize_reference_label(label)) {
            out.push(dest.clone());
        }
    }
    out
}

fn next_char_boundary(text: &str, at: usize) -> usize {
    text[at..]
        .chars()
        .next()
        .map_or(text.len() + 1, |c| at + c.len_utf8())
}

/// `markdown_without_code(content)`: blank out fenced blocks, inline code
/// spans and HTML comments (same length/lines, so offsets stay stable).
pub fn markdown_without_code(content: &str) -> String {
    let mut fence: Option<String> = None;
    let mut without_fences = String::with_capacity(content.len());
    for line in lines(content) {
        if let Some(open) = &fence {
            let closing = FENCE_CLOSE
                .captures(line)
                .ok()
                .flatten()
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_owned());
            if let Some(close) = closing
                && close.chars().next() == open.chars().next()
                && close.chars().count() >= open.chars().count()
            {
                fence = None;
            }
            without_fences.push_str(&mask(line));
        } else if let Some(marker) = FENCE_OPEN
            .captures(line)
            .ok()
            .flatten()
            .and_then(|c| c.get(1))
        {
            fence = Some(marker.as_str().to_owned());
            without_fences.push_str(&mask(line));
        } else {
            without_fences.push_str(line);
        }
    }
    let without_comments = mask_matches(&HTML_COMMENT, &without_fences);
    mask_matches(&INLINE_CODE, &without_comments)
}

/// `normalize_reference_label`: downcase + collapse whitespace + strip.
pub fn normalize_reference_label(label: &str) -> String {
    let unescaped = unescape_punctuation(&unescape_html(label));
    let stripped = ruby_strip(&unescaped);
    SPACES.replace_all(stripped, " ").to_lowercase()
}

/// Ruby `String#strip` (ASCII whitespace and NUL).
fn ruby_strip(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\0'))
}

/// `canonical_media_path(reference, note_path)`: decode, strip `#`/`?` suffix
/// and Markdown escapes, resolve note-relative / `/images` / `/notes/images`,
/// reject anything escaping the root or outside [`MEDIA_ROOTS`].
pub fn canonical_media_path(reference: &str, note_path: &str) -> Option<String> {
    let path = unescape_html(ruby_strip(reference));
    let path = strip_unescaped_url_suffix(&path);
    let path = unescape_punctuation(&path);
    if path.is_empty() || SCHEME_OR_NETWORK.is_match(&path).unwrap_or(false) {
        return None;
    }
    let absolute = path.starts_with('/');
    // Ruby `split("/")` drops trailing empty fields.
    let mut raw: Vec<&str> = path.split('/').collect();
    while raw.last() == Some(&"") {
        raw.pop();
    }
    let mut raw = raw.as_slice();
    if absolute {
        raw = raw.get(1..).unwrap_or_default();
        if raw.first() == Some(&"notes") {
            raw = &raw[1..];
        }
    }
    let mut segments: Vec<String> = if absolute {
        Vec::new()
    } else {
        match note_path.rsplit_once('/') {
            Some((dir, _)) if !dir.is_empty() => dir.split('/').map(str::to_owned).collect(),
            _ => Vec::new(),
        }
    };
    for raw_segment in raw {
        if raw_segment.is_empty() {
            return None;
        }
        let segment = percent_decode(raw_segment);
        if segment.contains('/') || segment.contains('\\') {
            return None;
        }
        match segment.as_str() {
            "." | "" => {}
            ".." => {
                segments.pop()?;
            }
            _ => segments.push(segment),
        }
    }
    let rooted = segments
        .first()
        .is_some_and(|first| MEDIA_ROOTS.contains(&first.as_str()));
    (rooted && segments.len() > 1).then(|| segments.join("/"))
}

/// `URI::DEFAULT_PARSER.unescape`: `%XX` → byte; invalid UTF-8 decoded lossily.
fn percent_decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| char::from(b).to_digit(16);
        if bytes[i] == b'%'
            && let (Some(Some(hi)), Some(Some(lo))) = (
                bytes.get(i + 1).map(|b| hex(*b)),
                bytes.get(i + 2).map(|b| hex(*b)),
            )
        {
            out.push((hi * 16 + lo) as u8);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `strip_unescaped_url_suffix(path)`: drop an unescaped `?query` / `#fragment`.
pub fn strip_unescaped_url_suffix(path: &str) -> String {
    let mut prefix = String::with_capacity(path.len());
    let mut escaped = false;
    for c in path.chars() {
        if (c == '?' || c == '#') && !escaped {
            break;
        }
        prefix.push(c);
        escaped = if c == '\\' { !escaped } else { false };
    }
    prefix
}

/// `dev, ino, size, mtime, ctime` (with nanoseconds) of one file.
#[cfg(unix)]
fn stat_fingerprint(meta: &std::fs::Metadata) -> Vec<u8> {
    use std::os::unix::fs::MetadataExt;
    let mut out = Vec::with_capacity(56);
    for n in [meta.dev(), meta.ino(), meta.size()] {
        out.extend_from_slice(&n.to_le_bytes());
    }
    for n in [
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
    ] {
        out.extend_from_slice(&n.to_le_bytes());
    }
    out
}

/// Size and modification/creation times (no inode numbers off Unix).
#[cfg(not(unix))]
fn stat_fingerprint(meta: &std::fs::Metadata) -> Vec<u8> {
    let nanos = |t: std::io::Result<std::time::SystemTime>| {
        t.ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos())
    };
    let mut out = meta.len().to_le_bytes().to_vec();
    out.extend_from_slice(&nanos(meta.modified()).to_le_bytes());
    out.extend_from_slice(&nanos(meta.created()).to_le_bytes());
    out
}
