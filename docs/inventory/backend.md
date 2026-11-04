# FrankMD Backend Inventory (for the Rust rewrite)

Source analysed: `~/Projects/FrankMD` at commit `d47a89e` ("release: v1.13.0 — explorer ergonomics"),
`FrankMD::VERSION = "1.13.0"` (`lib/frankmd/version.rb:4`). Rails 8.1.4, Ruby 3.4.x, no database.

Scope: everything server-side, meaning models, services, controllers, routes, initializers, config files, launcher
scripts, Docker, and ENV. The frontend (Stimulus/CodeMirror/ERB views) is inventoried separately. Where the frontend
depends on a backend behavior, this document says so.

Conventions used below:

- `file:line` citations are relative to the FrankMD repo root.
- **[COMPAT]** marks a quirk that a drop-in replacement **must replicate** because it shows up in data on disk or
  in observable behavior that users or files depend on.
- **[QUIRK-DROP]** marks a bug or accident we **may drop** (or should, for security). Each one says what the
  current behavior is.
- **[DECIDE]** marks a point where the team has to choose.
- `NOTES` means the resolved notes root directory (see §2.1).

---

## 0. Architecture summary

- Stateless request handlers. **Every request builds new `Config` objects** (`Config.new`), so `.fed` is re-read from
  disk many times per request. There is no in-memory config cache and no file watcher, so a change to `.fed` takes
  effect on the next request. That is the whole "live reload" mechanism (§1.9).
- Filesystem-only persistence: notes (`*.md`), `.fed` (config + recent files + custom theme), media under
  `NOTES/images` and `NOTES/videos`, two advisory lock files at the notes root.
- Server-rendered HTML fragments are returned by several endpoints (tree, recent files, search results, YouTube
  results, image library results, editor config, Turbo Streams). A native rewrite will probably render these on the
  client. The data contracts behind them are documented here.
- External integrations: S3 (aws-sdk-s3), LLMs (ruby_llm 2.1.1), Google CSE, YouTube Data API v3, DuckDuckGo
  scraping, ImageMagick CLI (`magick`/`convert`/`identify`).

---

## 1. Config system (`app/models/config.rb`)

### 1.1 File locations and constants

| Constant | Value | Cite |
|---|---|---|
| `CONFIG_FILE` | `.fed` (at `NOTES` root) | config.rb:6 |
| `LOCK_FILE` | `.frankmd-config.lock` (at `NOTES` root) | config.rb:7 |
| `CONFIG_VERSION` | `6`, **unused** (never written to the file) | config.rb:8 |

Base path: `base_path:` arg, else `ENV["NOTES_PATH"]`, else `<Rails.root>/notes` (config.rb:286).

### 1.2 SCHEMA: every key

Types: `string`, `integer`, `boolean`, `json`, `custom_theme`. "UI" means the key is in `UI_KEYS` (config.rb:96-114):
it is returned by `ui_settings` and writable through `PATCH /config`. "Secret" means the key is in `SENSITIVE_KEYS`
(config.rb:83-93) and is masked in `/logs/config` and `all_settings`.

| Key | Type | Default | ENV var | UI | Secret | Notes |
|---|---|---|---|---|---|---|
| `theme` | string | `nil` | — | ✔ | | No server validation. The frontend sends `"custom"` for a custom theme. |
| `recent_files` | json | `[]` | — | | | Array of `{path, opened_at}`, see §6. Not in UI_KEYS, so it is written only by RecentFilesService. |
| `custom_theme` | custom_theme | `nil` | — | ✔ | | JSON object, validated, see §11.2. |
| `locale` | string | `"en"` | `FRANKMD_LOCALE` | ✔ | | |
| `editor_font` | string | `"cascadia-code"` | — | ✔ | | |
| `editor_font_size` | integer | `14` | — | ✔ | | No server clamp (README says 8-32; the frontend clamps). |
| `preview_zoom` | integer | `100` | — | ✔ | | No server clamp (README says 50-200). |
| `sidebar_visible` | boolean | `true` | — | ✔ | | |
| `view_mode` | string | `"split"` | — | ✔ | | |
| `typewriter_mode` | boolean | `false` | — | ✔ | | |
| `cursor_recenter_on_click` | boolean | `false` | — | ✔ | | |
| `vim_mode` | boolean | `false` | — | ✔ | | |
| `scroll_sync` | boolean | `true` | — | ✔ | | |
| `editor_indent` | integer | `2` | — | ✔ | | 0 = tab, 1-6 = spaces (template comment). No server clamp. |
| `editor_line_numbers` | integer | `0` | — | ✔ | | Mode enum owned by the frontend. |
| `show_stats` | boolean | `false` | — | ✔ | | |
| `editor_width` | integer | `72` | — | ✔ | | Template says "minimum: 72". The server does **not** clamp. |
| `images_path` | string | `nil` | `IMAGES_PATH` | | | `~` is expanded when used (`Pathname#expand_path`, images_service.rb:25). Boot-time fallback in §1.10. |
| `media_storage_provider` | string | `"local"` | — (no ENV) | ✔ | | Must be `local` or `s3`. `s3` is rejected unless S3 is configured (config.rb:631-634). |
| `image_upload_extensions` | string | `".jpg,.jpeg,.png,.gif,.webp,.bmp"` | `IMAGE_UPLOAD_EXTENSIONS` | | | Parsed by `upload_extensions` (§1.8). |
| `video_upload_extensions` | string | `".mp4,.webm,.mkv,.mov,.avi,.m4v,.ogv"` | `VIDEO_UPLOAD_EXTENSIONS` | | | |
| `aws_access_key_id` | string | `nil` | `AWS_ACCESS_KEY_ID` | | ✔ | |
| `aws_secret_access_key` | string | `nil` | `AWS_SECRET_ACCESS_KEY` | | ✔ | |
| `aws_s3_bucket` | string | `nil` | `AWS_S3_BUCKET` | | | |
| `aws_region` | string | `nil` | `AWS_REGION` | | | Falls back to `us-east-1` at use sites. |
| `aws_s3_endpoint` | string | `nil` | `AWS_S3_ENDPOINT` | | | Custom S3-compatible endpoint. |
| `aws_s3_force_path_style` | boolean | `false` | `AWS_S3_FORCE_PATH_STYLE` | | | |
| `youtube_api_key` | string | `nil` | `YOUTUBE_API_KEY` | | ✔ | |
| `google_api_key` | string | `nil` | `GOOGLE_API_KEY` | | ✔ | |
| `google_cse_id` | string | `nil` | `GOOGLE_CSE_ID` | | | |
| `ai_provider` | string | `"auto"` | `AI_PROVIDER` | | | `auto\|openai\|anthropic\|openrouter\|ollama\|gemini` |
| `ai_model` | string | `nil` | `AI_MODEL` | | | Global model override. |
| `ollama_api_base` | string | `nil` | `OLLAMA_API_BASE` | | | AI credential key. Must be an http(s) URL **including `/v1`**. |
| `ollama_model` | string | `"llama3.2:latest"` | `OLLAMA_MODEL` | | | |
| `openrouter_api_key` | string | `nil` | `OPENROUTER_API_KEY` | | ✔ | AI credential key. |
| `openrouter_model` | string | `"openai/gpt-4o-mini"` | `OPENROUTER_MODEL` | | | |
| `anthropic_api_key` | string | `nil` | `ANTHROPIC_API_KEY` | | ✔ | AI credential key. |
| `anthropic_model` | string | `"claude-sonnet-4-20250514"` | `ANTHROPIC_MODEL` | | | |
| `gemini_api_key` | string | `nil` | `GEMINI_API_KEY` | | ✔ | AI credential key. |
| `gemini_model` | string | `"gemini-2.0-flash"` | `GEMINI_MODEL` | | | |
| `openai_api_key` | string | `nil` | `OPENAI_API_KEY` | | ✔ | AI credential key. |
| `openai_model` | string | `"gpt-4o-mini"` | `OPENAI_MODEL` | | | |
| `hugo_path_style` | string | `"dated"` | `HUGO_PATH_STYLE` | | | `flat` → `slug.md`. Any other value means dated. |
| `auth_token` | string | `nil` | `FRANKMD_AUTH_TOKEN` | | ✔ | Access token, §13.2. Can be set in `.fed` too. |
| `image_generation_model` | string | `"google/gemini-3.1-flash-image-preview"` | `IMAGE_GENERATION_MODEL` | | | OpenRouter model id. |

`AI_CREDENTIAL_KEYS` (config.rb:125-131) = `ollama_api_base, openrouter_api_key, anthropic_api_key, gemini_api_key,
openai_api_key`. `ai_provider` and the `*_model` keys are **not** credential keys.

`AI_PROVIDER_PRIORITY` (config.rb:120) = `openai, anthropic, openrouter, ollama, gemini`.
**[COMPAT]** The README lists a different order (Gemini third). The code order is the authoritative one.

### 1.3 Resolution priority

`get(key)` (config.rb:293-305):

1. Unknown key → `nil`.
2. If the key **was present in `.fed`** (parsed into `@values`), return that value. **This includes `nil`**: a line
   like `key =` with an empty value stores `nil`, and the ENV value and the default are then **not** consulted.
   **[COMPAT]**
3. Else, if the schema has an `env` name and `ENV[name]` is *present* (non-empty and not whitespace-only), return
   `cast(ENV[name])`.
4. Else return the schema default.

`get_ai(key)` (config.rb:415-430), the special AI-credentials rule:

- `ai_configured_in_file?` is true when **any** of the 5 `AI_CREDENTIAL_KEYS` is a key in `@values`. Presence of the
  key is enough. An empty `openai_api_key =` line counts. **[COMPAT]** (verified empirically.)
- When true ("file-only mode"), `get_ai(k)` returns `@values[k]` if present, else the **schema default**. **ENV is
  ignored for every key read through `get_ai`**: credentials, `ai_provider`, `ai_model`, and `*_model`.
- When false, `get_ai(k)` is the same as `get(k)`.
- Keys read via `get_ai`: the 5 credentials, `ai_provider`, `ai_model`, `<provider>_model`. These calls are made by
  `ai_providers_available`, `effective_ai_provider`, `effective_ai_model`, and `AiService.configure_client`.
- **Exception, image generation** (ai_service.rb:88-93): the OpenRouter key for images is
  `@values["openrouter_api_key"] || ENV["OPENROUTER_API_KEY"]`. It bypasses file-only mode. A blank string in the
  file is truthy in Ruby, so `openrouter_api_key =` in `.fed` gives `nil`, which falls through to ENV. An ENV value
  of `""` is then not `present?`, so image generation is disabled.
  The image *model* uses plain `get("image_generation_model")`, which respects ENV.

`ai_providers_available` (config.rb:433-441) returns providers whose credential is `present?`, **in this order**:
`ollama, openrouter, anthropic, gemini, openai`. That order shows up as `available_providers` in `/ai/config`.

`effective_ai_provider` (config.rb:444-457):

- With no providers available, it returns `nil`.
- If `get_ai("ai_provider")` is present, is not `"auto"`, **and** is in the available list, that provider is used.
- Otherwise (including when the configured provider is not available) it picks the first entry of
  `AI_PROVIDER_PRIORITY` that is available.

`effective_ai_model` = `get_ai("ai_model")` if present, else `get_ai("<provider>_model")`.

### 1.4 Casting (`cast_value`, config.rb:540-557)

- `nil`, or a string that is empty after `strip`, → `nil` (for every type).
- `integer` → Ruby `to_i`: leading-integer parse. `"12px"` → 12, `"abc"` → **0**, `"1.9"` → 1, `"-5"` → -5. It never
  errors. **[COMPAT]**
- `boolean` → true iff `downcase(value.to_s)` is one of `true, 1, yes, on`. Anything else, including `"false"`,
  `"no"`, and garbage, is `false`.
- `string` → `to_s`.
- `json` → `JSON.parse(string)`. A non-string value is round-tripped through JSON. A parse error makes `.fed`
  parsing skip that line (§1.5).
- `custom_theme` → `CustomTheme.parse` (§11.2). An `ArgumentError` skips the line.

### 1.5 Parsing rules (`parse_config`, config.rb:510-538)

For each line (`each_line`, so both LF and CRLF line endings work):

1. `strip` both ends. Skip it if it is empty or starts with `#`. There are **no inline comments**:
   `theme = dark # x` has the value `dark # x`.
2. Match `^([a-z0-9_]+)\s*=\s*(.*)$` case-insensitively. Keys are lowercased (`THEME = x` works). A line that does
   not match (no `=`, a key containing `-`, etc.) is ignored silently.
3. The value is the rest of the line after the **first** `=`, stripped. A `=` inside the value is kept
   (`images_path = /a=b` → `/a=b`).
4. Quote removal: if the value starts **and** ends with `"`, drop the first and last character. Then, separately, the
   same for `'`. There are no escape sequences. The two passes run one after the other, so `"'x'"` → `x`. A lone `"`
   becomes the empty string, which becomes `nil`. An unterminated `"abc` stays as-is, including the quote.
5. Unknown keys are ignored. They are preserved when the file is written back.
6. **Duplicate keys: the last one wins.**
7. A cast error (bad JSON, invalid custom theme) → log a warning and skip only that line.
8. Any other exception while reading → log, and `@values = {}` (so everything falls back to ENV and defaults).

### 1.6 File creation and upgrade (`ensure_config_file`, config.rb:478-490)

This is called by **every `Config.new`** (via `load_config`) and inside `modify`/`save_keys`. It runs under the
config lock (§1.8):

- If `.fed` does not exist → `create_template_config`, which runs `mkpath` on the notes dir and writes the template
  (§1.7) atomically. Errors (EACCES/EROFS/other) are logged and swallowed, so a read-only mount still works.
- If it exists → `upgrade_config_file` (config.rb:684-715). The upgrade adds **only** the AI/LLM section, and only
  when the file contains no line that includes `# AI/LLM` **and** no line matching `/^#?\s*KEY\s*=/i` for any of
  these keys: `ai_provider ai_model ollama_api_base ollama_model openrouter_api_key openrouter_model
  anthropic_api_key anthropic_model gemini_api_key gemini_model openai_api_key openai_model`. When it applies, it
  appends one `""` line if the last line is non-blank, then the AI section lines (the last block of §1.7), and
  rewrites the file atomically. **[COMPAT]**
- **[COMPAT]** A consequence: **any request at all creates `NOTES/.fed`** (and the lock files) if they are missing.

### 1.7 The auto-generated `.fed` template (verbatim)

Generated as `TEMPLATE_SECTIONS.flat_map(lines).join("\n") + "\n"` (config.rb:717-719, 746). The file is exactly
**3032 bytes, 101 lines, LF endings, a single trailing newline**, with
SHA-256 `796513f72555c60aa588382f7487bb172b48192e04ca3a3c1939d1d8210fb154`. It is written with mode `0600`.
The fenced block below is exact, and there is no trailing whitespace on any line:

```
# FrankMD Configuration
# Uncomment and modify values as needed.
# Environment variables are used as defaults if not specified here.

# UI Settings

# Theme: light, dark, gruvbox, tokyo-night, solarized-dark,
#        solarized-light, nord, cappuccino, osaka, hackerman
# theme = dark

# Editor font: cascadia-code, jetbrains-mono, fira-code,
#              source-code-pro, ubuntu-mono, roboto-mono, hack
# editor_font = cascadia-code

# editor_font_size = 14
# preview_zoom = 100
# sidebar_visible = true
# typewriter_mode = false
# cursor_recenter_on_click = false
# show_stats = false

# Vim mode: hjkl navigation, modal editing, and : ex-commands (default: false)
# vim_mode = false

# Editor indent: 0 = tab, 1-6 = spaces (default: 2)
# editor_indent = 2

# Editor width in characters (default: 72, minimum: 72)
# Increase for wider text area, e.g., 100, 120, or 150
# editor_width = 72

# Local Images

# images_path = /path/to/images

# Media Uploads (drag-and-drop)

# Destination for new media: local or s3 (requires AWS S3 configuration).
# media_storage_provider = local

# Comma-separated file extensions accepted by the image/video drop tabs.
# image_upload_extensions = .jpg,.jpeg,.png,.gif,.webp,.bmp
# video_upload_extensions = .mp4,.webm,.mkv,.mov,.avi,.m4v,.ogv

# AWS S3 (for image uploads)

# aws_access_key_id = your-access-key
# aws_secret_access_key = your-secret-key
# aws_s3_bucket = your-bucket-name
# aws_region = us-east-1

# S3-compatible services (MinIO, SeaweedFS, ...): set a custom endpoint
# and force path-style URLs (bucket in the path).
# aws_s3_endpoint = http://localhost:9000
# aws_s3_force_path_style = true

# YouTube API (for video search)

# youtube_api_key = your-youtube-api-key

# Google Custom Search (for image search)

# google_api_key = your-google-api-key
# google_cse_id = your-custom-search-engine-id

# Hugo Blog Posts

# Path style: dated = YYYY/MM/DD/slug/index.md (default)
#             flat  = slug.md (in current folder)
# hugo_path_style = dated

# Custom frontmatter: create a .hugo_template.md file at the notes root
# to override the generated frontmatter. Placeholders {{title}}, {{slug}}
# and {{date}} are substituted. Leave unset to use the built-in template.

# AI/LLM (for grammar checking)
# Provider priority when ai_provider = auto: openai > anthropic > openrouter > ollama > gemini
# Gemini is lowest priority for text (its key is mainly for image generation)

# ai_provider = auto
# ai_model = (uses provider-specific default if not set)

# OpenAI (recommended)
# openai_api_key = sk-...
# openai_model = gpt-4o-mini

# Anthropic (Claude models)
# anthropic_api_key = sk-ant-...
# anthropic_model = claude-sonnet-4-20250514

# Google Gemini
# gemini_api_key = ...
# gemini_model = gemini-2.0-flash

# OpenRouter (multiple providers, pay-per-use)
# openrouter_api_key = sk-or-...
# openrouter_model = openai/gpt-4o-mini

# Ollama (local, free) - for privacy, requires local setup
# ollama_api_base = http://localhost:11434
# ollama_model = llama3.2:latest
```

The AI upgrade block (§1.6) consists of the last 27 lines above, starting at the blank line before
`# AI/LLM (for grammar checking)` and ending with `# ollama_model = llama3.2:latest`. That section's line list
begins with `""`.

Template oddities that must be kept byte-for-byte:

- The theme list in the comment is stale ("cappuccino, osaka" are not current theme ids).
- `auth_token`, `image_generation_model`, `locale`, `view_mode`, `scroll_sync`, `editor_line_numbers`,
  `custom_theme`, and `recent_files` do not appear in the template at all.
- `ollama_api_base` in the template lacks `/v1`, although the README and `.env.example` say `/v1` is required.

`generate_config_lines` (config.rb:756-771) is **dead code**.

### 1.8 Writing back to `.fed` (surgical update)

Public API:

- `set(key, value)` casts the value, runs `format_value` to validate, saves, then updates `@values`. It returns
  `true`/`false` and never raises `ArgumentError`.
- `update(hash)` does the same for several keys in one write. Unknown keys are skipped silently. If any key fails
  validation, **nothing is written**.
- `modify(key) { |old| new }` is used by recent files. Under the lock it runs `ensure_config_file`, **re-reads and
  re-parses the file**, then calls `set(key, yield(get(key)))`. This read-modify-write is safe against concurrent
  clients.

`save_keys_without_lock(changes)` (config.rb:573-628). It reads the current file line by line:

| Current line (stripped) | Action |
|---|---|
| empty or whitespace-only | Emit `""`. Whitespace-only lines are normalized to empty. |
| starts with `#` and matches `^#\s*([a-z0-9_]+)\s*=`/i, the key is in `changes`, and this key has not been written yet | **Replace the commented line** with `format_value(key, v)`. This is how the app "uncomments" settings. Only the **first** commented occurrence is replaced. |
| any other comment | Emit `line.chomp` unchanged (leading whitespace kept). |
| matches `^([a-z0-9_]+)\s*=`/i with key in `changes` | Replace with `format_value`. **Every** active occurrence is replaced, so duplicate active lines all get the new value. Original indentation and spacing are lost on that line. |
| matches with key not in changes | Emit `line.chomp` unchanged. |
| anything else (malformed) | Emit `line.chomp` unchanged. |

After the loop, every changed key that was **not written** is appended at the **end of the file**, in hash insertion
order. The section structure is not considered. Note the interaction: if a commented line came first, it is replaced,
and a later active line is also replaced, which leaves two identical lines. CRLF files get converted to LF.

The file is always rewritten as `lines.join("\n") + "\n"`.

`format_value(key, value)` (config.rb:630-656). The output is the canonical line form `key = value`, with single
spaces around `=`:

- `media_storage_provider`: raises unless the value is `local` or `s3`. Raises if the value is `s3` and S3 is not
  configured (§7.3).
- `json` / `custom_theme`: `key = <JSON.generate(value)>`. This is compact JSON with no spaces. Ruby `JSON.generate`
  leaves non-ASCII and `/` unescaped and escapes `"`, `\`, and control characters. `nil` → `key = null`.
- `string`: **raises** if the value contains `"`, `\r` or `\n`. If the value contains a space or `=`, it is written
  double-quoted (`key = "a b"`), otherwise bare. `nil` → `key = ` (note the trailing space). **[COMPAT]** A blank
  string therefore round-trips to a "present but nil" key that blocks the ENV fallback.
- `boolean`: `true`/`false` (`nil` → `false`).
- `integer` and others: `key = #{value}` (`nil` → `key = `).

Atomic write (config.rb:741-754): temp file `NOTES/..fed.<12 hex>.tmp`. That is the basename `.fed` with a `.`
prepended, so the name begins with two dots. It is opened with `O_WRONLY|O_CREAT|O_EXCL` and mode `0600`, written,
flushed, `fsync`ed, then `rename`d over `.fed`. The temp file is removed in `ensure`. **[COMPAT]** As a consequence,
**`.fed` always ends up with mode 0600** after any write, whatever the user set before. The directory is not fsynced.

Locking (config.rb:721-739): `flock(LOCK_EX)` on `NOTES/.frankmd-config.lock`. The lock file is opened
`O_RDWR|O_CREAT|O_NOFOLLOW` with mode 0600, and its path comes from the realpath of the notes dir. The lock is
reentrant per thread (a thread-local set of held paths). It is used by `ensure_config_file`, `save_keys`,
`synchronize`, and `modify`. NotesService also takes this lock around delete/rename/delete_folder, so that
recent-files updates are atomic with the filesystem change (§2.6).

`upload_extensions(key)` (config.rb:309-311): `get(key).to_s.split(",").map(strip.downcase).reject(blank)`.
`" .JPG, .png "` → `[".jpg", ".png"]`. There is no check that entries start with `.`. An entry such as `jpg` (without
the dot) can never match, because extnames always include the dot.

`all_settings` (config.rb:372-385) returns sensitive keys as `<key>_configured: bool` unless
`include_sensitive: true`. It is currently unused by controllers.

`feature_available?(f)` (config.rb:388-403):

- `s3_upload` → `UploadStorage.s3_configured?`
- `youtube_search` → youtube key present
- `google_search` → google key and cse id both present
- `local_images` → `images_path` present (string non-blank; the directory is not checked)
- `ai` → any provider available
- anything else → false

### 1.9 Live reload

There is no watcher. Each HTTP request re-reads `.fed` many times, because every service calls `Config.new`. ENV is
read on each `get` (it can change only in-process). The frontend refetches `/config` after its own saves.
**[DECIDE]** A native app will probably cache config in memory. To stay a drop-in replacement it should watch `.fed`
(and re-read it before every read-modify-write), because another FrankMD instance or a sync tool may edit it.

**[QUIRK-DROP] In-app `.fed` editing is broken server-side.** The README ("Editing .fed in the App") and the frontend
(`app_controller.js:1118, 1566`) still treat `.fed` as an editable "config" file, but:

- `build_tree` hides dotfiles, so `.fed` never appears in the tree.
- `NotesController#reject_config_file` (notes_controller.rb:186-193) returns **403 `{"error":"forbidden"}`** for
  show/create/update/destroy/rename on any path that resolves to `NOTES/.fed`.

These were introduced by the commit `b03f36c fix(security): harden application boundaries`. **[DECIDE]** whether the
rewrite restores `.fed` editing.

### 1.10 Boot-time ENV mutation

`config/initializers/images.rb`: if `IMAGES_PATH` is **unset** (an empty string counts as set), the initializer sets
it to `XDG_PICTURES_DIR` if that is set, else to `~/Pictures` if that directory exists. In Docker, `IMAGES_PATH` is
always set (`/rails/images`) by the Dockerfile (Dockerfile:82-83). **[COMPAT]** A desktop app should apply the same
default.

---

## 2. Filesystem model (`app/services/notes_service.rb`, `app/models/note.rb`, `app/models/folder.rb`)

### 2.1 Notes root

`NOTES = ENV["NOTES_PATH"]` or `<app>/notes` (notes_service.rb:16). It is `mkdir_p`'d on `NotesService.new` if
missing. It is read through `ENV.fetch("NOTES_PATH", …)` in config.rb, notes_service.rb, backlinks_service.rb,
hugo_service.rb, upload_storage.rb, images_service.rb:463, notes_controller.rb:165,187, and
application_controller.rb:95. NOTES_PATH is **not** a `.fed` key.

**[DECIDE]** In the desktop app, NOTES comes from the CLI arg (as with `fed <path>`) or from NOTES_PATH.

### 2.2 Tree (`list_tree` / `build_tree`, notes_service.rb:437-485)

The tree is built under a shared `flock(LOCK_SH)` on the filesystem lock (§2.7). For each directory:

- **Skip every entry whose basename starts with `.`**. This hides `.fed`, the lock files, `.hugo_template.md`,
  `.git`, and the temp files.
- `lstat` each entry and **skip symlinks** (both file and directory symlinks). Entries that vanish or fail with
  ENOENT/ENOTDIR are skipped.
- Sort: **directories first, by `basename.downcase`** (Ruby `downcase` is Unicode-aware, and the comparison is
  bytewise on the lowercased string). Then **files by mtime descending, in whole seconds**. Ties are in unspecified
  order, because Ruby `sort_by` is not stable. **[COMPAT]** for the ordering. The directories-first rule and the
  file mtime order are user-visible. (Non-md files are sorted too but then dropped.)
- **Root-level special case** (notes_service.rb:461-464): at the **root only**, a directory named exactly `images`
  or `videos` is hidden **unless** it contains a `*.md` file somewhere beneath it (`glob("**/*.md")`). Subfolders
  with those names are always shown.
- Files: only `extname == ".md"`, compared **case-sensitively**, so `.MD` and `.markdown` are not shown. Other files
  are omitted.
- Empty directories are shown, with `children: []`.
- If a directory disappears mid-walk, the result for it is `[]`.

Node shape (internal Ruby hash; it is rendered into HTML by `TreeHelper`, see §2.10):

```json
{ "name": "folder", "path": "a/folder", "type": "folder", "children": [ ... ] }
{ "name": "note", "path": "a/folder/note.md", "type": "file", "file_type": "markdown", "mtime": 1767225600 }
```

`name` for files is the basename without `.md`. `path` is relative to NOTES and uses `/`. `mtime` is integer epoch
seconds.

### 2.3 Path normalization (`Note.normalize_path`, note.rb:43-52)

- blank → `""`
- If the basename is in `SPECIAL_FILES = [".fed"]`, the path is returned unchanged.
- Otherwise append `.md` unless the path already ends with `.md` (case-sensitive). So `foo` → `foo.md`, and
  `foo.txt` → `foo.txt.md`.

`Note` validation (note.rb:182-187, applied on `save`, i.e. create/update only): the path must be present and must
**not contain the substring `..` anywhere**. **[QUIRK-DROP]** This rejects legitimate names like `a..b.md`. The error
message is `"Path cannot contain directory traversal"`. Rename does **not** run this validation; PathSafety still
applies there. `Folder` has the same `..` rule for create (folder.rb:341-346).

### 2.4 Path safety (`app/services/path_safety.rb`)

`PathSafety.contain(base, path)` returns a cleaned absolute Pathname, or `nil` when the path is rejected:

1. `nil` path → `nil`.
2. `base = cleanpath(base)`. `candidate = cleanpath(base.join(path))`. The cleaning is lexical: it resolves `.` and
   `..` and collapses `//`. **An absolute `path` replaces `base`**, because `Pathname#join` behaves that way, so
   `/etc/passwd` is then rejected by the next check.
3. Reject unless `candidate == base` or `candidate` starts with `base + "/"`. The check is anchored on the separator,
   so `/data/notes-backup` is rejected when the base is `/data/notes`.
4. If `base` exists: walk up from `candidate` to the **nearest existing ancestor** (`exist?` follows symlinks, so a
   dangling symlink counts as non-existent and the walk continues to its parent). Reject unless
   `realpath(ancestor)` is within `realpath(base)` (same check as step 3). This catches symlinked directories that
   point outside the base.
5. Return `candidate`, which is the **lexical** path, not the realpath.

Note that `candidate == base` passes, so an empty path or `.` resolves to the root itself.

`NotesService#safe_path(path, must_exist:)` (notes_service.rb:423-435):

- If the result equals `NOTES/.frankmd-config.lock` → `InvalidPathError("Reserved application path")`. The check is
  made before the nil check. **[QUIRK-DROP]** `.frankmd-filesystem.lock` is *not* reserved here, but
  `serve_asset` hides both lock files.
- `nil` → `InvalidPathError("Invalid path: <path>")`.
- `must_exist` and the path does not exist → `NotFoundError("Path not found: <path>")`.

Other path validators:

- RecentFiles `valid_path?` is in §6.
- MediaLibrary `safe_media_file` is in §7.7.
- S3 `valid_key?` is in §7.8.
- `BacklinksService` raises `ArgumentError` if the path includes `..`. The controller does not rescue it, so the
  result is a 500.
- `MediaUsageService.canonical_media_path` is in §7.9.

### 2.5 Note operations

All mutations hold the exclusive filesystem lock (§2.7). Reads do **not** take the lock (except the tree).

| Op | Semantics | Cite |
|---|---|---|
| read | `safe_path(must_exist)`, must be a regular file (`file?` follows symlinks, so a symlinked note **inside** the root is readable even though the tree hides it). Read as bytes tagged UTF-8, with **no** BOM stripping and no newline handling. | notes_service.rb:24-29 |
| write (upsert) | `mkdir_p(parent)`, then atomic write. Used by Hugo slug updates on folder rename. | :31-38 |
| create | `mkdir_p(parent)`. If the destination exists (`exist?` or a **symlink**, dangling ones included) → `AlreadyExistsError`. Write a temp file `.<basename>.<12hex>.tmp` with `O_EXCL`, mode `0666 & ~umask`, then **`link(2)`** temp → dest, which fails if dest exists. If hard links are unsupported (EPERM/EOPNOTSUPP/ENOTSUP/EXDEV), check existence again and `rename(2)`. The temp file is always removed. | :44-53, 217-249 |
| update | Must already exist as a file, else `NotFoundError` (404). **Parent dirs are not created**, so a stale autosave cannot resurrect a moved note. Atomic write. | :57-65 |
| atomic write | Temp file `.<basename>.<12hex>.tmp` in the same dir. Write the content, **copy the permission bits of the existing file** (`mode & 0777`) onto the temp file, then `rename` it over the target. On error, delete the temp file and re-raise. No fsync. | :202-215 |
| delete | Under the recent-files lock, then the fs lock. Must be a file. If `expected_revision` is given and `sha256(file bytes) != expected_revision` → `RevisionConflictError` (409). Then unlink and remove the path from recent files. | :67-84 |
| rename / move | Under the recent-files lock, then the fs lock. The source must `exist?` (a **file or a directory**; folders use the same method). If the destination exists or is a symlink → `AlreadyExistsError`. `mkdir_p(dest parent)`, then `FileUtils.mv`: an atomic `rename(2)`, with a copy-and-delete fallback on EXDEV. EEXIST/ENOTEMPTY/EISDIR → AlreadyExists. Then remap recent files (§6). | :86-108 |
| create_folder | If it exists or is a symlink → AlreadyExists. `mkdir_p(parent)`, then `mkdir`. EEXIST → AlreadyExists. | :110-123 |
| delete_folder | Must be a directory. **It must be empty, hidden files included** (a stray `.DS_Store` blocks it with `InvalidPathError("Folder not empty: <path>")`, which is a 422). Then `rmdir` and remove from recent files (prefix match). Non-recursive. | :125-140 |
| folder rename | `service.rename`, then, if `<new>/index.md` exists, rewrite its front-matter slug to `slugify(basename(new))` (§4.4). Errors from the slug update are logged and ignored. | folder.rb:271-326 |

**[COMPAT] Revision** = `SHA256(content).hexdigest`, lowercase hex of the exact file bytes (note.rb:75-77). It is
returned by show, update, and the initial note. **Update does NOT check revisions**: the last write wins. Only delete
checks.

Encoding: content is stored and returned **verbatim**. No newline normalization, no trailing-newline addition, CRLF
preserved. **[QUIRK-DROP]** Files with invalid UTF-8 make the JSON render fail with a 500 on show. Search scrubs
invalid bytes (§3). Backlinks does not, and can 500.

### 2.6 Recent-files coupling

`with_recent_files_lock` takes the **config lock** around the filesystem lock, but only when `NOTES/.fed` is a
regular file. Lock errors raised before entering the block are logged, and the operation proceeds without the lock.
`update_recent_files` is a no-op when `.fed` is not a file. Errors are logged and swallowed, so history failures
never fail a note operation.

### 2.7 Filesystem lock (`with_filesystem_lock`, notes_service.rb:261-326)

- Shared lock file: `realpath(NOTES)/.frankmd-filesystem.lock`, opened `O_RDWR|O_CREAT|O_NOFOLLOW` with mode 0600.
  `flock` is `LOCK_EX` for mutations and `LOCK_SH` for tree reads.
- On EACCES/EROFS: for `LOCK_SH`, retry `O_RDONLY|O_NOFOLLOW`. Otherwise, or if that fails too, use the fallback.
  Any other `SystemCallError` also leads to the fallback.
- Fallback: a local lock at `<app>/tmp/frankmd_filesystem_locks/<sha256(realpath NOTES)>/notes.lock`, in a directory
  with mode 0700.
- **[COMPAT]** The two lock files `.frankmd-config.lock` and `.frankmd-filesystem.lock` are created in NOTES, which
  makes them part of the on-disk layout. Another FrankMD instance (old Rails or new Rust) sharing the folder
  coordinates only if it uses the **same flock files**. Keep the names and use `flock(2)`, not fcntl locks.

### 2.8 Conflict summary (HTTP)

| Situation | Status / body |
|---|---|
| create when the note exists | 422 `{"error":"Note already exists"}` (`errors.note_already_exists`) |
| update when missing | 404 `{"error":"Note not found"}` |
| delete revision mismatch | 409 `{"error":"This note changed since it was loaded. Reload it before deleting.","code":"revision_conflict"}` |
| delete missing | 404 |
| rename source missing | 404 `{"error":"Note not found"}` |
| rename destination exists | 422 `{"error":"Path Note already exists"}`. **[COMPAT-ish]** The `"Path "` prefix comes from ActiveModel `full_messages` on a `:path` error. |
| invalid path (create/update) | 422 `{"error":"Path Invalid path: x"}` or `"Path cannot contain directory traversal"` |
| EACCES/EPERM | 422 `"Permission denied. You may not have access to this file or folder."` |
| ENOENT (parent vanished) | 422 `errors.parent_folder_not_found` / `file_no_longer_exists` |
| folder create exists | 422 `{"error":"Folder already exists"}` |
| folder delete not empty | 422 `{"error":"Folder not empty: <path>"}` |
| folder op whose message contains "not found" | 404, otherwise 422 (folders_controller.rb:296, 312) |

Error messages are localized by the active locale (§12).

### 2.9 Asset serving (`NotesController#show` → `serve_asset`, notes_controller.rb:27-33, 164-184)

`GET /notes/*path`: if the raw path **ends with `.md`, or contains no `.` at all**, it is treated as a note.
Otherwise it is served as a raw file:

- `PathSafety.contain(NOTES, path)` nil → **403** (empty body).
- If it resolves to either lock file → **404**.
- If it is a regular file → `send_file`, `Content-Disposition: inline`, `Content-Type` from Rack::Mime by extension,
  falling back to `application/octet-stream`. Rails' default `X-Content-Type-Options: nosniff` is sent. Range
  requests are supported by Rack/Thruster, which matters for video seeking. **[COMPAT]** A native webview needs
  Range support for `<video>`.
- Otherwise → 404.
- The `.fed` path is blocked before this point by `reject_config_file` (403 JSON).
- **[QUIRK-DROP]** Any other hidden file is served (e.g. `/notes/.git/config`). Files inside a folder whose name
  contains a dot are served raw, but `/notes/v1.2/note` (no `.md`) is treated as an asset → 404.

This route is how all media references render. Local uploads return `images/<file>` / `videos/<file>` (relative to
the note root), and the Library uses `/notes/images/...` preview URLs (§7.7).

### 2.10 Endpoints that return server-rendered HTML (the frontend relies on these)

| Endpoint | Returns |
|---|---|
| `GET /notes/tree?expanded=a,b&selected=x` | HTML partial `notes/_file_tree` (`TreeHelper#render_tree_items`): `.tree-folder[data-path]`, `.tree-item[data-path,data-type,data-file-type,data-mtime]`. Folders listed in `expanded` are expanded. An empty root shows `sidebar.no_notes_yet`. The file finder (Ctrl+P) flattens this DOM, so its "recency" sort comes from `data-mtime`. |
| `POST/DELETE notes…`, `POST /notes/*/rename`, folders create/delete/rename, all with `Accept: text/vnd.turbo-stream.html` | Turbo Stream `update "file-tree-content"` with the tree partial. Note create also sets the header **`X-Created-Path: <path>`** with status 201. Folder rename remaps `expanded` paths from old to new prefix. |
| `GET /recent_files` | HTML partial `notes/_recent_files` (§6). |
| `GET /notes/search?q=` (HTML Accept) | Partial `notes/_search_results`. JSON when `format.json`. |
| `GET /youtube/search` (HTML) | Partial `youtube/_search_results`. |
| `GET /library/picker` | Turbo Stream replace `image-picker-library-results`. |
| `GET /config/editor` | Partial `config/_editor_config`. |
| `GET /` and `GET /notes/<note>` (HTML) | The full SPA page. Embedded values: `data-app-initial-path-value`, `data-app-initial-note-value` = `{path, content, revision, exists:true}` or `{path, content:null, exists:false, error:"This file no longer exists or was deleted."}`, the media-storage provider, fallback, and S3-available flags, the recent files, and `t('status')`. `?file=<path>` on `/` also preselects a note. |

---

## 3. Search (`NotesService#search_content`, notes_service.rb:159-193, 346-397)

Endpoint `GET /notes/search?q=…`: `context_lines: 3, max_results: 20` (notes_controller.rb:148).

- A blank query (`""` or whitespace) → `[]`.
- **Regex first**: compile `q` as a **Ruby (Onigmo) regex with IGNORECASE** and a 0.5 s match timeout. If it does
  not compile (`RegexpError`), fall back to the **escaped literal**, still case-insensitive. **[COMPAT]** Users type
  Ruby regex syntax. Ruby supports lookbehind and backreferences, so Rust's `regex` crate is not equivalent.
  **[DECIDE]** Use `fancy-regex` or `onig`.
- Files: recursive walk of NOTES using `Dir.children` (**unsorted**, in filesystem order). Symlinks are skipped.
  Basenames starting with `.` are skipped (files and dirs). Only `extname == ".md"`. **Unlike the tree, the root
  `images`/`videos` folders are not excluded.**
- Files larger than **5 MiB** (`MAX_SEARCH_FILE_BYTES = 5*1024*1024`) are skipped.
- Each file is read as lines (`readlines(chomp: true)`, so `\n` and `\r\n` are stripped) and every line is
  `scrub`bed (invalid UTF-8 → U+FFFD).
- Matching is **per line**. For each matching line, a context window of ±3 lines is clipped to the file. Context
  windows of nearby matches overlap independently; they are not merged.
- Limits: stop when total results ≥ 20. The check happens before each file and per match, so per-file matches are
  capped at the remaining count. **[QUIRK-DROP-ish]** Because the walk is unsorted, the 20 results are the *first
  found*, not the newest. They are **then** sorted by file mtime descending (integer seconds, unstable among ties).
  The mtime is stat'ed only for files that matched.
- A regex timeout or I/O error on one file → that file is skipped. A timeout at compile time → `[]`.

Result JSON (array):

```json
[{ "line_number": 12, "match_text": "the line", "context": [
     {"line_number": 9, "content": "...", "is_match": false}, ... ],
   "path": "a/b.md", "name": "b", "mtime": "2026-11-04T10:00:00.000-03:00" }]
```

`mtime` is a Ruby `Time`. It is serialized by ActiveSupport as ISO-8601 with milliseconds in the server's **local**
offset.

Full-text search has no index; it scans on every request.

---

## 4. Hugo blog posts (`app/services/hugo_service.rb`)

Trigger: `POST /notes` with `template=hugo, title, parent`. The frontend sends `parent: "blog"` when the destination
came from the selection (file_operations_controller.js:254). Otherwise it sends the explicitly chosen folder or `""`.

### 4.1 Slug algorithm (`slugify`, hugo_service.rb:123-131)

1. `text.downcase`, with full Unicode lowercasing (`É` → `é`, `İ` → `i̇`).
2. Map **each character** through `ACCENT_MAP`. Unmapped characters are kept:

   ```
   à á â ã ä å → a    æ → ae   ç č ć → c   è é ê ë ě → e   ì í î ï → i   ð ď → d
   ñ ň → n   ò ó ô õ ö ø → o   ù ú û ü ů → u   ý ÿ → y   ž ź ż → z   ß → ss   þ → th
   š ś → s   ř → r   ł → l
   ```

   These are single precomposed code points only; there is **no NFD decomposition**. `œ`, `ğ`, `ı`, `ő`, `ű`,
   Cyrillic, CJK, and emoji are not mapped and get removed in the next step.
3. `gsub(/[^a-z0-9]+/, "-")`: each run of anything outside ASCII lowercase alphanumerics becomes one `-`.
4. Strip leading and trailing `-`, then collapse runs of `-`.

Verified examples: `"Conexão à Internet"` → `conexao-a-internet`. `"What's New in 2026?"` → **`what-s-new-in-2026`**
(the README's `whats-new-in-2026` is wrong). `"Código & Programação"` → `codigo-programacao`. `"ÉCOLE Straße Ærø"` →
`ecole-strasse-aero`. `"Привет мир"` → `""`. `"Œuvre ğ ı İ"` → `uvre-i`.

**[QUIRK-DROP]** An empty slug yields the path `YYYY/MM/DD//index.md`. PathSafety cleans it to `YYYY/MM/DD/index.md`
on disk, but the API response `path` keeps the `//`.

### 4.2 Paths

- `path_style = Config.new.get("hugo_path_style") || "dated"`. `"flat"` → `<slug>.md`. Any other value →
  `<YYYY>/<MM>/<DD>/<slug>/index.md`.
- If `parent` is present: `<parent>/<relative>`.
- The note is created with `create_only` (it fails if it exists). Directories are created as needed.

### 4.3 Front matter and date

`DEFAULT_TEMPLATE` (hugo_service.rb:110-119) is exactly (note the **blank line after the closing `---`**, so the
content ends with `---\n\n`):

```
---
title: "{{title}}"
slug: "{{slug}}"
date: {{date}}
draft: true
tags: []
---

```

- Custom template: if `NOTES/.hugo_template.md` is a file, its **entire contents** replace the default. A read error
  falls back to the default.
- Substitution: `{{title}}` → the title with every `"` replaced by `\"`. **Backslashes are not escaped**, so
  `\o/` stays `\o/`. `{{slug}}` → slug. `{{date}}` → date. Every occurrence is replaced. Substitution is literal
  (no backreference interpretation).
- **Date**: `Time.current.strftime("%Y-%m-%dT%H:%M:%S%:z")`, e.g. `2026-11-04T15:30:00+00:00`. The date folders use
  `strftime("%Y/%m/%d")` on the same instant. **[COMPAT]/[DECIDE]** `Time.current` uses the **Rails time zone,
  which is unset and therefore UTC**. Front-matter dates always carry `+00:00`, and the date folder is the UTC date,
  even outside Docker. (The README example `-0300` is stale.) A desktop rewrite would naturally use local time, so
  decide whether to keep UTC for compatibility.

### 4.4 Slug update on folder rename (`update_frontmatter_slug`, hugo_service.rb:182-201)

- Applies only if the content starts with `---`. Split on `/^---\s*$/` into at most 3 parts. At least 3 are
  required.
- The front matter must contain `slug:`.
- Three successive gsubs on the front matter: `^slug:\s*"[^"]*"`, then `^slug:\s*'[^']*'`, then `^slug:\s*[^\s\n]+`,
  each replaced by `slug: "<new>"`.
- If nothing changed → `nil` and no write. Otherwise rebuild as `"---" + fm + "---" + body` and write it with the
  upsert write (atomic).
- The new slug is `slugify(File.basename(new_folder_path))`. Only `<folder>/index.md` is updated, not nested
  posts.

---

## 5. Wikilinks and backlinks (`app/services/backlinks_service.rb`)

`GET /notes/*path/backlinks` → `{ "backlinks": [...], "count": n }`. The path is normalized with `.md` first.

- Target name = `basename(path, ".md")`. Target path without extension = `path` minus `.md`.
- Three case-insensitive patterns, joined into one regex with `Regexp.union`:
  1. `\[\[<name>(\.md)?\]\]`
  2. `\[\[<path_no_ext>(\.md)?\]\]`
  3. `\[[^\]]*\]\([^)]*<path_no_ext>(\.md)?[^)]*\)`: a markdown link whose destination **contains** the path as a
     substring (so `[x](../archive/foo)` and `[x](foo-bar)` also count for `foo`).
- No `[[name|alias]]` or `[[name#heading]]` support (`]]` must follow directly). Name matching is a literal escaped
  substring inside `[[...]]`.
- Files: recursive, skipping dot-entries and requiring `extname == ".md"`. **Symlinks are NOT skipped here**, unlike
  search. **[QUIRK-DROP]** The walk is unsorted, there is no size cap, and lines are not scrubbed, so invalid UTF-8
  can raise. The target file itself is skipped.
- Result items use the same shape as search minus `mtime` (`line_number, match_text, context[±3], path, name`). They
  are **not sorted**: filesystem order, then line order.
- The path contains `..` → `ArgumentError` → 500.

Wikilink *resolution* for click-through (name → file) is done in the frontend, which uses the tree.

---

## 6. Recent files (`app/services/recent_files_service.rb`)

- **Storage**: the `recent_files` key in `NOTES/.fed`, as one line holding a compact JSON array:
  `recent_files = [{"path":"a/b.md","opened_at":"2026-11-04T15:30:00.123456Z"}]`.
  It is written via `Config#modify` (locked read-modify-write) and lands at the end of the file the first time.
- `opened_at` = `Time.current.utc.iso8601(6)`: UTC, **6 fractional digits**, `Z` suffix. **[COMPAT]**
- `LIMIT = 10`. Inputs are truncated to the first 100 before normalizing.
- `normalize` (applied on every read and before every mutation except `remap`): keep only items that are Hashes with
  a valid path, a String `opened_at`, and a file that is **present**. Then de-duplicate by path, keeping the first;
  take the first 10; slice each item to `{path, opened_at}`, so extra keys are dropped.
- `valid_path?`: a String of at most 4096 bytes, ending in `.md`, with no `\` and no NUL. No segment may be empty or
  start with `.`, which rules out leading `/`, `//`, hidden files, and `..`. It must also pass `PathSafety.contain`.
- `present?`: `stat(NOTES/path).file?` (follows symlinks). ENOENT/ENOTDIR → false. **EACCES → true**, because a
  permission problem is not evidence of deletion.
- `record(path)` (`POST /recent_files path=…`): raises NotFound (→ 404) if the path is invalid or missing.
  Otherwise it prepends `{path, opened_at: now}`, removes other entries with that path, and keeps the first 10.
  Returns 204 on success, 503 if the save failed.
- `clear` (`DELETE /recent_files`) → `[]` → 204/503.
- `remove(path)` (on note or folder delete): drops entries equal to the path **or** starting with `path + "/"`.
- `remap(old, new)` (on rename/move, for notes and folders): entries equal to `old` or prefixed by `old/` get `old`
  replaced by `new`. Normalization runs afterwards, so entries whose new path does not exist are dropped.
- Missing files are filtered **on read but not persisted** until the next mutation.
- Rendering (`RecentFilesHelper`): relative time buckets. 0 s → "Just now". Over 1 week → "Over a week ago". Then
  seconds, minutes, hours, or days (pluralized). An unparseable time → "Unknown time". The Home panel shows the first
  5 and hides the rest behind "Show more".

---

## 7. Images and media

### 7.1 Server image folder (`images_path`) (`ImagesService`)

- `enabled?` = `Config.get("images_path")` present. `images_path` = `Pathname(value).expand_path`, which expands
  `~` and resolves relative paths against the **process cwd**.
- `SUPPORTED_EXTENSIONS` (listing) = `.jpg .jpeg .png .gif .webp .svg .bmp`, compared case-insensitively.
- `GET /images?search=&limit=&offset=` → `{images, total, limit, offset}`:
  - `limit` defaults to 10 and is clamped to 1..100. `offset` is at least 0. Both are parsed with `to_i` in the
    controller and `Integer()` in the service.
  - Recursive glob `**/*` (Ruby glob skips dotfiles and does **not** descend into hidden dirs; it follows directory
    symlinks). Regular files with a supported extension only. `search` is a case-insensitive substring match on the
    basename.
  - Sorted by mtime descending (seconds), then sliced.
  - Item: `{name, path (relative to images_path), full_path (absolute), mtime (ISO-8601, local offset, no
    fraction), size, width, height}`.
  - `width`/`height` come from running **ImageMagick `identify -format %wx%h <file>` on every listed file** (the
    first `\d+x\d+` match is used). Failure → `null`.
  - `total` = the count of all matches.
- `GET /images/preview/*path` → `send_file` inline (MIME by extension), or 404. `find_image`: contained within
  `images_path` (PathSafety), must exist as a file, and **its extension must be in `image_upload_extensions +
  video_upload_extensions`**. **[QUIRK-DROP]** `.svg` is listed but cannot be previewed with the default config.
- `DELETE /images/file/*path` → **deletes a file from the user's server image folder** (same `find_image` rules) →
  `{success:true}` or 422 `{"error":"Image not found or could not be deleted"}`.
- `index`, `preview`, `destroy`, `upload_to_s3`, and `upload_external_to_s3` require `images_path` to be configured,
  else **404 `{"error":"Images not configured"}`** (images_controller.rb:9, 240-244). **[QUIRK-DROP]** The two S3
  upload endpoints should not need `images_path`.
- `GET /images/config` → `{enabled, s3_enabled, media_storage_provider, media_storage_fallback,
  web_search_enabled: true, google_enabled, pinterest_enabled: true, image_extensions: [...], video_extensions:
  [...]}`. It also runs storage recovery (§7.3) without consuming the flag.

### 7.2 Shared upload rules (`app/services/upload_storage.rb`)

- `SANITIZE_RE = /[^a-zA-Z0-9._-]/` → `_`. Path separators become `_`. Dots are **kept**:
  `../../../etc/evil.mp4` → `.._.._.._etc_evil.mp4`, a real example present in `notes/videos/`. Non-ASCII becomes
  `_` (one `_` per character, not per byte).
- `MAX_UPLOAD_BYTES = 512 MiB` (536870912). Exceeding it gives the error `"File is too large (max 512 MB)."`.
- Base64 encoded-size cap: `((MAX+2)/3)*4 + 1024` bytes, checked before decoding.
- Extension allow-list: `File.extname(name).downcase` must be in the configured list. Otherwise:
  `"<.ext|no extension> is not an accepted <kind> type. Accepted: .jpg, .jpeg, ..."` (comma-space joined).
- **Local destination filename** (`dest_filename`): `"<YYYYMMDD>_<HHMMSS>_<8 hex>_<sanitized original>"`. It uses
  `Time.now`, the **process local time zone** (UTC in Docker), and `SecureRandom.hex(4)`. **[COMPAT]** for the
  pattern. Older files on disk have the form `YYYYMMDD_HHMMSS_<name>` without the hex part. There is no dedupe;
  `cp` would overwrite on an (astronomically unlikely) collision.
- Temp copies: `<app>/tmp/uploads/<32 hex><ext>`, always deleted afterwards.
- `provider(legacy_s3:)` (upload_storage.rb:35-46):
  - The provider is `Config.get("media_storage_provider")`. Not `local`/`s3` → rejected `"Unknown media storage
    provider"`.
  - If the client sent `upload_to_s3` and its boolean cast **disagrees** with the provider → rejected `"Upload
    destination conflicts with the active media storage provider"`. Rails `Boolean.cast` is false for
    `"false","0","f","off",""…`.
  - Provider `s3` but not configured → rejected `"S3 is not configured"`.
- `s3_configured?`: access key, secret, and bucket are all present. If an endpoint is set, it must parse as an
  `http`/`https` URI with a host and **no userinfo**.

### 7.3 Media storage recovery (`app/controllers/concerns/media_storage_recovery.rb`)

- On `GET /`, `GET /notes/<note>` (HTML), `GET /config`, `GET /images/config`, and Library requests: if the provider
  is `s3` but S3 is unconfigured, **write `media_storage_provider = local` to `.fed`** and set the session flag
  `media_storage_fallback`. The flag is reported as `media_storage_fallback: true`. It is *consumed* (cleared) by
  index/show/`GET /config`, and cleared on `PATCH /config` that includes `media_storage_provider`.
- Before `upload`, `upload_base64`, `import`, `upload_to_s3`, `upload_external_to_s3`, and `media/upload`: the same
  check. If it switches to local → **422** `{"error":"S3 is no longer available. The upload provider is now Local.
  Please retry the upload.", "media_storage_provider":"local", "media_storage_fallback":true}`.
- After an S3 exception during upload: if the error is an *invalid configuration* error (`Aws::Errors::
  InvalidRegionError`, or an S3 ServiceError with code in `InvalidAccessKeyId SignatureDoesNotMatch InvalidToken
  ExpiredToken TokenRefreshRequired AccessDenied NoSuchBucket InvalidBucketName AuthorizationHeaderMalformed`) and
  the provider is `s3`, switch to local and return the same 422. Timeouts, throttling, and 5xx errors do not switch.
- **[DECIDE]** The session flag needs an equivalent (in-memory app state) in the desktop app.

### 7.4 Image upload: `POST /images/upload` (multipart)

Params: `file` (required, else 422 `"No file provided"`), `resize`, `upload_to_s3` (legacy), `s3_key`.

- `parse_resize_ratio` (images_controller.rb:251-259): `true`/`"true"` → 0.5. Blank, `"false"` or `false` → nil.
  Otherwise `to_f`, kept only if it lies in `(0, 1]`.
- Size cap, then extension against `image_upload_extensions`, then a temp copy.
- **Local**:
  - The safe name is the sanitized original filename.
  - If `resize` is present and positive: the destination name is `dest_filename("<basename without ext>.jpg")`. The
    file is resized through ImageMagick (§7.10) and the JPEG bytes are written. **[QUIRK-DROP]** If ImageMagick fails
    or is missing, the *original* bytes are written under the `.jpg` name.
  - Otherwise: `cp` to `NOTES/images/<dest_filename(safe)>`.
  - Returns `{"url":"images/<dest>"}`. This is a **relative URL without a leading slash**, inserted into the note
    as-is. **[COMPAT]**
- **S3**: see §7.6, with `custom_key: s3_key`.
- Errors: a rejection → 422 `{error: msg}`. Any other exception → recovery check, else 422 `{"error":"Upload
  failed"}`.

### 7.5 Base64 upload: `POST /images/upload_base64`

Params: `data` (required, else 400 `"No image data provided"`), `mime_type`, `filename`, `upload_to_s3`, `s3_prefix`.
The frontend uses this for AI-generated images.

- Strict base64 decode. Invalid input → `{"error":"Invalid base64 data: <msg>"}` (422).
- `mime_type` defaults to `image/png`. Extension map: jpeg → `.jpg`, png, gif, webp, `svg+xml` → `.svg`,
  `bmp`/`x-ms-bmp` → `.bmp`, anything else → `.jpg`.
- `filename` defaults to `ai_generated_<16 hex><ext>`. If it does not match `/\.\w+$/`, the extension is appended.
- Size cap, then `validate_filename!(filename)` against image extensions (so SVG is rejected by default).
- Local: `NOTES/images/<dest_filename(sanitized filename)>` → `{url:"images/..."}`. No resize.
- S3: key = `s3_key(filename, custom_prefix: s3_prefix)`. **[QUIRK-DROP]** The content type is derived from the
  *mime-derived temp extension*, not from the filename.

### 7.6 S3 uploads (all paths)

Client (`UploadStorage.s3_client`):

- Credentials, `region = aws_region || "us-east-1"`.
- If an endpoint is configured: `endpoint`, plus `force_path_style = !!aws_s3_force_path_style`.
- Standard SigV4 through aws-sdk-s3 1.235 (which defaults to CRC32 request checksums and honors standard `AWS_*`
  SDK env vars such as `AWS_ENDPOINT_URL*`, `AWS_PROFILE` and proxies; FrankMD does not use those on purpose).

`put_object(bucket, key, body, content_type, if_none_match: "*")`:

- This is a **conditional create** (no overwrite). `PreconditionFailed` → `nil`/error ("Failed to upload" /
  "Image upload failed" / "Video upload failed").
- **No ACL** is sent. `AccessControlListNotSupported` is rescued and ignored.
- Content type by extension: `.jpg/.jpeg` → image/jpeg, png, gif, webp, `svg+xml`, bmp, else
  `application/octet-stream`. A resize forces `image/jpeg`. Videos are covered in §7.11.

**Key naming** (`s3_key`, upload_storage.rb:187-213):

- `custom_key` given (the user typed a full key; the frontend precomputes the default `frankmd/YYYY/MM/<name>`):
  sanitize per segment (split on `/`, drop empty `.`/`..` segments, apply SANITIZE_RE to each segment, keep `/`).
  If non-empty → `unique_s3_key(key)`.
- Else: `safe = sanitize(filename)`. `prefix = sanitize_s3_key(custom_prefix)` or
  `"frankmd/" + Time.current.strftime("%Y/%m")` (**UTC**). Then `unique_s3_key("#{prefix}/#{safe}")`.
- `unique_s3_key(key, original)`: `ext = extname(key)`, or `extname(original)` if that is blank. Then
  `"<key minus ext>-<16 hex><ext>"`. Examples: `frankmd/2026/11/photo-0123456789abcdef.jpg`; custom `blog/cover.jpg`
  → `blog/cover-<16hex>.jpg`. **[COMPAT]**

**Public URL** (`s3_url`): each key segment is percent-encoded with `ERB::Util.url_encode` (everything except
`[A-Za-z0-9_.~-]`).

- No endpoint → `https://<bucket>.s3.<region>.amazonaws.com/<key>`.
- Endpoint plus path-style → `<endpoint without trailing />/<bucket>/<key>`.
- Endpoint, virtual-host style → `<scheme>://<bucket>.<host>[:<port> unless default]/<key>`. **[QUIRK-DROP]** Any
  path component of the endpoint is lost here.

Other S3 endpoints:

- `POST /images/upload_to_s3` (`path`, `resize`, `s3_key`): uploads an image from `images_path` (§7.1). It requires
  provider = s3, otherwise the `RejectedError` comes back as 422 `{"error":"UploadStorage::RejectedError: Upload
  destination conflicts…"}`, because this endpoint formats errors as `"#{class}: #{message}"`. It also requires
  `images_path` (404 otherwise) and S3 configured (422 `"S3 not configured"`). Returns `{url}`.
- `POST /images/upload_external_to_s3` (`url`, `resize`, `s3_prefix`): blank url → 400 `"URL is required"`. Fetches
  through the egress policy (§8), then names the object `basename(url path)` sanitized. If that is blank, `_`, or has
  no extension → `<16 hex><ext from content-type>`. Optional resize, then `s3_key(name, custom_prefix)`. Same
  `images_path` requirement and error formatting as above.

### 7.7 Import: `POST /images/import`

Params: exactly one of `url` / `path` (both strings), plus `resize`, `s3_key`, `s3_prefix`, `upload_to_s3`. The
frontend uses this for web-search "save a copy" and for server-folder images.

- If not exactly one is given → 422 `"Provide exactly one image URL or server image path"`.
- `path`: `find_image` in `images_path` (not found → `"Image not found"`). Validate its basename against image
  extensions and the size cap, then store.
- `url`: fetch via egress (fail → `"Image download failed"`). The Content-Type (before `;`, lowercased) must be one
  of `image/jpeg image/png image/gif image/webp image/bmp image/x-ms-bmp`, else `"Unsupported image content type"`.
  Then `validate_filename!("image<ext>")`. Name = sanitized URL basename with its extension replaced by the mime
  extension, defaulting to `image<ext>`.
- Store: S3 → `{url}`, or `{"error":"Image upload failed"}` on a nil result. Local → `{url:"images/<dest>"}` with an
  optional resize. The response is the whole result hash.

### 7.8 Media Library: local (`MediaLibraryService`)

`GET /library` (provider local) → `{"images":[...], "videos":[...]}`:

- The roots are `NOTES/images` and `NOTES/videos`. They are skipped if they are not directories or **are
  symlinks**.
- Recursive walk that skips symlinks and dot-entries, files only. Images must match `image_upload_extensions`,
  videos must match `video_upload_extensions`.
- Each item must satisfy `PathSafety.contain(NOTES, rel) == file`.
- Item: `{name, path: "images/sub/x.png", size, mtime: ISO-8601 (local offset, no fraction), preview_url:
  "/notes/<url-encoded segments>", type: "image"|"video"}`.
- Sorted by mtime descending (float precision), then path lowercase ascending.
- Files that vanish mid-listing are skipped.

`DELETE /library/file/*path` → 204, or 404 `{"error":"Media file not found."}`, or 422 on SystemCallError. The path
is checked by `safe_media_file` (media_library_service.rb:87-118):

- At least 2 segments. None blank, `.`, `..`, or containing `\` or NUL.
- The first segment must be `images` or `videos`. The extension must be in that kind's list.
- PathSafety-contained and equal to the lexical join, and also contained in the managed root.
- `lstat` every component: no symlinks anywhere. Intermediate components must be directories and the leaf must be a
  regular file.

`PATCH /library/file/*path` with a local provider → **always 422** `{"error":"Could not rename this media file."}`.
The local service has no `rename`. **[QUIRK-DROP]** Local rename is unimplemented.

`GET /library/picker?collection=managed|server&search=&limit=&offset=` → Turbo Stream HTML (`ImageLibraryService`).
An unknown collection → 400.

- `managed`: the local library's images, filtered by a case-insensitive name substring, then sliced. Each item gets
  `url = path` (relative).
- `server`: `ImagesService.list` items mapped to `{name, path, url: "/images/preview/<encoded>", preview_url:
  same}`.
- The result also carries `{collection, total, limit, offset, server_available: images_path is a dir}`.

### 7.9 Media usage scanning (`MediaUsageService`)

`GET /library/usage` → `{"usage_counts": {"images/a.png": 2, ...}}`. With `?path=images/a.png` →
`{"usage_notes": ["notes/x.md", ...]}`. A scan failure → 503 `{"error":"Could not scan notes for media usage."}`.

- Notes: recursive walk of NOTES, **sorted by full path string**. Dot-entries and symlinks are skipped. Regular files
  with `.md` only. The root images/videos dirs are included.
- Cache: the index is keyed by root and fingerprint. The fingerprint is a list of `[relpath, dev, ino, size,
  mtime(rational), ctime(rational)]` for every note. At most 8 roots are cached; it is process-global. A read error
  aborts the whole scan (no partial results).
- Content is read as UTF-8 and scrubbed. Then:
  1. Mask fenced code blocks. A fence is `^ {0,3}(```+|~~~+)`. It closes on a line containing only the same fence
     character, repeated at least as many times. Masked text is replaced by spaces, newlines kept.
  2. Mask HTML comments `<!-- … -->`, then inline code (a backtick run, a lazy match, the same run), unless the
     opening run is preceded by `\`.
  3. Reference definitions: `^ {0,3}\[label\]:[ \t]*(<dest>|dest)`. The label is normalized (HTML-unescape, unescape
     `\punct`, strip, collapse whitespace, downcase). The first definition wins. Definition lines are masked.
  4. Inline destinations: `[text](<dest>)` or `[text](dest)` (dest has no whitespace, and `\` escapes are allowed),
     not preceded by `\`. This also covers images, because `![` contains `[`.
  5. Reference uses: `![a][b]`, `![a][]`, `![a]`, `[a][b]`, `[a]`. A bare `[a]` followed by `(` is skipped. The
     label is the second label, or the first if the second is empty or missing. It is resolved through the
     definitions.
  6. HTML: `<img|video|audio|source|track|embed|object … src=("…"|'…'|bare)>`.
- `canonical_media_path(ref, note_path)`:
  - HTML-unescape and strip. Cut at the first unescaped `?` or `#`. Unescape `\punct`.
  - Reject an empty ref, a URL scheme (`^[a-z][a-z0-9+.-]*:`), and `//…`.
  - Absolute (`/…`): drop the leading `/`, and also a leading `notes/` segment, then resolve from the root.
    Relative: resolve against the note's directory.
  - Each segment is percent-decoded (`URI::DEFAULT_PARSER.unescape`). Reject an empty segment, or a decoded segment
    containing `/` or `\`. `.` is skipped. `..` pops a segment (rejected at the root).
  - Valid only when the first segment is `images` or `videos` and there are at least 2 segments. The result is
    `images/...`.
- The output is sorted by media path, and each note list is sorted.
- With S3 the frontend shows usage as "unknown" (no server call).

### 7.10 Image resizing (ImageMagick)

- Binary choice at call time: if `magick` is on `PATH`, use `magick` for convert and `magick identify` for identify.
  Otherwise use `convert` / `identify` (images_service.rb:492-505).
- `resize_and_compress`: copy the source to a temp `frankmd_source<ext>`, then run
  `convert <tmp> -resize <N>% -quality 95 -strip <tmp>.jpg` and read the output. The output name is
  `<basename>.jpg` and the type is `image/jpeg`.
- Ratio → percentage (`ratio.to_s`): `0.25` → 25%, `0.33/0.333` → 33%, `0.5`/`"true"` → 50%, `0.67/0.666/0.667` →
  67%, `0.75` → 75%, `1/1.0` → 100%. **Anything else → 50%** (for example 0.4).
- A non-zero exit or a missing binary → fall back to the original bytes, original content type, and original name.
  On the local path, the name was already chosen as `.jpg`; see §7.4.
- **[DECIDE]** Replace this with an in-process Rust image crate. Keep the percentages and JPEG quality 95, strip
  metadata (EXIF), and decide how to handle the "fallback to original" behavior. Note: an animated GIF converted
  through ImageMagick to a single `.jpg` would produce `out-0.jpg…` files, so the current behavior in that case is
  effectively broken.
- **No resizing, thumbnailing, or EXIF processing happens anywhere else.** Local uploads without `resize` are
  byte-for-byte copies.

### 7.11 Video upload: `POST /media/upload` (multipart)

Params: `file`, `upload_to_s3`, `s3_prefix`.

- Size cap and `video_upload_extensions`. Never re-encoded.
- Local → `NOTES/videos/<dest_filename(original)>` → `{"url":"videos/<dest>"}`.
- S3 → key `s3_key(original, custom_prefix: s3_prefix)`. Content type from the key extension: mp4 → video/mp4, webm
  → video/webm, mkv → video/x-matroska, mov → video/quicktime, avi → video/x-msvideo, m4v → video/x-m4v, ogv →
  video/ogg, else octet-stream. The body is streamed from the temp file, with `if_none_match: "*"`.
  PreconditionFailed → `{"error":"Video upload failed"}`.

### 7.12 S3 Media Library (`S3MediaLibraryService`, provider = s3)

- `GET /library`: `list_objects_v2(prefix: "frankmd/")`, paginated by continuation token. More than 5000 objects →
  RejectedError, which `LibraryController#index` does not rescue, so the response is a 500. **[QUIRK-DROP]** A
  truncated page without a token is also an error.
- Keys must be valid (`valid_key?`): a string starting with `frankmd/`, at most 1024 bytes, with every `/` segment
  non-empty and not `.`, `..`, containing `\`, or containing NUL.
- Classification is by extension: in image extensions → images (image takes priority), else video extensions →
  videos, else ignored.
- Item: `{name, path: key, size, mtime: last_modified ISO-8601, preview_url: url, url, type, provider:"s3"}`.
  Sorted by mtime descending, then key ascending.
- `DELETE /library/file/<key>` → `delete_object`, only for valid managed keys.
- `PATCH /library/file/<key>` with `name`. The new name must equal its own basename, must already be sanitized
  (unchanged by SANITIZE_RE), and must have an extension of the same kind. The destination is in the same directory
  and must be a valid key, different from the source, and absent (`head_object` returns NotFound). Then
  `copy_object(copy_source: url-encoded "<bucket>/<key segments>")` followed by `delete_object`. Response `{name,
  path, url}`. Failure → 422 `"Could not rename this media file."`.

### 7.13 Web image search

`GET /images/search_web?q=` (DuckDuckGo) and `GET /images/search_pinterest?q=` (DuckDuckGo with
`"<q> site:pinterest.com"`). Neither needs configuration. A blank q → 400 `{"error":"Query is required"}`.
images_controller.rb:310-393:

1. `GET https://duckduckgo.com/?q=<form-encoded query>&iax=images&ia=images` with
   `User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko)
   Chrome/120.0.0.0 Safari/537.36` and `Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8`.
   Timeouts: open 5 s, read 10 s.
2. Extract `vqd` with the first matching pattern of `/vqd=["']?([\d-]+)["']?/`, `/vqd=([\d-]+)/`,
   `/"vqd":"([\d-]+)"/`. If none matches → `{"images":[], "note":"DuckDuckGo search temporarily unavailable"}` (200).
3. `GET https://duckduckgo.com/i.js?l=us-en&o=json&q=<query>&vqd=<vqd>&f=,,,,,&p=1` with the same UA,
   `Accept: application/json, text/javascript, */*; q=0.01`, and `Referer: https://duckduckgo.com/`. A non-2xx
   status → `{"images":[], "error":"Failed to fetch images"}`.
4. Map the first 20 of `results` to `{url: image, thumbnail, title, source, width, height}` → `{"images":[...]}`.
   A JSON error → `{"images":[], "error":"Failed to parse results"}`. Other errors → `"Search failed"`. All of these
   are **status 200**.

`GET /images/search_google?q=&start=` requires `google_api_key` and `google_cse_id` (else 503
`"Google Custom Search not configured"`):

- `GET https://www.googleapis.com/customsearch/v1?key&cx&q&searchType=image&num=10&start=max(start,1)&safe=active`
  (Net::HTTP defaults: 60 s timeouts).
- A non-2xx response → `{"error": <error.message or "Google API error (<code>)">, "images":[]}` (200).
- Success → `{images:[{url: link, thumbnail: image.thumbnailLink||link, title, source: displayLink, width:
  image.width, height: image.height}], total: int(searchInformation.totalResults), next_start: start + 10}`.
  **[QUIRK-DROP]** `start=0` requests page 1 but answers `next_start=10`.
- Exceptions → 500 `{"error":"Search failed"}`.

These search fetches go to fixed hosts and are not subject to the egress policy.

---

## 8. Egress policy (SSRF protection) (`app/services/egress_policy.rb`)

It is used **only** by `ImagesService.fetch_external_image`, which serves `upload_external_to_s3` and `import(url)`.
It is not used for AI-returned image URLs (ruby_llm downloads those itself), Ollama, or the search APIs.

`checked_target(url)`:

1. `URI.parse`. Invalid → `"Invalid URL"`. The scheme must be `http` or `https`, and the host must be non-blank.
2. Resolve with `getaddrinfo(host, nil, nil, :STREAM)`, collecting all addresses. A resolution error or no results →
   `"Could not resolve <host>"`.
3. **Reject if ANY resolved address is blocked**:
   - An IPv4-mapped IPv6 address (`::ffff:a.b.c.d`) is judged by its IPv4 value.
   - Loopback (127/8, ::1), private (10/8, 172.16/12, 192.168/16, fc00::/7), link-local (169.254/16, fe80::/10).
   - IPv4 `0.0.0.0`.
   - Extra ranges: 0.0.0.0/8, 100.64.0.0/10, 192.0.0.0/24, 192.0.2.0/24, 198.18.0.0/15, 198.51.100.0/24,
     203.0.113.0/24, 240.0.0.0/4, `::/128`, `100::/64`.
   - Multicast 224.0.0.0/4 and ff00::/8.
   - Not covered (allowed): 255.255.255.255 is inside 240/4, so it is blocked; IPv6 documentation `2001:db8::/32`,
     `64:ff9b::/96` NAT64, 6to4, and IPv4-compatible `::a.b.c.d` are **not** blocked.
4. Return `[uri, first resolved address]`.

Fetch loop (images_service.rb:523-580):

- At most 5 redirects (6 hops). Each hop is re-checked, and the **connection is pinned to the validated IP**
  (`http.ipaddr =`), while the hostname is kept for Host and SNI.
- Open timeout 10 s, read timeout 30 s, and a **30 s total deadline** (monotonic clock).
- `User-Agent: Mozilla/5.0 (compatible; FrankMD/1.0)`.
- A 3xx with `Location` → resolve it relative to the current URL and loop. Other non-2xx → nil.
- `Content-Length` > 25 MiB → nil. The body is streamed with a 25 MiB cap (exceeding it → nil).
- Content type defaults to `image/jpeg` when absent.
- A blocked target is logged, and the caller returns its generic failure.

---

## 9. AI service (`app/services/ai_service.rb`, `config/initializers/ruby_llm_image_patch.rb`)

Library: **ruby_llm 2.1.1**. Gem defaults that apply: `request_timeout` **300 s**, `max_retries` **3** with
exponential backoff (interval 0.1 s, factor 2, randomness 0.5, max 30 s) on 429/5xx/transport errors. There is no
explicit temperature or max-tokens; Anthropic gets `max_tokens` from the model registry or the ruby_llm default.

### 9.1 Grammar fix: `POST /ai/fix_grammar` (`path`)

- A blank path → 400 `"No file provided"`. **The text is read from disk** (`Note.find(path)`), not from the editor
  buffer, so the frontend must save first. Not found → 404. Empty content (`blank?`, i.e. whitespace only) → 400
  `"Note is empty"`.
- **[QUIRK-DROP / security]** `.fed` passes `Note.find`, so `path=.fed` would send the config file, secrets
  included, to the LLM. This endpoint is not covered by `reject_config_file`.
- No provider or model → 422 `"AI not configured"` / `"No AI provider available"`.
- Request: provider = `effective_ai_provider`, model = `effective_ai_model`. The client is configured with **only**
  that provider's key, read through `get_ai`. For `ollama`, `ollama_api_base` must parse as http/https, else the
  request errors.
- `RubyLLM.chat(model:, provider:, assume_model_exists: provider == "ollama")`. **[QUIRK-DROP]** For the other
  providers the model must exist in ruby_llm's bundled registry, so an unknown or new model id fails with
  `"AI provider request failed"`.
- `with_instructions(GRAMMAR_PROMPT)` sends a system message. Then `ask(text)` sends the whole note as one user
  message. Non-streaming.
- Wire endpoints (ruby_llm 2.1.1 defaults):

  | Provider | Endpoint | Auth |
  |---|---|---|
  | openai | `https://api.openai.com/v1/responses` (**Responses API**) | `Authorization: Bearer` |
  | anthropic | `https://api.anthropic.com/v1/messages` | `x-api-key`, `anthropic-version: 2023-06-01` |
  | gemini | `https://generativelanguage.googleapis.com/v1beta/models/<model>:generateContent` | `x-goog-api-key` |
  | openrouter | `https://openrouter.ai/api/v1/chat/completions` | `Authorization: Bearer`, `HTTP-Referer: https://rubyllm.com`, `X-OpenRouter-Title: RubyLLM` |
  | ollama | `<ollama_api_base>/chat/completions` (OpenAI-compatible, so the base must end in `/v1`) | none |

- Response: 200 `{original: <file text>, corrected: <response content>, provider, model}`. Any exception → 422
  `{"error":"AI provider request failed"}`. Errors are logged without the content.

`GRAMMAR_PROMPT` verbatim (heredoc, ends with a newline):

```
You are a grammar and spelling corrector. Fix ONLY:
- Grammar errors
- Spelling mistakes
- Typos
- Punctuation errors

DO NOT change:
- Facts, opinions, or meaning
- Writing style or tone
- Markdown formatting (headers, links, code blocks, lists, etc.)
- Technical terms or proper nouns
- Code blocks or inline code

Return ONLY the corrected text with no explanations or commentary.
```

`GET /ai/config` → `{enabled, provider, model, available_providers: [...]}` (the order is given in §1.3).

### 9.2 Image generation: `POST /ai/generate_image` (`prompt`, `reference_image_path`)

- `enabled` = the OpenRouter key for images is present (§1.3 exception). The only provider is **OpenRouter**. The
  model is `get("image_generation_model")`, default `google/gemini-3.1-flash-image-preview`.
- A blank prompt → 400 `"No prompt provided"`. Not enabled → 422 `"Image generation not configured. Requires
  OpenRouter API key."`.
- `reference_image_path` is resolved with `ImagesService.find_image` (inside `images_path` only). If it is missing,
  it is silently ignored.
- The request is an OpenRouter chat completion with `modalities: ["text","image"]`. The user message is the prompt
  **verbatim**: there is no system prompt and no wrapping text. With a reference, the image is attached as an
  image-content part (ruby_llm `with:`).
- Response parsing: OpenRouter returns `choices[0].message.images[]`, each with `image_url.url` (a `data:<mime>;
  base64,<data>` URI or a plain URL). The initializer patch converts these into attachments, and the first
  attachment is used. **[DECIDE]** The patch hooks `Providers::OpenRouter#parse_completion_response`, but ruby_llm 2.x
  moved parsing into protocol classes, so the patch may be dead and the gem may parse images natively. The rewrite
  should parse `message.images[0].image_url.url` directly, decode data URIs, and download plain URLs.
- Success → `{data: <base64 of bytes>, mime_type: <attachment mime or "image/png">, model, revised_prompt: null}`.
  With no image → 422 `"No image data in response"`. An exception → 422 `"Image generation failed"`.
- The frontend then stores the image through `/images/upload_base64` (or `/images/import` for URLs).
- The initializer also registers model metadata for `google/gemini-3.1-flash-image-preview` (context 65536, max out
  32768) when it is missing from the registry.
- `GET /ai/image_config` → `{enabled, model}`.

---

## 10. YouTube search (`app/controllers/youtube_controller.rb`)

- `GET /youtube/config` → `{enabled: youtube_api_key present}`.
- `GET /youtube/search?q=`: a blank q → 400 `"Query is required"`. No key → 503 `"YouTube API key not configured"`.
- `GET https://www.googleapis.com/youtube/v3/search?part=snippet&q=<q>&type=video&maxResults=6&key=<key>`
  (Net::HTTP default timeouts).
- A non-2xx response → `{"error":"YouTube API error","videos":[]}` with status **200**.
- Success → `{videos:[{id: id.videoId, title: snippet.title, channel: snippet.channelTitle, thumbnail:
  snippet.thumbnails.medium.url || default.url}]}`. With an HTML Accept header it renders a partial instead.
- Exceptions → 500 `"Search failed"`.

Embedding is done in the frontend (iframe HTML or the Hugo `{{< youtube id="…" title="…" >}}` shortcode). CSP
`frame-src` allows youtube.com, www.youtube.com, youtube-nocookie.com, www.youtube-nocookie.com, and
player.vimeo.com.

---

## 11. Themes

### 11.1 Omarchy (`app/services/omarchy_theme_service.rb`, `GET /config/omarchy_theme`)

- Files (the paths are expanded with `~` at class load): `~/.config/omarchy/current/theme.name` and
  `~/.config/omarchy/current/theme/alacritty.toml`. Both must exist, else 404 `{"error":"Omarchy theme not
  available"}`. EACCES or ENOENT while reading → 404.
- `theme_name` = the contents of `theme.name`, stripped.
- Alacritty parse (a line-based TOML subset):
  - Section headers must match `^\[colors\.(\w+)\]$`. Any other `[...]` header ends color parsing until the next
    match.
  - Keys must match `^(\w+)\s*=\s*["']?(#[0-9a-fA-F]{6})["']?\s*$`. Only 6-digit hex is accepted. `0x...` is not,
    and inline comments make the line fail.
  - Sections `primary` → `<key>`, `cursor` → `cursor_<key>`, `selection` → `selection_<key>`, `normal` →
    `normal_<key>`, `bright` → `bright_<key>`. Others are ignored.
- Color math (exact):
  - `hex_to_rgb`. `rgb_to_hex` clamps to 0..255 and **rounds half away from zero** (Ruby `Float#round`), formatted as
    lowercase `#%02x%02x%02x`.
  - `lighten(h, p)` adds `255*p/100` to every channel. `darken` subtracts it.
  - `blend(fg, bg, f)` = `fg + (bg - fg)*f` per channel.
  - `dark_background?(hex)`: nil → true. Otherwise Rec.709 luma on **non-linearized** 0..1 channels
    (`0.2126 r + 0.7152 g + 0.0722 b`) `< 0.5`.
  - `contrast_ratio`: WCAG relative luminance (sRGB linearized: `v <= 0.04045 ? v/12.92 : ((v+0.055)/1.055)^2.4`),
    `(Lmax+0.05)/(Lmin+0.05)`.
  - `ensure_contrast(fg, bg)`: return `fg` if the ratio is ≥ 4.51. Otherwise the target is white if white contrasts
    better against bg than black, else black. Then 24 iterations of a binary search on `blend(fg, target, t)`, with
    the result `blend(fg, target, high)`.
- Mapping. Defaults: `bg = background || #121212`, `fg = foreground || #d0d0d0`, `dim_fg = dim_foreground`,
  `blue = normal_blue || #7ea7c9`, `bright_blue`, `bright_black = normal_black || #555555`,
  `sel_bg = selection_background`, `sel_text = selection_text`, `dark = dark_background?(bg)`,
  `code_bg = dark ? lighten(bg,7) : darken(bg,7)`.

  | Variable | Value |
  |---|---|
  | `--theme-bg-primary` | bg |
  | `--theme-bg-secondary` | dark ? lighten(bg,3) : darken(bg,3) |
  | `--theme-bg-tertiary` | dark ? lighten(bg,7) : darken(bg,7) |
  | `--theme-bg-hover` | dark ? lighten(bg,12) : darken(bg,12) |
  | `--theme-text-primary` | fg |
  | `--theme-text-secondary` | blend(fg,bg,0.08) |
  | `--theme-text-muted` | dim_fg \|\| blend(fg,bg,0.45) |
  | `--theme-text-faint` | blend(fg,bg,0.65) |
  | `--theme-border` | dark ? lighten(bg,12) : darken(bg,12) |
  | `--theme-accent` | blue |
  | `--theme-accent-hover` | bright_blue \|\| lighten(blue,15) |
  | `--theme-accent-text` | bg |
  | `--theme-selection` | sel_bg \|\| blend(fg,bg,0.25) |
  | `--theme-selection-text` | sel_text \|\| fg |
  | `--theme-scrollbar` | bright_black (i.e. normal_black) |
  | `--theme-scrollbar-hover` | lighten(bright_black,10) |
  | `--theme-success` | normal_green \|\| #8673d4 |
  | `--theme-warning` | normal_yellow \|\| #ca2edc |
  | `--theme-error` | normal_red \|\| #ff4da6 |
  | `--theme-code-bg` | code_bg |
  | `--theme-heading-1` | normal_magenta \|\| #b683c3 |
  | `--theme-heading-2` | blue |
  | `--theme-heading-3` | normal_cyan \|\| #8adb8a |
  | `--theme-folder-icon` | blue |
  | `--theme-file-icon` | dim_fg \|\| blend(fg,bg,0.45) |
  | `--theme-config-icon` | normal_yellow \|\| #ca2edc |

  The syntax colors below are all passed through `ensure_contrast(color, code_bg)`:

  | Variable | Value |
  |---|---|
  | `--theme-syntax-keyword` | normal_magenta \|\| normal_red \|\| #c792ea |
  | `--theme-syntax-function` | normal_yellow \|\| bright_yellow \|\| #ffd866 |
  | `--theme-syntax-type` | normal_cyan \|\| bright_cyan \|\| #65d5cb |
  | `--theme-syntax-string` | normal_green \|\| bright_green \|\| #95d96a |
  | `--theme-syntax-number` | bright_yellow \|\| normal_yellow \|\| #ffad66 |
  | `--theme-syntax-constant` | bright_cyan \|\| normal_cyan \|\| #79c0ff |
  | `--theme-syntax-identifier` | fg |
  | `--theme-syntax-property` | bright_blue \|\| blue |
  | `--theme-syntax-operator` | normal_white \|\| fg |
  | `--theme-syntax-comment` | dim_fg \|\| normal_black \|\| blend(fg,bg,0.45) |
  | `--theme-syntax-punctuation` | normal_white \|\| fg |
  | `--theme-syntax-invalid` | normal_red \|\| #ff8787 |

- Response: `{theme_name, variables: {...38 entries...}, is_dark: dark_background?(colors.background)}`. The
  frontend polls it for live sync.

### 11.2 Custom theme (`app/services/custom_theme.rb`)

It is stored in `.fed` as `custom_theme = {"version":1,"name":"…","base":"…","colors":{…}}` (compact JSON, one line).
The UI also saves `theme = custom`. `PATCH /config` receives `custom_theme` as a **JSON string**, because
strong-params `permit` does not allow nested hashes.

Validation (`CustomTheme.parse`). Any failure raises `ArgumentError`: the line is ignored when parsing, and the
PATCH returns 422 `"Failed to save"`.

- A string longer than 16 KiB (bytes) → "Theme is too large".
- JSON `null` → `nil`.
- It must be an object whose keys are a subset of `version, name, base, colors`, with `version == 1` (integer).
- `name`: a string, 1..60 characters after strip, with no control characters (`[[:cntrl:]]`). It is stored
  stripped.
- `base` is one of: `light dark catppuccin catppuccin-latte ethereal everforest flexoki-light gruvbox hackerman
  kanagawa matte-black nord osaka-jade ristretto rose-pine solarized-dark solarized-light tokyo-night`.
- `colors`: an object (it may be empty) whose keys are a subset of the 38 `COLOR_KEYS` (the same names as the
  `--theme-*` variables without the prefix: `bg-primary … syntax-invalid`, see custom_theme.rb:6-12). Every value
  must match `^#[0-9a-fA-F]{6}$`. Values are stored **lowercased**.
- The normalized output key order is `version, name, base, colors`.

---

## 12. Translations / i18n

- Locales: `en, pt-BR, pt-PT, es, he, ja, ko` (`config/application.rb:217`). Files: `config/locales/<locale>.yml`,
  each with 692 leaf keys in 18 top-level sections (`home app common languages header sidebar library settings
  editor preview context_menu status connection dialogs errors confirm success media_storage`). en has 824 lines,
  dialogs alone has 351 leaves. Rails loads them automatically. Pluralization uses `one`/`other` with `%{count}`
  interpolation.
- The default locale is `en`. `config.i18n.fallbacks = true`, so missing keys fall back to `en`.
- Per-request locale: `params[:locale]` || `Config.get("locale")` (`.fed` > `FRANKMD_LOCALE` > `"en"`) ||
  default. It is applied **only if** it is in the available list, otherwise ignored. **[QUIRK-DROP]**
  `I18n.locale` is thread-local and not reset, so with an invalid value the thread keeps the previous request's
  locale.
- `GET /translations[?locale=xx]` → `{"locale":"es","translations":{common, dialogs, status, errors, success, editor,
  sidebar, preview, context_menu, connection, library}}`. Only these 11 sections are included. For each one, the
  locale's hash is **deep-merged over the `en` hash**, so keys missing from the locale come from `en`.
  `home, settings, header, media_storage, languages, app, confirm` are only used server-side in ERB.
  **[DECIDE]** A client-rendered rewrite needs all 18 sections exposed.
- `<html lang="<locale>">`. There is no server-side `dir="rtl"` for Hebrew; any RTL handling is the frontend's.
- The language names in `languages` use the keys `en, pt_BR, pt_PT, es, he, ja, ko` (underscored), unlike the
  locale ids.
- Server-generated error and success strings (`errors.*`, `success.*`, `media_storage.*`, `library.*`) are
  localized. Some service errors are hard-coded English (upload rejections, AI errors, S3 errors).

---

## 13. Other endpoints, sessions, auth

### 13.1 Full route table (`config/routes.rb`)

| Method | Path | Handler | Response |
|---|---|---|---|
| GET | `/` | notes#index | SPA HTML (`?file=` preselects a note) |
| GET/POST | `/login` | sessions#new/create | login page / form post |
| DELETE | `/logout` | sessions#destroy | reset session, redirect to `/login` |
| GET/POST/DELETE | `/recent_files` | recent_files | HTML / 204 / 204 (§6) |
| GET | `/notes/tree` | notes#tree | HTML (§2.10) |
| GET | `/notes/search` | notes#search | HTML or JSON (§3) |
| POST | `/notes` | notes#create | Hugo creation (§4) |
| POST | `/notes/*path/rename` | notes#rename | `new_path` |
| GET | `/notes/*path/backlinks` | notes#backlinks | JSON (§5) |
| GET | `/notes/*path` | notes#show | JSON note / SPA HTML / raw asset |
| POST | `/notes/*path` | notes#create | `content` (optional) |
| PATCH | `/notes/*path` | notes#update | `content` |
| DELETE | `/notes/*path` | notes#destroy | `expected_revision` (optional) |
| POST | `/folders/*path/rename` | folders#rename | `new_path` |
| POST | `/folders/*path` | folders#create | |
| DELETE | `/folders/*path` | folders#destroy | |
| GET | `/images/config` | images#status | §7.1 |
| GET | `/images` | images#index | |
| GET | `/images/preview/*path` | images#preview | |
| DELETE | `/images/file/*path` | images#destroy | |
| POST | `/images/upload`, `/images/upload_to_s3`, `/images/upload_external_to_s3`, `/images/upload_base64`, `/images/import` | images | §7 |
| GET | `/images/search_web`, `/images/search_google`, `/images/search_pinterest` | images | §7.13 |
| POST | `/media/upload` | media#upload | §7.11 |
| GET | `/library/picker`, `/library/usage`, `/library` | library | §7.8-7.12 |
| PATCH/DELETE | `/library/file/*path` | library | |
| GET | `/youtube/config`, `/youtube/search` | youtube | §10 |
| GET | `/ai/config`, `/ai/image_config`; POST `/ai/fix_grammar`, `/ai/generate_image` | ai | §9 |
| GET | `/config/editor`, `/config/omarchy_theme`, `/config` | config | |
| PATCH | `/config` | config#update | UI keys only |
| GET | `/translations` | translations#show | §12 |
| GET | `/logs/tail`, `/logs/config` | logs | §13.4 |
| GET | `/up` | health#show | `OK` |

Notes routes use `format: false`, so the whole path is the param. **[QUIRK-DROP] (verified)** The folder routes do
not, so `POST /folders/my.notes` is parsed as `path="my"`, `format="notes"`, and creates `my`. Folder names
containing a dot are broken for create, delete, and rename over HTTP.

`PATCH /config`:

- Takes top-level params (form or JSON; `wrap_parameters false`), permitting only `UI_KEYS`. None present → 422
  `"No valid settings provided"`.
- `Config#update` is all-or-nothing → `{settings: ui_settings, message: "Settings saved"}`, or 422 `"Failed to
  save"`.
- `GET /config` → `{settings, media_storage_fallback, features: {s3_upload, youtube_search, google_search,
  local_images}}`. `ai` is not included in `features`.

### 13.2 Authentication (optional access token)

- Enabled when `Config.get("auth_token")` is present, from `.fed` `auth_token` or `FRANKMD_AUTH_TOKEN`.
  Unset means **no auth** (the default).
- **A token shorter than 32 bytes counts as misconfigured**: every request except `/login` and `/up` gets 503 (JSON
  `{"error":"service unavailable"}`, or plain text `Authentication is temporarily unavailable`).
- Accepted credentials:
  - The session cookie value `frankmd_auth_token_digest == sha256hex(token)`. Rotating the token invalidates
    sessions.
  - `Authorization: Bearer <token>`.
  - `X-Auth-Token: <token>`.
  - Comparison is constant-time on the SHA-256 digests.
- Header-authenticated requests skip CSRF.
- Unauthenticated requests: JSON → 401 `{"error":"unauthorized"}`. GET HTML → store `return_to` and redirect to
  `/login`. Anything else → 401 with an empty body.
- Login: `POST /login password=<token>`. Throttling: 10 failures per 5 minutes, counted per IP *and* per session (in
  the Rails cache, which is in-memory by default). Throttled → 429 "Too many login attempts. Please try again
  later.". Wrong token → 401 "Invalid access token". Success → reset the session, store the digest, and redirect to
  the safe `return_to` (it must start with `/` and not `//`) or `/`.
- **[DECIDE]** For a single-user desktop app this is probably irrelevant unless a "serve over LAN" mode is kept. The
  `auth_token` key must still be parsed and preserved.

### 13.3 Web-only concerns (likely to drop in a native app)

- CSRF with Rails authenticity tokens (`csrf_meta_tags`). Invalid tokens get 422.
- `allow_browser versions: :modern` → 406 for old browsers.
- CSP (content_security_policy.rb): `default-src 'self'`; `script-src 'self'` + nonce; `style-src 'self'
  'unsafe-inline'`; `img-src * data: blob:`; `media-src * data: blob:`; `font-src 'self' data:`;
  `connect-src 'self'`; `frame-src` (YouTube/Vimeo list); `frame-ancestors 'none'`; `object-src 'none'`;
  `base-uri 'self'`; `form-action 'self'`. **[COMPAT]** The *preview* relies on `img-src *`, so remote images in
  notes must load in the webview.
- `Referrer-Policy: no-referrer` in production.
- Host authorization via `FRANKMD_ALLOWED_HOSTS` (comma-separated). Empty means any host and no origin check.
- Filtered log params: `passw email secret token auth_token content prompt data base64 _key crypt salt certificate
  otp ssn cvv cvc`, so note contents never appear in logs.
- Sessions use the encrypted cookie store keyed by `SECRET_KEY_BASE` (generated randomly by the docker entrypoint if
  unset, so sessions do not survive restarts).

### 13.4 Logs (`app/controllers/logs_controller.rb`)

- `GET /logs/tail?lines=N`: N defaults to 100, and values ≤ 0 or > 500 become 100. It reads
  `<app>/log/<RAILS_ENV>.log`, backwards in 8 KiB chunks, and returns `{environment, file: "production.log", lines:
  [...]}`.
  - **[QUIRK-DROP]** In production, logs go to STDOUT, so the file usually does not exist. The response is then
    `{"error":"Log file not found","lines":[]}` (200).
  - When the file ends with a newline, the last element is `""`.
- `GET /logs/config` → `{config_file: <abs path>, config_file_exists, ai_configured_in_file, environment, entries:
  [{key, value, source: "file"|"env"|"default", env_var, sensitive}]}` for every SCHEMA key, in schema order.
  Sensitive keys with a value are shown as `"****"`. `source` is `file` if the key is in `@values` (even with an
  empty value), `env` if the ENV value is present, else `default`. **[COMPAT]** The log viewer's Config tab uses it.

### 13.5 Health

`GET /up` → 200 `text/plain` `OK` with `Access-Control-Allow-Origin: *` (needed so `splash.html` from `file://` can
poll it). There is no auth, CSRF, or browser check, and it is silenced in logs.

---

## 14. The `fed` launcher (`config/fed/fed.sh`, `config/fed/fed.fish`, `install.sh`, `splash.html`)

`install.sh` copies (from the local repo checkout, or by curling GitHub `master`) `fed.sh`, `fed.fish`,
`splash.html`, and `env.example` into `~/.config/frankmd/`. It then prints instructions to `source` the shell file.
It never modifies rc files.

`fed [path]` (bash/zsh and fish behave the same):

1. `notes = realpath(${1:-.})`. `splash = $HOME/.config/frankmd/splash.html`.
2. If a container whose name matches `frankmd` is running and its `/rails/notes` mount source differs from `notes`,
   run `docker stop frankmd`. (The `-f name=frankmd` filter is a **substring** match.)
3. If none is running:
   - `docker rm frankmd`.
   - Forward these host env vars **if set** (`printenv`): `AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY AWS_S3_BUCKET
     AWS_REGION YOUTUBE_API_KEY GOOGLE_API_KEY GOOGLE_CSE_ID AI_PROVIDER AI_MODEL OLLAMA_API_BASE OLLAMA_MODEL
     OPENROUTER_API_KEY OPENROUTER_MODEL ANTHROPIC_API_KEY ANTHROPIC_MODEL GEMINI_API_KEY GEMINI_MODEL OPENAI_API_KEY
     OPENAI_MODEL IMAGE_GENERATION_MODEL FRANKMD_LOCALE`. **Not** forwarded: `AWS_S3_ENDPOINT,
     AWS_S3_FORCE_PATH_STYLE, HUGO_PATH_STYLE, FRANKMD_AUTH_TOKEN, IMAGE/VIDEO_UPLOAD_EXTENSIONS` (these need
     `FRANKMD_ENV`).
   - Images dir detection: `$IMAGES_PATH`, then the first `^images_path\s*=` line of `<notes>/.fed` (value after `=`,
     surrounding quotes stripped, leading `~` → `$HOME`), then `$XDG_PICTURES_DIR`, then `~/Pictures` if it exists.
     If the result is a directory, it is mounted **read-only** at `/data/images` with `-e IMAGES_PATH=/data/images`.
     Otherwise the launcher warns `[fed] Warning: no images directory found (set IMAGES_PATH or create ~/Pictures)`.
     **[QUIRK-DROP]** If `.fed` has `images_path`, then `.fed` beats ENV inside the container, so the app uses the
     *host* path, which does not exist in the container.
   - `docker run -d --name frankmd --rm -p 7591:80 --user $(id -u):$(id -g) -v "<notes>:/rails/notes" [images mount]
     [env flags] [--env-file "$FRANKMD_ENV" if set] akitaonrails/frankmd:latest`.
4. Browser detection (`_fed_find_browser`):
   - `FRANKMD_BROWSER`, if it is a PATH command or an executable path; else warn and continue.
   - Then the first of `chromium chromium-browser firefox firefox-esr brave-browser brave google-chrome-stable
     google-chrome microsoft-edge-stable microsoft-edge` found on PATH.
   - Then, on macOS, `/Applications/Chromium.app/Contents/MacOS/Chromium`,
     `/Applications/Firefox.app/Contents/MacOS/firefox`, `/Applications/Brave Browser.app/Contents/MacOS/Brave
     Browser`, `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`, `/Applications/Microsoft
     Edge.app/Contents/MacOS/Microsoft Edge`.
   - With none found, print the 4-line error and return 1.
5. Launch: if the binary basename matches `firefox*`, run `<browser> --new-window file://<splash>`. Otherwise run
   `<browser> --app=file://<splash>`. The browser is backgrounded and disowned.
6. `splash.html` shows a purple-to-pink gradient (`#2D1B4E` → `#FF6B9D`), a logo loaded from GitHub raw, and a
   spinner with "Starting…". It polls `fetch('http://localhost:7591/up')` every 200 ms and then calls
   `location.replace('http://localhost:7591/')`.

- `fed-update`: compare `docker images --digests` before and after `docker pull akitaonrails/frankmd:latest`. If the
  digest changed, print "Updated! Restart FrankMD to use new version." and stop the container. Otherwise print
  "Already up to date.".
- `fed-stop`: `docker stop frankmd`, then print "Stopped." or "Not running.".

`env.example` (copied to `~/.config/frankmd/env.example`) documents a `FRANKMD_ENV` env-file in `KEY=value` form,
for example `FRANKMD_LOCALE`, `IMAGES_PATH`, AWS, YouTube, Google, AI keys and models. **[DECIDE]** The native app
should probably accept `FRANKMD_ENV` as a dotenv file so existing setups keep working. Docker `--env-file` semantics
apply: no quotes processing, no `export`, and `#` comments.

Docker image facts (Dockerfile, bin/docker-entrypoint):

- Base `ruby:3.4.1-slim` plus `imagemagick`, `curl`, `libjemalloc2`.
- User `rails`, UID/GID from build args (default 1000).
- `NOTES_PATH=/rails/notes`, `IMAGES_PATH=/rails/images`.
- `EXPOSE 80`. Thruster (port 80) proxies to Puma on `THRUSTER_TARGET_PORT` (= `EXTERNAL_PORT`, or 3000; 80/443 are
  coerced to 3000).
- The entrypoint fails fast with help text when NOTES_PATH is missing or not writable, and generates
  `SECRET_KEY_BASE`.
- docker-compose publishes `${FRANKMD_BIND_ADDRESS:-0.0.0.0}:${PORT:-7591}:80`, mounts the notes and images dirs,
  has a healthcheck that curls `/up`, and includes an optional cloudflared sidecar.

---

## 15. Every ENV var read anywhere

R = read by app code at runtime; B = boot or infra only; S = launcher or shell scripts; T = tests only.

| Var | Where | Purpose / default |
|---|---|---|
| `NOTES_PATH` | R: config.rb:286, notes_service.rb:16, backlinks_service.rb:5, hugo_service.rb:128, upload_storage.rb:149, images_service.rb:463, notes_controller.rb:165,187, application_controller.rb:95; B: Dockerfile, entrypoint, compose | Notes root. Default `<app>/notes`. |
| `IMAGES_PATH` | R: Config `images_path`; B: initializers/images.rb (fallback set), Dockerfile, compose; S: fed.sh/fish | Server image folder. |
| `XDG_PICTURES_DIR` | B: initializers/images.rb:7; S: fed | Fallback for IMAGES_PATH. |
| `HOME` (via `~`) | R: initializers/images.rb (`~/Pictures`), omarchy_theme_service.rb:4, `Pathname#expand_path` of images_path; S: fed | |
| `PATH` | R: images_service.rb:499 (`magick` detection) | |
| `FRANKMD_LOCALE` | R: Config `locale` | Default `en`. |
| `IMAGE_UPLOAD_EXTENSIONS` / `VIDEO_UPLOAD_EXTENSIONS` | R: Config | |
| `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_S3_BUCKET`, `AWS_REGION`, `AWS_S3_ENDPOINT`, `AWS_S3_FORCE_PATH_STYLE` | R: Config | (aws-sdk may also read the standard `AWS_*` SDK vars itself.) |
| `YOUTUBE_API_KEY` | R: Config | |
| `GOOGLE_API_KEY`, `GOOGLE_CSE_ID` | R: Config | |
| `AI_PROVIDER`, `AI_MODEL` | R: Config (via get_ai) | |
| `OLLAMA_API_BASE`, `OLLAMA_MODEL`, `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`, `ANTHROPIC_API_KEY`, `ANTHROPIC_MODEL`, `GEMINI_API_KEY`, `GEMINI_MODEL`, `OPENAI_API_KEY`, `OPENAI_MODEL` | R: Config (via get_ai) | `OPENROUTER_API_KEY` is also read **directly** at ai_service.rb:92. |
| `IMAGE_GENERATION_MODEL` | R: Config | |
| `HUGO_PATH_STYLE` | R: Config | `dated`/`flat` |
| `FRANKMD_AUTH_TOKEN` | R: Config `auth_token`; B: compose | ≥ 32 bytes required. |
| `FRANKMD_ALLOWED_HOSTS` | B: environments/production.rb:56 | Comma-separated host allowlist. |
| `RAILS_ENV` | B: Dockerfile (`production`); R: logs file name | |
| `RAILS_LOG_LEVEL` | B: production.rb:38 | Default `info`. |
| `RAILS_MAX_THREADS` | B: puma.rb:28 | Default 3. |
| `PORT` | B: puma.rb:34 (default 3000), bin/dev, compose (host port, default 7591) | |
| `PIDFILE` | B: puma.rb:41 | |
| `SECRET_KEY_BASE` | B: entrypoint (generated if empty), compose, `.env.example` | Session encryption. |
| `EXTERNAL_PORT` | B: entrypoint, compose | Printed URL and Thruster target. |
| `THRUSTER_TARGET_PORT` | B: entrypoint | |
| `BUNDLE_GEMFILE`, `BUNDLE_*`, `LD_PRELOAD` | B: boot.rb, Dockerfile | Ruby only. |
| `WEB_CONCURRENCY` | (comment only in puma.rb) | |
| `FRANKMD_BIND_ADDRESS`, `UID`, `GID`, `CLOUDFLARE_TUNNEL_TOKEN` | B: compose / `.env.example` | |
| `FRANKMD_BROWSER` | S: fed.sh:12, fed.fish:12 | Browser override. |
| `FRANKMD_ENV` | S: fed.sh:130, fed.fish:133 | Docker `--env-file`. |
| `IMAGE`, `SSH_HOST`, `REMOTE_DIR` | S: bin/deploy (maintainer deploy) | |
| `RUBY_DEBUG_OPEN`, `RUBY_DEBUG_LAZY` | S: bin/dev | |
| `CI` | T: environments/test.rb | |
| `CHROME_NO_SANDBOX` | T: application_system_test_case.rb | |

There is no `ENV` read in `app/javascript` that the backend serves (the frontend has no build-time env).
Legacy file: `NOTES/.webnotes` (the pre-rename config name, commit `0209e02`) is **not read** by current code. Ignore
it, but leave it untouched.

---

## 16. Gems (Gemfile / Gemfile.lock) and what each is for

| Gem (locked) | Use in FrankMD | Rust replacement idea |
|---|---|---|
| rails 8.1.4 (actionpack, actionview, activemodel, activesupport, railties) | HTTP routing, controllers, ERB views, ActiveModel validations on Note/Folder, i18n, JSON encoding, `Time.current` | axum/tauri commands, serde, fluent/YAML i18n |
| puma 8.0.2 | App server | n/a (in-process) |
| thruster 0.1.27 | Reverse proxy, compression, X-Sendfile in Docker | n/a |
| propshaft 1.3.2 | Asset pipeline | frontend bundler |
| importmap-rails | JS import maps | frontend |
| turbo-rails / stimulus-rails / requestjs-rails | Hotwire frontend and CSRF-aware fetch | frontend |
| tailwindcss-rails | Tailwind build | frontend |
| jbuilder | Present but unused (no `.jbuilder` views) | — |
| aws-sdk-s3 1.235.0 | S3 put/list/copy/delete/head, custom endpoints | `aws-sdk-s3` or `rusty-s3`/`s3` crates |
| ruby_llm 2.1.1 | LLM chat (grammar) and OpenRouter image generation | Direct `reqwest` calls per provider |
| bootsnap | Boot cache | — |
| kamal | Deploy tooling (config/deploy.yml) | — |
| tzinfo-data | Windows/JRuby tz data | — |
| debug, web-console, brakeman, bundler-audit, rubocop-rails-omakase | Dev tooling | — |
| capybara, selenium-webdriver, mocha, webmock, simplecov | Tests | — |

External binaries: **ImageMagick** (`magick` or `convert`/`identify`), plus `curl` in the image (healthcheck only).

---

## 17. Quirk register (summary)

Must replicate **[COMPAT]**:

1. `.fed` template bytes (§1.7). Mode 0600 after any write. Surgical write rules: uncomment the first commented
   occurrence, replace every active occurrence, append unknown new keys at EOF, `key = value` formatting, quoting
   when a value has a space or `=`, `key = ` for nil (§1.8).
2. Parsing: last duplicate wins, no inline comments, quote stripping, case-insensitive keys, `to_i` integers,
   boolean truthy set (§1.5).
3. An empty value in `.fed` blocks ENV fallback, and an empty AI credential line triggers AI file-only mode
   (§1.3).
4. The AI section is auto-appended on load when absent (§1.6). Any request creates `.fed` and the lock files.
5. Lock files `.frankmd-config.lock` and `.frankmd-filesystem.lock` with flock semantics (§2.7).
6. Tree: hidden dotfiles; root `images/` and `videos/` hidden unless they contain .md; folders first (case-insensitive
   name), then files by mtime descending; `.md` only, case-sensitive (§2.2).
7. `.md` appended to note paths; `.fed` special-cased (§2.3).
8. Revision = sha256 hex of the bytes. Delete-only conflict check (§2.5).
9. Upload naming `YYYYMMDD_HHMMSS_<8hex>_<sanitized>` under `images/` and `videos/`, relative URLs (§7.2, §7.4).
10. S3 key scheme `frankmd/YYYY/MM/<name>-<16hex>.<ext>` and the URL formats (§7.6).
11. Recent-files JSON in `.fed`, `opened_at` in ISO-8601 UTC with microseconds, limit 10 (§6).
12. Hugo: slug algorithm, the default template including its trailing blank line, UTC date format with `%:z`
    (§4) [DECIDE UTC].
13. The search uses Ruby regex semantics, case-insensitive, falling back to a literal (§3).
14. AI provider priority order and the default models (§1.2-1.3).

Dropping is allowed **[QUIRK-DROP]** (replicate only if trivial):

1. `.fed` in-app editing is blocked (403) even though the UI and README expect it (§1.9).
2. `..` substring rejection in names (§2.3).
3. Folder names with dots break the folder routes (§13.1).
4. `/ai/fix_grammar` can send `.fed` to an LLM (§9.1).
5. `/images/upload_to_s3` and `/upload_external_to_s3` require `images_path` (§7.1).
6. A failed resize writes original bytes under a `.jpg` name; the arbitrary 50% default ratio (§7.10).
7. Local Library rename is unimplemented (§7.8). An S3 library over 5000 objects returns 500 (§7.12).
8. Search returns the first 20 matches found, not the newest (§3). Backlinks has no symlink skip, no size cap, and
   no UTF-8 scrub (§5).
9. Empty Hugo slug produces a `//` in the response path (§4.1).
10. Google `next_start` off by one (§7.13). Log tail trailing `""` and missing file in production (§13.4).
11. Thread-local locale leak (§12). The virtual-host S3 URL drops the endpoint path (§7.6).
12. `serve_asset` serves hidden files such as `.git/*` (§2.9).
13. The `.svg` listing vs preview allow-list mismatch (§7.1).
14. README stale facts: the provider order, the `whats-new` slug example, the `-0300` date, the gear-icon `.fed`
    entry.

---

## 18. Backend feature checklist

| ID | Feature |
|---|---|
| BE-001 | Resolve NOTES root from CLI arg / `NOTES_PATH`, defaulting to `./notes`; create it if missing |
| BE-002 | Config SCHEMA with the 45 keys, types, defaults, and ENV names (§1.2) |
| BE-003 | Config resolution: `.fed` (including empty-as-nil) > non-blank ENV > default |
| BE-004 | AI file-only mode: any AI credential key in `.fed` → ignore all AI ENV via `get_ai` |
| BE-005 | Image-generation OpenRouter key exception (`.fed` value or ENV, bypassing file-only mode) |
| BE-006 | `.fed` parser (comments, quotes, `=` in values, last duplicate wins, unknown keys ignored, cast errors skip the line) |
| BE-007 | Value casting (`to_i` integers, boolean truthy set, json, custom_theme) |
| BE-008 | Byte-exact `.fed` template generation on first access, mode 0600 |
| BE-009 | AI/LLM section auto-upgrade append on load |
| BE-010 | Surgical `.fed` writer (uncomment, replace, append, formatting, quoting, string validation) |
| BE-011 | Atomic `.fed` write (O_EXCL temp file + fsync + rename) under the `.frankmd-config.lock` flock |
| BE-012 | Locked read-modify-write (`modify`) for shared keys |
| BE-013 | `.fed` change pickup (re-read per operation or file watch) |
| BE-014 | `upload_extensions` parsing for the image/video allow-lists |
| BE-015 | Feature flags: s3_upload, youtube_search, google_search, local_images, ai |
| BE-016 | `IMAGES_PATH` boot fallback (`XDG_PICTURES_DIR`, then `~/Pictures`) and `~` expansion |
| BE-017 | `GET /config`-equivalent: UI settings, features, media_storage_fallback |
| BE-018 | `PATCH /config`-equivalent: UI_KEYS only, all-or-nothing update |
| BE-019 | `media_storage_provider` validation (local/s3; s3 only if configured) |
| BE-020 | PathSafety containment (lexical, separator-anchored, realpath of the nearest existing ancestor) |
| BE-021 | Reserved paths: `.fed` blocked for note ops, lock files reserved or hidden |
| BE-022 | File tree build (hidden/symlink skip, root images/videos rule, `.md` only, sort order, node shape) |
| BE-023 | Shared/exclusive `.frankmd-filesystem.lock` flock, with the app-local fallback lock |
| BE-024 | Note path normalization (append `.md`, `.fed` exception) |
| BE-025 | Read note (raw bytes, revision = sha256) |
| BE-026 | Create note (no-replace via hard link, rename fallback, mkdir -p parents) |
| BE-027 | Update note (must exist, no parent creation, atomic, preserves mode bits) |
| BE-028 | Delete note with optional expected_revision conflict check (409) |
| BE-029 | Rename/move note or folder (no overwrite, mkdir -p destination parent, EXDEV fallback) |
| BE-030 | Create folder |
| BE-031 | Delete folder (empty only, hidden files included) |
| BE-032 | Folder rename updates Hugo `index.md` slug |
| BE-033 | Recent-files remove/remap on delete/rename/move (prefix-aware) |
| BE-034 | Serve raw assets under NOTES (MIME by extension, inline, Range support) |
| BE-035 | Initial-note loading for deep links (`/notes/<path>`, `?file=`) |
| BE-036 | Error-message localization for note and folder operations (§2.8) |
| BE-037 | Full-text search (Ruby-compatible regex, case-insensitive, literal fallback, 0.5 s timeout) |
| BE-038 | Search limits: 5 MiB file cap, 20 results, ±3 context lines, UTF-8 scrub, sort by mtime descending |
| BE-039 | File finder data: flattened tree with mtime |
| BE-040 | Hugo slugify (accent map, ASCII-only, hyphen collapse) |
| BE-041 | Hugo path styles: dated `YYYY/MM/DD/slug/index.md` and flat `slug.md`, under parent |
| BE-042 | Hugo front matter: default template, `.hugo_template.md` override, placeholder substitution, title quote escaping |
| BE-043 | Hugo date `%Y-%m-%dT%H:%M:%S%:z` (UTC, pending decision) |
| BE-044 | Backlinks: wikilink by name/path and markdown links, with context |
| BE-045 | Recent files: storage in `.fed`, normalize, limit 10, ISO-8601 µs UTC |
| BE-046 | Recent files: record / clear / list (filter missing files on read) |
| BE-047 | Recent files: relative-time labels (Just now, s/m/h/d, Over a week ago, Unknown time) |
| BE-048 | Server image folder listing (recursive, search, paging, mtime sort, dimensions) |
| BE-049 | Server image preview (allow-list = upload extensions) |
| BE-050 | Server image delete |
| BE-051 | Upload provider resolution and legacy `upload_to_s3` conflict check |
| BE-052 | Upload validation: 512 MiB cap, extension allow-list messages, filename sanitizer |
| BE-053 | Local image upload to `NOTES/images/<ts>_<hex8>_<name>`, relative URL |
| BE-054 | Optional resize (25/33/50/67/75/100 %, JPEG q95, metadata stripped) |
| BE-055 | Base64 image upload (mime → extension, default name `ai_generated_<hex16>`) |
| BE-056 | Image import from URL or server path to the active provider |
| BE-057 | S3 client (credentials, region default, custom endpoint, path style) |
| BE-058 | S3 conditional put (`If-None-Match: *`), content types, no ACL |
| BE-059 | S3 key scheme (custom key / custom prefix / default `frankmd/YYYY/MM`, `-<hex16>` suffix) |
| BE-060 | S3 public URL formats (AWS, path-style, virtual-host) |
| BE-061 | Upload server image to S3 (legacy endpoint) |
| BE-062 | Download external image and upload to S3 (legacy endpoint) |
| BE-063 | Video upload local (`NOTES/videos/…`) / S3 with video MIME map |
| BE-064 | Media storage recovery (auto-switch to local, fallback flag, invalid-config error codes) |
| BE-065 | Local Media Library listing (images/videos, symlink-safe, sort order) |
| BE-066 | Local Media Library delete (strict path validation) |
| BE-067 | S3 Media Library listing (`frankmd/` prefix, pagination, 5000 cap) |
| BE-068 | S3 Media Library delete and rename (copy + delete, name validation) |
| BE-069 | Image picker library: managed / server collections with paging and search |
| BE-070 | Media usage index (Markdown and HTML reference extraction, code masking, path canonicalization, cache) |
| BE-071 | Egress policy (scheme, DNS resolve-all, blocked ranges, IP pinning) |
| BE-072 | External fetch limits: 5 redirects, 25 MiB, 30 s deadline, timeouts, User-Agent |
| BE-073 | DuckDuckGo image search (vqd scrape, i.js, 20 results) |
| BE-074 | Pinterest search via DuckDuckGo `site:pinterest.com` |
| BE-075 | Google Custom Search image search |
| BE-076 | Image search status endpoint (`/images/config` payload) |
| BE-077 | AI provider detection, priority, and model selection |
| BE-078 | AI grammar fix (verbatim prompt, reads note from disk, provider endpoints, 300 s timeout, retries) |
| BE-079 | Ollama base URL validation (http/https) |
| BE-080 | AI status endpoint payload |
| BE-081 | AI image generation via OpenRouter (modalities, optional reference image, data-URI/URL parsing) |
| BE-082 | AI image config endpoint |
| BE-083 | YouTube search (Data API v3, 6 results, payload) and status |
| BE-084 | Omarchy theme detection and parsing of alacritty.toml |
| BE-085 | Omarchy color mapping, contrast enforcement, is_dark |
| BE-086 | Custom theme validation and storage in `.fed` |
| BE-087 | i18n: 7 locales, en fallback, per-request locale (param > config > en) |
| BE-088 | Translations payload with en deep-merge |
| BE-089 | Logs tail endpoint |
| BE-090 | Config inspector (`/logs/config`: values, source, masked secrets) |
| BE-091 | Health endpoint (`/up`, CORS *) |
| BE-092 | Optional access token auth (≥ 32 bytes, Bearer / X-Auth-Token / session, throttled login) [DECIDE] |
| BE-093 | Host allowlist / CSP / referrer policy [web-only, DECIDE] |
| BE-094 | `fed [path]` launcher CLI compatibility (path arg, single-instance switch to a new notes dir) |
| BE-095 | `FRANKMD_ENV` env-file support |
| BE-096 | Browser/window launch semantics (`FRANKMD_BROWSER`, detection order) [likely replaced by the native window] |
| BE-097 | `fed-update` / `fed-stop` equivalents |
| BE-098 | Splash/loading screen while the backend starts |
| BE-099 | Preserve unknown files untouched (`.webnotes`, `.hugo_template.md`, foreign files) |
| BE-100 | Sensitive-data hygiene: never log note contents, prompts, or keys (filter list §13.3) |
