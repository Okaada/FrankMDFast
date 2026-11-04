//! Logic port of test/javascript/controllers/autosave_controller.test.js
//! (+ the dispatch-payload tests of recovery_diff_controller.test.js) against
//! `frankmd_workspace::autosave::AutosaveSession` with a fake clock.
//!
//! Mapping of the original harness:
//! - `mockCodemirrorValue = x`        → `h.edit(x)` (editor mirror, no scheduling)
//! - `controller.scheduleAutoSave()`  → `h.schedule()`
//! - `global.fetch` called            → an `AutosaveEffect::Save` was emitted
//! - resolving the fetch              → `h.finish(req, SaveOutcome::…)`
//! - `vi.advanceTimersByTime(ms)`     → `h.advance(ms)` (clock + `tick`)
//! - `draftStorage.writeDraft` mocked to fail → a backend `setItem` fault
//! - `recovery.open(...)`             → `AutosaveEffect::OpenRecovery`
//! - banner `hidden` class            → last `AutosaveEffect::ContentLossBanner`
#![allow(clippy::unwrap_used)]

#[path = "draft_common/mod.rs"]
mod draft_common;

use draft_common::{Fault, Method, Session, make_content, session};
use frankmd_workspace::autosave::{
    AutosaveEffect, DeletionPrep, RecoveryRequest, RecoveryResolution, RecoverySource, SaveOutcome,
    SaveRequest, SaveStatus,
};
use frankmd_workspace::clock::{Clock, FakeClock};
use frankmd_workspace::draft_storage::{
    DraftRecord, ItemKind, backup_key, draft_conflict_path_prefix, draft_key,
};
use std::time::Instant;

const SAVE_DEBOUNCE_MS: u64 = 2000;
const DRAFT_DEBOUNCE_MS: u64 = 300;

struct H {
    s: Session,
    clock: FakeClock,
    saves: Vec<SaveRequest>,
    effects: Vec<AutosaveEffect>,
}

impl H {
    fn new() -> Self {
        let (s, clock) = session();
        Self { s, clock, saves: vec![], effects: vec![] }
    }
    fn now(&self) -> Instant {
        self.clock.now()
    }
    fn pump(&mut self) {
        for e in self.s.take_effects() {
            if let AutosaveEffect::Save(r) = &e {
                self.saves.push(r.clone());
            }
            self.effects.push(e);
        }
    }
    fn edit(&mut self, text: &str) {
        self.s.set_editor_text(text);
    }
    fn set_file(&mut self, path: &str, content: &str, rev: Option<&str>) {
        self.s.set_file(path, content, rev);
        self.pump();
    }
    fn schedule(&mut self) {
        let now = self.now();
        self.s.schedule_autosave(now);
        self.pump();
    }
    fn schedule_draft(&mut self) {
        let now = self.now();
        self.s.schedule_draft_write(now);
        self.pump();
    }
    fn advance(&mut self, ms: u64) {
        self.clock.advance_ms(ms);
        let now = self.now();
        self.s.tick(now);
        self.pump();
    }
    /// `saveNow()`; returns the request if one was emitted.
    fn save_now(&mut self) -> Option<SaveRequest> {
        let before = self.saves.len();
        let now = self.now();
        self.s.save_now(now);
        self.pump();
        self.saves.get(before).cloned()
    }
    fn finish(&mut self, req: &SaveRequest, outcome: SaveOutcome) {
        let now = self.now();
        self.s.save_finished(req.ticket, outcome, now);
        self.pump();
    }
    fn ok(rev: Option<&str>) -> SaveOutcome {
        SaveOutcome::Saved { revision: rev.map(str::to_owned) }
    }
    /// `await controller.saveNow()` with a fetch that resolves `{ok:true, json:{}}`.
    fn save_now_ok(&mut self) -> Option<SaveRequest> {
        let req = self.save_now()?;
        self.finish(&req, Self::ok(None));
        Some(req)
    }
    fn banner_visible(&self) -> bool {
        self.effects
            .iter()
            .rev()
            .find_map(|e| match e {
                AutosaveEffect::ContentLossBanner(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(false)
    }
    fn recoveries(&self) -> Vec<RecoveryRequest> {
        self.effects
            .iter()
            .filter_map(|e| match e {
                AutosaveEffect::OpenRecovery(r) => Some(r.clone()),
                _ => None,
            })
            .collect()
    }
    fn editor_sets(&self) -> Vec<String> {
        self.effects
            .iter()
            .filter_map(|e| match e {
                AutosaveEffect::SetEditorText(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    }
    fn has_effect(&self, want: &AutosaveEffect) -> bool {
        self.effects.contains(want)
    }
    fn draft(&mut self, path: &str) -> Option<DraftRecord> {
        self.s.drafts_mut().read_draft(path).unwrap().draft
    }
    fn write_draft(&mut self, path: &str, content: &str, rev: &str) -> DraftRecord {
        self.s.drafts_mut().write_draft(path, content, rev).unwrap().draft
    }
    fn fault(&mut self, f: Fault) {
        self.s.drafts_mut().backend_mut().add_fault(f);
    }
    fn clear_faults(&mut self) {
        self.s.drafts_mut().backend_mut().clear_faults();
    }
    fn rename(&mut self, old: &str, new: &str, kind: ItemKind) -> bool {
        let now = self.now();
        let moved = self.s.rename_file(old, new, kind, now);
        self.pump();
        moved
    }
    fn resolve(&mut self, r: RecoveryResolution) {
        let now = self.now();
        self.s.recovery_resolved(r, now);
        self.pump();
    }
}

fn resolution(source: RecoverySource) -> RecoveryResolution {
    RecoveryResolution {
        source,
        path: None,
        content: None,
        draft_revision: None,
        backup_content: None,
        backup_timestamp: None,
        conflict_id: None,
        server_content: None,
    }
}

// ===========================================================================
// saveNow() — content loss detection
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:142 "no warning for small deletion (< 20% or < 50 chars)"
#[test]
fn no_warning_for_small_deletion() {
    let mut h = H::new();
    h.set_file("test.md", &make_content(200), Some("revision-test"));
    h.edit(&make_content(180));

    assert!(h.save_now().is_some(), "proceeds with the save");
    assert!(!h.banner_visible());
    assert!(!h.s.content_loss_warning_active());
}

// port: test/javascript/controllers/autosave_controller.test.js:155 "shows warning for large deletion (> 20% AND > 50 chars)"
#[test]
fn shows_warning_for_large_deletion() {
    let mut h = H::new();
    h.set_file("test.md", &make_content(300), Some("revision-test"));
    h.edit("");

    assert!(h.save_now().is_none(), "save blocked");
    assert!(h.banner_visible());
    assert!(h.s.content_loss_warning_active());
    let d = h.draft("test.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("", "revision-test"));
}

// port: test/javascript/controllers/autosave_controller.test.js:174 "no warning when _contentLossOverride is true"
#[test]
fn no_warning_when_content_loss_override_is_true() {
    let mut h = H::new();
    h.set_file("test.md", &make_content(300), Some("revision-test"));
    h.edit("");
    h.s.set_content_loss_override(true);

    assert!(h.save_now().is_some());
}

// port: test/javascript/controllers/autosave_controller.test.js:185 "no warning when _lastSavedContent is null (fresh file)"
// The Rust session always has a baseline once a file is set; an empty baseline is the
// equivalent "nothing to lose" case (`baseline.length === 0` short-circuits the guard).
#[test]
fn no_warning_when_there_is_no_saved_baseline() {
    let mut h = H::new();
    h.set_file("test.md", "", None);
    h.edit("some new content");

    assert!(h.save_now().is_some());
}

// port: test/javascript/controllers/autosave_controller.test.js:196 "resets _contentLossOverride after successful save"
#[test]
fn resets_content_loss_override_after_successful_save() {
    let mut h = H::new();
    h.set_file("test.md", &make_content(300), Some("revision-test"));
    h.edit("");
    h.s.set_content_loss_override(true);

    h.save_now_ok().unwrap();

    assert!(!h.s.content_loss_override());
}

// ===========================================================================
// scheduleAutoSave()
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:213 "blocked while warning is active"
#[test]
fn schedule_blocked_while_warning_is_active() {
    let mut h = H::new();
    h.set_file("test.md", "saved", Some("revision-test"));
    h.edit("changed");
    h.s.show_content_loss_warning();

    h.schedule();

    assert!(!h.s.save_timer_armed());
    assert!(h.s.has_unsaved_changes());
}

// port: test/javascript/controllers/autosave_controller.test.js:227 "does not mark unchanged content unsaved after a change event"
#[test]
fn does_not_mark_unchanged_content_unsaved_after_a_change_event() {
    let mut h = H::new();
    h.set_file("test.md", "saved", Some("revision-test"));
    h.edit("saved");

    h.schedule();

    assert!(!h.s.has_unsaved_changes());
    assert!(!h.s.save_timer_armed());
    assert_eq!(h.s.status(), SaveStatus::Hidden);
    assert_eq!(h.draft("test.md"), None);
}

// port: test/javascript/controllers/autosave_controller.test.js:239 "clears the unsaved status when an edit returns to saved content"
#[test]
fn clears_the_unsaved_status_when_an_edit_returns_to_saved_content() {
    let mut h = H::new();
    h.set_file("test.md", "saved", Some("revision-test"));
    h.edit("changed");
    h.schedule();
    assert!(h.s.has_unsaved_changes());
    assert_eq!(h.s.status(), SaveStatus::Unsaved);

    h.edit("saved");
    h.schedule();

    assert!(!h.s.has_unsaved_changes());
    assert!(!h.s.save_timer_armed());
    assert!(!h.s.max_interval_timer_armed());
    assert_eq!(h.s.status(), SaveStatus::Hidden);
    assert_eq!(h.draft("test.md"), None);
}

// ===========================================================================
// Browser lifecycle draft flush (visibilitychange hidden / pagehide → flush_active_draft)
// ===========================================================================

fn lifecycle() -> H {
    let mut h = H::new();
    h.set_file("lifecycle.md", "server content", Some("revision-lifecycle"));
    h.edit("local content");
    h.schedule();
    h
}

// port: test/javascript/controllers/autosave_controller.test.js:266 "flushes immediately when the document becomes hidden without a server request"
#[test]
fn flushes_immediately_when_hidden_without_a_server_request() {
    let mut h = lifecycle();
    h.s.flush_active_draft().unwrap();
    h.pump();

    let d = h.draft("lifecycle.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("local content", "revision-lifecycle"));
    assert!(h.saves.is_empty());
    assert_eq!(h.s.pending_draft_writes(), 0);
}

// port: test/javascript/controllers/autosave_controller.test.js:279 "flushes immediately on pagehide without a server request"
#[test]
fn flushes_immediately_on_pagehide_without_a_server_request() {
    let mut h = lifecycle();
    h.s.flush_active_draft().unwrap();
    h.pump();

    let d = h.draft("lifecycle.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("local content", "revision-lifecycle"));
    assert!(h.saves.is_empty());
}

// port: test/javascript/controllers/autosave_controller.test.js:289 "flushes a dirty empty draft on %s"
#[test]
fn flushes_a_dirty_empty_draft_on_lifecycle_events() {
    for _event in ["visibilitychange", "pagehide"] {
        let mut h = lifecycle();
        h.edit("");
        h.s.flush_active_draft().unwrap();
        assert_eq!(h.draft("lifecycle.md").unwrap().content, "");
    }
}

// port: test/javascript/controllers/autosave_controller.test.js:301 "does not rewrite an unchanged draft for repeated lifecycle events"
#[test]
fn does_not_rewrite_an_unchanged_draft_for_repeated_lifecycle_events() {
    let mut h = lifecycle();
    assert_eq!(h.draft("lifecycle.md"), None);

    h.s.flush_active_draft().unwrap();
    let first = h.draft("lifecycle.md").unwrap();
    assert_eq!((first.content.as_str(), first.base_revision.as_str()), ("local content", "revision-lifecycle"));
    let writes = h.s.drafts().backend().set_calls;

    h.s.flush_active_draft().unwrap();

    assert_eq!(h.draft("lifecycle.md"), Some(first));
    assert_eq!(h.s.drafts().backend().set_calls, writes, "unchanged draft not rewritten");
}

// port: test/javascript/controllers/autosave_controller.test.js:317 "shows a storage error and requests a best-effort unload warning when persistence fails"
#[test]
fn shows_a_storage_error_and_requests_an_unload_warning_when_persistence_fails() {
    let mut h = lifecycle();
    h.fault(Fault::throw(Method::SetItem).on_key(&draft_key("lifecycle.md")).message("storage quota exceeded"));

    let _ = h.s.flush_active_draft();
    let prevent_close = h.s.before_unload();
    h.pump();

    assert_eq!(h.s.status(), SaveStatus::DraftStorageError);
    assert!(h.s.status().is_error());
    assert!(prevent_close, "close/unload must be warned about");
    assert!(h.has_effect(&AutosaveEffect::UnloadGuard(true)));

    h.clear_faults();
    h.s.flush_active_draft().unwrap();
    h.pump();

    assert_eq!(h.s.status(), SaveStatus::Unsaved);
    assert!(!h.s.unload_guard_active());
}

// ===========================================================================
// Large-deletion warning file scope
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:345 "does not let file A's warning block autosave for file B"
#[test]
fn file_a_warning_does_not_block_autosave_for_file_b() {
    let mut h = H::new();
    h.set_file("a.md", &make_content(300), Some("revision-a"));
    h.edit("");
    assert!(h.save_now().is_none());
    assert!(h.s.content_loss_warning_active());
    assert_eq!(h.draft("a.md").unwrap().content, "");

    h.s.prepare_for_transition().unwrap();
    h.set_file("b.md", "server B", Some("revision-b"));
    h.edit("edited B");
    h.schedule();
    h.advance(SAVE_DEBOUNCE_MS);

    assert_eq!(h.saves.len(), 1);
    assert_eq!(h.saves[0].path, "b.md");
    assert!(!h.s.content_loss_warning_active());
    assert_eq!(h.draft("a.md").unwrap().content, "");
}

// port: test/javascript/controllers/autosave_controller.test.js:365 "does not reuse save-anyway approval for a later editor snapshot"
#[test]
fn does_not_reuse_save_anyway_approval_for_a_later_editor_snapshot() {
    let mut h = H::new();
    h.set_file("a.md", &make_content(300), Some("revision-a"));
    h.edit("");
    h.s.set_content_loss_override(true);
    h.edit(&make_content(100));

    assert!(h.save_now().is_none());
    assert!(h.s.content_loss_warning_active());
    assert_eq!(h.draft("a.md").unwrap().content, make_content(100));
}

// port: test/javascript/controllers/autosave_controller.test.js:378 "recomputes the warning when a protected draft is restored against its baseline"
#[test]
fn recomputes_the_warning_when_a_protected_draft_is_restored_against_its_baseline() {
    let mut h = H::new();
    let baseline = make_content(300);
    h.write_draft("a.md", "", "revision-a");

    h.set_file("a.md", &baseline, Some("revision-a"));
    assert_eq!(h.s.recover_draft(&baseline, "revision-a"), "");
    h.pump();

    assert!(h.s.content_loss_warning_active());
    assert!(h.banner_visible());
}

// port: test/javascript/controllers/autosave_controller.test.js:389 "recomputes a divergent recovered draft against the latest server baseline"
#[test]
fn recomputes_a_divergent_recovered_draft_against_the_latest_server_baseline() {
    let mut h = H::new();
    let latest = make_content(300);
    let draft = h.write_draft("a.md", "", "old-revision");
    h.set_file("a.md", &latest, Some("latest-revision"));
    h.s.recover_draft(&latest, "latest-revision");
    h.pump();

    h.resolve(RecoveryResolution {
        content: Some(String::new()),
        draft_revision: Some(draft.draft_revision),
        ..resolution(RecoverySource::Draft)
    });

    assert!(h.s.content_loss_warning_active());
    let d = h.draft("a.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("", "latest-revision"));
}

// port: test/javascript/controllers/autosave_controller.test.js:410 "undo clears the saved draft when it restores the server baseline"
#[test]
fn undo_clears_the_saved_draft_when_it_restores_the_server_baseline() {
    let mut h = H::new();
    let baseline = make_content(300);
    h.set_file("a.md", &baseline, Some("revision-a"));
    h.edit("");
    h.save_now();
    assert!(h.draft("a.md").is_some());

    // The app runs the editor's undo (which restores the baseline) and reports it.
    h.edit(&baseline);
    h.s.undo_content_loss(true);
    h.pump();

    assert!(!h.s.content_loss_warning_active());
    assert_eq!(h.draft("a.md"), None);
}

// port: test/javascript/controllers/autosave_controller.test.js:427 "undo updates the protected draft when the deletion remains large"
#[test]
fn undo_updates_the_protected_draft_when_the_deletion_remains_large() {
    let mut h = H::new();
    let baseline = make_content(300);
    h.set_file("a.md", &baseline, Some("revision-a"));
    h.edit("");
    h.save_now();

    h.edit(&make_content(100));
    h.s.undo_content_loss(true);
    h.pump();

    assert!(h.s.content_loss_warning_active());
    let d = h.draft("a.md").unwrap();
    assert_eq!((d.content, d.base_revision.as_str()), (make_content(100), "revision-a"));
}

// port: test/javascript/controllers/autosave_controller.test.js:446 "restores the server baseline when a recovered deletion has no editor history"
#[test]
fn restores_the_server_baseline_when_a_recovered_deletion_has_no_editor_history() {
    let mut h = H::new();
    let baseline = make_content(300);
    h.set_file("a.md", &baseline, Some("revision-a"));
    h.edit(&make_content(100));
    let saved = h.s.flush_draft_write("a.md", &make_content(100), Some("revision-a")).unwrap().unwrap();
    assert_eq!(h.s.draft_revision(), Some(saved.draft_revision.as_str()));
    h.s.show_content_loss_warning();

    h.s.undo_content_loss(false);
    h.pump();

    assert!(h.has_effect(&AutosaveEffect::LoadEditorText { content: baseline.clone(), path: "a.md".into() }));
    assert_eq!(h.s.editor_text(), baseline, "the session mirrors the reloaded text");
    assert!(!h.s.content_loss_warning_active());
    assert_eq!(h.draft("a.md"), None);
}

// port: test/javascript/controllers/autosave_controller.test.js:463 "save anyway removes the matching draft only after success"
#[test]
fn save_anyway_removes_the_matching_draft_only_after_success() {
    let mut h = H::new();
    h.set_file("a.md", &make_content(300), Some("revision-a"));
    h.edit("");
    h.save_now();
    let revision = h.draft("a.md").unwrap().draft_revision;

    let now = h.now();
    h.s.save_anyway_after_warning(now);
    h.pump();
    let req = h.saves.last().cloned().expect("save anyway sends the save");
    h.finish(&req, H::ok(None));

    assert_eq!(h.s.drafts_mut().remove_draft_if_revision("a.md", &revision), Ok(false));
    assert_eq!(h.draft("a.md"), None);
}

// port: test/javascript/controllers/autosave_controller.test.js:475 "failed save anyway keeps its local draft"
#[test]
fn failed_save_anyway_keeps_its_local_draft() {
    let mut h = H::new();
    h.set_file("a.md", &make_content(300), Some("revision-a"));
    h.edit("");
    h.save_now();
    let saved = h.draft("a.md").unwrap();

    let now = h.now();
    h.s.save_anyway_after_warning(now);
    h.pump();
    let req = h.saves.last().cloned().unwrap();
    h.finish(&req, SaveOutcome::Failed);

    assert_eq!(h.draft("a.md").unwrap().draft_revision, saved.draft_revision);
}

// ===========================================================================
// File transition preparation and save scoping
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:489 "flushes the outgoing snapshot before cancelling timers without forcing a server save"
#[test]
fn transition_flushes_the_outgoing_snapshot_before_cancelling_timers() {
    let mut h = H::new();
    h.set_file("a.md", "server A", Some("revision-a"));
    h.edit("local draft A");
    h.schedule();

    let result = h.s.prepare_for_transition();
    h.pump();

    assert!(result.is_ok());
    let d = h.draft("a.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("local draft A", "revision-a"));
    assert!(!h.s.save_timer_armed());
    assert!(!h.s.max_interval_timer_armed());
    assert!(h.saves.is_empty());
}

// port: test/javascript/controllers/autosave_controller.test.js:509 "keeps the outgoing file active and does not cancel its save timers when draft storage fails"
#[test]
fn transition_keeps_the_file_active_and_timers_when_draft_storage_fails() {
    let mut h = H::new();
    h.fault(Fault::throw(Method::SetItem).on_key(&draft_key("a.md")).message("storage unavailable"));
    h.set_file("a.md", "server A", Some("revision-a"));
    h.edit("local draft A");
    h.schedule();

    let err = h.s.prepare_for_transition().unwrap_err();
    h.pump();

    assert_eq!(err.message, "storage unavailable");
    assert_eq!(h.s.current_file(), Some("a.md"));
    assert!(h.s.save_timer_armed());
    assert!(h.s.max_interval_timer_armed());
    assert!(h.saves.is_empty());
}

// port: test/javascript/controllers/autosave_controller.test.js:527 "blocks a dirty transition when the file has no server revision to anchor its draft"
#[test]
fn blocks_a_dirty_transition_without_a_server_revision() {
    let mut h = H::new();
    h.set_file("a.md", "server A", None);
    h.edit("local draft A");

    assert!(h.s.prepare_for_transition().is_err());
    h.pump();
    assert_eq!(h.s.current_file(), Some("a.md"));
    assert!(h.saves.is_empty());
}

// port: test/javascript/controllers/autosave_controller.test.js:539 "never lets an autosave scheduled for A read or save B"
#[test]
fn never_lets_an_autosave_scheduled_for_a_save_b() {
    let mut h = H::new();
    h.set_file("a.md", "server A", Some("revision-a"));
    h.edit("draft A");
    h.schedule();

    h.set_file("b.md", "server B", Some("revision-b"));
    h.edit("draft B");
    h.advance(SAVE_DEBOUNCE_MS);

    assert!(h.saves.is_empty());
    assert_eq!(h.s.current_file(), Some("b.md"));
}

// port: test/javascript/controllers/autosave_controller.test.js:555 "does not require a server save while offline before preparing a transition"
#[test]
fn transition_does_not_require_a_server_save_while_offline() {
    let mut h = H::new();
    h.s.connection_lost();
    h.set_file("a.md", "server A", Some("revision-a"));
    h.edit("local draft A");
    h.schedule();

    assert!(h.s.prepare_for_transition().is_ok());
    h.pump();
    assert_eq!(h.draft("a.md").unwrap().content, "local draft A");
    assert!(h.saves.is_empty());
}

// port: test/javascript/controllers/autosave_controller.test.js:571 "does not apply an in-flight A save response to the active B state"
#[test]
fn does_not_apply_an_in_flight_a_save_response_to_b() {
    let mut h = H::new();
    h.set_file("a.md", "server A", Some("revision-a"));
    h.edit("draft A");
    let req = h.save_now().unwrap();

    h.set_file("b.md", "server B", Some("revision-b"));
    h.edit("draft B");
    h.finish(&req, H::ok(Some("saved-revision-a")));

    assert_eq!(h.s.current_file(), Some("b.md"));
    assert_eq!(h.s.base_revision(), Some("revision-b"));
    assert_eq!(h.s.last_saved_content(), Some("server B"));
    assert_eq!(h.s.status(), SaveStatus::Hidden);
}

// ===========================================================================
// Online draft persistence and recovery
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:595 "debounces an online empty draft and stores it independently by path"
#[test]
fn debounces_an_online_empty_draft_by_path() {
    let mut h = H::new();
    h.set_file("empty.md", "server content", Some("server-revision"));
    h.edit("");

    h.schedule();
    h.advance(DRAFT_DEBOUNCE_MS);

    let d = h.draft("empty.md").unwrap();
    assert_eq!((d.path.as_str(), d.content.as_str(), d.base_revision.as_str()), ("empty.md", "", "server-revision"));
}

// port: test/javascript/controllers/autosave_controller.test.js:609 "keeps pending snapshots for different files independent"
#[test]
fn keeps_pending_snapshots_for_different_files_independent() {
    let mut h = H::new();
    h.set_file("first.md", "first server", Some("first-revision"));
    h.edit("first draft");
    h.schedule_draft();

    h.set_file("second.md", "second server", Some("second-revision"));
    h.edit("second draft");
    h.schedule_draft();
    h.advance(DRAFT_DEBOUNCE_MS);

    let a = h.draft("first.md").unwrap();
    let b = h.draft("second.md").unwrap();
    assert_eq!((a.content.as_str(), a.base_revision.as_str()), ("first draft", "first-revision"));
    assert_eq!((b.content.as_str(), b.base_revision.as_str()), ("second draft", "second-revision"));
}

// port: test/javascript/controllers/autosave_controller.test.js:624 "automatically restores a draft when its server baseline still matches"
#[test]
fn automatically_restores_a_draft_when_its_baseline_still_matches() {
    let mut h = H::new();
    h.write_draft("test.md", "local draft", "same-baseline");
    h.set_file("test.md", "server version", Some("same-baseline"));

    assert_eq!(h.s.recover_draft("server version", "same-baseline"), "local draft");
    assert!(h.s.has_unsaved_changes());
}

// port: test/javascript/controllers/autosave_controller.test.js:632 "clears a draft when the server already contains its content"
#[test]
fn clears_a_draft_when_the_server_already_contains_its_content() {
    let mut h = H::new();
    let write = h.write_draft("test.md", "already saved", "old-baseline");
    h.set_file("test.md", "already saved", Some("current-baseline"));

    assert_eq!(h.s.recover_draft("already saved", "current-baseline"), "already saved");
    assert_eq!(h.s.drafts_mut().remove_draft_if_revision("test.md", &write.draft_revision), Ok(false));
    assert_eq!(h.draft("test.md"), None);
}

// port: test/javascript/controllers/autosave_controller.test.js:641 "preserves and opens a diverged draft in the recovery workflow"
#[test]
fn preserves_and_opens_a_diverged_draft_in_the_recovery_workflow() {
    let mut h = H::new();
    let write = h.write_draft("test.md", "local draft", "old-baseline");
    h.set_file("test.md", "different server version", Some("new-baseline"));

    assert_eq!(h.s.recover_draft("different server version", "new-baseline"), "different server version");
    h.pump();

    assert_eq!(
        h.recoveries(),
        vec![RecoveryRequest {
            path: "test.md".into(),
            server_content: "different server version".into(),
            backup_content: "local draft".into(),
            backup_timestamp: write.updated_at,
            source: RecoverySource::Draft,
            draft_revision: Some(write.draft_revision.clone()),
            conflict_id: None,
        }]
    );
    assert_eq!(h.draft("test.md").unwrap().draft_revision, write.draft_revision);
}

fn collision_setup() -> (H, DraftRecord, DraftRecord) {
    let mut h = H::new();
    h.set_file("old.md", "source server", Some("source-revision"));
    h.edit("source local draft");
    let source = h.write_draft("old.md", "source local draft", "source-revision");
    let destination = h.write_draft("new.md", "destination local draft", "destination-revision");
    h.rename("old.md", "new.md", ItemKind::File);
    (h, source, destination)
}

// port: test/javascript/controllers/autosave_controller.test.js:659 "opens a collision recovery choice and promotes the selected moved draft safely"
#[test]
fn opens_a_collision_recovery_choice_and_promotes_the_moved_draft_safely() {
    let (mut h, source, destination) = collision_setup();
    let conflict = h.s.drafts_mut().list_draft_conflicts("new.md").unwrap()[0].clone();

    let opened = h.recoveries();
    let req = opened.last().expect("recovery opened");
    assert_eq!(req.path, "new.md");
    assert_eq!(req.server_content, "source server");
    assert_eq!(req.backup_content, "source local draft");
    assert_eq!(req.source, RecoverySource::DraftConflict);
    assert_eq!(req.conflict_id.as_deref(), Some(conflict.conflict_id.as_str()));
    assert_eq!(h.draft("old.md"), None);
    assert!(
        h.s.drafts_mut()
            .list_draft_conflicts("new.md")
            .unwrap()
            .iter()
            .any(|c| c.source_path == "old.md" && c.content == source.content)
    );
    assert_eq!(h.draft("new.md"), Some(destination.clone()));

    let now = h.now();
    h.s.resolve_draft_conflict(
        "new.md",
        &conflict.conflict_id,
        &conflict.draft_revision,
        Some(&conflict.content),
        None,
        true,
        now,
    );
    h.pump();

    assert_eq!(h.draft("new.md").unwrap().content, "source local draft");
    assert_eq!(h.draft("old.md"), None);
    let left = h.s.drafts_mut().list_draft_conflicts("new.md").unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].source_path, "new.md");
    assert_eq!(left[0].content, "destination local draft");
    assert_eq!(left[0].draft_revision, destination.draft_revision);
    assert!(!h.s.is_draft_blocked("new.md"));
}

// port: test/javascript/controllers/autosave_controller.test.js:705 "keeps the destination draft as a recovery copy when the user chooses the server version"
#[test]
fn keeps_the_destination_draft_as_a_recovery_copy_when_choosing_the_server_version() {
    let (mut h, source, destination) = collision_setup();
    let conflict = h.s.drafts_mut().list_draft_conflicts("new.md").unwrap()[0].clone();

    h.resolve(RecoveryResolution {
        path: Some("new.md".into()),
        server_content: Some("source server".into()),
        conflict_id: Some(conflict.conflict_id.clone()),
        draft_revision: Some(conflict.draft_revision.clone()),
        ..resolution(RecoverySource::Server)
    });

    assert!(h.editor_sets().contains(&"source server".to_owned()));
    assert_eq!(h.draft("old.md"), None);
    assert_eq!(h.draft("new.md"), None);
    let left = h.s.drafts_mut().list_draft_conflicts("new.md").unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].content, "destination local draft");
    assert_eq!(left[0].draft_revision, destination.draft_revision);
    assert_ne!(left[0].draft_revision, source.draft_revision);
}

// port: test/javascript/controllers/autosave_controller.test.js:734 "keeps a divergent draft recoverable after Escape is canceled and an edit schedules autosave"
// (The dialog's Escape-cancel itself is UI; the logic part is that a pending recovery
// blocks saves and draft writes.)
#[test]
fn keeps_a_divergent_draft_recoverable_while_recovery_is_pending() {
    let mut h = H::new();
    let original = h.write_draft("test.md", "original divergent draft", "old-baseline");
    h.set_file("test.md", "server version", Some("new-baseline"));

    assert_eq!(h.s.recover_draft("server version", "new-baseline"), "server version");
    h.pump();
    assert_eq!(h.recoveries().len(), 1);
    assert!(h.s.has_pending_recovery("test.md"));

    h.edit("edit after attempted dismissal");
    h.schedule();
    h.advance(SAVE_DEBOUNCE_MS + DRAFT_DEBOUNCE_MS);

    assert!(h.saves.is_empty());
    let d = h.draft("test.md").unwrap();
    assert_eq!(d.content, "original divergent draft");
    assert_eq!(d.base_revision, "old-baseline");
    assert_eq!(d.draft_revision, original.draft_revision);
}

// port: test/javascript/controllers/autosave_controller.test.js:761 "prefers a draft over a stale legacy backup and removes the superseded backup"
#[test]
fn prefers_a_draft_over_a_stale_legacy_backup() {
    let mut h = H::new();
    let write = h.write_draft("test.md", "new draft", "baseline");
    let ts = write.updated_at as u64 - 1;
    h.s.drafts_mut()
        .backend_mut()
        .put_raw(&backup_key("test.md"), &format!(r#"{{"content":"old backup","timestamp":{ts}}}"#));
    h.set_file("test.md", "server", Some("baseline"));

    assert_eq!(h.s.recover_draft("server", "baseline"), "new draft");
    assert_eq!(h.s.drafts().backend().raw(&backup_key("test.md")), None);
}

// port: test/javascript/controllers/autosave_controller.test.js:770 "opens a newer, different legacy backup for an explicit recovery choice"
#[test]
fn opens_a_newer_different_legacy_backup_for_an_explicit_choice() {
    let mut h = H::new();
    let write = h.write_draft("test.md", "draft", "baseline");
    let ts = write.updated_at as u64 + 1;
    h.s.drafts_mut()
        .backend_mut()
        .put_raw(&backup_key("test.md"), &format!(r#"{{"content":"newer backup","timestamp":{ts}}}"#));
    h.set_file("test.md", "server", Some("baseline"));

    h.s.recover_draft("server", "baseline");
    h.pump();

    assert_eq!(
        h.recoveries(),
        vec![RecoveryRequest {
            path: "test.md".into(),
            server_content: "server".into(),
            backup_content: "newer backup".into(),
            backup_timestamp: ts as f64,
            source: RecoverySource::Backup,
            draft_revision: Some(write.draft_revision.clone()),
            conflict_id: None,
        }]
    );
    assert_eq!(h.draft("test.md").unwrap().draft_revision, write.draft_revision);
}

// port: test/javascript/controllers/autosave_controller.test.js:791 "reports draft storage failures through the save status"
#[test]
fn reports_draft_storage_failures_through_the_save_status() {
    let mut h = H::new();
    h.fault(Fault::throw(Method::SetItem).on_key(&draft_key("test.md")).message("storage unavailable"));
    h.set_file("test.md", "server", Some("baseline"));
    h.edit("local edit");

    h.schedule_draft();
    h.advance(DRAFT_DEBOUNCE_MS);

    assert_eq!(h.s.status(), SaveStatus::DraftStorageError);
    assert!(h.s.status().is_error());
}

// port: test/javascript/controllers/autosave_controller.test.js:808 "restores an explicitly selected empty draft"
#[test]
fn restores_an_explicitly_selected_empty_draft() {
    let mut h = H::new();
    let write = h.write_draft("test.md", "", "old-baseline");
    h.set_file("test.md", "server content", Some("current-baseline"));

    h.resolve(RecoveryResolution {
        content: Some(String::new()),
        draft_revision: Some(write.draft_revision),
        ..resolution(RecoverySource::Draft)
    });

    assert!(h.editor_sets().contains(&String::new()));
    let d = h.draft("test.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("", "current-baseline"));
}

// port: test/javascript/controllers/autosave_controller.test.js:820 "keeps a newer draft when an earlier save completes"
#[test]
fn keeps_a_newer_draft_when_an_earlier_save_completes() {
    let mut h = H::new();
    h.set_file("test.md", "server", Some("baseline"));
    h.edit("saved snapshot");
    let req = h.save_now().unwrap();

    let sent = h.draft("test.md").unwrap();
    h.edit("newer edit");
    h.schedule_draft();
    h.advance(DRAFT_DEBOUNCE_MS + 20);
    let newer = h.draft("test.md").unwrap();
    assert_ne!(newer.draft_revision, sent.draft_revision);

    h.finish(&req, H::ok(Some("saved-revision")));

    let d = h.draft("test.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("newer edit", "saved-revision"));
}

// ===========================================================================
// Content-loss banner controls
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:850 "hides banner and resets flags"
#[test]
fn dismiss_hides_banner_and_resets_flags() {
    let mut h = H::new();
    h.set_file("test.md", "x", Some("r"));
    h.s.show_content_loss_warning();
    h.pump();
    assert!(h.s.content_loss_warning_active());
    assert!(h.banner_visible());

    h.s.dismiss_content_loss_warning();
    h.pump();

    assert!(!h.banner_visible());
    assert!(!h.s.content_loss_warning_active());
    assert!(!h.s.content_loss_override());
}

// port: test/javascript/controllers/autosave_controller.test.js:869 "sets override, dismisses banner, and calls saveNow"
#[test]
fn save_anyway_sets_override_dismisses_banner_and_saves() {
    let mut h = H::new();
    h.set_file("test.md", &make_content(300), Some("revision-test"));
    h.edit("");
    h.s.show_content_loss_warning();

    let now = h.now();
    h.s.save_anyway_after_warning(now);
    h.pump();

    assert!(!h.banner_visible());
    assert!(h.s.content_loss_override(), "override held until the save succeeds");
    assert_eq!(h.saves.len(), 1, "saveNow ran and sent the save");
}

// port: test/javascript/controllers/autosave_controller.test.js:890 "auto-dismisses warning when content is restored (e.g. Ctrl+Z)"
#[test]
fn check_content_restored_auto_dismisses_warning() {
    let mut h = H::new();
    let original = make_content(300);
    h.set_file("test.md", &original, Some("r"));
    h.s.show_content_loss_warning();

    h.s.check_content_restored(&original);
    h.pump();

    assert!(!h.s.content_loss_warning_active());
    assert!(!h.banner_visible());
}

// port: test/javascript/controllers/autosave_controller.test.js:907 "warning stays if content is still below threshold"
#[test]
fn check_content_restored_keeps_warning_while_still_a_large_deletion() {
    let mut h = H::new();
    h.set_file("test.md", &make_content(300), Some("r"));
    h.s.show_content_loss_warning();

    h.s.check_content_restored("");
    h.pump();

    assert!(h.s.content_loss_warning_active());
    assert!(h.banner_visible());
}

// ===========================================================================
// saveNow() — concurrent save guard
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:927 "prevents concurrent saves (_isSaving blocks second call)"
#[test]
fn prevents_concurrent_saves() {
    let mut h = H::new();
    h.set_file("test.md", "old", None);
    h.edit("new");

    let req = h.save_now().unwrap();
    assert!(h.s.is_saving());
    assert!(h.save_now().is_none());
    assert_eq!(h.saves.len(), 1);

    h.finish(&req, H::ok(None));
    assert!(!h.s.is_saving());
}

// port: test/javascript/controllers/autosave_controller.test.js:952 "clears _isSaving after fetch error"
#[test]
fn clears_is_saving_after_fetch_error() {
    let mut h = H::new();
    h.set_file("test.md", "old", None);
    h.edit("new");

    let req = h.save_now().unwrap();
    h.finish(&req, SaveOutcome::Failed);

    assert!(!h.s.is_saving());
    assert_eq!(h.s.status(), SaveStatus::ErrorSaving);
}

// port: test/javascript/controllers/autosave_controller.test.js:964 "reschedules save if content changed during fetch"
#[test]
fn reschedules_save_if_content_changed_during_fetch() {
    let mut h = H::new();
    h.set_file("test.md", "old", None);
    h.edit("version1");
    let req = h.save_now().unwrap();

    h.edit("version2");
    h.finish(&req, H::ok(None));

    assert!(h.s.has_unsaved_changes());
    assert!(h.s.save_timer_armed());
}

// port: test/javascript/controllers/autosave_controller.test.js:989 "does not reschedule if content unchanged after save"
#[test]
fn does_not_reschedule_if_content_unchanged_after_save() {
    let mut h = H::new();
    h.set_file("test.md", "old", None);
    h.edit("new");

    h.save_now_ok().unwrap();

    assert!(!h.s.has_unsaved_changes());
    assert!(!h.s.save_timer_armed());
    assert_eq!(h.s.status(), SaveStatus::Saved);
    h.advance(2000);
    assert_eq!(h.s.status(), SaveStatus::Hidden, "the Saved label clears after 2 s");
}

// ===========================================================================
// prepareForFileDeletion()
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:1003 "waits for an in-flight autosave and returns its acknowledged revision"
#[test]
fn deletion_waits_for_an_in_flight_autosave_and_returns_its_revision() {
    let mut h = H::new();
    h.set_file("created.md", "initial", Some("initial-revision"));
    h.edit("edited");
    let req = h.save_now().unwrap();

    assert_eq!(h.s.prepare_for_file_deletion("created.md"), DeletionPrep::AwaitSave);
    assert!(h.s.is_saving());

    h.finish(&req, H::ok(Some("autosaved-revision")));

    assert_eq!(h.s.resume_file_deletion("created.md"), DeletionPrep::Ready { revision: "autosaved-revision".into() });
}

// port: test/javascript/controllers/autosave_controller.test.js:1028 "cancels a follow-up debounce scheduled by an in-flight autosave"
#[test]
fn deletion_cancels_a_follow_up_debounce_scheduled_by_an_in_flight_autosave() {
    let mut h = H::new();
    h.set_file("created.md", "initial", Some("initial-revision"));
    h.edit("first edit");
    let req = h.save_now().unwrap();
    h.edit("latest editor content");
    assert_eq!(h.s.prepare_for_file_deletion("created.md"), DeletionPrep::AwaitSave);

    h.finish(&req, H::ok(Some("autosaved-revision")));

    assert_eq!(h.s.resume_file_deletion("created.md"), DeletionPrep::Ready { revision: "autosaved-revision".into() });
    assert!(!h.s.save_timer_armed());
    assert!(!h.s.max_interval_timer_armed());
    assert_eq!(h.s.known_base_revision("created.md"), Some("autosaved-revision"));
}

// port: test/javascript/controllers/autosave_controller.test.js:1056 "abandons preparation if the active note changes while waiting for a save"
#[test]
fn deletion_preparation_is_abandoned_if_the_active_note_changes() {
    let mut h = H::new();
    h.set_file("created.md", "initial", Some("initial-revision"));
    h.edit("edited");
    let req = h.save_now().unwrap();
    assert_eq!(h.s.prepare_for_file_deletion("created.md"), DeletionPrep::AwaitSave);
    h.set_file("other.md", "other content", Some("other-revision"));

    h.finish(&req, H::ok(Some("autosaved-revision")));

    assert_eq!(h.s.resume_file_deletion("created.md"), DeletionPrep::Stale);
}

// contract: autosave_controller.js#prepareForFileDeletion `currentFile !== path → {ok:false, stale:true}`
#[test]
fn deletion_preparation_for_a_non_active_note_is_stale() {
    let mut h = H::new();
    h.set_file("a.md", "x", Some("r"));
    assert_eq!(h.s.prepare_for_file_deletion("b.md"), DeletionPrep::Stale);
}

// ===========================================================================
// Offline legacy backup
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:1095 "scheduleOfflineBackup while offline triggers backup (debounced)"
#[test]
fn offline_backup_is_debounced_while_offline() {
    let mut h = H::new();
    h.s.connection_lost();
    h.set_file("test.md", "", None);
    h.edit("offline content");

    let now = h.now();
    h.s.schedule_offline_backup(now);
    h.pump();
    let backup = AutosaveEffect::OfflineBackup { path: "test.md".into(), content: "offline content".into() };
    assert!(!h.has_effect(&backup), "not immediate");

    h.advance(1100);

    assert!(h.has_effect(&backup));
}

// port: test/javascript/controllers/autosave_controller.test.js:1111 "scheduleOfflineBackup while online does NOT backup"
#[test]
fn offline_backup_not_scheduled_while_online() {
    let mut h = H::new();
    h.set_file("test.md", "", None);
    h.edit("online content");

    let now = h.now();
    h.s.schedule_offline_backup(now);

    assert!(!h.s.offline_backup_armed());
}

// port: test/javascript/controllers/autosave_controller.test.js:1122 "onConnectionLost immediately backs up unsaved content"
#[test]
fn connection_lost_immediately_backs_up_unsaved_content() {
    let mut h = H::new();
    h.set_file("test.md", "saved version", None);
    h.edit("unsaved version");

    h.s.connection_lost();
    h.pump();

    assert!(h.has_effect(&AutosaveEffect::OfflineBackup { path: "test.md".into(), content: "unsaved version".into() }));
    assert!(h.has_effect(&AutosaveEffect::OfflineChanged(true)));
    assert_eq!(h.s.status(), SaveStatus::Disconnected);
}

// port: test/javascript/controllers/autosave_controller.test.js:1132 "onConnectionLost does NOT backup if content unchanged"
#[test]
fn connection_lost_does_not_back_up_unchanged_content() {
    let mut h = H::new();
    h.set_file("test.md", "same content", None);
    h.edit("same content");

    h.s.connection_lost();
    h.pump();

    assert!(!h.effects.iter().any(|e| matches!(e, AutosaveEffect::OfflineBackup { .. })));
}

// port: test/javascript/controllers/autosave_controller.test.js:1142 "saveNow success clears backup"
#[test]
fn save_success_clears_the_offline_backup() {
    let mut h = H::new();
    h.set_file("test.md", "old", None);
    h.edit("new");

    h.save_now_ok().unwrap();

    assert!(h.has_effect(&AutosaveEffect::ClearOfflineBackup { path: "test.md".into() }));
}

// port: test/javascript/controllers/autosave_controller.test.js:1154 "checkOfflineBackup with differing backup shows recovery dialog"
#[test]
fn check_offline_backup_with_a_differing_backup_opens_recovery() {
    let mut h = H::new();
    h.s.drafts_mut()
        .backend_mut()
        .put_raw(&backup_key("test.md"), r#"{"content":"backup content","timestamp":1700000000000}"#);
    h.set_file("test.md", "server content", None);

    h.s.check_offline_backup("server content");
    h.pump();

    assert_eq!(
        h.recoveries(),
        vec![RecoveryRequest {
            path: "test.md".into(),
            server_content: "server content".into(),
            backup_content: "backup content".into(),
            backup_timestamp: 1_700_000_000_000.0,
            source: RecoverySource::Backup,
            draft_revision: None,
            conflict_id: None,
        }]
    );
}

// port: test/javascript/controllers/autosave_controller.test.js:1179 "checkOfflineBackup with matching backup does NOT show dialog"
#[test]
fn check_offline_backup_with_a_matching_backup_does_not_open_recovery() {
    let mut h = H::new();
    h.s.drafts_mut()
        .backend_mut()
        .put_raw(&backup_key("test.md"), r#"{"content":"server content","timestamp":1700000000000}"#);
    h.set_file("test.md", "server content", None);

    h.s.check_offline_backup("server content");
    h.pump();

    assert!(h.recoveries().is_empty());
    assert_eq!(h.s.drafts().backend().raw(&backup_key("test.md")), None, "matching backup auto-cleared");
}

// port: test/javascript/controllers/autosave_controller.test.js:1194 "onRecoveryResolved with 'backup' sets editor content and marks unsaved"
#[test]
fn recovery_resolved_with_backup_sets_editor_content_and_marks_unsaved() {
    let mut h = H::new();
    h.set_file("test.md", "server content", Some("server-revision"));
    let ts = 1_700_000_000_000u64;
    h.s.drafts_mut()
        .backend_mut()
        .put_raw(&backup_key("test.md"), &format!(r#"{{"content":"backup content","timestamp":{ts}}}"#));

    h.resolve(RecoveryResolution {
        content: Some("backup content".into()),
        backup_content: Some("backup content".into()),
        backup_timestamp: Some(ts as f64),
        ..resolution(RecoverySource::Backup)
    });

    assert!(h.editor_sets().contains(&"backup content".to_owned()));
    assert_eq!(h.s.last_saved_content(), Some("server content"));
    assert!(h.s.has_unsaved_changes());
    let d = h.draft("test.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("backup content", "server-revision"));
    assert_eq!(h.s.drafts().backend().raw(&backup_key("test.md")), None);
}

// port: test/javascript/controllers/autosave_controller.test.js:1216 "consumes an accepted legacy backup and does not offer it again"
#[test]
fn consumes_an_accepted_legacy_backup_and_does_not_offer_it_again() {
    let mut h = H::new();
    h.set_file("test.md", "server content", Some("server-revision"));
    h.s.drafts_mut()
        .backend_mut()
        .put_raw(&backup_key("test.md"), r#"{"content":"backup content","timestamp":1700000000000}"#);

    assert_eq!(h.s.recover_draft("server content", "server-revision"), "server content");
    h.pump();
    let opened = h.recoveries();
    assert_eq!(opened.len(), 1);

    h.resolve(RecoveryResolution::accept_local(&opened[0]));

    assert_eq!(h.s.drafts().backend().raw(&backup_key("test.md")), None);
    let d = h.draft("test.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("backup content", "server-revision"));

    assert_eq!(h.s.recover_draft("server content", "server-revision"), "backup content");
    h.pump();
    assert_eq!(h.recoveries().len(), 1, "not offered again");
}

// ===========================================================================
// Offline → in-flight save → online
// ===========================================================================

// port: test/javascript/controllers/autosave_controller.test.js:1243 "saves pending changes after reconnection even if in-flight save completed during offline"
#[test]
fn saves_pending_changes_after_reconnection_even_if_in_flight_save_completed_offline() {
    let mut h = H::new();
    h.set_file("test.md", "original", None);
    h.edit("version1");
    let req = h.save_now().unwrap();

    h.s.connection_lost();
    h.pump();
    assert!(h.s.is_offline());

    h.edit("version2");
    h.schedule();
    assert!(h.s.has_unsaved_changes());

    h.finish(&req, H::ok(None));

    let before = h.saves.len();
    let now = h.now();
    h.s.connection_restored(now);
    h.pump();
    assert!(!h.s.is_offline());
    assert_eq!(h.saves.len(), before + 1, "reconnection saves the pending change");
    assert_eq!(h.saves.last().unwrap().content, "version2");
}

// port: test/javascript/controllers/autosave_controller.test.js:1281 "onConnectionLost clears all pending save timers"
#[test]
fn connection_lost_clears_all_pending_save_timers() {
    let mut h = H::new();
    h.set_file("test.md", "old", None);
    h.edit("new");
    h.schedule();
    assert!(h.s.save_timer_armed());
    assert!(h.s.max_interval_timer_armed());

    h.s.connection_lost();

    assert!(!h.s.save_timer_armed());
    assert!(!h.s.max_interval_timer_armed());
    assert!(h.s.has_unsaved_changes());
}

// port: test/javascript/controllers/autosave_controller.test.js:1300 "scheduleAutoSave is a no-op while offline"
#[test]
fn schedule_autosave_is_a_no_op_while_offline() {
    let mut h = H::new();
    h.set_file("test.md", "saved", Some("revision-test"));
    h.edit("changed");
    h.s.connection_lost();

    h.schedule();

    assert!(!h.s.save_timer_armed());
    assert!(h.s.has_unsaved_changes());
}

// port: test/javascript/controllers/autosave_controller.test.js:1312 "saveNow is a no-op while offline"
#[test]
fn save_now_is_a_no_op_while_offline() {
    let mut h = H::new();
    h.s.connection_lost();
    h.set_file("test.md", "old", None);
    h.edit("new");

    assert!(h.save_now().is_none());
    assert!(h.s.has_unsaved_changes());
}

// port: test/javascript/controllers/autosave_controller.test.js:1324 "onConnectionRestored triggers saveNow when there are pending changes"
#[test]
fn connection_restored_saves_when_there_are_pending_changes() {
    let mut h = H::new();
    h.set_file("test.md", "old", None);
    h.edit("new");
    h.s.connection_lost();
    h.schedule(); // offline: only marks the note dirty
    assert!(h.s.has_unsaved_changes());

    let now = h.now();
    h.s.connection_restored(now);
    h.pump();

    assert!(!h.s.is_offline());
    assert_eq!(h.saves.len(), 1);
}

// port: test/javascript/controllers/autosave_controller.test.js:1339 "onConnectionRestored does NOT save when there are no pending changes"
#[test]
fn connection_restored_does_not_save_without_pending_changes() {
    let mut h = H::new();
    h.set_file("test.md", "same", None);
    h.edit("same");
    h.s.connection_lost();

    let now = h.now();
    h.s.connection_restored(now);
    h.pump();

    assert!(h.saves.is_empty());
}

// ===========================================================================
// File lifecycle changes (rename / delete)
// ===========================================================================

/// The suite's `beforeEach`: `setFile("foo.md", "saved content")` (no revision) + edit.
fn lifecycle_foo() -> H {
    let mut h = H::new();
    h.set_file("foo.md", "saved content", None);
    h.edit("edited content");
    h
}

// port: test/javascript/controllers/autosave_controller.test.js:1363 "renames a pending autosave without clearing dirty state"
#[test]
fn renames_a_pending_autosave_without_clearing_dirty_state() {
    let mut h = lifecycle_foo();
    h.set_file("foo.md", "saved content", Some("revision-foo"));
    h.schedule();

    assert!(h.rename("foo.md", "bar.md", ItemKind::File));
    assert_eq!(h.s.current_file(), Some("bar.md"));
    assert_eq!(h.s.last_saved_content(), Some("saved content"));
    assert!(h.s.has_unsaved_changes());

    h.advance(SAVE_DEBOUNCE_MS);

    assert_eq!(h.saves.len(), 1);
    assert_eq!(h.saves[0].path, "bar.md");
    assert_eq!(h.saves[0].content, "edited content");
    let req = h.saves[0].clone();
    h.finish(&req, H::ok(None));
    assert!(!h.s.has_unsaved_changes());
}

// port: test/javascript/controllers/autosave_controller.test.js:1381 "remaps a pending autosave inside a renamed folder"
#[test]
fn remaps_a_pending_autosave_inside_a_renamed_folder() {
    let mut h = lifecycle_foo();
    h.set_file("docs/foo.md", "saved content", Some("revision-docs"));
    h.schedule();

    assert!(h.rename("docs", "archive", ItemKind::Folder));
    assert_eq!(h.s.current_file(), Some("archive/foo.md"));
    assert!(h.s.has_unsaved_changes());

    h.advance(SAVE_DEBOUNCE_MS);

    assert_eq!(h.saves.len(), 1);
    assert_eq!(h.saves[0].path, "archive/foo.md");
}

// port: test/javascript/controllers/autosave_controller.test.js:1396 "moves the active local draft before scheduling a save at the renamed path"
#[test]
fn moves_the_active_local_draft_before_scheduling_a_save_at_the_renamed_path() {
    let mut h = lifecycle_foo();
    h.set_file("foo.md", "saved content", Some("revision-foo"));
    h.schedule();
    h.s.flush_draft_write("foo.md", "edited content", Some("revision-foo")).unwrap();
    let original = h.draft("foo.md").unwrap();

    assert!(h.rename("foo.md", "bar.md", ItemKind::File));

    assert_eq!(h.draft("foo.md"), None);
    assert_eq!(h.draft("bar.md"), Some(DraftRecord { path: "bar.md".into(), ..original }));
    assert_eq!(h.s.current_file(), Some("bar.md"));
    assert!(h.s.save_timer_armed());

    h.advance(SAVE_DEBOUNCE_MS);

    assert_eq!(h.saves.len(), 1);
    assert_eq!(h.saves[0].path, "bar.md");
}

/// Make `remapDrafts("foo.md" → "bar.md")` fail the way the suite's mock does
/// (`{ok:false, sourcePath:"foo.md", destinationPath:"bar.md", affectedPaths:["bar.md"]}`):
/// the destination draft write throws.
fn fail_remap_to_bar(h: &mut H) {
    h.fault(Fault::throw(Method::SetItem).on_key(&draft_key("bar.md")).message("draft remap failed"));
}

// port: test/javascript/controllers/autosave_controller.test.js:1415 "makes a remap failure recoverable and blocks writes until the user resolves it"
#[test]
fn makes_a_remap_failure_recoverable_and_blocks_writes_until_resolved() {
    let mut h = lifecycle_foo();
    fail_remap_to_bar(&mut h);
    h.set_file("foo.md", "saved content", Some("revision-foo"));
    h.edit("recoverable local draft");

    assert!(h.rename("foo.md", "bar.md", ItemKind::File));
    h.clear_faults();

    let conflict = h.s.drafts_mut().list_draft_conflicts("bar.md").unwrap()[0].clone();
    assert_eq!(h.s.current_file(), Some("bar.md"));
    assert_eq!(conflict.source_path, "foo.md");
    assert_eq!(conflict.content, "recoverable local draft");
    assert_eq!(conflict.base_revision, "revision-foo");
    assert_eq!(h.draft("foo.md").unwrap().content, "recoverable local draft");
    assert!(h.s.is_draft_blocked("bar.md"));
    assert!(h.s.has_pending_recovery("bar.md"));

    h.schedule();
    let blocked = h.s.write_draft_snapshot("bar.md", "later edit", "revision-foo");
    assert!(blocked.is_err());
    assert!(!h.s.save_timer_armed());
    assert!(h.saves.is_empty());
    assert!(
        h.s.drafts_mut()
            .list_draft_conflicts("bar.md")
            .unwrap()
            .iter()
            .any(|c| c.conflict_id == conflict.conflict_id && c.content == "recoverable local draft")
    );

    let req = h.recoveries().last().cloned().expect("recovery opened for the preserved copy");
    h.resolve(RecoveryResolution::accept_local(&req));

    assert_eq!(h.draft("bar.md").unwrap().content, "recoverable local draft");
    assert_eq!(h.draft("foo.md"), None);
    assert_eq!(h.s.drafts_mut().list_draft_conflicts("bar.md").unwrap(), vec![]);
    assert!(!h.s.is_draft_blocked("bar.md"));
    assert!(!h.s.has_pending_recovery("bar.md"));
    assert!(h.s.save_timer_armed());
}

// port: test/javascript/controllers/autosave_controller.test.js:1468 "retains the source draft and retries recovery when fallback storage also fails"
#[test]
fn retains_the_source_draft_and_retries_recovery_when_fallback_storage_also_fails() {
    let mut h = lifecycle_foo();
    fail_remap_to_bar(&mut h);
    h.fault(
        Fault::throw(Method::SetItem).on_prefix(&draft_conflict_path_prefix("bar.md")).message("recovery storage unavailable").once(),
    );
    h.set_file("foo.md", "saved content", Some("revision-foo"));
    h.edit("recoverable local draft");

    h.rename("foo.md", "bar.md", ItemKind::File);
    h.clear_faults();

    assert_eq!(h.draft("foo.md").unwrap().content, "recoverable local draft");
    assert_eq!(h.s.drafts_mut().list_draft_conflicts("bar.md").unwrap(), vec![]);
    assert_eq!(h.s.pending_remap_recovery("bar.md"), Some("foo.md"));
    assert!(h.s.is_draft_blocked("bar.md"));

    h.schedule();
    assert!(!h.s.save_timer_armed());
    assert!(h.saves.is_empty());

    assert_eq!(h.s.recover_draft("server content", "revision-foo"), "server content");
    h.pump();

    assert_eq!(h.s.pending_remap_recovery("bar.md"), None);
    assert!(
        h.s.drafts_mut()
            .list_draft_conflicts("bar.md")
            .unwrap()
            .iter()
            .any(|c| c.source_path == "foo.md" && c.content == "recoverable local draft")
    );
    assert!(h.s.has_pending_recovery("bar.md"));
}

// port: test/javascript/controllers/autosave_controller.test.js:1504 "does not let a late save response remove the source draft after remap failure"
#[test]
fn late_save_response_does_not_remove_the_source_draft_after_remap_failure() {
    let mut h = lifecycle_foo();
    fail_remap_to_bar(&mut h);
    h.fault(
        Fault::throw(Method::SetItem).on_prefix(&draft_conflict_path_prefix("bar.md")).message("recovery storage unavailable").once(),
    );
    h.set_file("foo.md", "saved content", Some("revision-foo"));
    h.edit("recoverable local draft");
    h.schedule();
    let req = h.save_now().unwrap();

    h.rename("foo.md", "bar.md", ItemKind::File);
    h.clear_faults();
    h.finish(&req, H::ok(Some("new-server-revision")));

    let d = h.draft("foo.md").unwrap();
    assert_eq!((d.content.as_str(), d.base_revision.as_str()), ("recoverable local draft", "revision-foo"));
    assert_eq!(h.s.drafts_mut().list_draft_conflicts("bar.md").unwrap(), vec![]);
    assert_eq!(h.s.pending_remap_recovery("bar.md"), Some("foo.md"));
    assert!(!h.s.save_timer_armed());

    h.s.recover_draft("saved content", "revision-foo");
    h.pump();

    assert!(
        h.s.drafts_mut()
            .list_draft_conflicts("bar.md")
            .unwrap()
            .iter()
            .any(|c| c.source_path == "foo.md" && c.content == "recoverable local draft")
    );
    assert!(h.s.has_pending_recovery("bar.md"));
}

// port: test/javascript/controllers/autosave_controller.test.js:1547 "does not overwrite a destination draft collision and blocks local writes there"
#[test]
fn does_not_overwrite_a_destination_draft_collision_and_blocks_writes_there() {
    let mut h = lifecycle_foo();
    h.set_file("foo.md", "saved content", Some("revision-source"));
    h.s.flush_draft_write("foo.md", "source edit", Some("revision-source")).unwrap();
    let source = h.draft("foo.md").unwrap();
    let destination = h.write_draft("bar.md", "other note draft", "revision-destination");
    h.edit("source edit");

    assert!(h.rename("foo.md", "bar.md", ItemKind::File));

    assert_eq!(h.s.current_file(), Some("bar.md"));
    assert_eq!(h.draft("foo.md"), None);
    assert!(
        h.s.drafts_mut()
            .list_draft_conflicts("bar.md")
            .unwrap()
            .iter()
            .any(|c| c.source_path == "foo.md" && c.content == source.content)
    );
    assert_eq!(h.draft("bar.md"), Some(destination.clone()));
    assert!(h.s.is_draft_blocked("bar.md"));
    assert!(!h.s.save_timer_armed());
    assert!(h.s.write_draft_snapshot("bar.md", "new edit", "revision-source").is_err());
    assert_eq!(h.draft("bar.md"), Some(destination));
}

// port: test/javascript/controllers/autosave_controller.test.js:1570 "does not remap a path that only shares a folder prefix"
#[test]
fn does_not_remap_a_path_that_only_shares_a_folder_prefix() {
    let mut h = lifecycle_foo();
    h.set_file("docs-old/foo.md", "saved content", None);

    assert!(!h.rename("docs", "archive", ItemKind::Folder));
    assert_eq!(h.s.current_file(), Some("docs-old/foo.md"));
}

// port: test/javascript/controllers/autosave_controller.test.js:1577 "clears a pending autosave when the note is deleted"
#[test]
fn clears_a_pending_autosave_when_the_note_is_deleted() {
    let mut h = lifecycle_foo();
    h.schedule();

    assert!(h.s.delete_file("foo.md", ItemKind::File).is_ok());
    h.pump();
    assert_eq!(h.s.current_file(), None);
    assert!(!h.s.save_timer_armed());
    assert!(!h.s.max_interval_timer_armed());
    assert!(!h.s.has_unsaved_changes());

    h.advance(SAVE_DEBOUNCE_MS);
    assert!(h.saves.is_empty());
}

// port: test/javascript/controllers/autosave_controller.test.js:1591 "removes file drafts and legacy backups after a successful deletion"
#[test]
fn removes_file_drafts_and_legacy_backups_after_a_successful_deletion() {
    let mut h = lifecycle_foo();
    h.set_file("foo.md", "saved content", Some("revision-foo"));
    h.write_draft("foo.md", "edited content", "revision-foo");
    h.s.drafts_mut().write_backup("foo.md", "offline content").unwrap();

    assert_eq!(h.s.delete_file("foo.md", ItemKind::File), Ok(2));
    assert_eq!(h.s.current_file(), None);
    assert_eq!(h.draft("foo.md"), None);
    assert_eq!(h.s.drafts_mut().read_backup("foo.md").unwrap().backup, None);
}

// port: test/javascript/controllers/autosave_controller.test.js:1602 "clears active state even when local deletion cleanup fails"
#[test]
fn clears_active_state_even_when_local_deletion_cleanup_fails() {
    let mut h = lifecycle_foo();
    h.set_file("foo.md", "saved content", Some("revision-foo"));
    h.write_draft("foo.md", "edited content", "revision-foo");
    h.fault(Fault::throw(Method::RemoveItem).message("storage unavailable"));

    let err = h.s.delete_file("foo.md", ItemKind::File).unwrap_err();
    h.pump();

    assert_eq!(err.error.message, "storage unavailable");
    assert_eq!(h.s.current_file(), None);
    assert!(!h.s.has_unsaved_changes());
    assert!(!h.s.save_timer_armed());
}

// port: test/javascript/controllers/autosave_controller.test.js:1614 "ignores a late response from a save started before deletion"
#[test]
fn ignores_a_late_response_from_a_save_started_before_deletion() {
    let mut h = lifecycle_foo();
    h.schedule();
    let req = h.save_now().unwrap();
    assert!(h.s.is_saving());

    h.s.delete_file("foo.md", ItemKind::File).unwrap();
    h.finish(&req, H::ok(None));

    assert_eq!(h.s.current_file(), None);
    assert!(!h.s.has_unsaved_changes());
    assert_eq!(h.s.last_saved_content(), None);
}

// port: test/javascript/controllers/autosave_controller.test.js:1633 "reschedules the renamed note when the old in-flight save fails"
#[test]
fn reschedules_the_renamed_note_when_the_old_in_flight_save_fails() {
    let mut h = lifecycle_foo();
    h.schedule();
    let req = h.save_now().unwrap();
    h.rename("foo.md", "bar.md", ItemKind::File);

    h.finish(&req, SaveOutcome::Failed);

    assert_eq!(h.s.current_file(), Some("bar.md"));
    assert!(h.s.has_unsaved_changes());
    assert!(h.s.save_timer_armed());
}

// port: test/javascript/controllers/autosave_controller.test.js:1650 "keeps the remapped newer draft when an old-path save response arrives"
#[test]
fn keeps_the_remapped_newer_draft_when_an_old_path_save_response_arrives() {
    let mut h = lifecycle_foo();
    h.set_file("foo.md", "saved content", Some("revision-foo"));
    h.schedule();
    let req = h.save_now().unwrap();
    h.edit("newer edit");
    h.schedule();
    h.rename("foo.md", "bar.md", ItemKind::File);
    let moved = h.draft("bar.md").unwrap();

    h.finish(&req, H::ok(Some("new-server-revision")));

    assert_eq!(h.s.current_file(), Some("bar.md"));
    assert_eq!(h.draft("foo.md"), None);
    assert_eq!(h.draft("bar.md"), Some(moved.clone()));
    assert_eq!(moved.content, "newer edit");
    assert!(h.s.has_unsaved_changes());
}

// ===========================================================================
// Debounce / max-wait with the fake clock (contract: frontend.md B.3;
// SAVE_DEBOUNCE_MS = 2000, SAVE_MAX_INTERVAL_MS = 30000)
// ===========================================================================

// contract: autosave_controller.js:13 SAVE_DEBOUNCE_MS = 2000
#[test]
fn debounce_saves_2000ms_after_the_last_keystroke() {
    let mut h = H::new();
    h.set_file("n.md", "a", Some("r"));
    h.edit("ab");
    h.schedule();
    h.advance(1999);
    assert!(h.saves.is_empty());
    h.edit("abc");
    h.schedule(); // restarts the debounce
    h.advance(1999);
    assert!(h.saves.is_empty());
    h.advance(1);
    assert_eq!(h.saves.len(), 1);
    assert_eq!(h.saves[0].content, "abc");
}

// contract: autosave_controller.js:14 SAVE_MAX_INTERVAL_MS = 30000
#[test]
fn max_interval_forces_a_save_after_30000ms_of_continuous_typing() {
    let mut h = H::new();
    h.set_file("n.md", "a", Some("r"));
    let mut text = String::from("a");
    // Type every second: the 2 s debounce never elapses.
    for _ in 0..29 {
        text.push('x');
        h.edit(&text);
        h.schedule();
        h.advance(1000);
    }
    assert!(h.saves.is_empty(), "debounce keeps restarting");
    text.push('x');
    h.edit(&text);
    h.schedule();
    h.advance(1000); // t = 30 000 ms since the first unsaved edit
    assert_eq!(h.saves.len(), 1, "max-wait forced a save");
    assert_eq!(h.saves[0].content.len(), 31);
}

// contract: autosave_controller.js:15 DRAFT_DEBOUNCE_MS = 300
#[test]
fn draft_write_debounce_is_300ms() {
    let mut h = H::new();
    h.set_file("n.md", "a", Some("r"));
    h.edit("ab");
    h.schedule_draft();
    h.advance(299);
    assert_eq!(h.draft("n.md"), None);
    h.advance(1);
    assert_eq!(h.draft("n.md").unwrap().content, "ab");
}

// ===========================================================================
// recovery_diff_controller.test.js — dispatched resolution payloads
// ===========================================================================

fn offered(source: RecoverySource, conflict: Option<&str>) -> RecoveryRequest {
    RecoveryRequest {
        path: "test.md".into(),
        server_content: "hello world".into(),
        backup_content: "hello universe".into(),
        backup_timestamp: 1_700_000_000_000.0,
        source,
        draft_revision: conflict.map(|_| "draft-1".to_owned()),
        conflict_id: conflict.map(str::to_owned),
    }
}

// port: test/javascript/controllers/recovery_diff_controller.test.js:117 "dispatches resolved event with source server and closes dialog"
#[test]
fn recovery_accept_server_payload() {
    let r = RecoveryResolution::accept_server(&offered(RecoverySource::Backup, None));
    assert_eq!(
        r,
        RecoveryResolution {
            source: RecoverySource::Server,
            path: Some("test.md".into()),
            content: None,
            draft_revision: None,
            backup_content: Some("hello universe".into()),
            backup_timestamp: Some(1_700_000_000_000.0),
            conflict_id: None,
            server_content: None,
        }
    );
}

// port: test/javascript/controllers/recovery_diff_controller.test.js:146 "dispatches resolved event with source backup and content, then closes"
#[test]
fn recovery_accept_backup_payload() {
    let r = RecoveryResolution::accept_local(&offered(RecoverySource::Backup, None));
    assert_eq!(
        r,
        RecoveryResolution {
            source: RecoverySource::Backup,
            path: Some("test.md".into()),
            content: Some("hello universe".into()),
            draft_revision: None,
            backup_content: Some("hello universe".into()),
            backup_timestamp: Some(1_700_000_000_000.0),
            conflict_id: None,
            server_content: None,
        }
    );
}

// port: test/javascript/controllers/recovery_diff_controller.test.js:173 "includes the selected conflict identity so recovery cannot consume another draft"
#[test]
fn recovery_payload_echoes_the_conflict_identity() {
    let req = offered(RecoverySource::DraftConflict, Some("conflict-1"));
    let local = RecoveryResolution::accept_local(&req);
    assert_eq!(local.source, RecoverySource::DraftConflict);
    assert_eq!(local.conflict_id.as_deref(), Some("conflict-1"));
    assert_eq!(local.draft_revision.as_deref(), Some("draft-1"));
    let server = RecoveryResolution::accept_server(&req);
    assert_eq!(server.conflict_id.as_deref(), Some("conflict-1"));
    assert_eq!(server.server_content.as_deref(), Some("hello world"));
}

// ===========================================================================
// AutosaveTimer (the workspace store's per-document debounce)
// ===========================================================================

use frankmd_workspace::autosave::{AutosavePolicy, AutosaveTimer};

// contract: autosave_controller.js:13-14 debounce 2000 ms / max-wait 30000 ms
#[test]
fn autosave_timer_debounce_and_max_wait() {
    let clock = FakeClock::new();
    let p = AutosavePolicy::default();
    assert_eq!(p.debounce, std::time::Duration::from_millis(2000));
    assert_eq!(p.max_wait, std::time::Duration::from_millis(30_000));
    let mut t = AutosaveTimer::default();
    assert!(!t.due(clock.now(), &p), "clean timer is never due");
    assert_eq!(t.next_deadline(&p), None);

    t.edited(clock.now());
    clock.advance_ms(1999);
    assert!(!t.due(clock.now(), &p));
    clock.advance_ms(1);
    assert!(t.due(clock.now(), &p));

    t.saved();
    assert!(!t.due(clock.now(), &p));
    let start = clock.now();
    t.edited(start);
    for _ in 0..29 {
        clock.advance_ms(1000);
        t.edited(clock.now());
        assert!(!t.due(clock.now(), &p));
    }
    assert_eq!(t.next_deadline(&p), Some(start + p.max_wait));
    clock.advance_ms(1000);
    assert!(t.due(clock.now(), &p), "max-wait reached while typing continuously");
}
