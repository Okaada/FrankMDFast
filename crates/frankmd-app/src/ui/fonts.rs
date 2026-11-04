//! Editor font ids (`.fed` `editor_font`, FrankMD's 9 ids) resolved to
//! installed families through fontconfig (ui-spec §7.2, OM-016).
//!
//! `fc-match` is a process spawn: call [`resolve_editor_font`] off the UI thread.

/// FrankMD's editor fonts: `(id, family)` (`settings_controller.js editorFonts`).
pub const EDITOR_FONTS: [(&str, &str); 9] = [
    ("cascadia-code", "Cascadia Code"),
    ("consolas", "Consolas"),
    ("dejavu-mono", "DejaVu Sans Mono"),
    ("fira-code", "Fira Code"),
    ("hack", "Hack"),
    ("jetbrains-mono", "JetBrains Mono"),
    ("roboto-mono", "Roboto Mono"),
    ("source-code-pro", "Source Code Pro"),
    ("ubuntu-mono", "Ubuntu Mono"),
];

/// Family name of a FrankMD font id.
pub fn family_of(id: &str) -> Option<&'static str> {
    EDITOR_FONTS.iter().find(|(i, _)| *i == id).map(|(_, f)| *f)
}

/// What fontconfig picks for `pattern` (`None` without fontconfig).
fn fc_match(pattern: &str) -> Option<String> {
    let out = std::process::Command::new("fc-match")
        .args(["-f", "%{family[0]}", pattern])
        .output()
        .ok()?;
    let family = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (out.status.success() && !family.is_empty()).then_some(family)
}

/// Pick the family for editor font `id`: the FrankMD font when installed
/// (fontconfig doesn't substitute it), else the system `monospace`.
/// `None` means "use the theme's monospace font".
pub fn resolve_editor_font(id: Option<&str>) -> Option<String> {
    let wanted = id.and_then(family_of);
    if let Some(wanted) = wanted
        && let Some(found) = fc_match(wanted)
        && found.eq_ignore_ascii_case(wanted)
    {
        return Some(found);
    }
    fc_match("monospace")
}

#[cfg(test)]
mod tests {
    use super::family_of;

    #[test]
    fn every_frankmd_font_id_has_a_family() {
        assert_eq!(family_of("jetbrains-mono"), Some("JetBrains Mono"));
        assert_eq!(family_of("comic-sans"), None);
    }
}
