//! Golden conformance: `tests/fixtures/golden/js_codemirror_wikilink.json`,
//! the `[[` completion source (`wikilinkCompletions`) through
//! [`frankmd_core::wikilink::complete`].
//!
//! Fixture positions are UTF-16 code units; the Rust API uses UTF-8 byte
//! offsets, so the test converts `pos` in and `from`/`to`/`anchor` out.
//! `files: null` and `"<null provider>"` both mean "no files" (empty slice).
//! `filter: false` is CodeMirror wiring and is constant. The two
//! `createWikilinkAutocomplete(config)` cases record CodeMirror
//! `autocompletion()` options (activateOnTyping, maxRenderedOptions, icons,
//! extra sources): UI configuration for frankmd-app, not ported here.
#![allow(missing_docs, clippy::unwrap_used)]

use frankmd_core::wikilink::{WikiFile, complete};
use frankmd_testkit::serde_json::{self, Value, json};
use frankmd_testkit::{assert_cases, decode_js, golden};

fn utf16_to_byte(s: &str, units: usize) -> usize {
    let mut n = 0;
    for (b, ch) in s.char_indices() {
        if n >= units {
            return b;
        }
        n += ch.len_utf16();
    }
    s.len()
}

fn byte_to_utf16(s: &str, byte: usize) -> usize {
    s[..byte].encode_utf16().count()
}

#[test]
fn golden_wikilink_completions() {
    let g = golden("js_codemirror_wikilink");
    assert_cases(
        "js_codemirror_wikilink",
        g.cases_for("wikilinkCompletions"),
        |c| {
            let input = decode_js(&c.input);
            let doc = input["doc"].as_str().unwrap();
            let pos = utf16_to_byte(doc, input["pos"].as_u64().unwrap() as usize);
            let files: Vec<WikiFile> = match &input["files"] {
                Value::Array(_) => {
                    serde_json::from_value(input["files"].clone()).map_err(|e| e.to_string())?
                }
                _ => vec![], // null or "<null provider>"
            };
            let got = complete(doc, pos, &files).map(|r| {
                let from = byte_to_utf16(doc, r.from);
                let to = byte_to_utf16(doc, r.to);
                json!({
                    "from": from,
                    "filter": false,
                    "options": r.options.iter().map(|o| json!({
                        "label": o.label,
                        "detail": o.detail,
                        "boost": o.boost,
                        "apply": {
                            "changes": {"from": from, "to": to, "insert": o.insert},
                            "selection": {"anchor": from + o.insert.encode_utf16().count()},
                        },
                    })).collect::<Vec<_>>(),
                })
            });
            let got = got.unwrap_or(Value::Null);
            let want = decode_js(&c.expected);
            if got == want {
                Ok(())
            } else {
                Err(format!("\n    expected {want}\n    got      {got}"))
            }
        },
    );
}

#[test]
fn autocompletion_config_cases_are_ui_only() {
    let g = golden("js_codemirror_wikilink");
    assert_eq!(g.cases_for("createWikilinkAutocomplete(config)").count(), 2);
}
