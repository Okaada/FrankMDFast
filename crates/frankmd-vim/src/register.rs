//! Registers: unnamed `"`, yank `0`, numbered `1`-`9`, small delete `-`,
//! named `a`-`z` (uppercase appends) and black hole `_`. `+`/`*` behave as
//! ordinary named registers (the system clipboard belongs to the app).

use std::collections::HashMap;

/// How register text is put back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RegKind {
    /// Characterwise.
    #[default]
    Chars,
    /// Whole lines; the text ends with `\n`.
    Lines,
    /// Blockwise; one `\n`-separated piece per line.
    Block,
}

/// One register's content.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RegContent {
    pub(crate) text: String,
    pub(crate) kind: RegKind,
}

/// All registers.
#[derive(Debug, Default)]
pub(crate) struct Registers {
    map: HashMap<char, RegContent>,
}

/// Whether a register name is writable/readable by this engine.
pub(crate) fn is_valid_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '"' | '-' | '_' | '+' | '*')
}

impl Registers {
    /// Store the result of a yank (`yank = true`) or delete/change.
    pub(crate) fn store(&mut self, name: Option<char>, content: RegContent, yank: bool) {
        match name {
            Some('_') => return,
            Some(c) if c.is_ascii_uppercase() => {
                let lower = c.to_ascii_lowercase();
                let entry = self.map.entry(lower).or_default();
                if content.kind == RegKind::Lines
                    && entry.kind != RegKind::Lines
                    && !entry.text.is_empty()
                {
                    entry.text.push('\n');
                }
                entry.text.push_str(&content.text);
                if content.kind == RegKind::Lines {
                    entry.kind = RegKind::Lines;
                }
                let merged = entry.clone();
                self.map.insert('"', merged);
                return;
            }
            Some(c) if c != '"' => {
                self.map.insert(c, content.clone());
            }
            _ => {
                if yank {
                    self.map.insert('0', content.clone());
                } else if content.kind == RegKind::Lines || content.text.contains('\n') {
                    for n in (1..9u8).rev() {
                        let from = char::from(b'0' + n);
                        if let Some(v) = self.map.remove(&from) {
                            self.map.insert(char::from(b'0' + n + 1), v);
                        }
                    }
                    self.map.insert('1', content.clone());
                } else {
                    self.map.insert('-', content.clone());
                }
            }
        }
        self.map.insert('"', content);
    }

    /// Read a register (`None` = unnamed).
    pub(crate) fn get(&self, name: Option<char>) -> Option<&RegContent> {
        let key = name.map_or('"', |c| c.to_ascii_lowercase());
        self.map.get(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(t: &str) -> RegContent {
        RegContent {
            text: t.into(),
            kind: RegKind::Chars,
        }
    }

    #[test]
    fn named_and_append() {
        let mut r = Registers::default();
        r.store(Some('a'), chars("x"), true);
        r.store(Some('A'), chars("y"), true);
        assert_eq!(r.get(Some('a')).map(|c| c.text.as_str()), Some("xy"));
        assert_eq!(r.get(None).map(|c| c.text.as_str()), Some("xy"));
        r.store(Some('_'), chars("z"), false);
        assert_eq!(r.get(None).map(|c| c.text.as_str()), Some("xy"));
    }

    #[test]
    fn yank_and_delete_registers() {
        let mut r = Registers::default();
        r.store(None, chars("y"), true);
        r.store(
            None,
            RegContent {
                text: "l\n".into(),
                kind: RegKind::Lines,
            },
            false,
        );
        r.store(None, chars("s"), false);
        assert_eq!(r.get(Some('0')).map(|c| c.text.as_str()), Some("y"));
        assert_eq!(r.get(Some('1')).map(|c| c.text.as_str()), Some("l\n"));
        assert_eq!(r.get(Some('-')).map(|c| c.text.as_str()), Some("s"));
        assert_eq!(r.get(None).map(|c| c.text.as_str()), Some("s"));
    }
}
