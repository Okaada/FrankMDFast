//! Root view: explorer | editor | preview (placeholder layout).

use gpui_kit::component::input::{Editor, EditorState, InputEvent};
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::{ActiveTheme as _, TitleBar, v_flex};
use gpui_kit::*;

const SAMPLE: &str = "# FrankMD\n\nNative **GPUI** scaffold.\n\n- [ ] explorer\n- [ ] editor\n- [ ] preview\n";

/// Top-level workspace view.
pub(crate) struct WorkspaceView {
    editor: Entity<EditorState>,
    _subscriptions: Vec<Subscription>,
}

impl WorkspaceView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .line_number(false)
                .soft_wrap(true)
                .default_value(SAMPLE)
        });
        let sub = cx.subscribe(&editor, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Change) {
                cx.notify();
            }
        });
        Self { editor, _subscriptions: vec![sub] }
    }

    fn explorer(&self, cx: &App) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_2()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .child("Explorer")
    }

    fn preview(&self, cx: &App) -> impl IntoElement {
        // Placeholder: the real preview renders frankmd_markdown::RenderDoc.
        let text = self.editor.read(cx).value();
        div()
            .id("preview")
            .size_full()
            .p_4()
            .overflow_y_scroll()
            .child(text)
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(TitleBar::new().child("FrankMD"))
            .child(
                div().flex_1().overflow_hidden().child(
                    h_resizable("frankmd-panes")
                        .child(resizable_panel().size(px(240.)).child(self.explorer(cx)))
                        .child(
                            resizable_panel().child(
                                Editor::new(&self.editor).h(relative(1.)).appearance(false),
                            ),
                        )
                        .child(resizable_panel().child(self.preview(cx))),
                ),
            )
    }
}
