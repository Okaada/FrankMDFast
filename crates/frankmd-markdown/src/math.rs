//! Math. The IR keeps TeX source; the app renders it with RaTeX
//! (`ratex-parser` → `ratex-layout` → display list) into GPUI paint calls or a
//! cached raster. See docs/research/gpui-stack.md §Math.
//!
//! **[COMPAT]** FrankMD's KaTeX options (`math_renderer.js`):
//! `{throwOnError: false, trust: false, strict: "ignore", maxExpand: 1000}`.
//! Golden: `js_math_renderer.json`.

/// Inline vs display math.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MathStyle {
    /// `$...$` (KaTeX `displayMode: false`).
    Inline,
    /// `$$...$$` (KaTeX `displayMode: true`).
    Display,
}

/// The hardened math policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathPolicy {
    /// `trust`: `\href`, `\url`, `\htmlClass`, `\htmlId`, `\htmlStyle`,
    /// `\htmlData`, `\includegraphics` are never executed when false.
    pub trust: bool,
    /// Errors render inline (source shown) instead of failing the preview.
    pub throw_on_error: bool,
    /// Macro expansion bound (DoS guard).
    pub max_expand: u32,
}

impl MathPolicy {
    /// FrankMD's options.
    pub const FRANKMD: MathPolicy = MathPolicy {
        trust: false,
        throw_on_error: false,
        max_expand: 1000,
    };
}

/// Why a TeX snippet does not render cleanly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathError {
    /// KaTeX `ParseError` (incl. "Too many expansions"): the preview shows the
    /// raw source in the error color (`katex-error`).
    Parse(String),
    /// Parsed, but these commands are unknown or untrusted (KaTeX with
    /// `throwOnError: false` renders each as red text, `color: #cc0000`).
    Unsupported(Vec<String>),
}

/// Commands KaTeX refuses under `trust: false`: each renders as red text
/// (its arguments are swallowed) instead of executing.
const UNTRUSTED_COMMANDS: [&str; 7] = [
    r"\href",
    r"\url",
    r"\htmlClass",
    r"\htmlId",
    r"\htmlStyle",
    r"\htmlData",
    r"\includegraphics",
];

/// Upper bound on re-parses while neutralizing undefined commands.
const MAX_REPARSES: usize = 64;

/// Prefix of the parser's undefined-command error message.
const UNDEFINED_PREFIX: &str = "Undefined control sequence: ";

/// Validate TeX cheaply under [`MathPolicy::FRANKMD`] so the preview can show
/// the source on errors (KaTeX `throwOnError: false` behaviour).
///
/// Mirrors KaTeX's verdicts: a parse error (including "Too many expansions"
/// past `maxExpand`) is [`MathError::Parse`]; otherwise every unknown command
/// and every untrusted command is listed, in source order, as
/// [`MathError::Unsupported`]. KaTeX renders an unknown command as a plain
/// ord, so the parser is re-run with each one neutralized (an empty macro)
/// until it either succeeds or fails for another reason. The style does not
/// change what parses.
pub fn validate(tex: &str, _style: MathStyle) -> Result<(), MathError> {
    let mut undefined: Vec<String> = Vec::new();
    for _ in 0..MAX_REPARSES {
        let mut parser = ratex_parser::Parser::new(tex);
        for name in &undefined {
            parser.gullet.set_macro(
                name.clone(),
                ratex_parser::macro_expander::MacroDefinition::Tokens {
                    tokens: Vec::new(),
                    num_args: 0,
                },
            );
        }
        match parser.parse() {
            Ok(_) => return unsupported_in(tex, &undefined),
            Err(err) => match err.message.strip_prefix(UNDEFINED_PREFIX) {
                Some(name) if !undefined.iter().any(|u| u == name) => {
                    undefined.push(name.to_owned())
                }
                _ => return Err(MathError::Parse(err.message)),
            },
        }
    }
    Err(MathError::Parse(
        "too many undefined control sequences".to_owned(),
    ))
}

/// The verdict for TeX that parsed: unknown (`undefined`) and untrusted
/// commands in source order, one entry per occurrence.
fn unsupported_in(tex: &str, undefined: &[String]) -> Result<(), MathError> {
    let found: Vec<String> = control_words(tex)
        .filter(|w| UNTRUSTED_COMMANDS.contains(w) || undefined.iter().any(|u| u == w))
        .map(str::to_owned)
        .collect();
    if found.is_empty() {
        Ok(())
    } else {
        Err(MathError::Unsupported(found))
    }
}

/// Control words (`\name`, letters only) in source order. Control symbols
/// like `\$` are skipped together with their character.
fn control_words(tex: &str) -> impl Iterator<Item = &str> {
    let bytes = tex.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < bytes.len() {
            if bytes[i] != b'\\' {
                i += 1;
                continue;
            }
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i - start > 1 {
                return Some(&tex[start..i]);
            }
            // Control symbol: skip the escaped character (one UTF-8 char).
            if let Some(c) = tex[i..].chars().next() {
                i += c.len_utf8();
            }
        }
        None
    })
}
