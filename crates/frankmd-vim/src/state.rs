//! The engine: mode handling and dispatch of parsed commands to motions,
//! operators and the command line.

use std::collections::HashMap;

use crate::buffer::VimBuffer;
use crate::cmdline::{self, ExLine, LineCtx, Substitute};
use crate::ex::ExCall;
use crate::keymap::{self, Key};
use crate::motion::{FindSpec, Motion, MotionCtx, Target, word_end};
use crate::operator::{self, Operator, rewrite};
use crate::parse::{self, Action, InsertAt, OpTarget, Parse, Parsed, Tok, VisualAction};
use crate::region::{Region, map_pieces};
use crate::register::{RegKind, Registers};
use crate::search::{self, Search};
use crate::text::{
    CharClass, at_column, clamp_boundary, clamp_normal, class_at, column, first_non_blank,
    indentation, line_count, line_end, line_index, line_start, line_start_of, next_pos, prev_pos,
};

/// Editing mode (drives the `VIM · NORMAL` indicator and cursor shape).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
    Replace,
    /// Typing an ex command after `:` (or a search after `/`, `?`).
    CommandLine,
}

/// What the app should do with the key after the engine saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Consumed; do not forward to the editor.
    Handled,
    /// Not ours (insert-mode text, IME composition, passthrough Ctrl-key):
    /// forward to the editor / app keymap.
    PassThrough,
    /// An ex command mapped to an app action.
    App(ExCall),
}

impl Mode {
    /// **[COMPAT]** indicator label (`observeVimMode`): `"normal"`, `"insert"`, `"visual"`,
    /// `"visual-linewise"`, `"visual-blockwise"`, `"replace"`. While typing a `:`/`/` line
    /// codemirror-vim still reports normal mode, so [`Mode::CommandLine`] → `"normal"`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal | Self::CommandLine => "normal",
            Self::Insert => "insert",
            Self::Visual => "visual",
            Self::VisualLine => "visual-linewise",
            Self::VisualBlock => "visual-blockwise",
            Self::Replace => "replace",
        }
    }
}

/// A repeatable change, replayed by `.` as keys.
#[derive(Debug, Clone, Default)]
struct Change {
    register: Option<char>,
    count: Option<usize>,
    /// Command keys without register prefix and counts.
    keys: Vec<Key>,
    /// Keys typed in insert/replace mode (or on the search line).
    typed: Vec<Key>,
    /// Replay ends with `<Esc>`.
    ends_insert: bool,
}

/// How `o`/`O` opened the line (for count repetition).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenLine {
    Below,
    Above,
}

/// An insert or replace session in progress.
#[derive(Debug, Clone)]
struct InsertSession {
    /// Change to record on `<Esc>` (`None` = not repeatable).
    change: Option<Change>,
    /// `3ifoo<Esc>` inserts three times.
    count: usize,
    open: Option<OpenLine>,
    typed: Vec<Key>,
}

/// An operator waiting for a `/` or `?` line.
#[derive(Debug, Clone)]
struct PendingOp {
    op: Operator,
    register: Option<char>,
    change: Change,
}

/// The command line being typed.
#[derive(Debug, Clone, Default)]
struct CmdLine {
    /// `:`, `/` or `?`.
    kind: char,
    pending_op: Option<PendingOp>,
    /// Visual mode the line was opened from.
    from_visual: Option<Mode>,
}

/// Engine state. One per editor view.
#[derive(Debug, Default)]
pub struct VimState {
    mode: Mode,
    pending: Vec<Key>,
    command_line: String,
    /// Platform: on macOS vim keeps its whole Ctrl keymap (no passthrough).
    macos: bool,
    /// Mode change not yet observed by the app.
    mode_changed: Option<Mode>,
    cmd: CmdLine,
    /// Visual selection ends (byte offsets of chars).
    anchor: usize,
    head: usize,
    /// For `gv`.
    last_visual: Option<(Mode, usize, usize)>,
    /// Desired column for vertical motions (`usize::MAX` after `$`).
    goal_col: Option<usize>,
    last_find: Option<FindSpec>,
    search: Search,
    registers: Registers,
    /// Marks by name; `'` is the previous jump position.
    marks: HashMap<char, usize>,
    last_change: Option<Change>,
    insert: Option<InsertSession>,
    /// Replaying `.`: insert-mode keys are applied by the engine.
    replaying: bool,
    message: Option<String>,
}

impl VimState {
    /// New engine in normal mode. `macos` selects the passthrough policy
    /// ([`crate::keymap::app_owned_keys`]).
    pub fn new(macos: bool) -> Self {
        Self {
            macos,
            ..Self::default()
        }
    }

    /// The mode the engine switched to since the last call, if it changed
    /// (`vim-mode-change`; no change → `None`, like events without a mode).
    pub fn take_mode_change(&mut self) -> Option<Mode> {
        self.mode_changed.take()
    }

    /// Current mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Text of the `:` / `/` line being typed, if any.
    pub fn command_line(&self) -> Option<&str> {
        (self.mode == Mode::CommandLine).then_some(self.command_line.as_str())
    }

    /// Status message from the last command (`Pattern not found: x`, ...), if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The pattern the editor should highlight (last search, until `:noh`).
    pub fn search_highlight(&self) -> Option<&str> {
        self.search
            .pattern
            .as_deref()
            .filter(|_| self.search.highlight)
    }

    /// Feed one key. `composing` = IME preedit active (always PassThrough).
    pub fn feed(&mut self, key: &Key, composing: bool, buf: &mut dyn VimBuffer) -> Outcome {
        if composing {
            return Outcome::PassThrough;
        }
        if self.mode != Mode::CommandLine && keymap::is_app_owned(key, self.macos) {
            self.pending.clear();
            return Outcome::PassThrough;
        }
        match self.mode {
            Mode::Insert => self.feed_insert(key, buf),
            Mode::Replace => self.feed_replace(key, buf),
            Mode::CommandLine => self.feed_command_line(key, buf),
            Mode::Normal => self.feed_normal(key, buf),
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock => self.feed_visual(key, buf),
        }
    }

    /// Whether IME should be enabled (insert/replace only, OM-007).
    pub fn ime_allowed(&self) -> bool {
        matches!(self.mode, Mode::Insert | Mode::Replace | Mode::CommandLine)
    }

    // ----- helpers ---------------------------------------------------------

    fn set_mode(&mut self, mode: Mode) {
        if self.mode != mode {
            let old = self.mode;
            self.mode = mode;
            if old.label() != mode.label() {
                self.mode_changed = Some(mode);
            }
        }
    }

    /// Normal-mode caret (selection head, snapped onto a char).
    fn caret(buf: &dyn VimBuffer) -> usize {
        let t = buf.text();
        clamp_normal(&t, buf.selection().end)
    }

    fn set_caret(buf: &mut dyn VimBuffer, pos: usize) {
        let p = clamp_normal(&buf.text(), pos);
        buf.set_selection(p..p);
    }

    /// Insert-mode caret: may sit at the end of a line.
    fn set_insert_caret(buf: &mut dyn VimBuffer, pos: usize) {
        let p = clamp_boundary(&buf.text(), pos);
        buf.set_selection(p..p);
    }

    fn motion_ctx<'a>(
        &'a self,
        text: &'a str,
        pos: usize,
        count: Option<usize>,
        buf: &dyn VimBuffer,
    ) -> MotionCtx<'a> {
        MotionCtx {
            text,
            pos,
            count,
            goal_col: self.goal_col,
            last_find: self.last_find,
            search: &self.search,
            marks: &self.marks,
            visible: buf.visible_lines(),
        }
    }

    /// Resolve a motion from `pos`, remembering `f`/`t` targets for `;`.
    fn resolve(
        &mut self,
        m: Motion,
        text: &str,
        pos: usize,
        count: Option<usize>,
        buf: &dyn VimBuffer,
    ) -> Option<Target> {
        if let Motion::Find(spec) = m {
            self.last_find = Some(spec);
        }
        m.resolve(&self.motion_ctx(text, pos, count, buf))
    }

    /// Goal column after a motion from `from`.
    fn update_goal(&mut self, m: Motion, text: &str, from: usize) {
        self.goal_col = if m.is_vertical() {
            Some(self.goal_col.unwrap_or_else(|| column(text, from)))
        } else if m == Motion::LineEnd {
            Some(usize::MAX)
        } else {
            None
        };
    }

    fn record(&mut self, change: Change) {
        self.last_change = Some(change);
    }

    // ----- normal mode -----------------------------------------------------

    fn feed_normal(&mut self, key: &Key, buf: &mut dyn VimBuffer) -> Outcome {
        self.pending.push(key.clone());
        let toks: Vec<Tok> = self.pending.iter().map(Tok::of).collect();
        match parse::normal(&toks) {
            Parse::Incomplete => Outcome::Handled,
            Parse::Unbound => {
                self.pending.clear();
                Outcome::PassThrough
            }
            Parse::Invalid => {
                self.pending.clear();
                Outcome::Handled
            }
            Parse::Done(parsed) => {
                let keys = std::mem::take(&mut self.pending);
                self.message = None;
                self.run_normal(parsed, &keys, buf)
            }
        }
    }

    fn run_normal(&mut self, p: Parsed<Action>, keys: &[Key], buf: &mut dyn VimBuffer) -> Outcome {
        let n = p.count.unwrap_or(1).max(1);
        let cursor = Self::caret(buf);
        let change = Change {
            register: p.register,
            count: p.count,
            keys: keys
                .iter()
                .enumerate()
                .filter(|(i, _)| !p.prefix_keys.contains(i))
                .map(|(_, k)| k.clone())
                .collect(),
            typed: Vec::new(),
            ends_insert: false,
        };
        if !matches!(p.action, Action::Move(_)) {
            self.goal_col = None;
        }
        match p.action {
            Action::Move(m) => self.move_caret(m, p.count, cursor, buf),
            Action::Operate(op, target) => {
                if let Some(region) = self.op_region(op, target, p.count, cursor, buf) {
                    self.run_operator(op, &region, p.register, cursor, 1, buf, Some(change));
                }
            }
            Action::OperateSearch(op, forward) => {
                let pending = PendingOp {
                    op,
                    register: p.register,
                    change,
                };
                self.open_command_line(if forward { '/' } else { '?' }, Some(pending), None);
            }
            Action::Put { before } => {
                if let Some(c) = operator::put(buf, &self.registers, p.register, before, n, cursor)
                {
                    Self::set_caret(buf, c);
                    self.record(change);
                }
            }
            Action::Join => {
                let t = buf.text().into_owned();
                if let Some(c) = operator::join(buf, line_index(&t, cursor), n) {
                    Self::set_caret(buf, c);
                    self.record(change);
                }
            }
            Action::ToggleCaseChar => {
                let t = buf.text().into_owned();
                let end = (0..n).fold(cursor, |p, _| next_pos(&t, p).min(line_end(&t, cursor)));
                if end > cursor {
                    operator::apply(
                        Operator::ToggleCase,
                        &Region::Chars(cursor..end),
                        buf,
                        &mut self.registers,
                        None,
                        cursor,
                        1,
                    );
                    Self::set_caret(buf, end);
                    self.record(change);
                }
            }
            Action::ReplaceChar(c) => self.replace_chars(c, n, cursor, buf, change),
            Action::Insert(at) => self.start_insert(at, n, cursor, buf, Some(change)),
            Action::Visual(mode) => {
                self.anchor = cursor;
                self.head = cursor;
                self.set_mode(mode);
                self.show_visual(buf);
            }
            Action::Reselect => {
                if let Some((mode, a, h)) = self.last_visual {
                    let t = buf.text();
                    self.anchor = clamp_normal(&t, a);
                    self.head = clamp_normal(&t, h);
                    drop(t);
                    self.set_mode(mode);
                    self.show_visual(buf);
                }
            }
            Action::ReplaceMode => {
                self.insert = Some(InsertSession {
                    change: Some(change),
                    count: 1,
                    open: None,
                    typed: Vec::new(),
                });
                self.set_mode(Mode::Replace);
            }
            Action::Undo | Action::Redo => {
                for _ in 0..n {
                    if p.action == Action::Undo {
                        buf.undo()
                    } else {
                        buf.redo()
                    }
                }
                let pos = buf.selection().start;
                Self::set_caret(buf, pos);
            }
            Action::Repeat => self.repeat(p.count, buf),
            Action::CommandLine(kind) => self.open_command_line(kind, None, None),
            Action::SearchWord(forward) => self.search_word(forward, n, cursor, buf),
            Action::SetMark(c) => {
                self.marks.insert(c, cursor);
            }
            Action::Scroll(anchor) => {
                let line = line_index(&buf.text(), cursor);
                buf.scroll_to_line(line, anchor);
            }
            Action::Nop => {}
        }
        Outcome::Handled
    }

    fn move_caret(
        &mut self,
        m: Motion,
        count: Option<usize>,
        cursor: usize,
        buf: &mut dyn VimBuffer,
    ) {
        let t = buf.text().into_owned();
        let Some(target) = self.resolve(m, &t, cursor, count, buf) else {
            return;
        };
        if m.is_jump() {
            self.marks.insert('\'', cursor);
        }
        self.update_goal(m, &t, cursor);
        Self::set_caret(buf, target.pos);
    }

    /// The region an operator acts on in normal mode.
    fn op_region(
        &mut self,
        op: Operator,
        target: OpTarget,
        count: Option<usize>,
        cursor: usize,
        buf: &dyn VimBuffer,
    ) -> Option<Region> {
        let t = buf.text().into_owned();
        let n = count.unwrap_or(1).max(1);
        match target {
            OpTarget::Line => {
                let first = line_index(&t, cursor);
                Some(Region::Lines {
                    first,
                    last: (first + n - 1).min(line_count(&t) - 1),
                })
            }
            OpTarget::Object(obj) => obj.resolve(&t, cursor),
            OpTarget::Motion(m) => {
                // `cw` on a non-blank changes to the end of the word (like `ce`).
                if let (Operator::Change, Motion::WordForward { big }) = (op, m)
                    && class_at(&t, cursor, big) != CharClass::Space
                {
                    let mut p = cursor;
                    let at_end =
                        class_at(&t, next_pos(&t, cursor), big) != class_at(&t, cursor, big);
                    for _ in 0..n - usize::from(at_end) {
                        p = word_end(&t, p, big);
                    }
                    return Some(Region::Chars(cursor..next_pos(&t, p)));
                }
                let target = self.resolve(m, &t, cursor, count, buf)?;
                Some(motion_region(
                    &t,
                    cursor,
                    target,
                    matches!(m, Motion::WordForward { .. }),
                ))
            }
        }
    }

    /// Run an operator and handle the caret/mode/record bookkeeping.
    #[allow(clippy::too_many_arguments)]
    fn run_operator(
        &mut self,
        op: Operator,
        region: &Region,
        register: Option<char>,
        cursor: usize,
        shift: usize,
        buf: &mut dyn VimBuffer,
        change: Option<Change>,
    ) {
        let out = operator::apply(
            op,
            region,
            buf,
            &mut self.registers,
            register,
            cursor,
            shift,
        );
        if out.insert {
            Self::set_insert_caret(buf, out.cursor);
            self.insert = Some(InsertSession {
                change,
                count: 1,
                open: None,
                typed: Vec::new(),
            });
            self.set_mode(Mode::Insert);
        } else {
            Self::set_caret(buf, out.cursor);
            self.set_mode(Mode::Normal);
            if op.is_change()
                && let Some(c) = change
            {
                self.record(c);
            }
        }
    }

    fn replace_chars(
        &mut self,
        c: char,
        n: usize,
        cursor: usize,
        buf: &mut dyn VimBuffer,
        change: Change,
    ) {
        let t = buf.text().into_owned();
        let le = line_end(&t, cursor);
        let mut end = cursor;
        for _ in 0..n {
            if end >= le {
                return; // Not enough chars: vim beeps.
            }
            end = next_pos(&t, end);
        }
        if c == '\n' {
            buf.replace(cursor..end, "\n");
            Self::set_caret(buf, cursor + 1);
        } else {
            let text = c.to_string().repeat(n);
            buf.replace(cursor..end, &text);
            Self::set_caret(buf, cursor + text.len() - c.len_utf8());
        }
        self.record(change);
    }

    fn start_insert(
        &mut self,
        at: InsertAt,
        n: usize,
        cursor: usize,
        buf: &mut dyn VimBuffer,
        change: Option<Change>,
    ) {
        let t = buf.text().into_owned();
        let ls = line_start(&t, cursor);
        let le = line_end(&t, cursor);
        let (pos, open) = match at {
            InsertAt::Cursor => (cursor, None),
            InsertAt::After => (
                if cursor < le {
                    next_pos(&t, cursor)
                } else {
                    cursor
                },
                None,
            ),
            InsertAt::LineStart => (first_non_blank(&t, ls), None),
            InsertAt::LineEnd => (le, None),
            InsertAt::OpenBelow => {
                let indent = indentation(&t, ls).to_owned();
                buf.replace(le..le, &format!("\n{indent}"));
                (le + 1 + indent.len(), Some(OpenLine::Below))
            }
            InsertAt::OpenAbove => {
                let indent = indentation(&t, ls).to_owned();
                buf.replace(ls..ls, &format!("{indent}\n"));
                (ls + indent.len(), Some(OpenLine::Above))
            }
        };
        Self::set_insert_caret(buf, pos);
        self.insert = Some(InsertSession {
            change,
            count: n,
            open,
            typed: Vec::new(),
        });
        self.set_mode(Mode::Insert);
    }

    fn search_word(&mut self, forward: bool, n: usize, cursor: usize, buf: &mut dyn VimBuffer) {
        let t = buf.text().into_owned();
        let Some((start, word)) = search::word_at(&t, cursor) else {
            self.message = Some("E348: No string under cursor".to_owned());
            return;
        };
        let pattern = format!("\\<{}\\>", regex::escape(word));
        if let Some(p) = search::find(&t, start, &pattern, forward, n) {
            self.marks.insert('\'', cursor);
            Self::set_caret(buf, p);
        }
        self.search.set(pattern, forward);
    }

    /// `.`: replay the last change as keys.
    fn repeat(&mut self, count: Option<usize>, buf: &mut dyn VimBuffer) {
        let Some(change) = self.last_change.clone() else {
            return;
        };
        let mut keys = Vec::new();
        if let Some(r) = change.register {
            keys.push(Key::plain("\""));
            keys.push(Key::plain(&r.to_string()));
        }
        if let Some(n) = count.or(change.count) {
            keys.extend(n.to_string().chars().map(|d| Key::plain(&d.to_string())));
        }
        keys.extend(change.keys);
        keys.extend(change.typed);
        if change.ends_insert {
            keys.push(Key::plain("escape"));
        }
        let was = std::mem::replace(&mut self.replaying, true);
        for k in &keys {
            self.feed(k, false, buf);
        }
        self.replaying = was;
    }

    // ----- insert / replace ------------------------------------------------

    fn feed_insert(&mut self, key: &Key, buf: &mut dyn VimBuffer) -> Outcome {
        let tok = Tok::of(key);
        if tok == Tok::Esc {
            self.finish_insert(buf);
            return Outcome::Handled;
        }
        if let Some(session) = self.insert.as_mut() {
            match tok {
                Tok::Char(_) | Tok::Enter | Tok::Backspace | Tok::Tab | Tok::Delete => {
                    session.typed.push(key.clone())
                }
                // Moving around starts a new insert as far as repetition is concerned.
                Tok::Left
                | Tok::Right
                | Tok::Up
                | Tok::Down
                | Tok::Home
                | Tok::End
                | Tok::PageUp
                | Tok::PageDown => {
                    session.typed.clear();
                    session.count = 1;
                    session.open = None;
                }
                _ => {}
            }
        }
        if self.replaying {
            apply_insert_key(buf, tok);
            return Outcome::Handled;
        }
        Outcome::PassThrough
    }

    fn finish_insert(&mut self, buf: &mut dyn VimBuffer) {
        if let Some(session) = self.insert.take() {
            for _ in 1..session.count {
                if session.open.is_some() {
                    let t = buf.text().into_owned();
                    let le = line_end(&t, clamp_boundary(&t, buf.selection().end));
                    buf.replace(le..le, "\n");
                    Self::set_insert_caret(buf, le + 1);
                }
                for k in &session.typed {
                    apply_insert_key(buf, Tok::of(k));
                }
            }
            if let Some(mut change) = session.change {
                change.typed = session.typed;
                change.ends_insert = true;
                self.record(change);
            }
        }
        let t = buf.text().into_owned();
        let pos = clamp_boundary(&t, buf.selection().end);
        let p = if pos > line_start(&t, pos) {
            prev_pos(&t, pos)
        } else {
            pos
        };
        buf.set_selection(p..p);
        self.set_mode(Mode::Normal);
    }

    fn feed_replace(&mut self, key: &Key, buf: &mut dyn VimBuffer) -> Outcome {
        let tok = Tok::of(key);
        if tok == Tok::Esc {
            self.finish_insert(buf);
            return Outcome::Handled;
        }
        let t = buf.text().into_owned();
        let pos = clamp_boundary(&t, buf.selection().end);
        let ch = match tok {
            Tok::Char(c) => Some(c),
            Tok::Enter => Some('\n'),
            Tok::Tab => Some('\t'),
            _ => None,
        };
        if let Some(c) = ch {
            let end = if c != '\n' && pos < line_end(&t, pos) {
                next_pos(&t, pos)
            } else {
                pos
            };
            buf.replace(pos..end, &c.to_string());
            Self::set_insert_caret(buf, pos + c.len_utf8());
        } else if tok == Tok::Backspace && pos > line_start(&t, pos) {
            Self::set_insert_caret(buf, prev_pos(&t, pos));
        } else {
            return Outcome::Handled;
        }
        if let Some(session) = self.insert.as_mut() {
            session.typed.push(key.clone());
        }
        Outcome::Handled
    }

    // ----- command line ----------------------------------------------------

    fn open_command_line(
        &mut self,
        kind: char,
        pending_op: Option<PendingOp>,
        from_visual: Option<Mode>,
    ) {
        self.cmd = CmdLine {
            kind,
            pending_op,
            from_visual,
        };
        self.command_line.clear();
        if kind == ':' && from_visual.is_some() {
            self.command_line.push_str("'<,'>");
        }
        self.set_mode(Mode::CommandLine);
    }

    fn feed_command_line(&mut self, key: &Key, buf: &mut dyn VimBuffer) -> Outcome {
        match Tok::of(key) {
            Tok::Esc | Tok::Ctrl('c') => self.cancel_command_line(buf),
            Tok::Enter => {
                let line = std::mem::take(&mut self.command_line);
                return self.run_command_line(line, buf);
            }
            Tok::Backspace | Tok::Ctrl('h') => {
                if self.command_line.pop().is_none() {
                    self.cancel_command_line(buf);
                }
            }
            Tok::Ctrl('u') => self.command_line.clear(),
            Tok::Char(c) => self.command_line.push(c),
            Tok::Tab => self.command_line.push('\t'),
            _ => {}
        }
        Outcome::Handled
    }

    fn cancel_command_line(&mut self, buf: &mut dyn VimBuffer) {
        let cmd = std::mem::take(&mut self.cmd);
        self.command_line.clear();
        self.set_mode(Mode::Normal);
        if cmd.from_visual.is_some() {
            Self::set_caret(buf, self.head);
        }
    }

    fn run_command_line(&mut self, line: String, buf: &mut dyn VimBuffer) -> Outcome {
        let cmd = std::mem::take(&mut self.cmd);
        self.message = None;
        if cmd.kind == ':' {
            self.set_mode(Mode::Normal);
            if cmd.from_visual.is_some() {
                Self::set_caret(buf, self.head);
            }
            return self.run_ex(&line, buf);
        }
        let forward = cmd.kind == '/';
        let Some(pattern) = Some(line.clone())
            .filter(|l| !l.is_empty())
            .or_else(|| self.search.pattern.clone())
        else {
            self.message = Some("E35: No previous regular expression".to_owned());
            self.set_mode(Mode::Normal);
            return Outcome::Handled;
        };
        self.search.set(pattern.clone(), forward);
        let t = buf.text().into_owned();
        let cursor = if cmd.from_visual.is_some() {
            self.head
        } else {
            Self::caret(buf)
        };
        let found = search::find(&t, cursor, &pattern, forward, 1);
        if found.is_none() {
            self.message = Some(format!("Pattern not found: {pattern}"));
        }
        match (cmd.pending_op, cmd.from_visual) {
            (Some(pending), _) => {
                self.set_mode(Mode::Normal);
                let mut change = pending.change;
                change.typed = line.chars().map(|c| Key::plain(&c.to_string())).collect();
                change.typed.push(Key::plain("enter"));
                if let Some(pos) = found {
                    let region = motion_region(
                        &t,
                        cursor,
                        Target {
                            pos,
                            linewise: false,
                            inclusive: false,
                        },
                        false,
                    );
                    self.run_operator(
                        pending.op,
                        &region,
                        pending.register,
                        cursor,
                        1,
                        buf,
                        Some(change),
                    );
                }
            }
            (None, Some(mode)) => {
                if let Some(pos) = found {
                    self.head = pos;
                }
                self.set_mode(mode);
                self.show_visual(buf);
            }
            (None, None) => {
                self.set_mode(Mode::Normal);
                if let Some(pos) = found {
                    self.marks.insert('\'', cursor);
                    Self::set_caret(buf, pos);
                }
            }
        }
        Outcome::Handled
    }

    fn run_ex(&mut self, line: &str, buf: &mut dyn VimBuffer) -> Outcome {
        let t = buf.text().into_owned();
        let cursor = Self::caret(buf);
        let parsed = cmdline::parse(
            line,
            &LineCtx {
                text: &t,
                cursor,
                marks: &self.marks,
            },
        );
        match parsed {
            ExLine::App(call) => return Outcome::App(call),
            ExLine::Empty => {}
            ExLine::Goto(line) => {
                self.marks.insert('\'', cursor);
                Self::set_caret(buf, first_non_blank(&t, line_start_of(&t, line)));
            }
            ExLine::NoHighlight => self.search.highlight = false,
            ExLine::Substitute(s) => self.substitute(s, buf),
            ExLine::Error(msg) => self.message = Some(msg),
        }
        Outcome::Handled
    }

    fn substitute(&mut self, s: Substitute, buf: &mut dyn VimBuffer) {
        let Some(pattern) = s.pattern.or_else(|| self.search.pattern.clone()) else {
            self.message = Some("E35: No previous regular expression".to_owned());
            return;
        };
        self.search.pattern = Some(pattern.clone());
        self.search.highlight = true;
        let Some(re) = search::compile_with(&pattern, s.ignore_case) else {
            return;
        };
        let replacement = search::translate_replacement(&s.replacement);
        let t = buf.text().into_owned();
        let span = Region::Lines {
            first: s.first,
            last: s.last,
        }
        .span(&t);
        let mut out = String::with_capacity(span.len());
        let mut last_changed = None;
        for (i, line) in t[span.clone()].split('\n').enumerate() {
            if i > 0 {
                out.push('\n');
            }
            if re.is_match(line) {
                last_changed = Some(out.len());
                let new = if s.global {
                    re.replace_all(line, replacement.as_str())
                } else {
                    re.replacen(line, 1, replacement.as_str())
                };
                out.push_str(&new);
            } else {
                out.push_str(line);
            }
        }
        let Some(offset) = last_changed else {
            self.message = Some(format!("No matches for {pattern}"));
            return;
        };
        rewrite(buf, &t, span.clone(), &out);
        let new = buf.text().into_owned();
        let pos = clamp_boundary(&new, span.start + offset);
        Self::set_caret(buf, first_non_blank(&new, line_start(&new, pos)));
    }

    // ----- visual mode -----------------------------------------------------

    fn feed_visual(&mut self, key: &Key, buf: &mut dyn VimBuffer) -> Outcome {
        self.pending.push(key.clone());
        let toks: Vec<Tok> = self.pending.iter().map(Tok::of).collect();
        match parse::visual(&toks) {
            Parse::Incomplete => Outcome::Handled,
            Parse::Unbound => {
                self.pending.clear();
                Outcome::PassThrough
            }
            Parse::Invalid => {
                self.pending.clear();
                Outcome::Handled
            }
            Parse::Done(parsed) => {
                self.pending.clear();
                self.message = None;
                self.run_visual(parsed, buf);
                Outcome::Handled
            }
        }
    }

    /// The selection as an operator region.
    fn visual_region(&self, t: &str) -> Region {
        let (s, e) = (self.anchor.min(self.head), self.anchor.max(self.head));
        match self.mode {
            Mode::VisualLine => Region::Lines {
                first: line_index(t, s),
                last: line_index(t, e),
            },
            Mode::VisualBlock => {
                let (ca, ch) = (column(t, self.anchor), column(t, self.head));
                let (c1, c2) = (ca.min(ch), ca.max(ch));
                let to_end = self.goal_col == Some(usize::MAX);
                let pieces = (line_index(t, s)..=line_index(t, e))
                    .map(|l| {
                        let ls = line_start_of(t, l);
                        let le = line_end(t, ls);
                        let start = at_column(t, ls, c1);
                        let end = if to_end {
                            le
                        } else {
                            next_pos(t, at_column(t, ls, c2)).min(le)
                        };
                        start..end.max(start)
                    })
                    .collect();
                Region::Block(pieces)
            }
            _ => Region::Chars(s..next_pos(t, e)),
        }
    }

    fn show_visual(&self, buf: &mut dyn VimBuffer) {
        let t = buf.text().into_owned();
        let span = self.visual_region(&t).span(&t);
        buf.set_selection(span);
    }

    /// Remember the selection for `gv` and the `'<` / `'>` marks.
    fn save_visual(&mut self) {
        self.last_visual = Some((self.mode, self.anchor, self.head));
        self.marks.insert('<', self.anchor.min(self.head));
        self.marks.insert('>', self.anchor.max(self.head));
    }

    fn exit_visual(&mut self, buf: &mut dyn VimBuffer) {
        self.save_visual();
        self.set_mode(Mode::Normal);
        Self::set_caret(buf, self.head);
    }

    fn run_visual(&mut self, p: Parsed<VisualAction>, buf: &mut dyn VimBuffer) {
        let n = p.count.unwrap_or(1).max(1);
        let t = buf.text().into_owned();
        let start = self.anchor.min(self.head);
        match p.action {
            VisualAction::Move(m) => {
                let from = self.head;
                if let Some(target) = self.resolve(m, &t, from, p.count, buf) {
                    if m.is_jump() {
                        self.marks.insert('\'', from);
                    }
                    self.update_goal(m, &t, from);
                    self.head = clamp_normal(&t, target.pos);
                }
                self.show_visual(buf);
            }
            VisualAction::Operate(op) | VisualAction::OperateLines(op) => {
                let mut region = self.visual_region(&t);
                if matches!(p.action, VisualAction::OperateLines(_))
                    || matches!(op, Operator::Indent | Operator::Outdent)
                {
                    let (first, last) = region.lines(&t);
                    region = Region::Lines { first, last };
                }
                self.save_visual();
                self.set_mode(Mode::Normal);
                let cursor = if op == Operator::Yank {
                    start
                } else {
                    self.head
                };
                self.run_operator(op, &region, p.register, cursor, n, buf, None);
                if op == Operator::Yank {
                    Self::set_caret(buf, start);
                }
            }
            VisualAction::Object(obj) => {
                match obj.resolve(&t, self.head) {
                    Some(Region::Chars(r)) if !r.is_empty() => {
                        let end = prev_pos(&t, r.end);
                        if self.anchor == self.head {
                            self.anchor = r.start;
                            self.head = end;
                        } else {
                            self.anchor = self.anchor.min(r.start);
                            self.head = self.head.max(end);
                        }
                    }
                    Some(Region::Lines { first, last }) => {
                        self.anchor = line_start_of(&t, first);
                        self.head = clamp_normal(&t, line_end(&t, line_start_of(&t, last)));
                        if self.mode == Mode::Visual {
                            self.set_mode(Mode::VisualLine);
                        }
                    }
                    _ => {}
                }
                self.show_visual(buf);
            }
            VisualAction::SwapEnds => {
                std::mem::swap(&mut self.anchor, &mut self.head);
                self.show_visual(buf);
            }
            VisualAction::Switch(mode) => {
                if mode == self.mode {
                    self.exit_visual(buf);
                } else {
                    self.set_mode(mode);
                    self.show_visual(buf);
                }
            }
            VisualAction::Exit => self.exit_visual(buf),
            VisualAction::Put => self.visual_put(p.register, &t, buf),
            VisualAction::ReplaceChar(c) => {
                let region = self.visual_region(&t);
                let span = region.span(&t);
                let new = map_pieces(&t, &span, &region.pieces(&t), |s| {
                    s.chars()
                        .map(|ch| if ch == '\n' { '\n' } else { c })
                        .collect()
                });
                rewrite(buf, &t, span, &new);
                self.save_visual();
                self.set_mode(Mode::Normal);
                Self::set_caret(buf, start);
            }
            VisualAction::Join => {
                let (first, last) = self.visual_region(&t).lines(&t);
                self.save_visual();
                self.set_mode(Mode::Normal);
                match operator::join(buf, first, last - first + 1) {
                    Some(c) => Self::set_caret(buf, c),
                    None => Self::set_caret(buf, self.head),
                }
            }
            VisualAction::Insert(append) => {
                let span = self.visual_region(&t).span(&t);
                self.save_visual();
                let pos = if append { span.end } else { span.start };
                Self::set_insert_caret(buf, pos);
                self.insert = Some(InsertSession {
                    change: None,
                    count: 1,
                    open: None,
                    typed: Vec::new(),
                });
                self.set_mode(Mode::Insert);
            }
            VisualAction::CommandLine(kind) => {
                self.save_visual();
                let mode = self.mode;
                self.open_command_line(kind, None, Some(mode));
            }
        }
    }

    /// `p` in visual mode: replace the selection with a register.
    fn visual_put(&mut self, register: Option<char>, t: &str, buf: &mut dyn VimBuffer) {
        let Some(content) = self.registers.get(register).cloned() else {
            return;
        };
        let region = self.visual_region(t);
        let span = region.span(t);
        let replaced = operator::region_content(t, &region);
        let text = match (&region, content.kind) {
            (Region::Lines { .. }, RegKind::Lines) => content
                .text
                .strip_suffix('\n')
                .unwrap_or(&content.text)
                .to_owned(),
            (Region::Lines { .. }, _) => content.text.clone(),
            (_, RegKind::Lines) => format!("\n{}", content.text),
            _ => content.text.clone(),
        };
        let new = match &region {
            Region::Block(pieces) => {
                let mut first = true;
                map_pieces(t, &span, pieces, |_| {
                    if std::mem::take(&mut first) {
                        text.clone()
                    } else {
                        String::new()
                    }
                })
            }
            _ => text,
        };
        rewrite(buf, t, span.clone(), &new);
        self.registers.store(None, replaced, false);
        self.save_visual();
        self.set_mode(Mode::Normal);
        Self::set_caret(buf, span.start);
    }
}

/// Region for an operator + charwise/linewise motion from `cursor`, with
/// vim's exclusive-motion adjustments (`:h o_v`, `:h exclusive-linewise`).
fn motion_region(t: &str, cursor: usize, target: Target, word_motion: bool) -> Region {
    let (start, end) = (cursor.min(target.pos), cursor.max(target.pos));
    if target.linewise {
        return Region::Lines {
            first: line_index(t, start),
            last: line_index(t, end),
        };
    }
    let mut end = if target.inclusive {
        next_pos(t, end)
    } else {
        end
    };
    let (start_line, end_line) = (line_index(t, start), line_index(t, end));
    if word_motion && end_line > start_line {
        // `dw` on the last word of a line stops at the line end.
        while end > start && char_at_is_space(t, prev_pos(t, end)) {
            end = prev_pos(t, end);
        }
    } else if !target.inclusive && end > start && end == line_start(t, end) && end_line > start_line
    {
        if start <= first_non_blank(t, line_start(t, start)) {
            return Region::Lines {
                first: start_line,
                last: end_line - 1,
            };
        }
        end = prev_pos(t, end);
    }
    Region::Chars(start..end)
}

fn char_at_is_space(t: &str, pos: usize) -> bool {
    crate::text::char_at(t, pos).is_some_and(char::is_whitespace)
}

/// Apply one insert-mode key during `.` replay or count repetition.
fn apply_insert_key(buf: &mut dyn VimBuffer, tok: Tok) {
    let sel = buf.selection();
    let t = buf.text().into_owned();
    let (range, text) = match tok {
        Tok::Char(c) => (sel, c.to_string()),
        Tok::Enter => (sel, "\n".to_owned()),
        Tok::Tab => (sel, "\t".to_owned()),
        Tok::Backspace if sel.is_empty() => (prev_pos(&t, sel.start)..sel.start, String::new()),
        Tok::Delete if sel.is_empty() => (sel.start..next_pos(&t, sel.start), String::new()),
        Tok::Backspace | Tok::Delete => (sel, String::new()),
        _ => return,
    };
    let at = range.start + text.len();
    buf.replace(range, &text);
    buf.set_selection(at..at);
}
