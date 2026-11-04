//! Engine behaviour beyond the ported FrankMD tests: the codemirror-vim subset
//! FrankMD users rely on (docs/research/gpui-stack.md §6.2 scope), checked
//! against stock vim semantics.
#![allow(clippy::unwrap_used)]

mod common;

use common::{FakeBuffer, feed_str};
use frankmd_vim::buffer::VimBuffer;
use frankmd_vim::keymap::Key;
use frankmd_vim::{Mode, Outcome, VimState};

/// Feed `keys` like the app does: keys the engine passes through in insert
/// mode are typed into the buffer, as the editor would.
fn type_keys(vim: &mut VimState, buf: &mut FakeBuffer, keys: &str) {
    for c in keys.chars() {
        let name = match c {
            '\n' => "enter".to_owned(),
            '\x1b' => "escape".to_owned(),
            c => c.to_string(),
        };
        if vim.feed(&Key::plain(&name), false, buf) == Outcome::PassThrough
            && vim.mode() == Mode::Insert
        {
            let typed = if c == '\n' {
                "\n".to_owned()
            } else {
                c.to_string()
            };
            let at = buf.sel.clone();
            buf.replace(at, &typed);
        }
    }
}

/// Run `keys` on `text` with the caret at `caret`; returns (text, caret).
fn run(text: &str, caret: usize, keys: &str) -> (String, usize) {
    let mut buf = FakeBuffer::new(text);
    buf.sel = caret..caret;
    let mut vim = VimState::new(false);
    type_keys(&mut vim, &mut buf, keys);
    (buf.text, buf.sel.end)
}

fn text(text: &str, caret: usize, keys: &str) -> String {
    run(text, caret, keys).0
}

fn caret(text: &str, caret: usize, keys: &str) -> usize {
    run(text, caret, keys).1
}

#[test]
fn basic_motions() {
    let t = "hello world foo\nsecond line\n";
    assert_eq!(caret(t, 0, "w"), 6);
    assert_eq!(caret(t, 0, "2w"), 12);
    assert_eq!(caret(t, 0, "e"), 4);
    assert_eq!(caret(t, 12, "b"), 6);
    assert_eq!(caret(t, 0, "$"), 14);
    assert_eq!(caret(t, 14, "0"), 0);
    assert_eq!(caret(t, 3, "j"), 19);
    assert_eq!(caret(t, 3, "jk"), 3);
    assert_eq!(caret(t, 0, "G"), 28);
    assert_eq!(caret(t, 20, "gg"), 0);
    assert_eq!(caret(t, 0, "2G"), 16);
    assert_eq!(caret(t, 0, "fo"), 4);
    assert_eq!(caret(t, 0, "fo;"), 7);
    assert_eq!(caret(t, 0, "to"), 3);
    assert_eq!(caret(t, 0, "l"), 1);
    assert_eq!(caret(t, 14, "l"), 14, "l stops at the last char");
    assert_eq!(caret("  x", 0, "^"), 2);
    assert_eq!(caret("a (b) c", 0, "%"), 4);
    assert_eq!(caret("a\nb\n\nc\n", 0, "}"), 4);
    assert_eq!(caret("a\nb\n\nc\n", 5, "{"), 4);
}

#[test]
fn sticky_column_and_dollar() {
    let t = "long line here\nab\nanother long one\n";
    assert_eq!(caret(t, 10, "j"), 16);
    assert_eq!(caret(t, 10, "jj"), 28);
    assert_eq!(caret(t, 0, "$jj"), 33);
}

#[test]
fn delete_change_yank() {
    let t = "one two three\n";
    assert_eq!(text(t, 0, "dw"), "two three\n");
    assert_eq!(text(t, 0, "d2w"), "three\n");
    assert_eq!(text(t, 0, "2dw"), "three\n");
    assert_eq!(text(t, 4, "de"), "one  three\n");
    assert_eq!(text(t, 4, "d$"), "one \n");
    assert_eq!(text(t, 4, "D"), "one \n");
    assert_eq!(text(t, 0, "x"), "ne two three\n");
    assert_eq!(text(t, 0, "3x"), " two three\n");
    assert_eq!(text(t, 2, "X"), "oe two three\n");
    assert_eq!(
        text(t, 8, "dw"),
        "one two \n",
        "dw on the last word keeps the newline"
    );
    assert_eq!(text(t, 0, "cwONE\x1b"), "ONE two three\n");
    assert_eq!(text(t, 4, "ciwX\x1b"), "one X three\n");
    assert_eq!(text(t, 4, "daw"), "one three\n");
    assert_eq!(text(t, 0, "dfw"), "o three\n");
    assert_eq!(text(t, 0, "dtw"), "wo three\n");
    assert_eq!(text(t, 0, "yyp"), "one two three\none two three\n");
    assert_eq!(text(t, 4, "ywP"), "one two two three\n");
    assert_eq!(text(t, 0, "Cx\x1b"), "x\n");
    assert_eq!(text(t, 0, "sX\x1b"), "Xne two three\n");
}

#[test]
fn linewise_operators() {
    let t = "a\nb\nc\nd";
    assert_eq!(text(t, 0, "dd"), "b\nc\nd");
    assert_eq!(text(t, 2, "2dd"), "a\nd");
    assert_eq!(text(t, 6, "dd"), "a\nb\nc");
    assert_eq!(text(t, 0, "dj"), "c\nd");
    assert_eq!(text(t, 2, "dk"), "c\nd");
    assert_eq!(text(t, 2, "dG"), "a");
    assert_eq!(text(t, 4, "dgg"), "d");
    assert_eq!(text(t, 0, "ddp"), "b\na\nc\nd");
    assert_eq!(text(t, 6, "ddP"), "a\nb\nd\nc");
    assert_eq!(text(t, 0, "yyjp"), "a\nb\na\nc\nd");
    assert_eq!(text(t, 0, "ccX\x1b"), "X\nb\nc\nd");
    assert_eq!(text(t, 0, ">>"), "  a\nb\nc\nd");
    assert_eq!(text(t, 0, ">j"), "  a\n  b\nc\nd");
    assert_eq!(text("    a", 0, "<<"), "  a");
    assert_eq!(text(t, 0, "J"), "a b\nc\nd");
    assert_eq!(text(t, 0, "3J"), "a b c\nd");
    assert_eq!(caret("abc\n\ndef\n", 0, "dap"), 0);
    assert_eq!(text("abc\n\ndef\n", 0, "dap"), "def\n");
}

#[test]
fn case_and_replace() {
    assert_eq!(text("hello world", 0, "gUiw"), "HELLO world");
    assert_eq!(text("HELLO world", 0, "guu"), "hello world");
    assert_eq!(text("Hello", 0, "g~~"), "hELLO");
    assert_eq!(text("abc", 0, "~~"), "ABc");
    assert_eq!(text("abc", 0, "rx"), "xbc");
    assert_eq!(text("abc", 0, "2rx"), "xxc");
    assert_eq!(text("abc", 0, "5rx"), "abc", "not enough chars");
    assert_eq!(text("abcd", 1, "RXY\x1b"), "aXYd");
}

#[test]
fn text_objects() {
    let t = r#"say "hi there" (a, (b)) ok"#;
    assert_eq!(text(t, 6, "di\""), r#"say "" (a, (b)) ok"#);
    assert_eq!(text(t, 6, "da\""), "say (a, (b)) ok");
    assert_eq!(text(t, 16, "di("), r#"say "hi there" () ok"#);
    assert_eq!(text(t, 20, "da("), r#"say "hi there" (a, ) ok"#);
    assert_eq!(text(t, 16, "ci)X\x1b"), r#"say "hi there" (X) ok"#);
}

#[test]
fn insert_entries_and_escape() {
    assert_eq!(run("abc", 1, "i\x1b"), ("abc".into(), 0));
    assert_eq!(run("abc", 0, "a\x1b"), ("abc".into(), 0));
    assert_eq!(text("abc\n", 1, "oX\x1b"), "abc\nX\n");
    assert_eq!(text("  abc\n", 3, "oX\x1b"), "  abc\n  X\n");
    assert_eq!(text("abc\n", 1, "OX\x1b"), "X\nabc\n");
    assert_eq!(caret("  abc", 4, "I\x1b"), 1, "Esc steps back one char");
    assert_eq!(caret("  abc", 2, "A"), 5);
    assert_eq!(text("ab", 0, "3ix\x1b"), "xxxab");
    assert_eq!(text("a\n", 0, "2ox\x1b"), "a\nx\nx\n");
}

#[test]
fn dot_repeat() {
    assert_eq!(text("a b c d", 0, "dw."), "c d");
    assert_eq!(text("a b c d e", 0, "dw2."), "d e");
    assert_eq!(text("one two three", 0, "cwX\x1bw."), "X X three");
    assert_eq!(text("a\nb\nc\n", 0, "dd."), "c\n");
    assert_eq!(text("ab", 0, "ix\x1b."), "xxab");
    assert_eq!(text("abc", 0, "x.."), "");
    assert_eq!(text("a\nb\n", 0, "Ax\x1bj."), "ax\nbx\n");
}

#[test]
fn undo_redo() {
    let mut buf = FakeBuffer::new("one two");
    let mut vim = VimState::new(false);
    feed_str(&mut vim, &mut buf, "dw");
    assert_eq!(buf.text, "two");
    feed_str(&mut vim, &mut buf, "u");
    assert_eq!(buf.text, "one two");
    vim.feed(&Key::ctrl("r"), false, &mut buf);
    assert_eq!(buf.text, "two");
}

#[test]
fn registers() {
    // CodeMirror line model: no trailing newline, so `dd` on the last line leaves "a".
    let t = "a\nb";
    assert_eq!(text(t, 0, "\"ayyj\"ap"), "a\nb\na");
    assert_eq!(text(t, 0, "\"ayyjdd\"ap"), "a\na");
    assert_eq!(text(t, 0, "yyjdd\"0p"), "a\na");
    assert_eq!(
        text(t, 0, "\"_ddp"),
        "b",
        "black hole leaves the unnamed register empty"
    );
}

#[test]
fn visual_modes() {
    let t = "hello world\nsecond\nthird\n";
    assert_eq!(text(t, 0, "vlld"), "lo world\nsecond\nthird\n");
    assert_eq!(text(t, 0, "vey$p"), "hello worldhello\nsecond\nthird\n");
    assert_eq!(text(t, 0, "Vjd"), "third\n");
    assert_eq!(text(t, 0, "VjU"), "HELLO WORLD\nSECOND\nthird\n");
    assert_eq!(text(t, 0, "Vj>"), "  hello world\n  second\nthird\n");
    assert_eq!(text(t, 0, "viwcX\x1b"), "X world\nsecond\nthird\n");
    assert_eq!(text(t, 6, "vex"), "hello \nsecond\nthird\n");
    assert_eq!(text(t, 0, "vlrx"), "xxllo world\nsecond\nthird\n");
    assert_eq!(
        text(t, 0, "VJ"),
        "hello world second\nthird\n",
        "J joins at least two lines"
    );
    assert_eq!(text(t, 0, "VjJ"), "hello world second\nthird\n");
    let mut buf = FakeBuffer::new(t);
    let mut vim = VimState::new(false);
    feed_str(&mut vim, &mut buf, "vl");
    assert_eq!(vim.mode(), Mode::Visual);
    assert_eq!(buf.sel, 0..2);
    feed_str(&mut vim, &mut buf, "V");
    assert_eq!(vim.mode(), Mode::VisualLine);
    feed_str(&mut vim, &mut buf, "\x1b");
    assert_eq!(vim.mode(), Mode::Normal);
    assert_eq!(buf.sel, 1..1);
}

#[test]
fn visual_block() {
    let t = "abcd\nefgh\nijkl\n";
    let mut buf = FakeBuffer::new(t);
    buf.sel = 1..1;
    let mut vim = VimState::new(false);
    vim.feed(&Key::ctrl("q"), false, &mut buf);
    assert_eq!(vim.mode(), Mode::VisualBlock);
    feed_str(&mut vim, &mut buf, "jld");
    assert_eq!(buf.text, "ad\neh\nijkl\n");
    feed_str(&mut vim, &mut buf, "jp");
    assert_eq!(buf.text, "ad\nehbc\nfg\nijkl\n", "block put is charwise");
}

#[test]
fn search_and_star() {
    let t = "foo bar\nbaz foo\nfoo\n";
    assert_eq!(caret(t, 0, "/foo\n"), 12);
    assert_eq!(caret(t, 0, "/foo\nn"), 16);
    assert_eq!(caret(t, 0, "/foo\nnn"), 0, "wraps");
    assert_eq!(caret(t, 0, "/foo\nN"), 0);
    assert_eq!(caret(t, 0, "?foo\n"), 16);
    assert_eq!(caret(t, 0, "*"), 12);
    assert_eq!(caret(t, 0, "#"), 16);
    assert_eq!(text(t, 0, "d/baz\n"), "baz foo\nfoo\n");
    let mut buf = FakeBuffer::new(t);
    let mut vim = VimState::new(false);
    feed_str(&mut vim, &mut buf, "/nope\n");
    assert_eq!(vim.message(), Some("Pattern not found: nope"));
    feed_str(&mut vim, &mut buf, "/bar\n");
    assert_eq!(vim.search_highlight(), Some("bar"));
    feed_str(&mut vim, &mut buf, ":noh\n");
    assert_eq!(vim.search_highlight(), None);
}

#[test]
fn ex_native_commands() {
    let t = "a foo foo\nb foo\nc\n";
    assert_eq!(caret(t, 0, ":3\n"), 16);
    assert_eq!(caret(t, 0, ":$\n"), 18);
    assert_eq!(text(t, 0, ":s/foo/bar/\n"), "a bar foo\nb foo\nc\n");
    assert_eq!(text(t, 0, ":s/foo/bar/g\n"), "a bar bar\nb foo\nc\n");
    assert_eq!(text(t, 0, ":%s/foo/X/g\n"), "a X X\nb X\nc\n");
    assert_eq!(
        text(t, 0, ":%s/(f)oo/[\\1&]/\n"),
        "a [ffoo] foo\nb [ffoo]\nc\n"
    );
    assert_eq!(text(t, 0, ":2,3s/^/> /\n"), "a foo foo\n> b foo\n> c\n");
    assert_eq!(text(t, 0, "Vj:s/o/0/g\n"), "a f00 f00\nb f00\nc\n");
    assert_eq!(text("Foo foo", 0, ":s/foo/x/gi\n"), "x x");
    let mut buf = FakeBuffer::new(t);
    let mut vim = VimState::new(false);
    feed_str(&mut vim, &mut buf, ":bogus\n");
    assert_eq!(vim.message(), Some("Not an editor command: bogus"));
    feed_str(&mut vim, &mut buf, ":s/zzz/y/\n");
    assert_eq!(vim.message(), Some("No matches for zzz"));
    // Escape and backspace on an empty line cancel.
    feed_str(&mut vim, &mut buf, ":abc\x1b");
    assert_eq!(vim.mode(), Mode::Normal);
    feed_str(&mut vim, &mut buf, ":");
    vim.feed(&Key::plain("backspace"), false, &mut buf);
    assert_eq!(vim.mode(), Mode::Normal);
}

#[test]
fn marks() {
    let t = "a\nb\nc foo\n";
    assert_eq!(caret(t, 6, "magg'a"), 4);
    assert_eq!(caret(t, 6, "magg`a"), 6);
    assert_eq!(caret(t, 6, "gg''"), 4);
    assert_eq!(text(t, 0, "majjd'a"), "");
}

#[test]
fn utf8_offsets() {
    let t = "héllo wörld\n";
    assert_eq!(caret(t, 0, "l"), 1);
    assert_eq!(caret(t, 0, "ll"), 3);
    assert_eq!(caret(t, 0, "w"), 7);
    assert_eq!(caret(t, 0, "$"), 12);
    assert_eq!(text(t, 1, "x"), "hllo wörld\n");
    assert_eq!(text(t, 0, "gUiw"), "HÉLLO wörld\n");
    assert_eq!(text(t, 7, "rx"), "héllo xörld\n");
}

#[test]
fn unbound_ctrl_keys_reach_the_app() {
    let mut buf = FakeBuffer::new("x");
    let mut vim = VimState::new(false);
    assert_eq!(
        vim.feed(&Key::ctrl("s"), false, &mut buf),
        Outcome::PassThrough
    );
    assert_eq!(vim.feed(&Key::ctrl("d"), false, &mut buf), Outcome::Handled);
    // A pending count is dropped by a passthrough key.
    feed_str(&mut vim, &mut buf, "2");
    assert_eq!(
        vim.feed(&Key::ctrl("p"), false, &mut buf),
        Outcome::PassThrough
    );
    assert_eq!(caret("abc", 0, "l"), 1);
}
