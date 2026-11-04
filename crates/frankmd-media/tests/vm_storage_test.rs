//! "Storage for new media" and the dialog storage row. Adapted rows of
//! docs/testing/ui-test-plan.md §12 (media_storage_controller.test.js,
//! s3_option_controller.test.js).
#![allow(clippy::unwrap_used)]

use frankmd_media::vm::Text;
use frankmd_media::vm::storage::*;

// adapts: media_storage_controller.test.js row 1
#[test]
fn provider_switch_waits_for_the_ack_and_releases_uploads() {
    let mut s = MediaStorageState::new(Provider::Local, true);
    assert!(s.request(Provider::S3));
    assert!(s.controls_disabled());
    assert_eq!(s.provider(), Provider::Local, "shows the acknowledged value while pending");
    assert_eq!(s.ready(), Ready::Wait);
    let out = s.ack(Ok((Provider::S3, true)));
    assert_eq!(out, AckOutcome { saved: true, released: 1 });
    assert_eq!(s.provider(), Provider::S3);
    assert!(!s.controls_disabled());
    assert_eq!(s.ready(), Ready::Go);
}

// adapts: media_storage_controller.test.js rows 2 and 8
#[test]
fn failed_save_keeps_the_previous_provider_and_fails_waiters() {
    let mut s = MediaStorageState::new(Provider::Local, true);
    s.request(Provider::S3);
    assert_eq!(s.ready(), Ready::Wait);
    let out = s.ack(Err(None));
    assert_eq!(out, AckOutcome { saved: false, released: 1 });
    assert_eq!(s.provider(), Provider::Local);
    assert_eq!(s.error(), Some(&Text::key("media_storage.save_failed")));
    s.request(Provider::S3);
    s.ack(Err(Some("offline".into())));
    assert_eq!(s.error(), Some(&Text::Raw("offline".into())));
}

// adapts: media_storage_controller.test.js row 3 (S3 without credentials can't be re-picked)
#[test]
fn s3_cannot_be_picked_when_not_configured() {
    let mut s = MediaStorageState::new(Provider::Local, false);
    assert!(!s.request(Provider::S3));
    assert_eq!(s.s3_disabled_reason(), Some(Text::key("media_storage.not_configured")));
    assert!(!s.request(Provider::Local), "same value: nothing to save");
}

// adapts: media_storage_controller.test.js rows 4-7 (fallback announced once, only real fallbacks)
#[test]
fn fallback_is_announced_exactly_once() {
    let mut s = MediaStorageState::new(Provider::S3, true);
    assert!(s.apply(Provider::Local, false, true));
    assert!(!s.apply(Provider::Local, false, true));
    assert!(!s.apply(Provider::Local, false, false));
    assert_eq!(s.provider(), Provider::Local);
    let mut user = MediaStorageState::new(Provider::S3, true);
    user.request(Provider::Local);
    user.ack(Ok((Provider::Local, true)));
    assert!(!user.apply(Provider::Local, true, false), "a user switch is not a fallback");
}

// adapts: s3_option_controller.test.js rows 1, 5-10
#[test]
fn resize_only_for_s3() {
    let mut row = StorageRow::new(Provider::Local, true);
    assert!(!row.resize_enabled());
    assert!(!row.uses_s3());
    assert_eq!(row.resize_ratio(), None);
    row.set_provider(Provider::S3);
    assert!(row.resize_enabled());
    assert!(row.uses_s3());
    row.set_resize(0.75);
    assert_eq!(row.resize_ratio(), Some(0.75));
    row.set_provider(Provider::Local);
    assert!(!row.resize_enabled());
    assert_eq!(row.resize(), 0.5, "leaving S3 resets the resize");
}

// adapts: s3_option_controller.test.js rows 2-4
#[test]
fn s3_availability_drives_the_row() {
    let mut row = StorageRow::new(Provider::S3, true);
    assert!(row.s3_enabled());
    row.set_resize(0.75);
    row.set_s3_available(false);
    assert_eq!(row.provider(), Provider::Local);
    assert_eq!(row.resize(), 0.5);
    assert!(!row.s3_enabled());
    assert_eq!(StorageRow::new(Provider::S3, false).provider(), Provider::Local);
}

// adapts: s3_option_controller.test.js rows 11-15
#[test]
fn destination_key_defaults_trim_and_stick() {
    let mut row = StorageRow::new(Provider::S3, true);
    assert_eq!(row.resize_options(), [0.25, 0.5, 0.75, 1.0]);
    row.set_default_key("frankmd/2026/07/photo.png");
    assert!(row.key_visible());
    assert_eq!(row.key_text(), "frankmd/2026/07/photo.png");
    row.on_key_edited("  blog/hero.png  ");
    assert_eq!(row.key_value().as_deref(), Some("blog/hero.png"));
    row.set_default_key("frankmd/2026/07/b.png");
    assert_eq!(row.key_text(), "  blog/hero.png  ", "user edits survive a new selection");
    row.set_provider(Provider::Local);
    assert_eq!(row.key_value(), None, "key only for S3");
    row.reset(Provider::Local);
    assert_eq!(row.key_text(), "");
    assert!(!row.key_visible());
}

// adapts: s3_option_controller.test.js rows 16-18, 20-21
#[test]
fn reset_and_default_row() {
    let mut row = StorageRow::new(Provider::S3, true);
    row.set_resize(1.0);
    row.reset(Provider::Local);
    assert_eq!(row.provider(), Provider::Local, "reset goes to the global default");
    assert!(!row.resize_enabled());
    assert_eq!(row.resize(), 0.5);
    row.reset(Provider::S3);
    assert_eq!(row.provider(), Provider::S3);
    let default = StorageRow::default();
    assert!(!default.uses_s3());
    assert_eq!(default.resize_ratio(), None);
}

#[test]
fn reupload_sources_hotlink_unless_save_copy() {
    let mut row = StorageRow::new(Provider::S3, true);
    row.set_mode(RowMode::Reupload, KeyMode::Prefix);
    assert!(row.save_copy_visible());
    assert!(!row.stores());
    assert!(!row.key_visible());
    row.set_save_copy(true);
    assert!(row.stores() && row.uses_s3() && row.resize_enabled());
    row.set_mode(RowMode::S3Only, KeyMode::Prefix);
    assert!(row.uses_s3());
    assert!(!row.resize_enabled(), "AI images: destination folder only");
    assert_eq!(Provider::from_setting(Some("s3")), Provider::S3);
    assert_eq!(Provider::from_setting(None), Provider::Local);
    assert_eq!(Provider::S3.as_setting(), "s3");
}
