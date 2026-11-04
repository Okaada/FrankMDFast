//! The headless highlighting backends behind [`super::BuiltinHighlighter`]:
//! syntect (two-face's grammar set) for most languages, mapped onto the
//! theme roles of [`TokenClass`], and the ported HCL tokenizer.
//!
//! syntect scopes are TextMate names, not Lezer tags, so [`role`] translates
//! them with a few per-language knobs chosen to give the same roles FrankMD's
//! Lezer/legacy-mode parsers give (see `code_block_highlighter_test.rs`).

use std::ops::Range;
use std::sync::LazyLock;

use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

use super::hcl::{self, HclState, HclStyle};
use crate::ir::{HighlightedLine, TokenClass, TokenStyle};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

/// syntect grammar name for a [`super::Language`] id.
fn syntax_name(id: &str) -> Option<&'static str> {
    Some(match id {
        "javascript" => "JavaScript",
        "jsx" | "tsx" => "TypeScriptReact",
        "typescript" => "TypeScript",
        "html" | "svg" => "HTML",
        "css" => "CSS",
        "json" | "jsonc" => "JSON",
        "yaml" => "YAML",
        "php" => "PHP",
        "ruby" => "Ruby",
        "dotenv" => "DotENV",
        "shell" => "Bourne Again Shell (bash)",
        "sql" | "postgresql" | "mysql" | "mariadb" | "sqlite" => "SQL",
        "toml" => "TOML",
        "dockerfile" => "Dockerfile",
        "rust" => "Rust",
        "go" => "Go",
        "python" => "Python",
        "java" => "Java",
        "c" => "C",
        "cpp" => "C++",
        "csharp" => "C#",
        "kotlin" => "Kotlin",
        _ => return None,
    })
}

fn style(class: TokenClass, definition: bool) -> TokenStyle {
    TokenStyle { class, definition }
}

/// Highlight with syntect. `None` when the language has no grammar. A
/// grammar error leaves the block plain (FrankMD's `catch` keeps the source
/// readable).
pub(super) fn highlight_syntect(id: &str, lines: &[&str]) -> Option<Vec<HighlightedLine>> {
    let set = &*SYNTAXES;
    let syntax: &SyntaxReference = set.find_syntax_by_name(syntax_name(id)?)?;
    let plain = || {
        lines
            .iter()
            .map(|l| HighlightedLine {
                text: (*l).to_owned(),
                runs: Vec::new(),
            })
            .collect()
    };
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut prev_token = String::new();
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        // The grammars are the "newlines" variants: feed each line with its `\n`.
        let with_nl = format!("{line}\n");
        let Ok(ops) = state.parse_line(&with_nl, set) else {
            return Some(plain());
        };
        let mut runs = Vec::new();
        let mut pos = 0;
        let ends = ops
            .iter()
            .map(|(i, op)| (*i, Some(op)))
            .chain(std::iter::once((with_nl.len(), None)));
        for (at, op) in ends {
            let end = at.min(line.len());
            if end > pos {
                let text = &line[pos..end];
                let names: Vec<String> =
                    stack.as_slice().iter().map(|s| s.build_string()).collect();
                let s = role(id, &names, text, &prev_token);
                if s.class != TokenClass::Plain {
                    runs.push((pos..end, s));
                } else if id == "css" {
                    runs.extend(css_bare_selectors(&names, text, pos));
                }
                if !text.trim().is_empty() {
                    text.trim().clone_into(&mut prev_token);
                }
                pos = end;
            }
            if let Some(op) = op
                && stack.apply(op).is_err()
            {
                return Some(plain());
            }
        }
        out.push(HighlightedLine {
            text: (*line).to_owned(),
            runs: tidy_runs(line, runs),
        });
    }
    Some(out)
}

/// Merge touching runs of the same style, then trim whitespace off run edges
/// (so `//` + ` note` is one `// note` comment and `RUN ` is `RUN`).
fn tidy_runs(line: &str, runs: Vec<(Range<usize>, TokenStyle)>) -> Vec<(Range<usize>, TokenStyle)> {
    let mut merged: Vec<(Range<usize>, TokenStyle)> = Vec::with_capacity(runs.len());
    for (range, s) in runs {
        match merged.last_mut() {
            Some((last, ls)) if last.end == range.start && *ls == s => last.end = range.end,
            _ => merged.push((range, s)),
        }
    }
    merged
        .into_iter()
        .filter_map(|(r, s)| {
            let text = &line[r.clone()];
            let start = r.start + (text.len() - text.trim_start().len());
            let end = r.end - (text.len() - text.trim_end().len());
            (start < end).then_some((start..end, s))
        })
        .collect()
}

/// Scopes that open code inside a string (`#{..}`, `${..}`, `$var` in shell):
/// below them the enclosing string no longer colors the text.
fn is_string_barrier(scope: &str) -> bool {
    [
        "meta.interpolation",
        "meta.embedded",
        "meta.group.expansion",
        "meta.template.expression",
        "source.",
    ]
    .iter()
    .any(|p| scope.starts_with(p))
}

/// Punctuation that belongs to its parent construct (string quotes, comment
/// markers, `$` sigils, interpolation braces): it takes the parent's role.
fn is_transparent_punctuation(scope: &str) -> bool {
    [
        "punctuation.definition.string",
        "punctuation.definition.comment",
        "punctuation.definition.variable",
        "punctuation.section.interpolation",
        "punctuation.section.embedded",
    ]
    .iter()
    .any(|p| scope.starts_with(p))
}

const RUST_PRIMITIVES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char", "str",
];

/// Role of a token from its scope stack (outermost first).
fn role(lang: &str, names: &[String], text: &str, prev_token: &str) -> TokenStyle {
    let plain = style(TokenClass::Plain, false);
    if lang == "shell" && names.iter().any(|n| n.starts_with("meta.group.expansion")) {
        // Legacy shell mode styles `$var` expansions as definitions.
        return style(TokenClass::Identifier, true);
    }
    if let Some(si) = names.iter().rposition(|n| n.starts_with("string.")) {
        let inner = &names[si + 1..];
        let code_inside = inner.iter().any(|n| is_string_barrier(n));
        // YAML keys are `entity.name.tag` inside a plain-scalar string scope.
        let key_inside = inner.iter().any(|n| n.starts_with("entity.name.tag"));
        if !code_inside && !key_inside {
            return style(TokenClass::String, false);
        }
    }
    names
        .iter()
        .rev()
        .find_map(|n| scope_role(lang, n, names, text, prev_token))
        .unwrap_or(plain)
}

/// Lezer's CSS parser tags any selector identifier (`x`, `my-tag`) as a type
/// name; the TextMate grammar leaves unknown elements unscoped. Returns the
/// identifier runs of an unscoped top-level CSS segment starting at `base`.
fn css_bare_selectors(
    names: &[String],
    text: &str,
    base: usize,
) -> Vec<(Range<usize>, TokenStyle)> {
    if !names
        .iter()
        .all(|n| n == "source.css" || n.starts_with("meta.selector"))
    {
        return Vec::new();
    }
    let mut runs = Vec::new();
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        let ident = c.is_ascii_alphanumeric() || c == '-' || c == '_';
        match (start, ident) {
            (None, true) if c.is_ascii_alphabetic() => start = Some(i),
            (Some(s), false) => {
                runs.push((base + s..base + i, style(TokenClass::Type, false)));
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// Role for one scope name, `None` to defer to the enclosing scope.
fn scope_role(
    lang: &str,
    n: &str,
    names: &[String],
    text: &str,
    prev_token: &str,
) -> Option<TokenStyle> {
    use TokenClass::*;
    let has = |p: &str| names.iter().any(|x| x.starts_with(p));
    let s = |c| Some(style(c, false));
    if n.starts_with("comment") {
        return s(Comment);
    }
    if is_transparent_punctuation(n) {
        return None;
    }
    if lang == "toml"
        && (n.starts_with("punctuation.definition.table") || n.starts_with("entity.name.table"))
    {
        return s(Constant);
    }
    if n.starts_with("punctuation") {
        return s(Punctuation);
    }
    if n.starts_with("invalid") {
        return s(Invalid);
    }
    if n.starts_with("keyword.other.unit") {
        return s(Number);
    }
    if n.starts_with("keyword.operator") {
        // Word operators (`AND`, `in`, `instanceof`) read as keywords.
        let wordy = text.chars().all(|c| c.is_alphabetic() || c == '_');
        return s(if wordy { Keyword } else { Operator });
    }
    if n.starts_with("keyword") {
        return s(Keyword);
    }
    if n.starts_with("storage.type") {
        return s(storage_type_role(lang, n, text));
    }
    if n.starts_with("storage") {
        return s(Keyword);
    }
    if n.starts_with("entity.name.function") {
        let class = if matches!(lang, "java" | "ruby") {
            Identifier
        } else {
            Function
        };
        return Some(style(class, !has("meta.function-call")));
    }
    if n.starts_with("entity.name.tag") {
        return match lang {
            "yaml" => Some(style(Property, true)),
            "toml" => s(Property),
            _ => s(Type),
        };
    }
    if [
        "entity.name.class",
        "entity.name.type",
        "entity.name.struct",
        "entity.name.enum",
        "entity.name.trait",
    ]
    .iter()
    .chain(&["entity.name.interface", "entity.name.union"])
    .any(|p| n.starts_with(p))
    {
        return match lang {
            "java" | "kotlin" | "csharp" => Some(style(Identifier, true)),
            "ruby" => s(Type),
            _ => Some(style(Type, true)),
        };
    }
    if n.starts_with("entity.name.namespace") || n.starts_with("entity.name.module") {
        return s(Type);
    }
    if n.starts_with("entity.name") {
        return Some(style(Identifier, true));
    }
    if n.starts_with("entity.other.attribute-name") {
        return s(Property);
    }
    if n.starts_with("entity.other.inherited-class") {
        return s(Type);
    }
    if n.starts_with("support.type.property-name") {
        return s(Property);
    }
    if n.starts_with("support.type") {
        return s(if lang == "python" { Identifier } else { Type });
    }
    if n.starts_with("support.class") {
        return s(Type);
    }
    if n.starts_with("support.function") {
        return s(if lang == "shell" {
            Identifier
        } else {
            Function
        });
    }
    if n.starts_with("support.macro") {
        return s(Function);
    }
    if n.starts_with("support.constant") {
        return s(Constant);
    }
    if n.starts_with("support.variable") {
        return s(Identifier);
    }
    if n.starts_with("constant.numeric") {
        return s(Number);
    }
    if n.starts_with("constant.character") {
        return s(String);
    }
    if n.starts_with("constant") {
        return s(Constant);
    }
    if n.starts_with("variable.function") {
        // Lezer Go styles `pkg.Func(` as a property access.
        return s(if lang == "go" && prev_token == "." {
            Property
        } else {
            Function
        });
    }
    if n.starts_with("variable.parameter") {
        return Some(style(Identifier, true));
    }
    if n.starts_with("variable.language") {
        return s(Keyword);
    }
    if n.starts_with("variable.other.member") || n.starts_with("variable.other.property") {
        return s(Property);
    }
    if n.starts_with("variable") {
        let def = lang == "dotenv" || has("meta.binding.name") || has("meta.definition.variable");
        return Some(style(Identifier, def));
    }
    if n.starts_with("meta.string") {
        // Interpolation delimiters (`#{`, `}`) sit in the string's meta scope.
        return s(String);
    }
    if n.starts_with("meta.mapping.key") {
        return Some(style(Property, true));
    }
    None
}

/// `storage.type.*`: declaration keywords (`fn`, `class`, `const`) versus
/// type names (`int`, `string`), which grammars spell inconsistently.
fn storage_type_role(lang: &str, n: &str, text: &str) -> TokenClass {
    if n.starts_with("storage.type.string") {
        // Python's `f` / `r` string prefixes.
        return TokenClass::String;
    }
    if n.starts_with("storage.type.function.arrow") {
        return TokenClass::Operator;
    }
    if [
        "storage.type.primitive",
        "storage.type.buildin",
        "storage.type.builtin",
    ]
    .iter()
    .any(|p| n.starts_with(p))
    {
        return TokenClass::Type;
    }
    // A bare `storage.type.<lang>` (three components) is ambiguous.
    let bare = n.split('.').count() == 3;
    if !bare {
        return TokenClass::Keyword;
    }
    match lang {
        "php" | "c" | "cpp" | "csharp" | "go" => TokenClass::Type,
        "rust" if RUST_PRIMITIVES.contains(&text) => TokenClass::Type,
        _ => TokenClass::Keyword,
    }
}

/// Highlight Terraform/HCL with the ported FrankMD stream tokenizer.
pub(super) fn highlight_hcl(lines: &[&str]) -> Vec<HighlightedLine> {
    let mut state = HclState::default();
    lines
        .iter()
        .map(|line| {
            let mut runs = Vec::new();
            if line.is_empty() {
                hcl::blank_line(&mut state);
            } else {
                let mut pos = 0;
                for (text, style) in hcl::tokenize_line(line, &mut state) {
                    let end = pos + text.len();
                    if let Some(st) = style {
                        runs.push((
                            pos..end,
                            TokenStyle {
                                class: hcl_class(st),
                                definition: false,
                            },
                        ));
                    }
                    pos = end;
                }
            }
            HighlightedLine {
                text: (*line).to_owned(),
                runs,
            }
        })
        .collect()
}

/// StreamLanguage style → theme role (`atom` → constant, `variableName` →
/// identifier, `variableName.function` → function).
fn hcl_class(style: HclStyle) -> TokenClass {
    match style {
        HclStyle::Keyword => TokenClass::Keyword,
        HclStyle::String => TokenClass::String,
        HclStyle::Comment => TokenClass::Comment,
        HclStyle::Atom => TokenClass::Constant,
        HclStyle::Number => TokenClass::Number,
        HclStyle::Function => TokenClass::Function,
        HclStyle::Property => TokenClass::Property,
        HclStyle::Variable => TokenClass::Identifier,
        HclStyle::Operator => TokenClass::Operator,
        HclStyle::Punctuation => TokenClass::Punctuation,
    }
}
