//! Name of the sibling copy the Recovery dialog's "Keep both" writes
//! (ui-spec §5.11): `<name> (draft 2026-11-04 14-02).md` next to the note.

use frankmd_core::RelPath;

/// Path of the copy for `note`, with the draft's time (epoch ms) in `zone`.
/// `attempt` 1 is the plain name; later attempts add ` 2`, ` 3`, … before
/// `.md` so an existing file is never overwritten. `None` for an invalid
/// timestamp or a path that can't hold a sibling.
pub fn copy_path(
    note: &RelPath,
    backup_ms: f64,
    zone: &jiff::tz::TimeZone,
    attempt: u32,
) -> Option<RelPath> {
    if !backup_ms.is_finite() {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let at = jiff::Timestamp::from_millisecond(backup_ms as i64)
        .ok()?
        .to_zoned(zone.clone());
    let name = note.file_name()?;
    let stem = name.strip_suffix(".md").unwrap_or(name);
    let suffix = if attempt <= 1 {
        String::new()
    } else {
        format!(" {attempt}")
    };
    let file = format!(
        "{stem} (draft {}){suffix}.md",
        at.strftime("%Y-%m-%d %H-%M")
    );
    let full = match note.parent().filter(|p| !p.is_root()) {
        Some(parent) => format!("{}/{file}", parent.as_str()),
        None => file,
    };
    RelPath::parse(&full).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RelPath {
        RelPath::parse(s).expect("path")
    }

    #[test]
    fn names_the_copy_after_the_draft_time() {
        let zone = jiff::tz::TimeZone::UTC;
        // 2026-11-04T14:02:00Z
        let ms = 1_793_800_920_000.0;
        assert_eq!(
            copy_path(&p("blog/Project brief.md"), ms, &zone, 1).map(|r| r.as_str().to_owned()),
            Some("blog/Project brief (draft 2026-11-04 14-02).md".to_owned())
        );
        assert_eq!(
            copy_path(&p("Inbox.md"), ms, &zone, 3).map(|r| r.as_str().to_owned()),
            Some("Inbox (draft 2026-11-04 14-02) 3.md".to_owned())
        );
        assert_eq!(copy_path(&p("Inbox.md"), f64::NAN, &zone, 1), None);
    }
}
