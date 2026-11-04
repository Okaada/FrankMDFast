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
//! **Paths** here are the record *subjects*: vault-relative paths in the
//! original and in the golden scenarios. The native app files every record
//! under the note's DocId instead ([`crate::drafts::draft_subject`], HP-001/007),
//! so a rename never needs [`DraftStorage::remap_drafts`]; it remains for
//! re-keying a draft from one DocId to another.

mod codec;
mod keys;

use codec::{Json, Object};
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
        Self {
            message: message.into(),
        }
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
        uuid::Uuid::now_v7().to_string()
    }
    fn now_ms(&mut self) -> f64 {
        // Date.now(): whole milliseconds since the epoch.
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_millis() as f64)
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
        Self {
            kind: DraftErrorKind::Storage,
            message: e.message,
        }
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
    format!("{DRAFT_PREFIX}{}", keys::encode_uri_component(path))
}

/// `draft_conflict_path_prefix(path) + conflict_id`.
pub fn draft_conflict_key(path: &str, conflict_id: &str) -> String {
    format!("{}{conflict_id}", draft_conflict_path_prefix(path))
}

/// `frankmd:draft-conflict:` + `encodeURIComponent(path)` + `:`.
pub fn draft_conflict_path_prefix(path: &str) -> String {
    format!(
        "{DRAFT_CONFLICT_PREFIX}{}:",
        keys::encode_uri_component(path)
    )
}

/// `frankmd:backup:` + raw path (historic format, not encoded).
pub fn backup_key(path: &str) -> String {
    // Keep the historic key format so backups written by older FrankMD
    // versions remain recoverable.
    format!("{BACKUP_PREFIX}{path}")
}

/// `decodeURIComponent` inverse of the key encoding; `None` on malformed escapes
/// (such keys are skipped by [`DraftStorage::remove_drafts`]).
pub fn decode_key_path(encoded: &str) -> Option<String> {
    keys::decode_uri_component(encoded)
}

/// Path-segment boundary match: `File` → equal; `Folder` → equal or starts with
/// `target + "/"` (so `docs` never matches `docs-old/…`).
pub fn path_matches(path: &str, target: &str, kind: ItemKind) -> bool {
    match kind {
        ItemKind::Folder => {
            path == target
                || path
                    .strip_prefix(target)
                    .is_some_and(|rest| rest.starts_with('/'))
        }
        ItemKind::File => path == target,
    }
}

/// Remap `path` for a rename of `old` → `new` (`None`/unaffected paths are
/// returned unchanged). Folder: `new + path[old.len()..]` when [`path_matches`].
pub fn remap_path(path: &str, old: &str, new: &str, kind: ItemKind) -> String {
    match path.strip_prefix(old) {
        Some(rest) if path_matches(path, old, kind) => format!("{new}{rest}"),
        _ => path.to_owned(),
    }
}

/// Inverse of [`remap_path`]: the pre-rename path of `path`, or `None` if it is
/// not under `new`.
pub fn reverse_remap_path(path: &str, old: &str, new: &str, kind: ItemKind) -> Option<String> {
    let rest = path.strip_prefix(new)?;
    path_matches(path, new, kind).then(|| format!("{old}{rest}"))
}

/// Destination of `path` (which matched `old`) — the JS
/// `` `${newPath}${path.slice(oldPath.length)}` ``.
fn moved_path(path: &str, old: &str, new: &str) -> String {
    format!("{new}{}", path.get(old.len()..).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Record codec (validation mirrors `validDraft` / `validDraftConflict` / `validBackup`)
// ---------------------------------------------------------------------------

/// Draft fields with a fixed position; everything else is an "extra".
const DRAFT_FIELDS: [&str; 6] = [
    "schemaVersion",
    "path",
    "content",
    "baseRevision",
    "draftRevision",
    "updatedAt",
];

fn non_empty(s: Option<&str>) -> Option<&str> {
    s.filter(|s| !s.is_empty())
}

impl DraftRecord {
    /// `validDraft(value, path)` and conversion.
    fn from_json(value: &Json, path: &str) -> Option<Self> {
        let obj = value.as_object()?;
        if !codec::is_one(obj, "schemaVersion") || codec::get_str(obj, "path")? != path {
            return None;
        }
        Some(Self {
            path: path.to_owned(),
            content: codec::get_str(obj, "content")?.to_owned(),
            base_revision: non_empty(codec::get_str(obj, "baseRevision"))?.to_owned(),
            draft_revision: non_empty(codec::get_str(obj, "draftRevision"))?.to_owned(),
            updated_at: codec::get_finite(obj, "updatedAt")?,
            extra: obj
                .iter()
                .filter(|(k, _)| !DRAFT_FIELDS.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), codec::stringify(v)))
                .collect(),
        })
    }

    /// `JSON.stringify(draft)`: canonical fields, then the extras.
    fn to_json(&self) -> String {
        let fixed: Object = vec![
            (
                "schemaVersion".into(),
                Json::Number(f64::from(SCHEMA_VERSION)),
            ),
            ("path".into(), Json::String(self.path.clone())),
            ("content".into(), Json::String(self.content.clone())),
            (
                "baseRevision".into(),
                Json::String(self.base_revision.clone()),
            ),
            (
                "draftRevision".into(),
                Json::String(self.draft_revision.clone()),
            ),
            ("updatedAt".into(), Json::Number(self.updated_at)),
        ];
        let mut out = codec::stringify_object(&fixed);
        if !self.extra.is_empty() {
            out.pop(); // closing brace
            for (key, raw) in &self.extra {
                out.push(',');
                out.push_str(&codec::stringify(&Json::String(key.clone())));
                out.push(':');
                out.push_str(raw);
            }
            out.push('}');
        }
        out
    }

    /// `validDraft(draft, draft.path)` for a caller-supplied record.
    fn is_valid(&self) -> bool {
        !self.base_revision.is_empty()
            && !self.draft_revision.is_empty()
            && self.updated_at.is_finite()
    }

    /// `{...draft, path}`.
    fn with_path(&self, path: &str) -> Self {
        Self {
            path: path.to_owned(),
            ..self.clone()
        }
    }
}

/// `sameDraftRecord(left, right)`.
fn same_draft_record(left: &DraftRecord, right: &DraftRecord) -> bool {
    left.path == right.path
        && left.content == right.content
        && left.base_revision == right.base_revision
        && left.draft_revision == right.draft_revision
        && left.updated_at == right.updated_at
}

impl DraftConflict {
    /// `validDraftConflict(value)` and conversion.
    fn from_json(value: &Json) -> Option<Self> {
        let obj = value.as_object()?;
        if !codec::is_one(obj, "schemaVersion") {
            return None;
        }
        let s = |key: &str| codec::get_str(obj, key).map(str::to_owned);
        Some(Self {
            path: s("path")?,
            source_path: s("sourcePath")?,
            conflict_id: s("conflictId")?,
            content: s("content")?,
            base_revision: s("baseRevision")?,
            draft_revision: s("draftRevision")?,
            updated_at: codec::get_finite(obj, "updatedAt")?,
        })
    }

    /// The record as an ordered object (the original's literal key order).
    fn to_object(&self) -> Object {
        vec![
            (
                "schemaVersion".into(),
                Json::Number(f64::from(SCHEMA_VERSION)),
            ),
            ("path".into(), Json::String(self.path.clone())),
            ("sourcePath".into(), Json::String(self.source_path.clone())),
            ("conflictId".into(), Json::String(self.conflict_id.clone())),
            ("content".into(), Json::String(self.content.clone())),
            (
                "baseRevision".into(),
                Json::String(self.base_revision.clone()),
            ),
            (
                "draftRevision".into(),
                Json::String(self.draft_revision.clone()),
            ),
            ("updatedAt".into(), Json::Number(self.updated_at)),
        ]
    }
}

/// `sameDraftConflict(left, right)`.
fn same_draft_conflict(left: &DraftConflict, right: &DraftConflict) -> bool {
    left.path == right.path
        && left.source_path == right.source_path
        && left.conflict_id == right.conflict_id
        && left.content == right.content
        && left.base_revision == right.base_revision
        && left.draft_revision == right.draft_revision
        && left.updated_at == right.updated_at
}

impl LegacyBackup {
    /// `validBackup(value)` and conversion.
    fn from_json(value: &Json) -> Option<Self> {
        let obj = value.as_object()?;
        Some(Self {
            content: codec::get_str(obj, "content")?.to_owned(),
            timestamp: codec::get_finite(obj, "timestamp")?,
        })
    }
}

/// `JSON.parse(raw || "null")`, mapping a syntax error to a storage error.
fn parse_or_null(raw: Option<&str>) -> Result<Json, DraftError> {
    let text = raw.filter(|r| !r.is_empty()).unwrap_or("null");
    codec::parse(text)
        .ok_or_else(|| storage_error("Unexpected token in JSON stored for a draft record"))
}

fn storage_error(message: impl Into<String>) -> DraftError {
    DraftError {
        kind: DraftErrorKind::Storage,
        message: message.into(),
    }
}

fn type_error(message: &str) -> DraftError {
    DraftError {
        kind: DraftErrorKind::InvalidArgument,
        message: message.to_owned(),
    }
}

impl RemapError {
    /// A bare `{ok:false, error}`.
    fn plain(error: DraftError) -> Self {
        Self {
            error,
            affected_paths: None,
            source_path: None,
            destination_path: None,
            backup_collision: false,
            collision: false,
            conflict_id: None,
            collisions: None,
        }
    }

    /// `{ok:false, error, sourcePath, destinationPath}`.
    fn at(error: DraftError, source_path: &str, destination_path: &str) -> Self {
        Self {
            source_path: Some(source_path.to_owned()),
            destination_path: Some(destination_path.to_owned()),
            ..Self::plain(error)
        }
    }
}

// ---------------------------------------------------------------------------
// Remap plans
// ---------------------------------------------------------------------------

/// A draft to move.
struct DraftMove {
    source_path: String,
    destination_path: String,
    draft: DraftRecord,
    /// The record as read, so the copy keeps its exact key order (`{...draft, path}`).
    object: Object,
    destination_exists: bool,
}

/// A legacy backup to move (raw bytes are copied verbatim).
struct BackupMove {
    source_key: String,
    source_path: String,
    destination_path: String,
    destination_key: String,
    raw: String,
}

/// A recovery conflict copy to move.
struct ConflictMove {
    source_path: String,
    destination_path: String,
    destination_key: String,
    conflict: DraftConflict,
    object: Object,
    destination_exists: bool,
}

impl ConflictMove {
    fn moved(&self) -> bool {
        self.source_path != self.destination_path
    }
}

/// A boxed bare `{ok:false, error}`.
fn boxed_plain(error: DraftError) -> Box<RemapError> {
    Box::new(RemapError::plain(error))
}

/// `{ok:false, error, sourcePath, destinationPath}` closure helper.
fn err_at(source: &str, destination: &str) -> impl Fn(DraftError) -> Box<RemapError> {
    let (source, destination) = (source.to_owned(), destination.to_owned());
    move |e| Box::new(RemapError::at(e, &source, &destination))
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

/// Raw backend access (the JS `storage.*` calls), with errors as [`DraftError`].
impl<B: DraftBackend> DraftStorage<B> {
    /// `this.storageProvider()`.
    fn provider(&self) -> Result<(), DraftError> {
        Ok(self.backend.available()?)
    }

    fn get(&self, key: &str) -> Result<Option<String>, DraftError> {
        Ok(self.backend.get_item(key)?)
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), DraftError> {
        Ok(self.backend.set_item(key, value)?)
    }

    fn remove(&mut self, key: &str) -> Result<(), DraftError> {
        Ok(self.backend.remove_item(key)?)
    }

    /// `keysWithPrefix(storage, prefix)` (re-reads `length` every iteration).
    fn keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, DraftError> {
        let mut keys = Vec::new();
        let mut i = 0;
        while i < self.backend.len()? {
            if let Some(key) = self.backend.key(i)?
                && key.starts_with(prefix)
            {
                keys.push(key);
            }
            i += 1;
        }
        Ok(keys)
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
        self.provider()?;
        let Some(raw) = self.get(&draft_key(path))? else {
            return Ok(ReadDraft {
                draft: None,
                malformed: false,
            });
        };
        let Some(value) = codec::parse(&raw) else {
            return self.discard_malformed_draft(path);
        };
        if let Some(obj) = value.as_object()
            && let Some(Json::Number(version)) = codec::get(obj, "schemaVersion")
            && *version != 1.0
        {
            return Err(storage_error(format!(
                "Unsupported draft schema version: {}",
                codec::js_number(*version)
            )));
        }
        match DraftRecord::from_json(&value, path) {
            Some(draft) => Ok(ReadDraft {
                draft: Some(draft),
                malformed: false,
            }),
            None => self.discard_malformed_draft(path),
        }
    }

    fn discard_malformed_draft(&mut self, path: &str) -> Result<ReadDraft, DraftError> {
        self.provider()?;
        self.remove(&draft_key(path))?;
        Ok(ReadDraft {
            draft: None,
            malformed: true,
        })
    }

    /// Write a draft snapshot. `base_revision` must be non-empty
    /// (`TypeError "A draft requires a path, string content, and server revision"`).
    /// An identical content+baseline is not rewritten (`unchanged: true`).
    pub fn write_draft(
        &mut self,
        path: &str,
        content: &str,
        base_revision: &str,
    ) -> Result<WriteDraft, DraftError> {
        if base_revision.is_empty() {
            return Err(type_error(
                "A draft requires a path, string content, and server revision",
            ));
        }
        if let Some(existing) = self.read_draft(path)?.draft
            && existing.content == content
            && existing.base_revision == base_revision
        {
            return Ok(WriteDraft {
                draft: existing,
                unchanged: true,
            });
        }
        let draft = DraftRecord {
            path: path.to_owned(),
            content: content.to_owned(),
            base_revision: base_revision.to_owned(),
            draft_revision: self.env.new_revision(),
            updated_at: self.env.now_ms(),
            extra: Vec::new(),
        };
        self.provider()?;
        self.set(&draft_key(path), &draft.to_json())?;
        Ok(WriteDraft {
            draft,
            unchanged: false,
        })
    }

    /// Remove the draft only if its `draftRevision` matches. `Ok(removed)`.
    pub fn remove_draft_if_revision(
        &mut self,
        path: &str,
        draft_revision: &str,
    ) -> Result<bool, DraftError> {
        self.provider()?;
        let key = draft_key(path);
        let Some(raw) = self.get(&key)? else {
            return Ok(false);
        };
        let matches = codec::parse(&raw)
            .and_then(|v| DraftRecord::from_json(&v, path))
            .is_some_and(|d| d.draft_revision == draft_revision);
        if !matches {
            return Ok(false);
        }
        self.remove(&key)?;
        Ok(true)
    }

    /// Unconditionally remove the draft for `path`.
    pub fn remove_draft(&mut self, path: &str) -> Result<(), DraftError> {
        self.provider()?;
        self.remove(&draft_key(path))
    }

    /// Move every draft, conflict copy and legacy backup under `old` to `new`
    /// (copy → verify → delete sources). Never overwrites a different
    /// destination record: a draft collision preserves all sources as
    /// conflicts and fails with `collision`; a legacy-backup collision fails
    /// with `backup_collision` before anything is written. `Ok(remapped)` counts
    /// drafts + conflicts + backups that moved.
    pub fn remap_drafts(
        &mut self,
        old: &str,
        new: &str,
        kind: ItemKind,
    ) -> Result<usize, Box<RemapError>> {
        if old.is_empty() || new.is_empty() {
            return Err(Box::new(RemapError::plain(type_error(
                "Draft remapping requires source and destination paths",
            ))));
        }
        if old == new {
            return Ok(0);
        }

        let listed = self
            .list_draft_entries()
            .map_err(|e| Box::new(RemapError::plain(e)))?;
        let mut plan: Vec<DraftMove> = listed
            .into_iter()
            .filter(|(draft, _)| path_matches(&draft.path, old, kind))
            .map(|(draft, object)| DraftMove {
                source_path: draft.path.clone(),
                destination_path: moved_path(&draft.path, old, new),
                draft,
                object,
                destination_exists: false,
            })
            .collect();
        let mut affected: Vec<String> = plan
            .iter()
            .filter(|item| item.source_path != item.destination_path)
            .map(|item| item.destination_path.clone())
            .collect();
        match self.remap_planned(old, new, kind, &mut plan, &mut affected) {
            Ok(count) => Ok(count),
            Err(mut e) => {
                e.affected_paths = Some(affected);
                Err(e)
            }
        }
    }

    /// The body of `remapDrafts` after the draft plan is built; every error
    /// goes through the original's `fail()` (adds `affectedPaths`).
    fn remap_planned(
        &mut self,
        old: &str,
        new: &str,
        kind: ItemKind,
        plan: &mut [DraftMove],
        affected: &mut Vec<String>,
    ) -> Result<usize, Box<RemapError>> {
        // Check all destinations before writing any of them. A stale destination
        // draft may be the only copy of another note, so never overwrite it.
        let mut collisions: Vec<usize> = Vec::new();
        for (index, item) in plan.iter_mut().enumerate() {
            let source = self.read_draft(&item.source_path).map_err(boxed_plain)?;
            if source.draft.as_ref().map(|d| d.draft_revision.as_str())
                != Some(item.draft.draft_revision.as_str())
            {
                return Err(boxed_plain(storage_error(format!(
                    "Draft changed while remapping {}",
                    item.source_path
                ))));
            }
            let destination = self
                .read_draft(&item.destination_path)
                .map_err(boxed_plain)?;
            if let Some(existing) = &destination.draft
                && !same_draft_record(existing, &item.draft.with_path(&item.destination_path))
            {
                collisions.push(index);
                item.destination_exists = true;
                continue;
            }
            item.destination_exists = destination.draft.is_some();
        }

        self.provider().map_err(boxed_plain)?;

        let backup_plan = self.plan_backups(old, new, kind).map_err(boxed_plain)?;
        for item in &backup_plan {
            if item.source_path != item.destination_path
                && !affected.contains(&item.destination_path)
            {
                affected.push(item.destination_path.clone());
            }
        }

        // Legacy backups are keyed directly by path. Preflight every destination
        // before changing any record so an existing, unrelated recovery copy is
        // never silently replaced.
        for item in &backup_plan {
            if item.source_path == item.destination_path {
                continue;
            }
            let at = err_at(&item.source_path, &item.destination_path);
            let raw = self.get(&item.destination_key).map_err(&at)?;
            if raw.is_some_and(|raw| raw != item.raw) {
                return Err(backup_collision(item));
            }
        }

        let mut conflict_plan = self.plan_conflicts(old, new, kind).map_err(boxed_plain)?;
        for item in &conflict_plan {
            if item.moved() && !affected.contains(&item.destination_path) {
                affected.push(item.destination_path.clone());
            }
        }

        for item in &mut conflict_plan {
            if !item.moved() {
                continue;
            }
            let at = err_at(&item.source_path, &item.destination_path);
            let Some(raw) = self.get(&item.destination_key).map_err(&at)? else {
                continue;
            };
            let exists_error = || {
                boxed_plain(storage_error(format!(
                    "A recovery copy already exists at {}",
                    item.destination_path
                )))
            };
            let expected = DraftConflict {
                path: item.destination_path.clone(),
                ..item.conflict.clone()
            };
            let same = codec::parse(&raw)
                .and_then(|v| DraftConflict::from_json(&v))
                .is_some_and(|existing| same_draft_conflict(&existing, &expected));
            if !same {
                return Err(exists_error());
            }
            item.destination_exists = true;
        }

        // A folder remap may contain several drafts, and a collision in one child
        // must not strand the remaining drafts at paths the server just removed.
        // Preserve each source draft at its new path before removing any source.
        if !collisions.is_empty() {
            return Err(self.preserve_collided(plan, &collisions, &conflict_plan, &backup_plan));
        }

        for item in plan.iter() {
            if item.source_path == item.destination_path || item.destination_exists {
                continue;
            }
            let at = err_at(&item.source_path, &item.destination_path);
            let destination_draft = item.draft.with_path(&item.destination_path);
            let mut object = item.object.clone();
            codec::set(
                &mut object,
                "path",
                Json::String(item.destination_path.clone()),
            );
            self.set(
                &draft_key(&item.destination_path),
                &codec::stringify_object(&object),
            )
            .map_err(&at)?;

            let verification = self.read_draft(&item.destination_path).map_err(&at)?;
            if !verification
                .draft
                .is_some_and(|d| same_draft_record(&d, &destination_draft))
            {
                return Err(at(storage_error(format!(
                    "Unable to verify the local draft at {}",
                    item.destination_path
                ))));
            }
        }

        self.copy_conflicts(&conflict_plan)?;
        self.copy_legacy_backups(&backup_plan)?;

        // Only remove sources after every destination has been confirmed. A failed
        // removal leaves two valid copies, which is safer than losing the draft.
        for item in plan.iter() {
            if item.source_path == item.destination_path {
                continue;
            }
            self.remove_moved_draft(item)?;
        }
        self.remove_conflict_sources(&conflict_plan)?;
        self.remove_legacy_backup_sources(&backup_plan)?;

        Ok(plan
            .iter()
            .filter(|item| item.source_path != item.destination_path)
            .count()
            + conflict_plan.iter().filter(|item| item.moved()).count()
            + backup_plan
                .iter()
                .filter(|item| item.source_path != item.destination_path)
                .count())
    }

    /// The collision branch of `remapDrafts`: keep every source draft as a
    /// recovery conflict at its destination, move conflicts and backups, then
    /// remove the sources. Always returns the `collision` failure (or the
    /// first error on the way).
    fn preserve_collided(
        &mut self,
        plan: &[DraftMove],
        collisions: &[usize],
        conflict_plan: &[ConflictMove],
        backup_plan: &[BackupMove],
    ) -> Box<RemapError> {
        let mut preserved: Vec<DraftConflict> = Vec::with_capacity(plan.len());
        for item in plan {
            let at = err_at(&item.source_path, &item.destination_path);
            let listed = match self.list_draft_conflicts(&item.destination_path) {
                Ok(listed) => listed,
                Err(e) => return at(e),
            };
            let draft = &item.draft;
            let existing = listed.into_iter().find(|c| {
                c.source_path == item.source_path
                    && c.content == draft.content
                    && c.base_revision == draft.base_revision
                    && c.draft_revision == draft.draft_revision
                    && c.updated_at == draft.updated_at
            });
            let conflict = match existing {
                Some(conflict) => conflict,
                None => match self.preserve_draft_conflict(&item.destination_path, draft) {
                    Ok(conflict) => conflict,
                    Err(e) => return at(e),
                },
            };
            preserved.push(conflict);
        }

        // Existing recovery records also follow the moved folder. The source
        // record is retained until its destination copy has been verified.
        if let Err(e) = self.copy_conflicts(conflict_plan) {
            return e;
        }
        if let Err(e) = self.copy_legacy_backups(backup_plan) {
            return e;
        }
        for item in plan {
            if let Err(e) = self.remove_moved_draft(item) {
                return e;
            }
        }
        if let Err(e) = self.remove_conflict_sources(conflict_plan) {
            return e;
        }
        if let Err(e) = self.remove_legacy_backup_sources(backup_plan) {
            return e;
        }

        let Some(&first_index) = collisions.first() else {
            return boxed_plain(storage_error("A local draft already exists"));
        };
        let first = &plan[first_index];
        Box::new(RemapError {
            collision: true,
            conflict_id: preserved.get(first_index).map(|c| c.conflict_id.clone()),
            collisions: Some(
                collisions
                    .iter()
                    .map(|&i| PathPair {
                        source_path: plan[i].source_path.clone(),
                        destination_path: plan[i].destination_path.clone(),
                    })
                    .collect(),
            ),
            ..RemapError::at(
                storage_error(format!(
                    "A local draft already exists at {}",
                    first.destination_path
                )),
                &first.source_path,
                &first.destination_path,
            )
        })
    }

    /// Legacy backups under `old` (`backupPlan`).
    fn plan_backups(
        &self,
        old: &str,
        new: &str,
        kind: ItemKind,
    ) -> Result<Vec<BackupMove>, DraftError> {
        let mut out = Vec::new();
        for source_key in self.keys_with_prefix(BACKUP_PREFIX)? {
            let source_path = source_key
                .get(BACKUP_PREFIX.len()..)
                .unwrap_or_default()
                .to_owned();
            if !path_matches(&source_path, old, kind) {
                continue;
            }
            let Some(raw) = self.get(&source_key)? else {
                continue;
            };
            let destination_path = moved_path(&source_path, old, new);
            out.push(BackupMove {
                destination_key: backup_key(&destination_path),
                source_key,
                source_path,
                destination_path,
                raw,
            });
        }
        Ok(out)
    }

    /// Valid conflict copies whose `path` is under `old` (`conflictPlan`).
    fn plan_conflicts(
        &self,
        old: &str,
        new: &str,
        kind: ItemKind,
    ) -> Result<Vec<ConflictMove>, DraftError> {
        let mut out = Vec::new();
        for source_key in self.keys_with_prefix(DRAFT_CONFLICT_PREFIX)? {
            let Some(raw) = self.get(&source_key)? else {
                continue;
            };
            let Some(value) = codec::parse(&raw) else {
                continue;
            };
            let Some(conflict) = DraftConflict::from_json(&value) else {
                continue;
            };
            let Json::Object(object) = value else {
                continue;
            };
            if !path_matches(&conflict.path, old, kind) {
                continue;
            }
            let destination_path = moved_path(&conflict.path, old, new);
            out.push(ConflictMove {
                source_path: conflict.path.clone(),
                destination_key: draft_conflict_key(&destination_path, &conflict.conflict_id),
                destination_path,
                conflict,
                object,
                destination_exists: false,
            });
        }
        Ok(out)
    }

    /// Write and verify every moved conflict copy whose destination is absent.
    fn copy_conflicts(&mut self, plan: &[ConflictMove]) -> Result<(), Box<RemapError>> {
        for item in plan {
            if !item.moved() || item.destination_exists {
                continue;
            }
            let at = err_at(&item.source_path, &item.destination_path);
            let destination = DraftConflict {
                path: item.destination_path.clone(),
                ..item.conflict.clone()
            };
            let mut object = item.object.clone();
            codec::set(
                &mut object,
                "path",
                Json::String(item.destination_path.clone()),
            );
            self.set(&item.destination_key, &codec::stringify_object(&object))
                .map_err(&at)?;

            let raw = self.get(&item.destination_key).map_err(&at)?;
            let verification = parse_or_null(raw.as_deref()).map_err(&at)?;
            if !DraftConflict::from_json(&verification)
                .is_some_and(|v| same_draft_conflict(&v, &destination))
            {
                return Err(at(storage_error(format!(
                    "Unable to verify the recovery copy at {}",
                    item.destination_path
                ))));
            }
        }
        Ok(())
    }

    /// `removeDraftIfRevision` for a moved source, failing if it changed.
    fn remove_moved_draft(&mut self, item: &DraftMove) -> Result<(), Box<RemapError>> {
        let at = err_at(&item.source_path, &item.destination_path);
        let removed = self
            .remove_draft_if_revision(&item.source_path, &item.draft.draft_revision)
            .map_err(&at)?;
        if !removed {
            return Err(at(storage_error(format!(
                "The local draft at {} changed before it could be moved",
                item.source_path
            ))));
        }
        Ok(())
    }

    /// Remove every moved conflict source, failing if one changed.
    fn remove_conflict_sources(&mut self, plan: &[ConflictMove]) -> Result<(), Box<RemapError>> {
        for item in plan {
            if !item.moved() {
                continue;
            }
            let at = err_at(&item.source_path, &item.destination_path);
            let removed = self
                .remove_draft_conflict_if_revision(
                    &item.source_path,
                    &item.conflict.conflict_id,
                    &item.conflict.draft_revision,
                )
                .map_err(&at)?;
            if !removed {
                return Err(at(storage_error(format!(
                    "The recovery copy at {} changed before it could be moved",
                    item.source_path
                ))));
            }
        }
        Ok(())
    }

    /// `copyLegacyBackups(storage, plan)`.
    fn copy_legacy_backups(&mut self, plan: &[BackupMove]) -> Result<(), Box<RemapError>> {
        for item in plan {
            if item.source_path == item.destination_path {
                continue;
            }
            let at = err_at(&item.source_path, &item.destination_path);
            let source_raw = self.get(&item.source_key).map_err(&at)?;
            let destination_raw = self.get(&item.destination_key).map_err(&at)?;
            if source_raw.as_deref() != Some(item.raw.as_str()) {
                return Err(at(storage_error(format!(
                    "The legacy backup at {} changed while it was being moved",
                    item.source_path
                ))));
            }
            match destination_raw {
                Some(raw) if raw != item.raw => return Err(backup_collision(item)),
                Some(_) => {}
                None => self.set(&item.destination_key, &item.raw).map_err(&at)?,
            }
            let verification = self.get(&item.destination_key).map_err(&at)?;
            if verification.as_deref() != Some(item.raw.as_str()) {
                return Err(at(storage_error(format!(
                    "Unable to verify the legacy backup at {}",
                    item.destination_path
                ))));
            }
        }
        Ok(())
    }

    /// `removeLegacyBackupSources(storage, plan)`.
    fn remove_legacy_backup_sources(&mut self, plan: &[BackupMove]) -> Result<(), Box<RemapError>> {
        for item in plan {
            if item.source_path == item.destination_path {
                continue;
            }
            let at = err_at(&item.source_path, &item.destination_path);
            let Some(source_raw) = self.get(&item.source_key).map_err(&at)? else {
                continue;
            };
            if source_raw != item.raw {
                return Err(at(storage_error(format!(
                    "The legacy backup at {} changed before it could be moved",
                    item.source_path
                ))));
            }
            self.remove(&item.source_key).map_err(&at)?;
            if self.get(&item.source_key).map_err(&at)?.is_some() {
                return Err(at(storage_error(format!(
                    "Unable to remove the legacy backup at {}",
                    item.source_path
                ))));
            }
        }
        Ok(())
    }

    /// Remove drafts, legacy backups and conflict copies for a deleted item
    /// (path-boundary match). Continues past individual failures and reports
    /// the first one. `Ok(removed)`.
    pub fn remove_drafts(
        &mut self,
        path: &str,
        kind: ItemKind,
    ) -> Result<usize, RemoveDraftsError> {
        if path.is_empty() {
            return Err(RemoveDraftsError {
                error: type_error("Draft cleanup requires a path"),
                removed: None,
            });
        }
        self.provider().map_err(|error| RemoveDraftsError {
            error,
            removed: None,
        })?;
        let enumerate = |store: &Self| -> Result<_, DraftError> {
            Ok((
                store.keys_with_prefix(DRAFT_PREFIX)?,
                store.keys_with_prefix(BACKUP_PREFIX)?,
                store.keys_with_prefix(DRAFT_CONFLICT_PREFIX)?,
            ))
        };
        let (draft_keys, backup_keys, conflict_keys) =
            enumerate(self).map_err(|error| RemoveDraftsError {
                error,
                removed: Some(0),
            })?;

        let mut first_error: Option<DraftError> = None;
        let mut removed = 0;
        let mut remove_counted =
            |store: &mut Self, key: &str, first_error: &mut Option<DraftError>| match store
                .remove(key)
            {
                Ok(()) => removed += 1,
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            };

        for key in &draft_keys {
            let raw_path = key.get(DRAFT_PREFIX.len()..).unwrap_or_default();
            let Some(key_path) = decode_key_path(raw_path) else {
                continue;
            };
            if path_matches(&key_path, path, kind) {
                remove_counted(self, key, &mut first_error);
            }
        }

        for key in &backup_keys {
            let backup_path = key.get(BACKUP_PREFIX.len()..).unwrap_or_default();
            if path_matches(backup_path, path, kind) {
                remove_counted(self, key, &mut first_error);
            }
        }

        for key in &conflict_keys {
            let parsed = self.get(key).and_then(|raw| parse_or_null(raw.as_deref()));
            let conflict = match parsed {
                Ok(value) => DraftConflict::from_json(&value),
                Err(e) => {
                    first_error.get_or_insert(e);
                    continue;
                }
            };
            if conflict.is_some_and(|c| path_matches(&c.path, path, kind)) {
                remove_counted(self, key, &mut first_error);
            }
        }

        match first_error {
            Some(error) => Err(RemoveDraftsError {
                error,
                removed: Some(removed),
            }),
            None => Ok(removed),
        }
    }

    /// Valid conflict copies whose key and `path` both belong to `path`,
    /// sorted by `updatedAt` ascending.
    pub fn list_draft_conflicts(&mut self, path: &str) -> Result<Vec<DraftConflict>, DraftError> {
        self.provider()?;
        let mut conflicts = Vec::new();
        for key in self.keys_with_prefix(&draft_conflict_path_prefix(path))? {
            let Some(raw) = self.get(&key)? else { continue };
            let Some(conflict) = codec::parse(&raw).and_then(|v| DraftConflict::from_json(&v))
            else {
                continue;
            };
            if conflict.path == path && key == draft_conflict_key(path, &conflict.conflict_id) {
                conflicts.push(conflict);
            }
        }
        // Stable, like Array.prototype.sort; values are finite.
        conflicts.sort_by(|a, b| a.updated_at.total_cmp(&b.updated_at));
        Ok(conflicts)
    }

    /// Store `draft` as a verified recovery copy at `path` with a fresh conflict id.
    pub fn preserve_draft_conflict(
        &mut self,
        path: &str,
        draft: &DraftRecord,
    ) -> Result<DraftConflict, DraftError> {
        if !draft.is_valid() || path.is_empty() {
            return Err(type_error(
                "A draft conflict requires a valid draft and destination path",
            ));
        }
        let conflict = DraftConflict {
            path: path.to_owned(),
            source_path: draft.path.clone(),
            conflict_id: self.env.new_revision(),
            content: draft.content.clone(),
            base_revision: draft.base_revision.clone(),
            draft_revision: draft.draft_revision.clone(),
            updated_at: draft.updated_at,
        };
        self.provider()?;
        let key = draft_conflict_key(path, &conflict.conflict_id);
        self.set(&key, &codec::stringify_object(&conflict.to_object()))?;
        let saved = parse_or_null(self.get(&key)?.as_deref())?;
        if !DraftConflict::from_json(&saved).is_some_and(|s| s.conflict_id == conflict.conflict_id)
        {
            return Err(storage_error(format!(
                "Unable to verify the recovery copy for {path}"
            )));
        }
        Ok(conflict)
    }

    /// Remove a conflict copy only if its `draftRevision` matches. `Ok(removed)`.
    pub fn remove_draft_conflict_if_revision(
        &mut self,
        path: &str,
        conflict_id: &str,
        draft_revision: &str,
    ) -> Result<bool, DraftError> {
        self.provider()?;
        let key = draft_conflict_key(path, conflict_id);
        let Some(raw) = self.get(&key)? else {
            return Ok(false);
        };
        let matches = codec::parse(&raw)
            .and_then(|v| DraftConflict::from_json(&v))
            .is_some_and(|c| c.draft_revision == draft_revision);
        if !matches {
            return Ok(false);
        }
        self.remove(&key)?;
        Ok(true)
    }

    /// All valid drafts stored under their canonical key, in backend key order.
    pub fn list_drafts(&mut self) -> Result<Vec<DraftRecord>, DraftError> {
        Ok(self
            .list_draft_entries()?
            .into_iter()
            .map(|(draft, _)| draft)
            .collect())
    }

    /// `listDrafts()`, also returning each record's object as read.
    fn list_draft_entries(&self) -> Result<Vec<(DraftRecord, Object)>, DraftError> {
        self.provider()?;
        let mut drafts = Vec::new();
        for key in self.keys_with_prefix(DRAFT_PREFIX)? {
            let Some(raw) = self.get(&key)? else { continue };
            let Some(value) = codec::parse(&raw) else {
                continue;
            };
            let Some(path) = value
                .as_object()
                .and_then(|o| non_empty(codec::get_str(o, "path")))
            else {
                continue;
            };
            let Some(draft) = DraftRecord::from_json(&value, path) else {
                continue;
            };
            if key != draft_key(&draft.path) {
                continue;
            }
            if let Json::Object(object) = value {
                drafts.push((draft, object));
            }
        }
        Ok(drafts)
    }

    /// Remove every `frankmd:draft:` and `frankmd:draft-conflict:` key (other keys untouched).
    pub fn remove_all_drafts(&mut self) -> Result<(), DraftError> {
        self.provider()?;
        for key in self.keys_with_prefix(DRAFT_PREFIX)? {
            self.remove(&key)?;
        }
        for key in self.keys_with_prefix(DRAFT_CONFLICT_PREFIX)? {
            self.remove(&key)?;
        }
        Ok(())
    }

    /// Read the legacy backup for `path` (malformed → removed, `malformed: true`).
    pub fn read_backup(&mut self, path: &str) -> Result<ReadBackup, DraftError> {
        self.provider()?;
        let key = backup_key(path);
        let Some(raw) = self.get(&key)? else {
            return Ok(ReadBackup {
                backup: None,
                malformed: false,
            });
        };
        match codec::parse(&raw).and_then(|v| LegacyBackup::from_json(&v)) {
            Some(backup) => Ok(ReadBackup {
                backup: Some(backup),
                malformed: false,
            }),
            None => {
                self.remove(&key)?;
                Ok(ReadBackup {
                    backup: None,
                    malformed: true,
                })
            }
        }
    }

    /// Write a legacy backup `{content, timestamp: now}`.
    pub fn write_backup(&mut self, path: &str, content: &str) -> Result<(), DraftError> {
        self.provider()?;
        let record: Object = vec![
            ("content".into(), Json::String(content.to_owned())),
            ("timestamp".into(), Json::Number(self.env.now_ms())),
        ];
        self.set(&backup_key(path), &codec::stringify_object(&record))
    }

    /// Remove the legacy backup for `path`.
    pub fn remove_backup(&mut self, path: &str) -> Result<(), DraftError> {
        self.provider()?;
        self.remove(&backup_key(path))
    }

    /// Remove the legacy backup only if it is exactly this snapshot. `Ok(removed)`.
    pub fn remove_backup_if_snapshot(
        &mut self,
        path: &str,
        content: &str,
        timestamp: f64,
    ) -> Result<bool, DraftError> {
        self.provider()?;
        let key = backup_key(path);
        let Some(raw) = self.get(&key)? else {
            return Ok(false);
        };
        let matches = codec::parse(&raw)
            .and_then(|v| LegacyBackup::from_json(&v))
            .is_some_and(|b| b.content == content && b.timestamp == timestamp);
        if !matches {
            return Ok(false);
        }
        self.remove(&key)?;
        Ok(true)
    }

    /// Remove every `frankmd:backup:` key.
    pub fn remove_all_backups(&mut self) -> Result<(), DraftError> {
        self.provider()?;
        for key in self.keys_with_prefix(BACKUP_PREFIX)? {
            self.remove(&key)?;
        }
        Ok(())
    }
}

/// `{ok:false, backupCollision:true, …}` for a legacy backup move.
fn backup_collision(item: &BackupMove) -> Box<RemapError> {
    Box::new(RemapError {
        backup_collision: true,
        ..RemapError::at(
            storage_error(format!(
                "A legacy backup already exists at {}",
                item.destination_path
            )),
            &item.source_path,
            &item.destination_path,
        )
    })
}

/// Legacy offline-backup facade (port of `offline_backup_controller.js`):
/// failures are logged (warn) and swallowed.
impl<B: DraftBackend> DraftStorage<B> {
    /// `save(path, content)`: write a backup; failure → warning only.
    pub fn backup_save(&mut self, path: &str, content: &str) -> Result<(), DraftError> {
        self.write_backup(path, content)
    }

    /// `check(path, serverContent)`: no/corrupt backup → `None`; equal to the
    /// server content → removed and `None`; otherwise the backup.
    pub fn backup_check(&mut self, path: &str, server_content: &str) -> Option<LegacyBackup> {
        let backup = self.read_backup(path).ok()?.backup?;
        if backup.content == server_content {
            // `clear(path)`: a failed removal is only a warning in the original.
            let _ = self.remove_backup(path);
            return None;
        }
        Some(backup)
    }
}
