//! Rendering of the shell regions (ui-spec §3): sidebar/explorer, editor pane
//! (header, banners, text or docked quick open), preview, status line and the
//! overlay layer. Element ids follow docs/testing/ui-test-plan.md §3.3.

use frankmd_core::tree_view::{RowKind, TreeView, tree_rows};
use frankmd_workspace::banners::{BannerKind, BannerRole, BannerStack};
use frankmd_workspace::focus::FocusRegion;
use frankmd_workspace::layout::{PreviewMode, SidebarMode, WidthClass};
use frankmd_workspace::overlay::OverlayKind;
use frankmd_workspace::status::SaveState;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, MoveDown, MoveUp, ToggleCodeActions};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::component::{Selectable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::actions;
use super::i18n;
use super::shell::{HelpTab, Shell};

/// Sidebar width (ui-spec §3.1 default).
const SIDEBAR_WIDTH: f32 = frankmd_workspace::layout::SIDEBAR_WIDTH;

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = f32::from(window.viewport_size().width);
        if (width - self.width).abs() > f32::EPSILON {
            self.on_width(width, window, cx);
        }
        let theme = cx.theme().clone();
        let arrangement = self.arrangement();
        let handler = Shell::command_handler(cx);

        let main = h_flex()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .when(arrangement.sidebar == SidebarMode::Docked, |el| {
                el.child(self.render_sidebar(&theme, cx))
            })
            .when(arrangement.media_library, |el| {
                el.child(self.render_media_library(&theme, cx))
            })
            .when(arrangement.editor && !arrangement.media_library, |el| {
                el.child(self.render_editor_pane(&arrangement, &theme, window, cx))
            })
            .when(
                arrangement.preview != PreviewMode::Hidden && !arrangement.media_library,
                |el| {
                    el.child(self.render_preview(
                        arrangement.preview == PreviewMode::Swapped,
                        &theme,
                        cx,
                    ))
                },
            );

        let root = div()
            .id("shell")
            .key_context(actions::CX_WORKSPACE)
            .track_focus(&self.root_focus)
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.background)
            .text_color(theme.foreground)
            .font_family(theme.font_family.clone())
            .text_size(px(13.));
        actions::on_commands(root, handler)
            .child(main)
            .child(self.render_status_line(&arrangement, &theme, cx))
            .when(arrangement.sidebar == SidebarMode::Drawer, |el| {
                el.child(self.render_drawer(&theme, cx))
            })
            .when_some(self.overlays.current().cloned(), |el, kind| {
                el.child(self.render_overlay(kind, &theme, window, cx))
            })
    }
}

impl Shell {
    // ----- sidebar ------------------------------------------------------------------

    fn render_sidebar(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let new_note = this.clone();
        v_flex()
            .id("sidebar")
            .test_support()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(
                h_flex()
                    .h(px(32.))
                    .px_3()
                    .gap_1()
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .font_semibold()
                            .child(self.vault_name.clone()),
                    )
                    .child(
                        Button::new("sidebar-new-note")
                            .ghost()
                            .xsmall()
                            .label("+")
                            .tooltip(i18n::t(cx, "shell.new_note"))
                            .on_click(move |_, window, cx| {
                                let _ = new_note.update(cx, |shell, cx| {
                                    shell.run_command("file.new_note", window, cx)
                                });
                            }),
                    ),
            )
            .child(self.render_explorer(theme, cx))
            .child(
                h_flex()
                    .h(px(28.))
                    .px_2()
                    .gap_1()
                    .border_t_1()
                    .border_color(theme.sidebar_border)
                    .child(self.dock_button(
                        "dock-media-library",
                        "▤",
                        "shell.media_library",
                        "view.media_library",
                        cx,
                    ))
                    .child(self.dock_button(
                        "dock-settings",
                        "⚙",
                        "shell.settings",
                        "app.settings",
                        cx,
                    ))
                    .child(self.dock_button("dock-help", "?", "shell.help", "help.toggle", cx)),
            )
    }

    fn dock_button(
        &self,
        id: &'static str,
        glyph: &'static str,
        tooltip: &str,
        command: &'static str,
        cx: &mut Context<Self>,
    ) -> Button {
        let this = cx.entity().downgrade();
        Button::new(id)
            .ghost()
            .xsmall()
            .label(glyph)
            .tooltip(i18n::t(cx, tooltip))
            .on_click(move |_, window, cx| {
                let _ = this.update(cx, |shell, cx| shell.run_command(command, window, cx));
            })
    }

    fn render_explorer(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let tree = self
            .core
            .as_ref()
            .map(|c| c.ws.snapshot().tree)
            .unwrap_or_default();
        let active = self.active_path().unwrap_or_default();
        let view = tree_rows(&tree, &self.explorer.expanded, &active);
        let this = cx.entity().downgrade();
        let cursor = self.explorer.cursor.clone();
        let focused = self.sidebar_focus.clone();
        let mut list = v_flex()
            .id("explorer-tree")
            .test_support()
            .track_focus(&self.sidebar_focus)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .py_1()
            .on_key_down({
                let this = this.clone();
                move |ev: &KeyDownEvent, window, cx| {
                    let _ = this.update(cx, |shell, cx| {
                        if shell.explorer_key(&ev.keystroke.key, window, cx) {
                            cx.stop_propagation();
                        }
                    });
                }
            });
        match view {
            TreeView::Empty => {
                list = list.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_color(theme.muted_foreground)
                        .child(i18n::t(cx, "sidebar.no_notes_yet")),
                );
            }
            TreeView::Rows(rows) => {
                for row in rows.into_iter().filter(|r| r.visible) {
                    let path = row.path.clone();
                    let is_cursor = cursor.as_deref() == Some(path.as_str());
                    let this = this.clone();
                    let focused = focused.clone();
                    #[allow(clippy::cast_precision_loss)]
                    let indent = 8.0 + row.depth as f32 * 12.0;
                    let glyph = match row.kind {
                        RowKind::Folder if row.expanded => "▾",
                        RowKind::Folder => "▸",
                        _ => " ",
                    };
                    let kind = row.kind;
                    list = list.child(
                        h_flex()
                            .id(SharedString::from(format!("explorer-row-{}", row.path)))
                            .test_support()
                            .h(px(24.))
                            .pl(px(indent))
                            .pr_2()
                            .gap_1()
                            .when(row.selected, |el| {
                                el.bg(theme.selection)
                                    .border_l_2()
                                    .border_color(theme.primary)
                            })
                            .when(is_cursor && !row.selected, |el| el.bg(theme.list_hover))
                            .hover(|s| s.bg(theme.list_hover))
                            .child(
                                div()
                                    .w(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(glyph),
                            )
                            .child(div().flex_1().truncate().child(row.name.clone()))
                            .on_click(move |_, window, cx| {
                                let path = path.clone();
                                let _ = this.update(cx, |shell, cx| {
                                    shell.explorer.cursor = Some(path.clone());
                                    if kind == RowKind::Folder {
                                        shell.toggle_folder(&path);
                                        focused.focus(window, cx);
                                    } else {
                                        shell.open_path(&path, window, cx);
                                    }
                                    cx.notify();
                                });
                            }),
                    );
                }
            }
        }
        list
    }

    fn toggle_folder(&mut self, path: &str) {
        if !self.explorer.expanded.remove(path) {
            self.explorer.expanded.insert(path.to_owned());
        }
    }

    /// Explorer keyboard (§5.14 subset): ↑/↓ move, →/← expand/collapse, Enter opens.
    fn explorer_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let tree = self
            .core
            .as_ref()
            .map(|c| c.ws.snapshot().tree)
            .unwrap_or_default();
        let active = self.active_path().unwrap_or_default();
        let TreeView::Rows(rows) = tree_rows(&tree, &self.explorer.expanded, &active) else {
            return false;
        };
        let rows: Vec<_> = rows.into_iter().filter(|r| r.visible).collect();
        if rows.is_empty() {
            return false;
        }
        let at = self
            .explorer
            .cursor
            .as_ref()
            .and_then(|c| rows.iter().position(|r| &r.path == c));
        let current = at.and_then(|i| rows.get(i));
        match key {
            "down" => {
                let next = at.map_or(0, |i| (i + 1).min(rows.len() - 1));
                self.explorer.cursor = Some(rows[next].path.clone());
            }
            "up" => {
                let prev = at.map_or(0, |i| i.saturating_sub(1));
                self.explorer.cursor = Some(rows[prev].path.clone());
            }
            "right" => {
                if let Some(row) = current.filter(|r| r.kind == RowKind::Folder) {
                    self.explorer.expanded.insert(row.path.clone());
                }
            }
            "left" => {
                if let Some(row) = current {
                    if row.kind == RowKind::Folder && row.expanded {
                        self.explorer.expanded.remove(&row.path);
                    } else if let Some((parent, _)) = row.path.rsplit_once('/') {
                        self.explorer.cursor = Some(parent.to_owned());
                    }
                }
            }
            "enter" | "space" => {
                let Some(row) = current.cloned() else {
                    return false;
                };
                if row.kind == RowKind::Folder {
                    self.toggle_folder(&row.path);
                } else if key == "enter" {
                    self.open_path(&row.path, window, cx);
                }
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    fn render_drawer(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let drawer_width = (self.width * 0.85).min(320.);
        div()
            .absolute()
            .inset_0()
            .flex()
            .child(
                div()
                    .w(px(drawer_width))
                    .h_full()
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(self.render_sidebar(theme, cx)),
            )
            .child(
                div()
                    .id("drawer-scrim")
                    .test_support()
                    .flex_1()
                    .h_full()
                    .bg(theme.background.opacity(0.4))
                    .on_click(move |_, window, cx| {
                        let _ = this.update(cx, |shell, cx| {
                            shell.layout.close_drawer();
                            shell.focus_region(FocusRegion::Editor, window, cx);
                        });
                    }),
            )
    }

    // ----- editor pane ----------------------------------------------------------------

    fn render_editor_pane(
        &mut self,
        arrangement: &frankmd_workspace::layout::PaneArrangement,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let has_note = self.active_path().is_some();
        let focused = self.editor.focus_handle(cx).contains_focused(window, cx);
        v_flex()
            .id("editor-pane")
            .test_support()
            // gpui-kit's editor binds Ctrl+. (code actions) deeper than our
            // Editor-context binding; take it in the capture phase so Ctrl+.
            // is always the context action (§6.2).
            .capture_action(cx.listener(|this, _: &ToggleCodeActions, window, cx| {
                cx.stop_propagation();
                this.run_command("edit.context_action", window, cx);
            }))
            .flex_1()
            .min_w(px(
                frankmd_workspace::layout::EDITOR_MIN_WIDTH.min(self.width)
            ))
            .h_full()
            .overflow_hidden()
            .when(arrangement.pane_header && has_note, |el| {
                el.child(self.render_pane_header(focused, theme, cx))
            })
            .child(self.render_banners(theme, cx))
            .child(if has_note {
                div()
                    .flex_1()
                    .min_h_0()
                    .child(self.editor.render())
                    .into_any_element()
            } else {
                self.render_empty_state(theme, cx).into_any_element()
            })
    }

    fn render_pane_header(
        &mut self,
        focused: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let path = self.active_path().unwrap_or_default();
        let mut parts: Vec<&str> = path.split('/').collect();
        let title = parts
            .pop()
            .unwrap_or_default()
            .trim_end_matches(".md")
            .to_owned();
        let narrow = self.layout.width_class == WidthClass::Narrow;
        let crumbs = if narrow && !parts.is_empty() {
            "…/".to_owned()
        } else {
            parts.iter().map(|p| format!("{p} / ")).collect::<String>()
        };
        let dirty = self.core.as_ref().is_some_and(|c| c.ws.snapshot().dirty);
        let sidebar_hidden = self.arrangement().sidebar == SidebarMode::Hidden;
        let preview_on = self.arrangement().preview != PreviewMode::Hidden;
        h_flex()
            .id("pane-header")
            .test_support()
            .h(px(32.))
            .px_3()
            .gap_1()
            .flex_shrink_0()
            .border_b_1()
            .border_color(theme.sidebar_border)
            .when(focused, |el| el.border_t_2().border_color(theme.primary))
            .when(sidebar_hidden, |el| {
                el.child(self.header_button(
                    "pane-open-sidebar",
                    "◧",
                    "shell.open_sidebar",
                    "view.toggle_sidebar",
                    false,
                    cx,
                ))
            })
            .child(div().text_color(theme.muted_foreground).child(crumbs))
            .child(
                div()
                    .id("pane-title")
                    .test_support()
                    .font_medium()
                    .truncate()
                    .child(title),
            )
            .when(dirty, |el| {
                el.child(div().text_color(theme.muted_foreground).child("●"))
            })
            .child(div().flex_1())
            .when(!narrow, |el| {
                el.child(self.header_button(
                    "pane-insert-image",
                    "▣",
                    "shell.insert_image",
                    "insert.image",
                    false,
                    cx,
                ))
                .child(self.header_button(
                    "pane-find",
                    "⌕",
                    "shell.find",
                    "find.find",
                    false,
                    cx,
                ))
            })
            .child(self.header_button(
                "pane-toggle-preview",
                "◫",
                "shell.toggle_preview",
                "view.toggle_preview",
                preview_on,
                cx,
            ))
            .child(self.header_button(
                "pane-menu",
                "⋯",
                "shell.document_menu",
                "app.command_palette",
                false,
                cx,
            ))
    }

    fn header_button(
        &self,
        id: &'static str,
        glyph: &'static str,
        tooltip: &str,
        command: &'static str,
        pressed: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        let this = cx.entity().downgrade();
        Button::new(id)
            .ghost()
            .xsmall()
            .selected(pressed)
            .label(glyph)
            .tooltip(i18n::t(cx, tooltip))
            .on_click(move |_, window, cx| {
                let _ = this.update(cx, |shell, cx| shell.run_command(command, window, cx));
            })
    }

    fn render_banners(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let vault = self.vault_root.display().to_string();
        v_flex()
            .flex_shrink_0()
            .children(self.banners.visible().into_iter().map(|kind| {
                let role = match kind.role() {
                    BannerRole::Danger => theme.danger,
                    BannerRole::Warning => theme.warning,
                    BannerRole::Info => theme.info,
                };
                let message = match kind {
                    BannerKind::VaultUnreachable => {
                        i18n::t_args(cx, kind.message_key(), &[("vault", &vault)])
                    }
                    _ => i18n::t(cx, kind.message_key()),
                };
                let buttons = BannerStack::buttons(kind).into_iter().map(|key| {
                    let this = this.clone();
                    Button::new(SharedString::from(kind.button_id(key)))
                        .xsmall()
                        .outline()
                        .label(i18n::t(cx, key))
                        .on_click(move |_, window, cx| {
                            let _ = this
                                .update(cx, |shell, cx| shell.banner_action(kind, key, window, cx));
                        })
                });
                let buttons: Vec<_> = buttons.collect();
                h_flex()
                    .id(SharedString::from(kind.element_id()))
                    .test_support()
                    .h(px(32.))
                    .px_3()
                    .gap_2()
                    .bg(role.opacity(0.14))
                    .border_l_2()
                    .border_color(role)
                    .child(div().flex_1().truncate().child(message))
                    .children(buttons)
            }))
    }

    fn render_empty_state(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let count = i64::try_from(self.note_count()).unwrap_or(i64::MAX);
        let docked = self.overlays.current() != Some(&OverlayKind::QuickOpen);
        let error = self.startup_error.clone();
        let vault = self.vault_root.display().to_string();
        v_flex()
            .id("empty-state")
            .test_support()
            .flex_1()
            .items_center()
            .pt(px(64.))
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        svg()
                            .path("brand/icon.svg")
                            .size(px(40.))
                            .text_color(theme.primary),
                    )
                    .child(
                        v_flex()
                            .child(div().font_semibold().child(self.vault_name.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(i18n::t_count(cx, "empty_state.notes", count)),
                            ),
                    ),
            )
            .when_some(error, |el, reason| {
                el.child(div().text_color(theme.danger).child(i18n::t_args(
                    cx,
                    "empty_state.vault_unavailable",
                    &[("vault", &vault), ("reason", &reason)],
                )))
            })
            .when(docked && self.core.is_some(), |el| {
                el.child(
                    div()
                        .w(px(560.))
                        .max_w(relative(1.))
                        .child(self.render_quick_open_list(true, theme, cx)),
                )
            })
            .child(
                h_flex()
                    .gap_3()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(format!("Ctrl+N {}", i18n::t(cx, "empty_state.new_note")))
                    .child(format!(
                        "Ctrl+Shift+F {}",
                        i18n::t(cx, "empty_state.search")
                    ))
                    .child(format!(
                        "Ctrl+Shift+P {}",
                        i18n::t(cx, "empty_state.commands")
                    )),
            )
    }

    /// The quick-open list: docked in the empty state or as an overlay.
    pub(crate) fn render_quick_open_list(
        &mut self,
        docked: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let selected = self.quick_open.model.selected();
        let details = self.quick_open.model.row_details().to_vec();
        let suggestion = self.quick_open.model.create_suggestion();
        let up = this.clone();
        let down = this.clone();
        let rows = details.into_iter().enumerate().map(|(ix, row)| {
            let this = this.clone();
            let path = row.path.clone();
            let (folder, name) = match row.path.rsplit_once('/') {
                Some((folder, name)) => {
                    (folder.to_owned(), name.trim_end_matches(".md").to_owned())
                }
                None => (String::new(), row.path.trim_end_matches(".md").to_owned()),
            };
            h_flex()
                .id(SharedString::from(format!("quick-open-row-{}", row.path)))
                .test_support()
                .h(px(28.))
                .px_3()
                .gap_2()
                .when(selected == Some(ix), |el| {
                    el.bg(theme.selection)
                        .border_l_2()
                        .border_color(theme.primary)
                })
                .child(div().truncate().child(name))
                .child(
                    div()
                        .flex_1()
                        .truncate()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(folder),
                )
                .when(row.recent, |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(i18n::t(cx, "quick_open.recent")),
                    )
                })
                .on_click(move |_, window, cx| {
                    let _ = this.update(cx, |shell, cx| {
                        shell.overlays.close();
                        shell.open_path(&path, window, cx);
                    });
                })
        });
        let rows: Vec<_> = rows.collect();
        let empty = rows.is_empty();
        v_flex()
            .id(if docked {
                "quick-open-docked"
            } else {
                "quick-open"
            })
            .test_support()
            .key_context(actions::CX_OVERLAY)
            .w_full()
            .max_h(px(440.))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .overflow_hidden()
            .when(!docked, |el| el.shadow_lg())
            .capture_action(move |_: &MoveDown, window, cx| {
                let _ = down.update(cx, |shell, cx| shell.list_step(true, window, cx));
                cx.stop_propagation();
            })
            .capture_action(move |_: &MoveUp, window, cx| {
                let _ = up.update(cx, |shell, cx| shell.list_step(false, window, cx));
                cx.stop_propagation();
            })
            .child(
                h_flex()
                    .h(px(36.))
                    .px_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div().flex_1().child(
                            Input::new(&self.quick_open.input)
                                .id("quick-open-input")
                                .appearance(false),
                        ),
                    ),
            )
            .child(
                div()
                    .id("quick-open-rows")
                    .flex_1()
                    .overflow_y_scroll()
                    .py_1()
                    .children(rows)
                    .when(empty, |el| {
                        el.child(
                            div()
                                .px_3()
                                .py_2()
                                .text_color(theme.muted_foreground)
                                .child(i18n::t(cx, "quick_open.no_matches")),
                        )
                        .when_some(suggestion, |el, name| {
                            el.child(div().px_3().pb_2().text_xs().child(i18n::t_args(
                                cx,
                                "quick_open.create_hint",
                                &[("name", &name)],
                            )))
                        })
                    }),
            )
    }

    // ----- preview & library -----------------------------------------------------------

    fn render_preview(
        &mut self,
        swapped: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let zoom_out = this.clone();
        let zoom_reset = this.clone();
        let zoom_in = this.clone();
        let sync = this.clone();
        let zoom = self.preview.read(cx).zoom();
        let sync_on = self.scroll_sync;
        v_flex()
            .id("preview-region")
            .when(swapped, |el| el.flex_1())
            .when(!swapped, |el| {
                el.w(relative(0.45))
                    .min_w(px(frankmd_workspace::layout::PREVIEW_MIN_WIDTH))
            })
            .h_full()
            .border_l_1()
            .border_color(theme.sidebar_border)
            .child(
                h_flex()
                    .id("preview-header")
                    .h(px(32.))
                    .px_3()
                    .gap_1()
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(theme.sidebar_border)
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .child(i18n::t(cx, "preview.title")),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("preview-zoom-out")
                            .ghost()
                            .xsmall()
                            .label("−")
                            .tooltip(i18n::t(cx, "preview.zoom_out"))
                            .on_click(move |_, _, cx| {
                                let _ = zoom_out.update(cx, |shell, cx| {
                                    shell.preview.update(cx, |p, cx| p.zoom_out(cx))
                                });
                            }),
                    )
                    .child(
                        Button::new("preview-zoom-reset")
                            .ghost()
                            .xsmall()
                            .label(format!("{zoom}%"))
                            .tooltip(i18n::t(cx, "shell.reset_zoom"))
                            .on_click(move |_, _, cx| {
                                let _ = zoom_reset.update(cx, |shell, cx| {
                                    shell.preview.update(cx, |p, cx| p.reset_zoom(cx))
                                });
                            }),
                    )
                    .child(
                        Button::new("preview-zoom-in")
                            .ghost()
                            .xsmall()
                            .label("+")
                            .tooltip(i18n::t(cx, "preview.zoom_in"))
                            .on_click(move |_, _, cx| {
                                let _ = zoom_in.update(cx, |shell, cx| {
                                    shell.preview.update(cx, |p, cx| p.zoom_in(cx))
                                });
                            }),
                    )
                    .child(
                        Button::new("preview-scroll-sync")
                            .ghost()
                            .xsmall()
                            .selected(sync_on)
                            .label("⇅")
                            .tooltip(i18n::t(cx, "shell.scroll_sync"))
                            .on_click(move |_, window, cx| {
                                let _ = sync.update(cx, |shell, cx| {
                                    shell.run_command("view.toggle_scroll_sync", window, cx)
                                });
                            }),
                    )
                    .child(
                        Button::new("preview-close")
                            .ghost()
                            .xsmall()
                            .label("×")
                            .tooltip(i18n::t(cx, "shell.close_preview"))
                            .on_click(move |_, window, cx| {
                                let _ = this.update(cx, |shell, cx| {
                                    shell.run_command("view.toggle_preview", window, cx)
                                });
                            }),
                    ),
            )
            .child(div().flex_1().min_h_0().child(self.preview.clone()))
    }

    fn render_media_library(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        v_flex()
            .id("media-library")
            .test_support()
            .track_focus(&self.library_focus)
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            .text_color(theme.muted_foreground)
            .child(i18n::t(cx, "shell.media_library_later"))
    }

    // ----- status line -------------------------------------------------------------------

    fn render_status_line(
        &mut self,
        arrangement: &frankmd_workspace::layout::PaneArrangement,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let save = self.save_state();
        let save_text = save.label_key().map(|key| {
            let glyph = match save {
                SaveState::Unsaved => "●",
                SaveState::Saving => "↻",
                SaveState::Saved => "✓",
                SaveState::Failed => "✕",
                SaveState::DraftNotStored => "⚠",
                SaveState::Clean => "",
            };
            let label = if save == SaveState::DraftNotStored {
                "⚠".into()
            } else {
                i18n::t(cx, key)
            };
            format!("{glyph} {label}")
        });
        let message = self.messages.current(self.clock.instant()).cloned();
        let has_note = self.active_path().is_some();
        let (line, col) = if has_note {
            self.editor.caret()
        } else {
            (0, 0)
        };
        let vim_mode = if self.vim {
            self.editor.vim_mode(cx)
        } else {
            None
        };
        let vim_line = if self.vim {
            self.editor.vim_line(cx)
        } else {
            None
        };
        let compact = arrangement.status_compact;
        let vault_down = self
            .banners
            .is_raised(frankmd_workspace::banners::BannerKind::VaultUnreachable);
        h_flex()
            .id("status-line")
            .test_support()
            .h(px(24.))
            .px_2()
            .gap_3()
            .flex_shrink_0()
            .text_xs()
            .bg(theme.status_bar)
            .border_t_1()
            .border_color(theme.status_bar_border)
            .when(self.layout.zen, |el| el.opacity(0.5))
            .when_some(vim_mode, |el, mode| {
                // §3.6 mode colours: normal accent, insert green, visual magenta, command yellow.
                let bg = match mode {
                    "INSERT" => theme.success,
                    "VISUAL" | "V-LINE" | "V-BLOCK" => theme.magenta,
                    "COMMAND" => theme.warning,
                    _ => theme.primary,
                };
                let label = mode;
                el.child(
                    div()
                        .id("status-mode")
                        .test_support()
                        .px_1()
                        .bg(bg)
                        .text_color(theme.primary_foreground)
                        .font_semibold()
                        .child(label),
                )
            })
            .child(
                div()
                    .id("status-save")
                    .test_support()
                    .when(save.is_error(), |el| el.text_color(theme.danger))
                    .when(save == SaveState::Saved, |el| el.text_color(theme.success))
                    .children(save_text),
            )
            .child(
                div()
                    .id("status-message")
                    .test_support()
                    .flex_1()
                    .truncate()
                    .when_some(vim_line.clone(), |el, line| {
                        el.font_family(theme.mono_font_family.clone()).child(line)
                    })
                    .when(vim_line.is_none(), |el| {
                        el.when_some(message, |el, m| {
                            el.when(m.error, |el| el.text_color(theme.danger))
                                .child(m.text)
                        })
                    }),
            )
            .when(has_note, |el| {
                el.child(div().id("status-position").child(format!("{line}:{col}")))
            })
            .when(self.layout.typewriter && !compact, |el| el.child("⌶"))
            .when(!self.scroll_sync && !compact, |el| el.child("⛓"))
            .when(
                !compact && self.layout.width_class == WidthClass::Wide,
                |el| {
                    el.child(
                        div()
                            .id("status-vault")
                            .when(vault_down, |el| el.text_color(theme.danger))
                            .child(self.vault_name.clone()),
                    )
                },
            )
    }

    // ----- overlays ---------------------------------------------------------------------

    fn render_overlay(
        &mut self,
        kind: OverlayKind,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let dismissible = !matches!(kind, OverlayKind::Dialog("recovery"));
        let top = if self.layout.width_class == WidthClass::Narrow {
            px(16.)
        } else {
            window.viewport_size().height * 0.12
        };
        let content: AnyElement = match kind {
            OverlayKind::Palette(_) => self.render_palette(window, cx).into_any_element(),
            OverlayKind::QuickOpen => div()
                .w(px(680.))
                .max_w(relative(1.))
                .child(self.render_quick_open_list(false, theme, cx))
                .into_any_element(),
            OverlayKind::Settings => self.render_settings(cx).into_any_element(),
            OverlayKind::Help => self.render_help(theme, cx).into_any_element(),
            OverlayKind::Dialog("recovery") => self.render_recovery(theme, cx).into_any_element(),
            OverlayKind::Dialog(_) | OverlayKind::SearchInFiles => div().into_any_element(),
        };
        div()
            .id("overlay-scrim")
            .test_support()
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .pt(top)
            .px_4()
            .bg(theme.background.opacity(0.6))
            .on_click(move |_, window, cx| {
                if dismissible {
                    let _ = this.update(cx, |shell, cx| shell.close_overlay(window, cx));
                }
            })
            .child(
                div()
                    .max_w(relative(1.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(content),
            )
    }

    fn render_help(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let tab = self.help_tab;
        let tabs = [
            (HelpTab::Markdown, "help_dialog.markdown"),
            (HelpTab::Keyboard, "help_dialog.keyboard"),
            (HelpTab::Vim, "help_dialog.vim"),
        ];
        let tab_buttons = tabs.into_iter().map(|(t, key)| {
            let this = this.clone();
            Button::new(SharedString::from(format!(
                "help-tab-{}",
                key.rsplit('.').next().unwrap_or(key)
            )))
            .small()
            .ghost()
            .selected(t == tab)
            .label(i18n::t(cx, key))
            .on_click(move |_, _, cx| {
                let _ = this.update(cx, |shell, cx| {
                    shell.help_tab = t;
                    cx.notify();
                });
            })
        });
        let tab_buttons: Vec<_> = tab_buttons.collect();
        let body: Vec<AnyElement> = match tab {
            HelpTab::Markdown => [
                ("help_dialog.heading", "# Heading"),
                ("help_dialog.bold", "**bold**"),
                ("help_dialog.italic", "*italic*"),
                ("help_dialog.link", "[text](https://…)"),
                ("help_dialog.list", "- item"),
                ("help_dialog.task", "- [ ] task"),
                ("help_dialog.code", "`code`  ```lang"),
                ("help_dialog.quote", "> quote"),
                ("help_dialog.table", "| a | b |"),
            ]
            .into_iter()
            .map(|(label, sample)| {
                h_flex()
                    .h(px(24.))
                    .gap_3()
                    .child(div().w(px(160.)).child(i18n::t(cx, label)))
                    .child(
                        div()
                            .font_family(theme.mono_font_family.clone())
                            .text_color(theme.muted_foreground)
                            .child(sample),
                    )
                    .into_any_element()
            })
            .collect(),
            HelpTab::Keyboard => self
                .registry
                .commands()
                .iter()
                .filter(|c| c.in_palette)
                .filter_map(|c| c.shortcut().map(|s| (c, s)))
                .map(|(c, shortcut)| {
                    h_flex()
                        .id(SharedString::from(format!("help-key-{}", c.id)))
                        .test_support()
                        .h(px(24.))
                        .gap_3()
                        .child(div().flex_1().child(c.title()))
                        .child(
                            div()
                                .px_1()
                                .rounded(theme.radius)
                                .bg(theme.secondary)
                                .text_xs()
                                .child(shortcut),
                        )
                        .into_any_element()
                })
                .collect(),
            HelpTab::Vim => vec![
                div()
                    .child(i18n::t(cx, "help_dialog.vim_intro"))
                    .into_any_element(),
            ],
        };
        let close = this.clone();
        v_flex()
            .id("help-dialog")
            .test_support()
            .track_focus(&self.overlay_focus)
            .w(px(760.))
            .h(px(600.))
            .max_w(relative(1.))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .shadow_lg()
            .overflow_hidden()
            .child(
                h_flex()
                    .h(px(40.))
                    .px_4()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .font_semibold()
                            .child(i18n::t(cx, "help_dialog.title")),
                    )
                    .children(tab_buttons)
                    .child(div().flex_1())
                    .child(
                        Button::new("help-close")
                            .ghost()
                            .small()
                            .label("×")
                            .on_click(move |_, window, cx| {
                                let _ =
                                    close.update(cx, |shell, cx| shell.close_overlay(window, cx));
                            }),
                    ),
            )
            .child(
                v_flex()
                    .id("help-body")
                    .flex_1()
                    .p_4()
                    .overflow_y_scroll()
                    .children(body),
            )
    }

    fn render_recovery(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let keep = this.clone();
        let (path, file, draft) = self
            .recovery
            .as_ref()
            .map(|r| {
                (
                    r.path.clone(),
                    r.server_content.clone(),
                    r.backup_content.clone(),
                )
            })
            .unwrap_or_default();
        let column = |title: SharedString, text: String| {
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(title),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("recovery-{}", text.len())))
                        .flex_1()
                        .p_2()
                        .bg(theme.secondary)
                        .rounded(theme.radius)
                        .font_family(theme.mono_font_family.clone())
                        .overflow_y_scroll()
                        .child(text),
                )
        };
        v_flex()
            .id("dialog-recovery")
            .test_support()
            .track_focus(&self.overlay_focus)
            .w(px(1100.))
            .h(px(560.))
            .max_w(relative(1.))
            .p_4()
            .gap_3()
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .child(
                div()
                    .font_semibold()
                    .child(i18n::t(cx, "dialogs.recovery.title")),
            )
            .child(div().text_color(theme.muted_foreground).child(path))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .gap_3()
                    .items_start()
                    .child(column(i18n::t(cx, "dialogs.recovery.server_version"), file))
                    .child(column(
                        i18n::t(cx, "dialogs.recovery.backup_version"),
                        draft,
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("recovery-use-file")
                            .label(i18n::t(cx, "dialogs.recovery.use_server"))
                            .on_click(move |_, window, cx| {
                                let _ = this.update(cx, |shell, cx| {
                                    shell.resolve_recovery(false, window, cx)
                                });
                            }),
                    )
                    .child(
                        Button::new("recovery-restore-draft")
                            .primary()
                            .label(i18n::t(cx, "dialogs.recovery.restore_backup"))
                            .on_click(move |_, window, cx| {
                                let _ = keep.update(cx, |shell, cx| {
                                    shell.resolve_recovery(true, window, cx)
                                });
                            }),
                    ),
            )
    }
}
