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
        todo!()
    }

    /// JS `Number(x)` (`null` → 0, `"abc"` → NaN, `undefined` → NaN).
    pub fn to_number(&self) -> f64 {
        todo!()
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
pub fn image_picker_text(t: Option<&dyn Translator>, key: &str, options: &TextOptions, fallback: Option<&str>) -> String {
    todo!()
}

/// `imagePickerSearchResultMessage(count)`: `search_result_one` iff `Number(count) === 1`,
/// else `search_result_many` with `count: Number(count)`.
pub fn image_picker_search_result_message(t: Option<&dyn Translator>, count: &JsArg) -> String {
    todo!()
}

/// `DEFAULT_IMAGE_EXTENSIONS`.
pub const DEFAULT_IMAGE_EXTENSIONS: [&str; 6] = [".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"];

/// `extOf(filename)`: lowercased trailing `\.[^.]+$`, else `no_file_extension` ("no extension").
pub fn ext_of(t: Option<&dyn Translator>, filename: &str) -> String {
    todo!()
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
        todo!()
    }
    /// `search(query)` → `{error}` | `{images, message}` (message: `note` or
    /// "No images found" when empty, else the result-count message).
    pub fn search(&mut self, cx: &mut PickerCx<'_>, query: &JsArg) -> Outcome {
        todo!()
    }
    /// `saveCopy(url, resize, s3Prefix = "")` → POST `/images/import {url, resize[, s3_prefix]}`;
    /// non-ok throws `data.error || "Failed to upload to S3"`.
    pub fn save_copy(&self, cx: &mut PickerCx<'_>, url: &Value, resize: &Value, s3_prefix: Option<&str>) -> Outcome {
        todo!()
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
        todo!()
    }
    /// `search(query)`.
    pub fn search(&mut self, cx: &mut PickerCx<'_>, query: &JsArg) -> Outcome {
        todo!()
    }
}

/// `GoogleImageSource` (`/images/search_google?q=&start=`, infinite scroll).
#[derive(Debug, Clone, Default)]
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

impl GoogleImageSource {
    /// `reset()` (`nextStart = 0`).
    pub fn reset(&mut self) {
        todo!()
    }
    /// `search(query)`: blank → `{error}`; else reset to `nextStart = 1` and `loadMore()`.
    pub fn search(&mut self, cx: &mut PickerCx<'_>, query: &JsArg) -> Outcome {
        todo!()
    }
    /// `loadMore()`: while `loading` returns `{images: results}` without a request.
    pub fn load_more(&mut self, cx: &mut PickerCx<'_>) -> Outcome {
        todo!()
    }
    /// `shouldLoadMore(scrollBottom)`: `< 100 && !loading && results non-empty`.
    pub fn should_load_more(&self, scroll_bottom: f64) -> bool {
        todo!()
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
        todo!()
    }
    /// `load(search = "", {limit = 10, offset = 0})` → GET `/images?[search=]&limit=&offset=`
    /// (each value `encodeURIComponent(String(v))`); non-ok → `{error: data.error || "Error loading images"}`.
    pub fn load(&mut self, cx: &mut PickerCx<'_>, search: &JsArg, limit: &JsArg, offset: &JsArg) -> Outcome {
        todo!()
    }
    /// `deleteImage(path)` → DELETE `/images/file/<encodePath>`; non-ok throws.
    pub fn delete_image(&self, cx: &mut PickerCx<'_>, path: &str) -> Outcome {
        todo!()
    }
    /// `saveCopy(path, resize, s3Key = "")` → POST `/images/import {path, resize[, s3_key]}`.
    pub fn save_copy(&self, cx: &mut PickerCx<'_>, path: &Value, resize: &Value, s3_key: Option<&str>) -> Outcome {
        todo!()
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
        todo!()
    }
    /// `loadConfig()` → GET `/ai/image_config`; failures keep the previous values.
    pub fn load_config(&mut self, cx: &mut PickerCx<'_>) -> Outcome {
        todo!()
    }
    /// `generate(prompt)`: blank → `ai_prompt_required`; disabled → `ai_not_configured`;
    /// POST `/ai/generate_image {prompt[, reference_image_path]}` (abortable) →
    /// `{success, imageData, previewSrc}` | `{error}` | `{cancelled: true}`.
    pub fn generate(&mut self, cx: &mut PickerCx<'_>, prompt: &JsArg) -> Outcome {
        todo!()
    }
    /// `abort()`: cancel an in-flight `generate` (no-op when none). The app's
    /// transport resolves the cancelled request as [`PickerResponse::Aborted`].
    pub fn abort(&mut self) {
        todo!()
    }
    /// An `abortController` is live (a `generate` is in flight).
    pub fn is_generating(&self) -> bool {
        todo!()
    }
    /// `setRefImage(path, name)`.
    pub fn set_ref_image(&mut self, path: &str, name: &str) {
        todo!()
    }
    /// `clearRefImage()`.
    pub fn clear_ref_image(&mut self) {
        todo!()
    }
    /// `loadRefImages(search = "")` → GET `/images[?search=]`.
    pub fn load_ref_images(&self, cx: &mut PickerCx<'_>, search: &JsArg) -> Outcome {
        todo!()
    }
    /// `save(s3Prefix = "")`: no data → `{error}`; base64 data → POST
    /// `/images/upload_base64 {data, mime_type, filename: "ai_<now_ms>.png"[, s3_prefix]}`,
    /// URL → POST `/images/import {url[, s3_prefix]}`; non-ok throws.
    pub fn save(&self, cx: &mut PickerCx<'_>, s3_prefix: Option<&str>, now_ms: i64) -> Outcome {
        todo!()
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
        todo!()
    }
    /// `browse(allowed)`: `picker` `None` → `folder_api_not_supported`; abort →
    /// `{cancelled}`; error → `folder_access_failed`; else `{count}` (files whose
    /// lowercased name ends with an allowed ext; dirs skipped).
    pub fn browse(&mut self, t: Option<&dyn Translator>, picker: Option<&mut dyn DirectoryPicker>, allowed: Option<&[String]>) -> Outcome {
        todo!()
    }
    /// `ingest(files, allowed)` → `{count, rejected: [{name, ext}]}` (sort by
    /// `lastModified` desc, then `filter("")`).
    pub fn ingest(&mut self, t: Option<&dyn Translator>, files: Vec<PickedFile>, allowed: Option<&[String]>) -> Outcome {
        todo!()
    }
    /// `filter(term, {offset = 0, limit = 10})` → `{displayed, total, offset, limit}`
    /// (case-insensitive trimmed substring).
    pub fn filter(&mut self, term: &str, offset: usize, limit: usize) -> Outcome {
        todo!()
    }
    /// `getImage(index)`.
    pub fn get_image(&self, index: usize) -> Option<&DisplayedImage> {
        todo!()
    }
    /// The status line `renderGrid` wrote: none displayed → "No images found";
    /// `total_count > shown` → `folder_showing_recent`; else `folder_images_found_one|many`.
    pub fn status_text(&self, t: Option<&dyn Translator>, total_count: Option<usize>) -> String {
        todo!()
    }
    /// `upload(file, resize, s3Key = "")` → POST `/images/upload` FormData
    /// `file[, resize][, s3_key]` (falsy values omitted); non-ok throws `"Upload failed"`.
    pub fn upload(&self, cx: &mut PickerCx<'_>, file: &PickedFile, resize: &JsArg, s3_key: Option<&str>) -> Outcome {
        todo!()
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
    todo!()
}

/// `width`x`height` label when both are finite and > 0 (`Number(x)` semantics:
/// `"0"`, `"a"`, missing → no label).
pub fn dimensions_label(width: &Value, height: &Value) -> Option<String> {
    todo!()
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
    todo!()
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
        todo!()
    }
}

impl AiImageSource {
    /// `renderRefImageGrid(images)` data.
    pub fn ref_grid_items(&self, images: &[Value]) -> Vec<LocalGridItem> {
        todo!()
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
        todo!()
    }
}
