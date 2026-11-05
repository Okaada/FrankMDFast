//! Recovery dialog (ui-spec §5.11, FE-145/146): the one blocking modal.
//! File on disk on the left, draft on the right, as a word diff (unified
//! below 900 px); Use file on disk / Keep both / Restore draft. Esc and the
//! backdrop do nothing.
//!
//! The same view serves the "changed on disk" banner's **Compare…**
//! (§5.12): disk version vs the editor, with Reload / Keep mine.

use crate::scale::px;
use frankmd_core::text::diff::{DiffKind, DiffOp, compute_word_diff, tokenize};
use frankmd_workspace::autosave::RecoveryRequest;

use super::prelude::*;
use gpui_kit::*;

/// Token-product limit for the LCS diff; larger pairs show plain columns.
const MAX_DIFF_CELLS: usize = 4_000_000;

/// What the dialog compares.
pub(crate) enum RecoveryView {
    /// A draft that differs from the file (autosave recovery).
    Draft(RecoveryRequest),
    /// The file changed on disk under unsaved edits.
    Compare {
        /// Note path.
        path: String,
        /// Bytes on disk now.
        disk: String,
        /// Editor text.
        mine: String,
    },
}

/// Dialog state: what is compared and its diff (computed once on open).
pub(crate) struct RecoveryState {
    pub(crate) view: RecoveryView,
    diff: Option<Vec<DiffOp>>,
}

impl RecoveryState {
    pub(crate) fn new(view: RecoveryView) -> Self {
        let (old, new) = match &view {
            RecoveryView::Draft(r) => (r.server_content.as_str(), r.backup_content.as_str()),
            RecoveryView::Compare { disk, mine, .. } => (disk.as_str(), mine.as_str()),
        };
        let cells = tokenize(old).len().saturating_mul(tokenize(new).len());
        let diff = (cells <= MAX_DIFF_CELLS).then(|| compute_word_diff(old, new));
        Self { view, diff }
    }

    fn texts(&self) -> (&str, &str, &str) {
        match &self.view {
            RecoveryView::Draft(r) => (&r.path, &r.server_content, &r.backup_content),
            RecoveryView::Compare { path, disk, mine } => (path, disk, mine),
        }
    }
}

/// `text` with the ops of `kinds` highlighted in `color` (one column of the diff).
fn diff_column(ops: &[DiffOp], keep: DiffKind, color: Hsla) -> StyledText {
    let mut text = String::new();
    let mut highlights = Vec::new();
    for op in ops {
        if op.kind != DiffKind::Equal && op.kind != keep {
            continue;
        }
        let start = text.len();
        text.push_str(&op.value);
        if op.kind == keep {
            highlights.push((
                start..text.len(),
                HighlightStyle {
                    background_color: Some(color),
                    ..Default::default()
                },
            ));
        }
    }
    StyledText::new(text).with_highlights(highlights)
}

/// Both sides in one column: deletions struck through, insertions tinted.
fn unified(ops: &[DiffOp], del: Hsla, ins: Hsla) -> StyledText {
    let mut text = String::new();
    let mut highlights = Vec::new();
    for op in ops {
        let start = text.len();
        text.push_str(&op.value);
        let style = match op.kind {
            DiffKind::Equal => continue,
            DiffKind::Delete => HighlightStyle {
                background_color: Some(del),
                strikethrough: Some(StrikethroughStyle {
                    thickness: px(1.),
                    color: None,
                }),
                ..Default::default()
            },
            DiffKind::Insert => HighlightStyle {
                background_color: Some(ins),
                ..Default::default()
            },
        };
        highlights.push((start..text.len(), style));
    }
    StyledText::new(text).with_highlights(highlights)
}

impl Shell {
    /// Show the recovery dialog for an autosave recovery request (the
    /// autosave session raises these; tests call it directly).
    pub fn open_recovery(
        &mut self,
        request: RecoveryRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.recovery = Some(RecoveryState::new(RecoveryView::Draft(request)));
        self.open_overlay(OverlayKind::Dialog("recovery"), window, cx);
    }

    /// Banner "Compare…": disk vs editor.
    pub(crate) fn open_compare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.active_path() else {
            return;
        };
        let Some(core) = self.core.as_ref() else {
            return;
        };
        let Ok(file) = frankmd_fs::service::NotesService::new(&core.vault).read(&path) else {
            return;
        };
        let disk = String::from_utf8_lossy(&file.bytes).into_owned();
        let mine = self.editor.text(cx);
        self.recovery = Some(RecoveryState::new(RecoveryView::Compare {
            path,
            disk,
            mine,
        }));
        self.open_overlay(OverlayKind::Dialog("recovery"), window, cx);
    }

    /// Recovery dialog answer: `Some(true)` restore draft, `Some(false)` use
    /// the file, `None` keep both.
    pub(crate) fn resolve_recovery(
        &mut self,
        keep_draft: Option<bool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use frankmd_workspace::autosave::RecoveryResolution;
        let Some(state) = self.recovery.take() else {
            return;
        };
        self.overlays.close();
        match state.view {
            RecoveryView::Draft(request) => {
                let cmd = match keep_draft {
                    Some(true) => {
                        Command::ResolveRecovery(RecoveryResolution::accept_local(&request))
                    }
                    Some(false) => {
                        Command::ResolveRecovery(RecoveryResolution::accept_server(&request))
                    }
                    None => Command::RecoveryKeepBoth(request),
                };
                self.dispatch(cmd, window, cx);
            }
            RecoveryView::Compare { .. } => {
                if keep_draft == Some(false) {
                    self.banner_action(
                        BannerKind::ChangedOnDisk,
                        "banners.changed_on_disk.reload",
                        window,
                        cx,
                    );
                } else {
                    // Keep mine: the next save writes the editor's text.
                    self.banners.clear(BannerKind::ChangedOnDisk);
                }
            }
        }
        if self.overlays.current().is_none() {
            self.focus_region(FocusRegion::Editor, window, cx);
        }
        cx.notify();
    }

    pub(super) fn render_recovery(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let Some(state) = self.recovery.as_ref() else {
            return div().into_any_element();
        };
        let compare = matches!(state.view, RecoveryView::Compare { .. });
        let (path, file, draft) = state.texts();
        let (path, file, draft) = (path.to_owned(), file.to_owned(), draft.to_owned());
        let narrow = f32::from(window.viewport_size().width) < 900.;
        let del = theme.danger.opacity(0.25);
        let ins = theme.success.opacity(0.25);
        let left_title = i18n::t(cx, "dialogs.recovery_native.file");
        let right_title = if compare {
            i18n::t(cx, "dialogs.recovery_native.mine")
        } else {
            i18n::t(cx, "dialogs.recovery_native.draft")
        };
        let column = |id: &'static str, title: SharedString, body: AnyElement| {
            v_flex()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(title),
                )
                .child(
                    div()
                        .id(id)
                        .test_support()
                        .flex_1()
                        .p_2()
                        .bg(theme.secondary)
                        .rounded(theme.radius)
                        .font_family(theme.mono_font_family.clone())
                        .text_xs()
                        .overflow_y_scroll()
                        .child(body),
                )
        };
        let columns: AnyElement = match (&state.diff, narrow) {
            (Some(ops), true) => column(
                "recovery-unified",
                format!("{left_title} → {right_title}").into(),
                unified(ops, del, ins).into_any_element(),
            )
            .into_any_element(),
            (Some(ops), false) => h_flex()
                .flex_1()
                .min_h_0()
                .gap_3()
                .items_start()
                .child(column(
                    "recovery-file",
                    left_title,
                    diff_column(ops, DiffKind::Delete, del).into_any_element(),
                ))
                .child(column(
                    "recovery-draft",
                    right_title,
                    diff_column(ops, DiffKind::Insert, ins).into_any_element(),
                ))
                .into_any_element(),
            (None, _) => h_flex()
                .flex_1()
                .min_h_0()
                .gap_3()
                .items_start()
                .child(column("recovery-file", left_title, file.into_any_element()))
                .child(column(
                    "recovery-draft",
                    right_title,
                    draft.into_any_element(),
                ))
                .into_any_element(),
        };
        let this = cx.entity().downgrade();
        let button =
            |id: &'static str,
             key: &str,
             answer: Option<bool>,
             primary: bool,
             cx: &mut Context<Self>| {
                let this = this.clone();
                let b = Button::new(id).small().label(i18n::t(cx, key)).on_click(
                    move |_, window, cx| {
                        let _ =
                            this.update(cx, |shell, cx| shell.resolve_recovery(answer, window, cx));
                    },
                );
                if primary { b.primary() } else { b }
            };
        let buttons: Vec<Button> = if compare {
            vec![
                button(
                    "recovery-use-file",
                    "dialogs.recovery_native.reload",
                    Some(false),
                    false,
                    cx,
                ),
                button(
                    "recovery-keep-mine",
                    "dialogs.recovery_native.keep_mine",
                    Some(true),
                    true,
                    cx,
                ),
            ]
        } else {
            vec![
                button(
                    "recovery-use-file",
                    "dialogs.recovery_native.use_file",
                    Some(false),
                    false,
                    cx,
                ),
                button(
                    "recovery-keep-both",
                    "dialogs.recovery_native.keep_both",
                    None,
                    false,
                    cx,
                ),
                button(
                    "recovery-restore-draft",
                    "dialogs.recovery_native.restore",
                    Some(true),
                    true,
                    cx,
                ),
            ]
        };
        let title = if compare {
            i18n::t(cx, "dialogs.recovery_native.compare_title")
        } else {
            i18n::t(cx, "dialogs.recovery_native.title")
        };
        let height = (f32::from(window.viewport_size().height) * 0.9).min(760.);
        v_flex()
            .id("dialog-recovery")
            .test_support()
            .track_focus(&self.overlay_focus)
            .w(px(1100.))
            .h(gpui_kit::px(height))
            .max_w(relative(1.))
            .p_4()
            .gap_3()
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .child(div().font_semibold().child(title))
            .child(div().text_color(theme.muted_foreground).child(path))
            .child(columns)
            .child(h_flex().gap_2().justify_end().children(buttons))
            .into_any_element()
    }
}
