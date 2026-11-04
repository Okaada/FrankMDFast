//! Test doubles for draft/autosave tests: an insertion-ordered in-memory
//! `DraftBackend` with fault injection (the original suite's `MemoryStorage` /
//! the golden generator's `FaultyStorage`), a pinned `DraftEnv`, and a fake clock.
//! Include with `#[path = "draft_common/mod.rs"] mod draft_common;`.
#![allow(dead_code, clippy::unwrap_used, unreachable_pub)]

use frankmd_workspace::autosave::{AutosaveConfig, AutosaveSession};
use frankmd_workspace::clock::{Clock, FakeClock};
use frankmd_workspace::draft_storage::{BackendError, DraftBackend, DraftEnv, DraftStorage};
use std::time::Instant;

/// Which storage call a fault applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Length,
    Key,
    GetItem,
    SetItem,
    RemoveItem,
}

/// Throw an error, or silently do nothing (`getItem` noop returns null).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultMode {
    Throw,
    Noop,
}

/// One installed fault (first matching fault wins, like the generator).
#[derive(Debug, Clone)]
pub struct Fault {
    pub method: Method,
    pub key: Option<String>,
    pub prefix: Option<String>,
    pub mode: FaultMode,
    pub message: Option<String>,
    /// Remaining hits before the fault disarms itself (`None` = forever).
    pub times: Option<usize>,
}

impl Fault {
    pub fn throw(method: Method) -> Self {
        Self { method, key: None, prefix: None, mode: FaultMode::Throw, message: None, times: None }
    }
    pub fn on_key(mut self, key: &str) -> Self {
        self.key = Some(key.to_owned());
        self
    }
    pub fn on_prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_owned());
        self
    }
    pub fn noop(mut self) -> Self {
        self.mode = FaultMode::Noop;
        self
    }
    pub fn message(mut self, m: &str) -> Self {
        self.message = Some(m.to_owned());
        self
    }
    pub fn once(mut self) -> Self {
        self.times = Some(1);
        self
    }
    fn default_message(&self) -> &'static str {
        match self.method {
            Method::Length => "length failed",
            Method::Key => "key failed",
            Method::GetItem => "getItem failed",
            Method::SetItem => "quota exceeded",
            Method::RemoveItem => "storage unavailable",
        }
    }
}

/// Insertion-ordered key/value store (JS `Map` semantics: `set` on an existing
/// key keeps its position).
#[derive(Debug, Default)]
pub struct MemoryBackend {
    pub entries: Vec<(String, String)>,
    pub faults: std::cell::RefCell<Vec<Fault>>,
    pub provider_fails: bool,
    /// Count of successful `set_item` calls (the `vi.spyOn(setItem)` counter).
    pub set_calls: usize,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn seeded(entries: &[(&str, &str)]) -> Self {
        Self { entries: entries.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect(), ..Self::default() }
    }
    pub fn raw(&self, key: &str) -> Option<&str> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
    pub fn put_raw(&mut self, key: &str, value: &str) {
        if let Some(e) = self.entries.iter_mut().find(|(k, _)| k == key) {
            e.1 = value.to_owned();
        } else {
            self.entries.push((key.to_owned(), value.to_owned()));
        }
    }
    pub fn add_fault(&mut self, f: Fault) {
        self.faults.get_mut().push(f);
    }
    pub fn clear_faults(&mut self) {
        self.faults.get_mut().clear();
    }
    fn fault(&self, method: Method, key: Option<&str>) -> Option<Fault> {
        let mut faults = self.faults.borrow_mut();
        let idx = faults.iter().position(|f| {
            f.method == method
                && f.key.as_deref().is_none_or(|k| Some(k) == key)
                && f.prefix.as_deref().is_none_or(|p| key.is_some_and(|k| k.starts_with(p)))
        })?;
        let hit = faults[idx].clone();
        if let Some(t) = faults[idx].times.as_mut() {
            *t -= 1;
            if *t == 0 {
                faults.remove(idx);
            }
        }
        Some(hit)
    }
    fn err(f: &Fault) -> BackendError {
        BackendError::new(f.message.clone().unwrap_or_else(|| f.default_message().to_owned()))
    }
}

impl DraftBackend for MemoryBackend {
    fn available(&self) -> Result<(), BackendError> {
        if self.provider_fails { Err(BackendError::new("localStorage unavailable")) } else { Ok(()) }
    }
    fn len(&self) -> Result<usize, BackendError> {
        if let Some(f) = self.fault(Method::Length, None) {
            return Err(Self::err(&f));
        }
        Ok(self.entries.len())
    }
    fn key(&self, index: usize) -> Result<Option<String>, BackendError> {
        if let Some(f) = self.fault(Method::Key, None) {
            return Err(Self::err(&f));
        }
        Ok(self.entries.get(index).map(|(k, _)| k.clone()))
    }
    fn get_item(&self, key: &str) -> Result<Option<String>, BackendError> {
        if let Some(f) = self.fault(Method::GetItem, Some(key)) {
            return match f.mode {
                FaultMode::Noop => Ok(None),
                FaultMode::Throw => Err(Self::err(&f)),
            };
        }
        Ok(self.raw(key).map(str::to_owned))
    }
    fn set_item(&mut self, key: &str, value: &str) -> Result<(), BackendError> {
        if let Some(f) = self.fault(Method::SetItem, Some(key)) {
            return match f.mode {
                FaultMode::Noop => Ok(()),
                FaultMode::Throw => Err(Self::err(&f)),
            };
        }
        self.set_calls += 1;
        self.put_raw(key, value);
        Ok(())
    }
    fn remove_item(&mut self, key: &str) -> Result<(), BackendError> {
        if let Some(f) = self.fault(Method::RemoveItem, Some(key)) {
            return match f.mode {
                FaultMode::Noop => Ok(()),
                FaultMode::Throw => Err(Self::err(&f)),
            };
        }
        self.entries.retain(|(k, _)| k != key);
        Ok(())
    }
}

/// Golden-generator determinism: revisions `uuid-0001`, `uuid-0002`, …;
/// `now_ms` = 1700000000000 + 1000·n (n = previous calls).
#[derive(Debug, Default)]
pub struct SeqEnv {
    uuid: u32,
    now: u32,
}

impl DraftEnv for SeqEnv {
    fn new_revision(&mut self) -> String {
        self.uuid += 1;
        format!("uuid-{:04}", self.uuid)
    }
    fn now_ms(&mut self) -> f64 {
        let n = self.now;
        self.now += 1;
        1_700_000_000_000.0 + 1000.0 * f64::from(n)
    }
}

pub type Drafts = DraftStorage<MemoryBackend>;

/// A draft store over an empty memory backend with the pinned env.
pub fn drafts() -> Drafts {
    DraftStorage::with_env(MemoryBackend::new(), Box::new(SeqEnv::default()))
}

/// A draft store over a seeded backend with the pinned env.
pub fn drafts_seeded(entries: &[(&str, &str)]) -> Drafts {
    DraftStorage::with_env(MemoryBackend::seeded(entries), Box::new(SeqEnv::default()))
}

pub type Session = AutosaveSession<MemoryBackend>;

/// An autosave session with default **[COMPAT]** timings plus a fake clock.
pub fn session() -> (Session, FakeClock) {
    (AutosaveSession::new(drafts(), AutosaveConfig::default()), FakeClock::default())
}

/// `clock.now()` shorthand.
pub fn now(clock: &FakeClock) -> Instant {
    clock.instant()
}

/// `"x".repeat(n)` (the suite's `makeContent`).
pub fn make_content(n: usize) -> String {
    "x".repeat(n)
}
