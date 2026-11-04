//! Back/forward note history (ui-spec §6.1) and the Ctrl+Tab recent-note
//! switcher (§5.2).

const MRU_CAP: usize = 50;

fn remap_path(path: &str, from: &str, to: &str) -> Option<String> {
    if path == from {
        return Some(to.to_owned());
    }
    let rest = path.strip_prefix(from)?.strip_prefix('/')?;
    Some(format!("{to}/{rest}"))
}

fn is_under(path: &str, root: &str) -> bool {
    remap_path(path, root, "").is_some()
}

/// Back/forward note history. `None` entries are the empty state.
#[derive(Debug, Default)]
pub struct History {
    back: Vec<Option<String>>,
    current: Option<String>,
    forward: Vec<Option<String>>,
}

impl History {
    /// Record navigation to `path`; no-op when it equals the current entry.
    pub fn visit(&mut self, path: Option<&str>) {
        if self.current.as_deref() == path {
            return;
        }
        let prev = std::mem::replace(&mut self.current, path.map(str::to_owned));
        self.back.push(prev);
        self.forward.clear();
    }

    /// The current entry.
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Whether Back is available.
    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    /// Whether Forward is available.
    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Step back; returns the new current entry.
    pub fn back(&mut self) -> Option<Option<String>> {
        let target = self.back.pop()?;
        let prev = std::mem::replace(&mut self.current, target.clone());
        self.forward.push(prev);
        Some(target)
    }

    /// Step forward; returns the new current entry.
    pub fn forward(&mut self) -> Option<Option<String>> {
        let target = self.forward.pop()?;
        let prev = std::mem::replace(&mut self.current, target.clone());
        self.back.push(prev);
        Some(target)
    }

    /// Segment-wise prefix remap of every entry.
    pub fn remap(&mut self, from: &str, to: &str) {
        let f = |e: &mut Option<String>| {
            if let Some(n) = e.as_deref().and_then(|p| remap_path(p, from, to)) {
                *e = Some(n);
            }
        };
        self.back.iter_mut().for_each(f);
        self.forward.iter_mut().for_each(f);
        f(&mut self.current);
    }

    /// Drop entries at or under `path`; current under `path` becomes `None`.
    pub fn remove(&mut self, path: &str) {
        let under = |e: &Option<String>| e.as_deref().is_some_and(|p| is_under(p, path));
        self.back.retain(|e| !under(e));
        self.forward.retain(|e| !under(e));
        if under(&self.current) {
            self.current = None;
        }
        self.back.dedup();
        self.forward.dedup();
        // A trailing duplicate of the current entry is not a place to go back to.
        while self.back.last() == Some(&self.current) {
            self.back.pop();
        }
        while self.forward.last() == Some(&self.current) {
            self.forward.pop();
        }
    }
}

/// Ctrl+Tab MRU switcher: notes opened this session, newest first.
#[derive(Debug, Default)]
pub struct RecentSwitcher {
    mru: Vec<String>,
    active: Option<usize>,
}

impl RecentSwitcher {
    /// Move `path` to the front (deduped, capped at 50).
    pub fn touch(&mut self, path: &str) {
        self.mru.retain(|p| p != path);
        self.mru.insert(0, path.to_owned());
        self.mru.truncate(MRU_CAP);
    }

    /// Notes, newest first.
    pub fn mru(&self) -> &[String] {
        &self.mru
    }

    /// Whether the switcher is showing.
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    /// Open the switcher: forward selects the previous note, backward the last.
    pub fn start(&mut self, forward: bool) {
        if self.mru.is_empty() {
            return;
        }
        self.active = Some(if forward {
            1.min(self.mru.len() - 1)
        } else {
            self.mru.len() - 1
        });
    }

    /// Move the selection (wrapping); starts the switcher if inactive.
    pub fn step(&mut self, forward: bool) {
        let n = self.mru.len();
        match self.active {
            None => self.start(forward),
            Some(i) => {
                self.active = Some(if forward {
                    (i + 1) % n
                } else {
                    (i + n - 1) % n
                })
            }
        }
    }

    /// The highlighted note.
    pub fn selected(&self) -> Option<&str> {
        self.mru.get(self.active?).map(String::as_str)
    }

    /// Close and return the selected note.
    pub fn commit(&mut self) -> Option<String> {
        let sel = self.selected().map(str::to_owned);
        self.active = None;
        sel
    }

    /// Close without choosing.
    pub fn cancel(&mut self) {
        self.active = None;
    }

    /// Segment-wise prefix remap (dedupes if a rename collides).
    pub fn remap(&mut self, from: &str, to: &str) {
        let sel = self.selected().map(str::to_owned);
        for p in &mut self.mru {
            if let Some(n) = remap_path(p, from, to) {
                *p = n;
            }
        }
        let mut seen = std::collections::HashSet::new();
        self.mru.retain(|p| seen.insert(p.clone()));
        self.reselect(sel.and_then(|s| remap_path(&s, from, to).or(Some(s))));
    }

    /// Drop notes at or under `path`.
    pub fn remove(&mut self, path: &str) {
        let sel = self.selected().map(str::to_owned);
        self.mru.retain(|p| !is_under(p, path));
        self.reselect(sel);
    }

    fn reselect(&mut self, sel: Option<String>) {
        if self.active.is_none() {
            return;
        }
        self.active = match sel.and_then(|s| self.mru.iter().position(|p| *p == s)) {
            Some(i) => Some(i),
            None if self.mru.is_empty() => None,
            None => Some(self.active.unwrap_or(0).min(self.mru.len() - 1)),
        };
    }
}
