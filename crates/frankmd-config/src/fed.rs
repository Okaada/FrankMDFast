//! Line-preserving `.fed` text operations.
//!
//! Hand-written because the writer must *uncomment* `# key = default` lines
//! in place and leave every other byte alone (backend.md §1.5–1.8). This is
//! **not** TOML: no inline comments, first `=` splits, last duplicate wins.
//!
//! Everything here is pure `&str → String`; [`crate::store::ConfigStore`]
//! adds locking and atomic writes on top.

use crate::{Key, Value};

/// One parsed `.fed` entry, before casting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    /// Schema key (line key lowercased, then [`Key::from_name`]).
    pub key: Key,
    /// Value after the first `=`, stripped, quotes removed (still uncast).
    pub raw: String,
}

/// Result of [`parse`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parsed {
    /// Present keys in first-insertion order, last duplicate wins (**[COMPAT]**
    /// Ruby hash semantics). `None` = present but nil (blocks ENV).
    pub values: Vec<(Key, Option<Value>)>,
    /// One message per skipped line (bad JSON / invalid custom theme); each
    /// names the key (`Ignoring invalid setting custom_theme`).
    pub warnings: Vec<String>,
}

/// **[COMPAT]** `parse_config` (backend.md §1.5): LF or CRLF; comments,
/// blanks, malformed lines and unknown keys ignored.
pub fn parse(text: &str) -> Parsed {
    todo!("backend.md §1.5")
}

/// Every recognised entry, uncast, in file order (duplicates kept).
pub fn raw_entries(text: &str) -> Vec<RawEntry> {
    todo!()
}

/// **[COMPAT]** `save_keys_without_lock` line rewrite (backend.md §1.8):
/// first matching `# key =` comment is uncommented, every active `key =` line
/// replaced, whitespace-only lines emptied, missing keys appended in `changes`
/// order, output `lines.join("\n") + "\n"` (CRLF → LF).
/// `changes` are already-formatted lines (`key = value`).
pub fn apply_changes(text: &str, changes: &[(Key, String)]) -> String {
    todo!("backend.md §1.8")
}

/// **[COMPAT]** The upgrade condition of `upgrade_config_file` (§1.6): no line
/// contains `# AI/LLM` and no line matches `/^#?\s*KEY\s*=/i` for any of
/// [`Key::AI_UPGRADE_MARKERS`].
pub fn needs_ai_section(text: &str) -> bool {
    todo!()
}

/// **[COMPAT]** `upgrade_config_file`: `Some(new text)` when
/// [`needs_ai_section`]; the original lines (split on `\n`, chomped) get one
/// `""` appended if the last is non-blank, then [`ai_section_lines`], joined
/// with `\n` plus a trailing `\n`. `None` when no upgrade applies.
pub fn upgrade_ai_section(text: &str) -> Option<String> {
    todo!()
}

/// A `TEMPLATE_SECTIONS` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemplateSection {
    /// The section's marker comment (`# UI Settings`, `# AI/LLM (for grammar checking)`, ...).
    pub marker: &'static str,
    /// Its lines (leading `""` included where the original has it).
    pub lines: &'static [&'static str],
}

/// **[COMPAT]** `Config::TEMPLATE_SECTIONS`, in order.
pub fn template_sections() -> &'static [TemplateSection] {
    todo!()
}

/// The last template section (the AI/LLM block appended by the upgrade; it
/// starts with `""`).
pub fn ai_section_lines() -> &'static [&'static str] {
    todo!()
}

/// The verbatim auto-generated template (3032 bytes, SHA-256
/// `796513f7…fb154`). Golden: `tests/fixtures/golden/fed_template.fed`.
pub fn template() -> &'static str {
    todo!("embed the golden template byte-for-byte")
}
