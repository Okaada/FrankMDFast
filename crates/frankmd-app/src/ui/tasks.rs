//! Background work of a window: tree scans, autosave ticks, file watchers,
//! Hyprland radius and editor font lookups. Results come back as commands.

use frankmd_fs::Vault;

use super::appearance;
use super::prelude::*;
use gpui_kit::*;

impl Shell {
    pub(crate) fn start_watchers(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn load_hyprland_radius(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn load_editor_font(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn spawn_scan(
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

    pub(crate) fn spawn_tick(&self, delay: std::time::Duration, cx: &mut Context<Self>) -> Task<()> {
        let window = self.window;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = window.update(cx, |_, window, cx| {
                let _ = this.update(cx, |shell, cx| shell.dispatch(Command::Tick, window, cx));
            });
        })
    }

    pub(crate) fn spawn_watch_loop(
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
