# FrankMD test-contract map

Source: `~/Projects/FrankMD` @ `d47a89e92d89f807878d318182fa5dcea0329ec4` (Rails 8 + Stimulus/CodeMirror).
Every test file under `test/` was read in full. For each file this document lists the behavioural
contracts its tests assert, written so the Rust desktop rewrite can re-derive them.

## Categories

| Tag | Meaning | Rust strategy |
|---|---|---|
| **A** | Pure core logic: config, path safety, notes tree/CRUD/rename, search, Hugo, backlinks, recent files, media usage, egress policy, themes, translations, pure JS libs. Filesystem logic in temp dirs counts as A. | Rust unit tests, mostly driven by `tests/fixtures/golden/*.json` |
| **B** | Integration with external services: HTTP, S3, AI providers, YouTube, image-search APIs, `fetch` to endpoints. | Integration tests with mocked HTTP/S3 |
| **C** | UI/system: Stimulus/DOM controllers, CodeMirror EditorView behaviour, Capybara system tests, keyboard and focus. | Re-expressed as UI/integration tests in the new app |
| **P** | Extra flag, not a category: the test is only about Rails/web plumbing that doesn't matter for a desktop rewrite (auth token/login throttling, CSRF, turbo-stream responses, HTML data-attribute bootstrapping, `/up` health, `evaluate_script` lint, log-tail HTTP params). A P test still has an A/B/C tag. | Drop, or keep only the domain part |

## Totals

| Part | Scope | Files | Tests | A | B | C | P |
|---|---|---:|---:|---:|---:|---:|---:|
| 1 | Ruby `test/models`, `test/services`, `test/helpers`, `test/i18n_*` | 28 | 855 | 800 | 43 | 12 | 1 |
| 2 | Ruby `test/controllers`, `test/system` | 29 | 385 | 239 | 37 | 109 | 47 |
| 3 | JS top-level `*_utils`, `test/javascript/lib`, `image_sources` | 47 | 872 | 648 | 55 | 169 | 0 |
| 4 | JS controllers `ai_grammar` … `file_operations` | 25 | 699 | 77 | 9 | 613 | 0 |
| 5 | JS controllers `find_replace` … `video_dialog` | 26 | 647 | 128 | 4 | 515 | 0 |
| **Total** | | **155** | **3458** | **1892 (55%)** | **148 (4%)** | **1418 (41%)** | **48** |

How the tests were counted:
- **Ruby:** one per `test "..."`. Tests generated in loops are counted per row they produce. `i18n_media_upload_keys_test.rb` is 7 locales × 37 keys = 259 tests and `i18n_vim_keys_test.rb` is 7 × 18 = 126, which is why part 1 is mostly A. Loops in `image_import`, `media_storage_recovery` and `upload_csrf` are expanded too, so a plain grep gives 361 instead of 385 for part 2.
- **JS:** one per `it(`/`test(`.
  - Parts 3 and 5 expand `it.each` tables per row. Part 3 also counts the per-theme tests in `theme_syntax_palette`.
  - Part 4 counts each `it.each` table as one test and notes its row count in the file header.
  - For comparison, vitest itself reports 2238 tests across 98 files. The JS parts here total 2218.
- Infrastructure files are described but not counted: `test_helper.rb`, `application_system_test_case.rb`, `test/javascript/setup.js`, `helpers/` and `mocks/`.

### Rails-plumbing tests (P, irrelevant to the desktop rewrite)

48 tests in total.
- **Most of them:** `test/controllers/auth_test.rb` (token auth, login redirect, Bearer/X-Auth-Token, throttling at 10 failures then 429, fail-closed on short tokens) and `upload_csrf_test.rb` (CSRF on the upload endpoints).
- **Turbo-stream variants** of create/destroy/rename in `notes_controller_test.rb` and `folders_controller_test.rb`.
- **HTML bootstrapping** of `data-app-initial-*` values in `notes_controller_test.rb`.
- **Smaller items:**
  - `health_controller_test.rb` (`/up`)
  - the `logs_controller_test.rb` HTTP params
  - `GET /config/editor` partial
  - the `video_dialog` `data-action` attribute check
  - `test/helpers/evaluate_script_convention_test.rb` (a lint over the system tests)

The domain behaviour around these tests is still listed: `.fed` protection, revision conflicts, and so on.

### Where the golden fixtures come from

Part 2 of this work executed the original code for most of the A contracts and saved the results as
`tests/fixtures/golden/*.json` (see `tools/golden/README.md`). The JS unit-test inputs were harvested
automatically by re-running FrankMD's vitest suite with recording wrappers.

---

<!-- part totals: files=28 tests=855 A=800 B=43 C=12 P=1 -->

## Part 1 — Ruby: top-level, models, services, helpers

(files = 28 test files with tests; `test/test_helper.rb` and `test/application_system_test_case.rb` are infrastructure with 0 tests and are described, not counted.)

### `test/application_system_test_case.rb` — infrastructure (0 tests)
- Capybara + Selenium headless Chrome at 1400×900 (`--no-sandbox` if `CHROME_NO_SANDBOX=1`), `parallelize(workers: 1)`, `Capybara.default_max_wait_time = 5`.
- setup: saves `FRANKMD_AUTH_TOKEN` and sets it to `""` (turns auth off), then calls `setup_test_notes_dir`. teardown restores both.
- Helpers: `wait_for_tree_refresh(path)` polls `performance.getEntriesByType("resource")` until a request to `/notes/tree?selected=<path>` shows up (5s timeout). `editor_content` gets the `codemirror` Stimulus controller from `[data-controller~="codemirror"]` and returns `ctrl.getValue()`.

### `test/i18n_media_upload_keys_test.rb` — subject: `config/locales/*.yml` (259 tests: A=259 B=0 C=0; P=0)
- [A] Generated: 7 locales `en es he ja ko pt-BR pt-PT` × 37 keys = 259 tests (`"<locale> defines <key>"`). Each key must be `present?` (non-blank) in `config/locales/<locale>.yml` under the `<locale>` root. Keys: `media_storage.{label,not_configured,recovery_title,recovered,retry_local,local,s3,save_failed,load_failed,save_copy,collection,managed_images,server_images}`, `library.{rename,rename_prompt,rename_failed,renamed}`, `dialogs.image_picker.{tab_drop,drop_hint,drop_rejected,processing_image_paste,per_page,prev_page,next_page,page_status,delete_image,delete_confirm,delete_failed}`, `dialogs.s3_option.{custom_key,custom_key_hint,custom_prefix,custom_prefix_hint}`, `dialogs.video.{tab_drop,drop_hint,drop_rejected,uploading}`, `status.upload_failed`. Why: the client-side `window.t` gives back the raw key when a key is missing, and the translations endpoint does not merge in a fallback.

### `test/i18n_vim_keys_test.rb` — subject: `config/locales/*.yml` (126 tests: A=126 B=0 C=0; P=0)
- [A] Generated: the same 7 locales × 18 keys = 126 tests. Keys: `settings.vim`, `dialogs.help.tab_vim`, `dialogs.help.vim.{intro,modes_title,mode_normal,mode_insert,mode_visual,commands_title,col_command,col_action,cmd_write,cmd_quit,cmd_wq,cmd_edit,cmd_explore,cmd_next,cmd_prev,cmd_help}`.

### `test/test_helper.rb` — infrastructure (0 tests)
- Sets `RAILS_ENV=test` and `FRANKMD_LOCALE=en` (default only). Starts SimpleCov with branch coverage. Loads mocha and webmock, then calls `WebMock.allow_net_connect!`, so real network is allowed by default. Forces `I18n.default_locale = I18n.locale = :en`, runs tests in parallel (one worker per processor), and resets `I18n.locale` in teardown.
- `setup_test_notes_dir`: creates `Rails.root/tmp/test_notes_<hex16>` and sets `ENV["NOTES_PATH"]` to it, keeping the old value. `teardown_test_notes_dir` does `rm_rf` and restores the env var. `create_test_note(path, content="# Test\n\nContent")` runs mkdir -p on the parent and writes the file. `create_test_folder(path)` runs mkdir -p. All paths are relative to the temp NOTES_PATH. Many services default their base path to `ENV["NOTES_PATH"]`.

### `test/helpers/application_helper_test.rb` — subject: `app/helpers/application_helper.rb` (1 tests: A=1 B=0 C=0; P=0)
- [A] `app_version` returns `FrankMD::VERSION`.

### `test/helpers/evaluate_script_convention_test.rb` — subject: `test/system/*.rb` (lint) (1 tests: A=0 B=0 C=1; P=1)
- [C][P] Scans the system tests. A body inside `evaluate_script(<<~X)` whose first non-empty line starts with `const|let|var|if|for|while|return|switch|try` is reported as an offender, because Selenium wraps the script as `return (SCRIPT);`. Only applies to the Capybara test suite.

### `test/helpers/recent_files_helper_test.rb` — subject: `app/helpers/recent_files_helper.rb` (3 tests: A=3 B=0 C=0; P=0)
- [A] `recent_file_opened_ago(t)` uses a single unit with exact boundaries (seconds ago → label): 0 "Just now", 1 "1 second ago", 59 "59 seconds ago", 60 and 119 "1 minute ago", 600 "10 minutes ago", 3599 "59 minutes ago", 3600 "1 hour ago", 7200 "2 hours ago", 86399 "23 hours ago", 86400 "1 day ago", 172800 "2 days ago", 604800 "7 days ago", 604801 "Over a week ago". A future time gives "Just now".
- [A] Output follows the locale. In pt-BR: 1 day "há 1 dia", 2 days "há 2 dias", 8 days "Há mais de uma semana".
- [A] `recent_file_opened_at(iso)` parses with the offset kept: `"2026-10-26T09:30:00-03:00"` == 12:30 UTC, `utc_offset == -10800`. `"not a timestamp"` and `""` give nil.

### `test/helpers/tree_helper_test.rb` — subject: `app/helpers/tree_helper.rb` (11 tests: A=0 B=0 C=11; P=0)
- [C] An empty item list renders the `sidebar.no_notes_yet` text (class `text-sm`). An empty folder at depth > 0 does not render that message.
- [C] File item: `data-path`, `data-type="file"`, `data-file-type="markdown"`, a keydown rename action, and the name text. Selected file gets the `selected` class. Hash items with string keys are accepted.
- [C] Folders are focusable (`tabindex="0"`) and have focus, delete and rename keyboard actions, plus `click->app#toggleFolder`, `dragover`/`drop` drag-drop actions and the `tree-folder` class. Children sit in `tree-children hidden` unless the path is in `expanded_folders`; then the chevron is `tree-chevron expanded` and there is no hidden class.
- [C] Config file (`.fed`, `file_type: "config"`) is selectable (click selectFile and the delete/rename keydown actions are present) but is NOT `draggable="true"` and has no `contextmenu`. Regular files have `draggable="true"`, `contextmenu->app#showContextMenu` and `dragstart->drag-drop#onDragStart`.

### `test/models/config_test.rb` — subject: `app/models/config.rb` (67 tests: A=67 B=0 C=0; P=0)
Setup: NOTES_PATH = temp dir. Clears these env vars: `OPENAI_API_KEY OPENROUTER_API_KEY ANTHROPIC_API_KEY GEMINI_API_KEY OLLAMA_API_BASE AI_PROVIDER AI_MODEL OPENAI_MODEL OPENROUTER_MODEL ANTHROPIC_MODEL GEMINI_MODEL OLLAMA_MODEL AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY AWS_S3_BUCKET YOUTUBE_API_KEY GOOGLE_API_KEY GOOGLE_CSE_ID`. The config file is `<base>/.fed`.
- [A] media_storage_provider ×3: defaults to `"local"` even when S3 env credentials exist. `set("media_storage_provider","unknown")` returns false. `update({"media_storage_provider"=>"s3","theme"=>"dark"})` with no S3 credentials returns false and persists nothing (atomic: theme stays nil). With S3 credentials, setting `s3` persists and `ui_settings["media_storage_provider"]=="s3"`. The `s3` value is still returned after the credentials disappear, and it can be set back to `local`.
- [A] File creation ×3: `Config.new(base_path:)` creates `.fed` with mode `0600`. It creates missing parent dirs (nested/deeply/path). The template contains `# UI Settings`, `# theme =`, `# editor_font =`, `# AWS S3`, `# YouTube API`, `# Google Custom Search`, `# AI/LLM`, `# ollama_api_base`, `# anthropic_api_key`, `# gemini_api_key`.
- [A] Defaults ×1: editor_font "cascadia-code", editor_font_size 14 (Integer), preview_zoom 100, sidebar_visible true, typewriter_mode false, show_stats false, cursor_recenter_on_click false, vim_mode false, theme nil.
- [A] Schema/UI keys ×8: `vim_mode`, `show_stats`, `cursor_recenter_on_click` and `scroll_sync` are in `UI_KEYS` with `:boolean` type and defaults false/false/false/true. `view_mode` is `:string` with default "split". show_stats can be set and the value survives a reload. The file lines `vim_mode = true`, `cursor_recenter_on_click = true`, `scroll_sync = false` and `view_mode = single` are read back correctly.
- [A] upload_extensions ×3: `upload_extensions("image_upload_extensions")` includes `.png` and `.jpg`. The video list includes `.mp4` and `.webm`. `ENV VIDEO_UPLOAD_EXTENSIONS=" .MP4 , .MOV "` gives `[".mp4",".mov"]` (trimmed, lowercased).
- [A] Parsing ×5: `key = value` lines, with integers cast (`editor_font_size = 16` → 16). Booleans `true`/`false` work. Double and single quotes are stripped (`"tokyo-night"`, `'jetbrains-mono'`). `#` comment lines are ignored, including commented `# editor_font = ...`. Unknown keys are ignored (`get(:unknown_key)` nil).
- [A] ENV fallback ×2: `YOUTUBE_API_KEY` env is used for `youtube_api_key` when the file does not set it. A file value beats the env value.
- [A] Writing ×10: `set` and `update` persist (checked by re-instantiating). Setting `theme` replaces the template's `# theme =` comment line with `theme = nord` (no `# theme = nord` remains). Repeated sets keep a single `^theme = ` line. User ordering is kept, with no comments or blank lines added, when the file already has an AI marker (`# ai_model = placeholder`). Keys the user removed are not re-added (the in-memory values are not dumped). Only the target line changes; comments and other values are kept as they are. A key missing from the file is appended at the end. `update(theme: "light\"\nimages_path = /outside")` is rejected (returns false, file byte-identical) because values containing `"`, `\r` or `\n` are refused. `update` returns true and writes `theme = dark`. A pre-existing 0600 mode is kept after an update, and replaced files are always 0600.
- [A] `ui_settings` returns only UI keys (theme, editor_font), not `youtube_api_key`.
- [A] feature_available? ×3: `:s3_upload` requires `aws_access_key_id` + `aws_secret_access_key` + `aws_s3_bucket`. `:youtube_search` requires `youtube_api_key`. `:google_search` requires `google_api_key` + `google_cse_id`.
- [A] Robustness ×4: an empty file gives defaults. Malformed lines (`this is not valid`, `= no key`, `editor_font_size not an int`) are ignored while valid lines still parse (font size stays 14). Binary garbage `\x00\xFF\xFE\x00theme = dark` must not crash. If `.fed` is a directory there is no crash and defaults are used.
- [A] Casting ×3: `editor_font_size = 20` (no trailing newline) gives Integer 20. Boolean true ⇔ the lowercased value is one of `true 1 yes on` (TRUE, Yes also true). Anything else is false (`false 0 no off FALSE anything`).
- [A] AI ×14: `ai_providers_available` is empty when nothing is configured. With OPENAI + ANTHROPIC env set it includes openai and anthropic, not ollama or gemini. Auto priority is openai > anthropic > openrouter > ollama > gemini (all set → "openai"). An explicit `ai_provider = openai` is honoured when available. An unavailable explicit provider falls back by priority (only OLLAMA set → "ollama"). Default model for anthropic is "claude-sonnet-4-20250514". `ai_model = gpt-4-turbo` overrides globally. `anthropic_model = claude-3-opus-20240229` overrides per provider. `feature_available?("ai")` is true with only GEMINI_API_KEY and false with nothing. `ai_configured_in_file?` is true when a credential (`anthropic_api_key = ...`) is in the file and false when only `ai_provider` is there. **File-only mode**: once any AI credential is in the file, `get_ai` ignores ENV for all AI keys (env OPENAI/OPENROUTER → nil), so `effective_ai_provider == "anthropic"` and `ai_providers_available == ["anthropic"]`. With no AI credential in the file, `get_ai` uses ENV.
- [A] Upgrade ×3: an old config without `# AI/LLM` has the AI section appended on load (contains `ollama_api_base`, `anthropic_api_key`, `gemini_api_key`). Existing values and comments are kept and the mode stays 0600. If an AI section already exists it is not duplicated (exactly one `# AI/LLM`), and `openai_api_key = sk-existing` is still readable. The upgrade is skipped when any AI key appears in the file, commented or not (`/^#?\s*<key>\s*=/i`). Tests that write custom files use `# ai_model = placeholder` to stop the upgrade.

### `test/models/folder_test.rb` — subject: `app/models/folder.rb` (34 tests: A=34 B=0 C=0; P=0)
- [A] `Folder.find(path)` gives path, `name` (last segment) and `parent_path` ("deep/nested"). A missing folder raises `NotesService::NotFoundError`.
- [A] `create`: makes the dir, nested with mkdir -p. An existing dir returns false and adds an error on `:base` ("Folder already exists"). `../escape` is invalid (error on `:path`, "cannot contain directory traversal"). `""` is invalid (presence). If the base is chmod 0555, the error contains "Permission denied".
- [A] `destroy`: removes an empty dir. A non-empty dir returns false with a `:base` error and the dir stays. A missing or externally deleted dir returns false with an error containing "not found" ("Folder not found"). A read-only parent gives "Permission denied".
- [A] `rename(new)`: moves the folder and its contents and updates `folder.path`. A source that disappeared returns false with a `:base` error. **Hugo slug rewrite**: if `<new>/index.md` has front matter with a `slug:` field, the slug is rewritten to `slug: "<slugify(basename new)>"` (e.g. `café-açaí` → `slug: "cafe-acai"`) and the title and body are kept. `index.md` without front matter, or front matter without `slug`, is left byte-identical.
- [A] `exists?`/`persisted?`: true only for a directory; false for a missing path or a file. `name`, `parent_path` (nil at root), `to_param` == path. `children`: an Array of 3 for 2 files + 1 subfolder; `[]` for an empty or nonexistent folder.

### `test/models/note_test.rb` — subject: `app/models/note.rb` (35 tests: A=35 B=0 C=0; P=0)
- [A] `Note.find`: gives path, content, `name` (basename without .md) and `directory`. `find("test")` normalizes to `test.md`. A missing note raises `NotesService::NotFoundError`. `find(".fed")` works and returns the config content.
- [A] `Note.normalize_path` ×4: `"test"`→`"test.md"`, `"folder/file"`→`"folder/file.md"`, `"test.md"` unchanged, `".fed"` unchanged, `""`/nil→`""`.
- [A] `Note.all` is the tree as an array of hashes (`name` "note1", folder with `type: "folder"`). `Note.search("findme")` returns 1 result with `path: "searchable.md"`.
- [A] `save`: creates or overwrites the file with exact content and creates parent dirs. `../escape.md` is invalid (`:path` error). An empty path is invalid. Saving into a 0555 dir returns false with a `:base` error.
- [A] `destroy`: deletes the file. `destroy(expected_revision: sha256_hex(content))` deletes when the revision matches. On a mismatch it returns false, `revision_conflict?` is true and the file is untouched. A missing or externally deleted file returns false with an error containing "not found". A read-only parent gives "Permission denied".
- [A] `rename(new)`: moves the file (including into another dir) and updates `path`. A missing source returns false with a `:base` error.
- [A] Attributes: `exists?`/`persisted?` reflect the file. `name` ("my-note"). `directory` ("folder/subfolder", `""` at root). `to_param` == path. `as_json` == `{path:, name:, content:}`.

### `test/services/ai_service_test.rb` — subject: `app/services/ai_service.rb` (47 tests: A=34 B=13 C=0; P=0)
Two classes: `AiServiceTest` (42) and `AiServiceImageGenerationTest` (5, Config stubbed and returning openrouter key `sk-or-test-key`). The env vars listed under Config plus `IMAGE_GENERATION_MODEL` are cleared.
- [A] `enabled?` ×6: false with nothing set. true with any one of OPENAI_API_KEY, OPENROUTER_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY or OLLAMA_API_BASE.
- [A] `current_provider` ×7: priority openai > anthropic > openrouter > ollama > gemini. Gemini is last because its key is mostly used for Imagen. `AI_PROVIDER=openai` is honoured when available. When it is not available, the priority order is used (→ollama).
- [A] `current_model` ×7: defaults openai "gpt-4o-mini", ollama "llama3.2:latest", anthropic "claude-sonnet-4-20250514", gemini "gemini-2.0-flash", openrouter "openai/gpt-4o-mini". `AI_MODEL` overrides globally. `ANTHROPIC_MODEL` overrides per provider.
- [A] `fix_grammar` guards ×3: with nothing configured → `{error: "AI not configured"}`. `""` or nil → `{error: "No text provided"}`.
- [A] `provider_info` has keys `:enabled :provider :model :available_providers` (openai + anthropic → provider "openai"). `available_providers` lists only the configured providers.
- [A] Image-generation config ×5: `image_generation_enabled?` is true only if OPENROUTER_API_KEY is set. Default `image_generation_model` is "google/gemini-3.1-flash-image-preview". `image_generation_info` == `{enabled:, model:}` (false/true depending on the key).
- [A] `generate_image` guards ×3: not configured → error containing "not configured". `""` or nil prompt → "No prompt provided".
- [A] `fix_grammar` with `OLLAMA_API_BASE=file:///etc/passwd` → "AI provider request failed" (non-http base is rejected).
- [B] `generate_image("A sunset", reference_image_path: "nonexistent.jpg")` with only an env key → `result[:error].present?`. This makes a real OpenRouter call, since net connect is allowed.
- [B] fix_grammar with RubyLLM mocked ×4: success → `{corrected: "This is corrected text.", provider: "openai", model: "gpt-4o-mini"}`. An exception gives the generic `"AI provider request failed"` and does not leak the exception message. The logger never sees an API key fragment (`sk-sensiti`). Anthropic provider → `provider: "anthropic"`.
- [B] `extract_image_from_response(resp, model)` ×3: the first attachment becomes `{data: strict_base64(content), mime_type:, model:}` with no error. No attachments or text only → `"No image data in response"`.
- [B] generate_image with mocked chat ×5: text-only success (`ask(prompt)`) → base64 data, mime, model "google/gemini-3.1-flash-image-preview". An exception gives the generic `"Image generation failed"`. A reference image found under `images_path` is passed as `ask(prompt, with: <path String>)`. A response with no attachment → "No image data in response". A missing reference image falls back to a text-only ask.

### `test/services/backlinks_service_test.rb` — subject: `app/services/backlinks_service.rb` (13 tests: A=13 B=0 C=0; P=0)
- [A] `BacklinksService.new(base_path:).find_backlinks(target, context_lines: n)` returns `[]` when nothing links to the target.
- [A] Wikilinks match by name `[[target]]`, with extension `[[target.md]]`, case-insensitively (`Target.md` ← `[[target]]`) and by full path without .md (`[[projects/myproject]]`). Standard markdown links match `[this link](target.md)` and `[the guide](docs/guide.md)`. `match_text` contains the link text.
- [A] The target file itself is skipped. Results come from multiple files (1 per file) and multiple matches in one file (one result per line, `line_number` [1,2]).
- [A] Result keys are `path, name (basename without .md), line_number (1-based), match_text, context`. With `context_lines: 1` there are 3 entries `{content, is_match, ...}`, and the middle one has `is_match: true`.
- [A] `"../etc/passwd"` and `"folder/../../secret.md"` raise `ArgumentError`.

### `test/services/custom_theme_test.rb` — subject: `app/services/custom_theme.rb` (3 tests: A=3 B=0 C=0; P=0)
- [A] `CustomTheme.parse(json_string_or_hash)` → `{"version"=>1,"name"=>strip(name),"base"=>,"colors"=>downcased}`. For example, name " My colors " → "My colors" and "#AABBCC" → "#aabbcc". `parse("null")` → nil.
- [A] `BASE_THEMES` must equal the basenames of `app/assets/tailwind/themes/*.css`, and `COLOR_KEYS` must equal the `--theme-<key>:` names in `dark.css`. This is a consistency check against the shipped assets (see code for the 18 base and 38 color keys).
- [A] Raises ArgumentError for these colors: "red", "#123", "url(...)", "#aabbcc;display:none", "var(--secret)". Only `#rrggbb` is accepted. It also raises for an unknown color key ("font-family"), base "omarchy", version 2, a name that is empty, 61 chars long or contains a control char (`\n`), an extra key `css`, and a raw string over `MAX_BYTES` (16 KiB).

### `test/services/egress_policy_test.rb` — subject: `app/services/egress_policy.rb` (11 tests: A=11 B=0 C=0; P=0)
- [A] `checked_target(url)` raises `EgressPolicy::BlockedError` for a non-http(s) scheme (message includes "http"), for `http://` with no host, and for a host that resolves to nothing. DNS goes through `EgressPolicy.resolve`, which the tests stub.
- [A] `blocked_ip?` is true for 169.254.169.254, 127.0.0.1, 10.0.0.5, 172.16.0.1, 192.168.1.10, 169.254.1.1, ::1, fc00::1, fe80::1, 0.0.0.0, 100.64.0.1, 192.0.2.1, 198.18.0.1, 203.0.113.1, 240.0.0.1, 224.0.0.1, ff02::1 and ::ffff:127.0.0.1 (IPv4-mapped addresses are judged as IPv4). It is false for 93.184.216.34 and 2606:2800:220:1:248:1893:25c8:1946.
- [A] A host resolving to a private address raises with a message including "non-public". If ANY resolved address is blocked the URL is refused (split horizon). For a public host it returns `[uri, ip_string]` (`uri.host "example.com"`, `uri.path "/image.png"`, ip "93.184.216.34") so the caller can pin the IP.

### `test/services/hugo_service_test.rb` — subject: `app/services/hugo_service.rb` (28 tests: A=28 B=0 C=0; P=0)
- [A] `slugify` ×7: "Hello World"→"hello-world". "hello world"→"hello-world". "café açaí"→"cafe-acai" (ACCENT_MAP). "hello! @world#"→"hello-world". "hello---world"→"hello-world". "--hello-world--"→"hello-world". "Vibe Code: Fiz um Editor de Markdown do zero com Claude Code (FrankMD)"→"vibe-code-fiz-um-editor-de-markdown-do-zero-com-claude-code-frankmd". The algorithm is: downcase, map accents, `[^a-z0-9]+`→"-", trim dashes, collapse dashes.
- [A] `generate_blog_post(title, parent:, path_style:)` ×13 → `{path:, content:}`. The dated style (default) gives `YYYY/MM/DD/<slug>/index.md`, and with a parent `content/posts/YYYY/...`. The flat style gives `<slug>.md` / `content/posts/my-post.md`. The style comes from config `hugo_path_style` when no argument is given (nil → dated). Content starts with `---` and includes `title: "<title>"`, `slug: "<slug>"`, `draft: true` and `tags: []`. It must never contain a bare `-` list item (issue #69). Quotes in the title are escaped: `title: "Say \"Hello\" World"`. The date is the local time: at 2026-02-01 10:30 the path starts with `2026/02/01/` and the content has `date: 2026-02-01T10:30:00` (followed by the zone offset).
- [A] Custom template ×3: `<NOTES_PATH>/.hugo_template.md`, if present, replaces the default. Placeholders are `{{title}}` (quote-escaped), `{{slug}}` and `{{date}}`, and other lines (author, tags, `draft: false`) are kept. Without the file, the default template is used.
- [A] `update_frontmatter_slug(content, new_slug)` ×6 → new content with `slug: "new-slug"` and the body kept. Unquoted and single-quoted old slugs are handled. Returns nil when there is no front matter, no slug field, or the slug is unchanged.

### `test/services/images_service_test.rb` — subject: `app/services/images_service.rb` (69 tests: A=50 B=19 C=0; P=0)
`ImagesServiceTest` (48): Config stubbed with `images_path` = temp dir, provider local, image extensions `.jpg .jpeg .png .gif .webp .bmp`. `ImagesServiceS3Test` (21): provider s3, bucket test-bucket, region us-east-1, `EgressPolicy.resolve` stubbed to a public IP, WebMock blocks net.
- [A] `enabled?` is true iff `images_path` is set.
- [A] `list(search:, limit:, offset:)` ×10: returns `[]` when disabled or the dir is missing. Sorted by mtime, newest first. Only supported image extensions (no .txt/.js). Default limit 10 (`MAX_RESULTS`), limit/offset paging (15 files → 10 + 5), limit clamped to 100, negative offset → 0, garbage strings coerced without raising. Recurses into subdirs (`path` "subfolder/nested.png", `name` basename). `search` is a case-insensitive substring of the name ("cat" → cat.jpg + category.gif).
- [A] `count(search:)` ×4: 0 when disabled or missing. Returns the full total regardless of the limit (15). Respects search (2).
- [A] `delete(rel)` ×5: removes the file → true (subdirs ok). Missing → false. A traversal to a file outside → false and the file stays. A non-image (`secret.txt`) → false and the file stays.
- [A] `find_image(rel)` ×5: returns the full Pathname for an existing file (subdirs ok). Missing → nil. `../../../etc/passwd` → nil. Non-media (`secret.txt`, `.fed`) → nil.
- [A] `upload_base64_data(b64, mime_type:, filename:)` ×6 (local): writes under `<NOTES_PATH>/images/` and returns `{url: "images/<...test_ai...>"}`. With no filename the name contains `ai_generated_`. Invalid base64 → error including "Invalid base64". Encoded input over `UploadStorage.max_base64_encoded_bytes` → "too large", rejected before decoding. An SVG mime (`image/svg+xml`) or a `.html` filename → error including "not an accepted" and no file written.
- [A] `save_to_notes_directory` (private) gives unique URLs for two `photo.png` uploads in the same second.
- [A] `get_image_dimensions(path)` ×3: returns a hash with `:width`/`:height` keys (values when ImageMagick is available). Invalid or nonexistent file → both nil.
- [A] `resize_and_compress(path, name, ratio)` ×2 → `[content, content_type, filename]`. It becomes .jpg / image/jpeg when ImageMagick works, otherwise the original. If `Open3.capture3` raises ENOENT it returns the original bytes, "image/png", "source.png" (no 500).
- [A] `imagemagick_cmd` ×2: with IM7 (`magick` on PATH) → `["magick"]` for convert and `["magick","identify"]`. Otherwise `["convert"]` / `["identify"]`.
- [A] `imagemagick_resize_arg` ×3 (whitelist): 0.25→"25%", 0.5→"50%", 0.75→"75%", 1→"100%", "1.0"→"100%", "0.33"→"33%", "0.67"→"67%". Anything else (0.42, "arbitrary", nil, "50%; rm -rf /", "100%\nmalicious", "`whoami`") → "50%".
- [A] `content_type_for(Pathname)`: jpg/jpeg→image/jpeg, png, gif, webp, svg→image/svg+xml, bmp, unknown→application/octet-stream. `extension_for_content_type`: image/jpeg→.jpg, png, gif, webp, image/svg+xml→.svg, bmp, unknown→.jpg, `"image/png; charset=utf-8"`→.png.
- [A] `s3_enabled?` ×2: true when the access key, secret and bucket are set. false without the access key.
- [B] S3 provider upload ×3: when the provider is s3, both `upload_file` and `upload_base64_data` call `put_object(bucket:"test-bucket", if_none_match:"*")` and return an `https://test-bucket...` URL, never saving locally. With S3 credentials missing → `"S3 is not configured"` for both, with no local fallback. A `put_object` ServiceError is raised to the caller and nothing is written locally.
- [B] `upload_to_s3(rel, resize:)` ×6: nil when S3 is disabled or the image is missing. The URL matches `^https://test-bucket\.s3\.us-east-1\.amazonaws\.com/frankmd/YYYY/MM/<stem>-<16hex>\.<ext>$` and the put uses `if_none_match: "*"`. The key is sanitized (`my photo (1).jpg` → `my_photo__1_-<hex>.jpg`, no spaces). `AccessControlListNotSupported` is tolerated and a URL is still returned. With resize, `resize_s3-<hex>.(jpg|png)`.
- [B] `download_and_upload_to_s3(url)` / `fetch_external_image` ×10: nil when S3 is disabled. nil when the overall deadline `MAX_EXTERNAL_IMAGE_SECONDS` (30) expires (monotonic clock). Downloads and returns `.../frankmd/YYYY/MM/photo-<hex>.jpg`. A URL with no extension takes it from the Content-Type (`.png`). A 404 → nil. Content-Length over `MAX_EXTERNAL_IMAGE_BYTES` (25 MiB) → nil. Redirects are followed and each target is re-validated through EgressPolicy. A redirect to 169.254.169.254 → nil and the metadata host is never requested. A host resolving to a private address → nil, not requested. `file://` → nil.

### `test/services/media_library_service_test.rb` — subject: `app/services/media_library_service.rb` (9 tests: A=9 B=0 C=0; P=0)
Config stubbed: images `.png .jpg`, videos `.mp4 .webm`. `MediaLibraryService.new(base_path:, config:)`.
- [A] `list` → `{images: [...], videos: [...]}`, taken only from `<base>/images/**` and `<base>/videos/**` with supported extensions (svg and txt are excluded). Item keys: `path` ("images/photo.png"), `name`, `type` ("image"/"video"), `preview_url` ("/notes/images/photo.png"), `size` (bytes), `mtime` (String). Symlinked files and symlinked dirs are ignored.
- [A] `delete(rel)` ×7: deletes a supported file under images/ or videos/ (nested ok) → true. Returns false and leaves files intact for `images/../<outside>`, an absolute path, root `notes.png`, `attachments/photo.png`, unsupported `images/readme.md`, NUL bytes (`images/a\0b.png`, `images/\0`; no raise), a symlinked nested dir (`images/external/x.png`) and a symlinked leaf. Filesystem errors such as `Errno::EACCES` are re-raised to the caller and the file stays.

### `test/services/media_service_test.rb` — subject: `app/services/media_service.rb` (9 tests: A=6 B=3 C=0; P=0)
The local class does NOT use setup_test_notes_dir. It writes into the ambient `ENV["NOTES_PATH"]` (or `Rails.root/notes`)/videos and removes the files afterwards. Default video exts: `.mp4 .webm .mkv .mov .avi .m4v .ogv`.
- [A] `save_upload(nil)` → `{error:}`. An allowed video → `{url: "videos/<timestamp>_clip...mp4"}` and the file exists. `notes.txt` → error including ".txt" and "not an accepted video type", url nil. "README" → error "no extension". "my cool clip!.mp4" → url with no space or `!`. "../../../etc/evil.mp4" → flat name directly under `videos/`, contained by cleanpath.
- [B] S3 ×3: the provider decides the S3 upload. `put_object(content_type: "video/webm", if_none_match: "*")` is called, local storage is never used, and the URL is `https://test-bucket.s3.us-east-1.amazonaws.com/frankmd/YYYY/MM/movie-<16hex>.webm`. PreconditionFailed (collision) → generic "Video upload failed" with no provider detail. S3 unconfigured with `upload_to_s3: true` → "S3 is not configured", url nil.

### `test/services/media_usage_service_test.rb` — subject: `app/services/media_usage_service.rb` (12 tests: A=12 B=0 C=0; P=0)
- [A] `MediaUsageService.new(base_path:).build_index` → `{ "images/..."|"videos/..." (decoded, root-relative) => [sorted distinct note paths] }`. Sources counted: md images `![](...)`, links `[](...)` (including image-in-link), HTML `src` on img/video/audio/source/track/embed/object (quoted or unquoted), and reference-style links (full `![x][LABEL]`, collapsed `![video][]`, shortcut `[shortcut]`). Reference labels are matched case-insensitively with whitespace collapsed (`[  PHOTO   IMAGE ]` ↔ `[ photo image ]:`).
- [A] Normalization: percent-decoding (`my%20cat.png`→"my cat.png", `%28draft%29`), note-relative resolution (`../../images/...` from `projects/guides/guide.md`), a leading `/images/...` or `/notes/images/...` maps to root, `#fragment` is stripped, Markdown backslash escapes are removed (`diagram\(draft\)`→"diagram(draft).png", `hash\#name`→"hash#name.png", `question\?name`→"question?name.png").
- [A] Exclusions: unused reference definitions are not counted, and duplicate uses count once per note. Ignored: plain text mentions, external URLs that contain `/notes/images/`, `notimages/`, inline code, fenced code (including refs defined inside fences), HTML comments, paths escaping the notes root (`../../images/outside.png`, `images/\.\./../../outside.png`), `../notes/images/...` (wrong root), hidden notes (`.hidden/`) and symlinked .md files. Only MEDIA_ROOTS `images` and `videos` count.
- [A] A read error raises `MediaUsageService::ScanError` with "Could not read Markdown note", never a partial index. The index is cached: an unchanged tree does not rebuild (`build_index_from` called once for two calls), and editing a note rebuilds it.

### `test/services/notes_service_test.rb` — subject: `app/services/notes_service.rb` (63 tests: A=63 B=0 C=0; P=0)
- [A] `list_tree` ×16: `[]` for an empty dir. Items `{name (no .md), path, type: "file"|"folder", file_type: "markdown", children}`. Folders come before files and are ordered by name, not mtime (alpha, beta even if beta is newer). The code (`notes_service.rb:450`) sorts folders by `downcase` name and FILES by integer mtime, newest first; `notes_tree.json` captures this. Hidden entries (`.hidden.md`) are skipped, and so is `.fed` (also in subfolders). Broken symlinks are skipped without raising. Symlinked dirs are skipped (also by search). A file vanishing during lstat (ENOENT) is tolerated. `.fed.transient.tmp` is never even stat'ed. Root `images/` and `videos/` are hidden unless they contain markdown: root images/ with `notes/guide.md` stays visible, root videos/ with only clip.mp4 is hidden. Only ROOT media folders are hidden: `images-old/`, `archive/images`, `archive/videos` are shown. If the lock file in NOTES_PATH is read-only (EROFS), it falls back to an app tmp lock.
- [A] `read` returns the content. Missing → `NotFoundError`.
- [A] `write`/`create`/`update` ×13: `write` creates or overwrites, creates parent dirs (including Hugo `2026/01/30/my-first-post/index.md` byte-exact), and is atomic: the temp file is created in the destination dir (same filesystem) and renamed, no `.tmp` is left, the existing file mode (0600) is kept, and on rename failure (ENOSPC) the error is raised, the original is untouched and the temp is removed. `create` raises `AlreadyExistsError` without changing an existing file. When hard links are unsupported (`File.link` EOPNOTSUPP) it falls back to an atomic rename with no `.tmp` left. Of two concurrent creates of the same path, exactly one wins and the other gets AlreadyExistsError, and the dir then contains only `[".frankmd-filesystem.lock","shared.md"]`. Instances share one lock file per NOTES_PATH (`shared_filesystem_lock_path` is the same, and flock LOCK_EX|LOCK_NB contends) even with different app tmp roots. `update` overwrites an existing file. A missing file raises NotFoundError and creates no parent dirs.
- [A] `delete` ×4: removes the file. `expected_revision: sha256_hex(content)` must match, otherwise `RevisionConflictError` and the file is kept. Missing → NotFoundError.
- [A] `rename` ×7: file → file and file → other folder (creates the nested dest dir). Folder with contents. On EXDEV it falls back to a FileUtils copy+remove. Concurrent renames to the same destination: one wins, the loser gets AlreadyExistsError and its source survives. Missing source → NotFoundError. An existing destination → `InvalidPathError` and neither file is modified.
- [A] `create_folder` (nested ok). `delete_folder` removes an empty dir and raises InvalidPathError for a non-empty one.
- [A] Security ×3: `../../../etc/passwd` and `folder/../../../etc/passwd` raise InvalidPathError (rejected, not sanitized). `notes..v2.md` is a legal name.
- [A] `search_content(query, context_lines:, max_results:)` ×14: blank/nil → `[]`. Result `{path, line_number (1-based), match_text, context: [{line_number, content, is_match}]}`. Case-insensitive. Regex is supported (`foo\d+bar` → 2 results). An invalid regex (`[brackets`) is escaped and matched literally. Invalid UTF-8 is scrubbed and still matches. Unreadable files (EACCES) are skipped. Files over `MAX_SEARCH_FILE_BYTES` (5 MiB) are skipped. `Regexp::TimeoutError` is skipped (`search_file` → `[]`; timeout 0.5s). With `context_lines: 2` there are 5 lines (1..5). `max_results` caps the results (3 of 5). Searches recursively (`folder/nested.md`).

### `test/services/omarchy_theme_service_test.rb` — subject: `app/services/omarchy_theme_service.rb` (2 tests: A=2 B=0 C=0; P=0)
- [A] Reads `~/.config/omarchy/current/theme.name` and `~/.config/omarchy/current/theme/alacritty.toml` (`[colors.primary]` background/foreground/dim_foreground, `[colors.normal]`, `[colors.bright]`). `theme_data` → `{is_dark:, variables: {"--theme-..."=>hex}}`. For a dark bg #141414: `is_dark` is true and `--theme-syntax-keyword` == normal magenta "#ff00ff". For a light bg #eeeeee: `is_dark` is false. In both cases every `--theme-syntax-{keyword,function,type,string,number,constant,identifier,property,operator,comment,punctuation,invalid}` must reach a WCAG contrast of at least 4.5:1 against `--theme-code-bg`, and the low-contrast input (#202020 / #dddddd) must be adjusted (comment ≠ input).

### `test/services/path_safety_test.rb` — subject: `app/services/path_safety.rb` (11 tests: A=11 B=0 C=0; P=0)
- [A] `PathSafety.contain(base, rel)` → Pathname or nil. `"note.md"`→`/data/notes/note.md`, `"a/b.md"`→nested. Returns nil for `../../etc/passwd`, `/etc/passwd`, the sibling-prefix bypass `/data/notes-backup/secret.md` and `/data/notesX`, nil input, a symlinked leaf pointing outside, a path through a symlinked dir pointing outside (both for an existing and a nonexistent leaf, and no file gets created). Allows `notes..v2.md` and real files inside the base (realpath-based).

### `test/services/recent_files_service_test.rb` — subject: `app/services/recent_files_service.rb` (7 tests: A=7 B=0 C=0; P=0)
- [A] `RecentFilesService.new(config:)`. `record(path)` → true. History is stored in `.fed` (`recent_files`, JSON type) and shared between instances. Bounded to `LIMIT` 10, deduplicated, most recent first (re-recording 4.md moves it to the front). Entries are `{"path", "opened_at" (ISO8601)}`. The `.fed` must not contain note content.
- [A] `entries` filters out notes deleted externally and keeps the rest. `record` of a deleted note raises NotFoundError and does not create the file.
- [A] NotesService mutations update history: a folder rename `docs`→`renamed` rewrites `docs/first.md`→`renamed/first.md` but leaves `docs-other/other.md` alone (no prefix bug). A file rename rewrites the entry. Delete removes it. Expected entries: `["docs-other/other.md","renamed/first.md"]`, then `["docs-other/other.md"]`.
- [A] `record` raises NotFoundError for `../README.md`, `/tmp/secret.md`, `.secret.md`, `docs/../safe.md`, a symlink pointing outside (`outside.md`), `image.png` (non-md) and nil.
- [A] A permission error (stat EACCES) does not drop entries. Concurrent records from 6 Config instances plus `set("theme","nord")` keep all 6 entries and the theme (read-modify-write under a lock). A malformed `recent_files = [broken` gives `entries` empty while `theme = nord` is still read. `clear` → true and the history becomes empty.

### `test/services/s3_media_library_service_test.rb` — subject: `app/services/s3_media_library_service.rb` (5 tests: A=0 B=5 C=0; P=0)
- [B] `list` paginates `list_objects_v2(bucket:, prefix: "frankmd/", continuation_token:)` until not truncated. It keeps only supported extensions under `frankmd/` (`foreign/`, `.txt` dropped). Items have `provider: "s3"` and url `https://bucket.s3.us-east-1.amazonaws.com/frankmd/2026/photo.png`.
- [B] `delete(key)`: refuses without an S3 call for `foreign/photo.png`, `frankmd/../../photo.png` and `frankmd/readme.txt`. An exact managed key → `delete_object(bucket:, key:)` → true.
- [B] `rename(key, new_name)`, in order: `head_object` on the sibling key (NotFound = free), then `copy_object(copy_source: "bucket/<old key>")`, then `delete_object(old)` → `{path: "frankmd/2026/renamed.png", name: "renamed.png"}`. Returns nil with no copy or delete for `../outside.png`, `readme.txt`, a 1100-char name and an existing target.

### `test/services/translation_coverage_test.rb` — subject: `config/locales/*.yml` (2 tests: A=2 B=0 C=0; P=0)
- [A] Every locale in `config.i18n.available_locales` except en must contain every English leaf key path with a non-blank String value.
- [A] The checker reports, in order: `"pt-BR: missing key dialogs.help.find"`, `"pt-BR: blank or non-string value for errors.required (got String)"`, `"ja: blank or non-string value for dialogs.help.find (got FalseClass)"`.

### `test/services/upload_storage_test.rb` — subject: `app/services/upload_storage.rb` (27 tests: A=24 B=3 C=0; P=0)
- [A] `provider(legacy_s3:)`: returns the saved `media_storage_provider`. With local saved, `legacy_s3: "true"` raises RejectedError and `false` is OK. With s3 saved but no credentials (`aws_access_key_id/secret/bucket`) it raises. With credentials → "s3", and `legacy_s3: "false"` raises. An invalid value raises.
- [A] `validate_extension!(file, list_key, kind)` ×4 → normalized ext (`a.PNG`→".png"). A disallowed ext gives a message with ".html" and "not an accepted image type". No ext → "no extension". `evil.png.html` is judged by its last ext (.html).
- [A] `enforce_size!`: over `MAX_UPLOAD_BYTES` (512 MiB) → "too large". Exactly at the cap → nil.
- [A] `dest_filename(name)`: no `/`, space or `!` (SANITIZE_RE `[^a-zA-Z0-9._-]`→`_`), prefix `\A\d{8}_\d{6}_`, keeps `.mp4`. Unique for the same name within the same second.
- [A] `sanitize_s3_key` ×4: "blog/2026/hero" is kept. "../../etc/passwd"→"etc/passwd", "/a//b/"→"a/b", "a/./b"→"a/b", "my folder/a b"→"my_folder/a_b". "../.." and nil → "".
- [A] `s3_key(filename, custom_key:, custom_prefix:)` ×6: the default is `frankmd/YYYY/MM/<stem>-<16hex><ext>`. The filename is sanitized (`../ev il.png` → `.._ev_il-<hex>.png`). A custom_key gets `-<16hex>` before the ext (`blog/2026/hero-<hex>.png`) and wins over the prefix. custom_prefix + safe filename → `blog/2026/photo-<hex>.png`. Traversal in a custom_key → `etc/passwd-<hex>.png`. A custom value that sanitizes to empty ("../..", "///") falls back to the default.
- [A] `s3_url(bucket, region, key)` ×4: the AWS default is `https://mybucket.s3.us-west-2.amazonaws.com/frankmd/2026/09/a%20b.png` (key percent-encoded). A custom endpoint with path style gives `http://localhost:9000/mybucket/frankmd/x.png` (trailing slash on the endpoint is handled). Virtual-host style gives `https://mybucket.s3.example.com/k.png`. A port is kept: `http://b.minio.local:9000/k.png`.
- [A] `with_temp_copy(file, ext) { |temp_path| }`: the temp name is random (not from the client filename, no `..`), ends with ext and is deleted after the block.
- [B] `invalid_s3_configuration_error?`: true for InvalidAccessKeyId, SignatureDoesNotMatch, InvalidToken, ExpiredToken, AccessDenied, NoSuchBucket, AuthorizationHeaderMalformed and `Aws::Errors::InvalidRegionError`. false for SlowDown, InternalError, ServiceUnavailable, PreconditionFailed and Timeout::Error. These are the errors that trigger the "recover to local" UI.
- [B] `s3_client(cfg)` ×2: passes `endpoint` and `force_path_style: true` when configured (MinIO). Plain AWS has no endpoint and no force_path_style key. region defaults to "us-east-1" (DEFAULT_S3_REGION).

## Porting notes (Part 1)
- **`.fed` writer**: it edits the file in place. It replaces the `key = ...` line or a `# key =` commented template line, otherwise appends. It never re-adds keys the user deleted. It rejects values containing `"`, CR or LF, and every update is atomic (all-or-nothing, including validation of `media_storage_provider`). The file is always mode 0600. On load it appends the AI section if no AI key appears, even commented. Keys match `/^([a-z0-9_]+)\s*=\s*(.*)$/i`, and surrounding quotes are stripped. Booleans are true only for `true/1/yes/on`, case-insensitive.
- **AI file-only mode**: one AI credential in `.fed` makes all AI ENV vars invisible. Provider priority is openai > anthropic > openrouter > ollama > gemini. Error strings are generic ("AI provider request failed", "Image generation failed") and must not leak exception text or key fragments.
- **Concurrency**: the shared lock file `.frankmd-filesystem.lock` lives in the NOTES_PATH root and stays there after creates (it is hidden from the tree because it is a dotfile), with a fallback to the app tmp dir when NOTES_PATH is read-only. Create is a no-replace link-or-rename; concurrent create and rename have exactly one winner. Config read-modify-write (recent_files plus settings) is serialized.
- **Revision**: `expected_revision` is the lowercase SHA-256 hex of the file bytes.
- **Tree**: dotfiles, `.fed`, symlinked dirs, broken symlinks and root `images/`/`videos/` with no markdown are hidden. Order is folders first, then by name.
- **Path safety**: paths are contained by realpath (sibling-prefix and symlink escapes are blocked). `..` inside a filename is legal. `..` segments that escape the base raise InvalidPathError rather than being stripped. NUL bytes return not-found instead of raising.
- **Uploads**: local video/image names are `YYYYMMDD_HHMMSS_<sanitized>` and unique within the same second. S3 keys are `frankmd/YYYY/MM/<stem>-<16hex><ext>` with `If-None-Match: *`. SVG and HTML uploads are rejected even though `content_type_for(.svg)` exists. External fetch: 25 MiB cap, 5 redirects, 30 s deadline, and EgressPolicy re-checked on every hop.
- **Test quirks**: MediaService and ImagesService base64 upload tests write into the ambient NOTES_PATH (or `Rails.root/notes`) instead of a temp dir. The `generate_image ... reference_image_path` test in AiServiceTest makes a real OpenRouter request because WebMock allows the network by default. The Omarchy paths are hard-coded to `~/.config/omarchy/current`. Hugo `date:` uses the local zone offset.


---

<!-- part totals: files=29 tests=385 A=239 B=37 C=109 P=47 -->

## Part 2 — Ruby controller & system tests

Counting note: tests generated inside a Ruby `.each do … test "…"` loop are counted once per row they produce (this applies to image_import, media_storage_recovery and upload_csrf). A plain textual grep gives lower numbers for those three files, and the header of each one gives both counts. Controller tests run against a temp `NOTES_PATH` (`setup_test_notes_dir`). `.fed` means `<NOTES_PATH>/.fed`.

### `test/controllers/ai_controller_test.rb` — subject: `app/controllers/ai_controller.rb`, `Config` AI resolution (17 tests: A=17 B=0 C=0; P=0)
Setup clears these env vars: OPENAI_API_KEY OPENROUTER_API_KEY ANTHROPIC_API_KEY GEMINI_API_KEY OLLAMA_API_BASE AI_PROVIDER AI_MODEL OPENAI_MODEL OPENROUTER_MODEL ANTHROPIC_MODEL GEMINI_MODEL OLLAMA_MODEL IMAGE_GENERATION_MODEL.
- [A] `GET /ai/config` with no keys returns `{enabled:false, provider:null, model:null, available_providers:[]}`.
- [A] group "single provider ×5". Each provider is enabled by one env var and gets a default model: OPENAI_API_KEY→`openai`/`gpt-4o-mini` (also listed in `available_providers`); OPENROUTER_API_KEY→`openrouter`/`openai/gpt-4o-mini`; ANTHROPIC_API_KEY→`anthropic`/`claude-sonnet-4-20250514`; GEMINI_API_KEY→`gemini`/`gemini-2.0-flash`; OLLAMA_API_BASE→`ollama`/`llama3.2:latest`.
- [A] Priority: with ollama, openai and anthropic all set, the provider is `openai` and `available_providers` holds exactly those 3. The code's priority order is `openai > anthropic > openrouter > ollama > gemini` (`Config::AI_PROVIDER_PRIORITY`).
- [A] `AI_PROVIDER=openai` override is respected. `AI_MODEL=gpt-4-turbo` overrides the model.
- [A] `POST /ai/fix_grammar` validation ×4: blank path → 400 with error containing "No file path"; missing note → 404 "not found"; empty note → 400 "empty"; note has content but no AI configured → 422 "not configured". All validation runs before any network call.
- [A] `GET /ai/image_config`: `enabled` is false without OPENROUTER_API_KEY and true with it. `model` is `google/gemini-3.1-flash-image-preview` in both cases.
- [A] `POST /ai/generate_image`: blank prompt → 400 "No prompt"; no key → 422 "not configured".

### `test/controllers/auth_test.rb` — subject: `ApplicationController` auth, `SessionsController` (14 tests: A=14 B=0 C=0; P=14)
Optional web auth based on the `.fed` key `auth_token`. All of it is web plumbing; the desktop app most likely drops it, but must still accept and preserve the `auth_token` key in `.fed`.
- [A][P] A blank token gives open access, and the root page carries `id="app-loading"` plus SRI `integrity="sha384-…"`.
- [A][P] With a token configured, HTML requests redirect to `/login`. The login page itself needs no auth. A correct password (= the token) redirects to root; a wrong one → 401. Logout clears the session.
- [A][P] Anonymous JSON → 401 `{"error":"unauthorized"}`. `Authorization: Bearer <token>` authenticates API calls, and so does `X-Auth-Token: <token>`.
- [A][P] Rotating the token revokes existing sessions; the session stores a token digest.
- [A][P] A configured token shorter than 32 bytes fails closed: both GET / and POST /login → 503.
- [A][P] Throttling: 10 failures, then the 11th attempt → 429. A successful login clears the failure count. Throttling is per IP and survives a fresh session; other IPs are not affected.
- [A][P] A valid auth header bypasses CSRF. An invalid or missing header on PATCH → 422 and the note is unchanged.
- [A][P] `/up` health works without a token.

### `test/controllers/config_controller_test.rb` — subject: `ConfigController`, `Config` (16 tests: A=16 B=0 C=0; P=1)
- [A] `GET /config` returns `{settings:{…}, features:{s3_upload, youtube_search, google_search, local_images}, media_storage_fallback}`. `settings` includes theme, editor_font, editor_font_size, preview_zoom, sidebar_visible, typewriter_mode, show_stats and cursor_recenter_on_click.
- [A] Defaults: editor_font `cascadia-code`, editor_font_size `14` (int), preview_zoom `100`, sidebar_visible `true`, typewriter_mode/show_stats/cursor_recenter_on_click `false`.
- [A] Values from `.fed` (`theme = gruvbox`, `editor_font = hack`, `typewriter_mode = true`, …) are parsed with the right types (booleans come back as true).
- [A] media_storage_provider defaults to `local`. PATCH `local` persists it, and it is also exposed in `GET /images/config`. PATCH `unknown` → 422; PATCH `s3` while S3 is unconfigured → 422; the stored value stays `local` in both cases.
- [A] PATCH `{theme:"dark", editor_font_size:18}` returns the updated settings (size is an int) and persists them. Partial updates keep the other keys. `locale: "pt-BR"` persists. Booleans persist.
- [A] Only `Config::UI_KEYS` are permitted: theme custom_theme locale editor_font editor_font_size preview_zoom sidebar_visible view_mode typewriter_mode cursor_recenter_on_click vim_mode scroll_sync editor_indent editor_line_numbers show_stats editor_width media_storage_provider. A non-UI key (`aws_access_key_id`) is dropped; if nothing valid remains → 422 with an error containing "No valid settings", and nothing is written to `.fed`. Empty params give the same 422.
- [A] File format: after PATCH, `.fed` contains the lines `editor_font = hack` and `editor_font_size = 20`. Existing lines such as `theme = tokyo-night` are preserved when new keys are added.
- [A][P] `GET /config/editor` returns an HTML partial `#editor-config` with `data-controller="editor-config"` and data values for font, font-size, editor-width, preview-zoom and theme.
- [A] The partial reflects `.fed` values (hack/20/gruvbox). With no `.fed`, its defaults are font `cascadia-code`, size `14`, **editor_width `72`**, zoom `100`.

### `test/controllers/custom_theme_config_test.rb` — subject: `ConfigController`, `CustomTheme`, `Config` (3 tests: A=3 B=0 C=0; P=0)
- [A] PATCH `{theme:"custom", custom_theme:<JSON {version:1,name:'My "theme"',base:"dark",colors:{accent:"#AABBCC"}}>}` saves both values atomically. The response settings have theme `custom`, and colors are normalized to lowercase (`#aabbcc`). The name with quotes round-trips through `.fed`, and GET returns `base` `dark`. The root page then has `[data-controller="theme"][data-theme-custom-value]` and `dialog.custom-theme-editor`.
- [A] An invalid colour (`url(https://example.com)`) → 422. Nothing is partially applied: theme stays `nord` and custom_theme stays nil.
- [A] A malformed on-disk `custom_theme = {broken` line is read as nil while the other keys still parse (theme nord, editor_font_size 18 as an int). `Config#set("editor_font", 'unsafe"font')` returns false, so values containing quotes are rejected for that key.

### `test/controllers/folders_controller_test.rb` — subject: `FoldersController`, `NotesService` (15 tests: A=15 B=0 C=0; P=3)
- [A] `POST /folders/<path>` creates the folder → 201. It creates nested `parent/child` too. If the folder already exists → 422.
- [A] DELETE removes an empty folder → 200. A non-empty folder → 422 and stays on disk. A missing folder → 404.
- [A] `POST /folders/<path>/rename` with `new_path` moves the folder and its contents (`old_name`→`new_name`, `source`→`target/source`). A missing source → 404.
- [A][P] Turbo-stream variants of create, destroy and rename: `<turbo-stream action="update" target="file-tree-content">` containing the re-rendered tree (`data-path`, `data-type="folder"`). A deleted or renamed-away path is absent.
- [A] Create with `expanded=parent` renders that folder expanded (`class="tree-chevron expanded"`).
- [A] On rename, the expanded set is remapped. Renaming `project`→`app` with `expanded=project,project/src,other` renders `app`, `app/src` and `other` expanded. Unrelated entries (`beta`) are kept and the old path disappears.

### `test/controllers/health_controller_test.rb` — subject: `HealthController` (2 tests: A=2 B=0 C=0; P=2)
- [A][P] `GET /up` → 200 with body `OK` and header `Access-Control-Allow-Origin: *`.

### `test/controllers/image_import_test.rb` — subject: `ImagesController#import`, `ImagesService`, `UploadStorage`, `EgressPolicy` (14 tests [12 textual; 2 tests ×2 content types]: A=4 B=10 C=0; P=0)
`POST /images/import` takes exactly one of `url` (external) or `path` (relative to `images_path`, the "server folder"). The response is `{url}`: either `images/<file>` relative to the notes root (local provider) or `https://bucket…` (S3 provider).
- [B] An external URL is downloaded into `<notes>/images/…` with its bytes intact. This works even when `images_path` is empty.
- [B] With the S3 provider, both external and server sources upload with `put_object(bucket, key starting "chosen/" when s3_prefix=chosen, if_none_match:"*")`, the URL starts with `https://bucket`, and no local `images/` dir is created.
- [B] ×2 (image/bmp, image/x-ms-bmp): with `image_upload_extensions = .bmp`, a BMP is stored locally with extension `.bmp` and its bytes intact.
- [B] ×2: BMP to S3 is uploaded with `content_type image/bmp` and a key ending `.bmp`; no local dir is created.
- [A] A server image is copied locally and the original is untouched.
- [A] Exactly one string source is required. `{}`, both url and path, or url given as an array → 422.
- [A] Server imports reject `../outside.png`, a symlink pointing outside, and non-images (`script.html`) → 422; no `images/` dir is created.
- [B] External content-type `text/html` or `image/svg+xml` → 422. Content-Length above `MAX_EXTERNAL_IMAGE_BYTES` (25 MB) → 422.
- [B] Redirects are followed (302 to a relative Location). If DNS resolves to a private IP (127.0.0.1), the egress policy rejects it → 422.
- [B] An S3 ServiceError → 422 and the provider's error detail is not leaked. Missing credentials → 422. Neither case creates local media.
- [A] The legacy `/images/upload_to_s3` and `/images/upload_external_to_s3` → 422 while the active provider is local, even if S3 creds exist.

### `test/controllers/image_library_picker_test.rb` — subject: `LibraryController#picker`, `ImageLibraryService` (8 tests: A=8 B=0 C=0; P=0)
`GET /library/picker` (turbo-stream) has two collections: `managed` (default: `<notes>/images`) and `server` (`images_path`).
- [A] By default it lists only managed images. It renders `turbo-stream action=replace target=image-picker-library-results`. Names are HTML-escaped (`images/a &lt;photo&gt;.png`) and the src is percent-encoded (`/notes/images/a%20%3Cphoto%3E.png`). Server images and videos are excluded.
- [A] `collection=server` uses `/images/preview/server%20photo.png` and `data-collection="server"`, and the absolute filesystem path never appears.
- [A] Search is case-insensitive (`PHOTO` matches photo0..2 but not other.png), giving `data-total="3"`. limit=2/offset=0 → 2 items; offset=2 → 1.
- [A] Managed images stay browsable when `media_storage_provider = s3` but S3 creds are missing (`data-url="images/photo.png"`).
- [A] A missing or empty server folder → 200 with the i18n text `dialogs.image_picker.local_path_required`.
- [A] Symlinks and unsupported files (`.svg`) are omitted.
- [A] An unknown collection (`s3`) → 400. EACCES → 422 without leaking the path.

### `test/controllers/images_controller_test.rb` — subject: `ImagesController`, `ImagesService` (40 tests: A=30 B=10 C=0; P=0)
- [A] `GET /images/config` returns booleans `enabled, s3_enabled, web_search_enabled, google_enabled, pinterest_enabled`, plus `image_extensions` (includes `.png`) and `video_extensions` (includes `.mp4`).
- [A] `search_web`, `search_google` and `search_pinterest` with blank `q` → 400 "required". search_google without GOOGLE_API_KEY/GOOGLE_CSE_ID → 503 "not configured".
- [A] `POST /images/upload` without a file → 422 "No file".
- With images configured (images_path = temp dir, local provider, extensions `.jpg .jpeg .png .gif .webp .bmp`):
  - [A] config `enabled:true`.
  - [A] `GET /images` returns `{images:[…], total, limit, offset}`; the defaults are limit 10 and offset 0. `search=cat` filters by name. limit=2&offset=2 over 5 images → 2 items, total 5. limit 500 is clamped to 100 and offset −10 to 0.
  - [A] `GET /images/preview/test.png` → 200; a missing file → 404.
  - [A] `DELETE /images/file/<name>` → `{success:true}` and the file is removed; a missing file → 422 with `error`.
  - [A] Upload saves to `<notes>/images/`, not images_path. The url starts `images/` and contains the original stem; the stored name is `<YYYYmmdd_HHMMSS>_<hex8>_<sanitized>`.
  - [A] Uploading `evil.html` → 422 "not an accepted image type" and no .html is written (stored-XSS guard).
  - [A] Upload with `upload_to_s3=true` while the provider is local → 422 with an error containing "conflicts".
  - [A] `POST /images/upload_base64 {data, mime_type, filename}` saves into `images/` and the name contains `ai_test`. Without `filename` the name contains `ai_generated_`. Empty data → 400 "No image data"; invalid base64 → 422 "Invalid base64".
- [B] With S3 configured (bucket test-bucket, region us-west-2, mocked client): upload_to_s3 returns a URL matching `^https://test-bucket.s3.us-west-2.amazonaws.com/frankmd/`; missing access key → 422 with "S3". upload_external_to_s3 downloads, then uploads to the same URL pattern; blank url → 400 "required"; no key → 422 "S3".
- [B] Search (WebMock): DuckDuckGo first fetches the `vqd="…"` token from the `duckduckgo.com/?` page, then calls `i.js`, mapping results to `{url:image,title,…}`. A missing vqd → 200 `{images:[]}`. Google CSE maps `items[].link/title` and `total` = int(searchInformation.totalResults) (42). On a 403 it returns 200 `{images:[], error containing "quotaExceeded"}`. Pinterest is DDG with a site filter.
- [A] `parse_resize_ratio` ×4: `true` and `"true"` → 0.5; nil, "", "false" and false → nil; "0.25", "0.5", "0.67" and "1.0" parse as floats; "0", "-0.5" and "1.5" → nil (valid range is (0,1]).
- [A] With images disabled (no images_path): config `enabled:false`, `GET /images` → 404, preview → 404.

### `test/controllers/library_controller_test.rb` — subject: `LibraryController`, `MediaLibraryService`, `MediaUsageService` (13 tests: A=12 B=1 C=0; P=0)
- [A] `GET /library` returns `{images:[{name,path,preview_url,type}], videos:[…]}` from `<notes>/images` and `<notes>/videos`, filtered by the allowed extensions (svg excluded). Example item: `path "images/Photo.png"`, `preview_url "/notes/images/Photo.png"`, `type "image"`.
- [A] Symlinked media is excluded and its target is not exposed.
- [A] `GET /library/usage` returns `{usage_counts:{media_path: number of distinct notes}}`, with no `usage_notes`. References are resolved relative to each note (`../images/photo.png` from `nested/second.md` counts). Repeats within one note count once. Unused media is absent.
- [A] `?path=images/photo.png` returns `{usage_notes:["first.md","second.md"]}` (sorted) with no `usage_counts`.
- [A] The index is built once per request. A ScanError or EACCES → 503 with error = i18n `library.usage_scan_failed`, and no partial map is returned.
- [A] `DELETE /library/file/images/photo.png` → 204 and the file is removed. Traversal (`images/../x`), an unrelated root (`attachments/`) or an unsupported type (`images/readme.md`) → 404. EACCES → 422 with i18n `library.delete_failed`.
- [B] With the S3 provider, list, delete and rename (`PATCH /library/file/<key> {name}` → `{name,path,url}`) all go through `S3MediaLibraryService`.
- [A] The local provider rejects rename → 422 and the file stays.

### `test/controllers/logs_controller_test.rb` — subject: `LogsController`, `Config` introspection (12 tests: A=12 B=0 C=0; P=7)
- [A][P] `GET /logs/tail` returns `{environment, file:"<env>.log", lines:[String]}`. `lines` is a parameter. **Quirk: values ≤0 or >500 fall back to 100; they are not clamped to 500.** An empty log gives an array. A missing file gives `{error:"Log file not found", lines:[]}` (7 tests). The desktop port would only need this if it keeps the log viewer.
- [A] `GET /logs/config` returns `{config_file, config_file_exists, ai_configured_in_file, environment, entries:[{key,value,source∈file|env|default,env_var,sensitive}]}`, with an entry for every `Config::SCHEMA` key. Sensitive non-nil values are masked (the code always returns `"****"`). When YOUTUBE_API_KEY is set in env, source is `env` (or `file` if `.fed` overrides it).

### `test/controllers/media_controller_test.rb` — subject: `MediaController` (3 tests: A=3 B=0 C=0; P=0)
- [A] `POST /media/upload` without a file → 422 "No file". `clip.mp4` is saved and the url starts `videos/` and contains `clip`. `notes.txt` → 422 "not an accepted video type".

### `test/controllers/media_storage_recovery_test.rb` — subject: `MediaStorageRecovery` concern, `UploadStorage` (22 tests [11 textual; loops ×3 and ×4]: A=12 B=10 C=0; P=0)
"Recovery" means: when the S3 provider is selected but unusable, `.fed` is switched back to `media_storage_provider = local` and the response reports `media_storage_fallback: true` exactly once (it is consumed by the next GET /config).
- [A] ×3 (aws_access_key_id / aws_secret_access_key / aws_s3_bucket blanked): `GET /config` shows provider `local` and fallback true, and `.fed` is persisted as local. A second GET gives fallback false.
- [A] Invalid `aws_s3_endpoint` values (`not a URL`, `ftp://…`, `https:///`) recover to local. `http://minio.local:9000` stays valid (provider s3, fallback false).
- [A] `/images/config` reports `media_storage_provider: local` and fallback true without consuming the warning. The root page renders `[data-media-storage-provider-value="local"][data-media-storage-fallback-value="true"]` plus `dialog[role=alertdialog][data-media-storage-target=warningDialog]`; the next page load shows false.
- [A] ×4 endpoints (/images/upload, /images/upload_base64, /images/import, /media/upload): with creds missing, the first POST → 422 with fallback true, the provider becomes local, and no `images/` or `videos/` are written. Retrying → success and a local file.
- [B] ×4: an invalid SDK region (`bad_region`) → 422 with fallback true, switched to local, nothing written, and the upload is not retried. The warning is still pending on the next GET /config. A retry succeeds locally.
- [B] ×4: S3 returning InvalidAccessKeyId (put_object called once) → 422 with fallback true and local; a GET /config shows true, then false.
- [A] The legacy upload_to_s3 and upload_external_to_s3 with a missing secret also recover → 422 with fallback true and no import.
- [B] Temporary errors (SlowDown, InternalError, NetworkingError) → 422 with no provider change and no fallback flag.
- [A] If S3 is restored before the pending warning is shown, GET /config reports s3 and fallback false.
- [A] If persisting `local` fails (`set` returns false), S3 stays selected and no fallback is claimed; uploads → 422 without the flag.
- [B] AccessDenied plus a failed save → S3 retained and no fallback flag.

### `test/controllers/notes_controller_test.rb` — subject: `NotesController`, `NotesService`, `Note`, `HugoService` (67 tests: A=50 B=0 C=17; P=8)
Index / SPA shell:
- [C] Root renders `div[data-controller~=app]`. With no history there is no `.home-recent`.
- [C] Home: `#home-title` "FrankMD" with `img.home-logo[src=/icon.svg][alt=""]`; `.workspace-header[hidden]`; the description "A simple, self-hosted Markdown editor for writing and organizing your notes."; `.home-actions` holds first the "Find file" button (openFileFinder) and last New note (newNote), both with svg aria-hidden; no "Ctrl" text in the actions; a "Clear history" button in the recent heading; the text "Shared across devices…" is absent.
- [C] A Library toggle (aria-pressed=false). The library panel has an Images tab (pressed=true), a Videos tab and a grid.
- [C] Workspace chrome: exactly one library toggle and one settings toggle in the sidebar footer `.workspace-navigation` (3 buttons, 16px svgs, Home first with aria-label "Home", no text spans, the settings toggle has no aria-pressed). The title button is disabled and triggers renameCurrentNote. The autosave status labels are JSON (`unsaved` = i18n `status.unsaved`). The title input has aria-describedby hint/error and blur→onTitleRenameBlur. Header menu: 10 `menuitem`s, 10 pin `menuitemcheckbox`es, 10 pinned actions of which 3 are pinned by default. The sidebar header has 4 global actions (newNote, newFolder, openFileFinder, openContentSearch). The footer brand is non-interactive. Sidebar toggle/open buttons carry aria-controls=workspace-sidebar and i18n labels. The stats toggle in Settings has aria-pressed=false.
- [C] General settings has a media storage menu with Local (checked) and S3 options plus an error alert.
- [C] About is a Settings category: an `h2#settings-about-title`, "FrankMD", the description, a version `vN.N`, "Fabio Akita", and a GitHub link (target=_blank, rel=noopener noreferrer). There is no standalone about dialog.
- [C] In every locale, menu labels and aria-labels contain no "(Ctrl+…" or "(F…" shortcut text, while titles keep it (toggle_preview, find, emoji, help).
- [C] The image picker has a `p[data-folder-images-target=error][role=alert]`.
- [C][P] Root has `data-app-initial-path-value=""` and `data-app-initial-note-value="{}"`.
- [A] `GET /?file=./.fed` → 403 (canonical alias of the config file).
- [C] The root HTML includes the server-rendered tree (`data-path="test.md" data-type="file"`).

Tree:
- [A] `GET /notes/tree` returns tree HTML listing files and folders with data-path and data-type.
- [C] The `expanded` and `selected` params render `tree-chevron expanded` and `tree-item selected`.

Show / assets / path safety:
- [A] `GET /notes/test.md` (JSON) returns `{path, content, revision}`, where revision = SHA-256 hex of the content. Missing → 404.
- [A] `.fed` → 403 without leaking content. `/notes/foo/%2E%2E/.fed` → 403. `../../etc/passwd` → 403.
- [A] Non-markdown assets are served raw (`images/test.png` with Content-Type `image/png`); a missing asset → 404. The internal lock files `.frankmd-filesystem.lock` (`NotesService::FILESYSTEM_LOCK_FILENAME`) and `.frankmd-config.lock` → 404.

Create:
- [A] `POST /notes/new_note {content}` → 201 and writes `new_note.md` (`.md` is appended when missing). Subfolders work, and missing parent dirs are created (`2026/01/30/my-blog-post/index.md`, content byte-exact). An existing note → 422. `.fed` → 403.
- [A] Race: if the file appears after the existence check, create → 422 and the original content is preserved (exclusive create).
- [A] Hugo ×5: `POST /notes {template:"hugo", title}` at 2026-02-01 10:30 → path `2026/02/01/my-blog-post/index.md`. Content starts with `---` and includes `title: "My Blog Post"`, `slug: "my-blog-post"` and `draft: true`. `parent:"blog"` gives a prefix of `blog/2026/02/01/`. An empty title → 422 "required". `Café Açaí` → slug `cafe-acai`.

Update:
- [A] PATCH writes the content and returns `revision` = sha256(new content). A missing note → 404 `{"error":"Note not found"}` and the file is NOT created. A stale autosave after a rename, a delete or a parent-folder rename → 404, the old path is not recreated, and the new location keeps its content. `.fed` and `/notes/%2E/.fed` → 403 with `.fed` unchanged.

Destroy:
- [A] Deletes the note → 200. `expected_revision` matching the current sha256 → deleted. A stale revision → 409 `{code:"revision_conflict", error=~/changed since it was loaded/i}` and the file is preserved. Missing → 404. `.fed` → 403.

Rename:
- [A] `POST /notes/old.md/rename {new_path}` moves the note, including into a subfolder. An existing destination → 422 with both files unchanged (no overwrite). A missing source → 404. Source `.fed` → 403. A destination of `foo/../.fed` → 403 and the source is unchanged.

Search / backlinks:
- [A] `GET /notes/search?q=` returns an array of `{path, context:[…lines], …}`. The query is a regex (`foo\d+bar` matches). No match → `[]`. Context includes surrounding lines (more than 1).
- [A] `GET /notes/target.md/backlinks` → `{count, backlinks:[{path}]}`; a `[[target]]` in source.md counts as 1. None → `{count:0, backlinks:[]}`.

Bookmarkable HTML:
- [C][P] HTML `GET /notes/bookmarked.md` renders the SPA. initial-path-value is the plain path (no quotes) and initial-note-value is the JSON `{path, content, revision, exists:true}`.
- [C][P] Special characters (`"`, `&`, `<>`, `\`) round-trip through the data attribute JSON.
- [C][P] Nested Hugo paths render.
- [C] A missing file renders the SPA with `exists:false` and text matching /no longer exists|was deleted/.
- [C] `/?file=from_param.md` preloads that note.
- [C] The settings dialog (`aria-modal=true`) carries font, font-size and editor-width values.

Turbo:
- [A][P] ×4: create, destroy and rename turbo-stream responses update `file-tree-content` (new path present, removed or old path absent). Create honours `expanded`.

### `test/controllers/recent_files_controller_test.rb` — subject: `RecentFilesController`, `RecentFilesService` (7 tests: A=5 B=0 C=2; P=1)
History is stored in `.fed` (key `recent_files`, a list of `{path, opened_at ISO8601}`, LIMIT 10, newest first, deduped).
- [A] `POST /recent_files {path}` → 204. GET renders `a[data-path="docs/<sample>.md"]` with escaped text. `DELETE` → 204 and empties the entries; the panel is then omitted entirely.
- [A][P] GET always returns HTML, even when JSON is requested.
- [C] With 0 entries nothing renders. With 5–10 entries, 5 are visible and the rest are `li[hidden]`. A "show more" button (aria-controls=home-recent-files, aria-expanded=false) appears only when more than 5 exist, along with a hidden "show less" (aria-expanded=true).
- [C] Each entry shows `.home-file-name` (stem "timed") and `time.home-file-time[datetime=<iso>]` reading "10 minutes ago". A malformed `opened_at` renders no `<time>`, with the text "Unknown time".
- [A] Recording a missing note → 404 and no file is created. `PATCH /config {recent_files}` → 422 (not a UI key).
- [A] If `record` returns false → 503.
- [A] The `.frankmd-config.lock` file cannot be fetched (404), and `NotesService#write` raises InvalidPathError for it.

### `test/controllers/translations_controller_test.rb` — subject: `TranslationsController`, `config/locales/*` (22 tests: A=22 B=0 C=0; P=0)
`GET /translations` returns `{locale, translations:{…}}`.
- [A] The English default has the sections common, dialogs, status, errors, success, editor, sidebar, library, preview and context_menu. Sample values: common.cancel "Cancel", common.save "Save", status.saved "Saved", errors.note_not_found "Note not found", library.title "Media Library", library.images "Images".
- [A] Locale via the FRANKMD_LOCALE env var. pt-BR: Cancelar / Salvar / Salvo / "Biblioteca de mídia" / "Vídeos". es: Cancelar / Guardar / Guardado. ja: キャンセル / 保存 / 保存済み.
- [A] Keys missing in a locale are filled from English (a deep merge in which the locale wins). en returns the English tree unchanged (errors.title_required "Title is required").
- [A] Selected es strings: errors.title_required "El título es obligatorio"; errors.permission_denied "Permiso denegado. Es posible que no tengas acceso a este archivo o carpeta."; connection.offline_message "Se perdió la conexión. La edición está desactivada hasta que se restablezca.".
- [A] Locale precedence is `.fed locale` > non-empty `FRANKMD_LOCALE` > `en`. An empty FRANKMD_LOCALE="" is ignored. An invalid locale → `en`. `locale = es` in `.fed` works. `PATCH /config {locale:"ja"}` writes `locale = ja` (replacing the old line, no duplicates) and persists across requests.
- [A] Coverage across en, pt-BR, pt-PT, es, he, ja and ko: non-empty common.{cancel apply save create delete rename close search insert edit ok}, status.{saved unsaved error_saving error_loading searching no_matches} and errors.{note_not_found folder_not_found file_not_found}. The ~60 issue-187 dialog keys (log_viewer.*, file_finder.preview_error, image_picker.*, ai_diff.get_key_from, text_format.*, video.url_hint_*) are present in every locale. `video.url_hint_youtube` starts with `http(s)://` and `url_hint_file` starts with `/`.

### `test/controllers/upload_csrf_test.rb` — subject: CSRF on upload endpoints (10 tests [3 textual; loop ×8 + ×1 + 1]: A=10 B=0 C=0; P=10)
- [A][P] With forgery protection on, a POST without a token to /images/upload, /images/upload_to_s3, /images/upload_external_to_s3, /images/upload_base64, /images/import, /media/upload, /ai/fix_grammar or /ai/generate_image → 422. DELETE and PATCH on /library/file/… without a token → 422.

### `test/controllers/youtube_controller_test.rb` — subject: `YoutubeController` (10 tests: A=4 B=6 C=0; P=1)
- [A] `GET /youtube/config` `{enabled}` follows YOUTUBE_API_KEY. Search with no key → 503 "not configured". Blank q → 400 "required".
- [B] Search (WebMock on googleapis youtube/v3/search) → `{videos:[{id, title, channel, thumbnail}]}`. The thumbnail is the `medium` url, falling back to `default`. The request includes `q`, `type=video` and `maxResults=6`. API 403 → 200 `{videos:[], error}`. Empty items → `[]`.
- [B][P] The HTML results use `data-action="error->video-dialog#thumbnailError"` and no inline `onerror=`.

### `test/system/creation_destination_test.rb` — subject: new note/folder dialogs, explorer selection (3 tests: A=0 B=0 C=3; P=0)
- [C] Selecting folder `docs/nested` and then the toolbar "New Note (Ctrl+N)": the dialog previews `docs/nested/Sample.md` for name "Sample". Switching to the Hugo template previews `blog/YYYY/MM/DD/sample/index.md`; switching back to empty shows the docs path again. Create makes the file. New Folder with nested selected previews `docs/nested` and creates `docs/nested/child`.
- [C] #268: a selected but collapsed folder is still the destination. After create, the folder is expanded so the new note or folder is visible.
- [C] `HUGO_PATH_STYLE=flat`: a Hugo post from the toolbar goes to `blog/my-post.md`, ignoring the selected folder. From the folder context menu ("newNoteInFolder" on docs), it goes to `docs/explicit-post.md`.

### `test/system/custom_theme_editor_test.rb` — subject: theme controller / custom theme editor (2 tests: A=0 B=0 C=2; P=0)
- [C] Settings → open the custom editor. The fields are Theme name, Accent, Background and Text, and a low-contrast warning ("Low contrast") is shown. Edits preview live via the CSS var `--theme-accent`. The advanced `<summary>` has syntax colours (the default for Code functions is `#f0e68c`; Code keywords maps to `--theme-syntax-keyword`). Save closes the dialog, the current theme label becomes "Reference palette", and `.fed` gets theme=custom with accent #5b8cff. The note is untouched. Reopening, editing and pressing Escape reverts the live preview to the saved colour and returns focus to the open-editor button.
- [C] Mobile at 390×844: `#bad` disables Save and shows "six-digit hex". The dialog fits the viewport with no horizontal overflow and no "Translation missing". Escape without saving leaves custom_theme nil.

### `test/system/draft_recovery_test.rb` — subject: autosave drafts in localStorage (5 tests: A=0 B=0 C=5; P=0)
Drafts are stored under the localStorage key `frankmd:draft:<encodeURIComponent(path)>` with the JSON shape `{schemaVersion:1, path, content, baseRevision, draftRevision, updatedAt}`.
- [C] Switching files before the autosave debounce keeps the outgoing draft. Reopening a.md shows the draft while the disk still holds the server content. Browser back and forward navigate between notes and restore drafts; going back past the first note shows the placeholder. Setting the content to "" is respected as a draft.
- [C] After a hard reload, the unsaved draft is restored and the status shows "unsaved"; the disk is unchanged.
- [C] Renaming a note with a dirty draft moves the draft to the new key and deletes the old one. The disk keeps the server version.
- [C] Moving the file into a folder by drag and drop also moves the draft.
- [C] Renaming the folder `docs`→`archive` remaps nested drafts, and an unrelated `docs-old/…` draft is untouched (a prefix match must include the trailing `/`). Deleting the note and then the emptied folder clears matching drafts. A non-empty folder delete is refused.

### `test/system/explorer_background_test.rb` — subject: explorer empty-area menu (2 tests: A=0 B=0 C=2; P=0)
- [C] #261/#267: clicking the empty tree area deselects and shows the explorer menu. New Note then targets the root (preview `note-name.md`), and clicking outside dismisses the menu. The menu's "Select All" selects every visible item and closes the menu; "Find File" opens the file finder.
- [C] The menu's "New Note" previews `root/menu_note.md` and creates `menu_note.md`. Escape dismisses the menu.

### `test/system/home_recent_files_test.rb` — subject: Home recent list (4 tests: A=0 B=0 C=4; P=0)
- [C] The recent list appears after opening a note ("Recently opened"). "Clear history" asks for confirmation in an alertdialog (common.confirm), the section disappears, and it stays gone after a reload.
- [C] With 7 entries: 5 are shown with `time[datetime]` and a "Show more" button. Show more → 7 entries, focus moves to the 6th entry (note-1.md), and the button becomes "Show less". Show less → 5 entries and focus returns to "Show more". Clicking an entry opens it and shows the workspace header.
- [C] Opens are shared across visits, newest first. Externally deleted notes are filtered out of the list. Clicking an entry whose note was deleted after render shows "Note not found", never recreates the file, and the entry disappears.
- [C] Mobile at 390px: Home stays reachable from the overlay explorer and there is no horizontal overflow.

### `test/system/markdown_surround_test.rb` — subject: CodeMirror selection surround (2 tests: A=0 B=0 C=2; P=0)
- [C] Selecting the whole text and typing `*` wraps it to `*text*` and keeps the inner text selected. A second `*` gives `**text**`. Ctrl+Z undoes one level back to `*text*`.
- [C] In Vim mode, the visual command `gg0vw*` keeps its Vim meaning (search) and the text is unchanged.

### `test/system/multi_drag_test.rb` — subject: drag-drop controller (2 tests: A=0 B=0 C=2; P=0)
- [C] #266: after a Ctrl-click multi-selection, dragging one selected item onto a folder moves all selected items (first.md and second.md go to target/) and leaves unselected ones.
- [C] Dragging an unselected item moves only that item.

### `test/system/notes_test.rb` — subjects: app shell, explorer, file ops, table editor, image picker (41 tests: A=0 B=0 C=41; P=0) — includes `NotesWithImagesTest` (7)
- [C] Home: the document header is hidden and the title disabled, `#home-title` shows FrankMD, the tree is present, and there is no "Select a note to edit" placeholder text.
- [C] Notes appear in the tree. Clicking a note opens it: currentPath shows the stem, the tree item is `.explorer-selected:focus`, and the editor is NOT focused.
- [C] The New Note dialog opens directly with the Empty template pressed and creates `brand_new_note.md`. With the Hugo card, "Hello World" creates `blog/YYYY/MM/DD/hello-world/index.md` (toolbar Hugo posts go under `blog/`) and nothing at the root date path. Spanish template card titles wrap and are not clipped (#151).
- [C] New Folder creates the dir. The editor shows the file content. The preview toggle shows/hides the panel and renders h1 and strong. Folders expand and collapse on click.
- [C] Theme picker: Settings ("Open Settings") does not change the preview state. Changing Light↔Dark updates the current-theme label. Settings closes via the close button, a backdrop click and Escape; the settings toggle never gets aria-pressed and the preview stays on.
- [C] Right-click on a note opens a context menu with Rename and Delete. Rename via the menu and via F2 (on a note or a folder) renames on disk, appending `.md`. Delete asks for confirmation in an alertdialog. Ctrl-click multi-select then Delete says "2 selected items" and deletes both; the current path stays first_selected. The Delete key on a focused tree item deletes after confirmation. Deleting a non-empty folder shows "Folder not empty" and keeps it. The Delete key inside the editor edits text and does not delete the note.
- [C] Table editor (opened from the overflow menu "Insert/Edit Table"): the default grid is 3×3 (9 inputs) with headers "Header 1" and "Header 2". Add/Remove Column gives 12/9 inputs, and Add/Remove Row gives 12/9. Insert Table writes markdown containing `| Name` and `| ----`. With the cursor inside an existing table, the editor loads it (Col1, Col2). The cell context menu has Move Left/Right, Delete Column, Move Up/Down and Delete Row. Move Right swaps the headers. Move Down moves the row. Delete Column/Row leaves 6 inputs. Delete Row is disabled on the header row.
- [C] The "Insert Image" button is always visible, even with an empty IMAGES_PATH.
- [C] NotesWithImagesTest (IMAGES_PATH set): the image button is visible. The Local tab with collection=server shows 2 `.image-grid-item`s, and the search "Search images by filename..." filters to 1. Selecting an image shows options with selectedName `myimage.png` and alt prefilled with the stem `myimage`. Insert adds `![photo](…/images/preview/photo.png)`. A link wraps it as `[![clickable](…)](https://example.com)`. A custom alt gives `![A beautiful sunset]`.

### `test/system/preview_image_viewer_test.rb` — subject: preview image lightbox (2 tests: A=0 B=0 C=2; P=0)
- [C] A preview image is a keyboard-operable `.preview-image-trigger`. Enter opens `dialog.image-viewer` with the alt preserved ("Example") and focus on the close button. Escape or the Close button closes it, returns focus to the trigger, and leaves the URL unchanged (`/notes/images.md`).
- [C] A linked image keeps its link (no button inside the anchor) and gets a separate `.preview-image-enlarge` control. The viewer fits 390×844. Clicking the link navigates to the target note.

### `test/system/settings_dropdown_test.rb` — subject: Settings popover menus (4 tests: A=0 B=0 C=4; P=0)
- [C] At 1280×720, 390×600 and 360×360, the theme popover opens with the current option focused (aria-checked). Settings does not scroll. The menu stays within 7px of the viewport edges and scrolls internally. End focuses the last option, which is clickable. Tab closes it and moves focus to the custom-editor button. Escape closes the menu first (focus returns to its button, aria-expanded=false), and a second Escape closes Settings.
- [C] Only one popover is open at a time (theme / media storage / locale). Selecting an option closes the menu and refocuses its button. Choosing the theme dark sets `html[data-theme=dark]`. Switching category closes popovers.
- [C] Resizing keeps the focused option visible inside the menu.
- [C] Scrolling Settings dismisses the menu. Closing Settings leaves no open popover.

### `test/system/workspace_test.rb` — subject: workspace layout, editor chrome, title rename, menus (23 tests: A=0 B=0 C=23; P=0)
- [C] The editor uses the native selection (no `.cm-selectionLayer`). With multiple selections, secondary ranges get `.cm-secondarySelectionLayer` and `.cm-cursor-secondary`.
- [C] Editor margins are >40px on each side and the scrollbar is at the pane edge (14px wide), with or without typewriter mode. Clicks in the left margin land at column 0 and clicks in the right margin at line end. A click in the editor closes the overflow menu. The layout holds in split mode and at 320px.
- [C] Empty notes focus the editor. The save status shows the translated "unsaved" even before translations load. Reloading an unchanged note does not mark it unsaved or create a draft.
- [C] Home hides the header. For a missing note (`/notes/missing.md`), the title is disabled, the menu has 1 item, and the app returns to Home within about 6s.
- [C] Explorer header and footer layout: 4 global actions of 32×32 with 16px icons and 8px padding; the brand label is non-selectable; the sidebar toggle hides the sidebar and focuses sidebarOpen; About opens via the keyboard.
- [C] Title rename: Enter on the title opens an inline input with the stem selected to the end (caret at 8,8). Enter renames `folder/original.md`→`folder/renamed.md`, keeping the folder and the unsaved edits, which are saved to the new path. The URL becomes `/notes/folder/renamed.md` and focus returns to the title.
- [C] The input matches the label's geometry and resizes with the text. Escape cancels. A name collision shows an inline alert (/exist/i) with aria-invalid and no file changes, after which a different name succeeds. Blur submits the rename and keeps editor focus. A blank title shows `errors.title_required`, and correcting it on blur renames.
- [C] About is inside Settings at desktop and 320px. Escape returns focus to the settings toggle.
- [C] A hidden explorer can be reopened for Library and Settings, and preview mode is preserved. In single view mode the preview toggle swaps the panels. Overflow menu actions move focus into dialogs.
- [C] Copy content (menu and pinned button) copies the current editor text, including unsaved edits, and shows a toast `status.copied_to_clipboard`.
- [C] Menu pins: pinning "table" shows a toolbar button that persists across reloads; unpinning removes it. Unpinning preview still leaves preview reachable from the menu. Mouse and keyboard focus styling: pin opacity 0/1 rules, keyboard arrow-right focuses the pin, and Enter toggles it.
- [C] Long names and translated chrome (en/es/he with rtl, dark/light) fit at 1400, 768 and 320 with no horizontal overflow; the toolbar sits within the header and the editor starts below it.
- [C] Mobile explorer is an overlay: main is `inert`, choosing a file closes the overlay, and Escape closes it and focuses the opener. Resizing to desktop focuses the visible sidebar toggle.
- [C] The stats panel is hidden by default. The Settings → Editor stats toggle shows the word count ("3") and persists `show_stats = true` to `.fed`.

## Porting notes
- **Path safety on every note operation** (show, create, update, destroy, rename src/dst, `?file=`): `.fed` and any canonical alias (`./.fed`, `foo/../.fed`, percent-decoded `%2E%2E`, `%2E`) → 403 (forbidden), while `../../etc/passwd` → 403. The internal lock files `.frankmd-config.lock` and `.frankmd-filesystem.lock` are hidden (404) and cannot be written.
- **Revisions are SHA-256 hex of the file content.** Show and update return `revision`. Destroy accepts `expected_revision`; a mismatch → conflict with `code: "revision_conflict"`.
- **Update never creates a file.** A missing path (stale autosave after rename, delete or parent rename) → not found. Create is exclusive and must not overwrite on a race; rename must not overwrite its destination.
- Create appends `.md` and makes parent dirs. Hugo template: `[parent/]YYYY/MM/DD/<slug>/index.md` with front matter keys title, slug, date and draft:true; the slug strips accents. `HUGO_PATH_STYLE=flat` gives `blog/<slug>.md`. Toolbar Hugo posts go under `blog/`; an explicit folder from the context menu overrides that.
- Folder delete only works on empty folders. Folder rename remaps the expanded-folder set (prefix + `/`). Draft keys are remapped the same way in the UI.
- `.fed` writes are line-based `key = value`, preserve other lines, and replace an existing key in place (no duplicate `locale =`). Only `UI_KEYS` are writable from the UI. Typed parsing applies (ints, bools). A malformed `custom_theme` JSON line → nil without breaking the other keys. Values with `"` are rejected for `editor_font`. Custom theme colours are lowercased and validated as six-digit hex.
- Locale precedence: `.fed locale` > non-empty `FRANKMD_LOCALE` > `en`, and an invalid value → `en`. Translations deep-merge over English.
- AI provider auto-priority is openai > anthropic > openrouter > ollama > gemini, with default models as listed above. The image model default is `google/gemini-3.1-flash-image-preview` (needs OPENROUTER_API_KEY).
- Media storage: when S3 is misconfigured (missing key, secret or bucket, invalid endpoint or region, rejected creds), the config falls back to local, persists that to `.fed`, and reports a one-shot `media_storage_fallback` warning. Transient S3 errors must NOT switch providers. Nothing is written locally on a failed S3 attempt.
- Uploads: images go to `<notes>/images/`, videos to `<notes>/videos/`, named `YYYYmmdd_HHMMSS_<hex8>_<sanitized>`. Base64 uploads without a filename get `ai_generated_<hex8>`. Extensions are allow-listed (`image_upload_extensions` / `video_upload_extensions`), with server-side rejection of `.html` and `.svg`. External fetch: at most 25 MB, at most 5 redirects, egress policy blocks private IPs, and html/svg content types are rejected.
- The images index default limit is 10, clamped to [1..100] with offset ≥0. `parse_resize_ratio` accepts (0,1]; legacy `true` means 0.5.
- Media usage counts distinct notes per media path, resolves links relative to the note's dir, and on any scan error returns an error, never a partial map.
- Recent files: max 10 entries, newest first, deduped, stored in `.fed` `recent_files` (not user-writable via PATCH). Recording a missing note → not found. Home shows 5 entries plus Show more/less. Malformed timestamps show "Unknown time".
- Quirk: in the logs tail, `lines` >500 falls back to 100 rather than being clamped to 500 (the test name implies a clamp but only asserts ≤500). Sensitive config values are masked as the literal `****`.
- Auth, CSRF, turbo-stream and health (P) are web-only. The desktop app should still round-trip the `auth_token` key in `.fed` without breaking.


---

<!-- part totals: files=47 tests=872 A=648 B=55 C=169 P=0 -->

## JavaScript unit tests — top-level, lib/, image_sources/

**Infrastructure (no counts).** `vitest.config.js`: includes `test/javascript/**/*.test.js`, default `environment: 'node'` (files opt into jsdom via `@vitest-environment jsdom` docblock), `setupFiles: test/javascript/setup.js`; aliases `lib`→`app/javascript/lib`, `marked`/`katex`/`@hotwired/turbo-rails`/`@rails/request.js`/`@replit/codemirror-vim` → `test/javascript/mocks/*`, and `dompurify` → the *vendored* `vendor/javascript/dompurify.js` (sanitizer tests exercise the shipped artifact). `setup.js`: per-test `console.error`/`console.warn` spies (silenced, assertable), stubs `Range.prototype.getClientRects/getBoundingClientRect` for CodeMirror under jsdom, polyfills `requestAnimationFrame` with `setTimeout(0)`. `helpers/jsdom_globals.js`: `setupJsdomGlobals()` builds a fresh JSDOM and assigns window/document/Element/HTMLElement/HTMLDialogElement/CustomEvent/MutationObserver/Node/getComputedStyle globals. Mocks: `marked.js` (regex mini-renderer for #/##/###, **, *, paragraphs; `lexer` splits on blank lines into paragraph tokens with `raw = block+"\n\n"`), `katex.js` (records `render` calls/options, `throwNext` flag, injects `<span class="katex">`), `requestjs.js` (get/post/patch/destroy over `global.fetch`, sets Accept for json/turbo-stream, JSON-encodes object bodies, `FetchResponse` with `json/text/html/ok/statusCode`), `turbo-rails.js` (`Turbo.renderStreamMessage` applies `update` streams to DOM), `codemirror-vim.js` (records `Vim.defineEx/map/unmap`, `getCM().on("vim-mode-change")` handlers, `vim()` returns `{__isVim:true}` marker).

### `test/javascript/diff_utils.test.js` — subject: `app/javascript/lib/diff_utils.js` (17 tests: A=17 B=0 C=0; P=0)
- [A] group "tokenize ×5": splits into alternating word / whitespace-run tokens preserving exact whitespace: `"hello world"`→`["hello"," ","world"]`; `"hello  world"`→`[..,"  ",..]`; `"\n"` is its own token; `""`→`[]`; `"  hello  "`→`["  ","hello","  "]`.
- [A] group "computeWordDiff ×5": identical→`[{type:"equal",value:"hello world"}]`; insertion yields an `insert` op containing "beautiful"; deletion yields `delete` op; replacement yields both delete("world") and insert("universe"); `("","")`→`[]`, `("hello","")`→`[{delete,"hello"}]`, `("","hello")`→`[{insert,"hello"}]`.
- [A] group "lcsWordDiff ×7" (token arrays): identical→single `equal` with concatenated value `"abc"`; insert/delete ops of single tokens; `[]`,`[]`→`[]`; all-new→`[{insert,"ab"}]`; all-deleted→`[{delete,"ab"}]`; consecutive same-type ops merged (`[a,x,y,b]→[a,b]` gives exactly one delete `"xy"`).

### `test/javascript/indent_utils.test.js` — subject: `app/javascript/lib/indent_utils.js` (25 tests: A=25 B=0 C=0; P=0)
- [A] group "parseIndentSetting ×9": number or numeric string → indent unit. `undefined/null/""`→2 spaces; `0`/`"0"`→`"\t"`; 1,2,4,6 → that many spaces; >6 clamps to 6 spaces (7,10,100); negative (-1,-5) → default 2 spaces; non-numeric (`"abc"`) → 2 spaces.
- [A] group "indentLines ×5": prefixes every line (split on `\n`) with indent, including empty lines (`"line1\n\nline3"`→`"  line1\n  \n  line3"`); tab and 4-space units.
- [A] group "unindentLines ×8": removes up to one indent unit per line: exact unit removed; fewer leading spaces than unit → removes those spaces (`" hello"`,`"   hello"` with 4-space unit → `"hello"`); non-whitespace untouched (`"xhello"`); a leading tab is removed when unit is spaces; mixed lines each lose only one unit (`"  line1\n    line2\nline3"`→`"line1\n  line2\nline3"`); empty lines kept.
- [A] group "roundtrip ×3": indent→unindent restores original (spaces, tabs, already-indented text `"  already indented\n    more indented"` → indented `"    …\n      …"`).

### `test/javascript/markdown_utils.test.js` — subject: `app/javascript/lib/markdown_utils.js` (14 tests: A=14 B=0 C=0; P=0)
- [A] group "findTableAtPosition ×10" `(text, pos)` → `{lines, startLine, startPos, endPos}` or `null`: finds contiguous `|`-lines around cursor line (3-line table → `lines.length==3`, `lines[0]=="| Name | Age |"`); null when cursor outside; null for single pipe line; whole-document table → `startPos 0`, `endPos == text.length`; table at doc start (`startLine 0`) and end; **requires a separator row** (`| --- |`, colon-aligned `:---`, `:---:`, `---:` accepted) — consecutive pipe lines without separator → null, `| this is not` lines → null; cursor on separator row works.
- [A] group "findCodeBlockAtPosition ×4" → `{language, content,…}` or null: cursor inside ```` ```javascript ```` block → language `"javascript"`, content contains body; outside → null; bare fence → language `""`; with multiple blocks picks the one containing pos (`python`).

### `test/javascript/stats_utils.test.js` — subject: `app/javascript/lib/stats_utils.js` (18 tests: A=18 B=0 C=0; P=0)
- [A] group "calculateStats ×10" → `{wordCount, charCount, byteSize, readTimeMinutes}`: whitespace-separated word count (multiple spaces/newlines ok); charCount = JS length; `""`/null/undefined → all 0; read time = `ceil(words/200)` (200→1, 201→2); byteSize = UTF-8 bytes (`"café"`→5).
- [A] group "formatReadTime ×3": 0 and 1 → `"< 1 min"`; n≥2 → `"n min"`.
- [A] group "formatFileSize ×5": `<1024` → `"N B"` (0,100,1023); KB/MB/GB with 1 decimal when value <10 (`1024`→`"1.0 KB"`, `1536`→`"1.5 KB"`, `1048576`→`"1.0 MB"`, `1572864`→`"1.5 MB"`, `1073741824`→`"1.0 GB"`) and no decimal when ≥10 (`10240`→`"10 KB"`).

### `test/javascript/table_utils.test.js` — subject: `app/javascript/lib/table_utils.js` (21 tests: A=21 B=0 C=0; P=0)
- [A] group "parseMarkdownTable ×7" (string or array of lines) → array of rows of trimmed cell strings: skips separator rows (incl. colon-aligned) and blank lines; `''`/`[]`→`[]`; keeps inline markdown verbatim (`` `foo` ``, `A *bold* statement`).
- [A] group "generateMarkdownTable ×6": pads each column to max width (min width 3), header + `| --- |` separator with dashes = width; exact output for Name/Age example: `"| Name  | Age |\n| ----- | --- |\n| Alice | 30  |\n| Bob   | 25  |"`; all lines equal length; empty cell → `|     |`; `[]`/null/undefined→`''`; short rows padded to header column count.
- [A] roundtrip ×1: parse(generate(data)) == data.
- [A] group "row/column ops ×7" (all return new arrays, no mutation): `swapColumns(data,0,2)`; original unchanged; `swapRows(data,1,2)`; `deleteColumn(data,1)`; `deleteRow(data,1)`; `addColumn(data,'New')` appends header and `''` to other rows; `addRow(data)` appends row of `''` × column count.

### `test/javascript/text_utils.test.js` — subject: `app/javascript/lib/text_utils.js` (23 tests: A=23 B=0 C=0; P=0)
- [A] `escapeHtml` escapes `< > " '` as `&lt; &gt; &quot; &#39;` (attribute-safe).
- [A] `isSafeImageUrl`: true for `http:`, `https:`, `data:image/…`, `blob:`, root-relative `/images/…`; false for `javascript:`, protocol-relative `//evil`, `/\evil`, leading space or newline before value, `file:`.
- [A] group "escapeHtmlString ×6": `'` → `&#039;` (note: differs from escapeHtml's `&#39;`), `&`→`&amp;`, `"`→`&quot;`, `<>`; `""`/null/undefined → `""`; plain text unchanged.
- [A] group "fuzzyScore ×7" `(target, query)`: 0 when not a subsequence; >0 on match; consecutive > sparse; bonus after separator (`src/button` > `srcxbutton`); bonus at start (`readme.md` > `unreadme.md`); shorter target preferred (`app.js` > `application.js`); empty query → length bonus `max(0, 10-(len-0))` = 5 for `"hello"`.
- [A] group "levenshteinDistance ×8": standard edit distance (identical 0, vs empty = length, kitten/sitting 3, symmetric, `("","")`0).

### `test/javascript/tree_utils.test.js` — subject: `app/javascript/lib/tree_utils.js` (7 tests: A=7 B=0 C=0; P=0)
- [A] group "flattenTree ×7": depth-first collects only `type:"file"` nodes from nested `{type:"folder",children}` trees, preserving all file properties (`file_type`, `size`); `[]`/null/undefined → `[]`; empty folders and folders lacking `children` contribute nothing.

### `test/javascript/url_utils.test.js` — subject: `app/javascript/lib/url_utils.js` (16 tests: A=16 B=0 C=0; P=0)
- [A] group "encodePath ×6": `encodeURIComponent` per `/`-segment, slashes preserved (`"hello world/test file.md"`→`"hello%20world/test%20file.md"`, `#`→`%23`, `?`→`%3F`, `café/résumé.md`→`caf%C3%A9/r%C3%A9sum%C3%A9.md`); `""`/null/undefined → `""`.
- [A] group "extractYouTubeId ×10": extracts 11-char id from `youtube.com/watch?v=` (with/without www, extra `&t=30s`), `youtu.be/`, `/embed/`, `/v/`; a bare 11-char id returns itself; non-YouTube URL, `"not a url"`, empty/null/undefined → `null`.

### `test/javascript/lib/app_prompt.test.js` — subject: `app/javascript/lib/app_prompt.js` (9 tests: A=0 B=0 C=9; P=0)
In-app replacements for `alert/confirm/prompt` built as `<dialog>` elements, localized via `window.t` (`common.alert`="Notice", `common.confirm`, `common.ok`="OK", `common.cancel`="Cancel", `common.delete`="Delete").
- [C] `appAlert(msg)`: dialog `role="alertdialog"`, themed class (`bg-[var(--theme-bg-secondary)]`), shows title "Notice" + message, single "OK" button; click resolves `undefined` and removes dialog from DOM.
- [C] `appConfirm(msg,{acceptLabel,destructive:true})`: buttons in order `["Cancel","Delete"]`, accept button uses `bg-[var(--theme-error)]`; accept resolves `true`.
- [C] destructive confirm focuses the dialog itself (`tabIndex -1`), not either button; Cancel resolves `false`.
- [C] native `cancel` event (Esc) resolves `false` and is `preventDefault`ed.
- [C] `appTextPrompt(label,{defaultValue})`: default set as input *value* (no HTML injection — `photo" onerror="alert(1).png` yields no `onerror` attr); OK button (index 1) resolves current input value; Enter keydown in input also accepts.
- [C] prompts queue: only one dialog open at a time; second shows after first resolves; focus restored to original opener after the last.
- [C] focus restored to element active before the prompt; nested prompt over an open parent `<dialog>` restores focus into the parent and keeps parent open.

### `test/javascript/lib/clipboard_image.test.js` — subject: `app/javascript/lib/clipboard_image.js` (6 tests: A=6 B=0 C=0; P=0)
- [A] `imageFileFromClipboard(clipboardData)`: returns first item with `kind:"file"` and `type` starting `image/` (its `getAsFile()`); skips `string` items and non-image files (pdf) → `null`; `null`/`{items:[]}` → `null`.
- [A] `normalizeImageFile(file)`: nameless image gets `pasted-image-<something>.<ext>` with ext from MIME subtype (`image/png`→`.png`, `image/tiff`→`.tiff`, `image/svg+xml`→`.svg` — not faked as png so the server allow-list rejects honestly), type preserved; a file already having an extension is returned as the same object.

### `test/javascript/lib/code_block_highlighter.test.js` — subject: `app/javascript/lib/code_block_highlighter.js` (28 tests: A=26 B=0 C=2; P=0)
Preview-side syntax highlighting of `<pre><code class="language-X">` in rendered HTML using CodeMirror/Lezer parsers, emitting `<span class="tok-<role>">` and adding `data-code-language="<Label>"` on `<pre>`; keeps `class="language-…"`.
- [A] group "per-language token roles ×18": TypeScript/JS (`tok-keyword const`, `tok-number`, `tok-function` for calls describe/expect/toBe, `tok-constant true`, `tok-comment`, `tok-propertyName tok-definition` for object keys); YAML (+`yml`; keys `propertyName definition`, `tok-string "ready"`); Rust (`fn`, `tok-function tok-definition main`, `tok-typeName i32`); Go (+`golang`; `tok-propertyName Sprintf`); Python (+`py`; `tok-variableName str`); Java (class names `tok-variableName tok-definition`); PHP (`$name` variableName, `tok-typeName string`); Kotlin (+`kt`); Ruby (+`rb`; `tok-typeName Greeter`, string token `"hello #{`); Dotenv (+`env`; `DATABASE_URL` variableName definition); Shell (`bash`, `sh` → label "Shell"); SQL (keywords, `tok-constant TRUE`, `tok-operator &gt;=`; `postgresql`→"PostgreSQL", `mariadb`→"MariaDB" labels); TOML (`tok-constant [server]`); Dockerfile (+`docker`; FROM/WORKDIR/RUN keywords); Terraform (`tf`, label "Terraform"; `hcl` → "HCL"; `tok-function max`); C++ (`cpp`→"C++") and C ("C"); C# (`csharp`, `cs`→"C#").
- [A] aliases js/ts/jsx/tsx/json/jsonc/html/css produce expected token classes; extension-style fences `.env`→Dotenv, `.yaml`/`.yml`→YAML, `.rb`→Ruby, `.tf`→Terraform; HTML attrs → `tok-propertyName title` + `tok-string "ok"`; CSS `color` propertyName, `red` constant, `1px` number.
- [A] unsupported language (`language-unrecognized`) → output byte-identical to input.
- [A] limits: a single fence larger than threshold (1100 lines of JS) is not highlighted but text preserved and `data-code-language="JavaScript"` still set; total highlighted-source budget per render (6 blocks of ~850 lines → blocks 0..4 highlighted, block 5 not); max 100 blocks highlighted per render (block index 100 not).
- [A] code content stays text: `&lt;img onerror…&gt;` never becomes an `<img>` element.
- [C] `highlightCodeBlocksInElement(root)`: common languages (js) highlighted synchronously before lazy parser imports (tf) resolve; a lazy result is not applied if the root's content was replaced by a newer render.

### `test/javascript/lib/codemirror_adapter.test.js` — subject: `app/javascript/lib/codemirror_adapter.js` (22 tests: A=22 B=0 C=0; P=0)
Thin shims over an editor-controller interface (`getValue/setValue/getSelection/setSelection/focus/getCursorPosition/getCursorInfo`), tested with plain mock objects.
- [A] group "createTextareaAdapter ×12": `null`/`undefined` controller → `null`; textarea-like object: `value` get/set → `getValue()`/`setValue(v)`; `selectionStart/End` → `getSelection().from/.to`; `setSelectionRange(a,b)` → `setSelection(a,b)`; `focus()` delegates; `addEventListener("input",h)` stores `_inputHandler`, `removeEventListener` nulls it, `dispatchEvent(e)` calls handler with event, no-op without handler.
- [A] group "getters with defaults ×10": `getEditorContent(ctrl, fallbackTextarea)` prefers controller, else textarea `.value`, else `""`; `getEditorSelection(null)`→`{from:0,to:0,text:""}`; `getCursorPosition(null)`→`{line:1,column:1,offset:0}`; `getCursorInfo(null)`→`{currentLine:1,totalLines:1}`; otherwise pass through controller result.

### `test/javascript/lib/codemirror_content_insertion.test.js` — subject: `app/javascript/lib/codemirror_content_insertion.js` (34 tests: A=34 B=0 C=0; P=0)
Text-insertion rules against a mock controller (`getValue, getSelection, getCursorPosition, replaceRange(text,from,to), insertAt(pos,text), setSelection`). Functions return 0 on null controller / empty content.
- [A] group "insertBlockContent ×14": at cursor offset, ensures a blank line separates block from surrounding text: prefix `"\n\n"` if text before doesn't end in newline, `"\n"` if it ends in one, `""` if it ends in two or doc start; suffix `"\n\n"`/`"\n"`/`""` symmetrically based on text after (`"beforeafter"`@6 → insertAt(6,`"\n\ntable\n\n"`); `"before\nafter"` → `"\n\ntable\n"`; `"before\n\nafter"` → `"\n\ntable"`); empty doc → content verbatim. Slash-query replace `{from:7,to:13}` on `"before /table after"` → `replaceRange("\n\ntable\n\n", 6, 14)` (the adjacent single spaces are consumed). Edit mode `{editMode,startPos:10,endPos:50}` → `replaceRange(content,10,50)` (no padding), cursor at `startPos+len` (19) or `startPos+cursorOffset` (16); insert mode cursorOffset relative to insertion (6).
- [A] group "insertInlineContent ×6": replaces selection by default (`replaceRange("😀",5,10)`), `replaceSelection:false` → `insertAt(from,…)`; cursor after content (from+length, 10); slash range `{from:5,to:11}` → `replaceRange(":smile:",5,11)`, cursor 12.
- [A] group "insertImage ×7": no padding at doc start (`replaceRange("![alt](url)",0,0)`); prefix single `"\n"` when previous char not newline; suffix `"\n"` when next char not newline; replaces selection range; slash range `{7,13}` on `"before /image after"` → `replaceRange("\n![alt](url)\n",6,14)`.
- [A] group "insertTable/insertVideoEmbed ×4": both delegate to insertBlockContent (empty doc → `insertAt(0,table)`; `"text"`@4 + `{{< youtube abc123 >}}` → `"\n\n{{< youtube abc123 >}}"`; `/video` slash range → `"\n\n<video></video>\n\n"` @6..14; edit-mode passthrough).
- [A] group "insertCodeBlock ×3": cursor placed after opening fence line: offset = 3 + len(lang) + 1 (`javascript`→14, `""`→4; edit mode python @10 → 20).

### `test/javascript/lib/codemirror_extensions.test.js` — subject: `app/javascript/lib/codemirror_extensions.js` (18 tests: A=0 B=0 C=18; P=0)
- [C] `LINE_NUMBER_MODES` = `{OFF:0, ABSOLUTE:1, RELATIVE:2}` (×3) — these integers are the persisted setting values.
- [C] `createLineNumbers(mode)`: OFF → `[]`; ABSOLUTE/RELATIVE → non-empty extension arrays (×3).
- [C] `createExtensions(opts)` accepts `placeholderText`, `fontFamily/fontSize/lineHeight`, `lineNumberMode`, `onUpdate`, `onSelectionChange`, `onScroll` and returns non-empty array (×7); `createReadOnlyExtensions()` ⊇ base extensions (×2); exports `themeCompartment`, `lineNumbersCompartment`, `readOnlyCompartment` (×3) for runtime reconfiguration.

### `test/javascript/lib/codemirror_extensions_vim.test.js` — subject: `app/javascript/lib/codemirror_extensions.js` (vim) (4 tests: A=0 B=0 C=4; P=0)
- [C] `createVimExtension(true)` → `[vim({status:true}), vimCursorTheme]` (themed block cursor must come *after* vim to override library pink); `createVimExtension(false)` → `[]`.
- [C] `createExtensions({})[0].compartment === vimCompartment` — vim must be first so it receives keydown before any keymap (otherwise normal-mode Enter/Backspace/arrows are swallowed by defaultKeymap).

### `test/javascript/lib/codemirror_markdown_surround.test.js` — subject: `app/javascript/lib/codemirror_markdown_surround.js` (14 tests: A=0 B=0 C=14; P=0) — includes `it.each` with 6 rows
Typing a marker character with a non-empty selection wraps it instead of replacing (real EditorView, via `EditorView.inputHandler`).
- [C] `it.each ×6` markers `* _ \` ~ ^ =`: `"sample"` selected → `"<m>sample<m>"`, inner text stays selected, handler returns true.
- [C] repeated `*` twice → `**sample**`; each surround is its own undo step (undo→`*sample*`→`sample`, redo→`*sample*`).
- [C] multiple ranges incl. backward selection and empty cursor with Unicode: `"猫 🐈 abc z"` ranges (4→0),(5,8),cursor(10), type `_` → `"_猫 🐈_ _abc_ z_"`; selections `["猫 🐈","abc",""]`; backward direction preserved.
- [C] backtick wrapping text that contains backticks uses a longer delimiter plus space padding: `` `sample` `` → ``` `` `sample` `` ```; 130000 runs of ``a` `` handled (no argument-limit crash) → wrapped in ``` `` ```.
- [C] multiline selection wrapped as-is (`*first\nsecond*`, no newlines added).
- [C] not intercepted (returns false, doc unchanged): empty cursor; non-marker chars `# > - + [ ( plain`; `input.paste` user event; during IME composition; read-only editor.

### `test/javascript/lib/codemirror_slash_commands.test.js` — subject: `app/javascript/lib/codemirror_slash_commands.js` (48 tests: A=42 B=0 C=6; P=0) — 26 `it` + 22 `it.each` rows
Notion-style `/` command menu. Completion source runs on an `EditorState` with markdown language; commands transform the paragraph containing the query in **one transaction**. A-tests are text-in/text-out on EditorState with a fake view.
- [A] Menu contents & order (fallback English labels when `window.t` returns the key): `Heading 1, Heading 2, Heading 3, Bulleted list, Numbered list, To-do list, Quote, Code block, Equation, Divider, Table, Image, Video, Emoji` (14). `/he` inside a paragraph → first 3 are headings; `result.from` = index just after `/`; `filterText` of Heading 1 contains `h1`; `validFor` matches `"heading 1"`.
- [A] Icon ids: headings `hash`, `list-bullets`, `list-numbers`, `list-checks`, `quotes`, `code`, `sigma`, `minus`, `table`, `image`, `video-camera`, `smiley`.
- [A] Trigger rules: `/` at doc start or after blank line (`it.each ×2`); after `(` in text triggers; **no** trigger: inside inline code / fenced code (×2), in link/image destinations `[docs](/images…)`, `![logo](/images…)` (×2), slash inside a path `docs/heading`, when the enabled-provider says file isn't Markdown, in Vim Normal mode (`view.cm.state.vim.insertMode===false`; Insert mode triggers). Query is left in doc until selection. Query matches Unicode letters/numbers/combining marks and spaces/dashes (`/日本語 2-é` valid; `日本/語` invalid).
- [A] Block transforms on `"Before /command after"` (query removed, `it.each ×7`, one dispatch): Bulleted `- Before after`; Numbered `1. Before after`; To-do `- [ ] Before after`; Quote `> Before after`; Code block ```` ```\nBefore after\n``` ````; Equation `$$\nBefore after\n$$`; Divider `Before after\n\n---`.
- [A] Soft line breaks preserved (`it.each ×6`, `"first /x\nsecond"`): bullets continuation indented 2 (`- first\n  second`), numbered 3, to-do 6 spaces, quote prefixes every line (`> first\n> second`), code/equation wrap both lines.
- [A] Equation details: `/equation` alone → `$$\n\n$$` cursor at 3; after paragraph line `"Hello World\n/equation"` → `"Hello World\n\n$$\n\n$$"` (does not wrap previous paragraph), cursor inside; following soft line separated: `"…$$\n\ncontinued text"`.
- [A] Marker idempotence: wikilinks kept (`- [ ] [[linked note]]`); existing `- ` not stacked; `1. ` switched to `- `; `- [x]` preserved; `> ` not duplicated; heading level changed on existing ATX (`## Existing /h2` + Heading 3 → `### Existing`).
- [A] Code fence longer than any backtick run inside (```` ```` ````), cursor at end of content.
- [A] Headings `it.each ×3`: `"Before /head after"` → `"# Before after"` / `##` / `###`, cursor after `Before `; soft-break paragraph joined into one heading: `"First line /h1\nsecond line"` → `"# First line second line"`.
- [C] Insert-type commands (Table/Image/Video/Emoji) don't edit text: they dispatch `window` CustomEvent `frankmd:open-slash-command` with `detail {action:"table", from:7, to:13, query:"/table"}`, doc unchanged.
- [C] `renderSlashCommandIcon` → `<svg viewBox="0 0 256 256" aria-hidden="true">` with path; `null` for options without icon; menu items render `svg.frankmd-completion-icon` (14 items) when used inside `createWikilinkAutocomplete({additionalSources,addToOptions})`.
- [C] Keyboard in real EditorView: ArrowDown/ArrowUp moves selection, Enter on "Heading 2" on `/` → doc `"## "` and menu closed; Escape closes and keeps `/table`; Enter on Table closes completion synchronously before a dialog listener takes focus.

### `test/javascript/lib/codemirror_theme.test.js` — subject: `app/javascript/lib/codemirror_theme.js` (8 tests: A=0 B=0 C=8; P=0)
- [C] `createEditorTheme(opts?)` returns an extension with/without `fontFamily/fontSize/lineHeight` (×3); autocomplete list `ul` inside `.cm-tooltip-autocomplete` has `height:auto; max-height:16em` (×1).
- [C] `createSyntaxHighlighting()` defined (×1); `createTheme(opts)` returns exactly 2 extensions (editor theme + syntax highlighting) and accepts font options (×3).

### `test/javascript/lib/codemirror_typewriter.test.js` — subject: `app/javascript/lib/codemirror_typewriter.js` (28 tests: A=0 B=0 C=28; P=0) — 24 `it` + 2 `it.each` × 2 rows
Typewriter mode (keep cursor line vertically centered) and optional click-recentering, as CodeMirror state fields/effects.
- [C] `createTypewriterExtension(enabled=false, recenterOnClick=false)` returns array; `isTypewriterEnabled(view)` reflects init; `toggleTypewriter(view,bool)` flips it and adds/removes `typewriter-mode` class on `view.dom` (async) (×9 incl. isTypewriterEnabled ×2).
- [C] `getTypewriterSyncData(view)` → `{currentLine (1-based), totalLines}` (3 lines → 3; offset 10 → line 2) (×2).
- [C] `setTypewriterMode.of(bool)` effect and `typewriterState` field readable (×4); `cursorRecenterOnClickState` defaults false, toggled by `setCursorRecenterOnClick.of(true)` (×2).
- [C] Scroll math (geometry: scrollTop 25, cursorTop 300, clientHeight 100, scrollHeight 1000 → target scrollTop = cursorTop − clientHeight/2 = **275**): pointer click (`select.pointer`) not recentered when click-recentering off regardless of typewriter (it.each ×2, stays 25); recentered when on (it.each ×2, → 275); pointer *range* selection never recentered; typewriter keyboard selection and doc changes still recenter (→275) when click recentering off; click recenter clamped to `scrollHeight − clientHeight` (cursorTop 900, scrollHeight 500 → 400).
- [C] Padding: enabled → `scrollDOM.style.paddingBottom` set in px; disabled → removed; hidden editor (clientHeight 0) gets `0px`, then `refreshTypewriterLayout(view)` after becoming visible (clientHeight 400) → padding `200px` (= clientHeight/2) and scrollTop 400.

### `test/javascript/lib/custom_theme.test.js` — subject: `app/javascript/lib/custom_theme.js` (4 tests: A=4 B=0 C=0; P=0)
- [A] `normalizeColor`: trims and lowercases 6-digit hex (`" #AABBCC "`→`"#aabbcc"`); `null`, 3-digit `#abc`, named `red`, `url(...)`, `#ffffff;display:none` → `null`.
- [A] `applyCustomColors(map)` sets `--theme-<key>` on `<html>` only for keys in `COLOR_KEYS` (bg-primary…syntax-invalid, 38 keys) whose value normalizes (`accent:"#112233"` applied; `bg-primary:"url(secret)"` and unknown `font-family` ignored); `clearCustomColors()` removes all `--theme-*` color props but leaves others (`--editor-width: 72ch`).
- [A] `contrastRatio(a,b)` WCAG with linearized channels: white/black 21, same color 1, `#777777`/white ≈4.478, invalid → `null`.
- [A] `themeColor(computedStyles, key)` reads `--theme-<key>`, trims, expands minified `#fff`→`#ffffff` (base theme only; user input `#fff` still rejected by normalizeColor).

### `test/javascript/lib/draft_storage.test.js` — subject: `app/javascript/lib/draft_storage.js` (22 tests: A=22 B=0 C=0; P=0)
`DraftStorage(storageProvider)` = crash-recovery drafts in a localStorage-like store (tested with in-memory Map). Keys: `frankmd:draft:<path>`, `frankmd:draft-conflict:<path>…<conflictId>`, legacy `frankmd:backup:<path>`. All methods return `{ok,…}` and never throw (storage errors surface as `{ok:false,error}`).
- [A] `writeDraft(path, content, baseRevision)` → `{ok, draft:{schemaVersion:1, path, content, baseRevision, draftRevision (uuid), updatedAt (ms number)}}`; empty content is a valid draft; `readDraft(path)` → `{ok, draft}`. Re-writing identical content+base is skipped (`unchanged:true`, one setItem, same draftRevision).
- [A] `removeDraftIfRevision(path, rev)`: removes only when rev matches latest (`{removed:false}` for stale rev, `{removed:true}` then draft `null`).
- [A] group "remapDrafts(from,to,kind='file'|'folder') ×11": file rename moves draft preserving all metadata except `path` → `{ok,remapped:1}`; folder remap moves `docs/**` to `archive/**` by path-segment boundary (`docs-old/keep.md` untouched) → `remapped:2`; legacy backups remapped the same way (raw JSON copied byte-identical). Collisions: destination backup exists → `{ok:false, backupCollision:true, sourcePath, destinationPath}`, both kept; destination draft exists → `{ok:false, collision:true, conflictId}`; source draft moved into a **conflict copy** under destination (`listDraftConflicts("new.md").conflicts` contains `{conflictId, path:"new.md", sourcePath:"old.md", content, draftRevision}`), destination draft unchanged; in folder remap with one collision, *every* moved draft becomes a conflict copy (non-colliding `archive/index.md` has no draft but one conflict). Safety: destination write throws or silently fails (verify-after-write) → source retained, `{ok:false, sourcePath}`; removing verified source throws → both copies kept, later retry succeeds `{ok:true,remapped:1}`.
- [A] `removeDrafts(path, kind)` deletes drafts + backups + conflict copies under path boundary (`docs` folder → `removed:3`, `docs-old` kept); removal errors → `{ok:false, removed:0}` without throwing.
- [A] Malformed records: bad JSON → `readDraft` `{ok:true, draft:null, malformed:true}`; a record whose `path` ≠ key path is rejected; `listDrafts()` → `{ok:true,drafts:[]}`; `removeAllDrafts()` deletes only `frankmd:` keys (leaves `another-app:key`) incl. broken ones. Newer `schemaVersion:2` → `{ok:false, error:"Unsupported draft schema version…"}` and raw record preserved.
- [A] get/set/remove exceptions (e.g. `QuotaExceededError`) returned as `{ok:false, error}` identity; legacy `writeBackup/readBackup/removeBackup` still work (`backup.content`).

### `test/javascript/lib/find_utils.test.js` — subject: `app/javascript/lib/find_utils.js` (15 tests: A=15 B=0 C=0; P=0)
- [A] `escapeRegexChars("a+b*c?")`→`"a\\+b\\*c\\?"`; `""`→`""`.
- [A] group "findAllMatches(text, query, {caseSensitive, useRegex}) ×5" → `[{start, end, text, groups}]`: empty query → `[]`; default case-insensitive (`"Hello hello"`/`hello` → 2, first start 0); `caseSensitive:true` → 1; regex `c.t` → 3; capture `groups` `["abc","123"]`.
- [A] `replaceMatches(text, matches, replacement, {useRegex})`: literal (`"one two one"`→`"1 two 1"`); regex group refs `$2-$1` → `"123-abc"`.
- [A] group "findClosestMatchIndex(matches, pos, 'next'|'previous') ×4" on `"a b a b"`/`b` (starts 2,6): next from 0 → 0; past end wraps → 0; previous from 5 → 0; previous from 0 wraps → last (1).
- [A] `validateRegex("abc").valid` true; `"["` → `{valid:false, error:<msg>}`.

### `test/javascript/lib/floating_dropdown.test.js` — subject: `app/javascript/lib/floating_dropdown.js` (9 tests: A=0 B=0 C=9; P=0)
Accessible popover menu (`new FloatingDropdown(menu, trigger)`), viewport 400×600.
- [C] sets `popovertarget` on trigger; trigger click default-prevented; `open()` focuses `aria-checked="true"` option, `aria-expanded="true"`; ArrowUp skips `disabled` options; End/Home jump to last/first; Escape (prevented) closes, adds `hidden`, refocuses trigger, enclosing `<dialog>` stays open.
- [C] positioning: near viewport bottom flips above (`top 226px`), `left` clamped to `200px`, `maxHeight 320px`, `maxWidth 384px` (viewport − 2×8 gutter); short viewport (180) → `maxHeight 58px`, no forced minimum.
- [C] resize: trigger moved offscreen → closes (`aria-expanded="false"`); resize shrinking menu (maxHeight `128px`) scrolls to keep focused option visible (`scrollTop 192`).
- [C] menu's own scroll keeps it open; container scroll keeps it open unless trigger moves out of view → close; focus moving outside closes.
- [C] Tab closes, focus returns to trigger and Tab is *not* prevented; options get `tabIndex -1` while open.
- [C] native popover: `showPopover()` on open; `beforetoggle` with `newState:"closed"` syncs `aria-expanded=false`; parent dialog `close` → `hidePopover()` + hidden.
- [C] document `frankmd:dropdown-open` from another dropdown closes this one; `disconnect()` removes listeners.

### `test/javascript/lib/html_sanitizer.test.js` — subject: `app/javascript/lib/html_sanitizer.js` (DOMPurify config) (23 tests: A=23 B=0 C=0; P=0)
`sanitizeHtml(html)` for the markdown preview (string → string; Rust port: ammonia-equivalent policy).
- [A] group "task checkboxes ×6": `<input type="checkbox">` kept inside list items **with `disabled` stripped** (preview toggles tasks), `checked` preserved; only `type`/`checked` attributes allowed (name/value/class/onclick removed); non-checkbox inputs (text, submit, bare `<input>`) removed; `form`, `button`, `textarea`, `select` removed.
- [A] group "script vectors ×5": `<script>` removed (text kept), `on*` handlers removed (img/div/svg), `style=` attrs and `<style>` removed, `javascript:` hrefs removed, iframe `srcdoc` removed.
- [A] group "keeps app output ×5": YouTube embed `<div class="embed-container"><iframe src="https://www.youtube.com/embed/…" … allowfullscreen>` kept; `<video controls class="video-player"><source src="/notes/videos/a.mp4">` kept; `data-source-line` kept; wikilink `<a class="wikilink" data-wikilink-path="Note.md" data-action="click->app#openWikilink">` kept; h1/pre/code `language-js`/table kept.
- [A] group "iframe allow-list ×5": only `https://` hosts `www.youtube.com`, `www.youtube-nocookie.com`, `player.vimeo.com`; drops other hosts, `http:`, missing src, userinfo trick `youtube.com@evil.com`, suffix trick `youtube.com.evil.com`, protocol-relative `//evil.com`.
- [A] form/input/button stripped from `<form action=evil>` (×1); `""`/null/undefined → `""` (×1).

### `test/javascript/lib/icon_data.test.js` — subject: `app/javascript/lib/icon_data.js` (4 tests: A=4 B=0 C=0; P=0)
- [A] `ICON_DATA`: ≥190 tuples `[name, svgPath, "0 0 256 256", keywords]` (all strings, non-empty name/path); names unique.
- [A] `getIconMap()` → cached same object, `{name: {path, viewBox}}`, size == ICON_DATA.length; includes `rocket, git-branch, text-b, paperclip, seal-check, play, heart` (used by `:ph-<name>:` markdown icon syntax).

### `test/javascript/lib/keyboard_shortcuts.test.js` — subject: `app/javascript/lib/keyboard_shortcuts.js` (41 tests: A=41 B=0 C=0; P=0)
Default table (`SHORTCUTS === DEFAULT_SHORTCUTS`): newNote Ctrl+n, save Ctrl+s, togglePreview Ctrl+Shift+V, findInFile Ctrl+f, findReplace Ctrl+h, jumpToLine Ctrl+g, lineNumbers Ctrl+l, contentSearch Ctrl+Shift+F, fileFinder Ctrl+p, toggleSidebar Ctrl+e, typewriterMode Ctrl+`\`, toggleTask Ctrl+Enter, toggleScrollSync Ctrl+Shift+`\`, textFormat Ctrl+m, emojiPicker Ctrl+Shift+E, increaseWidth Ctrl+Shift+`+`, decreaseWidth Ctrl+Shift+`_`, logViewer Ctrl+Shift+O, help F1, closeDialogs `{key:"Escape", preventDefault:false}`. (Keys are compared against `event.key` exactly, so shifted letters are uppercase.)
- [A] SHORTCUTS definitions (×1) and DEFAULT_SHORTCUTS alias/closeDialogs/toggleScrollSync vs typewriterMode/toggleTask (×4); every chord `(key,ctrl,shift,alt)` unique (×1).
- [A] group "matchShortcut ×11": exact `event.key` match; `ctrl` satisfied by ctrlKey **or** metaKey; extra or missing ctrl/shift/alt rejects (F1 with Ctrl rejected); null/undefined/keyless shortcut → false.
- [A] group "formatShortcut ×6": `"Ctrl+S"`, `"Ctrl+N"`, `"Ctrl+Shift+V"`, `"Ctrl+Shift+F"`, `"F1"`, `"Ctrl+\\"`; mac style `"⌘S"`, `"⌘⇧V"`; null/undefined/`{}` → `""`. `isMacOS()` returns boolean (×1).
- [A] group "findMatchingAction ×6": returns action name (`save`, `togglePreview`, `toggleScrollSync` for Ctrl+Shift+\ vs `typewriterMode` for Ctrl+\, `toggleTask`, `closeDialogs`) or `null` (plain Enter → null).
- [A] group "createKeyHandler ×4": calls `handler(action, event)` on match and `preventDefault()` unless shortcut has `preventDefault:false` (Escape); no match → neither.
- [A] group "mergeShortcuts(defaults, user) ×5": user overrides; `null` value removes action; new actions added; defaults not mutated. `getShortcutForAction` returns def or `null` (×2).

### `test/javascript/lib/line_numbers.test.js` — subject: `app/javascript/lib/line_numbers.js` (10 tests: A=10 B=0 C=0; P=0)
- [A] `LINE_NUMBER_MODES` OFF=0, ABSOLUTE=1, RELATIVE=2 (×3).
- [A] `normalizeLineNumberMode(value, fallback)`: undefined → fallback; `"1"` → 1; `2` → 2; invalid (`99`, `"invalid"`) → fallback (×4).
- [A] `nextLineNumberMode` cycles OFF→ABSOLUTE→RELATIVE→OFF (×3).

### `test/javascript/lib/markdown_line_mapper.test.js` — subject: `app/javascript/lib/markdown_line_mapper.js` (14 tests: A=9 B=0 C=5; P=0)
Editor↔preview scroll sync: rendered block elements get `data-source-line` (1-based source line of the token start, computed by summing newlines in `token.raw`).
- [A] `parseWithLineNumbers(md, frontmatterLineOffset=0)`: `"# First\n\n# Second\n\n"` → lines 1 and 3; offset 4 → 5 and 7; fenced code `<pre data-source-line="1">` survives later `highlightCodeBlocks`; display math (`mathBlock` token) `<div data-source-line="3" class="math-block">` keeps following paragraph at line 7; **each `<li>` annotated** with its own line incl. nested items (`- [ ] a`=3, `- plain`=4, `- parent`=5, nested `- [x] child`=6), shifted by frontmatter offset (3 → 4..7); `""` → `""` (×7).
- [A] `findElementByLine(container, line)`: element with greatest `data-source-line` ≤ line (line 7 → element 5); none annotated → `null` (×2).
- [C] `findLineAtScroll(container, scrollTop)` uses `getBoundingClientRect` relative to the container (invariant to page offset, accounts for container `scrollTop`): element at/above position (150→5, 0→1, 999→10); above all elements → first element's line (3); no annotations → `null` (×5).

### `test/javascript/lib/marked_extensions.test.js` — subject: `app/javascript/lib/marked_extensions.js` (44 tests: A=44 B=0 C=0; P=0)
Custom marked inline/block extensions (each with `start(src)`, `tokenizer(src)`, `renderer(token)`).
- [A] group "wikilink ×14": `start` = index of `[[` or -1; tokenizer on `[[target]]` / `[[target|display]]` → `{type:"wikilink", raw, target, displayText|null}`, trims both parts, supports `folder/Note`; non-match/unclosed → `undefined`. Renderer: `<a class="wikilink" data-wikilink-path="<target>" data-action="click->app#openWikilink">LABEL</a>`, label = displayText or **basename** of target (`projects/My Note` → `My Note`); HTML-escapes `<`, `&` (`Notes &amp; Ideas`), `"` (`&quot;`) in both attribute and text.
- [A] group "icon `:ph-name:` ×7": `start` index of `:ph-` or -1 (plain `:smile:` → -1); tokenizer only for names in icon map → `{type:"icon", raw:":ph-heart:", path, viewBox:"0 0 256 256"}`, unknown/plain emoji/non-match → undefined; renderer `<svg class="frankmd-inline-icon" viewBox=… fill="currentColor" width="1em" height="1em"><path d="…"/></svg>` with no `style=`.
- [A] group "mathInline `$…$` ×15": `start` = first unescaped `$` (`\$5` → undefined; none → undefined). Tokenizer → `{type:"mathInline", raw, text}`: `$E = mc^2$`, `$x$`; rejects currency (`$10.`, `$10 and that costs $20.`, closing `$` followed by digit `$100 to $200`), space after opening or before closing `$`, `$$` delimiter; stops at first valid close (`$a$ and $b$` → `$a$`); keeps escaped `\$` inside (`a \$ b`). Renderer → exactly `<span class="math-inline">TEX</span>` HTML-escaped, no style.
- [A] group "mathBlock `$$…$$` ×8": `start` index of `$$` or undefined; multiline `$$\n…\n$$` and single-line `$$E=mc^2$$` → text trimmed; whitespace-only block and unterminated block → undefined; renderer → exactly `<div class="math-block">TEX</div>\n`, escaped.

### `test/javascript/lib/math_renderer.test.js` — subject: `app/javascript/lib/math_renderer.js` (7 tests: A=7 B=0 C=0; P=0)
- [A] `renderMathIn(container)` renders every `.math-inline` (displayMode false) and `.math-block` (displayMode true) with the node's text as TeX, options always `{trust:false, throwOnError:false, maxExpand:1000}`; marks node `data-math-rendered=""`; idempotent (second call no re-render); a throwing node gets class `math-error`, is still marked, and rendering continues; `null` container no-op. (KaTeX mocked.)

### `test/javascript/lib/menu_bounds.test.js` — subject: `app/javascript/lib/menu_bounds.js` (9 tests: A=3 B=0 C=6; P=0) — includes `it.each` with 2 rows
- [A] `menuMaxHeight(rect, viewportH, padding=8)` = `viewportH − top − padding` px (`900−400−8`), minimum `96px` near bottom, custom padding (`top 100, pad 20` → `780px`) (×3).
- [C] `clampMenuToViewport(menu, vh, vw)`: sets inline `maxHeight` (`top 300`→`592px`); null-safe; horizontally overhanging menu in 320px viewport shifted with `translateX` (left 177/right 449 → `translateX(-137px)`; left −20/right 252 → `translateX(28px)`) (it.each ×2); resets old transform before measuring (×5 total).
- [C] `toggleMenu(menu, forceHide?)`: shows hidden `.frankmd-menu` and applies clamp (returns true); hides visible one (returns false) (×2).

### `test/javascript/lib/new_note_utils.test.js` — subject: `app/javascript/lib/new_note_utils.js` (29 tests: A=29 B=0 C=0; P=0)
Client-side preview of where the "New note" dialog will create a file (must agree with server HugoService slugs).
- [A] group "slugify ×6": lowercase, non-alphanumeric runs → single `-`, trim `-` (`"Hello, World! (v2)"`→`hello-world-v2`, `--hello--`→`hello`); transliterate accents (`Olá, você está bem?`→`ola-voce-esta-bem`, `Grüße aus München`→`grusse-aus-munchen` — ß→ss); `???`/`""`→`""`; `.md` not special (`my-post.md`→`my-post-md`).
- [A] group "hugoDatePath ×2": `YYYY/MM/DD` zero-padded local date (`2026/08/18`, `2026/01/03`), default now.
- [A] group "isValidNoteName ×4": non-blank, no `/` anywhere, not `.`/`..`; `"my note.md"` ok; null → false.
- [A] group "canCreateNote(name, template) ×4": valid name required; for `hugo` the slug must be non-empty (`!!!` blocked; `My Post.md` ok).
- [A] group "previewNotePath({name, parent, template, date, rootSegment='root', namePlaceholder, pathStyle}) ×13" → `{valid, empty, path}`: empty template `root/my-note.md` (`.md` appended once; name trimmed); parent → `docs/guides/my-note.md`; localized root segment `raiz/x.md`; blank name → `{valid:false, empty:true, path:"root/<placeholder>.md"}`; `a/b` → invalid, path `root/a/b.md`. Hugo: page bundle `root/2026/08/18/my-post/index.md` (slugified, `.md` stripped: `Olá Mundo.md`→`ola-mundo`), inside parent `blog/2026/08/18/my-post/index.md`; `pathStyle:"flat"` + parent `blog` → `blog/my-post.md`; empty slug (`!!!`) → invalid with raw name in path `root/2026/08/18/!!!/index.md`; blank → placeholder slot, `empty:true`.

### `test/javascript/lib/s3_key.test.js` — subject: `app/javascript/lib/s3_key.js` (3 tests: A=3 B=0 C=0; P=0)
- [A] `defaultS3Prefix(date)` = `frankmd/YYYY/MM` (zero-padded month, `2026-07-09` → `frankmd/2026/07`); `defaultS3Key(filename, date)` = prefix + `/` + filename; both default to now.

### `test/javascript/lib/scroll_utils.test.js` — subject: `app/javascript/lib/scroll_utils.js` (32 tests: A=32 B=0 C=0; P=0)
- [A] group "calculateScrollRatio(scrollTop, scrollHeight, clientHeight) ×5": `scrollTop/(scrollHeight−clientHeight)` clamped [0,1]; 0 when content fits.
- [A] group "calculateScrollForLine(line, lineHeight, viewportH, offsetRatio=0.35) ×4": `max(0, (line−1)*lineHeight − viewportH*ratio)` (line 50 → 805; ratio 0.5 → 730).
- [A] group "calculateLineFromScroll(scrollTop, clientH, scrollH, totalLines) ×5": `round((scrollTop + clientH/2)/(scrollH/totalLines))` clamped to [1,totalLines] (0→5, 900→50, 1800→95); 0 lines → 1.
- [A] group "calculateSyncedScrollPosition(ratio, scrollH, clientH) ×4": `ratio*(scrollH−clientH)`, 0 if fits.
- [A] group "calculateScrollToCenterLine(line, totalLines, scrollH, clientH) ×5": `(line−1)/(total−1)*maxScroll` (1→0, last→1800, 50/100 ≈891); 1 or 0 lines → 0.
- [A] group "scrollTopForElement(elRect, containerRect, containerScrollTop) ×4": `el.top − container.top + scrollTop`, page-offset invariant, clamped ≥0 (`{-50},{50},100` → 0).
- [A] group "lineAtScroll(entries[{line,top}], scrollTop) ×5": last entry with `top ≤ scrollTop`; above all → first entry's line; tie keeps earlier entry; empty/null → `null`.

### `test/javascript/lib/task_utils.test.js` — subject: `app/javascript/lib/task_utils.js` (11 tests: A=11 B=0 C=0; P=0)
- [A] group "toggleTaskMarker(line) ×7": toggles GFM task marker `[ ]`↔`[x]` after bullets `-`,`*`,`+` or ordered `3.`/`1)`; uppercase `[X]` unchecks; end-of-line `- [x]` → `- [ ]`; indentation preserved; returns `null` for non-tasks (`plain text`, `- no marker`, `- [y] not gfm`, `[ ] no bullet`, `""`).
- [A] group "applyTaskToggle(line) ×4" (Ctrl+Enter): toggles existing task; plain line → `- [ ] <text>` keeping indentation (`"  indented"`→`"  - [ ] indented"`); empty → `"- [ ]"`; whitespace-only `"   "` → `"   - [ ]"`.

### `test/javascript/lib/text_editor_utils.test.js` — subject: `app/javascript/lib/text_editor_utils.js` (28 tests: A=28 B=0 C=0; P=0)
- [A] group "wrapSelection(text, start, end, prefix, suffix) ×4" → `{text, cursorStart, cursorEnd}` selecting inner text (`hello **world**` 8..13; empty selection `hel[]lo` cursor 4; multiline with ```` ```\n ```` / ```` \n``` ````).
- [A] group "insertAtPosition ×4" → `{text, cursorPosition = pos+len}`.
- [A] group "getLineBoundaries(text,start,end) ×4" → `{lineStart, lineEnd, lines[]}` covering whole lines of the selection (`"line1\nline2\nline3\nline4"`, 7..14 → 6..17, `["line2","line3"]`).
- [A] group "getCursorInfo(text,pos) ×5" → `{line, column (1-based), totalLines}`; end of line1 → column 6; `""` → line 1, total 1.
- [A] group "getLineAtPosition ×3" (1-based) and "getPositionForLine ×4" (line 1→0, 2→6, 3→12, beyond end → `text.length+1`).
- [A] group "replaceRange(text, from, to, repl) ×4" → `{text, cursorPosition = from+len(repl)}`; empty repl deletes.

### `test/javascript/lib/theme_syntax_palette.test.js` — subject: `app/assets/tailwind/themes/*.css`, `app/assets/tailwind/components/prose.css` (20 tests at runtime: A=20 B=0 C=0; P=0) — 2 static `it` + 1 generated per theme file (18)
- [A] exactly 18 built-in theme files: catppuccin-latte, catppuccin, dark, ethereal, everforest, flexoki-light, gruvbox, hackerman, kanagawa, light, matte-black, nord, osaka-jade, ristretto, rose-pine, solarized-dark, solarized-light, tokyo-night.
- [A] per theme (×18): defines `--theme-code-bg` (6-digit hex) and `--theme-syntax-{keyword,function,type,string,number,constant,identifier,property,operator,comment,punctuation,invalid}` each ≥ 4.5:1 WCAG contrast vs code-bg.
- [A] prose.css references every `var(--theme-syntax-<role>, …)` and never falls back to heading-1..3 or accent colors.

### `test/javascript/lib/typewriter_utils.test.js` — subject: `app/javascript/lib/typewriter_utils.js` (17 tests: A=17 B=0 C=0; P=0)
- [A] group "calculateTypewriterScroll(cursorY, viewportH, ratio=0.5) ×5": `max(0, cursorY − viewportH*ratio)` (500,400 → 300; ratio 0.3 → 380).
- [A] group "getTypewriterSyncData(text, pos) ×6" → `{currentLine, totalLines}` (1-based; pos right after `\n` is next line; `""` → 1/1).
- [A] group "calculateTypewriterSyncRatio(line, total) ×6": `(line−1)/(total−1)` clamped [0,1]; single-line → 0 (50/100 ≈ 0.495).

### `test/javascript/lib/vim_mode.test.js` — subject: `app/javascript/lib/vim_mode.js` (11 tests: A=3 B=0 C=8; P=0)
- [C] `configureVimKeys()` unmaps `APP_OWNED_KEYS` = `<C-f> <C-n> <C-p> <C-e> <C-b> <C-i> <C-v>` from vim (returns true) on non-mac; on macOS (app uses Cmd) returns false and unmaps nothing; only once; `<C-d>`, `<C-q>` stay with vim (×3).
- [C] `registerVimCommands({onCommand})` defines one ex-command per `EX_COMMANDS` entry `[name, prefix, action]`: write/w→`save`, quit/q→`close`, wq & xit/x→`save-and-close`, help→`help`, edit/e & find→`finder`, Explore→`toggle-sidebar`, next/n→`next-note`, prev→`prev-note`; `:nohlsearch` not overridden; running calls `onCommand(action, params)`; idempotent (×3).
- [C] `observeVimMode(view,{onModeChange})` maps `vim-mode-change` to `"normal"`, `"insert"`, `"visual-<subMode>"` (e.g. `visual-linewise`); events without mode ignored (×2).
- [A] `nextNoteIndex(count, current, dir)`: wraps forward/backward (3,2,+1→0; 3,0,−1→2); nothing selected (−1) → first (fwd) or last (back); count 0 → −1 (×3).

### `test/javascript/image_sources/ai_images.test.js` — subject: `app/javascript/lib/image_sources/ai_images.js` (34 tests: A=11 B=19 C=4; P=0)
`AiImageSource` — AI image generation tab of the image picker; talks to backend endpoints (which proxy to Gemini/Imagen). `fetch` mocked.
- [A] initial state: `enabled:false, model:null, generatedData:null, abortController:null, refImage:null, refImageSearchTimeout:null`; `reset()` clears generatedData/refImage, aborts pending request and nulls controller, clears ref search timeout (×4).
- [B] `loadConfig()`: `GET /ai/image_config` (`Accept: application/json`) → sets `enabled`, `model` from `{enabled, model}` and returns it; non-ok or network error → `enabled` stays false (×3).
- [A] `generate("")` → `{error:"Please enter a prompt describing the image you want to generate"}`; not enabled → `{error:"AI image generation is not configured. Please add your Gemini API key to .fed"}` (×2).
- [B] `generate(prompt)`: `POST /ai/generate_image` JSON `{prompt}` (+ `reference_image_path` when refImage set) with AbortSignal → `{success:true, imageData, previewSrc:"data:<mime>;base64,<data>"}` or `previewSrc = url` for URL responses; server `{error}` returned as `{error}`; `AbortError` → `{cancelled:true}`; other failures → `{error:"Failed to generate image. Please try again."}`; abortController nulled after completion (×7).
- [A] `abort()` calls controller.abort, safe with none; `setRefImage(path,name)` → `{path,name}`; `clearRefImage()` (×4).
- [B] `loadRefImages(search?)`: `GET /images` or `/images?search=cat` → array; non-ok → `{error:"Error loading images"}` (×3).
- [C] `renderRefImageGrid(images, container, action)`: empty → "No images found"; buttons with `data-path`, `data-name`, `data-action`; selected one gets `ring-2 ring-[var(--theme-accent)]`; names HTML-escaped (×4).
- [A] `save()` without generatedData → `{error:"No generated image data available"}` (×1).
- [B] `save()`: base64 → `POST /images/upload_base64` body containing `"data":…` (no `upload_to_s3` flag — destination chosen server-side by active provider) → `{url}`; URL result → `POST /images/import` `{url}` → `{url}`; non-ok → rejects with server `error` message ("Disk full", "S3 error") (×6).

### `test/javascript/image_sources/drop_images.test.js` — subject: `app/javascript/lib/image_sources/folder_images.js` (`ingest`, `extOf`) (6 tests: A=6 B=0 C=0; P=0)
- [A] `FolderImageSource.ingest(files, allowedExts)` (drag-and-drop tab) → `{count, rejected:[{name, ext}]}`: accepts by lowercase extension in list (default `DEFAULT_IMAGE_EXTENSIONS = [.jpg,.jpeg,.png,.gif,.webp,.bmp]`); `notes.txt` rejected with `ext ".txt"`; no extension → `ext "no extension"`; custom config list `[".webp"]` rejects png (×4).
- [A] `extOf("video.final.MP4")` → `".mp4"` (last segment, lowercased); `extOf("README")` → `"no extension"` (×2).

### `test/javascript/image_sources/folder_images.test.js` — subject: `app/javascript/lib/image_sources/folder_images.js` (32 tests: A=14 B=3 C=15; P=0)
`FolderImageSource` — browse a local folder via File System Access API, preview with object URLs, upload chosen file.
- [A] constructor: `displayedImages=[]`, `allImages=[]`; `reset()` calls `cleanup()`; `cleanup()` revokes every non-null `objectUrl` and clears both arrays (×4).
- [C] `isSupported` ⇔ `window.showDirectoryPicker` exists (×2); `browse()`: unsupported → `{error:"File System Access API not supported"}`, picker `AbortError` → `{cancelled:true}`, other error → `{error:"Error accessing folder"}` (×3).
- [A] group "filter(term, {offset=0, limit=10}) ×8" → `{displayed, total}`: case-insensitive name substring; default page size 10 (15 → 10/15); offset/limit paging (page 2 → 5, names `image10..14`); offset past end → 0; search+page combined; revokes old object URLs and creates new ones per displayed image (images sorted by `lastModified` desc in source).
- [C] `getImageDimensions(url)` → `{width,height}` from `Image.naturalWidth/Height`, `{null,null}` on error (×2).
- [C] group "renderGrid(container, statusEl, action, total?) ×6": empty → "No images found in folder"; items with `data-index`, `data-action`, `src=objectUrl`, `"800x600"`, size `"100 KB"`; status `"2 images found"` or `"Showing 1 most recent of 50 images"`; name HTML-escaped; quoted filename cannot inject `onerror` attribute.
- [A] `getImage(i)` returns displayed image or undefined (×2).
- [C] `deselectAll(container)` removes `selected` from `.image-grid-item`; null-safe (×2).
- [B] `upload(file, resize)` → `POST /images/upload` with FormData (`file`, optional `resize`, `s3_key`) → `{url}`; non-ok → throws server `error` ("File too large") or `"Upload failed"` (×3).

### `test/javascript/image_sources/google_images.test.js` — subject: `app/javascript/lib/image_sources/google_images.js` (21 tests: A=8 B=7 C=6; P=0)
`GoogleImageSource` — Google image search tab (paged, infinite scroll) via backend proxy.
- [A] initial `{enabled:false, results:[], nextStart:0, loading:false, query:""}`; `reset()` clears all but `enabled` (×2).
- [A] `search("")` → `{error:"Please enter search keywords"}` (×1); `loadMore()` while `loading` returns current results without fetching (×1).
- [B] `search(q)` resets state, stores query, calls loadMore (nextStart from response `next_start`) (×1); `loadMore()` → `GET /images/search_google?q=cats&start=1` → `{images, total}`, appends to `results`, updates `nextStart`; server `{error}` → `{error}` and `loading=false`; network error → `{error:"Search failed. Please try again."}` + `console.error("Google search error:", err)`; empty → `message "No images found"`; else `message "Found 150 images - click to select"` (uses `total`) (×6).
- [A] `shouldLoadMore(scrollBottom)` ⇔ `scrollBottom < 100 && !loading && results.length > 0` (×4).
- [C] `renderGrid(container, action)`: empty → "No images found"; buttons with `data-url`, `data-thumbnail`, `data-title`, `data-source`, `data-action`, `"1024x768"`; broken thumbnail removed via JS `error` listener (no inline `onerror`, removes `.external-image-item`); titles escaped (×4).
- [C] `deselectAll` removes `ring-2 ring-blue-500`; null-safe (×2).

### `test/javascript/image_sources/image_picker_text.test.js` — subject: `app/javascript/lib/image_sources/image_picker_text.js` (3 tests: A=3 B=0 C=0; P=0)
- [A] `imagePickerText(key, vars, fallback)` uses `window.t("dialogs.image_picker.<key>", vars)` when loaded; otherwise interpolates `%{name}` placeholders in the English fallback (`"Found %{count} images"` → `"Found 3 images"`).
- [A] `imagePickerSearchResultMessage(n)` → key `search_result_one` for 1, `search_result_many` otherwise, with `{count}`.

### `test/javascript/image_sources/local_images.test.js` — subject: `app/javascript/lib/image_sources/local_images.js` (24 tests: A=2 B=12 C=10; P=0)
`LocalImageSource` — images already in the configured images folder (server-listed).
- [A] `selectedPath` initially `null`; `reset()` clears it (×2).
- [B] group "load(search='', {limit=10, offset=0}) ×6": `GET /images?limit=10&offset=0`; with search `/images?search=sunset&limit=10&offset=0`; search `encodeURIComponent`d (`cats%20%26%20dogs`); custom `limit=25&offset=25`; returns server `{images,total,limit,offset}`; non-ok or network error → `{error:"Error loading images"}`.
- [C] group "renderGrid(images, container, action) ×8": empty/null → "No images found" with `image-grid-empty`; items `class="image-grid-item"` (+ ` selected` when `path===selectedPath`) with `data-path`, `data-name`, `data-action`, `"800x600"` only when dimensions known (`image-dimensions` omitted otherwise), `title=name`; thumbnail `src` = `/images/preview/` + per-segment-encoded path (`photos/my%20cat.jpg`); names escaped; quoted filename can't inject `onerror`.
- [C] `deselectAll` removes `selected` from `.image-grid-item`; null-safe (×2).
- [B] `saveCopy(path, resize)` → `POST /images/import` JSON `{path, resize}` → `{url}`; non-ok → throws server error ("S3 bucket not configured") or `"Failed to upload to S3"` (×3).
- [B] `deleteImage(path)` → `DELETE /images/file/<encoded path>` → `{success:true}`; non-ok → throws server error ("Image not found or could not be deleted") or `"Failed to delete image"` (×3).

### `test/javascript/image_sources/pinterest_images.test.js` — subject: `app/javascript/lib/image_sources/pinterest_images.js` (15 tests: A=4 B=4 C=7; P=0)
- [A] `results=[]` initially; `reset()` clears; `search("")`/`search(null)` → `{error:"Please enter search keywords"}` (×4).
- [B] `search(q)` → `GET /images/search_pinterest?q=home%20decor` → `{images, message:"Found 2 images - click to select"}` and stores results; server `{error}` → `{error}`, results `[]`; empty → `"No images found"`; network → `{error:"Search failed. Please try again."}` + `console.error("Pinterest search error:", err)` (×4).
- [C] `renderGrid`: empty state; `data-url/thumbnail/title/source`, `"600x800"`; broken thumbnail removed via listener (no inline onerror); missing fields → `data-title=""`, `data-source=""`; titles escaped `&lt;script&gt;` (×5). `deselectAll` on `.external-image-item` removes `ring-2 ring-blue-500`; null-safe (×2).

### `test/javascript/image_sources/web_images.test.js` — subject: `app/javascript/lib/image_sources/web_images.js` (24 tests: A=4 B=10 C=10; P=0)
- [A] `results=[]`; `reset()`; `search("")`/`search(null)` → `{error:"Please enter search keywords"}` (×4).
- [B] `search(q)` → `GET /images/search_web?q=cats` (query `encodeURIComponent`d: `cats%20%26%20dogs`) → `{images, message:"Found N images - click to select"}`; server `{error}` passed through verbatim (also when localized); empty → `"No images found"` or server `note` ("Try different keywords"); localized via `window.t` keys `dialogs.image_picker.{search_keywords_required,no_images_found,search_result_many}`; network → `"Search failed. Please try again."` (×7).
- [C] `renderGrid`: empty state; `data-url/title/action/source`; dimensions `800x600`; `src`/`data-thumbnail` use `thumbnail` when present; broken thumbnails removed via listener; titles escaped; quoted title can't inject attrs; unsafe image URL (`javascript:`) rendered as `src=""` (uses `isSafeImageUrl`) (×8). `deselectAll` (×2).
- [B] `saveCopy(url, resize)` → `POST /images/import` JSON `{url, resize}` → `{url}`; non-ok → throws server error ("Image too large") or `"Failed to upload to S3"` (×3).

## Porting notes (JS libs)
- **Two HTML escapers with different apostrophe entities**: `escapeHtml` → `&#39;`, `escapeHtmlString` → `&#039;`. Any golden comparison of rendered HTML must pick the right one per call site.
- `isSafeImageUrl` rejects leading whitespace/newlines and `//`/`/\` protocol-relative forms; only `http(s):`, `data:image/`, `blob:` and root-relative paths pass. The preview sanitizer iframe allow-list is exact-host https only (`www.youtube.com`, `www.youtube-nocookie.com`, `player.vimeo.com`), and task-list checkboxes are kept *enabled* (disabled attr stripped) so preview clicks toggle tasks.
- Slugs: JS `slugify` must match server `HugoService` (accent transliteration, `ß`→`ss`, `.md` not stripped by slugify itself). New-note path preview: Hugo page bundle `<parent|root>/YYYY/MM/DD/<slug>/index.md`, or `<parent>/<slug>.md` with `pathStyle:"flat"`; localized root segment only for display.
- `parseIndentSetting`: `0` → tab, 1..6 spaces, >6 clamps to 6, negative/non-numeric → 2 spaces. `LINE_NUMBER_MODES` 0/1/2 are persisted values; invalid → fallback.
- Editor block insertion rule: ensure exactly one blank line around inserted blocks (prefix/suffix of 0/1/2 `\n` depending on neighbors); images use single `\n` padding; slash-command replacement swallows one adjacent space on each side (range from−1..to+1). Code-block cursor = start + 3 + len(lang) + 1.
- Slash commands: 14 commands in fixed order; never trigger in code, link/image destinations, mid-path slashes, non-markdown files or Vim Normal mode; all block transforms are one undo step; list continuation indents = marker width (2/3/6); code fence grows beyond inner backtick runs; insert-type commands emit `frankmd:open-slash-command` `{action, from, to, query}`.
- Markdown surround (`* _ \` ~ ^ =` on selection) works on multiple/backward ranges, uses longer backtick fences + padding for content containing backticks, and is skipped for paste/IME/read-only.
- Keyboard: `ctrl` in shortcut definitions matches Ctrl **or** Meta; matching is on exact `event.key` (so `Ctrl+Shift+V` uses `"V"`). User overrides merge per action, `null` removes. Vim gives back `<C-f n p e b i v>` on non-mac only; ex commands map to logical actions (`save`, `close`, `save-and-close`, `finder`, `toggle-sidebar`, `next-note`, `prev-note`, `help`).
- Drafts (crash recovery): localStorage keys `frankmd:draft:<path>`, `frankmd:draft-conflict:…`, legacy `frankmd:backup:<path>`; schemaVersion 1 (newer versions preserved untouched, reported as error); rename/move remaps by **path-segment boundary** with verify-before-delete and conflict copies on collision. A native port should keep these semantics (even if storage moves to disk).
- Scroll sync: `data-source-line` 1-based + frontmatter offset, annotated per `<li>` and math blocks; geometry helpers must be page-offset invariant. Typewriter target = cursor centered (ratio 0.5), padding-bottom = clientHeight/2, click-recentering off by default.
- Syntax highlighting token roles (`tok-keyword`, `tok-function tok-definition`, …) and the per-render budgets (single-fence size cap ~1000 lines, total-source budget across blocks, max 100 blocks) and `data-code-language` labels (incl. aliases `yml`, `golang`, `py`, `kt`, `rb`, `env`, `sh`, `docker`, `tf`/`hcl`, `cs`, extension-style `.env`, `.yaml`…) are what the preview CSS and 18 theme palettes (≥4.5:1 contrast per role) are built around.
- Image picker endpoints relied on by JS (B tests): `GET /ai/image_config`, `POST /ai/generate_image {prompt, reference_image_path?}`, `GET /images?search=&limit=10&offset=0`, `GET /images/preview/<path>`, `POST /images/upload` (multipart `file`, `resize`, `s3_key`), `POST /images/upload_base64`, `POST /images/import {path|url, resize}`, `DELETE /images/file/<path>`, `GET /images/search_google?q=&start=`, `/images/search_pinterest?q=`, `/images/search_web?q=`; error strings ("Error loading images", "Search failed. Please try again.", "Failed to upload to S3", "Failed to delete image", "Upload failed", Gemini-not-configured message mentioning `.fed`). Upload destination (local vs S3) is decided server-side — the client never sends `upload_to_s3`.
- Default drop/folder image extension allow-list: `.jpg .jpeg .png .gif .webp .bmp` (config-overridable); pasted images get `pasted-image-*.<mime subtype>` names; default S3 key `frankmd/YYYY/MM/<filename>`.
- `formatFileSize`: one decimal below 10 units, none at ≥10 (`10 KB`); read time `ceil(words/200)`, shown as `< 1 min` for ≤1.


---

<!-- part totals: files=25 tests=699 A=77 B=9 C=613 P=0 -->

## JS controllers (a–f): `test/javascript/controllers/ai_grammar_controller.test.js` … `file_operations_controller.test.js`

### `test/javascript/controllers/ai_grammar_controller.test.js` — subject: `app/javascript/controllers/ai_grammar_controller.js` (28 tests: A=9 B=7 C=12; P=0)
- [C] connect: initial state `aiEnabled=false`, `aiProvider=null`, `aiModel=null`; connect() calls `checkAiAvailability` (×2).
- [B] checkAiAvailability → `GET /ai/config` (JSON): `{enabled:true,provider:"openai",model:"gpt-4"}` sets enabled/provider/model; `{enabled:false}` → disabled; network error → all reset to false/null (×3).
- [C] open(path) when AI disabled: shows `configNotice` (removes `hidden`), keeps `diffContent` hidden, still opens dialog (showModal).
- [C] open(null) with AI enabled → shared app alert `appAlert("errors.no_file_open")` (translation key).
- [B] open(path) enabled → `POST /ai/fix_grammar` body `{path}`; dispatches `ai-grammar:processing-started` before request; overlay hidden after completion (cleanup); response `{error:"AI service unavailable"}` → `appAlert("errors.failed_to_process_ai: AI service unavailable")` (key + ": " + message); success `{original,corrected,provider,model}` → corrected textarea value = `corrected`, dialog shown (×4).
- [C] close() closes dialog.
- [C] toggleEditMode: diff→edit hides `correctedDiff`, shows textarea, button text = t("preview.title"); edit→diff reverse, button text = t("common.edit") (×2).
- [C] accept(): dispatches `accepted` with `detail:{correctedText:<textarea value>}` and then closes (×2).
- [C] cleanup(): hides processing overlay, dispatches `processing-ended`, sets `aiAbortController=null` (×3).
- [A] escapeHtml: `<script>alert("xss")</script>` → `&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;`; `&`→`&amp;`; `'`→`&#039;` (×3).
- [A] renderDiffOriginal(diff ops): `equal`→`<span class="ai-diff-equal">v</span>`, `delete`→`<span class="ai-diff-del">v</span>`, `insert`→"" (×3). renderDiffCorrected: equal same, `insert`→`<span class="ai-diff-add">v</span>`, `delete`→"" (×3). Each value is passed through escapeHtml; spans concatenated with no separator.

### `test/javascript/controllers/ai_images_controller.test.js` — subject: `app/javascript/controllers/image_sources/ai_images_controller.js` (1 test: A=0 B=1 C=0; P=0)
- [B] generate(): calls `AiImageSource.generate(promptText)` (prompt textarea value "A landscape"); on `{error:"AI service unavailable"}` result shows the error inline in `error` target (text set, `hidden` removed) and never calls `window.alert`.

### `test/javascript/controllers/app_editor_focus.test.js` — subject: `app_controller.js#showEditor` (1 test [it.each ×3 rows]: A=0 B=0 C=1; P=0)
- [C] showEditor(content): calls `codemirror.loadContent(content, currentFile)`; on next animation frame focuses the editor ONLY when content is exactly `""`; `"note content"` and `" "` (whitespace) leave focus on the previously focused explorer item.

### `test/javascript/controllers/app_explorer_background.test.js` — subject: `app_controller.js` explorer background (6 tests: A=0 B=0 C=6; P=0) (issues #261, #267)
- [C] Click on empty file-tree surface (target === fileTree) clears multi-selection (size 0, anchor null, no `.explorer-selected`).
- [C] Background click shows explorer context menu (removes `hidden`) positioned at `left=clientX px`, `top=clientY px`.
- [C] Clicks whose target is a `.tree-item` are ignored (selection object unchanged, menu stays hidden).
- [C] Selecting an explorer item hides the explorer menu.
- [C] selectAllExplorerItems(): selects every *visible* `.tree-item` (skips children of collapsed folders, i.e. inside `.tree-children.hidden`) → paths `a.md, c.md, docs`; hides menu; 3 `.explorer-selected`.
- [C] Escape in workspace keydown closes open explorer menu and returns `true` (handled).

### `test/javascript/controllers/app_explorer_selection.test.js` — subject: `app_controller.js` explorer multi-selection & keys (15 tests [one it.each ×2 rows counted 1]: A=0 B=0 C=15; P=0)
- Selection model: `Map` keyed by `JSON.stringify([type, path])` (e.g. `["folder","docs"]`) → `{path,type}`; plus `explorerSelectionAnchor` key and `explorerActiveKey`. Classes: `explorer-selected` (selection bg), `explorer-active` (keyboard cursor), `selected` (open-file marker, independent).
- [C] toggleFolder click: selects folder row, adds path to `expandedFolders`, un-hides `.tree-children`; does not open a note.
- [C] Selecting a folder leaves the open file's `selected` marker intact and does not give it `explorer-selected`; folder gets `explorer-selected` + `explorer-active`.
- [C] Plain click replaces selection (only clicked row; active moves).
- [C] Ctrl or Meta click toggles row in/out of selection; on toggle-off active falls back to remaining row (it.each ctrlKey/metaKey).
- [C] Shift+click selects contiguous range of *visible* rows from anchor (rows inside collapsed `.tree-children.hidden` skipped; folder row itself included) → `[a.md, closed, b.md, c.md]`; Ctrl+Shift+click adds a range to existing selection.
- [C] Focus event fired by a pointer-down (`_explorerPointerDownItem === item`) must not collapse a modifier selection; keyboard focus (no pointer-down) selects just that row (×2).
- [C] selectFile with Shift modifier selects the file but does NOT load it.
- [C] remapExplorerSelection(old,new,"folder"): remaps folder and descendants (`old-folder/note.md`→`new-folder/note.md`), keeps outside entries, remaps anchor & active keys, reapplies classes. Prefix rule = equal or `old + "/"`.
- [C] removeExplorerSelection("docs","folder"): drops folder + descendants; anchor removed→null; active falls to remaining `["file","outside.md"]` and gets `explorer-active`.
- [C] `Delete` key on a selected row: preventDefault once and `fileOperations.deleteItems([{path:"note.md",type:"file"},{path:"docs",type:"folder"}])` (selection order).
- [C] `F2` renames only when exactly one eligible row selected → `renameExplorerItem({path,type})`; with 2 selected does nothing.
- [C] No delete when selection empty, when only the protected config (`.fed`, `data-file-type="config"`) is selected, or when the event target is outside the explorer tree. No rename for config row or multi-selection.
- [C] Ignored: `Backspace`, Delete with Ctrl, key `repeat:true`, F2 with Shift, F2 repeat.

### `test/javascript/controllers/app_file_rename.test.js` — subject: `app_controller.js#onFileRenamed/#onFileDeleted` (16 tests: A=12 B=0 C=4; P=0)
- [A] expandedFolders remap on folder rename (test re-implements the rule): each path equal to `old` or starting with `old + "/"` → `new + path.slice(old.length)`; others unchanged; `project-old` NOT remapped when renaming `project`; empty set → empty; deep `a/b/c`→`a/b/renamed` incl. descendants (×6).
- [A] currentFile remap: only if it starts with `old + "/"` (`project/src/main.md`→`app/src/main.md`, `docs/readme.md`→`documentation/readme.md`); outside or shared-prefix (`project-v2/file.md`) unchanged; null/undefined pass through (×6).
- [C] onFileRenamed file `foo.md`→`bar.md` (active): currentFile=`bar.md`, `autosave.renameFile("foo.md","bar.md","file")`.
- [C] onFileRenamed folder `docs`→`archive` with active `docs/foo.md`: currentFile=`archive/foo.md`, `autosave.renameFile("docs","archive","folder")`.
- [C] onFileDeleted of active file: currentFile=null, pending slash insertion cleared (null), `autosave.deleteFile("foo.md","file")`, `updateUrl(null,{replace:true})`.
- [C] Deleting active file shows placeholder and does NOT call `autosave.prepareForTransition` (never flush a draft to the deleted path), still calls `deleteFile`.

### `test/javascript/controllers/app_item_moved_reveal.test.js` — subject: `app_controller.js#onItemMoved` (3 tests: A=0 B=0 C=3; P=0) (issue #254)
- [C] Moving a note into a collapsed folder un-hides its `.tree-children`, adds `expanded` to its `.tree-chevron`, adds folder to `expandedFolders`.
- [C] Nested destination `outer/inner/moved.md` reveals every ancestor (`outer`, `outer/inner`).
- [C] If moved file is current: currentFile updated to new path and `updateUrl("target/moved.md")`; tree DOM left intact.

### `test/javascript/controllers/app_library.test.js` — subject: `app_controller.js` Library workspace & media insertion (11 tests: A=5 B=0 C=6; P=0)
- [C] toggleLibrary(): keeps sidebar visible, hides editor panel and preview, shows `libraryPanel`, toggle button `aria-pressed="true"`; then opening a file from Explorer (`applyLoadedFile`) sets currentFile, shows editor, hides library, RESTORES preview to its previous visible state, `aria-pressed="false"`.
- [C] Closing Library restores a preview that was hidden as hidden; editor visible; aria-pressed false.
- [C] Each *transition* into Library (not repeated calls while open) calls library controller `resetUsageForLibraryOpen()` + `load()` exactly once; reopen after editor → called again (2 total).
- [A] openLibraryNote path guard: opens `folder/used note.md`; rejects `../outside.md`, `/absolute/note.md`, and non-`.md` (`folder/file.txt`) — only one `openFileAndRevealInTree` call.
- [A] Image picker result for root-relative `images/photo.png` inserted into note `my-posts/note.md` is rewritten note-relative: `![photo](../images/photo.png)` (replaceRange at 0,0).
- [C] Preview hide/show goes through the preview controller (`hide()` on entering Library, `show()` on returning).
- [C][A-logic] insertLibraryMedia image: note `folder/sub/note.md`, item path `images/photo (one).png` name `photo one.png` → `![photo one.png](../../images/photo%20%28one%29.png)` (note-relative, spaces→%20, parens→%28/%29); returns true, focuses editor, calls `onEditorChange({detail:{docChanged:true}})`, sets `event.detail.status="inserted"`, returns to editor workspace (library hidden).
- [A] Video from root note: `insertAt(0, '<video controls class="video-player">\n  <source src="videos/nested/clip.mp4" type="video/mp4">\n</video>')`, status inserted.
- [A] S3 item (`provider:"s3"`) uses absolute `url`, re-encoding parens: `https://bucket.example/frankmd/2026/photo%20(one).png` → `...photo%20%28one%29.png`.
- [A] Unsafe S3 URLs rejected (return false, status `invalid_media`): `javascript:alert(1)`, URL with credentials `https://user:secret@...`, `not a URL`.
- [C] No open note → false, status `no_open_note`, `appAlert("library.no_open_note")`; open file is config `.fed` (type `config`) → status `markdown_note_required`, `appAlert("library.markdown_note_required")`.

### `test/javascript/controllers/app_navigation.test.js` — subject: `app_controller.js` note navigation, created-note undo/redo boundaries (33 tests [one it.each ×2 rows counted 1]: A=0 B=0 C=33; P=0)
Endpoints involved (backend mocked via `fetch`): load note `GET /notes/<encodePath(path)>` → `{content, revision}`; tree `GET /notes/tree?expanded=<csv>&selected=<path>` (HTML); delete `DELETE /notes/<path>?expected_revision=<rev>`; create `POST /notes` JSON `{path, content}` (201 `{path,message}`). URL bar mirrors `/notes/<path>` or `/`.
- [C] showHome(): cancels in-flight loadFile (late response ignored; currentFile=null, showEditor not called, `updateUrl(null)`); returns true. If `autosave.prepareForTransition()` returns `{ok:false}` (draft can't be stored), Home is refused (false), currentFile kept and `updateUrl("dirty.md",{replace:true})`.
- [C] Out-of-order loads: only the latest request (`c.md`) is applied (navigation generation counter); `showEditor("content C","markdown","revision-C")` once, `updateUrl("c.md")` once, prepareForTransition called once (only for the first transition from the open file).
- [C] applyLoadedFile selects the newly opened note in the Explorer (selection replaced with just it, active key set, classes synced) without moving focus.
- [C] Stale 404/network-error responses for superseded loads do not show file-not-found, change URL or editor.
- [C] Tree refresh responses that arrive after a Turbo stream mutation of `#file-tree-content` (`invalidateTreeRefreshesForStream`) or a file deletion are discarded (tree generation counter).
- [C] After refreshTree, all previously selected Explorer rows get `explorer-selected` again and the anchor is preserved.
- [C] If outgoing draft cannot be stored (prepareForTransition `{ok:false,error}`), loadFile aborts: currentFile, editor, path display and URL unchanged.
- [C] popstate (browser back/forward) with unflushable draft: stays on current file and restores URL via `updateUrl("a.md",{replace:true})`; placeholder not shown.
- [C] Initial server-provided note (`initialNoteValue {path,content,revision,exists:true}`) goes through guarded transition: prepareForTransition once, showEditor(content,"markdown",revision), `refreshTree(1)`.
- [C] onFileCreated records `createdNoteBoundaries[path] = {path, initialContent:<exact server content>, initialRevision, previousPath:<previously active note>}` and opens it.
- [C] Rename: `codemirror.remapHistoryPaths(old,new,"file")`; boundary re-keyed & its `path` updated; dependents' `previousPath` remapped; a stale `deleted` boundary already at the destination path is dropped.
- [C] Folder move: remaps currentFile (`old-folder/nested/open.md`→`new-folder/...`), boundary keys/paths and `previousPath` under the folder; `remapHistoryPaths(...,"folder")`.
- [C] Folder delete: `codemirror.evictHistoryPaths("deleted-folder","folder")`; boundaries inside removed; boundaries whose predecessor was inside get `previousPath=null`; unrelated untouched.
- [C] No boundary recorded when: the creation load became stale; server response lacks content or revision; load returns 404 (×3).
- [C] Undo at history start on a created note whose content equals `initialContent`: `onUndoAtHistoryStart` returns true and opens custom `undoCreatedNoteDialog` (not `window.confirm`) with message t("confirm.undo_created_note",{name}) → `Undoing creation will delete "created.md". You can redo it during this session.`, filename wrapped in `.confirm-dialog__filename`. On confirm (`returnValue="confirm"`): `autosave.prepareForFileDeletion(path)` → `{ok,revision}`; editor read-only lock acquired+released; `DELETE /notes/created.md?expected_revision=latest-own-save`; boundary kept with `deleted=true`; history not evicted; `autosave.deleteFile(path,"file")`; opens predecessor and `updateUrl("previous.md",{replace:true})`.
- [C] Cancel (`returnValue="cancel"`) or closing via dialog header (`closeUndoCreatedNoteDialog` → `dialog.close("cancel")`): nothing deleted, no fetch, stays on note (×2).
- [C] Not intercepted (returns false) for ordinary notes or when content differs from baseline.
- [C] DELETE 409 `{error:"revision conflict"}` → `showTemporaryMessage("revision conflict", 5000)`; boundary retained (not deleted), autosave resumed, lock released, no deleteFile.
- [C] Editor is read-only during autosave drain and in-flight DELETE (user edits rejected); unlocked after failure.
- [C] Content changed programmatically during drain → abort with `showTemporaryMessage("errors.failed_to_delete", 5000)`, no fetch, resume autosave.
- [C] Redo at history end on predecessor (`onRedoAtHistoryEnd("previous.md")`) recreates newest deleted boundary: `POST /notes` body `{path, content:initialContent}`, then loads it, `showEditor(initialContent,"markdown","recreated-revision")`, `refreshTree(1)`, `deleted=false`, autosave resumed. Paths with spaces/nesting (`nested/created note.md`).
- [C] Root-boundary redo (no predecessor, no open file) via global keydown Ctrl+Shift+Z or Ctrl+Y: preventDefault, recreate (DELETE then POST sequence observed) (it.each ×2). Not routed when keydown target is an `<input>`.
- [C] Multiple deleted boundaries sharing a predecessor → newest (last inserted) recreated.
- [C] Recreate 422 `{error:"Destination already exists"}` → temp message 5000ms, boundary stays deleted, active file unchanged, autosave resumed. Network throw → resume, pending-redo set cleared. prepareForTransition throwing → resume, no fetch.
- [C] Redo completing after user navigated elsewhere: keeps user's navigation, boundary deleted=false, resume.
- [C] Navigation change during deletion drain → no DELETE, lock released, resume, boundary intact.
- [C] codemirrorOutletConnected before connect() is a no-op (no throw); after connect (`_initializationReady=true`, `expandedFolders=new Set()`), it loads initial note, expands parent folders (`studies`), showEditor, removes splash screen once.

### `test/javascript/controllers/app_slash_commands.test.js` — subject: `app_controller.js` slash-command insertion (10 tests [two it.each ×4 rows counted 1 each]: A=0 B=0 C=10; P=0)
- [C] openSlashCommandAction(detail `{action,from,to,query}`) for `table|image|video|emoji`: stores `pendingSlashInsertionRange = detail` and opens the respective flow (image picker / video dialog / emoji picker `open()`; table editor).
- [C] Image inserted while a pending range `/image` at 7..13 in `"before /image after"`: block insert widens range to swallow the adjacent spaces → `replaceRange("\n![photo](photo.png)\n", 6, 14)`; pending cleared.
- [C] Table (new): `replaceRange("\n\n| A |\n|---|\n| 1 |\n\n", 6, 14)`. Video embed: `replaceRange("\n\n<video></video>\n\n", 6, 14)`. Emoji is inline: exact range `replaceRange(":smile:", 7, 13)`.
- [C] Table in edit mode (`editMode:true,startPos:0,endPos:39`) uses the table's own range, ignoring slash range: `replaceRange(newTable, 0, 39)`.
- [C] openTableEditor({slashInsertion:true}) inside an existing table dispatches `window` event `frankmd:open-table-editor` whose `detail.existingTable` has the `/table` query removed (`| | value |`) but the document itself is unchanged; dialog close (`onSlashCommandDialogClose`, `data-slash-command-action`) clears pending without editing.
- [C] After dialog close, toolbar insertions use the current selection: `replaceRange("\n![toolbar](toolbar.png)\n", 1, 1)`.
- [C] When active file is not Markdown (type `config`) none of table/image/video/emoji insert anything (no replaceRange/insertAt/onEditorChange), pending cleared (it.each ×4).
- [C] Stale range: if text at from..to no longer equals the saved query, nothing is replaced and pending cleared.

### `test/javascript/controllers/app_toggle_task.test.js` — subject: `app_controller.js#onPreviewToggleTask/#toggleTask` (6 tests: A=0 B=0 C=6; P=0) (issue #203)
- [C] Preview checkbox event `{detail:{line:7}}` → `codemirror.toggleTaskAtLine(7)` for markdown notes; no-op without editor, for non-markdown (`config`), or when `line` missing / not a number (string `"3"` rejected) (×4).
- [C] toggleTask() shortcut action → `codemirror.toggleTask()` for markdown; no-op without editor or when `currentFileType` null (×2).

### `test/javascript/controllers/app_typewriter_mode.test.js` — subject: `app_controller.js` typewriter mode (7 tests [it.each ×2 counted 1]: A=0 B=0 C=7; P=0)
- [C] initializeTypewriterMode before a Markdown file is active: `typewriter.setEnabled(false)` but toggle button `aria-pressed="true"` reflects saved preference; once `currentFileType="markdown"`: setEnabled(true), `body.typewriter-mode`, preview & scroll-sync `setTypewriterMode(true)`, preview shown once and `syncToTypewriter(currentLine=6,totalLines=20)` from codemirror `getTypewriterSyncData()`.
- [C] Saved preference stays inactive for non-Markdown (`config`): setEnabled(false), no body class, preview not shown.
- [C] onTypewriterToggled({enabled:true}) persists `saveConfig({typewriter_mode:true})` (config key `typewriter_mode`), updates settings value, enables typewriter and preview mode.
- [C] applyTypewriterMode(true) still shows preview + syncs while Settings dialog is open.
- [C] onEditorChange with `docChanged` must NOT call `maintainTypewriterScroll` (no extra selection transaction during doc change).
- [C] Toggling typewriter on/off never changes `sidebarVisible` nor calls applySidebarVisibility (it.each true/false).
- [C] showEditorWorkspace() after Library un-hides editor panel and calls `codemirror.refreshTypewriterLayout()` once.

### `test/javascript/controllers/app_view_mode.test.js` — subject: `app_controller.js` split/single view mode (11 tests: A=0 B=0 C=11; P=0)
- [C] initializeViewMode applies persisted settings `viewMode` (`single` → `body.single-view-mode`); defaults `split` without settings controller (×2).
- [C] setViewMode normalizes unknown values (`"diagonal"`) to `split`.
- [C] Single mode: hides preview, editor pane shown, body class added. Back to split: editor pane shown and preview stays visible if it was the active single pane; body class removed (×2).
- [C] Split: togglePreview delegates to preview controller `toggle()`.
- [C] Single: togglePreview switches to full-width preview pane (returns true; editor panel hidden but still connected in DOM — CodeMirror, autosave, undo, slash state must survive); toggling again returns false, hides preview, shows editor and focuses it (×2).
- [C] Opening/closing Settings in single mode preserves the active preview pane (no hide).
- [C] View mode changes apply live while Settings is open (hide called once).
- [C] Single mode still blocks preview for non-markdown files (no show; `showTemporaryMessage` called).

### `test/javascript/controllers/app_workspace.test.js` — subject: `app_controller.js` workspace chrome (header, title rename, copy, sidebar, overflow menu, pins, AI busy) (37 tests [3 it.each ×2 rows counted 1 each]: A=0 B=0 C=37; P=0)
- [C] `.workspace-header` hidden only while Home outlet is visible (no current file & home not `.hidden`); shown in Library and with a note open.
- Title rename (document title button → inline input):
  - [C] Click opens input focused with value = basename without `.md` (`folder/note.md`→`note`), caret at end (4,4), no selection; title button hidden; no rename yet.
  - [C] Width mirror: `titleSizer.textContent` = input value; input width = sizer width + horizontal padding + border (100+16+2 → `118px`).
  - [C] Enter → `fileOperations.renamePath({path:"folder/note.md",type:"file"}, "renamed")`, input hidden, focus back to title button. Escape cancels (returns true, preventDefault) and takes precedence over leaving Library (libraryVisible stays true); no rename.
  - [C] Blur (click-away) submits but does not steal focus back. Failed click-away (`{ok:false,error:"Name already exists"}`) keeps input editable & visible, shows error, focus remains where user went.
  - [C] Failure shows error in `titleRenameError` (role=alert), keeps value, `aria-invalid="true"`, focus stays in input; typing hides the error; retry succeeds (2 calls).
  - [C] `{ok:false,error:null}` (e.g. a draft warning already shown) → no filename error shown, input re-enabled, no aria-invalid.
  - [C] While saving: duplicate submits and blur ignored (1 call), input disabled; hidden after completion.
  - [C] Stale result after navigating to another file is discarded (no error shown, input hidden). Submit after currentFile changed → not sent.
  - [C] Enter with `isComposing:true` (IME) does not submit.
  - [C] Title rename disabled (button `disabled`) for no file (null) and for `.fed` config (it.each).
- Copy content:
  - [C] copyCurrentContent writes the *editor* value (unsaved draft) to `navigator.clipboard.writeText` for notes and `.fed`; toast `status.copied_to_clipboard` (it.each ×2); empty file copies `""`; falls back to `textarea` target value when CodeMirror unavailable; nothing when no file open; on rejection toast exactly once `errors.failed_to_copy_content` (×5 incl. each).
- Sidebar:
  - [C] toggleSidebar hides whole sidebar: class `is-hidden`, sidebar & explorer `inert`, `sidebarOpen` button shown, `aria-expanded="false"` on both toggles, focus moves to `sidebarOpen`, `refreshTypewriterLayout()` called; reopening via `sidebarOpen` restores and focuses `sidebarToggle`.
  - [C] Opening the Explorer via shortcut while focus is elsewhere (editor/title) keeps that focus.
  - [C] When the focused `sidebarOpen` becomes hidden (desktop, sidebar visible), focus moves to `sidebarToggle`.
  - [C] Mobile (`mobileWorkspace`): opening sidebar uses overlay (`is-overlay`), backdrop shown, `[data-workspace-main]` inert (global actions & toggle not inert); Tab from last focusable wraps to `sidebarToggle` (focus trap); Escape closes overlay → `is-hidden`, main not inert, focus to `sidebarOpen`.
- [C] Late outlet connections: settings outlet connecting after the note with `showStatsEnabled` → stats panel `show()`+`update()` once; path-display outlet → `update("folder/note.md")`.
- Overflow menu / action applicability:
  - [C] Single preview pane: toolbar visible, editing actions visible but `disabled`, preview toggle enabled; Library: toolbar hidden, editing rows hidden, preview toggle disabled; no file: `[data-workspace-document-action]` hidden; opening menu with keyboard focuses first enabled item (`Help`).
  - [C] Menu stays open and items recomputed (`Edit, Copy, Help`) when a note finishes loading after opening.
  - [C] Keyboard: ArrowDown from toggle opens & focuses first; ArrowDown next; End last; Home first; Escape closes and focuses toggle.
  - [C] Pointer open (click detail=1) keeps focus on toggle; then ArrowDown → first (`Edit`), ArrowUp → last (`Help`) (it.each).
  - [C] Menu closes before launching a dialog (e.g. Help) and focus stays in the dialog.
  - [C] Outside click closes (`aria-expanded="false"`); Tab closes and returns focus to toggle (no trap).
- Toolbar pins (sessionStorage key `frankmd.workspacePins`, JSON array of action ids):
  - [C] Pin checkbox (`role=menuitemcheckbox`, `data-workspace-pin="video"`) toggles `aria-checked`, aria-label `data-pin-label`/`data-unpin-label` ("Unpin Video"), toolbar icon `data-pinned="true|false"`; doesn't run the action or close the menu; keeps focus; unpin persists `[]`.
  - [C] Restore filters unknown ids (`["video","unknown"]` → `["video"]`).
  - [C] Malformed JSON → use defaults (markup's `data-pinned`); storage throwing on get/set → still toggles in memory.
  - [C] Pins preserved across modes: preview-only → pinned icon visible but disabled; Library → hidden and pin row removed from menu items; back → enabled and still pinned.
  - [C] ArrowRight/ArrowLeft moves between row action and its pin; reversed when menu `direction: rtl`; Escape returns focus to toggle.
- [C] AI busy: onAiProcessingStarted disables every `aiButton` target (toolbar + menu) with `aria-busy="true"` and `codemirror.setReadOnly(true)`; ended restores (no aria-busy, labels unchanged) and `setReadOnly(false)`.

### `test/javascript/controllers/autosave_controller.test.js` — subject: `app/javascript/controllers/autosave_controller.js` (+ `lib/draft_storage.js`, `recovery_diff_controller.js`) (83 tests [one it.each ×2 counted 1]: A=0 B=0 C=83; P=0)
Key constants/storage (load-bearing for the port): `SAVE_DEBOUNCE_MS=2000` (after last keystroke), `SAVE_MAX_INTERVAL_MS=30000` (forced save while continuously typing), `DRAFT_DEBOUNCE_MS=300` (local draft write), offline backup debounce `1000` ms. Save = `PATCH /notes/<encodePath(path)>` JSON body exactly `{content}`; response `{revision}` becomes new base revision. localStorage keys: draft `frankmd:draft:<encodeURIComponent(path)>` (record `{path, content, baseRevision, draftRevision, updatedAt}`), rename-collision copies `frankmd:draft-conflict:<encodeURIComponent(path)>:<conflictId>`, legacy offline backup `frankmd:backup:<raw path>` (JSON `{content,timestamp}`). Large-deletion rule (`isLargeDeletion`): baseline non-empty AND `lost/baseline.length > 0.2` AND `lostChars > 50`.
- Save status label:
  - [C] `showSaveStatus(t("status.unsaved"))` uses server-rendered `statusLabelsValue` (`{unsaved:"Unsaved changes"}`) when translations haven't loaded; never renders an unresolved key (text "" and `hidden`) (×2).
- saveNow / content-loss guard:
  - [C] 200→180 chars (10%, 20 chars) saves, banner hidden; 300→0 blocks save (no fetch), banner `contentLossBanner` shown (`flex`, not `hidden`), `_contentLossWarningActive`, draft persisted `{content:"",baseRevision}`; `_contentLossOverride` bypasses; null baseline (fresh) saves; override reset after successful save (×5).
- scheduleAutoSave:
  - [C] While warning active: no timer but `hasUnsavedChanges=true`. Content equal to saved → not dirty, no timer, no status text, no draft. Edit then revert to saved → dirty cleared, both timers null, status cleared, draft removed (×3).
- Browser lifecycle flush:
  - [C] `visibilitychange` to hidden and `pagehide` flush the draft synchronously (no server request, pending draft timers cleared); dirty empty draft flushed too (it.each); unchanged draft not rewritten on repeated events; draft persistence failure → status "Unable to save draft" (`draft_storage_error` label) with class `text-red-500` and `beforeunload` preventDefault (unload warning); after a later successful flush status back to "Unsaved changes" and beforeunload listener removed (×5).
- Large-deletion warning scope:
  - [C] File A's warning does not block autosave for file B after transition (B saved to `/notes/b.md` after 2000ms; A's empty draft kept).
  - [C] "Save anyway" approval is bound to the exact snapshot; a later different snapshot re-triggers the warning and is stored as draft.
  - [C] Restoring a protected draft (`recoverDraft`) recomputes the warning against the baseline; divergent recovered draft is re-based on the latest server revision and warning recomputed.
  - [C] undoContentLoss(): uses CodeMirror `undo`; if result equals baseline → warning cleared & draft removed; if still large → draft updated; if no undo history (`undo` returns false) → `loadContent(baseline, path)`, warning cleared, draft removed.
  - [C] saveAnywayAfterWarning removes the matching draft only after a successful save; failed save keeps the draft (×2). (Total in group: 9.)
- File transition (`prepareForTransition`) & scoping:
  - [C] Flushes outgoing snapshot to draft (with baseRevision), cancels both timers, does NOT force a server save; returns `{ok:true}`.
  - [C] If draft write fails → `{ok:false,error}`, file remains active and timers kept. Dirty file without a server revision → `{ok:false}`.
  - [C] Timer scheduled for A never reads/saves B's content after setFile(B). Offline → transition ok with draft only. In-flight A save response does not touch B's state (baseRevision/lastSaved/status) (×6).
- Online draft persistence & recovery:
  - [C] Empty draft debounced 300ms and stored per path; snapshots for different files independent.
  - [C] recoverDraft: same baseRevision → auto-restore draft (dirty); server already equals draft content → clear draft; diverged (different baseline) → keep draft and open recovery dialog with `{path, serverContent, backupContent, backupTimestamp:draft.updatedAt, source:"draft", draftRevision}`.
  - [C] Rename onto a path with an existing draft → source draft moved to a conflict entry, recovery opened with `source:"draft-conflict"` + `conflictId`; accepting promotes moved draft and preserves the displaced destination draft as a new conflict; choosing server sets editor to server content, clears both drafts, keeps destination copy as conflict.
  - [C] Recovery dialog cannot be dismissed with Escape (`cancel` preventDefault, stays open); edits meanwhile don't save or overwrite the original divergent draft.
  - [C] Draft newer than a stale legacy backup wins and removes the backup; newer *different* legacy backup is offered (`source:"backup"`).
  - [C] Draft storage failure reported via status (red). Explicitly chosen empty draft is restored (`setValue("")`, re-based). A newer draft written during an in-flight save survives the save completion (re-based to `saved-revision`) (×13 in group).
- [C] dismissContentLossWarning hides banner (removes `flex`), resets both flags. saveAnywayAfterWarning sets override, hides banner, calls saveNow. checkContentRestored auto-dismisses when below threshold again (e.g. Ctrl+Z), stays otherwise (×4).
- Concurrency:
  - [C] `_isSaving` blocks a second concurrent save (1 fetch); cleared after error; content changed during fetch → dirty and re-scheduled; unchanged → clean, no timer (×4).
  - [C] prepareForFileDeletion waits for in-flight save and resolves `{ok:true,revision:"autosaved-revision"}`; cancels follow-up debounce and records known revision; active note change while waiting → `{ok:false,stale:true}` (×3).
- Offline backup:
  - [C] scheduleOfflineBackup offline → `offlineBackup.save(path, content)` after 1000ms debounce; online → no timer; onConnectionLost backs up immediately only if content differs from last saved; successful save clears backup; checkOfflineBackup with differing backup opens recovery `{...,source:"backup",draftRevision:null}`, matching (check returns null) doesn't; onRecoveryResolved("backup") sets editor, keeps lastSaved=server, dirty, writes draft re-based, removes legacy key; accepted legacy backup consumed and not offered again (×9).
- Offline/online flow:
  - [C] In-flight save completing while offline + offline edits → reconnect triggers saveNow; onConnectionLost clears both timers (stays dirty); scheduleAutoSave & saveNow no-ops offline (dirty kept); onConnectionRestored saves only when dirty (×6).
- File lifecycle (rename/delete):
  - [C] renameFile file `foo.md→bar.md` keeps dirty state and pending save goes to `/notes/bar.md` with body `{content:"edited content"}`; folder rename `docs→archive` remaps `docs/foo.md`; local draft moved (`{...original,path:"bar.md"}`) before scheduling save.
  - [C] Draft remap failure → conflict entry with source content kept, destination path blocked (`_draftBlockedPaths`), pending recovery; writes/autosave blocked until user accepts recovery, then unblocked and save scheduled. If fallback preservation also fails → source draft retained, `_pendingRemapRecoveries[bar.md]=foo.md`, retried on next recoverDraft. Late save response doesn't remove source draft after remap failure.
  - [C] Destination draft collision is never overwritten; local writes to destination blocked.
  - [C] Folder rename does not remap `docs-old/foo.md` when renaming `docs` (returns false).
  - [C] deleteFile clears pending save/timers/dirty state (no fetch later); removes drafts + legacy backups (`{ok:true,removed:2}`); clears active state even if cleanup fails (`{ok:false,error}`); late save response after delete ignored; failed old-path save after rename re-schedules at new path; old-path save response doesn't clobber remapped newer draft (×12 in group).

### `test/javascript/controllers/code_dialog_controller.test.js` — subject: `app/javascript/controllers/code_dialog_controller.js` (23 tests: A=6 B=0 C=17; P=0)
- [C] connect: `codeEditMode=false`, start/end pos 0; language list contains javascript/python/ruby. (Full list, in order: javascript typescript python ruby go rust java c cpp csharp php swift kotlin scala haskell elixir erlang clojure lua perl html css scss sass less json yaml toml xml markdown sql graphql bash shell powershell dockerfile makefile nginx apache vim regex diff git plaintext.) (×2)
- [C] open() empty → shows modal, empty fields, not edit mode; open({language,content,editMode:true,startPos:10,endPos:50}) prefills and stores range (×2). close() closes (×1).
- [A] Language suggestions (`onLanguageInput`): value lowercased+trimmed; matches = list items that `startsWith` or `includes` value, in list order, max 6; shown unless no matches or the FIRST match equals the value exactly. `java` → shows (javascript, java); empty → hidden; `javascript` → hidden (×3).
- [C] Language field keys: Tab with visible suggestions → preventDefault and take first suggestion (`pyth`→`python`); Escape hides suggestions; Ctrl+Enter → preventDefault + insert() (×3).
- [C] Content field keys: Ctrl+Enter inserts; Tab inserts 2 spaces at caret (`line1`→`line1  `) or `\t` when "indent with tabs" checkbox checked (×3).
- [C] showSuggestions(list) renders one `<button>` per language (data-language) and unhides; hideSuggestions; selectLanguage from `data-language` sets value and hides (×3).
- [A] insert() dispatches `code-dialog:insert` with `{codeBlock:"```<lang>\n<content>\n```", language, editMode, startPos, endPos}`; edit mode passes stored range (10,50); empty content → "```ruby\n\n```" (×3).
- [C] Unrecognized language → `appConfirm("dialogs.code.unrecognized_language")`; cancel → no dispatch; dialog closes after insert (×3).

### `test/javascript/controllers/codemirror_controller.test.js` — subject: `app/javascript/controllers/codemirror_controller.js` (79 tests [one it.each ×2 counted 1]: A=6 B=0 C=73; P=0)
Values: content, placeholder, fontFamily, fontSize, lineHeight, lineNumberMode (0/1/2), typewriterMode, cursorRecenterOnClick, readOnly, vimMode.
- [C] connect creates editor with `content` value and mirrors it to hidden textarea; disconnect destroys editor (editor=null); getValue "" without editor; setValue replaces & syncs hidden textarea (×8).
- Per-note history (loadContent(content, path)):
  - [C] loadContent is not a user change (no `codemirror:change`, no undo entry); a recovered draft loaded before the preloaded `contentValueChanged` callback is kept.
  - [C] Independent undo/redo history per note path (switching back restores that note's history only if loaded content equals cached doc).
  - [C] remapHistoryPaths(old,new,"folder") remaps active + cached states for descendants; "file" remap moves history and discards stale destination history; evictHistoryPaths(path,"file"|"folder") removes history so reused paths start fresh; cached history discarded when loaded content differs (external update) (×7).
  - [C] LRU cache of note editor states capped at 20 (issue #192): after visiting 22 notes size=20 and `note-1` evicted; active note never evicted; re-visited note refreshes recency (restored without `replaceContentWithoutHistory`) (×3).
- [C] Ctrl+Z: normal undo first; when history exhausted calls app's `undoAtHistoryStartHandler` once and preventDefault; not in Vim mode. Ctrl+Y / Ctrl+Shift+Z: normal redo first, then `redoAtHistoryEndHandler`; not in Vim mode (×5).
- [C] Editor API: getSelection `{from,to,text}`; setSelection; replaceSelection; insertAt(5," Beautiful"); replaceRange("Goodbye",0,5); focus; hasFocus boolean; getScrollInfo `{top,left,height,clientHeight}`; scrollTo; getScrollRatio in [0,1] (×12).
- [C] getTopVisibleLine: 1-based, uses `lineBlockAtHeight(scrollTop + 1)` → line of block start; 1 without editor. scrollToLine(n): clamps n to [1, lines], sets `scrollDOM.scrollTop = lineBlock.top`; no-op without editor (×6).
- [A] toggleTaskAtLine(n) (issue #203): `- [ ]`↔`- [x]` on that source line (`- [ ] open\n- [x] done\nplain` → line1 checked, line2 unchecked); line number clamped to last line (99 → line 2) (×2).
- [C] toggleTaskAtLine preserves cursor (`{line:1,column:7,offset:6}`); non-task line → no dispatch; no editor → no throw (×3).
- [A] toggleTask() shortcut on cursor line(s): toggles existing marker; plain line → prefix `- [ ] ` (`buy milk`→`- [ ] buy milk`); empty line → `- [ ]` (no trailing space); multi-line selection applies per-line rule to every touched line (`- [ ] a\nplain\n- [x] c` → `- [x] a\n- [ ] plain\n- [ ] c`) (×4).
- [C] toggleTask is one undoable transaction; no editor → no throw (×2).
- [C] Cursor: get/setCursorPosition (1-based line, column), clamps to last line; getLineCount; getLine(n) ("" for 0/out of range); getCursorInfo `{currentLine,totalLines}` (×8).
- [C] Settings setters: setFontFamily/FontSize/LineHeight update values; toggleLineNumberMode cycles 0 (off)→1 (absolute)→2 (relative)→0; setTypewriterMode; setCursorRecenterOnClick updates state field and is re-applied when a cached note state is restored (×7).
- [C] Click recentering: collapsed pointer selection + mouseup recenters (`scrollTop = coordsTop - domTop + scrollTop - clientHeight/2` → 275) when cursorRecenterOnClick enabled, with or without Typewriter (it.each); range selection not recentered; setting off → not recentered (×3).
- [C] toggleTypewriterMode returns new state; isTypewriterMode; setReadOnly(true) → contenteditable="false"; acquireReadOnlyLock() keeps editor read-only even if setReadOnly(false) until released; getEditorView === editor; getPositionForLine(2)=7; jumpToLine(3); `codemirror:change` event with `detail.content` on doc change; contentValueChanged updates editor on external change and doesn't dispatch when identical (×10).

### `test/javascript/controllers/connection_monitor_controller.test.js` — subject: `app/javascript/controllers/connection_monitor_controller.js` (21 tests: A=0 B=0 C=21; P=0)
Health probe: `fetch("/up", {method:"HEAD", cache:"no-store", ...})` every `interval` (default 5000 ms) with `timeout` default 3000 ms (values from controller; a desktop port replaces this with its own backend-liveness notion).
- [C] connect: `isOnline=true`, monitor interval started (×2).
- [C] checkConnection: ok → online; rejected fetch → offline; `{ok:false}` → offline; overlapping checks prevented (3 calls → 1 fetch while pending) (×4).
- [C] handleOffline: shows `banner`, disables `textarea`, dispatches `connection-monitor:offline`; no-op if already offline (×4). handleOnline: hides banner, re-enables textarea, dispatches `online`; no-op if already online (×4).
- [C] Textarea disabled state preserved: if it was disabled before going offline it stays disabled after online; otherwise re-enabled (×2).
- [C] retry() → checkConnection; window `online` event → checkConnection; window `offline` → handleOffline (×3).
- [C] disconnect clears interval (null) and removes window `online`/`offline` listeners (×2).

### `test/javascript/controllers/content_search_controller.test.js` — subject: `app/javascript/controllers/content_search_controller.js` (40 tests: A=0 B=0 C=40; P=0)
Backend: `GET /notes/search?q=<encodeURIComponent(query)>` returning server-rendered HTML of result buttons (`data-index`, `data-path`, `data-line`); results data parsed from DOM as `{path, line_number:parseInt(data-line)}`. Debounce 300 ms. Status (hard-coded English, not translated): `"<n> match(es) found[ (showing first 20)] - use ↑↓ to navigate, Enter to open"` (" (showing first 20)" when n ≥ 20).
- [C] connect: `searchResultsData=[]`, `selectedIndex=0`, `searchTimeout=null`, `usingKeyboard=false`.
- [C] open(): showModal; clears data, index, input value, results HTML; status t("status.type_to_search_regex") (×4). close() (×1).
- [C] onInput: empty query clears results; non-empty → status t("status.searching"), debounced performSearch (not immediate, timeout set), new input replaces previous timeout (×4).
- [C] performSearch: calls `/notes/search?q=hello` with GET; parses buttons into data (`file1.md`/10, `file2.md`/20); resets index; status contains "2 matches"; empty → t("status.no_matches"); `!ok` → t("status.search_error") (×6).
- [C] Keys: ArrowDown/ArrowUp move selection (preventDefault, `usingKeyboard=true`), clamped to [0, len-1]; Enter → selectCurrent (×5).
- [C] scrollIntoView on selected `[data-index]` with `{block:"nearest"}`; missing element no throw (×2).
- [C] Hover selects index unless keyboard mode; same index → no update; mousemove re-enables mouse selection (×4).
- [C] selectFromClick dispatches `content-search:selected` `{path:"folder/note.md", lineNumber:25}` (line parsed int); selectCurrent dispatches for selected item; nothing when empty; dispatchSelected dispatches then closes (×4).
- [C] Spinner: show/hide toggles `isSearching` + `hidden`; shown during debounce; hidden on cleared input, after success and after failure (×6).
- [C] updateSelection(i): moves classes `bg-[var(--theme-accent)] text-[var(--theme-accent-text)]` from old to new item (×3).

### `test/javascript/controllers/custom_theme_editor_controller.test.js` — subject: `custom_theme_editor_controller.js` + `theme_controller.js` (3 tests: A=0 B=0 C=3; P=0)
- [C] Live preview: editing a color (`--theme-accent` → `#abcdef`) sets the CSS var on `<html>` style without any `/config` request; Cancel/close restores prior theme id (`dark`), clears the inline var, and returns focus to the opener button.
- [C] Invalid hex (`url(evil)`) disables Save and shows `invalidValue` ("Invalid color") + aria-invalid; contrast warning when lowest of contrast(text-primary,bg-primary) and contrast(accent-text,accent) < 4.5 → warning template `Low contrast %{ratio}` with ratio `toFixed(2)` (e.g. "1.00"); reset() → `draft.colors = {}`, Save enabled, warning cleared; changing base to `light` removes `dark` class from `<html>`.
- [C] Save failure (503) keeps dialog open, status "Save failed", form re-enabled; retry sends ONE atomic `PATCH /config` with body `{theme:"custom", custom_theme: JSON.stringify(draft)}` (custom_theme is a JSON *string*); on success `{settings:{theme:"custom",custom_theme:{...}}}` closes dialog, theme becomes `custom`, `customValue.colors.accent="#aabbcc"`.

### `test/javascript/controllers/drag_drop_controller.test.js` — subject: `app/javascript/controllers/drag_drop_controller.js` (42 tests: A=5 B=0 C=37; P=0)
Move = `POST /notes/<encodePath(old)>/rename` (files) or `/folders/<old>/rename` (folders), body `{new_path, expanded:"<comma-joined expandedFolders>"}`, turbo-stream response re-renders the tree; success dispatches `drag-drop:item-moved` `{oldPath,newPath,type}`.
- [C] connect: `draggedItem=null`, `activeDropTargetId=null`.
- [C] onDragStart: stores `{path,type}`; if grabbed row is part of a multi-selection (#266) drags the whole selection, else just the row (also when it's the only selected); `effectAllowed="move"`, `setData("text/plain", path)`; adds `dragging` class; clears any active drop highlight (×7).
- [A] Selection normalization: items covered by a selected ancestor folder are excluded (`[notes.md, folder1, folder1/file1.md, folder1/subfolder]` → `[notes.md, folder1]`).
- [C] onDragEnd: clears draggedItem, removes `dragging`/`drag-ghost`, clears row `drop-highlight` and tree `drop-highlight-root` (×3).
- [C] onDragOver / onDragOverRoot: preventDefault + `dropEffect="move"`; folder row resolved from nested child or text node target (`closest` through text node parent) → `drop-highlight` + activeDropTargetId (=`data-drop-id`); exactly one highlighted row while moving; invalid targets clear highlight: file rows, the dragged folder itself, its descendants, and the dragged item's current parent folder (×7).
- [C] onDragLeaveRoot keeps highlights if `relatedTarget` (or fallback `document.elementFromPoint(clientX,clientY)`) is still inside the tree; clears when leaving the container (×3).
- [A] isValidDropTarget(folder) for a dragged set: accepts a folder outside the set; rejects a folder inside any dragged folder; rejects the folder that is the shared current parent of EVERY dragged item; accepts a folder that is parent of only some items (×4).
- [C] onDrop on folder2 with 2 items → 2 sequential rename POSTs `/notes/folder1/file1.md/rename`, `/notes/folder1/file2.md/rename` with `new_path` `folder2/file1.md`, `folder2/file2.md`; preventDefault+stopPropagation; highlight cleared (×3).
- [C] onDragEnterRoot adds `drop-highlight-root` only if dragged item is nested (root-level items don't highlight root) (×2).
- [C] onDropToRoot: nested items moved to root (`new_path:"file1.md"`), items already at root skipped; group stops after first failure (422 → only 1 request); highlights cleared (×3).
- [C] `expandedFolders` getter: "" without app controller; else comma-joined Set (×2).
- [C] moveItem: request body contains `new_path` and `expanded`; dispatches item-moved; BEFORE posting expands destination folder and all ancestors in app `expandedFolders` (`folder2`, `folder2/sub`, `folder2/sub/deep`); error → `appAlert(error.message)`; moving the active file first calls `autosave.prepareForTransition()` and aborts (no fetch) if it returns ok:false; on request failure `resumeAfterTransition()` (×5).
- [C] disconnect clears drag state.

### `test/javascript/controllers/drop_images_controller.test.js` — subject: `app/javascript/controllers/image_sources/drop_images_controller.js` (9 tests: A=0 B=1 C=8; P=0)
- [C] ingestFiles: a second ingest clears a prior `selectedImage` and dispatches `drop-images:deselected` `{detail:{type:"drop"}}`; no deselected event when nothing was selected (×2).
- [C] Dropzone highlight class `border-[var(--theme-accent)]`: added on dragover, removed on drop; dragleave onto a child keeps it (no flicker); leaving to outside removes it (×3).
- [C] loadExternalFile (paste): image file ingested and auto-selected (dispatch `selected` with `{type:"drop", name:"pasted.png",...}`; grid item `[data-index="0"]` gets `selected`); non-image (`notes.txt`) rejected → nothing selected, `feedback` shown (×2).
- [B] getImageUrl with S3 enabled and the S3 option checked forwards the chosen key: `source.upload(file, resizeRatio "", "blog/hero.png")` → returns uploaded URL.
- [C] Dropzone keyboard: Enter and Space open the file picker; other keys ignored.

### `test/javascript/controllers/emoji_picker_controller.test.js` — subject: `app/javascript/controllers/emoji_picker_controller.js` (85 tests: A=16 B=0 C=69; P=0)
Data: built-in `allEmojis` rows `[shortcode, emoji, keywords]`, `allEmoticons` `[name, emoticon, keywords]`, `allIcons` (Phosphor) `[name, svgPath, viewBox, keywords]`; translated search data fetched from `/emoji-data/<locale>.json` (rows `{hexcode,label,unicode,tags}`) into `i18nSearchIndex: Map<unicode, {label,tags,searchTerms}>`. Column values: emoji 10, emoticons 5, icons 8 (from `data-emoji-picker-*-columns-value`).
- [C] connect: data arrays non-empty, `filteredItems = allEmojis`, `selectedIndex=0`, `activeTab="emoji"` (×4).
- [C] open(): showModal, clears input, resets to emoji tab and index 0, renders grid buttons with `data-emoji`, updates preview (×6). close() (×1).
- [C] switchToEmoji/Emoticons/Icons: sets tab, no-op (no `updateTabStyles`) if already active, resets index (×9).
- [A] Filtering (`onInput`): query = trim+lowercase; empty → all items of current tab; emoji tab → `searchEmojisWithI18n`; emoticons/icons → every whitespace-separated term must be a substring of `"<name> <keywords>"` (lowercased; keywords = last row element). Examples: emoji `heart`/`love`, emoticons `happy`/`smile`, icons `heart`/`love` match; `xyznonexistent123` → 0 items and grid shows t("status.no_matches"); index reset to 0 (×9).
- [C] getCurrentColumns per tab: 10 / 5 / 8 (×3).
- [C] Grid keys (`onKeydown`): ArrowRight +1 wrapping last→0; ArrowLeft −1 wrapping 0→last; ArrowDown +cols, from first item of last row wraps to same column in first row (→0); ArrowUp −cols, from first row wraps into last row; Tab NOT prevented and doesn't change tab (×9).
- [C] Tab-strip keys (`onTabKeydown`): ArrowRight cycles emoji→emoticons→icons→emoji; ArrowLeft reverse; others ignored (×7). Enter in grid selects current and dispatches `emoji-picker:selected`: emoji → `{text:":<shortcode>:", type:"emoji"}`; emoticon → `{text:<raw emoticon>, type:"emoticons"}` (×3).
- [C] Hover updates selection in place (no grid re-render, no scroll): selected style = `bg-[var(--theme-bg-hover)] ring-2 ring-[var(--theme-accent)]` (never `bg-[var(--theme-accent)]`); emoticons keep `text-[var(--theme-text-primary)]` (not accent-text); preview shows `:<shortcode>:`; same index → no-op (×4).
- [C] Click: emoji `data-shortcode="smile"` → `:smile:`; emoticon `data-emoticon` raw `(◕‿◕)`; icon `data-name="heart"` → `:ph-heart:` with type `icons` (×3).
- [C] Rendering: emoji buttons have `data-index/shortcode/emoji`; selected item styled; emoticon buttons `data-name/emoticon`, grid `grid-template-columns: repeat(5, …)`; icon buttons contain inline `<svg>`, `repeat(8, …)` (×6).
- [C] Preview: emoji shows glyph + `:smile:`; emoticon shows text + name; icon shows `<svg` + `:ph-heart:`; empty list → "" (×4).
- [C] dispatchSelected(text) → `{text, type:activeTab}` then closes (×2).
- [C] i18n: `i18nSearchIndex` null or Map, `i18nLoaded` boolean after connect; loadI18nData sets loaded=true and a Map keyed by unicode (`😀`,`😂`); fetch failure → empty Map; skipped if already loaded (×6).
- [A] searchEmojisWithI18n(query): terms = split on whitespace; match if every term is a substring of English `"<shortcode> <keywords>"` OR (if index has the emoji) every term is a substring of its translated `searchTerms` (e.g. pt-BR `feliz`→😀, `coração`→❤️, multi-word `rosto sorridente`→😀); English shortcode `grin` / keyword `happy`; no matches → []; works with null index (×7).
- [C] onInput uses i18n search only on the emoji tab (×2).

### `test/javascript/controllers/file_finder_controller.test.js` — subject: `app/javascript/controllers/file_finder_controller.js` (34 tests: A=5 B=0 C=29; P=0)
- [C] connect: `allFiles=[]`, `filteredResults=[]`, `selectedIndex=0`, `usingKeyboard=false`.
- [C] open(files): showModal, stores files, clears input, resets index (×3).
- [A] Initial/empty-query results = first 10 files in given order (20 files → 10).
- [A] onInput filter: query trim+lowercase; each file scored with `fuzzyScore(file.path.toLowerCase(), query)` (from `lib/text_utils`), keep score > 0, sort desc by score, max 10. `ban` → only `banana.md`; empty → all (3); matches on directory part (`folder` → `folder/cherry.md`); index reset (×4).
- [C] renderResults: empty → t("sidebar.no_files_found"); one `<button>` per result; directory part (`docs/notes`) shown for nested files; quoted path `photo" onerror="alert(1).md` cannot inject attributes (escaped) (×4).
- [C] Keys: ArrowDown/ArrowUp (preventDefault, clamp to [0,len-1], set `usingKeyboard`), Enter → selectCurrent (×5).
- [C] Hover: updates index; same index → no re-render; ignored in keyboard mode until mousemove resets `usingKeyboard` (×6 incl. keyboard/mouse group).
- [C] Selection dispatch `file-finder:selected` `{detail:{path}}` from click (`data-path`), from current selection, nothing when empty; dispatchSelected also closes (×4).
- [C] loadPreview: `fetch("/notes/<encodePath(path)>", {headers:{Accept:"application/json"}})`; shows first 10 lines of `content` HTML-escaped in a `<pre>` (appends `\n...` when ≥10 lines); empty results → ""; `!ok` or throw → escaped t("dialogs.file_finder.preview_error") (×5).

### `test/javascript/controllers/file_operations_controller.test.js` — subject: `app/javascript/controllers/file_operations_controller.js` (+ `lib/new_note_utils.js` previewNotePath/canCreateNote) (95 tests [two it.each counted 1 each: ×3 and ×2 rows]: A=13 B=0 C=82; P=0)
Endpoints: empty note `POST /notes/<encodePath(parent/name.md)>` body `{content:"", expanded}`; Hugo note `POST /notes` body `{template:"hugo", title, parent, expanded}`; folder `POST /folders/<path>?expanded=<enc csv>`; rename `POST /notes|folders/<old>/rename` body `{new_path, expanded}`; delete `DELETE /notes|folders/<path>?expanded=<csv>`. Errors come back as JSON `{error}` and are shown inline in the open dialog. Values: `hugoPathStyle` = `"dated"` (default) | `"flat"`.
- [C] connect: `contextItem=null`, `newItemType=null`, `newItemParent=""`, `newNoteTemplate="empty"` (×4).
- Context menu:
  - [C] showContextMenu positions at `clientX/clientY` px and stores `{path,type}`; right-click on a row already in a multi-selection keeps the selection (`contextSelection` = selection, no reselect); outside the selection → selects just that row (`selectExplorerItem(target)`, contextSelection `[{path,type,fileType}]`); not shown for config files (`data-file-type="config"`); hideContextMenu (×6).
- New note dialog:
  - [C] newNote() opens dialog; uses the single selected folder as parent (`docs/nested/sample.md` → `POST /notes/docs/nested/sample.md`); newFolder in that folder → `POST /folders/docs/nested/child?...`; `selectedCreationFolder()` = "" unless exactly one selected item and it is a folder; clears stale parent; resets input and template (empty card `aria-pressed="true"`, hugo `"false"`); Create disabled while name empty (×5).
  - [A] Toolbar Hugo template ignores the selected folder and targets `blog`: dated preview `blog/YYYY/MM/DD/<slug>/index.md` (regex `^blog\/\d{4}\/\d{2}\/\d{2}\/sample\/index\.md$`); switching to empty shows `<selected or t("dialogs.new_note.root")>/sample.md`; submit body `parent:"blog"` (it.each selection `""`,`blog`,`docs/nested`).
  - [A] Flat style: `My Post` → `blog/my-post.md`, body `{parent:"blog", title:"My Post"}`. Explicit destination (`openNewNoteDialog("content/posts")`) is kept for Hugo: `content/posts/sample.md`, parent `content/posts` (×2).
  - [C] newNoteInFolder: hides menu, opens dialog with folder parent; no-op for files or no context item (×3). [A] preview `parent/folder/my-note.md` (×1).
  - [C] Template cards: selecting hugo sets `aria-pressed`; selected card classes `border-[var(--theme-accent)] ring-1 bg-[var(--theme-bg-hover)]` (not `border-[var(--theme-border)]`) (×2). [A] dated Hugo preview for `my-post`; back to empty → `dialogs.new_note.root/my-post.md` (×2).
  - [A] Preview rules: empty name → `<root>/<t("dialogs.new_note.name_slot")>.md` with class `text-[var(--theme-text-faint)]`, Create disabled; `my-note` → `<root>/my-note.md`, muted class, enabled; names containing `/` invalid (`text-[var(--theme-error)]`, disabled, parent `title` = t("dialogs.new_note.invalid_name")); Hugo names that slugify to empty (`!!!`) invalid; `My Post` valid for Hugo (×5).
  - [C] closeNewNoteDialog closes and resets `newItemType=null`, parent "", template "empty" (×2).
  - [C] submitNewNote: no request for empty or slash names; empty doc → `POST /notes/test.md`; Hugo → `POST /notes` with `"template":"hugo"`, title, parent `blog` (or folder-scoped `docs`); dispatches `file-operations:file-created`; closes on success; failures inline (`newNoteError` text = server error, dialog stays open) (×8).
  - [C] Before posting, destination folder and ancestors are added to app expandedFolders so the tree reveals the new item (#268): note in `docs/nested` → body `expanded` = {docs, docs/nested, unrelated}; folder `sub` in `docs/deep` → URL `expanded=docs,docs/deep` (×2).
- New folder dialog:
  - [C] newFolder → `openNewItemDialog("folder","")`; location shows parent path (also as `title`) or t("dialogs.new_item.root_location"); newFolderInFolder from context folder (`parent/myfolder`), no-op otherwise (×2); dialog shown, type/parent stored, title t("dialogs.new_item.new_folder"); close resets; submit: empty → no request, `POST /folders/newfolder...`, dispatches `folder-created`, failures inline (×17 in group).
- Rename:
  - [C] renameExplorerItem({path,type}) / renameItem(): sets contextItem, hides menu, opens dialog; input = basename without `.md` for files (`folder/test.md`→`test`), folder name for folders (×6 incl. it.each).
  - [C] submitRename: empty → no request; files get `.md` appended (`newname.md` in body); dispatches `file-renamed` `{oldPath:"old.md", newPath:"new.md"}`; closes on success; errors inline; active file → `autosave.prepareForTransition()` before request and abort if it fails (×8 incl. closeRenameDialog).
  - [C] renamePath(item, name) (used by title rename): renames a captured item without touching dialog/contextItem → `{ok:true, path:"nested/renamed.md"}`; request failures return `{ok:false,error}` and `resumeAfterTransition()`, no dialog error; folders keep no extension (`parent/old`→`parent/new` via `/folders/parent/old/rename`) (×3). [A] blank/whitespace name → `{ok:false,error:t("errors.title_required")}` with no request; unchanged name after trim (` note `) → `{ok:true, path:"folder/note.md"}` with no request (×2).
- Delete:
  - [C] deleteItem hides menu; confirmation `appConfirm(t("dialogs.confirm.delete_file", …), {acceptLabel: t("common.delete"), destructive:true})`; folders use `dialogs.confirm.delete_folder` and `DELETE /folders/docs`; cancel → nothing; uses the item captured BEFORE the confirm even if contextItem changes; dispatches `file-deleted` `{path}`; active file → prepareForTransition first, aborted if it fails (×9).
  - [C] deleteItems(multi): ONE combined confirmation `dialogs.confirm.delete_multiple`, then sequential DELETEs in order (`/notes/first.md`, `/folders/docs`); descendants covered by a selected folder skipped (then single-folder message `delete_folder`); protected config `.fed` (fileType config) skipped; stops after first failure (`appAlert("delete failed")`), later rows left (×4).
- [C] Keyboard in rename / new-item / new-note inputs: Enter → preventDefault + submit; Escape → close (×6).

### Porting notes (JS controllers a–f)
- Counting: `it.each` tables are counted as 1 test each (rows noted in headers); per-file counts verified with grep and sum to 699.
- **Autosave contract is the single most important UI behaviour** to reproduce: 2000 ms debounce after the last keystroke, a forced save every 30000 ms while typing continuously, 300 ms local draft debounce, 1000 ms offline-backup debounce. The large-deletion guard blocks a save when `lost > 20%` AND `lost > 50 chars` of the last saved baseline. Drafts are keyed by path and anchored to a server `revision`. Every navigation, rename, move or delete runs `prepareForTransition()` first and aborts when the draft cannot be persisted. Stale async responses (load, save, tree refresh, rename) are discarded using generation counters. A desktop port can drop localStorage, but it must keep equivalent draft, conflict and backup recovery semantics, including rename collisions that preserve both drafts.
- Optimistic concurrency: deletes carry `expected_revision`; saves return `revision`; a 409 conflict surfaces as a 5000 ms temporary message. Undoing a newly created note (Ctrl+Z when history is exhausted, content equal to the server baseline) deletes the note after a custom confirm. Redo (Ctrl+Y / Ctrl+Shift+Z) recreates it with `POST /notes {path, content}`. Each note keeps its own undo history, held in an LRU cache of 20 editor states.
- Path rules used across the app: the folder-prefix test is `p == old || p.startsWith(old + "/")`, so a sibling such as `project-old` is never remapped. Library insertion computes a note-relative path (`../../images/photo%20%28one%29.png`) and percent-encodes spaces and parentheses. S3 URLs are rejected for `javascript:`, embedded credentials or an invalid URL. Library note links reject `..`, absolute paths and non-`.md` targets.
- Multi-selection semantics (Explorer, drag-drop, delete): items covered by a selected ancestor folder are dropped. Protected config `.fed` (`fileType: config`) is never deleted or renamed. Group operations run sequentially and stop at the first failure. The destination folder and all its ancestors are expanded before a create or move.
- Slash-command insertion: block inserts (image, table, video) widen the replace range by one adjacent char on each side and wrap the content in `\n` (image) or `\n\n` (table, video). Emoji inserts inline into the exact range. A range is stale when the doc text at from..to no longer equals the saved query, and nothing is inserted into non-Markdown files.
- Emoji picker output formats: `:shortcode:`, raw emoticon text, and `:ph-<name>:` for Phosphor icons. Search is multi-term AND substring matching over `"name keywords"`, with translated `searchTerms` from `/emoji-data/<locale>.json`.
- Code dialog block format is "```lang\ncontent\n```" (empty content gives a blank line). Its language suggestions use list order with substring matching, max 6, hidden when the first match equals the input exactly (a quirk).
- New-note paths: the toolbar Hugo template always targets `blog/` (dated `blog/YYYY/MM/DD/<slug>/index.md`, or flat `blog/<slug>.md`) unless a destination is passed explicitly (folder context menu or workspace). Names containing `/` are invalid. Hugo names that slugify to empty are invalid.
- Hard-coded English strings in the original, which a port may want to translate: content search status `"N matches found (showing first 20) - use ↑↓ to navigate, Enter to open"`. The file finder shows max 10 results and a 10-line preview.
- B-category items (AI grammar `/ai/config` + `/ai/fix_grammar`, AI image generation, S3 upload key) need mocked providers. Health polling of `/up` (HEAD, 5 s interval) is web-specific; the desktop app needs its own offline/online signal, and the editor must still be disabled while offline.


---

<!-- part totals: files=26 tests=647 A=128 B=4 C=515 P=0 -->

## Part 5 — JS Stimulus controllers (find_replace … video_dialog)

it.each tables counted per row (image_viewer ×3, local_images ×2, settings ×2). Counts verified with `grep -cE "^\s*(it|test)(\.each)?\("`.

### `test/javascript/controllers/find_replace_controller.test.js` — subject: `app/javascript/controllers/find_replace_controller.js` (8 tests: A=3 B=0 C=5; P=0)
- [C] On connect: `matches=[]`, `currentMatchIndex=-1`, `activeTab="find"`.
- [C] `open({textarea, tab:"replace"})` shows dialog (non-modal `show`/`showModal`), activeTab=`replace`, replace section un-hidden.
- [A] Matching is case-insensitive by default: text `"Hello hello"`, query `hello` → 2 matches; counter text = t(`dialogs.find_replace.match_count`,{current:1,total:2}) → "1 of 2" (current index 1-based, first match selected).
- [C] `findNext()` dispatches `find-replace:jump` `{start:0,end:3}` for "one one"/"one" with caret at 0 (picks closest match at/after `selectionEnd`, via `lib/find_utils.findClosestMatchIndex`; findAll picks closest at/after `selectionStart`). Invalid regex → counter shows validation error with `text-red-500`.
- [A] Regex mode (`useRegexValue=true`): find `(\w+)-(\w+)`, replace `$2-$1` on "foo-bar" → `replace` event detail `{start:0,end:7,replacement:"bar-foo"}` (JS `$n` group substitution resolved per match).
- [A] `replaceAll()` dispatches `replace-all` with `detail.updatedText` = whole text with all matches replaced ("a a" a→b ⇒ "b b").
- [C] After replace-all, counter shows t(`dialogs.find_replace.replaced_count`,{count}) "Replaced 3" with class `text-green-500`; feedback cleared (class removed) after 2000 ms.
  (2 tests: feedback, timeout.)

### `test/javascript/controllers/folder_images_controller.test.js` — subject: `app/javascript/controllers/image_sources/folder_images_controller.js` (2 tests: A=0 B=0 C=2; P=0)
- [C] `browse()`: if `window.showDirectoryPicker` rejects → inline error target shows t(`dialogs.image_picker.folder_access_failed`) and is un-hidden; the surrounding picker `<dialog>` stays open.
- [C] If iterating directory `values()` throws → inline error t(`dialogs.image_picker.folder_read_failed`), dialog stays open. (Rust: native folder picker; errors inline, not closing the image picker.)

### `test/javascript/controllers/help_controller.test.js` — subject: `app/javascript/controllers/help_controller.js` (5 tests: A=0 B=0 C=5; P=0)
- [C] `openHelp()` → help dialog `showModal`; `closeHelp()` → `close`.
- [C] `onKeydown`: `Escape` closes; other keys (Enter) do nothing.
- [C] Click whose target is the dialog element itself (backdrop) closes the dialog.

### `test/javascript/controllers/home_controller.test.js` — subject: `app/javascript/controllers/home_controller.js` (7 tests: A=0 B=0 C=7; P=0)
- [C] Recent-files backend contract: `record(path)` → `POST /recent_files` JSON body `{"path":"docs/one.md"}`; records are serialized in call order; `refresh({force:true})` → `GET /recent_files` returns server-rendered HTML injected into `files` target.
- [C] Network failure on refresh keeps previously displayed list, shows status message from `data-error-message` ("History unavailable"); later `record` still works.
- [C] Stale-response guard: an older in-flight refresh resolving after a newer one is ignored.
- [C] `open(event)` with plain left click calls app `loadFile(path)` from `data-path`; modified clicks (metaKey etc.) are not intercepted (browser default).
- [C] `clear()` → `DELETE /recent_files` (after confirm, `data-clear-confirmation`) then `GET` refresh.
- [C] "Show more/less": list renders 7 items, items index ≥5 hidden (`data-home-target="extraFile"`), i.e. 5 visible by default. Show more → all 7 visible, showMore hidden + `aria-expanded="true"`, focus moves to first revealed link ("Note 5"), showLess revealed and `scrollIntoView({behavior:"smooth",block:"end"})`. Expanded state persists across refresh; Show less → 5 visible, focus back on Show more, collapsed state persists across refresh.
- [C] With `prefers-reduced-motion` (matchMedia matches) scroll uses `behavior:"instant"`.

### `test/javascript/controllers/image_picker_controller.test.js` — subject: `app/javascript/controllers/image_picker_controller.js` (7 tests: A=0 B=0 C=7; P=0)
- [C] connect loads `/images/config` (`{s3_enabled, image_extensions}`).
- [C] `openWithFile(file)` (paste): opens picker, gets `drop-images` source, shows loading t(`dialogs.image_picker.processing_image_paste`), calls `loadExternalFile(file)`, hides loading; if drop source missing → open only, no loading; loading hidden even if ingest throws (error re-thrown).
- [C] Tab keyboard nav: ArrowRight from Upload tab → `currentTab="local"`, focus on Library tab, `aria-selected="true"`, other tab `tabIndex=-1`, local panel shown (WAI-ARIA tabs).
- [C] Insert failure stays inline: loading text t(`dialogs.image_picker.processing_image`), error target t(`dialogs.image_picker.insert_failed`) shown, no `alert()`, media-storage outlet `refresh()` called once.
- [C] Successful insert dispatches `image-picker:image-selected` detail `{markdown:"![photo](images/photo.png)", imageUrl:"images/photo.png", altText:"photo", linkUrl:""}` (markdown = `![alt](url)`).
- [C] Insert waits for `mediaStorage.ready()` (pending provider save) before calling source `getImageUrl()`.

### `test/javascript/controllers/image_viewer_controller.test.js` — subject: `app/javascript/controllers/image_viewer_controller.js` (7 tests: A=0 B=0 C=7; P=0) (it.each ×3 counted as 3)
- [C] `open({detail:{src,alt,trigger,fallback}})`: showModal, focus Close button, status = loading text (`data-image-viewer-loading-value`), fresh `<img>` with alt; on `load` image unhidden, status cleared; `close()` restores focus to trigger and empties stage.
- [C] If trigger was removed from DOM (preview rerender), focus goes to `fallback` element.
- [C] Stale resource events from a previous image are ignored; `error` on current image shows error text (`data-image-viewer-error-value` "Unavailable"), dialog stays open.
- [C] ×3 Rejects unsafe sources (no dialog, empty stage): `javascript:alert(1)`, `file:///secret.png`, `data:text/html,<script>…`.
- [C] Alt text is set as text attribute (no HTML injection; one `<img>`, no `onerror`).

### `test/javascript/controllers/jump_to_line_controller.test.js` — subject: `app/javascript/controllers/jump_to_line_controller.js` (5 tests: A=5 B=0 C=0; P=0)
- [A] Line-target resolution (text "one\ntwo\nthree\nfour\nfive", caret on line 2, 5 lines). Rules (from `jump()`): trim input; empty → no-op; starts with `+`/`-` → relative: `parseInt`; NaN→no-op; literal `"-1"` → last line; else current+delta; otherwise absolute `parseInt`; result clamped to `[1,totalLines]`; dialog closed then dispatch `jump-to-line:jump` `{lineNumber}`.
  Cases: `"4"`→4; `"+2"`→4; `"-2"`→1 (2-2=0 clamped); `"-1"`→5 (EOF special); `"100"`→5.

### `test/javascript/controllers/library_controller.test.js` — subject: `app/javascript/controllers/library_controller.js` (20 tests: A=2 B=0 C=18; P=0)
Backend endpoints exercised (Rust must serve equivalents): `GET /library` → `{images:[…], videos:[…]}` items `{name,path,size,mtime,preview_url,type:"image"|"video",provider?,url?}`; `GET /library/usage` → `{usage_counts:{path:n}}`; `GET /library/usage?path=<encodeURIComponent>` → `{usage_notes:[…]}`; `DELETE /library/file/<encodePath(path)>`; `PATCH /library/file/<path>` body `{name}` → `{name,path,url}`; error bodies `{error:"…"}`.
- [C] Library is not fetched on connect (only on explicit `load()` when workspace opens) — exactly one `GET /library`.
- [C] Renders one `.library-card` per item of the active collection (images tab default): `<img src>` = `/notes/<encodePath(path)>` for local (`/notes/images/photo%20one.png`), size "2 KB"; card buttons actions in order openPreview, deleteItem, insertItem. Videos tab → `aria-pressed="true"`, `<video src="/notes/videos/clip.mp4">`, "4 KB".
- [C] Usage lookup is lazy: IntersectionObserver (root = controller element) — no `/library/usage` until a card intersects; label state `checking` → one shared request for all cards → `used`/`unused` labels ("Used in N notes" via t(`library.usage_used`,{count})); unused label has no button; videos reuse cached map (no 2nd request).
- [C] Label states: `used` (count>0, clickable button), `unused` ("Used in 0 notes"), `unknown` on non-ok response (text t(`library.usage_unknown`)); `resetUsageForLibraryOpen()` resets to `checking` and refetches on reopen (2 requests total).
- [C] Usage dialog: clicking count opens dialog (title t(`library.usage_dialog_title`), not containing filename), focus on close button; fetches usage notes; renders only valid note paths (drops `../outside.md`) as `button[data-note-path]`; focus trap Tab/Shift+Tab cycles; clicking a note dispatches `library:open-note` `{path}` and closes dialog.
- Note (asserted inside the C usage-dialog test): usage-note path validity (`isValidUsageNotePath`): string, not starting `/`, ends with `.md`, every `/` segment non-empty, not `.`/`..`, no `\`. Dedupe preserving order.
- [C] Preview dialog: name, metadata "<size> · <date> · PNG" (uppercase extension); after image load adds aspect ratio "16:9" (1920×1080) → `… · 16:9 · PNG`; usage button ("Used in 2 notes", `data-path`) placed before metadata; usage dialog close restores focus to usage button; video preview `<video muted>`; video with no usage → "Used in 0 notes" no button.
- [C] Preview focus trap (Shift+Tab from close → last button "insert", Tab from last → close, defaultPrevented) and focus restored to card trigger on close.
- [C] Loading target visible during load; empty → grid text t(`library.empty`); fetch rejection → error target shows error message ("offline").
- [A] Item validation (`isValidItem`): name/path strings, type image|video, path has ≥2 segments all safe (no empty/./../backslash); local items must have no provider and root segment `images` (image) / `videos` (video); S3 items (`provider:"s3"`) root `frankmd`, `url` http(s) without userinfo. Rejects `images/../secret.png`, `https://…` paths, `foreign/…`, `frankmd/../…`, `javascript:` urls.
  (tests: "discards malformed media paths", "rejects malformed or foreign S3 items")
- [C] Items shown in server order (no client sort): zebra.png before photo one.png.
- [C] Copy path writes stored relative path (`images/photo one.png`) to clipboard, status t(`library.copied`); clipboard rejection shows error.
- [C] Delete: `appConfirm(t("library.delete_confirm"),{acceptLabel:t("library.delete"),destructive:true})` → DELETE; success removes item; `{ok:false, error}` shows error; cancel → no request.
- [C] S3 items: `<img src=item.url>`, usage state `unknown` (no scan for S3), rename via `appTextPrompt` → PATCH body `{name:"renamed.png"}`, item updated from response. (rename button only for managed/S3 objects)
- [C] `insertPreviewItem()` dispatches `library:insert-media` `{item,status:"pending"}`; when listener sets `status="inserted"` the preview closes.
- [C] XSS ×3: card, usage-label button and usage-note buttons escape quoted names/paths (`photo" onerror="alert(1)`) — no `onerror` attribute, data attrs hold raw strings.
- Note: `formatBytes`: <1024 → "N B"; else KB/MB/GB/TB, integer or ≥10 → toFixed(0), else toFixed(1).

### `test/javascript/controllers/locale_controller.test.js` — subject: `app/javascript/controllers/locale_controller.js` + `window.t` (38 tests: A=13 B=0 C=25; P=0)
- [A] Static locale list (×3): first is `en`; each has `id,name,flag`; includes en, pt-BR, es, ja. Full list in source order: en "English" us, pt-BR "Português (Brasil)" br, pt-PT "Português (Portugal)" pt, es "Español" es, he "עברית" il, ja "日本語" jp, ko "한국어" kr.
- [C] connect: `currentLocaleId` from `data-locale-initial-value` (default "en"); menu rendered with one button per locale; fetches `GET /translations` (Accept: application/json).
- [C] loadTranslations (×4): response `{locale, translations}` → sets `window.frankmdTranslations`, `window.frankmdLocale`, dispatches window event `frankmd:translations-loaded` detail `{locale, translations}`; fetch error → `console.warn`, no throw.
- [C] toggle (×2): toggles menu hidden; stops propagation.
- [C] selectLocale (×4): sets currentLocaleId, hides menu, calls updateDisplay+renderMenu immediately; same locale → no save.
- [C] updateDisplay (×3): label = locale name ("日本語"); checkmark `.checkmark` visible (no `opacity-0`) only on current; unknown id → falls back to "English".
- [C] renderMenu (×3): button per locale with `data-locale` in list order; flags 🇺🇸 🇧🇷 🇯🇵 present.
- [A] getFlag (×3): "us"→🇺🇸, "br"→🇧🇷, unknown → "" (country code → regional-indicator flag).
- [C] saveLocaleConfig (×2): `PATCH /config` JSON `{"locale":"ja"}` (Content-Type+Accept JSON) then page reload — reload happens even if save fails. (Rust: persist `locale` to .fed then re-render UI.)
- [C] config listener (×2): window `frankmd:config-changed` `{locale}` different from current → set + reload translations; same → ignored.
- [C] Document click outside closes menu.
- [C] disconnect removes `frankmd:config-changed` listener and disconnects dropdown.
- [A] `window.t(key, opts)` (×7; NB test re-defines an identical copy): no translations → returns key; simple and dotted nested lookup ("level1.level2.level3"); missing path → key; `%{name}` interpolation ("Hello John, you have 5 messages"); missing option leaves `%{name}` literally; non-string value (object) → key.

### `test/javascript/controllers/local_images_controller.test.js` — subject: `app/javascript/controllers/image_sources/local_images_controller.js` (9 tests: A=0 B=0 C=9; P=0) (it.each ×2 counted as 2)
- [C] `activate()` loads managed collection by default without stealing focus: `GET /library/picker?collection=managed&search=&limit=10&offset=0` with Accept `text/vnd.turbo-stream.html, text/html, application/xhtml+xml` (server-rendered grid; Rust: equivalent query with collection/search/limit/offset).
- [C] Changing collection/page size clears selected image, resets offset 0: `collection=server&search=photo&limit=25&offset=0`; `nextPage()` → `offset=25`.
- [C] `getImageUrl()` returns existing reference without upload: managed → `data-url` ("images/photo.png"); server collection → its url ("/images/preview/photo.png"); no fetch.
- [C] Server image + S3 option checked → `POST /images/import` body `{"path":"photo.png","resize":"","s3_key":"folder/photo.png"}`, returns response `url`.
- [C] ×2 Delete via `appConfirm("dialogs.image_picker.delete_confirm",{acceptLabel:"common.delete",destructive:true})` then `DELETE /library/file/<path>` (managed) or `DELETE /images/file/<path>` (server).
- [C] Cancelled confirm → no request.
- [C] Failures inline: delete `{ok:false,error:"delete failed"}` → error text; load rejection → "offline"; next successful load clears error.
- [C] `configure(false)` (server images unavailable) disables `server` option and forces `managed`.

### `test/javascript/controllers/log_viewer_controller.test.js` — subject: `app/javascript/controllers/log_viewer_controller.js` (28 tests: A=0 B=0 C=28; P=0)
Endpoints: `GET /logs/tail?lines=100` → `{environment,file,lines:[…]}`; `GET /logs/config` → `{config_file,config_file_exists,ai_configured_in_file,environment,entries:[{key,value,source,env_var,sensitive}]}`. (Shortcut Ctrl+Shift+O lives in app_controller.)
- [C] open (×3): showModal; immediately shows t(`dialogs.log_viewer.loading`) "Loading..."; default tab `logs`.
- [C] fetchLogs (×8): requests `/logs/tail?lines=100`; environment label "production - production.log" (`<env> - <file>`); content = lines joined by "\n"; empty → t(`empty_logs`) "(log is empty)"; status "5 lines" (t `line_count`) / singular "1 line" (t `line_count_one`,{count:1}); rejected fetch or non-ok (500) → t(`error_loading_logs`,{error:message}).
- [C] fetchConfig (×6): requests `/logs/config`; renders table with key/value/source (`.fed` label, `default` source); status "2 keys"/"1 key" (`key_count`/`key_count_one`); labels "Config file:", "File exists:", "AI configured in .fed:", empty entries → t(`empty_config`); missing config file label t(`no_config_file`); error → t(`error_loading_config`,{error}).
- [C] Tabs (×5): config loading text localized; showLogs shows `<pre>` hides config, title t(`title_logs`) "Rails Log"; showConfig inverse, title "Configuration", activeTab `config`.
- [C] refresh (×2): re-fetches the active tab's endpoint.
- [C] close (×1); backdrop click closes, inner click doesn't, Escape closes (×3).

### `test/javascript/controllers/media_storage_controller.test.js` — subject: `app/javascript/controllers/media_storage_controller.js` (8 tests: A=0 B=0 C=8; P=0)
Config contract: `GET /config` (refresh) / `PATCH /config` body `{"media_storage_provider":"s3"|"local"}`; response `{settings:{media_storage_provider}, features:{s3_upload:bool}, media_storage_fallback?:bool}`.
- [C] Provider change saves immediately; all provider selects disabled while saving; `ready()` promise (used by uploads) resolves only after ack; then every select shows new value, re-enabled.
- [C] Failed save (`ok:false`): returns false, waiting `ready()` rejects with t(`media_storage.save_failed`), selects restored to previous value, every error target shows the message.
- [C] Saved `s3` but `features.s3_upload=false` (credentials gone): keep showing `s3`, disable the s3 option, `s3Only` targets stay visible, no warning dialog; later refresh to local syncs.
- [C] Server-acknowledged fallback (`media_storage_fallback:true`, provider local while previously s3) → selects set to local, s3 disabled, warning dialog shown once (next tick), not repeated for re-apply/subsequent refreshes; auto-closes after 6000 ms.
- [C] Initial page `fallbackValue=true` → warning on connect; `dismissWarning()` closes early (close called once total).
- [C] `refresh(true)` (caller already knows a recovery happened because Settings' first `GET /config` consumed the one-shot `media_storage_fallback` flag): if resulting provider is `local` → warning shown.
- [C] Explicit switch to Local or failed refresh → no warning.
- [C] Network rejection: providerValue unchanged ("local"), error target shows "offline".

### `test/javascript/controllers/media_storage_menu_controller.test.js` — subject: `app/javascript/controllers/media_storage_menu_controller.js` (4 tests: A=0 B=0 C=4; P=0)
- [C] Selecting S3 in the custom menu: trigger+options disabled while saving, label keeps "Local" until ack, then "S3", option `aria-checked="true"`, linked `<select>` synced to `s3`.
- [C] Failed save keeps "Local", re-enables trigger, error t(`media_storage.save_failed`).
- [C] Saved S3 without credentials: label "S3", S3 option disabled with title = unavailable-message value; when available again title cleared, Local enabled.
- [C] Keyboard: open → `aria-expanded="true"`, focus first option; ArrowDown moves focus; Escape (defaultPrevented) closes and refocuses trigger; outside click closes (`aria-expanded="false"`).

### `test/javascript/controllers/offline_backup_controller.test.js` — subject: `app/javascript/controllers/offline_backup_controller.js` (8 tests: A=8 B=0 C=0; P=0)
Draft backup store (browser localStorage; Rust: equivalent local KV/drafts store).
- [A] `save(path, content)` writes key `frankmd:backup:<path>` = JSON `{"content":…, "timestamp":<Date.now() ms>}`.
- [A] Storage write failure (quota) is swallowed; `console.warn("localStorage backup failed:", err)`.
- [A] `check(path, serverContent)`: no backup → null; differs → `{content,timestamp}`; equal → null AND backup deleted; corrupt JSON → null AND deleted. (4 tests)
- [A] `clear(path)` removes only that key; `clearAll()` removes all `frankmd:backup:*` keys, leaves unrelated keys.

### `test/javascript/controllers/path_display_controller.test.js` — subject: `app/javascript/controllers/path_display_controller.js` (12 tests: A=0 B=0 C=12; P=0)
- [C] connect installs (debounced) resize handler.
- [C] `update(null)` → text t(`editor.select_note`), `data-full-path=""`; `update(path)` stores full path in `data-full-path`; shows full path when it fits (no `truncated` class).
- [C] Basename mode (`basenameValue`): "nested/<note>.md" → text "<note>" (extension stripped, as text not HTML), `title` = full path, `aria-label` = "<renameLabel>: <note>"; copy copies full path; null → title = t(`editor.select_note`).
- [C] copy(): writes `data-full-path` to clipboard; nothing if empty; toast `#app-toast` text t(`status.copied_to_clipboard`) removed after ≤2300 ms (duration + fade).
- [C] showFull only when `.truncated` (shows full path); hideFull re-runs `update(fullPath)`.
- [C] `debounce(fn,100)` collapses 3 calls into 1 after 100 ms.

### `test/javascript/controllers/preview_controller.test.js` — subject: `app/javascript/controllers/preview_controller.js` (89 tests: A=6 B=0 C=83; P=0)
- [A] Zoom levels exactly `[50,75,90,100,110,125,150,175,200]`; zoomIn steps to next (100→110), capped at 200; zoomOut to previous (100→90), floored at 50. (5 tests)
- [C] Initial zoom from `data-preview-zoom-value` (100) → label "100%"; `applyZoom` sets content `font-size: <zoom>%` and label; `zoomValueChanged` → applyZoom; zoomIn/Out dispatch `preview:zoom-changed` `{zoom}`. (6 tests)
- [C] Image activation (×3): after sanitization (strips `onerror`), standalone images wrapped in `<button type="button">`; click dispatches `preview:open-image` `{alt, src, trigger}`; images inside links keep link navigation and get a sibling `button.preview-image-enlarge`; `<picture>` (with `<source>`) kept intact inside the button.
- [C] toggle/show/hide/isVisible (×10): toggle returns new visibility, swaps `hidden`/`flex`, adds `body.preview-visible`, dispatches `preview:toggled` `{visible}`; show/hide no-op (no dispatch) if already in state.
- [A] `render("# Hello\n\nWorld")` produces `<h1 …>Hello</h1>` and `<p …>World</p>` (marked) with `data-source-line` attrs.
- [C] render (×4): no-op when hidden; empty → ""; sets `_isUpdatingContent=true`, cleared 100 ms after the LAST render (rapid renders extend window) — scroll events during that window are not dispatched.
- [C] update (×2): `update(md,{scrollRatio})` → render + syncScrollRatio; typewriter options → `syncToTypewriter(currentLine,totalLines)`.
- [C] syncScrollRatio (×5 + 4 edge + several signal tests): scrollTop = ratio × (scrollHeight−clientHeight) (0.5 → 300 of 600) via rAF; ratio ≤0.01 → exactly 0; ≥0.99 → exactly bottom; exact 0/1 handled; no-op when hidden or `syncScrollEnabledValue=false`; always dispatches `preview:programmatic-scroll`.
- [C] syncToLine (×2): smooth `scrollTo({top, behavior:"smooth"})`, ratio (line−1)/(total−1); no-op when totalLines ≤1.
- [C] syncToTypewriter (×3): aligns block with `data-source-line == cursor line` at pane top (top = rect.top − content.top + scrollTop → 380); fallback (line−1)/(total−1)×scrollHeight (≈444.444); no-op total ≤1; two rAF frames.
- [C] setTypewriterMode toggles class `preview-typewriter-mode`. (2)
- [C] Programmatic-scroll notification (×5+): syncToLineSmooth, syncScrollRatio, syncToTypewriter, syncToLine all dispatch `programmatic-scroll` (lock lives in scroll-sync; preview has no private `_scrollSource`).
- [C] onPreviewScroll: dispatches `preview:scroll` `{scrollRatio, sourceLine (top visible data-source-line or null), totalLines, typewriterMode}` (e.g. 50/(200−100)=0.5; totalLines from last render, 1 for "# Hello World"); suppressed when sync disabled or `_isUpdatingContent`; `_getPreviewScrollRatio` = scrollTop/(scrollHeight−clientHeight), 0 when not scrollable.
- [C] Content caching (×5): `_lastRenderedContent` starts null; `show()` invalidates; `updateWithSync` (debounced ~150 ms) skips render if content unchanged but still syncs when cursor line changes.
- [C] Unified line mapping (×6): source-line anchors; typing path `syncToLineSmooth(line,total)` scrolls to anchor top − 50 px padding (smooth); scroll path ratio→line `round(ratio×totalLines)+1` and anchor at viewport top (scrollTop=100); invariant to container page offset; front matter stripped but anchors use absolute lines (body `# First` at line 5 after 4 front-matter lines); fallback to element-count ratio when no annotations; explicit sourceLine arg overrides ratio.
- [C] Sync toggle (×4): `syncScrollEnabledValue=false` blocks ratio path, typing path, preview→editor, typewriter.
- [C] Task checkboxes (#203) (×4): rendered task checkboxes have `disabled` stripped; each task `<li>` gets `data-source-line` (lines 3,4 for "# Title\n\n- [ ] open\n- [x] done\n"); click dispatches `preview:toggle-task` `{line}` and reverts the visual check (editor is source of truth); checkboxes outside annotated `li` ignored.

### `test/javascript/controllers/recovery_diff_controller.test.js` — subject: `app/javascript/controllers/recovery_diff_controller.js` (7 tests: A=0 B=0 C=7; P=0)
- [C] `open({path,serverContent,backupContent,backupTimestamp})` renders word diff: server pane has `ai-diff-del` ("world") + `ai-diff-equal`; backup pane `ai-diff-add` ("universe") + equal ("hello world" vs "hello universe").
- [C] Timestamp badge shows non-empty locale-formatted date; dialog `showModal`.
- [C] Escape/`cancel` is prevented (must choose explicitly); dialog stays open.
- [C] `acceptServer()` dispatches `recovery-diff:resolved` `{source:"server", path, draftRevision, backupContent, backupTimestamp}` and closes.
- [C] `acceptBackup()` dispatches `{source:"backup", path, content, backupContent, draftRevision:null, backupTimestamp}`; with `source:"draft-conflict"`, `draftRevision`, `conflictId` given, they are echoed back (`source:"draft-conflict"`, `conflictId:"conflict-1"`).

### `test/javascript/controllers/s3_option_controller.test.js` — subject: `app/javascript/controllers/s3_option_controller.js` (22 tests: A=0 B=0 C=22; P=0)
- [C] connect hides resize option; show/hide toggle element `hidden`; hide resets checkbox, resize select to default `0.5`, hides resize option.
- [C] Checkbox change: checked → show resize option (and key option); unchecked → hide and reset select to `0.5`.
- [C] `isChecked`; `resizeRatio` = "" when unchecked else select value ("0.75"); resize options 0.25/0.5/0.75/1.
- [C] Destination key (×4): `setDefaultKey("frankmd/2026/07/photo.png")` prefills key input when checked (default S3 key shape `frankmd/YYYY/MM/<file>`); `keyValue` = trimmed input only when checked; manual edit (`onKeyInput`) not clobbered by later default; reset hides key option and clears input.
- [C] reset (×3): unchecks, hides resize, select back to `0.5`.
- [C] Without targets (×4): connects, isChecked false, resizeRatio "", reset doesn't throw.

### `test/javascript/controllers/scroll_sync_controller.test.js` — subject: `app/javascript/controllers/scroll_sync_controller.js` (33 tests: A=0 B=0 C=33; P=0)
Single scroll lock `_scrollSource ∈ {null,"editor","preview"}` cleared 400 ms after last mark.
- [C] onEditorScroll (×6): `preview.syncScrollRatio(ratio, editorTopVisibleLine)` (null if unknown); typewriter mode → `syncToTypewriter(currentLine,totalLines)` instead; skipped if preview hidden or lock = preview; marks lock `editor`.
- [C] onPreviewScroll (×7): with sourceLine → `editor.scrollToLine(sourceLine)`; else ratio × maxScroll (`height−clientHeight`) with ratio ≤0.01 → 0 and ≥0.99 → maxScroll (500); maxScroll 0 → no scroll; blocked when lock = editor; marks lock `preview`.
- [C] onProgrammaticPreviewScroll (×3): marks lock `editor`, blocks late echoes; lock expires after 400 ms then user preview scroll syncs.
- [C] Feedback loop (×2): editor scroll blocks reverse; lock clears after 400 ms.
- [C] updatePreviewWithSync (×3): normal → `preview.render(content)`; typewriter → `updateWithSync`; nothing if hidden. updatePreview (×2) same visibility guard.
- [C] onPreviewToggled (×3): visible → render + `syncScrollRatio(editorRatio, topLine)`; hidden → nothing; holds editor lock for echo then clears after two animation frames (before 400 ms).
- [C] onTypewriterToggled tracks `typewriterModeEnabled` (×1).
- [C] Re-anchor after resize/zoom (×6): typewriter → syncToTypewriter; editor lock → preview.syncScrollRatio(ratio, editorTopLine); preview lock → editor.scrollToLine(preview top source line) or ratio fallback (0.5×500=250); window resize debounced 200 ms; ResizeObserver on preview content re-anchors while lock held.

### `test/javascript/controllers/settings_controller.test.js` — subject: `app/javascript/controllers/settings_controller.js` (+ `app_controller.js` settings bits) (62 tests: A=4 B=0 C=58; P=0) (it.each ×2 counted as 2)
Setting names (`.fed` keys via `PATCH /config`, whitelisted server-side to `Config::UI_KEYS`): `editor_font`, `editor_font_size`, `editor_width`, `editor_line_numbers` (0 off/1 absolute/2 relative), `show_stats`, `editor_indent`, `preview_zoom`, `vim_mode`, `typewriter_mode`, `cursor_recenter_on_click`, `scroll_sync`, `theme`, `view_mode` (`split`|`single`). Defaults: font `cascadia-code`, size 14, width 72, zoom 100, line numbers 0, show_stats false, typewriter false, recenter false, vim false, scroll_sync true, indent 2, theme "", view_mode split.
- [A] editorFonts (×2): first `cascadia-code`; each `{id,name,family}`. Full list: cascadia-code "'Cascadia Code', monospace", consolas "Consolas, monospace", dejavu-mono "'DejaVu Mono', monospace", fira-code "'Fira Code', monospace", hack "Hack, monospace", jetbrains-mono "'JetBrains Mono', monospace", roboto-mono "'Roboto Mono', monospace", source-code-pro "Source Code Pro, monospace", ubuntu-mono "Ubuntu Mono, monospace".
- [C] connect (×3): sets CSS var `--editor-width: 72ch` on `<html>`; no throw without outlets; readiness flags false.
- [C] Value-change guards (×7): font/fontSize/lineNumbers changes don't apply until CodeMirror ready; previewZoom/scrollSync until preview ready; editorWidth applies CSS-only (`80ch`); theme change dispatches window `frankmd:config-changed` `{theme}`.
- [C] Outlet connected (×4): CodeMirror ready → applyFont, applyLineNumbers, applyCursorRecenterOnClick; preview ready → applyPreviewZoom, applyScrollSync; afterwards value changes apply.
- [C] applyEditorWidth `100ch`; applyTheme dispatches `frankmd:config-changed` `{theme}` only if theme non-empty (×2); applyScrollSync sets preview `syncScrollEnabledValue`, null-safe (×2); applyFont → `setFontFamily("'Fira Code', monospace")`, `setFontSize(16)` (×1).
- [C] Getters (×8): currentFont, currentFontSize 14, editorWidth 72, previewZoom 100, lineNumberMode 0, typewriterModeEnabled false, scrollSyncEnabled true, fonts.
- [C] Categories (×3): nav buttons general/editor/writing/preview/about — showCategory shows only that section, `aria-pressed` on nav; About is a section inside Settings.
- [C] Dialog lifecycle (×4): `openDialog()` → true, showModal once, syncs controls, focus first nav button, dispatches `settings:dialog-state-changed` `{open:true}`; `closeSettings()` → true + `{open:false}`; backdrop click closes, inner clicks don't; closing when closed → false.
- [C] Persist via PATCH /config (×12), saves debounced 500 ms (latest call's body replaces pending one) — bodies: `{editor_font:"fira-code"}`, `{editor_font_size:16}`, `{editor_width:96}` (+ `--editor-width:96ch`, label "96ch"), `{editor_line_numbers:2}`, `{show_stats:true}` (toggle shows stats panel and updates it; second toggle hides), `{cursor_recenter_on_click:true}` (live `setCursorRecenterOnClick(true)`, aria-pressed), `{editor_indent:4}`, `{preview_zoom:150}` (live preview zoomValue), `{view_mode:"single"}` (app.setViewMode). View-mode buttons sync aria-pressed. Successful save dispatches window `frankmd:config-file-modified`; non-ok shows t(`settings.save_failed`) status for 2 s.
- [A] `viewMode` getter: unknown value ("garbage") → `split`.
- [C] reload (×5): `GET /config` `{settings:{…}}` re-syncs all values & controls (width label "120ch", select values as strings); unknown view_mode ("diagonal") → split; missing `scroll_sync` leaves current (true); fetch rejection swallowed; when response has media-storage info, delegates to media-storage outlet `refresh(true)` rather than `apply` (avoids stale response).
- [A] syncControls zoom select: persisted zoom not in level list (97) → select shows `100`.
- [C] syncControls font preview gets family containing "JetBrains Mono" and `20px`; openDialog syncs controls.
- [C] AppController settings (×5): ×2 opening Settings overlays editor without hiding editor or changing preview visibility (false/true), settings toggle button has no aria-pressed, second open returns false; overlays Library without changing Library state (`libraryToggle aria-pressed="true"`, preview hidden while Library shown); background shortcuts (e.g. `togglePreview`) blocked while Settings open; vim/scroll-sync/typewriter switch `aria-pressed` kept in sync.

### `test/javascript/controllers/stats_panel_controller.test.js` — subject: `app/javascript/controllers/stats_panel_controller.js` (uses `lib/stats_utils`) (19 tests: A=6 B=0 C=13; P=0)
- [C] connect: `updateTimeout=null`; show/hide toggle panel `hidden`. (3)
- [A] `update(text)` (×6): words "Hello world test"→"3"; chars "Hello"→"5"; size label ends with "B" (byte size formatting); read time label contains "min"; "" → words "0", chars "0"; null → words "0".
- [C] scheduleUpdate (×3): debounced 500 ms, each call replaces previous timeout, eventually `update(text, undefined)`.
- [C] onUpdate (×2): `{detail:{text}}` → scheduleUpdate; `immediate:true` → update directly (second arg = cursor info).
- [C] disconnect clears timeout (×1).
- [C] Length cache (×4): `_lastContentLength` starts -1; QUIRK: if new text length equals last length ("Hello"→"World") stats are NOT recomputed (word/char targets untouched); different length recomputes ("Hello World" → 2 words, length 11); line position target always updated: `"5/10"` = `currentLine/totalLines`.

### `test/javascript/controllers/table_editor_controller.test.js` — subject: `app/javascript/controllers/table_editor_controller.js` (uses `lib/table_utils.parseMarkdownTable`) (40 tests: A=26 B=0 C=14; P=0)
- [A] `parseCount(str, fallback)` (×4): "5"→5; "0"/"-5"→1 (min 1); "abc"/"" → fallback; "3.9"→3 (parseInt).
- [C] syncDimensionInputs (×2): inputs reflect `tableData` (cols = row0 length, rows = row count); explicit `(rows, cols)` args.
- [A] setColumnCountTo (×5 A): growing adds header cells t(`dialogs.table_editor.header_n`,{n}) → "Header 4","Header 5" and "" for body rows; shrinking truncates every row; min 1 (0, -5 → 1); empty table → creates one row of N; fractional 2.9 → 2 (floor). [C] same count → no re-render (×1).
- [A] increment/decrementColumnCount (×3): ±1, never below 1.
- [A] setRowCountTo (×5 A): growing appends rows of "" matching column count; shrinking keeps header row; min 1; empty → N rows. [C] same count no re-render (×1).
- [A] increment/decrementRowCount (×3): ±1 (new row `[""]` for 1 col), never below 1.
- [C] Legacy `addColumn/removeColumn/addRow/removeRow` delegate to increment/decrement (×4).
- [A] setColumnCount/setRowCount(event) (×4): read `event.target.value`; invalid ("xyz", "") → fallback current count (no change).
- [C] renderGrid (×4): syncs inputs; `<tr>` per row, `<td>` per cell; header row cells have `font-semibold`; quoted cell value can't inject attributes (input value set safely).
- [A] handleOpen (×2): no existing table → 3×3 default (row0 = "Header 1..3", rest ""), `editMode=false`; with `{existingTable:"| A | B |\n| --- | --- |\n| 1 | 2 |", startPos:10, endPos:50}` → `editMode=true`, positions stored, `tableData=[["A","B"],["1","2"]]` (separator row dropped). Opened via window event `frankmd:open-table-editor`.
- [C] insert (×2): dispatches window `frankmd:insert-table` `{markdown (contains "| A"), editMode, …}`; empty data → just closes.

### `test/javascript/controllers/text_format_controller.test.js` — subject: `app/javascript/controllers/text_format_controller.js` (86 tests: A=33 B=0 C=53; P=0)
- [A] Static formats (×8), exactly 7 in order with hotkeys: bold `**`/`**` B; italic `*`/`*` I; strikethrough `~~`/`~~` S; highlight `==`/`==` H; subscript `~`/`~` U; superscript `^`/`^` P; link `[`/`](url)` L.
- [C] connect (×2): selectedIndex 0, selectionData null.
- [C] open(selectionData,x,y) (×6): shows menu, stores data, resets index 0, renders 7 buttons; null data or empty text → stays hidden. close (×2) hides + clears data.
- [C] renderMenu (×4): labels via t(`dialogs.text_format.<id>`); selected item class `bg-[var(--theme-accent)]`; hotkey column ["B","I","S","H","U","P","L"]; hotkey letter underlined (`<u>`).
- [A] `formatLabelWithHotkey(label,key)` (×4): first case-insensitive occurrence wrapped: ("Bold","B")→"<u>B</u>old"; ("Italic","I")→"<u>I</u>talic"; not found → unchanged; ("subscript","U")→"s<u>u</u>bscript".
- [C] Menu keyboard (×11): ArrowDown/Up move with wraparound (6→0, 0→6); Enter applies selected (`text-format:applied` `{format:"italic"}`); Escape closes + dispatches `text-format:closed`; hotkeys B/I/S/L (and lowercase `b`) apply directly.
- [C] Hover updates selection (no re-render if same) (×2); click applies by `data-index` (×1).
- [C] applyFormat (×4): dispatches `applied` `{format, prefix, suffix, selectionData}`, closes; null format or null selection → nothing.
- [A] getFormat(id) (×2): returns format; unknown → undefined.
- [C] positionMenu(x,y) → `left:100px; top:200px` (×1).
- [A] `isToggleable` (×3): prefix===suffix (bold, italic true; link false).
- [A] `getUnwrapInfo(format, textarea)` (×4): markers inside selection ("**bold**" selected) or immediately outside (only "bold" selected) → `{canUnwrap:true,newText:"bold"}`; none → false; italic works.
- [A] `applyFormatById(id, textarea)` toggle (×6): "some **bold** text" with "bold" or "**bold**" selected → "some bold text"; "some plain text" select "plain" → "some **plain** text"; italic/strikethrough unwrap likewise; link NOT toggleable — "some [link](url) text" selecting "link" → "some [[link](url)](url) text" (QUIRK, must reproduce).
- [C] openAtPosition / openAtCursor (×7): open only for non-empty, non-whitespace selection; selectionData `{start,end,text}`; position from coords.
- [A] applyFormatToEditor(format, sel, cm) (×6): wrap → `replaceRange("**Hello**",0,5)` + focus; unwrap when selection includes markers → `replaceRange("Hello",0,9)`, `setSelection(0,5)`; unwrap when markers surround selection ("**Hello** World", sel 2..7) → same; link → `replaceRange("[Hello](url)",0,5)` and selects the `url` placeholder (8..11); null format / null selection → no-op.
- [C] onContextMenu(event, cm, isMarkdown) (×7): opens at clientX/Y with preventDefault only when CM selection non-blank, file is markdown, and cm present; no textarea needed.
- [C] openFromKeyboard(cm) (×6): positions at `coordsAtPos(selection.to)` → left, bottom+4 (120, 184); fallback to editor element center (250,250); blank/no selection or null cm → not opened.

### `test/javascript/controllers/theme_controller.test.js` — subject: `app/javascript/controllers/theme_controller.js` (45 tests: A=9 B=0 C=36; P=0)
Built-in themes (order): light (sun), dark (moon), catppuccin, catppuccin-latte, ethereal, everforest, flexoki-light, gruvbox, hackerman, kanagawa, matte-black, nord, osaka-jade, ristretto, rose-pine, solarized-dark, solarized-light, tokyo-night (palette icon). `lightThemes = [light, solarized-light, catppuccin-latte, rose-pine, flexoki-light]`; all others dark.
- [C] Custom themes (×3): `previewCustom({version:1,name,base:"nord",colors:{accent:"#112233"}})` → `<html data-theme="nord">` (base), inline CSS var `--theme-accent:#112233`, name rendered as text (no HTML injection); `restoreTheme(captureTheme())` restores previous theme and removes overrides; config-changed with same id `custom` but new palette re-applies colors (`#abcdef`); switching to `light` clears overrides and `dark` class; stale `/config/omarchy_theme` response after custom preview is ignored.
- [A] Static themes (×2): first two light, dark; each `{id,name,icon}`.
- [C] connect (×4): initial theme from `data-theme-initial-value`; `<html data-theme="dark">`; menu buttons = runtimeThemes; label "Dark".
- [C] toggle (×2), selectTheme (×3): sets id, applies `data-theme` (gruvbox), hides menu.
- [C] applyTheme sets `data-theme` (tokyo-night) (×1); checkmarks updated (×1).
- [A] Dark-class resolution (×5): `html.dark` added for dark themes; removed for light, solarized-light, catppuccin-latte, flexoki-light (`isDark = !lightThemes.includes(baseId)`).
- [A] System fallback (×2): no theme set → `prefers-color-scheme: dark` ⇒ "dark", else "light" (separate describe).
- [C] renderMenu (×3), getIcon (×4: sun/moon/palette SVGs; unknown → palette), click-outside (×1), disconnect (×2: removes `frankmd:config-changed` listener, dropdown, clears pending save timeout).
- [C] saveThemeConfig (×2): debounced 500 ms → `PATCH /config` JSON `{"theme":"nord"}`; 3 rapid calls → single PATCH with last (`tokyo-night`).
- [C] config listener (×2): `frankmd:config-changed {theme}` applies; same theme ignored.
- [C] Omarchy (×8): `GET /config/omarchy_theme` → `{theme_name, variables:{"--theme-…":…}, is_dark}`; when ok, runtime themes append `{id:"omarchy",name:"Omarchy",icon:"sync"}` (menu = 18+1); applying injects `<style id="omarchy-theme-vars">` containing `--theme-bg-primary: #1a1a2e`, `data-theme="omarchy"`, `dark` class from `is_dark`; switching away removes the style; polling (every 3000 ms) stops on disconnect; when endpoint becomes 404 → falls back to `dark` (and saves it).

### `test/javascript/controllers/typewriter_controller.test.js` — subject: `app/javascript/controllers/typewriter_controller.js` (22 tests: A=0 B=0 C=22; P=0)
- [C] connect (×2): `enabledValue=false` default; no `typewriter-centered` class.
- [C] toggle (×3): flips enabled, dispatches `typewriter:toggled` `{enabled}`, calls CodeMirror `setTypewriterMode(bool)`.
- [C] applyMode (×7): `typewriter-centered` class on wrapper and body when enabled (removed when disabled); toggle button `aria-pressed` "true"/"false"; class `bg-[var(--theme-bg-hover)]` on button when enabled.
- [C] maintainScroll (×3): only when enabled → CM `maintainTypewriterScroll()`; null CM safe.
- [C] getSyncData (×2): CM `getTypewriterSyncData()` (`{currentLine,totalLines}`) or null.
- [C] isEnabled (×1); setEnabled (×2) sets value, applies mode, syncs CM; getCodemirrorController (×2) finds `[data-controller="codemirror"]` or null.

### `test/javascript/controllers/video_dialog_controller.test.js` — subject: `app/javascript/controllers/video_dialog_controller.js` (uses `lib/url_utils.extractYouTubeId`) (54 tests: A=13 B=4 C=37; P=0)
- [C] connect (×3): results `[]`, selected index -1, `GET /youtube/config` (`{enabled}`) to decide whether search form or config notice shows.
- [C] open (×5): showModal; clears URL input, disables insert, resets detected type/data and search results/index. close (×1). switchTab (×2) toggles url/search(/drop) panels.
- [A] URL detection (`onVideoUrlInput`, ×6): "" → null + insert disabled; `https://www.youtube.com/watch?v=dQw4w9WgXcQ` and `https://youtu.be/dQw4w9WgXcQ` → `youtube` id `dQw4w9WgXcQ` (regex `(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/embed/|youtube\.com/v/)([a-zA-Z0-9_-]{11})`); `…/video.mp4`, `.webm` → `file` `{url}`; `page.html` → null, insert disabled. Video exts: .mp4 .webm .mkv .mov .avi .m4v .ogv.
- [C] Enter in URL field inserts only if insert button enabled (×2).
- [C] thumbnailError (×1): first failure swaps to `https://img.youtube.com/vi/<id>/default.jpg` and sets `data-fallback-applied="true"`; second failure hides img.
- [A] insertVideo embed output (×3): youtube → HTML embed containing `youtube.com/embed/abc123`; Hugo checkbox → exactly `{{< youtube id="abc123" >}}`; file → `<video controls class="video-player">\n  <source src="<escaped url>" type="video/mp4">\n  Your browser does not support the video tag.\n</video>` (MIME by ext: mp4 video/mp4, webm video/webm, mkv video/x-matroska, mov video/quicktime, avi video/x-msvideo, m4v video/x-m4v, ogv video/ogg; default video/mp4).
- [C] insertVideo with no type just closes; closes after insert (×2).
- [A] youtubeEmbedCode(id, title="YouTube video player") (×4): HTML `<div class="embed-container">` + `<iframe src="https://www.youtube.com/embed/<id>" title="<title>" …allowfullscreen>`; Hugo: default/omitted title → `{{< youtube id="abc123" >}}`, custom → `{{< youtube id="abc123" title="My Custom Title" >}}` (title NOT escaped).
- [C] searchYoutube guards (×2): empty query → t(`status.please_enter_keywords`); API not configured → message containing `youtube_not_configured`.
- [B] searchYoutube remote (×4): `GET /youtube/search?q=test%20query` (server proxies YouTube Data API, returns HTML buttons with `data-video-id`/`data-video-title`); results parsed to `[{id,title}]`; network error → `search_failed`, results []; empty → t(`status.no_videos_found`).
- [C] updateYoutubeSelection (×3): selected result button gets `ring-2 ring-[var(--theme-accent)]`; -1 clears; empty no-op.
- [C] selectYoutubeVideo (×4): dispatches `video-dialog:video-selected` `{embedCode}` with id+title (Hugo → `{{< youtube id="xyz789" title="Test Title" >}}`); missing id → nothing; closes.
- [C] Drop upload (×5): disallowed extension (notes.txt) → no upload, feedback t(`dialogs.video.drop_rejected`); valid → `POST /media/upload` → `{url:"videos/clip.mp4"}`, type `file`, drop insert enabled; failure `{error:"boom"}` shown and media-storage `refresh()` called once; uploading label t(`dialogs.video.uploading`); second drop while uploading ignored.
- [C] Dropzone keyboard Enter/Space opens picker, other keys ignored (×3); openPicker clicks hidden file input; picker selection uploads (×2).
- [C] Search field: Enter searches; ArrowDown selects first result (×2).

## Porting notes (part 5)
- **Backend endpoints the UI depends on** (Rust IPC/handlers must offer the same semantics): `POST/GET/DELETE /recent_files` (body `{path}`, server-rendered list); `GET /library`, `GET /library/usage[?path=]`, `DELETE|PATCH /library/file/<path>` (`{name}`), `GET /library/picker?collection=managed|server&search=&limit=&offset=`, `DELETE /images/file/<path>`, `POST /images/import` `{path,resize,s3_key}`, `GET /images/config`; `POST /media/upload`; `GET /logs/tail?lines=100`, `GET /logs/config`; `GET /translations` → `{locale,translations}`; `GET|PATCH /config` (`{settings:{…},features:{s3_upload},media_storage_fallback}`), `GET /config/omarchy_theme` (polled every 3 s); `GET /youtube/config`, `GET /youtube/search?q=`.
- **Settings keys written by the UI** (all via PATCH /config to `.fed`): `locale`, `theme`, `media_storage_provider`, `editor_font`, `editor_font_size`, `editor_width`, `editor_line_numbers`, `show_stats`, `editor_indent`, `preview_zoom`, `view_mode`, `cursor_recenter_on_click` (plus `vim_mode`, `typewriter_mode`, `scroll_sync` read on reload). Settings saves debounce 500 ms and the latest call's body *replaces* the pending one (quick successive changes to different keys can drop the earlier one — quirk). Theme save also debounced 500 ms. Locale change reloads the whole page even when the save fails.
- **Draft backup store**: key `frankmd:backup:<path>` → `{"content","timestamp"(ms)}`; identical-to-server or corrupt entries auto-deleted on `check`; Rust needs a persistent equivalent with the same semantics (recovery dialog cannot be dismissed with Escape; resolution echoes `source`, `draftRevision`, `conflictId`).
- **Timings**: find/replace "Replaced N" feedback 2000 ms; preview `_isUpdatingContent` 100 ms after last render; preview updateWithSync debounce ~150 ms; scroll-sync lock 400 ms, resize re-anchor debounce 200 ms; stats debounce 500 ms; media-storage fallback warning auto-dismiss 6000 ms; path toast ≤2300 ms; settings status 2000 ms.
- **Scroll sync math**: ratio snapping ≤0.01→0, ≥0.99→bottom; ratio→line `round(ratio×totalLines)+1`; typing-path anchor top − 50 px; line ratio `(line−1)/(total−1)`; source-line anchors are absolute file lines (front matter counted).
- **Pure logic worth porting as Rust unit tests**: jump-to-line resolution (`-1` = EOF special case, clamp), preview zoom steps `[50,75,90,100,110,125,150,175,200]` (settings select falls back to 100 for off-list values), text-format toggle/unwrap rules (link not toggleable → double wrap `[[link](url)](url)`), table resize rules (min 1, floor, "Header N" labels), YouTube ID regex + embed/Hugo shortcode strings (title not escaped), video MIME map, library item/usage-path validation (`images/`/`videos/` roots, S3 `frankmd/` root, http(s) url w/o credentials, `.md` note paths without `.`/`..`/`\`), `formatBytes`, `window.t` lookup/interpolation (missing → key; `%{x}` left intact), theme dark/light resolution (`lightThemes` list), locale list + flags.
- **Quirks to reproduce or consciously diverge**: stats panel skips recomputation when text length is unchanged (stale counts for same-length edits); default S3 key shape `frankmd/YYYY/MM/<file>`; unknown `view_mode` → `split`; missing `scroll_sync` on reload leaves current value; Omarchy disappearance falls back to `dark` and persists it.
- **Accessibility contracts** used by tests (focus moves, `aria-pressed`, `aria-expanded`, `aria-selected`, focus traps with Tab/Shift+Tab, focus restoration to trigger/fallback, reduced-motion `behavior:"instant"`) should be mirrored in the native UI's keyboard behavior.
- Keyboard shortcut for log viewer (Ctrl+Shift+O) and global shortcuts live in app_controller tests (other partial); Settings dialog blocks background shortcuts while open.


---

