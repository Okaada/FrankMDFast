//! The media library workspace (ui-spec §5.15; FE-346…354).
//!
//! It replaces editor + preview while open. Media is listed only when the
//! library opens (once per open), usage is computed lazily the first time a
//! local card becomes visible (one shared scan, reset on reopen), and every
//! mutation (delete, S3 rename) is confirmed and reported in the status line.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use super::Text;
use super::markup::{self, LibraryEntry, LibraryInsertError, MediaInsert};
use super::picker::Thumb;
use crate::MediaKind;
use crate::library::{LibraryListing, MediaItem, RenamedItem};

/// Images / Videos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Category {
    /// Images.
    #[default]
    Images,
    /// Videos.
    Videos,
}

/// The usage line of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageLabel {
    /// Scan pending (`library.usage_checking`).
    Checking,
    /// Referenced by `n` notes (actionable: opens the "Used in" list).
    Used(usize),
    /// Not referenced ("Not used").
    NotUsed,
    /// S3 items and failed scans (`library.usage_unknown`).
    Unknown,
}

impl UsageLabel {
    /// The text.
    pub fn text(self) -> Text {
        match self {
            Self::Checking => Text::key("library.usage_checking"),
            Self::Used(n) => Text::with("library.usage_used", &[("count", &n.to_string())]),
            Self::NotUsed => Text::key("media_library.not_used"),
            Self::Unknown => Text::key("library.usage_unknown"),
        }
    }
}

/// One card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryCard {
    /// Library path (`images/a.png`) or S3 key; the card's id.
    pub path: String,
    /// Name (plain text).
    pub name: String,
    /// Image or video.
    pub kind: MediaKind,
    /// `formatBytes(size)`.
    pub size: String,
    /// Modification date (`YYYY-MM-DD HH:MM`).
    pub date: String,
    /// Usage line.
    pub usage: UsageLabel,
    /// Preview source.
    pub thumb: Thumb,
    /// Rename is offered (S3 items only).
    pub can_rename: bool,
}

/// Work for the view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryEffect {
    /// List media → [`MediaLibraryView::load_result`].
    Load {
        /// Request generation.
        generation: u64,
    },
    /// Usage counts → [`MediaLibraryView::usage_result`].
    LoadUsage {
        /// Request generation.
        generation: u64,
    },
    /// Notes using one item → [`MediaLibraryView::usage_notes_result`].
    LoadUsageNotes {
        /// Item path.
        path: String,
    },
    /// Copy to the clipboard → [`MediaLibraryView::copy_result`].
    WriteClipboard(String),
    /// Delete → [`MediaLibraryView::delete_result`].
    Delete {
        /// Item path.
        path: String,
    },
    /// S3 rename → [`MediaLibraryView::rename_result`].
    Rename {
        /// Item key.
        path: String,
        /// New file name.
        name: String,
    },
    /// Insert into the open note, close the library and focus the editor.
    Insert(MediaInsert),
    /// Open (and reveal) a note; the library closes.
    OpenNote(String),
    /// A status-line message (`true` = error).
    Status(Text, bool),
}

/// Load state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LoadState {
    /// Not opened yet.
    #[default]
    Idle,
    /// `library.loading`.
    Loading,
    /// Listed.
    Loaded,
    /// Error text.
    Failed(Text),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Usage {
    #[default]
    NotRequested,
    Checking,
    Loaded(BTreeMap<String, usize>),
    Failed,
}

/// A confirmation or prompt the view shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryPrompt {
    /// `library.delete_confirm` (destructive).
    Delete {
        /// Item path.
        path: String,
        /// Item name.
        name: String,
    },
    /// `library.rename_prompt` with the current name as default.
    Rename {
        /// Item key.
        path: String,
        /// Current name.
        name: String,
    },
}

/// The library state.
#[derive(Debug, Clone, Default)]
pub struct MediaLibraryView {
    notes_root: PathBuf,
    open: bool,
    opens: u64,
    generation: u64,
    load: LoadState,
    listing: LibraryListing,
    category: Category,
    filter: String,
    usage: Usage,
    usage_notes: HashMap<String, Vec<String>>,
    detail: Option<String>,
    prompt: Option<LibraryPrompt>,
}

impl MediaLibraryView {
    /// A closed library over `notes_root`.
    pub fn new(notes_root: PathBuf) -> Self {
        Self {
            notes_root,
            ..Self::default()
        }
    }

    /// Open (no-op when already open): reload and reset the usage cache.
    pub fn open(&mut self) -> Vec<LibraryEffect> {
        if self.open {
            return Vec::new();
        }
        self.open = true;
        self.opens += 1;
        self.generation += 1;
        self.load = LoadState::Loading;
        self.usage = Usage::NotRequested;
        self.usage_notes.clear();
        self.detail = None;
        self.prompt = None;
        vec![LibraryEffect::Load {
            generation: self.generation,
        }]
    }

    /// Close.
    pub fn close(&mut self) {
        self.open = false;
        self.detail = None;
        self.prompt = None;
    }

    /// Open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// How many times the library was opened (reload count).
    pub fn open_count(&self) -> u64 {
        self.opens
    }

    /// The listing arrived.
    pub fn load_result(&mut self, generation: u64, result: Result<LibraryListing, Text>) {
        if generation != self.generation {
            return;
        }
        match result {
            Ok(listing) => {
                self.listing = listing;
                self.load = LoadState::Loaded;
            }
            Err(error) => self.load = LoadState::Failed(error),
        }
    }

    /// Load state.
    pub fn load_state(&self) -> &LoadState {
        &self.load
    }

    /// The empty-state text when the active category is empty.
    pub fn empty_text(&self) -> Option<Text> {
        (self.load == LoadState::Loaded && self.items().is_empty())
            .then(|| Text::key("media_library.empty"))
    }

    /// Active category.
    pub fn category(&self) -> Category {
        self.category
    }

    /// Switch category.
    pub fn set_category(&mut self, category: Category) {
        self.category = category;
        self.detail = None;
    }

    /// `(images, videos)` counts.
    pub fn counts(&self) -> (usize, usize) {
        (self.listing.images.len(), self.listing.videos.len())
    }

    /// Name filter (case-insensitive substring).
    pub fn set_filter(&mut self, filter: &str) {
        filter.clone_into(&mut self.filter);
    }

    /// The filter text.
    pub fn filter(&self) -> &str {
        &self.filter
    }

    fn all_items(&self) -> &[MediaItem] {
        match self.category {
            Category::Images => &self.listing.images,
            Category::Videos => &self.listing.videos,
        }
    }

    fn items(&self) -> Vec<&MediaItem> {
        let needle = self.filter.trim().to_lowercase();
        self.all_items()
            .iter()
            .filter(|i| needle.is_empty() || i.name.to_lowercase().contains(&needle))
            .collect()
    }

    fn find(&self, path: &str) -> Option<&MediaItem> {
        self.listing
            .images
            .iter()
            .chain(&self.listing.videos)
            .find(|i| i.path == path)
    }

    /// Cards of the active category, in received order.
    pub fn cards(&self) -> Vec<LibraryCard> {
        self.items()
            .into_iter()
            .map(|item| self.card(item))
            .collect()
    }

    fn card(&self, item: &MediaItem) -> LibraryCard {
        let thumb = match &item.s3_url {
            Some(url) => Thumb::Url(url.clone()),
            None => Thumb::File(self.notes_root.join(&item.path)),
        };
        LibraryCard {
            path: item.path.clone(),
            name: item.name.clone(),
            kind: item.kind,
            size: markup::format_bytes(item.size),
            date: format_date(&item.mtime),
            usage: self.usage_label(item),
            thumb,
            can_rename: item.s3_url.is_some(),
        }
    }

    fn usage_label(&self, item: &MediaItem) -> UsageLabel {
        if item.s3_url.is_some() {
            return UsageLabel::Unknown;
        }
        match &self.usage {
            Usage::NotRequested | Usage::Checking => UsageLabel::Checking,
            Usage::Failed => UsageLabel::Unknown,
            Usage::Loaded(counts) => match counts.get(&item.path).copied().unwrap_or(0) {
                0 => UsageLabel::NotUsed,
                n => UsageLabel::Used(n),
            },
        }
    }

    /// A card scrolled into view: the first local card starts the one usage scan.
    pub fn mark_visible(&mut self, path: &str) -> Vec<LibraryEffect> {
        if self.usage != Usage::NotRequested || self.find(path).is_none_or(|i| i.s3_url.is_some()) {
            return Vec::new();
        }
        self.usage = Usage::Checking;
        vec![LibraryEffect::LoadUsage {
            generation: self.generation,
        }]
    }

    /// Usage counts arrived (only non-negative counts are kept).
    pub fn usage_result(&mut self, generation: u64, result: Result<BTreeMap<String, usize>, Text>) {
        if generation != self.generation {
            return;
        }
        self.usage = match result {
            Ok(counts) => Usage::Loaded(counts),
            Err(_) => Usage::Failed,
        };
    }

    // ----- detail sheet -----------------------------------------------------------------

    /// Enter on a card / click its preview: open the detail sheet.
    pub fn open_detail(&mut self, path: &str) -> Vec<LibraryEffect> {
        let Some(item) = self.find(path) else {
            return Vec::new();
        };
        let local_used = matches!(self.usage_label(item), UsageLabel::Used(_));
        self.detail = Some(path.to_owned());
        if local_used && !self.usage_notes.contains_key(path) {
            return vec![LibraryEffect::LoadUsageNotes {
                path: path.to_owned(),
            }];
        }
        Vec::new()
    }

    /// Close the sheet.
    pub fn close_detail(&mut self) {
        self.detail = None;
    }

    /// The open sheet's card.
    pub fn detail(&self) -> Option<LibraryCard> {
        self.detail
            .as_deref()
            .and_then(|p| self.find(p))
            .map(|i| self.card(i))
    }

    /// Sheet metadata: `size · date[ · ratio][ · FORMAT]` (ratio from the decoded image).
    pub fn detail_metadata(&self, dims: Option<(u32, u32)>) -> Option<String> {
        let item = self.detail.as_deref().and_then(|p| self.find(p))?;
        let mut parts = vec![markup::format_bytes(item.size), format_date(&item.mtime)];
        if item.kind == MediaKind::Image {
            if let Some(ratio) = dims.and_then(|(w, h)| markup::aspect_ratio(w, h)) {
                parts.push(ratio);
            }
            if let Some(format) = markup::format_label(&item.name) {
                parts.push(format);
            }
        }
        Some(parts.join(" · "))
    }

    /// "Used in" notes arrived for `path` (validated relative `.md` paths, deduped).
    pub fn usage_notes_result(&mut self, path: &str, result: Result<Vec<String>, Text>) {
        let notes = match result {
            Ok(notes) => {
                let mut seen = Vec::new();
                for note in notes {
                    if is_relative_note(&note) && !seen.contains(&note) {
                        seen.push(note);
                    }
                }
                seen
            }
            Err(_) => Vec::new(),
        };
        self.usage_notes.insert(path.to_owned(), notes);
    }

    /// The sheet's "Used in" list (`None` while loading).
    pub fn detail_notes(&self) -> Option<&[String]> {
        self.detail
            .as_deref()
            .and_then(|p| self.usage_notes.get(p))
            .map(Vec::as_slice)
    }

    /// Click a note in "Used in".
    pub fn open_note(&mut self, note: &str) -> Vec<LibraryEffect> {
        if !is_relative_note(note) {
            return Vec::new();
        }
        self.close();
        vec![LibraryEffect::OpenNote(note.to_owned())]
    }

    // ----- actions ----------------------------------------------------------------------

    /// Copy path: the S3 URL, or the library path (`images/foo.png`).
    pub fn copy_path(&self, path: &str) -> Vec<LibraryEffect> {
        let Some(item) = self.find(path) else {
            return Vec::new();
        };
        vec![LibraryEffect::WriteClipboard(
            item.s3_url.clone().unwrap_or_else(|| item.path.clone()),
        )]
    }

    /// The clipboard write finished.
    pub fn copy_result(&self, ok: bool) -> Vec<LibraryEffect> {
        if ok {
            vec![LibraryEffect::Status(Text::key("library.copied"), false)]
        } else {
            vec![LibraryEffect::Status(
                Text::key("library.copy_failed"),
                true,
            )]
        }
    }

    /// Delete: ask first.
    pub fn request_delete(&mut self, path: &str) -> Option<Text> {
        let item = self.find(path)?;
        let text = Text::with("library.delete_confirm", &[("name", &item.name)]);
        self.prompt = Some(LibraryPrompt::Delete {
            path: item.path.clone(),
            name: item.name.clone(),
        });
        Some(text)
    }

    /// Rename (S3 only): ask for the new name.
    pub fn request_rename(&mut self, path: &str) -> Option<String> {
        let item = self.find(path).filter(|i| i.s3_url.is_some())?;
        let name = item.name.clone();
        self.prompt = Some(LibraryPrompt::Rename {
            path: item.path.clone(),
            name: name.clone(),
        });
        Some(name)
    }

    /// The open confirmation / prompt.
    pub fn prompt(&self) -> Option<&LibraryPrompt> {
        self.prompt.as_ref()
    }

    /// Cancel the prompt.
    pub fn cancel_prompt(&mut self) {
        self.prompt = None;
    }

    /// Accept the prompt (`text` = the new name for Rename).
    pub fn accept_prompt(&mut self, text: &str) -> Vec<LibraryEffect> {
        match self.prompt.take() {
            Some(LibraryPrompt::Delete { path, .. }) => vec![LibraryEffect::Delete { path }],
            Some(LibraryPrompt::Rename { path, name }) => {
                let new = text.trim();
                if new.is_empty() || new == name {
                    return Vec::new();
                }
                vec![LibraryEffect::Rename {
                    path,
                    name: new.to_owned(),
                }]
            }
            None => Vec::new(),
        }
    }

    /// The delete finished.
    pub fn delete_result(&mut self, path: &str, result: Result<(), Text>) -> Vec<LibraryEffect> {
        match result {
            Ok(()) => {
                self.listing.images.retain(|i| i.path != path);
                self.listing.videos.retain(|i| i.path != path);
                if self.detail.as_deref() == Some(path) {
                    self.detail = None;
                }
                vec![LibraryEffect::Status(Text::key("library.deleted"), false)]
            }
            Err(error) => vec![LibraryEffect::Status(error, true)],
        }
    }

    /// The rename finished: the item is updated in place (references are not).
    pub fn rename_result(
        &mut self,
        path: &str,
        result: Result<RenamedItem, Text>,
    ) -> Vec<LibraryEffect> {
        match result {
            Ok(renamed) => {
                for item in self
                    .listing
                    .images
                    .iter_mut()
                    .chain(self.listing.videos.iter_mut())
                {
                    if item.path == path {
                        item.name = renamed.name.clone();
                        item.path = renamed.path.clone();
                        item.s3_url = Some(renamed.url.clone());
                        item.preview_url = renamed.url.clone();
                    }
                }
                if self.detail.as_deref() == Some(path) {
                    self.detail = Some(renamed.path);
                }
                vec![LibraryEffect::Status(Text::key("library.renamed"), false)]
            }
            Err(error) => vec![LibraryEffect::Status(error, true)],
        }
    }

    /// Insert into the open note (`open_note` = its vault path).
    pub fn insert(&mut self, path: &str, open_note: Option<&str>) -> Vec<LibraryEffect> {
        let Some(item) = self.find(path) else {
            return vec![LibraryEffect::Status(
                Text::key(LibraryInsertError::InvalidMedia.key()),
                true,
            )];
        };
        match markup::library_insert(&LibraryEntry::from(item), open_note) {
            Ok(insert) => {
                self.close();
                vec![LibraryEffect::Insert(insert)]
            }
            Err(error) => vec![LibraryEffect::Status(Text::key(error.key()), true)],
        }
    }
}

/// A usage note path: relative, `.md`, no empty / `.` / `..` / `\` segments.
fn is_relative_note(path: &str) -> bool {
    path.ends_with(".md")
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != ".." && !s.contains('\\'))
}

/// `2026-07-04T10:15:00-03:00` → `2026-07-04 10:15`; other strings unchanged.
pub fn format_date(mtime: &str) -> String {
    match mtime.parse::<jiff::Timestamp>() {
        Ok(ts) => {
            let local = ts.to_zoned(jiff::tz::TimeZone::system());
            local.strftime("%Y-%m-%d %H:%M").to_string()
        }
        Err(_) => mtime.to_owned(),
    }
}
