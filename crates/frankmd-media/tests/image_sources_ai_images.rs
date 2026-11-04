//! Port of `test/javascript/image_sources/ai_images.test.js` (34 tests).

#[path = "common/client_picker.rs"]
mod client_picker;

use client_picker::{FakeTransport, s};
use frankmd_media::picker::{AiImageSource, JsArg, PickerCx, PickerResponse, RefImage, RequestBody};
use serde_json::{Value, json};

const NOW: i64 = 1_700_000_000_000;

fn enabled() -> AiImageSource {
    AiImageSource { enabled: true, ..AiImageSource::default() }
}

fn generate(tr: &mut FakeTransport, src: &mut AiImageSource, prompt: &str) -> Value {
    let mut cx = PickerCx { transport: tr, t: None };
    src.generate(&mut cx, &s(prompt)).expect("ok")
}

fn json_body(tr: &FakeTransport, i: usize) -> Value {
    match &tr.requests[i].body {
        RequestBody::Json(v) => v.clone(),
        other => panic!("expected JSON body, got {other:?}"),
    }
}

fn fail(tr: &mut FakeTransport, json: Value) {
    tr.replies.push_back(PickerResponse::Reply { ok: false, status: 422, json });
}

// port: test/javascript/image_sources/ai_images.test.js:22 "initializes with default state"
#[test]
fn default_state() {
    let src = AiImageSource::default();
    assert!(!src.enabled);
    assert_eq!(src.model, None);
    assert_eq!(src.generated_data, None);
    assert!(!src.is_generating());
    assert_eq!(src.ref_image, None);
}

// port: test/javascript/image_sources/ai_images.test.js:33 "clears generated data and ref image"
#[test]
fn reset_clears_data_and_ref() {
    let mut src = AiImageSource {
        generated_data: Some(json!({"data": "base64..."})),
        ref_image: Some(RefImage { path: "test.jpg".into(), name: String::new() }),
        ..AiImageSource::default()
    };
    src.reset();
    assert_eq!(src.generated_data, None);
    assert_eq!(src.ref_image, None);
}

// port: test/javascript/image_sources/ai_images.test.js:43 "aborts pending request"
#[test]
fn reset_leaves_no_pending_request() {
    // A synchronous transport has no request pending between calls; the
    // contract kept is that reset() never leaves an abort handle behind.
    let mut src = enabled();
    src.reset();
    assert!(!src.is_generating());
}

// skipped (debounce timer lives in the view): ai_images.test.js:53 "clears ref image search timeout"

// port: test/javascript/image_sources/ai_images.test.js:65 "loads config from API"
#[test]
fn load_config() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"enabled": true, "model": "imagen-3.0"}));
    let mut src = AiImageSource::default();
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let r = src.load_config(&mut cx).expect("ok");
    assert_eq!(tr.requests[0].method, "GET");
    assert_eq!(tr.requests[0].url, "/ai/image_config");
    assert!(src.enabled);
    assert_eq!(src.model.as_deref(), Some("imagen-3.0"));
    assert_eq!(r, json!({"enabled": true, "model": "imagen-3.0"}));
}

// port: test/javascript/image_sources/ai_images.test.js:82 "handles failed response"
#[test]
fn load_config_failed() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, Value::Null);
    let mut src = AiImageSource::default();
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let _ = src.load_config(&mut cx);
    assert!(!src.enabled);
}

// port: test/javascript/image_sources/ai_images.test.js:90 "handles network error"
#[test]
fn load_config_network_error() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::NetworkError("Network error".into()));
    let mut src = AiImageSource::default();
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let _ = src.load_config(&mut cx);
    assert!(!src.enabled);
}

// port: test/javascript/image_sources/ai_images.test.js:100 "returns error for empty prompt"
#[test]
fn generate_empty_prompt() {
    let mut tr = FakeTransport::default();
    let r = generate(&mut tr, &mut AiImageSource::default(), "");
    assert_eq!(r["error"], "Please enter a prompt describing the image you want to generate");
}

// port: test/javascript/image_sources/ai_images.test.js:105 "returns error when not enabled"
#[test]
fn generate_not_enabled() {
    let mut tr = FakeTransport::default();
    let r = generate(&mut tr, &mut AiImageSource::default(), "a cat");
    assert_eq!(r["error"], "AI image generation is not configured. Please add your Gemini API key to .fed");
    assert!(tr.requests.is_empty());
}

// port: test/javascript/image_sources/ai_images.test.js:113 "generates image successfully"
#[test]
fn generate_success() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"data": "base64imagedata", "url": null, "mime_type": "image/png", "model": "imagen-3.0", "revised_prompt": "A cute cat sitting"}));
    let mut src = enabled();
    let r = generate(&mut tr, &mut src, "a cat");
    assert_eq!(tr.requests[0].method, "POST");
    assert_eq!(tr.requests[0].url, "/ai/generate_image");
    assert!(tr.requests[0].abortable);
    assert_eq!(json_body(&tr, 0), json!({"prompt": "a cat"}));
    assert_eq!(r["success"], true);
    assert_eq!(r["imageData"]["data"], "base64imagedata");
    assert_eq!(r["previewSrc"], "data:image/png;base64,base64imagedata");
    assert!(src.generated_data.is_some());
}

// port: test/javascript/image_sources/ai_images.test.js:144 "includes reference image in request"
#[test]
fn generate_includes_reference() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"data": "base64", "mime_type": "image/png"}));
    let mut src = enabled();
    src.ref_image = Some(RefImage { path: "ref/photo.jpg".into(), name: "photo.jpg".into() });
    generate(&mut tr, &mut src, "a cat like this");
    assert_eq!(json_body(&tr, 0)["reference_image_path"], "ref/photo.jpg");
}

// port: test/javascript/image_sources/ai_images.test.js:159 "handles URL-based response"
#[test]
fn generate_url_response() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"url": "https://example.com/generated.jpg", "data": null}));
    let r = generate(&mut tr, &mut enabled(), "a cat");
    assert_eq!(r["previewSrc"], "https://example.com/generated.jpg");
}

// port: test/javascript/image_sources/ai_images.test.js:174 "handles API error"
#[test]
fn generate_api_error() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"error": "Content policy violation"}));
    assert_eq!(generate(&mut tr, &mut enabled(), "bad prompt")["error"], "Content policy violation");
}

// port: test/javascript/image_sources/ai_images.test.js:186 "handles abort"
#[test]
fn generate_abort() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::Aborted);
    assert_eq!(generate(&mut tr, &mut enabled(), "a cat")["cancelled"], true);
}

// port: test/javascript/image_sources/ai_images.test.js:199 "handles network error"
#[test]
fn generate_network_error() {
    let mut tr = FakeTransport::default();
    tr.replies.push_back(PickerResponse::NetworkError("Network error".into()));
    assert_eq!(generate(&mut tr, &mut enabled(), "a cat")["error"], "Failed to generate image. Please try again.");
}

// port: test/javascript/image_sources/ai_images.test.js:209 "clears abort controller after completion"
#[test]
fn generate_clears_abort_handle() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"data": "base64", "mime_type": "image/png"}));
    let mut src = enabled();
    generate(&mut tr, &mut src, "a cat");
    assert!(!src.is_generating());
}

// port: test/javascript/image_sources/ai_images.test.js:223 "aborts pending request"
#[test]
fn abort_without_pending_is_safe_and_idle() {
    // See reset_leaves_no_pending_request: cancellation itself is exercised by `generate_abort`.
    let mut src = enabled();
    src.abort();
    assert!(!src.is_generating());
}

// port: test/javascript/image_sources/ai_images.test.js:232 "handles no pending request"
#[test]
fn abort_with_no_pending_request() {
    AiImageSource::default().abort();
}

// port: test/javascript/image_sources/ai_images.test.js:239 "sets reference image"
#[test]
fn set_ref_image() {
    let mut src = AiImageSource::default();
    src.set_ref_image("/images/ref.jpg", "ref.jpg");
    assert_eq!(src.ref_image, Some(RefImage { path: "/images/ref.jpg".into(), name: "ref.jpg".into() }));
}

// port: test/javascript/image_sources/ai_images.test.js:247 "clears reference image"
#[test]
fn clear_ref_image() {
    let mut src = AiImageSource { ref_image: Some(RefImage { path: "test.jpg".into(), name: "test.jpg".into() }), ..AiImageSource::default() };
    src.clear_ref_image();
    assert_eq!(src.ref_image, None);
}

// port: test/javascript/image_sources/ai_images.test.js:257 "loads images from API"
#[test]
fn load_ref_images() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!([{"path": "a.jpg", "name": "a.jpg"}]));
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let r = AiImageSource::default().load_ref_images(&mut cx, &JsArg::Undefined).expect("ok");
    assert_eq!(r, json!([{"path": "a.jpg", "name": "a.jpg"}]));
    assert_eq!(tr.requests[0].url, "/images");
}

// port: test/javascript/image_sources/ai_images.test.js:274 "loads images with search"
#[test]
fn load_ref_images_with_search() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!([]));
    let mut cx = PickerCx { transport: &mut tr, t: None };
    AiImageSource::default().load_ref_images(&mut cx, &s("cat")).expect("ok");
    assert_eq!(tr.requests[0].method, "GET");
    assert_eq!(tr.requests[0].url, "/images?search=cat");
}

// port: test/javascript/image_sources/ai_images.test.js:288 "handles error"
#[test]
fn load_ref_images_error() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, Value::Null);
    let mut cx = PickerCx { transport: &mut tr, t: None };
    let r = AiImageSource::default().load_ref_images(&mut cx, &JsArg::Undefined).expect("ok");
    assert_eq!(r["error"], "Error loading images");
}

// port: test/javascript/image_sources/ai_images.test.js:298 "renders empty state when no images"
#[test]
fn ref_grid_empty() {
    assert!(AiImageSource::default().ref_grid_items(&[]).is_empty());
}

// port: test/javascript/image_sources/ai_images.test.js:306 "renders image buttons"
#[test]
fn ref_grid_items() {
    let items = AiImageSource::default().ref_grid_items(&[json!({"path": "photos/ref.jpg", "name": "ref.jpg"})]);
    assert_eq!(items[0].path, "photos/ref.jpg");
    assert_eq!(items[0].name, "ref.jpg");
    assert_eq!(items[0].preview_url, "/images/preview/photos/ref.jpg");
}

// port: test/javascript/image_sources/ai_images.test.js:319 "marks selected reference image"
#[test]
fn ref_grid_marks_selected() {
    let src = AiImageSource { ref_image: Some(RefImage { path: "photos/a.jpg".into(), name: String::new() }), ..AiImageSource::default() };
    let items = src.ref_grid_items(&[json!({"path": "photos/a.jpg", "name": "a.jpg"}), json!({"path": "photos/b.jpg", "name": "b.jpg"})]);
    assert!(items[0].selected);
    assert!(!items[1].selected);
}

// skipped (C, HTML escaping): ai_images.test.js:332 "escapes HTML"

fn save(tr: &mut FakeTransport, src: &AiImageSource) -> Result<Value, frankmd_media::picker::Thrown> {
    let mut cx = PickerCx { transport: tr, t: None };
    src.save(&mut cx, None, NOW)
}

// port: test/javascript/image_sources/ai_images.test.js:345 "returns error when no generated data"
#[test]
fn save_without_data() {
    let mut tr = FakeTransport::default();
    assert_eq!(save(&mut tr, &AiImageSource::default()).expect("ok")["error"], "No generated image data available");
}

fn with_data(v: Value) -> AiImageSource {
    AiImageSource { generated_data: Some(v), ..AiImageSource::default() }
}

// port: test/javascript/image_sources/ai_images.test.js:353 "saves base64 image locally"
#[test]
fn save_base64_locally() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"url": "/images/ai_123.png"}));
    let r = save(&mut tr, &with_data(json!({"data": "base64imagedata", "mime_type": "image/png"}))).expect("ok");
    assert_eq!(tr.requests[0].url, "/images/upload_base64");
    assert_eq!(json_body(&tr, 0), json!({"data": "base64imagedata", "mime_type": "image/png", "filename": format!("ai_{NOW}.png")}));
    assert_eq!(r["url"], "/images/ai_123.png");
}

// port: test/javascript/image_sources/ai_images.test.js:377 "saves base64 image to S3"
#[test]
fn save_base64_never_sends_upload_to_s3_flag() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"url": "https://s3.example.com/ai_123.png"}));
    let r = save(&mut tr, &with_data(json!({"data": "base64imagedata", "mime_type": "image/png"}))).expect("ok");
    assert!(json_body(&tr, 0).get("upload_to_s3").is_none());
    assert_eq!(r["url"], "https://s3.example.com/ai_123.png");
}

// port: test/javascript/image_sources/ai_images.test.js:395 "downloads URL results into the active provider"
#[test]
fn save_url_imports() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"url": "images/generated.jpg"}));
    let r = save(&mut tr, &with_data(json!({"url": "https://ai-service.com/generated.jpg"}))).expect("ok");
    assert_eq!(tr.requests[0].url, "/images/import");
    assert_eq!(json_body(&tr, 0), json!({"url": "https://ai-service.com/generated.jpg"}));
    assert_eq!(r["url"], "images/generated.jpg");
}

// port: test/javascript/image_sources/ai_images.test.js:407 "uploads URL image to S3"
#[test]
fn save_url_with_null_data_imports() {
    let mut tr = FakeTransport::default();
    tr.push_json(json!({"url": "https://s3.example.com/ai_123.jpg"}));
    let r = save(&mut tr, &with_data(json!({"url": "https://ai-service.com/generated.jpg", "data": null}))).expect("ok");
    assert_eq!(tr.requests[0].method, "POST");
    assert_eq!(tr.requests[0].url, "/images/import");
    assert_eq!(json_body(&tr, 0), json!({"url": "https://ai-service.com/generated.jpg"}));
    assert_eq!(r["url"], "https://s3.example.com/ai_123.jpg");
}

// port: test/javascript/image_sources/ai_images.test.js:431 "handles save error for base64"
#[test]
fn save_base64_error_throws() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, json!({"error": "Disk full"}));
    assert_eq!(save(&mut tr, &with_data(json!({"data": "base64", "mime_type": "image/png"}))).expect_err("throws").message, "Disk full");
}

// port: test/javascript/image_sources/ai_images.test.js:442 "handles save error for S3 URL upload"
#[test]
fn save_url_error_throws() {
    let mut tr = FakeTransport::default();
    fail(&mut tr, json!({"error": "S3 error"}));
    assert_eq!(save(&mut tr, &with_data(json!({"url": "https://example.com/img.jpg"}))).expect_err("throws").message, "S3 error");
}
