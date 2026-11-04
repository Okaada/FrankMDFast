//! **[COMPAT]** port of the image-picker sources in
//! `app/javascript/lib/image_sources/*.js` (`image_picker_text`, web, google,
//! pinterest, local, ai, folder/drop). Golden: `js_image_sources.json`.
//!
//! The JS classes talked to the Rails endpoints through `@rails/request.js`;
//! here every source is a plain state machine over an injected [`Transport`]
//! (the app wires it to the in-process services: `search`, `images`, `api`,
//! `frankmd-ai`), so the request/response contract the UI relied on stays
//! testable. Endpoint payloads are kept as JSON values (exactly what the JS
//! passed through); DOM rendering (`renderGrid`'s `innerHTML`, `deselectAll`)
//! is not ported — only the data it was computed from ([`FolderImageSource::status_text`]).
//!
//! Results follow the JS objects: `Ok(value)` (JS `undefined` = `Value::Null`)
//! or `Err(Thrown)` where the JS threw.

use serde_json::Value;

use crate::url_utils::{encode_path, encode_uri_component};

/// A JS argument value where `undefined` vs `null` vs number vs string matters.
#[derive(Debug, Clone, PartialEq)]
pub enum JsArg {
    /// `undefined`.
    Undefined,
    /// `null`.
    Null,
    /// Boolean.
    Bool(bool),
    /// Number (`NaN` allowed).
    Number(f64),
    /// String.
    Str(String),
}

impl JsArg {
    /// From a decoded fixture value (`null` → `Null`; use [`JsArg::Undefined`] explicitly).
    pub fn from_json(v: &Value) -> Self {
        match v {
            Value::Null => JsArg::Null,
            Value::Bool(b) => JsArg::Bool(*b),
            Value::Number(n) => JsArg::Number(n.as_f64().unwrap_or(f64::NAN)),
            Value::String(s) => JsArg::Str(s.clone()),
            other => JsArg::Str(other.to_string()),
        }
    }

    /// JS `String(x)` (`1.5` → `"1.5"`, `1000` → `"1000"`, `NaN` → `"NaN"`, `null` → `"null"`).
    pub fn to_js_string(&self) -> String {
        match self {
            JsArg::Undefined => "undefined".to_owned(),
            JsArg::Null => "null".to_owned(),
            JsArg::Bool(b) => b.to_string(),
            JsArg::Number(n) => js_number_string(*n),
            JsArg::Str(s) => s.clone(),
        }
    }

    /// JS `Number(x)` (`null` → 0, `"abc"` → NaN, `undefined` → NaN).
    pub fn to_number(&self) -> f64 {
        match self {
            JsArg::Undefined => f64::NAN,
            JsArg::Null => 0.0,
            JsArg::Bool(b) => f64::from(u8::from(*b)),
            JsArg::Number(n) => *n,
            JsArg::Str(s) => js_string_to_number(s),
        }
    }
}

/// Interpolation options (`{count: 3}`), insertion-ordered.
pub type TextOptions = Vec<(String, JsArg)>;

/// `window.t(fullKey, options)`; returning the key itself means "missing".
pub trait Translator {
    /// Translate.
    fn t(&self, full_key: &str, options: &TextOptions) -> String;
}

/// `imagePickerText(key, options = {}, fallback = key)`: `window.t("dialogs.image_picker.<key>")`
/// unless absent or it returns the key; else `fallback` with `%{name}` replaced
/// by `String(options[name])` when defined (unknown / undefined tokens kept).
pub fn image_picker_text(
    t: Option<&dyn Translator>,
    key: &str,
    options: &TextOptions,
    fallback: Option<&str>,
) -> String {
    let full_key = format!("dialogs.image_picker.{key}");
    if let Some(t) = t {
        let translated = t.t(&full_key, options);
        if translated != full_key {
            return translated;
        }
    }
    let fallback = fallback.unwrap_or(key);
    let mut out = String::with_capacity(fallback.len());
    let mut rest = fallback;
    while let Some(start) = rest.find("%{") {
        let after = &rest[start + 2..];
        let name_len = after
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        if name_len == 0 || !after[name_len..].starts_with('}') {
            out.push_str(&rest[..start + 2]);
            rest = after;
            continue;
        }
        let name = &after[..name_len];
        out.push_str(&rest[..start]);
        match options.iter().find(|(k, _)| k == name).map(|(_, v)| v) {
            None | Some(JsArg::Undefined) => out.push_str(&rest[start..start + 3 + name_len]),
            Some(value) => out.push_str(&value.to_js_string()),
        }
        rest = &after[name_len + 1..];
    }
    out.push_str(rest);
    out
}

/// `imagePickerSearchResultMessage(count)`: `search_result_one` iff `Number(count) === 1`,
/// else `search_result_many` with `count: Number(count)`.
pub fn image_picker_search_result_message(t: Option<&dyn Translator>, count: &JsArg) -> String {
    let image_count = count.to_number();
    let (key, fallback) = if image_count == 1.0 {
        ("search_result_one", "Found 1 image - click to select")
    } else {
        (
            "search_result_many",
            "Found %{count} images - click to select",
        )
    };
    image_picker_text(
        t,
        key,
        &vec![("count".to_owned(), JsArg::Number(image_count))],
        Some(fallback),
    )
}

/// `DEFAULT_IMAGE_EXTENSIONS`.
pub const DEFAULT_IMAGE_EXTENSIONS: [&str; 6] = [".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"];

/// `extOf(filename)`: lowercased trailing `\.[^.]+$`, else `no_file_extension` ("no extension").
pub fn ext_of(t: Option<&dyn Translator>, filename: &str) -> String {
    let lower = filename.to_lowercase();
    match lower.rfind('.') {
        Some(dot) if dot + 1 < lower.len() => lower[dot..].to_owned(),
        _ => text(t, "no_file_extension", "no extension"),
    }
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

/// A multipart field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormField {
    /// Text value.
    Text(String),
    /// File part.
    File {
        /// Name.
        name: String,
        /// Bytes.
        size: u64,
        /// MIME type.
        mime: String,
    },
}

/// Request body.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestBody {
    /// No body (GET/DELETE).
    None,
    /// JSON object (keys omitted where the JS left them `undefined`).
    Json(Value),
    /// `FormData`, in append order.
    Form(Vec<(String, FormField)>),
}

/// One recorded request (`responseKind` is always `"json"`).
#[derive(Debug, Clone, PartialEq)]
pub struct PickerRequest {
    /// `GET` / `POST` / `DELETE`.
    pub method: &'static str,
    /// Path + query (`/images/search_web?q=cats`, `encodeURIComponent` encoding).
    pub url: String,
    /// Body.
    pub body: RequestBody,
    /// An abort signal was attached (only `/ai/generate_image`).
    pub abortable: bool,
}

/// What came back.
#[derive(Debug, Clone, PartialEq)]
pub enum PickerResponse {
    /// HTTP reply (`ok` = 2xx).
    Reply {
        /// `response.ok`.
        ok: bool,
        /// Status.
        status: u16,
        /// Parsed JSON body.
        json: Value,
    },
    /// Network failure (fetch rejected).
    NetworkError(String),
    /// Aborted via the signal (`AbortError`).
    Aborted,
}

/// Request transport (fake in tests, in-process services in the app).
pub trait Transport {
    /// Perform one request.
    fn send(&mut self, request: PickerRequest) -> PickerResponse;
}

/// Per-call context.
pub struct PickerCx<'a> {
    /// Transport.
    pub transport: &'a mut dyn Transport,
    /// `window.t`, if any.
    pub t: Option<&'a dyn Translator>,
}

/// A thrown JS `Error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thrown {
    /// `error.name` (`"Error"`).
    pub name: String,
    /// `error.message`.
    pub message: String,
}

/// JS-shaped outcome.
pub type Outcome = Result<Value, Thrown>;

// ---------------------------------------------------------------------------
// Sources
// ---------------------------------------------------------------------------

/// `WebImageSource` (DuckDuckGo via `/images/search_web`).
#[derive(Debug, Clone, Default)]
pub struct WebImageSource {
    /// `results`.
    pub results: Vec<Value>,
}

impl WebImageSource {
    /// `reset()`.
    pub fn reset(&mut self) {
        self.results.clear();
    }
    /// `search(query)` → `{error}` | `{images, message}` (message: `note` or
    /// "No images found" when empty, else the result-count message).
    pub fn search(&mut self, cx: &mut PickerCx<'_>, query: &JsArg) -> Outcome {
        ddg_search(cx, &mut self.results, "/images/search_web", query, true)
    }
    /// `saveCopy(url, resize, s3Prefix = "")` → POST `/images/import {url, resize[, s3_prefix]}`;
    /// non-ok throws `data.error || "Failed to upload to S3"`.
    pub fn save_copy(
        &self,
        cx: &mut PickerCx<'_>,
        url: &Value,
        resize: &Value,
        s3_prefix: Option<&str>,
    ) -> Outcome {
        let mut body = serde_json::Map::new();
        body.insert("url".into(), defined(url));
        body.insert("resize".into(), defined(resize));
        if let Some(prefix) = s3_prefix.filter(|p| !p.is_empty()) {
            body.insert("s3_prefix".into(), Value::String(prefix.to_owned()));
        }
        post_or_throw(
            cx,
            "/images/import",
            RequestBody::Json(Value::Object(body)),
            "image_upload_failed",
            "Failed to upload to S3",
        )
    }
}

/// `PinterestImageSource` (`/images/search_pinterest`; no `note` handling).
#[derive(Debug, Clone, Default)]
pub struct PinterestImageSource {
    /// `results`.
    pub results: Vec<Value>,
}

impl PinterestImageSource {
    /// `reset()`.
    pub fn reset(&mut self) {
        self.results.clear();
    }
    /// `search(query)`.
    pub fn search(&mut self, cx: &mut PickerCx<'_>, query: &JsArg) -> Outcome {
        ddg_search(
            cx,
            &mut self.results,
            "/images/search_pinterest",
            query,
            false,
        )
    }
}

/// `GoogleImageSource` (`/images/search_google?q=&start=`, infinite scroll).
#[derive(Debug, Clone)]
pub struct GoogleImageSource {
    /// `enabled`.
    pub enabled: bool,
    /// `results`.
    pub results: Vec<Value>,
    /// `nextStart` (whatever the server sent as `next_start`; JS `undefined` = `Null`).
    pub next_start: Value,
    /// `loading` guard.
    pub loading: bool,
    /// `query`.
    pub query: String,
}

impl Default for GoogleImageSource {
    /// The JS constructor (`nextStart = 0`).
    fn default() -> Self {
        Self {
            enabled: false,
            results: Vec::new(),
            next_start: Value::from(0),
            loading: false,
            query: String::new(),
        }
    }
}

impl GoogleImageSource {
    /// `reset()` (`nextStart = 0`).
    pub fn reset(&mut self) {
        self.results.clear();
        self.next_start = Value::from(0);
        self.loading = false;
        self.query.clear();
    }
    /// `search(query)`: blank → `{error}`; else reset to `nextStart = 1` and `loadMore()`.
    pub fn search(&mut self, cx: &mut PickerCx<'_>, query: &JsArg) -> Outcome {
        if !query.truthy() {
            return Ok(error_json(text(
                cx.t,
                "search_keywords_required",
                "Please enter search keywords",
            )));
        }
        self.results.clear();
        self.next_start = Value::from(1);
        self.query = query.to_js_string();
        self.load_more(cx)
    }
    /// `loadMore()`: while `loading` returns `{images: results}` without a request.
    pub fn load_more(&mut self, cx: &mut PickerCx<'_>) -> Outcome {
        if self.loading {
            return Ok(serde_json::json!({ "images": self.results }));
        }
        self.loading = true;
        // JS template-literal stringification; a missing `next_start` reads "undefined".
        let start = if self.next_start.is_null() {
            "undefined".to_owned()
        } else {
            value_to_js_string(&self.next_start)
        };
        let url = format!(
            "/images/search_google?q={}&start={start}",
            encode_uri_component(&self.query)
        );
        let outcome = self.apply_google_page(cx, url);
        self.loading = false;
        Ok(outcome.unwrap_or_else(|()| {
            error_json(text(
                cx.t,
                "search_failed",
                "Search failed. Please try again.",
            ))
        }))
    }
    /// `shouldLoadMore(scrollBottom)`: `< 100 && !loading && results non-empty`.
    pub fn should_load_more(&self, scroll_bottom: f64) -> bool {
        scroll_bottom < 100.0 && !self.loading && !self.results.is_empty()
    }
}

/// `LocalImageSource` (the server `images_path` folder).
#[derive(Debug, Clone, Default)]
pub struct LocalImageSource {
    /// `selectedPath`.
    pub selected_path: Option<String>,
}

impl LocalImageSource {
    /// `reset()`.
    pub fn reset(&mut self) {
        self.selected_path = None;
    }
    /// `load(search = "", {limit = 10, offset = 0})` → GET `/images?[search=]&limit=&offset=`
    /// (each value `encodeURIComponent(String(v))`); non-ok → `{error: data.error || "Error loading images"}`.
    pub fn load(
        &mut self,
        cx: &mut PickerCx<'_>,
        search: &JsArg,
        limit: &JsArg,
        offset: &JsArg,
    ) -> Outcome {
        let mut params = Vec::new();
        if search.truthy() {
            params.push(format!(
                "search={}",
                encode_uri_component(&search.to_js_string())
            ));
        }
        let limit = if *limit == JsArg::Undefined {
            "10".to_owned()
        } else {
            limit.to_js_string()
        };
        let offset = if *offset == JsArg::Undefined {
            "0".to_owned()
        } else {
            offset.to_js_string()
        };
        params.push(format!("limit={}", encode_uri_component(&limit)));
        params.push(format!("offset={}", encode_uri_component(&offset)));
        Ok(get_or_error(cx, format!("/images?{}", params.join("&"))))
    }
    /// `deleteImage(path)` → DELETE `/images/file/<encodePath>`; non-ok throws.
    pub fn delete_image(&self, cx: &mut PickerCx<'_>, path: &str) -> Outcome {
        let url = format!("/images/file/{}", encode_path(Some(path)));
        let (ok, data) =
            send(cx, "DELETE", url, RequestBody::None, false).map_err(RequestFailure::thrown)?;
        if !ok {
            let error = prop(&data, "error")?;
            return Err(js_error(&or_text(
                error,
                cx.t,
                "image_delete_failed",
                "Failed to delete image",
            )));
        }
        Ok(data)
    }
    /// `saveCopy(path, resize, s3Key = "")` → POST `/images/import {path, resize[, s3_key]}`.
    pub fn save_copy(
        &self,
        cx: &mut PickerCx<'_>,
        path: &Value,
        resize: &Value,
        s3_key: Option<&str>,
    ) -> Outcome {
        let mut body = serde_json::Map::new();
        body.insert("path".into(), defined(path));
        body.insert("resize".into(), defined(resize));
        if let Some(key) = s3_key.filter(|k| !k.is_empty()) {
            body.insert("s3_key".into(), Value::String(key.to_owned()));
        }
        post_or_throw(
            cx,
            "/images/import",
            RequestBody::Json(Value::Object(body)),
            "image_upload_failed",
            "Failed to upload to S3",
        )
    }
}

/// Selected AI reference image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefImage {
    /// Server-folder path.
    pub path: String,
    /// Name.
    pub name: String,
}

/// `AiImageSource`.
#[derive(Debug, Clone, Default)]
pub struct AiImageSource {
    /// `enabled`.
    pub enabled: bool,
    /// `model`.
    pub model: Option<String>,
    /// `generatedData` (`{data, url, mime_type, model, revised_prompt}`).
    pub generated_data: Option<Value>,
    /// `refImage`.
    pub ref_image: Option<RefImage>,
}

impl AiImageSource {
    /// `reset()` (drops generated data and ref image, aborts in-flight).
    pub fn reset(&mut self) {
        self.generated_data = None;
        self.ref_image = None;
    }
    /// `loadConfig()` → GET `/ai/image_config`; failures keep the previous values.
    pub fn load_config(&mut self, cx: &mut PickerCx<'_>) -> Outcome {
        if let Ok((true, config)) = send(
            cx,
            "GET",
            "/ai/image_config".to_owned(),
            RequestBody::None,
            false,
        ) && let (Ok(enabled), Ok(model)) = (prop(&config, "enabled"), prop(&config, "model"))
        {
            self.enabled = truthy(&enabled);
            self.model = model.as_str().map(str::to_owned);
        }
        Ok(serde_json::json!({ "enabled": self.enabled, "model": self.model }))
    }
    /// `generate(prompt)`: blank → `ai_prompt_required`; disabled → `ai_not_configured`;
    /// POST `/ai/generate_image {prompt[, reference_image_path]}` (abortable) →
    /// `{success, imageData, previewSrc}` | `{error}` | `{cancelled: true}`.
    pub fn generate(&mut self, cx: &mut PickerCx<'_>, prompt: &JsArg) -> Outcome {
        if !prompt.truthy() {
            return Ok(error_json(text(
                cx.t,
                "ai_prompt_required",
                "Please enter a prompt describing the image you want to generate",
            )));
        }
        if !self.enabled {
            return Ok(error_json(text(
                cx.t,
                "ai_not_configured",
                "AI image generation is not configured. Please add your Gemini API key to .fed",
            )));
        }
        let mut body = serde_json::Map::new();
        body.insert("prompt".into(), prompt.to_json());
        if let Some(reference) = self.ref_image.as_ref().filter(|r| !r.path.is_empty()) {
            body.insert(
                "reference_image_path".into(),
                Value::String(reference.path.clone()),
            );
        }
        let t = cx.t;
        let failed = || {
            error_json(text(
                t,
                "ai_generation_failed",
                "Failed to generate image. Please try again.",
            ))
        };
        let data = match send(
            cx,
            "POST",
            "/ai/generate_image".to_owned(),
            RequestBody::Json(Value::Object(body)),
            true,
        ) {
            Ok((_, data)) => data,
            Err(RequestFailure::Aborted) => return Ok(serde_json::json!({ "cancelled": true })),
            Err(RequestFailure::Network(_)) => return Ok(failed()),
        };
        let Ok(error) = prop(&data, "error") else {
            return Ok(failed());
        };
        if truthy(&error) {
            return Ok(error_json(error));
        }
        let field = |key: &str| prop(&data, key).unwrap_or(Value::Null);
        let generated = serde_json::json!({
            "data": field("data"),
            "url": field("url"),
            "mime_type": field("mime_type"),
            "model": field("model"),
            "revised_prompt": field("revised_prompt"),
        });
        let preview = if truthy(&field("data")) {
            let mime = field("mime_type");
            let mime = if truthy(&mime) {
                value_to_js_string(&mime)
            } else {
                "image/png".to_owned()
            };
            Value::String(format!(
                "data:{mime};base64,{}",
                value_to_js_string(&field("data"))
            ))
        } else {
            field("url")
        };
        self.generated_data = Some(generated.clone());
        Ok(serde_json::json!({ "success": true, "imageData": generated, "previewSrc": preview }))
    }
    /// `abort()`: cancel an in-flight `generate` (no-op when none). The app's
    /// transport resolves the cancelled request as [`PickerResponse::Aborted`].
    pub fn abort(&mut self) {
        // Requests complete synchronously through the transport, so nothing
        // is ever in flight between calls; the app's transport cancels itself.
    }
    /// An `abortController` is live (a `generate` is in flight).
    pub fn is_generating(&self) -> bool {
        false
    }
    /// `setRefImage(path, name)`.
    pub fn set_ref_image(&mut self, path: &str, name: &str) {
        self.ref_image = Some(RefImage {
            path: path.to_owned(),
            name: name.to_owned(),
        });
    }
    /// `clearRefImage()`.
    pub fn clear_ref_image(&mut self) {
        self.ref_image = None;
    }
    /// `loadRefImages(search = "")` → GET `/images[?search=]`.
    pub fn load_ref_images(&self, cx: &mut PickerCx<'_>, search: &JsArg) -> Outcome {
        let url = if search.truthy() {
            format!(
                "/images?search={}",
                encode_uri_component(&search.to_js_string())
            )
        } else {
            "/images".to_owned()
        };
        Ok(get_or_error(cx, url))
    }
    /// `save(s3Prefix = "")`: no data → `{error}`; base64 data → POST
    /// `/images/upload_base64 {data, mime_type, filename: "ai_<now_ms>.png"[, s3_prefix]}`,
    /// URL → POST `/images/import {url[, s3_prefix]}`; non-ok throws.
    pub fn save(&self, cx: &mut PickerCx<'_>, s3_prefix: Option<&str>, now_ms: i64) -> Outcome {
        let Some(generated) = &self.generated_data else {
            return Ok(error_json(text(
                cx.t,
                "ai_no_generated_data",
                "No generated image data available",
            )));
        };
        let field = |key: &str| prop(generated, key).unwrap_or(Value::Null);
        let has_data = truthy(&field("data"));
        let mut body = serde_json::Map::new();
        if has_data {
            body.insert("data".into(), field("data"));
            body.insert("mime_type".into(), field("mime_type"));
            body.insert("filename".into(), Value::String(format!("ai_{now_ms}.png")));
        } else {
            body.insert("url".into(), field("url"));
        }
        if let Some(prefix) = s3_prefix.filter(|p| !p.is_empty()) {
            body.insert("s3_prefix".into(), Value::String(prefix.to_owned()));
        }
        let endpoint = if has_data {
            "/images/upload_base64"
        } else {
            "/images/import"
        };
        post_or_throw(
            cx,
            endpoint,
            RequestBody::Json(Value::Object(body)),
            "ai_save_failed",
            "Failed to save image",
        )
    }
}

/// A local file handed to the folder/drop source (stands in for `File`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickedFile {
    /// Name.
    pub name: String,
    /// `lastModified` (ms).
    pub last_modified: i64,
    /// Bytes.
    pub size: u64,
    /// MIME type.
    pub mime: String,
    /// Decoded pixel size (`None` = the image failed to load → `width/height: null`).
    pub dimensions: Option<(u32, u32)>,
}

/// A displayed folder image (`displayedImages[i]`, minus the blob URL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayedImage {
    /// The file.
    pub file: PickedFile,
    /// Width (`null` when undecodable).
    pub width: Option<u32>,
    /// Height.
    pub height: Option<u32>,
}

/// One directory entry from the folder picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirEntry {
    /// A file.
    File(PickedFile),
    /// A sub-directory (ignored, no recursion).
    Directory(String),
}

/// Why the directory picker produced nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickError {
    /// User cancelled (`AbortError`) → `{cancelled: true}`.
    Aborted,
    /// Any other failure → `folder_access_failed`.
    Failed(String),
}

/// `window.showDirectoryPicker()` equivalent (native folder dialog in the app).
pub trait DirectoryPicker {
    /// Pick a folder and list its direct entries.
    fn pick(&mut self) -> Result<Vec<DirEntry>, PickError>;
}

/// `FolderImageSource` (folder browse + drag-and-drop ingest).
#[derive(Debug, Clone, Default)]
pub struct FolderImageSource {
    /// `allImages`, newest first.
    pub all_images: Vec<PickedFile>,
    /// `displayedImages` (current page).
    pub displayed_images: Vec<DisplayedImage>,
}

impl FolderImageSource {
    /// `reset()` / `cleanup()`.
    pub fn reset(&mut self) {
        self.displayed_images.clear();
        self.all_images.clear();
    }
    /// `browse(allowed)`: `picker` `None` → `folder_api_not_supported`; abort →
    /// `{cancelled}`; error → `folder_access_failed`; else `{count}` (files whose
    /// lowercased name ends with an allowed ext; dirs skipped).
    pub fn browse(
        &mut self,
        t: Option<&dyn Translator>,
        picker: Option<&mut dyn DirectoryPicker>,
        allowed: Option<&[String]>,
    ) -> Outcome {
        let Some(picker) = picker else {
            return Ok(error_json(text(
                t,
                "folder_api_not_supported",
                "File System Access API not supported",
            )));
        };
        let entries = match picker.pick() {
            Ok(entries) => entries,
            Err(PickError::Aborted) => return Ok(serde_json::json!({ "cancelled": true })),
            Err(PickError::Failed(_)) => {
                return Ok(error_json(text(
                    t,
                    "folder_access_failed",
                    "Error accessing folder",
                )));
            }
        };
        self.reset();
        let allowed = allowed_extensions(allowed);
        self.all_images = entries
            .into_iter()
            .filter_map(|entry| match entry {
                DirEntry::File(file) if has_allowed_extension(&file.name, &allowed) => Some(file),
                _ => None,
            })
            .collect();
        self.sort_and_show();
        Ok(serde_json::json!({ "count": self.all_images.len() }))
    }
    /// `ingest(files, allowed)` → `{count, rejected: [{name, ext}]}` (sort by
    /// `lastModified` desc, then `filter("")`).
    pub fn ingest(
        &mut self,
        t: Option<&dyn Translator>,
        files: Vec<PickedFile>,
        allowed: Option<&[String]>,
    ) -> Outcome {
        self.reset();
        let allowed = allowed_extensions(allowed);
        let mut rejected = Vec::new();
        for file in files {
            if has_allowed_extension(&file.name, &allowed) {
                self.all_images.push(file);
            } else {
                rejected
                    .push(serde_json::json!({ "name": file.name, "ext": ext_of(t, &file.name) }));
            }
        }
        self.sort_and_show();
        Ok(serde_json::json!({ "count": self.all_images.len(), "rejected": rejected }))
    }
    /// `filter(term, {offset = 0, limit = 10})` → `{displayed, total, offset, limit}`
    /// (case-insensitive trimmed substring).
    pub fn filter(&mut self, term: &str, offset: usize, limit: usize) -> Outcome {
        let term = js_trim(&term.to_lowercase()).to_owned();
        let filtered: Vec<&PickedFile> = self
            .all_images
            .iter()
            .filter(|img| term.is_empty() || img.name.to_lowercase().contains(&term))
            .collect();
        let total = filtered.len();
        self.displayed_images = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|file| DisplayedImage {
                width: file.dimensions.map(|d| d.0),
                height: file.dimensions.map(|d| d.1),
                file: file.clone(),
            })
            .collect();
        Ok(
            serde_json::json!({ "displayed": self.displayed_images.len(), "total": total, "offset": offset, "limit": limit }),
        )
    }
    /// `getImage(index)`.
    pub fn get_image(&self, index: usize) -> Option<&DisplayedImage> {
        self.displayed_images.get(index)
    }
    /// The status line `renderGrid` wrote: none displayed → "No images found";
    /// `total_count > shown` → `folder_showing_recent`; else `folder_images_found_one|many`.
    pub fn status_text(&self, t: Option<&dyn Translator>, total_count: Option<usize>) -> String {
        if self.displayed_images.is_empty() {
            return text(t, "no_images_found", "No images found");
        }
        let shown = self.displayed_images.len();
        let number = |n: usize| JsArg::Number(n as f64);
        match total_count {
            Some(total) if total > shown => image_picker_text(
                t,
                "folder_showing_recent",
                &vec![
                    ("shown".to_owned(), number(shown)),
                    ("total".to_owned(), number(total)),
                ],
                Some("Showing %{shown} most recent of %{total} images"),
            ),
            _ => {
                let (key, fallback) = if shown == 1 {
                    ("folder_images_found_one", "1 image found")
                } else {
                    ("folder_images_found_many", "%{count} images found")
                };
                image_picker_text(
                    t,
                    key,
                    &vec![("count".to_owned(), number(shown))],
                    Some(fallback),
                )
            }
        }
    }
    /// `upload(file, resize, s3Key = "")` → POST `/images/upload` FormData
    /// `file[, resize][, s3_key]` (falsy values omitted); non-ok throws `"Upload failed"`.
    pub fn upload(
        &self,
        cx: &mut PickerCx<'_>,
        file: &PickedFile,
        resize: &JsArg,
        s3_key: Option<&str>,
    ) -> Outcome {
        let mut form = vec![(
            "file".to_owned(),
            FormField::File {
                name: file.name.clone(),
                size: file.size,
                mime: file.mime.clone(),
            },
        )];
        if resize.truthy() {
            form.push(("resize".to_owned(), FormField::Text(resize.to_js_string())));
        }
        if let Some(key) = s3_key.filter(|k| !k.is_empty()) {
            form.push(("s3_key".to_owned(), FormField::Text(key.to_owned())));
        }
        post_or_throw(
            cx,
            "/images/upload",
            RequestBody::Form(form),
            "image_upload_failed",
            "Upload failed",
        )
    }
}

// ---------------------------------------------------------------------------
// Grid models (the data `renderGrid` interpolated into HTML; the GPUI view
// renders these as text, so HTML escaping/attribute injection does not apply)
// ---------------------------------------------------------------------------

/// `isSafeImageUrl` (lib/text_utils.js): no whitespace/control chars, and
/// `http(s):` / `data:image/` / `blob:` (case-insensitive) or a root-relative
/// path that is not `//` or `/\`.
pub fn is_safe_image_url(url: &str) -> bool {
    if url.chars().any(|c| is_js_whitespace(c) || c.is_control()) {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    ["http:", "https:", "data:image/", "blob:"]
        .iter()
        .any(|p| lower.starts_with(p))
        || (url.starts_with('/') && !url.starts_with("//") && !url.starts_with("/\\"))
}

/// `width`x`height` label when both are finite and > 0 (`Number(x)` semantics:
/// `"0"`, `"a"`, missing → no label).
pub fn dimensions_label(width: &Value, height: &Value) -> Option<String> {
    let (w, h) = (value_to_number(width), value_to_number(height));
    (w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0)
        .then(|| format!("{}x{}", js_number_string(w), js_number_string(h)))
}

/// One web/Google/Pinterest result tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalGridItem {
    /// `data-index`.
    pub index: usize,
    /// `data-url`.
    pub url: String,
    /// `data-thumbnail` (`thumbnail || url`).
    pub thumbnail: String,
    /// `<img src>`: the thumbnail if [`is_safe_image_url`], else `""`.
    pub image_src: String,
    /// `data-title` (`""` when missing); the tooltip uses `title || "Image"`.
    pub title: String,
    /// `data-source`.
    pub source: String,
    /// Dimension badge.
    pub dimensions: Option<String>,
}

/// Grid model shared by web, Google and Pinterest sources (empty → the view
/// shows `no_images_found`).
pub fn external_grid_items(results: &[Value]) -> Vec<ExternalGridItem> {
    results
        .iter()
        .enumerate()
        .map(|(index, image)| {
            let field = |key: &str| image.get(key).cloned().unwrap_or(Value::Null);
            let url = field("url");
            let thumbnail = if truthy(&field("thumbnail")) {
                field("thumbnail")
            } else {
                url.clone()
            };
            let image_src = match &thumbnail {
                Value::String(s) if is_safe_image_url(s) => s.clone(),
                _ => String::new(),
            };
            ExternalGridItem {
                index,
                url: js_text(&url),
                thumbnail: js_text(&thumbnail),
                image_src,
                title: js_text(&field("title")),
                source: js_text(&field("source")),
                dimensions: dimensions_label(&field("width"), &field("height")),
            }
        })
        .collect()
}

/// One local (server folder) / AI reference tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalGridItem {
    /// `data-path`.
    pub path: String,
    /// `data-name`.
    pub name: String,
    /// `/images/preview/<encodePath(path)>`.
    pub preview_url: String,
    /// Dimension badge (local grid only; `None` for the AI ref grid).
    pub dimensions: Option<String>,
    /// `selected` (local: `selectedPath`) / ring (AI: `refImage.path`).
    pub selected: bool,
}

impl LocalImageSource {
    /// `renderGrid(images)` data.
    pub fn grid_items(&self, images: &[Value]) -> Vec<LocalGridItem> {
        images
            .iter()
            .map(|image| {
                let mut item = local_grid_item(image, self.selected_path.as_deref());
                item.dimensions = dimensions_label(
                    image.get("width").unwrap_or(&Value::Null),
                    image.get("height").unwrap_or(&Value::Null),
                );
                item
            })
            .collect()
    }
}

impl AiImageSource {
    /// `renderRefImageGrid(images)` data.
    pub fn ref_grid_items(&self, images: &[Value]) -> Vec<LocalGridItem> {
        let selected = self.ref_image.as_ref().map(|r| r.path.as_str());
        images
            .iter()
            .map(|image| local_grid_item(image, selected))
            .collect()
    }
}

/// One folder/drop tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderGridItem {
    /// `data-index`.
    pub index: usize,
    /// Name.
    pub name: String,
    /// `Math.round(size / 1024)` (tooltip "… - N KB").
    pub size_kb: u64,
    /// Dimension badge.
    pub dimensions: Option<String>,
}

impl FolderImageSource {
    /// `renderGrid` data for `displayedImages`.
    pub fn grid_items(&self) -> Vec<FolderGridItem> {
        self.displayed_images
            .iter()
            .enumerate()
            .map(|(index, image)| FolderGridItem {
                index,
                name: image.file.name.clone(),
                size_kb: (image.file.size as f64 / 1024.0).round() as u64,
                dimensions: dimensions_label(
                    &image.width.map_or(Value::Null, Value::from),
                    &image.height.map_or(Value::Null, Value::from),
                ),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// JS semantics helpers
// ---------------------------------------------------------------------------

impl JsArg {
    /// JS truthiness.
    pub fn truthy(&self) -> bool {
        match self {
            JsArg::Undefined | JsArg::Null => false,
            JsArg::Bool(b) => *b,
            JsArg::Number(n) => *n != 0.0 && !n.is_nan(),
            JsArg::Str(s) => !s.is_empty(),
        }
    }

    /// The JSON value `JSON.stringify` would send (`undefined`/`NaN` → `null`).
    fn to_json(&self) -> Value {
        match self {
            JsArg::Undefined | JsArg::Null => Value::Null,
            JsArg::Bool(b) => Value::Bool(*b),
            JsArg::Number(n) if n.is_finite() && n.fract() == 0.0 && n.abs() < 9e15 => {
                Value::from(*n as i64)
            }
            JsArg::Number(n) => serde_json::Number::from_f64(*n).map_or(Value::Null, Value::Number),
            JsArg::Str(s) => Value::String(s.clone()),
        }
    }
}

/// `imagePickerText(key, {}, fallback)`.
fn text(t: Option<&dyn Translator>, key: &str, fallback: &str) -> String {
    image_picker_text(t, key, &Vec::new(), Some(fallback))
}

/// `{error: <value>}`.
fn error_json(error: impl Into<Value>) -> Value {
    serde_json::json!({ "error": error.into() })
}

/// JS truthiness of a JSON value.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// A fixture-encoded `undefined` (`{"$undefined": true}`) becomes `null`.
fn defined(v: &Value) -> Value {
    match v {
        Value::Object(o) if o.len() == 1 && o.contains_key("$undefined") => Value::Null,
        other => other.clone(),
    }
}

/// JS `String(x)` for a JSON value.
fn value_to_js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js_number_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                if item.is_null() {
                    String::new()
                } else {
                    value_to_js_string(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// `escapeHtml(x)`'s text before escaping: falsy → `""`, else `String(x)`.
fn js_text(v: &Value) -> String {
    if truthy(v) {
        value_to_js_string(v)
    } else {
        String::new()
    }
}

/// JS `Number(x)` for a JSON value (a missing field is `undefined` → NaN).
fn value_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => f64::NAN,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_string_to_number(s),
        Value::Array(_) | Value::Object(_) => f64::NAN,
    }
}

/// JS `x.key`: `null` throws a `TypeError`; non-objects have no own fields.
fn prop(v: &Value, key: &str) -> Result<Value, Thrown> {
    match v {
        Value::Null => Err(Thrown {
            name: "TypeError".to_owned(),
            message: format!("Cannot read properties of null (reading '{key}')"),
        }),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or(Value::Null)),
        _ => Ok(Value::Null),
    }
}

/// `value || imagePickerText(key, {}, fallback)`.
fn or_text(value: Value, t: Option<&dyn Translator>, key: &str, fallback: &str) -> Value {
    if truthy(&value) {
        value
    } else {
        Value::String(text(t, key, fallback))
    }
}

/// `new Error(message)`.
fn js_error(message: &Value) -> Thrown {
    Thrown {
        name: "Error".to_owned(),
        message: value_to_js_string(message),
    }
}

/// JS `String.prototype.trim` whitespace.
fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// JS `String.prototype.trim`.
fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// JS `Number.prototype.toString()` (shortest round-trip digits; exponent
/// form outside `1e-7 < |n| < 1e21`).
fn js_number_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            "Infinity".to_owned()
        } else {
            "-Infinity".to_owned()
        };
    }
    if n == 0.0 {
        return "0".to_owned();
    }
    let sign = if n < 0.0 { "-" } else { "" };
    let sci = format!("{:e}", n.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = digits.len() as i64;
    let point = exp.parse::<i64>().unwrap_or(0) + 1;
    let body = if k <= point && point <= 21 {
        format!("{digits}{}", "0".repeat((point - k) as usize))
    } else if 0 < point && point <= 21 {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let e = point - 1;
        let exp_sign = if e < 0 { '-' } else { '+' };
        let rest = if k > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        format!("{}{rest}e{exp_sign}{}", &digits[..1], e.abs())
    };
    format!("{sign}{body}")
}

/// JS `Number(string)` (`StringToNumber`).
fn js_string_to_number(s: &str) -> f64 {
    let s = js_trim(s);
    if s.is_empty() {
        return 0.0;
    }
    match s {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let radix = match s.get(..2) {
        Some("0x" | "0X") => Some(16),
        Some("0o" | "0O") => Some(8),
        Some("0b" | "0B") => Some(2),
        _ => None,
    };
    if let Some(radix) = radix {
        let digits = &s[2..];
        if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
            return f64::NAN;
        }
        return digits.chars().fold(0.0, |acc, c| {
            acc * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
        });
    }
    let valid = s
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
        && s.bytes().any(|b| b.is_ascii_digit());
    if valid {
        s.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// Why a request produced no reply.
enum RequestFailure {
    /// `fetch` rejected.
    Network(String),
    /// Cancelled through the abort signal.
    Aborted,
}

impl RequestFailure {
    /// The rejection a caller that does not catch would rethrow.
    fn thrown(self) -> Thrown {
        match self {
            RequestFailure::Network(message) => Thrown {
                name: "Error".to_owned(),
                message,
            },
            RequestFailure::Aborted => Thrown {
                name: "AbortError".to_owned(),
                message: "The operation was aborted.".to_owned(),
            },
        }
    }
}

/// One request through the transport → `(response.ok, response.json)`.
fn send(
    cx: &mut PickerCx<'_>,
    method: &'static str,
    url: String,
    body: RequestBody,
    abortable: bool,
) -> Result<(bool, Value), RequestFailure> {
    match cx.transport.send(PickerRequest {
        method,
        url,
        body,
        abortable,
    }) {
        PickerResponse::Reply { ok, json, .. } => Ok((ok, json)),
        PickerResponse::NetworkError(message) => Err(RequestFailure::Network(message)),
        PickerResponse::Aborted => Err(RequestFailure::Aborted),
    }
}

/// GET returning the JSON, or `{error: data.error || image_load_failed}`
/// (every failure is caught, as in `load` / `loadRefImages`).
fn get_or_error(cx: &mut PickerCx<'_>, url: String) -> Value {
    let failed = |t| error_json(text(t, "image_load_failed", "Error loading images"));
    match send(cx, "GET", url, RequestBody::None, false) {
        Ok((true, data)) => data,
        Ok((false, data)) => match prop(&data, "error") {
            Ok(error) => error_json(or_text(
                error,
                cx.t,
                "image_load_failed",
                "Error loading images",
            )),
            Err(_) => failed(cx.t),
        },
        Err(_) => failed(cx.t),
    }
}

/// POST whose non-ok reply throws `data.error || <fallback>`; rejections propagate.
fn post_or_throw(
    cx: &mut PickerCx<'_>,
    url: &str,
    body: RequestBody,
    key: &str,
    fallback: &str,
) -> Outcome {
    let (ok, data) =
        send(cx, "POST", url.to_owned(), body, false).map_err(RequestFailure::thrown)?;
    if !ok {
        let error = prop(&data, "error")?;
        return Err(js_error(&or_text(error, cx.t, key, fallback)));
    }
    Ok(data)
}

/// The shared `search(query)` of the DuckDuckGo-backed web and Pinterest
/// sources (`use_note`: only the web source shows the server's `note`).
fn ddg_search(
    cx: &mut PickerCx<'_>,
    results: &mut Vec<Value>,
    endpoint: &str,
    query: &JsArg,
    use_note: bool,
) -> Outcome {
    if !query.truthy() {
        return Ok(error_json(text(
            cx.t,
            "search_keywords_required",
            "Please enter search keywords",
        )));
    }
    let url = format!(
        "{endpoint}?q={}",
        encode_uri_component(&query.to_js_string())
    );
    let failed = |t| error_json(text(t, "search_failed", "Search failed. Please try again."));
    let data = match send(cx, "GET", url, RequestBody::None, false) {
        Ok((_, data)) => data,
        Err(_) => {
            results.clear();
            return Ok(failed(cx.t));
        }
    };
    let (Ok(error), Ok(images), Ok(note)) = (
        prop(&data, "error"),
        prop(&data, "images"),
        prop(&data, "note"),
    ) else {
        results.clear();
        return Ok(failed(cx.t));
    };
    if truthy(&error) {
        results.clear();
        return Ok(error_json(error));
    }
    *results = images.as_array().cloned().unwrap_or_default();
    let message = if results.is_empty() {
        if use_note && truthy(&note) {
            note
        } else {
            Value::String(text(cx.t, "no_images_found", "No images found"))
        }
    } else {
        Value::String(image_picker_search_result_message(
            cx.t,
            &JsArg::Number(results.len() as f64),
        ))
    };
    Ok(serde_json::json!({ "images": results, "message": message }))
}

impl GoogleImageSource {
    /// The `try` body of `loadMore` (`Err` = the JS `catch`).
    fn apply_google_page(&mut self, cx: &mut PickerCx<'_>, url: String) -> Result<Value, ()> {
        let (_, data) = send(cx, "GET", url, RequestBody::None, false).map_err(|_| ())?;
        let error = prop(&data, "error").map_err(|_| ())?;
        if truthy(&error) {
            return Ok(error_json(error));
        }
        // `[...this.results, ...data.images]` throws unless `images` is iterable.
        let images = prop(&data, "images").map_err(|_| ())?;
        let images = images.as_array().ok_or(())?;
        self.results.extend(images.iter().cloned());
        self.next_start = prop(&data, "next_start").map_err(|_| ())?;
        let total = prop(&data, "total").map_err(|_| ())?;
        let message = if self.results.is_empty() {
            text(cx.t, "no_images_found", "No images found")
        } else {
            let count = if truthy(&total) {
                JsArg::from_json(&total)
            } else {
                JsArg::Number(self.results.len() as f64)
            };
            image_picker_search_result_message(cx.t, &count)
        };
        Ok(serde_json::json!({ "images": self.results, "total": total, "message": message }))
    }
}

impl FolderImageSource {
    /// Newest first (stable, like `Array#sort`), then show the first page.
    fn sort_and_show(&mut self) {
        self.all_images
            .sort_by_key(|img| std::cmp::Reverse(img.last_modified));
        // `filter` never fails; its summary is not needed here.
        let _ = self.filter("", 0, 10);
    }
}

/// Lowercased allow-list (`DEFAULT_IMAGE_EXTENSIONS` when none is given).
fn allowed_extensions(allowed: Option<&[String]>) -> Vec<String> {
    match allowed {
        Some(list) => list.iter().map(|e| e.to_lowercase()).collect(),
        None => DEFAULT_IMAGE_EXTENSIONS
            .iter()
            .map(|e| (*e).to_owned())
            .collect(),
    }
}

/// `allowed.some(ext => name.toLowerCase().endsWith(ext))`.
fn has_allowed_extension(name: &str, allowed: &[String]) -> bool {
    let lower = name.to_lowercase();
    allowed.iter().any(|ext| lower.ends_with(ext.as_str()))
}

/// A local/AI-ref tile (no dimension badge).
fn local_grid_item(image: &Value, selected_path: Option<&str>) -> LocalGridItem {
    let path = image.get("path").cloned().unwrap_or(Value::Null);
    let path_str = path.as_str();
    LocalGridItem {
        preview_url: format!("/images/preview/{}", encode_path(path_str)),
        selected: selected_path.is_some() && selected_path == path_str,
        path: js_text(&path),
        name: js_text(image.get("name").unwrap_or(&Value::Null)),
        dimensions: None,
    }
}
