# Test port: frankmd-media library, usage and S3 library

This port covers `src/library.rs`, `src/usage.rs` and `src/s3_library.rs`. The tests compile and clippy shows only `todo` warnings. They fail at runtime on `todo!()` until the code is implemented.

| Original | Rust | Original tests | Ported | Skipped |
|---|---|---:|---:|---|
| `test/services/media_library_service_test.rb` | `tests/media_library_service_test.rs` | 9 | 9 | none |
| `test/services/media_usage_service_test.rb` | `tests/media_usage_service_test.rs` | 12 | 12 | none |
| `test/services/s3_media_library_service_test.rb` | `tests/s3_media_library_service_test.rs` | 5 | 5 | none |
| `test/controllers/library_controller_test.rb` | `tests/library_controller_test.rs` | 13 | 13 | none |
| `test/controllers/image_library_picker_test.rb` | `tests/image_library_picker_test.rs` | 8 | 8 | none (see note below) |
| golden `media_usage.json` | `tests/golden_media_usage.rs` | 233 cases | 233 | none |
| golden `media.json` (library functions only) | `tests/golden_media_library.rs` | 49 cases | 49 | none |

For the picker tests, the turbo-stream markup is Rails plumbing. The tests check the `PickerPage` data instead: the raw name, the percent-encoded preview URL, `total` and `collection`.

The 49 `media.json` cases are `MediaLibraryService#list` (4), `MediaLibraryService#delete` (33) and `ImageLibraryService#list` (12).

The test helpers are in `tests/common/library.rs`:
- They build golden tree specs, including `{content, mtime}` entries, `{symlink}` entries (with `<OUTSIDE>`) and `{base64}` entries.
- They provide `chmod` guards for the EACCES cases.
- They build S3 `ListObjectsV2` XML responses.
- They create a `MediaEnv` whose `s3_transport` points at wiremock.

## API

Local library:
- `MediaLibrary`: `new`, `from_config`, `list`, `safe_media_file`, `delete`. Items are `MediaItem` values inside a `LibraryListing`.

Picker:
- `picker_list` takes a `PickerQuery` and returns a `PickerPage` of `PickerImage { .., item }` values.
- `PickerQuery` holds raw `Param`s, coerced with Ruby `to_i` rules.
- The server collection reuses `images::list` and `images::count` from the media-uploads port.

Controller logic:
- `Library` routes by storage provider. It has `list`, `delete`, `rename` and `usage`.
- `LibraryError` variants carry their i18n keys: `library.media_not_found`, `delete_failed`, `rename_failed` and `usage_scan_failed`.

Usage index:
- `MediaUsage`: `new`, `build_index`, `build_count`.
- `build_count` is a test seam. It replaces the original's `expects(:build_index_from).once`.
- Free functions: `references_in`, `markdown_without_code`, `normalize_reference_label`, `canonical_media_path`, `strip_unescaped_url_suffix`, `usage_counts`.
- The regex sources are constants in their original Onigmo syntax. They use look-behind, so the implementation needs `fancy-regex`.

S3 library:
- `S3MediaLibrary`: `new(env, config)`, `list`, `delete`, `rename`.

## Decisions and quirks

- **EACCES:**
  - The original made `File.delete` and `Pathname#read` raise through stubs.
  - The Rust tests use `chmod` instead: 0o555 on the parent directory, or 0o000 on the file.
  - These tests skip themselves when run as root.
- **mtime:**
  - The fixtures recorded mtimes in the process time zone, which was UTC.
  - The Rust port prints the local zone.
  - So the golden tests compare mtimes as instants, not as strings.
- **Invalid UTF-8 (deliberate divergence):**
  - In Ruby, `canonical_media_path("images/%FF.png")` produced a string that is not valid UTF-8.
  - Rust keys are `String`, so the test expects the lossy decode.
- **`.frankmd-config.lock` is ignored:** it shows up in the recorded `tree_after` only because Rails' `Config.new` created it.
- **Cache scope:**
  - The usage cache lives in each `MediaUsage` instance, and the app keeps one per vault.
  - The original used a process-wide LRU cache of 8 roots.
- **S3 rename:**
  - Validation follows the original's sanitizing rule (`SANITIZE_RE`).
  - It will reuse helpers from `upload_storage`.

## What this needs from other crates

- From `frankmd_fs::Vault`: deleting a contained regular file when no part of its path is a symlink, and passing EACCES back to the caller.
- From `frankmd_config::ResolvedConfig`, via `MediaConfig::from_resolved`: the upload extensions and `images_path`.
