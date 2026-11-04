//! Search-in-files view model (ui-spec §5.3): debounce, generations, grouping
//! and keyboard selection. Time is injected; searches run off-thread and come
//! back through [`ContentSearchModel::apply_results`].

use std::time::{Duration, Instant};

use frankmd_search::content::{Hit, Query, Results, SEARCH_REGEX_TIMEOUT, SearchRegex};

/// Quiet period after the last keystroke before a search is requested.
pub const DEBOUNCE: Duration = Duration::from_millis(300);

/// Search toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchOptions {
    /// Match case exactly (default off).
    pub case_sensitive: bool,
    /// Treat the query as a regex (default on, parity FE-290).
    pub regex: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            regex: true,
        }
    }
}

/// The pattern handed to the search engine for `query` under `options`, plus
/// whether an invalid regex fell back to literal text.
pub fn effective_pattern(query: &str, options: SearchOptions) -> (String, bool) {
    if frankmd_search::content::is_blank(query) {
        return (query.to_owned(), false);
    }
    let mut fell_back = false;
    let mut pattern = if !options.regex {
        frankmd_search::content::regexp_escape(query)
    } else if SearchRegex::compile(query, SEARCH_REGEX_TIMEOUT)
        .is_some_and(|r| r.is_literal_fallback())
    {
        fell_back = true;
        frankmd_search::content::regexp_escape(query)
    } else {
        query.to_owned()
    };
    if options.case_sensitive {
        pattern = format!("(?-i:{pattern})");
    }
    (pattern, fell_back)
}

/// A search the app should run off-thread.
#[derive(Debug, Clone)]
pub struct SearchRequest {
    /// Generation to echo back with the results.
    pub generation: u64,
    /// The query to run.
    pub query: Query,
}

/// What the status line shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchStatus {
    /// "Type to search".
    Hint,
    /// A search is in flight and nothing has completed yet.
    Searching,
    /// Results arrived.
    Count {
        /// Total matching lines.
        matches: usize,
        /// Distinct notes with a match.
        notes: usize,
        /// The result cap cut the search short.
        truncated: bool,
    },
    /// The search found nothing.
    NoMatches,
    /// The search failed.
    Error,
}

/// One note's hits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultGroup {
    /// Note path.
    pub path: String,
    /// Display name.
    pub name: String,
    /// Indices into [`ContentSearchModel::results`].
    pub hits: Vec<usize>,
}

/// Open a note at a line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenAt {
    /// Note path.
    pub path: String,
    /// 1-based line number.
    pub line: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    None,
    Done {
        matches: usize,
        notes: usize,
        truncated: bool,
    },
    Error,
}

/// The search overlay's state.
#[derive(Debug)]
pub struct ContentSearchModel {
    query: String,
    options: SearchOptions,
    pending: Option<Instant>,
    in_flight: bool,
    generation: u64,
    invalid: bool,
    hits: Vec<Hit>,
    selected: Option<usize>,
    outcome: Outcome,
}

impl Default for ContentSearchModel {
    fn default() -> Self {
        Self {
            query: String::new(),
            options: SearchOptions::default(),
            pending: None,
            in_flight: false,
            generation: 0,
            invalid: false,
            hits: Vec::new(),
            selected: None,
            outcome: Outcome::None,
        }
    }
}

impl ContentSearchModel {
    fn clear(&mut self) {
        self.pending = None;
        self.in_flight = false;
        self.invalid = false;
        self.hits.clear();
        self.selected = None;
        self.outcome = Outcome::None;
        self.generation += 1;
    }

    /// Reset query, results, selection and pending work; keeps options.
    pub fn open(&mut self) {
        self.query.clear();
        self.clear();
    }

    /// Current query text.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Current toggles.
    pub fn options(&self) -> SearchOptions {
        self.options
    }

    fn arm(&mut self, now: Instant) -> Option<Instant> {
        if frankmd_search::content::is_blank(&self.query) {
            self.clear();
            return None;
        }
        let deadline = now + DEBOUNCE;
        self.pending = Some(deadline);
        Some(deadline)
    }

    /// Update the query; returns the debounce deadline, or `None` when blank.
    pub fn set_query(&mut self, query: &str, now: Instant) -> Option<Instant> {
        self.query = query.to_owned();
        self.arm(now)
    }

    /// Update the toggles; re-arms the debounce when the query is not blank.
    pub fn set_options(&mut self, options: SearchOptions, now: Instant) -> Option<Instant> {
        self.options = options;
        self.arm(now)
    }

    fn request(&mut self) -> SearchRequest {
        self.pending = None;
        self.in_flight = true;
        self.generation += 1;
        let (pattern, fell_back) = effective_pattern(&self.query, self.options);
        self.invalid = fell_back;
        SearchRequest {
            generation: self.generation,
            query: Query::ui(pattern),
        }
    }

    /// Emit the request once the debounce deadline has passed.
    pub fn tick(&mut self, now: Instant) -> Option<SearchRequest> {
        match self.pending {
            Some(deadline) if now >= deadline => Some(self.request()),
            _ => None,
        }
    }

    /// Search immediately (Enter before the debounce elapses).
    pub fn search_now(&mut self) -> Option<SearchRequest> {
        if frankmd_search::content::is_blank(&self.query) {
            return None;
        }
        Some(self.request())
    }

    /// Install results; stale generations are ignored. Selects the first hit.
    pub fn apply_results(&mut self, generation: u64, results: Results) {
        if generation != self.generation {
            return;
        }
        self.in_flight = false;
        let mut notes: Vec<&str> = Vec::new();
        for h in &results.hits {
            if !notes.contains(&h.path.as_str()) {
                notes.push(&h.path);
            }
        }
        self.outcome = if results.hits.is_empty() {
            Outcome::Done {
                matches: 0,
                notes: 0,
                truncated: false,
            }
        } else {
            Outcome::Done {
                matches: results.hits.len(),
                notes: notes.len(),
                truncated: results.truncated,
            }
        };
        self.selected = if results.hits.is_empty() {
            None
        } else {
            Some(0)
        };
        self.hits = results.hits;
    }

    /// The search failed; stale generations are ignored.
    pub fn apply_error(&mut self, generation: u64) {
        if generation != self.generation {
            return;
        }
        self.in_flight = false;
        self.hits.clear();
        self.selected = None;
        self.outcome = Outcome::Error;
    }

    /// True from the first keystroke until results, error or clear.
    pub fn searching(&self) -> bool {
        self.pending.is_some() || self.in_flight
    }

    /// Status line state.
    pub fn status(&self) -> SearchStatus {
        match self.outcome {
            Outcome::Error => SearchStatus::Error,
            Outcome::Done { matches: 0, .. } => SearchStatus::NoMatches,
            Outcome::Done {
                matches,
                notes,
                truncated,
            } => SearchStatus::Count {
                matches,
                notes,
                truncated,
            },
            Outcome::None if self.searching() => SearchStatus::Searching,
            Outcome::None => SearchStatus::Hint,
        }
    }

    /// The last request fell back to literal text ("Invalid pattern, searching as text").
    pub fn invalid_pattern(&self) -> bool {
        self.invalid
    }

    /// Flat hit list.
    pub fn results(&self) -> &[Hit] {
        &self.hits
    }

    /// Hits grouped by note in first-appearance order.
    pub fn groups(&self) -> Vec<ResultGroup> {
        let mut groups: Vec<ResultGroup> = Vec::new();
        for (ix, h) in self.hits.iter().enumerate() {
            match groups.iter_mut().find(|g| g.path == h.path) {
                Some(g) => g.hits.push(ix),
                None => groups.push(ResultGroup {
                    path: h.path.clone(),
                    name: h.name.clone(),
                    hits: vec![ix],
                }),
            }
        }
        groups
    }

    /// Selected hit index.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Select a hit (ignored when out of range).
    pub fn select(&mut self, ix: usize) {
        if ix < self.hits.len() {
            self.selected = Some(ix);
        }
    }

    /// Next hit, wrapping.
    pub fn select_next(&mut self) {
        let n = self.hits.len();
        if n > 0 {
            self.selected = Some(self.selected.map_or(0, |s| (s + 1) % n));
        }
    }

    /// Previous hit, wrapping.
    pub fn select_prev(&mut self) {
        let n = self.hits.len();
        if n > 0 {
            self.selected = Some(self.selected.map_or(n - 1, |s| (s + n - 1) % n));
        }
    }

    /// The selected hit as an open target.
    pub fn activate(&self) -> Option<OpenAt> {
        let h = self.hits.get(self.selected?)?;
        Some(OpenAt {
            path: h.path.clone(),
            line: h.line_number,
        })
    }
}
