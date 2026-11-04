//! Conformance with `tests/fixtures/golden/js_find_utils.json` (FrankMD
//! `app/javascript/lib/find_utils.js`).
//!
//! Offsets are UTF-16 code units, as in the fixture. Known deviation (see the
//! `find` module docs): Rust matches on Unicode scalars, so a match never
//! starts/ends inside a surrogate pair. For the few cases where JS (no `u`
//! flag) split an astral char — `.` returning lone surrogates, `x*` empty
//! matches at mid-pair positions — the check is: every expected match whose
//! boundaries are on scalar boundaries appears, in order, in the actual
//! result, and no actual boundary falls inside a pair.
//!
//! `validateRegex` error reasons are V8's wording; only the
//! `Invalid regular expression: /<pattern>/: ` prefix and a non-empty reason
//! are contractual (the text is shown in the find bar).
#![allow(clippy::unwrap_used)]

use frankmd_search::find::{
    Direction, FindOptions, Match, escape_regex_chars, find_all_matches, find_closest_match_index, replace_matches,
    validate_regex,
};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, decode_js, golden};

fn arg(input: &[Value], i: usize) -> &Value {
    input.get(i).unwrap_or(&Value::Null)
}

fn opt_bool(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn match_from_json(v: &Value) -> Match {
    Match {
        start: v["start"].as_u64().unwrap() as usize,
        end: v["end"].as_u64().unwrap() as usize,
        text: v.get("text").and_then(Value::as_str).unwrap_or_default().to_owned(),
        groups: v
            .get("groups")
            .and_then(Value::as_array)
            .map(|g| g.iter().map(|x| x.as_str().map(str::to_owned)).collect())
            .unwrap_or_default(),
    }
}

fn match_to_json(m: &Match) -> Value {
    json!({"start": m.start, "end": m.end, "text": m.text, "groups": m.groups})
}

fn matches_arg(v: &Value) -> Vec<Match> {
    v.as_array().map(|a| a.iter().map(match_from_json).collect()).unwrap_or_default()
}

/// UTF-16 offsets that fall between the two halves of a surrogate pair.
fn mid_pair_offsets(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut pos = 0;
    for c in text.chars() {
        if c.len_utf16() == 2 {
            out.push(pos + 1);
        }
        pos += c.len_utf16();
    }
    out
}

fn check_find_all(case: &Case, input: &[Value]) -> Result<(), String> {
    let text = arg(input, 0).as_str().unwrap_or_default();
    let query = arg(input, 1).as_str().unwrap_or_default();
    let o = arg(input, 2);
    let opts = FindOptions { case_sensitive: opt_bool(o, "caseSensitive"), use_regex: opt_bool(o, "useRegex"), whole_word: false };
    let actual: Vec<Value> = find_all_matches(text, query, opts).iter().map(match_to_json).collect();
    let expected: Vec<Value> = decode_js(&case.expected).as_array().unwrap().clone();

    let mid = mid_pair_offsets(text);
    let splits = |m: &Value| {
        let (s, e) = (m["start"].as_u64().unwrap() as usize, m["end"].as_u64().unwrap() as usize);
        mid.contains(&s) || mid.contains(&e)
    };
    if !expected.iter().any(splits) {
        return if actual == expected {
            Ok(())
        } else {
            Err(format!("\n  expected: {}\n  actual:   {}", Value::Array(expected), Value::Array(actual)))
        };
    }
    // Deviation mode (JS split a surrogate pair).
    if let Some(bad) = actual.iter().find(|m| splits(m)) {
        return Err(format!("match splits a surrogate pair: {bad}"));
    }
    let mut it = actual.iter();
    for want in expected.iter().filter(|m| !splits(m)) {
        if !it.any(|got| got == want) {
            return Err(format!("missing (or out of order) scalar-aligned match {want}"));
        }
    }
    Ok(())
}

fn check(case: &Case) -> Result<(), String> {
    let input_v = decode_js(&case.input);
    let input = input_v.as_array().unwrap().as_slice();
    let func = case.func.as_deref().unwrap();
    let expected = decode_js(&case.expected);
    let actual = match func {
        "escapeRegexChars" => json!(escape_regex_chars(arg(input, 0).as_str().unwrap_or_default())),
        "findAllMatches" => return check_find_all(case, input),
        "findClosestMatchIndex" => {
            let dir = match arg(input, 2) {
                Value::String(s) if s == "previous" => Direction::Previous,
                Value::Number(n) if n.as_i64() == Some(-1) => Direction::Previous,
                _ => Direction::Next,
            };
            let pos = arg(input, 1).as_i64().unwrap_or(0);
            let idx = find_closest_match_index(&matches_arg(arg(input, 0)), pos, dir);
            json!(idx.map_or(-1, |i| i as i64))
        }
        "replaceMatches" => {
            let text = arg(input, 0).as_str().unwrap_or_default();
            let use_regex = opt_bool(arg(input, 3), "useRegex");
            // JS: regex mode treats a falsy replacement as ""; literal mode would
            // template-stringify null as "null".
            let repl = match arg(input, 2) {
                Value::String(s) => s.clone(),
                _ if use_regex => String::new(),
                _ => "null".to_owned(),
            };
            json!(replace_matches(text, &matches_arg(arg(input, 1)), &repl, use_regex))
        }
        "validateRegex" => {
            let pattern = arg(input, 0).as_str().unwrap_or_default();
            let got = validate_regex(pattern);
            let want_valid = expected["valid"].as_bool().unwrap();
            return match (got, want_valid) {
                (Ok(()), true) => Ok(()),
                (Err(e), false) => {
                    let prefix = format!("Invalid regular expression: /{pattern}/: ");
                    if e.starts_with(&prefix) && e.len() > prefix.len() {
                        Ok(())
                    } else {
                        Err(format!("error {e:?} lacks prefix {prefix:?} (V8 said {})", expected["error"]))
                    }
                }
                (got, _) => Err(format!("expected {expected}, got {got:?}")),
            };
        }
        other => return Err(format!("unknown fn {other}")),
    };
    if actual == expected { Ok(()) } else { Err(format!("expected {expected}, got {actual}")) }
}

macro_rules! golden_fn {
    ($test:ident, $func:literal) => {
        #[test]
        fn $test() {
            let g = golden("js_find_utils");
            assert_cases(concat!("js_find_utils.json#", $func), g.cases_for($func), check);
        }
    };
}

golden_fn!(golden_escape_regex_chars, "escapeRegexChars");
golden_fn!(golden_find_all_matches, "findAllMatches");
golden_fn!(golden_find_closest_match_index, "findClosestMatchIndex");
golden_fn!(golden_replace_matches, "replaceMatches");
golden_fn!(golden_validate_regex, "validateRegex");
