//! Insert image dialog (ui-spec §5.6): source rail, per-source panels, shared
//! footer with alt/link and the storage row. Renders
//! [`frankmd_media::vm::picker::ImagePicker`] and runs its effects.

use crate::scale::px;
use std::collections::HashMap;
use std::sync::Arc;

use frankmd_media::Param;
use frankmd_media::api::Base64Upload;
use frankmd_media::import::ImportRequest;
use frankmd_media::library::{Library, PickerCollection, PickerQuery};
use frankmd_media::upload_storage::Upload;
use frankmd_media::vm::Text;
use frankmd_media::vm::picker::{
    AiResult, Busy, Engine, ImageCard, ImagePicker, LocalFile, PickerConfig, PickerEffect, Source,
    StoreError, StoreRequest, Thumb,
};
use frankmd_media::vm::storage::{KeyMode, Provider, Ready};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::services::MediaServices;
use super::storage::{StorageEvent, StorageModel};
use super::task::{Abort, spawn_net};
use super::widgets::{self, RemoteThumbs, t, tr};
use super::{MediaEvent, MediaInsert};
use crate::preview::remote::RemoteImages;

/// A running job; dropping it aborts the work.
struct Job {
    _abort: Option<Abort>,
    _task: Task<()>,
}

/// The Insert image dialog view.
pub struct ImagePickerDialog {
    services: Arc<MediaServices>,
    storage: Entity<StorageModel>,
    vm: ImagePicker,
    config: PickerConfig,
    open: bool,
    focus: FocusHandle,
    rail_focus: FocusHandle,
    dropzone_focus: FocusHandle,
    alt: Entity<InputState>,
    link: Entity<InputState>,
    key: Entity<InputState>,
    lib_search: Entity<InputState>,
    folder_search: Entity<InputState>,
    web_query: Entity<InputState>,
    google_query: Entity<InputState>,
    pinterest_query: Entity<InputState>,
    prompt: Entity<TextareaState>,
    ref_search: Entity<InputState>,
    remote: RemoteImages,
    google_scroll: ScrollHandle,
    jobs: HashMap<&'static str, Job>,
    debounce: Option<Task<()>>,
    syncing: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<MediaEvent> for ImagePickerDialog {}

impl Focusable for ImagePickerDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl RemoteThumbs for ImagePickerDialog {
    fn remote(&mut self) -> &mut RemoteImages {
        &mut self.remote
    }

    fn remote_done(&mut self, url: &str, ok: bool, _cx: &mut Context<Self>) {
        if ok {
            return;
        }
        for engine in [Engine::Web, Engine::Google, Engine::Pinterest] {
            while let Some(card) = self
                .vm
                .search_cards(engine)
                .into_iter()
                .find(|c| c.thumb == Some(Thumb::Url(url.to_owned())))
            {
                self.vm.thumbnail_failed(engine, &card.key);
            }
        }
    }
}

impl ImagePickerDialog {
    /// Build (closed).
    pub fn new(
        services: Arc<MediaServices>,
        storage: Entity<StorageModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = |placeholder: &str, window: &mut Window, cx: &mut Context<Self>| {
            let placeholder = t(cx, placeholder);
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let alt = input("dialogs.image_picker.alt_placeholder", window, cx);
        let link = input("dialogs.image_picker.link_placeholder", window, cx);
        let key = input("dialogs.s3_option.custom_key", window, cx);
        let lib_search = input("dialogs.image_picker.search_images_placeholder", window, cx);
        let folder_search = input("dialogs.image_picker.search_images_placeholder", window, cx);
        let web_query = input("dialogs.image_picker.search_web_placeholder", window, cx);
        let google_query = input("dialogs.image_picker.search_google_placeholder", window, cx);
        let pinterest_query = input(
            "dialogs.image_picker.search_pinterest_placeholder",
            window,
            cx,
        );
        let ref_search = input(
            "dialogs.image_picker.search_reference_placeholder",
            window,
            cx,
        );
        let prompt_placeholder = t(cx, "dialogs.image_picker.prompt_placeholder");
        let prompt = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(2)
                .submit_on_enter(true)
                .placeholder(prompt_placeholder)
        });

        let mut subs = vec![cx.subscribe_in(&storage, window, Self::on_storage)];
        subs.push(
            cx.subscribe_in(&alt, window, |this, input, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) && !this.syncing {
                    this.vm.set_alt(&input.read(cx).value());
                }
            }),
        );
        subs.push(
            cx.subscribe_in(&link, window, |this, input, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) && !this.syncing {
                    this.vm.set_link(&input.read(cx).value());
                }
            }),
        );
        subs.push(
            cx.subscribe_in(&key, window, |this, input, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) && !this.syncing {
                    this.vm.row_mut().on_key_edited(&input.read(cx).value());
                }
            }),
        );
        subs.push(cx.subscribe_in(
            &lib_search,
            window,
            |this, input, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) && !this.syncing {
                    let text = input.read(cx).value().to_string();
                    this.debounced(window, cx, move |this, window, cx| {
                        let effects = this.vm.set_library_search(&text);
                        this.run(effects, window, cx);
                    });
                }
            },
        ));
        subs.push(cx.subscribe_in(
            &folder_search,
            window,
            |this, input, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) && !this.syncing {
                    let text = input.read(cx).value().to_string();
                    this.debounced(window, cx, move |this, _, cx| {
                        this.vm.set_folder_search(&text);
                        cx.notify();
                    });
                }
            },
        ));
        subs.push(cx.subscribe_in(
            &ref_search,
            window,
            |this, input, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Change) && !this.syncing {
                    let text = input.read(cx).value().to_string();
                    this.debounced(window, cx, move |this, window, cx| {
                        let effects = this.vm.set_ref_search(&text);
                        this.run(effects, window, cx);
                    });
                }
            },
        ));
        for (state, engine) in [
            (&web_query, Engine::Web),
            (&google_query, Engine::Google),
            (&pinterest_query, Engine::Pinterest),
        ] {
            subs.push(cx.subscribe_in(
                state,
                window,
                move |this, input, ev: &InputEvent, window, cx| match ev {
                    InputEvent::Change if !this.syncing => {
                        this.vm.set_query(engine, &input.read(cx).value())
                    }
                    InputEvent::PressEnter { .. } => {
                        this.vm.set_query(engine, &input.read(cx).value());
                        let effects = this.vm.search(engine);
                        this.run(effects, window, cx);
                    }
                    _ => {}
                },
            ));
        }
        subs.push(cx.subscribe_in(
            &prompt,
            window,
            |this, input, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change if !this.syncing => this.vm.set_prompt(&input.read(cx).value()),
                InputEvent::PressEnter { shift: false, .. } => {
                    this.vm.set_prompt(&input.read(cx).value());
                    let effects = this.vm.generate();
                    this.run(effects, window, cx);
                }
                _ => {}
            },
        ));

        Self {
            services,
            storage,
            vm: ImagePicker::default(),
            config: PickerConfig::default(),
            open: false,
            focus: cx.focus_handle(),
            rail_focus: cx.focus_handle(),
            dropzone_focus: cx.focus_handle(),
            alt,
            link,
            key,
            lib_search,
            folder_search,
            web_query,
            google_query,
            pinterest_query,
            prompt,
            ref_search,
            remote: RemoteImages::default(),
            google_scroll: ScrollHandle::new(),
            jobs: HashMap::new(),
            debounce: None,
            syncing: false,
            _subscriptions: subs,
        }
    }

    /// The view model (tests, shell state queries).
    pub fn vm(&self) -> &ImagePicker {
        &self.vm
    }

    /// Is the dialog open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open on Upload for the note at `note` (vault path, for note-relative links).
    pub fn open(&mut self, note: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let (provider, s3) = {
            let state = self.storage.read(cx).state();
            (state.provider(), state.s3_available())
        };
        let today = jiff::Zoned::now().date();
        self.jobs.clear();
        self.remote = RemoteImages::default();
        self.vm.open(self.config.clone(), note, provider, s3, today);
        self.open = true;
        self.sync_inputs(window, cx);
        self.dropzone_focus.focus(window, cx);
        self.refresh_config(cx);
        cx.notify();
    }

    /// Paste / editor drop: open with `files` in Upload, the first one selected.
    pub fn open_with_files(
        &mut self,
        note: Option<String>,
        files: Vec<LocalFile>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open(note, window, cx);
        self.vm.ingest_external(files);
        self.sync_inputs(window, cx);
        cx.notify();
    }

    /// Open with OS paths dropped on the editor (described off-thread).
    pub fn open_with_paths(
        &mut self,
        note: Option<String>,
        paths: Vec<std::path::PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open(note, window, cx);
        let work = cx
            .background_executor()
            .spawn(async move { super::files::describe_all(&paths) });
        let task = cx.spawn_in(window, async move |this, cx| {
            let files = work.await;
            this.update_in(cx, |this, window, cx| {
                this.vm.ingest_external(files);
                this.sync_inputs(window, cx);
                cx.notify();
            })
            .ok();
        });
        self.jobs.insert(
            "describe",
            Job {
                _abort: None,
                _task: task,
            },
        );
    }

    /// Close without inserting (Esc, Cancel, ×). Running work is aborted.
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.jobs.clear();
        self.debounce = None;
        cx.emit(MediaEvent::Closed);
        cx.notify();
    }

    fn refresh_config(&mut self, cx: &mut Context<Self>) {
        let services = Arc::clone(&self.services);
        let work = cx
            .background_executor()
            .spawn(async move { services.picker_config() });
        let storage = self.storage.clone();
        let task = cx.spawn(async move |this, cx| {
            let (config, snapshot) = work.await;
            storage.update(cx, |s, cx| s.apply(snapshot, cx));
            this.update(cx, |this, cx| {
                this.config = config.clone();
                this.vm.set_config(config);
                cx.notify();
            })
            .ok();
        });
        self.jobs.insert(
            "config",
            Job {
                _abort: None,
                _task: task,
            },
        );
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
        match ev {
            StorageEvent::Changed => self.vm.storage_changed(provider, s3),
            StorageEvent::Acked { saved } => {
                self.vm.storage_changed(provider, s3);
                let effects = self.vm.resume_insert(*saved);
                self.run(effects, window, cx);
            }
            StorageEvent::Fallback => cx.emit(MediaEvent::StorageFallback),
        }
        self.sync_inputs(window, cx);
        cx.notify();
    }

    fn debounced(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.debounce = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(300))
                .await;
            this.update_in(cx, |this, window, cx| {
                f(this, window, cx);
                cx.notify();
            })
            .ok();
        }));
    }

    /// Mirror VM text into the inputs (after open / selection).
    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.syncing = true;
        let alt = self.vm.alt().to_owned();
        let link = self.vm.link().to_owned();
        let key = self.vm.row().key_text().to_owned();
        let set = |state: &Entity<InputState>,
                   value: String,
                   window: &mut Window,
                   cx: &mut Context<Self>| {
            if state.read(cx).value() != value.as_str() {
                state.update(cx, |s, cx| s.set_value(value, window, cx));
            }
        };
        set(&self.alt, alt, window, cx);
        set(&self.link, link, window, cx);
        set(&self.key, key, window, cx);
        if !self.open {
            self.syncing = false;
            return;
        }
        if self.vm.library_search().is_empty() {
            set(&self.lib_search, String::new(), window, cx);
        }
        if self.vm.folder_search().is_empty() {
            set(&self.folder_search, String::new(), window, cx);
        }
        for (state, engine) in [
            (self.web_query.clone(), Engine::Web),
            (self.google_query.clone(), Engine::Google),
            (self.pinterest_query.clone(), Engine::Pinterest),
        ] {
            let q = self.vm.query(engine).to_owned();
            set(&state, q, window, cx);
        }
        let prompt = self.vm.prompt().to_owned();
        if self.prompt.read(cx).value() != prompt.as_str() {
            self.prompt
                .update(cx, |s, cx| s.set_value(prompt, window, cx));
        }
        self.syncing = false;
    }

    // ----- effects ---------------------------------------------------------------------

    fn run(&mut self, effects: Vec<PickerEffect>, window: &mut Window, cx: &mut Context<Self>) {
        for effect in effects {
            self.run_one(effect, window, cx);
        }
        self.sync_inputs(window, cx);
        cx.notify();
    }

    fn run_one(&mut self, effect: PickerEffect, window: &mut Window, cx: &mut Context<Self>) {
        let services = Arc::clone(&self.services);
        match effect {
            PickerEffect::ChooseFiles => {
                let rx = cx.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: true,
                    prompt: Some(t(cx, "dialogs.image_picker.insert_image")),
                });
                let task = cx.spawn_in(window, async move |this, cx| {
                    let Ok(Ok(Some(paths))) = rx.await else {
                        return;
                    };
                    let files = cx
                        .background_executor()
                        .spawn(async move { super::files::describe_all(&paths) })
                        .await;
                    this.update_in(cx, |this, window, cx| {
                        this.vm.ingest(files);
                        this.sync_inputs(window, cx);
                        cx.notify();
                    })
                    .ok();
                });
                self.jobs.insert(
                    "choose",
                    Job {
                        _abort: None,
                        _task: task,
                    },
                );
            }
            PickerEffect::ChooseFolder => {
                let rx = cx.prompt_for_paths(PathPromptOptions {
                    files: false,
                    directories: true,
                    multiple: false,
                    prompt: Some(t(cx, "dialogs.image_picker.browse_folder")),
                });
                let task = cx.spawn_in(window, async move |this, cx| {
                    let result = match rx.await {
                        Ok(Ok(Some(mut paths))) => Ok(paths.pop()),
                        Ok(Ok(None)) => Ok(None),
                        Ok(Err(err)) => Err(Text::Raw(err.to_string())),
                        Err(_) => Ok(None),
                    };
                    this.update_in(cx, |this, window, cx| {
                        let effects = this.vm.folder_chosen(result);
                        this.run(effects, window, cx);
                    })
                    .ok();
                });
                self.jobs.insert(
                    "choose",
                    Job {
                        _abort: None,
                        _task: task,
                    },
                );
            }
            PickerEffect::ReadFolder { generation, dir } => {
                let work = cx
                    .background_executor()
                    .spawn(async move { super::files::list_folder(&dir) });
                let task = cx.spawn(async move |this, cx| {
                    let result = work.await.map_err(|e| Text::Raw(e.to_string()));
                    this.update(cx, |this, cx| {
                        this.vm.folder_listing(generation, result);
                        cx.notify();
                    })
                    .ok();
                });
                self.jobs.insert(
                    "folder",
                    Job {
                        _abort: None,
                        _task: task,
                    },
                );
            }
            PickerEffect::LoadLibrary {
                generation,
                collection,
                search,
                limit,
                offset,
            } => {
                let work = cx.background_executor().spawn(async move {
                    let config = services.media_config();
                    let query = PickerQuery {
                        collection: Param::str(match collection {
                            PickerCollection::Managed => "managed",
                            PickerCollection::Server => "server",
                        }),
                        search: Param::str(search),
                        limit: Param::Int(i64::try_from(limit).unwrap_or(10)),
                        offset: Param::Int(i64::try_from(offset).unwrap_or(0)),
                    };
                    frankmd_media::library::picker_list(&services.vault, &config, &query)
                        .map_err(|_| Text::key("dialogs.image_picker.image_load_failed"))
                });
                let task = cx.spawn(async move |this, cx| {
                    let result = work.await;
                    this.update(cx, |this, cx| {
                        this.vm.library_result(generation, result);
                        cx.notify();
                    })
                    .ok();
                });
                self.jobs.insert(
                    "library",
                    Job {
                        _abort: None,
                        _task: task,
                    },
                );
            }
            PickerEffect::DeleteImage { collection, path } => {
                let (abort, work) = spawn_net(cx, async move {
                    match collection {
                        PickerCollection::Managed => {
                            Library::new(&services.vault, &services.api.env)
                                .with_recovery(&services.api.recovery)
                                .delete(&path)
                                .await
                                .map_err(|e| {
                                    e.i18n_key().map_or(
                                        Text::key("dialogs.image_picker.delete_failed"),
                                        Text::key,
                                    )
                                })
                        }
                        PickerCollection::Server => services
                            .api
                            .destroy(&path)
                            .map_err(|_| Text::key("dialogs.image_picker.image_delete_failed")),
                    }
                });
                let task = cx.spawn_in(window, async move |this, cx| {
                    let result = work
                        .await
                        .unwrap_or_else(|| Err(Text::key("dialogs.image_picker.delete_failed")));
                    this.update_in(cx, |this, window, cx| {
                        let effects = this.vm.delete_result(result);
                        this.run(effects, window, cx);
                    })
                    .ok();
                });
                self.jobs.insert(
                    "delete",
                    Job {
                        _abort: abort,
                        _task: task,
                    },
                );
            }
            PickerEffect::Search {
                generation,
                engine,
                query,
                start,
            } => {
                let (abort, work) = spawn_net(cx, async move {
                    let result = match engine {
                        Engine::Web => services.search.search_web(&query).await,
                        Engine::Pinterest => services.search.search_pinterest(&query).await,
                        Engine::Google => {
                            let config = services.media_config();
                            services.search.search_google(&config, &query, start).await
                        }
                    };
                    result.map_err(|e| Text::key(e.i18n_key()))
                });
                let task = cx.spawn(async move |this, cx| {
                    let result = work
                        .await
                        .unwrap_or_else(|| Err(Text::key("dialogs.image_picker.search_failed")));
                    this.update(cx, |this, cx| {
                        this.vm.search_result(engine, generation, start, result);
                        cx.notify();
                    })
                    .ok();
                });
                self.jobs.insert(
                    engine_job(engine),
                    Job {
                        _abort: abort,
                        _task: task,
                    },
                );
            }
            PickerEffect::LoadRefImages { search } => {
                let work = cx.background_executor().spawn(async move {
                    let search = if search.is_empty() {
                        None
                    } else {
                        Some(search.as_str())
                    };
                    services
                        .api
                        .index(search, &Param::Nil, &Param::Nil)
                        .map(|index| {
                            index
                                .images
                                .into_iter()
                                .map(|i| (i.path, i.name))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                });
                let task = cx.spawn(async move |this, cx| {
                    let images = work.await;
                    this.update(cx, |this, cx| {
                        this.vm.ref_images_result(images);
                        cx.notify();
                    })
                    .ok();
                });
                self.jobs.insert(
                    "refs",
                    Job {
                        _abort: None,
                        _task: task,
                    },
                );
            }
            PickerEffect::Generate {
                generation,
                prompt,
                reference,
            } => {
                let (abort, work) = spawn_net(cx, async move {
                    let service = services.ai();
                    let refs = ServerRefs(Arc::clone(&services));
                    frankmd_ai::generate_image_request(
                        &service,
                        &prompt,
                        reference.as_deref(),
                        &refs,
                    )
                    .await
                });
                let task = cx.spawn(async move |this, cx| {
                    let result = work.await;
                    this.update(cx, |this, cx| {
                        let result = match result {
                            Some(Ok(image)) => Ok(AiResult {
                                data: image.data,
                                mime: image.mime_type,
                                model: image.model,
                                revised_prompt: None,
                            }),
                            Some(Err(frankmd_ai::AiRequestError::BadRequest(key))) => {
                                Err(Text::key(key))
                            }
                            Some(Err(frankmd_ai::AiRequestError::Service(e))) => {
                                Err(Text::Raw(e.to_string()))
                            }
                            _ => Err(Text::key("dialogs.image_picker.ai_generation_failed")),
                        };
                        this.vm
                            .generate_result(generation, result, super::files::now_ms() as i64);
                        cx.notify();
                    })
                    .ok();
                });
                self.jobs.insert(
                    "generate",
                    Job {
                        _abort: abort,
                        _task: task,
                    },
                );
            }
            PickerEffect::CancelGenerate => {
                self.jobs.remove("generate");
            }
            PickerEffect::Store(request) => self.store(request, window, cx),
            PickerEffect::RefreshStorage => self.storage.update(cx, |s, cx| s.refresh(cx)),
            PickerEffect::Insert(result) => {
                self.open = false;
                self.jobs.clear();
                cx.emit(MediaEvent::Insert(MediaInsert::Image(result.markdown)));
            }
        }
    }

    fn store(&mut self, request: StoreRequest, window: &mut Window, cx: &mut Context<Self>) {
        let services = Arc::clone(&self.services);
        let name = match &request {
            StoreRequest::Upload { file, .. } => file.name.clone(),
            StoreRequest::ImportUrl { url, .. } => url.rsplit('/').next().unwrap_or("").to_owned(),
            StoreRequest::ImportPath { path, .. } => path.clone(),
            StoreRequest::Base64 { filename, .. } => filename.clone(),
        };
        cx.emit(MediaEvent::Status {
            text: i18n_upload_status(cx, &name),
            error: false,
        });
        let float = |r: Option<f64>| r.map_or(Param::Nil, Param::Float);
        // Read local bytes off the UI thread first (cap checked before reading).
        let read = cx.background_executor().spawn(async move {
            match request {
                StoreRequest::Upload {
                    file,
                    resize,
                    s3_key,
                } => super::files::read_bytes(&file)
                    .map(|bytes| (Some((file.name, bytes, resize, s3_key)), None)),
                other => Ok((None, Some(other))),
            }
        });
        let task = cx.spawn_in(window, async move |this, cx| {
            let prepared = read.await;
            let (bytes, other) = match prepared {
                Ok(parts) => parts,
                Err(message) => {
                    this.update_in(cx, |this, window, cx| {
                        let effects = this.vm.store_result(Err(StoreError {
                            message,
                            fallback: false,
                        }));
                        this.run(effects, window, cx);
                    })
                    .ok();
                    return;
                }
            };
            let (abort, work) = match cx.update(|_, cx| {
                spawn_net(cx, async move {
                    if let Some((name, bytes, resize, s3_key)) = bytes {
                        return services
                            .api
                            .upload(
                                Some(Upload::new(name, bytes)),
                                &float(resize),
                                &Param::Nil,
                                s3_key.as_deref(),
                            )
                            .await;
                    }
                    match other {
                        Some(StoreRequest::ImportUrl {
                            url,
                            resize,
                            s3_prefix,
                        }) => {
                            let req = ImportRequest {
                                url: Param::str(url),
                                resize,
                                s3_prefix,
                                ..ImportRequest::default()
                            };
                            services.api.import(&req).await
                        }
                        Some(StoreRequest::ImportPath {
                            path,
                            resize,
                            s3_key,
                        }) => {
                            let req = ImportRequest {
                                path: Param::str(path),
                                resize,
                                s3_key,
                                ..ImportRequest::default()
                            };
                            services.api.import(&req).await
                        }
                        Some(StoreRequest::Base64 {
                            data,
                            mime,
                            filename,
                            s3_prefix,
                        }) => {
                            let req = Base64Upload {
                                data,
                                mime_type: mime,
                                filename,
                                s3_prefix,
                                ..Base64Upload::default()
                            };
                            services.api.upload_base64(&req).await
                        }
                        _ => Ok(String::new()),
                    }
                })
            }) {
                Ok(pair) => pair,
                Err(_) => return,
            };
            let _abort = abort;
            let result = work.await;
            this.update_in(cx, |this, window, cx| {
                let result = match result {
                    Some(Ok(url)) => Ok(url),
                    Some(Err(error)) => Err(StoreError {
                        message: tr(cx, &Text::from_api(&error.message)).to_string(),
                        fallback: error.media_storage_fallback,
                    }),
                    None => Err(StoreError {
                        message: t(cx, "errors.upload_failed").to_string(),
                        fallback: false,
                    }),
                };
                let failed = result.is_err();
                if failed {
                    cx.emit(MediaEvent::Status {
                        text: t(cx, "dialogs.image_picker.image_upload_failed"),
                        error: true,
                    });
                }
                let effects = this.vm.store_result(result);
                this.run(effects, window, cx);
            })
            .ok();
        });
        self.jobs.insert(
            "store",
            Job {
                _abort: None,
                _task: task,
            },
        );
    }

    // ----- actions ---------------------------------------------------------------------

    fn insert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ready = self.storage.update(cx, |s, _| s.ready()) == Ready::Go;
        let effects = self.vm.insert(ready);
        self.run(effects, window, cx);
    }

    fn select_source(&mut self, source: Source, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.vm.select_source(source);
        self.run(effects, window, cx);
        self.keep_focus(window, cx);
    }

    fn step(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.vm.step_source(forward);
        self.run(effects, window, cx);
        self.keep_focus(window, cx);
    }

    /// The focused panel element may be gone after a source switch: move
    /// focus to the rail (ui-spec §5.6) so the dialog's keys keep working.
    fn keep_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.rail_focus.focus(window, cx);
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        if self.vm.generating() {
            let _ = self.vm.cancel_generate();
            self.jobs.remove("generate");
            cx.notify();
            return;
        }
        if self.vm.pending_delete().is_some() {
            self.vm.confirm_delete(false);
            cx.notify();
            return;
        }
        self.close(cx);
    }

    fn set_provider(&mut self, provider: Provider, cx: &mut Context<Self>) {
        self.storage.update(cx, |s, cx| s.request(provider, cx));
    }
}

/// `images_path` reference images for AI generation.
struct ServerRefs(Arc<MediaServices>);

impl frankmd_ai::ReferenceImages for ServerRefs {
    fn find(&self, rel_path: &str) -> Option<frankmd_ai::ReferenceImage> {
        let path = self.0.api.preview(rel_path).ok()?;
        let bytes = std::fs::read(&path).ok()?;
        let mime = frankmd_media::images::content_type_for(&path.to_string_lossy()).to_owned();
        Some(frankmd_ai::ReferenceImage {
            bytes,
            mime_type: mime,
        })
    }
}

fn engine_job(engine: Engine) -> &'static str {
    match engine {
        Engine::Web => "search-web",
        Engine::Google => "search-google",
        Engine::Pinterest => "search-pinterest",
    }
}

fn i18n_upload_status(cx: &App, name: &str) -> SharedString {
    crate::ui::i18n::t_args(cx, "media_dialogs.uploading", &[("name", name)])
}

// ----- rendering --------------------------------------------------------------------------

impl Render for ImagePickerDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().id("dialog-insert-image-closed").into_any_element();
        }
        let theme = cx.theme().clone();
        // Remote thumbnails of the visible search source.
        if let Some(engine) = self.vm.source().engine() {
            for card in self.vm.search_cards(engine) {
                if let Some(Thumb::Url(url)) = &card.thumb {
                    widgets::fetch_remote(self, url, cx);
                }
            }
        }
        let this = cx.entity().downgrade();
        let viewport = window.viewport_size();
        let width = px(880.).min(viewport.width - px(16.));
        let height = px(600.).min(viewport.height - px(16.));
        div()
            .id("dialog-insert-image")
            .test_support()
            .track_focus(&self.focus)
            .key_context(super::CONTEXT)
            .on_action(cx.listener(|this, _: &super::Confirm, window, cx| this.insert(window, cx)))
            .on_action(cx.listener(|this, _: &super::Cancel, _, cx| this.cancel(cx)))
            .on_action(
                cx.listener(|this, _: &super::NextSource, window, cx| this.step(true, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &super::PrevSource, window, cx| this.step(false, window, cx)),
            )
            .on_action(cx.listener(|this, _: &super::Source1, window, cx| {
                this.select_source(Source::ALL[0], window, cx)
            }))
            .on_action(cx.listener(|this, _: &super::Source2, window, cx| {
                this.select_source(Source::ALL[1], window, cx)
            }))
            .on_action(cx.listener(|this, _: &super::Source3, window, cx| {
                this.select_source(Source::ALL[2], window, cx)
            }))
            .on_action(cx.listener(|this, _: &super::Source4, window, cx| {
                this.select_source(Source::ALL[3], window, cx)
            }))
            .on_action(cx.listener(|this, _: &super::Source5, window, cx| {
                this.select_source(Source::ALL[4], window, cx)
            }))
            .on_action(cx.listener(|this, _: &super::Source6, window, cx| {
                this.select_source(Source::ALL[5], window, cx)
            }))
            .on_action(cx.listener(|this, _: &super::Source7, window, cx| {
                this.select_source(Source::ALL[6], window, cx)
            }))
            .w(width)
            .h(height)
            .flex()
            .flex_col()
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .overflow_hidden()
            .child(widgets::header(
                t(cx, "dialogs.image_picker.title"),
                "picker-close",
                &theme,
                move |_, cx| {
                    let _ = this.update(cx, |this, cx| this.close(cx));
                },
            ))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .child(self.render_rail(&theme, cx))
                    .child(
                        div()
                            .id("picker-panel")
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .p_3()
                            .overflow_y_scroll()
                            .child(self.render_panel(&theme, window, cx)),
                    ),
            )
            .child(self.render_footer(&theme, cx))
            .into_any_element()
    }
}

impl ImagePickerDialog {
    fn render_rail(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let active = self.vm.source();
        v_flex()
            .id("picker-rail")
            .test_support()
            .key_context("MediaRail")
            .track_focus(&self.rail_focus)
            .w(px(160.))
            .h_full()
            .p_2()
            .gap_1()
            .border_r_1()
            .border_color(theme.border)
            .children(Source::ALL.iter().map(|source| {
                let source = *source;
                let setup = self.config.needs_setup(source);
                let label = t(cx, source.label_key());
                Button::new(SharedString::from(format!("picker-source-{}", source.id())))
                    .ghost()
                    .small()
                    .w_full()
                    .selected(source == active)
                    .label(if setup {
                        SharedString::from(format!("{label}  · {}", t(cx, "media_dialogs.set_up")))
                    } else {
                        label
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_source(source, window, cx);
                        this.rail_focus.focus(window, cx);
                    }))
            }))
    }

    fn render_panel(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.vm.source() {
            Source::Upload => self.render_upload(theme, cx).into_any_element(),
            Source::Library => self.render_library(theme, cx).into_any_element(),
            Source::Folder => self.render_folder(theme, cx).into_any_element(),
            Source::Web => self
                .render_search(Engine::Web, theme, window, cx)
                .into_any_element(),
            Source::Google => self
                .render_search(Engine::Google, theme, window, cx)
                .into_any_element(),
            Source::Pinterest => self
                .render_search(Engine::Pinterest, theme, window, cx)
                .into_any_element(),
            Source::Ai => self.render_ai(theme, cx).into_any_element(),
        }
    }

    fn grid(
        &self,
        prefix: &'static str,
        cards: Vec<ImageCard>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Div {
        let remote = &self.remote;
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(cards.into_iter().map(|card| {
                let key = card.key.clone();
                let border = if card.selected {
                    theme.accent
                } else {
                    theme.border
                };
                v_flex()
                    .id(SharedString::from(format!(
                        "picker-card-{prefix}-{}",
                        card.key
                    )))
                    .test_support()
                    .w(px(120.))
                    .gap_1()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.vm.select_card(&key);
                        this.sync_inputs(window, cx);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .relative()
                            .size(px(120.))
                            .rounded(theme.radius)
                            .overflow_hidden()
                            .border_2()
                            .border_color(border)
                            .child(widgets::thumb_element(card.thumb.as_ref(), remote, theme))
                            .when_some(card.badge.clone(), |el, badge| {
                                el.child(
                                    div()
                                        .absolute()
                                        .bottom_1()
                                        .left_1()
                                        .px_1()
                                        .rounded_sm()
                                        .text_xs()
                                        .bg(theme.background.opacity(0.8))
                                        .child(badge),
                                )
                            }),
                    )
                    .child(div().text_xs().truncate().child(card.label.clone()))
                    .when_some(card.caption.clone(), |el, caption| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .truncate()
                                .child(caption),
                        )
                    })
            }))
    }

    fn render_upload(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let hover = self.vm.dropzone_hover();
        let border = if hover { theme.accent } else { theme.border };
        let accent = theme.accent;
        v_flex()
            .gap_2()
            .child(
                v_flex()
                    .id("upload-dropzone")
                    .test_support()
                    .key_context("MediaActivate")
                    .track_focus(&self.dropzone_focus)
                    .p_6()
                    .items_center()
                    .justify_center()
                    .border_2()
                    .border_dashed()
                    .border_color(border)
                    .rounded(theme.radius_lg)
                    .cursor_pointer()
                    .drag_over::<ExternalPaths>(move |style, _, _, _| style.border_color(accent))
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                        let paths = paths.paths().to_vec();
                        let work = cx
                            .background_executor()
                            .spawn(async move { super::files::describe_all(&paths) });
                        let task = cx.spawn_in(window, async move |this, cx| {
                            let files = work.await;
                            this.update_in(cx, |this, window, cx| {
                                this.vm.ingest(files);
                                this.sync_inputs(window, cx);
                                cx.notify();
                            })
                            .ok();
                        });
                        this.jobs.insert(
                            "describe",
                            Job {
                                _abort: None,
                                _task: task,
                            },
                        );
                    }))
                    .on_action(cx.listener(|this, _: &super::Activate, window, cx| {
                        let effects = this.vm.choose_files();
                        this.run(effects, window, cx);
                    }))
                    .on_click(cx.listener(|this, _, window, cx| {
                        let effects = this.vm.choose_files();
                        this.run(effects, window, cx);
                    }))
                    .child(t(cx, "media_dialogs.drop_images_hint")),
            )
            .children(
                self.vm
                    .upload_rejections()
                    .iter()
                    .map(|r| widgets::error_line(tr(cx, r), theme)),
            )
            .when_some(self.vm.upload_status(), |el, status| {
                el.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(tr(cx, &status)),
                )
            })
            .child(self.grid("upload", self.vm.upload_cards(), theme, cx))
    }

    fn render_library(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let collection = self.vm.collection();
        let server_disabled = self.vm.server_collection_disabled_reason();
        let per_page = self.vm.library_per_page();
        let selected_path = (self.vm.selected_source() == Some(Source::Library))
            .then(|| {
                self.vm
                    .library_cards()
                    .into_iter()
                    .find(|c| c.selected)
                    .map(|c| c.key)
            })
            .flatten();
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("library-collection-managed")
                            .small()
                            .selected(collection == PickerCollection::Managed)
                            .label(t(cx, "media_storage.managed_images"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                let effects = this.vm.set_collection(PickerCollection::Managed);
                                this.run(effects, window, cx);
                            })),
                    )
                    .child(
                        Button::new("library-collection-server")
                            .small()
                            .selected(collection == PickerCollection::Server)
                            .disabled(server_disabled.is_some())
                            .when_some(server_disabled.as_ref(), |b, reason| {
                                b.tooltip(tr(cx, reason))
                            })
                            .label(t(cx, "media_storage.server_images"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                let effects = this.vm.set_collection(PickerCollection::Server);
                                this.run(effects, window, cx);
                            })),
                    )
                    .child(
                        div()
                            .w(px(220.))
                            .child(Input::new(&self.lib_search).small()),
                    )
                    .children([10usize, 25, 50].map(|n| {
                        Button::new(SharedString::from(format!("library-per-page-{n}")))
                            .xsmall()
                            .selected(per_page == n)
                            .label(n.to_string())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let effects = this.vm.set_library_per_page(n);
                                this.run(effects, window, cx);
                            }))
                    })),
            )
            .when_some(self.vm.library_error().cloned(), |el, e| {
                el.child(widgets::error_line(tr(cx, &e), theme))
            })
            .when_some(
                self.vm
                    .pending_delete()
                    .map(|(p, n)| (p.to_owned(), n.to_owned())),
                |el, (_, name)| {
                    el.child(
                        h_flex()
                            .id("library-delete-confirm")
                            .test_support()
                            .gap_2()
                            .p_2()
                            .rounded(theme.radius)
                            .bg(theme.secondary)
                            .child(div().flex_1().text_sm().child(crate::ui::i18n::t_args(
                                cx,
                                "dialogs.image_picker.delete_confirm",
                                &[("name", &name)],
                            )))
                            .child(
                                Button::new("library-delete-cancel")
                                    .small()
                                    .label(t(cx, "common.cancel"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.vm.confirm_delete(false);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("library-delete-accept")
                                    .small()
                                    .danger()
                                    .label(t(cx, "common.delete"))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        let effects = this.vm.confirm_delete(true);
                                        this.run(effects, window, cx);
                                    })),
                            ),
                    )
                },
            )
            .when(self.vm.library_loading(), |el| {
                el.child(Progress::new("library-loading").loading(true))
            })
            .when(
                !self.vm.library_loading()
                    && self.vm.library_cards().is_empty()
                    && self.vm.library_error().is_none(),
                |el| {
                    el.child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "dialogs.image_picker.no_images_found")),
                    )
                },
            )
            .child(self.grid("library", self.vm.library_cards(), theme, cx))
            .when_some(selected_path, |el, path| {
                el.child(
                    Button::new("library-delete")
                        .small()
                        .danger()
                        .label(t(cx, "dialogs.image_picker.delete_image"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.vm.request_delete(&path);
                            cx.notify();
                        })),
                )
            })
            .when_some(
                self.vm.library_pager(),
                |el, (from, to, total, prev, next)| {
                    el.child(self.pager(
                        "library",
                        from,
                        to,
                        total,
                        prev,
                        next,
                        cx,
                        |this, next, window, cx| {
                            let effects = this.vm.library_page(next);
                            this.run(effects, window, cx);
                        },
                    ))
                },
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn pager(
        &self,
        id: &'static str,
        from: usize,
        to: usize,
        total: usize,
        prev: bool,
        next: bool,
        cx: &mut Context<Self>,
        go: fn(&mut Self, bool, &mut Window, &mut Context<Self>),
    ) -> Div {
        let status = crate::ui::i18n::t_args(
            cx,
            "dialogs.image_picker.page_status",
            &[
                ("from", &from.to_string()),
                ("to", &to.to_string()),
                ("total", &total.to_string()),
            ],
        );
        h_flex()
            .gap_2()
            .items_center()
            .child(
                Button::new(SharedString::from(format!("{id}-prev")))
                    .small()
                    .disabled(!prev)
                    .label(t(cx, "dialogs.image_picker.prev_page"))
                    .on_click(cx.listener(move |this, _, window, cx| go(this, false, window, cx))),
            )
            .child(div().text_sm().child(status))
            .child(
                Button::new(SharedString::from(format!("{id}-next")))
                    .small()
                    .disabled(!next)
                    .label(t(cx, "dialogs.image_picker.next_page"))
                    .on_click(cx.listener(move |this, _, window, cx| go(this, true, window, cx))),
            )
    }

    fn render_folder(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let dir = self.vm.folder_dir().map(|d| d.display().to_string());
        let per_page = self.vm.folder_per_page();
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("folder-choose")
                            .small()
                            .label(if dir.is_some() {
                                t(cx, "dialogs.image_picker.change_folder")
                            } else {
                                t(cx, "dialogs.image_picker.browse_folder")
                            })
                            .on_click(cx.listener(|this, _, window, cx| {
                                let effects = this.vm.choose_folder();
                                this.run(effects, window, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .truncate()
                            .child(dir.clone().map_or_else(
                                || t(cx, "dialogs.image_picker.select_folder_prompt"),
                                SharedString::from,
                            )),
                    ),
            )
            .when(dir.is_some(), |el| {
                el.child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .w(px(220.))
                                .child(Input::new(&self.folder_search).small()),
                        )
                        .children([10usize, 25, 50].map(|n| {
                            Button::new(SharedString::from(format!("folder-per-page-{n}")))
                                .xsmall()
                                .selected(per_page == n)
                                .label(n.to_string())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.vm.set_folder_per_page(n);
                                    cx.notify();
                                }))
                        })),
                )
            })
            .when_some(self.vm.folder_error().cloned(), |el, e| {
                el.child(widgets::error_line(tr(cx, &e), theme))
            })
            .when(self.vm.folder_empty(), |el| {
                el.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(t(cx, "dialogs.image_picker.no_images_in_folder")),
                )
            })
            .child(self.grid("folder", self.vm.folder_cards(), theme, cx))
            .when_some(
                self.vm.folder_pager(),
                |el, (from, to, total, prev, next)| {
                    el.child(self.pager(
                        "folder",
                        from,
                        to,
                        total,
                        prev,
                        next,
                        cx,
                        |this, next, _, cx| {
                            this.vm.folder_page(next);
                            cx.notify();
                        },
                    ))
                },
            )
    }

    fn render_search(
        &mut self,
        engine: Engine,
        theme: &Theme,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if engine == Engine::Google && !self.config.google_enabled {
            return widgets::setup_notice(
                "google-setup",
                t(cx, "dialogs.image_picker.google_api_required"),
                vec![
                    t(cx, "dialogs.image_picker.google_api_description"),
                    t(cx, "dialogs.image_picker.google_setup_step_2"),
                    t(cx, "dialogs.image_picker.google_setup_step_3"),
                ],
                theme,
                t(cx, "media_dialogs.open_settings"),
                {
                    let this = cx.entity().downgrade();
                    move |_, cx| {
                        let _ = this.update(cx, |_, cx| cx.emit(MediaEvent::OpenSettings));
                    }
                },
            )
            .into_any_element();
        }
        let input = match engine {
            Engine::Web => self.web_query.clone(),
            Engine::Google => self.google_query.clone(),
            Engine::Pinterest => self.pinterest_query.clone(),
        };
        let (status, is_error) = self.vm.search_status(engine);
        let status = status.map_or_else(
            || t(cx, "dialogs.image_picker.search_status"),
            |s| tr(cx, s),
        );
        let searching = self.vm.searching(engine);
        let search_button =
            Button::new(SharedString::from(format!("search-{}", engine_job(engine))))
                .small()
                .label(t(cx, "common.search"))
                .disabled(searching)
                .on_click(cx.listener(move |this, _, window, cx| {
                    let effects = this.vm.search(engine);
                    this.run(effects, window, cx);
                }));
        let search_button = if engine == Engine::Pinterest {
            search_button.danger()
        } else {
            search_button.primary()
        };
        let grid = self.grid(engine_job(engine), self.vm.search_cards(engine), theme, cx);
        let scroll = self.google_scroll.clone();
        v_flex()
            .gap_2()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&input).small()))
                    .child(search_button),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(if is_error {
                        theme.danger
                    } else {
                        theme.muted_foreground
                    })
                    .child(status),
            )
            .when(searching, |el| {
                el.child(Progress::new("search-progress").loading(true))
            })
            .child(if engine == Engine::Google {
                div()
                    .id("google-results")
                    .flex_1()
                    .max_h(px(380.))
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .on_scroll(cx.listener(move |this, _, window, cx| {
                        let offset = -scroll.offset().y;
                        let max = scroll.max_offset().y;
                        if max - offset < px(100.) {
                            let effects = this.vm.load_more(Engine::Google);
                            this.run(effects, window, cx);
                        }
                    }))
                    .child(grid)
                    .into_any_element()
            } else {
                grid.into_any_element()
            })
            .into_any_element()
    }

    fn render_ai(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        if !self.config.ai_enabled {
            return widgets::setup_notice(
                "ai-setup",
                t(cx, "media_dialogs.ai_image_required"),
                vec![
                    t(cx, "media_dialogs.ai_image_description"),
                    t(cx, "media_dialogs.ai_image_step"),
                ],
                theme,
                t(cx, "media_dialogs.open_settings"),
                {
                    let this = cx.entity().downgrade();
                    move |_, cx| {
                        let _ = this.update(cx, |_, cx| cx.emit(MediaEvent::OpenSettings));
                    }
                },
            )
            .into_any_element();
        }
        let model = self.config.ai_model.clone();
        let generating = self.vm.generating();
        let reference = self
            .vm
            .reference()
            .map(|(p, n)| (p.to_owned(), n.to_owned()));
        let result = self.vm.ai_result().cloned();
        let refs = self.vm.ref_cards();
        v_flex()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "{} {model}",
                        t(cx, "dialogs.image_picker.model_label")
                    )),
            )
            .when(self.config.server_images, |el| {
                el.child(
                    div()
                        .text_sm()
                        .child(t(cx, "dialogs.image_picker.reference_image_label")),
                )
                .child(match reference.clone() {
                    Some((_, name)) => h_flex()
                        .gap_2()
                        .child(div().text_sm().child(name))
                        .child(
                            Button::new("ai-remove-reference")
                                .xsmall()
                                .label(t(cx, "dialogs.image_picker.remove_reference"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    let effects = this.vm.clear_ref();
                                    this.run(effects, window, cx);
                                })),
                        )
                        .into_any_element(),
                    None => v_flex()
                        .gap_1()
                        .child(
                            div()
                                .w(px(220.))
                                .child(Input::new(&self.ref_search).small()),
                        )
                        .child(if refs.is_empty() {
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(t(cx, "dialogs.image_picker.no_images_found"))
                        } else {
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_1()
                                .children(refs.into_iter().map(|card| {
                                    let path = card.key.clone();
                                    div()
                                        .id(SharedString::from(format!("ai-ref-{}", card.key)))
                                        .size(px(56.))
                                        .rounded(theme.radius)
                                        .overflow_hidden()
                                        .border_1()
                                        .border_color(if card.selected {
                                            theme.accent
                                        } else {
                                            theme.border
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.vm.choose_ref(&path);
                                            cx.notify();
                                        }))
                                        .child(widgets::thumb_element(
                                            card.thumb.as_ref(),
                                            &self.remote,
                                            theme,
                                        ))
                                }))
                        })
                        .into_any_element(),
                })
            })
            .child(
                div()
                    .text_sm()
                    .child(t(cx, "dialogs.image_picker.prompt_label")),
            )
            .child(Textarea::new(&self.prompt))
            .child(
                Button::new("ai-generate")
                    .primary()
                    .small()
                    .disabled(generating)
                    .label(t(cx, "dialogs.image_picker.generate_image"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        let prompt = this.prompt.read(cx).value().to_string();
                        this.vm.set_prompt(&prompt);
                        let effects = this.vm.generate();
                        this.run(effects, window, cx);
                    })),
            )
            .when(generating, |el| {
                el.child(
                    v_flex()
                        .id("ai-generating")
                        .test_support()
                        .gap_1()
                        .child(Progress::new("ai-progress").loading(true))
                        .child(div().text_sm().child(format!(
                            "{} {model}  ·  {}",
                            t(cx, "dialogs.image_picker.generating_with"),
                            t(cx, "media_dialogs.esc_to_cancel")
                        ))),
                )
            })
            .when_some(self.vm.ai_error().cloned(), |el, e| {
                el.child(widgets::error_line(tr(cx, &e), theme))
            })
            .when_some(result, |el, result| {
                let bytes = result.decoded();
                el.child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .child(t(cx, "dialogs.image_picker.generated_image")),
                        )
                        .child(div().w(px(200.)).h(px(150.)).child(widgets::thumb_element(
                            bytes.map(|b| Thumb::Bytes(Arc::new(b))).as_ref(),
                            &self.remote,
                            theme,
                        )))
                        .when_some(result.revised_prompt.clone(), |el, revised| {
                            el.child(div().text_xs().text_color(theme.muted_foreground).child(
                                format!(
                                    "{}: {revised}",
                                    t(cx, "dialogs.image_picker.revised_prompt")
                                ),
                            ))
                        }),
                )
            })
            .into_any_element()
    }

    fn render_footer(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let storage = self.storage.read(cx).state().clone();
        let row = self.vm.row().clone();
        let can_insert = self.vm.can_insert();
        let busy = self.vm.busy();
        let selected = self.vm.selected_name().map(str::to_owned);
        let row_visible = self.vm.storage_row_visible();
        v_flex()
            .gap_2()
            .p_3()
            .border_t_1()
            .border_color(theme.border)
            .when_some(self.vm.error().cloned(), |el, e| {
                el.child(
                    widgets::error_line(tr(cx, &e), theme)
                        .id("picker-error")
                        .test_support(),
                )
            })
            .when_some(storage.error().cloned(), |el, e| {
                el.child(widgets::error_line(tr(cx, &e), theme))
            })
            .when_some(selected, |el, name| {
                el.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div().text_sm().child(format!(
                                "{} {name}",
                                t(cx, "dialogs.image_picker.selected")
                            )),
                        )
                        .child(div().flex_1().child(Input::new(&self.alt).small()))
                        .child(div().flex_1().child(Input::new(&self.link).small())),
                )
            })
            .when(row_visible, |el| {
                el.child(self.render_storage_row(&storage, &row, theme, cx))
            })
            .when(busy.is_some(), |el| {
                let label = match busy {
                    Some(Busy::Paste) => t(cx, "dialogs.image_picker.processing_image_paste"),
                    _ if self.vm.waiting_for_storage() => {
                        t(cx, "media_dialogs.waiting_for_storage")
                    }
                    _ => t(cx, "dialogs.image_picker.processing_image"),
                };
                el.child(
                    v_flex()
                        .id("picker-progress")
                        .test_support()
                        .gap_1()
                        .child(Progress::new("picker-upload-progress").loading(true))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(label),
                        ),
                )
            })
            .child(
                h_flex()
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("picker-cancel")
                            .label(t(cx, "common.cancel"))
                            .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
                    )
                    .child(
                        Button::new("picker-insert")
                            .primary()
                            .disabled(!can_insert)
                            .label(format!(
                                "{}  Ctrl+↵",
                                t(cx, "dialogs.image_picker.insert_image")
                            ))
                            .on_click(cx.listener(|this, _, window, cx| this.insert(window, cx))),
                    ),
            )
    }

    fn render_storage_row(
        &self,
        storage: &frankmd_media::vm::storage::MediaStorageState,
        row: &frankmd_media::vm::storage::StorageRow,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let disabled = storage.controls_disabled();
        let s3_reason = storage.s3_disabled_reason();
        let provider = storage.provider();
        h_flex()
            .gap_2()
            .items_center()
            .flex_wrap()
            .child(div().text_sm().child(t(cx, "media_storage.label")))
            .child(
                Button::new("storage-local")
                    .xsmall()
                    .selected(provider == Provider::Local)
                    .disabled(disabled)
                    .label(t(cx, "media_storage.local"))
                    .on_click(cx.listener(|this, _, _, cx| this.set_provider(Provider::Local, cx))),
            )
            .child(
                Button::new("storage-s3")
                    .xsmall()
                    .selected(provider == Provider::S3)
                    .disabled(disabled || s3_reason.is_some())
                    .when_some(s3_reason.as_ref(), |b, reason| b.tooltip(tr(cx, reason)))
                    .label(t(cx, "media_storage.s3"))
                    .on_click(cx.listener(|this, _, _, cx| this.set_provider(Provider::S3, cx))),
            )
            .when(row.save_copy_visible(), |el| {
                el.child(
                    Checkbox::new("storage-save-copy")
                        .checked(row.save_copy())
                        .label(t(cx, "media_storage.save_copy"))
                        .on_click(cx.listener(|this, checked: &bool, window, cx| {
                            this.vm.row_mut().set_save_copy(*checked);
                            this.sync_inputs(window, cx);
                            cx.notify();
                        })),
                )
            })
            .when(row.resize_enabled(), |el| {
                let current = row.resize();
                el.child(div().text_sm().child(t(cx, "dialogs.s3_option.resize")))
                    .children(row.resize_options().map(|ratio| {
                        Button::new(SharedString::from(format!(
                            "storage-resize-{}",
                            (ratio * 100.0) as u32
                        )))
                        .xsmall()
                        .selected((current - ratio).abs() < f64::EPSILON)
                        .label(format!("{}%", (ratio * 100.0) as u32))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.vm.row_mut().set_resize(ratio);
                            cx.notify();
                        }))
                    }))
            })
            .when(row.key_visible(), |el| {
                let label = match row.key_mode() {
                    KeyMode::Full => t(cx, "dialogs.s3_option.custom_key"),
                    KeyMode::Prefix => t(cx, "dialogs.s3_option.custom_prefix"),
                };
                el.child(div().text_sm().child(label))
                    .child(div().w(px(260.)).child(Input::new(&self.key).small()))
            })
            .text_color(theme.foreground)
    }
}
