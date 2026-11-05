//! Image viewer (lightbox, ui-spec §5.20): full-window overlay with the
//! image fitted inside 32 px margins, the alt text as caption and an `i/n`
//! counter. Left/Right move between the note's images, `+`/`-`/`0` zoom,
//! Esc or a click outside closes; focus returns to the image that opened it
//! (or the pane when a re-render removed it).

use crate::scale::px;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::base::TestTextExt as _;
use gpui_kit::{
    AnyElement, Context, FocusHandle, InteractiveElement as _, IntoElement, KeyDownEvent,
    ObjectFit, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
    StyledImage as _, Window, accesskit::Role, anchored, deferred, div, img, point,
};

use super::PreviewPane;

/// Viewer state.
pub(super) struct Viewer {
    /// Ordinal of the image shown.
    pub index: usize,
    /// Zoom factor (1 = fitted).
    pub zoom: f32,
    /// The viewer's own focus.
    pub focus: FocusHandle,
}

const MARGIN: f32 = 32.0;
const ZOOM_STEP: f32 = 1.25;

impl PreviewPane {
    /// Open the viewer on image `ordinal` and move focus into it.
    pub(super) fn open_viewer(
        &mut self,
        ordinal: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ordinal >= self.layout.images.len() {
            return;
        }
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        self.viewer = Some(Viewer {
            index: ordinal,
            zoom: 1.0,
            focus,
        });
        cx.notify();
    }

    /// Close the viewer, returning focus to its image (or the pane).
    pub(super) fn close_viewer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(v) = self.viewer.take() else { return };
        match self.image_focus.get(v.index) {
            Some(handle) if v.index < self.layout.images.len() => handle.focus(window, cx),
            _ => self.focus.focus(window, cx),
        }
        cx.notify();
    }

    fn viewer_key(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.layout.images.len();
        let Some(v) = self.viewer.as_mut() else {
            return;
        };
        match e.keystroke.key.as_str() {
            "escape" => {
                self.close_viewer(window, cx);
            }
            "left" if count > 0 => {
                v.index = (v.index + count - 1) % count;
                v.zoom = 1.0;
            }
            "right" if count > 0 => {
                v.index = (v.index + 1) % count;
                v.zoom = 1.0;
            }
            "+" | "=" => v.zoom = (v.zoom * ZOOM_STEP).min(8.0),
            "-" => v.zoom = (v.zoom / ZOOM_STEP).max(1.0),
            "0" => v.zoom = 1.0,
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub(super) fn render_viewer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some((index, zoom, focus)) = self
            .viewer
            .as_ref()
            .map(|v| (v.index, v.zoom, v.focus.clone()))
        else {
            return div().into_any_element();
        };
        let colors = self.colors.clone();
        let viewport = window.viewport_size();
        let (src, alt) = self.layout.images.get(index).cloned().unwrap_or_default();
        let count = self.layout.images.len();
        let source = self.image_source(src.as_deref(), cx);
        let max_w = (viewport.width - px(2. * MARGIN)).max(px(1.));
        let max_h = (viewport.height - px(2. * MARGIN) - px(40.)).max(px(1.));
        let error = self.t("preview.image_viewer.error", "Unable to load this image.");
        let picture: AnyElement = match source {
            Some(source) => {
                let err = error.clone();
                let fallback_color = colors.muted;
                img(source)
                    .max_w(max_w * zoom)
                    .max_h(max_h * zoom)
                    .object_fit(ObjectFit::Contain)
                    .with_fallback(move || {
                        div()
                            .text_color(fallback_color)
                            .child(err.clone())
                            .into_any_element()
                    })
                    .test_alt_text(alt.clone())
                    .into_any_element()
            }
            None => div()
                .text_color(colors.muted)
                .child(error.clone().test_text(error))
                .into_any_element(),
        };
        let caption = if count > 1 {
            format!("{alt}  {}/{count}", index + 1)
        } else {
            alt.clone()
        };
        let title = self.t("preview.image_viewer.title", "Image preview");
        deferred(
            anchored().position(point(px(0.), px(0.))).child(
                div()
                    .id("image-viewer")
                    .role(Role::Dialog)
                    .aria_label(title)
                    .test_support()
                    .track_focus(&focus)
                    .occlude()
                    .w(viewport.width)
                    .h(viewport.height)
                    .bg(colors.background.opacity(0.92))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .p(px(MARGIN))
                    .on_key_down(
                        cx.listener(|this, e: &KeyDownEvent, w, cx| this.viewer_key(e, w, cx)),
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.close_viewer(window, cx)))
                    .child(
                        div()
                            .id("image-viewer-image")
                            .overflow_scroll()
                            .max_w(max_w)
                            .max_h(max_h)
                            // Clicks on the image itself do not close.
                            .on_click(|_, _, cx| cx.stop_propagation())
                            .child(picture),
                    )
                    .child(
                        div()
                            .text_color(colors.text_secondary)
                            .child(SharedString::from(caption.clone()).test_text(caption)),
                    ),
            ),
        )
        .with_priority(1)
        .into_any_element()
    }
}
