//! # frankmd-vim
//!
//! A modal-editing state machine that knows nothing about GPUI. The app crate
//! implements [`VimBuffer`] over gpui-kit's `EditorState` and feeds it
//! keystrokes intercepted *before* the editor (HP-018: vim layer has the
//! highest precedence; app Ctrl-shortcuts listed in [`keymap::PASSTHROUGH`]
//! are never swallowed on non-macOS, **[COMPAT]** `vim_mode.js`).
//!
//! Scope = what FrankMD users get from `@replit/codemirror-vim` today
//! (frontend.md A.9): normal/insert/visual(char,line,block)/replace modes,
//! counts, operators `d c y > < g~ gu gU` × motions/text objects, registers,
//! `.` repeat, `u`/`Ctrl-R`, `/ ? n N * #`, marks, `:s`, and FrankMD ex
//! commands (`:w :q :wq :x :help :e :find :Explore :n :prev`).
#![allow(unused_variables, dead_code)] // skeleton

pub mod buffer;
pub mod ex;
pub mod keymap;
pub mod state;

pub use buffer::VimBuffer;
pub use ex::{AppCommand, ExCall};
pub use state::{Mode, Outcome, VimState};
