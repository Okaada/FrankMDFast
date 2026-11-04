//! Shared test support for frankmd-app UI tests (docs/testing/ui-test-plan.md).
//!
//! - [`harness`]: open headless windows, drive keys/actions, query elements.
//! - [`actions`]: the proposed GPUI action set and default keymap (ui-spec §6).
//! - [`vm`]: proposed GUI-free view-model types. They are STUBS here
//!   (`todo!()` bodies); the implementation phase moves them into the GUI-free
//!   crates named on each type and these tests switch their imports.
//! - [`shell`]: the app-shell entry point the level-b tests drive. Stubbed
//!   until frankmd-app exposes its root view through a library target.
//!
//! Every integration-test file includes this with `mod support;`, so items a
//! given file doesn't use are expected.
#![allow(dead_code, unreachable_pub, missing_docs, clippy::todo)]

pub mod actions;
pub mod harness;
pub mod shell;
pub mod vm;
