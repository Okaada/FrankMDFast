//! Code-block highlighting for the preview (bounded, cancellable, HP-017).
//!
//! **[COMPAT]** `app/javascript/lib/code_block_highlighter.js`: the explicit
//! fence-alias table, the `data-code-language` labels and the per-render
//! budgets. Golden: `js_code_block_highlighter.json`.

mod builtin;

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
    let key = info.to_lowercase();
    LANGUAGE_PARSERS
        .iter()
        .find(|(alias, _)| *alias == key)
        .map(|(_, lang)| *lang)
}

const fn lang(id: &'static str, label: &'static str) -> Language {
    Language { id, label }
}

const JAVASCRIPT: Language = lang("javascript", "JavaScript");
const JSX: Language = lang("jsx", "JSX");
const TYPESCRIPT: Language = lang("typescript", "TypeScript");
const TSX: Language = lang("tsx", "TSX");
const HTML: Language = lang("html", "HTML");
const SVG: Language = lang("svg", "SVG");
const CSS: Language = lang("css", "CSS");
const JSON: Language = lang("json", "JSON");
const JSONC: Language = lang("jsonc", "JSONC");
const YAML: Language = lang("yaml", "YAML");
const PHP: Language = lang("php", "PHP");
const RUBY: Language = lang("ruby", "Ruby");
const DOTENV: Language = lang("dotenv", "Dotenv");
const SHELL: Language = lang("shell", "Shell");
const SQL: Language = lang("sql", "SQL");
const POSTGRES: Language = lang("postgresql", "PostgreSQL");
const MYSQL: Language = lang("mysql", "MySQL");
const MARIADB: Language = lang("mariadb", "MariaDB");
const SQLITE: Language = lang("sqlite", "SQLite");
const TOML: Language = lang("toml", "TOML");
const DOCKERFILE: Language = lang("dockerfile", "Dockerfile");
const TERRAFORM: Language = lang("terraform", "Terraform");
const HCL: Language = lang("hcl", "HCL");
const RUST: Language = lang("rust", "Rust");
const GO: Language = lang("go", "Go");
const PYTHON: Language = lang("python", "Python");
const JAVA: Language = lang("java", "Java");
const C: Language = lang("c", "C");
const CPP: Language = lang("cpp", "C++");
const CSHARP: Language = lang("csharp", "C#");
const KOTLIN: Language = lang("kotlin", "Kotlin");

/// FrankMD's explicit `LANGUAGE_PARSERS` map (fence alias → language). Fence
/// names are untrusted note content, so only these are ever selected.
const LANGUAGE_PARSERS: &[(&str, Language)] = &[
    ("js", JAVASCRIPT),
    ("javascript", JAVASCRIPT),
    ("mjs", JAVASCRIPT),
    ("cjs", JAVASCRIPT),
    ("jsx", JSX),
    ("ts", TYPESCRIPT),
    ("typescript", TYPESCRIPT),
    ("tsx", TSX),
    ("html", HTML),
    ("htm", HTML),
    ("svg", SVG),
    ("css", CSS),
    ("json", JSON),
    ("jsonc", JSONC),
    ("yaml", YAML),
    ("yml", YAML),
    (".yaml", YAML),
    (".yml", YAML),
    ("php", PHP),
    ("phtml", PHP),
    ("ruby", RUBY),
    ("rb", RUBY),
    (".rb", RUBY),
    ("rake", RUBY),
    ("env", DOTENV),
    (".env", DOTENV),
    ("dotenv", DOTENV),
    ("shell", SHELL),
    ("bash", SHELL),
    ("sh", SHELL),
    ("zsh", SHELL),
    ("sql", SQL),
    ("postgres", POSTGRES),
    ("postgresql", POSTGRES),
    ("pgsql", POSTGRES),
    ("mysql", MYSQL),
    ("mariadb", MARIADB),
    ("sqlite", SQLITE),
    ("toml", TOML),
    ("tml", TOML),
    ("dockerfile", DOCKERFILE),
    ("docker", DOCKERFILE),
    ("tf", TERRAFORM),
    (".tf", TERRAFORM),
    ("tfvars", TERRAFORM),
    (".tfvars", TERRAFORM),
    ("terraform", TERRAFORM),
    ("hcl", HCL),
    (".hcl", HCL),
    ("rs", RUST),
    ("rust", RUST),
    ("go", GO),
    ("golang", GO),
    ("py", PYTHON),
    ("python", PYTHON),
    ("python3", PYTHON),
    ("java", JAVA),
    ("c", C),
    ("h", C),
    ("ino", C),
    ("cc", CPP),
    ("cpp", CPP),
    ("c++", CPP),
    ("cxx", CPP),
    ("hpp", CPP),
    ("hh", CPP),
    ("hxx", CPP),
    ("cs", CSHARP),
    ("csharp", CSHARP),
    ("c#", CSHARP),
    ("kotlin", KOTLIN),
    ("kt", KOTLIN),
    ("kts", KOTLIN),
];

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
        Self {
            max_block_len: 20_000,
            max_total_len: 50_000,
            max_blocks: 100,
        }
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
    let skipped = BlockPlan {
        language: None,
        highlight: false,
    };
    let mut remaining = budget.max_total_len;
    let mut out = Vec::with_capacity(blocks.len());
    for (ix, &(token, len)) in blocks.iter().enumerate() {
        if ix >= budget.max_blocks {
            // The original `break`s: later blocks are never even labelled.
            out.push(skipped);
            continue;
        }
        let Some(language) = language_for_fence(token) else {
            out.push(skipped);
            continue;
        };
        let fits = remaining > 0 && len <= budget.max_block_len && len <= remaining;
        if fits {
            remaining -= len;
        }
        out.push(BlockPlan {
            language: Some(language),
            highlight: fits,
        });
    }
    out
}

/// Unhighlighted lines for `code` as marked emits it: one line per `\n`
/// separated segment, a single trailing newline not starting a new line.
/// Use this for blocks the [`plan`] does not highlight so the text matches
/// what [`BuiltinHighlighter`] would have produced.
pub fn plain_lines(code: &str) -> Vec<HighlightedLine> {
    split_code_lines(code)
        .map(|text| HighlightedLine {
            text: text.to_owned(),
            runs: Vec::new(),
        })
        .collect()
}

/// Split code into lines without their newlines (see [`plain_lines`]).
fn split_code_lines(code: &str) -> std::str::Split<'_, char> {
    code.strip_suffix('\n').unwrap_or(code).split('\n')
}

/// Headless highlighter covering every language in the FrankMD table
/// (syntect/two-face, plus [`hcl`] for Terraform/HCL).
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltinHighlighter;

impl CodeHighlighter for BuiltinHighlighter {
    fn highlight(&self, lang: Language, code: &str) -> Option<Vec<HighlightedLine>> {
        let lines: Vec<&str> = split_code_lines(code).collect();
        match lang.id {
            "terraform" | "hcl" => Some(builtin::highlight_hcl(&lines)),
            id => builtin::highlight_syntect(id, &lines),
        }
    }
}

/// **[COMPAT]** FrankMD's hand-written Terraform/HCL stream tokenizer
/// (`terraform_hcl_stream_parser.js`), line-at-a-time like CodeMirror's
/// `StreamLanguage`. Golden: `js_terraform_hcl_stream_parser.json`.
pub mod hcl;
