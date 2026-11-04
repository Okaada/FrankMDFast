# Port: media client side (search, YouTube, picker sources, small JS helpers)

Crate: `crates/frankmd-media`. Modules: `search.rs`, `youtube.rs`, `picker.rs`,
`s3_key.rs`, `url_utils.rs` and `clipboard.rs`. Every body is `todo!()`. The tests
are the acceptance criteria and fail at runtime until those bodies are implemented.

## Original tests → Rust tests

| Original | Rust | Orig. | Ported | Skipped (reason) |
|---|---|---:|---:|---|
| test/controllers/youtube_controller_test.rb | tests/youtube_controller_test.rs | 10 | 9 | 1 P: the HTML partial's `data-action` attribute check |
| test/controllers/images_controller_test.rb (web-search part only) | tests/image_search_test.rs | 9 | 9 | none. The other 31 tests are in `port-media-uploads.md`. |
| test/javascript/image_sources/image_picker_text.test.js | tests/image_sources_image_picker_text.rs | 3 | 3 | – |
| test/javascript/image_sources/drop_images.test.js | tests/image_sources_drop_images.rs | 6 | 6 | – |
| test/javascript/image_sources/web_images.test.js | tests/image_sources_web_images.rs | 24 | 19 | 5 C: DOM `error` event, HTML escaping, attribute injection, 2 `deselectAll` |
| test/javascript/image_sources/pinterest_images.test.js | tests/image_sources_pinterest_images.rs | 15 | 11 | 4 C: same kinds as above |
| test/javascript/image_sources/google_images.test.js | tests/image_sources_google_images.rs | 21 | 17 | 4 C: same kinds as above |
| test/javascript/image_sources/local_images.test.js | tests/image_sources_local_images.rs | 24 | 20 | 4 C: escaping, injection, 2 `deselectAll` |
| test/javascript/image_sources/ai_images.test.js | tests/image_sources_ai_images.rs | 34 | 32 | 1: ref-search debounce timer, which belongs to the view. 1 C: escaping. |
| test/javascript/image_sources/folder_images.test.js | tests/image_sources_folder_images.rs | 32 | 25 | 3 blob-URL bookkeeping (`createObjectURL`/`revokeObjectURL`). 4 C: escaping, injection, 2 `deselectAll`. |
| test/javascript/lib/s3_key.test.js | tests/s3_key_test.rs | 3 | 3 | – |
| test/javascript/url_utils.test.js | tests/url_utils_test.rs | 16 | 16 | – |
| test/javascript/lib/clipboard_image.test.js | tests/clipboard_image_test.rs | 6 | 6 | – |
| **Total** | | **203** | **176** | **27** |

How the C `renderGrid` tests were ported: the GPUI view renders text, so HTML output
has no Rust counterpart. Instead the tests check the data the HTML was built from:
- `external_grid_items` for the web, Google and Pinterest sources,
- `LocalImageSource::grid_items`,
- `AiImageSource::ref_grid_items`,
- `FolderImageSource::{grid_items, status_text}`.

The fields checked are url, thumbnail, title, source, dimension label, the selection
flag, the encoded preview URL, the size in KB, and `is_safe_image_url` for the image
`src`. Tests that only make sense for HTML (escaping, `onerror` attribute injection,
DOM event removal, CSS classes in `deselectAll`) are skipped.

## Golden fixtures

| Fixture | Rust | Cases wired |
|---|---|---:|
| js_s3_key.json | tests/golden_js_s3_key.rs | 20/20 |
| js_url_utils.json | tests/golden_js_url_utils.rs | 95/95 |
| js_image_sources.json | tests/golden_js_image_sources.rs | 70/70 (52 helper cases + 18 scenarios) |

Scenario harness (`tests/common/client_picker.rs`):
- `FakeTransport` serves canned replies in FIFO order. `{json, ok, status}` maps to
  `Reply`, `{reject}` to `NetworkError`, `{abort}` to `Aborted`.
- Requests are recorded in the fixture encoding:
  `{method, url, responseKind, body | {$formData}, signal}`.
- The three `window.t` modes are reproduced. `none` is `None`. `identity` returns the
  key. `pt` returns `"[pt] key {json}"`, omitting the JSON part when the options are
  empty.

For each step the harness compares `result`, `requests` and the watched `state`.

Not compared:
- `rendered.innerHTML` (DOM). Only the folder source's `rendered.status` is compared.
- The JS-only `file` and `objectUrl` fields of folder images.

## API shape decisions

- **`picker.rs`.** Each source is a synchronous state machine over an injected
  `Transport` (`PickerRequest` → `PickerResponse`).
  - Endpoint payloads stay `serde_json::Value`, because the JS passed them through
    untouched. Results are `Result<Value, Thrown>`, where `Thrown` mirrors a JS throw
    and JS `undefined` is `Value::Null`.
  - `AiImageSource::generated_data` must always contain all five keys
    (`data, url, mime_type, model, revised_prompt`), using `null` for undefined.
  - `JsArg` keeps JS's `undefined`/`null`/number/string distinction, so
    `Number("abc") → NaN`, `String(1.5)` and the `%{}` interpolation behave as in JS.
  - The folder picker is behind a `DirectoryPicker` trait (`None` means unsupported).
    Decoded dimensions come from `PickedFile::dimensions`.
- **`search.rs`** (`ImageSearch` + `SearchEndpoints`) and **`youtube.rs`**
  (`YoutubeSearch::with_base`) use `frankmd_net::TrustedClient`.
  - Request-level failures become `SearchError` with the i18n key:
    `QueryRequired` (400), `NotConfigured` (503), `Failed` (500).
  - Upstream failures are still `Ok`, with `error` or `note` set on the results.
    Examples: Google's `quotaExceeded` message, DDG's "temporarily unavailable" note,
    and "YouTube API error".
  - Result URLs are only listed here. They are fetched later through the egress-guarded
    import, as in the original.
- **`s3_key.rs`** takes a `jiff::civil::Date` (the local date). There are also `*_now`
  variants that take a `Clock`.
- **`url_utils.rs`** takes `Option<&str>`, where `None` stands for `null`/`undefined`.
- **`clipboard.rs`**: `ClipboardItem` and `PastedFile` stand in for `DataTransferItem`
  and `File`.

## Recorded HTTP data (`tests/data/`)

These are small hand-recorded bodies:
- the DDG vqd page and the no-token page,
- `i.js` (web and pinterest),
- a Google CSE response,
- a YouTube search response.

## Quirks kept / flagged

- The Pinterest search is DDG with `"<q> site:pinterest.com"` and has no `note`
  handling. The web search does handle `note`.
- `imagePickerSearchResultMessage("abc")` → "Found NaN images" (golden).
  `Number(null)` → 0 → plural form.
- Google `loadMore` keeps the `next_start` the server sent, including `null` or
  undefined. `reset()` sets it to `0`.
- Every picker `saveCopy` / `upload` error falls back to the original's messages,
  including "Failed to upload to S3", even when the provider is local.
- One shared-file change: `serde_json` was added as a regular dependency of
  frankmd-media, because `picker.rs` payloads are JSON values.
