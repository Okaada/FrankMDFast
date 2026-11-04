//! Emoji / emoticon / icon picker popover (ui-spec §5.5, FE-405..408) over
//! [`frankmd_core::emoji::picker::EmojiPicker`].
//!
//! 360×320 at the caret: search on top, the tab strip (Ctrl+Tab cycles; ←/→ when
//! the strip has focus), the grid (10 / 5 / 8 columns, arrows wrap, Enter or a
//! click inserts) and a footer with the large glyph and its code. Inserts exactly
//! what FrankMD did: `:shortcode:`, `:ph-<name>:` or the raw kaomoji. The new
//! "Frequently used" row is kept for the session (a GPUI global).

use frankmd_core::emoji::picker::{
    CellGlyph, EmojiPicker, GridMove, PickerCell, PickerTab, TabKey,
};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{InsertDialogEvent, key_text};
use crate::ui::i18n;

/// Popover size (ui-spec §5.5).
pub const POPOVER_SIZE: Size<Pixels> = Size {
    width: px(360.),
    height: px(320.),
};

/// Emoji shortcodes used this session, most recent first.
#[derive(Default)]
struct FrequentEmoji(Vec<String>);

impl Global for FrequentEmoji {}

/// The picker popover view.
pub struct EmojiPopover {
    picker: EmojiPicker,
    search: Entity<InputState>,
    focus: FocusHandle,
    tabs_focus: FocusHandle,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<InsertDialogEvent> for EmojiPopover {}

impl Focusable for EmojiPopover {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Open the picker for the UI `locale` (localized emoji search; FrankMD maps
/// he → en and pt-BR/pt-PT → pt).
pub fn open(locale: Option<&str>, window: &mut Window, cx: &mut App) -> Entity<EmojiPopover> {
    let frequent = cx
        .try_global::<FrequentEmoji>()
        .map(|f| f.0.clone())
        .unwrap_or_default();
    let mut picker = EmojiPicker::new(locale);
    picker.set_frequent(frequent);
    picker.open();
    let popover = cx.new(|cx| EmojiPopover::new(picker, window, cx));
    popover.update(cx, |p, cx| p.focus(window, cx));
    popover
}

impl EmojiPopover {
    fn new(picker: EmojiPicker, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let placeholder = i18n::t(cx, "dialogs.emoji_picker.search_placeholder");
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let weak = cx.entity().downgrade();
        let interceptor = cx.intercept_keystrokes(move |event, window, cx| {
            let Some(popover) = weak.upgrade() else {
                return;
            };
            if popover.update(cx, |p, cx| p.on_key(&event.keystroke, window, cx)) {
                cx.stop_propagation();
            }
        });
        let search_sub = cx.subscribe_in(&search, window, |this, input, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::Change) {
                let query = input.read(cx).value();
                this.picker.set_query(&query);
                this.scroll.scroll_to_item(0);
                cx.notify();
            }
        });
        Self {
            picker,
            search,
            focus: cx.focus_handle(),
            tabs_focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            _subscriptions: vec![interceptor, search_sub],
        }
    }

    /// The picker state (tests, host).
    pub fn picker(&self) -> &EmojiPicker {
        &self.picker
    }

    /// Focus the search field.
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |s, cx| s.focus(window, cx));
    }

    /// Insert the item at `index` (or the selection) and close.
    pub fn activate(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        let insert = match index {
            Some(i) => self.picker.activate_at(i),
            None => self.picker.activate(),
        };
        let Some(insert) = insert else { return };
        self.picker.record_use(&insert);
        let frequent = self.picker.frequent().to_vec();
        cx.set_global(FrequentEmoji(frequent));
        cx.emit(InsertDialogEvent::Inline {
            content: insert.text,
        });
    }

    /// Close without inserting.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(InsertDialogEvent::Cancel);
    }

    fn switch_tab(&mut self, tab: PickerTab, cx: &mut Context<Self>) {
        if self.picker.switch_tab(tab) {
            self.scroll.scroll_to_item(0);
            cx.notify();
        }
    }

    fn scroll_to_selection(&mut self) {
        if let Some(index) = self.picker.take_scroll_request() {
            self.scroll
                .scroll_to_item(index / self.picker.columns().max(1));
        }
    }

    fn on_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.focus.contains_focused(window, cx)
            || self
                .search
                .update(cx, |s, cx| s.marked_text_range(window, cx))
                .is_some()
        {
            return false;
        }
        let on_tabs = self.tabs_focus.is_focused(window);
        match key_text(keystroke).as_str() {
            "escape" => self.cancel(cx),
            "ctrl-tab" => {
                self.picker.next_tab(true);
                self.scroll.scroll_to_item(0);
            }
            "ctrl-shift-tab" => {
                self.picker.next_tab(false);
                self.scroll.scroll_to_item(0);
            }
            "left" if on_tabs => {
                self.picker.tab_strip_key(TabKey::Left);
            }
            "right" if on_tabs => {
                self.picker.tab_strip_key(TabKey::Right);
            }
            "left" => self.picker.move_selection(GridMove::Left),
            "right" => self.picker.move_selection(GridMove::Right),
            "up" => self.picker.move_selection(GridMove::Up),
            "down" => self.picker.move_selection(GridMove::Down),
            "enter" => {
                self.activate(None, cx);
                return true;
            }
            _ => return false,
        }
        self.scroll_to_selection();
        cx.notify();
        true
    }

    fn render_glyph(glyph: &CellGlyph, code: &str, size: Pixels, color: Hsla) -> AnyElement {
        match glyph {
            CellGlyph::Text(text) => div().child(SharedString::from(*text)).into_any_element(),
            CellGlyph::Icon(_) => {
                let name = code.trim_matches(':').trim_start_matches("ph-");
                svg()
                    .path(SharedString::from(format!("icons/ph/{name}.svg")))
                    .size(size)
                    .text_color(color)
                    .into_any_element()
            }
        }
    }

    fn render_cell(&self, cell: PickerCell, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let tab = self.picker.tab();
        let index = cell.index;
        let base = div()
            .id(SharedString::from(format!("emoji-cell-{index}")))
            .test_support()
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme.radius)
            .cursor_pointer()
            .when(cell.selected, |el| {
                el.bg(theme.accent).border_2().border_color(theme.ring)
            })
            .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, _, cx| {
                if this.picker.hover(index) {
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, _, _, cx| this.activate(Some(index), cx)));
        match tab {
            PickerTab::Emoji => base
                .size(px(32.))
                .text_size(px(20.))
                .child(Self::render_glyph(
                    &cell.glyph,
                    &cell.code,
                    px(20.),
                    theme.foreground,
                ))
                .into_any_element(),
            PickerTab::Icons => base
                .size(px(36.))
                .child(Self::render_glyph(
                    &cell.glyph,
                    &cell.code,
                    px(20.),
                    theme.foreground,
                ))
                .into_any_element(),
            PickerTab::Emoticons => base
                .flex_1()
                .min_w_0()
                .h(px(32.))
                .px_1()
                .text_xs()
                .overflow_hidden()
                .child(Self::render_glyph(
                    &cell.glyph,
                    &cell.code,
                    px(16.),
                    theme.foreground,
                ))
                .into_any_element(),
        }
    }

    fn render_grid(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        if self.picker.is_empty() {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(i18n::t(cx, "status.no_matches"))
                .into_any_element();
        }
        let columns = self.picker.columns().max(1);
        let mut cells = self.picker.cells().into_iter().peekable();
        let mut rows = Vec::new();
        while cells.peek().is_some() {
            let row: Vec<AnyElement> = cells
                .by_ref()
                .take(columns)
                .map(|c| self.render_cell(c, cx))
                .collect();
            rows.push(h_flex().gap_1().children(row).into_any_element());
        }
        v_flex()
            .id("emoji-grid")
            .test_support()
            .flex_1()
            .min_h_0()
            .gap_1()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .children(rows)
            .into_any_element()
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let active = self.picker.tab();
        let tabs = PickerTab::ALL.iter().map(|&tab| {
            let selected = tab == active;
            div()
                .id(SharedString::from(
                    format!("emoji-tab-{tab:?}").to_lowercase(),
                ))
                .test_support()
                .px_2()
                .py_0p5()
                .text_xs()
                .rounded(theme.radius)
                .cursor_pointer()
                .when(selected, |el| {
                    el.bg(theme.accent).text_color(theme.accent_foreground)
                })
                .when(!selected, |el| el.text_color(theme.muted_foreground))
                .on_click(cx.listener(move |this, _, _, cx| this.switch_tab(tab, cx)))
                .child(i18n::t(cx, tab.label_key()))
        });
        h_flex()
            .id("emoji-tabs")
            .test_support()
            .track_focus(&self.tabs_focus)
            .gap_1()
            .on_scroll(cx.listener(|this, ev: &ScrollEvent, _, cx| {
                let dy = ev.delta.pixel_delta(px(16.)).y;
                if dy != px(0.) {
                    this.picker.next_tab(dy < px(0.));
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .children(tabs.collect::<Vec<_>>())
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let footer = self.picker.footer();
        h_flex()
            .id("emoji-footer")
            .test_support()
            .h(px(44.))
            .flex_none()
            .gap_3()
            .px_2()
            .items_center()
            .bg(theme.secondary)
            .rounded(theme.radius)
            .when_some(footer, |el, f| {
                el.child(div().text_size(px(26.)).child(Self::render_glyph(
                    &f.glyph,
                    &f.code,
                    px(28.),
                    theme.foreground,
                )))
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.mono_font_family.clone())
                        .text_color(theme.muted_foreground)
                        .child(f.code),
                )
            })
    }
}

impl Render for EmojiPopover {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .id("emoji-picker")
            .test_support()
            .track_focus(&self.focus)
            .w(POPOVER_SIZE.width)
            .h(POPOVER_SIZE.height)
            .p_2()
            .gap_2()
            .bg(theme.popover)
            .text_color(theme.popover_foreground)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.cancel(cx)))
            .child(Input::new(&self.search).small())
            .child(self.render_tabs(cx))
            .child(self.render_grid(cx))
            .child(self.render_footer(cx))
    }
}
