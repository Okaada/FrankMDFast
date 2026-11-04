//! Vault watcher integration tests (real inotify on temp dirs).
#![allow(clippy::disallowed_methods, clippy::unwrap_used)] // tests create/modify files directly

mod common;

use common::{changed, channel, drain_until_disconnected, removed, wait_for};
use frankmd_core::Revision;
use frankmd_watch::{WatchConfig, WatchEvent, start_with};
use std::fs;
use std::path::PathBuf;

fn vault_only(dir: &tempfile::TempDir) -> WatchConfig {
    WatchConfig::new(dir.path().to_path_buf())
}

#[test]
fn external_create_reports_rel_path() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = channel();
    let _w = start_with(vault_only(&dir), sink).unwrap();
    fs::write(dir.path().join("hello.md"), "hi").unwrap();
    let (ev, _) = wait_for(&rx, |e| changed(e, "hello.md"));
    assert!(!removed(&ev.unwrap(), "hello.md"));
}

#[test]
fn external_modify_in_nested_folder() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("blog/2026")).unwrap();
    fs::write(dir.path().join("blog/2026/post.md"), "v1").unwrap();
    let (sink, rx) = channel();
    let _w = start_with(vault_only(&dir), sink).unwrap();
    fs::write(dir.path().join("blog/2026/post.md"), "v2").unwrap();
    assert!(
        wait_for(&rx, |e| changed(e, "blog/2026/post.md"))
            .0
            .is_some()
    );
}

#[test]
fn new_folder_contents_are_watched() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = channel();
    let _w = start_with(vault_only(&dir), sink).unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    assert!(wait_for(&rx, |e| changed(e, "sub")).0.is_some());
    fs::write(dir.path().join("sub/n.md"), "x").unwrap();
    assert!(wait_for(&rx, |e| changed(e, "sub/n.md")).0.is_some());
}

#[test]
fn delete_is_reported_as_removal() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("gone.md"), "x").unwrap();
    let (sink, rx) = channel();
    let _w = start_with(vault_only(&dir), sink).unwrap();
    fs::remove_file(dir.path().join("gone.md")).unwrap();
    assert!(wait_for(&rx, |e| removed(e, "gone.md")).0.is_some());
}

#[test]
fn rename_reports_both_sides() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.md"), "x").unwrap();
    let (sink, rx) = channel();
    let _w = start_with(vault_only(&dir), sink).unwrap();
    fs::rename(dir.path().join("a.md"), dir.path().join("b.md")).unwrap();
    let (ev, seen) = wait_for(&rx, |e| changed(e, "b.md"));
    let ev = ev.unwrap();
    let all: Vec<_> = seen.iter().chain(std::iter::once(&ev)).collect();
    assert!(all.iter().any(|e| removed(e, "a.md")), "{all:?}");
    assert!(!removed(&ev, "b.md"));
}

#[test]
fn dotfiles_temp_and_lock_files_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = channel();
    let _w = start_with(vault_only(&dir), sink).unwrap();
    fs::write(dir.path().join(".hidden.md"), "x").unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".git/config"), "x").unwrap();
    fs::write(dir.path().join(".note.md.0123456789ab.tmp"), "x").unwrap();
    fs::write(dir.path().join(".frankmd-filesystem.lock"), "").unwrap();
    fs::write(dir.path().join(".frankmd-config.lock"), "").unwrap();
    fs::write(dir.path().join("marker.md"), "x").unwrap();
    let (ev, seen) = wait_for(&rx, |e| changed(e, "marker.md"));
    let ev = ev.unwrap();
    for e in seen.iter().chain(std::iter::once(&ev)) {
        if let WatchEvent::VaultChanged { paths, .. } = e {
            assert!(
                paths.iter().all(|p| !p.as_str().starts_with('.')),
                "{paths:?}"
            );
        }
        assert_ne!(e, &WatchEvent::ConfigChanged);
    }
}

#[test]
fn fed_yields_config_changed_not_vault_changed() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = channel();
    let _w = start_with(vault_only(&dir), sink).unwrap();
    fs::write(dir.path().join(".fed"), "theme = dark\n").unwrap();
    let (ev, seen) = wait_for(&rx, |e| *e == WatchEvent::ConfigChanged);
    assert!(ev.is_some());
    assert!(seen.iter().all(|e| !changed(e, ".fed")));
}

#[test]
fn self_write_is_suppressed() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = channel();
    let w = start_with(vault_only(&dir), sink).unwrap();
    let shared = w.self_writes();
    shared
        .lock()
        .unwrap()
        .expect(PathBuf::from("mine.md"), Revision::of(b"saved"));
    shared
        .lock()
        .unwrap()
        .expect(PathBuf::from(".fed"), Revision::of(b"a = 1\n"));
    fs::write(dir.path().join("mine.md"), "saved").unwrap();
    fs::write(dir.path().join(".fed"), "a = 1\n").unwrap();
    fs::write(dir.path().join("theirs.md"), "x").unwrap();
    let (ev, mut seen) = wait_for(&rx, |e| changed(e, "theirs.md"));
    seen.extend(ev);
    // Echo batches may trail the marker's; wait (bounded) for both tokens to
    // be consumed, then flush the pipeline with a second marker.
    let deadline = std::time::Instant::now() + common::TIMEOUT;
    while !shared.lock().unwrap().is_empty() && std::time::Instant::now() < deadline {
        std::thread::park_timeout(std::time::Duration::from_millis(20));
    }
    assert!(shared.lock().unwrap().is_empty(), "echo never observed");
    fs::write(dir.path().join("marker.md"), "x").unwrap();
    let (ev, more) = wait_for(&rx, |e| changed(e, "marker.md"));
    seen.extend(more.into_iter().chain(ev));
    for e in &seen {
        assert!(!changed(e, "mine.md"), "{seen:?}");
        assert_ne!(e, &WatchEvent::ConfigChanged, "{seen:?}");
        assert_ne!(e, &WatchEvent::ConfigChanged);
    }
}

#[test]
fn self_write_with_other_content_is_external() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = channel();
    let w = start_with(vault_only(&dir), sink).unwrap();
    w.self_writes()
        .lock()
        .unwrap()
        .expect(PathBuf::from("n.md"), Revision::of(b"ours"));
    fs::write(dir.path().join("n.md"), "theirs").unwrap();
    assert!(wait_for(&rx, |e| changed(e, "n.md")).0.is_some());
}

#[test]
fn own_removal_is_suppressed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("old.md"), "x").unwrap();
    let (sink, rx) = channel();
    let w = start_with(vault_only(&dir), sink).unwrap();
    w.self_writes()
        .lock()
        .unwrap()
        .expect_removal(PathBuf::from("old.md"));
    fs::remove_file(dir.path().join("old.md")).unwrap();
    fs::write(dir.path().join("marker.md"), "x").unwrap();
    let (ev, seen) = wait_for(&rx, |e| changed(e, "marker.md"));
    for e in seen.iter().chain(ev.iter()) {
        assert!(!changed(e, "old.md"), "{e:?}");
    }
}

#[test]
fn missing_vault_root_fails_to_start() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, _rx) = channel();
    assert!(start_with(WatchConfig::new(dir.path().join("nope")), sink).is_err());
}

#[test]
fn dropping_watchers_stops_events() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = channel();
    let w = start_with(vault_only(&dir), sink).unwrap();
    fs::write(dir.path().join("before.md"), "x").unwrap();
    assert!(wait_for(&rx, |e| changed(e, "before.md")).0.is_some());
    drop(w);
    fs::write(dir.path().join("after.md"), "x").unwrap();
    // The worker threads exit and drop the sink, disconnecting the channel.
    let rest = drain_until_disconnected(&rx).expect("sink was not dropped");
    assert!(rest.iter().all(|e| !changed(e, "after.md")), "{rest:?}");
}

#[test]
fn sink_returning_false_stops_watching() {
    struct Once(std::sync::mpsc::Sender<WatchEvent>);
    impl frankmd_watch::EventSink for Once {
        fn send(&self, ev: WatchEvent) -> bool {
            let _ = self.0.send(ev);
            false
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let _w = start_with(vault_only(&dir), Once(tx)).unwrap();
    fs::write(dir.path().join("one.md"), "x").unwrap();
    assert!(wait_for(&rx, |e| changed(e, "one.md")).0.is_some());
    fs::write(dir.path().join("two.md"), "x").unwrap();
    let rest = drain_until_disconnected(&rx).expect("sink was not dropped");
    assert!(rest.is_empty(), "{rest:?}");
}
