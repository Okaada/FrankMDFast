//! Table editor modal (ui-spec §5.8) over
//! [`frankmd_workspace::table_editor::TableEditorModel`].
//!
//! Keys while the dialog has focus: Tab / Shift+Tab next / previous cell (Tab in
//! the last cell adds a row), Enter moves down, Alt+↑/↓ move the row, Alt+←/→
//! move the column, Ctrl+Shift+Backspace deletes the row, Ctrl+Alt+Backspace
//! deletes the column, Ctrl+Enter inserts/updates, Esc closes the cell menu or
//! cancels. Right-click on a cell opens the cell menu (FE-386) with the same
//! actions and their shortcuts.

use crate::scale::px;
use frankmd_workspace::table_editor::{Alignment, CellAction, TableEditorModel, TableMode};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{InsertDialogEvent, key_text, modal_frame};
use crate::ui::i18n;

/// Widest the dialog grows (ui-spec §5.8: up to 960 px).
const MAX_WIDTH: f32 = 960.;
/// Minimum cell width (FrankMD: 100 px).
const CELL_WIDTH: f32 = 120.;

/// The open cell menu.
#[derive(Debug, Clone, Copy)]
struct CellMenu {
    row: usize,
    col: usize,
    position: Point<Pixels>,
}

/// The table editor dialog view.
pub struct TableDialog {
    model: TableEditorModel,
    cells: Vec<Vec<Entity<InputState>>>,
    focus: FocusHandle,
    current: (usize, usize),
    menu: Option<CellMenu>,
    cell_subscriptions: Vec<Subscription>,
    _interceptor: Subscription,
}

impl EventEmitter<InsertDialogEvent> for TableDialog {}

impl Focusable for TableDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Open the table editor on `model` (new table, or the one under the caret).
pub fn open(model: TableEditorModel, window: &mut Window, cx: &mut App) -> Entity<TableDialog> {
    let dialog = cx.new(|cx| TableDialog::new(model, window, cx));
    dialog.update(cx, |d, cx| d.focus(window, cx));
    dialog
}

impl TableDialog {
    fn new(model: TableEditorModel, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let weak = cx.entity().downgrade();
        let interceptor = cx.intercept_keystrokes(move |event, window, cx| {
            let Some(dialog) = weak.upgrade() else { return };
            if dialog.update(cx, |d, cx| d.on_key(&event.keystroke, window, cx)) {
                cx.stop_propagation();
            }
        });
        let mut this = Self {
            model,
            cells: Vec::new(),
            focus: cx.focus_handle(),
            current: (0, 0),
            menu: None,
            cell_subscriptions: Vec::new(),
            _interceptor: interceptor,
        };
        this.rebuild_cells(window, cx);
        this
    }

    /// The model (tests, host).
    pub fn model(&self) -> &TableEditorModel {
        &self.model
    }

    /// The focused cell `(row, col)`.
    pub fn current_cell(&self) -> (usize, usize) {
        self.current
    }

    /// Focus the current cell's input.
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (r, c) = self.current;
        match self.cells.get(r).and_then(|row| row.get(c)) {
            Some(input) => input.update(cx, |s, cx| s.focus(window, cx)),
            None => self.focus.focus(window, cx),
        }
    }

    /// Insert / Update (Ctrl+Enter): emits [`InsertDialogEvent::Block`], or
    /// [`InsertDialogEvent::Cancel`] for an empty table.
    pub fn confirm(&mut self, cx: &mut Context<Self>) {
        let event = match self.model.confirm() {
            Some(done) => InsertDialogEvent::Block {
                content: done.markdown,
                replace: match done.mode {
                    TableMode::Update(range) => Some(range),
                    TableMode::Insert => None,
                },
                cursor_offset: None,
            },
            None => InsertDialogEvent::Cancel,
        };
        cx.emit(event);
    }

    /// Cancel (Esc, Cancel button).
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(InsertDialogEvent::Cancel);
    }

    /// Run a grid action on the current cell and keep focus on the moved cell.
    pub fn run(&mut self, action: CellAction, window: &mut Window, cx: &mut Context<Self>) {
        let (row, col) = self.current;
        self.run_at(action, row, col, window, cx);
    }

    fn run_at(
        &mut self,
        action: CellAction,
        row: usize,
        col: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu = None;
        if let Some(focus) = self.model.apply(action, row, col) {
            self.current = focus;
            self.rebuild_cells(window, cx);
            self.focus(window, cx);
        }
        cx.notify();
    }

    /// Change the size with the steppers.
    fn resize(&mut self, rows: usize, cols: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.model.set_size(rows, cols);
        self.current = (
            self.current.0.min(self.model.rows() - 1),
            self.current.1.min(self.model.cols() - 1),
        );
        self.rebuild_cells(window, cx);
        cx.notify();
    }

    fn set_alignment(&mut self, alignment: Alignment, cx: &mut Context<Self>) {
        let col = self.current.1;
        let next = if self.model.alignment(col) == alignment {
            Alignment::None
        } else {
            alignment
        };
        self.model.set_alignment(col, next);
        cx.notify();
    }

    /// One input per cell, re-created after every structural change (FrankMD
    /// re-rendered the grid too); typing only updates the model.
    fn rebuild_cells(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cell_subscriptions.clear();
        let header_placeholder = i18n::t(cx, "dialogs.table_editor.header");
        let mut cells = Vec::with_capacity(self.model.rows());
        for r in 0..self.model.rows() {
            let mut row = Vec::with_capacity(self.model.cols());
            for c in 0..self.model.cols() {
                let value = self.model.cell(r, c).to_owned();
                let placeholder = header_placeholder.clone();
                let input = cx.new(|cx| {
                    let state = InputState::new(window, cx).default_value(value);
                    if r == 0 {
                        state.placeholder(placeholder)
                    } else {
                        state
                    }
                });
                let sub = cx.subscribe_in(
                    &input,
                    window,
                    move |this, input, event: &InputEvent, _, cx| match event {
                        InputEvent::Change => {
                            let value = input.read(cx).value();
                            this.model.set_cell(r, c, &value);
                        }
                        InputEvent::Focus => {
                            this.current = (r, c);
                            cx.notify();
                        }
                        _ => {}
                    },
                );
                self.cell_subscriptions.push(sub);
                row.push(input);
            }
            cells.push(row);
        }
        self.cells = cells;
    }

    fn move_focus(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (rows, cols) = self.model.dimensions();
        let index = self.current.0 * cols + self.current.1;
        let next = if forward {
            if index + 1 >= rows * cols {
                self.model.add_row();
                self.rebuild_cells(window, cx);
            }
            index + 1
        } else if index == 0 {
            return;
        } else {
            index - 1
        };
        self.current = (next / cols, next % cols);
        self.focus(window, cx);
        cx.notify();
    }

    fn composing(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let (r, c) = self.current;
        self.cells
            .get(r)
            .and_then(|row| row.get(c))
            .is_some_and(|input| {
                input
                    .update(cx, |s, cx| s.marked_text_range(window, cx))
                    .is_some()
            })
    }

    fn on_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.focus.contains_focused(window, cx) || self.composing(window, cx) {
            return false;
        }
        match key_text(keystroke).as_str() {
            "escape" => {
                if self.menu.take().is_some() {
                    cx.notify();
                } else {
                    self.cancel(cx);
                }
            }
            "ctrl-enter" => self.confirm(cx),
            "tab" => self.move_focus(true, window, cx),
            "shift-tab" => self.move_focus(false, window, cx),
            "enter" => {
                if self.current.0 + 1 < self.model.rows() {
                    self.current.0 += 1;
                    self.focus(window, cx);
                    cx.notify();
                }
            }
            "alt-up" => self.run(CellAction::MoveUp, window, cx),
            "alt-down" => self.run(CellAction::MoveDown, window, cx),
            "alt-left" => self.run(CellAction::MoveLeft, window, cx),
            "alt-right" => self.run(CellAction::MoveRight, window, cx),
            "ctrl-shift-backspace" => self.run(CellAction::DeleteRow, window, cx),
            "ctrl-alt-backspace" => self.run(CellAction::DeleteColumn, window, cx),
            _ => return false,
        }
        true
    }

    // ----- rendering ------------------------------------------------------------------

    fn render_stepper(
        &self,
        id: &'static str,
        label_key: &str,
        count: usize,
        titles: (&str, &str),
        on_change: fn(&mut Self, isize, &mut Window, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .gap_1()
            .items_center()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(i18n::t(cx, label_key).to_uppercase()),
            )
            .child(
                Button::new(SharedString::from(format!("{id}-dec")))
                    .icon(crate::ui::icons::ph("minus"))
                    .xsmall()
                    .ghost()
                    .disabled(count <= 1)
                    .tooltip(i18n::t(cx, titles.0))
                    .on_click(
                        cx.listener(move |this, _, window, cx| on_change(this, -1, window, cx)),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from(format!("{id}-count")))
                    .test_support()
                    .min_w(px(24.))
                    .text_sm()
                    .text_center()
                    .child(count.to_string()),
            )
            .child(
                Button::new(SharedString::from(format!("{id}-inc")))
                    .icon(crate::ui::icons::ph("plus"))
                    .xsmall()
                    .ghost()
                    .tooltip(i18n::t(cx, titles.1))
                    .on_click(
                        cx.listener(move |this, _, window, cx| on_change(this, 1, window, cx)),
                    ),
            )
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (rows, cols) = self.model.dimensions();
        let current = self.model.alignment(self.current.1);
        let align = |id: &'static str,
                     label: &'static str,
                     a: Alignment,
                     key: &str,
                     cx: &mut Context<Self>| {
            Button::new(id)
                .label(label)
                .xsmall()
                .ghost()
                .toggled(current == a)
                .tooltip(i18n::t(cx, key))
                .on_click(cx.listener(move |this, _, _, cx| this.set_alignment(a, cx)))
        };
        h_flex()
            .gap_4()
            .flex_wrap()
            .child(self.render_stepper(
                "table-rows",
                "dialogs.table_editor.row",
                rows,
                (
                    "dialogs.table_editor.remove_row",
                    "dialogs.table_editor.add_row",
                ),
                |this, delta, window, cx| {
                    let rows = this.model.rows().saturating_add_signed(delta);
                    let cols = this.model.cols();
                    this.resize(rows, cols, window, cx);
                },
                cx,
            ))
            .child(self.render_stepper(
                "table-cols",
                "dialogs.table_editor.column",
                cols,
                (
                    "dialogs.table_editor.remove_column",
                    "dialogs.table_editor.add_column",
                ),
                |this, delta, window, cx| {
                    let cols = this.model.cols().saturating_add_signed(delta);
                    let rows = this.model.rows();
                    this.resize(rows, cols, window, cx);
                },
                cx,
            ))
            .child(
                h_flex()
                    .id("table-align")
                    .test_support()
                    .gap_0p5()
                    .child(align(
                        "table-align-left",
                        "⇤",
                        Alignment::Left,
                        "insert_dialogs.table.align_left",
                        cx,
                    ))
                    .child(align(
                        "table-align-center",
                        "↔",
                        Alignment::Center,
                        "insert_dialogs.table.align_center",
                        cx,
                    ))
                    .child(align(
                        "table-align-right",
                        "⇥",
                        Alignment::Right,
                        "insert_dialogs.table.align_right",
                        cx,
                    )),
            )
    }

    fn render_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let rows = self.cells.iter().enumerate().map(|(r, row)| {
            let cells = row.iter().enumerate().map(|(c, input)| {
                let focused = self.current == (r, c);
                div()
                    .id(SharedString::from(format!("table-cell-{r}-{c}")))
                    .test_support()
                    .w(px(CELL_WIDTH))
                    .flex_none()
                    .border_1()
                    .border_color(if focused { theme.ring } else { theme.border })
                    .when(r == 0, |el| {
                        el.bg(theme.muted).font_weight(FontWeight::SEMIBOLD)
                    })
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                            this.current = (r, c);
                            this.menu = Some(CellMenu {
                                row: r,
                                col: c,
                                position: ev.position,
                            });
                            this.focus(window, cx);
                            cx.notify();
                        }),
                    )
                    .child(Input::new(input).appearance(false).xsmall())
            });
            h_flex().children(cells.collect::<Vec<_>>())
        });
        div()
            .id("table-grid")
            .max_h(px(420.))
            .overflow_scroll()
            .rounded(theme.radius)
            .child(v_flex().children(rows.collect::<Vec<_>>()))
    }

    fn render_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.menu?;
        let theme = cx.theme();
        let rows =
            self.model
                .cell_menu(menu.row, menu.col)
                .into_iter()
                .map(|item| {
                    let action = item.action;
                    let color = if action.is_destructive() {
                        theme.danger
                    } else {
                        theme.popover_foreground
                    };
                    div()
                        .id(SharedString::from(
                            format!("table-menu-{action:?}").to_lowercase(),
                        ))
                        .test_support()
                        .flex()
                        .justify_between()
                        .gap_4()
                        .px_3()
                        .py_1()
                        .text_sm()
                        .text_color(color)
                        .when(!item.enabled, |el| el.opacity(0.5))
                        .when(item.enabled, |el| {
                            el.hover(|s| s.bg(theme.accent)).on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.run_at(action, menu.row, menu.col, window, cx)
                                },
                            ))
                        })
                        .child(i18n::t(cx, action.label_key()))
                        .child(div().text_xs().text_color(theme.muted_foreground).child(
                            action.shortcut(
                                frankmd_workspace::shortcuts::Platform::current().is_macos(),
                            ),
                        ))
                        .into_any_element()
                });
        let mut items: Vec<AnyElement> = rows.collect();
        // Divider between the column and the row groups.
        if items.len() > 3 {
            items.insert(
                3,
                div().my_1().h(px(1.)).bg(theme.border).into_any_element(),
            );
        }
        Some(
            deferred(
                anchored()
                    .position(menu.position)
                    .snap_to_window_with_margin(px(10.))
                    .child(
                        v_flex()
                            .id("table-cell-menu")
                            .test_support()
                            .min_w(px(200.))
                            .py_1()
                            .bg(theme.popover)
                            .border_1()
                            .border_color(theme.border)
                            .rounded(theme.radius)
                            .shadow_md()
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                this.menu = None;
                                cx.notify();
                            }))
                            .children(items),
                    ),
            )
            .with_priority(2)
            .into_any_element(),
        )
    }
}

impl Render for TableDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = px((CELL_WIDTH * self.model.cols() as f32 + 48.).clamp(480., MAX_WIDTH));
        let primary = i18n::t(cx, self.model.primary_label_key());
        let footer = h_flex()
            .justify_end()
            .gap_2()
            .child(
                Button::new("table-cancel")
                    .label(i18n::t(cx, "common.cancel"))
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
            )
            .child(
                Button::new("table-insert")
                    .label(format!("{primary}  {}↵", crate::ui::actions::ctrl_label()))
                    .small()
                    .primary()
                    .on_click(cx.listener(|this, _, _, cx| this.confirm(cx))),
            );
        let body = v_flex()
            .gap_3()
            .min_h_0()
            .child(self.render_toolbar(cx))
            .child(self.render_grid(cx));
        let title = i18n::t(cx, "dialogs.table_editor.title");
        let menu = self.render_menu(cx);
        modal_frame(
            "dialog-table",
            &self.focus,
            width,
            title,
            body,
            footer,
            menu,
            cx,
        )
    }
}
