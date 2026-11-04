//! Block lexer: `Lexer.blockTokens` and the block tokenizers of
//! `Tokenizer.ts`, in marked's order.

use super::rules::{self, exec, group, search};
use super::token::{BlockKind, BlockTok, Inl, ListItemTok};
use super::{Lexer, LinkDef, fast, first_line, js_trim, js_trim_end, newlines, tail};
use crate::ir::Align;
use crate::parse::ext;

impl Lexer {
    /// `blockTokens(src, tokens, lastParagraphClipped)`; `base` is the line of
    /// `src`'s first line.
    pub(super) fn block_tokens(
        &mut self,
        src: &str,
        tokens: &mut Vec<BlockTok>,
        clipped: bool,
        base: u32,
    ) {
        let mut last_paragraph_clipped = clipped;
        let mut rest = src;
        let mut line = base;
        while !rest.is_empty() {
            let Some(len) = self.block_step(rest, line, tokens, &mut last_paragraph_clipped) else {
                break;
            };
            let len = len.max(1).min(rest.len());
            line += newlines(&rest[..len]);
            rest = tail(rest, len);
        }
        self.top = true;
    }

    /// One iteration of the `blockTokens` loop; returns the consumed length.
    fn block_step(
        &mut self,
        src: &str,
        line: u32,
        tokens: &mut Vec<BlockTok>,
        clipped: &mut bool,
    ) -> Option<usize> {
        let tok = |raw: &str, kind| BlockTok {
            raw: raw.to_owned(),
            line,
            kind,
        };

        // extensions (block): FrankMD's mathBlock
        let pre = Prefilter::new(src);
        if let Some(s) = ext::scan_math_block(src) {
            tokens.push(tok(
                &src[..s.raw_len],
                BlockKind::MathBlock { text: s.value },
            ));
            return Some(s.raw_len);
        }
        // space
        if pre.newline {
            let raw = &src[..fast::newline(src)];
            if !raw.is_empty() {
                match tokens.last_mut() {
                    Some(last) if raw.len() == 1 => last.raw.push('\n'),
                    _ => tokens.push(tok(raw, BlockKind::Space)),
                }
                return Some(raw.len());
            }
        }
        // code (indented)
        if let Some(cap) = pre
            .indented
            .then(|| exec(&rules::BLOCK_CODE, src))
            .flatten()
        {
            let raw = group(&cap, 0);
            let text = remove_code_indent(raw);
            let text = text.trim_end_matches('\n').to_owned();
            if let Some(last) = tokens.last_mut()
                && let BlockKind::Paragraph { inl } | BlockKind::Text { inl } = &mut last.kind
            {
                last.raw.push('\n');
                last.raw.push_str(raw);
                inl.text.push('\n');
                inl.text.push_str(&text);
            } else {
                tokens.push(tok(raw, BlockKind::Code { lang: None, text }));
            }
            return Some(raw.len());
        }
        // fences
        if let Some(f) = pre
            .lead_is(&['`', '~'])
            .then(|| fast::fences(src))
            .flatten()
        {
            let raw = &src[..f.raw_len];
            let text = indent_code_compensation(raw, f.body.unwrap_or(""));
            let lang = Some(rules::unescape_punctuation(js_trim(f.info))).filter(|l| !l.is_empty());
            tokens.push(tok(raw, BlockKind::Code { lang, text }));
            return Some(raw.len());
        }
        // heading
        if let Some(cap) = pre
            .lead_is(&['#'])
            .then(|| exec(&rules::HEADING, src))
            .flatten()
        {
            let raw = group(&cap, 0);
            let mut text = js_trim(group(&cap, 2));
            if text.ends_with('#') {
                let trimmed = text.trim_end_matches('#');
                if trimmed.is_empty() || trimmed.ends_with(' ') {
                    text = js_trim(trimmed);
                }
            }
            let depth = u8::try_from(group(&cap, 1).len()).unwrap_or(6);
            tokens.push(tok(
                raw,
                BlockKind::Heading {
                    depth,
                    inl: Inl::new(text),
                },
            ));
            return Some(raw.len());
        }
        // hr
        if let Some(cap) = pre
            .lead_is(&['-', '_', '*'])
            .then(|| exec(&rules::HR, src))
            .flatten()
        {
            let raw = group(&cap, 0).trim_end_matches('\n');
            tokens.push(tok(raw, BlockKind::Hr));
            return Some(raw.len());
        }
        // blockquote
        if let Some(t) = pre
            .lead_is(&['>'])
            .then(|| self.blockquote(src, line))
            .flatten()
        {
            let len = t.raw.len();
            tokens.push(t);
            return Some(len);
        }
        // list
        if let Some(t) = pre.list.then(|| self.list(src, line)).flatten() {
            let len = t.raw.len();
            tokens.push(t);
            return Some(len);
        }
        // html
        if let Some(cap) = pre
            .lead_is(&['<'])
            .then(|| exec(&rules::HTML, src))
            .flatten()
        {
            let raw = group(&cap, 0);
            tokens.push(tok(
                raw,
                BlockKind::Html {
                    text: raw.to_owned(),
                },
            ));
            return Some(raw.len());
        }
        // def
        if let Some(cap) = pre
            .lead_is(&['['])
            .then(|| exec(&rules::DEF, src))
            .flatten()
        {
            let raw = group(&cap, 0);
            if let Some(last) = tokens.last_mut()
                && let BlockKind::Paragraph { inl } | BlockKind::Text { inl } = &mut last.kind
            {
                last.raw.push('\n');
                last.raw.push_str(raw);
                inl.text.push('\n');
                inl.text.push_str(raw);
            } else {
                let tag = collapse_space(&group(&cap, 1).to_lowercase());
                let href = cap.get(2).map_or(String::new(), |m| {
                    let h = m.as_str();
                    let h = h
                        .strip_prefix('<')
                        .and_then(|x| x.strip_suffix('>'))
                        .unwrap_or(h);
                    rules::unescape_punctuation(h)
                });
                let title = cap.get(3).map(|m| {
                    let t = m.as_str();
                    rules::unescape_punctuation(&t[1..t.len() - 1])
                });
                self.links.entry(tag).or_insert(LinkDef { href, title });
            }
            return Some(raw.len());
        }
        // table (gfm)
        if let Some(t) = pre.table.then(|| table(src, line)).flatten() {
            let len = t.raw.len();
            tokens.push(t);
            return Some(len);
        }
        // lheading
        if let Some(cap) = pre.setext.then(|| exec(&rules::LHEADING, src)).flatten() {
            let raw = group(&cap, 0);
            let depth = if group(&cap, 2).starts_with('=') {
                1
            } else {
                2
            };
            tokens.push(tok(
                raw,
                BlockKind::Heading {
                    depth,
                    inl: Inl::new(group(&cap, 1)),
                },
            ));
            return Some(raw.len());
        }
        // top-level paragraph, clipped at the next block extension start
        let mut cut = src;
        if let Some(first) = src.chars().next() {
            let skip = first.len_utf8();
            // A paragraph never crosses an empty line, so a `$$` beyond the
            // first one cannot clip it (and searching the whole remaining
            // document for every block would be quadratic).
            let window = src.find("\n\n").map_or(src.len(), |i| i + 2).max(skip);
            if let Some(ix) = ext::start(ext::Extension::MathBlock, &src[skip..window]) {
                cut = &src[..skip + ix];
            }
        }
        if self.top
            && let Some(len) = fast::paragraph(cut)
        {
            let raw = &cut[..len];
            let text = raw;
            match tokens.last_mut() {
                Some(BlockTok {
                    raw: last_raw,
                    kind: BlockKind::Paragraph { inl },
                    ..
                }) if *clipped => {
                    last_raw.push('\n');
                    last_raw.push_str(raw);
                    inl.text.push('\n');
                    inl.text.push_str(text);
                }
                _ => tokens.push(tok(
                    raw,
                    BlockKind::Paragraph {
                        inl: Inl::new(text),
                    },
                )),
            }
            *clipped = cut.len() != src.len();
            return Some(raw.len());
        }
        // text
        // text: `^[^\n]+`
        let raw = first_line(src);
        if !raw.is_empty() {
            match tokens.last_mut() {
                Some(BlockTok {
                    raw: last_raw,
                    kind: BlockKind::Text { inl },
                    ..
                }) => {
                    last_raw.push('\n');
                    last_raw.push_str(raw);
                    inl.text.push('\n');
                    inl.text.push_str(raw);
                }
                _ => tokens.push(tok(raw, BlockKind::Text { inl: Inl::new(raw) })),
            }
            return Some(raw.len());
        }
        // marked throws "Infinite loop on byte"; stop instead.
        None
    }

    /// `Tokenizer.blockquote`.
    fn blockquote(&mut self, src: &str, line: u32) -> Option<BlockTok> {
        let cap = exec(&rules::BLOCKQUOTE, src)?;
        let whole = group(&cap, 0).trim_end_matches('\n');
        let mut lines: Vec<String> = whole.split('\n').map(str::to_owned).collect();
        let mut raw = String::new();
        let mut text = String::new();
        let mut tokens: Vec<BlockTok> = Vec::new();
        while !lines.is_empty() {
            let mut in_blockquote = false;
            let mut current: Vec<String> = Vec::new();
            let mut i = 0;
            while i < lines.len() {
                if exec(&rules::BLOCKQUOTE_START, &lines[i]).is_some() {
                    current.push(lines[i].clone());
                    in_blockquote = true;
                } else if !in_blockquote {
                    current.push(lines[i].clone());
                } else {
                    break;
                }
                i += 1;
            }
            lines.drain(..i);
            let current_raw = current.join("\n");
            let current_text = blockquote_text(&current_raw);
            let segment_line = if raw.is_empty() {
                line
            } else {
                line + newlines(&raw) + 1
            };
            if raw.is_empty() {
                raw = current_raw.clone();
            } else {
                raw = format!("{raw}\n{current_raw}");
            }
            if text.is_empty() {
                text = current_text.clone();
            } else {
                text = format!("{text}\n{current_text}");
            }
            let top = self.top;
            self.top = true;
            self.block_tokens(&current_text, &mut tokens, true, segment_line);
            self.top = top;
            if lines.is_empty() {
                break;
            }
            let Some(last) = tokens.last() else { continue };
            match &last.kind {
                BlockKind::Code { .. } => break,
                BlockKind::Blockquote { text: old_text, .. } => {
                    let (old_raw, old_text, old_line) =
                        (last.raw.clone(), old_text.clone(), last.line);
                    let new_src = format!("{old_raw}\n{}", lines.join("\n"));
                    let Some(new_tok) = self.blockquote(&new_src, old_line) else {
                        break;
                    };
                    let BlockKind::Blockquote { text: new_text, .. } = &new_tok.kind else {
                        break;
                    };
                    raw = format!(
                        "{}{}",
                        &raw[..raw.len().saturating_sub(old_raw.len())],
                        new_tok.raw
                    );
                    text = format!(
                        "{}{}",
                        &text[..text.len().saturating_sub(old_text.len())],
                        new_text
                    );
                    let n = tokens.len();
                    tokens[n - 1] = new_tok;
                    break;
                }
                BlockKind::List { .. } => {
                    let (old_raw, old_line) = (last.raw.clone(), last.line);
                    let new_src = format!("{old_raw}\n{}", lines.join("\n"));
                    let Some(new_tok) = self.list(&new_src, old_line) else {
                        break;
                    };
                    raw = format!(
                        "{}{}",
                        &raw[..raw.len().saturating_sub(old_raw.len())],
                        new_tok.raw
                    );
                    text = format!(
                        "{}{}",
                        &text[..text.len().saturating_sub(old_raw.len())],
                        new_tok.raw
                    );
                    lines = tail(&new_src, new_tok.raw.len())
                        .split('\n')
                        .map(str::to_owned)
                        .collect();
                    let n = tokens.len();
                    tokens[n - 1] = new_tok;
                    continue;
                }
                _ => {}
            }
        }
        Some(BlockTok {
            raw,
            line,
            kind: BlockKind::Blockquote {
                text,
                children: tokens,
            },
        })
    }

    /// `Tokenizer.list`.
    fn list(&mut self, src: &str, line: u32) -> Option<BlockTok> {
        let cap = exec(&rules::LIST, src)?;
        let bull = js_trim(group(&cap, 1));
        let ordered = bull.chars().count() > 1;
        let (start, bull_re) = if ordered {
            let num = &bull[..bull.len() - 1];
            let delim = &bull[bull.len() - 1..];
            (
                num.parse::<u64>().unwrap_or(0),
                format!(r"\d{{1,9}}\{delim}"),
            )
        } else {
            (1, format!(r"\{bull}"))
        };
        let item_re = rules::list_item_regex(&bull_re);
        let mut loose = false;
        let mut ends_with_blank_line = false;
        let mut items: Vec<ListItemTok> = Vec::new();
        let mut list_raw = String::new();
        // Lines consumed by earlier items (kept incrementally: recounting
        // `list_raw` per item is quadratic on long lists).
        let mut item_line = line;
        let mut rest = src;

        while !rest.is_empty() {
            let Some(cap) = exec(&item_re, rest) else {
                break;
            };
            if pre_hr(rest) && exec(&rules::HR, rest).is_some() {
                break;
            }
            let mut raw = group(&cap, 0).to_owned();
            let cap1 = group(&cap, 1);
            let cap2 = group(&cap, 2);
            rest = tail(rest, raw.len());
            let mut cur_line = expand_leading_tabs(first_line(cap2));
            let mut next_line = first_line(rest);
            let mut blank_line = js_trim(&cur_line).is_empty();
            let mut item_contents = String::new();
            let indent;
            if blank_line {
                indent = cap1.len() + 1;
            } else {
                let mut ind = first_non_space(cap2).unwrap_or(0);
                if ind > 4 {
                    ind = 1;
                }
                item_contents = tail(&cur_line, ind).to_owned();
                indent = ind + cap1.len();
            }
            let mut end_early = false;
            if blank_line && exec(&rules::BLANK_LINE, next_line).is_some() {
                raw.push_str(next_line);
                raw.push('\n');
                rest = tail(rest, next_line.len() + 1);
                end_early = true;
            }
            if !end_early {
                let br = rules::ItemBreakers::get(indent);
                while !rest.is_empty() {
                    let raw_line = first_line(rest);
                    next_line = raw_line;
                    let without_tabs = next_line.replace('\t', "    ");
                    if br.breaks(next_line) {
                        break;
                    }
                    if first_non_space(&without_tabs).is_some_and(|i| i >= indent)
                        || js_trim(next_line).is_empty()
                    {
                        item_contents.push('\n');
                        item_contents.push_str(tail(&without_tabs, indent));
                    } else {
                        if blank_line {
                            break;
                        }
                        if first_non_space(&cur_line.replace('\t', "    ")).is_some_and(|i| i >= 4)
                            || exec(&br.fences, &cur_line).is_some()
                            || exec(&br.heading, &cur_line).is_some()
                            || exec(&br.hr, &cur_line).is_some()
                        {
                            break;
                        }
                        item_contents.push('\n');
                        item_contents.push_str(next_line);
                    }
                    if !blank_line && js_trim(next_line).is_empty() {
                        blank_line = true;
                    }
                    raw.push_str(raw_line);
                    raw.push('\n');
                    rest = tail(rest, raw_line.len() + 1);
                    cur_line = tail(&without_tabs, indent).to_owned();
                }
            }
            if !loose {
                if ends_with_blank_line {
                    loose = true;
                } else if ends_with_double_blank_line(&raw) {
                    ends_with_blank_line = true;
                }
            }
            let mut task = None;
            if let Some(t) = item_contents
                .starts_with('[')
                .then(|| exec(&rules::LIST_IS_TASK, &item_contents))
                .flatten()
            {
                task = Some(group(&t, 0) != "[ ] ");
                item_contents = rules::LIST_REPLACE_TASK
                    .replace(&item_contents, "")
                    .into_owned();
            }
            items.push(ListItemTok {
                line: item_line,
                raw: raw.clone(),
                task,
                text: item_contents,
                children: Vec::new(),
            });
            item_line += newlines(&raw);
            list_raw.push_str(&raw);
        }

        let last = items.last_mut()?;
        last.raw = js_trim_end(&last.raw).to_owned();
        last.text = js_trim_end(&last.text).to_owned();
        let list_raw = js_trim_end(&list_raw).to_owned();

        for item in &mut items {
            self.top = false;
            let mut children = Vec::new();
            let text = item.text.clone();
            self.block_tokens(&text, &mut children, false, item.line);
            if !loose {
                loose = children.iter().any(|t| {
                    matches!(t.kind, BlockKind::Space) && search(&rules::ANY_LINE, &t.raw).is_some()
                });
            }
            item.children = children;
        }
        Some(BlockTok {
            raw: list_raw,
            line,
            kind: BlockKind::List {
                ordered,
                start,
                loose,
                items,
            },
        })
    }
}

/// Cheap necessary conditions for the block rules, so the regexes (most of
/// a render's time) only run where they can match. Each flag is a superset
/// of its rule: when it is false the rule cannot match.
struct Prefilter {
    /// First char after at most 3 leading spaces (`None` with 4+ spaces).
    lead: Option<char>,
    newline: bool,
    indented: bool,
    list: bool,
    table: bool,
    setext: bool,
}

impl Prefilter {
    fn new(src: &str) -> Self {
        let spaces = src.len() - src.trim_start_matches(' ').len();
        let lead = if spaces <= 3 {
            src[spaces..].chars().next()
        } else {
            None
        };
        let first = src.as_bytes().first().copied();
        // A setext heading's text cannot cross an empty line.
        let region = src.find("\n\n").map_or(src, |i| &src[..i + 1]);
        let setext = region.split('\n').skip(1).any(|l| {
            let l = l.trim_start_matches(' ');
            l.starts_with('=') || l.starts_with('-')
        });
        Self {
            lead,
            newline: matches!(first, Some(b' ' | b'\t' | b'\n')),
            indented: matches!(first, Some(b' ' | b'\t')),
            list: lead.is_some_and(|c| matches!(c, '*' | '+' | '-') || c.is_ascii_digit()),
            // The delimiter row is all `|`, `:`, `-` and spaces, with a `-`.
            table: fast::table_second_line(src),
            setext,
        }
    }

    fn lead_is(&self, chars: &[char]) -> bool {
        self.lead.is_some_and(|c| chars.contains(&c))
    }
}

/// Necessary condition for `rules::HR`: `-`, `_` or `*` after at most 3 spaces.
fn pre_hr(src: &str) -> bool {
    let t = src.trim_start_matches(' ');
    src.len() - t.len() <= 3 && t.starts_with(['-', '_', '*'])
}

/// `/\n[ \t]*\n[ \t]*$/.test(raw)`.
fn ends_with_double_blank_line(raw: &str) -> bool {
    let ws: &[char] = &[' ', '\t'];
    raw.trim_end_matches(ws)
        .strip_suffix('\n')
        .is_some_and(|r| r.trim_end_matches(ws).ends_with('\n'))
}

/// `Tokenizer.table`.
fn table(src: &str, line: u32) -> Option<BlockTok> {
    let cap = fast::table(src)?;
    // The delimiter row needs a `|` or `:` (otherwise it is a setext heading).
    search(&rules::TABLE_DELIMITER, cap.align)?;
    let headers = split_cells(cap.header, None);
    let align_src = cap.align;
    let align_src = align_src.strip_prefix('|').unwrap_or(align_src);
    let align_src = strip_trailing_pipe(align_src);
    let aligns: Vec<&str> = align_src.split('|').collect();
    let rows_src = cap.rows.unwrap_or("");
    let rows: Vec<String> = if !js_trim(rows_src).is_empty() {
        rules::TABLE_ROW_BLANK_LINE
            .replace(rows_src, "")
            .split('\n')
            .map(str::to_owned)
            .collect()
    } else {
        Vec::new()
    };
    if headers.len() != aligns.len() {
        return None;
    }
    let align: Vec<Align> = aligns
        .iter()
        .map(|a| {
            if exec(&rules::TABLE_ALIGN_RIGHT, a).is_some() {
                Align::Right
            } else if exec(&rules::TABLE_ALIGN_CENTER, a).is_some() {
                Align::Center
            } else if exec(&rules::TABLE_ALIGN_LEFT, a).is_some() {
                Align::Left
            } else {
                Align::None
            }
        })
        .collect();
    let header: Vec<Inl> = headers.into_iter().map(Inl::new).collect();
    let rows = rows
        .iter()
        .map(|r| {
            split_cells(r, Some(header.len()))
                .into_iter()
                .map(Inl::new)
                .collect()
        })
        .collect();
    Some(BlockTok {
        raw: src[..cap.raw_len].to_owned(),
        line,
        kind: BlockKind::Table {
            align,
            header,
            rows,
        },
    })
}

/// `other.tableAlignChars` = `/^\||\| *$/g` (the leading `|` is handled by the caller).
fn strip_trailing_pipe(s: &str) -> &str {
    let trimmed = s.trim_end_matches(' ');
    trimmed.strip_suffix('|').unwrap_or(s)
}

/// marked's `splitCells(tableRow, count)`.
fn split_cells(row: &str, count: Option<usize>) -> Vec<String> {
    // Put a space before every unescaped pipe, then split on " |".
    let bytes = row.as_bytes();
    let mut spaced = String::with_capacity(row.len() + 8);
    for (i, ch) in row.char_indices() {
        if ch == '|' {
            let mut escaped = false;
            let mut j = i;
            while j > 0 && bytes[j - 1] == b'\\' {
                escaped = !escaped;
                j -= 1;
            }
            spaced.push_str(if escaped { "|" } else { " |" });
        } else {
            spaced.push(ch);
        }
    }
    let mut cells: Vec<String> = spaced.split(" |").map(str::to_owned).collect();
    if cells.first().is_some_and(|c| js_trim(c).is_empty()) {
        cells.remove(0);
    }
    if cells.last().is_some_and(|c| js_trim(c).is_empty()) {
        cells.pop();
    }
    if let Some(count) = count {
        if cells.len() > count {
            cells.truncate(count);
        } else {
            while cells.len() < count {
                cells.push(String::new());
            }
        }
    }
    cells
        .iter()
        .map(|c| js_trim(c).replace("\\|", "|"))
        .collect()
}

/// Blockquote line text: setext continuations get 4 spaces, `>` prefixes go.
fn blockquote_text(raw: &str) -> String {
    let protected = rules::BLOCKQUOTE_SETEXT.replace_all(raw, "\n    $1");
    protected
        .split('\n')
        .map(|l| match rules::exec(&rules::BLOCKQUOTE_LINE_PREFIX, l) {
            Some(m) => tail(l, group(&m, 0).len()).to_owned(),
            None => l.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `indentCodeCompensation(raw, text)`.
fn indent_code_compensation(raw: &str, text: &str) -> String {
    let Some(cap) = exec(&rules::INDENT_CODE_COMPENSATION, raw) else {
        return text.to_owned();
    };
    let indent = group(&cap, 1).len();
    text.split('\n')
        .map(|node| {
            let lead = node.len() - node.trim_start_matches(super::is_js_space).len();
            if lead > 0 && lead >= indent {
                tail(node, indent)
            } else {
                node
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `raw.replace(/^(?: {1,4}| {0,3}\t)/gm, '')`.
fn remove_code_indent(raw: &str) -> String {
    raw.split('\n')
        .map(|l| match exec(&rules::CODE_REMOVE_INDENT_LINE, l) {
            Some(m) => tail(l, group(&m, 0).len()),
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `line.replace(/^\t+/, t => ' '.repeat(3 * t.length))`.
fn expand_leading_tabs(line: &str) -> String {
    let tabs = line.len() - line.trim_start_matches('\t').len();
    format!("{}{}", " ".repeat(3 * tabs), &line[tabs..])
}

/// `s.search(/[^ ]/)`.
fn first_non_space(s: &str) -> Option<usize> {
    s.bytes().position(|b| b != b' ')
}

/// `s.replace(/\s+/g, ' ')`.
fn collapse_space(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    for c in s.chars() {
        if super::is_js_space(c) {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}
