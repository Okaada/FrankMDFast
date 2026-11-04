# Test port: editor text ops, data safety, vim, keymap

Scope: `frankmd-core::{text, guard, emoji}`, `frankmd-vim`, `frankmd-workspace`.
Status: every test compiles (`cargo test -p frankmd-core -p frankmd-workspace -p frankmd-vim --no-run`);
`cargo clippy --tests` reports only `todo` warnings. At runtime the tests fail on `todo!()` bodies, as
intended. They are the acceptance criteria for the implementation phase.

**Offset unit.** The Rust text API takes `&str` and **UTF-8 byte offsets**, the same unit as `VimBuffer`.
JS fixtures record UTF-16 code units. The tests convert offsets with their own helpers in
`crates/frankmd-core/tests/text_common/mod.rs` (`u16_to_byte`, `u16_to_byte_floor`, `byte_to_u16`), so a
failing case points at the function under test, not at the conversion. Values that JS shows to the user as
UTF-16 counts keep that unit and say so in the field name: `DocStats::char_count_utf16`,
`CursorInfo::column_utf16`, and the table padding width. None of the assigned fixtures contains a
lone-surrogate (`$utf16`) string.

## Original test files → Rust

| Original | Rust test file(s) | Orig | Ported | Skipped (reason) |
|---|---|---|---|---|
| `test/javascript/text_utils.test.js` | core `tests/text_utils_test.rs` | 23 | 23 | 0 |
| `test/javascript/markdown_utils.test.js` | core `tests/text_markdown_utils_test.rs` | 14 | 14 | 0 |
| `test/javascript/table_utils.test.js` | core `tests/text_table_utils_test.rs` | 21 | 21 (+1 extra: out-of-range swap is a no-op) | 0 |
| `test/javascript/indent_utils.test.js` | core `tests/text_indent_utils_test.rs` | 25 | 25 | 0 |
| `test/javascript/diff_utils.test.js` | core `tests/text_diff_utils_test.rs` | 17 | 17 | 0 |
| `test/javascript/stats_utils.test.js` | core `tests/text_stats_utils_test.rs` | 18 | 18 | 0 |
| `lib/task_utils.test.js` | core `tests/text_task_utils_test.rs` | 11 | 11 | 0 |
| `lib/text_editor_utils.test.js` | core `tests/text_editor_utils_test.rs` | 28 | 28 | 0 |
| `lib/typewriter_utils.test.js` | core `tests/text_typewriter_utils_test.rs` | 17 | 17 | 0 |
| `lib/codemirror_content_insertion.test.js` | core `tests/text_content_insertion_test.rs` | 34 | 34 (+1 byte-offset test) | 0 |
| `lib/codemirror_adapter.test.js` | core `tests/text_adapter_test.rs` | 22 | 22 | 0 |
| `lib/clipboard_image.test.js` | core `tests/text_clipboard_image_test.rs` | 6 | 6 (+1 MIME table pin) | 0 |
| — (new) | core `tests/text_offsets_test.rs` | — | 8 | — |
| `lib/icon_data.test.js` | core `tests/emoji_icon_data_test.rs` | 4 | 4 | 0 |
| emoji_data / emoji_i18n | — | 0 | — | No such test files exist. Their tests live in `emoji_picker_controller.test.js` (UI agent). Covered by golden only. |
| `lib/menu_bounds.test.js` | workspace `tests/menu_bounds_test.rs` | 9 | 8 | 1: "ignores missing menu elements" (a null element can't be passed to the typed API) |
| `lib/keyboard_shortcuts.test.js` | workspace `tests/keyboard_shortcuts_test.rs` | 41 | 41 | 0 (`isMacOS` "returns a boolean" now checks `Platform::current()` against the build target) |
| `lib/vim_mode.test.js` | vim `tests/vim_mode_test.rs` | 11 | 11 (+1: ex commands ⊆ shortcut actions) | 0. The C tests built on mocks are rewritten as engine logic: passthrough keys on non-mac only, the ex table and prefixes, mode labels, `next_note_index`. |
| `lib/codemirror_extensions_vim.test.js` | vim `tests/codemirror_extensions_vim_test.rs` | 4 | 1 (normal mode consumes Enter/Backspace/arrows; insert mode and IME pass through) | 3: CodeMirror extension object identity, disabled `[]`, cursor-theme ordering (CodeMirror-specific) |
| `lib/draft_storage.test.js` | workspace `tests/draft_storage_test.rs` | 22 | 22 | 0 |
| `controllers/offline_backup_controller.test.js` | workspace `tests/draft_storage_test.rs` (`backup_save`/`backup_check`) | 8 | 8 | 0 |
| `controllers/autosave_controller.test.js` | workspace `tests/autosave_test.rs` | 83 | 81 (+5 contract tests: 2000/30000/300 ms, `AutosaveTimer`, stale deletion of a non-active note) | 2: status-label DOM tests (:122, :132), left to the UI agent |
| `controllers/recovery_diff_controller.test.js` | workspace `tests/autosave_test.rs` | 7 | 3 (resolved-event payloads) | 4: DOM rendering, timestamp, `showModal`, Escape (UI agent) |
| `controllers/app_file_rename.test.js` | workspace `tests/file_rename_remap_test.rs` | 16 | 12 | 4: app-controller C tests (UI agent) |
| content-loss guard (autosave contract) | core `tests/guard_test.rs` | — | 5 | — |
| `test/i18n_vim_keys_test.rb` | — | 126 | — | Owned by the i18n agent |

No category-P tests in scope.

## Golden fixtures

| Fixture | Rust test | Cases wired / total | Skipped (reason) |
|---|---|---|---|
| js_text_utils | core `text_golden_utils.rs` | 247/255 | 8 non-string args (`escapeHtml(42)`, `isSafeImageUrl({})`) |
| js_markdown_utils | core `text_golden_utils.rs` | 230/249 | 19 negative positions |
| js_table_utils | core `text_golden_utils.rs` | 85/90 | 5 sparse-array results / negative indices |
| js_indent_utils | core `text_golden_utils.rs` | 125/125 | — |
| js_diff_utils | core `text_golden_diff.rs` | 55/55 | — |
| js_stats_utils | core `text_golden_stats.rs` | 77/80 | 3 `formatFileSize` with fractional/negative input (`u64` API) |
| js_task_utils | core `text_golden_task.rs` | 103/103 | — |
| js_text_editor_utils | core `text_golden_text_editor.rs` | 83/85 | 2 negative offsets |
| js_typewriter_utils | core `text_golden_typewriter.rs` | 42/43 | 1 negative offset. In one case the JS offset falls inside an emoji's surrogate pair; the test snaps it down to the character start. |
| js_emoji_data | core `emoji_golden.rs` | 3/3 | — |
| js_emoji_i18n | core `emoji_golden.rs` | 28/28 | `fetched` URL list ignored (data is embedded); 2 prototype-key expectations overridden (see quirks) |
| js_icon_data | core `emoji_golden.rs` | 4/4 | — |
| js_keyboard_shortcuts | workspace `golden_keyboard_shortcuts.rs` | 749/751 (incl. all 632 `findMatchingAction`) | 2 `getShortcutForAction("constructor"/"toString")`, which come from the JS object prototype |
| js_menu_bounds | workspace `golden_menu_bounds.rs` | 23/23 | — (default viewport 768×1024 when omitted, per the notes) |
| js_draft_storage | workspace `golden_draft_storage.rs` | 44/44 scenarios | Every step result plus the exact `storage_after` entries. Where JS passes `null` or a number, the typed API can't take it, so the test only checks that the expected result is a TypeError. |

Every skip count is asserted in its test, so a changed fixture fails loudly.

## API shaped

- `frankmd_core::text` (module tree): `utf16_to_byte`, `byte_to_utf16`, `utf16_len`; submodules `utils`, `markdown`, `table`,
  `indent`, `diff`, `stats`, `task`, `editor`, `typewriter`, `insertion` (`EditorSurface` trait, six
  `insert_*` fns, `TextareaAdapter` and `get_*` fallbacks over `Option<&dyn EditorSurface>`), `clipboard`.
- `frankmd_core::guard`: `check_deletion(Option<&str>, &str)`, `is_large_deletion` (lengths in UTF-16
  units, matching `lostPercent > 0.2 && lostChars > 50`).
- `frankmd_core::emoji`: `shortcodes() -> &'static [Emoji]`, `emoticons()`, `emoji_map()`, `lookup()`;
  `emoji::icons` (from `assets/icons/phosphor.json`); `emoji::i18n` (`LOCALE_MAP`, `emojibase_locale`,
  `build_search_index`, `translated_emojis`, `search_emojis`).
- `frankmd_workspace`: `shortcuts` (`KeyEvent`, `Shortcut`, ordered `ShortcutMap`, `DEFAULT_SHORTCUTS` as
  real const data, `Platform` injected instead of `navigator`), `menu_bounds`, `draft_storage`
  (`DraftBackend` trait shaped like localStorage, `DraftEnv` with injectable ids and clock), `autosave`
  (`AutosaveSession` state machine emitting `AutosaveEffect`s, `RecoveryRequest/Resolution`,
  `DeletionPrep`, timing constants, `AutosaveTimer::next_deadline`), `clock::FakeClock` (implemented,
  since it is test infrastructure).
- `frankmd_vim`: `ExCommand`/`EX_COMMANDS`/`ExCall`. `ex::parse` now returns `Option<ExCall>` and
  `Outcome::App(ExCall)`. Also new: `AppCommand::logical_name`, `next_note_index`, `app_owned_keys`/`is_app_owned`,
  `Key::notation`, `VimState::new(macos)`, `take_mode_change`, `Mode::label`. Dev-dep on frankmd-workspace.

## Cross-crate requirements

- frankmd-core needs `serde_json` as a normal dependency for parsing the embedded emoji/icon JSON
  (not added: Cargo.toml `[dependencies]` is shared with the core agent; only dev-deps were added).
- frankmd-workspace needs `uuid`, `serde`, `serde_json` at implementation time.
- frankmd-fs: a file-backed, ordered key/value store with atomic writes to back `DraftBackend` on disk.

## Behaviour decisions and quirks

- **Drafts keyed by path vs DocId:** the compat model (`frankmd:draft:<path>`, `draft-conflict`, legacy
  `backup`, schemaVersion 1, segment-boundary remap with verify-before-delete) is kept as the tested
  semantics. architecture.md keys drafts by DocId. Mapping documented in `drafts.rs`; **open question**.
  `updatedAt` must serialize as an integer; unknown record fields must round-trip.
- **Original bug not replicated:** `formatFileSize` gives "1.0 undefined" for 1 TiB and up. Rust continues to
  TB/PB (test rewrites those expectations).
- **JS prototype leaks not replicated:** `setEmojiLocale("toString"/"__proto__")` falls back to `en`;
  `getShortcutForAction("constructor")` cases skipped.
- **Table edits:** out-of-range swap is a no-op (JS created sparse arrays); `usize` indices drop JS
  `splice(-1)` semantics.
- **Replicated as-is:**
  - `getPositionForLine` past the last line returns length+1.
  - `getLineBoundaries` can return `lineEnd < lineStart` for an empty selection after a newline.
  - A reversed `wrapSelection` range duplicates text.
  - 1048575 bytes formats as "1024 KB".
  - `findTableAtPosition` past the end checks the last line, then builds from line 0.
  - An escaped `\|` still splits a table cell.
  - `parseIndentSetting("0x4")` gives a tab.
  - Fuzzy score, Levenshtein and table width count UTF-16 units.
  - Menu shift is checked before rounding (a 0.3px shift gives `translateX(0px)`).
  - A `lastModified` of 0 names the file `pasted-image-clip`.
- **Vim:** `Mode::CommandLine` reports label "normal" (as codemirror-vim does).
- **Fixture key order:** serde_json (no `preserve_order`) loses object key order, so fixture shortcut maps are
  rebuilt in default-table order.
- `remapCurrentFile` in `app_file_rename` ignores the case where the current file equals the renamed folder.
  Harmless; flagged.
