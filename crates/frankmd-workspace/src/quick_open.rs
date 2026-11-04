//! Quick open (Ctrl+P) list model (ui-spec §5.2): recents first then by
//! mtime, the file finder's fuzzy scorer ([`frankmd_search::finder`]), top 50,
//! wrapping selection, and the "Ctrl+N creates `<query>.md`" suggestion.

use frankmd_search::finder;

/// How many rows quick open shows.
pub const MAX_ROWS: usize = 50;

/// Note entry fed to quick open.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteEntry {
    /// Vault-relative path with `.md`.
    pub path: String,
    /// Modification time (seconds).
    pub mtime: i64,
}

/// One visible row.
#[derive(Debug, Clone, PartialEq)]
pub struct QuickOpenRow {
    /// Vault-relative path.
    pub path: String,
    /// Matched character indices in `path` (for highlighting).
    pub matches: Vec<u32>,
    /// One of the shared recents.
    pub recent: bool,
}

/// Quick-open list model.
#[derive(Debug, Default)]
pub struct QuickOpenModel {
    /// Candidates ordered by mtime, newest first.
    by_mtime: Vec<String>,
    recents: Vec<String>,
    query: String,
    rows: Vec<QuickOpenRow>,
    selected: Option<usize>,
}

impl QuickOpenModel {
    /// Recents (the shared `.fed` recents, newest first) first, then the
    /// other notes by mtime desc.
    pub fn new(mut notes: Vec<NoteEntry>, recents: Vec<String>) -> Self {
        notes.sort_by_key(|n| std::cmp::Reverse(n.mtime));
        let by_mtime = notes.into_iter().map(|n| n.path).collect();
        let mut model = Self {
            by_mtime,
            recents,
            ..Self::default()
        };
        model.refresh();
        model
    }

    /// Update the query. Whitespace separates words; each word must match in
    /// order (`blog brief` finds `blog/2026/brief.md`).
    pub fn set_query(&mut self, query: &str) {
        query.clone_into(&mut self.query);
        self.refresh();
    }

    fn refresh(&mut self) {
        let compact: String = self.query.split_whitespace().collect();
        self.rows = finder::rank(&compact, &self.by_mtime, &self.recents, MAX_ROWS)
            .into_iter()
            .map(|r| QuickOpenRow {
                recent: self.recents.contains(&r.path),
                path: r.path,
                matches: r.indices,
            })
            .collect();
        self.selected = (!self.rows.is_empty()).then_some(0);
    }

    /// The query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Visible rows as paths.
    pub fn rows(&self) -> Vec<String> {
        self.rows.iter().map(|r| r.path.clone()).collect()
    }

    /// Visible rows with highlighting data.
    pub fn row_details(&self) -> &[QuickOpenRow] {
        &self.rows
    }

    /// ↓ with wrap.
    pub fn select_next(&mut self) {
        let len = self.rows.len();
        self.selected = self.selected.map(|i| (i + 1) % len.max(1));
    }

    /// ↑ with wrap.
    pub fn select_prev(&mut self) {
        let len = self.rows.len();
        self.selected = self
            .selected
            .map(|i| if i == 0 { len.saturating_sub(1) } else { i - 1 });
    }

    /// Selected row index.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Path of the selected row (what Enter opens).
    pub fn selected_path(&self) -> Option<&str> {
        self.selected
            .and_then(|i| self.rows.get(i))
            .map(|r| r.path.as_str())
    }

    /// Empty result offers "Ctrl+N creates `<query>.md`".
    pub fn create_suggestion(&self) -> Option<String> {
        let query = self.query.trim();
        if !self.rows.is_empty() || query.is_empty() {
            return None;
        }
        let path = frankmd_core::path::normalize_note_path(query);
        (!frankmd_core::path::is_config_file(&path)).then_some(path)
    }
}
