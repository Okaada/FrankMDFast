//! Golden conformance: tests/fixtures/golden/js_keyboard_shortcuts.json
//! (lib/keyboard_shortcuts.js). Every fn in the fixture is driven here.
//!
//! Notes:
//! - `"<DEFAULT_SHORTCUTS>"` in an input means the module table → `ShortcutMap::defaults()`.
//! - The testkit's serde_json does not preserve object key order. Shortcut tables given as
//!   JSON objects are rebuilt in DEFAULT_SHORTCUTS order (unknown keys appended, sorted);
//!   every harvested table is the default table or a one-key object, so nothing is lost.
//!   `mergeShortcuts` expectations recorded as objects are compared order-insensitively;
//!   those recorded as `Object.entries` arrays are compared in order.
//! - `getShortcutForAction(…, "constructor"|"toString")` returned inherited JS prototype
//!   functions; the Rust map has no prototype, so these 2 cases are skipped (documented quirk).
#![allow(clippy::unwrap_used)]

use frankmd_testkit::serde_json::{Map, Value, json};
use frankmd_testkit::{Case, assert_cases, decode_js, golden};
use frankmd_workspace::shortcuts::{
    DEFAULT_SHORTCUTS, KeyEvent, NavigatorInfo, Platform, Shortcut, ShortcutMap, format_shortcut,
    match_shortcut,
};

const FIXTURE: &str = "js_keyboard_shortcuts";

fn event(v: &Value) -> KeyEvent {
    let b = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    KeyEvent {
        key: v
            .get("key")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        ctrl: b("ctrlKey"),
        shift: b("shiftKey"),
        alt: b("altKey"),
        meta: b("metaKey"),
    }
}

/// JS shortcut value → Rust. `null`/`undefined`/non-object → None; `{}` → empty key.
fn shortcut(v: &Value) -> Option<Shortcut> {
    let o = v.as_object()?;
    let b = |k: &str| o.get(k).and_then(Value::as_bool).unwrap_or(false);
    let mut s = Shortcut::new(o.get("key").and_then(Value::as_str).unwrap_or(""));
    s.ctrl = b("ctrl");
    s.shift = b("shift");
    s.alt = b("alt");
    s.prevent_default = o.get("preventDefault") != Some(&Value::Bool(false));
    Some(s)
}

/// Rust shortcut → the JS object shape the fixture records (only truthy fields, plus
/// `preventDefault: false` when disabled).
fn shortcut_json(s: &Shortcut) -> Value {
    let mut o = Map::new();
    o.insert("key".into(), json!(s.key));
    if s.ctrl {
        o.insert("ctrl".into(), json!(true));
    }
    if s.shift {
        o.insert("shift".into(), json!(true));
    }
    if s.alt {
        o.insert("alt".into(), json!(true));
    }
    if !s.prevent_default {
        o.insert("preventDefault".into(), json!(false));
    }
    Value::Object(o)
}

fn ordered_keys(o: &Map<String, Value>) -> Vec<String> {
    let mut keys: Vec<String> = DEFAULT_SHORTCUTS
        .iter()
        .map(|(a, _)| (*a).to_owned())
        .filter(|a| o.contains_key(a))
        .collect();
    let mut rest: Vec<String> = o.keys().filter(|k| !keys.contains(k)).cloned().collect();
    rest.sort();
    keys.extend(rest);
    keys
}

fn table(v: &Value) -> ShortcutMap {
    if v.as_str() == Some("<DEFAULT_SHORTCUTS>") {
        return ShortcutMap::defaults();
    }
    let o = v.as_object().expect("shortcut table object");
    ShortcutMap::from_entries(ordered_keys(o).into_iter().map(|k| {
        let s = shortcut(&o[&k]).unwrap_or_else(|| panic!("table entry {k} is not a shortcut"));
        (k, s)
    }))
}

fn overrides(v: &Value) -> Vec<(String, Option<Shortcut>)> {
    match v.as_object() {
        None => Vec::new(), // undefined → {}
        Some(o) => ordered_keys(o)
            .into_iter()
            .map(|k| (k.clone(), shortcut(&o[&k])))
            .collect(),
    }
}

fn entries_json(m: &ShortcutMap) -> Value {
    Value::Array(
        m.entries()
            .iter()
            .map(|(a, s)| json!([a, shortcut_json(s)]))
            .collect(),
    )
}

fn check_eq(got: Value, want: &Value) -> Result<(), String> {
    if &got == want {
        Ok(())
    } else {
        Err(format!("got {got}, want {want}"))
    }
}

fn run(case: &Case) -> Result<(), String> {
    let input = decode_js(&case.input);
    let want = decode_js(&case.expected);
    let arg = |i: usize| input.get(i).cloned().unwrap_or(Value::Null);
    match case.func.as_deref().unwrap() {
        "DEFAULT_SHORTCUTS" => {
            check_eq(entries_json(&ShortcutMap::defaults()), &want).and_then(|()| {
                // The const table itself must match too.
                let c = ShortcutMap::from_entries(DEFAULT_SHORTCUTS.iter().cloned());
                check_eq(entries_json(&c), &want)
            })
        }
        // JS: SHORTCUTS === DEFAULT_SHORTCUTS. Rust has a single table.
        "SHORTCUTS" => check_eq(
            json!(ShortcutMap::defaults().entries().len() == DEFAULT_SHORTCUTS.len()),
            &want,
        ),
        "matchShortcut" => check_eq(
            json!(match_shortcut(&event(&arg(0)), shortcut(&arg(1)).as_ref())),
            &want,
        ),
        "formatShortcut" => {
            let mac = arg(1).as_bool().unwrap_or(false);
            check_eq(
                json!(format_shortcut(shortcut(&arg(0)).as_ref(), mac)),
                &want,
            )
        }
        "findMatchingAction" => {
            let m = table(&arg(1));
            check_eq(json!(m.find_matching_action(&event(&arg(0)))), &want)
        }
        "createKeyHandler" => {
            let d = table(&arg(1)).dispatch(&event(&arg(0)));
            let got = match d {
                Some(d) => {
                    json!({"actions": [d.action], "preventDefaultCalls": u8::from(d.prevent_default)})
                }
                None => json!({"actions": [], "preventDefaultCalls": 0}),
            };
            check_eq(got, &want)
        }
        "mergeShortcuts" => {
            let merged = table(&arg(0)).merge(overrides(&arg(1)));
            if want.is_array() {
                check_eq(entries_json(&merged), &want)
            } else {
                // Recorded as a plain object: order-insensitive comparison.
                let got: Map<String, Value> = merged
                    .entries()
                    .iter()
                    .map(|(a, s)| (a.clone(), shortcut_json(s)))
                    .collect();
                check_eq(Value::Object(got), &want)
            }
        }
        "getShortcutForAction" => {
            let m = table(&arg(0));
            let action = arg(1);
            check_eq(
                m.get(action.as_str().unwrap())
                    .map_or(Value::Null, shortcut_json),
                &want,
            )
        }
        "isMacOS" => {
            let nav = &input["navigator"];
            let info = nav.as_object().map(|o| NavigatorInfo {
                user_agent_data_platform: o.get("userAgentData").filter(|v| v.is_object()).map(
                    |d| {
                        d.get("platform")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned()
                    },
                ),
                user_agent: o
                    .get("userAgent")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            });
            check_eq(
                json!(Platform::from_navigator(info.as_ref()).is_macos()),
                &want,
            )
        }
        other => Err(format!("unhandled fn {other}")),
    }
}

/// JS prototype artefacts (`"constructor"`, `"toString"`) — see module notes.
fn is_prototype_quirk(case: &Case) -> bool {
    case.expected.get("$function").is_some()
}

macro_rules! golden_fn {
    ($test:ident, $func:literal) => {
        #[test]
        fn $test() {
            let g = golden(FIXTURE);
            assert_cases(
                &format!("{FIXTURE}#{}", $func),
                g.cases_for($func).filter(|c| !is_prototype_quirk(c)),
                run,
            );
        }
    };
}

golden_fn!(golden_default_shortcuts, "DEFAULT_SHORTCUTS");
golden_fn!(golden_shortcuts_alias, "SHORTCUTS");
golden_fn!(golden_match_shortcut, "matchShortcut");
golden_fn!(golden_format_shortcut, "formatShortcut");
golden_fn!(golden_find_matching_action, "findMatchingAction");
golden_fn!(golden_create_key_handler, "createKeyHandler");
golden_fn!(golden_merge_shortcuts, "mergeShortcuts");
golden_fn!(golden_get_shortcut_for_action, "getShortcutForAction");
golden_fn!(golden_is_macos, "isMacOS");

#[test]
fn golden_every_case_is_wired() {
    let g = golden(FIXTURE);
    let known = [
        "DEFAULT_SHORTCUTS",
        "SHORTCUTS",
        "matchShortcut",
        "formatShortcut",
        "findMatchingAction",
        "createKeyHandler",
        "mergeShortcuts",
        "getShortcutForAction",
        "isMacOS",
    ];
    for c in &g.cases {
        assert!(
            known.contains(&c.func.as_deref().unwrap_or("")),
            "unwired case {}",
            c.name
        );
    }
    assert_eq!(g.cases.iter().filter(|c| is_prototype_quirk(c)).count(), 2);
}
