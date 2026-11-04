//! # frankmd-search
//!
//! - [`finder`]: Ctrl+P fuzzy file finder (FrankMD's `fuzzyScore` + recency order).
//! - [`content`]: Ctrl+Shift+F full-text search over the vault with the exact
//!   semantics of FrankMD's `NotesService#search_content` (Ruby/Onigmo regex,
//!   case-insensitive, literal fallback, per-line matching, context windows,
//!   5 MiB cap, scrubbed invalid UTF-8, skip-and-report unreadable files —
//!   HP-017/028). Golden: `search.json`.
//! - [`find`]: in-editor find/replace with the original JS semantics
//!   (`find_utils.js`: literal or regex, case toggle, `$&`/`$N` replacement,
//!   zero-length advance). Golden: `js_find_utils.json`.
//!
//! Reading the vault is allowed here (only *writes* are confined to
//! `frankmd-fs`, see clippy.toml); the walker never follows symlinks.

pub mod content;
pub mod find;
pub mod finder;
