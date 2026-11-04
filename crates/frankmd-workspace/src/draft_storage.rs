//! Crash-recovery drafts, recovery-conflict copies and legacy offline backups
//! (**[COMPAT]** port of `app/javascript/lib/draft_storage.js`).
//!
//! The original keeps everything in `localStorage`. This port keeps the exact
//! record formats and key scheme over an injectable, ordered key→string store
//! ([`DraftBackend`]) so the golden scenarios (`js_draft_storage.json`) run
//! against an in-memory fake with fault injection, while the app plugs in a
//! file-backed store under the per-vault XDG state dir (through `frankmd-fs`).
//!
//! Keys (all **[COMPAT]**):
//! - `frankmd:draft:<encodeURIComponent(path)>` → [`DraftRecord`] JSON
//! - `frankmd:draft-conflict:<encodeURIComponent(path)>:<conflictId>` → [`DraftConflict`] JSON
//! - `frankmd:backup:<raw path>` → [`LegacyBackup`] JSON (historic, unencoded key)
//!
//! Every operation returns a `Result` and never panics: storage faults
//! surface as [`DraftError`] values, exactly like the original's `{ok:false, error}`.
//!
//! **Paths** here are vault-relative strings, as in the original. The DocId-keyed
//! [`crate::drafts::DraftStore`] is the native design (HP-007); see that module
//! for how the two are reconciled.

use std::fmt;

/// `frankmd:draft:` key prefix.
pub const DRAFT_PREFIX: &str = "frankmd:draft:";
/// `frankmd:draft-conflict:` key prefix.
pub const DRAFT_CONFLICT_PREFIX: &str = "frankmd:draft-conflict:";
/// `frankmd:backup:` (legacy offline backup) key prefix.
pub const BACKUP_PREFIX: &str = "frankmd:backup:";
/// The only draft schema this build reads. Newer records are preserved untouched.
pub const SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------

/// A failure raised by the storage backend (the JS `Error`/`DOMException` a
/// `localStorage` call threw). The message is kept verbatim for logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    /// Message, e.g. `"QuotaExceededError"`.
    pub message: String,
}

impl BackendError {
    /// Build from a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// The `Storage` interface the original uses (`length`, `key(i)`, `getItem`,
/// `setItem`, `removeItem`), plus [`DraftBackend::available`] for the JS
/// `storageProvider()` call that may itself throw.
///
/// Implementations must keep a stable, insertion-like key order: `key(i)` for
/// `i in 0..len()` enumerates every key once. A write is allowed to be
/// silently dropped (the original verifies every copy before deleting a source).
pub trait DraftBackend {
    /// The provider lookup (`storageProvider()`); `Err` = storage unavailable.
    fn available(&self) -> Result<(), BackendError> {
        Ok(())
    }
    /// Number of keys.
    fn len(&self) -> Result<usize, BackendError>;
    /// True when there are no keys.
    fn is_empty(&self) -> Result<bool, BackendError> {
        self.len().map(|n| n == 0)
    }
    /// Key at `index` (`None` past the end).
    fn key(&self, index: usize) -> Result<Option<String>, BackendError>;
    /// Raw value for `key`.
    fn get_item(&self, key: &str) -> Result<Option<String>, BackendError>;
    /// Store `value` under `key` (replacing in place, keeping the key's position).
    fn set_item(&mut self, key: &str, value: &str) -> Result<(), BackendError>;
    /// Remove `key` (no-op when absent).
    fn remove_item(&mut self, key: &str) -> Result<(), BackendError>;
}

/// Sources of non-determinism, injectable for tests: the draft-revision token
/// (`crypto.randomUUID()`) and wall-clock milliseconds (`Date.now()`).
pub trait DraftEnv {
    /// A fresh, unique draft revision / conflict id.
    fn new_revision(&mut self) -> String;
    /// Wall-clock time in epoch milliseconds.
    fn now_ms(&mut self) -> f64;
}

/// Real environment: UUIDv7 tokens and the system clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemDraftEnv;

impl DraftEnv for SystemDraftEnv {
    fn new_revision(&mut self) -> String {
        todo!("uuid v7 string")
    }
    fn now_ms(&mut self) -> f64 {
        todo!("SystemTime::now() as epoch ms")
    }
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

/// A versioned local draft (`schemaVersion: 1`).
///
/// Serialized as JSON with keys in this exact order:
/// `schemaVersion, path, content, baseRevision, draftRevision, updatedAt`,
/// followed by any unknown fields that were read (they round-trip through a remap).
/// `updated_at` is a JS number: integral values must serialize without a
/// fraction (`1700000000000`, not `1700000000000.0`).
#[derive(Debug, Clone, PartialEq)]
pub struct DraftRecord {
    /// Vault-relative path. Must equal the path encoded in the key.
    pub path: String,
    /// Unsaved editor text (may be empty — an empty draft is still a draft).
    pub content: String,
    /// Server revision the draft was based on (non-empty).
    pub base_revision: String,
    /// Equality token for this exact snapshot (non-empty).
    pub draft_revision: String,
    /// `Date.now()` at write time (any finite number is accepted on read).
    pub updated_at: f64,
    /// Unknown extra fields from a read record, as raw `(key, JSON)` pairs in order.
    pub extra: Vec<(String, String)>,
}

/// A preserved recovery copy created when a remap would have overwritten a
/// destination draft (or when the user's choice displaced one).
#[derive(Debug, Clone, PartialEq)]
pub struct DraftConflict {
    /// Path this copy belongs to (the destination).
    pub path: String,
    /// Path the draft was originally written for.
    pub source_path: String,
    /// Conflict id (a fresh revision token).
    pub conflict_id: String,
    /// Draft text.
    pub content: String,
    /// Baseline of the original draft.
    pub base_revision: String,
    /// Draft revision of the original draft.
    pub draft_revision: String,
    /// `updatedAt` of the original draft.
    pub updated_at: f64,
}

/// Legacy offline backup `{content, timestamp}` (pre-draft FrankMD versions and
/// the offline-backup path).
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyBackup {
    /// Backup text.
    pub content: String,
    /// `Date.now()` at write time.
    pub timestamp: f64,
}

/// File or folder, for path-boundary matching (`"file"` / `"folder"` in JS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemKind {
    /// Exact path match only.
    #[default]
    File,
    /// The path itself or anything below `path + "/"`.
    Folder,
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// Error class of the original (`TypeError` for argument validation,
/// `Error` for everything else). Drives the JS-compat golden comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftErrorKind {
    /// Invalid arguments (`TypeError`).
    InvalidArgument,
    /// Storage failure or consistency failure (`Error`).
    Storage,
}

/// The `error` of an `{ok:false}` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftError {
    /// Class.
    pub kind: DraftErrorKind,
    /// Message, verbatim from the original where it builds one
    /// (e.g. `"Unsupported draft schema version: 2"`), or the backend's message.
    pub message: String,
}

impl fmt::Display for DraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DraftError {}

impl From<BackendError> for DraftError {
    fn from(e: BackendError) -> Self {
        Self { kind: DraftErrorKind::Storage, message: e.message }
    }
}

/// `readDraft` success: `{ok:true, draft}` or `{ok:true, draft:null, malformed:true}`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadDraft {
    /// The valid draft, if any.
    pub draft: Option<DraftRecord>,
    /// A malformed record was found and discarded.
    pub malformed: bool,
}

/// `writeDraft` success: `{ok:true, draft, unchanged}`.
#[derive(Debug, Clone, PartialEq)]
pub struct WriteDraft {
    /// The stored (or already-present identical) draft.
    pub draft: DraftRecord,
    /// The same content+baseline was already stored; nothing was written and
    /// the existing `draftRevision` is kept.
    pub unchanged: bool,
}

/// `readBackup` success: `{ok:true, backup}` / `{…, malformed:true}`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadBackup {
    /// The valid backup, if any.
    pub backup: Option<LegacyBackup>,
    /// A malformed backup was found and removed.
    pub malformed: bool,
}

/// A `{sourcePath, destinationPath}` pair reported by a remap collision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPair {
    /// Original path.
    pub source_path: String,
    /// Remapped path.
    pub destination_path: String,
}

/// `remapDrafts` failure. Field presence mirrors the original's result object.
#[derive(Debug, Clone, PartialEq)]
pub struct RemapError {
    /// The error.
    pub error: DraftError,
    /// Destinations that may now hold partial state (`affectedPaths`); `None`
    /// for argument/listing failures that happen before planning.
    pub affected_paths: Option<Vec<String>>,
    /// Item that failed (`sourcePath`).
    pub source_path: Option<String>,
    /// Its destination (`destinationPath`).
    pub destination_path: Option<String>,
    /// A different legacy backup already sits at the destination (`backupCollision`).
    pub backup_collision: bool,
    /// A different draft already sat at a destination; every source draft was
    /// preserved as a recovery conflict at its destination (`collision`).
    pub collision: bool,
    /// Conflict id preserved for the first collision (`conflictId`).
    pub conflict_id: Option<String>,
    /// All collisions (`collisions`), present only when `collision`.
    pub collisions: Option<Vec<PathPair>>,
}

/// `removeDrafts` failure: `{ok:false, error}` or `{ok:false, error, removed}`.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveDraftsError {
    /// First error encountered.
    pub error: DraftError,
    /// Records removed before/around the failure (absent for validation and
    /// provider failures, `Some(0)` when key enumeration failed).
    pub removed: Option<usize>,
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// `frankmd:draft:` + `encodeURIComponent(path)` (**[COMPAT]** JS encoding:
/// everything except `A-Z a-z 0-9 - _ . ! ~ * ' ( )` is percent-encoded as UTF-8).
pub fn draft_key(path: &str) -> String {
    todo!()
}

/// `draft_conflict_path_prefix(path) + conflict_id`.
pub fn draft_conflict_key(path: &str, conflict_id: &str) -> String {
    todo!()
}

/// `frankmd:draft-conflict:` + `encodeURIComponent(path)` + `:`.
pub fn draft_conflict_path_prefix(path: &str) -> String {
    todo!()
}

/// `frankmd:backup:` + raw path (historic format, not encoded).
pub fn backup_key(path: &str) -> String {
    todo!()
}

/// `decodeURIComponent` inverse of the key encoding; `None` on malformed escapes
/// (such keys are skipped by [`DraftStorage::remove_drafts`]).
pub fn decode_key_path(encoded: &str) -> Option<String> {
    todo!()
}

/// Path-segment boundary match: `File` → equal; `Folder` → equal or starts with
/// `target + "/"` (so `docs` never matches `docs-old/…`).
pub fn path_matches(path: &str, target: &str, kind: ItemKind) -> bool {
    todo!()
}

/// Remap `path` for a rename of `old` → `new` (`None`/unaffected paths are
/// returned unchanged). Folder: `new + path[old.len()..]` when [`path_matches`].
pub fn remap_path(path: &str, old: &str, new: &str, kind: ItemKind) -> String {
    todo!()
}

/// Inverse of [`remap_path`]: the pre-rename path of `path`, or `None` if it is
/// not under `new`.
pub fn reverse_remap_path(path: &str, old: &str, new: &str, kind: ItemKind) -> Option<String> {
    todo!()
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

/// The draft store (`DraftStorage` class).
pub struct DraftStorage<B: DraftBackend> {
    backend: B,
    env: Box<dyn DraftEnv + Send>,
}

impl<B: DraftBackend> fmt::Debug for DraftStorage<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DraftStorage").finish_non_exhaustive()
    }
}

impl<B: DraftBackend> DraftStorage<B> {
    /// Store over `backend` with the real environment.
    pub fn new(backend: B) -> Self {
        Self::with_env(backend, Box::new(SystemDraftEnv))
    }

    /// Store with an injected revision/clock source (tests pin these).
    pub fn with_env(backend: B, env: Box<dyn DraftEnv + Send>) -> Self {
        Self { backend, env }
    }

    /// The backend (tests inspect raw entries through it).
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Mutable backend (tests seed raw keys and install faults through it).
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Read the draft for `path`. Malformed JSON or an invalid/mismatched
    /// record is removed and reported as `malformed`; a record with a numeric
    /// `schemaVersion != 1` is **kept** and reported as an error
    /// (`"Unsupported draft schema version: <n>"`).
    pub fn read_draft(&mut self, path: &str) -> Result<ReadDraft, DraftError> {
        todo!()
    }

    /// Write a draft snapshot. `base_revision` must be non-empty
    /// (`TypeError "A draft requires a path, string content, and server revision"`).
    /// An identical content+baseline is not rewritten (`unchanged: true`).
    pub fn write_draft(&mut self, path: &str, content: &str, base_revision: &str) -> Result<WriteDraft, DraftError> {
        todo!()
    }

    /// Remove the draft only if its `draftRevision` matches. `Ok(removed)`.
    pub fn remove_draft_if_revision(&mut self, path: &str, draft_revision: &str) -> Result<bool, DraftError> {
        todo!()
    }

    /// Unconditionally remove the draft for `path`.
    pub fn remove_draft(&mut self, path: &str) -> Result<(), DraftError> {
        todo!()
    }

    /// Move every draft, conflict copy and legacy backup under `old` to `new`
    /// (copy → verify → delete sources). Never overwrites a different
    /// destination record: a draft collision preserves all sources as
    /// conflicts and fails with `collision`; a legacy-backup collision fails
    /// with `backup_collision` before anything is written. `Ok(remapped)` counts
    /// drafts + conflicts + backups that moved.
    pub fn remap_drafts(&mut self, old: &str, new: &str, kind: ItemKind) -> Result<usize, Box<RemapError>> {
        todo!()
    }

    /// Remove drafts, legacy backups and conflict copies for a deleted item
    /// (path-boundary match). Continues past individual failures and reports
    /// the first one. `Ok(removed)`.
    pub fn remove_drafts(&mut self, path: &str, kind: ItemKind) -> Result<usize, RemoveDraftsError> {
        todo!()
    }

    /// Valid conflict copies whose key and `path` both belong to `path`,
    /// sorted by `updatedAt` ascending.
    pub fn list_draft_conflicts(&mut self, path: &str) -> Result<Vec<DraftConflict>, DraftError> {
        todo!()
    }

    /// Store `draft` as a verified recovery copy at `path` with a fresh conflict id.
    pub fn preserve_draft_conflict(&mut self, path: &str, draft: &DraftRecord) -> Result<DraftConflict, DraftError> {
        todo!()
    }

    /// Remove a conflict copy only if its `draftRevision` matches. `Ok(removed)`.
    pub fn remove_draft_conflict_if_revision(
        &mut self,
        path: &str,
        conflict_id: &str,
        draft_revision: &str,
    ) -> Result<bool, DraftError> {
        todo!()
    }

    /// All valid drafts stored under their canonical key, in backend key order.
    pub fn list_drafts(&mut self) -> Result<Vec<DraftRecord>, DraftError> {
        todo!()
    }

    /// Remove every `frankmd:draft:` and `frankmd:draft-conflict:` key (other keys untouched).
    pub fn remove_all_drafts(&mut self) -> Result<(), DraftError> {
        todo!()
    }

    /// Read the legacy backup for `path` (malformed → removed, `malformed: true`).
    pub fn read_backup(&mut self, path: &str) -> Result<ReadBackup, DraftError> {
        todo!()
    }

    /// Write a legacy backup `{content, timestamp: now}`.
    pub fn write_backup(&mut self, path: &str, content: &str) -> Result<(), DraftError> {
        todo!()
    }

    /// Remove the legacy backup for `path`.
    pub fn remove_backup(&mut self, path: &str) -> Result<(), DraftError> {
        todo!()
    }

    /// Remove the legacy backup only if it is exactly this snapshot. `Ok(removed)`.
    pub fn remove_backup_if_snapshot(&mut self, path: &str, content: &str, timestamp: f64) -> Result<bool, DraftError> {
        todo!()
    }

    /// Remove every `frankmd:backup:` key.
    pub fn remove_all_backups(&mut self) -> Result<(), DraftError> {
        todo!()
    }
}

/// Legacy offline-backup facade (port of `offline_backup_controller.js`):
/// failures are logged (warn) and swallowed.
impl<B: DraftBackend> DraftStorage<B> {
    /// `save(path, content)`: write a backup; failure → warning only.
    pub fn backup_save(&mut self, path: &str, content: &str) -> Result<(), DraftError> {
        todo!()
    }

    /// `check(path, serverContent)`: no/corrupt backup → `None`; equal to the
    /// server content → removed and `None`; otherwise the backup.
    pub fn backup_check(&mut self, path: &str, server_content: &str) -> Option<LegacyBackup> {
        todo!()
    }
}
