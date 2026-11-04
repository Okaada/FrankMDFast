//! The Insert video dialog (ui-spec §5.7; FE-337…345).

use super::Text;
use super::markup::{self, MediaInsert};
use super::picker::{LocalFile, StoreError};
use super::storage::{KeyMode, Provider, RowMode, StorageRow};
use crate::youtube::{Video, VideoResults};

/// The three tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoTab {
    /// Upload a local video file.
    #[default]
    Upload,
    /// YouTube URL or video file path.
    Url,
    /// YouTube Data API search.
    YouTube,
}

impl VideoTab {
    /// Tab order.
    pub const ALL: [VideoTab; 3] = [VideoTab::Upload, VideoTab::Url, VideoTab::YouTube];

    /// Label key.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Upload => "dialogs.video.tab_drop",
            Self::Url => "dialogs.video.tab_url",
            Self::YouTube => "dialogs.video.tab_search",
        }
    }

    /// Element id suffix.
    pub fn id(self) -> &'static str {
        match self {
            Self::Upload => "upload",
            Self::Url => "url",
            Self::YouTube => "youtube",
        }
    }
}

/// What the URL field (or an upload) resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detected {
    /// A YouTube video id.
    YouTube {
        /// 11-character id.
        id: String,
    },
    /// A video file URL or path.
    File {
        /// As entered / as the store returned it.
        url: String,
        /// Last path segment.
        filename: String,
    },
}

/// Upload progress.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UploadState {
    /// Nothing uploaded.
    #[default]
    Idle,
    /// In flight (`dialogs.video.uploading`, indeterminate bar).
    Uploading {
        /// File name.
        name: String,
    },
    /// Stored.
    Done {
        /// File name.
        name: String,
    },
}

/// Work for the view.
#[derive(Debug, Clone, PartialEq)]
pub enum VideoEffect {
    /// Portal chooser (one video file).
    ChooseFile,
    /// `POST /media/upload` → [`VideoDialog::upload_result`].
    Upload {
        /// The file.
        file: LocalFile,
        /// S3 key prefix.
        s3_prefix: Option<String>,
    },
    /// YouTube search → [`VideoDialog::search_result`].
    Search {
        /// Request generation.
        generation: u64,
        /// Query.
        query: String,
    },
    /// Re-read the storage provider.
    RefreshStorage,
    /// Insert the block and close.
    Insert(MediaInsert),
    /// Close without inserting.
    Close,
}

/// Grid navigation keys in the YouTube results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridKey {
    /// ←
    Left,
    /// →
    Right,
    /// ↑ (a row = 2 results)
    Up,
    /// ↓
    Down,
}

/// The dialog state. `hugo` survives reopening (FE-338).
#[derive(Debug, Clone, Default)]
pub struct VideoDialog {
    tab: VideoTab,
    hugo: bool,
    youtube_enabled: bool,
    extensions: Vec<String>,
    row: StorageRow,
    url: String,
    detected: Option<Detected>,
    thumb_stage: u8,
    upload: UploadState,
    upload_detected: Option<Detected>,
    upload_feedback: Option<Text>,
    query: String,
    results: Vec<Video>,
    selected: Option<usize>,
    status: Option<Text>,
    status_error: bool,
    searching: bool,
    generation: u64,
}

impl VideoDialog {
    /// `open()`: every tab reset, Upload active; the Hugo flag is kept.
    pub fn open(
        &mut self,
        provider: Provider,
        s3_available: bool,
        youtube_enabled: bool,
        video_extensions: Vec<String>,
        today: jiff::civil::Date,
    ) {
        let hugo = self.hugo;
        let generation = self.generation + 1;
        *self = Self::default();
        self.hugo = hugo;
        self.generation = generation;
        self.youtube_enabled = youtube_enabled;
        self.extensions = if video_extensions.is_empty() {
            crate::config::DEFAULT_VIDEO_EXTENSIONS.iter().map(|e| (*e).to_owned()).collect()
        } else {
            video_extensions
        };
        self.row = StorageRow::new(provider, s3_available);
        self.row.set_mode(RowMode::S3Only, KeyMode::Prefix);
        self.row.set_default_key(&crate::s3_key::default_s3_prefix(today));
    }

    /// Configuration read after opening (YouTube key, video extensions).
    pub fn set_config(&mut self, youtube_enabled: bool, video_extensions: Vec<String>) {
        self.youtube_enabled = youtube_enabled;
        if !video_extensions.is_empty() {
            self.extensions = video_extensions;
        }
        let url = self.url.clone();
        self.set_url(&url);
    }

    /// Active tab.
    pub fn tab(&self) -> VideoTab {
        self.tab
    }

    /// Switch tab.
    pub fn set_tab(&mut self, tab: VideoTab) {
        self.tab = tab;
    }

    /// Next / previous tab with wrap.
    pub fn step_tab(&mut self, forward: bool) {
        let index = VideoTab::ALL.iter().position(|t| *t == self.tab).unwrap_or(0);
        let n = VideoTab::ALL.len();
        self.tab = VideoTab::ALL[if forward { (index + 1) % n } else { (index + n - 1) % n }];
    }

    /// "Use Hugo shortcode instead of HTML embed".
    pub fn hugo(&self) -> bool {
        self.hugo
    }

    /// Toggle the Hugo checkbox.
    pub fn set_hugo(&mut self, on: bool) {
        self.hugo = on;
    }

    /// YouTube search is configured.
    pub fn youtube_enabled(&self) -> bool {
        self.youtube_enabled
    }

    /// The storage row (S3 prefix for uploads).
    pub fn row(&self) -> &StorageRow {
        &self.row
    }

    /// Mutable storage row.
    pub fn row_mut(&mut self) -> &mut StorageRow {
        &mut self.row
    }

    /// The global provider changed.
    pub fn storage_changed(&mut self, provider: Provider, s3_available: bool) {
        self.row.set_s3_available(s3_available);
        self.row.set_provider(provider);
    }

    // ----- URL tab ------------------------------------------------------------------------

    /// URL field text.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Live detection on every input (FE-340/341).
    pub fn set_url(&mut self, url: &str) {
        url.clone_into(&mut self.url);
        self.thumb_stage = 0;
        let url = url.trim();
        self.detected = if url.is_empty() {
            None
        } else if let Some(id) = crate::url_utils::extract_youtube_id(Some(url)) {
            Some(Detected::YouTube { id })
        } else if self.extensions.iter().any(|ext| url.to_lowercase().ends_with(ext.as_str())) {
            Some(Detected::File { url: url.to_owned(), filename: url.rsplit('/').next().unwrap_or(url).to_owned() })
        } else {
            None
        };
    }

    /// What the URL field resolved to.
    pub fn detected(&self) -> Option<&Detected> {
        self.detected.as_ref()
    }

    /// The URL preview text when nothing was detected: the hint for an empty
    /// field, else "Unknown format…".
    pub fn url_hint(&self) -> Option<Text> {
        if self.detected.is_some() {
            None
        } else if self.url.trim().is_empty() {
            Some(Text::key("dialogs.video.preview_hint"))
        } else {
            Some(Text::key("dialogs.video.unknown_format"))
        }
    }

    /// The YouTube thumbnail to show: `mqdefault.jpg`, then `default.jpg`, then none.
    pub fn url_thumbnail(&self) -> Option<String> {
        let Some(Detected::YouTube { id }) = &self.detected else { return None };
        match self.thumb_stage {
            0 => Some(format!("https://img.youtube.com/vi/{id}/mqdefault.jpg")),
            1 => Some(format!("https://img.youtube.com/vi/{id}/default.jpg")),
            _ => None,
        }
    }

    /// The thumbnail failed to load.
    pub fn url_thumbnail_failed(&mut self) {
        self.thumb_stage = self.thumb_stage.saturating_add(1).min(2);
    }

    /// Insert is enabled on the active tab.
    pub fn can_insert(&self) -> bool {
        match self.tab {
            VideoTab::Url => self.detected.is_some(),
            VideoTab::Upload => self.upload_detected.is_some(),
            VideoTab::YouTube => self.selected.is_some(),
        }
    }

    /// Enter in the URL field: inserts only when something was detected.
    pub fn url_enter(&mut self) -> Vec<VideoEffect> {
        if self.detected.is_some() { self.confirm() } else { Vec::new() }
    }

    /// Insert button / Ctrl+Enter (`insertVideo`): nothing detected just closes.
    pub fn confirm(&mut self) -> Vec<VideoEffect> {
        if self.tab == VideoTab::YouTube {
            return match self.selected {
                Some(index) => self.choose_result(index),
                None => vec![VideoEffect::Close],
            };
        }
        let detected = if self.tab == VideoTab::Upload { &self.upload_detected } else { &self.detected };
        match detected {
            Some(Detected::YouTube { id }) => vec![VideoEffect::Insert(MediaInsert::Block(markup::youtube_embed(id, None, self.hugo)))],
            Some(Detected::File { url, .. }) => vec![VideoEffect::Insert(MediaInsert::Block(markup::video_file_embed(url)))],
            None => vec![VideoEffect::Close],
        }
    }

    // ----- Upload tab ---------------------------------------------------------------------

    /// Enter / Space / click on the dropzone.
    pub fn choose_file(&self) -> Vec<VideoEffect> {
        if matches!(self.upload, UploadState::Uploading { .. }) { Vec::new() } else { vec![VideoEffect::ChooseFile] }
    }

    /// Dropped / chosen files: only the first is used; wrong types are refused;
    /// a second drop while uploading is ignored. `ready` = no provider save pending.
    pub fn drop_files(&mut self, files: Vec<LocalFile>) -> Vec<VideoEffect> {
        if matches!(self.upload, UploadState::Uploading { .. }) {
            return Vec::new();
        }
        let Some(file) = files.into_iter().next() else { return Vec::new() };
        let lower = file.name.to_lowercase();
        if !self.extensions.iter().any(|ext| lower.ends_with(ext.as_str())) {
            let ext = match lower.rfind('.') {
                Some(dot) if dot + 1 < lower.len() => Text::Raw(lower[dot..].to_owned()),
                _ => Text::key("dialogs.image_picker.no_file_extension"),
            };
            self.upload_feedback = Some(Text::Key(
                "dialogs.video.drop_rejected",
                vec![("name", Text::Raw(file.name.clone())), ("ext", ext), ("list", Text::Raw(self.extensions.join(", ")))],
            ));
            return Vec::new();
        }
        self.upload_feedback = None;
        self.upload_detected = None;
        self.upload = UploadState::Uploading { name: file.name.clone() };
        vec![VideoEffect::Upload { file, s3_prefix: self.row.key_value() }]
    }

    /// The upload finished.
    pub fn upload_result(&mut self, result: Result<String, StoreError>) -> Vec<VideoEffect> {
        let UploadState::Uploading { name } = std::mem::take(&mut self.upload) else { return Vec::new() };
        match result {
            Ok(url) => {
                let filename = url.rsplit('/').next().unwrap_or(&url).to_owned();
                self.upload_detected = Some(Detected::File { url, filename });
                self.upload = UploadState::Done { name };
                Vec::new()
            }
            Err(error) => {
                self.upload_feedback = Some(if error.message.is_empty() {
                    Text::key("status.upload_failed")
                } else {
                    Text::Raw(error.message)
                });
                if self.row.provider() == Provider::S3 || error.fallback { vec![VideoEffect::RefreshStorage] } else { Vec::new() }
            }
        }
    }

    /// Upload state.
    pub fn upload_state(&self) -> &UploadState {
        &self.upload
    }

    /// The upload message (rejection or failure).
    pub fn upload_feedback(&self) -> Option<&Text> {
        self.upload_feedback.as_ref()
    }

    // ----- YouTube tab --------------------------------------------------------------------

    /// Search field.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Edit the search field.
    pub fn set_query(&mut self, query: &str) {
        query.clone_into(&mut self.query);
    }

    /// Enter / Search.
    pub fn search(&mut self) -> Vec<VideoEffect> {
        if !self.youtube_enabled {
            return Vec::new();
        }
        let query = self.query.trim().to_owned();
        if query.is_empty() {
            self.status = Some(Text::key("status.please_enter_keywords"));
            self.status_error = false;
            return Vec::new();
        }
        self.generation += 1;
        self.searching = true;
        self.results.clear();
        self.selected = None;
        self.status = Some(Text::key("status.searching"));
        self.status_error = false;
        vec![VideoEffect::Search { generation: self.generation, query }]
    }

    /// Results arrived.
    pub fn search_result(&mut self, generation: u64, result: Result<VideoResults, Text>) {
        if generation != self.generation {
            return;
        }
        self.searching = false;
        match result {
            Ok(results) => {
                if let Some(error) = results.error.filter(|e| !e.is_empty()) {
                    self.status = Some(Text::Raw(error));
                    self.status_error = true;
                    return;
                }
                self.results = results.videos;
                self.status_error = false;
                self.status = Some(if self.results.is_empty() {
                    Text::key("status.no_videos_found")
                } else {
                    Text::with("status.found_videos", &[("count", &self.results.len().to_string())])
                });
            }
            Err(error) => {
                self.status = Some(error);
                self.status_error = true;
            }
        }
    }

    /// Status line and whether it is an error.
    pub fn status(&self) -> (Option<&Text>, bool) {
        (self.status.as_ref(), self.status_error)
    }

    /// Searching.
    pub fn searching(&self) -> bool {
        self.searching
    }

    /// Results.
    pub fn results(&self) -> &[Video] {
        &self.results
    }

    /// Selected result.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Mark a result (no-op without results).
    pub fn select(&mut self, index: usize) {
        if index < self.results.len() {
            self.selected = Some(index);
        }
    }

    /// Unmark.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Arrow keys: ↓ from the field enters the grid; ←/→ ±1, ↑/↓ ±2 (a row),
    /// clamped; ↑ from the first row returns to the field (`false` = focus the field).
    pub fn grid_key(&mut self, key: GridKey) -> bool {
        let n = self.results.len();
        if n == 0 {
            return false;
        }
        let Some(current) = self.selected else {
            if key == GridKey::Down {
                self.selected = Some(0);
                return true;
            }
            return false;
        };
        match key {
            GridKey::Right => self.selected = Some((current + 1).min(n - 1)),
            GridKey::Down => self.selected = Some((current + 2).min(n - 1)),
            GridKey::Left => self.selected = Some(current.saturating_sub(1)),
            GridKey::Up if current < 2 => {
                self.selected = None;
                return false;
            }
            GridKey::Up => self.selected = Some(current - 2),
        }
        true
    }

    /// Thumbnail of a result (the API's, or `mqdefault.jpg`).
    pub fn result_thumbnail(&self, index: usize) -> Option<String> {
        let video = self.results.get(index)?;
        video
            .thumbnail
            .clone()
            .filter(|t| !t.is_empty())
            .or_else(|| video.id.as_ref().map(|id| format!("https://img.youtube.com/vi/{id}/mqdefault.jpg")))
    }

    /// Click / Enter on a result: insert immediately and close.
    pub fn choose_result(&mut self, index: usize) -> Vec<VideoEffect> {
        let Some(video) = self.results.get(index) else { return Vec::new() };
        let Some(id) = video.id.clone().filter(|id| !id.is_empty()) else { return Vec::new() };
        let title = video.title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| markup::YOUTUBE_SEARCH_FALLBACK_TITLE.to_owned());
        vec![VideoEffect::Insert(MediaInsert::Block(markup::youtube_embed(&id, Some(&title), self.hugo)))]
    }
}
