//! Settings sheet (ui-spec §5.16): one place for every preference, a search
//! field filtering rows across categories, and a provenance chip on each row.
//! Changes apply live and are written to `.fed` through the workspace.
//!
//! This round covers Appearance, Editor, Writing, Preview, Files, Diagnostics
//! (config sources) and About; Media, Integrations and Hugo rows come with
//! their dialogs.

use frankmd_config::{Key, Provenance, Value};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::component::{Selectable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::i18n;
use super::shell::Shell;

/// Settings categories, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Category {
    #[default]
    Appearance,
    Editor,
    Writing,
    Preview,
    Files,
    Diagnostics,
    About,
}

impl Category {
    const ALL: [Category; 7] = [
        Self::Appearance,
        Self::Editor,
        Self::Writing,
        Self::Preview,
        Self::Files,
        Self::Diagnostics,
        Self::About,
    ];

    fn slug(self) -> &'static str {
        match self {
            Self::Appearance => "appearance",
            Self::Editor => "editor",
            Self::Writing => "writing",
            Self::Preview => "preview",
            Self::Files => "files",
            Self::Diagnostics => "diagnostics",
            Self::About => "about",
        }
    }
}

/// What a row's control does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    /// On/off key.
    Switch(Key),
    /// Opens a palette picker (theme, language).
    Picker(&'static str),
    /// Split / Single.
    ViewMode,
    /// Read-only value.
    Value(Key),
    /// The vault path.
    NotesFolder,
}

struct RowSpec {
    category: Category,
    label: &'static str,
    description: Option<&'static str>,
    control: Control,
}

const ROWS: &[RowSpec] = &[
    RowSpec {
        category: Category::Appearance,
        label: "settings.theme",
        description: Some("settings.theme_description"),
        control: Control::Picker("theme.select"),
    },
    RowSpec {
        category: Category::Appearance,
        label: "settings.language",
        description: None,
        control: Control::Picker("language.select"),
    },
    RowSpec {
        category: Category::Editor,
        label: "settings.font",
        description: None,
        control: Control::Value(Key::EditorFont),
    },
    RowSpec {
        category: Category::Editor,
        label: "settings.font_size",
        description: None,
        control: Control::Value(Key::EditorFontSize),
    },
    RowSpec {
        category: Category::Editor,
        label: "settings.editor_width",
        description: Some("settings.editor_width_description"),
        control: Control::Value(Key::EditorWidth),
    },
    RowSpec {
        category: Category::Editor,
        label: "settings.line_numbers",
        description: None,
        control: Control::Value(Key::EditorLineNumbers),
    },
    RowSpec {
        category: Category::Editor,
        label: "settings.indent",
        description: Some("settings.indent_description"),
        control: Control::Value(Key::EditorIndent),
    },
    RowSpec {
        category: Category::Editor,
        label: "settings.vim",
        description: Some("settings.vim_description"),
        control: Control::Switch(Key::VimMode),
    },
    RowSpec {
        category: Category::Writing,
        label: "settings.typewriter",
        description: Some("settings.typewriter_description"),
        control: Control::Switch(Key::TypewriterMode),
    },
    RowSpec {
        category: Category::Writing,
        label: "settings.cursor_recenter_on_click",
        description: Some("settings.cursor_recenter_on_click_description"),
        control: Control::Switch(Key::CursorRecenterOnClick),
    },
    RowSpec {
        category: Category::Writing,
        label: "settings.stats",
        description: Some("settings.stats_description"),
        control: Control::Switch(Key::ShowStats),
    },
    RowSpec {
        category: Category::Preview,
        label: "settings.view_mode",
        description: Some("settings.view_mode_description"),
        control: Control::ViewMode,
    },
    RowSpec {
        category: Category::Preview,
        label: "settings.preview_zoom",
        description: Some("settings.preview_zoom_description"),
        control: Control::Value(Key::PreviewZoom),
    },
    RowSpec {
        category: Category::Preview,
        label: "settings.scroll_sync",
        description: Some("settings.scroll_sync_description"),
        control: Control::Switch(Key::ScrollSync),
    },
    RowSpec {
        category: Category::Files,
        label: "settings_sheet.notes_folder",
        description: Some("settings_sheet.notes_folder_description"),
        control: Control::NotesFolder,
    },
];

/// Sheet state.
pub(crate) struct SettingsState {
    pub(crate) category: Category,
    pub(crate) search: Entity<InputState>,
}

impl SettingsState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Shell>) -> (Self, Subscription) {
        let search = cx.new(|cx| InputState::new(window, cx));
        let sub = cx.subscribe(&search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Change) {
                cx.notify();
            }
        });
        (
            Self {
                category: Category::default(),
                search,
            },
            sub,
        )
    }

    /// Focus the search field.
    pub(crate) fn open(&mut self, window: &mut Window, cx: &mut Context<Shell>) {
        self.search.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
    }
}

fn value_text(value: Option<&Value>) -> String {
    match value {
        None => "—".to_owned(),
        Some(Value::Str(s)) if s.is_empty() => "—".to_owned(),
        Some(Value::Str(s)) => s.clone(),
        Some(Value::Int(i)) => i.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(v) => v.to_json().to_string(),
    }
}

impl Shell {
    fn provenance_chip(&self, source: Provenance, cx: &App) -> SharedString {
        match source {
            Provenance::Fed => i18n::t(cx, "settings_sheet.source.fed"),
            Provenance::Env(name) => {
                i18n::t_args(cx, "settings_sheet.source.env", &[("name", name)])
            }
            Provenance::Default => i18n::t(cx, "settings_sheet.source.default"),
        }
    }

    /// The sheet.
    pub(crate) fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme().clone();
        let query = self.settings.search.read(cx).value().to_lowercase();
        let config = self.core.as_ref().map(|c| c.ws.snapshot().config);
        let this = cx.entity().downgrade();

        let categories = Category::ALL.iter().map(|cat| {
            let cat = *cat;
            let selected = cat == self.settings.category && query.is_empty();
            let this = this.clone();
            div()
                .id(SharedString::from(format!(
                    "settings-category-{}",
                    cat.slug()
                )))
                .test_support()
                .h(px(28.))
                .px_3()
                .flex()
                .items_center()
                .rounded(theme.radius)
                .when(selected, |el| el.bg(theme.selection))
                .child(i18n::t(
                    cx,
                    &format!("settings_sheet.categories.{}", cat.slug()),
                ))
                .on_click(move |_, _, cx| {
                    let _ = this.update(cx, |shell, cx| {
                        shell.settings.category = cat;
                        cx.notify();
                    });
                })
        });
        let categories: Vec<_> = categories.collect();

        let mut body: Vec<AnyElement> = Vec::new();
        let rows: Vec<&RowSpec> = ROWS
            .iter()
            .filter(|r| {
                if query.is_empty() {
                    r.category == self.settings.category
                } else {
                    i18n::t(cx, r.label).to_lowercase().contains(&query)
                        || r.description
                            .is_some_and(|d| i18n::t(cx, d).to_lowercase().contains(&query))
                }
            })
            .collect();
        for row in &rows {
            body.push(
                self.render_setting_row(row, config.as_deref(), &theme, cx)
                    .into_any_element(),
            );
        }
        if query.is_empty()
            && self.settings.category == Category::Diagnostics
            && let Some(config) = config.as_deref()
        {
            {
                body.push(
                    div()
                        .pt_2()
                        .font_semibold()
                        .child(i18n::t(cx, "settings_sheet.config_sources"))
                        .into_any_element(),
                );
                for entry in config.entries() {
                    let value = if entry.sensitive && !entry.value.is_null() {
                        "****".to_owned()
                    } else {
                        entry.value.to_string()
                    };
                    let source = self.provenance_chip(entry.source, cx);
                    body.push(
                        h_flex()
                            .id(SharedString::from(format!(
                                "config-source-{}",
                                entry.key.name()
                            )))
                            .test_support()
                            .h(px(24.))
                            .gap_3()
                            .text_xs()
                            .child(div().w(px(200.)).truncate().child(entry.key.name()))
                            .child(
                                div()
                                    .flex_1()
                                    .truncate()
                                    .font_family(theme.mono_font_family.clone())
                                    .child(value),
                            )
                            .child(div().text_color(theme.muted_foreground).child(source))
                            .into_any_element(),
                    );
                }
            }
        }
        if query.is_empty() && self.settings.category == Category::About {
            body.push(
                v_flex()
                    .id("settings-about")
                    .test_support()
                    .gap_2()
                    .items_center()
                    .pt_4()
                    .child(
                        svg()
                            .path("brand/icon.svg")
                            .size(px(64.))
                            .text_color(theme.primary),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .child(i18n::t(cx, "app.name")),
                    )
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .child(i18n::t(cx, "home.description")),
                    )
                    .child(div().text_xs().child(i18n::t_args(
                        cx,
                        "settings_sheet.version",
                        &[("version", env!("CARGO_PKG_VERSION"))],
                    )))
                    .into_any_element(),
            );
        }
        if body.is_empty() {
            body.push(
                div()
                    .text_color(theme.muted_foreground)
                    .child(i18n::t(cx, "settings_sheet.no_results"))
                    .into_any_element(),
            );
        }

        let close_this = this.clone();
        v_flex()
            .id("settings-sheet")
            .test_support()
            .track_focus(&self.overlay_focus)
            .w(px(960.))
            .h(px(680.))
            .max_w(relative(1.))
            .max_h(relative(1.))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .shadow_lg()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                h_flex()
                    .h(px(44.))
                    .px_4()
                    .gap_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(div().font_semibold().child(i18n::t(cx, "settings.title")))
                    .child(div().flex_1())
                    .child(
                        div()
                            .w(px(280.))
                            .child(Input::new(&self.settings.search).id("settings-search")),
                    )
                    .child(
                        Button::new("settings-close")
                            .ghost()
                            .small()
                            .label("×")
                            .on_click(move |_, window, cx| {
                                let _ = close_this
                                    .update(cx, |shell, cx| shell.close_overlay(window, cx));
                            }),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .items_start()
                    .overflow_hidden()
                    .child(
                        v_flex()
                            .w(px(200.))
                            .h_full()
                            .p_2()
                            .gap_1()
                            .bg(theme.sidebar)
                            .children(categories),
                    )
                    .child(
                        v_flex()
                            .id("settings-rows")
                            .flex_1()
                            .h_full()
                            .p_4()
                            .gap_1()
                            .overflow_y_scroll()
                            .children(body),
                    ),
            )
    }

    fn render_setting_row(
        &self,
        row: &RowSpec,
        config: Option<&frankmd_config::ResolvedConfig>,
        theme: &gpui_kit::component::theme::Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let id_slug = row.label.rsplit('.').next().unwrap_or(row.label).to_owned();
        let label = i18n::t(cx, row.label);
        let description = row.description.map(|d| i18n::t(cx, d));
        let resolved = |key: Key| config.map(|c| c.get(key));
        let mut chip: Option<SharedString> = None;
        let control: AnyElement = match row.control {
            Control::Switch(key) => {
                let r = resolved(key);
                let on = r
                    .as_ref()
                    .and_then(|r| r.value.as_ref())
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                chip = r.map(|r| self.provenance_chip(r.source, cx));
                Switch::new(SharedString::from(format!("setting-{id_slug}")))
                    .checked(on)
                    .on_click(move |checked, window, cx| {
                        let checked = *checked;
                        let _ = this.update(cx, |shell, cx| {
                            shell.update_settings(
                                vec![(key, Some(Value::Bool(checked)))],
                                window,
                                cx,
                            )
                        });
                    })
                    .into_any_element()
            }
            Control::Picker(command) => {
                let text = match command {
                    "theme.select" => match &self.theme.omarchy_name {
                        Some(name) => {
                            i18n::t_args(cx, "palette.follow_omarchy", &[("theme", name)])
                                .to_string()
                        }
                        None => self.theme.id.clone(),
                    },
                    _ => i18n::language_name(cx, i18n::current(cx).locale()).to_string(),
                };
                if command == "theme.select" && self.theme.omarchy_name.is_some() {
                    chip = Some(i18n::t(cx, "settings_sheet.source.omarchy"));
                }
                Button::new(SharedString::from(format!("setting-{id_slug}")))
                    .small()
                    .outline()
                    .label(format!("{text} ▾"))
                    .on_click(move |_, window, cx| {
                        let _ = this.update(cx, |shell, cx| {
                            shell.overlays.close();
                            shell.run_command(command, window, cx);
                        });
                    })
                    .into_any_element()
            }
            Control::ViewMode => {
                let single = self.layout.view_mode == frankmd_workspace::layout::ViewMode::Single;
                chip = resolved(Key::ViewMode).map(|r| self.provenance_chip(r.source, cx));
                let this2 = this.clone();
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("setting-view-mode-split")
                            .small()
                            .selected(!single)
                            .label(i18n::t(cx, "settings.view_mode_split"))
                            .on_click(move |_, window, cx| {
                                let _ = this.update(cx, |shell, cx| {
                                    shell.update_settings(
                                        vec![(Key::ViewMode, Some(Value::Str("split".into())))],
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    )
                    .child(
                        Button::new("setting-view-mode-single")
                            .small()
                            .selected(single)
                            .label(i18n::t(cx, "settings.view_mode_single"))
                            .on_click(move |_, window, cx| {
                                let _ = this2.update(cx, |shell, cx| {
                                    shell.update_settings(
                                        vec![(Key::ViewMode, Some(Value::Str("single".into())))],
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    )
                    .into_any_element()
            }
            Control::Value(key) => {
                let r = resolved(key);
                chip = r.as_ref().map(|r| self.provenance_chip(r.source, cx));
                div()
                    .child(value_text(r.as_ref().and_then(|r| r.value.as_ref())))
                    .into_any_element()
            }
            Control::NotesFolder => div()
                .text_xs()
                .child(self.vault_root.display().to_string())
                .into_any_element(),
        };
        h_flex()
            .id(SharedString::from(format!("setting-row-{id_slug}")))
            .test_support()
            .min_h(px(44.))
            .gap_3()
            .child(
                v_flex()
                    .flex_1()
                    .child(div().child(label))
                    .when_some(description, |el, d| {
                        el.child(div().text_xs().text_color(theme.muted_foreground).child(d))
                    }),
            )
            .when_some(chip, |el, c| {
                el.child(
                    div()
                        .text_xs()
                        .px_1()
                        .rounded(theme.radius)
                        .bg(theme.secondary)
                        .text_color(theme.muted_foreground)
                        .child(c),
                )
            })
            .child(control)
    }
}
