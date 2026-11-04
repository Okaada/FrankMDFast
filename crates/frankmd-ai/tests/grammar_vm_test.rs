//! Grammar check state. Adapted rows of docs/testing/ui-test-plan.md §8
//! (ai_grammar_controller.test.js) plus the per-change accept/reject of
//! ui-spec §5.10 and the `.fed` refusal (security decision).
#![allow(clippy::unwrap_used)]

use frankmd_ai::ProviderInfo;
use frankmd_ai::grammar::*;
use frankmd_config::AiProviderId;

fn configured() -> GrammarState {
    GrammarState::from_config(&ProviderInfo {
        enabled: true,
        provider: Some(AiProviderId::Anthropic),
        model: Some("claude-sonnet-4".into()),
        available_providers: vec![AiProviderId::Anthropic],
    })
}

fn not_configured() -> GrammarState {
    GrammarState::from_config(&ProviderInfo { enabled: false, provider: None, model: None, available_providers: vec![] })
}

fn result(original: &str, corrected: &str) -> GrammarResult {
    GrammarResult { original: original.into(), corrected: corrected.into(), provider: "anthropic".into(), model: "m".into() }
}

fn started(state: &mut GrammarState) -> u64 {
    let effects = state.start(Some("notes/a.md"), "Teh text").unwrap();
    let Some(GrammarEffect::Request { id, path }) = effects.last().cloned() else { panic!("{effects:?}") };
    assert_eq!(path, "notes/a.md");
    id
}

// rows 1-2
#[test]
fn availability_from_config() {
    assert_eq!(GrammarState::default().availability(), &Availability::Unknown);
    assert!(!GrammarState::default().in_flight());
    assert_eq!(
        configured().availability(),
        &Availability::Available { provider: "anthropic".into(), model: "claude-sonnet-4".into() }
    );
    assert_eq!(not_configured().availability(), &Availability::NotConfigured);
}

// rows 3-4, FE-441 + security (.fed never sent)
#[test]
fn start_refusals() {
    let mut s = not_configured();
    assert_eq!(s.start(Some("a.md"), "text"), Err(StartError::NotConfigured));
    assert!(!s.in_flight(), "no request, no read-only lock");
    let mut s = configured();
    assert_eq!(s.start(None, "text"), Err(StartError::NoFileOpen));
    assert_eq!(StartError::NoFileOpen.key(), Some("errors.no_file_open"));
    assert_eq!(s.start(Some("a.md"), "  \n "), Err(StartError::NoText));
    assert_eq!(s.start(Some(".fed"), "secret = 1"), Err(StartError::NotANote));
    assert_eq!(s.start(Some("dir/.fed"), "x"), Err(StartError::NotANote));
    assert_eq!(s.start(Some("../x.md"), "x"), Err(StartError::NotANote));
    assert!(!s.in_flight());
}

// FE-442/444/445: flush, lock, banner, request; rows 10-12
#[test]
fn start_locks_and_finish_unlocks() {
    let mut s = configured();
    let effects = s.start(Some("notes/a.md"), "Teh text").unwrap();
    assert_eq!(
        effects[..3],
        [
            GrammarEffect::FlushAutosave,
            GrammarEffect::SetReadOnly(true),
            GrammarEffect::ShowBanner { provider: "anthropic".into(), model: "claude-sonnet-4".into() },
        ]
    );
    let GrammarEffect::Request { id, .. } = effects[3].clone() else { panic!() };
    assert_eq!(s.start(Some("notes/a.md"), "x"), Err(StartError::Busy));
    assert!(s.finish(id + 99, Ok(result("a", "b"))).is_empty(), "stale id ignored");
    let effects = s.finish(id, Ok(result("Teh text", "The text")));
    assert_eq!(effects, vec![GrammarEffect::SetReadOnly(false), GrammarEffect::HideBanner, GrammarEffect::ShowDialog]);
    assert!(!s.in_flight());
    assert!(s.cancel().is_empty(), "cancel after completion is a no-op");
    assert!(s.start(Some("notes/a.md"), "x").is_ok(), "a new check can start");
}

// FE-446: Esc aborts silently
#[test]
fn cancel_aborts_and_unlocks() {
    let mut s = configured();
    let id = started(&mut s);
    assert_eq!(s.cancel(), vec![GrammarEffect::Abort, GrammarEffect::SetReadOnly(false), GrammarEffect::HideBanner]);
    assert!(s.finish(id, Ok(result("a", "b"))).is_empty(), "late answer dropped");
    assert!(s.result().is_none());
}

// §5.10 errors inline with Retry (FE-447 changed)
#[test]
fn errors_show_in_the_dialog_with_retry() {
    let mut s = configured();
    let id = started(&mut s);
    let effects = s.finish(id, Err("Failed to process text with AI: AI provider request failed".into()));
    assert!(effects.contains(&GrammarEffect::ShowDialog));
    assert_eq!(s.error(), Some("Failed to process text with AI: AI provider request failed"));
    let retry = s.retry("Teh text").unwrap();
    assert!(matches!(retry.last(), Some(GrammarEffect::Request { path, .. }) if path == "notes/a.md"));
}

// FE-448: word diff hunks; rows 6-8 (edit toggle, apply)
#[test]
fn hunks_accept_reject_and_apply() {
    let mut s = configured();
    let id = started(&mut s);
    s.finish(id, Ok(result("Teh cat sat on teh mat.", "The cat sat on the mat.")));
    assert_eq!(s.hunks().len(), 2);
    assert_eq!((s.hunks()[0].deleted.as_str(), s.hunks()[0].inserted.as_str()), ("Teh", "The"));
    assert_eq!(s.counter(), (1, 2));
    s.focus_step(true);
    assert_eq!(s.counter(), (2, 2));
    s.focus_step(true);
    assert_eq!(s.counter(), (1, 2), "wraps");
    assert_eq!(s.merged_text(), "The cat sat on the mat.");
    s.set_accepted(1, false);
    assert_eq!(s.merged_text(), "The cat sat on teh mat.");
    assert!(s.has_edits());
    assert_eq!(s.apply().as_deref(), Some("The cat sat on teh mat."));
    assert!(s.result().is_none(), "dialog closed after apply");
}

#[test]
fn default_apply_is_the_corrected_text() {
    let mut s = configured();
    let id = started(&mut s);
    s.finish(id, Ok(result("a  b\nc", "A  b\nc d")));
    assert_eq!(s.apply().as_deref(), Some("A  b\nc d"));
}

#[test]
fn edit_toggle_keeps_edits_and_labels() {
    let mut s = configured();
    let id = started(&mut s);
    s.finish(id, Ok(result("Teh", "The")));
    assert_eq!(s.toggle_label_key(), "common.edit");
    s.toggle_edit();
    assert_eq!(s.view(), View::Edit);
    assert_eq!(s.edited(), Some("The"));
    assert_eq!(s.toggle_label_key(), "preview.title");
    s.set_edited("Thee");
    s.toggle_edit();
    assert_eq!(s.view(), View::Diff);
    assert_eq!(s.edited(), Some("Thee"), "back to diff keeps the edits");
    s.set_accepted(0, false);
    assert!(s.hunks()[0].accepted, "hunk toggles are off once edited");
    assert_eq!(s.apply().as_deref(), Some("Thee"));
}

// row 5: cancel discards
#[test]
fn close_discards() {
    let mut s = configured();
    let id = started(&mut s);
    s.finish(id, Ok(result("Teh", "The")));
    s.close();
    assert!(s.result().is_none());
    assert_eq!(s.apply(), None);
}
