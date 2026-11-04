//! # frankmd-core
//!
//! Pure domain logic for FrankMD. **No GUI, no network, no async runtime, no
//! filesystem mutation** (mutations live in `frankmd-fs`). Everything here is
//! deterministic and unit-testable against the golden fixtures in
//! `tests/fixtures/golden/`.
//!
//! Key design rules (see `docs/architecture.md`):
//! - A note's identity is a [`DocId`], never its path (HP-001). The path is a
//!   mutable attribute held by exactly one [`registry::DocRegistry`].
//! - Paths that came from the UI or from disk are validated into
//!   [`path::RelPath`] before anything else touches them (HP-008).
//! - Async results carry the [`Generation`] they were computed from so stale
//!   results can be dropped (HP-003).


pub mod clock;
pub mod emoji;
pub mod error;
pub mod guard;
pub mod hugo;
pub mod ids;
pub mod new_note;
pub mod note;
pub mod path;
pub mod recent;
pub mod registry;
pub mod slug;
pub mod text;
pub mod tree;
pub mod tree_view;
pub mod wikilink;

pub use clock::{Clock, FakeClock, SystemClock};
pub use error::CoreError;
pub use ids::{DocId, Generation, Revision};
pub use path::RelPath;
