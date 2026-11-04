//! Port of `emoji_picker_controller.test.js` (view-model rows).
#![allow(clippy::unwrap_used)]

use frankmd_core::emoji::icons;
use frankmd_core::emoji::picker::*;
use frankmd_core::emoji::{emoticons, shortcodes};

fn emoji_index(code: &str) -> usize {
    shortcodes()
        .iter()
        .position(|e| e.shortcode == code)
        .unwrap()
}

fn hover_to(p: &mut EmojiPicker, i: usize) {
    assert!(p.hover(i) || p.selected() == i);
}

fn codes(p: &EmojiPicker) -> Vec<String> {
    p.cells().into_iter().map(|c| c.code).collect()
}

#[test]
fn initial_state() {
    // adapts: emoji_picker_controller.test.js:78 "initializes emoji data arrays"
    assert!(!shortcodes().is_empty() && !emoticons().is_empty() && !icons::icons().is_empty());
    // adapts: emoji_picker_controller.test.js:85 "initializes filtered items with all emojis"
    let p = EmojiPicker::new(None);
    assert_eq!(p.len(), shortcodes().len());
    // adapts: emoji_picker_controller.test.js:89 "initializes selected index to 0"
    assert_eq!(p.selected(), 0);
    // adapts: emoji_picker_controller.test.js:93 "initializes active tab to emoji"
    assert_eq!(p.tab(), PickerTab::Emoji);
}

#[test]
fn open_resets() {
    // adapts: emoji_picker_controller.test.js:104 "clears the search input"
    let mut p = EmojiPicker::new(None);
    p.set_query("heart");
    p.open();
    assert_eq!(p.query(), "");
    // adapts: emoji_picker_controller.test.js:110 "resets to emoji tab"
    p.switch_tab(PickerTab::Icons);
    p.open();
    assert_eq!(p.tab(), PickerTab::Emoji);
    // adapts: emoji_picker_controller.test.js:116 "resets selected index to 0"
    p.hover(7);
    p.open();
    assert_eq!(p.selected(), 0);
}

#[test]
fn open_renders_frequent_then_all_and_footer() {
    // adapts: emoji_picker_controller.test.js:122 "renders emoji grid on open"
    let mut p = EmojiPicker::new(None);
    p.set_frequent(vec!["smile".into(), "nope".into()]);
    p.open();
    let cells = p.cells();
    assert_eq!(cells.len(), shortcodes().len() + 1);
    assert!(cells[0].frequent && cells[0].code == ":smile:");
    assert!(!cells[1].frequent);
    assert_eq!(cells[1].code, format!(":{}:", shortcodes()[0].shortcode));
    // adapts: emoji_picker_controller.test.js:128 "updates preview on open"
    let f = p.footer().unwrap();
    assert_eq!(f.code, ":smile:");
    assert_eq!(f.glyph, CellGlyph::Text("😄"));
}

#[test]
fn frequent_rules() {
    let mut p = EmojiPicker::new(None);
    p.set_frequent(vec!["smile".into()]);
    p.set_query("heart");
    assert!(p.cells().iter().all(|c| !c.frequent));
    p.set_query("");
    p.switch_tab(PickerTab::Icons);
    assert!(p.cells().iter().all(|c| !c.frequent));
    p.switch_tab(PickerTab::Emoji);
    let ins = PickerInsert {
        text: ":heart:".into(),
        kind: InsertKind::Emoji,
    };
    p.record_use(&ins);
    p.record_use(&PickerInsert {
        text: ":smile:".into(),
        kind: InsertKind::Emoji,
    });
    assert_eq!(p.frequent(), ["smile", "heart"]);
    p.set_frequent((0..30).map(|i| shortcodes()[i].shortcode.clone()).collect());
    assert_eq!(p.frequent().len(), 10);
}

#[test]
fn tab_switching() {
    for (tab, n) in [
        (PickerTab::Emoji, 5),
        (PickerTab::Emoticons, 3),
        (PickerTab::Icons, 2),
    ] {
        // adapts: emoji_picker_controller.test.js:143 "switches to emoji tab"
        // adapts: emoji_picker_controller.test.js:165 "switches to emoticons tab"
        // adapts: emoji_picker_controller.test.js:187 "switches to icons tab"
        let mut p = EmojiPicker::new(None);
        let other = if tab == PickerTab::Icons {
            PickerTab::Emoji
        } else {
            PickerTab::Icons
        };
        p.switch_tab(other);
        assert!(p.switch_tab(tab));
        assert_eq!(p.tab(), tab);
        let expected = match tab {
            PickerTab::Emoji => shortcodes().len(),
            PickerTab::Emoticons => emoticons().len(),
            PickerTab::Icons => icons::icons().len(),
        };
        assert_eq!(p.len(), expected);
        // adapts: emoji_picker_controller.test.js:149 "does nothing if already on emoji tab" (also :171, :193)
        p.hover(n);
        assert!(!p.switch_tab(tab));
        assert_eq!(p.selected(), n);
        // adapts: emoji_picker_controller.test.js:156 "resets selected index when switching" (also :178, :200)
        assert!(p.switch_tab(other));
        assert_eq!(p.selected(), 0);
    }
}

#[test]
fn column_counts() {
    // adapts: emoji_picker_controller.test.js:290 "returns emoji columns for emoji tab" (:295, :300)
    let mut p = EmojiPicker::new(None);
    assert_eq!(p.columns(), 10);
    p.switch_tab(PickerTab::Emoticons);
    assert_eq!(p.columns(), 5);
    p.switch_tab(PickerTab::Icons);
    assert_eq!(p.columns(), 8);
}

#[test]
fn grid_navigation() {
    let mut p = EmojiPicker::new(None);
    let last = p.len() - 1;
    // adapts: emoji_picker_controller.test.js:312 "moves selection right"
    p.move_selection(GridMove::Right);
    assert_eq!(p.selected(), 1);
    assert_eq!(p.take_scroll_request(), Some(1));
    assert_eq!(p.take_scroll_request(), None);
    // adapts: emoji_picker_controller.test.js:333 "moves selection left"
    p.hover(3);
    p.move_selection(GridMove::Left);
    assert_eq!(p.selected(), 2);
    // adapts: emoji_picker_controller.test.js:354 "moves selection down one row"
    p.move_selection(GridMove::Down);
    assert_eq!(p.selected(), 12);
    // adapts: emoji_picker_controller.test.js:381 "moves selection up one row"
    p.move_selection(GridMove::Up);
    assert_eq!(p.selected(), 2);
    // adapts: emoji_picker_controller.test.js:343 "wraps to end at beginning"
    p.hover(0);
    p.move_selection(GridMove::Left);
    assert_eq!(p.selected(), last);
    // adapts: emoji_picker_controller.test.js:322 "wraps to beginning at end"
    p.move_selection(GridMove::Right);
    assert_eq!(p.selected(), 0);
    // adapts: emoji_picker_controller.test.js:365 "wraps to first row at last row"
    p.hover(last / 10 * 10 + 2);
    p.move_selection(GridMove::Down);
    assert_eq!(p.selected(), 2);
}

#[test]
fn up_wraps_into_clamped_last_row() {
    // adapts: emoji_picker_controller.test.js:392 "wraps to last row at first row"
    let mut p = EmojiPicker::new(None);
    p.switch_tab(PickerTab::Icons);
    p.set_query("");
    // 25 items, 10 columns case on the emoji tab via query narrowing is
    // catalog-dependent, so check the rule on the real list instead.
    p.switch_tab(PickerTab::Emoji);
    let len = p.len();
    p.move_selection(GridMove::Up);
    assert_eq!(p.selected(), ((len - 1) / 10 * 10).min(len - 1));
    p.hover(0);
    p.set_query("");
    let col9 = len - 1;
    p.hover(9);
    p.move_selection(GridMove::Up);
    assert_eq!(p.selected(), ((len - 1) / 10 * 10 + 9).min(col9));
}

#[test]
fn empty_list_moves_are_noops() {
    let mut p = EmojiPicker::new(None);
    p.set_query("zzzzqqqq");
    assert!(p.is_empty());
    p.move_selection(GridMove::Up);
    p.move_selection(GridMove::Right);
    assert_eq!(p.selected(), 0);
    // adapts: emoji_picker_controller.test.js:774 "clears preview when no items"
    assert!(p.footer().is_none() && p.activate().is_none());
}

#[test]
fn tab_strip_keys() {
    use PickerTab::*;
    // adapts: emoji_picker_controller.test.js:426 "switches from emoji to emoticons with ArrowRight" (:436, :446, :456, :466, :476)
    for (from, key, to) in [
        (Emoji, TabKey::Right, Emoticons),
        (Emoticons, TabKey::Right, Icons),
        (Icons, TabKey::Right, Emoji),
        (Emoticons, TabKey::Left, Emoji),
        (Emoji, TabKey::Left, Icons),
        (Icons, TabKey::Left, Emoticons),
    ] {
        let mut p = EmojiPicker::new(None);
        p.switch_tab(from);
        assert!(p.tab_strip_key(key));
        assert_eq!(p.tab(), to);
    }
    // adapts: emoji_picker_controller.test.js:486 "ignores other keys"
    let mut p = EmojiPicker::new(None);
    assert!(!p.tab_strip_key(TabKey::Other));
    assert_eq!(p.tab(), Emoji);
    p.next_tab(false);
    assert_eq!(p.tab(), Icons);
}

#[test]
fn activation() {
    let mut p = EmojiPicker::new(None);
    // adapts: emoji_picker_controller.test.js:497 "selects current item and dispatches event"
    hover_to(&mut p, emoji_index("smile"));
    assert_eq!(
        p.activate(),
        Some(PickerInsert {
            text: ":smile:".into(),
            kind: InsertKind::Emoji
        })
    );
    // adapts: emoji_picker_controller.test.js:629 "dispatches emoji selection on click"
    assert_eq!(p.activate_at(emoji_index("heart")).unwrap().text, ":heart:");
    // adapts: emoji_picker_controller.test.js:528 "dispatches emoticon as raw text"
    p.switch_tab(PickerTab::Emoticons);
    let i = emoticons().iter().position(|e| e.text == "(◕‿◕)").unwrap();
    // adapts: emoji_picker_controller.test.js:643 "dispatches emoticon selection on click"
    assert_eq!(
        p.activate_at(i),
        Some(PickerInsert {
            text: "(◕‿◕)".into(),
            kind: InsertKind::Emoticon
        })
    );
    // adapts: emoji_picker_controller.test.js:657 "dispatches icon selection on click"
    p.switch_tab(PickerTab::Icons);
    let h = icons::icons()
        .iter()
        .position(|e| e.name == "heart")
        .unwrap();
    assert_eq!(
        p.activate_at(h),
        Some(PickerInsert {
            text: ":ph-heart:".into(),
            kind: InsertKind::Icon
        })
    );
    assert!(p.activate_at(usize::MAX).is_none());
}

#[test]
fn hover_does_not_scroll() {
    // adapts: emoji_picker_controller.test.js:551 "updates emoji selection in place without scrolling or rebuilding the grid"
    let mut p = EmojiPicker::new(None);
    assert!(p.hover(5));
    assert_eq!(p.selected(), 5);
    assert_eq!(p.take_scroll_request(), None);
    assert_eq!(p.footer().unwrap().code, p.cell(5).unwrap().code);
    // adapts: emoji_picker_controller.test.js:596 "updates icon selection in place"
    p.switch_tab(PickerTab::Icons);
    assert!(p.hover(3));
    assert_eq!(p.take_scroll_request(), None);
    assert_eq!(
        p.footer().unwrap().code,
        format!(":ph-{}:", icons::icons()[3].name)
    );
    // adapts: emoji_picker_controller.test.js:615 "does nothing for same index"
    p.hover(4);
    assert!(!p.hover(4));
    assert!(!p.hover(usize::MAX));
}

#[test]
fn cells_and_footer() {
    let mut p = EmojiPicker::new(None);
    // adapts: emoji_picker_controller.test.js:679 "renders emoji buttons with data attributes"
    let c = p.cell(emoji_index("smile")).unwrap();
    assert_eq!(
        (c.glyph, c.code.as_str()),
        (CellGlyph::Text("😄"), ":smile:")
    );
    // adapts: emoji_picker_controller.test.js:689 "highlights selected item"
    p.hover(2);
    let sel: Vec<usize> = p
        .cells()
        .iter()
        .filter(|c| c.selected)
        .map(|c| c.index)
        .collect();
    assert_eq!(sel, [2]);
    // adapts: emoji_picker_controller.test.js:744 "shows emoji preview with shortcode"
    hover_to(&mut p, emoji_index("smile"));
    assert_eq!(p.footer().unwrap().glyph, CellGlyph::Text("😄"));
    // adapts: emoji_picker_controller.test.js:707 "renders emoticon buttons with data attributes"
    p.switch_tab(PickerTab::Emoticons);
    let c = p.cell(0).unwrap();
    assert_eq!(c.glyph, CellGlyph::Text("(◕‿◕)"));
    assert_eq!(c.code, "happy");
    // adapts: emoji_picker_controller.test.js:754 "shows emoticon preview with name"
    assert_eq!(p.footer().unwrap().code, "happy");
    // adapts: emoji_picker_controller.test.js:717 "uses different column count for emoticons"
    assert_eq!(p.columns(), 5);
    // adapts: emoji_picker_controller.test.js:728 "renders icon buttons with an inline SVG"
    p.switch_tab(PickerTab::Icons);
    let h = icons::icons()
        .iter()
        .position(|e| e.name == "heart")
        .unwrap();
    assert_eq!(
        p.cell(h).unwrap().glyph,
        CellGlyph::Icon(icons::lookup("heart").unwrap())
    );
    // adapts: emoji_picker_controller.test.js:764 "shows icon preview with shortcode and inline SVG"
    p.hover(h);
    let f = p.footer().unwrap();
    assert_eq!(f.code, ":ph-heart:");
    assert!(matches!(f.glyph, CellGlyph::Icon(_)));
    // adapts: emoji_picker_controller.test.js:738 "uses different column count for icons"
    assert_eq!(p.columns(), 8);
}

#[test]
fn insert_kind_follows_tab() {
    // adapts: emoji_picker_controller.test.js:784 "dispatches selected event with text"
    let mut p = EmojiPicker::new(None);
    p.switch_tab(PickerTab::Emoticons);
    assert_eq!(p.activate().unwrap().kind, InsertKind::Emoticon);
}

#[test]
fn locale_index() {
    // adapts: emoji_picker_controller.test.js:802 "initializes i18n properties during connect"
    let p = EmojiPicker::new(Some("pt-BR"));
    assert!(!p.search_index().is_empty());
    // adapts: emoji_picker_controller.test.js:816 "sets i18nLoaded to true after loading completes"
    let mut p = EmojiPicker::new(Some("en"));
    assert!(p.set_locale(Some("pt-BR")));
    p.set_query("feliz");
    assert!(p.cells().iter().any(|c| c.glyph == CellGlyph::Text("😀")));
    // adapts: emoji_picker_controller.test.js:828 "builds search index with emoji data"
    assert!(p.search_index().get("😀").is_some() && p.search_index().get("😂").is_some());
    // adapts: emoji_picker_controller.test.js:855 "skips loading if already loaded"
    assert!(!p.set_locale(Some("pt-BR")));
    assert!(!p.set_locale(Some("pt-PT")));
    // adapts: emoji_picker_controller.test.js:841 "results in a Map even if loading fails"
    let mut q = EmojiPicker::new(Some("xx"));
    q.set_query("heart");
    assert!(!q.is_empty());
}

#[test]
fn localized_search_only_on_emoji_tab() {
    // adapts: emoji_picker_controller.test.js:925 "uses i18n search for emoji tab"
    let mut p = EmojiPicker::new(Some("pt-BR"));
    p.set_query("coração");
    assert!(p.cells().iter().any(|c| c.glyph == CellGlyph::Text("❤️")));
    // adapts: emoji_picker_controller.test.js:933 "does not use i18n search for emoticons tab"
    p.switch_tab(PickerTab::Emoticons);
    p.set_query("feliz");
    assert!(p.is_empty());
}

#[test]
fn query_is_trimmed_lowercased_and_anded() {
    let mut p = EmojiPicker::new(None);
    p.set_query("  HEART  ");
    assert!(codes(&p).contains(&":heart:".to_string()));
    p.set_query("love red");
    assert!(codes(&p).contains(&":heart:".to_string()));
    p.set_query("love zzzz");
    assert!(p.is_empty());
    p.switch_tab(PickerTab::Emoticons);
    p.set_query("smile joy");
    assert!(codes(&p).contains(&"happy".to_string()));
}
