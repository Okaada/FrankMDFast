# Test port: config + themes (`frankmd-config`, `frankmd-theme`)

Status: tests written, crate bodies are still `todo!()`. All of these tests are the acceptance
criteria for the implementation phase. Today `cargo test -p frankmd-config -p frankmd-theme --no-run`
and `cargo clippy ... --tests` are clean apart from `todo` warnings. At runtime every failure is a
`not yet implemented` panic. The only tests that pass now are the constant checks.

## Original tests → Rust tests

| Original | Rust | Orig. | Ported | Skipped (reason) |
|---|---|---|---|---|
| `test/models/config_test.rb` | `crates/frankmd-config/tests/config_test.rs` | 67 | 67 | 0 |
| `test/controllers/config_controller_test.rb` | `crates/frankmd-config/tests/config_controller_test.rs` | 16 | 15 | 1: `editor returns HTML partial…` [P], Rails partial markup |
| `test/controllers/custom_theme_config_test.rb` | `crates/frankmd-config/tests/custom_theme_config_test.rs` | 3 | 3 | 0. The root-page `assert_select` lines of test 1 are dropped (view plumbing) |
| `test/controllers/ai_controller_test.rb` (config resolution) | `crates/frankmd-config/tests/ai_controller_test.rs` | 17 | 11 | 6: `fix_grammar` ×4 and `generate_image` ×2 belong to frankmd-ai (request validation). Their config inputs (`effective_ai_provider() == None`, `image_generation_key() == None`) are covered |
| `test/controllers/logs_controller_test.rb` (Config introspection) | `crates/frankmd-config/tests/logs_controller_test.rs` | 12 | 5 | 7: `tail …` [P], Rails log file |
| `test/services/custom_theme_test.rb` | `crates/frankmd-theme/tests/custom_theme_test.rs` | 3 | 3 | 0 |
| `test/services/omarchy_theme_service_test.rb` | `crates/frankmd-theme/tests/omarchy_theme_service_test.rs` | 2 | 2 | 0. Runs against the faithful `legacy` port |
| `test/javascript/lib/custom_theme.test.js` | `crates/frankmd-theme/tests/custom_theme_js_test.rs` | 4 | 4 | 0. DOM apply/clear becomes the pure override filter plus `UiTokens::with_overrides` |
| `test/javascript/lib/theme_syntax_palette.test.js` | `crates/frankmd-theme/tests/theme_syntax_palette_test.rs` | 20 | 19 (1 + 18 generated per theme) | 1: the `prose.css` var-fallback check → frankmd-markdown/app (preview CSS) |
| `test/javascript/lib/codemirror_theme.test.js` [C] | `crates/frankmd-theme/tests/codemirror_theme_test.rs` | 8 | 3 adapted (defaults, 16em autocomplete, highlight rules) | 5: CodeMirror `Extension` shape only |
| new, not ports | `crates/frankmd-theme/tests/omarchy_45_test.rs` | — | 13 tests marked `// new: omarchy 4.5` | — |

Each ported test has `// port: <file>:<line> "<name>"` above it, and each skip has `// skip: …` with the
reason.

## Golden fixtures wired (`assert_cases`, all mismatches reported at once)

| Fixture | Rust test(s) | Cases |
|---|---|---|
| `config_parse.json` | `golden_config.rs::golden_config_parse_values` (pure `ResolvedConfig::from_bytes`), `::golden_config_parse_load_rewrites_file` (`ConfigStore::open`; `fed_after_load` bytes + resolved) | 145 × 2 |
| `config_env.json` | `golden_config_env_precedence` (file_values, resolved, sources, get_ai, ai flags/providers/model, features, all_settings_masked, upload_extensions) | 286 |
| | `golden_config_env_notes_root` (`dirs::notes_root`, `config_file_path`) | 3 |
| | `golden_config_env_images_initializer` (`EnvSnapshot::with_images_path_default`) | 6 |
| | `golden_config_env_openrouter_key_for_images` (`image_generation_key`, `get_ai`) | 4 |
| | `golden_config_env_schema_table` (`keys` table, 45 rows: type/default/env/sensitive/ui/ai_credential, schema order) + `config_constants` | 45 rows |
| | skipped: `production.rb allowed_hosts` (`FRANKMD_ALLOWED_HOSTS`, web host allowlist) | 7 |
| `config_writeback.json` | `golden_config_writeback` (op runner: set/update/reload/external_write/modify_*/ui_settings/format_value; checks result, disk `.fed` after every step, snapshot, `reread_file_values`, `tree_after`, 0600 modes) | 109 (`generate_config_lines`, dead code, is a no-op) |
| `config_template.json` | `golden_config_template_creation` (tree_after, modes, file path, read-only root, symlinked lock, `.fed` dir/symlink) | 12 (1 skipped: `NOTES_PATH=""` relative to the cwd) |
| | `golden_config_template_upgrade_pure` (`fed::needs_ai_section`, `fed::upgrade_ai_section`), `…_upgrade_on_open` (disk + mode 0644→0600) | 26 × 2 |
| | `golden_template_sections` (`fed::template_sections`, `ai_section_lines`) | — |
| `fed_template.fed` | `golden_fed_template_is_byte_identical` (byte for byte, 3032 bytes) | — |
| `custom_theme.json` | `golden_theme.rs::golden_custom_theme_parse` (+ colour key order from the raw input), `golden_custom_theme_constants` | 102 |
| `js_custom_theme.json` | `golden_js_{color_keys,normalize_color,contrast_ratio,theme_color,apply_custom_colors,theme_palette_matches_builtin}` | 84 (all) |
| `js_codemirror_theme.json` | `golden_codemirror_syntax_highlighting` (26 rules), `golden_codemirror_editor_theme` (chrome token map, font defaults, 16em) | 6 of 7 (`createTheme shape` skipped) |
| `omarchy_theme.json` | `golden_omarchy_color_math` (217), `golden_omarchy_parse_and_map` (parse_alacritty, map_variables incl. order, constants), `golden_omarchy_theme_data` (64 real filesystem cases: symlinks, EISDIR, EACCES, invalid UTF-8) | 284 (all) |
| `assets/themes/builtin.json` | `golden_theme.rs::builtin_json_has_18_themes_with_all_38_roles` (every theme has all 38 roles, `builtin()` matches the JSON, mode from luminance) | 18 themes |

## API changes made (bodies still `todo!()`)

**frankmd-config**
- `schema`: `Key::ALL` (schema order, which is observable), `Key::name`, exact `Key::from_name` (the
  `.fed` parser lowercases first, so the `update("THEME"…)` skip is preserved), `Key::ui_keys()`,
  `Key::AI_UPGRADE_MARKERS`, `KeyType::as_str`. `Key` derives `Ord`.
- `value`: `Value::CustomTheme(frankmd_theme::CustomTheme)`, `to_json/as_str/as_int/as_bool`, and
  `coerce(key, &serde_json::Value)`, which is Ruby `cast_value` for values passed to `set`/`update`.
  The old free `format_line` moved to `ResolvedConfig::format_line`, because it needs
  `s3_configured()`.
- `fed`: pure text functions `parse → Parsed{values, warnings}`, `raw_entries`, `apply_changes`,
  `needs_ai_section`, `upgrade_ai_section`, `template_sections`, `ai_section_lines`, `template`. The
  `FedDocument` line model was dropped: the upgrade has to keep whitespace-only lines verbatim while
  the write-back normalizes them, so one normalizing model can't serve both.
- `resolve`: `ResolvedConfig::{from_bytes, file_values, value, ai_configured_in_file,
  effective_ai_model, s3_configured, feature_available(Feature), ui_settings, all_settings_masked,
  entries() -> Vec<ConfigEntry>, format_line}`. Also new: `Provenance::as_str`,
  `AiProviderId::{PRIORITY, AVAILABILITY_ORDER, as_str, from_name, credential_key, model_key}`, and the
  `Feature` enum. File values are now an ordered `Vec`.
- `store`: `ConfigStore::open` is **infallible** (Ruby never raises; problems go to `warnings`). Also
  new: `path()`, `update_ui_settings` (UI_KEYS filter → `ConfigError::NoValidSettings`), and
  `CONFIG_FILE`.
- `error`: one variant per original message (`UnsafeString`, `UnknownMediaProvider`,
  `S3NotConfigured`, `CustomTheme`, `InvalidJson`, `InvalidValue`, `NoValidSettings`, `Fs`). Each
  `Display` is the Ruby text.
- `dirs::{notes_root, config_file_path}`, `EnvSnapshot::with_images_path_default` (the boot rule
  from `images.rb`, applied without mutating ENV).
- New dependency: **frankmd-config → frankmd-theme**, for custom-theme validation during the cast.

**frankmd-theme**
- `COLOR_KEYS` (38), `BASE_THEMES` (18), `SYNTAX_ROLES`, `Rgb::to_hex`, `UiTokens::{get,
  with_overrides}`, `builtin_ids()`, `relative_luminance`, `contrast_ratio`, and new `ThemeError::{Io,
  Encoding}` variants.
- `custom`: `CustomTheme::{parse, from_value, to_json_string, to_json}` (colours keep input order),
  `CustomThemeError` (Ruby messages), `normalize_color`, `theme_color`, `contrast_ratio_hex`,
  `applicable_overrides`.
- `legacy`: a faithful port of `OmarchyThemeService`: `~/.config/omarchy/current`, the
  alacritty.toml parse, the float colour math, `ensure_contrast` at 4.51, `theme_data`/`available`.
- `omarchy`: `OmarchyPaths::from_home` (`~/.local/state/omarchy/current`, plus colors.toml,
  light.mode, theme.name and theme.generation), `OmarchyPalette::{parse, get}`, `from_omarchy`,
  `load_current`. A mid-swap state (current exists, `theme/` missing) returns `Err`, not `None`
  (HP-024).
- `editor`: `EditorFont` (defaults Cascadia Code/14px/1.6), `markdown_highlights()` (26 rules as token
  names), `editor_chrome()`, `AUTOCOMPLETE_MAX_HEIGHT_EM`.

## Behaviour decisions and quirks

1. **Ruby `instance_values` staleness is not replicated.** Ruby `set` merges only the changed key
   into `@values`, so an instance can keep keys that were removed externally. Ruby also keeps `nil`
   for `vim_mode = nil` while the disk holds `false`. A `ConfigStore` snapshot is always re-parsed
   from disk after a write (architecture §3.2). The golden runner compares each step against the
   next fixture value that reflects disk (`reload` result, else `reread_file_values`). The bytes
   written to disk match the original exactly.
2. **Unknown keys** (`bogus`, `THEME` in `update`, `get(:unknown_key)`) can't be expressed with a
   typed `Key`. They are asserted as `Key::from_name(..) == None` and treated as no-ops, which is the
   original's observable behaviour: `set` → false, `update` skips them, `modify` → false.
3. **Logs are not a compat surface.** We assert only that every `Ignoring invalid setting <key>` and
   `Failed to load` shows up as a `ResolvedConfig::warnings` entry naming the key.
4. **`set(:editor_font_size, true)` raised `NoMethodError` and `set(:recent_files, "[broken")`
   raised `JSON::ParserError`** in the original, which leaves the file unchanged. Rust returns `Err`
   from `coerce` with the same no-write outcome. The original's "update with only unknown keys returns
   true" is kept (`update(&[])` is `Ok` and writes nothing).
5. **Read-only root:** in the write-back fixture the lock file already existed, so the runner makes
   the root read-only *after* the first open. In the template-creation fixture it is read-only before
   the open, so the lock falls back to the state dir and nothing appears in the vault. Both cases are
   skipped when running as root.
6. **`NOTES_PATH=""`** (resolved relative to the cwd) is skipped. `notes_root` returns `None` for
   empty values, and the app has to ask for a folder.
7. **JS `contrastRatio(" #808080 ", …)` returns `NaN`** (it slices the raw, unnormalized string). This
   is a JS bug. Rust returns the ratio of the normalized colours.
8. **`FRANKMD_ALLOWED_HOSTS`** (production host allowlist, 7 cases) is web-only and not ported.
9. **Bugs in the original, kept for compat:** the template's theme list is stale (`cappuccino`,
   `osaka`). An empty `openrouter_api_key =` falls through to ENV for image generation, while
   `get_ai` returns nil. The Ruby custom theme accepts `"version": 1.0`. NBSP-only names are not
   stripped. In the legacy Omarchy mapping, `scrollbar = normal.black = background`, so the scrollbar
   is invisible, and `accent` falls back to blue. That last one is only in `legacy`; the 4.5 mapping
   fixes it.
10. **Omarchy path:** FrankMD reads `~/.config/omarchy/current`, which no Omarchy 4.x install has. The
    goldens pin the legacy parsing exactly. The new tests cover `~/.local/state/omarchy/current`: the
    cascade (short names, ANSI fallbacks, derived orange/brown/dark/darker/bright_*, purple→magenta,
    selection chain, cursor), mode precedence (`mode` > `theme_type` > `light.mode` > R+G+B>382), mix
    rounding, all 23 stock `colors.toml` files (complete tokens, syntax ≥ 4.5 vs code-bg, text-primary
    ≥ 4.5, accent-text ≥ 4.5 on accent, the §2.6 direct mapping, readable sources left alone), gradient
    values ignored, uppercase hex, and missing / present / mid-swap / malformed.
    **Open question:** should `load_current` also fall back to the legacy dir for very old installs?
    No test requires it.

## Cross-crate requirements

- **frankmd-fs**:
  - `Vault::open(root, state)` must `mkdir -p` the root (nested-path test). It must succeed on a
    read-only root, and also when `.frankmd-config.lock` is a symlink.
  - `lock::acquire` on the config lock must fail with ELOOP on a symlinked lock (no fallback; the
    caller degrades), fall back to the state dir only on EACCES/EROFS, and create the lock file 0600.
  - `atomic_replace` must use temp name `..fed.<12hex>.tmp`, always end at mode 0600, and replace a
    `.fed` symlink with a regular file while leaving the target untouched.
  - `lock::CONFIG_LOCK` is used by the tests.
  - `frankmd_fs::recent::RecentStore` expects `ConfigStore` to implement it with `&self` methods.
    `ConfigStore::{update, modify}` take `&mut self`, so the implementation needs interior
    mutability (or a thin adapter) to satisfy that trait.
- **architecture.md**: add the `config → theme` edge to the dependency graph.
- **frankmd-ai**: port the 6 skipped `ai_controller_test` cases. Read credentials only through
  `get_ai`/`effective_ai_*`, and the image key through `ResolvedConfig::image_generation_key` (it
  bypasses file-only mode).
- **frankmd-media**: reuse `ResolvedConfig::s3_configured()` (endpoint validation included) instead
  of re-deriving it.
- **frankmd-app**: the settings dialog writes through `update_ui_settings`. It applies
  `builtin(base).with_overrides(custom)` for `theme = custom` and `from_omarchy` for `theme = omarchy`.
  It maps `editor_chrome()`/`markdown_highlights()` onto the editor.
- **frankmd-watch**: watch `~/.local/state/omarchy/current/` (the `theme/` dir is replaced by rename)
  and call `load_current`, keeping the last good palette on `Err`.
- **frankmd-markdown/app**: port the `prose.css` syntax-role check (code-highlight roles must not fall
  back to heading/accent colours).
