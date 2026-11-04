//! Code-block highlighting for the preview (bounded, cancellable, HP-017).
//!
//! **[COMPAT]** `app/javascript/lib/code_block_highlighter.js`: the explicit
//! fence-alias table, the `data-code-language` labels and the per-render
//! budgets. Golden: `js_code_block_highlighter.json`.

use crate::ir::HighlightedLine;

/// A highlighter backend (tree-sitter via gpui-kit's registry in the app, or
/// [`BuiltinHighlighter`] here for headless `render`).
pub trait CodeHighlighter: Send + Sync {
    /// Highlight `code` as `lang`; `None` if the language is unknown.
    fn highlight(&self, lang: Language, code: &str) -> Option<Vec<HighlightedLine>>;
}

/// A language FrankMD ships a parser for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Language {
    /// Stable id (`"javascript"`, `"terraform"`, ...).
    pub id: &'static str,
    /// UI label (`data-code-language`): `"JavaScript"`, `"C++"`, `"HCL"`...
    pub label: &'static str,
}

/// **[COMPAT]** resolve a fence info string (`marked` puts its first word in
/// `class="language-<x>"`) through FrankMD's `LANGUAGE_PARSERS` table,
/// case-insensitively. Unknown → `None` (block left untouched, no label).
pub fn language_for_fence(info: &str) -> Option<Language> {
    todo!()
}

/// Per-render highlighting limits. Lengths are **UTF-16 code units** (JS
/// `string.length`), as in FrankMD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightBudget {
    /// A single block longer than this is never highlighted (20 000).
    pub max_block_len: usize,
    /// Total highlighted source per render (50 000).
    pub max_total_len: usize,
    /// Only the first N code blocks (with a `language-*` class) are even
    /// inspected — labelled or highlighted (100).
    pub max_blocks: usize,
}

impl Default for HighlightBudget {
    fn default() -> Self {
        Self { max_block_len: 20_000, max_total_len: 50_000, max_blocks: 100 }
    }
}

/// What the preview does with one code block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockPlan {
    /// Resolved language (label shown), `None` for unknown or uninspected blocks.
    pub language: Option<Language>,
    /// Whether token highlighting runs for this block.
    pub highlight: bool,
}

/// **[COMPAT]** budget planning, in document order. `blocks` has one entry
/// per code block that has a class (a fenced block with an info string):
/// `(language token, UTF-16 length of the code text as marked emits it, i.e.
/// including the trailing newline)`; use `""` for a classed block without a
/// `language-*` class. Fences without an info string are not listed.
/// Mirrors `highlightCodeBlocksInElement`'s loop: the inspected-block counter
/// counts every listed block (known language or not) and stops at
/// `max_blocks`; a block longer than `max_block_len` or than the remaining
/// total is skipped without consuming the total; once the total reaches 0
/// nothing else is highlighted (labels are still assigned).
pub fn plan(blocks: &[(&str, usize)], budget: &HighlightBudget) -> Vec<BlockPlan> {
    todo!()
}

/// Headless highlighter covering every language in the FrankMD table
/// (syntect/two-face, plus [`hcl`] for Terraform/HCL).
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltinHighlighter;

impl CodeHighlighter for BuiltinHighlighter {
    fn highlight(&self, lang: Language, code: &str) -> Option<Vec<HighlightedLine>> {
        todo!()
    }
}

/// **[COMPAT]** FrankMD's hand-written Terraform/HCL stream tokenizer
/// (`terraform_hcl_stream_parser.js`), line-at-a-time like CodeMirror's
/// `StreamLanguage`. Golden: `js_terraform_hcl_stream_parser.json`.
pub mod hcl {
    /// Tokenizer state carried across lines (`startState()`; `copyState` = `Clone`).
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct HclState {
        /// Inside `/* ... */`.
        pub block_comment: bool,
        /// Inside a `<<EOF` / `<<-EOF` heredoc: the closing marker.
        pub heredoc_marker: Option<String>,
        /// The previous token was a `.` (next identifier is a property).
        pub after_dot: bool,
    }

    /// StreamLanguage style of a token.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[allow(missing_docs)]
    pub enum HclStyle {
        Keyword,
        String,
        Comment,
        /// `true`/`false`/`null`.
        Atom,
        Number,
        /// `variableName.function`: identifier followed by `(`.
        Function,
        /// After a `.`, or followed by `=`.
        Property,
        Variable,
        Operator,
        Punctuation,
    }

    /// Tokenize one non-empty line: `(token text, style)` in order, covering
    /// the whole line; whitespace runs are `(text, None)`.
    pub fn tokenize_line(line: &str, state: &mut HclState) -> Vec<(String, Option<HclStyle>)> {
        todo!()
    }

    /// `blankLine(state)`: called for empty lines (resets `after_dot`).
    pub fn blank_line(state: &mut HclState) {
        todo!()
    }
}
