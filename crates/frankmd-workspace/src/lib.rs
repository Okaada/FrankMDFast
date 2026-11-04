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
#![allow(unused_variables, dead_code)] // skeleton

pub mod autosave;
pub mod clock;
pub mod command;
pub mod draft_storage;
pub mod drafts;
pub mod menu_bounds;
pub mod shortcuts;
pub mod startup;
pub mod store;

pub use command::{Command, Effect, Event};
pub use frankmd_core::Generation;
pub use store::{Snapshot, Workspace};
