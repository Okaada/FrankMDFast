//! Rendering helpers shared by the media dialogs: text resolution, thumbnails
//! (local files, in-memory bytes, egress-guarded remote URLs), dialog chrome.

use std::sync::Arc;

use frankmd_media::vm::Text;
use frankmd_media::vm::picker::Thumb;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use crate::preview::remote::{self, RemoteImages, RemoteState};
use crate::ui::i18n;

/// Resolve a [`Text`] (arguments may be keys themselves).
pub(crate) fn tr(cx: &App, text: &Text) -> SharedString {
    match text {
        Text::Raw(raw) => raw.clone().into(),
        Text::Key(key, args) => {
            let resolved: Vec<(&str, String)> = args.iter().map(|(k, v)| (*k, tr(cx, v).to_string())).collect();
            let refs: Vec<(&str, &str)> = resolved.iter().map(|(k, v)| (*k, v.as_str())).collect();
            i18n::t_args(cx, key, &refs)
        }
    }
}

/// Translate a key.
pub(crate) fn t(cx: &App, key: &str) -> SharedString {
    i18n::t(cx, key)
}

/// A view that shows remote thumbnails.
pub(crate) trait RemoteThumbs: 'static + Sized {
    /// The cache.
    fn remote(&mut self) -> &mut RemoteImages;
    /// A fetch finished (`ok = false` → broken thumbnail).
    fn remote_done(&mut self, url: &str, ok: bool, cx: &mut Context<Self>);
}

/// Start fetching `url` through the egress client (once per URL).
pub(crate) fn fetch_remote<V: RemoteThumbs>(view: &mut V, url: &str, cx: &mut Context<V>) {
    if !view.remote().begin(url) {
        return;
    }
    let Some(client) = remote::client().cloned() else {
        view.remote().finish(url, RemoteState::Unavailable);
        view.remote_done(url, false, cx);
        return;
    };
    let fetch_url = url.to_owned();
    let (abort, task) = super::task::spawn_net(cx, async move { client.fetch(&fetch_url).await });
    let url = url.to_owned();
    cx.spawn(async move |this, cx| {
        let _abort = abort;
        let state = match task.await {
            Some(Ok(fetched)) => {
                let bytes = fetched.bytes;
                cx.background_executor().spawn(async move { remote::decode(bytes) }).await
            }
            _ => RemoteState::Unavailable,
        };
        let ok = matches!(state, RemoteState::Ready(_));
        this.update(cx, |this, cx| {
            this.remote().finish(&url, state);
            this.remote_done(&url, ok, cx);
            cx.notify();
        })
        .ok();
    })
    .detach();
}

/// A thumbnail element (cover-fit). Remote URLs come from the cache only.
pub(crate) fn thumb_element(thumb: Option<&Thumb>, cache: &RemoteImages, theme: &Theme) -> AnyElement {
    let placeholder = || div().size_full().bg(theme.secondary).into_any_element();
    match thumb {
        Some(Thumb::File(path)) => img(path.clone())
            .size_full()
            .object_fit(ObjectFit::Cover)
            .with_fallback(move || div().size_full().into_any_element())
            .into_any_element(),
        Some(Thumb::Bytes(bytes)) => match remote::sniff(bytes) {
            Some(format) => img(Arc::new(Image::from_bytes(format, bytes.as_ref().clone())))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => placeholder(),
        },
        Some(Thumb::Url(url)) => match cache.get(url) {
            Some(RemoteState::Ready(image)) => {
                img(image).size_full().object_fit(ObjectFit::Cover).into_any_element()
            }
            _ => placeholder(),
        },
        None => placeholder(),
    }
}

/// Dialog header: title + close button.
pub(crate) fn header(title: SharedString, close_id: &'static str, theme: &Theme, on_close: impl Fn(&mut Window, &mut App) + 'static) -> Div {
    h_flex()
        .h(px(40.))
        .px_4()
        .justify_between()
        .border_b_1()
        .border_color(theme.border)
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(
            gpui_kit::component::button::Button::new(close_id)
                .label("×")
                .on_click(move |_, window, cx| on_close(window, cx)),
        )
}

/// A setup notice (source or provider not configured): title, explanation,
/// `.fed` lines and an "Open Settings" button.
pub(crate) fn setup_notice(
    id: &'static str,
    title: SharedString,
    lines: Vec<SharedString>,
    theme: &Theme,
    open_settings: SharedString,
    on_settings: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    v_flex()
        .id(id)
        .test_support()
        .gap_2()
        .p_4()
        .rounded(theme.radius)
        .bg(theme.secondary)
        .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
        .children(lines.into_iter().map(|line| {
            div().text_sm().text_color(theme.muted_foreground).font_family(theme.mono_font_family.clone()).child(line)
        }))
        .child(
            gpui_kit::component::button::Button::new(SharedString::from(format!("{id}-settings")))
                .label(open_settings)
                .on_click(move |_, window, cx| on_settings(window, cx)),
        )
        .into_any_element()
}

/// An inline error line (`danger`).
pub(crate) fn error_line(text: SharedString, theme: &Theme) -> Div {
    div().text_sm().text_color(theme.danger).child(text)
}
