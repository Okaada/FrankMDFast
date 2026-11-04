//! State of the emoji / emoticon / icon picker popover (**[COMPAT]** port of
//! `emoji_picker_controller.js`, GUI-free): tabs, search, grid selection and
//! the "Frequently used" row.

use super::i18n::{self, SearchIndex};
use super::icons::{self, IconGlyph};

/// Maximum number of "Frequently used" emojis (one Emoji-tab row).
const FREQUENT_MAX: usize = 10;
const EMOJI_COLUMNS: usize = 10;
const EMOTICON_COLUMNS: usize = 5;
const ICON_COLUMNS: usize = 8;

/// Picker tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerTab {
    /// Emoji shortcodes.
    Emoji,
    /// Kaomoji / text emoticons.
    Emoticons,
    /// Phosphor icons.
    Icons,
}

impl PickerTab {
    /// All tabs in display order.
    pub const ALL: [PickerTab; 3] = [PickerTab::Emoji, PickerTab::Emoticons, PickerTab::Icons];

    /// i18n key of the tab label.
    pub fn label_key(self) -> &'static str {
        match self {
            PickerTab::Emoji => "dialogs.emoji_picker.tab_emoji",
            PickerTab::Emoticons => "dialogs.emoji_picker.tab_emoticons",
            PickerTab::Icons => "dialogs.emoji_picker.tab_icons",
        }
    }

    fn position(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

/// Arrow-key movement inside the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridMove {
    /// One cell back (wraps).
    Left,
    /// One cell forward (wraps).
    Right,
    /// One row up (wraps to the last row).
    Up,
    /// One row down (wraps to the first row).
    Down,
}

/// Key pressed while the tab strip has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKey {
    /// Previous tab.
    Left,
    /// Next tab.
    Right,
    /// Anything else (ignored).
    Other,
}

/// What kind of text an insertion is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertKind {
    /// `:shortcode:`.
    Emoji,
    /// Raw emoticon text.
    Emoticon,
    /// `:ph-<name>:`.
    Icon,
}

/// Text to insert into the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerInsert {
    /// The text.
    pub text: String,
    /// Its kind.
    pub kind: InsertKind,
}

/// What a cell or the footer draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellGlyph {
    /// The emoji character or the emoticon text.
    Text(&'static str),
    /// A Phosphor icon.
    Icon(IconGlyph),
}

/// One visible grid cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerCell {
    /// Position in the grid (frequent row first).
    pub index: usize,
    /// What to draw.
    pub glyph: CellGlyph,
    /// `:smile:`, the emoticon name, or `:ph-heart:`.
    pub code: String,
    /// Whether this is the selected cell.
    pub selected: bool,
    /// Whether the cell belongs to the "Frequently used" row.
    pub frequent: bool,
}

/// Footer preview of the selected item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerFooter {
    /// Large glyph.
    pub glyph: CellGlyph,
    /// Code label (emoticons: the name).
    pub code: String,
}

/// Emoji / emoticon / icon picker state.
#[derive(Debug, Clone)]
pub struct EmojiPicker {
    tab: PickerTab,
    query: String,
    /// Indices into the active tab's catalog, frequent row first.
    items: Vec<usize>,
    /// How many leading `items` are the "Frequently used" row.
    frequent_len: usize,
    selected: usize,
    scroll: Option<usize>,
    frequent: Vec<String>,
    locale: &'static str,
    index: SearchIndex,
}

impl EmojiPicker {
    /// New picker on the Emoji tab with the search index for `app_locale`.
    pub fn new(app_locale: Option<&str>) -> Self {
        let mut picker = Self {
            tab: PickerTab::Emoji,
            query: String::new(),
            items: Vec::new(),
            frequent_len: 0,
            selected: 0,
            scroll: None,
            frequent: Vec::new(),
            locale: i18n::emojibase_locale(app_locale),
            index: i18n::build_search_index(i18n::translated_emojis(app_locale)),
        };
        picker.refilter();
        picker
    }

    /// Load the search index for a new app locale; true when it changed.
    pub fn set_locale(&mut self, app_locale: Option<&str>) -> bool {
        let locale = i18n::emojibase_locale(app_locale);
        if locale == self.locale {
            return false;
        }
        self.locale = locale;
        self.index = i18n::build_search_index(i18n::translated_emojis(app_locale));
        self.refilter();
        true
    }

    /// The localized search index.
    pub fn search_index(&self) -> &SearchIndex {
        &self.index
    }

    /// Replace the "Frequently used" list (unknown shortcodes dropped,
    /// deduplicated, capped at one row).
    pub fn set_frequent(&mut self, shortcodes: Vec<String>) {
        let mut list: Vec<String> = Vec::new();
        for code in shortcodes {
            if list.len() < FREQUENT_MAX && !list.contains(&code) && emoji_position(&code).is_some()
            {
                list.push(code);
            }
        }
        self.frequent = list;
        self.refilter();
    }

    /// The "Frequently used" shortcodes, most recent first.
    pub fn frequent(&self) -> &[String] {
        &self.frequent
    }

    /// Remember an inserted emoji as most recently used.
    pub fn record_use(&mut self, insert: &PickerInsert) {
        if insert.kind != InsertKind::Emoji {
            return;
        }
        let code = insert.text.trim_matches(':');
        if emoji_position(code).is_none() {
            return;
        }
        let mut list = vec![code.to_owned()];
        list.extend(self.frequent.iter().filter(|c| *c != code).cloned());
        self.set_frequent(list);
    }

    /// Reset for a fresh opening: Emoji tab, empty query, first cell.
    pub fn open(&mut self) {
        self.tab = PickerTab::Emoji;
        self.query.clear();
        self.refilter();
    }

    /// Active tab.
    pub fn tab(&self) -> PickerTab {
        self.tab
    }

    /// Activate `tab`; false (no change) when already active.
    pub fn switch_tab(&mut self, tab: PickerTab) -> bool {
        if tab == self.tab {
            return false;
        }
        self.tab = tab;
        self.refilter();
        true
    }

    /// Cycle tabs with wrap (Ctrl+Tab / mouse wheel).
    pub fn next_tab(&mut self, forward: bool) {
        let n = PickerTab::ALL.len();
        let pos = self.tab.position();
        let next = if forward {
            (pos + 1) % n
        } else {
            (pos + n - 1) % n
        };
        self.switch_tab(PickerTab::ALL[next]);
    }

    /// Left/Right on the tab strip; false for other keys.
    pub fn tab_strip_key(&mut self, key: TabKey) -> bool {
        match key {
            TabKey::Left => self.next_tab(false),
            TabKey::Right => self.next_tab(true),
            TabKey::Other => return false,
        }
        true
    }

    /// Current search text as typed.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Filter the active tab and select the first cell.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_owned();
        self.refilter();
    }

    /// Grid columns of the active tab.
    pub fn columns(&self) -> usize {
        match self.tab {
            PickerTab::Emoji => EMOJI_COLUMNS,
            PickerTab::Emoticons => EMOTICON_COLUMNS,
            PickerTab::Icons => ICON_COLUMNS,
        }
    }

    /// Number of visible cells.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when nothing matches.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Selected cell index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Keyboard navigation with the original wrap rules.
    pub fn move_selection(&mut self, dir: GridMove) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let cols = self.columns();
        let sel = self.selected;
        self.selected = match dir {
            GridMove::Right => (sel + 1) % len,
            GridMove::Left => (sel + len - 1) % len,
            GridMove::Down if sel + cols < len => sel + cols,
            GridMove::Down => sel % cols,
            GridMove::Up if sel >= cols => sel - cols,
            GridMove::Up => {
                let last_row_start = (len - 1) / cols * cols;
                (last_row_start + sel % cols).min(len - 1)
            }
        };
        self.scroll = Some(self.selected);
    }

    /// Index to scroll into view after a keyboard move (taken once).
    pub fn take_scroll_request(&mut self) -> Option<usize> {
        self.scroll.take()
    }

    /// Mouse hover selects without scrolling; false if unchanged/out of range.
    pub fn hover(&mut self, index: usize) -> bool {
        if index >= self.items.len() || index == self.selected {
            return false;
        }
        self.selected = index;
        true
    }

    /// All visible cells in order.
    pub fn cells(&self) -> Vec<PickerCell> {
        (0..self.items.len()).filter_map(|i| self.cell(i)).collect()
    }

    /// One cell by grid index.
    pub fn cell(&self, index: usize) -> Option<PickerCell> {
        let item = *self.items.get(index)?;
        let (glyph, code) = self.describe(item);
        Some(PickerCell {
            index,
            glyph,
            code,
            selected: index == self.selected,
            frequent: index < self.frequent_len,
        })
    }

    /// Footer preview of the selected item.
    pub fn footer(&self) -> Option<PickerFooter> {
        let item = *self.items.get(self.selected)?;
        let (glyph, code) = self.describe(item);
        Some(PickerFooter { glyph, code })
    }

    /// Insertion for the selected item.
    pub fn activate(&self) -> Option<PickerInsert> {
        self.activate_at(self.selected)
    }

    /// Insertion for the item at `index`.
    pub fn activate_at(&self, index: usize) -> Option<PickerInsert> {
        let item = *self.items.get(index)?;
        Some(match self.tab {
            PickerTab::Emoji => PickerInsert {
                text: format!(":{}:", super::shortcodes()[item].shortcode),
                kind: InsertKind::Emoji,
            },
            PickerTab::Emoticons => PickerInsert {
                text: super::emoticons()[item].text.clone(),
                kind: InsertKind::Emoticon,
            },
            PickerTab::Icons => PickerInsert {
                text: format!(":ph-{}:", icons::icons()[item].name),
                kind: InsertKind::Icon,
            },
        })
    }

    fn describe(&self, item: usize) -> (CellGlyph, String) {
        match self.tab {
            PickerTab::Emoji => {
                let e = &super::shortcodes()[item];
                (
                    CellGlyph::Text(e.emoji.as_str()),
                    format!(":{}:", e.shortcode),
                )
            }
            PickerTab::Emoticons => {
                let e = &super::emoticons()[item];
                (CellGlyph::Text(e.text.as_str()), e.name.clone())
            }
            PickerTab::Icons => {
                let i = &icons::icons()[item];
                let glyph = IconGlyph {
                    path: i.path.as_str(),
                    view_box: i.view_box.as_str(),
                };
                (CellGlyph::Icon(glyph), format!(":ph-{}:", i.name))
            }
        }
    }

    /// Rebuild `items` for the tab and query; selection back to 0.
    fn refilter(&mut self) {
        self.selected = 0;
        self.scroll = None;
        self.frequent_len = 0;
        let query = self.query.trim().to_lowercase();
        let terms: Vec<&str> = query.split_whitespace().collect();
        match self.tab {
            PickerTab::Emoji => {
                let all = super::shortcodes();
                let mut items: Vec<usize> = Vec::new();
                if terms.is_empty() {
                    items.extend(self.frequent.iter().filter_map(|c| emoji_position(c)));
                    self.frequent_len = items.len();
                    items.extend(0..all.len());
                } else {
                    // Results come back in catalog order, so one forward scan
                    // recovers their indices.
                    let mut from = 0;
                    for hit in i18n::search_emojis(&query, Some(&self.index)) {
                        if let Some(off) = all[from..].iter().position(|e| std::ptr::eq(e, hit)) {
                            from += off;
                            items.push(from);
                        }
                    }
                }
                self.items = items;
            }
            PickerTab::Emoticons => {
                self.items = filter_indices(
                    super::emoticons().iter().map(|e| (&e.name, &e.keywords)),
                    &terms,
                );
            }
            PickerTab::Icons => {
                self.items = filter_indices(
                    icons::icons().iter().map(|i| (&i.name, &i.keywords)),
                    &terms,
                );
            }
        }
    }
}

/// Catalog position of the first row with this shortcode.
fn emoji_position(shortcode: &str) -> Option<usize> {
    super::shortcodes()
        .iter()
        .position(|e| e.shortcode == shortcode)
}

/// Indices of rows whose `"name keywords"` contains every term.
fn filter_indices<'a>(
    rows: impl Iterator<Item = (&'a String, &'a String)>,
    terms: &[&str],
) -> Vec<usize> {
    rows.enumerate()
        .filter(|(_, (name, keywords))| {
            let hay = format!("{name} {keywords}").to_lowercase();
            terms.iter().all(|t| hay.contains(t))
        })
        .map(|(i, _)| i)
        .collect()
}
