//! Golden conformance: tests/fixtures/golden/js_menu_bounds.json.
//! Element stubs `{rect:{left,right,top}, hidden}` → [`MenuRect`] + flag;
//! defaults when the case omits viewport args: innerHeight 768, innerWidth 1024
//! (fixture notes). `null` left/right = non-finite → `None`.
#![allow(clippy::unwrap_used)]

use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{assert_cases, decode_js, golden};
use frankmd_workspace::menu_bounds::{
    MenuPlacement, MenuRect, VIEWPORT_PADDING, clamp_menu_to_viewport, menu_max_height, toggle_menu,
};

const FIXTURE: &str = "js_menu_bounds";
const DEFAULT_H: f64 = 768.0;
const DEFAULT_W: f64 = 1024.0;

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|x| x.is_finite())
}

fn rect(v: &Value) -> MenuRect {
    MenuRect { top: num(v.get("top")).unwrap(), left: num(v.get("left")), right: num(v.get("right")) }
}

fn style(p: Option<MenuPlacement>) -> Value {
    let (transform, max_height) = match p {
        Some(p) => (
            p.shift_x.map(|x| format!("translateX({x}px)")).unwrap_or_default(),
            format!("{}px", p.max_height),
        ),
        None => (String::new(), String::new()),
    };
    json!({"transform": transform, "maxHeight": max_height})
}

fn check(case: &frankmd_testkit::Case) -> Result<(), String> {
    let input = decode_js(&case.input);
    let args = input.as_array().unwrap();
    let expected = decode_js(&case.expected);
    let got = match case.func.as_deref().unwrap() {
        "menuMaxHeight" => {
            let top = num(args[0].get("top")).unwrap();
            let pad = num(args.get(2)).unwrap_or(VIEWPORT_PADDING);
            json!(format!("{}px", menu_max_height(top, num(args.get(1)).unwrap(), pad)))
        }
        "clampMenuToViewport" => {
            let el = &args[0];
            let h = num(args.get(1)).unwrap_or(DEFAULT_H);
            let w = num(args.get(2)).unwrap_or(DEFAULT_W);
            let p = clamp_menu_to_viewport(&rect(&el["rect"]), h, w);
            json!({"return": Value::Null, "style": style(Some(p)), "hidden": el["hidden"]})
        }
        "toggleMenu" => {
            let el = &args[0];
            let force = args.get(1).and_then(Value::as_bool).unwrap_or(false);
            let t = toggle_menu(el["hidden"].as_bool().unwrap(), force, &rect(&el["rect"]), DEFAULT_H, DEFAULT_W);
            json!({"return": t.visible, "style": style(t.placement), "hidden": !t.visible})
        }
        other => return Err(format!("unknown fn {other}")),
    };
    if got == expected { Ok(()) } else { Err(format!("expected {expected}, got {got}")) }
}

#[test]
fn golden_menu_bounds_all_cases() {
    let g = golden(FIXTURE);
    assert_cases(FIXTURE, &g.cases, check);
}
