//! Help dialog (ui-spec §5.18).

use super::prelude::*;
use gpui_kit::*;

impl Shell {
    pub(super) fn render_help(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
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
}
