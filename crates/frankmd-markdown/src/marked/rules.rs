//! marked 15.0.6's grammar (`rules.ts`) with the app's options (`gfm: true,
//! breaks: true`), already expanded (marked builds them with `edit().replace()`).
//!
//! The sources are kept in JavaScript regex syntax and translated by [`js`]
//! so they can be compared line by line with marked's. The translation keeps
//! JS semantics where Rust differs: `\s` is the ECMAScript whitespace set,
//! `\w`/`\d` are ASCII, `.` excludes the four JS line terminators.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use fancy_regex::{Captures, Regex, RegexBuilder};

/// ECMAScript `WhiteSpace` + `LineTerminator` (the members of `\s`).
const JS_SPACE: &str = r"\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";

/// Translate a JavaScript regex source to fancy-regex syntax (see module docs).
fn translate(src: &str) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    let mut in_class = false;
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let Some(n) = chars.next() else { break };
                match (n, in_class) {
                    ('s', true) => out.push_str(JS_SPACE),
                    ('s', false) => out.push_str(&format!("[{JS_SPACE}]")),
                    ('S', false) => out.push_str(&format!("[^{JS_SPACE}]")),
                    ('w', true) => out.push_str("A-Za-z0-9_"),
                    ('w', false) => out.push_str("[A-Za-z0-9_]"),
                    ('d', true) => out.push_str("0-9"),
                    ('d', false) => out.push_str("[0-9]"),
                    ('b', false) => out.push_str(r"(?-u:\b)"),
                    ('/', _) => out.push('/'),
                    _ => {
                        out.push('\\');
                        out.push(n);
                    }
                }
            }
            '[' if !in_class && rest_starts_with(&chars, r"\s\S]") => {
                // `[\s\S]` = any char; the JS `\s` expansion would lose U+0085.
                out.push_str("(?s:.)");
                for _ in 0..5 {
                    chars.next();
                }
            }
            '[' if !in_class => {
                in_class = true;
                out.push('[');
                if chars.peek() == Some(&'^') {
                    out.push('^');
                    chars.next();
                }
            }
            '[' => out.push_str(r"\["),
            ']' if in_class => {
                in_class = false;
                out.push(']');
            }
            '&' | '~' if in_class => {
                out.push('\\');
                out.push(c);
            }
            '.' if !in_class => out.push_str(r"[^\n\r\x{2028}\x{2029}]"),
            _ => out.push(c),
        }
    }
    out
}

fn rest_starts_with(chars: &std::iter::Peekable<std::str::Chars<'_>>, prefix: &str) -> bool {
    chars.clone().take(prefix.chars().count()).eq(prefix.chars())
}

/// Compile a JS regex source (`flags` may contain `i`).
pub(crate) fn js(src: &str, flags: &str) -> Regex {
    let mut pattern = translate(src);
    if flags.contains('i') {
        pattern = format!("(?i){pattern}");
    }
    RegexBuilder::new(&pattern)
        .backtrack_limit(10_000_000)
        .build()
        .unwrap_or_else(|e| panic!("marked rule does not compile: {e}\n{src}"))
}

/// `regex.exec(src)` for a `^`-anchored rule.
pub(crate) fn exec<'t>(re: &Regex, src: &'t str) -> Option<Captures<'t, str>> {
    re.captures_input(fancy_regex::RegexInput::new(src).anchored(true)).ok().flatten()
}

/// `regex.exec(src)` for an unanchored rule (first match anywhere).
pub(crate) fn search<'t>(re: &Regex, src: &'t str) -> Option<Captures<'t, str>> {
    re.captures(src).ok().flatten()
}

/// Global-flag `exec` from `last_index`.
pub(crate) fn exec_from<'t>(re: &Regex, src: &'t str, last_index: usize) -> Option<Captures<'t, str>> {
    if last_index > src.len() {
        return None;
    }
    re.captures_from_pos(src, last_index).ok().flatten()
}

/// Group `i` as `&str`, `""` when it did not participate (`cap[i] || ''`).
pub(crate) fn group<'t>(cap: &Captures<'t, str>, i: usize) -> &'t str {
    cap.get(i).map_or("", |m| m.as_str())
}

/// Whether group `i` participated with a non-empty match (JS truthiness).
pub(crate) fn has(cap: &Captures<'_, str>, i: usize) -> bool {
    cap.get(i).is_some_and(|m| !m.as_str().is_empty())
}

macro_rules! rule {
    ($name:ident, $src:expr) => {
        rule!($name, $src, "");
    };
    ($name:ident, $src:expr, $flags:expr) => {
        pub(crate) static $name: LazyLock<Regex> = LazyLock::new(|| js(&$src, $flags));
    };
}

// ---------------------------------------------------------------------------
// Shared fragments (marked's `edit().replace()` inputs, carets removed)
// ---------------------------------------------------------------------------

const BULLET: &str = r"(?:[*+-]|\d{1,9}[.)])";
const HR_BODY: &str = r" {0,3}((?:-[\t ]*){3,}|(?:_[ \t]*){3,}|(?:\*[ \t]*){3,})(?:\n+|$)";
const TAG_NAMES: &str = "address|article|aside|base|basefont|blockquote|body|caption\
|center|col|colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption\
|figure|footer|form|frame|frameset|h[1-6]|head|header|hr|html|iframe\
|legend|li|link|main|menu|menuitem|meta|nav|noframes|ol|optgroup|option\
|p|param|search|section|summary|table|tbody|td|tfoot|th|thead|title\
|tr|track|ul";
const COMMENT: &str = r"<!--(?:-?>|[\s\S]*?(?:-->|$))";
const BLOCK_LABEL: &str = r"(?!\s*\])(?:\\.|[^\[\]\\])+";
const INLINE_LABEL: &str = r"(?:\[(?:\\.|[^\[\]\\])*\]|\\.|`[^`]*`|[^\[\]\\`])*?";
const PUNCT: &str = r"[\p{P}\p{S}]";
const PUNCT_SPACE: &str = r"[\s\p{P}\p{S}]";
const NOT_PUNCT_SPACE: &str = r"[^\s\p{P}\p{S}]";
const PUNCT_GFM: &str = r"(?!~)[\p{P}\p{S}]";
const PUNCT_SPACE_GFM: &str = r"(?!~)[\s\p{P}\p{S}]";
const NOT_PUNCT_SPACE_GFM: &str = r"(?:[^\s\p{P}\p{S}]|~)";

fn heading_interrupt() -> &'static str {
    r" {0,3}#{1,6}(?:\s|$)"
}
fn fences_interrupt() -> &'static str {
    r" {0,3}(?:`{3,}(?=[^`\n]*\n)|~{3,})[^\n]*\n"
}
fn list_interrupt() -> &'static str {
    r" {0,3}(?:[*+-]|1[.)]) "
}
fn html_interrupt() -> String {
    format!(r"</?(?:{TAG_NAMES})(?: +|\n|/?>)|<(?:script|pre|style|textarea|!--)")
}

/// The blocks that end a table's rows (its rows' negative look-ahead), anchored.
pub(crate) fn table_row_stop() -> Regex {
    js(
        &format!(
            r"^(?: *\n|{hr}|{heading}| {{0,3}}>|(?: {{4}}| {{0,3}}\t)[^\n]|{fences}|{list}|{html})",
            hr = HR_BODY,
            heading = heading_interrupt(),
            fences = fences_interrupt(),
            list = list_interrupt(),
            html = html_interrupt(),
        ),
        "",
    )
}

/// GFM table body (without the leading `^`), also used as a paragraph interrupt.
fn table_body() -> String {
    format!(
        r"{lead}\n {{0,3}}((?:\| *)?:?-+:? *(?:\| *:?-+:? *)*(?:\| *)?)(?:\n((?:(?! *\n|{hr}|{heading}| {{0,3}}>|(?: {{4}}| {{0,3}}\t)[^\n]|{fences}|{list}|{html}).*(?:\n|$))*)\n*|$)",
        lead = r" *([^\n ].*)",
        hr = HR_BODY,
        heading = heading_interrupt(),
        fences = fences_interrupt(),
        list = list_interrupt(),
        html = html_interrupt(),
    )
}

/// The non-table interrupts of a GFM paragraph line, anchored at the line.
pub(crate) fn paragraph_stop() -> Regex {
    js(
        &format!(
            r"^(?:{hr}|{heading}| {{0,3}}>|{fences}|{list}|{html}| +\n)",
            hr = HR_BODY,
            heading = heading_interrupt(),
            fences = fences_interrupt(),
            list = list_interrupt(),
            html = html_interrupt(),
        ),
        "",
    )
}

/// marked's normal-grammar paragraph body (no table interrupt), carets removed.
fn paragraph_body(with_table: bool) -> String {
    let table = if with_table { format!("|{}", table_body()) } else { String::new() };
    format!(
        r"([^\n]+(?:\n(?!{hr}|{heading}| {{0,3}}>|{fences}|{list}|{html}{table}| +\n)[^\n]+)*)",
        hr = HR_BODY,
        heading = heading_interrupt(),
        fences = fences_interrupt(),
        list = list_interrupt(),
        html = html_interrupt(),
    )
}

// ---------------------------------------------------------------------------
// Block grammar (GFM)
// ---------------------------------------------------------------------------

#[cfg(test)]
rule!(NEWLINE, r"^(?:[ \t]*(?:\n|$))+");
rule!(BLOCK_CODE, r"^((?: {4}| {0,3}\t)[^\n]+(?:\n(?:[ \t]*(?:\n|$))*)?)+");
#[cfg(test)]
rule!(
    FENCES,
    r"^ {0,3}(`{3,}(?=[^`\n]*(?:\n|$))|~{3,})([^\n]*)(?:\n|$)(?:|([\s\S]*?)(?:\n|$))(?: {0,3}\1[~`]* *(?=\n|$)|$)"
);
rule!(HR, format!("^{HR_BODY}"));
rule!(HEADING, r"^ {0,3}(#{1,6})(?=\s|$)(.*)(?:\n+|$)");
rule!(LHEADING, {
    let interrupts = format!(r"{BULLET} |(?: {{4}}| {{0,3}}\t)| {{0,3}}(?:`{{3,}}|~{{3,}})| {{0,3}}>| {{0,3}}#{{1,6}}| {{0,3}}<[^\n>]+>\n");
    format!(r"^(?!{interrupts})((?:.|\n(?!\s*?\n|{interrupts}))+?)\n {{0,3}}(=+|-+) *(?:\n+|$)")
});
#[cfg(test)]
rule!(PARAGRAPH, format!("^{}", paragraph_body(true)));
rule!(
    DEF,
    format!(
        r#"^ {{0,3}}\[({BLOCK_LABEL})\]: *(?:\n[ \t]*)?([^<\s][^\s]*|<.*?>)(?:(?: +(?:\n[ \t]*)?| *\n[ \t]*)({title}))? *(?:\n+|$)"#,
        title = r#"(?:"(?:\\"?|[^"\\])*"|'[^'\n]*(?:\n[^'\n]+)*\n?'|\([^()]*\))"#
    )
);
rule!(LIST, format!(r"^( {{0,3}}{BULLET})([ \t][^\n]+?)?(?:\n|$)"));
rule!(
    HTML,
    format!(
        r#"^ {{0,3}}(?:<(script|pre|style|textarea)[\s>][\s\S]*?(?:</\1>[^\n]*\n+|$)|{COMMENT}[^\n]*(\n+|$)|<\?[\s\S]*?(?:\?>\n*|$)|<![A-Z][\s\S]*?(?:>\n*|$)|<!\[CDATA\[[\s\S]*?(?:\]\]>\n*|$)|</?({TAG_NAMES})(?: +|\n|/?>)[\s\S]*?(?:(?:\n[ \t]*)+\n|$)|<(?!script|pre|style|textarea)([a-z][\w-]*)(?:{attr})*? */?>(?=[ \t]*(?:\n|$))[\s\S]*?(?:(?:\n[ \t]*)+\n|$)|</(?!script|pre|style|textarea)[a-z][\w-]*\s*>(?=[ \t]*(?:\n|$))[\s\S]*?(?:(?:\n[ \t]*)+\n|$))"#,
        attr = r#" +[a-zA-Z:_][\w.:-]*(?: *= *"[^"\n]*"| *= *'[^'\n]*'| *= *[^\s"'=<>`]+)?"#
    ),
    "i"
);
rule!(BLOCKQUOTE, format!(r"^( {{0,3}}> ?({}|[^\n]*)(?:\n|$))+", paragraph_body(false)));
#[cfg(test)]
rule!(TABLE, format!("^{}", table_body()));

// `other.*` helpers used by the block tokenizers.
rule!(CODE_REMOVE_INDENT_LINE, r"^(?: {1,4}| {0,3}\t)");
rule!(INDENT_CODE_COMPENSATION, r"^(\s+)(?:```)");
rule!(BLOCKQUOTE_START, r"^ {0,3}>");
rule!(BLOCKQUOTE_LINE_PREFIX, r"^ {0,3}>[ \t]?");
rule!(BLOCKQUOTE_SETEXT, r"\n {0,3}((?:=+|-+) *)(?=\n|$)");
rule!(LIST_IS_TASK, r"^\[[ xX]\] ");
rule!(LIST_REPLACE_TASK, r"^\[[ xX]\] +");
rule!(BLANK_LINE, r"^[ \t]*$");
rule!(ANY_LINE, r"\n.*\n");
rule!(TABLE_DELIMITER, r"[:|]");
rule!(TABLE_ALIGN_RIGHT, r"^ *-+: *$");
rule!(TABLE_ALIGN_CENTER, r"^ *:-+: *$");
rule!(TABLE_ALIGN_LEFT, r"^ *:-+ *$");
rule!(TABLE_ROW_BLANK_LINE, r"\n[ \t]*$");

/// `other.listItemRegex(bull)` (compiled once per bullet pattern).
pub(crate) fn list_item_regex(bull: &str) -> Arc<Regex> {
    static CACHE: LazyLock<Mutex<HashMap<String, Arc<Regex>>>> = LazyLock::new(Mutex::default);
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(bull.to_owned())
        .or_insert_with(|| Arc::new(js(&format!(r"^( {{0,3}}{bull})((?:[\t ][^\n]*)?(?:\n|$))"), "")))
        .clone()
}

/// The five `other.*BeginRegex(indent)` / `hrRegex(indent)` of a list item.
pub(crate) struct ItemBreakers {
    pub(crate) next_bullet: Regex,
    pub(crate) hr: Regex,
    pub(crate) fences: Regex,
    pub(crate) heading: Regex,
    pub(crate) html: Regex,
}

impl ItemBreakers {
    /// The breakers for a list item with content `indent` (only `min(3,
    /// indent - 1)` matters, so there are four distinct sets, compiled once).
    pub(crate) fn get(indent: usize) -> &'static ItemBreakers {
        static SETS: LazyLock<[ItemBreakers; 4]> = LazyLock::new(|| std::array::from_fn(ItemBreakers::new));
        &SETS[indent.saturating_sub(1).min(3)]
    }

    /// Whether `line` ends the item: a fence, heading, HTML block, next
    /// bullet or rule (each regex only runs when its first char can match).
    pub(crate) fn breaks(&self, line: &str) -> bool {
        let Some(c) = line.trim_start_matches(' ').chars().next() else { return false };
        (matches!(c, '`' | '~') && exec(&self.fences, line).is_some())
            || (c == '#' && exec(&self.heading, line).is_some())
            || (c == '<' && exec(&self.html, line).is_some())
            || ((matches!(c, '*' | '+' | '-') || c.is_ascii_digit()) && exec(&self.next_bullet, line).is_some())
            || (matches!(c, '-' | '_' | '*') && exec(&self.hr, line).is_some())
    }

    fn new(n: usize) -> Self {
        Self {
            next_bullet: js(&format!(r"^ {{0,{n}}}(?:[*+-]|\d{{1,9}}[.)])((?:[ \t][^\n]*)?(?:\n|$))"), ""),
            hr: js(&format!(r"^ {{0,{n}}}((?:- *){{3,}}|(?:_ *){{3,}}|(?:\* *){{3,}})(?:\n+|$)"), ""),
            fences: js(&format!(r"^ {{0,{n}}}(?:```|~~~)"), ""),
            heading: js(&format!(r"^ {{0,{n}}}#"), ""),
            html: js(&format!(r"^ {{0,{n}}}<(?:[a-z].*>|!--)"), "i"),
        }
    }
}

// ---------------------------------------------------------------------------
// Inline grammar (GFM + breaks)
// ---------------------------------------------------------------------------

rule!(ESCAPE, r##"^\\([!"#$%&'()*+,\-./:;<=>?@\[\]\\^_`{|}~])"##);
rule!(CODESPAN, r"^(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)");
rule!(BR, r"^( *|\\)\n(?!\s*$)");
#[cfg(test)]
rule!(
    INLINE_TEXT,
    r"^([`~]+|[^`~])(?:(?= *\n)|(?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)|[\s\S]*?(?:(?=[\\<!\[`*~_]| *\n|https?:\/\/|ftp:\/\/|www\.|$)|[^ ](?= *\n)|[^a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-](?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)))"
);
rule!(PUNCTUATION, format!(r"^((?![*_]){PUNCT_SPACE})"));
rule!(BLOCK_SKIP, r"\[[^\[\]]*?\]\((?:\\.|[^\\\(\)]|\((?:\\.|[^\\\(\)])*\))*\)|`[^`]*?`|<[^<>]*?>");
rule!(
    EM_STRONG_LDELIM,
    format!(r"^(?:\*+(?:((?!\*){PUNCT_GFM})|[^\s*]))|^_+(?:((?!_){PUNCT_GFM})|([^\s_]))")
);
rule!(
    EM_STRONG_RDELIM_AST,
    format!(
        r"^[^_*]*?__[^_*]*?\*[^_*]*?(?=__)|[^*]+(?=[^*])|(?!\*){p}(\*+)(?=[\s]|$)|{nps}(\*+)(?!\*)(?={ps}|$)|(?!\*){ps}(\*+)(?={nps})|[\s](\*+)(?!\*)(?={p})|(?!\*){p}(\*+)(?!\*)(?={p})|{nps}(\*+)(?={nps})",
        p = PUNCT_GFM,
        ps = PUNCT_SPACE_GFM,
        nps = NOT_PUNCT_SPACE_GFM
    )
);
rule!(
    EM_STRONG_RDELIM_UND,
    format!(
        r"^[^_*]*?\*\*[^_*]*?_[^_*]*?(?=\*\*)|[^_]+(?=[^_])|(?!_){p}(_+)(?=[\s]|$)|{nps}(_+)(?!_)(?={ps}|$)|(?!_){ps}(_+)(?={nps})|[\s](_+)(?!_)(?={p})|(?!_){p}(_+)(?!_)(?={p})",
        p = PUNCT,
        ps = PUNCT_SPACE,
        nps = NOT_PUNCT_SPACE
    )
);
rule!(ANY_PUNCTUATION, format!(r"\\({PUNCT})"));
// marked's caret stripping removes the `^` from the email class here.
rule!(
    AUTOLINK,
    r"^<([a-zA-Z][a-zA-Z0-9+.-]{1,31}:[^\s\x00-\x1f<>]*|[a-zA-Z0-9.!#$%&'*+/=?_`{|}~-]+(@)[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+(?![-_]))>"
);
rule!(
    TAG,
    r#"^<!--(?:-?>|[\s\S]*?-->)|^</[a-zA-Z][\w:-]*\s*>|^<[a-zA-Z][\w-]*(?:\s+[a-zA-Z:_][\w.:-]*(?:\s*=\s*"[^"]*"|\s*=\s*'[^']*'|\s*=\s*[^\s"'=<>`]+)?)*?\s*/?>|^<\?[\s\S]*?\?>|^<![a-zA-Z]+\s[\s\S]*?>|^<!\[CDATA\[[\s\S]*?\]\]>"#
);
rule!(
    LINK,
    format!(
        r#"^!?\[({INLINE_LABEL})\]\(\s*(<(?:\\.|[^\n<>\\])+>|[^\s\x00-\x1f]*)(?:\s+("(?:\\"?|[^"\\])*"|'(?:\\'?|[^'\\])*'|\((?:\\\)?|[^)\\])*\)))?\s*\)"#
    )
);
rule!(REFLINK, format!(r"^!?\[({INLINE_LABEL})\]\[({BLOCK_LABEL})\]"));
rule!(NOLINK, format!(r"^!?\[({BLOCK_LABEL})\](?:\[\])?"));
rule!(
    REFLINK_SEARCH,
    format!(r"!?\[({INLINE_LABEL})\]\[({BLOCK_LABEL})\]|!?\[({BLOCK_LABEL})\](?:\[\])?(?!\()")
);
rule!(
    URL,
    r"^((?:ftp|https?):\/\/|www\.)(?:[a-zA-Z0-9\-]+\.?)+[^\s<]*|^[A-Za-z0-9._+-]+(@)[a-zA-Z0-9\-_]+(?:\.[a-zA-Z0-9\-_]*[a-zA-Z0-9])+(?![-_])",
    "i"
);
rule!(BACKPEDAL, r#"(?:[^?!.,:;*_'"~()&]+|\([^)]*\)|&(?![a-zA-Z0-9]+;$)|[?!.,:;*_'"~)]+(?!$))+"#);
rule!(DEL, r"^(~~?)(?=[^\s~])((?:\\.|[^\\])*?(?:\\.|[^\s~\\]))\1(?=[^~]|$)");
rule!(UNICODE_ALNUM, r"[\p{L}\p{N}]");
rule!(START_A_TAG, r"^<a ", "i");
rule!(END_A_TAG, r"^</a>", "i");
rule!(START_PRE_SCRIPT_TAG, r"^<(pre|code|kbd|script)(\s|>)", "i");
rule!(END_PRE_SCRIPT_TAG, r"^</(pre|code|kbd|script)(\s|>)", "i");

/// `str.replace(anyPunctuation, '$1')`: drop the backslash of escaped punctuation.
pub(crate) fn unescape_punctuation(s: &str) -> String {
    ANY_PUNCTUATION.replace_all(s, "$1").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_compiles() {
        for re in [
            &BLOCK_CODE, &HR, &HEADING, &LHEADING, &DEF, &LIST, &HTML,
            &BLOCKQUOTE, &ESCAPE, &CODESPAN, &BR, &PUNCTUATION, &BLOCK_SKIP,
            &EM_STRONG_LDELIM, &EM_STRONG_RDELIM_AST, &EM_STRONG_RDELIM_UND, &ANY_PUNCTUATION, &AUTOLINK, &TAG,
            &LINK, &REFLINK, &NOLINK, &REFLINK_SEARCH, &URL, &BACKPEDAL, &DEL,
        ] {
            LazyLock::force(re);
        }
        let _ = ItemBreakers::get(2);
        let _ = list_item_regex(r"\-");
    }

    #[test]
    fn js_semantics() {
        assert!(exec(&HEADING, "# a").is_some());
        assert!(exec(&HEADING, "#a").is_none());
        assert!(exec(&HR, "- - -").is_some());
    }
}
