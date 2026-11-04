//! Code block modal (ui-spec §5.9) over
//! [`frankmd_workspace::code_block::CodeBlockForm`].
//!
//! The language field shows FrankMD's suggestion list (Tab takes the first,
//! ↓/↑ move through it, Enter picks the highlighted one, Esc hides it before
//! closing the dialog). The code area is a gpui-kit code editor highlighted for
//! the chosen language as you type; Tab / Shift+Tab indent with two spaces or a
//! tab ("Indent with tabs"). An unknown language shows an inline warning row with
//! "Use <suggestion>" / "Keep <typed>" instead of FrankMD's confirm prompt.
//! Ctrl+Enter inserts (or updates the fence under the caret).

use frankmd_workspace::code_block::{CodeBlockForm, CodeBlockMode, CodeBlockOutcome};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Editor, EditorState, Input, InputEvent, InputState, TabSize};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{InsertDialogEvent, key_text, modal_frame};
use crate::ui::i18n;

/// The code block dialog view.
pub struct CodeDialog {
    form: CodeBlockForm,
    language_input: Entity<InputState>,
    code: Entity<EditorState>,
    /// Language the code editor is highlighted for.
    code_language: String,
    highlighted: usize,
    focus: FocusHandle,
    subscriptions: Vec<Subscription>,
    _interceptor: Subscription,
}

impl EventEmitter<InsertDialogEvent> for CodeDialog {}

impl Focusable for CodeDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Open the dialog on `form` (empty, or the fence under the caret).
pub fn open(form: CodeBlockForm, window: &mut Window, cx: &mut App) -> Entity<CodeDialog> {
    let dialog = cx.new(|cx| CodeDialog::new(form, window, cx));
    dialog.update(cx, |d, cx| d.focus(window, cx));
    dialog
}

/// Tree-sitter grammar name for a fence language (gpui-kit falls back to plain
/// text for names it doesn't know).
fn grammar_for(language: &str) -> String {
    match language.to_lowercase().as_str() {
        "shell" | "sh" | "zsh" => "bash".to_owned(),
        "plaintext" | "" => "text".to_owned(),
        other => other.to_owned(),
    }
}

impl CodeDialog {
    fn new(form: CodeBlockForm, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let placeholder = i18n::t(cx, "dialogs.code.language_placeholder");
        let language = form.language().to_owned();
        let language_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder).default_value(language.clone()));
        let code = Self::make_editor(&grammar_for(&language), form.content(), form.use_tabs(), window, cx);
        let weak = cx.entity().downgrade();
        let interceptor = cx.intercept_keystrokes(move |event, window, cx| {
            let Some(dialog) = weak.upgrade() else { return };
            if dialog.update(cx, |d, cx| d.on_key(&event.keystroke, window, cx)) {
                cx.stop_propagation();
            }
        });
        let mut this = Self {
            form,
            language_input,
            code,
            code_language: grammar_for(&language),
            highlighted: 0,
            focus: cx.focus_handle(),
            subscriptions: Vec::new(),
            _interceptor: interceptor,
        };
        this.subscribe(window, cx);
        this
    }

    fn make_editor(
        grammar: &str,
        content: &str,
        use_tabs: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<EditorState> {
        let placeholder = i18n::t(cx, "dialogs.code.code_placeholder");
        let content = content.to_owned();
        let grammar = grammar.to_owned();
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(grammar)
                .line_number(false)
                .searchable(false)
                .soft_wrap(false)
                .tab_size(TabSize { tab_size: 2, hard_tabs: use_tabs })
                .placeholder(placeholder)
        });
        editor.update(cx, |s, cx| s.set_value(content, window, cx));
        editor
    }

    fn subscribe(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.subscriptions = vec![
            cx.subscribe_in(&self.language_input, window, |this, input, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    let value = input.read(cx).value();
                    this.form.set_language(&value);
                    this.highlighted = 0;
                    cx.notify();
                }
            }),
            cx.subscribe_in(&self.code, window, |this, code, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    let value = code.read(cx).text().to_string();
                    this.form.set_content(&value);
                }
            }),
        ];
    }

    /// The form (tests, host).
    pub fn form(&self) -> &CodeBlockForm {
        &self.form
    }

    /// Focus the language field (on open).
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.language_input.update(cx, |s, cx| s.focus(window, cx));
    }

    /// Re-highlight the code area when the language settles on a different grammar.
    fn sync_grammar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let grammar = grammar_for(self.form.language().trim());
        if grammar == self.code_language {
            return;
        }
        self.code_language = grammar.clone();
        let focused = self.code.read(cx).focus_handle(cx).is_focused(window);
        self.code = Self::make_editor(&grammar, self.form.content(), self.form.use_tabs(), window, cx);
        self.subscribe(window, cx);
        if focused {
            self.code.update(cx, |s, cx| s.focus(window, cx));
        }
    }

    fn set_language_text(&mut self, language: &str, window: &mut Window, cx: &mut Context<Self>) {
        let language = language.to_owned();
        self.language_input.update(cx, |s, cx| s.set_value(language, window, cx));
    }

    fn pick(&mut self, language: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.form.select_suggestion(language);
        let picked = self.form.language().to_owned();
        self.set_language_text(&picked, window, cx);
        self.form.hide_suggestions();
        self.sync_grammar(window, cx);
        self.code.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    fn set_use_tabs(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.form.set_use_tabs(on);
        self.code.update(cx, |s, cx| s.set_tab_size(TabSize { tab_size: 2, hard_tabs: on }, cx));
        let _ = window;
        cx.notify();
    }

    /// Insert / Update (Ctrl+Enter). An unknown language shows the warning row instead.
    pub fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let outcome = self.form.confirm();
        self.finish(Some(outcome), window, cx);
    }

    fn finish(&mut self, outcome: Option<CodeBlockOutcome>, window: &mut Window, cx: &mut Context<Self>) {
        match outcome {
            Some(CodeBlockOutcome::Insert { markdown, cursor_offset, mode }) => {
                cx.emit(InsertDialogEvent::Block {
                    content: markdown,
                    replace: match mode {
                        CodeBlockMode::Update(range) => Some(range),
                        CodeBlockMode::Insert => None,
                    },
                    cursor_offset: Some(cursor_offset),
                });
            }
            Some(CodeBlockOutcome::UnknownLanguage { .. }) | None => {
                let language = self.form.language().to_owned();
                self.set_language_text(&language, window, cx);
                cx.notify();
            }
        }
    }

    /// Cancel (Esc with no suggestions, Cancel button).
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(InsertDialogEvent::Cancel);
    }

    fn composing(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.language_input.update(cx, |s, cx| s.marked_text_range(window, cx)).is_some()
            || self.code.update(cx, |s, cx| s.marked_text_range(window, cx)).is_some()
    }

    fn on_key(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.focus.contains_focused(window, cx) || self.composing(window, cx) {
            return false;
        }
        let in_language = self.language_input.read(cx).focus_handle(cx).is_focused(window);
        let key = key_text(keystroke);
        match key.as_str() {
            "ctrl-enter" => self.confirm(window, cx),
            "escape" if self.form.suggestions_visible() => {
                self.form.hide_suggestions();
                cx.notify();
            }
            "escape" => self.cancel(cx),
            "tab" if in_language => {
                if self.form.key_tab() {
                    let language = self.form.language().to_owned();
                    self.set_language_text(&language, window, cx);
                    self.sync_grammar(window, cx);
                } else {
                    self.sync_grammar(window, cx);
                    self.code.update(cx, |s, cx| s.focus(window, cx));
                }
                cx.notify();
            }
            "down" | "up" if in_language && self.form.suggestions_visible() => {
                let n = self.form.suggestions().len().max(1);
                self.highlighted = if key == "down" {
                    (self.highlighted + 1) % n
                } else {
                    (self.highlighted + n - 1) % n
                };
                cx.notify();
            }
            "enter" if in_language && self.form.suggestions_visible() => {
                if let Some(lang) = self.form.suggestions().get(self.highlighted).copied() {
                    self.pick(lang, window, cx);
                }
            }
            "enter" if in_language => {
                self.sync_grammar(window, cx);
                self.code.update(cx, |s, cx| s.focus(window, cx));
            }
            _ => return false,
        }
        true
    }

    // ----- rendering ----------------------------------------------------------------

    fn render_suggestions(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.form.suggestions_visible() {
            return None;
        }
        let theme = cx.theme();
        let rows = self.form.suggestions().iter().enumerate().map(|(i, &lang)| {
            let selected = i == self.highlighted;
            div()
                .id(SharedString::from(format!("code-lang-{lang}")))
                .test_support()
                .px_2()
                .py_1()
                .text_sm()
                .font_family(theme.mono_font_family.clone())
                .when(selected, |el| el.bg(theme.accent).text_color(theme.accent_foreground))
                .hover(|s| s.bg(theme.accent))
                .on_click(cx.listener(move |this, _, window, cx| this.pick(lang, window, cx)))
                .child(lang)
        });
        Some(
            v_flex()
                .id("code-lang-suggestions")
                .test_support()
                .max_h(px(192.))
                .overflow_y_scroll()
                .border_1()
                .border_color(theme.border)
                .rounded(theme.radius)
                .bg(theme.popover)
                .children(rows.collect::<Vec<_>>())
                .into_any_element(),
        )
    }

    fn render_warning(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Some(CodeBlockOutcome::UnknownLanguage { typed, did_you_mean }) = self.form.warning() else {
            return None;
        };
        let theme = cx.theme();
        let message = match did_you_mean {
            Some(s) => i18n::t_args(
                cx,
                "insert_dialogs.code.unknown_language_suggest",
                &[("language", typed), ("suggestion", s)],
            ),
            None => i18n::t_args(cx, "insert_dialogs.code.unknown_language", &[("language", typed)]),
        };
        let keep = i18n::t_args(cx, "insert_dialogs.code.keep", &[("language", typed)]);
        let row = h_flex()
            .id("code-unknown-language")
            .test_support()
            .gap_2()
            .flex_wrap()
            .items_center()
            .px_2()
            .py_1()
            .rounded(theme.radius)
            .bg(theme.warning.opacity(0.15))
            .child(div().flex_1().text_sm().child(message))
            .when_some(*did_you_mean, |el, s| {
                el.child(
                    Button::new("code-use-suggested")
                        .label(i18n::t_args(cx, "insert_dialogs.code.use", &[("language", s)]))
                        .xsmall()
                        .primary()
                        .on_click(cx.listener(|this, _, window, cx| {
                            let outcome = this.form.use_suggested();
                            this.finish(outcome, window, cx);
                        })),
                )
            })
            .child(
                Button::new("code-keep-typed")
                    .label(keep)
                    .xsmall()
                    .on_click(cx.listener(|this, _, window, cx| {
                        let outcome = this.form.keep_typed();
                        this.finish(outcome, window, cx);
                    })),
            );
        Some(row.into_any_element())
    }
}

impl Render for CodeDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let mono = theme.mono_font_family.clone();
        let border = theme.border;
        let radius = theme.radius;
        let primary = i18n::t(cx, self.form.primary_label_key());
        let language_field = v_flex()
            .gap_1()
            .child(div().text_xs().text_color(muted).child(i18n::t(cx, "dialogs.code.language_label")))
            .child(
                div()
                    .id("code-language")
                    .test_support()
                    .font_family(mono.clone())
                    .child(Input::new(&self.language_input).small()),
            )
            .children(self.render_suggestions(cx));
        let use_tabs = self.form.use_tabs();
        let code_header = h_flex()
            .justify_between()
            .child(div().text_xs().text_color(muted).child(i18n::t(cx, "dialogs.code.code_label")))
            .child(
                Checkbox::new("code-use-tabs")
                    .label(i18n::t(cx, "insert_dialogs.code.indent_with_tabs"))
                    .checked(use_tabs)
                    .on_click(cx.listener(|this, checked: &bool, window, cx| {
                        this.set_use_tabs(*checked, window, cx)
                    })),
            );
        let code_area = div()
            .id("code-content")
            .test_support()
            .h(px(260.))
            .border_1()
            .border_color(border)
            .rounded(radius)
            .p_1()
            .child(Editor::new(&self.code).h(relative(1.)).font_family(mono).text_size(px(13.)));
        let body = v_flex()
            .gap_3()
            .child(language_field)
            .children(self.render_warning(cx))
            .child(v_flex().gap_1().child(code_header).child(code_area));
        let footer = h_flex()
            .justify_end()
            .gap_2()
            .child(
                Button::new("code-cancel")
                    .label(i18n::t(cx, "common.cancel"))
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
            )
            .child(
                Button::new("code-insert")
                    .label(format!("{primary}  Ctrl+↵"))
                    .small()
                    .primary()
                    .on_click(cx.listener(|this, _, window, cx| this.confirm(window, cx))),
            );
        let title = i18n::t(cx, "dialogs.code.title");
        modal_frame("dialog-code-block", &self.focus, px(720.), title, body, footer, None, cx)
    }
}
