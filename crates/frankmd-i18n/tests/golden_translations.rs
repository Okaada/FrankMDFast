//! Conformance against `tests/fixtures/golden/translations.json` (37 cases) and the
//! `translations/<locale>.flat.json` / `translations/served_*.json` bodies.

use std::collections::BTreeMap;

use frankmd_config::{EnvSnapshot, Key, ResolvedConfig, Value};
use frankmd_i18n::{
    Leaf, Locale, Node, flatten_leaves, interpolation_vars, key_present, locale_tree,
    resolve_locale, served_translations, translation_failures,
};
use frankmd_testkit::serde_json::{self, Value as Json, json};
use frankmd_testkit::{Case, assert_cases, golden, golden_bytes, golden_path};

fn locale(code: &str) -> Result<Locale, String> {
    Locale::from_code(code).ok_or_else(|| format!("unknown locale {code}"))
}

fn leaf_json(l: &Leaf) -> Json {
    Node::Leaf(l.clone()).to_json()
}

fn load_json(rel: &str) -> Json {
    serde_json::from_slice(&golden_bytes(rel)).expect("golden json")
}

fn eq(actual: &Json, expected: &Json) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected}\n     got {actual}"))
    }
}

#[test]
fn locales() {
    let g = golden("translations");
    assert_cases("translations locales", g.cases_for("locales"), |c| {
        let codes: Vec<&str> = Locale::ALL.iter().map(|l| l.code()).collect();
        eq(&json!(codes), &c.expected["available_locales"])?;
        for (code, chain) in c.expected["i18n_fallbacks_chain"]
            .as_object()
            .ok_or("chain")?
        {
            eq(&json!(locale(code)?.fallback_chain()), chain)?;
        }
        Ok(())
    });
}

#[test]
fn flatten_leaves_matches_flat_files() {
    let g = golden("translations");
    assert_cases(
        "translations flatten_leaves",
        g.cases_for("flatten_leaves"),
        |c| {
            let l = locale(c.input["locale"].as_str().ok_or("locale")?)?;
            let leaves = flatten_leaves(&locale_tree(l).map_err(|e| e.to_string())?);
            if leaves.len() as u64 != c.expected["leaf_count"].as_u64().ok_or("leaf_count")? {
                return Err(format!("leaf_count {}", leaves.len()));
            }
            let mut keys: Vec<&str> = leaves.iter().map(|(k, _)| k.as_str()).collect();
            keys.sort_unstable();
            eq(&json!(keys), &c.expected["keys"])?;
            let values: serde_json::Map<String, Json> = leaves
                .iter()
                .map(|(k, v)| (k.clone(), leaf_json(v)))
                .collect();
            let file = c.expected["values_file"].as_str().ok_or("values_file")?;
            eq(&Json::Object(values), &load_json(file))
        },
    );
}

#[test]
fn interpolation_vars_per_locale() {
    let g = golden("translations");
    let en = interpolation_vars(&locale_tree(Locale::En).expect("en"));
    assert_cases(
        "translations interpolation_vars",
        g.cases_for("interpolation_vars"),
        |c| {
            let l = locale(c.input["locale"].as_str().ok_or("locale")?)?;
            let vars = interpolation_vars(&locale_tree(l).map_err(|e| e.to_string())?);
            eq(&json!(vars), &c.expected["vars"])?;
            let differing: Vec<&String> = vars
                .iter()
                .filter(|(k, v)| en.get(*k) != Some(*v))
                .map(|(k, _)| k)
                .collect();
            eq(
                &json!(differing),
                &c.expected["keys_with_vars_differing_from_en"],
            )
        },
    );
}

#[test]
fn required_keys() {
    let g = golden("translations");
    assert_cases(
        "translations required_keys",
        g.cases_for("required_keys"),
        |c| {
            let keys: Vec<&str> = c.input["keys"]
                .as_array()
                .ok_or("keys")?
                .iter()
                .filter_map(Json::as_str)
                .collect();
            if let Some(present) = c.expected.get("present") {
                // i18n_*_keys_test.rb shape: {present: {loc: {key: bool}}, all_present}
                let mut all = true;
                let mut got = serde_json::Map::new();
                for code in c.input["locales"]
                    .as_array()
                    .ok_or("locales")?
                    .iter()
                    .filter_map(Json::as_str)
                {
                    let tree = locale_tree(locale(code)?).map_err(|e| e.to_string())?;
                    let row: serde_json::Map<String, Json> = keys
                        .iter()
                        .map(|k| {
                            let p = key_present(&tree, k);
                            all &= p;
                            ((*k).to_owned(), Json::Bool(p))
                        })
                        .collect();
                    got.insert(code.to_owned(), Json::Object(row));
                }
                eq(&Json::Object(got), present)?;
                return eq(&Json::Bool(all), &c.expected["all_present"]);
            }
            // translations_controller_test.rb shape: {loc: {key: value}} (section prefix or full paths)
            let section = c.input["section"].as_str();
            for (code, expected_row) in c.expected.as_object().ok_or("expected map")? {
                let served = served_translations(locale(code)?).map_err(|e| e.to_string())?;
                let tr = &served["translations"];
                let row: serde_json::Map<String, Json> = keys
                    .iter()
                    .map(|k| {
                        let path = section.map_or_else(|| (*k).to_owned(), |s| format!("{s}.{k}"));
                        (
                            (*k).to_owned(),
                            path.split('.').fold(tr, |n, seg| &n[seg]).clone(),
                        )
                    })
                    .collect();
                eq(&Json::Object(row), expected_row).map_err(|e| format!("{code}: {e}"))?;
            }
            Ok(())
        },
    );
}

#[test]
fn translation_failures_cases() {
    let g = golden("translations");
    // serde_json maps are sorted; the Ruby hash order matters for the report, so the
    // locale order is taken from the raw fixture text.
    // Mechanical fix: the fixture is pretty-printed, so whitespace is dropped before
    // searching for the compact `"<loc>":{"dialogs"` pattern (only positions matter).
    let raw: String = String::from_utf8(golden_bytes("translations.json"))
        .expect("utf8")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert_cases(
        "translations translation_failures",
        g.cases_for("translation_failures"),
        |c: &Case| {
            let actual = if c.input["english"].is_string() {
                let english = locale_tree(Locale::En).map_err(|e| e.to_string())?;
                let codes: Vec<&str> = c.input["locales"]
                    .as_array()
                    .ok_or("locales")?
                    .iter()
                    .filter_map(Json::as_str)
                    .collect();
                let trees: Vec<(&str, Node)> = codes
                    .iter()
                    .map(|code| {
                        Ok((
                            *code,
                            locale_tree(locale(code)?).map_err(|e| e.to_string())?,
                        ))
                    })
                    .collect::<Result<_, String>>()?;
                let refs: Vec<(&str, &Node)> = trees.iter().map(|(c, t)| (*c, t)).collect();
                translation_failures(&english, &refs)
            } else {
                let english = Node::from_json(&c.input["english"]);
                let map = c.input["translations"].as_object().ok_or("translations")?;
                let mut codes: Vec<&String> = map.keys().collect();
                codes.sort_by_key(|k| {
                    raw.find(&format!("\"{k}\":{{\"dialogs\""))
                        .unwrap_or(usize::MAX)
                });
                let trees: Vec<(&str, Node)> = codes
                    .iter()
                    .map(|k| (k.as_str(), Node::from_json(&map[k.as_str()])))
                    .collect();
                let refs: Vec<(&str, &Node)> = trees.iter().map(|(c, t)| (*c, t)).collect();
                translation_failures(&english, &refs)
            };
            eq(&json!(actual), &c.expected)
        },
    );
}

#[test]
fn translations_controller_show() {
    let g = golden("translations");
    assert_cases(
        "translations controller#show",
        g.cases_for("translations_controller#show"),
        |c| {
            let env_locale = c.input["env"]["FRANKMD_LOCALE"].as_str();
            let env = EnvSnapshot::from_pairs(env_locale.map(|v| ("FRANKMD_LOCALE", v)));
            let cfg = ResolvedConfig::new(c.input["fed"].as_str(), env);
            let configured = match cfg.value(Key::Locale) {
                Some(Value::Str(s)) => Some(s),
                _ => None,
            };
            let l = resolve_locale(c.input["query_locale"].as_str(), configured.as_deref());
            let body = served_translations(l).map_err(|e| e.to_string())?;
            eq(&body["locale"], &c.expected["locale"])?;
            let file = c.expected["body_file"].as_str().ok_or("body_file")?;
            eq(&body, &load_json(file))?;
            // leaf count + the keys that came from en / Rails gem locale files
            let tree = Node::from_json(&body["translations"]);
            let leaves: BTreeMap<String, Leaf> = flatten_leaves(&tree).into_iter().collect();
            if leaves.len() as u64 != c.expected["leaf_count"].as_u64().ok_or("leaf_count")? {
                return Err(format!("leaf_count {}", leaves.len()));
            }
            let own = locale_tree(l).map_err(|e| e.to_string())?;
            let fill: Vec<&String> = leaves
                .keys()
                .filter(|k| !key_present_any(&own, k))
                .collect();
            eq(&json!(fill), &c.expected["en_fill_keys"])
        },
    );
    assert!(golden_path("translations/served_en.json").exists());
}

/// Leaf exists in the locale's own YAML (any value).
fn key_present_any(tree: &Node, dotted: &str) -> bool {
    dotted
        .split('.')
        .try_fold(tree, |n, seg| n.get(seg))
        .is_some_and(|n| matches!(n, Node::Leaf(_)))
}
