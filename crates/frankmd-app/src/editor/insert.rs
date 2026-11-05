//! Insert surfaces the editor pane hosts (ui-spec §5.5, §5.8, §5.9, §6.2):
//!
//! - caret popovers: the slash menu (`/` at a block start, FE-088..090), wikilink
//!   completion (`[[`, FE-092) and link completion after Ctrl+K;
//! - the insert dialogs of [`crate::dialogs::insert`] (table, code block, emoji),
//!   opened by their commands, by a slash command or by the Ctrl+. context action;
//! - the pending `/query` a dialog-opening slash command leaves behind, replaced
//!   on insert only if it is still there (**[COMPAT]** FrankMD's pending range).
//!
//! All text logic is GUI-free (`frankmd_core::text::{slash, pending,
//! link_completion}`, `frankmd_core::wikilink`, `frankmd_workspace::{slash_menu,
//! table_editor, code_block}`); this module routes keys and renders.

use crate::scale::px;
use std::ops::Range;
use std::rc::Rc;

use frankmd_core::text::edit::Edit;
use frankmd_core::text::link_completion::{
    CaretLink, LinkSuggestion, link_at_caret, link_destination_at, link_suggestions,
};
use frankmd_core::text::markdown::{find_code_block_at_position, find_table_at_position};
use frankmd_core::text::pending::{
    BlockTarget, PendingAction, PendingInsertion, block_edit, inline_edit,
};
use frankmd_core::text::slash::{SlashCommand, slash_trigger};
use frankmd_core::wikilink::{WikiCompletion, WikiFile, complete as wiki_complete};
use frankmd_vim::Mode;
use frankmd_workspace::code_block::CodeBlockForm;
use frankmd_workspace::slash_menu::{MENU_WIDTH_PX, ROW_HEIGHT_PX, SlashMenuModel, SlashOutcome};
use frankmd_workspace::table_editor::{DEFAULT_HEADER_TEMPLATE, TableEditorModel};
use gpui_kit::component::input::RopeExt as _;
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::pane::EditorPane;
use crate::dialogs::insert::{self, InsertDialog, InsertDialogEvent};
use crate::ui::i18n;

/// One rendered popover row: icon, label, dim detail.
type PopoverRow = (Option<&'static str>, SharedString, Option<SharedString>);

/// Most completion rows rendered (FrankMD `maxRenderedOptions: 20`).
const MAX_COMPLETIONS: usize = 20;

/// What Ctrl+. does at the caret (ui-spec §6.2 "Context action").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextAction {
    /// The caret is in a fenced code block: edit it in the code dialog.
    EditCodeBlock,
    /// The caret is on a link: open it (the shell decides how; http(s)/mailto only).
    OpenLink(String),
    /// The caret is on a `[[wikilink]]`: open that note.
    OpenWikilink(String),
    /// The caret is on an image: show it in the image viewer.
    ViewImage(String),
    /// The caret is in a pipe table: edit it in the table editor.
    EditTable,
}

impl ContextAction {
    /// i18n key of the context chip / palette hint ("Edit table", "Open link"...).
    pub fn label_key(&self) -> &'static str {
        match self {
            Self::EditCodeBlock => "insert_dialogs.context.edit_code_block",
            Self::OpenLink(_) => "insert_dialogs.context.open_link",
            Self::OpenWikilink(_) => "insert_dialogs.context.open_note",
            Self::ViewImage(_) => "insert_dialogs.context.view_image",
            Self::EditTable => "insert_dialogs.context.edit_table",
        }
    }
}

/// What the pane asks of its host (the shell): things outside the editor.
/// A second event type next to [`super::EditorEvent`], so existing subscribers
/// keep compiling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorRequest {
    /// Open a link (Ctrl+. on `[text](url)` or a bare URL).
    OpenLink(String),
    /// Open the note a `[[wikilink]]` points to.
    OpenWikilink(String),
    /// Show an image (`![alt](src)`) in the image viewer.
    ViewImage(String),
    /// `/image` or `/video` was chosen: open that media dialog. The pending
    /// `/query` stays in [`EditorPane::pending_insertion`] until the dialog
    /// inserts through [`EditorPane::insert_image_markdown`] /
    /// [`EditorPane::insert_block`] or calls [`EditorPane::clear_pending_insertion`].
    OpenMediaDialog(PendingAction),
}

impl EventEmitter<EditorRequest> for EditorPane {}

/// An open caret popover.
enum Popover {
    Slash(SlashMenuModel),
    Wiki {
        from: usize,
        to: usize,
        options: Vec<WikiCompletion>,
        selected: usize,
    },
    Link {
        range: Range<usize>,
        rows: Vec<LinkSuggestion>,
        selected: usize,
    },
}

impl Popover {
    fn len(&self) -> usize {
        match self {
            Self::Slash(menu) => menu.rows().len(),
            Self::Wiki { options, .. } => options.len(),
            Self::Link { rows, .. } => rows.len(),
        }
    }

    fn step(&mut self, forward: bool) {
        match self {
            Self::Slash(menu) if forward => menu.select_next(),
            Self::Slash(menu) => menu.select_prev(),
            Self::Wiki {
                selected, options, ..
            } => *selected = wrap(*selected, options.len(), forward),
            Self::Link { selected, rows, .. } => *selected = wrap(*selected, rows.len(), forward),
        }
    }

    fn anchor(&self) -> usize {
        match self {
            Self::Slash(menu) => menu.trigger().slash,
            Self::Wiki { from, .. } => from.saturating_sub(2),
            Self::Link { range, .. } => range.start,
        }
    }
}

fn wrap(i: usize, n: usize, forward: bool) -> usize {
    match n {
        0 => 0,
        _ if forward => (i + 1) % n,
        _ => (i + n - 1) % n,
    }
}

/// The pane's insert state.
#[derive(Default)]
pub(crate) struct InsertHost {
    dialog: Option<InsertDialog>,
    dialog_sub: Option<Subscription>,
    pending: Option<PendingInsertion>,
    popover: Option<Popover>,
    /// Slash offset whose menu the user dismissed with Esc (don't reopen it).
    dismissed_slash: Option<usize>,
    notes: Rc<Vec<String>>,
    note_path: Option<String>,
    markdown: Option<bool>,
    locale: Option<String>,
}

impl EditorPane {
    // ----- host hooks (called by the shell) ----------------------------------------------------

    /// Tell the pane about the open note: its vault-relative path (relative link
    /// completion) and whether it is markdown (`false` for `.fed`: no slash menu,
    /// completions or insert dialogs).
    pub fn set_note_context(
        &mut self,
        path: Option<String>,
        markdown: bool,
        cx: &mut Context<Self>,
    ) {
        self.insert.note_path = path;
        self.insert.markdown = Some(markdown);
        if !markdown {
            self.close_insert_surfaces(cx);
        }
    }

    /// The vault's note paths (vault-relative, `/`-separated) for `[[` and Ctrl+K
    /// completion. Cheap to call on every tree refresh.
    pub fn set_vault_notes(&mut self, notes: Vec<String>) {
        self.insert.notes = Rc::new(notes);
    }

    /// The UI language code (`pt-BR`, `ja`...) for localized emoji search.
    pub fn set_ui_locale(&mut self, locale: Option<String>) {
        self.insert.locale = locale;
    }

    /// The open insert dialog, if any (the shell may render it in its own overlay
    /// layer; by default the pane renders it).
    pub fn insert_dialog(&self) -> Option<&InsertDialog> {
        self.insert.dialog.as_ref()
    }

    /// Whether a caret popover (slash menu, completion) is showing.
    pub fn is_popover_open(&self) -> bool {
        self.insert.popover.is_some()
    }

    /// The `/query` waiting for a dialog, if any.
    pub fn pending_insertion(&self) -> Option<&PendingInsertion> {
        self.insert.pending.as_ref()
    }

    /// Forget the pending `/query` (its dialog closed without inserting).
    pub fn clear_pending_insertion(&mut self) {
        self.insert.pending = None;
    }

    fn is_markdown(&self) -> bool {
        self.insert.markdown.unwrap_or(true)
    }

    fn slash_allowed(&self) -> bool {
        self.is_markdown()
            && !self.read_only
            && (!self.settings.vim || matches!(self.vim.mode(), Mode::Insert | Mode::Replace))
    }

    pub(super) fn close_insert_surfaces(&mut self, cx: &mut Context<Self>) {
        self.insert.popover = None;
        self.insert.pending = None;
        self.insert.dialog = None;
        self.insert.dialog_sub = None;
        self.insert.dismissed_slash = None;
        cx.notify();
    }

    // ----- inserting (also used by the media dialogs) ---------------------------------------------

    /// Insert block markdown at the caret, or over the pending `/query` (dropped
    /// when it went stale), or over `replace` (Update mode). One undo step.
    pub fn insert_block(
        &mut self,
        content: &str,
        replace: Option<Range<usize>>,
        cursor_offset: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (text, selection) = self.insert_text_and_selection(cx);
        let target = match (replace, self.insert.pending.take()) {
            (Some(range), _) => BlockTarget::Replace(range),
            (None, Some(pending)) => BlockTarget::Slash(pending),
            (None, None) => BlockTarget::Caret,
        };
        if let Some(edit) = block_edit(&text, selection, content, &target, cursor_offset) {
            self.apply_insert_edit(edit, window, cx);
        }
    }

    /// Insert inline text over the selection or the pending `/query`.
    pub fn insert_inline(&mut self, content: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (text, selection) = self.insert_text_and_selection(cx);
        let pending = self.insert.pending.take();
        if let Some(edit) = inline_edit(&text, selection, content, pending.as_ref()) {
            self.apply_insert_edit(edit, window, cx);
        }
    }

    /// Insert image markdown padded with single newlines (**[COMPAT]**
    /// `insertImage`), over the selection or the pending `/image` query.
    pub fn insert_image_markdown(
        &mut self,
        markdown: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (text, selection) = self.insert_text_and_selection(cx);
        let pending = self.insert.pending.take();
        if let Some(edit) =
            frankmd_core::text::pending::image_edit(&text, selection, markdown, pending.as_ref())
        {
            self.apply_insert_edit(edit, window, cx);
        }
    }

    fn insert_text_and_selection(&self, cx: &App) -> (String, Range<usize>) {
        let s = self.state.read(cx);
        (s.text().to_string(), s.selected_range())
    }

    fn apply_insert_edit(&mut self, edit: Edit, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only {
            return;
        }
        self.apply_edit(edit, window, cx);
        self.focus(window, cx);
    }

    // ----- dialogs ----------------------------------------------------------------------------

    /// Open the table editor: on the table under the caret, else a new 3×3 table.
    pub fn open_table_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_markdown() {
            return;
        }
        let (text, selection) = self.insert_text_and_selection(cx);
        let template = i18n::t(cx, "dialogs.table_editor.header_n");
        let template = if template.contains("%{n}") {
            template.to_string()
        } else {
            DEFAULT_HEADER_TEMPLATE.to_owned()
        };
        let slash = self
            .insert
            .pending
            .as_ref()
            .filter(|p| p.action == PendingAction::Table)
            .map(|p| (p.range.clone(), p.query.clone()));
        let model = TableEditorModel::from_document(
            &text,
            selection.end,
            slash.as_ref().map(|(r, q)| (r.clone(), q.as_str())),
            &template,
        );
        let dialog = insert::table::open(model, window, cx);
        self.host_dialog(InsertDialog::Table(dialog.clone()), &dialog, window, cx);
    }

    /// Open the code block dialog: on the fence under the caret, else empty.
    pub fn open_code_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (text, selection) = self.insert_text_and_selection(cx);
        let form = CodeBlockForm::from_document(&text, selection.end);
        let dialog = insert::code::open(form, window, cx);
        self.host_dialog(InsertDialog::Code(dialog.clone()), &dialog, window, cx);
    }

    /// Open the emoji picker at the caret.
    pub fn open_emoji_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_markdown() {
            return;
        }
        let locale = self.insert.locale.clone();
        let popover = insert::emoji::open(locale.as_deref(), window, cx);
        self.host_dialog(InsertDialog::Emoji(popover.clone()), &popover, window, cx);
    }

    fn host_dialog<V: EventEmitter<InsertDialogEvent>>(
        &mut self,
        dialog: InsertDialog,
        entity: &Entity<V>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.insert.popover = None;
        let sub = cx.subscribe_in(
            entity,
            window,
            |pane, _, event: &InsertDialogEvent, window, cx| {
                pane.on_dialog_event(event.clone(), window, cx);
            },
        );
        self.insert.dialog = Some(dialog);
        self.insert.dialog_sub = Some(sub);
        cx.notify();
    }

    fn on_dialog_event(
        &mut self,
        event: InsertDialogEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.insert.dialog = None;
        self.insert.dialog_sub = None;
        match event {
            InsertDialogEvent::Block {
                content,
                replace,
                cursor_offset,
            } => {
                self.insert_block(&content, replace, cursor_offset, window, cx);
            }
            InsertDialogEvent::Inline { content } => self.insert_inline(&content, window, cx),
            InsertDialogEvent::Cancel => {
                self.insert.pending = None;
                self.focus(window, cx);
            }
        }
        cx.notify();
    }

    // ----- context action (Ctrl+.) ---------------------------------------------------------------

    /// What Ctrl+. would do at the caret (for the context chip and palette).
    pub fn context_action(&self, cx: &App) -> Option<ContextAction> {
        let s = self.state.read(cx);
        let caret = s.selected_range().end;
        let text = s.text().to_string();
        if find_code_block_at_position(&text, caret).is_some() {
            return Some(ContextAction::EditCodeBlock);
        }
        if let Some(link) = link_at_caret(&text, caret) {
            return Some(match link {
                CaretLink::Url(url) => ContextAction::OpenLink(url),
                CaretLink::Wiki(target) => ContextAction::OpenWikilink(target),
                CaretLink::Image(src) => ContextAction::ViewImage(src),
            });
        }
        (self.is_markdown() && find_table_at_position(&text, caret).is_some())
            .then_some(ContextAction::EditTable)
    }

    /// Run the context action: the table / code dialogs open here; links and
    /// images go to the shell as an [`EditorRequest`]. Returns what ran.
    pub fn run_context_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<ContextAction> {
        let action = self.context_action(cx)?;
        match &action {
            ContextAction::EditCodeBlock => self.open_code_dialog(window, cx),
            ContextAction::EditTable => self.open_table_editor(window, cx),
            ContextAction::OpenLink(url) => cx.emit(EditorRequest::OpenLink(url.clone())),
            ContextAction::OpenWikilink(target) => {
                cx.emit(EditorRequest::OpenWikilink(target.clone()))
            }
            ContextAction::ViewImage(src) => cx.emit(EditorRequest::ViewImage(src.clone())),
        }
        Some(action)
    }

    // ----- popovers ---------------------------------------------------------------------------------

    /// After Ctrl+K: offer link completion inside the new `(url)`.
    pub(super) fn open_link_completion(&mut self, cx: &mut Context<Self>) {
        if !self.is_markdown() {
            return;
        }
        let (text, selection) = self.insert_text_and_selection(cx);
        let Some(range) = link_destination_at(&text, selection.end) else {
            return;
        };
        self.show_link_rows(&text, range, cx);
    }

    fn show_link_rows(&mut self, text: &str, range: Range<usize>, cx: &mut Context<Self>) {
        let query = &text[range.clone()];
        let clipboard = cx.read_from_clipboard().and_then(|c| c.text());
        let rows = link_suggestions(
            query,
            clipboard.as_deref(),
            &self.insert.notes,
            self.insert.note_path.as_deref(),
        );
        self.insert.popover = (!rows.is_empty()).then_some(Popover::Link {
            range,
            rows,
            selected: 0,
        });
        cx.notify();
    }

    /// Text changed: open, refresh or close the caret popovers.
    pub(super) fn refresh_popovers_after_edit(&mut self, cx: &mut Context<Self>) {
        if self.insert.dialog.is_some() || !self.slash_allowed() {
            self.insert.popover = None;
            return;
        }
        let (caret, line_before) = {
            let s = self.state.read(cx);
            let caret = s.selected_range().end;
            let rope = s.text();
            let row = rope.offset_to_point(caret).row;
            let start = rope.line_start_offset(row);
            (caret, rope.slice(start..caret).to_string())
        };
        // Cheap line checks first: the full text is copied only when a trigger is possible.
        let maybe_slash = line_before.contains('/');
        let maybe_wiki = line_before
            .rfind("[[")
            .is_some_and(|i| !line_before[i..].contains("]]"));
        let in_link = matches!(self.insert.popover, Some(Popover::Link { .. }));
        if !maybe_slash && !maybe_wiki && !in_link {
            self.insert.popover = None;
            return;
        }
        let text = self.insert_text_and_selection(cx).0;
        if in_link {
            match link_destination_at(&text, caret) {
                Some(range) => self.show_link_rows(&text, range, cx),
                None => self.insert.popover = None,
            }
            return;
        }
        if maybe_wiki && let Some(found) = wiki_complete(&text, caret, &self.wiki_files()) {
            let mut options = found.options;
            options.sort_by_key(|o| std::cmp::Reverse(o.boost));
            options.truncate(MAX_COMPLETIONS);
            self.insert.popover = Some(Popover::Wiki {
                from: found.from,
                to: found.to,
                options,
                selected: 0,
            });
            cx.notify();
            return;
        }
        if maybe_slash {
            if let Some(Popover::Slash(menu)) = self.insert.popover.as_mut() {
                if menu.update(&text, caret) && !menu.rows().is_empty() {
                    cx.notify();
                    return;
                }
            } else if let Some(trigger) = slash_trigger(&text, caret)
                && self.insert.dismissed_slash != Some(trigger.slash)
            {
                let menu = SlashMenuModel::open(trigger, |c| slash_label(c, cx));
                if !menu.rows().is_empty() {
                    self.insert.popover = Some(Popover::Slash(menu));
                    cx.notify();
                    return;
                }
            }
        }
        self.insert.popover = None;
        cx.notify();
    }

    /// Caret moved: close the popover when the caret left its line or moved before
    /// its start (an edit refreshes it right after, see [`Self::refresh_popovers_after_edit`]).
    pub(super) fn refresh_popovers_after_move(&mut self, cx: &mut Context<Self>) {
        let Some(anchor) = self.insert.popover.as_ref().map(Popover::anchor) else {
            return;
        };
        let keep = {
            let s = self.state.read(cx);
            let caret = s.selected_range().end;
            let rope = s.text();
            caret >= anchor
                && anchor <= rope.len()
                && rope.offset_to_point(caret).row == rope.offset_to_point(anchor).row
        };
        if !keep {
            self.insert.popover = None;
            cx.notify();
        }
    }

    fn wiki_files(&self) -> Vec<WikiFile> {
        self.insert
            .notes
            .iter()
            .map(|p| WikiFile {
                path: Some(p.clone()),
                name: None,
            })
            .collect()
    }

    /// Keys for an open popover: ↑/↓ (and Ctrl+N/P) move, Enter/Tab accept, Esc
    /// dismisses (the typed text stays). `None` = not a popover key.
    pub(super) fn popover_key(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        let popover = self.insert.popover.as_mut()?;
        match key {
            "down" | "ctrl-n" => popover.step(true),
            "up" | "ctrl-p" => popover.step(false),
            "enter" | "tab" => {
                if popover.len() == 0 {
                    return None;
                }
                self.accept_popover(None, window, cx);
            }
            "escape" => {
                if let Some(Popover::Slash(menu)) = &self.insert.popover {
                    self.insert.dismissed_slash = Some(menu.trigger().slash);
                }
                self.insert.popover = None;
            }
            _ => return None,
        }
        cx.notify();
        Some(true)
    }

    fn accept_popover(
        &mut self,
        index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(popover) = self.insert.popover.take() else {
            return;
        };
        let text = self.insert_text_and_selection(cx).0;
        match popover {
            Popover::Slash(menu) => {
                let outcome = match index {
                    Some(i) => menu.accept_at(i, &text),
                    None => menu.accept(&text),
                };
                match outcome {
                    Some(SlashOutcome::Edit(edit)) => self.apply_insert_edit(edit, window, cx),
                    Some(SlashOutcome::OpenDialog {
                        command,
                        range,
                        query,
                    }) => self.open_slash_dialog(command, range, query, window, cx),
                    None => {}
                }
            }
            Popover::Wiki {
                from,
                to,
                options,
                selected,
            } => {
                if let Some(option) = options.get(index.unwrap_or(selected)) {
                    let caret = option.caret_after(from);
                    // The completion ends with `]]`; swallow a pair the editor auto-closed.
                    let to = if text[to..].starts_with("]]") {
                        to + 2
                    } else {
                        to
                    };
                    self.apply_insert_edit(
                        Edit {
                            range: from..to,
                            text: option.insert.clone(),
                            selection: caret..caret,
                        },
                        window,
                        cx,
                    );
                }
            }
            Popover::Link {
                range,
                rows,
                selected,
            } => {
                if let Some(row) = rows.get(index.unwrap_or(selected)) {
                    let caret = range.start + row.insert.len();
                    // Leave the caret after the `)` so typing continues after the link.
                    let after = if text[range.end..].starts_with(')') {
                        caret + 1
                    } else {
                        caret
                    };
                    self.apply_insert_edit(
                        Edit {
                            range,
                            text: row.insert.clone(),
                            selection: after..after,
                        },
                        window,
                        cx,
                    );
                }
            }
        }
        cx.notify();
    }

    fn open_slash_dialog(
        &mut self,
        command: SlashCommand,
        range: Range<usize>,
        query: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let action = match command {
            SlashCommand::Table => PendingAction::Table,
            SlashCommand::Image => PendingAction::Image,
            SlashCommand::Video => PendingAction::Video,
            SlashCommand::Emoji => PendingAction::Emoji,
            _ => return,
        };
        self.insert.pending = Some(PendingInsertion {
            action,
            range,
            query,
        });
        match action {
            PendingAction::Table => self.open_table_editor(window, cx),
            PendingAction::Emoji => self.open_emoji_picker(window, cx),
            PendingAction::Image | PendingAction::Video => {
                cx.emit(EditorRequest::OpenMediaDialog(action))
            }
        }
    }

    // ----- rendering --------------------------------------------------------------------------------

    /// The popover and dialog layers (deferred, so they paint over the pane).
    pub(super) fn render_insert_layers(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut layers = Vec::new();
        if let Some(popover) = self.render_popover(cx) {
            layers.push(popover);
        }
        if let Some(dialog) = &self.insert.dialog {
            if dialog.is_modal() {
                layers.push(insert::modal_scrim(dialog.view(), window, cx));
            } else {
                let anchor = self.caret_anchor(cx);
                layers.push(
                    deferred(
                        anchored()
                            .position(anchor)
                            .snap_to_window_with_margin(px(8.))
                            .child(dialog.view()),
                    )
                    .with_priority(1)
                    .into_any_element(),
                );
            }
        }
        layers
    }

    fn caret_anchor(&self, cx: &App) -> Point<Pixels> {
        let state = self.state.read(cx);
        let caret = state.selected_range().end;
        state
            .range_to_bounds(&(caret..caret))
            .map(|b| b.bottom_left())
            .unwrap_or_else(|| state.input_bounds().center())
    }

    fn render_popover(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let popover = self.insert.popover.as_ref()?;
        let anchor = {
            let state = self.state.read(cx);
            let at = popover.anchor();
            state
                .range_to_bounds(&(at..at))
                .map(|b| b.bottom_left())
                .unwrap_or_else(|| state.input_bounds().center())
        };
        let theme = cx.theme();
        let (id, rows, selected): (&str, Vec<PopoverRow>, usize) = match popover {
            Popover::Slash(menu) => (
                "slash-menu",
                menu.rows()
                    .iter()
                    .map(|r| (Some(r.icon), r.label.clone().into(), None))
                    .collect(),
                menu.selected(),
            ),
            Popover::Wiki {
                options, selected, ..
            } => (
                "wikilink-completion",
                options
                    .iter()
                    .map(|o| {
                        (
                            None,
                            o.label.clone().into(),
                            o.detail.clone().map(Into::into),
                        )
                    })
                    .collect(),
                *selected,
            ),
            Popover::Link { rows, selected, .. } => (
                "link-completion",
                rows.iter()
                    .map(|r| {
                        (
                            None,
                            r.label.clone().into(),
                            r.detail.clone().map(Into::into),
                        )
                    })
                    .collect(),
                *selected,
            ),
        };
        let row_height = px(ROW_HEIGHT_PX);
        let items = rows
            .into_iter()
            .enumerate()
            .map(|(i, (icon, label, detail))| {
                let is_selected = i == selected;
                h_flex()
                    .id(SharedString::from(format!("{id}-row-{i}")))
                    .test_support()
                    .h(row_height)
                    .flex_none()
                    .gap_2()
                    .px_2()
                    .items_center()
                    .text_sm()
                    .when(is_selected, |el| {
                        el.bg(theme.accent).text_color(theme.accent_foreground)
                    })
                    .when_some(icon, |el, icon| {
                        el.child(
                            svg()
                                .path(SharedString::from(format!("icons/ph/{icon}.svg")))
                                .size(px(14.))
                                .flex_none()
                                .text_color(theme.muted_foreground),
                        )
                    })
                    .child(div().flex_1().min_w_0().overflow_hidden().child(label))
                    .when_some(detail, |el, d| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .overflow_hidden()
                                .child(d),
                        )
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.accept_popover(Some(i), window, cx)
                    }))
            });
        let list = v_flex()
            .id(id)
            .test_support()
            .w(px(MENU_WIDTH_PX))
            .max_h(row_height * 8.)
            .overflow_y_scroll()
            .py_1()
            .bg(theme.popover)
            .text_color(theme.popover_foreground)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .shadow_md()
            .children(items.collect::<Vec<_>>());
        Some(
            deferred(
                anchored()
                    .position(anchor)
                    .snap_to_window_with_margin(px(8.))
                    .child(list),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }
}

/// Localized slash label, falling back to the original English label.
fn slash_label(command: SlashCommand, cx: &App) -> String {
    let label = i18n::t(cx, command.label_key());
    if label.as_ref() == command.label_key() || label.is_empty() {
        command.fallback_label().to_owned()
    } else {
        label.to_string()
    }
}
