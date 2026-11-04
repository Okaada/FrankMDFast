# Test port: notes filesystem domain (`frankmd-core` notes parts + `frankmd-fs`)

Status: every test compiles (`cargo test -p frankmd-core -p frankmd-fs --no-run`) and passes clippy, with
only the expected `todo` warnings left. The business logic is still `todo!()`, so the tests panic at
runtime. They are the acceptance criteria for the implementation phase.

## Original tests → Rust tests

| Original | Rust | Orig | Ported | Skipped (reason) |
|---|---|---|---|---|
| test/services/path_safety_test.rb | fs/tests/path_safety_test.rs | 11 | 11 | – |
| test/services/notes_service_test.rb | fs/tests/notes_service_test.rs | 63 | 49 (50 `#[test]`, one split) | 14 `search_content` tests, plus the search half of "list_tree and search skip symlinked directories": these belong to the frankmd-search port |
| test/models/note_test.rb | fs/tests/note_test.rs | 35 | 33 | `Note.search` (frankmd-search); `to_param` (Rails routing) |
| test/models/folder_test.rb | fs/tests/folder_test.rs | 34 | 33 | `to_param` (Rails routing) |
| test/services/hugo_service_test.rb | core/tests/hugo_service_test.rs (28 pure), fs/tests/hugo_template_test.rs (3 that need a `.hugo_template.md` on disk) | 28 | 28 | – |
| test/services/backlinks_service_test.rb | fs/tests/backlinks_service_test.rs | 13 | 13 | – |
| test/services/recent_files_service_test.rb | fs/tests/recent_files_service_test.rs | 7 | 7 (1 `#[ignore]`) | The concurrent-records test is ported but ignored: it needs the real `frankmd_config::ConfigStore` lock, and the in-test fake store does no locking. The "theme still read" check in the malformed-history test was dropped (config's job). |
| test/helpers/recent_files_helper_test.rb | core/tests/recent_files_helper_test.rs | 3 | 3 | – |
| test/helpers/tree_helper_test.rb | core/tests/tree_helper_test.rs | 11 (C) | 11 (data part) | Markup-only assertions (CSS classes, tabindex, Stimulus `data-action` strings) |
| test/helpers/application_helper_test.rb | – | 1 | 0 | `app_version == FrankMD::VERSION` is Rails view plumbing; the app version belongs to frankmd-app |
| test/controllers/notes_controller_test.rb | fs/tests/notes_controller_test.rs | 67 | 47 (incl. 4 turbo tests rewritten as tree-data checks) | 4 search tests (frankmd-search); 16 HTML/SPA-shell tests (C/P) |
| test/controllers/folders_controller_test.rb | fs/tests/folders_controller_test.rs | 15 | 15 (turbo tests rewritten as tree data plus the expanded-set remap) | – |
| test/controllers/recent_files_controller_test.rb | fs/tests/recent_files_controller_test.rs | 7 | 4 | "index always answers HTML" (P); 2 Home-panel rendering tests (C); the `PATCH /config recent_files → 422` half belongs to frankmd-config |
| test/javascript/tree_utils.test.js | core/tests/tree_utils_test.rs | 7 | 7 | `null`/`undefined` input and the extra `size` property have no typed equivalent; asserted as an empty slice and ignored |
| test/javascript/lib/new_note_utils.test.js | core/tests/new_note_utils_test.rs | 29 | 29 | – |
| test/javascript/url_utils.test.js | – | 16 | 0 | `encodePath` is URL routing for the web client, which the native app doesn't have. `extractYouTubeId` belongs to the frankmd-markdown/media embed port |

The controller tests run through `fs/tests/common/http.rs`. That file is a test-only, in-process
stand-in for the Rails controllers: routing (including Rails stripping a trailing `.ext` from folder
paths), percent-decoding, the 403 config-file guard, lock files → 404, status codes, and English
bodies via the i18n catalog. Exceptions Rails didn't rescue become 500s. All of that Rails plumbing
stays in the test, and the shim only calls the real API.

Two controller tests deviate from the original:
- **Race-create** calls `model::note_save(CreateOnly)` directly, where Rails stubbed `exists?`.
- **History failure → 503** uses a store whose write fails, where Rails stubbed `record`.

## Golden fixtures wired

All goldens use `assert_cases`.

| Fixture | Rust file | Cases wired |
|---|---|---|
| path_safety.json (889) | core/golden_path_lexical.rs | lexical `contain` 469, `within?` 9, `normalize_path` 32, Note attrs 32, Folder attrs 32 |
| | fs/golden_path_safety.rs | `contain` on real trees 49, `safe_path` 49, `exists?`/`file?`/`directory?` 49 each |
| | fs/golden_http.rs | http 21 |
| notes_tree.json (51) | fs/golden_notes_tree.rs | `list_tree` + `tree_after` 32, `Folder#children` 8, `Note.all` 1, `Folder.find` 7 |
| | fs/golden_http.rs | http 3 (compared as the `data-path` sequence) |
| notes_crud.json (121) | fs/golden_notes_crud.rs | 96 service/model op scenarios |
| | fs/golden_http.rs | 25 with http ops |
| notes_rename.json (68) | fs/golden_notes_rename.rs | rename 38, Folder#rename 12, Note#rename 5, rename+delete 1 |
| | fs/golden_http.rs | http 8 |
| | core/golden_tree_view_remap.rs | expanded remap 4 |
| hugo.json (400) | core/golden_hugo.rs | slugify 265/267, render_template 17, update_frontmatter_slug 59/60, generate_blog_post 38, constants 1 |
| | fs/golden_hugo_template.rs | template_source 13 |
| backlinks.json (59) | fs/golden_backlinks.rs | 55 |
| | fs/golden_http.rs | http 4 |
| recent_files.json (24) | fs/golden_recent_files.rs | 21 (the 3 containing http ops are skipped) |
| recent_files_helper.json (235) | core/golden_recent_files_helper.rs | opened_ago 217 (7 locales), opened_at 18 |
| tree_helper.json (25) | core/golden_tree_helper.rs | 25 |
| js_tree_utils.json (11) | core/golden_js_tree_utils.rs | 11 |
| js_new_note_utils.json (192) | core/golden_js_new_note_utils.rs | 192 |
| js_codemirror_wikilink.json (28) | core/golden_js_codemirror_wikilink.rs | completions 26 |

Cases not wired:
- **hugo.json, 3 cases** expect a Ruby `NoMethodError` for a `nil` or Integer argument, which a `&str` parameter can't express.
- **hugo.json `notes_root`, 4 cases**: NOTES_PATH and Rails-root resolution belong to frankmd-config/app.
- **js_codemirror_wikilink.json, 2 autocompletion-config cases** are CodeMirror settings. The test only asserts that they exist.

How the goldens compare results:
- **Tree order ties:** the original doesn't order siblings whose sort keys are equal. The tests sort such runs by path on both sides.
- **Backlinks:** results are sorted by `(path, line_number)`, as the fixture notes say.
- **http `tree_after`:** `.fed` and `.frankmd-config.lock` are dropped on both sides. Creating them is a frankmd-config side effect.
- **Recents in `.fed`:** compared through the parsed `recent_files` value. The test-local `FedRecentStore` stands in for ConfigStore, and the `.fed` line surgery itself is frankmd-config's contract.
- **Ruby `Errno::*` errors:** only the class is compared, because Ruby's message text can't be reproduced.

## API shaped for these tests

### frankmd-core

**`path`**
- New functions: `lexical_contain`, `is_within`, `is_config_file`, `classify`/`PathKind`.
- New constants: `CONFIG_FILE`, `NOTE_EXT`.
- New `RelPath` methods: `is_root`, `join`, `Display`.

**New modules**
- **`note`**: model attributes and validation, with `ValidationError` whose `Display` is the Rails full message.
- **`tree_view`**: `TreeRow`/`TreeView`/`tree_rows`, plus `parse_expanded`/`remap_expanded`.
- **`new_note`**: the New Note dialog preview (`preview_slugify`, `hugo_date_path`, `is_valid_note_name`, `can_create_note`, `preview_note_path`).

**`tree`**
- `TreeNode::File` gained `file_type: FileType` (`markdown`/`config`).
- The serde form is exactly the Rails hash.
- New: `flatten_files`.

**`slug`**: new `ACCENT_MAP` constant.

**`hugo`**
- New: `TEMPLATE_FILE`, `DEFAULT_TEMPLATE`, `format_date`, `date_path`, `render_template`.
- `NewPost.parent` is now `Option<&str>`.
- `GeneratedPost.path` is now a `String`, so it can keep the `//` quirk.

**`wikilink`**: new `BacklinkQuery`, `Backlink`/`ContextLine`, `scan_note`, and the `[[` completion API (`WikiFile`, `complete`, which uses UTF-8 byte offsets).

**`recent`**
- New constants: `LIMIT`, `MAX_INPUT`, `MAX_PATH_BYTES`.
- New: `normalize`, `remap_raw`, `to_json` (keys in Rails order), `is_valid_path`, `format_opened_at`, `parse_opened_at`/`OpenedAt`, `opened_ago`/`OpenedAgo::i18n`.

**New dependency**: `serde_json`.

### frankmd-fs

**`FsError`**
- `Display` is now the Rails message verbatim.
- New: `compat_class()`, `is_invalid_path()`.

**`Vault`**
- New: `real_root`, `filesystem_lock_path`, `safe_path(raw, must_exist)`, `folder_children`, `write_note` (upsert).

**New modules**
- **`path_safety::contain`**: the full port, symlink-aware.
- **`service::NotesService`**: the raw-string compat facade, with `with_recents`, `rename_folder` (Hugo slug), `hugo_template`, `create_hugo_post`, `backlinks`.
- **`model`**: the Note/Folder operations and the Rails rescue tables. `ModelError::full_message` uses an i18n translator.
- **`recent`**: the `RecentStore` trait (raw JSON text of `recent_files`) and `RecentFilesService`.
- **`fault`**: thread-local fault injection behind the `fault-injection` feature, which a self dev-dependency enables for tests. It covers what the Rails suite mocha-stubbed: ENOSPC on the atomic rename, `link(2)` unsupported, EXDEV, lstat ENOENT, EROFS lock, and stat EACCES.

**New dependencies**: `serde_json`, `jiff`.

## Cross-crate requirements

- **frankmd-config** must implement `frankmd_fs::RecentStore` for `ConfigStore`:
  - `load` returns the raw `recent_files` value.
  - `modify` is a locked read-modify-write that writes `recent_files = <json>` (replace in place, else append).
  - `is_present` is true when `.fed` is a regular file.
- **frankmd-config** must also resolve `hugo_path_style` (`.fed` > ENV), with an empty value treated as unset. Callers pass the result to `HugoPathStyle::from_config`.
- **frankmd-i18n**: `Locale::from_code` is still `todo!()`. The core goldens map locale codes themselves.
- **frankmd-search** owns the `search_content` tests in notes_service, note and notes_controller.

## Behavior decisions and quirks the tests pin

These follow the original unless noted.

**Hugo**
- Dates use the zone the caller passes. The fixtures cover UTC and non-UTC zones, e.g. `-03:00` for America/Sao_Paulo. Rails effectively always used UTC, so the workspace must decide which zone to pass. Pass UTC to stay byte-compatible.
- A whitespace-only parent counts as no parent. Parents like `"a/"` and `"../escape"` are joined verbatim, e.g. `a//2026/...`.
- An empty slug gives the path `YYYY/MM/DD//index.md`, which is cleaned on create.
- An explicit `path_style: ""` gives dated and does not fall back to config.
- ENV values are not trimmed or case-folded: `" flat "` and `FLAT` both give dated.

**Paths**
- **Possible bug in the original:** the Rails `..` substring rule (`notes..v2.md` is invalid in `Note#save`/`Folder#create` validation, but legal at the service level) is kept for the model ops, because the fixtures pin it. `RelPath` itself accepts such names.
- A NUL byte in a path made Ruby raise `ArgumentError`. The Rust tests accept `None` from `contain` and `InvalidPath` from `safe_path` and the predicates.

**Recents**
- `opened_at` keeps 6 fractional digits plus `Z`. The fixtures' times come from `travel_to`, which truncates microseconds.
- `parse_opened_at` must copy Ruby's lenient `Time.iso8601`:
  - Feb 30 rolls over to Mar 2, and `24:00` and `:60` roll over too.
  - A leading space and lowercase `t`/`z` are accepted.
- A stat EACCES counts as "present", so a permission error doesn't drop entries.
- A `RECENT_FILES =` key in uppercase is matched case-insensitively. The fixture's fake store handles this; ConfigStore must too.
- Non-string paths in the fixtures (e.g. `42`) are passed as `None`.

**Tree and links**
- Rails omitted `data-mtime` when the mtime was absent. `TreeNode` can't tell "absent" from 0, so the test treats 0 as absent.
- Wikilink completions only offer paths ending in lowercase `.md`, in provider order. Ranking by boost is the UI's job.
- Backlinks follow symlinks and don't scrub invalid UTF-8. Rails raised in that case, and the Rust error kind for it is still open: the test only requires some error.

**Test helpers**
- The atomic-write "temp file in the target's own directory" test is only a partial port. `fault` can't yet report the path of the rename source; a `fault::last_path` hook would make it a full check.
