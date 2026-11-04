//! A single text edit plus the selection to restore afterwards: re-exported from
//! [`frankmd_core::text::edit`], where every GUI-free editor transform lives.

pub use frankmd_core::text::edit::{
    Edit, common_prefix, common_suffix, map_offset, minimal_replacement,
};
