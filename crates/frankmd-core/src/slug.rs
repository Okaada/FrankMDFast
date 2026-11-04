//! **[COMPAT]** hand port of `HugoService.slugify` and its `ACCENT_MAP`
//! (`app/services/hugo_service.rb`). Golden: `tests/fixtures/golden/hugo.json`
//! (`slugify`, `constants.ACCENT_MAP`).

/// **[COMPAT]** `HugoService::ACCENT_MAP`, in source order: single precomposed
/// lowercase code points only (no NFD decomposition).
pub const ACCENT_MAP: &[(char, &str)] = &[
    ('à', "a"),
    ('á', "a"),
    ('â', "a"),
    ('ã', "a"),
    ('ä', "a"),
    ('å', "a"),
    ('æ', "ae"),
    ('ç', "c"),
    ('č', "c"),
    ('ć', "c"),
    ('è', "e"),
    ('é', "e"),
    ('ê', "e"),
    ('ë', "e"),
    ('ě', "e"),
    ('ì', "i"),
    ('í', "i"),
    ('î', "i"),
    ('ï', "i"),
    ('ð', "d"),
    ('ď', "d"),
    ('ñ', "n"),
    ('ň', "n"),
    ('ò', "o"),
    ('ó', "o"),
    ('ô', "o"),
    ('õ', "o"),
    ('ö', "o"),
    ('ø', "o"),
    ('ù', "u"),
    ('ú', "u"),
    ('û', "u"),
    ('ü', "u"),
    ('ů', "u"),
    ('ý', "y"),
    ('ÿ', "y"),
    ('ž', "z"),
    ('ź', "z"),
    ('ż', "z"),
    ('ß', "ss"),
    ('þ', "th"),
    ('š', "s"),
    ('ś', "s"),
    ('ř', "r"),
    ('ł', "l"),
];

/// Slugify a title exactly like the Ruby implementation: Ruby `downcase`
/// (full Unicode), map each char through [`ACCENT_MAP`], `[^a-z0-9]+` → `-`,
/// trim `-`, collapse `-` runs.
pub fn slugify(title: &str) -> String {
    let mut mapped = String::with_capacity(title.len());
    for c in title.to_lowercase().chars() {
        match accent(c) {
            Some(ascii) => mapped.push_str(ascii),
            None => mapped.push(c),
        }
    }
    collapse_to_slug(&mapped)
}

/// The ASCII replacement of `c` from [`ACCENT_MAP`], if any.
pub fn accent(c: char) -> Option<&'static str> {
    ACCENT_MAP.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)
}

/// Replace every run of characters outside `[a-z0-9]` by one `-` and trim
/// `-` at both ends (the regex tail shared by the Ruby and JS slugifiers).
pub(crate) fn collapse_to_slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_dash = false;
    for c in text.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    out
}
