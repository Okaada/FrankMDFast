//! Insert video dialog (ui-spec §5.7): Upload / URL / YouTube tabs, the Hugo
//! shortcode checkbox (kept for the session) and the storage row.

use crate::scale::px;
use std::sync::Arc;

use frankmd_media::Param;
use frankmd_media::upload_storage::Upload;
use frankmd_media::vm::Text;
use frankmd_media::vm::picker::{LocalFile, StoreError, Thumb};
use frankmd_media::vm::storage::{Provider, Ready};
use frankmd_media::vm::video::{
    Detected, GridKey, UploadState, VideoDialog, VideoEffect, VideoTab,
};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::MediaEvent;
use super::services::MediaServices;
use super::storage::{StorageEvent, StorageModel};
use super::task::{Abort, spawn_net};
use super::widgets::{self, RemoteThumbs, t, tr};
use crate::preview::remote::RemoteImages;

actions!(
    media_video,
    [
        /// ← in the YouTube results.
        GridLeft,
        /// →.
        GridRight,
        /// ↑ (a row; from the first row back to the field).
        GridUp,
        /// ↓ (from the field into the grid).
        GridDown,
    ]
);

/// The Insert video dialog view.
pub struct VideoDialogView {
    services: Arc<MediaServices>,
    storage: Entity<StorageModel>,
    vm: VideoDialog,
    open: bool,
    focus: FocusHandle,
    dropzone_focus: FocusHandle,
    query_focus: FocusHandle,
    results_focus: FocusHandle,
    url: Entity<InputState>,
    query: Entity<InputState>,
    key: Entity<InputState>,
    remote: RemoteImages,
    pending_upload: Option<Vec<LocalFile>>,
    upload: Option<(Option<Abort>, Task<()>)>,
    search: Option<(Option<Abort>, Task<()>)>,
    jobs: Vec<Task<()>>,
    syncing: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<MediaEvent> for VideoDialogView {}

impl Focusable for VideoDialogView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl RemoteThumbs for VideoDialogView {
    fn remote(&mut self) -> &mut RemoteImages {
        &mut self.remote
    }

    fn remote_done(&mut self, url: &str, ok: bool, _cx: &mut Context<Self>) {
        if !ok && self.vm.url_thumbnail().as_deref() == Some(url) {
            self.vm.url_thumbnail_failed();
        }
    }
}

/// Bind the result-grid arrows.
pub(crate) fn bind_keys(cx: &mut App) {
    let ctx = Some("MediaVideoGrid");
    cx.bind_keys([
        KeyBinding::new("left", GridLeft, ctx),
        KeyBinding::new("right", GridRight, ctx),
        KeyBinding::new("up", GridUp, ctx),
        KeyBinding::new("down", GridDown, ctx),
        KeyBinding::new("enter", super::Activate, ctx),
        KeyBinding::new("down", GridDown, Some("MediaVideoQuery")),
    ]);
}

impl VideoDialogView {
    /// Build (closed).
    pub fn new(
        services: Arc<MediaServices>,
        storage: Entity<StorageModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let url_placeholder = t(cx, "dialogs.video.url_placeholder");
        let query_placeholder = t(cx, "dialogs.video.search_placeholder");
        let key_placeholder = t(cx, "dialogs.s3_option.custom_prefix");
        let url = cx.new(|cx| InputState::new(window, cx).placeholder(url_placeholder));
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(query_placeholder));
        let key = cx.new(|cx| InputState::new(window, cx).placeholder(key_placeholder));
        let subs = vec![
            cx.subscribe_in(&storage, window, Self::on_storage),
            cx.subscribe_in(
                &url,
                window,
                |this, input, ev: &InputEvent, window, cx| match ev {
                    InputEvent::Change if !this.syncing => {
                        this.vm.set_url(&input.read(cx).value());
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => {
                        let effects = this.vm.url_enter();
                        this.run(effects, window, cx);
                    }
                    _ => {}
                },
            ),
            cx.subscribe_in(
                &query,
                window,
                |this, input, ev: &InputEvent, window, cx| match ev {
                    InputEvent::Change if !this.syncing => {
                        this.vm.set_query(&input.read(cx).value())
                    }
                    InputEvent::PressEnter { .. } => {
                        this.vm.set_query(&input.read(cx).value());
                        let effects = this.vm.search();
                        this.run(effects, window, cx);
                    }
                    _ => {}
                },
            ),
            cx.subscribe_in(&key, window, |this, input, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) && !this.syncing {
                    this.vm.row_mut().on_key_edited(&input.read(cx).value());
                }
            }),
        ];
        Self {
            services,
            storage,
            vm: VideoDialog::default(),
            open: false,
            focus: cx.focus_handle(),
            dropzone_focus: cx.focus_handle(),
            query_focus: cx.focus_handle(),
            results_focus: cx.focus_handle(),
            url,
            query,
            key,
            remote: RemoteImages::default(),
            pending_upload: None,
            upload: None,
            search: None,
            jobs: Vec::new(),
            syncing: false,
            _subscriptions: subs,
        }
    }

    /// The view model.
    pub fn vm(&self) -> &VideoDialog {
        &self.vm
    }

    /// Is the dialog open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open on Upload (the Hugo checkbox keeps its state).
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (provider, s3) = {
            let state = self.storage.read(cx).state();
            (state.provider(), state.s3_available())
        };
        self.upload = None;
        self.search = None;
        self.pending_upload = None;
        self.vm
            .open(provider, s3, false, Vec::new(), jiff::Zoned::now().date());
        self.open = true;
        self.sync_inputs(window, cx);
        self.dropzone_focus.focus(window, cx);
        // Configuration (YouTube key, video extensions) is read off-thread.
        let services = Arc::clone(&self.services);
        let work = cx.background_executor().spawn(async move {
            let config = services.media_config();
            (
                frankmd_media::youtube::enabled(&config),
                config.video_upload_extensions.clone(),
            )
        });
        self.storage.update(cx, |s, cx| s.refresh(cx));
        self.jobs.push(cx.spawn(async move |this, cx| {
            let (youtube, extensions) = work.await;
            this.update(cx, |this, cx| {
                this.vm.set_config(youtube, extensions);
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// Close without inserting (aborts uploads and searches).
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.upload = None;
        self.search = None;
        self.jobs.clear();
        cx.emit(MediaEvent::Closed);
        cx.notify();
    }

    fn on_storage(
        &mut self,
        storage: &Entity<StorageModel>,
        ev: &StorageEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (provider, s3) = {
            let state = storage.read(cx).state();
            (state.provider(), state.s3_available())
        };
        self.vm.storage_changed(provider, s3);
        match ev {
            StorageEvent::Acked { saved } => {
                if let Some(files) = self.pending_upload.take() {
                    if *saved {
                        let effects = self.vm.drop_files(files);
                        self.run(effects, window, cx);
                    } else {
                        cx.emit(MediaEvent::Status {
                            text: t(cx, "media_storage.save_failed"),
                            error: true,
                        });
                    }
                }
            }
            StorageEvent::Fallback => cx.emit(MediaEvent::StorageFallback),
            StorageEvent::Changed => {}
        }
        cx.notify();
    }

    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.syncing = true;
        let url = self.vm.url().to_owned();
        let query = self.vm.query().to_owned();
        let key = self.vm.row().key_text().to_owned();
        for (state, value) in [
            (self.url.clone(), url),
            (self.query.clone(), query),
            (self.key.clone(), key),
        ] {
            if state.read(cx).value() != value.as_str() {
                state.update(cx, |s, cx| s.set_value(value, window, cx));
            }
        }
        self.syncing = false;
    }

    /// Files dropped on the dialog (or handed over by the shell): upload the first.
    pub fn drop_local_files(
        &mut self,
        files: Vec<LocalFile>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.drop_files(files, window, cx);
    }

    fn drop_files(&mut self, files: Vec<LocalFile>, window: &mut Window, cx: &mut Context<Self>) {
        if self.storage.update(cx, |s, _| s.ready()) == Ready::Wait {
            self.pending_upload = Some(files);
            return;
        }
        let effects = self.vm.drop_files(files);
        self.run(effects, window, cx);
    }

    fn drop_paths(
        &mut self,
        paths: Vec<std::path::PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let work = cx
            .background_executor()
            .spawn(async move { super::files::describe_all(&paths) });
        self.jobs.push(cx.spawn_in(window, async move |this, cx| {
            let files = work.await;
            this.update_in(cx, |this, window, cx| this.drop_files(files, window, cx))
                .ok();
        }));
    }

    fn run(&mut self, effects: Vec<VideoEffect>, window: &mut Window, cx: &mut Context<Self>) {
        for effect in effects {
            match effect {
                VideoEffect::ChooseFile => {
                    let rx = cx.prompt_for_paths(PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: Some(t(cx, "dialogs.video.insert_video")),
                    });
                    self.jobs.push(cx.spawn_in(window, async move |this, cx| {
                        let Ok(Ok(Some(paths))) = rx.await else {
                            return;
                        };
                        this.update_in(cx, |this, window, cx| this.drop_paths(paths, window, cx))
                            .ok();
                    }));
                }
                VideoEffect::Upload { file, s3_prefix } => self.upload(file, s3_prefix, window, cx),
                VideoEffect::Search { generation, query } => {
                    let services = Arc::clone(&self.services);
                    let (abort, work) = spawn_net(cx, async move {
                        let config = services.media_config();
                        services
                            .youtube
                            .search(&config, &query)
                            .await
                            .map_err(|e| Text::key(e.i18n_key()))
                    });
                    let task = cx.spawn(async move |this, cx| {
                        let result = work
                            .await
                            .unwrap_or_else(|| Err(Text::key("status.search_failed_retry")));
                        this.update(cx, |this, cx| {
                            this.vm.search_result(generation, result);
                            cx.notify();
                        })
                        .ok();
                    });
                    self.search = Some((abort, task));
                }
                VideoEffect::RefreshStorage => self.storage.update(cx, |s, cx| s.refresh(cx)),
                VideoEffect::Insert(insert) => {
                    self.open = false;
                    self.upload = None;
                    self.search = None;
                    cx.emit(MediaEvent::Insert(insert));
                }
                VideoEffect::Close => self.close(cx),
            }
        }
        cx.notify();
    }

    fn upload(
        &mut self,
        file: LocalFile,
        s3_prefix: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let services = Arc::clone(&self.services);
        cx.emit(MediaEvent::Status {
            text: crate::ui::i18n::t_args(cx, "media_dialogs.uploading", &[("name", &file.name)]),
            error: false,
        });
        let read = cx
            .background_executor()
            .spawn(async move { super::files::read_bytes(&file).map(|b| (file.name, b)) });
        let task = cx.spawn_in(window, async move |this, cx| {
            let bytes = read.await;
            let result = match bytes {
                Err(message) => Some(Err(StoreError {
                    message,
                    fallback: false,
                })),
                Ok((name, bytes)) => {
                    let Ok((abort, work)) = cx.update(|_, cx| {
                        spawn_net(cx, async move {
                            services
                                .api
                                .media_upload(
                                    Some(Upload::new(name, bytes)),
                                    &Param::Nil,
                                    s3_prefix.as_deref(),
                                )
                                .await
                        })
                    }) else {
                        return;
                    };
                    let _abort = abort;
                    let result = work.await;
                    let Ok(mapped) = cx.update(|_, cx| match result {
                        Some(Ok(url)) => Ok(url),
                        Some(Err(e)) => Err(StoreError {
                            message: tr(cx, &Text::from_api(&e.message)).to_string(),
                            fallback: e.media_storage_fallback,
                        }),
                        None => Err(StoreError {
                            message: String::new(),
                            fallback: false,
                        }),
                    }) else {
                        return;
                    };
                    Some(mapped)
                }
            };
            this.update_in(cx, |this, window, cx| {
                if let Some(result) = result {
                    if result.is_err() {
                        cx.emit(MediaEvent::Status {
                            text: t(cx, "status.upload_failed"),
                            error: true,
                        });
                    }
                    let effects = this.vm.upload_result(result);
                    this.run(effects, window, cx);
                }
            })
            .ok();
        });
        self.upload = Some((None, task));
    }

    fn grid_key(&mut self, key: GridKey, window: &mut Window, cx: &mut Context<Self>) {
        if self.vm.grid_key(key) {
            self.results_focus.focus(window, cx);
        } else {
            self.query_focus.focus(window, cx);
            self.query.update(cx, |s, cx| s.focus(window, cx));
        }
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.close(cx);
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.vm.confirm();
        self.run(effects, window, cx);
    }
}

impl Render for VideoDialogView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().id("dialog-insert-video-closed").into_any_element();
        }
        let theme = cx.theme().clone();
        if let Some(url) = self
            .vm
            .url_thumbnail()
            .filter(|_| self.vm.tab() == VideoTab::Url)
        {
            widgets::fetch_remote(self, &url, cx);
        }
        if self.vm.tab() == VideoTab::YouTube {
            for i in 0..self.vm.results().len() {
                if let Some(url) = self.vm.result_thumbnail(i) {
                    widgets::fetch_remote(self, &url, cx);
                }
            }
        }
        let this = cx.entity().downgrade();
        let viewport = window.viewport_size();
        let tab = self.vm.tab();
        div()
            .id("dialog-insert-video")
            .test_support()
            .track_focus(&self.focus)
            .key_context(super::CONTEXT)
            .on_action(cx.listener(|this, _: &super::Confirm, window, cx| this.confirm(window, cx)))
            .on_action(cx.listener(|this, _: &super::Cancel, _, cx| this.cancel(cx)))
            .on_action(cx.listener(|this, _: &super::NextSource, window, cx| {
                this.vm.step_tab(true);
                this.focus.focus(window, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &super::PrevSource, window, cx| {
                this.vm.step_tab(false);
                this.focus.focus(window, cx);
                cx.notify();
            }))
            .w(px(720.).min(viewport.width - px(16.)))
            .h(px(520.).min(viewport.height - px(16.)))
            .flex()
            .flex_col()
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .overflow_hidden()
            .child(widgets::header(
                t(cx, "dialogs.video.title"),
                "video-close",
                &theme,
                move |_, cx| {
                    let _ = this.update(cx, |this, cx| this.close(cx));
                },
            ))
            .child(
                h_flex()
                    .gap_1()
                    .px_3()
                    .pt_2()
                    .children(VideoTab::ALL.iter().map(|candidate| {
                        let candidate = *candidate;
                        Button::new(SharedString::from(format!("video-tab-{}", candidate.id())))
                            .ghost()
                            .small()
                            .selected(candidate == tab)
                            .label(t(cx, candidate.label_key()))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.vm.set_tab(candidate);
                                this.focus.focus(window, cx);
                                cx.notify();
                            }))
                    })),
            )
            .child(
                div()
                    .id("video-panel")
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .overflow_y_scroll()
                    .child(match tab {
                        VideoTab::Upload => self.render_upload(&theme, cx).into_any_element(),
                        VideoTab::Url => self.render_url(&theme, cx).into_any_element(),
                        VideoTab::YouTube => self.render_youtube(&theme, cx),
                    }),
            )
            .child(self.render_footer(&theme, cx))
            .into_any_element()
    }
}

impl VideoDialogView {
    fn render_upload(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let accent = theme.accent;
        let state = self.vm.upload_state().clone();
        v_flex()
            .gap_2()
            .child(
                v_flex()
                    .id("video-dropzone")
                    .test_support()
                    .key_context("MediaActivate")
                    .track_focus(&self.dropzone_focus)
                    .p_6()
                    .items_center()
                    .border_2()
                    .border_dashed()
                    .border_color(theme.border)
                    .rounded(theme.radius_lg)
                    .cursor_pointer()
                    .drag_over::<ExternalPaths>(move |style, _, _, _| style.border_color(accent))
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                        this.drop_paths(paths.paths().to_vec(), window, cx);
                    }))
                    .on_action(cx.listener(|this, _: &super::Activate, window, cx| {
                        let effects = this.vm.choose_file();
                        this.run(effects, window, cx);
                    }))
                    .on_click(cx.listener(|this, _, window, cx| {
                        let effects = this.vm.choose_file();
                        this.run(effects, window, cx);
                    }))
                    .child(t(cx, "dialogs.video.drop_hint")),
            )
            .when_some(self.vm.upload_feedback().cloned(), |el, text| {
                el.child(widgets::error_line(tr(cx, &text), theme))
            })
            .child(match state {
                UploadState::Idle => div().into_any_element(),
                UploadState::Uploading { name } => v_flex()
                    .id("video-uploading")
                    .test_support()
                    .gap_1()
                    .child(Progress::new("video-upload-progress").loading(true))
                    .child(
                        div()
                            .text_sm()
                            .child(format!("{} {name}", t(cx, "dialogs.video.uploading"))),
                    )
                    .into_any_element(),
                UploadState::Done { name } => div()
                    .text_sm()
                    .child(format!("▶ {name}"))
                    .into_any_element(),
            })
    }

    fn render_url(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let preview = match self.vm.detected().cloned() {
            Some(Detected::YouTube { id }) => {
                let thumb = self.vm.url_thumbnail().map(Thumb::Url);
                h_flex()
                    .gap_3()
                    .child(
                        div()
                            .w(px(128.))
                            .h(px(72.))
                            .rounded(theme.radius)
                            .overflow_hidden()
                            .child(widgets::thumb_element(thumb.as_ref(), &self.remote, theme)),
                    )
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("YouTube Video"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("ID: {id}")),
                            ),
                    )
                    .into_any_element()
            }
            Some(Detected::File { filename, .. }) => h_flex()
                .gap_3()
                .child("▶")
                .child(
                    v_flex()
                        .child(div().font_weight(FontWeight::SEMIBOLD).child("Video File"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(filename),
                        ),
                )
                .into_any_element(),
            None => {
                let hint = self.vm.url_hint().map(|h| tr(cx, &h)).unwrap_or_default();
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(hint)
                    .into_any_element()
            }
        };
        v_flex()
            .gap_2()
            .child(div().text_sm().child(t(cx, "dialogs.video.url_label")))
            .child(
                div()
                    .id("video-url")
                    .test_support()
                    .child(Input::new(&self.url)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t(cx, "dialogs.video.url_hint_youtube")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t(cx, "dialogs.video.url_hint_file")),
            )
            .child(
                div()
                    .id("video-url-preview")
                    .test_support()
                    .p_2()
                    .rounded(theme.radius)
                    .bg(theme.secondary)
                    .child(preview),
            )
    }

    fn render_youtube(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        if !self.vm.youtube_enabled() {
            let this = cx.entity().downgrade();
            return widgets::setup_notice(
                "youtube-setup",
                t(cx, "dialogs.video.youtube_api_required"),
                vec![
                    t(cx, "dialogs.video.youtube_api_description"),
                    t(cx, "dialogs.video.setup_step_1"),
                    t(cx, "dialogs.video.setup_step_2"),
                ],
                theme,
                t(cx, "media_dialogs.open_settings"),
                move |_, cx| {
                    let _ = this.update(cx, |_, cx| cx.emit(MediaEvent::OpenSettings));
                },
            )
            .into_any_element();
        }
        let (status, error) = self.vm.status();
        let status = status.map_or_else(|| t(cx, "dialogs.video.search_status"), |s| tr(cx, s));
        let selected = self.vm.selected();
        let results: Vec<_> = self.vm.results().to_vec();
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .key_context("MediaVideoQuery")
                            .track_focus(&self.query_focus)
                            .on_action(cx.listener(|this, _: &GridDown, window, cx| {
                                this.grid_key(GridKey::Down, window, cx)
                            }))
                            .child(Input::new(&self.query).small()),
                    )
                    .child(
                        Button::new("youtube-search")
                            .small()
                            .danger()
                            .label(t(cx, "common.search"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                let effects = this.vm.search();
                                this.run(effects, window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(if error {
                        theme.danger
                    } else {
                        theme.muted_foreground
                    })
                    .child(status),
            )
            .when(self.vm.searching(), |el| {
                el.child(Progress::new("youtube-progress").loading(true))
            })
            .child(
                div()
                    .id("youtube-results")
                    .test_support()
                    .key_context("MediaVideoGrid")
                    .track_focus(&self.results_focus)
                    .on_action(cx.listener(|this, _: &GridLeft, window, cx| {
                        this.grid_key(GridKey::Left, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &GridRight, window, cx| {
                        this.grid_key(GridKey::Right, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &GridUp, window, cx| {
                        this.grid_key(GridKey::Up, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &GridDown, window, cx| {
                        this.grid_key(GridKey::Down, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &super::Activate, window, cx| {
                        if let Some(i) = this.vm.selected() {
                            let effects = this.vm.choose_result(i);
                            this.run(effects, window, cx);
                        }
                    }))
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .children(results.into_iter().enumerate().map(|(i, video)| {
                        let thumb = self.vm.result_thumbnail(i).map(Thumb::Url);
                        v_flex()
                            .id(SharedString::from(format!("youtube-result-{i}")))
                            .test_support()
                            .p_1()
                            .gap_1()
                            .rounded(theme.radius)
                            .border_2()
                            .border_color(if selected == Some(i) {
                                theme.accent
                            } else {
                                theme.border
                            })
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let effects = this.vm.choose_result(i);
                                this.run(effects, window, cx);
                            }))
                            .child(
                                div().w_full().h(px(120.)).overflow_hidden().child(
                                    widgets::thumb_element(thumb.as_ref(), &self.remote, theme),
                                ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .line_clamp(2)
                                    .child(video.title.clone().unwrap_or_default()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(video.channel.clone().unwrap_or_default()),
                            )
                    })),
            )
            .into_any_element()
    }

    fn render_footer(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let storage = self.storage.read(cx).state().clone();
        let row = self.vm.row().clone();
        let can_insert = self.vm.can_insert();
        let tab = self.vm.tab();
        v_flex()
            .gap_2()
            .p_3()
            .border_t_1()
            .border_color(theme.border)
            .when(tab == VideoTab::Upload, |el| {
                let s3_reason = storage.s3_disabled_reason();
                el.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().text_sm().child(t(cx, "media_storage.label")))
                        .child(
                            Button::new("video-storage-local")
                                .xsmall()
                                .selected(storage.provider() == Provider::Local)
                                .disabled(storage.controls_disabled())
                                .label(t(cx, "media_storage.local"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.storage
                                        .update(cx, |s, cx| s.request(Provider::Local, cx));
                                })),
                        )
                        .child(
                            Button::new("video-storage-s3")
                                .xsmall()
                                .selected(storage.provider() == Provider::S3)
                                .disabled(storage.controls_disabled() || s3_reason.is_some())
                                .when_some(s3_reason.as_ref(), |b, reason| {
                                    b.tooltip(tr(cx, reason))
                                })
                                .label(t(cx, "media_storage.s3"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.storage.update(cx, |s, cx| s.request(Provider::S3, cx));
                                })),
                        )
                        .when(row.key_visible(), |el| {
                            el.child(
                                div()
                                    .text_sm()
                                    .child(t(cx, "dialogs.s3_option.custom_prefix")),
                            )
                            .child(div().w(px(240.)).child(Input::new(&self.key).small()))
                        }),
                )
            })
            .when_some(storage.error().cloned(), |el, e| {
                el.child(widgets::error_line(tr(cx, &e), theme))
            })
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new("video-hugo")
                            .checked(self.vm.hugo())
                            .label(t(cx, "dialogs.video.use_hugo_shortcode"))
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.vm.set_hugo(*checked);
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("video-cancel")
                            .label(t(cx, "common.cancel"))
                            .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
                    )
                    .child(
                        Button::new("video-insert")
                            .primary()
                            .disabled(!can_insert)
                            .label(format!(
                                "{}  {}↵",
                                t(cx, "dialogs.video.insert_video"),
                                crate::ui::actions::ctrl_label()
                            ))
                            .on_click(cx.listener(|this, _, window, cx| this.confirm(window, cx))),
                    ),
            )
    }
}
