//! Golden conformance: tests/fixtures/golden/media.json — every fn except the
//! library ones (`MediaLibraryService#list`, `MediaLibraryService#delete`,
//! `ImageLibraryService#list`, ported by the media-library tests).
//!
//! Notes honoured: clock pinned (input.pinned, default 2026-01-01T12:00:00Z,
//! process TZ for `dest_filename`, UTC for S3 keys), `SecureRandom.hex` pinned
//! (`PinnedHex`), `input.fed` / `input.env` → `ResolvedConfig` →
//! `MediaConfig::from_resolved`, non-UTF-8 bytes as `{base64}`. `tree_after`
//! is compared for `images/` and `videos/` only: `.fed` (template written by
//! `Config.new`) and `.frankmd-config.lock` are config-layer side effects.
#![allow(clippy::disallowed_methods, clippy::unwrap_used)]

#[path = "common/uploads.rs"]
mod uploads;

use std::collections::BTreeMap;
use std::path::Path;

use frankmd_config::{EnvSnapshot, ResolvedConfig};
use frankmd_media::config::MediaConfig;
use frankmd_media::error::MediaError;
use frankmd_media::images;
use frankmd_media::s3::S3Error;
use frankmd_media::upload_storage::{self as us, Upload, UploadKind};
use frankmd_media::{video, StorageProvider};
use frankmd_testkit::serde_json::{json, Value};
use frankmd_testkit::{assert_cases, golden, Case, Golden, TempTree};
use uploads::*;

const LIBRARY_FNS: [&str; 3] = ["MediaLibraryService#list", "MediaLibraryService#delete", "ImageLibraryService#list"];

fn fixture() -> Golden {
    golden("media")
}

fn run(func: &str, check: impl FnMut(&Case) -> Result<(), String>) {
    let g = fixture();
    assert_cases(&format!("media.json {func}"), g.cases_for(func), check);
}

/// `input.fed` + `input.env` → the media view of the resolved config.
fn config_for(input: &Value, root: &Path) -> MediaConfig {
    let env: Vec<(String, String)> = input["env"]
        .as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned())).collect())
        .unwrap_or_default();
    let resolved = ResolvedConfig::new(input["fed"].as_str(), EnvSnapshot::from_pairs(env));
    MediaConfig::from_resolved(&resolved, root)
}

fn opt_str(v: &Value) -> Option<&str> {
    v.as_str()
}

/// `{"error": "UploadStorage::RejectedError", "message": m}` ⇔ `Err(Rejected(m))`.
fn check_rejected<T: std::fmt::Debug + PartialEq>(
    got: Result<T, MediaError>,
    expected: &Value,
    ok: impl FnOnce(&T) -> Result<(), String>,
) -> Result<(), String> {
    match (got, expected.get("error")) {
        (Err(MediaError::Rejected(m)), Some(_)) if Some(m.as_str()) == expected["message"].as_str() => Ok(()),
        (Ok(v), None) => ok(&v),
        (got, _) => Err(format!("got {got:?}, expected {expected}")),
    }
}

fn eq<T: PartialEq + std::fmt::Debug>(got: T, want: T) -> Result<(), String> {
    if got == want { Ok(()) } else { Err(format!("got {got:?}, expected {want:?}")) }
}

/// `{url}` / `{error}` result shape.
fn check_result(got: Result<String, MediaError>, expected: &Value) -> Result<(), String> {
    match (&got, expected.get("url"), expected.get("error")) {
        (Ok(u), Some(want), _) if want.as_str() == Some(u.as_str()) => Ok(()),
        (Err(e), _, Some(want)) if want.as_str() == Some(e.to_string().as_str()) => Ok(()),
        _ => Err(format!("got {got:?}, expected {expected}")),
    }
}

/// Snapshot of `images/` and `videos/` as bytes (dirs → `None`).
fn media_tree(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    let mut out = BTreeMap::new();
    for top in ["images", "videos"] {
        let dir = root.join(top);
        if dir.is_dir() {
            out.insert(format!("{top}/"), None);
            for e in std::fs::read_dir(&dir).unwrap() {
                let p = e.unwrap().path();
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                out.insert(rel, Some(std::fs::read(&p).unwrap()));
            }
        }
    }
    out
}

fn expected_tree(v: &Value) -> BTreeMap<String, Option<Vec<u8>>> {
    v.as_object()
        .map(|m| {
            m.iter()
                .filter(|(k, _)| k.starts_with("images/") || k.starts_with("videos/"))
                .map(|(k, v)| {
                    let bytes = match v {
                        Value::Null => None,
                        Value::String(s) => Some(s.as_bytes().to_vec()),
                        Value::Object(o) => Some(unb64(o["base64"].as_str().unwrap())),
                        other => panic!("tree value {other}"),
                    };
                    (k.clone(), bytes)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn check_tree(root: &Path, expected: &Value) -> Result<(), String> {
    eq(media_tree(root), expected_tree(&expected["tree_after"]))
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}

#[test]
fn every_case_is_wired() {
    let g = fixture();
    let wired = [
        "Config#upload_extensions",
        "ImagesService.clamp_limit",
        "ImagesService.clamp_offset",
        "ImagesService.constants",
        "ImagesService.content_type_for",
        "ImagesService.extension_for_content_type",
        "ImagesService.imagemagick_resize_arg",
        "ImagesService.upload_base64_data",
        "ImagesService.upload_file",
        "MediaService.VIDEO_MIME_TYPES",
        "MediaService.save_upload",
        "UploadStorage.dest_filename",
        "UploadStorage.enforce_base64_size!",
        "UploadStorage.enforce_bytes!",
        "UploadStorage.enforce_size!",
        "UploadStorage.invalid_s3_configuration_error?",
        "UploadStorage.max_base64_encoded_bytes",
        "UploadStorage.provider",
        "UploadStorage.s3_configured?",
        "UploadStorage.s3_key",
        "UploadStorage.s3_region",
        "UploadStorage.s3_url",
        "UploadStorage.sanitize_s3_key",
        "UploadStorage.unique_s3_key",
        "UploadStorage.validate_filename!",
    ];
    for c in &g.cases {
        let f = c.func.as_deref().unwrap_or_default();
        assert!(wired.contains(&f) || LIBRARY_FNS.contains(&f), "unwired golden fn {f} ({})", c.name);
    }
}

#[test]
fn config_upload_extensions() {
    let t = TempTree::empty();
    run("Config#upload_extensions", |c| {
        let cfg = config_for(&c.input, t.path());
        let got = match c.input["key"].as_str().unwrap() {
            "image_upload_extensions" => cfg.image_upload_extensions,
            _ => cfg.video_upload_extensions,
        };
        eq(json!(got), c.expected.clone())
    });
}

#[test]
fn images_clamp_limit() {
    run("ImagesService.clamp_limit", |c| eq(json!(images::clamp_limit(&param(&c.input["value"]))), c.expected.clone()));
}

#[test]
fn images_clamp_offset() {
    run("ImagesService.clamp_offset", |c| eq(json!(images::clamp_offset(&param(&c.input["value"]))), c.expected.clone()));
}

#[test]
fn images_constants() {
    run("ImagesService.constants", |c| {
        let got = json!({
            "SUPPORTED_EXTENSIONS": images::SUPPORTED_EXTENSIONS,
            "MAX_RESULTS": images::MAX_RESULTS,
            "MAX_EXTERNAL_IMAGE_BYTES": images::MAX_EXTERNAL_IMAGE_BYTES,
            "MAX_EXTERNAL_IMAGE_REDIRECTS": images::MAX_EXTERNAL_IMAGE_REDIRECTS,
            "MAX_EXTERNAL_IMAGE_SECONDS": images::MAX_EXTERNAL_IMAGE_SECONDS,
            "UploadStorage": {
                "SANITIZE_RE": us::SANITIZE_RE,
                "MAX_UPLOAD_BYTES": us::MAX_UPLOAD_BYTES,
                "S3_KEY_PREFIX": us::S3_KEY_PREFIX,
                "DEFAULT_S3_REGION": us::DEFAULT_S3_REGION,
            }
        });
        eq(got, c.expected.clone())
    });
}

#[test]
fn images_content_type_for() {
    run("ImagesService.content_type_for", |c| {
        eq(images::content_type_for(c.input["path"].as_str().unwrap()), c.expected.as_str().unwrap())
    });
}

#[test]
fn images_extension_for_content_type() {
    run("ImagesService.extension_for_content_type", |c| {
        let got = images::extension_for_content_type(opt_str(&c.input["content_type"]));
        // NoMethodError in Ruby (nil / "" input) ⇔ None.
        let want = if c.expects_error() { None } else { c.expected.as_str() };
        eq(got, want)
    });
}

#[test]
fn images_imagemagick_resize_arg() {
    run("ImagesService.imagemagick_resize_arg", |c| {
        eq(images::resize_arg(&param(&c.input["ratio"])), c.expected.as_str().unwrap())
    });
}

#[test]
fn images_upload_base64_data() {
    let rt = rt();
    run("ImagesService.upload_base64_data", |c| {
        let t = TempTree::empty();
        let (_, mut env) = env_for(config_for(&c.input, t.path()), None);
        env.ctx = ctx_from_pinned(&c.input);
        let got = rt.block_on(images::upload_base64_data(
            &env,
            c.input["base64_data"].as_str().unwrap(),
            opt_str(&c.input["mime_type"]),
            opt_str(&c.input["filename"]),
            &param(&c.input["upload_to_s3"]),
            None,
        ));
        check_result(got, &c.expected["result"])?;
        check_tree(t.path(), &c.expected)
    });
}

#[test]
fn images_upload_file() {
    let rt = rt();
    run("ImagesService.upload_file", |c| {
        let t = TempTree::empty();
        let (_, mut env) = env_for(config_for(&c.input, t.path()), None);
        env.ctx = ctx_from_pinned(&c.input);
        let file = c.input["original_filename"]
            .as_str()
            .map(|n| Upload::new(n, c.input["content"].as_str().unwrap_or_default()));
        let resize = images::parse_resize_ratio(&param(&c.input["resize"]));
        let got = rt.block_on(images::upload_file(&env, file, resize, &param(&c.input["upload_to_s3"]), None));
        check_result(got, &c.expected["result"])?;
        check_tree(t.path(), &c.expected)
    });
}

#[test]
fn media_service_video_mime_types() {
    run("MediaService.VIDEO_MIME_TYPES", |c| {
        let got: frankmd_testkit::serde_json::Map<String, Value> =
            video::VIDEO_MIME_TYPES.iter().map(|(k, v)| ((*k).to_owned(), json!(v))).collect();
        eq(Value::Object(got), c.expected.clone())
    });
}

#[test]
fn media_service_save_upload() {
    let rt = rt();
    run("MediaService.save_upload", |c| {
        let t = TempTree::empty();
        let (_, mut env) = env_for(config_for(&c.input, t.path()), None);
        env.ctx = ctx_from_pinned(&c.input);
        let file = c.input["original_filename"]
            .as_str()
            .map(|n| Upload::new(n, c.input["content"].as_str().unwrap_or("fake video bytes")));
        let got = rt.block_on(video::save_upload(&env, file, &param(&c.input["upload_to_s3"]), None));
        check_result(got, &c.expected["result"])?;
        check_tree(t.path(), &c.expected)
    });
}

#[test]
fn upload_storage_dest_filename() {
    run("UploadStorage.dest_filename", |c| {
        let got = us::dest_filename(&ctx_from_pinned(&c.input), opt_str(&c.input["original_filename"]));
        eq(got.as_str(), c.expected.as_str().unwrap())
    });
}

fn size_input(v: &Value) -> Option<i64> {
    v.as_i64()
}

#[test]
fn upload_storage_enforce_base64_size() {
    run("UploadStorage.enforce_base64_size!", |c| {
        let size = c.input["size"].as_u64().unwrap();
        check_rejected(us::enforce_base64_size(size, us::max_base64_encoded_bytes()), &c.expected, |_| Ok(()))
    });
}

#[test]
fn upload_storage_enforce_bytes() {
    run("UploadStorage.enforce_bytes!", |c| {
        check_rejected(us::enforce_bytes(size_input(&c.input["size"]), us::MAX_UPLOAD_BYTES), &c.expected, |_| Ok(()))
    });
}

#[test]
fn upload_storage_enforce_size() {
    run("UploadStorage.enforce_size!", |c| {
        let max = us::MAX_UPLOAD_BYTES as i64;
        let size = match c.input["object"].as_str().unwrap() {
            "size=MAX" => Some(max),
            "size=MAX+1" => Some(max + 1),
            "no size method" | "size nil" => None,
            other => return Err(format!("unknown object {other}")),
        };
        check_rejected(us::enforce_bytes(size, us::MAX_UPLOAD_BYTES), &c.expected, |_| Ok(()))
    });
}

#[test]
fn upload_storage_invalid_s3_configuration_error() {
    run("UploadStorage.invalid_s3_configuration_error?", |c| {
        let class = c.input["error_class"].as_str().unwrap();
        let err = match (class, c.input["code"].as_str()) {
            ("Aws::Errors::InvalidRegionError", _) => S3Error::InvalidRegion,
            (_, Some(code)) => S3Error::Service { code: code.into(), status: 400 },
            ("Timeout::Error", None) => S3Error::Network("timeout".into()),
            ("Seahorse::Client::NetworkingError", None) => S3Error::Network("networking".into()),
            other => return Err(format!("unmapped {other:?}")),
        };
        eq(json!(us::invalid_s3_configuration_error(&err)), c.expected.clone())
    });
}

#[test]
fn upload_storage_max_base64_encoded_bytes() {
    run("UploadStorage.max_base64_encoded_bytes", |c| {
        eq(json!(us::max_base64_encoded_bytes()), c.expected.clone())
    });
}

#[test]
fn upload_storage_provider() {
    let t = TempTree::empty();
    run("UploadStorage.provider", |c| {
        let cfg = config_for(&c.input, t.path());
        check_rejected(us::provider(&cfg, &param(&c.input["legacy_s3"])), &c.expected, |p| {
            let name = match p {
                StorageProvider::Local => "local",
                StorageProvider::S3 => "s3",
            };
            eq(name, c.expected.as_str().unwrap_or_default())
        })
    });
}

#[test]
fn upload_storage_s3_configured() {
    let t = TempTree::empty();
    run("UploadStorage.s3_configured?", |c| {
        eq(json!(us::s3_configured(&config_for(&c.input, t.path()))), c.expected.clone())
    });
}

#[test]
fn upload_storage_s3_key() {
    run("UploadStorage.s3_key", |c| {
        let got = us::s3_key(
            &ctx_from_pinned(&c.input),
            opt_str(&c.input["original_filename"]),
            opt_str(&c.input["custom_key"]),
            opt_str(&c.input["custom_prefix"]),
        );
        eq(got.as_str(), c.expected.as_str().unwrap())
    });
}

#[test]
fn upload_storage_s3_region() {
    let t = TempTree::empty();
    run("UploadStorage.s3_region", |c| {
        eq(us::s3_region(&config_for(&c.input, t.path())).as_str(), c.expected.as_str().unwrap())
    });
}

#[test]
fn upload_storage_s3_url() {
    let t = TempTree::empty();
    run("UploadStorage.s3_url", |c| {
        let cfg = config_for(&c.input, t.path());
        let i = &c.input;
        let got = us::s3_url(&cfg, i["bucket"].as_str().unwrap(), i["region"].as_str().unwrap(), i["key"].as_str().unwrap());
        eq(got.as_str(), c.expected.as_str().unwrap())
    });
}

#[test]
fn upload_storage_sanitize_s3_key() {
    run("UploadStorage.sanitize_s3_key", |c| {
        eq(us::sanitize_s3_key(opt_str(&c.input["value"])).as_str(), c.expected.as_str().unwrap())
    });
}

#[test]
fn upload_storage_unique_s3_key() {
    run("UploadStorage.unique_s3_key", |c| {
        let got =
            us::unique_s3_key(&ctx_from_pinned(&c.input), c.input["key"].as_str().unwrap(), opt_str(&c.input["original_filename"]));
        eq(got.as_str(), c.expected.as_str().unwrap())
    });
}

#[test]
fn upload_storage_validate_filename() {
    let t = TempTree::empty();
    run("UploadStorage.validate_filename!", |c| {
        let cfg = config_for(&c.input, t.path());
        let kind = match (c.input["config_key"].as_str(), c.input["kind"].as_str()) {
            (Some("image_upload_extensions"), Some("image")) => UploadKind::Image,
            (Some("video_upload_extensions"), Some("video")) => UploadKind::Video,
            other => return Err(format!("unmapped {other:?}")),
        };
        let filename = c.input["filename"].as_str().unwrap_or_default();
        check_rejected(us::validate_filename(&cfg, filename, kind), &c.expected, |ext| {
            eq(ext.as_str(), c.expected.as_str().unwrap_or_default())
        })
    });
}
