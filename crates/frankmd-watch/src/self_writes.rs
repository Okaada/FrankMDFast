//! Tokens for writes FrankMD performs itself, so the watcher drops their echo.

use frankmd_core::Revision;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A [`SelfWrites`] shared between the workspace (which registers tokens)
/// and the vault watcher thread (which consumes them).
pub type SharedSelfWrites = Arc<Mutex<SelfWrites>>;

/// Default lifetime of an unconsumed token.
pub const DEFAULT_TOKEN_TTL: Duration = Duration::from_secs(10);

/// After a token matched, further echoes of the *same* content are still
/// recognized for this long: one write can surface in several debounce
/// batches (create, then modify/close), and only the first consumes it.
pub const SETTLE_GRACE: Duration = Duration::from_secs(2);

/// At most this many tokens are kept; registering more drops the oldest.
pub const MAX_PENDING: usize = 256;

#[derive(Debug)]
struct Token {
    path: PathBuf,
    /// `None` = we are about to remove (or rename away) the path.
    revision: Option<Revision>,
    registered: Instant,
    /// When the token matched; it then lingers for [`SETTLE_GRACE`].
    consumed: Option<Instant>,
}

/// Record of writes we performed, used to drop our own echo events.
///
/// Keys are **vault-relative** paths exactly as the watcher reports them
/// (`PathBuf::from(rel.as_str())`, e.g. `"blog/post.md"` or `".fed"`).
/// A token matches an observed change when the path is equal and the file's
/// current [`Revision`] equals the registered one (or, for removal tokens,
/// the path is gone). A match consumes the token; a consumed token still
/// absorbs repeat echoes of the same content for [`SETTLE_GRACE`] but no
/// longer counts as pending. Unmatched tokens expire
/// after the TTL ([`DEFAULT_TOKEN_TTL`]) and at most [`MAX_PENDING`] are kept,
/// so a write whose echo never arrives cannot hide a later external change
/// for long.
#[derive(Debug)]
pub struct SelfWrites {
    pending: Vec<Token>,
    ttl: Duration,
}

impl Default for SelfWrites {
    fn default() -> Self {
        Self::with_ttl(DEFAULT_TOKEN_TTL)
    }
}

impl SelfWrites {
    /// Empty record with a custom token lifetime.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            pending: Vec::new(),
            ttl,
        }
    }

    /// Empty record wrapped for sharing with the watcher.
    pub fn shared() -> SharedSelfWrites {
        Arc::new(Mutex::new(Self::default()))
    }

    /// Register a write of content with revision `rev` to `path` that we are
    /// about to perform.
    pub fn expect(&mut self, path: PathBuf, rev: Revision) {
        self.push(path, Some(rev), Instant::now());
    }

    /// Register a removal (delete, or the source side of a rename) of `path`
    /// that we are about to perform.
    pub fn expect_removal(&mut self, path: PathBuf) {
        self.push(path, None, Instant::now());
    }

    /// `true` if an observed change is our own write (consumes the token).
    pub fn is_own(&mut self, path: &Path, current: &Revision) -> bool {
        self.take(path, Some(current), Instant::now())
    }

    /// `true` if an observed removal of `path` is our own (consumes the token).
    pub fn is_own_removal(&mut self, path: &Path) -> bool {
        self.take(path, None, Instant::now())
    }

    /// `true` if any live token (including one still in its settle grace)
    /// exists for `path`; lets the watcher skip reading files nobody expects.
    pub fn has_pending(&mut self, path: &Path) -> bool {
        self.prune(Instant::now());
        self.pending.iter().any(|t| t.path == path)
    }

    /// Number of live, not yet consumed tokens.
    pub fn len(&mut self) -> usize {
        self.prune(Instant::now());
        self.pending.iter().filter(|t| t.consumed.is_none()).count()
    }

    /// `true` when no unconsumed token is pending.
    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    fn push(&mut self, path: PathBuf, revision: Option<Revision>, now: Instant) {
        self.prune(now);
        if self.pending.len() >= MAX_PENDING {
            self.pending.remove(0);
        }
        self.pending.push(Token {
            path,
            revision,
            registered: now,
            consumed: None,
        });
    }

    fn take(&mut self, path: &Path, revision: Option<&Revision>, now: Instant) -> bool {
        self.prune(now);
        let found = self
            .pending
            .iter()
            .position(|t| t.path == path && t.revision.as_ref() == revision);
        match found {
            Some(index) => {
                let token = &mut self.pending[index];
                token.consumed.get_or_insert(now);
                true
            }
            None => false,
        }
    }

    fn prune(&mut self, now: Instant) {
        let ttl = self.ttl;
        self.pending.retain(|t| match t.consumed {
            Some(at) => now.saturating_duration_since(at) < SETTLE_GRACE,
            None => now.saturating_duration_since(t.registered) < ttl,
        });
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn rev(s: &str) -> Revision {
        Revision::of(s.as_bytes())
    }

    #[test]
    fn matching_token_is_consumed_once() {
        let mut sw = SelfWrites::default();
        sw.expect("a.md".into(), rev("x"));
        assert!(sw.has_pending(Path::new("a.md")));
        assert!(sw.is_own(Path::new("a.md"), &rev("x")));
        assert!(sw.is_empty());
    }

    #[test]
    fn consumed_token_absorbs_repeat_echoes_only_during_grace() {
        let mut sw = SelfWrites::default();
        let start = Instant::now();
        sw.push("a.md".into(), Some(rev("x")), start);
        assert!(sw.take(Path::new("a.md"), Some(&rev("x")), start));
        let soon = start + SETTLE_GRACE / 2;
        assert!(sw.take(Path::new("a.md"), Some(&rev("x")), soon));
        assert!(!sw.take(Path::new("a.md"), Some(&rev("y")), soon));
        let later = start + SETTLE_GRACE + Duration::from_millis(1);
        assert!(!sw.take(Path::new("a.md"), Some(&rev("x")), later));
    }

    #[test]
    fn different_revision_or_path_is_external() {
        let mut sw = SelfWrites::default();
        sw.expect("a.md".into(), rev("x"));
        assert!(!sw.is_own(Path::new("a.md"), &rev("y")));
        assert!(!sw.is_own(Path::new("b.md"), &rev("x")));
        assert!(!sw.is_own_removal(Path::new("a.md")));
        // The token survives non-matching checks.
        assert!(sw.is_own(Path::new("a.md"), &rev("x")));
    }

    #[test]
    fn removal_tokens() {
        let mut sw = SelfWrites::default();
        sw.expect_removal("old.md".into());
        assert!(!sw.is_own(Path::new("old.md"), &rev("x")));
        assert!(sw.is_own_removal(Path::new("old.md")));
        assert!(sw.is_empty());
    }

    #[test]
    fn tokens_expire_after_ttl() {
        let mut sw = SelfWrites::with_ttl(Duration::from_secs(10));
        let start = Instant::now();
        sw.push("a.md".into(), Some(rev("x")), start);
        assert!(!sw.take(
            Path::new("a.md"),
            Some(&rev("x")),
            start + Duration::from_secs(11)
        ));
        assert!(sw.pending.is_empty());
    }

    #[test]
    fn oldest_token_dropped_beyond_capacity() {
        let mut sw = SelfWrites::default();
        let now = Instant::now();
        for i in 0..=MAX_PENDING {
            sw.push(format!("{i}.md").into(), Some(rev("x")), now);
        }
        assert_eq!(sw.len(), MAX_PENDING);
        assert!(!sw.take(Path::new("0.md"), Some(&rev("x")), now));
        assert!(sw.take(Path::new("1.md"), Some(&rev("x")), now));
    }
}
