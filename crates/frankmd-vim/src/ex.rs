//! FrankMD's ex commands (**[COMPAT]** `vim_mode.js` `EX_COMMANDS`, `nextNoteIndex`).

/// App-level commands reachable from ex mode. They dispatch the *same*
/// actions as the keyboard shortcuts (one registry, HP-018).
///
/// Variant ↔ original logical name: see [`AppCommand::logical_name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppCommand {
    /// `"save"` (`:w`, `:write`).
    Save,
    /// `"close"` (`:q`, `:quit`). Closes dialogs/menus/Library — **not** the note (**[COMPAT]**).
    CloseDialogs,
    /// `"save-and-close"` (`:wq`, `:x`, `:xit`).
    SaveAndClose,
    /// `"help"` (`:help`).
    Help,
    /// `"finder"` (`:e`, `:edit`, `:find`).
    OpenFinder,
    /// `"toggle-sidebar"` (`:Explore`).
    ToggleSidebar,
    /// `"next-note"` (`:n`, `:next`).
    NextNote,
    /// `"prev-note"` (`:prev`).
    PrevNote,
}

impl AppCommand {
    /// The original logical command name passed to `onCommand`.
    pub fn logical_name(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::CloseDialogs => "close",
            Self::SaveAndClose => "save-and-close",
            Self::Help => "help",
            Self::OpenFinder => "finder",
            Self::ToggleSidebar => "toggle-sidebar",
            Self::NextNote => "next-note",
            Self::PrevNote => "prev-note",
        }
    }

    /// Keyboard-shortcut action ids (`frankmd_workspace::shortcuts`) this command runs, in
    /// order (`app_controller.js#onVimCommand`): save → `["save"]`, close →
    /// `["closeDialogs"]`, save-and-close → `["save", "closeDialogs"]`, finder →
    /// `["fileFinder"]`, toggle-sidebar → `["toggleSidebar"]`, help → `["help"]`.
    /// Next/prev note are tree navigation (see [`next_note_index`]) → `[]`.
    pub fn shortcut_actions(self) -> &'static [&'static str] {
        match self {
            Self::Save => &["save"],
            Self::CloseDialogs => &["closeDialogs"],
            Self::SaveAndClose => &["save", "closeDialogs"],
            Self::Help => &["help"],
            Self::OpenFinder => &["fileFinder"],
            Self::ToggleSidebar => &["toggleSidebar"],
            Self::NextNote | Self::PrevNote => &[],
        }
    }
}

/// One ex-command definition: `(full name, shortest accepted prefix, command)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExCommand {
    /// Full name (`write`).
    pub name: &'static str,
    /// Shortest accepted abbreviation (`w`).
    pub prefix: &'static str,
    /// What it does.
    pub command: AppCommand,
}

/// **[COMPAT]** the table, verbatim and in order. `:noh`/`:nohlsearch` is intentionally
/// absent — the engine implements it natively.
pub const EX_COMMANDS: [ExCommand; 10] = [
    ExCommand {
        name: "write",
        prefix: "w",
        command: AppCommand::Save,
    },
    ExCommand {
        name: "quit",
        prefix: "q",
        command: AppCommand::CloseDialogs,
    },
    ExCommand {
        name: "wq",
        prefix: "wq",
        command: AppCommand::SaveAndClose,
    },
    ExCommand {
        name: "xit",
        prefix: "x",
        command: AppCommand::SaveAndClose,
    },
    ExCommand {
        name: "help",
        prefix: "help",
        command: AppCommand::Help,
    },
    ExCommand {
        name: "edit",
        prefix: "e",
        command: AppCommand::OpenFinder,
    },
    ExCommand {
        name: "find",
        prefix: "find",
        command: AppCommand::OpenFinder,
    },
    ExCommand {
        name: "Explore",
        prefix: "Explore",
        command: AppCommand::ToggleSidebar,
    },
    ExCommand {
        name: "next",
        prefix: "n",
        command: AppCommand::NextNote,
    },
    ExCommand {
        name: "prev",
        prefix: "prev",
        command: AppCommand::PrevNote,
    },
];

/// A parsed app ex command with its whitespace-split arguments (`:e notes/a.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExCall {
    /// Command.
    pub command: AppCommand,
    /// Arguments after the command word (`params.args || []`).
    pub args: Vec<String>,
}

/// Parse an ex line (without the leading `:`). A word `c` selects the entry whose
/// `name.starts_with(c) && c.starts_with(prefix)` (vim abbreviation rule, case-sensitive).
/// Returns `None` for commands handled natively by the engine (`:s`, `:noh`, `:<n>`, ...)
/// or unknown ones.
pub fn parse(line: &str) -> Option<ExCall> {
    let line = line.trim_start().trim_start_matches(':').trim_start();
    // Like codemirror-vim, the command name is the leading run of word chars.
    let name_len = line
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(line.len());
    let (word, rest) = line.split_at(name_len);
    if word.is_empty() {
        return None;
    }
    let entry = EX_COMMANDS
        .iter()
        .find(|c| c.name.starts_with(word) && word.starts_with(c.prefix))?;
    let args = rest.split_whitespace().map(str::to_owned).collect();
    Some(ExCall {
        command: entry.command,
        args,
    })
}

/// `nextNoteIndex` for `:n` / `:prev`: wraps around; with nothing selected starts at the
/// first note (forward) or last (backward). `None` when there are no notes.
pub fn next_note_index(count: usize, current: Option<usize>, forward: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match current {
        None if forward => 0,
        None => count - 1,
        Some(i) if forward => (i % count + 1) % count,
        Some(i) => (i % count + count - 1) % count,
    })
}
