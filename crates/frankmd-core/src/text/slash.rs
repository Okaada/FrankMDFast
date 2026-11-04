//! Slash commands of the Markdown editor (`/` menu): trigger detection and
//! the text edits behind the heading and block commands.
//!
//! Port of `app/javascript/lib/codemirror_slash_commands.js`. The original
//! asks CodeMirror's Lezer Markdown syntax tree where the slash sits
//! (`Paragraph` / `ATXHeading1-6`, `ListItem` + `ListMark`, `BulletList` /
//! `OrderedList`, `Blockquote`, `FencedCode` / `InlineCode`, `URL`). This
//! crate has no Markdown parser, so a small line-based block analyzer
//! reproduces just the node ranges those commands use.
//!
//! # Approximation (honest list of differences from Lezer)
//!
//! * Containers: one list stack (nesting by indentation against the item's
//!   content column) or one top-level blockquote. A list inside a blockquote
//!   or a quote inside a list item is not modelled (the inner marker is plain
//!   text of the outer block).
//! * No lazy continuation: a paragraph line continues a list item only when
//!   indented to its content column, and a blockquote only while lines start
//!   with `>`.
//! * Setext headings (`text` + `---` / `===`), indented code, fenced code and
//!   table rows (`|…`) are recognised and are not editable blocks. HTML
//!   blocks, link reference definitions and GFM extensions are not.
//! * Ordered markers interrupt a paragraph whatever their number.
//! * Inline code is a backtick span of equal-length runs inside one
//!   paragraph; a link destination is a slash right after `](`.
//! * `\p{L}\p{N}\p{M}` is approximated with `char::is_alphabetic`,
//!   `is_numeric` and the common combining-mark ranges.

use std::ops::Range;

use crate::text::edit::Edit;

/// A command of the slash menu, in the original menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlashCommand {
    /// `# ` heading.
    Heading1,
    /// `## ` heading.
    Heading2,
    /// `### ` heading.
    Heading3,
    /// `- ` list.
    BulletedList,
    /// `1. ` list.
    NumberedList,
    /// `- [ ] ` task list.
    TodoList,
    /// `> ` blockquote.
    Quote,
    /// Fenced code block.
    CodeBlock,
    /// `$$` display equation.
    Equation,
    /// `---` divider after the block.
    Divider,
    /// Opens the table editor.
    Table,
    /// Opens the image picker.
    Image,
    /// Opens the video dialog.
    Video,
    /// Opens the emoji picker.
    Emoji,
}

impl SlashCommand {
    /// All commands in menu order.
    pub const ALL: [SlashCommand; 14] = [
        SlashCommand::Heading1,
        SlashCommand::Heading2,
        SlashCommand::Heading3,
        SlashCommand::BulletedList,
        SlashCommand::NumberedList,
        SlashCommand::TodoList,
        SlashCommand::Quote,
        SlashCommand::CodeBlock,
        SlashCommand::Equation,
        SlashCommand::Divider,
        SlashCommand::Table,
        SlashCommand::Image,
        SlashCommand::Video,
        SlashCommand::Emoji,
    ];

    /// Stable command id (`name` in the original).
    pub fn id(self) -> &'static str {
        match self {
            Self::Heading1 => "heading-1",
            Self::Heading2 => "heading-2",
            Self::Heading3 => "heading-3",
            Self::BulletedList => "bulleted-list",
            Self::NumberedList => "numbered-list",
            Self::TodoList => "to-do-list",
            Self::Quote => "quote",
            Self::CodeBlock => "code-block",
            Self::Equation => "equation",
            Self::Divider => "divider",
            Self::Table => "table",
            Self::Image => "image",
            Self::Video => "video",
            Self::Emoji => "emoji",
        }
    }

    /// Phosphor icon name.
    pub fn icon(self) -> &'static str {
        match self {
            Self::Heading1 | Self::Heading2 | Self::Heading3 => "hash",
            Self::BulletedList => "list-bullets",
            Self::NumberedList => "list-numbers",
            Self::TodoList => "list-checks",
            Self::Quote => "quotes",
            Self::CodeBlock => "code",
            Self::Equation => "sigma",
            Self::Divider => "minus",
            Self::Table => "table",
            Self::Image => "image",
            Self::Video => "video-camera",
            Self::Emoji => "smiley",
        }
    }

    /// i18n key of the label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Heading1 => "editor.slash_commands.heading_1",
            Self::Heading2 => "editor.slash_commands.heading_2",
            Self::Heading3 => "editor.slash_commands.heading_3",
            Self::BulletedList => "editor.slash_commands.bulleted_list",
            Self::NumberedList => "editor.slash_commands.numbered_list",
            Self::TodoList => "editor.slash_commands.to_do_list",
            Self::Quote => "editor.slash_commands.quote",
            Self::CodeBlock => "editor.slash_commands.code_block",
            Self::Equation => "editor.slash_commands.equation",
            Self::Divider => "editor.slash_commands.divider",
            Self::Table => "editor.slash_commands.table",
            Self::Image => "editor.slash_commands.image",
            Self::Video => "editor.slash_commands.video",
            Self::Emoji => "editor.slash_commands.emoji",
        }
    }

    /// English label used while translations are unavailable.
    pub fn fallback_label(self) -> &'static str {
        match self {
            Self::Heading1 => "Heading 1",
            Self::Heading2 => "Heading 2",
            Self::Heading3 => "Heading 3",
            Self::BulletedList => "Bulleted list",
            Self::NumberedList => "Numbered list",
            Self::TodoList => "To-do list",
            Self::Quote => "Quote",
            Self::CodeBlock => "Code block",
            Self::Equation => "Equation",
            Self::Divider => "Divider",
            Self::Table => "Table",
            Self::Image => "Image",
            Self::Video => "Video",
            Self::Emoji => "Emoji",
        }
    }

    /// Text the fuzzy matcher runs against, given the (localized) label.
    pub fn filter_text(self, label: &str) -> String {
        if let Some(level) = self.heading_level() {
            return format!("{label} heading {level} heading{level} h{level}");
        }
        let extra = match self {
            Self::BulletedList => "bullet bulleted unordered list",
            Self::NumberedList => "numbered ordered list",
            Self::TodoList => "to-do todo task checklist",
            Self::Quote => "quote blockquote",
            Self::CodeBlock => "code block fenced",
            Self::Equation => "equation math formula latex tex",
            Self::Divider => "divider horizontal rule hr",
            Self::Table => "insert table grid",
            Self::Image => "insert image picture photo",
            Self::Video => "insert video embed youtube",
            Self::Emoji => "insert emoji emoticon icon",
            _ => "",
        };
        format!("{label} {extra}")
    }

    /// Whether choosing the command opens a dialog instead of editing text.
    pub fn opens_dialog(self) -> bool {
        matches!(self, Self::Table | Self::Image | Self::Video | Self::Emoji)
    }

    fn heading_level(self) -> Option<usize> {
        match self {
            Self::Heading1 => Some(1),
            Self::Heading2 => Some(2),
            Self::Heading3 => Some(3),
            _ => None,
        }
    }
}

/// An active slash trigger in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashTrigger {
    /// Byte offset of the `/`.
    pub slash: usize,
    /// Byte offset of the caret (end of the query).
    pub caret: usize,
    /// Text between the `/` and the caret.
    pub query: String,
}

/// Detect a slash trigger ending at `caret`: the regex
/// `(?:^|[\s([{])\/([\p{L}\p{N}\p{M} -]*)$` on the caret line before the
/// caret, outside link destinations, code and non-paragraph blocks.
pub fn slash_trigger(text: &str, caret: usize) -> Option<SlashTrigger> {
    let before = text.get(..caret)?;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let slash = line_start + find_trigger_slash(&before[line_start..])?;
    if is_link_destination(text, slash) {
        return None;
    }
    analyze(text).editable_block_at(text, slash)?;
    Some(SlashTrigger {
        slash,
        caret,
        query: text[slash + 1..caret].to_owned(),
    })
}

/// Apply a heading or block command to the paragraph/heading holding the
/// slash. `None` for dialog commands or when no editable block is found.
/// The result is a single edit (one undo step).
pub fn apply_command(text: &str, trigger: &SlashTrigger, command: SlashCommand) -> Option<Edit> {
    if command.opens_dialog()
        || trigger.slash >= trigger.caret
        || !text.is_char_boundary(trigger.slash)
        || !text.is_char_boundary(trigger.caret)
    {
        return None;
    }
    let analysis = analyze(text);
    let block = analysis.editable_block_at(text, trigger.slash)?;
    let ctx = analysis.context(block);
    match command.heading_level() {
        Some(level) => heading_edit(text, block, trigger, level),
        None => block_edit(text, block, &ctx, trigger, command),
    }
}

/// Range and text stored for dialog commands: `slash..caret` and the query
/// including the slash.
pub fn dialog_range(trigger: &SlashTrigger) -> (Range<usize>, String) {
    (trigger.slash..trigger.caret, format!("/{}", trigger.query))
}

// ---------------------------------------------------------------------------
// Trigger helpers
// ---------------------------------------------------------------------------

/// Offset of the trigger slash in `line_before` (the caret line up to the
/// caret). Only the last `/` can qualify because the query excludes `/`.
fn find_trigger_slash(line_before: &str) -> Option<usize> {
    let slash = line_before.rfind('/')?;
    if !line_before[slash + 1..].chars().all(is_query_char) {
        return None;
    }
    match line_before[..slash].chars().next_back() {
        None => Some(slash),
        Some(c) if c.is_whitespace() || matches!(c, '(' | '[' | '{') => Some(slash),
        Some(_) => None,
    }
}

fn is_query_char(c: char) -> bool {
    c == ' ' || c == '-' || c.is_alphabetic() || c.is_numeric() || is_combining_mark(c)
}

fn is_combining_mark(c: char) -> bool {
    matches!(c as u32,
        0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x064B..=0x065F
        | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE00..=0xFE0F
        | 0xFE20..=0xFE2F)
}

/// The slash starts a link destination: `](` (optional blanks) before it.
fn is_link_destination(text: &str, slash: usize) -> bool {
    text[..slash].trim_end_matches([' ', '\t']).ends_with("](")
}

/// `pos` (relative to `s`) lies inside a backtick code span.
fn in_code_span(s: &str, pos: usize) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'`' => {
                let open = backtick_run(b, i);
                match find_closing_run(b, i + open, open) {
                    Some(end) => {
                        if (i..end).contains(&pos) {
                            return true;
                        }
                        i = end;
                    }
                    None => i += open,
                }
            }
            _ => i += 1,
        }
    }
    false
}

fn backtick_run(b: &[u8], at: usize) -> usize {
    b[at..].iter().take_while(|&&c| c == b'`').count()
}

/// End offset of the next backtick run of exactly `len` at or after `from`.
fn find_closing_run(b: &[u8], from: usize, len: usize) -> Option<usize> {
    let mut i = from;
    while i < b.len() {
        if b[i] == b'`' {
            let run = backtick_run(b, i);
            if run == len {
                return Some(i + run);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Block analyzer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Paragraph,
    Heading,
    /// Not editable (setext heading).
    Other,
}

/// A list item (`ListItem` + `ListMark`).
#[derive(Debug)]
struct Item {
    mark: Range<usize>,
    ordered: bool,
    content_col: usize,
    end: usize,
}

#[derive(Debug)]
struct Block {
    kind: BlockKind,
    from: usize,
    to: usize,
    item: Option<usize>,
    quote: Option<usize>,
    /// Kind of the outermost list (what `getBlockContext` ends up with).
    outer_ordered: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
enum Container {
    None,
    Quote,
    Item(usize),
}

#[derive(Debug, Clone, Copy)]
struct Fence {
    ch: u8,
    len: usize,
    container: Container,
}

#[derive(Debug, Default)]
struct Analysis {
    blocks: Vec<Block>,
    items: Vec<Item>,
    /// End offset of each blockquote.
    quotes: Vec<usize>,
}

/// List/quote context of a block (the original `getBlockContext`).
struct BlockCtx {
    item_mark: Option<Range<usize>>,
    item_end: Option<usize>,
    list_ordered: Option<bool>,
    quote_end: Option<usize>,
}

impl Analysis {
    fn editable_block_at(&self, text: &str, pos: usize) -> Option<&Block> {
        let block = self.blocks.iter().find(|b| {
            matches!(b.kind, BlockKind::Paragraph | BlockKind::Heading)
                && (b.from..b.to).contains(&pos)
        })?;
        (!in_code_span(&text[block.from..block.to], pos - block.from)).then_some(block)
    }

    fn context(&self, block: &Block) -> BlockCtx {
        let item = block.item.map(|i| &self.items[i]);
        BlockCtx {
            item_mark: item.map(|i| i.mark.clone()),
            item_end: item.map(|i| i.end),
            list_ordered: block.outer_ordered,
            quote_end: block.quote.map(|q| self.quotes[q]),
        }
    }
}

fn analyze(text: &str) -> Analysis {
    let mut a = Analyzer {
        text,
        out: Analysis::default(),
        stack: Vec::new(),
        quote: None,
        para: None,
        fence: None,
    };
    let mut start = 0;
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        a.feed(start, start + line.len());
        start += raw.len() + 1;
    }
    a.out
}

struct Analyzer<'a> {
    text: &'a str,
    out: Analysis,
    /// Open list items, outermost first.
    stack: Vec<usize>,
    quote: Option<usize>,
    /// Index of the open paragraph block.
    para: Option<usize>,
    fence: Option<Fence>,
}

#[derive(Clone, Copy)]
struct Ctx {
    item: Option<usize>,
    quote: Option<usize>,
}

impl Analyzer<'_> {
    fn feed(&mut self, ls: usize, le: usize) {
        let line = &self.text[ls..le];
        if self.continue_fence(le, line) {
            return;
        }
        if line.trim().is_empty() {
            self.para = None;
            self.quote = None;
            return;
        }
        let (indent, ws) = leading_indent(line);
        while let Some(&top) = self.stack.last() {
            if indent < self.out.items[top].content_col {
                self.stack.pop();
            } else {
                break;
            }
        }
        for &i in &self.stack {
            self.out.items[i].end = le;
        }
        if self.stack.is_empty() && indent <= 3 && line[ws..].starts_with('>') {
            self.quote_line(ls, le, ws);
        } else {
            self.quote = None;
            self.plain_line(ls, le, indent, ws);
        }
    }

    /// Consume a line inside an open fence; `false` when the fence ended.
    fn continue_fence(&mut self, le: usize, line: &str) -> bool {
        let Some(f) = self.fence else { return false };
        let content = match f.container {
            Container::None => line,
            Container::Quote => {
                let (indent, ws) = leading_indent(line);
                if indent > 3 || !line[ws..].starts_with('>') {
                    self.fence = None;
                    return false;
                }
                if let Some(q) = self.quote {
                    self.out.quotes[q] = le;
                }
                strip_quote_marker(&line[ws..])
            }
            Container::Item(col) => {
                if line.trim().is_empty() {
                    return true;
                }
                if leading_indent(line).0 < col {
                    self.fence = None;
                    return false;
                }
                for &i in &self.stack {
                    self.out.items[i].end = le;
                }
                line
            }
        };
        let t = content.trim_start_matches([' ', '\t']);
        let run = t.bytes().take_while(|&c| c == f.ch).count();
        if run >= f.len && t[run..].trim().is_empty() {
            self.fence = None;
        }
        true
    }

    fn quote_line(&mut self, ls: usize, le: usize, ws: usize) {
        let q = match self.quote {
            Some(q) => q,
            None => {
                self.out.quotes.push(le);
                self.quote = Some(self.out.quotes.len() - 1);
                self.out.quotes.len() - 1
            }
        };
        self.out.quotes[q] = le;
        let after = &self.text[ls + ws..le];
        let content = strip_quote_marker(after);
        let content_offset = ls + ws + (after.len() - content.len());
        let (rel, lead) = leading_indent(content);
        let ctx = Ctx {
            item: None,
            quote: Some(q),
        };
        self.handle_content(&content[lead..], content_offset + lead, le, rel, ctx);
    }

    fn plain_line(&mut self, ls: usize, le: usize, indent: usize, ws: usize) {
        let item = self.stack.last().copied();
        let ctx = Ctx { item, quote: None };
        let base = item.map_or(0, |i| self.out.items[i].content_col);
        let rel = indent.saturating_sub(base);
        let t = &self.text[ls + ws..le];
        let blocked = is_hr(t) || (self.para_matches(ctx) && is_setext_underline(t));
        if rel < 4
            && !blocked
            && let Some(m) = list_marker(&self.text[ls..le])
        {
            self.start_item(ls, le, indent, &m);
            return;
        }
        self.handle_content(t, ls + ws, le, rel, ctx);
    }

    fn start_item(&mut self, ls: usize, le: usize, indent: usize, m: &Marker) {
        self.para = None;
        let content = &self.text[ls + m.content_offset..le];
        let blank = content.trim().is_empty();
        let content_col = if blank || m.space_cols >= 5 {
            indent + m.len + 1
        } else {
            indent + m.len + m.space_cols
        };
        let mark_start = ls + m.ws;
        self.out.items.push(Item {
            mark: mark_start..mark_start + m.len,
            ordered: m.ordered,
            content_col,
            end: le,
        });
        let idx = self.out.items.len() - 1;
        self.stack.push(idx);
        if blank {
            return;
        }
        let rel = if m.space_cols >= 5 {
            m.space_cols - 1
        } else {
            0
        };
        let ctx = Ctx {
            item: Some(idx),
            quote: None,
        };
        self.handle_content(content, ls + m.content_offset, le, rel, ctx);
    }

    fn para_matches(&self, ctx: Ctx) -> bool {
        self.para.is_some_and(|p| {
            let b = &self.out.blocks[p];
            b.item == ctx.item && b.quote == ctx.quote
        })
    }

    /// Classify the content of a line (after container prefixes) and update
    /// the open paragraph. `t` starts at its first non-blank char, `start` is
    /// its offset, `rel` its indentation relative to the container.
    fn handle_content(&mut self, t: &str, start: usize, le: usize, rel: usize, ctx: Ctx) {
        let continues = self.para_matches(ctx);
        if t.trim().is_empty() {
            self.para = None;
        } else if rel >= 4 {
            if continues {
                self.extend_para(le);
            } else {
                self.para = None; // indented code
            }
        } else if continues && is_setext_underline(t) {
            if let Some(p) = self.para.take() {
                self.out.blocks[p].kind = BlockKind::Other;
            }
        } else if let Some((ch, len)) = fence_open(t) {
            self.para = None;
            self.fence = Some(Fence {
                ch,
                len,
                container: self.container_of(ctx),
            });
        } else if is_hr(t) || t.starts_with('|') {
            self.para = None;
        } else if is_atx_heading(t) {
            self.para = None;
            self.push_block(BlockKind::Heading, start, le, ctx);
        } else if continues {
            self.extend_para(le);
        } else {
            self.para = Some(self.push_block(BlockKind::Paragraph, start, le, ctx));
        }
    }

    fn extend_para(&mut self, le: usize) {
        if let Some(p) = self.para {
            self.out.blocks[p].to = le;
        }
    }

    fn container_of(&self, ctx: Ctx) -> Container {
        match (ctx.quote, ctx.item) {
            (Some(_), _) => Container::Quote,
            (None, Some(i)) => Container::Item(self.out.items[i].content_col),
            (None, None) => Container::None,
        }
    }

    fn push_block(&mut self, kind: BlockKind, from: usize, to: usize, ctx: Ctx) -> usize {
        let outer_ordered = self.stack.first().map(|&i| self.out.items[i].ordered);
        self.out.blocks.push(Block {
            kind,
            from,
            to,
            item: ctx.item,
            quote: ctx.quote,
            outer_ordered,
        });
        self.out.blocks.len() - 1
    }
}

struct Marker {
    ws: usize,
    len: usize,
    ordered: bool,
    /// Offset in the line of the first non-blank char after the marker.
    content_offset: usize,
    space_cols: usize,
}

/// `^[ \t]*([-*+]|\d{1,9}[.)])[ \t]+`
fn list_marker(line: &str) -> Option<Marker> {
    let ws = line.len() - line.trim_start_matches([' ', '\t']).len();
    let rest = &line.as_bytes()[ws..];
    let (len, ordered) = match *rest.first()? {
        b'-' | b'*' | b'+' => (1, false),
        b'0'..=b'9' => {
            let digits = rest.iter().take_while(|c| c.is_ascii_digit()).count();
            if digits > 9 || !matches!(rest.get(digits), Some(b'.' | b')')) {
                return None;
            }
            (digits + 1, true)
        }
        _ => return None,
    };
    let spaces = &line[ws + len..];
    let space_len = spaces.len() - spaces.trim_start_matches([' ', '\t']).len();
    if space_len == 0 {
        return None;
    }
    Some(Marker {
        ws,
        len,
        ordered,
        content_offset: ws + len + space_len,
        space_cols: columns(&spaces[..space_len]),
    })
}

fn columns(ws: &str) -> usize {
    ws.bytes().map(|c| if c == b'\t' { 4 } else { 1 }).sum()
}

/// Indentation in columns (tab = 4) and in bytes.
fn leading_indent(line: &str) -> (usize, usize) {
    let ws = line.len() - line.trim_start_matches([' ', '\t']).len();
    (columns(&line[..ws]), ws)
}

/// Text after the `>` of a quote line (which `s` starts with) and one
/// optional blank.
fn strip_quote_marker(s: &str) -> &str {
    let rest = &s[1..];
    rest.strip_prefix([' ', '\t']).unwrap_or(rest)
}

fn fence_open(t: &str) -> Option<(u8, usize)> {
    let ch = *t.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let run = t.bytes().take_while(|&c| c == ch).count();
    if run < 3 || (ch == b'`' && t[run..].contains('`')) {
        return None;
    }
    Some((ch, run))
}

fn is_hr(t: &str) -> bool {
    let mut chars = t.chars().filter(|c| !matches!(c, ' ' | '\t'));
    let Some(first) = chars.next() else {
        return false;
    };
    matches!(first, '-' | '*' | '_') && 1 + chars.clone().count() >= 3 && chars.all(|c| c == first)
}

fn is_setext_underline(t: &str) -> bool {
    let t = t.trim_end_matches([' ', '\t']);
    !t.is_empty() && (t.bytes().all(|c| c == b'=') || t.bytes().all(|c| c == b'-'))
}

fn is_atx_heading(t: &str) -> bool {
    let n = t.bytes().take_while(|&c| c == b'#').count();
    (1..=6).contains(&n) && matches!(t.as_bytes().get(n), None | Some(b' ' | b'\t'))
}

// ---------------------------------------------------------------------------
// Edits
// ---------------------------------------------------------------------------

fn is_blank_ws(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// `removeSlashQuery`: drop `[slash_off, query_end)` from `original`,
/// tidying the whitespace at the join. Returns the text and the cursor offset.
fn remove_slash_query(
    original: &str,
    slash_off: usize,
    query_end: usize,
) -> Option<(String, usize)> {
    if query_end < slash_off || query_end > original.len() {
        return None;
    }
    let mut before = &original[..slash_off];
    let mut after = &original[query_end..];
    let before_ws = before.ends_with(is_blank_ws);
    let after_ws = after.starts_with(is_blank_ws);
    if (before_ws && after_ws) || (before.is_empty() && after_ws) {
        after = &after[1..];
    } else if (after.trim().is_empty() || after.starts_with('\n')) && before_ws {
        before = before.trim_end_matches(is_blank_ws);
    }
    Some((format!("{before}{after}"), before.len()))
}

/// Result of `stripHeadingMarker`.
struct Stripped<'a> {
    leading: &'a str,
    body: &'a str,
    cursor: usize,
}

fn strip_heading_marker(text: &str, cursor: usize) -> Stripped<'_> {
    let lead_len = text.len() - text.trim_start_matches(is_blank_ws).len();
    let marker = heading_marker_len(&text[lead_len..]);
    let body = &text[lead_len + marker..];
    let body_cursor = cursor.saturating_sub(lead_len + marker);
    Stripped {
        leading: &text[..lead_len],
        body,
        cursor: body_cursor.min(body.len()),
    }
}

/// Length of `^#{1,6}(?:[ \t]+|$)` at the start of `s` (0 = no marker).
fn heading_marker_len(s: &str) -> usize {
    let n = s.bytes().take_while(|&c| c == b'#').count();
    if !(1..=6).contains(&n) {
        return 0;
    }
    let rest = &s[n..];
    if rest.is_empty() || rest.starts_with(is_blank_ws) {
        s.len() - rest.trim_start_matches(is_blank_ws).len()
    } else {
        0
    }
}

/// `text.replace(/[ \t]*\n[ \t]*/g, " ")`
fn normalize_soft_breaks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(nl) = rest.find('\n') {
        out.push_str(rest[..nl].trim_end_matches(is_blank_ws));
        out.push(' ');
        rest = rest[nl + 1..].trim_start_matches(is_blank_ws);
    }
    out.push_str(rest);
    out
}

fn heading_edit(text: &str, block: &Block, trigger: &SlashTrigger, level: usize) -> Option<Edit> {
    let (clean, cursor) = remove_slash_query(
        &text[block.from..block.to],
        trigger.slash.checked_sub(block.from)?,
        trigger.caret.checked_sub(block.from)?,
    )?;
    let s = strip_heading_marker(&clean, cursor);
    let cursor_in_body = normalize_soft_breaks(&s.body[..s.cursor]).len();
    let body = normalize_soft_breaks(s.body);
    let marker = format!("{} ", "#".repeat(level));
    let caret = block.from + s.leading.len() + marker.len() + cursor_in_body.min(body.len());
    Some(Edit {
        range: block.from..block.to,
        text: format!("{}{marker}{body}", s.leading),
        selection: caret..caret,
    })
}

fn prefix_lines(text: &str, first: &str, cont: &str) -> String {
    format!("{first}{}", text.replace('\n', &format!("\n{cont}")))
}

fn offset_with_prefixes(text: &str, cursor: usize, first: &str, cont: &str) -> usize {
    first.len() + cursor + text[..cursor].matches('\n').count() * cont.len()
}

/// `^[ \t]*\[[ xX]\](?:[ \t]+|$)`
fn has_task_marker(s: &str) -> bool {
    let t = s.trim_start_matches(is_blank_ws).as_bytes();
    t.len() >= 3
        && t[0] == b'['
        && matches!(t[1], b' ' | b'x' | b'X')
        && t[2] == b']'
        && (t.len() == 3 || matches!(t[3], b' ' | b'\t'))
}

fn fenced_code(text: &str) -> (String, String) {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(3.max(longest + 1));
    let wrapped = format!("{fence}\n{text}\n{fence}");
    (fence, wrapped)
}

struct Change {
    from: usize,
    to: usize,
    insert: String,
}

/// Merge non-overlapping changes into one edit (one undo step).
fn merge_changes(text: &str, mut changes: Vec<Change>, caret: usize) -> Edit {
    changes.sort_by_key(|c| c.from);
    let start = changes.iter().map(|c| c.from).min().unwrap_or(0);
    let end = changes.iter().map(|c| c.to).max().unwrap_or(0);
    let mut out = String::new();
    let mut pos = start;
    for c in &changes {
        out.push_str(&text[pos..c.from]);
        out.push_str(&c.insert);
        pos = c.to;
    }
    out.push_str(&text[pos..end]);
    Edit {
        range: start..end,
        text: out,
        selection: caret..caret,
    }
}

/// Prefix of continuation lines of a block inside a quote or list item.
fn continuation_prefix(block: &Block, ctx: &BlockCtx) -> String {
    if ctx.quote_end.is_some() {
        "> ".to_owned()
    } else if let Some(mark) = &ctx.item_mark {
        " ".repeat(1.max(block.from.saturating_sub(mark.start)))
    } else {
        String::new()
    }
}

/// Equation started on a later line of a multi-line paragraph: only that
/// line becomes the display equation.
fn equation_on_own_line(
    text: &str,
    block: &Block,
    trigger: &SlashTrigger,
    line: Range<usize>,
) -> Option<Edit> {
    let to = line.end.min(block.to);
    let from = line.start;
    let (clean, cursor) = remove_slash_query(
        &text[from..to],
        trigger.slash.checked_sub(from)?,
        trigger.caret.checked_sub(from)?,
    )?;
    let s = strip_heading_marker(&clean, cursor);
    let content = format!("{}{}", s.leading, s.body);
    let content_cursor = s.leading.len() + s.cursor;
    let after = if line.end < block.to { "\n" } else { "" };
    let caret = from + 1 + 3 + content_cursor;
    Some(Edit {
        range: from..to,
        text: format!("\n$$\n{content}\n$${after}"),
        selection: caret..caret,
    })
}

fn block_edit(
    text: &str,
    block: &Block,
    ctx: &BlockCtx,
    trigger: &SlashTrigger,
    command: SlashCommand,
) -> Option<Edit> {
    use SlashCommand as C;
    let line_start = text[..trigger.slash].rfind('\n').map_or(0, |i| i + 1);
    let mut line_end = text[trigger.slash..]
        .find('\n')
        .map_or(text.len(), |i| trigger.slash + i);
    if text[..line_end].ends_with('\r') {
        line_end -= 1;
    }
    if command == C::Equation
        && ctx.item_mark.is_none()
        && ctx.quote_end.is_none()
        && line_start > block.from
    {
        return equation_on_own_line(text, block, trigger, line_start..line_end);
    }

    let (clean, clean_cursor) = remove_slash_query(
        &text[block.from..block.to],
        trigger.slash.checked_sub(block.from)?,
        trigger.caret.checked_sub(block.from)?,
    )?;
    let s = strip_heading_marker(&clean, clean_cursor);
    let (lead, body, cursor) = (s.leading, s.body, s.cursor);
    let content = format!("{lead}{body}");
    let content_cursor = lead.len() + cursor;
    let mut changes = Vec::new();
    let mut replacement = content.clone();
    let mut rep_cursor = content_cursor;

    match command {
        C::BulletedList | C::NumberedList | C::TodoList => {
            let numbered = command == C::NumberedList;
            let list_prefix = match command {
                C::NumberedList => "1. ",
                C::TodoList => "- [ ] ",
                _ => "- ",
            };
            if let Some(mark) = &ctx.item_mark {
                if command != C::TodoList && ctx.list_ordered != Some(numbered) {
                    changes.push(Change {
                        from: mark.start,
                        to: mark.end,
                        insert: if numbered { "1." } else { "-" }.to_owned(),
                    });
                }
                if command == C::TodoList && !has_task_marker(&replacement) {
                    replacement = format!("{lead}[ ] {body}");
                    rep_cursor += 4;
                }
            } else {
                let prefix = if command == C::TodoList && has_task_marker(&replacement) {
                    "- "
                } else {
                    list_prefix
                };
                let cont = " ".repeat(prefix.len());
                replacement = format!("{lead}{}", prefix_lines(body, prefix, &cont));
                rep_cursor = lead.len() + offset_with_prefixes(body, cursor, prefix, &cont);
            }
        }
        C::Quote => {
            if ctx.quote_end.is_none() {
                replacement = format!("{lead}{}", prefix_lines(body, "> ", "> "));
                rep_cursor = lead.len() + offset_with_prefixes(body, cursor, "> ", "> ");
            }
        }
        C::CodeBlock | C::Equation => {
            let (open, wrapped) = if command == C::CodeBlock {
                fenced_code(&content)
            } else {
                ("$$".to_owned(), format!("$$\n{content}\n$$"))
            };
            let cont = continuation_prefix(block, ctx);
            replacement = prefix_lines(&wrapped, "", &cont);
            rep_cursor = open.len() + 1 + content_cursor;
            rep_cursor += content[..content_cursor].matches('\n').count() * cont.len();
        }
        C::Divider => {
            replacement = clean;
            rep_cursor = clean_cursor;
        }
        _ => return None,
    }

    changes.push(Change {
        from: block.from,
        to: block.to,
        insert: replacement,
    });
    if command == C::Divider {
        let after = ctx.item_end.or(ctx.quote_end).unwrap_or(block.to);
        changes.push(Change {
            from: after,
            to: after,
            insert: "\n\n---".to_owned(),
        });
    }
    let mut caret = block.from + rep_cursor;
    for c in changes.iter().filter(|c| c.from < block.from) {
        caret = caret + c.insert.len() - (c.to - c.from);
    }
    Some(merge_changes(text, changes, caret))
}
