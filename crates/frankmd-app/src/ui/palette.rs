//! Command palette (ui-spec §5.1) and its sub-modes: theme picker (live
//! preview, Esc reverts, Enter keeps, §5.17), language picker (switches live),
//! go to line. Rows come from the command registry; running a row runs the
//! same command as its shortcut.

use crate::scale::px;
use frankmd_config::{Key, Value};
use frankmd_i18n::Locale;
use frankmd_workspace::commands::PaletteRow;
use frankmd_workspace::overlay::{OverlayKind, PaletteMode};
use gpui_kit::component::input::{Input, InputEvent, InputState, MoveDown, MoveUp};
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::appearance::{self, ResolvedTheme};
use super::i18n;
use super::shell::Shell;

/// One palette row, whatever the mode.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PaletteItem {
    /// A registry command.
    Command(PaletteRow),
    /// A theme (`.fed` `theme` value).
    Theme {
        id: String,
        label: String,
        dark: bool,
    },
    /// A language; `None` follows the system.
    Language {
        locale: Option<Locale>,
        label: String,
    },
    /// One of FrankMD's editor fonts (`.fed` `editor_font` id).
    Font {
        id: &'static str,
        label: &'static str,
    },
}

impl PaletteItem {
    fn element_id(&self) -> String {
        match self {
            Self::Command(row) => format!("palette-row-{}", row.id),
            Self::Theme { id, .. } => format!("palette-row-theme-{id}"),
            Self::Language { locale, .. } => format!(
                "palette-row-language-{}",
                locale.map_or("system", Locale::code)
            ),
            Self::Font { id, .. } => format!("palette-row-font-{id}"),
        }
    }
}

/// Palette state.
pub(crate) struct PaletteState {
    pub(crate) input: Entity<InputState>,
    pub(crate) mode: PaletteMode,
    pub(crate) selected: usize,
    /// The theme to restore when a live theme preview is cancelled.
    pub(crate) preview_theme: Option<ResolvedTheme>,
}

impl PaletteState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Shell>) -> (Self, Subscription) {
        let input = cx.new(|cx| InputState::new(window, cx));
        let sub = cx.subscribe_in(
            &input,
            window,
            |shell, input, ev: &InputEvent, _window, cx| {
                if matches!(ev, InputEvent::Change) {
                    let text = input.read(cx).value().to_string();
                    if shell.palette.mode == PaletteMode::Commands && text.starts_with(':') {
                        shell.palette.mode = PaletteMode::GoToLine;
                        shell.overlays.open(
                            OverlayKind::Palette(PaletteMode::GoToLine),
                            shell.last_region,
                        );
                    }
                    shell.palette.selected = 0;
                    cx.notify();
                }
            },
        );
        (
            Self {
                input,
                mode: PaletteMode::Commands,
                selected: 0,
                preview_theme: None,
            },
            sub,
        )
    }

    /// Reset for `mode` and focus the input.
    pub(crate) fn open(&mut self, mode: PaletteMode, window: &mut Window, cx: &mut Context<Shell>) {
        self.mode = mode;
        self.selected = 0;
        let placeholder = i18n::t(
            cx,
            match mode {
                PaletteMode::Commands => "palette.placeholder",
                PaletteMode::GoToLine => "palette.go_to_line_placeholder",
                PaletteMode::Theme => "palette.theme_placeholder",
                PaletteMode::Language => "palette.language_placeholder",
                PaletteMode::Font => "palette.font_placeholder",
                PaletteMode::Vault => "palette.vault_placeholder",
            },
        );
        self.input.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx);
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
    }

    fn query(&self, cx: &App) -> String {
        let text = self.input.read(cx).value().to_string();
        let text = match self.mode {
            PaletteMode::Commands => text.strip_prefix('>').unwrap_or(&text).to_owned(),
            PaletteMode::GoToLine => text.strip_prefix(':').unwrap_or(&text).to_owned(),
            _ => text,
        };
        text.trim().to_owned()
    }
}

/// Case-insensitive "every word is a substring" filter for picker rows.
fn matches(label: &str, query: &str) -> bool {
    let label = label.to_lowercase();
    query
        .split_whitespace()
        .all(|w| label.contains(&w.to_lowercase()))
}

impl Shell {
    /// Rows for the current mode and query.
    pub(crate) fn palette_items(&self, cx: &App) -> Vec<PaletteItem> {
        let query = self.palette.query(cx);
        match self.palette.mode {
            PaletteMode::Commands => {
                let recent: Vec<&str> = self.recent_commands.clone();
                self.registry
                    .query(&query, &self.command_context(), &recent)
                    .into_iter()
                    .map(PaletteItem::Command)
                    .collect()
            }
            PaletteMode::Theme => {
                let mut items = Vec::new();
                if let Some((name, tokens)) = appearance::omarchy_tokens(self.omarchy.as_ref()) {
                    let label =
                        i18n::t_args(cx, "palette.follow_omarchy", &[("theme", &name)]).to_string();
                    items.push(PaletteItem::Theme {
                        id: appearance::OMARCHY.to_owned(),
                        label,
                        dark: tokens.mode == frankmd_theme::Mode::Dark,
                    });
                }
                for id in frankmd_theme::builtin_ids() {
                    let dark = frankmd_theme::builtin(id)
                        .is_some_and(|t| t.mode == frankmd_theme::Mode::Dark);
                    items.push(PaletteItem::Theme {
                        id: id.to_owned(),
                        label: theme_label(id),
                        dark,
                    });
                }
                items.retain(|i| match i {
                    PaletteItem::Theme { label, id, .. } => {
                        matches(label, &query) || matches(id, &query)
                    }
                    _ => true,
                });
                items
            }
            PaletteMode::Language => {
                let system = i18n::language_name(cx, self.system_locale());
                let mut items = vec![PaletteItem::Language {
                    locale: None,
                    label: i18n::t_args(cx, "palette.system_language", &[("language", &system)])
                        .to_string(),
                }];
                items.extend(Locale::ALL.iter().map(|l| PaletteItem::Language {
                    locale: Some(*l),
                    label: i18n::language_name(cx, *l).to_string(),
                }));
                items.retain(|i| match i {
                    PaletteItem::Language { label, locale } => {
                        matches(label, &query) || locale.is_some_and(|l| matches(l.code(), &query))
                    }
                    _ => true,
                });
                items
            }
            PaletteMode::Font => super::fonts::EDITOR_FONTS
                .iter()
                .filter(|(id, family)| matches(family, &query) || matches(id, &query))
                .map(|&(id, label)| PaletteItem::Font { id, label })
                .collect(),
            PaletteMode::GoToLine | PaletteMode::Vault => Vec::new(),
        }
    }

    /// ↑/↓ (wrapping). In the theme picker the highlighted theme previews live.
    pub(crate) fn palette_step(
        &mut self,
        forward: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let len = self.palette_items(cx).len();
        if len == 0 {
            return;
        }
        let s = self.palette.selected.min(len - 1);
        self.palette.selected = if forward {
            (s + 1) % len
        } else {
            (s + len - 1) % len
        };
        self.preview_selected_theme(cx);
        cx.notify();
    }

    fn preview_selected_theme(&mut self, cx: &mut Context<Self>) {
        if self.palette.mode != PaletteMode::Theme {
            return;
        }
        let items = self.palette_items(cx);
        let Some(PaletteItem::Theme { id, .. }) = items.get(self.palette.selected) else {
            return;
        };
        let Some(core) = self.core.as_ref() else {
            return;
        };
        let config = core.ws.snapshot().config;
        if self.palette.preview_theme.is_none() {
            self.palette.preview_theme = Some(self.theme.clone());
        }
        let resolved = appearance::resolve_id(Some(id), &config, self.omarchy.as_ref());
        self.apply_tokens(&resolved.tokens, cx);
    }

    /// Esc / backdrop: revert a theme preview.
    pub(crate) fn palette_cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(original) = self.palette.preview_theme.take() {
            self.apply_tokens(&original.tokens, cx);
            self.theme = original;
        }
    }

    /// Enter: run / commit the selected row.
    pub(crate) fn palette_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.mode == PaletteMode::GoToLine {
            self.overlays.close();
            self.palette.preview_theme = None;
            self.focus_region(frankmd_workspace::focus::FocusRegion::Editor, window, cx);
            self.run_command("edit.go_to_line", window, cx);
            return;
        }
        let items = self.palette_items(cx);
        let Some(item) = items
            .get(self.palette.selected.min(items.len().saturating_sub(1)))
            .cloned()
        else {
            return;
        };
        match item {
            PaletteItem::Command(row) => {
                if row.unavailable.is_some() {
                    return;
                }
                // Close first so the command sees the user's region focused.
                self.close_overlay(window, cx);
                self.run_command(row.id, window, cx);
            }
            PaletteItem::Theme { id, .. } => {
                self.palette.preview_theme = None;
                self.overlays.close();
                self.update_settings(vec![(Key::Theme, Some(Value::Str(id)))], window, cx);
                self.focus_region(self.last_region, window, cx);
            }
            PaletteItem::Language { locale, .. } => {
                self.overlays.close();
                self.locale_override = None;
                let value = locale.map(|l| Value::Str(l.code().to_owned()));
                self.update_settings(vec![(Key::Locale, value)], window, cx);
                self.apply_locale(cx);
                self.focus_region(self.last_region, window, cx);
            }
            PaletteItem::Font { id, .. } => {
                // Back to Settings, where the font row was clicked.
                self.update_settings(
                    vec![(Key::EditorFont, Some(Value::Str(id.to_owned())))],
                    window,
                    cx,
                );
                self.open_overlay(OverlayKind::Settings, window, cx);
            }
        }
    }

    /// The palette overlay.
    pub(crate) fn render_palette(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme().clone();
        let items = self.palette_items(cx);
        let selected = self.palette.selected.min(items.len().saturating_sub(1));
        let current_font = self
            .core
            .as_ref()
            .and_then(|c| c.ws.snapshot().config.value(Key::EditorFont))
            .and_then(|v| v.as_str().map(str::to_owned));
        let prompt = match self.palette.mode {
            PaletteMode::Commands => ">",
            PaletteMode::GoToLine => ":",
            PaletteMode::Theme => "",
            PaletteMode::Language => "",
            PaletteMode::Font => "",
            PaletteMode::Vault => "",
        };
        let unavailable_tail = items
            .iter()
            .all(|i| matches!(i, PaletteItem::Command(r) if r.unavailable.is_some()));
        let entity = cx.entity().downgrade();
        let rows = items.into_iter().enumerate().map(|(ix, item)| {
            let is_selected = ix == selected;
            let id = item.element_id();
            let (title, trailing, checked, reason): (
                String,
                Option<String>,
                Option<bool>,
                Option<SharedString>,
            ) = match &item {
                PaletteItem::Command(row) => (
                    row.title.clone(),
                    row.shortcut.clone(),
                    row.checked,
                    row.unavailable.map(|key| i18n::t(cx, key)),
                ),
                PaletteItem::Theme { label, dark, .. } => {
                    let mode = if *dark {
                        "palette.dark"
                    } else {
                        "palette.light"
                    };
                    (
                        label.clone(),
                        Some(i18n::t(cx, mode).to_string()),
                        None,
                        None,
                    )
                }
                PaletteItem::Language { label, .. } => (label.clone(), None, None, None),
                PaletteItem::Font { id, label } => (
                    (*label).to_owned(),
                    None,
                    Some(current_font.as_deref() == Some(*id)),
                    None,
                ),
            };
            let entity = entity.clone();
            div()
                .id(SharedString::from(id))
                .test_support()
                .h(px(28.))
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .when(is_selected, |el| {
                    el.bg(theme.selection)
                        .border_l_2()
                        .border_color(theme.primary)
                })
                .when(reason.is_some(), |el| el.text_color(theme.muted_foreground))
                .child(
                    div()
                        .w(px(14.))
                        .child(if checked == Some(true) { "✓" } else { "" }),
                )
                .child(div().flex_1().truncate().child(title))
                .when_some(reason, |el, r| el.child(div().text_xs().child(r)))
                .when_some(trailing, |el, t| {
                    el.child(div().text_xs().text_color(theme.muted_foreground).child(t))
                })
                .on_click(move |_, window, cx| {
                    let _ = entity.update(cx, |shell, cx| {
                        shell.palette.selected = ix;
                        shell.palette_confirm(window, cx);
                    });
                })
        });
        let rows: Vec<_> = rows.collect();
        let empty = rows.is_empty();
        let no_matches = i18n::t(cx, "palette.no_matches");
        let unavailable = i18n::t(cx, "palette.unavailable_heading");
        let input = self.palette.input.clone();
        let this = cx.entity().downgrade();
        let this2 = this.clone();
        v_flex()
            .id("command-palette")
            .test_support()
            .key_context(super::actions::CX_OVERLAY)
            .w(px(600.))
            .max_w(relative(1.))
            .max_h(px(440.))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .shadow_lg()
            .overflow_hidden()
            .capture_action(move |_: &MoveDown, window, cx| {
                let _ = this.update(cx, |shell, cx| shell.palette_step(true, window, cx));
                cx.stop_propagation();
            })
            .capture_action(move |_: &MoveUp, window, cx| {
                let _ = this2.update(cx, |shell, cx| shell.palette_step(false, window, cx));
                cx.stop_propagation();
            })
            .child(
                h_flex()
                    .h(px(36.))
                    .px_3()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .when(!prompt.is_empty(), |el| {
                        el.child(div().text_color(theme.muted_foreground).child(prompt))
                    })
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&input).id("palette-input").appearance(false)),
                    ),
            )
            .child(
                div()
                    .id("palette-rows")
                    .flex_1()
                    .overflow_y_scroll()
                    .py_1()
                    .when(unavailable_tail && !empty, |el| {
                        el.child(
                            div()
                                .px_3()
                                .py_1()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(unavailable),
                        )
                    })
                    .children(rows)
                    .when(empty && self.palette.mode != PaletteMode::GoToLine, |el| {
                        el.child(
                            div()
                                .px_3()
                                .py_2()
                                .text_color(theme.muted_foreground)
                                .child(no_matches),
                        )
                    }),
            )
    }
}

/// `tokyo-night` → `Tokyo Night`.
fn theme_label(id: &str) -> String {
    id.split('-')
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui_kit glob would shadow `#[test]`.
    use super::{matches, theme_label};

    #[test]
    fn theme_labels_are_title_cased() {
        assert_eq!(theme_label("tokyo-night"), "Tokyo Night");
        assert_eq!(theme_label("dark"), "Dark");
    }

    #[test]
    fn picker_filter_needs_every_word() {
        assert!(matches("Tokyo Night", "tok ni"));
        assert!(!matches("Tokyo Night", "nord"));
        assert!(matches("anything", ""));
    }
}
