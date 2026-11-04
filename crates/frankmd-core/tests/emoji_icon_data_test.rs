//! Port of test/javascript/lib/icon_data.test.js (Phosphor icon subset, issue #137).
#![allow(clippy::unwrap_used)]

use frankmd_core::emoji::icons::{self, VIEW_BOX};
use std::collections::HashSet;

// port: test/javascript/lib/icon_data.test.js:9 "holds a well-formed [name, path, viewBox, keywords] tuple per icon"
#[test]
fn holds_a_well_formed_tuple_per_icon() {
    let all = icons::icons();
    assert!(all.len() >= 190, "expected >= 190 icons, got {}", all.len());
    for icon in all {
        assert!(!icon.name.is_empty());
        assert!(!icon.path.is_empty(), "{} has an empty path", icon.name);
        assert_eq!(icon.view_box, VIEW_BOX, "{}", icon.name);
        // keywords is a string by construction; it may be empty.
    }
}

// port: test/javascript/lib/icon_data.test.js:22 "has no duplicate icon names"
#[test]
fn has_no_duplicate_icon_names() {
    let names: Vec<&str> = icons::icons().iter().map(|i| i.name.as_str()).collect();
    let unique: HashSet<&str> = names.iter().copied().collect();
    assert_eq!(unique.len(), names.len());
}

// port: test/javascript/lib/icon_data.test.js:27 "includes icons from the expanded editor/dev/media batch"
#[test]
fn includes_icons_from_the_expanded_batch() {
    let map = icons::icon_map();
    for name in [
        "rocket",
        "git-branch",
        "text-b",
        "paperclip",
        "seal-check",
        "play",
    ] {
        let glyph = map
            .get(name)
            .unwrap_or_else(|| panic!("expected :ph-{name}: to resolve"));
        assert!(!glyph.path.is_empty());
        assert_eq!(glyph.view_box, VIEW_BOX);
        assert_eq!(icons::lookup(name), Some(*glyph));
    }
}

// port: test/javascript/lib/icon_data.test.js:36 "getIconMap() returns a stable cached map keyed by icon name"
#[test]
fn icon_map_is_stable_cached_and_keyed_by_name() {
    let first = icons::icon_map();
    assert!(std::ptr::eq(first, icons::icon_map()));
    let heart = first.get("heart").unwrap();
    assert!(!heart.path.is_empty());
    assert_eq!(heart.view_box, VIEW_BOX);
    assert_eq!(first.len(), icons::icons().len());
}
