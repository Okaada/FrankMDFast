//! View model of the "Code block" dialog (insert or edit a fenced block).
//!
//! Port of `code_dialog_controller.js`. Unknown languages produce an inline
//! warning (`CodeBlockOutcome::UnknownLanguage`) instead of a confirm dialog.

use std::ops::Range;

use frankmd_core::text::markdown::find_code_block_at_position;

/// Language ids offered by the dialog, in FrankMD order.
pub const LANGUAGES: [&str; 44] = [
    "javascript", "typescript", "python", "ruby", "go", "rust", "java", "c", "cpp", "csharp",
    "php", "swift", "kotlin", "scala", "haskell", "elixir", "erlang", "clojure", "lua", "perl",
    "html", "css", "scss", "sass", "less", "json", "yaml", "toml", "xml", "markdown", "sql",
    "graphql", "bash", "shell", "powershell", "dockerfile", "makefile", "nginx", "apache", "vim",
    "regex", "diff", "git", "plaintext",
];

/// Maximum number of language suggestion rows.
pub const MAX_SUGGESTIONS: usize = 6;

/// Whether the dialog inserts a new block or replaces an existing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeBlockMode {
    /// Insert at the caret.
    Insert,
    /// Replace the given byte range (the existing fenced block).
    Update(Range<usize>),
}

/// Result of confirming the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeBlockOutcome {
    /// Ready: the fence markdown and the caret offset inside it (3 + language.len() + 1,
    /// first content line); `mode` tells whether to replace a range.
    Insert {
        /// The complete fenced block.
        markdown: String,
        /// Caret offset (bytes) inside `markdown`.
        cursor_offset: usize,
        /// Insert at caret or replace a range.
        mode: CodeBlockMode,
    },
    /// Unknown language: show the inline warning; nothing inserted yet.
    UnknownLanguage {
        /// The language as typed (trimmed).
        typed: String,
        /// Closest known language, if any.
        did_you_mean: Option<&'static str>,
    },
}

/// State of the "Code block" dialog.
#[derive(Debug, Clone)]
pub struct CodeBlockForm {
    mode: CodeBlockMode,
    language: String,
    content: String,
    use_tabs: bool,
    suggestions: Vec<&'static str>,
    suggestions_visible: bool,
    warning: Option<CodeBlockOutcome>,
}

impl CodeBlockForm {
    /// Empty form in insert mode.
    pub fn new_insert() -> Self {
        Self {
            mode: CodeBlockMode::Insert,
            language: String::new(),
            content: String::new(),
            use_tabs: false,
            suggestions: Vec::new(),
            suggestions_visible: false,
            warning: None,
        }
    }

    /// Update mode when `caret` (byte offset) is inside a fenced block, else insert mode.
    pub fn from_document(text: &str, caret: usize) -> Self {
        let mut form = Self::new_insert();
        if let Some(block) = find_code_block_at_position(text, caret) {
            form.mode = CodeBlockMode::Update(block.start_pos..block.end_pos);
            form.language = block.language;
            form.content = block.content;
        }
        form
    }

    /// Current mode.
    pub fn mode(&self) -> &CodeBlockMode {
        &self.mode
    }

    /// i18n key of the primary button.
    pub fn primary_label_key(&self) -> &'static str {
        match self.mode {
            CodeBlockMode::Insert => "dialogs.code.insert_code",
            CodeBlockMode::Update(_) => "dialogs.code.update_code",
        }
    }

    /// All selectable languages.
    pub fn languages(&self) -> &'static [&'static str] {
        &LANGUAGES
    }

    /// The language field text.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Set the language field and recompute suggestions; clears any warning.
    pub fn set_language(&mut self, value: &str) {
        self.language = value.to_string();
        self.warning = None;
        let needle = value.trim().to_lowercase();
        if needle.is_empty() {
            self.suggestions_visible = false;
            return;
        }
        self.suggestions = LANGUAGES
            .iter()
            .copied()
            .filter(|l| l.contains(needle.as_str()))
            .take(MAX_SUGGESTIONS)
            .collect();
        self.suggestions_visible = self.suggestions.first().is_some_and(|f| *f != needle);
    }

    /// Current suggestion rows.
    pub fn suggestions(&self) -> &[&'static str] {
        &self.suggestions
    }

    /// Whether the suggestion list is shown.
    pub fn suggestions_visible(&self) -> bool {
        self.suggestions_visible
    }

    /// Hide the suggestion list (rows are kept).
    pub fn hide_suggestions(&mut self) {
        self.suggestions_visible = false;
    }

    /// Tab in the language field: accept the first suggestion when visible.
    /// Returns true when the key was consumed.
    pub fn key_tab(&mut self) -> bool {
        if !self.suggestions_visible {
            return false;
        }
        if let Some(first) = self.suggestions.first().copied() {
            self.language = first.to_string();
            self.warning = None;
        }
        self.suggestions_visible = false;
        true
    }

    /// Pick a suggestion row.
    pub fn select_suggestion(&mut self, lang: &str) {
        self.language = lang.to_string();
        self.warning = None;
        self.suggestions_visible = false;
    }

    /// The code area text.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Replace the code area text.
    pub fn set_content(&mut self, value: &str) {
        self.content = value.to_string();
    }

    /// Whether "Indent with tabs" is checked.
    pub fn use_tabs(&self) -> bool {
        self.use_tabs
    }

    /// Toggle "Indent with tabs".
    pub fn set_use_tabs(&mut self, on: bool) {
        self.use_tabs = on;
    }

    /// The indent unit: a tab or two spaces.
    pub fn indent_unit(&self) -> &'static str {
        if self.use_tabs { "\t" } else { "  " }
    }

    fn clamp(&self, pos: usize) -> usize {
        let mut p = pos.min(self.content.len());
        while !self.content.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    /// Insert the indent unit at byte `caret`; returns the new caret.
    pub fn indent_at_caret(&mut self, caret: usize) -> usize {
        let caret = self.clamp(caret);
        let unit = self.indent_unit();
        self.content.insert_str(caret, unit);
        caret + unit.len()
    }

    fn line_bounds(&self, selection: &Range<usize>) -> (usize, usize) {
        let start = self.clamp(selection.start);
        let end = self.clamp(selection.end.max(selection.start));
        let line_start = self.content[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = self.content[end..]
            .find('\n')
            .map_or(self.content.len(), |i| end + i);
        (line_start, line_end)
    }

    fn replace_lines(
        &mut self,
        selection: &Range<usize>,
        f: impl Fn(&str) -> String,
    ) -> Range<usize> {
        let (ls, le) = self.line_bounds(selection);
        let new_text = self.content[ls..le]
            .split('\n')
            .map(f)
            .collect::<Vec<_>>()
            .join("\n");
        let len = new_text.len();
        self.content.replace_range(ls..le, &new_text);
        ls..ls + len
    }

    /// Tab with a selection: indent every touched line; returns the reselected block.
    pub fn indent_lines(&mut self, selection: Range<usize>) -> Range<usize> {
        let unit = self.indent_unit();
        self.replace_lines(&selection, |l| format!("{unit}{l}"))
    }

    /// Shift+Tab: no selection outdents the current line (caret moves back by the
    /// removed width); otherwise every touched line is outdented.
    pub fn outdent(&mut self, selection: Range<usize>) -> Range<usize> {
        let unit = self.indent_unit();
        if selection.start == selection.end {
            let caret = self.clamp(selection.start);
            let line_start = self.content[..caret].rfind('\n').map_or(0, |i| i + 1);
            let before = &self.content[line_start..caret];
            let remove = if before.starts_with(unit) {
                unit.len()
            } else if before.starts_with('\t') {
                1
            } else if before.starts_with(' ') {
                before.bytes().take_while(|b| *b == b' ').count().min(unit.len())
            } else {
                0
            };
            self.content.replace_range(line_start..line_start + remove, "");
            return caret - remove..caret - remove;
        }
        self.replace_lines(&selection, |line| {
            let n = if line.starts_with(unit) {
                unit.len()
            } else if line.starts_with('\t') {
                1
            } else if line.starts_with(' ') {
                line.bytes().take_while(|b| *b == b' ').count().min(unit.len())
            } else {
                0
            };
            line[n..].to_string()
        })
    }

    /// The pending unknown-language warning, if any.
    pub fn warning(&self) -> Option<&CodeBlockOutcome> {
        self.warning.as_ref()
    }

    fn insert_outcome(&self, language: &str) -> CodeBlockOutcome {
        CodeBlockOutcome::Insert {
            markdown: build_fence(language, &self.content),
            cursor_offset: 3 + language.len() + 1,
            mode: self.mode.clone(),
        }
    }

    /// Confirm the dialog: an unknown language yields (and stores) the warning.
    pub fn confirm(&mut self) -> CodeBlockOutcome {
        let language = self.language.trim().to_string();
        let lower = language.to_lowercase();
        if !language.is_empty() && !LANGUAGES.contains(&lower.as_str()) {
            let outcome = CodeBlockOutcome::UnknownLanguage {
                did_you_mean: closest_language(&lower),
                typed: language,
            };
            self.warning = Some(outcome.clone());
            return outcome;
        }
        self.warning = None;
        self.insert_outcome(&language)
    }

    /// "Use <suggestion>": adopt the suggested language and insert.
    pub fn use_suggested(&mut self) -> Option<CodeBlockOutcome> {
        let Some(CodeBlockOutcome::UnknownLanguage { did_you_mean: Some(lang), .. }) =
            self.warning.take()
        else {
            return None;
        };
        self.language = lang.to_string();
        Some(self.insert_outcome(lang))
    }

    /// "Keep <typed>": insert with the typed language.
    pub fn keep_typed(&mut self) -> Option<CodeBlockOutcome> {
        let Some(CodeBlockOutcome::UnknownLanguage { typed, .. }) = self.warning.take() else {
            return None;
        };
        Some(self.insert_outcome(&typed))
    }
}

fn closest_language(lower: &str) -> Option<&'static str> {
    if let Some(l) = LANGUAGES.iter().copied().find(|l| l.starts_with(lower)) {
        return Some(l);
    }
    LANGUAGES
        .iter()
        .copied()
        .map(|l| (levenshtein(l, lower), l))
        .filter(|(d, _)| *d <= 2)
        .min_by_key(|(d, _)| *d)
        .map(|(_, l)| l)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Build the fenced block markdown.
pub fn build_fence(language: &str, content: &str) -> String {
    if content.is_empty() {
        format!("```{language}\n\n```")
    } else if content.ends_with('\n') {
        format!("```{language}\n{content}```")
    } else {
        format!("```{language}\n{content}\n```")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_distance() {
        assert_eq!(levenshtein("python", "pyhton"), 2);
        assert_eq!(levenshtein("", "abc"), 3);
    }
}
