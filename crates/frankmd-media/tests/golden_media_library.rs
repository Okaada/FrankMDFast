//! Conformance for the library functions of `tests/fixtures/golden/media.json`
//! (`MediaLibraryService#list` 4, `MediaLibraryService#delete` 33,
//! `ImageLibraryService#list` 12; the other 470 cases are in golden_media.rs).
//!
//! Normalizations (fixture notes): `input.fed` → `ResolvedConfig` → `MediaConfig`;
//! `<ROOT>` = temp notes root; mtimes were pinned with `File.utime` and printed in
//! the process zone (UTC when recorded) → compared as instants, since the Rust
//! port prints the local zone too; `.frankmd-config.lock` in `tree_after` is a
//! side effect of Rails' `Config.new` and is ignored; `path: null` is `nil.to_s`.
#![allow(clippy::disallowed_methods)]

#[path = "common/library.rs"]
mod library;

use frankmd_config::{EnvSnapshot, ResolvedConfig};
use frankmd_media::MediaKind;
use frankmd_media::config::MediaConfig;
use frankmd_media::library::{MediaItem, MediaLibrary, PickerCollection, PickerQuery, picker_list};
use frankmd_media::param::Param;
use frankmd_testkit::serde_json::{Map, Value, json};
use frankmd_testkit::{assert_cases, golden};
use library::Notes;

fn media_config(n: &Notes, fed: &Value) -> MediaConfig {
    let cfg = ResolvedConfig::new(fed.as_str(), EnvSnapshot::default());
    MediaConfig::from_resolved(&cfg, n.root())
}

fn instant(s: &str) -> Option<jiff::Timestamp> {
    s.parse::<jiff::Timestamp>().ok()
}

/// Item → fixture JSON, with `mtime` replaced by the expected string when both
/// denote the same instant (zone-independent comparison).
fn item_json(i: &MediaItem, expected: Option<&Value>) -> Value {
    let mtime = match expected.and_then(|e| e["mtime"].as_str()) {
        Some(e) if instant(e).is_some() && instant(e) == instant(&i.mtime) => e.to_owned(),
        _ => i.mtime.clone(),
    };
    json!({
        "name": i.name, "path": i.path, "size": i.size, "mtime": mtime, "preview_url": i.preview_url,
        "type": match i.kind { MediaKind::Image => "image", MediaKind::Video => "video" },
    })
}

fn items_json(items: &[MediaItem], expected: &Value) -> Value {
    Value::Array(
        items
            .iter()
            .enumerate()
            .map(|(k, i)| item_json(i, expected.get(k)))
            .collect(),
    )
}

fn eq(actual: &Value, expected: &Value) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected}\n     got {actual}"))
    }
}

fn without_config_lock(v: &Value) -> Value {
    let mut m: Map<String, Value> = v.as_object().cloned().unwrap_or_default();
    m.remove(".frankmd-config.lock");
    // Mechanical: deletion goes through frankmd-fs, which takes the shared
    // compat lock `.frankmd-filesystem.lock` (Rails' bare `File.delete` did not).
    m.remove(".frankmd-filesystem.lock");
    m.remove(".fed");
    Value::Object(m)
}

fn param(v: Option<&Value>) -> Param {
    match v {
        None | Some(Value::Null) => Param::Nil,
        Some(Value::Bool(b)) => Param::Bool(*b),
        Some(Value::Number(n)) => n
            .as_i64()
            .map_or_else(|| Param::Float(n.as_f64().unwrap_or_default()), Param::Int),
        Some(Value::String(s)) => Param::Str(s.clone()),
        Some(Value::Array(a)) => Param::List(a.iter().map(|x| param(Some(x))).collect()),
        Some(other) => Param::Str(other.to_string()),
    }
}

#[test]
fn media_library_list() {
    let g = golden("media");
    assert_cases(
        "media MediaLibraryService#list",
        g.cases_for("MediaLibraryService#list"),
        |c| {
            let n = Notes::from_golden(&c.input["tree"], None);
            let vault = n.vault();
            let listing =
                MediaLibrary::from_config(&vault, &media_config(&n, &c.input["fed"])).list();
            eq(
                &items_json(&listing.images, &c.expected["images"]),
                &c.expected["images"],
            )?;
            eq(
                &items_json(&listing.videos, &c.expected["videos"]),
                &c.expected["videos"],
            )
        },
    );
}

#[test]
fn media_library_delete() {
    let g = golden("media");
    assert_cases(
        "media MediaLibraryService#delete",
        g.cases_for("MediaLibraryService#delete"),
        |c| {
            let n = Notes::from_golden(&c.input["tree"], None);
            let vault = n.vault();
            let lib = MediaLibrary::from_config(&vault, &media_config(&n, &c.input["fed"]));
            let path = c.input["path"]
                .as_str()
                .unwrap_or("")
                .replace("<ROOT>", &n.root().display().to_string());
            let safe = lib
                .safe_media_file(&path)
                .map(|p| n.normalize(&p.display().to_string()));
            eq(&json!(safe), &c.expected["safe_media_file"])?;
            let result = lib.delete(&path).map_err(|e| format!("raised {e:?}"))?;
            eq(&json!(result), &c.expected["result"])?;
            eq(
                &without_config_lock(&n.snapshot()),
                &without_config_lock(&c.expected["tree_after"]),
            )
        },
    );
}

#[test]
fn image_library_list() {
    let g = golden("media");
    assert_cases(
        "media ImageLibraryService#list",
        g.cases_for("ImageLibraryService#list"),
        |c| {
            let n = Notes::from_golden(&c.input["tree"], None);
            let cfg = media_config(&n, &c.input["fed"]);
            let kw = &c.input["kwargs"];
            let query = PickerQuery {
                collection: param(kw.get("collection")),
                search: param(kw.get("search")),
                // ImageLibraryService#list defaults: limit 10, offset 0 (kwargs omitted → defaults).
                limit: kw.get("limit").map_or(Param::Int(10), |v| param(Some(v))),
                offset: kw.get("offset").map_or(Param::Int(0), |v| param(Some(v))),
            };
            let got = match picker_list(&n.vault(), &cfg, &query) {
                Ok(page) => {
                    let exp_images = &c.expected["images"];
                    let images: Vec<Value> = page
                        .images
                        .iter()
                        .enumerate()
                        .map(|(k, i)| {
                            // Managed items are MediaLibrary items + url; server items only name/path/url/preview_url.
                            let mut v = i
                                .item
                                .as_ref()
                                .map_or_else(|| json!({}), |it| item_json(it, exp_images.get(k)));
                            v["name"] = json!(i.name);
                            v["path"] = json!(i.path);
                            v["url"] = json!(i.url);
                            v["preview_url"] = json!(i.preview_url);
                            v
                        })
                        .collect();
                    json!({
                        "images": images,
                        "collection": match page.collection { PickerCollection::Managed => "managed", PickerCollection::Server => "server" },
                        "total": page.total, "limit": page.limit, "offset": page.offset,
                        "server_available": page.server_available,
                    })
                }
                Err(e) => {
                    json!({"error": "ArgumentError", "message": e.to_string().trim_start_matches("bad request: ")})
                }
            };
            eq(&got, &c.expected)
        },
    );
}
