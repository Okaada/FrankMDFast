//! # frankmd-net
//!
//! **Every outbound request goes through this crate (HP-010).**
//! `reqwest::Client` is a disallowed type everywhere else (`clippy.toml`).
//!
//! - [`runtime::NetRuntime`] owns the process's single multi-thread tokio
//!   runtime. GPUI has its own executor and does not run tokio futures; the
//!   app bridges with `runtime.spawn(fut)` + awaiting the join handle from a
//!   GPUI background task (docs/architecture.md §Threading).
//! - [`client::TrustedClient`]: configured API endpoints (AI providers, S3,
//!   YouTube, Google CSE). Still scheme-checked, size/time-capped, no proxy.
//! - [`client::EgressClient`]: user-supplied URLs (external image import, web
//!   image search results). DNS resolution goes through [`egress::GuardedResolver`],
//!   which rejects any non-public address, so reqwest connects only to vetted
//!   IPs (no DNS rebinding); redirects are re-validated per hop; `.no_proxy()`.
#![allow(clippy::disallowed_types)] // this crate is the sanctioned owner of reqwest::Client
#![allow(unused_variables, dead_code)] // skeleton

pub mod client;
pub mod egress;
pub mod runtime;

pub use client::{EgressClient, Fetched, Limits, NetError, TrustedClient};
pub use egress::{HostResolver, StaticResolver, SystemResolver};
pub use runtime::NetRuntime;
