//! Shared test support for frankmd-app UI tests (docs/testing/ui-test-plan.md).
//!
//! - [`harness`]: open headless windows, drive keys/actions, query elements.
//! - [`actions`]: the app's GPUI actions and registry-generated keymap.
//! - [`vm`]: the GUI-free view models (re-exported from frankmd-workspace).
//! - [`shell`]: the real app shell over a temporary vault.
//!
//! Every integration-test file includes this with `mod support;`, so items a
//! given file doesn't use are expected.
#![allow(dead_code, unreachable_pub, missing_docs, unused_imports)]

pub mod actions;
pub mod harness;
pub mod shell;
pub mod vm;
