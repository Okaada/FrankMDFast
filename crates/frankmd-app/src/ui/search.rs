//! Search in files (ui-spec §5.3, FE-290…295): query + `Aa` / `.*` toggles,
//! debounced searches run off the UI thread, results grouped by note with
//! ±3 context lines; Enter opens at the line. The state machine is
//! `frankmd_workspace::content_search::ContentSearchModel`.

use crate::scale::px;
use std::time::Instant;

use frankmd_workspace::content_search::{
    ContentSearchModel, SearchOptions, SearchRequest, SearchStatus,
};

use super::prelude::*;
use gpui_kit::component::input::InputEvent;
use gpui_kit::*;

/// Overlay state.
pub(crate) struct SearchState {
    pub(crate) model: ContentSearchModel,
    pub(crate) input: Entity<InputState>,
    debounce: Option<Task<()>>,
    running: Option<Task<()>>,
    /// The `?` regex cheat-sheet is shown.
    pub(crate) cheatsheet: bool,
}

impl SearchState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Shell>) -> (Self, Vec<Subscription>) {
        let placeholder = i18n::t(cx, "dialogs.content_search.placeholder");
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let sub = cx.subscribe_in(&input, window, |shell, input, ev, window, cx| match ev {
            InputEvent::Change => {
                let query = input.read(cx).value().to_string();
                let now = Instant::now();
                let deadline = shell.search.model.set_query(&query, now);
                shell.arm_search(deadline, cx);
                cx.notify();
            }
            // Plain Enter arrives as `list.confirm` (the input propagates it);
            // Ctrl+Enter opens and keeps the overlay (§5.3).
            InputEvent::PressEnter {
                secondary: true, ..
            } => {
                shell.search_activate(true, window, cx);
            }
            _ => {}
        });
        (
            Self {
                model: ContentSearchModel::default(),
                input,
                debounce: None,
                running: None,
                cheatsheet: false,
            },
            vec![sub],
        )
    }
}

impl Shell {
    /// Ctrl+Shift+F: open (the query resets; toggles are kept).
    pub(crate) fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.model.open();
        self.search.debounce = None;
        self.search.running = None;
        self.open_overlay(OverlayKind::SearchInFiles, window, cx);
        let input = self.search.input.clone();
        input.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
    }

    /// The search view model (tests read status and results).
    pub fn content_search(&self) -> &ContentSearchModel {
        &self.search.model
    }

    /// Run the debounce: wake at `deadline`, then search.
    fn arm_search(&mut self, deadline: Option<Instant>, cx: &mut Context<Self>) {
        let Some(deadline) = deadline else {
            self.search.debounce = None;
            return;
        };
        let delay = deadline.saturating_duration_since(Instant::now());
        self.search.debounce = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |shell, cx| {
                // The debounce can fire a hair early on coarse timers; the
                // model's own deadline decides.
                let request = shell
                    .search
                    .model
                    .tick(Instant::now())
                    .or_else(|| shell.search.model.search_now());
                if let Some(request) = request {
                    shell.run_search(request, cx);
                }
            });
        }));
    }

    /// Search now (Enter before the debounce, toggles, tests).
    pub(crate) fn search_now(&mut self, cx: &mut Context<Self>) {
        self.search.debounce = None;
        if let Some(request) = self.search.model.search_now() {
            self.run_search(request, cx);
        }
    }

    fn run_search(&mut self, request: SearchRequest, cx: &mut Context<Self>) {
        let Some(root) = self
            .core
            .as_ref()
            .map(|c| c.vault.real_root().to_path_buf())
        else {
            return;
        };
        let SearchRequest { generation, query } = request;
        self.search.running = Some(cx.spawn(async move |this, cx| {
            let results = cx
                .background_executor()
                .spawn(async move { frankmd_search::content::search(&root, &query) })
                .await;
            let _ = this.update(cx, |shell, cx| {
                shell.search.model.apply_results(generation, results);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn set_search_options(&mut self, options: SearchOptions, cx: &mut Context<Self>) {
        let deadline = self.search.model.set_options(options, Instant::now());
        if deadline.is_some() {
            self.search_now(cx);
        }
        cx.notify();
    }

    /// Enter: open the selected match at its line; Ctrl+Enter keeps the
    /// overlay open to walk the results (§5.3).
    pub(crate) fn search_activate(
        &mut self,
        keep_open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.search.model.results().is_empty() {
            // Enter before the debounce fired: search now.
            self.search_now(cx);
            return;
        }
        let Some(target) = self.search.model.activate() else {
            return;
        };
        if !keep_open {
            self.overlays.close();
        }
        self.open_at_line(&target.path, target.line as usize, window, cx);
        if keep_open {
            if !self.overlays.is_open(&OverlayKind::SearchInFiles) {
                self.open_overlay(OverlayKind::SearchInFiles, window, cx);
            }
            let input = self.search.input.clone();
            input.update(cx, |input, cx| input.focus(window, cx));
        }
        cx.notify();
    }

    fn status_text(&self, cx: &App) -> SharedString {
        match self.search.model.status() {
            SearchStatus::Hint => i18n::t(cx, "dialogs.content_search.status"),
            SearchStatus::Searching => i18n::t(cx, "search.searching"),
            SearchStatus::NoMatches => i18n::t(cx, "search.no_matches"),
            SearchStatus::Error => i18n::t(cx, "errors.search_failed"),
            SearchStatus::Count {
                matches,
                notes,
                truncated,
            } => {
                let matches_text = i18n::t_count(
                    cx,
                    "search.matches",
                    i64::try_from(matches).unwrap_or(i64::MAX),
                );
                let notes_text =
                    i18n::t_count(cx, "search.notes", i64::try_from(notes).unwrap_or(i64::MAX));
                let base = i18n::t_args(
                    cx,
                    "search.count",
                    &[("matches", &matches_text), ("notes", &notes_text)],
                );
                if truncated {
                    let more =
                        i18n::t_args(cx, "search.truncated", &[("count", &matches.to_string())]);
                    format!("{base} {more}").into()
                } else {
                    base
                }
            }
        }
    }

    pub(super) fn render_search(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme().clone();
        let this = cx.entity().downgrade();
        let options = self.search.model.options();
        let status = self.status_text(cx);
        let invalid = self.search.model.invalid_pattern();
        let searching = self.search.model.searching();
        let selected = self.search.model.selected();
        let hits = self.search.model.results().to_vec();
        let groups = self.search.model.groups();
        let toggle = |id: &'static str,
                      label: &'static str,
                      tooltip: &str,
                      on: bool,
                      cx: &mut Context<Self>| {
            let this = this.clone();
            Button::new(id)
                .ghost()
                .xsmall()
                .selected(on)
                .label(label)
                .tooltip(i18n::t(cx, tooltip))
                .on_click(move |_, _, cx| {
                    let _ = this.update(cx, |shell, cx| {
                        let mut o = shell.search.model.options();
                        if id == "search-case" {
                            o.case_sensitive = !o.case_sensitive;
                        } else {
                            o.regex = !o.regex;
                        }
                        shell.set_search_options(o, cx);
                    });
                })
        };
        let case = toggle(
            "search-case",
            "Aa",
            "search.case_sensitive",
            options.case_sensitive,
            cx,
        );
        let regex = toggle("search-regex", ".*", "search.regex", options.regex, cx);
        let help = {
            let this = this.clone();
            Button::new("search-help")
                .ghost()
                .xsmall()
                .selected(self.search.cheatsheet)
                .icon(crate::ui::icons::ph("question"))
                .on_click(move |_, _, cx| {
                    let _ = this.update(cx, |shell, cx| {
                        shell.search.cheatsheet = !shell.search.cheatsheet;
                        cx.notify();
                    });
                })
        };
        let mut body: Vec<AnyElement> = Vec::new();
        for group in groups {
            body.push(
                h_flex()
                    .h(px(28.))
                    .px_3()
                    .gap_2()
                    .child(div().font_semibold().child(group.name.clone()))
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(group.path.trim_end_matches(".md").to_owned()),
                    )
                    .into_any_element(),
            );
            for ix in group.hits {
                let Some(hit) = hits.get(ix) else { continue };
                let is_selected = selected == Some(ix);
                let this = this.clone();
                let lines = hit.context.iter().map(|line| {
                    h_flex()
                        .gap_2()
                        .when(line.is_match, |el| el.bg(theme.selection))
                        .child(
                            div()
                                .w(px(40.))
                                .flex_shrink_0()
                                .text_right()
                                .text_color(theme.muted_foreground)
                                .child(line.line_number.to_string()),
                        )
                        .child(div().flex_1().truncate().child(line.content.clone()))
                });
                body.push(
                    v_flex()
                        .id(SharedString::from(format!(
                            "search-result-{}-{}",
                            hit.path, hit.line_number
                        )))
                        .test_support()
                        .mx_2()
                        .mb_1()
                        .p_1()
                        .rounded(theme.radius)
                        .font_family(theme.mono_font_family.clone())
                        .text_xs()
                        .bg(theme.secondary)
                        .when(is_selected, |el| {
                            el.border_l_2()
                                .border_color(theme.primary)
                                .bg(theme.selection.opacity(0.5))
                        })
                        .children(lines.collect::<Vec<_>>())
                        .on_click(move |_, window, cx| {
                            let _ = this.update(cx, |shell, cx| {
                                shell.search.model.select(ix);
                                shell.search_activate(false, window, cx);
                            });
                        })
                        .into_any_element(),
                );
            }
        }
        let cheatsheet = self.search.cheatsheet.then(|| {
            let rows = [
                ("hello", "dialogs.content_search.literal_text"),
                ("hello|world", "dialogs.content_search.or_operator"),
                ("hel+o", "dialogs.content_search.one_or_more"),
                ("hel.*o", "dialogs.content_search.any_chars"),
                ("^hello", "dialogs.content_search.start_of_line"),
                ("hello$", "dialogs.content_search.end_of_line"),
                ("\\d+", "dialogs.content_search.digits"),
                ("\\bword\\b", "dialogs.content_search.whole_word"),
            ];
            v_flex()
                .id("search-cheatsheet")
                .test_support()
                .px_3()
                .py_2()
                .gap_1()
                .text_xs()
                .border_b_1()
                .border_color(theme.border)
                .children(rows.into_iter().map(|(pattern, key)| {
                    h_flex()
                        .gap_3()
                        .child(
                            div()
                                .w(px(96.))
                                .font_family(theme.mono_font_family.clone())
                                .child(pattern),
                        )
                        .child(
                            div()
                                .text_color(theme.muted_foreground)
                                .child(i18n::t(cx, key)),
                        )
                }))
        });
        let down = this.clone();
        let up = this.clone();
        v_flex()
            .id("search-overlay")
            .test_support()
            .key_context(actions::CX_OVERLAY)
            .w(px(760.))
            .max_w(relative(1.))
            .max_h(px(560.))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .shadow_lg()
            .overflow_hidden()
            .capture_action(move |_: &gpui_kit::component::input::MoveDown, _, cx| {
                let _ = down.update(cx, |shell, cx| {
                    shell.search.model.select_next();
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .capture_action(move |_: &gpui_kit::component::input::MoveUp, _, cx| {
                let _ = up.update(cx, |shell, cx| {
                    shell.search.model.select_prev();
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .child(
                h_flex()
                    .h(px(40.))
                    .px_3()
                    .gap_1()
                    .border_b_1()
                    .border_color(if searching {
                        theme.primary
                    } else {
                        theme.border
                    })
                    .child(
                        div().flex_1().child(
                            Input::new(&self.search.input)
                                .id("search-input")
                                .appearance(false),
                        ),
                    )
                    .child(case)
                    .child(regex)
                    .child(help),
            )
            .children(cheatsheet)
            .child(
                h_flex()
                    .id("search-status")
                    .test_support()
                    .h(px(24.))
                    .px_3()
                    .gap_2()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(status)
                    .when(invalid, |el| {
                        el.child(
                            div()
                                .text_color(theme.warning)
                                .child(i18n::t(cx, "search.invalid_pattern")),
                        )
                    }),
            )
            .child(
                div()
                    .id("search-results")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pb_2()
                    .children(body),
            )
    }
}
