//! Omarchy theme / language / fontconfig watcher integration tests.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)] // tests create/modify files directly

mod common;

use common::{channel, wait_for};
use frankmd_watch::{WatchConfig, WatchEvent, Watchers, start_with};
use std::fs;
use std::path::Path;
use std::sync::mpsc::Receiver;

struct Env {
    vault: tempfile::TempDir,
    home: tempfile::TempDir,
}

fn env() -> Env {
    Env {
        vault: tempfile::tempdir().unwrap(),
        home: tempfile::tempdir().unwrap(),
    }
}

fn start(
    env: &Env,
    edit: impl FnOnce(&mut WatchConfig, &Path),
) -> (Watchers, Receiver<WatchEvent>) {
    let mut config = WatchConfig::new(env.vault.path().to_path_buf());
    edit(&mut config, env.home.path());
    let (sink, rx) = channel();
    (start_with(config, sink).unwrap(), rx)
}

fn theme_dir(root: &Path, name: &str) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("colors.toml"), format!("name = \"{name}\"\n")).unwrap();
}

#[test]
fn replacing_theme_dir_emits_theme_changed() {
    let env = env();
    let current = env.home.path().join("current");
    theme_dir(&current.join("theme"), "old");
    fs::write(current.join("theme.name"), "old").unwrap();
    let staging = env.home.path().join("staging");
    theme_dir(&staging, "new");
    let (_w, rx) = start(&env, |c, _| c.omarchy_current_dir = Some(current.clone()));

    fs::remove_dir_all(current.join("theme")).unwrap();
    fs::rename(&staging, current.join("theme")).unwrap();
    assert!(
        wait_for(&rx, |e| *e == WatchEvent::OmarchyThemeChanged)
            .0
            .is_some()
    );

    // The replaced dir is watched too: an in-place edit is seen again.
    fs::write(current.join("theme/colors.toml"), "name = \"edited\"\n").unwrap();
    assert!(
        wait_for(&rx, |e| *e == WatchEvent::OmarchyThemeChanged)
            .0
            .is_some()
    );
}

#[test]
fn theme_name_change_emits_but_background_does_not() {
    let env = env();
    let current = env.home.path().join("current");
    theme_dir(&current.join("theme"), "a");
    let (_w, rx) = start(&env, |c, _| c.omarchy_current_dir = Some(current.clone()));
    fs::write(current.join("background"), "img").unwrap();
    fs::write(current.join("theme.name"), "b").unwrap();
    let (ev, seen) = wait_for(&rx, |e| *e == WatchEvent::OmarchyThemeChanged);
    assert!(ev.is_some());
    assert!(seen.is_empty(), "{seen:?}");
}

#[test]
fn missing_omarchy_dir_rearms_when_created() {
    let env = env();
    let current = env.home.path().join(".local/state/omarchy/current");
    let (_w, rx) = start(&env, |c, _| c.omarchy_current_dir = Some(current.clone()));

    theme_dir(&current.join("theme"), "first");
    assert!(
        wait_for(&rx, |e| *e == WatchEvent::OmarchyThemeChanged)
            .0
            .is_some()
    );

    // Now armed on `current/` itself.
    fs::write(current.join("theme.name"), "second").unwrap();
    assert!(
        wait_for(&rx, |e| *e == WatchEvent::OmarchyThemeChanged)
            .0
            .is_some()
    );
}

#[test]
fn language_file_change_emits_system_language_changed() {
    let env = env();
    let file = env.home.path().join(".config/omarchy/language");
    let (_w, rx) = start(&env, |c, _| c.omarchy_language_file = Some(file.clone()));
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "pt-BR\n").unwrap();
    assert!(
        wait_for(&rx, |e| *e == WatchEvent::SystemLanguageChanged)
            .0
            .is_some()
    );
    fs::write(file.with_file_name("other"), "x").unwrap();
    fs::write(&file, "en\n").unwrap();
    assert!(
        wait_for(&rx, |e| *e == WatchEvent::SystemLanguageChanged)
            .0
            .is_some()
    );
}

#[test]
fn fontconfig_change_emits_fonts_changed() {
    let env = env();
    let dir = env.home.path().join("fontconfig");
    fs::create_dir_all(dir.join("conf.d")).unwrap();
    let (_w, rx) = start(&env, |c, _| c.fontconfig_dir = Some(dir.clone()));
    fs::write(dir.join("conf.d/10-mono.conf"), "<fontconfig/>").unwrap();
    assert!(
        wait_for(&rx, |e| *e == WatchEvent::FontsChanged)
            .0
            .is_some()
    );
}

#[test]
fn dropping_watchers_stops_system_events() {
    let env = env();
    let current = env.home.path().join("current");
    theme_dir(&current.join("theme"), "a");
    let (w, rx) = start(&env, |c, _| c.omarchy_current_dir = Some(current.clone()));
    drop(w);
    fs::write(current.join("theme.name"), "b").unwrap();
    let rest = common::drain_until_disconnected(&rx).expect("sink was not dropped");
    assert!(rest.is_empty(), "{rest:?}");
}
