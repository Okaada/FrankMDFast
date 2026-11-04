//! Golden conformance: `custom_theme.json`, `js_custom_theme.json`,
//! `js_codemirror_theme.json`, `omarchy_theme.json`, plus the embedded
//! `assets/themes/builtin.json`.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_testkit::{Case, assert_cases, decode_js, golden};
use frankmd_theme::custom::{
    applicable_overrides, contrast_ratio_hex, normalize_color, theme_color,
};
use frankmd_theme::editor::{Decoration, editor_chrome, markdown_highlights};
use frankmd_theme::{BASE_THEMES, COLOR_KEYS, CustomTheme, CustomThemeError, Mode, legacy};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;

// ------------------------------------------------------------------ builtin.json

#[test]
fn builtin_json_has_18_themes_with_all_38_roles() {
    let raw: J = serde_json::from_str(frankmd_theme::BUILTIN_THEMES_JSON).unwrap();
    let themes = raw["themes"].as_object().unwrap();
    assert_eq!(themes.len(), 18);
    assert_eq!(frankmd_theme::builtin_ids().len(), 18);
    let mut failures = Vec::new();
    for (id, t) in themes {
        let colors = t["colors"].as_object().unwrap();
        let mut keys: Vec<&str> = colors.keys().map(String::as_str).collect();
        let mut want = COLOR_KEYS.to_vec();
        keys.sort_unstable();
        want.sort_unstable();
        if keys != want {
            failures.push(format!("{id}: roles {keys:?}"));
        }
        let Some(tokens) = frankmd_theme::builtin(id) else {
            failures.push(format!("{id}: builtin() is None"));
            continue;
        };
        if tokens.colors.len() != 38 {
            failures.push(format!("{id}: {} tokens", tokens.colors.len()));
        }
        for (k, v) in colors {
            let got = tokens.get(k).map(|c| c.to_hex());
            if got.as_deref() != Some(v.as_str().unwrap().to_lowercase().as_str()) {
                failures.push(format!("{id}.{k}: {got:?} vs {v}"));
            }
        }
        let want_mode = if legacy::dark_background(colors["bg-primary"].as_str()) {
            Mode::Dark
        } else {
            Mode::Light
        };
        if tokens.mode != want_mode {
            failures.push(format!("{id}: mode {:?}", tokens.mode));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    for id in [
        "light",
        "solarized-light",
        "catppuccin-latte",
        "flexoki-light",
        "rose-pine",
    ] {
        assert_eq!(
            frankmd_theme::builtin(id).unwrap().mode,
            Mode::Light,
            "{id}"
        );
    }
    for id in ["dark", "tokyo-night", "nord", "gruvbox"] {
        assert_eq!(frankmd_theme::builtin(id).unwrap().mode, Mode::Dark, "{id}");
    }
    assert_eq!(frankmd_theme::builtin("omarchy"), None);
    assert_eq!(frankmd_theme::builtin("custom"), None);
}

// ------------------------------------------------------------------ custom_theme.json (Ruby)

#[test]
fn golden_custom_theme_constants() {
    let g: J = serde_json::from_slice(&frankmd_testkit::golden_bytes("custom_theme.json")).unwrap();
    assert_eq!(g["version"], json!(frankmd_theme::custom::VERSION));
    assert_eq!(g["max_bytes"], json!(frankmd_theme::custom::MAX_BYTES));
    assert_eq!(g["base_themes"], json!(BASE_THEMES.as_slice()));
    assert_eq!(g["color_keys"], json!(COLOR_KEYS.as_slice()));
}

fn custom_error_variant(e: &CustomThemeError) -> &'static str {
    match e {
        CustomThemeError::Json(_) => "JSON::ParserError",
        _ => "ArgumentError",
    }
}

#[test]
fn golden_custom_theme_parse() {
    let g = golden("custom_theme");
    assert_cases("custom_theme", &g.cases, |c| {
        let (result, raw_text) = if let Some(raw) = c.input.get("raw_json").and_then(J::as_str) {
            (CustomTheme::parse(raw), Some(raw.to_owned()))
        } else {
            (CustomTheme::from_value(&c.input["raw"]), None)
        };
        match result {
            Err(e) => {
                let want_class = c.expected["error"]
                    .as_str()
                    .ok_or_else(|| format!("unexpected error {e}"))?;
                eq(
                    "error class",
                    &json!(custom_error_variant(&e)),
                    &json!(want_class),
                )?;
                if want_class == "ArgumentError" {
                    eq(
                        "error message",
                        &json!(e.to_string()),
                        &c.expected["message"],
                    )?;
                }
                Ok(())
            }
            Ok(None) => eq("result", &J::Null, &c.expected),
            Ok(Some(t)) => {
                eq("result", &t.to_json(), &c.expected)?;
                eq(
                    "to_json_string roundtrip",
                    &serde_json::from_str(&t.to_json_string()).unwrap(),
                    &c.expected,
                )?;
                // colors keep input order (Ruby hash order; serde_json maps here may not).
                if let Some(raw) = raw_text {
                    let mut by_pos: Vec<(usize, &str)> = t
                        .colors
                        .iter()
                        .map(|(k, _)| (raw.rfind(&format!("\"{k}\"")).unwrap(), k.as_str()))
                        .collect();
                    let order: Vec<&str> = t.colors.iter().map(|(k, _)| k.as_str()).collect();
                    by_pos.sort_unstable();
                    let want: Vec<&str> = by_pos.into_iter().map(|(_, k)| k).collect();
                    eq("colors order", &json!(order), &json!(want))?;
                }
                Ok(())
            }
        }
    });
}

#[test]
fn custom_theme_json_string_is_compact_ruby_json() {
    let t = CustomTheme::parse(r##"{"version":1,"name":"My \"theme\"","base":"dark","colors":{"text-primary":"#111111","accent":"#AABBCC"}}"##)
        .unwrap()
        .unwrap();
    assert_eq!(
        t.to_json_string(),
        r##"{"version":1,"name":"My \"theme\"","base":"dark","colors":{"text-primary":"#111111","accent":"#aabbcc"}}"##
    );
}

// ------------------------------------------------------------------ js_custom_theme.json

fn arg0(c: &Case) -> J {
    decode_js(&c.input[0])
}

#[test]
fn golden_js_color_keys() {
    let g = golden("js_custom_theme");
    assert_cases("js COLOR_KEYS", g.cases_for("COLOR_KEYS"), |c| {
        eq("COLOR_KEYS", &json!(COLOR_KEYS.as_slice()), &c.expected)
    });
}

#[test]
fn golden_js_normalize_color() {
    let g = golden("js_custom_theme");
    assert_cases("js normalizeColor", g.cases_for("normalizeColor"), |c| {
        let got = match arg0(c) {
            J::String(s) => json!(normalize_color(&s)),
            _ => J::Null, // non-strings → null (typeof check); unrepresentable as &str
        };
        eq("normalizeColor", &got, &decode_js(&c.expected))
    });
}

#[test]
fn golden_js_contrast_ratio() {
    let g = golden("js_custom_theme");
    assert_cases("js contrastRatio", g.cases_for("contrastRatio"), |c| {
        let (a, b) = (decode_js(&c.input[0]), decode_js(&c.input[1]));
        let got = match (a.as_str(), b.as_str()) {
            (Some(a), Some(b)) => contrast_ratio_hex(a, b),
            _ => None,
        };
        match (&c.expected, got) {
            // JS quirk: padded input normalizes but `luminance` slices the raw
            // string → NaN. We compute on the normalized colors instead.
            (J::Object(o), Some(r)) if o.get("$number") == Some(&json!("NaN")) => {
                let (na, nb) = (
                    normalize_color(a.as_str().unwrap()).unwrap(),
                    normalize_color(b.as_str().unwrap()).unwrap(),
                );
                let want = contrast_ratio_hex(&na, &nb).unwrap();
                if r.is_finite() && approx(r, want) {
                    Ok(())
                } else {
                    Err(format!("got {r}"))
                }
            }
            (J::Null, None) => Ok(()),
            (J::Number(n), Some(r)) if approx(r, n.as_f64().unwrap()) => Ok(()),
            (want, got) => Err(format!("got {got:?} want {want}")),
        }
    });
}

#[test]
fn golden_js_theme_color() {
    let g = golden("js_custom_theme");
    assert_cases("js themeColor", g.cases_for("themeColor"), |c| {
        let raw = c.input["getPropertyValue"].as_str().unwrap();
        // `propertyRead` is always `--theme-<key>`: implicit in the token name.
        eq(
            "themeColor",
            &json!(theme_color(raw)),
            &c.expected["result"],
        )
    });
}

/// `"--editor-width: 72ch; --theme-accent: #abcdef;"` → theme pairs.
fn style_pairs(style: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = style
        .split(';')
        .filter_map(|d| d.split_once(':'))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .filter_map(|(k, v)| k.strip_prefix("--theme-").map(|k| (k.to_owned(), v)))
        .collect();
    v.sort();
    v
}

#[test]
fn golden_js_apply_custom_colors() {
    let g = golden("js_custom_theme");
    assert_cases(
        "js applyCustomColors",
        g.cases_for("applyCustomColors"),
        |c| {
            let mut got: Vec<(String, String)> = applicable_overrides(&arg0(c))
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect();
            got.sort();
            let want = style_pairs(c.expected["styleAfterApply"].as_str().unwrap());
            eq("applied", &json!(got), &json!(want))?;
            // clearCustomColors leaves no --theme-* behind (base tokens again).
            eq(
                "after clear",
                &json!(style_pairs(c.expected["styleAfterClear"].as_str().unwrap())),
                &json!([]),
            )
        },
    );
}

#[test]
fn golden_js_theme_palette_matches_builtin() {
    let g = golden("js_custom_theme");
    let raw: J = serde_json::from_str(frankmd_theme::BUILTIN_THEMES_JSON).unwrap();
    assert_cases("js themePalette", g.cases_for("themePalette"), |c| {
        let file = c.input["file"].as_str().unwrap();
        let id = file.rsplit('/').next().unwrap().trim_end_matches(".css");
        eq(
            "vars (raw)",
            &raw["themes"][id]["colors"],
            &c.expected["vars"],
        )?;
        let tokens = frankmd_theme::builtin(id).ok_or("missing builtin")?;
        let got: BTreeMap<&str, String> = COLOR_KEYS
            .iter()
            .map(|k| (*k, tokens.get(k).unwrap().to_hex()))
            .collect();
        eq("themeColor", &json!(got), &c.expected["themeColor"])?;
        for (role, want) in c.expected["syntaxContrastVsCodeBg"].as_object().unwrap() {
            let r = frankmd_theme::contrast_ratio(
                tokens.get(&format!("syntax-{role}")).unwrap(),
                tokens.get("code-bg").unwrap(),
            );
            if !approx(r, want.as_f64().unwrap()) {
                return Err(format!("{role}: {r} vs {want}"));
            }
        }
        Ok(())
    });
}

// ------------------------------------------------------------------ js_codemirror_theme.json

fn css_var(v: &J) -> Option<&str> {
    v.as_str()?.strip_prefix("var(--theme-")?.strip_suffix(')')
}

#[test]
fn golden_codemirror_syntax_highlighting() {
    let g = golden("js_codemirror_theme");
    assert_cases(
        "createSyntaxHighlighting",
        g.cases_for("createSyntaxHighlighting"),
        |c| {
            let specs = c.expected["specs"].as_array().unwrap();
            let rules = markdown_highlights();
            eq("rule count", &json!(rules.len()), &json!(specs.len()))?;
            for (r, s) in rules.iter().zip(specs) {
                let want = json!({
                    "tag": s["tag"][0],
                    "color": css_var(&s["color"]),
                    "background": s.get("backgroundColor").and_then(css_var),
                    "bold": s.get("fontWeight") == Some(&json!("bold")),
                    "italic": s.get("fontStyle") == Some(&json!("italic")),
                    "decoration": match s.get("textDecoration").and_then(J::as_str) {
                        Some("underline") => json!("underline"),
                        Some("line-through") => json!("line-through"),
                        _ => J::Null,
                    },
                });
                let got = json!({
                    "tag": r.tag, "color": r.color, "background": r.background, "bold": r.bold, "italic": r.italic,
                    "decoration": match r.decoration {
                        Some(Decoration::Underline) => json!("underline"),
                        Some(Decoration::LineThrough) => json!("line-through"),
                        None => J::Null,
                    },
                });
                eq(&format!("rule {}", r.tag), &got, &want)?;
            }
            Ok(())
        },
    );
}

/// `var(--theme-X)` used for `prop` in the first CSS rule whose selector contains `sel`.
fn rule_var(css: &str, sel: &str, prop: &str) -> Option<String> {
    let line = css
        .lines()
        .find(|l| l.split('{').next().is_some_and(|s| s.contains(sel)))?;
    let body = line.split_once('{')?.1;
    body.split(';').find_map(|d| {
        let (k, v) = d.split_once(':')?;
        (k.trim() == prop).then(|| {
            v.split("var(--theme-")
                .nth(1)
                .map(|r| r.split(')').next().unwrap().to_owned())
        })?
    })
}

#[test]
fn golden_codemirror_editor_theme() {
    let g = golden("js_codemirror_theme");
    let ch = editor_chrome();
    assert_cases("createEditorTheme", g.cases_for("createEditorTheme"), |c| {
        let css = c.expected["css"].as_str().unwrap();
        let checks = [
            (".<SCOPE> {", "background-color", ch.background),
            (".<SCOPE> {", "color", ch.text),
            (".cm-selectionBackground", "background-color", ch.selection),
            (".cm-gutters", "background-color", ch.gutter_background),
            (".cm-gutters", "color", ch.gutter_text),
            (".cm-gutters", "border-right", ch.gutter_border),
            (".cm-activeLineGutter", "color", ch.active_gutter_text),
            (".cm-placeholder", "color", ch.placeholder),
            (".cm-searchMatch {", "background-color", ch.search_match),
            (".cm-searchMatch {", "outline", ch.search_match_outline),
            (
                ".cm-searchMatch-selected",
                "background-color",
                ch.search_match_selected,
            ),
            (".cm-panels", "background-color", ch.panel_background),
        ];
        for (sel, prop, token) in checks {
            let sel = sel.trim_end_matches(" {");
            eq(
                &format!("{sel} {prop}"),
                &json!(rule_var(css, sel, prop)),
                &json!(token),
            )?;
        }
        if c.input[0] == json!({"$undefined": true}) || c.input[0] == json!({}) {
            let f = frankmd_theme::editor::EditorFont::default();
            if !css.contains(&format!("font-size: {}px", f.size_px))
                || !css.contains(&format!("font-family: {};", f.family))
            {
                return Err(format!("defaults {f:?} not in css"));
            }
            if !css.contains(&format!("line-height: {};", f.line_height)) {
                return Err("line-height default".into());
            }
        }
        if !css.contains(&format!(
            "max-height: {}em",
            frankmd_theme::editor::AUTOCOMPLETE_MAX_HEIGHT_EM
        )) {
            return Err("autocomplete max-height".into());
        }
        Ok(())
    });
}

// skip: js_codemirror_theme.json "createTheme shape" ({length: 2}) — CodeMirror extension array, no native analog.

// ------------------------------------------------------------------ omarchy_theme.json (legacy port)

fn f(c: &Case, k: &str) -> f64 {
    c.input[k].as_f64().unwrap()
}
fn st<'a>(c: &'a Case, k: &str) -> &'a str {
    c.input[k].as_str().unwrap()
}

#[test]
fn golden_omarchy_color_math() {
    let g = golden("omarchy_theme");
    let math = [
        "dark_background?",
        "relative_luminance",
        "lighten",
        "darken",
        "blend",
        "contrast_ratio",
        "ensure_contrast",
        "rgb_to_hex",
        "hex_to_rgb",
    ];
    let cases = g
        .cases
        .iter()
        .filter(|c| math.contains(&c.func.as_deref().unwrap_or("")));
    assert_cases("omarchy color math", cases, |c| {
        let got = match c.func.as_deref().unwrap() {
            "dark_background?" => json!(legacy::dark_background(c.input["hex"].as_str())),
            "relative_luminance" => {
                let r = legacy::relative_luminance(st(c, "hex"));
                return if approx(r, c.expected.as_f64().unwrap()) {
                    Ok(())
                } else {
                    Err(format!("{r}"))
                };
            }
            "contrast_ratio" => {
                let r = legacy::contrast_ratio(st(c, "fg"), st(c, "bg"));
                return if approx(r, c.expected.as_f64().unwrap()) {
                    Ok(())
                } else {
                    Err(format!("{r}"))
                };
            }
            "lighten" => json!(legacy::lighten(st(c, "hex"), f(c, "percent"))),
            "darken" => json!(legacy::darken(st(c, "hex"), f(c, "percent"))),
            "blend" => json!(legacy::blend(st(c, "fg"), st(c, "bg"), f(c, "factor"))),
            "ensure_contrast" => json!(legacy::ensure_contrast(st(c, "fg"), st(c, "bg"))),
            "rgb_to_hex" => json!(legacy::rgb_to_hex(f(c, "r"), f(c, "g"), f(c, "b"))),
            "hex_to_rgb" => match legacy::hex_to_rgb(st(c, "hex")) {
                Ok(v) => json!(v),
                Err(_) => {
                    return if c.expects_error() {
                        Ok(())
                    } else {
                        Err("unexpected error".into())
                    };
                }
            },
            _ => unreachable!(),
        };
        eq(c.func.as_deref().unwrap(), &got, &c.expected)
    });
}

fn pairs_obj(v: &[(String, String)]) -> J {
    J::Object(v.iter().map(|(k, v)| (k.clone(), json!(v))).collect())
}

#[test]
fn golden_omarchy_parse_and_map() {
    let g = golden("omarchy_theme");
    assert_cases(
        "omarchy parse_alacritty",
        g.cases_for("parse_alacritty"),
        |c| {
            eq(
                "parse_alacritty",
                &json!(legacy::parse_alacritty(st(c, "content"))),
                &c.expected,
            )
        },
    );
    assert_cases("omarchy map_variables", g.cases_for("map_variables"), |c| {
        let colors: BTreeMap<String, String> =
            serde_json::from_value(c.input["colors"].clone()).unwrap();
        let vars = legacy::map_variables(&colors);
        let names: Vec<&str> = vars.iter().map(|(k, _)| k.as_str()).collect();
        let want_names: Vec<String> = COLOR_KEYS.iter().map(|k| format!("--theme-{k}")).collect();
        eq("variable order", &json!(names), &json!(want_names))?;
        eq("map_variables", &pairs_obj(&vars), &c.expected)
    });
    assert_cases("omarchy constants", g.cases_for("constants"), |c| {
        let home = std::path::Path::new("/h");
        eq(
            "OMARCHY_DIR",
            &json!(legacy::legacy_current_dir(home)),
            &json!(
                c.expected["OMARCHY_DIR"]
                    .as_str()
                    .unwrap()
                    .replace('~', "/h")
            ),
        )?;
        eq(
            "THEME_NAME_FILE",
            &json!(format!(
                "~/{}/{}",
                legacy::LEGACY_CURRENT_DIR,
                legacy::THEME_NAME_FILE
            )),
            &c.expected["THEME_NAME_FILE"],
        )?;
        eq(
            "ALACRITTY_FILE",
            &json!(format!(
                "~/{}/{}",
                legacy::LEGACY_CURRENT_DIR,
                legacy::ALACRITTY_FILE
            )),
            &c.expected["ALACRITTY_FILE"],
        )
    });
}

#[test]
fn golden_omarchy_theme_data() {
    let g = golden("omarchy_theme");
    let root = is_root();
    assert_cases("omarchy theme_data", g.cases_for("theme_data"), |c| {
        let has_mode0 = c.input["files"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v.get("mode").is_some());
        if has_mode0 && root {
            return Ok(()); // EACCES cannot be simulated as root
        }
        let h = OmarchyHome::new();
        if let Some(t) = c.input.get("themes_dir_files") {
            h.put_spec(&h.themes(), t);
        }
        let current = legacy::legacy_current_dir(h.home());
        h.mkdir(legacy::LEGACY_CURRENT_DIR);
        h.put_spec(&current, &c.input["files"]);

        eq(
            "available",
            &json!(legacy::available(&current)),
            &c.expected["available"],
        )?;
        let want = &c.expected["theme_data"];
        match legacy::theme_data(&current) {
            Err(e) => {
                if want.get("error").is_none() {
                    return Err(format!("unexpected error {e}"));
                }
                Ok(())
            }
            Ok(None) => eq("theme_data", &J::Null, want),
            Ok(Some(d)) => {
                let got = json!({"theme_name": d.theme_name, "variables": pairs_obj(&d.variables), "is_dark": d.is_dark});
                eq("theme_data", &got, want)
            }
        }
    });
}
