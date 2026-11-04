//! Golden conformance for `frankmd_core::emoji` against
//! `js_emoji_data.json`, `js_emoji_i18n.json`, `js_icon_data.json`.
#![allow(clippy::unwrap_used)]

use frankmd_core::emoji::{self, i18n, icons};
use frankmd_testkit::serde_json::{self, Value, json};
use frankmd_testkit::{Case, assert_cases, decode_js, golden};

fn ensure(cond: bool, msg: impl FnOnce() -> String) -> Result<(), String> {
    if cond { Ok(()) } else { Err(msg()) }
}

fn eq(got: &Value, want: &Value) -> Result<(), String> {
    ensure(got == want, || {
        format!("\n  got:  {}\n  want: {}", trunc(got), trunc(want))
    })
}

fn trunc(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 400 {
        format!(
            "{}…",
            &s[..s
                .char_indices()
                .take_while(|(i, _)| *i < 400)
                .last()
                .map_or(0, |(i, c)| i + c.len_utf8())]
        )
    } else {
        s
    }
}

fn emoji_map_json() -> Value {
    Value::Object(
        emoji::emoji_map()
            .iter()
            .map(|(k, v)| ((*k).to_owned(), json!(v)))
            .collect(),
    )
}

fn icon_map_json() -> Value {
    Value::Object(
        icons::icon_map()
            .iter()
            .map(|(k, g)| {
                (
                    (*k).to_owned(),
                    json!({"path": g.path, "viewBox": g.view_box}),
                )
            })
            .collect(),
    )
}

// ---------------------------------------------------------------- emoji data

#[test]
fn golden_js_emoji_data() {
    let g = golden("js_emoji_data");
    assert_cases("js_emoji_data", &g.cases, |c: &Case| {
        let want = decode_js(&c.expected);
        match c.func.as_deref() {
            Some("EMOJI_DATA") => {
                let got: Vec<Value> = emoji::shortcodes()
                    .iter()
                    .map(|e| json!([e.shortcode, e.emoji, e.keywords]))
                    .collect();
                eq(&Value::Array(got), &want)
            }
            Some("EMOJI_DATA(shape)") => {
                let rows = emoji::shortcodes();
                let map = emoji::emoji_map();
                let mut seen = std::collections::HashSet::new();
                let dups: Vec<&str> = rows
                    .iter()
                    .filter(|e| !seen.insert(e.shortcode.as_str()))
                    .map(|e| e.shortcode.as_str())
                    .collect();
                let got = json!({
                    "entries": rows.len(),
                    "mapSize": map.len(),
                    "duplicateShortcodes": dups,
                    "entryArity": [3],
                    "cached": std::ptr::eq(map, emoji::emoji_map()),
                });
                eq(&got, &want)
            }
            Some("getEmojiMap") => {
                eq(&emoji_map_json(), &want)?;
                // lookup() must agree with the map for every shortcode.
                for (k, v) in want.as_object().unwrap() {
                    ensure(emoji::lookup(k) == v.as_str(), || {
                        format!("lookup({k:?}) != {v}")
                    })?;
                }
                ensure(
                    emoji::lookup("definitely_not_a_shortcode").is_none(),
                    || "unknown shortcode resolved".into(),
                )
            }
            other => Err(format!("unhandled fn {other:?}")),
        }
    });
}

// ---------------------------------------------------------------- emoji i18n

/// `{"$map": [[emoji, {label,tags,searchTerms}], ...]}` encoding of a SearchIndex.
fn index_json(idx: &i18n::SearchIndex) -> Value {
    let rows: Vec<Value> = idx
        .iter()
        .map(|(k, e)| json!([k, {"label": e.label, "tags": e.tags, "searchTerms": e.search_terms}]))
        .collect();
    json!({ "$map": rows })
}

fn entries_from(v: &Value) -> Vec<i18n::EmojibaseEntry> {
    serde_json::from_value(v.clone()).unwrap()
}

/// `public/emoji-data/pt.json` -> `pt`.
fn locale_of_file(file: &str) -> &str {
    file.rsplit('/').next().unwrap().trim_end_matches(".json")
}

fn key_counts(entries: &[i18n::EmojibaseEntry]) -> Value {
    let mut counts = serde_json::Map::new();
    for e in entries {
        if let Value::Object(o) = serde_json::to_value(e).unwrap() {
            for k in o.keys() {
                let n = counts.entry(k.clone()).or_insert(json!(0));
                *n = json!(n.as_u64().unwrap() + 1);
            }
        }
    }
    Value::Object(counts)
}

#[test]
fn golden_js_emoji_i18n() {
    let g = golden("js_emoji_i18n");
    assert_cases("js_emoji_i18n", &g.cases, |c: &Case| {
        let want = decode_js(&c.expected);
        match c.func.as_deref() {
            Some("setEmojiLocale+getEmojibaseLocale") => {
                let arg = decode_js(&c.input[0]);
                let got = i18n::emojibase_locale(arg.as_str());
                // Deliberate deviation (documented on `emojibase_locale`): JS returned
                // inherited Object.prototype members for these keys; we fall back to "en".
                let want = match arg.as_str() {
                    Some("toString" | "__proto__") => json!("en"),
                    _ => want,
                };
                eq(&json!(got), &want)
            }
            Some("buildSearchIndex") => {
                let idx = i18n::build_search_index(&entries_from(&c.input[0]));
                eq(&index_json(&idx), &want)
            }
            Some("emojiDataset(shape)") => {
                let loc = locale_of_file(c.input["file"].as_str().unwrap());
                let data = i18n::dataset(loc).ok_or_else(|| format!("no dataset for {loc}"))?;
                let got = json!({
                    "entries": data.len(),
                    "keyCounts": key_counts(data),
                    "first": serde_json::to_value(data.first().unwrap()).unwrap(),
                    "last": serde_json::to_value(data.last().unwrap()).unwrap(),
                    "indexSize": i18n::build_search_index(data).len(),
                });
                eq(&got, &want)
            }
            Some("getTranslatedEmojis") => {
                // `fetched` (URLs requested through the JS per-locale cache) has no
                // equivalent: data is embedded. Compare count + first row only.
                let data = i18n::translated_emojis(c.input["locale"].as_str());
                let got = json!({
                    "count": data.len(),
                    "first": serde_json::to_value(data.first().unwrap()).unwrap(),
                });
                let want = json!({"count": want["count"], "first": want["first"]});
                eq(&got, &want)
            }
            other => Err(format!("unhandled fn {other:?}")),
        }
    });
}

// ---------------------------------------------------------------- icons

#[test]
fn golden_js_icon_data() {
    let g = golden("js_icon_data");
    assert_cases("js_icon_data", &g.cases, |c: &Case| {
        let want = decode_js(&c.expected);
        let all = icons::icons();
        match c.func.as_deref() {
            Some("ICON_DATA") => {
                let got: Vec<Value> = all
                    .iter()
                    .map(|i| json!([i.name, i.path, i.view_box, i.keywords]))
                    .collect();
                eq(&Value::Array(got), &want)
            }
            Some("ICON_DATA(names)") => {
                let got: Vec<&str> = all.iter().map(|i| i.name.as_str()).collect();
                eq(&json!(got), &want)
            }
            Some("ICON_DATA(shape)") => {
                let map = icons::icon_map();
                let mut seen = std::collections::HashSet::new();
                let dups: Vec<&str> = all
                    .iter()
                    .filter(|i| !seen.insert(i.name.as_str()))
                    .map(|i| i.name.as_str())
                    .collect();
                let mut vb_seen = std::collections::HashSet::new();
                let vbs: Vec<&str> = all
                    .iter()
                    .map(|i| i.view_box.as_str())
                    .filter(|v| vb_seen.insert(*v))
                    .collect();
                let got = json!({
                    "entries": all.len(),
                    "mapSize": map.len(),
                    "duplicateNames": dups,
                    "viewBoxes": vbs,
                    "arity": [4],
                    "cached": std::ptr::eq(map, icons::icon_map()),
                });
                eq(&got, &want)
            }
            Some("getIconMap") => eq(&icon_map_json(), &want),
            other => Err(format!("unhandled fn {other:?}")),
        }
    });
}
