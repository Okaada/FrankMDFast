//! The app shell: one window's root view (ui-spec §2–§4).
//!
//! It owns the window's [`Workspace`] (the single authoritative state,
//! HP-002) and the GUI-free view models of `frankmd-workspace` (layout,
//! focus, overlays, banners, status, quick open, command registry). Views
//! render from them and every user action is a registry command
//! ([`Shell::run_command`]); no domain logic lives here.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use frankmd_config::store::ConfigStore;
use frankmd_config::{EnvSnapshot, Key, Value};
use frankmd_core::tree::{FileType, TreeNode};
use frankmd_core::{Clock, RelPath};
use frankmd_fs::Vault;
use frankmd_fs::recent::RecentFilesService;
use frankmd_i18n::Locale;
use frankmd_theme::OmarchyPaths;
use frankmd_workspace::autosave::{
    AutosaveEffect, RecoveryRequest, RecoveryResolution, SaveStatus,
};
use frankmd_workspace::banners::{BannerKind, BannerStack};
use frankmd_workspace::commands::{CommandContext, CommandRegistry};
use frankmd_workspace::doc_index::DocIndex;
use frankmd_workspace::drafts::open_draft_storage;
use frankmd_workspace::focus::FocusRegion;
use frankmd_workspace::layout::{LayoutState, PreviewMode, SidebarMode, ViewMode, WidthClass};
use frankmd_workspace::overlay::{OverlayKind, OverlayStack, PaletteMode};
use frankmd_workspace::quick_open::{NoteEntry, QuickOpenModel};
use frankmd_workspace::status::{MessageSlot, SaveState};
use frankmd_workspace::{Command, Effect, Event, Workspace};
use gpui_kit::component::input::InputState;
use gpui_kit::*;

use super::appearance::{self, ResolvedTheme};
use super::editor_host::EditorHost;
use super::i18n;
use super::palette::PaletteState;
use super::settings::SettingsState;

/// Everything a shell needs to open a vault. Tests build it over a temp dir.
#[derive(Debug, Clone)]
pub struct ShellConfig {
    /// Notes root.
    pub vault_root: PathBuf,
    /// Per-vault state directory outside the vault (drafts, DocId index), created if missing.
    pub state_dir: PathBuf,
    /// Environment snapshot (config resolution).
    pub env: EnvSnapshot,
    /// Note to open at startup (`fed <path>`), vault-relative or absolute inside the vault.
    pub open: Option<String>,
    /// Time source (autosave timers, status-message TTLs).
    pub clock: Arc<dyn Clock>,
    /// Omarchy locations (`None`: don't follow Omarchy).
    pub omarchy: Option<OmarchyPaths>,
    /// Start the file watchers (vault, `.fed`, Omarchy theme, fonts).
    pub watch: bool,
    /// Locale override (`--locale`).
    pub locale: Option<String>,
}

/// `Box<dyn Clock>` over a shared clock (the workspace takes ownership of its clock).
#[derive(Debug)]
struct SharedClock(Arc<dyn Clock>);

impl Clock for SharedClock {
    fn now(&self) -> jiff::Zoned {
        self.0.now()
    }
    fn instant(&self) -> Instant {
        self.0.instant()
    }
}

/// The opened vault.
pub(crate) struct Core {
    pub(crate) ws: Workspace,
    pub(crate) vault: Vault,
    /// `.fed` handle used only for `recent_files` (the workspace owns the rest).
    recents: ConfigStore,
}

impl Core {
    fn open(config: &ShellConfig) -> anyhow::Result<Self> {
        crate::app_fs::ensure_private_dir(&config.state_dir)?;
        let vault = Vault::open(&config.vault_root, &config.state_dir)?;
        let store = ConfigStore::open(vault.clone(), config.env.clone());
        let drafts = open_draft_storage(&config.state_dir).map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let index = DocIndex::open(&config.state_dir);
        let ws = Workspace::new(
            vault.clone(),
            store,
            drafts,
            index,
            Box::new(SharedClock(Arc::clone(&config.clock))),
        );
        let recents = ConfigStore::open(vault.clone(), config.env.clone());
        Ok(Self { ws, vault, recents })
    }

    fn recent_paths(&self) -> Vec<String> {
        RecentFilesService::new(&self.vault, &self.recents)
            .entries()
            .into_iter()
            .map(|r| r.path)
            .collect()
    }
}

/// Explorer state the tree doesn't carry: expansion and the keyboard cursor.
#[derive(Debug, Default)]
pub(crate) struct ExplorerState {
    pub(crate) expanded: BTreeSet<String>,
    pub(crate) cursor: Option<String>,
}

impl ExplorerState {
    /// Expand every ancestor folder of `path`.
    pub(crate) fn reveal(&mut self, path: &str) {
        let mut acc = String::new();
        let parts: Vec<&str> = path.split('/').collect();
        for part in &parts[..parts.len().saturating_sub(1)] {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(part);
            self.expanded.insert(acc.clone());
        }
    }
}

/// Quick open (overlay and docked empty state share it).
pub(crate) struct QuickOpenState {
    pub(crate) model: QuickOpenModel,
    pub(crate) input: Entity<InputState>,
}

/// Help dialog tab (§5.18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum HelpTab {
    #[default]
    Markdown,
    Keyboard,
    Vim,
}

/// Root view of a window.
pub struct Shell {
    pub(crate) vault_root: PathBuf,
    pub(crate) vault_name: SharedString,
    pub(crate) core: Option<Core>,
    pub(crate) startup_error: Option<SharedString>,
    pub(crate) registry: CommandRegistry,
    pub(crate) layout: LayoutState,
    pub(crate) overlays: OverlayStack,
    pub(crate) banners: BannerStack,
    pub(crate) messages: MessageSlot,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) width: f32,
    pub(crate) root_focus: FocusHandle,
    pub(crate) sidebar_focus: FocusHandle,
    pub(crate) preview_focus: FocusHandle,
    pub(crate) preview: Entity<crate::preview::PreviewPane>,
    pub(crate) editor_font: Option<SharedString>,
    pub(crate) overlay_focus: FocusHandle,
    pub(crate) library_focus: FocusHandle,
    pub(crate) editor: EditorHost,
    pub(crate) explorer: ExplorerState,
    pub(crate) palette: PaletteState,
    pub(crate) quick_open: QuickOpenState,
    pub(crate) settings: SettingsState,
    pub(crate) help_tab: HelpTab,
    pub(crate) recovery: Option<RecoveryRequest>,
    pub(crate) theme: ResolvedTheme,
    pub(crate) omarchy: Option<OmarchyPaths>,
    pub(crate) radius: f32,
    pub(crate) locale_override: Option<Locale>,
    pub(crate) recent_commands: Vec<&'static str>,
    pub(crate) scroll_sync: bool,
    pub(crate) vim: bool,
    autosave_timer: Option<Task<()>>,
    message_timer: Option<Task<()>>,
    watchers: Option<frankmd_watch::Watchers>,
    pub(crate) last_region: FocusRegion,
    window: AnyWindowHandle,
    _subscriptions: Vec<Subscription>,
}

impl Shell {
    /// Open the shell on `config` (does the startup phases of HP-016 in order:
    /// config → theme/locale → workspace → first scan → initial note).
    pub fn new(config: ShellConfig, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let vault_name: SharedString = config
            .vault_root
            .file_name()
            .map_or_else(
                || config.vault_root.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            )
            .into();
        let (core, startup_error) = match Core::open(&config) {
            Ok(core) => (Some(core), None),
            Err(err) => {
                tracing::error!(%err, "vault unavailable");
                (None, Some(SharedString::from(err.to_string())))
            }
        };

        let (editor, editor_sub) = EditorHost::new(window, cx);
        let qo_placeholder = i18n::t(cx, "quick_open.placeholder");
        let qo_input = cx.new(|cx| InputState::new(window, cx).placeholder(qo_placeholder));
        let qo_sub = cx.subscribe_in(&qo_input, window, |this, input, ev, window, cx| {
            if matches!(ev, gpui_kit::component::input::InputEvent::Change) {
                let q = input.read(cx).value().to_string();
                this.quick_open.model.set_query(&q);
                let _ = window;
                cx.notify();
            }
        });
        let (palette, palette_sub) = PaletteState::new(window, cx);
        let (settings, settings_sub) = SettingsState::new(window, cx);
        let preview = cx.new(|cx| crate::preview::PreviewPane::new(window, cx));
        let preview_sub = cx.subscribe_in(&preview, window, Self::on_preview_event);

        let locale_override = config.locale.as_deref().and_then(Locale::from_code);
        let mut shell = Self {
            vault_root: config.vault_root.clone(),
            vault_name,
            core,
            startup_error,
            registry: CommandRegistry::builtin(),
            layout: LayoutState::new(),
            overlays: OverlayStack::default(),
            banners: BannerStack::default(),
            messages: MessageSlot::default(),
            clock: Arc::clone(&config.clock),
            width: f32::from(window.viewport_size().width),
            root_focus: cx.focus_handle(),
            sidebar_focus: cx.focus_handle(),
            preview_focus: preview.read(cx).focus_handle(cx),
            preview,
            editor_font: None,
            overlay_focus: cx.focus_handle(),
            library_focus: cx.focus_handle(),
            editor,
            explorer: ExplorerState::default(),
            palette,
            quick_open: QuickOpenState {
                model: QuickOpenModel::default(),
                input: qo_input,
            },
            settings,
            help_tab: HelpTab::default(),
            recovery: None,
            theme: appearance::resolve_id(
                None,
                &frankmd_config::ResolvedConfig::new(None, EnvSnapshot::default()),
                None,
            ),
            omarchy: config.omarchy.clone(),
            radius: crate::bridge::theme::DEFAULT_RADIUS,
            locale_override,
            recent_commands: Vec::new(),
            scroll_sync: true,
            vim: false,
            autosave_timer: None,
            message_timer: None,
            watchers: None,
            last_region: FocusRegion::Editor,
            window: window.window_handle(),
            _subscriptions: vec![editor_sub, qo_sub, palette_sub, settings_sub, preview_sub],
        };
        shell.layout.on_resize(shell.width);
        shell.apply_settings(window, cx);
        shell.preview.update(cx, |p, cx| {
            p.set_vault_root(Some(config.vault_root.clone()), cx)
        });
        if config.watch {
            shell.start_watchers(cx);
            shell.load_hyprland_radius(cx);
            shell.load_editor_font(cx);
        }

        // First tree scan synchronously: the explorer and quick open must be
        // complete on the first frame (the scan is a short readdir walk).
        if let Some(core) = shell.core.as_mut() {
            let effect = core.ws.rescan();
            if let Effect::ScanTree { generation } = effect {
                let tree = core.vault.list_tree().unwrap_or_default();
                shell.dispatch(Command::TreeScanned { generation, tree }, window, cx);
            }
        }
        shell.refresh_quick_open(cx);
        shell.open_initial(config.open.as_deref(), window, cx);
        shell.update_title(window);
        shell
    }

    /// Notes root of this window.
    pub fn vault_root(&self) -> &std::path::Path {
        &self.vault_root
    }

    /// Open a forwarded `fed <path>` note argument in this window.
    pub fn open_argument(&mut self, arg: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.open_initial(Some(arg), window, cx);
    }

    // ----- startup ------------------------------------------------------------

    fn open_initial(&mut self, arg: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(core) = self.core.as_ref() else {
            self.focus_region(FocusRegion::Editor, window, cx);
            return;
        };
        let root = core.vault.root().to_path_buf();
        let real = core.vault.real_root().to_path_buf();
        let vault = core.vault.clone();
        let target = frankmd_workspace::startup::resolve_initial_note(
            &[root.as_path(), real.as_path()],
            arg,
            |p| {
                frankmd_fs::service::NotesService::new(&vault)
                    .read(p.as_str())
                    .is_ok()
            },
        );
        match target {
            frankmd_workspace::startup::StartupTarget::Note(path) => {
                self.open_note(path, window, cx)
            }
            frankmd_workspace::startup::StartupTarget::Missing(path) => {
                let name = path.as_str().trim_end_matches(".md").to_owned();
                self.show_message(
                    i18n::t_args(cx, "status.note_not_found", &[("name", &name)]).as_ref(),
                    false,
                    cx,
                );
                self.focus_region(FocusRegion::Editor, window, cx);
                self.quick_open
                    .input
                    .update(cx, |input, cx| input.set_value(name, window, cx));
            }
            frankmd_workspace::startup::StartupTarget::None => {
                self.focus_region(FocusRegion::Editor, window, cx)
            }
        }
    }

    fn start_watchers(&mut self, cx: &mut Context<Self>) {
        let (tx, rx) = async_channel::unbounded::<frankmd_watch::WatchEvent>();
        struct Sink(async_channel::Sender<frankmd_watch::WatchEvent>);
        impl frankmd_watch::EventSink for Sink {
            fn send(&self, ev: frankmd_watch::WatchEvent) -> bool {
                self.0.send_blocking(ev).is_ok()
            }
        }
        let config = frankmd_watch::WatchConfig::with_system_defaults(self.vault_root.clone());
        match frankmd_watch::start_with(config, Sink(tx)) {
            Ok(watchers) => {
                if let Some(core) = self.core.as_mut() {
                    core.ws.set_self_writes(watchers.self_writes());
                }
                self.watchers = Some(watchers);
            }
            Err(err) => {
                tracing::warn!(%err, "file watchers unavailable");
                return;
            }
        }
        self.spawn_watch_loop(rx, cx);
    }

    fn load_hyprland_radius(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let radius = cx
                .background_executor()
                .spawn(async { appearance::hyprland_rounding() })
                .await;
            if let Some(radius) = radius {
                let _ = this.update(cx, |this, cx| {
                    this.radius = radius;
                    let tokens = this.theme.tokens.clone();
                    this.apply_tokens(&tokens, cx);
                });
            }
        })
        .detach();
    }

    // ----- workspace plumbing ----------------------------------------------------

    /// Send one command to the workspace and apply its events and effects.
    pub(crate) fn dispatch(&mut self, cmd: Command, window: &mut Window, cx: &mut Context<Self>) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        let (events, effects) = core.ws.handle(cmd);
        for event in events {
            self.apply_event(event, window, cx);
        }
        for effect in effects {
            self.run_effect(effect, cx);
        }
        cx.notify();
    }

    fn apply_event(&mut self, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            Event::DocOpened { doc, path, text } => {
                self.editor.set_document(Some(doc), &text, window, cx);
                self.render_preview_now(cx);
                self.layout.note_opened();
                self.explorer.reveal(path.as_str());
                self.explorer.cursor = Some(path.as_str().to_owned());
                self.banners.clear(BannerKind::ChangedOnDisk);
                self.banners.clear(BannerKind::DeletedOnDisk);
                self.banners.clear(BannerKind::ContentLoss);
                let read_only = self
                    .core
                    .as_ref()
                    .and_then(|c| c.ws.open_doc())
                    .is_some_and(|d| d.read_only);
                self.banners.set(BannerKind::ReadOnly, read_only);
                self.record_recent(path.as_str(), cx);
                self.update_title(window);
            }
            Event::DocClosed { .. } | Event::DocDeleted { .. } => {
                self.editor.set_document(None, "", window, cx);
                self.render_preview_now(cx);
                for kind in [
                    BannerKind::ChangedOnDisk,
                    BannerKind::DeletedOnDisk,
                    BannerKind::ContentLoss,
                    BannerKind::ReadOnly,
                ] {
                    self.banners.clear(kind);
                }
                self.update_title(window);
            }
            Event::DocReloaded { text, .. } => {
                self.editor.set_text(&text, window, cx);
                self.render_preview_now(cx);
            }
            Event::DocConflict { .. } => self.banners.raise(BannerKind::ChangedOnDisk),
            Event::DocDeletedOnDisk { .. } => self.banners.raise(BannerKind::DeletedOnDisk),
            Event::DocSaved { .. } | Event::Remapped(_) => self.update_title(window),
            Event::TreeChanged => self.refresh_quick_open(cx),
            Event::Reveal { path } => self.explorer.reveal(path.as_str()),
            Event::SettingsChanged => self.apply_settings(window, cx),
            Event::Autosave(effect) => self.apply_autosave(effect, window, cx),
            Event::System(ev) => self.apply_system(ev, window, cx),
            Event::Error(err) => {
                let args: Vec<(&str, &str)> =
                    err.args.iter().map(|(k, v)| (*k, v.as_str())).collect();
                let text = i18n::t_args(cx, err.key, &args);
                self.show_message(&text, true, cx);
            }
        }
    }

    fn apply_autosave(
        &mut self,
        effect: AutosaveEffect,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match effect {
            AutosaveEffect::Status(SaveStatus::DraftStorageError) => {
                self.banners.raise(BannerKind::DraftStorageFailed)
            }
            AutosaveEffect::Status(SaveStatus::Saved) => {
                self.banners.clear(BannerKind::DraftStorageFailed)
            }
            AutosaveEffect::ContentLossBanner(shown) => {
                self.banners.set(BannerKind::ContentLoss, shown)
            }
            AutosaveEffect::OfflineChanged(offline) => {
                self.banners.set(BannerKind::VaultUnreachable, offline)
            }
            AutosaveEffect::SetEditorText(text)
            | AutosaveEffect::LoadEditorText { content: text, .. } => {
                self.editor.set_text(&text, window, cx);
            }
            AutosaveEffect::OpenRecovery(request) => {
                self.recovery = Some(request);
                self.open_overlay(OverlayKind::Dialog("recovery"), window, cx);
            }
            AutosaveEffect::Status(_)
            | AutosaveEffect::Save(_)
            | AutosaveEffect::UnloadGuard(_)
            | AutosaveEffect::ConfigSaved => {}
        }
    }

    fn apply_system(
        &mut self,
        ev: frankmd_watch::WatchEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use frankmd_watch::WatchEvent;
        match ev {
            WatchEvent::OmarchyThemeChanged => {
                if self.theme.id == appearance::OMARCHY || self.theme.omarchy_name.is_some() {
                    match appearance::omarchy_tokens(self.omarchy.as_ref()) {
                        Some((name, tokens)) => {
                            self.theme = ResolvedTheme {
                                tokens,
                                id: appearance::OMARCHY.to_owned(),
                                omarchy_name: Some(name),
                            };
                            let tokens = self.theme.tokens.clone();
                            self.apply_tokens(&tokens, cx);
                        }
                        None => {
                            let text = i18n::t(cx, "status.theme_sync_failed");
                            self.show_message(&text, true, cx);
                        }
                    }
                }
            }
            WatchEvent::SystemLanguageChanged => self.apply_locale(cx),
            WatchEvent::Error(_) => {
                let text = i18n::t(cx, "status.watcher_failed");
                self.show_message(&text, true, cx);
            }
            WatchEvent::FontsChanged => self.load_editor_font(cx),
            WatchEvent::ConfigChanged | WatchEvent::VaultChanged { .. } => {}
        }
        let _ = window;
    }

    fn run_effect(&mut self, effect: Effect, cx: &mut Context<Self>) {
        match effect {
            Effect::ScanTree { generation } => {
                let Some(core) = self.core.as_ref() else {
                    return;
                };
                let vault = core.vault.clone();
                self.spawn_scan(vault, generation, cx);
            }
            Effect::Render { .. } => self.render_preview_now(cx),
            Effect::WakeAt(at) => {
                let delay = at.saturating_duration_since(self.clock.instant());
                self.autosave_timer = Some(self.spawn_tick(delay, cx));
            }
        }
    }

    /// The editor reported a user edit.
    pub(crate) fn editor_changed(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = self
            .core
            .as_ref()
            .and_then(|c| c.ws.open_doc())
            .map(|d| d.id)
        else {
            return;
        };
        self.dispatch(Command::Edited { doc, text }, window, cx);
    }

    fn record_recent(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(core) = self.core.as_ref() else {
            return;
        };
        let now = self.clock.timestamp();
        if let Err(err) =
            RecentFilesService::new(&core.vault, &core.recents).record(Some(path), now)
        {
            tracing::warn!(?err, "recent files not recorded");
        }
        self.refresh_quick_open(cx);
    }

    /// Rebuild quick open's candidates from the tree and the shared recents.
    pub(crate) fn refresh_quick_open(&mut self, cx: &mut Context<Self>) {
        let Some(core) = self.core.as_ref() else {
            return;
        };
        fn walk(nodes: &[TreeNode], out: &mut Vec<NoteEntry>) {
            for node in nodes {
                match node {
                    TreeNode::Folder { children, .. } => walk(children, out),
                    TreeNode::File {
                        path,
                        file_type: FileType::Markdown,
                        mtime,
                        ..
                    } => {
                        out.push(NoteEntry {
                            path: path.as_str().to_owned(),
                            mtime: *mtime,
                        });
                    }
                    TreeNode::File { .. } => {}
                }
            }
        }
        let mut notes = Vec::new();
        walk(&core.ws.snapshot().tree, &mut notes);
        let query = self.quick_open.input.read(cx).value().to_string();
        self.quick_open.model = QuickOpenModel::new(notes, core.recent_paths());
        self.quick_open.model.set_query(&query);
    }

    /// Number of notes in the vault (empty state header).
    pub(crate) fn note_count(&self) -> usize {
        fn count(nodes: &[TreeNode]) -> usize {
            nodes
                .iter()
                .map(|n| match n {
                    TreeNode::Folder { children, .. } => count(children),
                    TreeNode::File { .. } => 1,
                })
                .sum()
        }
        self.core
            .as_ref()
            .map_or(0, |c| count(&c.ws.snapshot().tree))
    }

    /// Re-read `.fed`-backed preferences (startup, our writes, external edits).
    pub(crate) fn apply_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(core) = self.core.as_ref() else {
            self.apply_locale(cx);
            let tokens = self.theme.tokens.clone();
            self.apply_tokens(&tokens, cx);
            return;
        };
        let config = core.ws.snapshot().config;
        let read_only = core.ws.open_doc().is_some_and(|d| d.read_only);
        let bool_of = |key: Key, default: bool| {
            config
                .value(key)
                .and_then(|v| v.as_bool())
                .unwrap_or(default)
        };
        self.layout.sidebar_visible = bool_of(Key::SidebarVisible, true);
        self.layout.view_mode =
            ViewMode::parse(config.value(Key::ViewMode).as_ref().and_then(Value::as_str));
        self.layout.typewriter = bool_of(Key::TypewriterMode, false);
        self.scroll_sync = bool_of(Key::ScrollSync, true);
        self.vim = bool_of(Key::VimMode, false);
        if self.palette.preview_theme.is_none() {
            self.theme = appearance::resolve(&config, self.omarchy.as_ref());
            let tokens = self.theme.tokens.clone();
            self.apply_tokens(&tokens, cx);
        }
        self.apply_locale(cx);
        self.editor
            .apply_config(&config, self.editor_font.clone(), read_only, window, cx);
        let int = |key: Key, default: i64| {
            config
                .value(key)
                .and_then(|v| v.as_int())
                .unwrap_or(default)
        };
        let zoom = u16::try_from(int(Key::PreviewZoom, 100).clamp(50, 200)).unwrap_or(100);
        let width = u16::try_from(int(Key::EditorWidth, 72).clamp(40, 200)).unwrap_or(72);
        self.preview.update(cx, |p, cx| {
            p.set_zoom(zoom, cx);
            p.set_text_width(width, cx);
        });
        cx.notify();
    }

    /// Apply a palette to gpui-kit's theme and the preview.
    pub(crate) fn apply_tokens(
        &mut self,
        tokens: &frankmd_theme::UiTokens,
        cx: &mut Context<Self>,
    ) {
        crate::bridge::theme::apply(tokens, self.radius, cx);
        self.preview.update(cx, |p, cx| p.set_theme(tokens, cx));
    }

    /// Hand the editor text to the preview (it parses off-thread, generation-tagged).
    pub(crate) fn render_preview_now(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.active_path() else {
            self.preview
                .update(cx, |p, cx| p.set_markdown("", None, cx));
            return;
        };
        if !path.ends_with(".md") {
            self.preview.update(cx, |p, cx| p.set_not_markdown(cx));
            return;
        }
        let text = self.editor.text(cx);
        let base = std::path::Path::new(&path)
            .parent()
            .map(|p| self.vault_root.join(p));
        self.preview
            .update(cx, |p, cx| p.set_markdown(text, base, cx));
    }

    fn on_preview_event(
        &mut self,
        _pane: &Entity<crate::preview::PreviewPane>,
        ev: &crate::preview::PreviewEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::preview::PreviewEvent;
        match ev {
            PreviewEvent::VisibleSourceLine(line) => {
                if self.scroll_sync {
                    let line = (*line as usize).saturating_sub(1);
                    self.editor
                        .pane
                        .update(cx, |p, cx| p.scroll_to_line(line, cx));
                }
            }
            PreviewEvent::ToggleTask { line, .. } => {
                self.toggle_task_line(*line as usize, window, cx)
            }
            PreviewEvent::OpenWikiLink { target } => self.open_wikilink(target, window, cx),
            PreviewEvent::OpenNote { path } => {
                let rel = path
                    .strip_prefix(&self.vault_root)
                    .ok()
                    .and_then(|p| p.to_str())
                    .map(str::to_owned);
                if let Some(rel) = rel {
                    self.open_path(&rel, window, cx);
                }
            }
            PreviewEvent::ZoomChanged(zoom) => {
                self.update_settings(
                    vec![(Key::PreviewZoom, Some(Value::Int(i128::from(*zoom))))],
                    window,
                    cx,
                );
            }
        }
    }

    /// Preview checkbox: toggle the task marker on 1-based `line` through the
    /// editor (one undo step) so the edit reaches the workspace like typing.
    fn toggle_task_line(&mut self, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.editor.text(cx);
        let mut start = 0;
        for (ix, l) in text.split_inclusive('\n').enumerate() {
            if ix + 1 == line {
                let body = l.trim_end_matches(['\n', '\r']);
                if let Some(toggled) = frankmd_core::text::task::toggle_task_marker(body) {
                    let range = start..start + body.len();
                    let caret = range.start + toggled.len();
                    let edit = crate::editor::Edit {
                        range,
                        text: toggled,
                        selection: caret..caret,
                    };
                    self.editor
                        .pane
                        .update(cx, |p, cx| p.apply_edit(edit, window, cx));
                }
                return;
            }
            start += l.len();
        }
    }

    /// `[[target]]`: open the matching note, or the note-not-found flow.
    fn open_wikilink(&mut self, target: &str, window: &mut Window, cx: &mut Context<Self>) {
        let link = frankmd_core::wikilink::extract(&format!("[[{target}]]"))
            .into_iter()
            .next();
        let found = link.and_then(|link| {
            self.quick_open_candidates().into_iter().find(|p| {
                RelPath::parse(p).is_ok_and(|rel| frankmd_core::wikilink::resolves_to(&link, &rel))
            })
        });
        match found {
            Some(path) => self.open_path(&path, window, cx),
            None => {
                let text = i18n::t_args(cx, "status.note_not_found", &[("name", target)]);
                self.show_message(&text, false, cx);
                self.open_overlay(OverlayKind::QuickOpen, window, cx);
                let target = target.to_owned();
                self.quick_open
                    .input
                    .update(cx, |input, cx| input.set_value(target.clone(), window, cx));
                self.quick_open.model.set_query(&target);
            }
        }
    }

    /// Every note path of the tree.
    pub(crate) fn quick_open_candidates(&self) -> Vec<String> {
        fn walk(nodes: &[TreeNode], out: &mut Vec<String>) {
            for node in nodes {
                match node {
                    TreeNode::Folder { children, .. } => walk(children, out),
                    TreeNode::File {
                        path,
                        file_type: FileType::Markdown,
                        ..
                    } => out.push(path.as_str().to_owned()),
                    TreeNode::File { .. } => {}
                }
            }
        }
        let mut out = Vec::new();
        if let Some(core) = self.core.as_ref() {
            walk(&core.ws.snapshot().tree, &mut out);
        }
        out
    }

    fn load_editor_font(&mut self, cx: &mut Context<Self>) {
        let id = self
            .core
            .as_ref()
            .and_then(|c| c.ws.snapshot().config.value(Key::EditorFont))
            .and_then(|v| v.as_str().map(str::to_owned));
        let window = self.window;
        cx.spawn(async move |this, cx| {
            let font = cx
                .background_executor()
                .spawn(async move { super::fonts::resolve_editor_font(id.as_deref()) })
                .await;
            let _ = window.update(cx, |_, window, cx| {
                let _ = this.update(cx, |shell, cx| {
                    shell.editor_font = font.map(SharedString::from);
                    shell.apply_settings(window, cx);
                });
            });
        })
        .detach();
    }

    /// The UI language: `--locale`, then `.fed`/`FRANKMD_LOCALE`, then the system.
    pub(crate) fn apply_locale(&mut self, cx: &mut Context<Self>) {
        let locale = self
            .locale_override
            .unwrap_or_else(|| self.configured_locale());
        i18n::set_locale(locale, cx);
        let catalog = i18n::current(cx);
        self.preview.update(cx, |p, cx| p.set_catalog(catalog, cx));
    }

    /// Locale from config, else the system's.
    pub(crate) fn configured_locale(&self) -> Locale {
        let configured = self.core.as_ref().and_then(|c| {
            let config = c.ws.snapshot().config;
            config
                .value(Key::Locale)
                .and_then(|v| v.as_str().and_then(Locale::from_code))
        });
        configured.unwrap_or_else(|| self.system_locale())
    }

    /// `LC_ALL` / `LC_MESSAGES` / `LANG`, else English.
    pub(crate) fn system_locale(&self) -> Locale {
        let env = self
            .core
            .as_ref()
            .map(|c| c.ws.snapshot().config.env().clone())
            .unwrap_or_default();
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|name| env.present(name).and_then(Locale::from_system))
            .unwrap_or(Locale::En)
    }

    /// Write `.fed` keys through the workspace.
    pub(crate) fn update_settings(
        &mut self,
        changes: Vec<(Key, Option<Value>)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch(Command::UpdateSettings { changes }, window, cx);
    }

    // ----- navigation -------------------------------------------------------------

    /// Open `path` and give the editor focus.
    pub(crate) fn open_note(&mut self, path: RelPath, window: &mut Window, cx: &mut Context<Self>) {
        self.dispatch(Command::OpenNote { path }, window, cx);
        if self.overlays.current().is_some() {
            self.overlays.close();
        }
        self.focus_region(FocusRegion::Editor, window, cx);
    }

    /// Open the note at vault-relative `path` (string form).
    pub(crate) fn open_path(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        match RelPath::parse(path) {
            Ok(p) => self.open_note(p, window, cx),
            Err(_) => tracing::warn!(path, "invalid note path"),
        }
    }

    /// Text of the editor (what the workspace last received as an edit).
    pub fn editor_text(&self, cx: &App) -> String {
        self.editor.text(cx)
    }

    /// Current status-line message, if not expired.
    pub fn status_message(&self) -> Option<String> {
        self.messages
            .current(self.clock.instant())
            .map(|m| m.text.clone())
    }

    /// `:n` / `:prev`: next or previous note in tree order (wrapping).
    pub(crate) fn step_note(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let notes = self.quick_open_candidates();
        if notes.is_empty() {
            return;
        }
        let at = self
            .active_path()
            .and_then(|p| notes.iter().position(|n| *n == p));
        let len = notes.len();
        let next = match at {
            Some(i) if forward => (i + 1) % len,
            Some(i) => (i + len - 1) % len,
            None => 0,
        };
        let path = notes[next].clone();
        self.open_path(&path, window, cx);
    }

    /// Path of the active note.
    pub fn active_path(&self) -> Option<String> {
        self.core
            .as_ref()
            .and_then(|c| c.ws.open_doc())
            .map(|d| d.path.as_str().to_owned())
    }

    fn update_title(&self, window: &mut Window) {
        let title = match self.active_path() {
            Some(path) => {
                let name = path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&path)
                    .trim_end_matches(".md")
                    .to_owned();
                format!("{name} — FrankMD")
            }
            None => format!("FrankMD — {}", self.vault_name),
        };
        window.set_window_title(&title);
    }

    // ----- status -----------------------------------------------------------------

    /// Show a status-line message (2 s info / 5 s error).
    pub(crate) fn show_message(&mut self, text: &str, error: bool, cx: &mut Context<Self>) {
        let now = self.clock.instant();
        self.messages.show(text, error, now);
        let delay = self
            .messages
            .next_deadline()
            .map(|d| d.saturating_duration_since(now))
            .unwrap_or_default();
        self.message_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |_, cx| cx.notify());
        }));
        cx.notify();
    }

    /// Current save-state slot.
    pub fn save_state(&self) -> SaveState {
        match self.core.as_ref() {
            Some(core) => {
                let autosave = core.ws.autosave();
                SaveState::from_autosave(
                    autosave.status(),
                    autosave.is_saving(),
                    core.ws.snapshot().dirty,
                )
            }
            None => SaveState::Clean,
        }
    }

    // ----- layout & focus -----------------------------------------------------------

    /// Layout view model.
    pub fn layout(&self) -> &LayoutState {
        &self.layout
    }

    /// Open overlay.
    pub fn overlay(&self) -> Option<OverlayKind> {
        self.overlays.current().cloned()
    }

    /// Visible banners, top to bottom.
    pub fn banners(&self) -> Vec<BannerKind> {
        self.banners.visible()
    }

    /// Raise a banner for a condition the workspace reports out of band
    /// (vault reachability probe, storage fallback) or, in tests, to drive
    /// the banner wiring directly.
    pub fn raise_banner(&mut self, kind: BannerKind, cx: &mut Context<Self>) {
        self.banners.raise(kind);
        cx.notify();
    }

    /// Current pane arrangement.
    pub(crate) fn arrangement(&self) -> frankmd_workspace::layout::PaneArrangement {
        self.layout.arrangement(self.width)
    }

    pub(crate) fn on_width(&mut self, width: f32, window: &mut Window, cx: &mut Context<Self>) {
        self.width = width;
        self.layout.on_resize(width);
        self.ensure_focus_visible(window, cx);
    }

    /// The region that holds keyboard focus.
    pub fn focused_region(&self, window: &Window, cx: &App) -> Option<FocusRegion> {
        if self.sidebar_focus.contains_focused(window, cx) {
            Some(FocusRegion::Sidebar)
        } else if self.library_focus.contains_focused(window, cx)
            || self.editor.focus_handle(cx).contains_focused(window, cx)
            || (self.active_path().is_none()
                && self
                    .quick_open
                    .input
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window)
                && self.overlays.current().is_none())
        {
            Some(FocusRegion::Editor)
        } else if self.preview_focus.contains_focused(window, cx) {
            Some(FocusRegion::Preview)
        } else {
            None
        }
    }

    /// Move focus to `region` (opening what it needs: the narrow drawer, a hidden preview).
    pub(crate) fn focus_region(
        &mut self,
        region: FocusRegion,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match region {
            FocusRegion::Sidebar => {
                if self.layout.width_class == WidthClass::Narrow {
                    self.layout.drawer_open = true;
                } else if !self.layout.sidebar_visible
                    || self.arrangement().sidebar == SidebarMode::Hidden
                {
                    self.layout.sidebar_visible = true;
                }
                self.sidebar_focus.focus(window, cx);
            }
            FocusRegion::Editor | FocusRegion::StatusLine => {
                if self.layout.media_library {
                    self.library_focus.focus(window, cx);
                } else if self.active_path().is_some() {
                    self.editor.focus(window, cx);
                } else {
                    let input = self.quick_open.input.clone();
                    input.update(cx, |input, cx| input.focus(window, cx));
                }
            }
            FocusRegion::Preview => {
                if self.arrangement().preview == PreviewMode::Hidden {
                    self.layout.toggle_preview();
                }
                self.preview_focus.focus(window, cx);
            }
        }
        self.last_region = region;
        cx.notify();
    }

    /// After a layout change: if the focused region disappeared, move to its fallback.
    pub(crate) fn ensure_focus_visible(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.current().is_some() {
            return;
        }
        let arrangement = self.arrangement();
        let region = self.focused_region(window, cx).unwrap_or(self.last_region);
        if !region.is_visible(&arrangement, false) {
            let to = region.fallback(&arrangement);
            self.focus_region(to, window, cx);
        }
    }

    // ----- overlays ---------------------------------------------------------------

    /// Open `kind` (replacing any open overlay) and focus it.
    pub(crate) fn open_overlay(
        &mut self,
        kind: OverlayKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let from = self.focused_region(window, cx).unwrap_or(self.last_region);
        self.overlays.open(kind.clone(), from);
        match kind {
            OverlayKind::Palette(mode) => self.palette.open(mode, window, cx),
            OverlayKind::QuickOpen => {
                let input = self.quick_open.input.clone();
                input.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                    input.focus(window, cx);
                });
                self.quick_open.model.set_query("");
            }
            OverlayKind::Settings => self.settings.open(window, cx),
            OverlayKind::Help | OverlayKind::Dialog(_) | OverlayKind::SearchInFiles => {
                self.overlay_focus.focus(window, cx)
            }
        }
        cx.notify();
    }

    /// Close the overlay and give focus back.
    pub(crate) fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(OverlayKind::Palette(_)) = self.overlays.current() {
            self.palette_cancel(cx);
        }
        if let Some(region) = self.overlays.close() {
            self.focus_region(region.fallback(&self.arrangement()), window, cx);
        }
        cx.notify();
    }

    /// Esc: inline layers first (§6.1 chain); never leaves zen.
    pub(crate) fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(kind) = self.overlays.current() {
            if kind.closes_on_escape() {
                self.close_overlay(window, cx);
            }
            return;
        }
        if self.layout.close_drawer() {
            self.focus_region(FocusRegion::Editor, window, cx);
            return;
        }
        if self.layout.media_library {
            self.layout.close_media_library();
            self.focus_region(FocusRegion::Editor, window, cx);
        }
        cx.notify();
    }

    /// Context for palette filtering.
    pub(crate) fn command_context(&self) -> CommandContext {
        let open = self.active_path();
        CommandContext {
            markdown_open: open.as_deref().is_some_and(|p| p.ends_with(".md")),
            caret_in_table: false,
            vim: self.vim,
            note_open: open.is_some(),
            zen: self.layout.zen,
            preview_visible: self.layout.preview_visible,
            sidebar_visible: self.layout.sidebar_visible,
            typewriter: self.layout.typewriter,
            scroll_sync: self.scroll_sync,
        }
    }

    // ----- commands -----------------------------------------------------------------

    /// Run registry command `id` (from a shortcut, the palette or a button).
    pub(crate) fn run_command(
        &mut self,
        id: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // FE-029 changed: inside Settings only Esc and Ctrl+, work.
        if self.overlays.is_open(&OverlayKind::Settings)
            && !matches!(id, "app.settings" | "app.escape" | "list.escape")
        {
            return;
        }
        if !matches!(
            id,
            "app.escape" | "list.escape" | "list.confirm" | "list.select_next" | "list.select_prev"
        ) {
            self.recent_commands.retain(|c| *c != id);
            self.recent_commands.insert(0, id);
            self.recent_commands.truncate(20);
        }
        let doc = self
            .core
            .as_ref()
            .and_then(|c| c.ws.open_doc())
            .map(|d| d.id);
        match id {
            "app.command_palette" => {
                self.toggle_overlay(OverlayKind::Palette(PaletteMode::Commands), window, cx)
            }
            "file.quick_open" => self.toggle_overlay(OverlayKind::QuickOpen, window, cx),
            "theme.select" => {
                self.open_overlay(OverlayKind::Palette(PaletteMode::Theme), window, cx)
            }
            "language.select" => {
                self.open_overlay(OverlayKind::Palette(PaletteMode::Language), window, cx)
            }
            "app.settings" => self.toggle_overlay(OverlayKind::Settings, window, cx),
            "app.diagnostics" => {
                self.settings.category = super::settings::Category::Diagnostics;
                self.open_overlay(OverlayKind::Settings, window, cx);
            }
            "help.toggle" => {
                self.help_tab = HelpTab::Markdown;
                self.toggle_overlay(OverlayKind::Help, window, cx);
            }
            "help.keyboard_shortcuts" => {
                self.help_tab = HelpTab::Keyboard;
                self.open_overlay(OverlayKind::Help, window, cx);
            }
            "app.escape" | "list.escape" => self.escape(window, cx),
            "list.confirm" => self.list_confirm(window, cx),
            "list.select_next" => self.list_step(true, window, cx),
            "list.select_prev" => self.list_step(false, window, cx),
            "app.quit" => self.quit(window, cx),
            "file.save" => {
                if let Some(doc) = doc {
                    self.dispatch(Command::SaveNow { doc }, window, cx);
                }
            }
            "file.close" => {
                self.dispatch(Command::CloseNote, window, cx);
                self.focus_region(FocusRegion::Editor, window, cx);
            }
            "view.toggle_sidebar" => {
                self.layout.toggle_sidebar();
                if self.layout.width_class == WidthClass::Narrow {
                    if self.layout.drawer_open {
                        self.sidebar_focus.focus(window, cx);
                    } else {
                        self.focus_region(FocusRegion::Editor, window, cx);
                    }
                } else {
                    let visible = self.layout.sidebar_visible;
                    self.update_settings(
                        vec![(Key::SidebarVisible, Some(Value::Bool(visible)))],
                        window,
                        cx,
                    );
                }
                self.ensure_focus_visible(window, cx);
            }
            "view.toggle_preview" => {
                let was_swapped = self.arrangement().preview == PreviewMode::Swapped;
                self.layout.toggle_preview();
                if was_swapped {
                    self.focus_region(FocusRegion::Editor, window, cx);
                }
                self.ensure_focus_visible(window, cx);
            }
            "view.toggle_zen" => {
                self.layout.toggle_zen();
                self.ensure_focus_visible(window, cx);
            }
            "view.media_library" => {
                self.layout.toggle_media_library();
                self.focus_region(FocusRegion::Editor, window, cx);
            }
            "view.toggle_typewriter" => {
                let on = !self.layout.typewriter;
                self.update_settings(
                    vec![(Key::TypewriterMode, Some(Value::Bool(on)))],
                    window,
                    cx,
                );
            }
            "view.toggle_scroll_sync" => {
                let on = !self.scroll_sync;
                self.update_settings(vec![(Key::ScrollSync, Some(Value::Bool(on)))], window, cx);
            }
            "editor.toggle_vim" => {
                let on = !self.vim;
                self.update_settings(vec![(Key::VimMode, Some(Value::Bool(on)))], window, cx);
            }
            "focus.sidebar" => self.focus_region(FocusRegion::Sidebar, window, cx),
            "focus.editor" => self.focus_region(FocusRegion::Editor, window, cx),
            "focus.preview" => self.focus_region(FocusRegion::Preview, window, cx),
            "focus.next_region" | "focus.prev_region" => {
                let arrangement = self.arrangement();
                let from = self.focused_region(window, cx).unwrap_or(self.last_region);
                let to = from.step(&arrangement, false, id == "focus.next_region");
                self.focus_region(to, window, cx);
            }
            "file.copy_path" => {
                if let Some(path) = self.active_path() {
                    cx.write_to_clipboard(ClipboardItem::new_string(path));
                    let text = i18n::t(cx, "status.copied_to_clipboard");
                    self.show_message(&text, false, cx);
                }
            }
            "file.copy_contents" => {
                if doc.is_some() {
                    cx.write_to_clipboard(ClipboardItem::new_string(self.editor.text(cx)));
                    let text = i18n::t(cx, "status.copied_to_clipboard");
                    self.show_message(&text, false, cx);
                }
            }
            "file.reveal" => {
                if let Some(path) = self.active_path() {
                    self.explorer.reveal(&path);
                    self.explorer.cursor = Some(path);
                    self.focus_region(FocusRegion::Sidebar, window, cx);
                }
            }
            "edit.go_to_line" => {
                self.open_overlay(OverlayKind::Palette(PaletteMode::GoToLine), window, cx)
            }
            "view.zoom_in" | "view.zoom_out" | "view.zoom_reset"
                if self.preview_focus.contains_focused(window, cx) =>
            {
                self.preview.update(cx, |p, cx| match id {
                    "view.zoom_in" => p.zoom_in(cx),
                    "view.zoom_out" => p.zoom_out(cx),
                    _ => p.reset_zoom(cx),
                });
            }
            _ => {
                if !self.editor.run_command(id, window, cx) {
                    self.not_available(id, cx);
                }
            }
        }
        cx.notify();
    }

    fn not_available(&mut self, id: &str, cx: &mut Context<Self>) {
        let title = self
            .registry
            .get(id)
            .map_or_else(|| id.to_owned(), |c| c.title());
        let text = i18n::t_args(cx, "palette.not_available", &[("command", &title)]);
        self.show_message(&text, false, cx);
    }

    fn toggle_overlay(&mut self, kind: OverlayKind, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.is_open(&kind) {
            self.close_overlay(window, cx);
        } else {
            self.open_overlay(kind, window, cx);
        }
    }

    /// Ctrl+Q: drafts make quitting safe; flush them and quit (§5.23).
    fn quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dispatch(Command::FlushDrafts, window, cx);
        cx.quit();
    }

    /// ↑/↓ in the open list.
    pub(crate) fn list_step(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        match self.overlays.current() {
            Some(OverlayKind::Palette(_)) => self.palette_step(forward, window, cx),
            Some(OverlayKind::QuickOpen) | None => {
                if forward {
                    self.quick_open.model.select_next();
                } else {
                    self.quick_open.model.select_prev();
                }
            }
            _ => {}
        }
        cx.notify();
    }

    /// Enter in the open list.
    pub(crate) fn list_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.overlays.current() {
            Some(OverlayKind::Palette(_)) => self.palette_confirm(window, cx),
            Some(OverlayKind::QuickOpen) | None => {
                if let Some(path) = self.quick_open.model.selected_path().map(str::to_owned) {
                    self.overlays.close();
                    self.open_path(&path, window, cx);
                }
            }
            _ => {}
        }
    }

    /// Recovery dialog answer.
    pub(crate) fn resolve_recovery(
        &mut self,
        keep_draft: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = self.recovery.take() else {
            return;
        };
        let resolution = if keep_draft {
            RecoveryResolution::accept_local(&request)
        } else {
            RecoveryResolution::accept_server(&request)
        };
        self.overlays.close();
        self.dispatch(Command::ResolveRecovery(resolution), window, cx);
        self.focus_region(FocusRegion::Editor, window, cx);
    }

    /// A banner button was pressed (`key` is the button's i18n key).
    pub(crate) fn banner_action(
        &mut self,
        kind: BannerKind,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match key {
            "banners.content_loss.undo" => {
                self.banners.clear(BannerKind::ContentLoss);
                self.dispatch(
                    Command::ContentLossUndo {
                        undid_editor_change: false,
                    },
                    window,
                    cx,
                );
            }
            "banners.content_loss.save_anyway" => {
                self.banners.clear(BannerKind::ContentLoss);
                self.dispatch(Command::ContentLossSaveAnyway, window, cx);
            }
            "banners.changed_on_disk.reload" => {
                self.banners.clear(kind);
                if let Some(path) = self.active_path() {
                    self.dispatch(Command::CloseNote, window, cx);
                    self.open_path(&path, window, cx);
                }
            }
            "banners.deleted_on_disk.close" => {
                self.banners.clear(kind);
                self.dispatch(Command::CloseNote, window, cx);
            }
            "banners.deleted_on_disk.save_again" => {
                self.banners.clear(kind);
                if let Some(doc) = self
                    .core
                    .as_ref()
                    .and_then(|c| c.ws.open_doc())
                    .map(|d| d.id)
                {
                    self.dispatch(Command::SaveNow { doc }, window, cx);
                }
            }
            "banners.offline.retry" => {
                if self.vault_root.is_dir() {
                    self.banners.clear(kind);
                }
            }
            "banners.storage_fallback.dismiss" => self.banners.dismiss(kind),
            "banners.storage_fallback.settings" => {
                self.open_overlay(OverlayKind::Settings, window, cx)
            }
            "banners.draft_storage.details" => {
                self.settings.category = super::settings::Category::Diagnostics;
                self.open_overlay(OverlayKind::Settings, window, cx);
            }
            _ => {}
        }
        if self.overlays.current().is_none() {
            self.focus_region(FocusRegion::Editor, window, cx);
        }
        cx.notify();
    }

    /// Commands listener for the root element.
    pub(crate) fn command_handler(cx: &mut Context<Self>) -> super::actions::CommandHandler {
        let this = cx.entity().downgrade();
        Rc::new(move |id, window, cx| {
            let _ = this.update(cx, |shell, cx| shell.run_command(id, window, cx));
        })
    }
}

impl Shell {
    fn spawn_scan(
        &self,
        vault: Vault,
        generation: frankmd_core::Generation,
        cx: &mut Context<Self>,
    ) {
        let window = self.window;
        cx.spawn(async move |this, cx| {
            let tree = cx
                .background_executor()
                .spawn(async move { vault.list_tree().unwrap_or_default() })
                .await;
            let _ = window.update(cx, |_, window, cx| {
                let _ = this.update(cx, |shell, cx| {
                    shell.dispatch(Command::TreeScanned { generation, tree }, window, cx)
                });
            });
        })
        .detach();
    }

    fn spawn_tick(&self, delay: std::time::Duration, cx: &mut Context<Self>) -> Task<()> {
        let window = self.window;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = window.update(cx, |_, window, cx| {
                let _ = this.update(cx, |shell, cx| shell.dispatch(Command::Tick, window, cx));
            });
        })
    }

    fn spawn_watch_loop(
        &self,
        rx: async_channel::Receiver<frankmd_watch::WatchEvent>,
        cx: &mut Context<Self>,
    ) {
        let window = self.window;
        cx.spawn(async move |this, cx| {
            while let Ok(ev) = rx.recv().await {
                let stop = window
                    .update(cx, |_, window, cx| {
                        this.update(cx, |shell, cx| match ev {
                            frankmd_watch::WatchEvent::OmarchyThemeChanged
                            | frankmd_watch::WatchEvent::SystemLanguageChanged
                            | frankmd_watch::WatchEvent::FontsChanged
                            | frankmd_watch::WatchEvent::Error(_) => {
                                shell.apply_system(ev, window, cx)
                            }
                            other => shell.dispatch(Command::External(other), window, cx),
                        })
                        .is_err()
                    })
                    .unwrap_or(true);
                if stop {
                    break;
                }
            }
        })
        .detach();
    }
}
