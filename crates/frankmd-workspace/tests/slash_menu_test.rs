#![allow(clippy::unwrap_used)]
//! Slash menu model (plan rows for `codemirror_slash_commands.test.js`).

use frankmd_core::text::slash::{SlashCommand, slash_trigger};
use frankmd_workspace::slash_menu::{
    MAX_VISIBLE_ROWS, ROW_HEIGHT_PX, SlashMenuModel, SlashOutcome,
};

fn open(text: &str, caret: usize) -> SlashMenuModel {
    let t = slash_trigger(text, caret).unwrap();
    SlashMenuModel::open(t, |c| c.fallback_label().to_owned())
}

fn commands(m: &SlashMenuModel) -> Vec<SlashCommand> {
    m.rows().iter().map(|r| r.command).collect()
}

#[test]
fn empty_query_lists_all_in_order_with_icons() {
    // port: codemirror_slash_commands.test.js:137 "renders the semantic SVG icons inside the completion menu"
    let m = open("/", 1);
    assert_eq!(commands(&m), SlashCommand::ALL);
    assert!(m.rows().iter().all(|r| !r.icon.is_empty()));
    assert_eq!(m.rows()[0].label, "Heading 1");
    assert_eq!(m.rows()[10].icon, "table");
}

#[test]
fn layout_caps_at_eight_rows() {
    // port: codemirror_slash_commands.test.js:31 "lets autocomplete results grow" (changed: 8 x 28px)
    let m = open("/", 1);
    assert_eq!(m.visible_rows(), MAX_VISIBLE_ROWS);
    assert_eq!(m.height_px(), 8.0 * ROW_HEIGHT_PX);
    let m = open("/quote", 6);
    assert!(m.visible_rows() < 8);
    assert_eq!(m.height_px(), m.visible_rows() as f32 * ROW_HEIGHT_PX);
}

#[test]
fn arrow_keys_and_enter_apply_heading_2() {
    // port: codemirror_slash_commands.test.js:167 "moves through slash options with arrow keys and accepts"
    let mut m = open("/", 1);
    m.select_next();
    m.select_next();
    m.select_prev();
    assert_eq!(m.selected(), 1);
    match m.accept("/").unwrap() {
        SlashOutcome::Edit(e) => {
            let mut s = "/".to_owned();
            e.apply_to(&mut s);
            assert_eq!(s, "## ");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn selection_wraps() {
    let mut m = open("/", 1);
    m.select_prev();
    assert_eq!(m.selected(), 13);
    m.select_next();
    assert_eq!(m.selected(), 0);
    m.hover(5);
    assert_eq!(m.selected(), 5);
    m.hover(99);
    assert_eq!(m.selected(), 5);
}

#[test]
fn table_opens_dialog_with_slash_range() {
    // port: codemirror_slash_commands.test.js:241 "dispatches insert actions with the original slash range"
    let text = "Before /table after";
    let m = open(text, 13);
    assert_eq!(m.rows()[0].command, SlashCommand::Table);
    assert_eq!(
        m.accept(text),
        Some(SlashOutcome::OpenDialog {
            command: SlashCommand::Table,
            range: 7..13,
            query: "/table".to_owned()
        })
    );
}

#[test]
fn fuzzy_filter_ranks_and_matches_aliases() {
    let m = open("/h2", 3);
    assert_eq!(m.rows()[0].command, SlashCommand::Heading2);
    let m = open("/todo", 5);
    assert_eq!(m.rows()[0].command, SlashCommand::TodoList);
    let m = open("/hr", 3);
    assert_eq!(m.rows()[0].command, SlashCommand::Divider);
    assert!(open("/zzzz", 5).rows().is_empty());
}

#[test]
fn update_keeps_selection_and_reports_trigger_loss() {
    let mut text = "/".to_owned();
    let mut m = open(&text, 1);
    m.hover(10);
    text.push_str("ta");
    assert!(m.update(&text, 3));
    assert_eq!(m.rows()[m.selected()].command, SlashCommand::Table);
    assert_eq!(m.trigger().query, "ta");
    text.push_str("/x");
    assert!(!m.update(&text, text.len()));
    // Escape: dropping the model leaves the query in the text.
    assert_eq!(text, "/ta/x");
}

#[test]
fn accept_out_of_range_is_none() {
    let m = open("/zzzz", 5);
    assert!(m.accept("/zzzz").is_none());
    assert!(open("/", 1).accept_at(99, "/").is_none());
}
