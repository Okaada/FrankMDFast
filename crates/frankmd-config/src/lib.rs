//! # frankmd-config
//!
//! **Single source of truth for configuration (HP-013, HP-014).** Every reader
//! in the app goes through [`ResolvedConfig`]; nobody reads `std::env` or
//! `.fed` directly (`std::env::set_var` is banned by `clippy.toml`).
//!
//! Resolution is **exactly** the original's (`app/models/config.rb`, see
//! `docs/inventory/backend.md` §1.3): `.fed` value if the key is *present*
//! (even if empty → `None`, blocking ENV) > ENV if non-blank > schema default,
//! plus the AI "file-only mode" and the image-generation key exception.
//! Drop-in compatibility is a hard requirement: the golden fixtures
//! `config_parse.json`, `config_env.json`, `config_writeback.json` and
//! `config_template.json` are the spec.

pub mod dirs;
pub mod env;
pub mod error;
pub mod fed;
pub mod resolve;
pub mod schema;
pub mod store;
mod uri;
pub mod value;

pub use env::EnvSnapshot;
pub use error::ConfigError;
pub use resolve::{AiProviderId, ConfigEntry, Feature, Provenance, Resolved, ResolvedConfig};
pub use schema::{Key, KeySpec, KeyType};
pub use store::ConfigStore;
pub use value::Value;
