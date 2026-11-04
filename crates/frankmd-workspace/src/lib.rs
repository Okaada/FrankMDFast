//! # frankmd-workspace
//!
//! **The single authoritative state (HP-002).** The GPUI layer holds one
//! `Entity<WorkspaceModel>` wrapping a [`Workspace`]; views render from
//! [`Snapshot`]s and send [`Command`]s. No view keeps a private copy that
//! needs "refreshing".
//!
//! - All mutations are commands, handled one at a time (a serialized queue,
//!   HP-003/005); each returns [`Event`]s that views subscribe to.
//! - Long work (tree rescans, renders, searches, network) is *requested* via
//!   [`Effect`]s that the app runs off-thread; results come back as commands
//!   tagged with the [`Generation`] they were computed from and are dropped
//!   if stale.
//! - Time is injected ([`clock::Clock`]) so autosave/debounce logic is tested
//!   without sleeping (HP-026).
//! - Drafts, autosave timers and acknowledged revisions are keyed by `DocId`
//!   ([`drafts`]); a rename never touches draft storage.
//!
//! GUI-free view models for the app shell live here too: [`layout`],
//! [`focus`], [`overlay`], [`commands`] (the one registry the keymap and the
//! palette come from), [`quick_open`], [`banners`] and [`status`].

pub mod autosave;
pub mod banners;
pub mod clock;
pub mod command;
pub mod commands;
pub mod doc_index;
pub mod draft_storage;
pub mod drafts;
pub mod focus;
pub mod layout;
pub mod menu_bounds;
pub mod overlay;
pub mod quick_open;
pub mod shortcuts;
pub mod startup;
pub mod status;
pub mod store;

pub use command::{Command, Effect, Event};
pub use frankmd_core::Generation;
pub use store::{Snapshot, Workspace};
