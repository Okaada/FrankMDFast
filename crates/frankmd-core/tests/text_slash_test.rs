#![allow(clippy::unwrap_used)]
//! Ports of the GUI-free cases of `codemirror_slash_commands.test.js`.

use frankmd_core::text::slash::{
    SlashCommand, SlashTrigger, apply_command, dialog_range, slash_trigger,
};

fn run(text: &str, cmd: SlashCommand, caret: usize) -> (String, usize) {
    let trigger = slash_trigger(text, caret).unwrap();
    let edit = apply_command(text, &trigger, cmd).unwrap();
    let mut out = text.to_owned();
    edit.apply_to(&mut out);
    (out, edit.selection.start)
}

fn apply(text: &str, cmd: SlashCommand) -> String {
    run(text, cmd, text.len()).0
}

fn trigger(text: &str) -> Option<SlashTrigger> {
    slash_trigger(text, text.len())
}

#[test]
fn command_table_matches_original() {
    // port: codemirror_slash_commands.test.js:119 "maps each menu label to a semantic Phosphor icon"
    let icons: Vec<_> = SlashCommand::ALL.iter().map(|c| c.icon()).collect();
    assert_eq!(
        icons,
        [
            "hash",
            "hash",
            "hash",
            "list-bullets",
            "list-numbers",
            "list-checks",
            "quotes",
            "code",
            "sigma",
            "minus",
            "table",
            "image",
            "video-camera",
            "smiley"
        ]
    );
    // port: codemirror_slash_commands.test.js:94 "uses fallback labels while translations are still unavailable"
    let labels: Vec<_> = SlashCommand::ALL
        .iter()
        .map(|c| c.fallback_label())
        .collect();
    assert_eq!(
        labels,
        [
            "Heading 1",
            "Heading 2",
            "Heading 3",
            "Bulleted list",
            "Numbered list",
            "To-do list",
            "Quote",
            "Code block",
            "Equation",
            "Divider",
            "Table",
            "Image",
            "Video",
            "Emoji"
        ]
    );
    assert_eq!(SlashCommand::TodoList.id(), "to-do-list");
    assert_eq!(
        SlashCommand::Heading2.label_key(),
        "editor.slash_commands.heading_2"
    );
    assert_eq!(
        SlashCommand::Heading1.filter_text("Heading 1"),
        "Heading 1 heading 1 heading1 h1"
    );
    assert_eq!(
        SlashCommand::Table.filter_text("Table"),
        "Table insert table grid"
    );
    assert_eq!(
        SlashCommand::ALL
            .iter()
            .filter(|c| c.opens_dialog())
            .count(),
        4
    );
}

#[test]
fn trigger_inside_paragraph() {
    // port: codemirror_slash_commands.test.js:76 "offers localized heading commands from a slash typed inside a paragraph"
    let t = trigger("An existing paragraph /he").unwrap();
    assert_eq!(t.slash + 1, "An existing paragraph /".len());
    assert_eq!(t.query, "he");
}

#[test]
fn trigger_on_new_lines() {
    // port: codemirror_slash_commands.test.js:233 "offers commands when the slash starts a new line"
    assert!(trigger("/").is_some());
    assert!(trigger("Text\n\n/").is_some());
    assert!(trigger("Text /").is_some());
}

#[test]
fn no_trigger_in_code() {
    // port: codemirror_slash_commands.test.js:437 "does not trigger inside inline code"
    let t = "`example /heading`";
    assert!(slash_trigger(t, t.len() - 1).is_none());
    // port: codemirror_slash_commands.test.js:437 "does not trigger inside fenced code"
    let t = "```md\n/heading\n```";
    assert!(slash_trigger(t, "```md\n/heading".len()).is_none());
}

#[test]
fn no_trigger_in_path_or_link_destination() {
    // port: codemirror_slash_commands.test.js:463 "does not treat a slash inside a path as a command"
    assert!(trigger("docs/heading").is_none());
    // port: codemirror_slash_commands.test.js:468 "does not trigger in a Markdown link/image destination"
    for t in ["[docs](/images/logo)", "![logo](/images/logo)"] {
        let caret = t.find("/images").unwrap() + "/images".len();
        assert!(slash_trigger(t, caret).is_none());
    }
    // port: codemirror_slash_commands.test.js:477 "still triggers after an opening parenthesis in paragraph text"
    assert!(trigger("Text (/heading").is_some());
}

#[test]
fn trigger_unicode_query() {
    // port: codemirror_slash_commands.test.js:481 "matches Unicode letters, numbers, and combining marks"
    let t = trigger("Text /日本語 2-e\u{301}").unwrap();
    assert_eq!(t.query, "日本語 2-e\u{301}");
    assert!(trigger("Text /日本/語").is_none());
}

#[test]
fn query_stays_in_document() {
    // port: codemirror_slash_commands.test.js:455 "leaves the slash query in the document until a command is selected"
    let text = "Text /heading";
    assert!(trigger(text).is_some());
}

#[test]
fn dialog_commands_report_range() {
    // port: codemirror_slash_commands.test.js:241 "dispatches insert actions with the original slash range"
    let text = "Before /table after";
    let t = slash_trigger(text, 13).unwrap();
    assert_eq!(dialog_range(&t), (7..13, "/table".to_owned()));
    assert!(apply_command(text, &t, SlashCommand::Table).is_none());
}

#[test]
fn block_commands_on_paragraph_single_edit() {
    // port: codemirror_slash_commands.test.js:268 "applies %s to the paragraph and commits one editor transaction"
    let text = "Before /command after";
    let caret = text.find(" after").unwrap();
    let cases = [
        (SlashCommand::BulletedList, "- Before after"),
        (SlashCommand::NumberedList, "1. Before after"),
        (SlashCommand::TodoList, "- [ ] Before after"),
        (SlashCommand::Quote, "> Before after"),
        (SlashCommand::CodeBlock, "```\nBefore after\n```"),
        (SlashCommand::Equation, "$$\nBefore after\n$$"),
        (SlashCommand::Divider, "Before after\n\n---"),
    ];
    for (cmd, expected) in cases {
        assert_eq!(run(text, cmd, caret).0, expected, "{cmd:?}");
    }
}

#[test]
fn soft_line_breaks_preserved() {
    // port: codemirror_slash_commands.test.js:282 "preserves soft line breaks when applying %s"
    let cases = [
        (
            SlashCommand::BulletedList,
            "first /list\nsecond",
            "- first\n  second",
        ),
        (
            SlashCommand::NumberedList,
            "first /list\nsecond",
            "1. first\n   second",
        ),
        (
            SlashCommand::TodoList,
            "first /list\nsecond",
            "- [ ] first\n      second",
        ),
        (
            SlashCommand::Quote,
            "first /quote\nsecond",
            "> first\n> second",
        ),
        (
            SlashCommand::CodeBlock,
            "first /code\nsecond",
            "```\nfirst\nsecond\n```",
        ),
        (
            SlashCommand::Equation,
            "first /equation\nsecond",
            "$$\nfirst\nsecond\n$$",
        ),
    ];
    for (cmd, text, expected) in cases {
        let caret = text.find('\n').unwrap();
        assert_eq!(run(text, cmd, caret).0, expected, "{cmd:?}");
    }
}

#[test]
fn equation_cases() {
    // port: codemirror_slash_commands.test.js:326 "places the cursor inside an empty display equation block"
    assert_eq!(
        run("/equation", SlashCommand::Equation, 9),
        ("$$\n\n$$".to_owned(), 3)
    );
    // port: codemirror_slash_commands.test.js:334 "starts an equation on its own line without wrapping the preceding paragraph"
    let (out, caret) = run("Hello World\n/equation", SlashCommand::Equation, 21);
    assert_eq!(out, "Hello World\n\n$$\n\n$$");
    assert_eq!(caret, "Hello World\n\n$$\n".len());
    // port: codemirror_slash_commands.test.js:341 "separates following soft lines from an equation started below paragraph text"
    let text = "Hello World\n/equation\ncontinued text";
    let caret = text.find("\ncontinued").unwrap();
    assert_eq!(
        run(text, SlashCommand::Equation, caret).0,
        "Hello World\n\n$$\n\n$$\n\ncontinued text"
    );
}

#[test]
fn lists_quotes_and_tasks() {
    // port: codemirror_slash_commands.test.js:348 "keeps wikilinks intact when creating a to-do item"
    assert_eq!(
        apply("[[linked note]] /todo", SlashCommand::TodoList),
        "- [ ] [[linked note]]"
    );
    // port: codemirror_slash_commands.test.js:354 "does not stack an existing list marker and can change its kind"
    assert_eq!(
        apply("- Existing /bullet", SlashCommand::BulletedList),
        "- Existing"
    );
    assert_eq!(
        apply("1. Existing /bullet", SlashCommand::BulletedList),
        "- Existing"
    );
    // port: codemirror_slash_commands.test.js:362 "preserves an existing checked task marker"
    assert_eq!(
        apply("- [x] Done /todo", SlashCommand::TodoList),
        "- [x] Done"
    );
    // port: codemirror_slash_commands.test.js:368 "does not duplicate an existing blockquote marker"
    assert_eq!(
        apply("> Existing /quote", SlashCommand::Quote),
        "> Existing"
    );
}

#[test]
fn code_block_with_backticks() {
    // port: codemirror_slash_commands.test.js:374 "chooses a longer fence when the paragraph contains a backtick run"
    let (out, caret) = run("Example ```token``` /code", SlashCommand::CodeBlock, 25);
    assert_eq!(out, "````\nExample ```token```\n````");
    assert_eq!(caret, "````\nExample ```token```".len());
}

#[test]
fn headings() {
    // port: codemirror_slash_commands.test.js:383 "applies Heading %i in place and preserves text around the query"
    for (cmd, marker) in [
        (SlashCommand::Heading1, "# "),
        (SlashCommand::Heading2, "## "),
        (SlashCommand::Heading3, "### "),
    ] {
        let text = "Before /head after";
        let (out, caret) = run(text, cmd, text.find(" after").unwrap());
        assert_eq!(out, format!("{marker}Before after"));
        assert_eq!(caret, format!("{marker}Before ").len());
    }
    // port: codemirror_slash_commands.test.js:405 "keeps a soft-line-break paragraph in one heading"
    let text = "First line /h1\nsecond line";
    assert_eq!(
        run(text, SlashCommand::Heading1, text.find('\n').unwrap()).0,
        "# First line second line"
    );
    // port: codemirror_slash_commands.test.js:423 "can change the level of an existing ATX heading"
    assert_eq!(
        apply("## Existing /h2", SlashCommand::Heading3),
        "### Existing"
    );
}

#[test]
fn analyzer_extras() {
    // list continuation paragraph and divider after the item
    assert_eq!(
        apply("- item\n  more /div", SlashCommand::Divider),
        "- item\n  more\n\n---"
    );
    // code block inside a list item indents continuation lines
    assert_eq!(
        run("- a /code\n  b", SlashCommand::CodeBlock, 9).0,
        "- ```\n  a\n    b\n  ```"
    );
    // slash in a table row or after a fence is not editable
    assert!(trigger("| a | /").is_none());
    assert!(trigger("```\ncode\n```\n/").is_some());
    assert!(trigger("```\ncode\n/").is_none());
}
