//! GUI-free view models of the media dialogs (ui-spec §5.6, §5.7, §5.15).
//!
//! The GPUI views in `frankmd-app/src/dialogs/media` render from these and
//! run the [`Effect`](picker::PickerEffect)s they return (uploads, searches,
//! file choosers) on the tokio bridge; results come back through the
//! `*_result` methods. Nothing here does IO (docs/testing/ui-test-plan.md D-2),
//! and every user-visible string is a [`Text`] (an i18n key with arguments, or
//! a server message verbatim), resolved by the view (D-3).
//!
//! - [`markup`]: the exact Markdown/HTML each dialog inserts (**[COMPAT]**,
//!   byte for byte with FrankMD's `app_controller.js` / `video_dialog_controller.js`).
//! - [`storage`]: the shared "Storage for new media" state and the per-dialog storage row.
//! - [`picker`]: the Insert image dialog.
//! - [`video`]: the Insert video dialog.
//! - [`library`]: the media library workspace.

pub mod library;
pub mod markup;
pub mod picker;
pub mod storage;
mod text;
pub mod video;

pub use text::Text;
