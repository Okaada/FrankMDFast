//! Identity and versioning primitives.

use serde::{Deserialize, Serialize};

/// Stable identity of a note for the lifetime of the app's state directory.
///
/// Never derived from the path: renames/moves keep the same `DocId`
/// (HP-001). Persisted in the per-vault index under `$XDG_STATE_HOME`, never
/// inside the notes folder or the note's front matter (compatibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DocId(pub uuid::Uuid);

impl DocId {
    /// Mint a new, time-ordered id (UUIDv7).
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl Default for DocId {
    fn default() -> Self {
        Self::new()
    }
}

/// Content revision: lowercase hex SHA-256 of the exact file bytes.
///
/// **[COMPAT]** identical to Rails `Note#revision` (`SHA256(content).hexdigest`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Revision(pub String);

impl Revision {
    /// Compute the revision of `bytes`.
    pub fn of(bytes: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(bytes);
        let mut hex = String::with_capacity(64);
        for b in digest {
            hex.push_str(&format!("{b:02x}"));
        }
        Self(hex)
    }
}

/// Monotonic generation counter attached to every async result (tree scans,
/// renders, searches, saves). A result whose generation is older than the
/// current one for its subject is discarded (HP-003).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub struct Generation(pub u64);

impl Generation {
    /// The next generation.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
