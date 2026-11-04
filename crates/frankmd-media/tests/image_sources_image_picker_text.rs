//! Port of `test/javascript/image_sources/image_picker_text.test.js` (3 tests).

use frankmd_media::picker::{JsArg, TextOptions, Translator, image_picker_search_result_message, image_picker_text};

fn opts(pairs: &[(&str, f64)]) -> TextOptions {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), JsArg::Number(*v))).collect()
}

fn num(v: &JsArg) -> String {
    match v {
        JsArg::Number(n) => format!("{n}"),
        other => format!("{other:?}"),
    }
}

struct Interpolating;
impl Translator for Interpolating {
    fn t(&self, key: &str, options: &TextOptions) -> String {
        assert_eq!(key, "dialogs.image_picker.folder_showing_recent");
        let get = |k: &str| options.iter().find(|(n, _)| n == k).map(|(_, v)| num(v)).unwrap_or_default();
        format!("Showing {} recent images out of {}", get("shown"), get("total"))
    }
}

struct KeyCount;
impl Translator for KeyCount {
    fn t(&self, key: &str, options: &TextOptions) -> String {
        let count = options.iter().find(|(n, _)| n == "count").map(|(_, v)| num(v)).unwrap_or_default();
        format!("{key}:{count}")
    }
}

// port: test/javascript/image_sources/image_picker_text.test.js:22 "uses a loaded translation and interpolates its values"
#[test]
fn uses_loaded_translation() {
    let got = image_picker_text(
        Some(&Interpolating),
        "folder_showing_recent",
        &opts(&[("shown", 2.0), ("total", 5.0)]),
        Some("Showing %{shown} most recent images out of %{total}"),
    );
    assert_eq!(got, "Showing 2 recent images out of 5");
}

// port: test/javascript/image_sources/image_picker_text.test.js:35 "uses and interpolates its English fallback when translations are unavailable"
#[test]
fn uses_english_fallback_without_translations() {
    assert_eq!(image_picker_text(None, "search_result_many", &opts(&[("count", 3.0)]), Some("Found %{count} images")), "Found 3 images");
}

// port: test/javascript/image_sources/image_picker_text.test.js:42 "selects singular and plural search result translations"
#[test]
fn selects_singular_and_plural() {
    assert_eq!(image_picker_search_result_message(Some(&KeyCount), &JsArg::Number(1.0)), "dialogs.image_picker.search_result_one:1");
    assert_eq!(image_picker_search_result_message(Some(&KeyCount), &JsArg::Number(4.0)), "dialogs.image_picker.search_result_many:4");
}
