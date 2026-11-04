//! The Insert image dialog (ui-spec §5.6; FE-306…336, FE-358/359).
//!
//! One [`ImagePicker`] per window. The view calls the input methods, runs the
//! returned [`PickerEffect`]s (file chooser, folder read, searches, uploads)
//! off the UI thread and feeds results back with the `*_result` methods.
//! Generations tag every async request so a late answer for an old query is
//! dropped. Grid data is plain text ([`ImageCard`]): names and titles are
//! shown literally, never interpreted as markup.

use std::path::PathBuf;
use std::sync::Arc;

use super::Text;
use super::markup::{self, ImageMarkdown};
use super::storage::{KeyMode, Provider, RowMode, StorageRow};
use crate::library::{PickerCollection, PickerPage};
use crate::search::SearchResults;

/// The image sources, in rail order (Ctrl+1…7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// Upload (drop / choose files / paste).
    Upload,
    /// Note images or the server image folder.
    Library,
    /// A local folder chosen through the portal.
    Folder,
    /// DuckDuckGo.
    Web,
    /// Google Custom Search.
    Google,
    /// Pinterest (DuckDuckGo site filter).
    Pinterest,
    /// AI image generation.
    Ai,
}

impl Source {
    /// Rail order.
    pub const ALL: [Source; 7] =
        [Source::Upload, Source::Library, Source::Folder, Source::Web, Source::Google, Source::Pinterest, Source::Ai];

    /// Position in the rail.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// The tab label key.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Upload => "dialogs.image_picker.tab_drop",
            Self::Library => "dialogs.image_picker.tab_local",
            Self::Folder => "dialogs.image_picker.tab_folder",
            Self::Web => "dialogs.image_picker.tab_web",
            Self::Google => "dialogs.image_picker.tab_google",
            Self::Pinterest => "dialogs.image_picker.tab_pinterest",
            Self::Ai => "dialogs.image_picker.tab_ai",
        }
    }

    /// Element id suffix (`picker-source-<id>`).
    pub fn id(self) -> &'static str {
        match self {
            Self::Upload => "upload",
            Self::Library => "library",
            Self::Folder => "folder",
            Self::Web => "web",
            Self::Google => "google",
            Self::Pinterest => "pinterest",
            Self::Ai => "ai",
        }
    }

    /// The search engine of a search source.
    pub fn engine(self) -> Option<Engine> {
        match self {
            Self::Web => Some(Engine::Web),
            Self::Google => Some(Engine::Google),
            Self::Pinterest => Some(Engine::Pinterest),
            _ => None,
        }
    }
}

/// Web search engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    /// DuckDuckGo images.
    Web,
    /// Google CSE (paged, infinite scroll).
    Google,
    /// Pinterest via DuckDuckGo.
    Pinterest,
}

impl Engine {
    fn source(self) -> Source {
        match self {
            Self::Web => Source::Web,
            Self::Google => Source::Google,
            Self::Pinterest => Source::Pinterest,
        }
    }

    /// Search field placeholder key.
    pub fn placeholder_key(self) -> &'static str {
        match self {
            Self::Web => "dialogs.image_picker.search_web_placeholder",
            Self::Google => "dialogs.image_picker.search_google_placeholder",
            Self::Pinterest => "dialogs.image_picker.search_pinterest_placeholder",
        }
    }
}

/// What the dialog knows about the configuration (from `images_config` and
/// the AI service), read at open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PickerConfig {
    /// Notes root (managed images live under `images/`).
    pub notes_root: PathBuf,
    /// The server image folder, when configured.
    pub images_path: Option<PathBuf>,
    /// The server image folder is usable.
    pub server_images: bool,
    /// Google key + CSE id configured.
    pub google_enabled: bool,
    /// Image generation configured.
    pub ai_enabled: bool,
    /// The image generation model (shown even when not configured).
    pub ai_model: String,
    /// Allowed image extensions (`.png`, …; empty = FrankMD defaults).
    pub image_extensions: Vec<String>,
}

impl PickerConfig {
    /// The source needs setup before it can be used ("Set up" tag).
    pub fn needs_setup(&self, source: Source) -> bool {
        match source {
            Source::Google => !self.google_enabled,
            Source::Ai => !self.ai_enabled,
            _ => false,
        }
    }

    fn extensions(&self) -> Vec<String> {
        if self.image_extensions.is_empty() {
            crate::config::DEFAULT_IMAGE_EXTENSIONS.iter().map(|e| (*e).to_owned()).collect()
        } else {
            self.image_extensions.clone()
        }
    }
}

/// A local file offered for upload (dropped, chosen, pasted, or in a folder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFile {
    /// File name.
    pub name: String,
    /// Where it is on disk (`None` for pasted data).
    pub path: Option<PathBuf>,
    /// Contents when already in memory (paste).
    pub bytes: Option<Arc<Vec<u8>>>,
    /// Size in bytes.
    pub size: u64,
    /// Modification time (ms since the epoch).
    pub modified_ms: i64,
    /// Pixel size, when the header could be read.
    pub dims: Option<(u32, u32)>,
}

/// A thumbnail source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Thumb {
    /// A local file.
    File(PathBuf),
    /// In-memory bytes (paste, AI result).
    Bytes(Arc<Vec<u8>>),
    /// A remote URL (fetched through the egress client).
    Url(String),
}

/// One grid tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCard {
    /// Stable id within its source.
    pub key: String,
    /// Name / title shown (plain text).
    pub label: String,
    /// Source domain caption (search results).
    pub caption: Option<String>,
    /// Thumbnail (`None` = placeholder: unsafe URL or nothing to show).
    pub thumb: Option<Thumb>,
    /// `WxH` badge.
    pub badge: Option<String>,
    /// Tooltip.
    pub tooltip: String,
    /// Selected.
    pub selected: bool,
}

/// A store request run when the user clicks Insert.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreRequest {
    /// `POST /images/upload`.
    Upload {
        /// The file.
        file: LocalFile,
        /// S3 resize.
        resize: Option<f64>,
        /// S3 key.
        s3_key: Option<String>,
    },
    /// `POST /images/import {url}`.
    ImportUrl {
        /// Remote image.
        url: String,
        /// S3 resize.
        resize: Option<f64>,
        /// S3 prefix.
        s3_prefix: Option<String>,
    },
    /// `POST /images/import {path}` (server image folder).
    ImportPath {
        /// `images_path`-relative path.
        path: String,
        /// S3 resize.
        resize: Option<f64>,
        /// S3 key.
        s3_key: Option<String>,
    },
    /// `POST /images/upload_base64` (AI result).
    Base64 {
        /// Base64 data.
        data: String,
        /// MIME type.
        mime: String,
        /// `ai_<ms>.png`.
        filename: String,
        /// S3 prefix.
        s3_prefix: Option<String>,
    },
}

/// Work for the view.
#[derive(Debug, Clone, PartialEq)]
pub enum PickerEffect {
    /// Open the portal file chooser (images, multiple).
    ChooseFiles,
    /// Open the portal folder chooser.
    ChooseFolder,
    /// List a folder's images (non-recursive) → [`ImagePicker::folder_listing`].
    ReadFolder {
        /// Request generation.
        generation: u64,
        /// The folder.
        dir: PathBuf,
    },
    /// Load a library page → [`ImagePicker::library_result`].
    LoadLibrary {
        /// Request generation.
        generation: u64,
        /// Collection.
        collection: PickerCollection,
        /// Filename filter.
        search: String,
        /// Page size.
        limit: usize,
        /// Offset.
        offset: usize,
    },
    /// Delete a library image → [`ImagePicker::delete_result`].
    DeleteImage {
        /// Collection (managed: library delete; server: image-folder delete).
        collection: PickerCollection,
        /// Path.
        path: String,
    },
    /// Search → [`ImagePicker::search_result`].
    Search {
        /// Request generation.
        generation: u64,
        /// Engine.
        engine: Engine,
        /// Query.
        query: String,
        /// Google `start` (1-based).
        start: i64,
    },
    /// Load the AI reference images → [`ImagePicker::ref_images_result`].
    LoadRefImages {
        /// Filename filter.
        search: String,
    },
    /// Generate → [`ImagePicker::generate_result`].
    Generate {
        /// Request generation.
        generation: u64,
        /// Prompt.
        prompt: String,
        /// Reference image (`images_path`-relative).
        reference: Option<String>,
    },
    /// Abort the running generation.
    CancelGenerate,
    /// Store the selection → [`ImagePicker::store_result`].
    Store(StoreRequest),
    /// Re-read the storage provider (after an S3 failure).
    RefreshStorage,
    /// Insert and close.
    Insert(ImageMarkdown),
}

/// A store failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError {
    /// What to show after "Failed to insert image: ".
    pub message: String,
    /// Recovery switched the provider to Local.
    pub fallback: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Target {
    File(LocalFile),
    Managed { url: String },
    Server { path: String, url: String },
    Remote { url: String },
    Generated { data: String, mime: String, filename: String },
}

#[derive(Debug, Clone, PartialEq)]
struct Selected {
    source: Source,
    key: String,
    name: String,
    target: Target,
}

#[derive(Debug, Default, Clone)]
struct UploadState {
    files: Vec<LocalFile>,
    total: usize,
    rejected: Vec<Text>,
    hover: bool,
}

#[derive(Debug, Clone)]
struct LibraryState {
    collection: PickerCollection,
    search: String,
    limit: usize,
    offset: usize,
    page: Option<PickerPage>,
    loading: bool,
    error: Option<Text>,
    confirm: Option<(String, String)>,
}

impl Default for LibraryState {
    fn default() -> Self {
        Self {
            collection: PickerCollection::Managed,
            search: String::new(),
            limit: 10,
            offset: 0,
            page: None,
            loading: false,
            error: None,
            confirm: None,
        }
    }
}

#[derive(Debug, Clone)]
struct FolderState {
    dir: Option<PathBuf>,
    all: Vec<LocalFile>,
    search: String,
    limit: usize,
    offset: usize,
    error: Option<Text>,
    loading: bool,
}

impl Default for FolderState {
    fn default() -> Self {
        Self { dir: None, all: Vec::new(), search: String::new(), limit: 10, offset: 0, error: None, loading: false }
    }
}

#[derive(Debug, Default, Clone)]
struct SearchState {
    query: String,
    results: Vec<crate::search::WebImage>,
    status: Option<Text>,
    status_error: bool,
    searching: bool,
    searched: bool,
    next_start: Option<i64>,
    total: Option<u64>,
}

/// An AI result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiResult {
    /// Base64 image data.
    pub data: String,
    /// MIME type.
    pub mime: String,
    /// Model used.
    pub model: String,
    /// The provider's revised prompt.
    pub revised_prompt: Option<String>,
}

impl AiResult {
    /// The decoded image bytes (preview).
    pub fn decoded(&self) -> Option<Vec<u8>> {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.decode(&self.data).ok()
    }
}

#[derive(Debug, Default, Clone)]
struct AiState {
    prompt: String,
    generating: bool,
    result: Option<AiResult>,
    error: Option<Text>,
    reference: Option<(String, String)>,
    ref_search: String,
    ref_images: Vec<(String, String)>,
    refs_loaded: bool,
}

/// Async request slots (each keeps its own latest generation).
#[derive(Debug, Clone, Copy)]
enum Slot {
    Library,
    Folder,
    Web,
    Google,
    Pinterest,
    Ai,
}

impl From<Engine> for Slot {
    fn from(engine: Engine) -> Self {
        match engine {
            Engine::Web => Self::Web,
            Engine::Google => Self::Google,
            Engine::Pinterest => Self::Pinterest,
        }
    }
}

/// Inline progress row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Busy {
    /// Ingesting a pasted image.
    Paste,
    /// Insert: waiting for the provider save or the upload.
    Insert,
}

/// The Insert image dialog state.
#[derive(Debug, Clone)]
pub struct ImagePicker {
    config: PickerConfig,
    today: Option<jiff::civil::Date>,
    note: Option<String>,
    source: Source,
    selected: Option<Selected>,
    alt: String,
    link: String,
    row: StorageRow,
    busy: Option<Busy>,
    error: Option<Text>,
    waiting_for_storage: bool,
    generation: u64,
    latest: [u64; 6],
    upload: UploadState,
    library: LibraryState,
    folder: FolderState,
    web: SearchState,
    google: SearchState,
    pinterest: SearchState,
    ai: AiState,
}

impl Default for ImagePicker {
    fn default() -> Self {
        Self::new(PickerConfig::default())
    }
}

impl ImagePicker {
    /// A closed picker.
    pub fn new(config: PickerConfig) -> Self {
        Self {
            config,
            today: None,
            note: None,
            source: Source::Upload,
            selected: None,
            alt: String::new(),
            link: String::new(),
            row: StorageRow::default(),
            busy: None,
            error: None,
            waiting_for_storage: false,
            generation: 0,
            latest: [0; 6],
            upload: UploadState::default(),
            library: LibraryState::default(),
            folder: FolderState::default(),
            web: SearchState::default(),
            google: SearchState::default(),
            pinterest: SearchState::default(),
            ai: AiState::default(),
        }
    }

    /// `open()`: fresh configuration, everything reset, Upload active.
    /// `today` is the local date for default S3 keys.
    pub fn open(
        &mut self,
        config: PickerConfig,
        note: Option<String>,
        provider: Provider,
        s3_available: bool,
        today: jiff::civil::Date,
    ) {
        let generation = self.generation;
        *self = Self::new(config);
        self.generation = generation + 1;
        self.note = note;
        self.today = Some(today);
        self.row = StorageRow::new(provider, s3_available);
        if !self.config.server_images {
            self.library.collection = PickerCollection::Managed;
        }
    }

    /// `openWithFile`: open, then ingest `files` into Upload with the first
    /// accepted one selected (paste / drop on the editor, FE-358, §5.6).
    pub fn open_with_files(
        &mut self,
        config: PickerConfig,
        note: Option<String>,
        provider: Provider,
        s3_available: bool,
        today: jiff::civil::Date,
        files: Vec<LocalFile>,
    ) {
        self.open(config, note, provider, s3_available, today);
        self.busy = Some(Busy::Paste);
        self.ingest_external(files);
        self.busy = None;
    }

    /// Fresh configuration read after opening (the dialog opens at once with
    /// the last known one). The Image folder collection falls back to Note
    /// images when it is no longer configured.
    pub fn set_config(&mut self, config: PickerConfig) {
        if !config.server_images {
            self.library.collection = PickerCollection::Managed;
        }
        self.config = config;
    }

    // ----- accessors --------------------------------------------------------------------

    /// Configuration in use.
    pub fn config(&self) -> &PickerConfig {
        &self.config
    }

    /// The active source.
    pub fn source(&self) -> Source {
        self.source
    }

    /// The selection's name (`Selected: <name>`).
    pub fn selected_name(&self) -> Option<&str> {
        self.selected.as_ref().map(|s| s.name.as_str())
    }

    /// The selection's source.
    pub fn selected_source(&self) -> Option<Source> {
        self.selected.as_ref().map(|s| s.source)
    }

    /// Alt input.
    pub fn alt(&self) -> &str {
        &self.alt
    }

    /// Link input.
    pub fn link(&self) -> &str {
        &self.link
    }

    /// The storage row.
    pub fn row(&self) -> &StorageRow {
        &self.row
    }

    /// The storage row, for edits (resize, key, save a copy).
    pub fn row_mut(&mut self) -> &mut StorageRow {
        &mut self.row
    }

    /// The progress row.
    pub fn busy(&self) -> Option<Busy> {
        self.busy
    }

    /// The inline error above the footer.
    pub fn error(&self) -> Option<&Text> {
        self.error.as_ref()
    }

    /// Insert is enabled.
    pub fn can_insert(&self) -> bool {
        self.selected.is_some() && self.busy.is_none()
    }

    /// Edit the alt text.
    pub fn set_alt(&mut self, alt: &str) {
        alt.clone_into(&mut self.alt);
    }

    /// Edit the link URL.
    pub fn set_link(&mut self, link: &str) {
        link.clone_into(&mut self.link);
    }

    fn issue(&mut self, slot: Slot) -> u64 {
        self.generation += 1;
        self.latest[slot as usize] = self.generation;
        self.generation
    }

    fn today(&self) -> jiff::civil::Date {
        self.today.unwrap_or_else(|| jiff::civil::date(2000, 1, 1))
    }

    // ----- sources ----------------------------------------------------------------------

    /// Switch source (`switchToTab` + `activate()`).
    pub fn select_source(&mut self, source: Source) -> Vec<PickerEffect> {
        self.source = source;
        match source {
            Source::Library => vec![self.library_load()],
            Source::Ai if self.config.server_images && !self.ai.refs_loaded => {
                self.ai.refs_loaded = true;
                vec![PickerEffect::LoadRefImages { search: self.ai.ref_search.clone() }]
            }
            _ => Vec::new(),
        }
    }

    /// Next / previous source with wrap (rail Up/Down, Ctrl+Tab, wheel).
    pub fn step_source(&mut self, forward: bool) -> Vec<PickerEffect> {
        let n = Source::ALL.len();
        let index = self.source.index();
        let next = if forward { (index + 1) % n } else { (index + n - 1) % n };
        self.select_source(Source::ALL[next])
    }

    fn select(&mut self, source: Source, key: String, name: String, alt: String, target: Target) {
        let (mode, key_mode, default_key) = match &target {
            Target::File(file) => (RowMode::Upload, KeyMode::Full, Some(file.name.clone())),
            Target::Server { .. } => (RowMode::Reupload, KeyMode::Full, Some(name.clone())),
            Target::Managed { .. } => (RowMode::Upload, KeyMode::Full, None),
            Target::Remote { .. } => (RowMode::Reupload, KeyMode::Prefix, None),
            Target::Generated { .. } => (RowMode::S3Only, KeyMode::Prefix, None),
        };
        self.row.set_mode(mode, key_mode);
        let today = self.today();
        match default_key {
            Some(file) => self.row.set_default_key(&crate::s3_key::default_s3_key(&file, today)),
            None => self.row.set_default_key(&crate::s3_key::default_s3_prefix(today)),
        }
        self.selected = Some(Selected { source, key, name, target });
        self.alt = alt;
    }

    fn deselect(&mut self, source: Source) -> bool {
        if self.selected.as_ref().is_some_and(|s| s.source == source) {
            self.selected = None;
            return true;
        }
        false
    }

    /// Whether the managed image (collection Managed) is selected: no storage row.
    pub fn storage_row_visible(&self) -> bool {
        !matches!(self.selected.as_ref().map(|s| &s.target), Some(Target::Managed { .. }) | None)
    }

    // ----- Upload ------------------------------------------------------------------------

    /// Files dropped on / chosen in the Upload tab: replaces the grid and
    /// clears the selection. Returns `true` when a selection was dropped
    /// (`deselected`).
    pub fn ingest(&mut self, files: Vec<LocalFile>) -> bool {
        let allowed = self.config.extensions();
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        for file in files {
            let lower = file.name.to_lowercase();
            if allowed.iter().any(|ext| lower.ends_with(ext.as_str())) {
                accepted.push(file);
            } else {
                let ext = match file.name.rfind('.') {
                    Some(dot) if dot + 1 < file.name.len() => file.name[dot..].to_lowercase(),
                    _ => String::new(),
                };
                let ext = if ext.is_empty() { Text::key("dialogs.image_picker.no_file_extension") } else { Text::Raw(ext) };
                rejected.push(Text::Key(
                    "dialogs.image_picker.drop_rejected",
                    vec![("name", Text::Raw(file.name.clone())), ("ext", ext), ("list", Text::Raw(allowed.join(", ")))],
                ));
            }
        }
        accepted.sort_by_key(|f| std::cmp::Reverse(f.modified_ms));
        self.upload.total = accepted.len();
        accepted.truncate(10);
        self.upload.files = accepted;
        self.upload.rejected = rejected;
        self.upload.hover = false;
        self.deselect(Source::Upload)
    }

    /// Paste / editor drop: ingest and pre-select the first accepted file.
    pub fn ingest_external(&mut self, files: Vec<LocalFile>) {
        self.source = Source::Upload;
        self.ingest(files);
        if let Some(first) = self.upload.files.first().cloned() {
            self.select_upload(0, first);
        }
    }

    fn select_upload(&mut self, index: usize, file: LocalFile) {
        let alt = markup::file_alt(&file.name);
        self.select(Source::Upload, format!("upload-{index}"), file.name.clone(), alt, Target::File(file));
    }

    /// Per-file rejection lines (`drop_rejected`).
    pub fn upload_rejections(&self) -> &[Text] {
        &self.upload.rejected
    }

    /// Files are dragged over the dropzone.
    pub fn set_dropzone_hover(&mut self, hover: bool) {
        self.upload.hover = hover;
    }

    /// Dropzone highlight.
    pub fn dropzone_hover(&self) -> bool {
        self.upload.hover
    }

    /// Enter/Space/click on the dropzone.
    pub fn choose_files(&self) -> Vec<PickerEffect> {
        vec![PickerEffect::ChooseFiles]
    }

    /// Upload grid.
    pub fn upload_cards(&self) -> Vec<ImageCard> {
        self.file_cards(Source::Upload, &self.upload.files)
    }

    /// Upload status ("N images found" / "Showing 10 most recent of N images").
    pub fn upload_status(&self) -> Option<Text> {
        if self.upload.files.is_empty() && self.upload.total == 0 {
            return None;
        }
        Some(count_status(self.upload.files.len(), self.upload.total))
    }

    fn file_cards(&self, source: Source, files: &[LocalFile]) -> Vec<ImageCard> {
        let selected = self.selected.as_ref().filter(|s| s.source == source).map(|s| s.key.as_str());
        let prefix = source.id();
        files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                let key = format!("{prefix}-{index}");
                let badge = markup::dims_badge(file.dims);
                let kb = (file.size as f64 / 1024.0).round() as u64;
                let tooltip = match &badge {
                    Some(b) => format!("{} ({b}) - {kb} KB", file.name),
                    None => format!("{} - {kb} KB", file.name),
                };
                let thumb = match (&file.path, &file.bytes) {
                    (Some(path), _) => Some(Thumb::File(path.clone())),
                    (None, Some(bytes)) => Some(Thumb::Bytes(Arc::clone(bytes))),
                    (None, None) => None,
                };
                ImageCard {
                    selected: selected == Some(key.as_str()),
                    key,
                    label: file.name.clone(),
                    caption: None,
                    thumb,
                    badge,
                    tooltip,
                }
            })
            .collect()
    }

    // ----- selection from a card --------------------------------------------------------

    /// Click / Enter on a card of the active source.
    pub fn select_card(&mut self, key: &str) {
        let source = self.source;
        match source {
            Source::Upload | Source::Folder => {
                let files = if source == Source::Upload { &self.upload.files } else { &self.folder_page_files() };
                let index = key.rsplit('-').next().and_then(|i| i.parse::<usize>().ok());
                if let Some(file) = index.and_then(|i| files.get(i)).cloned() {
                    let alt = markup::file_alt(&file.name);
                    self.select(source, key.to_owned(), file.name.clone(), alt, Target::File(file));
                }
            }
            Source::Library => {
                let Some(page) = &self.library.page else { return };
                let Some(image) = page.images.iter().find(|i| i.path == key).cloned() else { return };
                let alt = markup::file_alt(&image.name);
                let target = match page.collection {
                    PickerCollection::Managed => Target::Managed { url: image.url.clone() },
                    PickerCollection::Server => Target::Server { path: image.path.clone(), url: image.url.clone() },
                };
                self.select(source, key.to_owned(), image.name, alt, target);
            }
            Source::Web | Source::Google | Source::Pinterest => {
                let state = self.search_state(source);
                let index = key.parse::<usize>().ok();
                let Some(image) = index.and_then(|i| state.results.get(i)).cloned() else { return };
                let title = image.title.clone().unwrap_or_default();
                let name = if title.is_empty() { "Image".to_owned() } else { title.clone() };
                let alt = markup::web_alt(&title);
                self.select(source, key.to_owned(), name, alt, Target::Remote { url: image.url });
            }
            Source::Ai => {}
        }
    }

    // ----- Library -----------------------------------------------------------------------

    fn library_load(&mut self) -> PickerEffect {
        let generation = self.issue(Slot::Library);
        self.library.loading = true;
        self.deselect(Source::Library);
        PickerEffect::LoadLibrary {
            generation,
            collection: self.library.collection,
            search: self.library.search.trim().to_owned(),
            limit: self.library.limit,
            offset: self.library.offset,
        }
    }

    /// Collection select (Image folder disabled when not configured).
    pub fn set_collection(&mut self, collection: PickerCollection) -> Vec<PickerEffect> {
        if collection == PickerCollection::Server && !self.config.server_images {
            return Vec::new();
        }
        self.library.collection = collection;
        self.library.offset = 0;
        vec![self.library_load()]
    }

    /// The collection.
    pub fn collection(&self) -> PickerCollection {
        self.library.collection
    }

    /// Why "Server image folder" is disabled.
    pub fn server_collection_disabled_reason(&self) -> Option<Text> {
        (!self.config.server_images).then(|| Text::key("dialogs.image_picker.local_path_required"))
    }

    /// Filename search (the view debounces 300 ms).
    pub fn set_library_search(&mut self, search: &str) -> Vec<PickerEffect> {
        search.clone_into(&mut self.library.search);
        self.library.offset = 0;
        vec![self.library_load()]
    }

    /// Per page 10 / 25 / 50.
    pub fn set_library_per_page(&mut self, limit: usize) -> Vec<PickerEffect> {
        self.library.limit = limit.clamp(1, 100);
        self.library.offset = 0;
        vec![self.library_load()]
    }

    /// Per page value.
    pub fn library_per_page(&self) -> usize {
        self.library.limit
    }

    /// Previous / Next.
    pub fn library_page(&mut self, next: bool) -> Vec<PickerEffect> {
        let total = self.library.page.as_ref().map_or(0, |p| p.total);
        if next {
            if self.library.offset + self.library.limit >= total {
                return Vec::new();
            }
            self.library.offset += self.library.limit;
        } else {
            if self.library.offset == 0 {
                return Vec::new();
            }
            self.library.offset = self.library.offset.saturating_sub(self.library.limit);
        }
        vec![self.library_load()]
    }

    /// A library page arrived.
    pub fn library_result(&mut self, generation: u64, result: Result<PickerPage, Text>) {
        if generation != self.latest[Slot::Library as usize] {
            return;
        }
        self.library.loading = false;
        match result {
            Ok(page) => {
                self.library.error = None;
                if !page.server_available && page.collection == PickerCollection::Server {
                    self.library.error = Some(Text::key("dialogs.image_picker.local_path_required"));
                }
                self.library.page = Some(page);
            }
            Err(error) => {
                self.library.page = None;
                self.library.error = Some(error);
            }
        }
    }

    /// Loading indicator.
    pub fn library_loading(&self) -> bool {
        self.library.loading
    }

    /// Inline error of the Library source.
    pub fn library_error(&self) -> Option<&Text> {
        self.library.error.as_ref()
    }

    /// The search text.
    pub fn library_search(&self) -> &str {
        &self.library.search
    }

    /// Library grid (keys are the image paths).
    pub fn library_cards(&self) -> Vec<ImageCard> {
        let Some(page) = &self.library.page else { return Vec::new() };
        let selected = self.selected.as_ref().filter(|s| s.source == Source::Library).map(|s| s.key.as_str());
        page.images
            .iter()
            .map(|image| {
                let root = match page.collection {
                    PickerCollection::Managed => Some(self.config.notes_root.clone()),
                    PickerCollection::Server => self.config.images_path.clone(),
                };
                ImageCard {
                    key: image.path.clone(),
                    label: image.name.clone(),
                    caption: None,
                    thumb: root.map(|r| Thumb::File(r.join(&image.path))),
                    badge: None,
                    tooltip: image.name.clone(),
                    selected: selected == Some(image.path.as_str()),
                }
            })
            .collect()
    }

    /// Pager: `(from, to, total, can_prev, can_next)` ("Showing a–b of N").
    pub fn library_pager(&self) -> Option<(usize, usize, usize, bool, bool)> {
        let page = self.library.page.as_ref()?;
        if page.total == 0 {
            return None;
        }
        let from = page.offset + 1;
        let to = (page.offset + page.images.len()).min(page.total);
        Some((from, to, page.total, page.offset > 0, page.offset + page.limit < page.total))
    }

    /// Trash button: ask first (`dialogs.image_picker.delete_confirm`).
    pub fn request_delete(&mut self, path: &str) -> Option<Text> {
        let page = self.library.page.as_ref()?;
        let image = page.images.iter().find(|i| i.path == path)?;
        self.library.confirm = Some((image.path.clone(), image.name.clone()));
        Some(Text::with("dialogs.image_picker.delete_confirm", &[("name", &image.name)]))
    }

    /// The pending delete confirmation `(path, name)`.
    pub fn pending_delete(&self) -> Option<(&str, &str)> {
        self.library.confirm.as_ref().map(|(p, n)| (p.as_str(), n.as_str()))
    }

    /// The confirmation answered.
    pub fn confirm_delete(&mut self, accept: bool) -> Vec<PickerEffect> {
        let Some((path, _)) = self.library.confirm.take() else { return Vec::new() };
        if !accept {
            return Vec::new();
        }
        vec![PickerEffect::DeleteImage { collection: self.library.collection, path }]
    }

    /// The delete finished: reload (one page back when the page emptied).
    pub fn delete_result(&mut self, result: Result<(), Text>) -> Vec<PickerEffect> {
        match result {
            Ok(()) => {
                let last_on_page = self.library.page.as_ref().is_some_and(|p| p.images.len() == 1);
                if last_on_page && self.library.offset > 0 {
                    self.library.offset = self.library.offset.saturating_sub(self.library.limit);
                }
                vec![self.library_load()]
            }
            Err(error) => {
                self.library.error = Some(error);
                Vec::new()
            }
        }
    }

    // ----- Folder ------------------------------------------------------------------------

    /// "Choose folder…".
    pub fn choose_folder(&self) -> Vec<PickerEffect> {
        vec![PickerEffect::ChooseFolder]
    }

    /// The chooser answered: `Ok(None)` = cancelled (nothing changes).
    pub fn folder_chosen(&mut self, result: Result<Option<PathBuf>, Text>) -> Vec<PickerEffect> {
        match result {
            Ok(None) => Vec::new(),
            Ok(Some(dir)) => {
                self.folder = FolderState { dir: Some(dir.clone()), loading: true, ..FolderState::default() };
                self.deselect(Source::Folder);
                vec![PickerEffect::ReadFolder { generation: self.issue(Slot::Folder), dir }]
            }
            Err(_) => {
                self.folder.error = Some(Text::key("dialogs.image_picker.folder_access_failed"));
                Vec::new()
            }
        }
    }

    /// The folder's files (all of them; filtered here by the allow-list,
    /// sorted newest first), or a read failure.
    pub fn folder_listing(&mut self, generation: u64, result: Result<Vec<LocalFile>, Text>) {
        if generation != self.latest[Slot::Folder as usize] {
            return;
        }
        self.folder.loading = false;
        match result {
            Ok(files) => {
                let allowed = self.config.extensions();
                let mut files: Vec<LocalFile> = files
                    .into_iter()
                    .filter(|f| {
                        let lower = f.name.to_lowercase();
                        allowed.iter().any(|e| lower.ends_with(e.as_str()))
                    })
                    .collect();
                files.sort_by_key(|f| std::cmp::Reverse(f.modified_ms));
                self.folder.all = files;
                self.folder.error = None;
            }
            Err(_) => {
                self.folder.all.clear();
                self.folder.error = Some(Text::key("dialogs.image_picker.folder_read_failed"));
            }
        }
    }

    /// The chosen folder.
    pub fn folder_dir(&self) -> Option<&PathBuf> {
        self.folder.dir.as_ref()
    }

    /// Folder error.
    pub fn folder_error(&self) -> Option<&Text> {
        self.folder.error.as_ref()
    }

    /// Folder search (debounced by the view); clears the selection.
    pub fn set_folder_search(&mut self, search: &str) {
        search.clone_into(&mut self.folder.search);
        self.folder.offset = 0;
        self.deselect(Source::Folder);
    }

    /// The folder search text.
    pub fn folder_search(&self) -> &str {
        &self.folder.search
    }

    /// Per page.
    pub fn set_folder_per_page(&mut self, limit: usize) {
        self.folder.limit = limit.max(1);
        self.folder.offset = 0;
        self.deselect(Source::Folder);
    }

    /// Folder per page.
    pub fn folder_per_page(&self) -> usize {
        self.folder.limit
    }

    fn folder_filtered(&self) -> Vec<&LocalFile> {
        let term = self.folder.search.trim().to_lowercase();
        self.folder.all.iter().filter(|f| term.is_empty() || f.name.to_lowercase().contains(&term)).collect()
    }

    fn folder_page_files(&self) -> Vec<LocalFile> {
        self.folder_filtered().into_iter().skip(self.folder.offset).take(self.folder.limit).cloned().collect()
    }

    /// Previous / Next.
    pub fn folder_page(&mut self, next: bool) {
        let total = self.folder_filtered().len();
        if next && self.folder.offset + self.folder.limit < total {
            self.folder.offset += self.folder.limit;
        } else if !next && self.folder.offset > 0 {
            self.folder.offset = self.folder.offset.saturating_sub(self.folder.limit);
        } else {
            return;
        }
        self.deselect(Source::Folder);
    }

    /// Folder grid.
    pub fn folder_cards(&self) -> Vec<ImageCard> {
        self.file_cards(Source::Folder, &self.folder_page_files())
    }

    /// Folder pager `(from, to, total, can_prev, can_next)`.
    pub fn folder_pager(&self) -> Option<(usize, usize, usize, bool, bool)> {
        let total = self.folder_filtered().len();
        if total == 0 {
            return None;
        }
        let shown = self.folder_page_files().len();
        let from = self.folder.offset + 1;
        Some((from, self.folder.offset + shown, total, self.folder.offset > 0, self.folder.offset + self.folder.limit < total))
    }

    /// Folder empty state (`no_images_in_folder`) once a folder was read.
    pub fn folder_empty(&self) -> bool {
        self.folder.dir.is_some() && !self.folder.loading && self.folder.error.is_none() && self.folder_filtered().is_empty()
    }

    // ----- Web / Google / Pinterest ------------------------------------------------------

    fn search_state(&self, source: Source) -> &SearchState {
        match source {
            Source::Google => &self.google,
            Source::Pinterest => &self.pinterest,
            _ => &self.web,
        }
    }

    fn search_state_mut(&mut self, engine: Engine) -> &mut SearchState {
        match engine {
            Engine::Web => &mut self.web,
            Engine::Google => &mut self.google,
            Engine::Pinterest => &mut self.pinterest,
        }
    }

    /// The query field.
    pub fn query(&self, engine: Engine) -> &str {
        &self.search_state(engine.source()).query
    }

    /// Edit the query field.
    pub fn set_query(&mut self, engine: Engine, query: &str) {
        query.clone_into(&mut self.search_state_mut(engine).query);
    }

    /// Enter / Search button.
    pub fn search(&mut self, engine: Engine) -> Vec<PickerEffect> {
        if engine == Engine::Google && !self.config.google_enabled {
            return Vec::new();
        }
        let generation = self.issue(Slot::from(engine));
        let source = engine.source();
        self.deselect(source);
        let state = self.search_state_mut(engine);
        let query = state.query.trim().to_owned();
        if query.is_empty() {
            state.status = Some(Text::key("dialogs.image_picker.search_keywords_required"));
            state.status_error = false;
            state.searching = false;
            return Vec::new();
        }
        state.results.clear();
        state.searching = true;
        state.status = Some(Text::key("status.searching"));
        state.status_error = false;
        state.next_start = None;
        vec![PickerEffect::Search { generation, engine, query, start: 1 }]
    }

    /// Google infinite scroll: within 100 px of the bottom.
    pub fn load_more(&mut self, engine: Engine) -> Vec<PickerEffect> {
        if engine != Engine::Google {
            return Vec::new();
        }
        let state = &self.google;
        let Some(start) = state.next_start.filter(|_| !state.searching && !state.results.is_empty()) else {
            return Vec::new();
        };
        let generation = self.issue(Slot::Google);
        self.google.searching = true;
        vec![PickerEffect::Search { generation, engine, query: self.google.query.trim().to_owned(), start }]
    }

    /// Search results (or an error message).
    pub fn search_result(&mut self, engine: Engine, generation: u64, start: i64, result: Result<SearchResults, Text>) {
        if generation != self.latest[Slot::from(engine) as usize] {
            return;
        }
        let state = self.search_state_mut(engine);
        state.searching = false;
        state.searched = true;
        match result {
            Ok(results) => {
                if let Some(error) = results.error.filter(|e| !e.is_empty()).filter(|_| results.images.is_empty()) {
                    state.status = Some(Text::Raw(error));
                    state.status_error = true;
                    return;
                }
                let safe = results.images.into_iter();
                if start > 1 {
                    state.results.extend(safe);
                } else {
                    state.results = safe.collect();
                }
                state.next_start = results.next_start;
                state.total = results.total;
                let count = match (engine, results.total) {
                    (Engine::Google, Some(total)) => total as usize,
                    _ => state.results.len(),
                };
                state.status_error = false;
                state.status = Some(if state.results.is_empty() {
                    match results.note.filter(|n| !n.is_empty()) {
                        Some(note) => Text::Raw(note),
                        None => Text::key("dialogs.image_picker.no_images_found"),
                    }
                } else if count == 1 {
                    Text::key("dialogs.image_picker.search_result_one")
                } else {
                    Text::with("dialogs.image_picker.search_result_many", &[("count", &count.to_string())])
                });
            }
            Err(error) => {
                state.status = Some(error);
                state.status_error = true;
            }
        }
    }

    /// Status line and whether it is an error.
    pub fn search_status(&self, engine: Engine) -> (Option<&Text>, bool) {
        let state = self.search_state(engine.source());
        (state.status.as_ref(), state.status_error)
    }

    /// A request is running.
    pub fn searching(&self, engine: Engine) -> bool {
        self.search_state(engine.source()).searching
    }

    /// Result grid (keys are result indexes). Unsafe thumbnail URLs get no thumbnail.
    pub fn search_cards(&self, engine: Engine) -> Vec<ImageCard> {
        let source = engine.source();
        let selected = self.selected.as_ref().filter(|s| s.source == source).map(|s| s.key.clone());
        self.search_state(source)
            .results
            .iter()
            .enumerate()
            .map(|(index, image)| {
                let thumb_url = image.thumbnail.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| image.url.clone());
                let safe = crate::picker::is_safe_image_url(&thumb_url) && !thumb_url.starts_with('/');
                let title = image.title.clone().unwrap_or_default();
                let badge = match (image.width, image.height) {
                    (Some(w), Some(h)) => markup::dims_badge(Some((
                        u32::try_from(w).unwrap_or(u32::MAX),
                        u32::try_from(h).unwrap_or(u32::MAX),
                    ))),
                    _ => None,
                };
                ImageCard {
                    key: index.to_string(),
                    label: title.clone(),
                    caption: image.source.clone().filter(|s| !s.is_empty()),
                    thumb: safe.then_some(Thumb::Url(thumb_url)),
                    badge,
                    tooltip: if title.is_empty() { "Image".to_owned() } else { title },
                    selected: selected.as_deref() == Some(index.to_string().as_str()),
                }
            })
            .collect()
    }

    /// A thumbnail failed to load: the card is removed (FE-329).
    pub fn thumbnail_failed(&mut self, engine: Engine, key: &str) {
        let Ok(index) = key.parse::<usize>() else { return };
        let source = engine.source();
        let state = self.search_state_mut(engine);
        if index >= state.results.len() {
            return;
        }
        state.results.remove(index);
        if let Some(selected) = &self.selected
            && selected.source == source
        {
            let chosen = selected.key.parse::<usize>().ok();
            match chosen {
                Some(i) if i == index => self.selected = None,
                Some(i) if i > index => {
                    if let Some(s) = self.selected.as_mut() {
                        s.key = (i - 1).to_string();
                    }
                }
                _ => {}
            }
        }
    }

    // ----- AI ----------------------------------------------------------------------------

    /// The prompt.
    pub fn prompt(&self) -> &str {
        &self.ai.prompt
    }

    /// Edit the prompt.
    pub fn set_prompt(&mut self, prompt: &str) {
        prompt.clone_into(&mut self.ai.prompt);
    }

    /// Enter (without Shift) / Generate.
    pub fn generate(&mut self) -> Vec<PickerEffect> {
        if self.ai.generating {
            return Vec::new();
        }
        let prompt = self.ai.prompt.trim().to_owned();
        if prompt.is_empty() {
            self.ai.error = Some(Text::key("dialogs.image_picker.ai_prompt_required"));
            return Vec::new();
        }
        if !self.config.ai_enabled {
            self.ai.error = Some(Text::key("dialogs.image_picker.ai_not_configured"));
            return Vec::new();
        }
        self.ai.error = None;
        self.ai.result = None;
        self.ai.generating = true;
        self.deselect(Source::Ai);
        let generation = self.issue(Slot::Ai);
        vec![PickerEffect::Generate { generation, prompt, reference: self.ai.reference.as_ref().map(|r| r.0.clone()) }]
    }

    /// Esc while generating.
    pub fn cancel_generate(&mut self) -> Vec<PickerEffect> {
        if !self.ai.generating {
            return Vec::new();
        }
        self.ai.generating = false;
        self.issue(Slot::Ai);
        vec![PickerEffect::CancelGenerate]
    }

    /// Generating.
    pub fn generating(&self) -> bool {
        self.ai.generating
    }

    /// The generation finished. `now_ms` names the file `ai_<ms>.png`.
    pub fn generate_result(&mut self, generation: u64, result: Result<AiResult, Text>, now_ms: i64) {
        if generation != self.latest[Slot::Ai as usize] || !self.ai.generating {
            return;
        }
        self.ai.generating = false;
        match result {
            Ok(result) => {
                let filename = format!("ai_{now_ms}.png");
                let alt = markup::ai_alt(self.ai.prompt.trim());
                let target = Target::Generated { data: result.data.clone(), mime: result.mime.clone(), filename: filename.clone() };
                self.ai.result = Some(result);
                self.select(Source::Ai, "ai-result".to_owned(), filename, alt, target);
            }
            Err(error) => self.ai.error = Some(error),
        }
    }

    /// The result.
    pub fn ai_result(&self) -> Option<&AiResult> {
        self.ai.result.as_ref()
    }

    /// AI error.
    pub fn ai_error(&self) -> Option<&Text> {
        self.ai.error.as_ref()
    }

    /// Reference search (debounced by the view).
    pub fn set_ref_search(&mut self, search: &str) -> Vec<PickerEffect> {
        search.clone_into(&mut self.ai.ref_search);
        vec![PickerEffect::LoadRefImages { search: search.trim().to_owned() }]
    }

    /// Reference images `(path, name)`.
    pub fn ref_images_result(&mut self, images: Vec<(String, String)>) {
        self.ai.ref_images = images;
    }

    /// Reference rows (empty → `no_images_found`).
    pub fn ref_cards(&self) -> Vec<ImageCard> {
        let chosen = self.ai.reference.as_ref().map(|r| r.0.as_str());
        self.ai
            .ref_images
            .iter()
            .map(|(path, name)| ImageCard {
                key: path.clone(),
                label: name.clone(),
                caption: None,
                thumb: self.config.images_path.as_ref().map(|r| Thumb::File(r.join(path))),
                badge: None,
                tooltip: name.clone(),
                selected: chosen == Some(path.as_str()),
            })
            .collect()
    }

    /// Pick a reference image.
    pub fn choose_ref(&mut self, path: &str) {
        if let Some((p, n)) = self.ai.ref_images.iter().find(|(p, _)| p == path) {
            self.ai.reference = Some((p.clone(), n.clone()));
        }
    }

    /// "Remove reference".
    pub fn clear_ref(&mut self) -> Vec<PickerEffect> {
        self.ai.reference = None;
        vec![PickerEffect::LoadRefImages { search: self.ai.ref_search.trim().to_owned() }]
    }

    /// The reference `(path, name)`.
    pub fn reference(&self) -> Option<(&str, &str)> {
        self.ai.reference.as_ref().map(|(p, n)| (p.as_str(), n.as_str()))
    }

    // ----- storage + insert --------------------------------------------------------------

    /// The global provider was acknowledged / refreshed.
    pub fn storage_changed(&mut self, provider: Provider, s3_available: bool) {
        self.row.set_s3_available(s3_available);
        self.row.set_provider(provider);
    }

    /// Insert. `ready` = no provider save pending (else the insert waits for
    /// [`Self::resume_insert`]).
    pub fn insert(&mut self, ready: bool) -> Vec<PickerEffect> {
        if self.selected.is_none() || self.busy.is_some() {
            return Vec::new();
        }
        self.error = None;
        self.busy = Some(Busy::Insert);
        if !ready {
            self.waiting_for_storage = true;
            return Vec::new();
        }
        self.store()
    }

    /// Waiting for a provider save before uploading.
    pub fn waiting_for_storage(&self) -> bool {
        self.waiting_for_storage
    }

    /// The provider save finished: continue (`saved`) or fail with `media_storage.save_failed`.
    pub fn resume_insert(&mut self, saved: bool) -> Vec<PickerEffect> {
        if !std::mem::take(&mut self.waiting_for_storage) {
            return Vec::new();
        }
        if saved {
            return self.store();
        }
        self.busy = None;
        self.error = Some(Text::Key(
            "dialogs.image_picker.insert_failed",
            vec![("error", Text::key("media_storage.save_failed"))],
        ));
        Vec::new()
    }

    fn store(&mut self) -> Vec<PickerEffect> {
        let Some(selected) = self.selected.clone() else {
            self.busy = None;
            return Vec::new();
        };
        let row = &self.row;
        let request = match selected.target {
            Target::Managed { url } => return self.finish(url),
            Target::Remote { url } if !row.stores() => return self.finish(url),
            Target::Server { url, .. } if !row.stores() => return self.finish(url),
            Target::File(file) => StoreRequest::Upload { file, resize: row.resize_ratio(), s3_key: row.key_value() },
            Target::Remote { url } => StoreRequest::ImportUrl { url, resize: row.resize_ratio(), s3_prefix: row.key_value() },
            Target::Server { path, .. } => StoreRequest::ImportPath { path, resize: row.resize_ratio(), s3_key: row.key_value() },
            Target::Generated { data, mime, filename } => {
                StoreRequest::Base64 { data, mime, filename, s3_prefix: row.key_value() }
            }
        };
        vec![PickerEffect::Store(request)]
    }

    fn finish(&mut self, url: String) -> Vec<PickerEffect> {
        self.busy = None;
        let name = self.selected.as_ref().map(|s| s.name.clone()).unwrap_or_default();
        let result = markup::final_image_markdown(self.note.as_deref(), &url, &self.alt, &name, &self.link);
        vec![PickerEffect::Insert(result)]
    }

    /// The store finished. A failure stays inline ("Failed to insert image: …")
    /// and re-reads the provider when S3 was involved (FE-314).
    pub fn store_result(&mut self, result: Result<String, StoreError>) -> Vec<PickerEffect> {
        if self.busy != Some(Busy::Insert) {
            return Vec::new();
        }
        match result {
            Ok(url) if !url.is_empty() => self.finish(url),
            Ok(_) => {
                self.busy = None;
                Vec::new()
            }
            Err(error) => {
                self.busy = None;
                self.error = Some(Text::Key("dialogs.image_picker.insert_failed", vec![("error", Text::Raw(error.message))]));
                if self.row.provider() == Provider::S3 || error.fallback { vec![PickerEffect::RefreshStorage] } else { Vec::new() }
            }
        }
    }
}

/// "1 image found" / "N images found" / "Showing N most recent of M images".
fn count_status(shown: usize, total: usize) -> Text {
    if shown == 0 {
        Text::key("dialogs.image_picker.no_images_found")
    } else if total > shown {
        Text::with(
            "dialogs.image_picker.folder_showing_recent",
            &[("shown", &shown.to_string()), ("total", &total.to_string())],
        )
    } else if shown == 1 {
        Text::key("dialogs.image_picker.folder_images_found_one")
    } else {
        Text::with("dialogs.image_picker.folder_images_found_many", &[("count", &shown.to_string())])
    }
}
