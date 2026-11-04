# Port: media uploads (frankmd-media: upload_storage, images, video, import, recovery, api)

Scope: the upload/import/recovery half of `frankmd-media`. The library/usage/S3-library
half and the client-side/search/YouTube half have their own `port-*.md`.

## Test files

| Original | Rust | Original tests | Ported | Skipped / delegated |
|---|---|---:|---:|---|
| test/services/upload_storage_test.rb | crates/frankmd-media/tests/upload_storage_test.rs | 27 | 26 | 1: `with_temp_copy` (no temp copies in the Rust pipeline; bytes go from memory to a no-clobber destination) |
| test/services/images_service_test.rb (`ImagesServiceTest`) | tests/images_service_test.rs | 48 | 46 | 2: `imagemagick_cmd` IM6/IM7 binary selection (no ImageMagick; `image` crate) |
| test/services/images_service_test.rb (`ImagesServiceS3Test`) | tests/images_service_s3_test.rs | 21 | 21 | – |
| test/services/media_service_test.rb | tests/media_service_test.rs | 9 | 9 | – |
| test/controllers/image_import_test.rb | tests/image_import_test.rs | 12 textual / 14 expanded | 14 | – |
| test/controllers/images_controller_test.rb | tests/images_controller_test.rs | 40 | 31 | 9 delegated to media-client: the 4 blank-query/no-key `search_*` tests and the 5 `SearchActionsTest` tests (left as `// port: … — media-client` markers) |
| test/controllers/media_controller_test.rb | tests/media_controller_test.rs | 3 | 3 | – |
| test/controllers/media_storage_recovery_test.rb | tests/media_storage_recovery_test.rs | 11 textual / 22 expanded | 22 | – (HTML data-attribute / alertdialog assertions of the page-load test are UI, kept as state assertions) |
| golden `media.json` | tests/golden_media.rs | 519 cases | 470 cases (25 fns) | 49 cases owned by media-library (`MediaLibraryService#list/#delete`, `ImageLibraryService#list`); `every_case_is_wired` guards new fns |

No category-P tests in scope (CSRF lives in upload_csrf_test.rb, not assigned).
Shared helpers: `tests/common/uploads.rs`. All tests compile; they fail at runtime on `todo!()`
by design (one guard test, `every_case_is_wired`, passes).

## API shaped (all bodies `todo!()`)

- `param::Param` — loosely-typed request params (nil/bool/int/float/str/list) so Ruby
  coercions (`Integer()`, `to_s`, `ActiveModel::Type::Boolean.cast`) can be reproduced.
- `env::{MediaEnv, SettingsStore, MemorySettings, UploadLimits}` — injected settings
  (fresh `current()` + fallible `set_provider_local()`), pinned ctx, `TrustedClient`,
  `EgressClient`, **`s3_transport` test seam** (S3 requests go to a wiremock base path-style
  while public URLs keep the original AWS/endpoint formatting), overridable size caps.
- `upload_storage` — `provider`, `s3_configured`, `invalid_s3_configuration_error(&S3Error)`,
  `validate_filename(cfg, name, UploadKind)`, `enforce_bytes`, `enforce_base64_size`,
  `max_base64_encoded_bytes`, `sanitize_filename`, `dest_filename`, `sanitize_s3_key`,
  `unique_s3_key`, `s3_key`, `s3_region`, `s3_url`, `Upload`, constants.
- `images` — server folder (`enabled`, `list`, `count`, `find_image`, `delete`,
  `clamp_limit/offset`, `image_dimensions`), `resize_arg`, `resize_and_compress` (pure, on
  bytes), `content_type_for`, `extension_for_content_type` (`None` = Ruby NoMethodError),
  `parse_resize_ratio`, async `upload_file`, `upload_base64_data`, `save_to_notes_directory`,
  `upload_to_s3`, `download_and_upload_to_s3`, `fetch_external_image`.
- `video::save_upload`, `VIDEO_MIME_TYPES`; `import::{ImportRequest, import_image}`.
- `recovery::StorageRecovery` — the Rails session flag as in-memory state:
  `recover_media_storage(consume)`, `recover_missing_upload_storage`, `recover_s3_upload`.
- `api::MediaApi` — endpoint facade: `storage_status` (GET /config & page loads, consumes the
  warning), `images_config` (keeps it), `index`, `preview`, `destroy`, `upload`,
  `upload_base64`, `import`, `upload_to_s3`, `upload_external_to_s3`, `media_upload`.
  Errors are `ApiError { status: BadRequest|NotFound|Unprocessable|ServiceUnavailable,
  message: Key(i18n key) | Text(service message), media_storage_fallback }`.
- Shared additions: `s3::S3Client::with_transport_base`, lib re-exports, new modules
  `api`, `env`, `param`. `MediaError` unchanged (messages carry the original strings).

## Decisions / quirks

- **Result shapes.** Ruby `{url}`/`{error}`/`nil`/raise collapse to `Result<String, MediaError>`:
  rejections → `Rejected(msg)`, validation → `Invalid(msg)` (e.g. "Invalid base64 data:
  invalid base64"), missing → `NotFound`, S3 collision on images → `Upstream("Image upload
  failed")` (Ruby returned nil, and the controller fell into its generic error), video
  collision → `Upstream("Video upload failed")`, other S3 errors propagate as `S3(_)` so
  recovery can classify them. Legacy `upload_to_s3`/`download_and_upload_to_s3` "returns nil"
  tests assert `is_err()` plus "nothing requested/uploaded".
- **Clocks.** `dest_filename` uses the clock's zone (Ruby: process local TZ; golden case
  America/Sao_Paulo); `s3_key` uses UTC (Ruby `Time.current` in the default Rails zone).
- **Magic bytes vs. goldens.** architecture.md §3.5 adds a magic-byte check to `store_media`,
  but the original never sniffed content and the goldens upload `"img"` / `"fake video
  bytes"` successfully. The service functions here (`images::upload_file`,
  `upload_base64_data`, `video::save_upload`) keep the original behaviour; if the magic-byte
  check is applied on these paths, the goldens/tests need a documented adaptation.
- **External URLs use `http://`** in tests (wiremock is plain HTTP), resolved by
  `StaticResolver` and dialled via `EgressClient::with_dial_override`. The Content-Length cap
  test sends a real 25 MiB + 1 body; the deadline test uses a 100 ms `Limits.timeout` against a
  delayed response instead of stubbing the monotonic clock.
- **NetworkingError** (temporary failure row) = S3 transport pointed at a closed port.
- `images_controller_test` "upload_to_s3/upload_external_to_s3 returns error when S3 not
  configured": the original stubbed `feature_available?(:s3_upload)=true`, so recovery did not
  run; the port makes recovery's write fail (`fail_writes`) to reach the same
  `errors.s3_not_configured` branch.
- `get_image_dimensions` / `resize_and_compress` assert concrete values (1×1, `source.jpg`)
  where the original tolerated a missing ImageMagick; the "IM missing" fallback test is adapted
  to "undecodable source → original bytes, `image/png`, original name".
- Golden `tree_after` is compared for `images/` + `videos/` only; `.fed` (template) and
  `.frankmd-config.lock` are `Config.new` side effects owned by frankmd-config.
- Original-bug flags (replicated, not fixed): `enforce_bytes!` accepts negative sizes;
  `provider` treats `legacy_s3: ""` as a conflict with both providers (Rails cast → nil);
  `extension_for_content_type("")` raised NoMethodError (→ `None`); `upload_to_s3` /
  `upload_external_to_s3` require `images_path` (404) even though they shouldn't
  ([QUIRK-DROP] candidate, backend.md §7.1).

## Cross-crate requirements

- **frankmd-config:** `ResolvedConfig::new(Option<&str>, EnvSnapshot)` and
  `EnvSnapshot::from_pairs` (used by goldens); `MediaConfig::from_resolved` needs typed reads
  of `images_path` (expanded), `media_storage_provider`, `upload_extensions(...)` (COMPAT
  parsing), all `aws_*` (blank → None), `youtube_api_key`, `google_*`. Production
  `SettingsStore` = `ConfigStore` wrapper whose `set_provider_local` returns false on a failed
  write.
- **frankmd-fs:** no-clobber create of new media files under `<notes>/images|videos` (creating
  the dir), and contained delete/read of files under the *external* `images_path` (symlink-aware
  containment for a root outside the vault).
- **frankmd-net:** `EgressClient::with_resolver`, `with_dial_override`, `Limits::EXTERNAL_IMAGE`,
  `fetch` honouring Content-Length + streamed cap, per-hop policy, relative `Location`, deadline;
  `TrustedClient::new` with a finite default timeout.
- **i18n keys used by `ApiMessage::Key`:** `errors.no_file_upload`, `errors.no_image_data`,
  `errors.url_required`, `errors.s3_not_configured`, `errors.images_not_configured`,
  `errors.upload_failed`, `errors.failed_to_upload`, `media_storage.retry_local`.
