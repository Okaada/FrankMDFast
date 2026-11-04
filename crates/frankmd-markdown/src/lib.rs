//! # frankmd-markdown
//!
//! Parses markdown with comrak and lowers it to a **GUI-free render IR**
//! ([`ir::RenderDoc`]) that the app crate turns into GPUI elements. The IR is
//! the sanitizer: it can only express allowlisted constructs, so there is no
//! HTML string to patch afterwards (HP-009). Every block carries its source
//! line span for sync scroll and task toggling.
//!
//! Also provides [`html::to_html`] for `frankmd render --html` and for the
//! conformance tests against the original marked/DOMPurify pipeline.
#![allow(unused_variables, dead_code)] // skeleton

pub mod embed;
pub mod frontmatter;
pub mod highlight;
pub mod html;
pub mod ir;
pub mod line_numbers;
pub mod linemap;
pub mod math;
pub mod parse;

pub use ir::RenderDoc;
pub use parse::{RenderOptions, parse_inline, render, render_body};
