//! Conformance against `tests/fixtures/golden/js_image_sources.json` (70 cases).
//!
//! Scenario cases drive the Rust picker state machines step by step through a
//! FIFO [`FakeTransport`]; per step we compare `result`, `requests` and the
//! watched `state`. Not compared (DOM): `rendered.innerHTML`; `renderGrid` /
//! `renderRefImageGrid` / `deselectAll` steps only check that no request is
//! made — except the folder source, whose `rendered.status` line is compared
//! via `FolderImageSource::status_text`. Folder images are compared without
//! the JS-only `file` / `objectUrl` (blob URL) fields.

#[path = "common/client_picker.rs"]
mod client_picker;

use client_picker::{FakeTransport, js_arg, picked_file, translator};
use frankmd_media::picker::{
    AiImageSource, DEFAULT_IMAGE_EXTENSIONS, DirEntry, DirectoryPicker, DisplayedImage, FolderImageSource,
    GoogleImageSource, LocalImageSource, Outcome, PickError, PickerCx, PinterestImageSource, RefImage, Translator,
    WebImageSource, ext_of, image_picker_search_result_message, image_picker_text,
};
use frankmd_testkit::serde_json::{Value, json};
use frankmd_testkit::{Case, assert_cases, decode_js, golden};

/// Ai `Date.now` pinned by the generator.
const NOW_MS: i64 = 1_700_000_000_000;

fn eq(what: &str, got: &Value, want: &Value) -> Result<(), String> {
    if got == want { Ok(()) } else { Err(format!("{what}: expected {want}\n       got {got}")) }
}

fn outcome_json(o: Outcome) -> Value {
    match o {
        Ok(v) => v,
        Err(t) => json!({"$thrown": {"name": t.name, "message": t.message}}),
    }
}

fn options(v: Option<&Value>) -> Vec<(String, frankmd_media::picker::JsArg)> {
    match v {
        Some(Value::Object(o)) if !o.contains_key("$undefined") => {
            o.iter().map(|(k, v)| (k.clone(), js_arg(Some(v)))).collect()
        }
        _ => Vec::new(),
    }
}

#[test]
fn text_helpers() {
    let g = golden("js_image_sources");
    let cases = g.cases.iter().filter(|c| c.func.as_deref() != Some("scenario"));
    assert_cases("js_image_sources helpers", cases, |c: &Case| {
        let want = decode_js(&c.expected);
        let got = match c.func.as_deref() {
            Some("DEFAULT_IMAGE_EXTENSIONS") => json!(DEFAULT_IMAGE_EXTENSIONS),
            Some("extOf") => json!(ext_of(None, c.input[0].as_str().ok_or("name")?)),
            Some("imagePickerText") => {
                let t = translator(c.input["translation"].as_str().unwrap_or("none"));
                let a = c.input["args"].as_array().ok_or("args")?;
                let fallback = decode_js(a.get(2).unwrap_or(&Value::Null));
                json!(image_picker_text(t, a[0].as_str().ok_or("key")?, &options(a.get(1)), fallback.as_str()))
            }
            Some("imagePickerSearchResultMessage") => {
                let t = translator(c.input["translation"].as_str().unwrap_or("none"));
                json!(image_picker_search_result_message(t, &js_arg(c.input["args"].get(0))))
            }
            other => return Err(format!("unknown fn {other:?}")),
        };
        eq("result", &got, &want)
    });
}

enum Source {
    Web(WebImageSource),
    Google(GoogleImageSource),
    Pinterest(PinterestImageSource),
    Local(LocalImageSource),
    Ai(AiImageSource),
    Folder(FolderImageSource),
}

struct FixturePicker(Value);

impl DirectoryPicker for FixturePicker {
    fn pick(&mut self) -> Result<Vec<DirEntry>, PickError> {
        match &self.0 {
            Value::String(s) if s == "abort" => Err(PickError::Aborted),
            Value::String(s) => Err(PickError::Failed(s.clone())),
            Value::Array(entries) => Ok(entries
                .iter()
                .map(|e| {
                    if e["kind"] == "directory" {
                        DirEntry::Directory(e["name"].as_str().unwrap_or_default().into())
                    } else {
                        DirEntry::File(picked_file(e))
                    }
                })
                .collect()),
            _ => Err(PickError::Failed("unexpected".into())),
        }
    }
}

fn displayed_json(d: &DisplayedImage) -> Value {
    json!({"name": d.file.name, "size": d.file.size, "lastModified": d.file.last_modified,
           "width": d.width, "height": d.height})
}

/// Drop JS-only fields from a decoded folder image.
fn strip_folder(v: &Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(o.iter().filter(|(k, _)| *k != "file" && *k != "objectUrl").map(|(k, v)| (k.clone(), v.clone())).collect()),
        Value::Array(a) => Value::Array(a.iter().map(strip_folder).collect()),
        other => other.clone(),
    }
}

fn state(src: &Source, prop: &str) -> Value {
    match (src, prop) {
        (Source::Web(s), "results") => json!(s.results),
        (Source::Pinterest(s), "results") => json!(s.results),
        (Source::Google(s), "results") => json!(s.results),
        (Source::Google(s), "nextStart") => s.next_start.clone(),
        (Source::Google(s), "loading") => json!(s.loading),
        (Source::Google(s), "query") => json!(s.query),
        (Source::Ai(s), "enabled") => json!(s.enabled),
        (Source::Ai(s), "model") => json!(s.model),
        (Source::Ai(s), "generatedData") => s.generated_data.clone().unwrap_or(Value::Null),
        (Source::Ai(s), "refImage") => s.ref_image.as_ref().map_or(Value::Null, |r| json!({"path": r.path, "name": r.name})),
        (Source::Folder(s), "displayedImages") => Value::Array(s.displayed_images.iter().map(displayed_json).collect()),
        (_, p) => json!({"unsupported watch": p}),
    }
}

fn str_arg(a: &[Value], i: usize) -> Option<String> {
    a.get(i).map(decode_js).and_then(|v| v.as_str().map(str::to_owned))
}

fn run_scenario(c: &Case) -> Result<(), String> {
    let input = &c.input;
    let t: Option<&dyn Translator> = translator(input["translation"].as_str().unwrap_or("none"));
    let setup = &input["setup"];
    let mut src = match input["source"].as_str().ok_or("source")? {
        "WebImageSource" => Source::Web(WebImageSource::default()),
        "GoogleImageSource" => {
            let mut g = GoogleImageSource::default();
            if let Some(l) = setup.get("loading").and_then(Value::as_bool) {
                g.loading = l;
            }
            if let Some(r) = setup.get("results").and_then(Value::as_array) {
                g.results = r.clone();
            }
            Source::Google(g)
        }
        "PinterestImageSource" => Source::Pinterest(PinterestImageSource::default()),
        "LocalImageSource" => Source::Local(LocalImageSource { selected_path: setup["selectedPath"].as_str().map(str::to_owned) }),
        "AiImageSource" => {
            let mut a = AiImageSource::default();
            if let Some(r) = setup.get("refImage") {
                a.ref_image = Some(RefImage { path: r["path"].as_str().unwrap_or("").into(), name: r["name"].as_str().unwrap_or("").into() });
            }
            Source::Ai(a)
        }
        "FolderImageSource" => Source::Folder(FolderImageSource::default()),
        other => return Err(format!("unknown source {other}")),
    };
    let watch: Vec<String> = input["watch"].as_array().map(|w| w.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect()).unwrap_or_default();
    let picker_spec = input.get("showDirectoryPicker").cloned();
    let expected = c.expected.as_array().ok_or("expected steps")?;
    let steps = input["steps"].as_array().ok_or("steps")?;
    let mut transport = FakeTransport::default();

    for (i, (step, want)) in steps.iter().zip(expected).enumerate() {
        let call = step["call"].as_str().ok_or("call")?;
        let args: Vec<Value> = step["args"].as_array().cloned().unwrap_or_default();
        for r in step["responses"].as_array().into_iter().flatten() {
            transport.push_fixture(r);
        }
        let mut status: Option<Value> = None;
        let result: Value = {
            let mut cx = PickerCx { transport: &mut transport, t };
            match (&mut src, call) {
                (Source::Web(s), "search") => outcome_json(s.search(&mut cx, &js_arg(args.first()))),
                (Source::Web(s), "saveCopy") => outcome_json(s.save_copy(&mut cx, &args[0], args.get(1).unwrap_or(&Value::Null), str_arg(&args, 2).as_deref())),
                (Source::Web(s), "reset") => { s.reset(); Value::Null }
                (Source::Pinterest(s), "search") => outcome_json(s.search(&mut cx, &js_arg(args.first()))),
                (Source::Pinterest(s), "reset") => { s.reset(); Value::Null }
                (Source::Google(s), "search") => outcome_json(s.search(&mut cx, &js_arg(args.first()))),
                (Source::Google(s), "loadMore") => outcome_json(s.load_more(&mut cx)),
                (Source::Google(s), "shouldLoadMore") => json!(s.should_load_more(args[0].as_f64().unwrap_or(0.0))),
                (Source::Google(s), "reset") => { s.reset(); Value::Null }
                (Source::Local(s), "load") => {
                    let opts = args.get(1).cloned().unwrap_or(Value::Null);
                    let limit = if opts.get("limit").is_some() { js_arg(opts.get("limit")) } else { frankmd_media::picker::JsArg::Number(10.0) };
                    let offset = if opts.get("offset").is_some() { js_arg(opts.get("offset")) } else { frankmd_media::picker::JsArg::Number(0.0) };
                    outcome_json(s.load(&mut cx, &js_arg(args.first()), &limit, &offset))
                }
                (Source::Local(s), "deleteImage") => outcome_json(s.delete_image(&mut cx, args[0].as_str().unwrap_or(""))),
                (Source::Local(s), "saveCopy") => outcome_json(s.save_copy(&mut cx, &args[0], args.get(1).unwrap_or(&Value::Null), str_arg(&args, 2).as_deref())),
                (Source::Local(s), "reset") => { s.reset(); Value::Null }
                (Source::Ai(s), "generate") => outcome_json(s.generate(&mut cx, &js_arg(args.first()))),
                (Source::Ai(s), "loadConfig") => outcome_json(s.load_config(&mut cx)),
                (Source::Ai(s), "save") => outcome_json(s.save(&mut cx, str_arg(&args, 0).as_deref(), NOW_MS)),
                (Source::Ai(s), "setRefImage") => { s.set_ref_image(args[0].as_str().unwrap_or(""), args[1].as_str().unwrap_or("")); Value::Null }
                (Source::Ai(s), "clearRefImage") => { s.clear_ref_image(); Value::Null }
                (Source::Ai(s), "loadRefImages") => outcome_json(s.load_ref_images(&mut cx, &js_arg(args.first()))),
                (Source::Ai(s), "reset") => { s.reset(); Value::Null }
                (Source::Folder(s), "ingest") => {
                    let files = args[0].as_array().ok_or("files")?.iter().map(picked_file).collect();
                    let allowed: Option<Vec<String>> = args.get(1).and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect());
                    outcome_json(s.ingest(t, files, allowed.as_deref()))
                }
                (Source::Folder(s), "browse") => {
                    let allowed: Option<Vec<String>> = args.first().and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect());
                    let mut fp = picker_spec.clone().filter(|v| !v.is_null()).map(FixturePicker);
                    let picker: Option<&mut dyn DirectoryPicker> = fp.as_mut().map(|p| p as &mut dyn DirectoryPicker);
                    outcome_json(s.browse(t, picker, allowed.as_deref()))
                }
                (Source::Folder(s), "filter") => {
                    let o = args.get(1).cloned().unwrap_or(Value::Null);
                    let offset = o.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
                    let limit = o.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
                    outcome_json(s.filter(args[0].as_str().unwrap_or(""), offset, limit))
                }
                (Source::Folder(s), "getImage") => s.get_image(args[0].as_u64().unwrap_or(0) as usize).map_or(Value::Null, displayed_json),
                (Source::Folder(s), "upload") => {
                    // Harness fix: the generator builds upload Files as
                    // `"x".repeat(size || 3)` (tools/golden/js/image_sources.mjs:68),
                    // unlike the 1024-byte default of folder-listing fakes.
                    let mut f = picked_file(&args[0]);
                    f.size = args[0].get("size").and_then(Value::as_u64).unwrap_or(3);
                    outcome_json(s.upload(&mut cx, &f, &js_arg(args.get(1)), str_arg(&args, 2).as_deref()))
                }
                (Source::Folder(s), "reset") => { s.reset(); Value::Null }
                (Source::Folder(s), "renderGrid") => {
                    if step.get("withStatus") != Some(&Value::Bool(false)) {
                        let total = args.get(1).and_then(Value::as_u64).map(|n| n as usize);
                        status = Some(json!(s.status_text(t, total)));
                    }
                    Value::Null
                }
                (_, "renderGrid" | "renderRefImageGrid" | "deselectAll") => Value::Null, // DOM only
                (_, other) => return Err(format!("step {i}: unsupported call {other}")),
            }
        };
        let ctx = format!("step {i} {call}");
        let mut want_result = decode_js(&want["result"]);
        if matches!(src, Source::Folder(_)) {
            want_result = strip_folder(&want_result);
        }
        eq(&format!("{ctx} result"), &result, &want_result)?;
        eq(&format!("{ctx} requests"), &transport.take_requests(), &decode_js(&want["requests"]))?;
        if let Some(st) = status {
            let want_status = want["rendered"].get("status").map(decode_js).unwrap_or(Value::Null);
            eq(&format!("{ctx} status"), &st, &want_status)?;
        }
        if !watch.is_empty() {
            for prop in &watch {
                let mut want_prop = decode_js(&want["state"][prop.as_str()]);
                if prop == "displayedImages" {
                    want_prop = strip_folder(&want_prop);
                }
                eq(&format!("{ctx} state.{prop}"), &state(&src, prop), &want_prop)?;
            }
        }
    }
    Ok(())
}

#[test]
fn scenarios() {
    let g = golden("js_image_sources");
    assert_cases("js_image_sources scenarios", g.cases_for("scenario"), run_scenario);
}
