# UI test plan: category C (UI/system) tests

Status: plan, 2026-11-04. Scope: the 1418 category-C tests of FrankMD @ `d47a89e` (Ruby `test/system/*.rb`, the
C parts of `test/helpers`, `test/controllers`, the JS `test/javascript/{lib,image_sources,controllers}` suites), as
tagged in `docs/inventory/test-contracts.md`. The A/B tests of the same files are ported literally by the
`docs/testing/port-*.md` work; they're not repeated here.

These tests can't be ported literally: they drive the DOM, Stimulus targets, CodeMirror and Capybara. Each one is
re-expressed against the native design in `docs/design/ui-spec.md` (mockups: `docs/design/mockups/index.html`).
The rule is **push the logic down**: wherever a test really asserts state, ordering, text transforms, validation
or timers, the adapted test runs against a GUI-free view-model with plain `#[test]` and an injected clock. GPUI
headless tests are kept for wiring (key binding → action, focus movement and restore, overlay presence, pointer
and drag hit-testing, layout at a width). Pixels, animation, real IME and portals are left to manual checks.

## 1. Totals

| | Tests | keep | changed by design | dropped | (a) GUI-free VM | (b) GPUI headless | (c) manual |
|---|---:|---:|---:|---:|---:|---:|---:|
| G1 Ruby helpers / controllers / system | 121 | 78 | 40 | 3 | 112 | 3 | 3 |
| G2 JS lib + image_sources | 169 | 141 | 7 | 21 | 145 | 3 | 0 |
| G3 JS `app_*` controllers + ai_grammar | 151 | 119 | 28 | 4 | 132 | 14 | 1 |
| G4 autosave, code_dialog, codemirror, connection_monitor | 194 | 147 | 38 | 9 | 177 | 8 | 0 |
| G5 content_search, custom_theme, drag_drop, drop_images, emoji, file_finder | 186 | 169 | 13 | 4 | 171 | 10 | 1 |
| G6 file_operations … log_viewer | 195 | 155 | 38 | 2 | 159 | 33 | 1 |
| G7 media_storage, path_display, preview, recovery_diff, s3_option, scroll_sync | 169 | 137 | 27 | 5 | 163 | 1 | 0 |
| G8 settings, stats, table_editor, text_format, theme, typewriter, video_dialog | 233 | 156 | 67 | 10 | 206 | 11 | 6 |
| **Total** | **1418** | **1102 (78 %)** | **258 (18 %)** | **58 (4 %)** | **1265** | **83** | **12** |

- Every file's row count equals its C count in test-contracts.md (checked by script). Counting rules follow
  test-contracts: Ruby loops expanded; JS parts 3 and 5 expand `it.each` rows; part 4 (controllers `ai_grammar` …
  `file_operations`) counts an `it.each` table as one test. One deviation: `menu_bounds.test.js` line 46 is one row
  `[rows 1–2]`, as test-contracts counts it.
- Level counts use each row's primary level. **177 level-a rows also carry a thin level-b wiring test** (`a + b`),
  so the headless suite is about 260 tests in total; the 1265 level-a rows are the bulk.
- Dropped rows have no level. Reasons are on each row: DOM-specific plumbing (listener cleanup, data attributes,
  innerHTML escaping with no native analogue), `design-removed` (toolbar pins §3.4, boot splash, hover-only
  controls), CodeMirror-internal registration details, or `covered-by-A` (the behavior is already a ported A test or
  golden fixture, which is named).

Most-used level-a homes (rows): `workspace::explorer` 96, `workspace::insert_image` 87, `workspace::scroll_sync` 79,
`markdown::edit` 71, `workspace::layout` 68, `core::emoji_picker` 66, `workspace::settings_vm` 55,
`workspace::autosave` 48, `workspace::dialogs` 43, `workspace::quick_open` 42, `workspace::content_search` 37,
`workspace::navigation` 36, `workspace::preview` 33, `workspace::insert_video` 31, `workspace::status` 30,
`workspace::commands` 29, `workspace::overlay` 28.

## 2. How to read the tables

Each original file gets a section (§6–§13) with one row per C test:
`# | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status`.

- **Level** `a:<crate>::<module>`: pure view-model/state test in that GUI-free crate (preferred). `b`: GPUI headless
  test in `crates/frankmd-app/tests/`. `c`: manual/visual check. `a + b`: VM test plus a thin wiring test.
- **Status** `keep`: same behavior, re-expressed. `changed (§x.y …)`: the new design intentionally changes it, and the
  adapted test asserts the new behavior. `dropped (reason)`.
- Names (modules, actions, element ids) follow one shared vocabulary: §4 for modules, `tests/support/actions.rs` for
  actions and the keymap, and the element ids in §3.3.

## 3. Test harness (`crates/frankmd-app/tests/`)

### 3.1 Files

| File | What | Runs today |
|---|---|---|
| `tests/support/harness.rs` | `open_window` through the production `gpui_kit::open_window` (Base `Root`), width-class sizes `WIDE`/`MEDIUM`/`NARROW`, `in_window`, `press("ctrl-shift-p escape")`, `type_text`, `is_shown(id)`, `has_focus(id)`, `resize(width)` | real |
| `tests/support/actions.rs` | proposed `actions!(frankmd, [...])` set and `default_keymap()` (ui-spec §6.1/§6.2/§6.4, both shifted forms bound), key contexts `Workspace` / `Editor` / `Overlay` | real data |
| `tests/support/vm.rs` | proposed view-model types: `LayoutState`/`WidthClass`/`PaneArrangement`, `FocusRegion`, `OverlayStack`/`OverlayKind`/`PaletteMode`, `CommandRegistry`/`PaletteRow`, `QuickOpenModel`, `BannerStack`/`BannerKind`, `SaveState`, `MessageSlot` | `todo!()` stubs; each names its target module |
| `tests/support/shell.rs` | `open_shell(cx, size, VaultFixture)` → `Shell` with state accessors (`layout`, `focus_region`, `overlay`, `banners`, `save_state`, `active_note`, `simulate(banner)`) | `todo!()` (see 3.4) |
| `tests/harness_selftest.rs` | 12 tests proving the gpui test facilities on a probe view | **12/12 pass** |
| `tests/ui_shell.rs` | 10 adapted tests: width-class hysteresis, narrow swap, media library restore, zen (VM and headless), F6 cycle (VM and headless), wide layout + preview toggle, narrow drawer + Esc, medium auto-hide | todo panics |
| `tests/ui_overlays.rs` | 10 adapted tests: keymap has no duplicate chords per context (**passes**), palette availability, toggle state and recents, overlay stack focus restore, quick open recents-first, wrap, path match and create suggestion, palette open/Esc/focus restore, palette runs a command, quick open → open note, empty state with docked quick open | 1 pass, 9 todo |
| `tests/ui_banners.rs` | 5 adapted tests: banner priority, content-loss buttons, status message TTL, vault-unreachable banner keeps editing, content-loss Undo clears it | todo panics |

`cargo test -p frankmd-app --no-run` compiles. `cargo clippy -p frankmd-app --tests` adds no warnings besides `todo`.
Every adapted test cites the original(s) it adapts (`// adapts: <file>:<line> "<name>"`).

### 3.2 What works headless on Linux (gpui-kit =0.9.0, gpui-pre =0.3.11, verified by running `harness_selftest`)

Setup: `[dev-dependencies] gpui-kit = { workspace = true, features = ["test-support"] }` (already wired; no extra
features needed). Each test file needs `extern crate gpui_kit as gpui;`, because `#[gpui_kit::test]` expands to
`gpui::…` paths. Import kit items explicitly, not `use gpui_kit::*`: the glob brings in GPUI's `test` attribute and
shadows `#[test]`.

| Facility | Status |
|---|---|
| `#[gpui_kit::test]` sync and `async` tests, `TestAppContext`, entities | works |
| Headless window via `gpui_kit::open_window` + `WindowOptions { window_bounds }`; `render_frame` | works |
| Element observation: `.id(..).test_support()` → `window.find(id)` / `try_find` → `visible()`, `bounds()`, `focused()`, `label()`, `value()`, `role()` | works |
| Keystrokes through the real keymap: `cx.bind_keys`, `window.press("ctrl-shift-p")`, `cx.simulate_keystrokes(handle, "...")` | works |
| `cx.dispatch_action(handle, Action)` | works |
| Context-scoped bindings (`KeyBinding::new(.., Some("Probe"))` + `.key_context("Probe")`) | works |
| Focus: `FocusHandle::focus`, `.test_support().track_focus(&h)` → `focused()` | works (register `.test_support()` **before** `.track_focus`) |
| Pointer: `click`, `double_click` with real hit-testing (also `right_click`, `hover`, `scroll`, `drag_to`, `drag` exist; not exercised in the self-test) | works |
| `cx.simulate_window_resize(handle, size)` → `window.viewport_size()` in render | works |
| Deterministic timers: `cx.executor().advance_clock(d)` + `run_until_parked`; `TestAppContextExt::wait_for` | works |
| Test clipboard `write_to_clipboard` / `read_from_clipboard`; `cx.open_url` → `cx.opened_url()` | works |
| Rendered text: only for elements wrapped with `TestTextExt::test_text("…")`, read with `gpui_kit::base::test_support::texts(window)` | works (opt-in per element) |
| **Gotcha:** with no focused element, keystrokes don't reach root `on_action` handlers | verified (`keystroke_without_focus_is_not_dispatched`). The shell must focus the editor or the empty-state input on open, or global shortcuts are dead after launch |
| Not available: general rendered-text assertions (only exact accessibility labels or opt-in `test_text`), pixel inspection, cursor style, OS IME, portals | pixels need the opt-in WGPU `rendering` harness (lavapipe), IME needs `TestInput` (composition only); the rest is manual (level c) |

### 3.3 Element ids (contract between views and level-b tests)

`sidebar`, `explorer-tree`, `explorer-row-<relpath>`, `editor-pane`, `editor`, `pane-header`, `pane-title`,
`preview-pane`, `status-line`, `status-save`, `status-message`, `status-mode`, `banner-<kind>` (+ `-<button>`),
`find-bar`, `find-input`, `replace-input`, `command-palette`, `palette-input`, `palette-row-<command-id>`,
`quick-open`, `quick-open-input`, `empty-state`, `search-overlay`, `dialog-<kind>`, `settings-sheet`, `help-dialog`,
`image-viewer`, `media-library`. Each must be registered with `.test_support()` (before `track_focus`) and carry an
accessibility label equal to its palette command or i18n string.

### 3.4 Requirement on frankmd-app (not done here: "don't restructure the app")

`frankmd-app` is binary-only, so `tests/` can't reach its views. The implementation phase should add `src/lib.rs`
exposing `ui::Shell::new(window, cx, ShellConfig { vault, clock, env })` and `keymap::install(cx)`, keep `main.rs`
as a thin binary, and then replace the two `todo!()`s in `tests/support/shell.rs`. As each `support::vm` type gets
its real home (§4), the tests switch their imports and the stub is deleted. The keymap table should then be
generated from `workspace::commands` (HP-018), and the duplicate-chord test should run against the registry.

## 4. View-model / state requirements for the GUI-free crates

Consolidated and de-duplicated from the per-group lists (kept verbatim at the end of each group section). The implementation phase places these; the citing groups are in brackets.

Baseline: ui-vocab.md "Proposed GUI-free homes". Rules: architecture.md §1–2 (no GPUI outside
`frankmd-app`; filesystem writes only in `frankmd-fs`; config only via `frankmd-config`; HTTP only in
`frankmd-net`; workspace deps = core, fs, config, watch). ⚠ = rule conflict, with where the item should go instead.

| module | purpose | #groups |
|---|---|---|
| workspace::status | status line: save state, TTL messages, stats, position, indicators | 7 |
| workspace::insert_image | Insert-image dialog VM (sources, cards, footer, storage row) | 5 |
| workspace::overlay | overlay stack, Esc chain, focus restore, pending insertion | 5 |
| workspace::navigation | open/close/switch note, generations, history, transition gate | 5 |
| workspace::explorer | tree selection, menus, drag/drop, reveal, remap | 4 |
| workspace::layout | panes, width class, sidebar mode, view mode, text column | 4 |
| workspace::typewriter | typewriter scroll math + padding | 4 |
| workspace::settings_vm | Settings sheet rows, popovers, debounced writer | 4 |
| workspace::commands | registry, availability, document menu, clipboard effects | 4 |
| workspace::dialogs | form models + PromptQueue + DeleteConfirm | 3 |
| workspace::scroll_sync | editor↔preview sync state + pure math | 3 |
| workspace::quick_open | quick open model + empty state | 3 |
| workspace::inline_rename (aka rename) | title / explorer-row inline rename | 3 |
| workspace::autosave | per-doc save session, content-loss guard, transition pause | 2 |
| workspace::drafts | draft store details, remap, debounce writer | 2 |
| workspace::recovery | open-time recovery decision + diff VM | 2 |
| workspace::banners | banner kinds/buttons | 2 |
| workspace::media_library | media library VM | 2 |
| workspace::media_storage | provider ack/pending/fallback state | 2 |
| workspace::startup | CLI/IPC/restore initial note | 2 |
| workspace::table_editor | table dialog model | 2 |
| workspace::recents | recent-files store flow | 2 |
| workspace::editor_state (aka editor_view) | editor prefs, view config, read-only locks | 2 |
| workspace::preview (aka preview_render) | preview VM, render generation, hit activation | 2 |
| workspace::pane_header (aka layout::pane_header / header_arrangement) | breadcrumb, title, header buttons | 3 |
| workspace::created_notes (aka dialogs::UndoCreation) | undo/redo of note creation at history boundaries | 2 |
| workspace::grammar | grammar-check flow | 1 |
| workspace::content_search | search-in-files overlay | 1 |
| workspace::list_nav | shared list cursor (kbd/pointer) | 1 |
| workspace::slash_menu | slash popup model | 1 |
| workspace::dropdown | select/popup list state | 1 |
| workspace::menu_bounds | popover placement | 1 |
| workspace::text_format (moved from markdown::edit) | Ctrl+M text format menu | 1 |
| workspace::find_bar | in-editor find/replace bar | 1 |
| workspace::code_block | code-block dialog | 1 |
| workspace::insert_video | Insert-video dialog | 1 |
| workspace::image_viewer | image viewer overlay | 1 |
| workspace::diagnostics | Diagnostics logs/config sources | 1 |
| workspace::connectivity | vault reachability | 1 |
| workspace::lifecycle | window deactivate / quit decision | 2 |
| workspace::disk_sync | external change handling | 1 |
| workspace::undo_cache | per-doc undo snapshot LRU | 1 |
| workspace::motion | reduced-motion durations | 2 |
| markdown::edit | pure text transforms on (text, selection) | 7 |
| markdown::table | table detect/parse/serialize | 3 |
| markdown::linemap | line/offset queries, block tops | 2 |
| markdown::parse | RenderDoc details | 1 |
| markdown::highlight | sync code highlighting | 1 |
| vim::keymap | passthrough keys, mode-gated bindings | 2 |
| vim::ex | ex table → AppCommand | 1 |
| vim::state | mode + cursor shape | 1 |
| theme::custom | custom theme editor | 3 |
| theme::tokens | EditorStyle from theme + fonts | 1 |
| theme::picker | theme picker preview/commit/revert | 1 |
| theme::omarchy | Omarchy follow state | 1 |
| i18n::picker | language picker | 1 |
| core::emoji_picker | emoji picker VM | 1 |
| media::sources::folder | folder image metadata | 1 |

### Cross-cutting decisions (apply before reading modules)

- **D-1 ⚠ workspace dependency edges.** Many workspace VMs call `markdown::{table,edit,linemap}`, `search::{finder,find}`, `core::stats`, `i18n` and `vim::state`. The §2 graph has workspace → {core, fs, config, watch} only. Either add `workspace → {markdown, search, i18n, vim}` (acyclic: those depend only on core), or keep the VM free of them and have it take precomputed inputs. Recommended: add `markdown`, `search` and `i18n` edges and keep `vim` out (the status VM takes a `ModeLabel` value). [G1, G2, G3, G4, G5, G6, G8]
- **D-2 Effects, not IO.** Every VM that "reads", "writes", "probes", "uploads", "copies" or "fetches" returns an `Effect` (`Load`, `ReadPreview`, `WriteConfig(patch)`, `WriteClipboard`, `OpenFileChooser`, `Probe`, `Search`, `Upload`, `Fetch`) and receives `*_result(gen, Result)`. The app runs fs effects through `frankmd-fs`, config through `frankmd-config::ConfigStore`, HTTP through `frankmd-net`, and clipboard/portals in `frankmd-app`. "Injected Clipboard trait" (G6, G7) becomes `Effect::WriteClipboard` + result. [G3, G6, G7]
- **D-3 Labels.** VMs return `MsgKey {key, args}`, never resolved strings. If D-1 adds the i18n edge, resolution with en fallback (never a raw key) is tested in workspace; otherwise in app. [G1, G4, G7]
- **D-4 Clock.** Every TTL/debounce/probe goes through the injected `workspace::clock`. [all]

### frankmd-workspace

#### workspace::status
Status-line VM.
- `SaveState {Unsaved, Saving, Saved, SaveFailed, DraftNotStored}` with `save_label()` via MsgKey (i18n, en fallback, never a raw key) [G1, G4]
- Message slot with TTL (2 s info / 5 s error); `StatusLine::error(msg)` for move/rename failures; "Copied to clipboard" info (also mirrored to the Diagnostics message log); a clipboard failure gives one error and no success message [G1, G3, G5, G7]
- Stats slot: visible iff `show_stats`, including after a late config load; "N words" from `core::stats`; recompute debounced 500 ms keyed by buffer **revision** (not length); immediate on note open, cancelled on close [G1, G3, G8]
- Position: `line:col` vs `line/total` by `LineNumberMode`; updates on caret move with the context chip [G2, G4, G8]
- Indicators include Typewriter; mode block NORMAL/INSERT/VISUAL/V-LINE/COMMAND from the vim mode (⚠ D-1: pass a `ModeLabel` in, no vim dep) [G2, G8]

#### workspace::insert_image
Insert-image dialog VM.
- Sources rail + per-source state; `ImageCard {id, label, title, caption, thumbnail: Option<ThumbSource{Path|Url}>, insert_target, dims_badge, size_label, is_selected}`; `thumbnail_failed(id)` removes the card; unsafe-URL thumbnails suppressed; selection cleared on new search/page/folder; status strings ("N images found", "Showing N most recent of M images") [G2]
- Library source: collection {NoteImages, ImageFolder} (ImageFolder disabled when unavailable); debounced filename search; query `{collection, search, limit, offset}` with collection/page-size reset; reuses existing references; `ImportRequest{path, resize, s3_key}`; per-collection delete ops [G1, G6]
- Upload source: `ingest(files)` (clears selection, emits `Deselected` only if something was selected); `ingest_external(file)` (auto-select; non-images → inline error); paste ingest with Processing always cleared; `dropzone_hover`; `Effect::OpenFileChooser` [G5, G6]
- Folder source errors `folder_access_failed` / `folder_read_failed`; folder pick `Cancelled | Err` [G2, G6]
- AI source: ref-chooser rows + `ref_image` [G2]
- Footer: selected_name, alt prefilled from the stem; `insert_markdown()` with optional link wrap `[![alt](url)](link)`; `InsertResult{markdown, url, alt, link}`; insert waits on a pending provider save (`media_storage::ready()`); error kept inline + one storage refresh [G1, G6]
- `StorageRow`: `provider`, `set_provider`, S3 `disabled_reason`, `resize` ∈ {0.25, 0.5, 0.75, 1.0} default 0.5 (S3 only), `set_default_key`, sticky `on_key_edited`, `key_value()` trimmed and S3-only, `reset()` to the global default, `uses_s3()`, `resize_ratio()`, `Default` [G7]
- ⚠ Thumbnail/web fetches are effects run through `frankmd-net` EgressClient; the VM only holds URLs (D-2) [G2]

#### workspace::overlay
Overlay stack, Esc priority, focus restore.
- Esc chain `escape_target(&UiState)`, highest first: Select popover in Settings → Settings → InlineRename → explorer context menu (Popover) → MediaLibrary → NarrowDrawer; `Dialog(Recovery)` → None (backdrop ignored too) [G1, G3, G7]
- Focus restore to the opener (dock-footer Settings button, preview image, "Customize…") [G1]
- A prompt can stack over its parent dialog (the parent stays open, focus returns into it); popovers owned by a dialog close with it; at most one popover open [G2]
- `PendingInsertion {range, query, kind}` stored when a slash item opens an overlay; cleared on cancel/close and on a non-markdown note [G3]
- Open Settings at a category/section (OpenDiagnostics → Diagnostics › Logs) [G6]

#### workspace::navigation
Open-note flow and history.
- `NavigationState::open(path) -> (Generation, Effect::Load)`; `complete(gen, LoadResult)` ignores stale results (incl. NotFound/IO); `close_note()` [G3, G4]
- Transition gate `prepare_transition()` (built on `autosave::prepare_for_transition`): flush draft, disarm timers, never force a save; refuses when the draft can't be stored or the base revision is unknown; used before switch, rename, move, delete; `resume_after_transition()` on every exit path [G3, G4, G5, G6]
- `OpenOutcome {focus_editor: content.is_empty(), select_in_explorer}`; an explorer click keeps Sidebar focus [G1, G3]
- Back/forward history across switches restores drafts; back past the first entry → empty state [G1]
- Note-not-found: status message + QuickOpen prefilled with the stem [G1]
- `on_renamed/on_moved/on_deleted(path, kind)` (FileCreated/Renamed/Deleted) remap the current note, autosave target, MRU, selection and history, clear pending insertion, or return to the empty state; `rename_path(item, name)` for the pane title [G3, G6]
- Preview-task / toggle-task guarded by open file kind [G3]

#### workspace::explorer
Tree VM.
- Selection: `click(row, Modifiers) -> Option<ExplorerEffect{OpenNote, ToggleFolder}>`, ctrl-toggle, shift-range, `select_all_visible()`; `on_focus(row, FocusCause{Pointer, Keyboard})`; `click_background()` clears the selection only; the open-note marker is separate from the selection [G1, G3]
- Context menu: `open_context_menu(row)` keeps a multi-selection if the row is in it, else reselects; `context_menu: Option<(MenuKind{Row, Background}, Point)>` closed by a selection change; background items = [NewNote, NewFolder, SelectAll, CollapseAll]; note rows exclude NewNoteHere/NewFolderHere [G1, G3, G6]
- `creation_destination(origin: Global | FolderContext(p))` = single selected folder (even collapsed), else root; `expand_ancestors` before create; `reveal(path)` after create/move expands and persists ancestors [G1, G3, G6]
- `delete_targets()` drops descendants and protected config; `rename_target()` excludes `.fed`; dotfiles never produce rows [G1, G3, G6]
- `DragState`: `begin_drag(row)` (payload = selection if the grabbed row is selected, else [grabbed]; dedupe ancestors); `drag_hover(Row|Background|Outside) -> Option<DropTarget{Folder, Root}>` (note → its folder); `drop_cursor()`, `is_dragging`, `drag_badge_count`, `root_highlighted`, `end_drag`, `drop() -> MovePlan`; `reset_tree` clears the drag [G1, G5]
- `execute_moves(plan)`: sequential, stops at the first error, expands destination + ancestors first, emits `Event::ItemMoved{old, new, kind}` (moves are fs effects, D-2) [G5]
- `remap/remove(path, kind)` incl. anchor & focus; tree snapshot `Generation` with stale-discard; selection and expansion restored after reload; `empty_message()` only at the root [G1, G3, G5]
- Inline rename → see `workspace::inline_rename` [G6]

#### workspace::layout
Pane arrangement.
- `SidebarMode {Docked, Hidden, Drawer{open}}`, drawer closes on open-note; focus fix-up hook on width-class change [G1]
- `text_column(pane_w, editor_width, char_w) -> (inline_margin, column_w)` [G1]
- `SinglePane {Editor, Preview}` kept across Settings open/close; `ViewMode::parse` falls back to Split [G3]
- `media_library_open` restores preview visibility on both Esc/toggle and open-note exits; `TypewriterRelayout` effect on library close and sidebar toggle [G3]
- `typewriter_pref` vs `typewriter_effective(file_kind)`; derived `preview_follow` (only when the preview is visible) [G3, G8]
- `set_preview_visible` / `toggle_preview -> Option<Effect::PreviewToggled{visible}>` (None when unchanged); visibility persisted per vault [G7]
- Non-markdown preview placeholder [G3]
- Header/title items moved to `workspace::pane_header` [G1, G3]

#### workspace::typewriter
Pure typewriter math.
- `ScrollCause {PointerClick, PointerRange, Keyboard, DocChange}`; `recenter_target(cause, typewriter, recenter_on_click, Geometry{scroll_top, caret_top, client_h, scroll_h}) -> Option<f32>` clamped to `scroll_h − client_h` [G2]
- `recenter_after_click(collapsed, pref, caret_top, viewport_top, scroll_top, client_h)` = caret_top − viewport_top + scroll_top − client_h/2 [G4]
- `scroll_target(caret_y, viewport) -> Option<f32>` (None when off / no editor); `on_doc_change -> ScrollAdjust` without a selection change [G3, G8]
- `bottom_padding(client_h) = client_h/2` (0 when off/hidden); `refresh(geometry)` after a visibility change [G2, G8]
- `sync_data(caret_line, total) -> Option<(line, total)>`, only when the preview is visible [G3, G8]

#### workspace::settings_vm
Settings sheet VM.
- `SettingsVm`: `selected_category` (Appearance…About), `visible_rows()`, search filter; rows with provenance [G8, vocab]
- Per-key setters apply live and emit `EditorConfigChanged` / `ThemeChanged` / `LayoutChanged`, even with no note open; `EditorConfig::from_settings` (font mapping, size, line numbers, indent unit, recenter, typewriter) [G8]
- `reload(ConfigSnapshot)`, tolerant (unknown view_mode → Split, missing key keeps current, read error keeps values + logs) [G8]
- Writer: 300 ms debounce coalescing **all** pending keys, flush on quit, `ConfigWritten`; inline row error + revert on write failure; writes go out as `Effect::WriteConfig(patch)` → `frankmd-config` (D-2) [G1, G8]
- `open_popover: Option<RowKey>` (single; closed on scroll, category switch, sheet close, option pick) [G1]
- Rows: About (logo, name, description, version, copyright, source link → OpenUri); Writing `show_stats` (debounced), Typewriter, Recenter caret on click (two-way bound to live state); Editor font/size/width with provenance; `font_options()` (System monospace + 9 ids), `font_preview()` [G1, G2, G8]
- `media_storage_provider` row bound to `MediaStorageState`: disabled while pending, option-level `disabled_reason` (S3), inline error, saves **immediately** (no debounce) [G1, G7]

#### workspace::commands
Command registry.
- `document_menu(&UiState) -> Vec<MenuRow{CommandId, enabled, shortcut}>` (aka `doc_menu`), groups per §3.4, per mode (Editor, SinglePreview, MediaLibrary, Empty); recomputed while open [G1, G3]
- Availability reasons: RenameNote (none, `.fed`), DeleteNote, CopyContents, CopyPath (no note); GrammarCheck (no note / not configured / running); ToggleTask, Insert*, TextFormatMenu (non-markdown); while Settings is open everything is Unavailable except Escape and OpenSettings [G1, G3, G7, G8]
- CopyContents / CopyPath → `Effect::WriteClipboard(text)`; result → status message (D-2) [G3, G7]
- Per-context bindings (`delete` in the Editor never maps to DeleteNote); i18n labels with no shortcut text in any locale; `toggle_state` for ToggleTypewriter [G1, G8]
- `header_buttons(width)` → moved to `pane_header` [G1]

#### workspace::dialogs
Dialog form models.
- `PromptQueue`: `PromptKind {Alert{msg}, Confirm{msg, accept_label, destructive}, TextPrompt{label, default}}`; `buttons()` secondary then primary (Danger when destructive); `initial_focus()` (Cancel when destructive); `PromptResult {Ack, Bool, Text, Cancelled}`; Enter/Ctrl+Enter accept in text input; FIFO with focus restore [G2]
- `NewNoteForm{destination, template, name, slug, error}`: `new(origin/selection, hugo_path_style, clock)`, path preview strings (incl. the root placeholder "note-name.md"), toolbar/global Hugo → `blog/` (dated or flat), folder-context Hugo → that folder, `can_submit`, `submit() -> Option<CreateRequest{Empty{path} | Hugo{title, parent, slug}}>` [G1, G6]
- `NewFolderForm{parent, name, error}` with subtitle/title keys [G6]
- `DeleteConfirm` from a captured target list: counts, file/folder/multi and trash wording, destructive, focus Cancel; non-empty folder per §12 Q3; sequential executor stops at the first error [G1, G6]
- `UndoCreation` → see `workspace::created_notes`

#### workspace::scroll_sync
Editor↔preview sync.
- `ScrollSync {lock: Option<ScrollSource>, lock_until 400 ms, typewriter, sync_enabled, preview_visible, pending_resize 200 ms}`; `on_editor_scroll`, `on_preview_scroll`, `on_programmatic_preview_scroll`, `on_preview_toggled`, `on_content_changed`, `refresh_preview`, `reanchor(Reason)`, `on_window_resized`, `on_preview_relayout`, `frame_tick` (toggle lock cleared after 2 frames) [G7]
- `PreviewBlockMap` from preview layout in content coordinates (independent of pane origin/scroll); `line_at(scroll_top)` [G2]
- Pure fns: `top_visible_line(scroll_top, line_tops)` probing `scroll_top + 1` (1 for empty layout), `scroll_top_for_line(n)` clamped to [1, lines], `clamp_scroll`, `scroll_ratio` ∈ [0,1], `ratio_to_offset` (snap ≤0.01/≥0.99), `preview_ratio`, `sync_ratio` (round(ratio×total)+1, anchor at top), `sync_to_line` ((line−1)/(total−1)), `sync_to_line_smooth` (anchor − 50 px, element-count fallback), `typewriter_target` [G4, G7]
- `ScrollIntent {top_px, animate}`; animate false under reduced motion (`workspace::motion`) [G7]

#### workspace::quick_open
Quick-open overlay and empty state.
- `QuickOpenModel`: `open(notes)` resets; `rows()` {name, folder, match ranges, selected} via `search::finder` (D-1); `empty_state()` NoMatch{query}; `activate(Open | OpenAndReveal | PreviewOnly)`; preview {None, Loading, Lines(12, truncated), Error(key)} via `Effect::ReadPreview(path, gen)`; list navigation via `list_nav` [G5]
- `EmptyStateModel`: logo, vault name, note count, recents section only when non-empty, `ClearHistory` → PromptQueue confirm, shortcut chips from the registry; recent rows with a relative-time label (injected Clock, ported `recent_file_opened_ago`, "Unknown time" fallback) [G1, G6]

#### workspace::recents
Recent-files flow (on top of `core` recents).
- Record on successful open only, in call order; cap 10, newest first; prune missing on read/refresh; a stale entry never creates a file [G1, G6]
- `clear_history()` with confirm text, persisted to `.fed` `recent_files` via `frankmd-config` effect; refresh generation guard; read failure keeps the previous rows + status error [G1, G6]

#### workspace::inline_rename (aka rename)
Inline rename shared by the pane title and explorer rows.
- `InlineRename {target: Title | ExplorerRow(path), value, caret, selection, state: Editing | Saving | Error(Option<MsgKey>), invalid, generation}` [G1, G3, G6]
- `start(path)`: basename without `.md`, caret at the end; `input`; `on_enter(composing)` (no submit while composing); `on_blur(to)`; `cancel` (focus back to origin); `on_result(gen, Result)` drops stale gens [G1, G3, G6]
- Validation: blank → `errors.title_required`; collision (/exist/i) with no FS change; `.md` appended to notes; error kept while editing [G1, G6]
- Ignores a duplicate submit while Saving; refuses a submit when the current note changed; title rename stays in the same folder; dirty buffer saved to the new path; focus target after commit [G1, G3]

#### workspace::autosave
Per-doc save session (extends existing `AutosaveTimer`).
- `DocSession` per DocId: `edited(text, now)`, `tick(now) -> Vec<Effect>`, `save_now()`, `save_completed(doc, edit_gen, Result<Revision>)`; single in-flight save, rescheduled if the text changed during the save; baseline text + base revision [G4]
- `vault_unreachable()/vault_reachable()`: disarm timers, keep dirty, save on reconnect [G4]
- Rename/folder rename via registry `Event::Remapped` (segment-wise); delete clears timers and ignores late acks; `prepare_delete(doc) -> Ready{revision} | Wait | Stale` [G4]
- `prepare_for_transition() -> Result<(), DraftNotStored>` / `resume_after_transition()` [G5]
- `ContentLossGuard {active, approved_snapshot_hash}` using `core::content_loss::is_large_deletion`; one-shot approval bound to the exact snapshot; `undo_result(text)`; `Effect::LoadBaseline(doc)` when the editor has no history; `dismiss()`; auto-clears when restored [G4]

#### workspace::drafts
Draft store details.
- `draft_revision` counter + `updated_at`; `discard_if_revision`, `rebase(doc, new_base)`, `remove_all_for(doc) -> usize` [G4]
- `DraftWriter` 300 ms per-doc debounce, independent across docs; in-memory retained snapshot + retry on failure; `Event::DraftStorageFailed / Recovered` [G4]
- Remap on rename/move (file and folder; the prefix match includes the trailing `/`); delete on note/folder delete; empty-string draft valid; no draft when buffer == disk; on restart restore → `SaveState::Unsaved` without Recovery when the base revision matches [G1]

#### workspace::recovery
Recovery record VM.
- `decide(draft, disk) -> AutoRestore | DiscardIdentical | Ask(RecoveryRecord)`; the record carries draft time + file mtime [G4]
- `RecoveryVm::diff()` word diff (Equal/Del/Add; file left, draft right); timestamp chips via MsgKey [G7]
- `choose(UseFile | KeepBoth | RestoreDraft) -> Resolution{choice, path, content, draft_content, draft_revision, draft_ts, kind, conflict_id}`; RestoreDraft rebases the draft and recomputes content loss; UseFile discards only that DocId's draft; resolving removes only that record; not offered again after resolution [G4, G7]

#### workspace::banners
- Kinds: `ContentLoss{removed_chars, percent}`, `DraftStorageFailed`, `VaultUnreachable{vault}`, `ReadOnly`, `ChangedOnDisk`, `StorageFallback` (Settings / Dismiss; dismiss sticky for the session, at most once per session); button ids `banner-<kind>-<button>` [G4, G7]

#### workspace::media_library
- Open-transition counter emits `Reload + ResetUsage` once per open [G3]
- Lazy usage on `mark_visible(card)`, one shared scan, cache reset on reopen; usage label Checking / Used(n) / NotUsed / Unknown (S3 always Unknown) [G6]
- Detail sheet: metadata string incl. aspect ratio, filtered + deduped "Used in"; copy path → `Effect::WriteClipboard` (D-2); delete/rename flows with inline errors; no re-sorting [G6]
- `insert(card, &OpenNote) -> InsertStatus{Inserted(Edit) | NoOpenNote | MarkdownNoteRequired | InvalidMedia}` (Pending → Inserted closes the sheet) + `CloseLibrary`, `FocusEditor` effects [G3, G6]

#### workspace::media_storage
Shared by the Settings row and the image footer.
- `MediaStorageState {acknowledged, pending: Option<Provider>, s3_available, error, fallback_announced}`; `request(p)`, `ack(Result<ConfigSnapshot, Error>)`, `apply(snapshot, fallback)`, `refresh(force / known_fallback)` (used by settings reload and upload failures instead of applying stale snapshots), `ready() -> Gate` (uploads wait; Err(`media_storage.save_failed`)); emits `BannerKind::StorageFallback` at most once per session [G7, G8]

#### workspace::startup
- `resolve(args) -> StartupTarget {None, Note(rel), Missing(rel)}`: CLI path relative to the vault, absolute paths inside the vault accepted, nested Hugo paths, `.fed` refused, Missing → note-not-found flow; byte-exact load [G1]
- Initial note (CLI/IPC/restore) opened through the guarded navigation transition after workspace ready; expands parents; no-op before ready [G3]

#### workspace::table_editor
- `TableEditorModel` (aka `TableEditorVm`): default 3×3 with headers "Header 1…n"; `dimensions()`, `set_size/set_rows/set_cols/inc/dec` (no-op + not dirty when unchanged), `dirty`; add/remove/move rows and columns with minimum sizes; deleting the header row disabled; `cell_menu(r, c)` with enabled state + shortcuts; verbatim cell text [G1, G8]
- `from_markdown(text, caret)` via `markdown::table` (edit mode → "Update"); `to_markdown()` padded with pipe escaping; `confirm() -> Option<TableEdit{markdown, replace_range}>` (None + close when empty); applied as one edit [G1, G8]

#### workspace::editor_state (aka editor_view)
- `EditorPrefs {font_family, font_size 12–24, line_height, line_numbers: LineNumberMode Off→Absolute→Relative, cursor_recenter_on_click}`: global, survive note switches [G4]
- `EditorViewConfig {placeholder, fonts, line_numbers, read_only, vim, typewriter}`; live reconfigure keeps doc/caret/undo depth; `gutter_labels(mode, total_lines, caret_line) -> Option<Vec<String>>` [G2]
- `ReadOnly {base, locks: counter}` with `acquire_lock() -> LockId` / `release`; connectivity never touches read-only [G4]

#### workspace::preview (aka preview_render)
- `PreviewVm {zoom, typewriter, sync_enabled, last_rendered, settle_until (100 ms after render), debounce 150 ms, last_synced_line, generation}`; `content_changed(src, caret)`, `on_rendered(doc)`, `on_user_scroll(PreviewMetrics) -> Option<PreviewScrolled>`, `zoom_in/out`, `text_px(zoom) = zoom × 15 / 100`, `header_label()`, `bottom_padding_px()` [G7]
- `activate(Hit) -> PreviewAction {OpenImageViewer{alt, src, return_focus}, OpenLink(url), ToggleTask{line, generation}}`; stale-generation guard [G7]
- Render generation counter; off-thread highlight results tagged with the generation and dropped when stale [G2]

#### workspace::pane_header (aka layout::pane_header / header_arrangement)
- `PaneHeaderVm {full_path, breadcrumb: Vec<Segment>, title (literal stem), tooltip, a11y_label ("Rename document: {name}")}`; `breadcrumb(path, WidthClass)` collapses to `…` below 720 px; `window_title(note, vault)` [G1, G7]
- `header_buttons(state, width)` enabled/hidden per mode; `pane_title(state)` [G1, G3]

#### workspace::created_notes (aka dialogs::UndoCreation)
- `CreatedNoteBoundaries`: `record(path, initial_content, initial_revision, previous)` only when the load is current and both baseline parts are known; `remap(old, new, kind)` with dependent `previous` remap and stale-destination drop; `evict(subtree)` [G3]
- `undo_at_history_start(path, content) -> Handled | NotHandled`; deletion state machine: drain autosave → ReadOnlyLock → Delete{expected_revision} → mark deleted → open predecessor; aborts on content/navigation change or conflict with status errors [G3]
- `redo_at_history_end(pred)` + root boundary; newest wins; recreate effect; stale completion ignored; autosave pause/resume on every exit path [G3]
- Suppressed in vim mode [G4]

#### workspace::grammar
- `GrammarState {availability: Unknown | NotConfigured | Available{provider, model}, request: Option<RequestId>, view: Diff | Edit, edited, hunks}`; `from_config`; `start(doc) -> (RequestId, [FlushAutosave, LockReadOnly, PushBanner])`; `finish(id, Result)` (ignores a stale id; clears request, unlocks, pops banner); `cancel()` no-op when idle; `toggle_edit()`; `apply() -> Edit` (one undo step, honours rejected hunks) [G3]
- ⚠ The AI call is `frankmd-ai` (→ net), which workspace must not depend on. Define `Hunk` in core or workspace and compute the diff with `similar` there; ai returns only the corrected text via an effect result [G3]

#### workspace::content_search
- `ContentSearchModel`: `open()` resets; `set_query` with 300 ms debounce; `Effect::Search{query, regex, case, scope, gen}`; `apply_results/apply_error(gen)` drop stale gens; `searching()` true from the first keystroke until result/clear; `status()` {Hint, Count{matches, notes, truncated}, NoMatches, Error, InvalidPatternFallback}; results grouped by note, header rows skipped by navigation; `activate(keep_open) -> Option<OpenAt{path, line}>` [G5]

#### workspace::list_nav
- `ListCursor {selected, len, input_mode: Keyboard | Pointer}`: `move_next/prev` with wrap, `hover(i) -> bool` (ignored in Keyboard mode, false on the same index), `pointer_moved()`, `select`, `is_selected`, `scroll_request()` (keyboard moves only); used by content_search and quick_open [G5]

#### workspace::slash_menu
- `SlashMenuModel`: rows `{CommandId, label, icon}`, up/down, `accept() -> SlashOutcome{Edit(EditResult) | OpenDialog{kind, from, to, query}}`, `dismiss()` keeps the query, `visible_rows()` ≤ 8 at 28 px [G2]

#### workspace::dropdown
- `DropdownState`: `open()` focuses the checked option; `move(dir)` skips disabled; Home/End; `escape()`/`tab()` close and return focus to the trigger (Tab not consumed); `on_resize(trigger_rect, viewport)`, `on_container_scroll`, `on_focus_out()`; `keep_visible(option_rect, max_h) -> scroll_top` [G2]

#### workspace::menu_bounds
- `place_popover(trigger, menu_size, content_h, viewport, gutter = 8) -> {top, left, max_height, max_width, flipped}` with **no** 96 px minimum (unlike `menu_max_height`) [G2]

#### workspace::text_format (moved from markdown::edit)
- `TextFormatMenu`: open(target)/close; the open guard rejects an empty/whitespace selection; selected index with wrap; case-insensitive mnemonic dispatch; rows with mnemonic + registry shortcut; `anchor_offset()` = selection end; `apply(id) -> Option<Edit>` delegating to `markdown::edit`; `on_context_click(selection, is_markdown) -> TextFormat | ContextMenu | None` [G8]
- ⚠ G8 proposed this in `frankmd-markdown::edit`, but it needs registry shortcuts (`workspace::commands`) and markdown can't depend on workspace. Menu state lives here; only the pure transforms stay in markdown [G8]

#### workspace::find_bar
- `FindBarState {open, replace_visible, query, regex, case, matches, current, status: Counter | Error | Replaced(n){since}}`; `find_next` starts from the caret selection end; Replaced feedback with a 2 s TTL; matching via `search::find` (D-1) [G6]

#### workspace::code_block
- `CodeBlockForm`: mode Insert / Update(range), language + content, language list (44 FrankMD ids first), suggestion rows (ported matcher), `key_tab()` accepts the first, `hide_suggestions`, `select_suggestion`, `indent_at_caret(use_tabs)`, `confirm() -> Insert(Edit) | UnknownLanguage{did_you_mean}`, `use_suggested()/keep_typed()` [G4]

#### workspace::insert_video
- Tabs Upload / Url / YouTube; reset on open; `youtube_status` Configured / NotConfigured; results grid (2 cols) with selection + wrap; thumbnail fallback chain; upload state Idle / Uploading{progress} / Error with a single-flight guard and an extension allow-list; file-chooser effect; persisted Hugo shortcode flag; URL detection [G8, vocab]
- ⚠ YouTube search/upload are effects run through `frankmd-media` → `frankmd-net` (D-2) [G8]

#### workspace::image_viewer
- `ImageViewerState {src, alt, generation, status: Loading | Loaded | Unavailable, return_focus}`; `open(src)` rejects `javascript:`, `file://` and non-image `data:` (accepts vault-relative and http(s)); stale-generation results ignored; focus falls back to the preview pane when the trigger is gone [G6]
- ⚠ Remote image bytes are fetched through `frankmd-net` EgressClient, never directly by the app [G6]

#### workspace::diagnostics
- `DiagnosticsState {section: Logs | ConfigSources, logs: Loading | Loaded{env, file, lines} | Empty | Error, config: Loading | Loaded{entries, file, file_exists, ai_in_file} | Empty | Error}`; tail N = 100; count strings with singular forms; `refresh()` reloads only the active section; secrets masked [G6]
- ⚠ Reading the log tail is an IO effect (tolerant read in `frankmd-fs`); config sources and provenance come from `frankmd-config::ResolvedConfig`, and masking belongs in config, not in the VM [G6]

#### workspace::connectivity
- `Connectivity {state: Reachable | Unreachable(reason), probe_in_flight, vault_gen}`; 5 s probe interval on the Clock; `retry()`; `on_probe_result(gen, Result)`; `on_io_error(kind)` classifies ENOTCONN/ESTALE/EIO/root NotFound/PermissionDenied; hints (watcher event on root, window activation); emits `VaultUnreachable/VaultReachable` once per transition [G4]
- ⚠ The probe (`stat` of the vault root) is `Effect::Probe`, executed via `frankmd-fs` [G4]

#### workspace::lifecycle
- `on_window_deactivated()` does a sync draft flush, no save; `quit_requested() -> QuitDecision {Proceed{effects}, Ask{count}}` per §5.23; no redundant rewrite of unchanged drafts; flushes pending settings/theme writes [G4, G8]

#### workspace::disk_sync
- External change: identical content → no-op; different → ChangedOnDisk banner; `Reload` → `Effect::ReplaceBuffer` [G4]

#### workspace::undo_cache
- LRU (cap 20) of editor undo snapshots keyed by DocId; the active doc is never evicted; refreshed on revisit; invalidated when loaded text ≠ cached text; evicted on delete (file and folder subtree) [G4]
- ⚠ The undo history type belongs to the GPUI editor. Make the cache generic `UndoCache<S>` over an opaque snapshot so workspace stays GPUI-free [G4]

#### workspace::motion
- `reduce_motion` flag (value supplied by the app from the portal); `scroll_duration()` 0 when reduced, else 120 ms; consumed by scroll_sync and typewriter. Could be a field of `layout` [G6, G7]

### frankmd-markdown

#### markdown::edit
Pure transforms on `(text, Selection) -> Option<Edit>`.
- `surround_on_type(text, &[SelRange{anchor, head}], ch, InputCtx{source: Typed | Paste, composing, read_only}) -> Option<EditResult>`: several ranges, backward ranges kept, inner selection kept, linear time, one undo group per keystroke [G1, G2]
- `apply_pending_insertion(text, PendingInsertion, Insert{Block | Inline}(s))`: block widens over adjacent spaces and pads newlines; inline exact; stale query → None [G3]
- `toggle_task_at_line(text, line) -> Option<Edit>`: None when out of range or not a task; preserves the caret [G3, G4, G7]
- Buffer primitives `selection_text`, `set_selection`, `replace_selection`, `insert_at`, `replace_range`, all returning `Edit`s [G4]
- `insert_emoji(text, sel, insert)` [G5]
- Format transforms used by `workspace::text_format` (the menu VM itself moved, see ⚠ there) [G8]

#### markdown::table
- Parse the table at the caret with a `/table` slash query stripped from its cell, leaving the document text untouched [G3]
- `from_markdown` / `to_markdown` (padded, pipe-escaped) used by `table_editor` [G1, G8]

#### markdown::linemap
- `line_count`, `line_text(n)` ("" when out of range), `line_start(n)`, `position(offset) -> (line, col)` 1-based, `offset_of(line, col)` clamped [G4]
- Block tops by index for the element-count fallback when spans are missing [G7]

#### markdown::parse
- `total_lines` (1 for "# Hello World"); raw-HTML allow-list maps `<img>` (attrs filtered to src/alt/title) and `<picture>` (single Image, `<img>` src) and never emits form inputs; task `ListItem.span.start_line` is absolute (front matter counted) [G7]

#### markdown::highlight
- Synchronous highlighting of all bundled languages within one render pass, same limits as the A tests [G2]

### frankmd-vim

#### vim::keymap
- Passthrough set `ctrl-f n p e b i v w .`, excluding `ctrl-d/u/q`; idempotent [G2]
- In Normal/Visual mode `*` resolves to search; surround-on-type only active in Insert / non-vim [G1]

#### vim::ex
- Name + abbrev table → `AppCommand`, no duplicates; `:noh`, `:qa`, `:set nu/rnu/nonu`, `:<N>` [G2]

#### vim::state
- Mode + visual submode; cursor shape Block/Bar [G2]

### frankmd-theme

#### theme::custom
- `CustomThemeEditor`: `set_color(key, raw)` with per-key `InvalidColor` ("six-digit hex"), `save_enabled()` / `can_save`, `contrast_warning() -> Option<ratio>` (min of text-primary/bg-primary and accent-text/accent, < 4.5, 2 dp), `reset()`, `set_base(id)` with base-theme syntax defaults (functions #f0e68c), `preview_tokens()` / `preview(palette)` [G1, G5, G8]
- `cancel()` restores the captured snapshot / prior theme id and reverts the live tokens [G1, G5, G8]
- `save()` → one atomic write {`theme = custom`, `custom_theme.colors`}; `SaveFailed` keeps the dialog editable [G1, G5]
- ⚠ theme has no config dependency: `save()` must return a `ConfigPatch` value applied by workspace/app via `frankmd-config` [G1, G5]

#### theme::tokens
- `EditorStyle::from(theme, fonts)` covering chrome roles + markdown/syntax highlight map; documented font defaults; fonts kept across a theme switch [G2]

#### theme::picker
- `ThemePickerModel`: rows FollowOmarchy (with the current Omarchy name when detected), FollowSystem, 18 built-ins with Light/Dark glyph + 4 swatches, custom, Customize…; `current` marker; preview/commit/revert; ignores a same-theme ConfigChanged [G8]

#### theme::omarchy
- Follow state active only while following Omarchy; generation counter so late reloads can't override a custom preview; removal → fallback dark; mid-swap → last good palette [G8]
- ⚠ The file watcher belongs in `frankmd-watch` (it already owns the Omarchy watcher); theme only holds the pure state + generation. Persisting `theme=dark` on removal (if decided) goes through config [G8]

### frankmd-i18n

#### i18n::picker
- Rows = System + 7 locales with native names in source order, current marker; `select(id)` is a no-op for the current id, emits `LocaleChanged`, keeps the live switch when the write fails, falls back to en for unknown ids, reacts to `ConfigChanged{locale}` [G6]
- ⚠ "writes `locale` to the scoped config" can't happen in i18n (no config dep). Emit `Effect::WriteConfig{locale}`, and the write goes through workspace/`frankmd-config` [G6]

### frankmd-core

#### core::emoji_picker
- `EmojiPicker`: tabs + `switch_tab -> bool`, `tab_strip_key(dir)`, `columns()` 10/5/8, `move(dir)` grid wrap, `hover -> bool` without a scroll request, `cells()` {glyph / text / IconGlyph, shortcode/name, selected}, `rows()`, `footer()`, `activate[_at]() -> Insert{text, kind}`, `set_locale(loc) -> bool` from bundled `core::emoji::i18n` (empty index on missing data) [G5]
- ⚠ The "Frequently used" row is read from the state dir, which is IO, and core is IO-free. Inject the list. It is persisted via `frankmd-fs` (state file under `frankmd-config::AppDirs`), owned by workspace [G5]

### frankmd-media

#### media::sources::folder
- Image dimensions parsed from the file header (None on failure); size label formatting ("100 KB") [G2]
- ⚠ The parse must be a pure fn over bytes. The directory listing and reads are tolerant reads through `frankmd-fs`, not ad-hoc `std::fs` in media [G2]

### Not VM (test infrastructure, recorded for completeness)
- `frankmd-app` test harness: fake clipboard service, fake AI provider, IME marked-text simulation if `TestAppContext` supports it [G3]
- `frankmd-testkit`: FakePortal FileChooser, fake clipboard image, simulated external-file drag (`ExternalPaths`) [G5, G6]
- Element ids to add to vocab: `emoji-picker`, `emoji-search`, `emoji-grid`, `emoji-tab-<kind>`, `dialog-custom-theme`, `dialog-insert-image`, `upload-dropzone`, `search-result-<n>`, `quick-open-row-<n>`, `banner-<kind>-<button>` [G4, G5]

## 5. Open questions

Format: question — [groups] — ui-spec § — **Default** (FrankMD parity / drop-in compatibility unless noted).

### Explorer, `.fed` and file operations

- **OQ-1** Is `.fed` ever listed in the explorer? FrankMD showed it as a non-draggable config row with no context menu, while §5.14 hides dotfiles and §5.16 moves config editing into Settings. — [G1, G6] — §5.14, §5.16 — **Default:** never listed. If it is ever shown (e.g. as "Vault settings" → Settings › This vault), it gets no Rename/Move/Delete.
- **OQ-2** Is `.fed` still openable/copyable as a document? Many rows (slash, toggle_task, CopyContents, library, typewriter, view_mode) assume it is. — [G3] — §5.16 — **Default:** not openable as a note. Retarget those rows to "any non-markdown file"; startup refuses `.fed`.
- **OQ-3** Where does focus go after clicking a note (or pressing Enter) in the explorer? — [G1] — §4.3, §5.14 — **Default:** stays on the tree row (FrankMD). An empty note focuses the Editor.
- **OQ-4** Do ↑/↓ move focus without changing the selection (VS Code-like split, Space toggles), or does focus replace the selection as in FrankMD? — [G3] — §5.14 — **Default:** focus moves and replaces the selection (parity). Space toggles with Ctrl.
- **OQ-5** Meta-click selection: Super belongs to Hyprland. — [G3] — §6 — **Default:** drop the meta row on Linux and keep Ctrl-click.
- **OQ-6** Does Delete always confirm, including for multi-select? FrankMD's key handler deleted directly. — [G3] — §5.13 — **Default:** always confirm, with focus on Cancel.
- **OQ-7** Non-empty folder delete depends on §12 Q3 (trash). — [G1] — §5.13, arch §12 — **Default:** with trash, the confirm lists counts and the folder is trashed. Without trash, keep "Folder not empty".
- **OQ-8** Failures in batches (multi-delete, multi-item drag move): status-line error, inline error in the still-open confirm, or a banner listing what moved and what didn't? — [G5, G6] — §5.12, §5.13 — **Default:** stop at the first error and show it in the status line (parity).
- **OQ-9** Wording for the root destination in path previews (FrankMD `root/menu_note.md`; §5.13 shows `In: blog/`). — [G1] — §5.13 — **Default:** `In: /`.
- **OQ-10** While dragging a nested item, a hovered note at the vault root targets Root? — [G5] — §5.14 — **Default:** yes (note → its folder = root).
- **OQ-11** Dragging an explorer note out of the window or into the editor (FrankMD put a text/plain payload on the drag). — [G5] — §5.14 — **Default:** unsupported in v1. Drop on the editor does nothing.

### Overlays, dialogs and focus

- **OQ-12** Does a backdrop/scrim click close Settings (and Diagnostics)? The §5 intro doesn't list Settings, but FrankMD closes it. — [G1, G6, G8] — §5 intro, §5.16 — **Default:** yes, close (parity).
- **OQ-13** Does a backdrop click on the theme picker revert the live preview (like Esc) or commit it? — [G8] — §2.1, §5.17 — **Default:** revert.
- **OQ-14** What gets focus when Settings opens: the first category button (FrankMD) or the search field (§5.16)? — [G8] — §5.16 — **Default:** search field (the new design adds it, and categories stay one Tab away).
- **OQ-15** Initial focus of confirms: FrankMD focuses the dialog itself. Does Cancel-first apply to every destructive generic confirm, and where does a non-destructive confirm start? — [G2] — §5.13 — **Default:** destructive → Cancel; non-destructive → primary button.
- **OQ-16** Can a prompt stack over an open dialog (e.g. library delete inside Insert image) despite "one overlay at a time"? — [G2] — §2 — **Default:** yes. A child prompt stacks on its parent and focus returns into the parent.
- **OQ-17** Does plain Enter submit single-line dialogs (text prompt, New folder, Move), or only Ctrl+Enter? — [G2, G6] — §5, §5.13 — **Default:** both work (parity). Multi-line fields need Ctrl+Enter.
- **OQ-18** Tab inside the code-block dialog: Tab indents in the code area and accepts a suggestion in the language field. How does the user leave the code area? — [G4] — §5, §9 — **Default:** F6 / Ctrl+Tab, as in the main editor.
- **OQ-19** Tab focus order in the emoji popover (search → tab strip → grid?); Ctrl+Tab cycles tabs. — [G5] — §5.5 — **Default:** search → tab strip → grid, as listed.
- **OQ-20** Focus target when the sidebar hides (Ctrl+E; FrankMD focused the floating opener) and when Narrow→Wide removes the opener. — [G1] — §4.2 — **Default:** the Editor in both cases, or the vault header if the sidebar becomes docked while it had focus.
- **OQ-21** Is there a pane header with no note open (open-sidebar button, `⋯`), and does the Document menu exist in the empty state? How does the user reopen a hidden sidebar with the mouse? — [G3, G7] — §3.4, §5.2 — **Default:** keep a minimal header with the sidebar toggle and a `⋯` menu, where note commands are hidden or Unavailable.
- **OQ-22** Title rename caret: at the end without selecting (FrankMD) or select the whole stem (VS Code)? — [G1] — §3.4 — **Default:** caret at the end (parity).
- **OQ-23** Is F2 (inline rename) unavailable while the media library is open? — [G3] — §5.15 — **Default:** yes (no title is shown).

### Layout, preview and scrolling

- **OQ-24** Default preview visibility for a first launch / a vault with no saved state. — [G1, G7] — §4.2 — **Default:** hidden (FrankMD).
- **OQ-25** Linked images in the preview: does a click follow the link or open the viewer, and what is the "enlarge" affordance? — [G1, G7] — §5.21 — **Default:** click follows the link. Enlarge goes through a separate focusable control and the context chip "View image" (Ctrl+.).
- **OQ-26** Typewriter recentering: does the >5 px threshold stay, is the 120 ms ease-out skipped inside the threshold, and is recentering skipped during mouse selection? — [G2] — §7.6 — **Default:** keep the threshold and the skip-during-selection rule. No animation when below the threshold.
- **OQ-27** Typewriter preview-follow while scroll sync is off. — [G7] — §4.2 — **Default:** blocked (parity).
- **OQ-28** Quick open below 900 px has no preview column. Is the preview read still issued? — [G5] — §5.2 — **Default:** no. Skip `ReadPreview` when the column is hidden.

### Quick open, search, recents, menus

- **OQ-29** Does reopening quick open / search-in-files clear the query and results (FrankMD) or keep the last query (Zed/VS Code)? — [G5] — §5.2, §5.3 — **Default:** clear.
- **OQ-30** Is any "Searching…" text shown in the status row while debouncing, or only the bar? — [G5] — §5.3 — **Default:** bar only, with `searching()` still exposed for the a11y label.
- **OQ-31** Recents wording: the "N minutes ago" of `recent_files_helper`, or the shorter "2 h ago" of §5.2? Does a malformed `opened_at` show "Unknown time" or nothing? — [G1] — §5.2 — **Default:** keep the ported helper strings, with "Unknown time" (existing locale strings carry over).
- **OQ-32** Empty-state recents: does it still collapse at 5 items with "Show more/less", or list up to 50 flat? — [G6] — §5.2 — **Default:** follow §5.2, no collapse.
- **OQ-33** Back/forward history scope: session-only? Does it skip notes deleted since? — [G1] — §6.1 — **Default:** session-only, skipping deleted notes. Back past the first entry → empty state.
- **OQ-34** Slash menu row count: §5.5 says 12, but FE-089 (10) + FE-090 (4) = 14, and the original test expects 14. — [G2] — §5.5 — **Default:** 14. Fix the spec.
- **OQ-35** Does the 8 × 28 px popover cap also apply to wikilink and link completion? FrankMD let those grow to 16em. — [G2] — §5.5 — **Default:** yes, one cap for all completion popovers.
- **OQ-36** Slash insertion whose range went stale: drop it (FrankMD), insert at the caret, or show a status message? — [G3] — §5.5 — **Default:** drop it, plus an info status message.

### Editing, undo, autosave, drafts

- **OQ-37** Does the content-loss banner have a Dismiss (×)? §5.12 lists only Undo / Save anyway, and FrankMD had `dismissContentLossWarning`. — [G4] — §5.12 — **Default:** add Dismiss (parity). The banner also clears on switch/close/restore.
- **OQ-38** Should switching notes force a disk save, or only flush the draft (FrankMD)? — [G4] — §10.1 — **Default:** flush the draft only (parity, no surprise writes).
- **OQ-39** "Dirty doc without a base revision" has no native trigger. Remove the case, or use `base_revision: Option<Revision>` for deleted-on-disk notes? — [G4] — §10.1 — **Default:** make it `Option` and refuse the transition when it is None.
- **OQ-40** Quit while unreachable: ask only when drafts couldn't be written to the vault (§5.23), or also when the draft store itself failed (FrankMD beforeunload)? — [G4] — §5.23 — **Default:** ask in both cases.
- **OQ-41** Changed on disk with a clean buffer: always show the banner (§5.12), or reload silently (FrankMD)? — [G4] — §5.12 — **Default:** auto-reload clean buffers and show the banner only when dirty.
- **OQ-42** Created-note undo/redo boundary entry points: is redo routed both from the editor at history end and with no note open? With no note open, the empty-state input has focus, and redo isn't routed from inputs. Which wins? — [G3, G4] — §5.2, §6.1 — **Default:** keep both entry points. Exempt the empty-state quick-open input when its query is empty.
- **OQ-43** Drafts keyed by DocId: rename may no longer re-key drafts. — [G3] — §10.1 — **Default:** assert only the observable "next save writes the new path". No question remains unless the DocId design changes.
- **OQ-44** Grammar Apply with per-hunk reject: one whole-document replace or a minimal patch (one undo step either way)? — [G3] — §5.x grammar — **Default:** a minimal patch of the accepted hunks, as one undo step.
- **OQ-45** Ctrl+M with no selection. — [G8] — §5.5 — **Default:** no-op (parity).
- **OQ-46** Right-click with a selection in a non-markdown file: plain context menu or nothing? — [G8] — §5.5 — **Default:** plain context menu.
- **OQ-47** Stats: drop the "skip when length unchanged" quirk? Keep the 500 ms debounce, or update every frame? — [G8] — §11 — **Default:** fix the quirk (key on revision) and keep 500 ms.

### Vim

- **OQ-48** Does vim mode suppress the Ctrl+Q Quit binding (so `<C-q>` stays blockwise visual)? Does `:q` close the note when no overlay is open? — [G2] — §6.1, §6.3 — **Default:** vim owns Ctrl+Q in the editor. `:q` closes overlays and, when none is open, closes the note.

### Settings, config, themes, i18n

- **OQ-49** Settings save debounce: FrankMD used 500 ms with latest-body-wins, which lost earlier keys. — [G8] — §5.16 — **Default:** 300 ms, coalescing all keys (fixes the bug), flushed at quit.
- **OQ-50** The media provider row: debounced like the other rows, or saved immediately with uploads gated on the ack (FrankMD)? — [G7] — §5.16 — **Default:** save immediately.
- **OQ-51** Pending theme/settings write at quit: FrankMD dropped it. — [G8] — §5.23 — **Default:** flush it.
- **OQ-52** Line-number storage: `.fed` keeps 0/1/2 while Settings shows Off/Absolute/Relative? — [G2] — §5.16 — **Default:** keep numeric storage (drop-in).
- **OQ-53** `editor_line_height`: a `.fed`-only key, or a Settings › Editor row? — [G4] — §5.16 — **Default:** keep the key and honour it. Add a row only if the spec wants one.
- **OQ-54** Default editor font: §7.2 makes System monospace the default, where FrankMD's was cascadia-code. Does an absent `editor_font` in an existing `.fed` change what users see? — [G8] — §7.2 — **Default:** absent → cascadia-code when it is installed (drop-in), else System monospace.
- **OQ-55** When Omarchy disappears: persist `theme=dark` (FrankMD), or keep "Follow Omarchy" and show dark until it returns? — [G8] — §5.17 — **Default:** keep the follow setting and don't write config (avoid a surprise write). Note that this departs from parity.
- **OQ-56** Custom theme save failure: inline dialog status or a status-line message? — [G5] — §5.17 — **Default:** inline "Save failed", with the dialog kept editable.
- **OQ-57** Custom theme editor groups: keep the "Code functions"/"Code keywords" labels so the locale strings carry over? — [G1] — §5.17 — **Default:** yes.
- **OQ-58** Language picker: does arrowing live-preview the UI and Esc revert (§2.1 picker sub-modes), or does only Enter apply (§5.1)? — [G6] — §2.1, §5.1 — **Default:** apply on Enter only (re-rendering the whole UI per arrow is jarring).
- **OQ-59** Language flags: keep FrankMD's flag emoji or show native names only? — [G6] — §5.1 — **Default:** native names only.
- **OQ-60** Vault probe cadence: a 5 s stat of the vault root (FrankMD), or only on IO errors + watcher/activation hints? — [G4] — §5.12 — **Default:** 5 s while unreachable, hints only while reachable.
- **OQ-61** Diagnostics log header `<env> - <file>`: what does it mean natively? — [G6] — §5.x diagnostics — **Default:** `FRANKMD_ENV` (or "production") plus the log file name.

### Media

- **OQ-62** URL written for inserted `images_path` images: FrankMD wrote the web route `/images/preview/photo.png`. What does the native app write, and how does the preview resolve old `/images/preview/…` URLs? — [G1] — §5.6, §5.21 — **Default:** write the same form (drop-in) and map `/images/preview/…` to `images_path` in the preview resolver.
- **OQ-63** Folder source without a FileChooser portal (non-portal WMs): a gpui path prompt or a "Set up" tag? — [G2] — §5.6 — **Default:** gpui path prompt fallback.
- **OQ-64** Web/Google/Pinterest thumbnails now go through `frankmd-net`. Is a failed fetch treated as broken (card removed) or retried? — [G2] — §5.6 — **Default:** removed (parity), no retry.
- **OQ-65** Dimension badge: "800x600" (ASCII) or "800×600"? — [G2] — §5.6 — **Default:** "800x600".
- **OQ-66** Image viewer sources: accept absolute paths inside the vault, in addition to vault-relative ones? `file://` is rejected. — [G6] — §5.21 — **Default:** accept them after a `contain()` check. Reject everything else.
- **OQ-67** Does the storage fallback banner ever auto-dismiss? The old dialog closed after 6 s. — [G7] — §5.12 — **Default:** never. Dismiss is sticky for the session.
- **OQ-68** Storage-row resize: S3-only with default 0.5 (FrankMD), or also local with default "none" (the §5.6 mock)? — [G7] — §5.6 — **Default:** S3-only, 0.5.
- **OQ-69** Per-dialog storage row reset target: the global "Storage for new media" value or always Local (FrankMD)? — [G7] — §5.6 — **Default:** the global value (the setting exists for that purpose).
- **OQ-70** Does inserting from the media library close it and focus the editor? — [G3] — §5.15 — **Default:** yes (parity).
- **OQ-71** Emoji picker: does it always reopen on the Emoji tab? Does the Frequently used row take part in grid navigation/wrap and column count? Should hover share `ListCursor` keyboard-mode semantics, which FrankMD lacked? — [G5] — §5.5 — **Default:** reset to the Emoji tab; Frequently used is a normal navigable row; hover follows ListCursor semantics.

### Plan bookkeeping

- **OQ-72** menu_bounds `it.each` (2 rows) is counted as 1 test in test-contracts (C = 6), but the part-3 rule expands `it.each` (C = 7). — [G2] — n/a — **Default:** follow test-contracts (one row with `[rows 1–2]`) and note the rule exception.

## 6. G1: Ruby helpers, controllers and system tests

The a/b/c tallies in each header count rows by their **primary** level, which is the first token of the Level cell. Dropped rows count in none of them.

### `test/helpers/evaluate_script_convention_test.rb` — C=1 (keep 0 / changed 0 / dropped 1; a=0 b=0 c=0)
Original subject: a lint over `test/system/*.rb` that catches multi-statement `evaluate_script` heredocs, because Selenium wraps the script in `return (…)`. New home: none. This is Capybara-suite hygiene with no product behaviour behind it.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 16 "multi-statement evaluate_script bodies are wrapped in an IIFE" | None; it protects the browser test suite from a Selenium wrapping quirk | — | Optional replacement, not counted here: a lint that every `find("<id>")` in `crates/frankmd-app/tests/*.rs` names an id from the central element-id list | — | dropped (browser-only: Selenium `return (SCRIPT)` wrapping has no native analogue) |

### `test/helpers/tree_helper_test.rb` — C=11 (keep 9 / changed 1 / dropped 1; a=10 b=0 c=0)
Original subject: `render_tree_items` server-renders explorer rows (data attributes, classes, Stimulus actions). New home: `workspace::explorer` row model rendered by the gpui-component Tree. Sidebar §3.3, explorer interactions §5.14, empty states §5.22.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 8 "renders empty state when no items" | An empty vault shows "No notes yet" in the tree | Explorer empty message §5.22 "Vault empty" | G: vault with no notes / W: build `ExplorerState::rows()` / T: no rows; `empty_message() == Some(i18n sidebar.no_notes_yet)` | a:frankmd-workspace::explorer | keep |
| 2 | 14 "renders file item with correct attributes" | A note row carries its path, kind = markdown file and display name (stem), and can be renamed from the keyboard | Explorer row §3.3, §5.14 | G: `test.md` / W: rows() / T: row {path "test.md", kind File(Markdown), label "test"}; `row_actions(row)` contains RenameInline (F2) | a:frankmd-workspace::explorer | keep |
| 3 | 25 "renders selected file with selected class" | The selected or open note is marked | Selection + open-note accent bar §3.3 | G: `test.md` open and selected / W: rows() / T: row.selected && row.is_open; other rows not selected | a:frankmd-workspace::explorer | keep |
| 4 | 32 "renders folders as focusable explorer items with selection and keyboard actions" | Folders are focusable and selectable; Delete and F2 act on them | Explorer keyboard §5.14 | G: folder1 focused / W: `on_key(Delete)` then `on_key(F2)` / T: Delete → `DeleteConfirm` for [folder1]; F2 → `InlineRename{path: folder1}`; focusing a row via keyboard selects it (FE-240) | a:frankmd-workspace::explorer | keep |
| 5 | 42 "renders folder with children" | A collapsed folder hides its children | Explorer tree §5.14 | G: folder1/note.md, nothing expanded / W: visible_rows() / T: rows = [folder1 (Folder, expanded=false)]; folder1/note.md absent | a:frankmd-workspace::explorer | keep |
| 6 | 56 "renders expanded folder without hidden class" | An expanded folder shows an open chevron and its children | Expansion state §5.14 | G: expansion = {folder1} / W: visible_rows() / T: folder1.expanded = true; folder1/note.md visible at depth 1 | a:frankmd-workspace::explorer | keep |
| 7 | 68 "renders config file without drag attributes" | `.fed` is selectable but cannot be dragged and has no context menu | Hidden items §5.14 (dotfiles hidden); config edited only in Settings §5.16 | G: vault root has `.fed` / W: visible_rows() / T: no `.fed` row; `drag_payload(".fed")` and `context_menu(".fed")` are None (defensive) | a:frankmd-workspace::explorer | changed (§5.14 dotfiles hidden, §5.16 Settings owns config; see open question) |
| 8 | 83 "renders regular files with drag and context menu" | A note can be dragged and has a context menu | Explorer mouse + drag §5.14 | G: `test.md` / W: `drag_payload([test.md])`, `context_menu(row)` / T: payload = [test.md]; menu ⊇ [Rename, Move to…, Delete] | a:frankmd-workspace::explorer | keep |
| 9 | 93 "renders folders with drag-drop actions" | Clicking a folder toggles it; a folder accepts drops | Explorer §5.14 | G: folder1 collapsed / W: `click(folder1)`; `drop_target(hover = folder1)` / T: folder1 expanded; drop target = folder1 | a:frankmd-workspace::explorer | keep |
| 10 | 103 "handles string keys in items" | None (Ruby Hash string vs symbol keys) | — | — (the typed `TreeNode` struct makes the case impossible) | — | dropped (Rails-specific: Ruby hash key types) |
| 11 | 111 "returns empty string for empty children at non-root depth" | An empty sub-folder shows no "No notes yet" text; only an empty root does | Explorer §5.22 | G: empty folder `empty`, expanded / W: rows(), empty_message() / T: row `empty` present, no child rows, empty_message() == None | a:frankmd-workspace::explorer | keep |

### `test/controllers/notes_controller_test.rb` — C=17 (keep 7 / changed 10 / dropped 0; a=17 b=0 c=0)
Original subject: the HTML SPA shell rendered by `NotesController#index` and `#show` (Home, workspace chrome, settings markup, bookmarkable URLs). New home: empty state §5.2, sidebar §3.3, pane header and Document menu §3.4, Settings §5.16, startup/CLI open (URL routing dropped §1), note-not-found §5.22. Only the 17 [C] tests appear here.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 16 "index renders the main page" | With no history, the landing view has no recents section | Empty state §5.2 | G: vault, empty `recent_files` / W: startup with no note / T: `EmptyStateModel.recents` is empty; no "Recently opened" header and no ClearHistory action | a:frankmd-workspace::quick_open + b | changed (§5.2 empty state replaces Home) |
| 2 | 23 "Home introduces FrankMD with find before create and concise history actions" | The landing view shows brand, find before create, and a Clear history action | Empty state §5.2, §5a | G: welcome.md recorded / W: no note open / T: model has logo, vault name, note count; quick-open input focused; recents header has "Clear history…"; chips = [NewNote, SearchInFiles, ToggleCommandPalette] with registry shortcuts; no description copy | a:frankmd-workspace::quick_open + b | changed (§5.2: shortcuts now printed and marketing copy dropped) |
| 3 | 43 "index renders the Library action and image and video workspace tabs" | A Library control exists (off by default); the library has Images (default) and Videos tabs and a grid | Dock footer §3.3; Media library §5.15 | G: fresh state / W: read layout + media VM / T: `media_library_open == false`; dock footer has MediaLibrary button; `MediaLibraryVm.tab == Images`, tabs = [Images, Videos] | a:frankmd-workspace::media_library + b | changed (§5.15 library is a main-area view) |
| 4 | 56 "workspace renders unique footer navigation and accessible header controls" | Unique, labelled chrome controls: footer nav, sidebar actions, title rename affordance, translated save labels, document menu | Sidebar §3.3, pane header §3.4, status §3.6 | G: no note / T: dock footer = [MediaLibrary, Settings, Help], each labelled with its palette name; vault header = [NewNote, NewFolder]; pane title disabled; `document_menu()` groups per §3.4 with a shortcut per row; SaveState::Unsaved label = i18n status.unsaved; every icon button has a non-empty label | a:frankmd-workspace::commands + b | changed (§3.3 Home/brand removed; §3.4 pins replaced by fixed buttons) |
| 5 | 104 "General settings renders the shared storage menu" | A storage select with Local (default) and S3, plus an inline error slot | Settings › Media §5.16 | G: no S3 creds / W: `SettingsVm::row("media_storage_provider")` / T: kind Select [Local ✓, S3 disabled(reason)]; a failed set shows an inline error under the row and keeps Local | a:frankmd-workspace::settings_vm | changed (§5.16 moved to the Media category) |
| 6 | 116 "About is a Settings section without standalone controls or dialog" | About lives only inside Settings and shows name, description, version, author, and a GitHub link opening externally | Settings › About §5.16 | G: Settings open / W: select About / T: rows: logo, "FrankMD", description (i18n), version `vN.N`, copyright with "Fabio Akita", Source code link → OpenUri(github…/FrankMD); no `OverlayKind::About` exists | a:frankmd-workspace::settings_vm | keep |
| 7 | 131 "workspace labels omit shortcuts while tooltips retain them in every locale" | Menu and accessible labels never embed "(Ctrl+…)"; shortcuts stay discoverable | Registry §5.1, principle §1.3, §8.1 | for each of the 7 locales: every `CommandRegistry` name and Document-menu label ∌ `(Ctrl+` / `(F\d`; TogglePreview, Find, EmojiPicker, ToggleHelp each have a non-empty shortcut field | a:frankmd-workspace::commands | changed (§1.3 shortcuts printed as kbd chips, not tooltips) |
| 8 | 146 "image picker has an inline alert target for folder browsing errors" | Folder-source errors show inline in the picker | Insert image §5.6 | G: Folder source, chooser errors / W: `set_error(Folder, msg)` / T: `source_state(Folder).status == Error(msg)`, shown inline above the footer; no modal alert | a:frankmd-workspace::insert_image | keep |
| 9 | 153 "index has empty initial-path-value and empty initial-note-value" | Launching with no target opens no note | Startup, empty state §5.2 | G: no CLI path / W: `startup::resolve(args)` / T: `StartupTarget::None` → UiState has no open note and shows the empty state | a:frankmd-workspace::startup | changed (§1 URL routing → CLI) |
| 10 | 172 "index includes tree data in rendered HTML" | The tree is populated at first paint | Explorer §3.3 | G: test.md / W: startup / T: explorer rows contain test.md (File) before any user action | a:frankmd-workspace::explorer | keep |
| 11 | 198 "tree accepts expanded and selected params" | Restored expansion and selection render expanded and selected rows | Expansion persisted per vault §5.14 | G: persisted expansion {folder1}, selected folder1/note1.md / W: restore + rows() / T: folder1 expanded, note1 selected | a:frankmd-workspace::explorer | keep |
| 12 | 652 "show with HTML request renders SPA with initial note data" | Opening a note by path at launch shows its content; revision is the SHA-256 | Startup / navigation | G: bookmarked.md / W: launch `frankmd <vault>/bookmarked.md` / T: open note path "bookmarked.md", buffer "# Bookmarked Content", `revision == sha256(content)`, exists | a:frankmd-workspace::startup | changed (§1 CLI replaces bookmarkable URLs) |
| 13 | 683 "show with HTML request embeds content with special characters correctly" | Content with `"` `&` `<>` `\` loads byte-exact | Startup / navigation | G: special.md with those characters / W: startup open / T: buffer == file bytes | a:frankmd-workspace::startup | keep |
| 14 | 699 "show with HTML request for nested path renders SPA" | A nested Hugo bundle path opens | Startup; breadcrumb §3.4 | G: 2026/01/30/my-post/index.md / W: startup open / T: open path matches; breadcrumb = [2026, 01, 30, my-post], title "index" | a:frankmd-workspace::startup | keep |
| 15 | 711 "show with HTML request for missing file renders SPA with error state" | A missing target tells the user and creates nothing | Note not found §5.22 | G: no nonexistent/file.md / W: startup open it / T: status message "No note named ‘file’. Ctrl+N creates it."; overlay QuickOpen with query "file"; no file created | a:frankmd-workspace::navigation + b | changed (§5.22 replaces the in-page error) |
| 16 | 724 "index with file query param loads initial note" | `?file=` preloads a note | CLI path argument | G: from_param.md / W: `startup::resolve(["from_param.md"])` (relative to vault) / T: note opened, buffer "# From Param"; `.fed` as target → refused (A-covered) | a:frankmd-workspace::startup | changed (§1 CLI replaces query param) |
| 17 | 734 "index renders settings dialog with editor config values" | Settings shows the current font, font size and editor width | Settings › Editor §5.16 | G: config editor_font, editor_font_size, editor_width / W: SettingsVm::new / T: those rows' values equal the config, with provenance chips | a:frankmd-workspace::settings_vm | keep |

### `test/controllers/recent_files_controller_test.rb` — C=2 (keep 1 / changed 1 / dropped 0; a=2 b=0 c=0)
Original subject: the Home recent-files panel HTML (5 visible plus Show more/less, time labels). New home: the recents section of quick open and the empty state §5.2.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 35 "recent files initially show five notes with a button only when more are available" | No section when empty; recents listed newest first; extra entries are not lost | Quick open empty query §5.2 | G: record 0, 5, 6 and 10 notes in turn / W: `QuickOpenModel::rows("")` / T: 0 → no recents header; n → first n rows are the recents, newest first (cap 10), then other notes by mtime; no Show more | a:frankmd-workspace::quick_open | changed (§5.2 recents first in one list, no 5-row paging) |
| 2 | 57 "opening age and machine-readable timestamp render with a fallback for malformed dates" | Each recent shows its stem and a relative open time; a malformed timestamp falls back to "Unknown time" | Quick open recent rows §5.2 | G: injected clock; timed.md opened 10 min ago / W: rows("") / T: row label "timed", time "10 minutes ago" (ported recent_files_helper labels); G: opened_at "broken" / T: time "Unknown time", no panic | a:frankmd-workspace::quick_open | keep |

### `test/system/creation_destination_test.rb` — C=3 (keep 3 / changed 0 / dropped 0; a=3 b=0 c=0)
Original subject: New note/folder destination from the explorer selection, Hugo path preview, #268 reveal. New home: New note/New folder dialogs §5.13; explorer §5.14.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "toolbar creation uses the selected folder and previews Hugo posts in blog" | The selected folder is the destination; Hugo previews `blog/YYYY/MM/DD/slug/index.md`; switching back restores the folder path; New folder previews and creates inside the selection | NewNoteForm / NewFolderForm §5.13 | G: docs/nested selected, clock fixed / W: NewNote (vault header), name "Sample", toggle Empty→Hugo→Empty, Create; then NewFolder "child" / T: preview docs/nested/Sample.md → blog/2026/11/04/sample/index.md → docs/nested/Sample.md; file created; folder form "In: docs/nested", creates docs/nested/child; b: vault-header New note opens `dialog-new-note` | a:frankmd-workspace::dialogs + b | keep |
| 2 | 35 "creating inside a collapsed selected folder opens the folder (#268)" | A selected but collapsed folder is still the destination; after create it is expanded so the new item shows | Explorer reveal §5.14 + dialogs | G: docs selected and collapsed / W: create note "Sample", then (re-collapse) create folder "child" / T: destination docs/; after each create, docs is in the expansion set and the new row is visible and revealed | a:frankmd-workspace::explorer | keep |
| 3 | 71 "flat Hugo posts use blog from the toolbar and preserve explicit folder destinations" | Flat style: Hugo from the global New note → blog/my-post.md, ignoring the selection; New note from a folder's context menu → that folder | NewNoteForm origin §5.13 | G: hugo_path_style=flat, docs selected / W: `NewNoteForm::new(origin: Global)` Hugo "My Post"; then `origin: FolderContext(docs)` Hugo "Explicit Post" / T: blog/my-post.md (not docs/ or root); docs/explicit-post.md (not blog/) | a:frankmd-workspace::dialogs | keep |

### `test/system/custom_theme_editor_test.rb` — C=2 (keep 2 / changed 0 / dropped 0; a=2 b=0 c=0)
Original subject: the custom theme editor dialog (fields, contrast, live CSS vars, save and revert). New home: custom theme editor §5.17 (`theme::custom`), opened from Settings › Appearance "Customize theme…".

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "creates a theme with live preview and persists it without modifying the note" | Name/Accent/Background/Text edit live; a low-contrast warning shows; syntax defaults exist; Save persists theme=custom and closes; the open note is untouched; reopen + edit + Esc reverts live colours and refocuses the opener | Custom theme editor §5.17 | G: theme dark, note open / W: CustomThemeVm set name, accent #5b8cff, bg #101010, text #ffffff / T: live tokens accent=#5b8cff; contrast warning on the text/bg pair; Code group "functions" = #f0e68c; keywords edit → token syntax-keyword; save → config theme=custom, custom_theme.colors.accent=#5b8cff; note bytes unchanged; reopen, accent #ff0000, cancel → live accent #5b8cff; b: Esc closes `dialog-custom-theme`, focus back on "Customize…" | a:frankmd-theme::custom + b | keep |
| 2 | 50 "mobile editor keeps controls reachable and validates colors" | An invalid hex disables Save with a "six-digit hex" message; valid input re-enables it; the dialog fits a small window; no missing translations; Esc without saving persists nothing | §5.17, overlay anatomy §5 (width min(spec, window−32)) | G: VM / W: accent "#bad" / T: field error "six-digit hex", can_save=false; "#112233" → true; comments "#aabbcc" ok; cancel → config custom_theme None; b: window 390×844 → dialog bounds within the window; i18n coverage handled by HP-022 test | a:frankmd-theme::custom + b | keep |

### `test/system/draft_recovery_test.rb` — C=5 (keep 3 / changed 2 / dropped 0; a=5 b=0 c=0)
Original subject: localStorage drafts (`frankmd:draft:<path>`) surviving switches, reloads, renames, moves and folder ops. New home: drafts in the state dir (`workspace::drafts`), in-app back/forward §6.1, explorer ops §5.13–5.14. Every row uses an injected Clock with debounces set to "never fire".

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "preserves the outgoing draft when switching files before autosave debounce" | Switching notes before autosave keeps the edit as a draft; reopening shows it; disk unchanged; back/forward walk notes and restore drafts; going back past the first note shows the empty state; an empty-string draft is respected | Navigation history §6.1 + drafts | G: a.md, b.md / W: open a, edit "# Local draft A", open b, open a, NavigateBack ×3, NavigateForward, set "" , open b, open a / T: draft(a)="# Local draft A" after the switch; buffer shows it; disk "# Server A"; back → b, a, then empty state; forward → a with draft; final buffer "" | a:frankmd-workspace::navigation | changed (§6.1 in-app back/forward replaces browser history; drafts move to the state dir) |
| 2 | 56 "restores an online unsaved draft after a hard reload" | After restart the unsaved draft is restored silently and marked Unsaved; disk unchanged | Drafts + startup + status §3.6 (FE-140) | G: draft.md, edit "# Local draft", draft flushed on switch/quit / W: new Workspace on the same state dir / T: buffer "# Local draft", SaveState::Unsaved, disk "# Original note", no Recovery dialog (base revision matches) | a:frankmd-workspace::drafts | changed (page reload → app restart) |
| 3 | 83 "moves an active dirty draft with a renamed file" | Renaming a note with a dirty draft moves the draft to the new path and removes the old one | Inline rename §5.13 + drafts | G: before.md dirty "# Local edit" / W: rename to "after" / T: open note after.md; draft(after.md)="# Local edit"; draft(before.md)=None; disk after.md="# Server version" | a:frankmd-workspace::drafts | keep |
| 4 | 120 "moves an active dirty draft to the destination folder" | Moving the open dirty note by drag and drop moves its draft | Explorer drop §5.14 + drafts | G: before.md dirty, folder target / W: `explorer.drop([before.md], target)` / T: open note target/before.md; draft moved; old key gone; disk content unchanged | a:frankmd-workspace::drafts | keep |
| 5 | 155 "remaps nested drafts and cleans them after deleting a folder" | Renaming a folder remaps nested drafts; an unrelated `docs-old/` draft is untouched (prefix includes `/`); deleting the note, then the emptied folder, clears matching drafts | Drafts remap/cleanup | G: docs/child.md dirty, draft docs-old/keep.md / W: rename docs→archive; delete archive/child.md; delete archive / T: draft archive/child.md exists, docs/child.md gone, docs-old/keep.md kept; after deletes no archive/* drafts, docs-old kept | a:frankmd-workspace::drafts | keep |

### `test/system/explorer_background_test.rb` — C=2 (keep 0 / changed 2 / dropped 0; a=2 b=0 c=0)
Original subject: a left-click on empty tree space clears the selection and opens the explorer menu (#261/#267). New home: explorer background behaviour §5.14 (left-click clears only; right-click opens the menu: New note, New folder, Select all, Collapse all).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "clicking the explorer's empty area deselects and offers the explorer menu (#261, #267)" | A background click clears the selection, so new notes target the root; the menu dismisses on outside interaction; Select All selects every visible row and closes the menu | Explorer background §5.14 | G: first.md selected / W: `click_background()` / T: selection empty, no menu, NewNoteForm destination root (preview "note-name.md"); W: `context_menu_background()` / T: items [NewNote, NewFolder, SelectAll, CollapseAll]; SelectAll → 2 selected, menu closed; b: right-click empty `explorer-tree` area shows the menu, clicking `editor` dismisses it | a:frankmd-workspace::explorer + b | changed (§5.14 left-click no longer opens a menu; Find File moved to the empty state/palette) |
| 2 | 46 "the explorer menu creates notes at the root and Escape dismisses it (#267)" | The background menu's New Note creates at the root; Esc dismisses the menu | §5.14, Esc chain §6.1 | G: empty vault / W: background menu → NewNote, name "menu_note", Create / T: preview at root ("menu_note.md"), file created; W: open menu again, Escape / T: `escape_target == Popover` then menu closed | a:frankmd-workspace::explorer + b | changed (§5.14 menu opens on right-click) |

### `test/system/home_recent_files_test.rb` — C=4 (keep 0 / changed 4 / dropped 0; a=4 b=0 c=0)
Original subject: the Home "Recently opened" list (appearance, clear history, show more/less, shared across visits, deleted entries, mobile). New home: empty state with docked quick open §5.2; note not found §5.22; narrow layout §4.1; Ctrl+W close §5.23.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "the recent section appears after opening a note and disappears after clearing history" | Recents appear after an open; Clear history asks for confirmation; once cleared they stay gone after restart | Empty state recents §5.2 + PromptQueue | G: first.md / W: open, CloseNote / T: recents = [first.md]; W: ClearHistory / T: PromptQueue confirm (text mentions every device); confirm → recents empty, header gone; new Workspace on the same vault → still empty | a:frankmd-workspace::quick_open | changed (§5.2 Home → empty state) |
| 2 | 31 "Home shows five recent notes and reveals the rest with Show more" | Every recent is reachable; focus lands sensibly; activating an entry opens the note and its chrome | Quick open list §5.2, lists §5 | G: 7 recents / W: rows("") / T: all 7 recents first, newest first, each with a time; no Show more/less; Down from the input wraps (§5 list rule); Enter on note-0 → note open, pane header present; b: Enter in `quick-open-input` opens and focuses `editor` | a:frankmd-workspace::quick_open + b | changed (§5.2 no 5-row paging) |
| 3 | 62 "Home shares successful opens across visits and filters external deletions" | Opens persist newest first; externally deleted notes are filtered out; activating a stale entry says "Note not found", never recreates the file, and drops the entry | Recents + note not found §5.22 | G: open docs/first.md then second.md / T: recents [second, docs/first]; delete docs/first.md on disk → refreshed rows omit it; delete second.md after rendering, activate it → status "No note named ‘second’…", quick open prefilled, file not created, entry pruned | a:frankmd-workspace::quick_open + b | changed (§5.22 message surface) |
| 4 | 96 "Home remains reachable with Explorer hidden on mobile" | At phone width the landing list stays reachable and nothing overflows horizontally | Narrow layout §4.1, CloseNote §5.23 | G: width 390, mobile.md open / W: CloseNote (Ctrl+W) / T: empty state, recents contain mobile.md; `resolve_panes(390)` → single column, no pane wider than the window; b: `simulate_window_resize(390,844)`, ctrl-w → `empty-state` visible | a:frankmd-workspace::layout + b | changed (§4.1 drawer has no Home button; §5.23 Ctrl+W) |

### `test/system/markdown_surround_test.rb` — C=2 (keep 2 / changed 0 / dropped 0; a=2 b=0 c=0)
Original subject: typing a marker over a selection surrounds it in CodeMirror (FE-059) and Vim keeps its meaning. New home: editor §6.2 "Surround selection by typing", vim §6.3.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "typed markers preserve selection and repeat to bold in the real editor" | `*` wraps the selection and keeps the inner text selected; a second `*` makes bold; Ctrl+Z undoes one level | `markdown::edit` surround §6.2 | G: "This is a sample of a text", selection 0..26 / W: `surround_on_type('*')` ×2, then undo / T: "*…*" with inner selected; "**…**" with inner selected; undo → "*…*" (one undo step per keystroke); b: select-all + `window.input("*")` → editor value wrapped | a:frankmd-markdown::edit + b | keep |
| 2 | 22 "native Vim visual commands retain their meaning" | In Vim mode, `gg0vw*` acts as Vim (`*` search) and the text is unchanged | `vim::keymap` §6.3 | G: vim on, "sample text sample" / W: feed keys g g 0 v w * / T: text unchanged; the `*` key resolves to a Vim search, not surround; surround is active only in insert mode | a:frankmd-vim::keymap | keep |

### `test/system/multi_drag_test.rb` — C=2 (keep 2 / changed 0 / dropped 0; a=2 b=0 c=0)
Original subject: drag-drop of multi-selections (#266). New home: explorer drag & drop §5.14 (multi-drag count badge).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "dragging a selected group moves every selected item to the destination (#266)" | Dragging one member of a Ctrl-click selection moves the whole selection; unselected notes stay | `ExplorerState` drag payload §5.14 | G: first, second (ctrl-selected), rooted, folder target / W: `drag_start(first)` → payload; `drop(target)` / T: payload = [first, second] (badge 2); after the move target/first.md and target/second.md exist, rooted.md remains; b: `drag_to(explorer-row-first.md, explorer-row-target)` | a:frankmd-workspace::explorer + b | keep |
| 2 | 47 "dragging an unselected item moves only that item (#266)" | Dragging an unselected row moves only that row | §5.14 | G: bystander selected / W: drag_start(solo), drop(target) / T: payload = [solo]; bystander stays | a:frankmd-workspace::explorer | keep |

### `test/system/notes_test.rb` — C=41 (keep 31 / changed 10 / dropped 0; a=40 b=0 c=1)
Original subject: the app shell, explorer, file ops, table editor and image picker. The last 7 rows are `NotesWithImagesTest` (IMAGES_PATH set). New home: empty state §5.2, explorer §5.14, dialogs §5.13, preview §4.2/§5.21, Settings §5.16/§5.17, pane header §3.4, table editor §5.8, insert image §5.6.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "visiting the home page shows the app" | With no note, the document header is inert, the brand shows, and the tree is present | Empty state §5.2 | G: no note / T: pane title not editable (RenameNote unavailable), empty state with logo + vault name, explorer present; b: `empty-state` and `explorer-tree` visible | a:frankmd-workspace::quick_open + b | changed (§5.2) |
| 2 | 16 "home hides the editor placeholder and shows its own introduction" | No "Select a note to edit" placeholder; the landing view has its own content | Empty state §5.2 | G: no note / T: EmptyStateModel has the docked quick open and chips; no placeholder string; no description copy | a:frankmd-workspace::quick_open | changed (§5.2 no marketing copy) |
| 3 | 23 "existing notes appear in the file tree" | Notes are listed by stem | Explorer §3.3 | G: my_note.md / T: row label "my_note" | a:frankmd-workspace::explorer | keep |
| 4 | 33 "clicking a note opens it in the editor" | A click opens the note, the title shows its stem, the row is selected and focused, and the editor does not steal focus | Explorer mouse §5.14, focus §4.3 | G: test.md / W: `click_row(test.md)` / T: open note test.md, pane title "test", row selected+focused, `focus == Sidebar`; b: click `explorer-row-test.md` → that row focused, `editor` not focused | a:frankmd-workspace::navigation + b | keep |
| 5 | 46 "creating a new note via dialog" | New note opens directly with Empty preselected and creates the file | New note §5.13 | G: vault / W: NewNote, name "brand_new_note", Create / T: form.template == Empty; brand_new_note.md created and row present; b: ctrl-n → `dialog-new-note` visible | a:frankmd-workspace::dialogs + b | keep |
| 6 | 76 "creating a new hugo post via dialog" | A Hugo post goes to blog/YYYY/MM/DD/slug/index.md, not the root date path | §5.13 | G: clock 2026-11-04 / W: Hugo, "Hello World", Create / T: blog/2026/11/04/hello-world/index.md exists; 2026/11/04/hello-world/index.md does not | a:frankmd-workspace::dialogs | keep |
| 7 | 106 "new note dialog template card titles do not clip in longer locales" | Long Spanish template labels wrap, never truncate (#151) | Segmented control §5.13, §8.2 buttons size to content | Visual: locale es, open New note, check "Publicación de Blog Hugo" fully visible at 480 px | c | keep |
| 8 | 127 "creating a new folder via dialog" | New folder creates the directory | New folder §5.13 | G: vault / W: NewFolder "new_folder", Create / T: dir exists, row present; b: ctrl-shift-n → `dialog-new-folder` | a:frankmd-workspace::dialogs + b | keep |
| 9 | 153 "editing a note updates the textarea" | Opening a note loads its content | Navigation | G: editable.md "Original content" / W: open / T: buffer == "Original content" | a:frankmd-workspace::navigation | keep |
| 10 | 164 "toggle preview panel" | Preview starts hidden; the toggle shows it; the markdown renders (h1, strong) | Toggle preview §4.2, §5.21 | G: fresh vault state / T: preview_visible false; W: TogglePreview / T: true; preview model of the note has Heading(1,"Preview Test") and Strong("bold"); b: ctrl-shift-v → `preview-pane` visible | a:frankmd-workspace::layout + b | keep |
| 11 | 191 "folders can be expanded and collapsed" | Clicking a folder toggles its children | Explorer §5.14 | G: my_folder/nested_note.md / W: click_row(my_folder) ×2 / T: nested visible, then hidden; b: click `explorer-row-my_folder` | a:frankmd-workspace::explorer + b | keep |
| 12 | 213 "theme picker can change theme" | Opening Settings leaves preview state alone; changing the theme updates the current-theme label; Settings closes via ×, backdrop and Esc; the preview stays on | Settings §5.16, theme picker §5.17 | G: note open, preview on / W: OpenSettings; Appearance Theme select Light↔Dark; close via ×, then Esc (backdrop: see open question) / T: theme row label changes; `preview_visible` stays true through every step; `escape_target == Overlay(Settings)`; b: ctrl-, opens `settings-sheet`, escape closes, `preview-pane` still visible | a:frankmd-workspace::settings_vm + b | changed (§5.16/§5.17 Select row + palette sub-mode) |
| 13 | 272 "context menu appears on right-click" | Right-clicking a note shows Rename and Delete | Explorer context menu §5.14 | G: context_test.md / W: context_menu(row) / T: ⊇ [Rename, Delete]; b: right-click row → menu visible | a:frankmd-workspace::explorer + b | keep |
| 14 | 288 "renaming a note via context menu" | Rename from the menu renames on disk and appends .md | Inline explorer rename §5.13 | G: old_name.md / W: menu Rename → `InlineRename` (initial "old_name"), type "new_name", Enter / T: new_name.md exists, old_name.md gone, row renamed | a:frankmd-workspace::rename | changed (§5.13 inline rename replaces the dialog) |
| 15 | 316 "renames the selected Explorer note with F2" | F2 on a selected note renames it | §5.13, §5.14 keyboard | G: keyboard_rename.md focused / W: F2, "renamed_note", Enter / T: renamed_note.md on disk; b: press f2 on the focused row → inline input focused | a:frankmd-workspace::rename + b | changed (§5.13 inline) |
| 16 | 335 "renames the selected Explorer folder with F2" | F2 renames a folder and keeps its children | §5.13 | G: keyboard_rename_folder/nested.md / W: F2 on the folder, "renamed_folder", Enter / T: renamed_folder/nested.md exists, old dir gone, expansion remapped | a:frankmd-workspace::rename | changed (§5.13 inline) |
| 17 | 355 "deleting a note via context menu" | Delete asks for confirmation, then removes the note | Delete confirm §5.13 | G: to_delete.md / W: menu Delete / T: DeleteConfirm text "Move ‘to_delete’ to the trash?", focus on Cancel; confirm → note gone from the vault and the tree | a:frankmd-workspace::dialogs + b | changed (§5.13 trash wording, §12 Q3) |
| 18 | 373 "Ctrl selection deletes multiple notes from the context menu" | Ctrl-click multi-selects without changing the open note; deleting reports the count and removes both | §5.14 mouse, §5.13 delete counts | G: first_selected open / W: ctrl_click(second_selected); context menu on first → Delete / T: both selected, open note still first_selected; confirm text lists "2 items"; both deleted | a:frankmd-workspace::explorer + b | keep |
| 19 | 406 "deletes the focused Explorer note with Delete after confirmation" | Delete on a focused row deletes after confirmation | §5.14 keyboard | G: keyboard_delete.md focused / W: Delete key, confirm / T: deleted; b: press "delete" with the row focused → `dialog-delete` | a:frankmd-workspace::explorer + b | keep |
| 20 | 422 "selecting a folder keeps expansion and Delete rejects nonempty folders" | Clicking a folder selects and expands it; deleting a non-empty folder never loses its contents silently | §5.13 delete, §12 Q3 | G: keyboard_folder/nested.md / W: click folder; Delete; confirm / T: folder selected+expanded; with trash: the confirm lists "1 note" and the folder goes to the trash; without trash: alert "Folder not empty", nested.md kept | a:frankmd-workspace::dialogs | changed (§5.13 trash-based delete; see open question) |
| 21 | 441 "Delete in the editor removes text instead of deleting the selected note" | The Delete key in the editor edits text and never deletes the note | Keymap contexts §6.2 | G: editor_delete.md "abc", row selected, editor focused / W: Home, Delete / T: buffer "bc", file exists, no DeleteConfirm; registry: DeleteNote has no `delete` binding in the Editor context | a:frankmd-workspace::commands + b | keep |
| 22 | 457 "toolbar appears when editing a note" | Opening a note shows the header controls; the overflow menu offers the table editor | Pane header + Document menu §3.4 | G: note open / T: header buttons [InsertImage, Find, TogglePreview, DocumentMenu]; document_menu Insert group contains "Table…"; b: click `pane-menu` → menu visible | a:frankmd-workspace::commands + b | changed (§3.4 fixed header buttons) |
| 23 | 469 "table editor dialog opens and shows grid" | The table editor opens with a 3×3 default grid | Table editor §5.8 | G: note, caret not in a table / W: InsertTable / T: TableEditorVm 3 cols × 3 rows (9 cells), header row first; b: Document menu → `dialog-table` | a:frankmd-workspace::table_editor + b | keep |
| 24 | 484 "table editor can add and remove columns" | Add/remove column changes 9→12→9 cells | Columns stepper §5.8 | G: default grid / W: add_column, remove_column / T: 12 then 9 cells | a:frankmd-workspace::table_editor | keep |
| 25 | 505 "table editor can add and remove rows" | Add/remove row changes 9→12→9 cells | Rows stepper §5.8 | G: default / W: add_row, remove_row / T: 12 then 9 | a:frankmd-workspace::table_editor | keep |
| 26 | 526 "inserting a table adds markdown to content" | Insert writes a markdown table with the typed header and a separator row | §5.8 + `markdown::table` serialize | G: cell(0,0)="Name" / W: confirm / T: buffer gains a markdown table whose header row starts with cell "Name" followed by a dashes separator row; one undoable edit; dialog closed | a:frankmd-workspace::table_editor | keep |
| 27 | 554 "editing existing table in content" | With the caret inside a table, the editor loads it | Ctrl+. context action §5.8, §3.6 chip | G: 2-column table Col1/Col2 with one body row A/B, caret in it / W: ContextAction / T: context chip "Edit table"; VM cells[0]=["Col1","Col2"]; confirm label "Update" | a:frankmd-workspace::table_editor | keep |
| 28 | 593 "table cell context menu appears on right-click" | The cell menu offers Move Left/Right, Delete Column, Move Up/Down, Delete Row | Cell menu §5.8 | G: default grid, cell (0,0) / W: cell_menu(0,0) / T: items [MoveLeft, MoveRight, DeleteColumn, MoveUp, MoveDown, DeleteRow], each with its shortcut | a:frankmd-workspace::table_editor + b | keep |
| 29 | 617 "move column right via context menu" | Move Right swaps the column with its neighbour | §5.8 (Alt+→) | G: headers "Header 1","Header 2" / W: move_column_right(0) / T: headers "Header 2","Header 1" | a:frankmd-workspace::table_editor | keep |
| 30 | 647 "move row down via context menu" | Move Down moves the row | §5.8 (Alt+↓) | G: cell(1,0)="Row2Col1" / W: move_row_down(1) / T: cell(2,0)=="Row2Col1" | a:frankmd-workspace::table_editor | keep |
| 31 | 678 "delete column via context menu" | Delete Column leaves 6 cells | §5.8 | G: 3×3 / W: delete_column(1) / T: 6 cells | a:frankmd-workspace::table_editor | keep |
| 32 | 702 "delete row via context menu" | Delete Row leaves 6 cells | §5.8 | G: 3×3 / W: delete_row(1) / T: 6 cells | a:frankmd-workspace::table_editor | keep |
| 33 | 726 "cannot delete header row" | Delete Row is disabled on the header row | §5.8 | G: focus on row 0 / W: cell_menu(0,0) / T: DeleteRow disabled; `delete_row(0)` is a no-op | a:frankmd-workspace::table_editor | keep |
| 34 | 747 "image button is always visible for web search access" | The Insert image button is always available, even without images_path | Pane header §3.4 | G: images_path empty, note open, width ≥ 720 / T: header buttons include InsertImage; insert_image sources include Web/Upload; < 720 → InsertImage in the ⋯ menu; b: `pane-insert-image` visible | a:frankmd-workspace::commands + b | keep |
| 35 | 815 "image button is visible when images are configured" | Same, with images_path set | §3.4 | Same as row 34 with images_path set (grouped) | a:frankmd-workspace::commands | keep |
| 36 | 829 "image picker dialog opens and shows images" | The image-folder source lists its images (2) | Insert image Library source §5.6 | G: images_path with photo1.png, photo2.png / W: InsertImage, select Library, collection ImageFolder / T: results = 2 items | a:frankmd-workspace::insert_image | changed (§5.6 source rail; Local tab → Library + collection select) |
| 37 | 846 "image picker search filters images" | Filename search narrows results | §5.6 Library search | G: cat.png, dog.png / W: query "cat" (debounced, injected clock) / T: 1 result | a:frankmd-workspace::insert_image | keep |
| 38 | 871 "selecting image shows options" | Selecting an image shows the footer with its name and alt prefilled with the stem | Footer §5.6 (FE-320) | G: myimage.png / W: select(0) / T: footer.selected_name "myimage.png", alt "myimage" | a:frankmd-workspace::insert_image | keep |
| 39 | 896 "inserting image adds markdown to content" | Insert writes `![photo](…/photo.png)` | §5.6 | G: photo.png selected / W: confirm / T: buffer contains "![photo](" + image URL ending "photo.png" (URL format: see open question) | a:frankmd-workspace::insert_image | keep |
| 40 | 927 "inserting image with link wraps in anchor" | A link URL wraps the image | §5.6 Link field | G: clickable.png, link "https://example.com" / W: confirm / T: buffer contains "[![clickable](" and "](https://example.com)" | a:frankmd-workspace::insert_image | keep |
| 41 | 962 "inserting image with custom alt text" | A custom alt overrides the prefill | §5.6 Alt field | G: boring-name.png, alt "A beautiful sunset" / W: confirm / T: buffer contains "![A beautiful sunset](" | a:frankmd-workspace::insert_image | keep |

### `test/system/preview_image_viewer_test.rb` — C=2 (keep 1 / changed 1 / dropped 0; a=2 b=0 c=0)
Original subject: the preview image lightbox (keyboard open, Esc and close, focus restore, linked images). New home: image viewer §5.20, preview images §5.21.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "images open through keyboard and close through Escape with focus restored" | A preview image is keyboard-focusable; Enter opens the viewer with the alt caption and focus on close; Esc or Close closes it and returns focus to the image; the open note does not change | Image viewer §5.20 | G: images.md preview visible, image 0 focused / W: Activate / T: overlay ImageViewer{index 0, caption "Example", counter 1/2}; Escape → closed, focus back on preview image 0, open note still images.md; b: press enter on the focused image → `image-viewer` visible; escape → hidden, `preview-pane` focused | a:frankmd-workspace::overlay + b | keep |
| 2 | 28 "linked image keeps navigation and its enlarge control fits mobile viewport" | A linked image keeps its link; a separate enlarge affordance opens the viewer; the viewer fits a phone-size window; clicking the link opens the target note | Preview links/images §5.21, viewer §5.20 | G: [![Linked](icon)](/notes/target.md) / W: activate the link / T: navigation opens target.md (internal link); W: enlarge action / T: ImageViewer opens; b: window 390×844 → `image-viewer` bounds within the window (32 px margins); c: visual fit | a:frankmd-workspace::navigation + b | changed (§5.21 no `<a><button>` DOM; enlarge affordance TBD, see open question) |

### `test/system/settings_dropdown_test.rb` — C=4 (keep 4 / changed 0 / dropped 0; a=2 b=2 c=0)
Original subject: Settings popover menus (theme, media storage, locale): focus, Esc order, one-at-a-time, viewport fit, resize and scroll. New home: Settings Select controls §5.16 (gpui-component dropdown as a popover, not clipped by the sheet); Esc chain §6.1.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "theme popover scrolls independently of Settings at desktop and mobile sizes" | At 1280×720, 390×600 and 360×360 the theme menu opens with the current option focused and checked, without scrolling Settings; it stays inside the window and scrolls itself; End reaches the last option; Tab closes it and moves to "Customize…"; the first Esc closes the menu (focus back on its button), the second closes Settings | Settings Select §5.16, Esc chain §6.1 | for each size: `simulate_window_resize`; ctrl-, ; click the Theme select / T: focused option = tokyo-night (checked); press end → last option focused; tab → popover gone, "Customize…" focused; reopen, escape → popover gone, select focused, `settings-sheet` visible; escape → sheet gone. a: `escape_target` = Popover before Overlay(Settings) | b | keep |
| 2 | 49 "settings pickers share dismissal and keep selection behavior" | Only one popover is open at a time; clicking its button again closes it; picking an option closes it and refocuses the button; choosing Dark applies it; switching category closes popovers | `SettingsVm` popover slot | G: SettingsVm / W: open(theme), toggle(theme), open(theme), open(storage), open(locale), pick(en), open(storage)+Esc, pick theme dark, open(theme)+select category Editor / T: `open_popover` holds at most one key at every step; pick → None and focus returns to that row's control; theme applied = dark; category switch → None; Esc in a popover leaves Settings open | a:frankmd-workspace::settings_vm + b | keep |
| 3 | 84 "resizing an open menu keeps its focused option visible" | A window resize while the menu is open keeps it in the window and the focused option visible | gpui-component popover placement | G: theme menu open at 1280×900, focus tokyo-night / W: `simulate_window_resize(1280,650)` / T: popover still open, focused option unchanged; visual in-bounds check at level c | b | keep |
| 4 | 107 "scrolling Settings dismisses a menu and closing the dialog leaves no popover" | Scrolling Settings closes an open menu; closing Settings closes any popover; reopening shows none | `SettingsVm` | G: theme popover open / W: on_scroll(40) / T: open_popover None; open(locale), close sheet / T: None; reopen Settings → None | a:frankmd-workspace::settings_vm | keep |

### `test/system/workspace_test.rb` — C=23 (keep 13 / changed 9 / dropped 1; a=19 b=1 c=2)
Original subject: workspace layout, editor chrome, title rename, the overflow menu with pins, responsive/mobile and stats. New home: editor layout §3.1, pane header and title rename §3.4, Document menu §3.4 (no pins), width classes §4.1, view modes §4.2, focus §4.3, status line §3.6, Settings §5.16.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 6 "multiline editor selection follows the text" | A multi-line selection highlights exactly the selected text; with several selections, secondary ranges and carets are drawn | Editor §6.2 (multi-cursor Ctrl+D, Ctrl+Alt+↑/↓) | c: select 6..31 in the sample, check the highlight follows the glyphs; add cursors, check secondary highlights and carets. b: select "line", ctrl-d, type "X" → `editor` value changed at both occurrences | c + b | keep |
| 2 | 47 "editor margins place the caret while the scrollbar stays at the pane edge" | The text column has margins > 40 px each side; the scrollbar sits at the pane edge; clicks in the left/right margin place the caret at column 0 / line end; same with typewriter, in split and at 320 px; a click in the editor closes an open menu | Editor pane §3.1 (column = editor_width ch, padding 32 px inline), §4.1 | a: `layout::text_column(pane_w, editor_width, char_w)` → inline margins ≥ 32 px at 1400 and split, column ≤ pane at 320; scrollbar x = pane edge; b: click left margin → caret col 0, right margin → line end (typewriter on/off); open `pane-menu`, click `editor` → menu closed | a:frankmd-workspace::layout + b | keep |
| 3 | 140 "empty notes focus the editor" | Opening an empty note puts focus in the editor | Navigation + focus §4.3 (FE-259) | G: empty.md / W: open / T: `focus == Editor`; b: `editor` focused | a:frankmd-workspace::navigation + b | keep |
| 4 | 147 "save status stays translated before locale data loads" | The save state shows translated "Unsaved", never a raw i18n key | Status line §3.6 | G: catalog compiled in (no async load) / W: edit → dirty / T: `StatusLine.save_label() == i18n status.unsaved` and ≠ "status.unsaved"; holds in all 7 locales | a:frankmd-workspace::status | keep |
| 5 | 162 "reloading an unchanged note does not mark it unsaved" | Reopening an unchanged note, even with a spurious change event, stays clean and writes no draft | Autosave / drafts | G: clean-reload.md opened, then a new Workspace (restart) / W: `on_editor_change(doc_changed, same text)` / T: SaveState Saved/clean, `has_unsaved_changes == false`, no draft written | a:frankmd-workspace::autosave | keep |
| 6 | 185 "Home hides the header and missing documents cannot be renamed" | With no note there is no document header; a missing target can't be renamed, offers a minimal menu, and falls back to the landing view | §5.22 note not found, command availability | G: no note → empty state; W: open "missing.md" / T: status message + quick open prefilled "missing"; no open note; RenameNote, DeleteNote, CopyContents `Unavailable("Open a markdown note first")`; no 6 s timer | a:frankmd-workspace::navigation + b | changed (§5.22 replaces the 6 s return-to-Home) |
| 7 | 197 "Explorer header separates toggle and actions while footer owns the brand" | Sidebar header holds its actions at a fixed size; the footer brand is decorative; hiding the sidebar moves focus to the opener; About is keyboard-reachable in Settings | Sidebar §3.3, pane header §3.4 | G: note open / T: vault header actions [NewNote, NewFolder]; footer = 3 icon buttons, no brand; W: ToggleSidebar / T: `sidebar_visible=false`, header shows OpenSidebar, focus moves out of the sidebar (target: see open question); b: ctrl-e hides `sidebar`; Settings › About reached via keyboard only; c: 28×28 buttons, 16 px icons | a:frankmd-workspace::layout + b | changed (§3.3 brand and Find/Search actions moved out of the sidebar) |
| 8 | 264 "title rename preserves nested paths and edited content" | Activating the title opens an inline input with the stem; Enter renames within the same folder, keeps unsaved edits and saves them to the new path; the title then shows the new name and regains focus | Pane title rename §3.4 | G: folder/original.md "Initial", edited " edited" (dirty) / W: `InlineRename::start(Title)` / T: value "original", caret at end (8,8); W: "renamed", Enter / T: path folder/renamed.md, buffer "Initial edited", after autosave disk folder/renamed.md == "Initial edited", original gone, window title "renamed — FrankMD", focus on pane-title; b: f2 in editor → inline input focused | a:frankmd-workspace::rename + b | keep |
| 9 | 293 "title input matches its label and resizes with the text" | The inline input occupies the title's place and auto-sizes with its text | §3.4 auto-sized inline input | Visual: the input overlays the title with the same x/y/height; typing a longer or shorter name grows or shrinks it | c | keep |
| 10 | 324 "About remains accessible inside Settings on desktop and phone" | At 1400 and 320 px, Settings › About shows FrankMD and the GitHub link, Settings has no horizontal overflow, a category switch hides About, Esc closes and refocuses the opener | Settings §5.16, overlay focus restore §4.3 | a: `SettingsVm` select(About) → about rows; select(Files) → About hidden; overlay pop restores the previous focus; b: at 1400×900 and 320×720 open via the dock-footer Settings button, select About, escape → `settings-sheet` gone, the Settings button focused; sheet bounds within the window | a:frankmd-workspace::settings_vm + b | keep |
| 11 | 346 "title rename cancellation and failure retain the current note" | Esc cancels and refocuses the title; a name collision shows an inline error, keeps the input focused and invalid, and changes no files; a free name then succeeds | §3.4 (inline error as a banner row) | G: original.md, occupied.md / W: start, "cancelled", Esc / T: path unchanged, focus on title; W: "occupied", Enter / T: error matches /exist/i, invalid=true, input still active, both files unchanged; W: "available", Enter / T: path available.md | a:frankmd-workspace::rename + b | keep |
| 12 | 370 "clicking away submits a title rename and preserves editor focus" | Blur commits the rename and focus lands where the user clicked (the editor) | §3.4 Enter/blur commits | G: folder/original.md / W: start, "renamed", `blur(to: Editor)` / T: path folder/renamed.md, input closed, `focus == Editor`; b: click `editor` while the input is active | a:frankmd-workspace::rename + b | keep |
| 13 | 383 "blank titles show inline validation and can be corrected on blur" | A blank name shows `errors.title_required`, keeps the input open and doesn't rename; correcting it and blurring renames | §3.4 | G: original.md / W: " " + blur / T: error i18n errors.title_required, invalid, path unchanged; W: "corrected" + blur / T: error cleared, input closed, corrected.md exists, original.md gone | a:frankmd-workspace::rename | keep |
| 14 | 402 "hidden Explorer reopens for Library and Settings and preserves preview mode" | With the sidebar hidden, Library and Settings are still reachable; closing the library restores the previous preview visibility | Media library §4.2 (restore preview), §3.3 dock footer, palette | G: preview on, sidebar hidden / W: ToggleMediaLibrary; OpenSettings; Escape; ToggleMediaLibrary / T: media_library_open replaces editor+preview; Settings opens over it; on return `preview_visible == true`; b: ctrl-shift-m works with the sidebar hidden | a:frankmd-workspace::layout + b | changed (§4.2: shortcuts and palette replace the sidebar-only buttons) |
| 15 | 422 "single preview retains its toggle and overflow moves focus into dialogs" | In single view the preview toggle swaps editor ⇄ preview; a Document-menu action opens a dialog that takes focus | View modes §4.2, overlay focus trap §4.3 | G: view_mode Single / W: TogglePreview ×2 / T: PaneArrangement Preview-only, then Editor-only; W: Document menu → Table… / T: menu closed, overlay Dialog(Table), focus inside; b: `dialog-table` contains the focus | a:frankmd-workspace::layout + b | keep |
| 16 | 440 "menu and pinned copy buttons copy the current document content" | Copy contents copies the current buffer, unsaved edits included, from the editor or the single-preview view, and confirms | Document menu "Copy contents" §3.4, status message §3.6 | G: nested/note.md dirty "…Original edited", fake clipboard / W: CopyContents (menu), then in Single+preview view via the palette / T: clipboard == buffer both times; status message "Copied to clipboard" (status.copied_to_clipboard) with a 2 s TTL; c: real clipboard service | a:frankmd-workspace::commands + c | changed (§3.6 status message replaces the toast; §3.4 no pinned button) |
| 17 | 469 "menu pins expose action icons and survive a page reload" | Pinning a menu action adds a header button that persists | — | — | — | dropped (design-removed: §3.4 pinning is gone; four fixed header buttons) |
| 18 | 491 "preview remains reachable from the menu after unpinning" | Preview stays reachable from the menu regardless of header buttons | Document menu View › Preview §3.4 | G: Single mode / W: `document_menu()` → View › Preview twice / T: row always present with its shortcut; toggles Preview-only ⇄ Editor-only | a:frankmd-workspace::commands | changed (§3.4 pins gone; the menu row always exists) |
| 19 | 509 "mouse clicks leave no selected row while keyboard focus stays visible" | Opening the menu by mouse highlights no row; opening it by keyboard focuses the first row with a visible ring | Document menu §3.4, focus visibility §9 | b: click `pane-menu` → no menu row focused; press enter on the focused `pane-menu` → first row focused; arrow keys move. c: focus ring visible, no hover-only affordances (pin opacity rules dropped) | b + c | changed (§3.4 pins dropped; §1.3 nothing hidden behind hover) |
| 20 | 538 "long names and translated chrome fit desktop tablet and phone layouts" | Long names and translated chrome (en/es/he, dark/light) fit at 1400, 768 and 320 with no horizontal overflow; the toolbar stays in the header; the editor starts below it; the rename input and menu fit | §3.4 narrow collapse, §4.1, §8.1 (no RTL mirroring in v1) | a: `layout::header_arrangement(width, title)`: ≥720 → [image, find, preview, ⋯] + full breadcrumb; <720 → [preview, ⋯], breadcrumb "…/ Title" with ellipsis; title never pushes buttons out. c: screenshots at 1400/768/320 × en/es/he × dark/light; Document menu clamped to the window | a:frankmd-workspace::layout + c | changed (§3.4 collapse rules; §8.1 he not mirrored) |
| 21 | 596 "mobile Explorer overlays the document and closes after selection and Escape" | At phone width the explorer is an overlay over an inert document with focus trapped; picking a file closes it; Settings opens from it; Esc closes it and refocuses the opener | Narrow drawer §4.1 | G: width 320 / T: sidebar mode Drawer, closed; W: open drawer / T: scrim, focus trapped; W: click mobile.md / T: drawer closed, note open; W: open drawer, OpenSettings, Esc, Esc / T: Settings closes first, then the drawer; focus on OpenSidebar; b: `simulate_window_resize(320,720)` and the same flow by ids | a:frankmd-workspace::layout + b | keep |
| 22 | 630 "resizing from mobile restores focus to the visible sidebar toggle" | Crossing back to wide docks the sidebar, and focus never stays on a control that vanished | Width classes §4.1 (24 px hysteresis) | G: Narrow, drawer closed, focus on OpenSidebar / W: resize to 1400 / T: class Wide, sidebar docked (per persisted sidebar_visible), OpenSidebar gone, focus moved to a visible target; hysteresis: 710↔730 doesn't flip | a:frankmd-workspace::layout + b | changed (§4.1; focus target per open question) |
| 23 | 648 "Stats starts hidden and can be enabled in Settings" | Stats are hidden by default; enabling them shows the word count ("3") and persists show_stats = true | Status line stats §3.6, Settings › Writing §5.16 | G: "Three words here", show_stats false / T: StatusLine.stats None; W: settings toggle show_stats / T: stats "3 words"; config show_stats == true after the 300 ms debounce (injected clock); a restart keeps it | a:frankmd-workspace::status | changed (§3.6 stats move to the status line) |

#### New VM requirements (G1)
- `frankmd-workspace::table_editor` — `TableEditorVm`: default 3×3 with headers "Header 1…n"; add/remove row and column (min sizes); move row up/down and column left/right; delete row (disabled on the header row) and column; `cell_menu(r,c)` items with enabled state and shortcuts; `from_markdown(text, caret)` via `markdown::table` (edit mode → "Update"); `to_markdown()` padded, with pipe escaping; applying is one edit (notes rows 23–33).
- `frankmd-workspace::rename` — `InlineRename {target: Title | ExplorerRow(path), value, selection, error: Option<RenameError>, invalid}`: `start` (stem, caret at end), `commit(on: Enter | Blur(to))`, `cancel` (focus back to the origin), validation `errors.title_required` for blank input, collision error (/exist/i) with no FS change, `.md` appended, same-folder rename for the title, dirty buffer saved to the new path, the focus target after commit (workspace 8, 11–13; notes 14–16; drafts 3).
- `frankmd-workspace::quick_open` — `EmptyStateModel`: logo, vault name, note count, recents section (only when non-empty) with `ClearHistory` → PromptQueue confirm, shortcut chips from the registry. Recent rows carry a relative-time label via the injected Clock, reusing the ported `recent_file_opened_ago`, with an "Unknown time" fallback. Recents cap 10, newest first, pruned of missing files on refresh (NC 1–2, RF 1–2, home 1–4, notes 1–2).
- `frankmd-workspace::recents` (or inside `navigation`) — record on successful open only; prune missing on read; clear (persisted to `.fed` `recent_files`); a stale entry for a missing note never creates a file (home 3).
- `frankmd-workspace::startup` — `resolve(args) -> StartupTarget {None, Note(rel), Missing(rel)}` for a CLI path relative to the vault (absolute paths inside the vault accepted); nested Hugo paths; byte-exact load; `.fed` refused; Missing → the note-not-found flow (NC 9, 12–16).
- `frankmd-workspace::navigation` — back/forward history across note switches that restores drafts; going back past the first entry → empty state; note-not-found flow (status message plus QuickOpen prefilled with the stem); open-note sets focus Editor for empty notes and keeps Sidebar focus for explorer clicks (drafts 1; notes 4; workspace 3, 6).
- `frankmd-workspace::drafts` — remap on rename/move (file and folder; the prefix match includes the trailing `/`); delete on note/folder delete; an empty-string draft is valid; no draft when the buffer equals disk; restore on restart → SaveState::Unsaved without Recovery when the base revision matches (drafts 1–5; workspace 5).
- `frankmd-workspace::explorer` — extras: `click_background()` clears the selection only; `context_menu_background()` = [NewNote, NewFolder, SelectAll, CollapseAll]; `creation_destination(origin: Global | FolderContext(p))` (single selected folder, even collapsed, else root); `reveal(path)` after create/move expands ancestors; dotfiles (`.fed`) never produce rows; drag payload = selection when the grabbed row is selected, else [grabbed]; `empty_message()` only at the root.
- `frankmd-workspace::dialogs` — `NewNoteForm::new(origin, hugo_path_style, clock)`: path preview strings (incl. the root form "note-name.md" placeholder), toolbar/global Hugo → `blog/` (dated or flat), folder-context Hugo → that folder; `DeleteConfirm` text with counts and the trash wording; non-empty folder handling per the §12 Q3 decision.
- `frankmd-workspace::layout` — `text_column(pane_w, editor_width, char_w) -> (inline_margin, column_w)`; `header_arrangement(width, title) -> {buttons, breadcrumb: Full | Collapsed}`; `SidebarMode {Docked, Hidden, Drawer{open}}` with close-on-open-note; a focus fix-up hook on width-class change (workspace 2, 7, 20–22; home 4).
- `frankmd-workspace::settings_vm` — `open_popover: Option<RowKey>` (single; closed on scroll, category switch, sheet close and option pick); `About` rows (logo, name, description, version, copyright, source link as OpenUri); Media `media_storage_provider` row with S3 disabled reason; Writing `show_stats` switch persisted with a debounce; Editor font, size and width values with provenance (NC 5, 6, 17; SD 2, 4; WS 10, 23).
- `frankmd-workspace::commands` — `document_menu(&UiState)` groups per §3.4; `header_buttons(width)`; availability reasons for RenameNote, DeleteNote and CopyContents with no open note; per-context bindings so `delete` in the Editor never maps to DeleteNote; i18n labels free of shortcut text in all locales (NC 4, 7; notes 21–22, 34; WS 16, 18).
- `frankmd-workspace::status` — `save_label()` from the compiled-in catalog; stats slot (`core::stats`, "N words"); the "Copied to clipboard" message (WS 4, 16, 23).
- `frankmd-workspace::overlay` — Esc chain: a Select popover inside Settings → Settings → narrow drawer; focus restore to the opener (the dock-footer Settings button, the preview image, "Customize…") (SD 1; WS 10, 21; PIV 1; CTE 1).
- `frankmd-workspace::insert_image` — Library source with collection {NoteImages, ImageFolder}; filename search (debounced); footer selected_name + alt prefilled from the stem; `insert_markdown()` with optional link wrap `[![alt](url)](link)` (notes 36–41).
- `frankmd-theme::custom` — field error "six-digit hex" + `can_save`; contrast warning per pair; base-theme syntax defaults (functions #f0e68c); `save()` → config `theme=custom` + `custom_theme.colors`; `cancel()` reverts the live tokens (CTE 1–2).
- `frankmd-markdown::edit` — surround-on-type keeps the inner selection and is one undo step per keystroke (MS 1).
- `frankmd-vim::keymap` — in Normal/Visual mode `*` resolves to search, and surround-on-type is only active in Insert/non-vim mode (MS 2).

#### Open questions (G1)
1. **`.fed` in the explorer.** FrankMD rendered `.fed` as a non-draggable config row (tree_helper 68). §5.14 hides dotfiles and §5.16 moves config editing into Settings. Confirm `.fed` is never listed.
2. **Focus after clicking a note in the explorer.** FrankMD keeps focus on the tree row (notes 33). §5.14 and §4.3 are silent. This plan assumes focus stays in the Sidebar for a click, and Enter behaves the same way.
3. **Default preview visibility on first launch.** FrankMD starts with the preview hidden (notes 164). §4.2 persists visibility but gives no first-run default.
4. **Backdrop click on Settings.** The §5 intro lists the overlays that close on backdrop click but doesn't mention Settings. FrankMD closes it on backdrop click (notes 213).
5. **Non-empty folder delete.** It depends on §12 Q3 (trash). With trash, notes 422 becomes "confirm lists counts, folder trashed". Without it, keep "Folder not empty".
6. **Linked images in the preview.** §5.21 says "Enter/click opens the viewer", which conflicts with a linked image whose click must follow the link (PIV 28). An enlarge affordance is needed. Decide its form (a focusable second control, or Ctrl+. / the context chip "View image").
7. **Inserted image URL for `images_path` images.** FrankMD wrote `/images/preview/photo.png`, a web route. Drop-in compatibility with existing notes needs a decision on what the native app writes and how the preview resolves old `/images/preview/…` URLs.
8. **Focus target when the sidebar hides** (Ctrl+E: FrankMD focused the floating opener, WS 197) **and when Narrow→Wide removes the opener** (WS 630). The design is silent. A proposal: Editor in both cases, or the vault header when the sidebar becomes docked and had focus.
9. **Recents in quick open.** Should the recents part of the list keep the "N minutes ago" wording of `recent_files_helper`, or the shorter "2 h ago" from §5.2? Should a malformed `opened_at` show "Unknown time" or nothing?
10. **Title rename caret.** FrankMD places the caret at the end without selecting (selection 8,8 in WS 264). §3.4 doesn't say. Keep it, or select the whole stem as VS Code does?
11. **Background-menu path preview.** FrankMD showed `root/menu_note.md`. §5.13 shows `In: blog/`. Confirm the root destination wording (`In: /` or the vault name).
12. **Back/forward history scope.** Is it session-only, and does it skip notes deleted since? Not specified in §6.1. Drafts 1 relies on back past the first entry → empty state.
13. **Custom theme editor groups.** FrankMD's syntax colours sat behind one `<summary>`. §5.17 uses a collapsible Code group. Confirm the "Code functions"/"Code keywords" labels are kept so existing locale strings carry over.

## 7. G2: JS lib and image_sources

### `test/javascript/lib/app_prompt.test.js` — C=9 (keep 8 / changed 1 / dropped 0; a=9 b=0 c=0)
Original subject: in-app `<dialog>` replacements for alert/confirm/prompt (localized, queued, focus-restoring). New home: generic alert/confirm/prompt modal dialogs (§2 Overlays, §5 dialog anatomy, §5.13 Delete confirm); `workspace::dialogs::PromptQueue` + `workspace::focus` restore stack.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 27 "shows a localized, app-styled alert and resolves when acknowledged" | Alert shows localized title "Notice" + message + single OK; OK closes it and resolves | Generic alert dialog §2/§5 | G: locale en / W: PromptQueue.push(Alert "Something went wrong") / T: current = title i18n `common.alert`, body text, buttons [OK]; ack → resolves (), queue empty; b: `dialog-alert` visible, click OK → not visible | a:frankmd-workspace::dialogs + b | keep |
| 2 | 43 "uses localized destructive actions for confirmations" | Destructive confirm lists Cancel then Delete; Delete styled danger; Delete → true | Delete confirm §5.13, dialog anatomy §5 (secondary then primary) | G: Confirm{accept_label "Delete", destructive} / W: inspect then accept / T: buttons [Cancel, Delete] in order; primary role Danger; resolves true | a:frankmd-workspace::dialogs | keep |
| 3 | 59 "focuses the dialog instead of either button for destructive confirmations" | Destructive confirm never starts focused on the destructive button; Cancel → false | §5.13 "focus starts on Cancel" | G: destructive Confirm / W: opened / T: initial_focus = Cancel (not Delete); b: focused element is `dialog-confirm` cancel button; Cancel resolves false | a:frankmd-workspace::dialogs + b | changed (§5.13 focus starts on Cancel, not on the dialog) |
| 4 | 75 "resolves false when a confirmation is cancelled" | Esc cancels a confirm (resolves false), nothing else handles that Esc | §5 "Esc cancels every dialog except Recovery"; §6.1 Escape chain | G: Confirm open / W: escape_target → Dialog; cancel / T: resolves false; b: press escape → dialog gone, no other layer closed | a:frankmd-workspace::overlay + b | keep |
| 5 | 86 "collects text without exposing HTML from the default value" | Text prompt pre-fills the default verbatim (quotes etc.) and returns the edited value on OK | Generic prompt dialog §2 | G: TextPrompt{default `photo" onerror="alert(1).png`} / W: read input, set "renamed.png", OK / T: input value == default verbatim; resolves "renamed.png" | a:frankmd-workspace::dialogs | keep |
| 6 | 97 "accepts a text prompt with Enter" | Enter in the prompt input accepts with current value | §5 dialog anatomy (Ctrl+Enter confirms; Enter in single-line input) | G: TextPrompt default "old.png" / W: input "new.png", press enter (and ctrl-enter) / T: resolves "new.png" | a:frankmd-workspace::dialogs + b | keep |
| 7 | 107 "queues prompts so only one is modal at a time" | Only one prompt visible; second appears after first resolves; focus returns to opener after last | §2 overlays one at a time | G: focus on X; push Alert A, Alert B / W: ack A, ack B / T: only A current, then B current, then none; focus restore target = X | a:frankmd-workspace::dialogs | keep |
| 8 | 129 "restores focus to the element active before the prompt" | Closing a prompt returns focus to where it was | §4.3 "Overlays … return it to the previous region" | G: explorer focused / W: alert opened then OK / T: FocusRegion = Sidebar, same row focused; b: `explorer-tree` focused() | a:frankmd-workspace::focus + b | keep |
| 9 | 144 "restores focus into the dialog underneath a nested app prompt" | Prompt opened from inside a dialog returns focus into that dialog and leaves it open | e.g. Insert image › Library delete confirm §5.6 over `dialog-insert-image` | G: Insert image dialog open, delete button focused / W: confirm pushed, Cancel / T: parent dialog still open; focus back on the delete button; b: `dialog-insert-image` visible + focused child | a:frankmd-workspace::overlay + b | keep |

### `test/javascript/lib/code_block_highlighter.test.js` — C=2 (keep 2 / changed 0 / dropped 0; a=2 b=0 c=0)
Original subject: async preview code highlighting with lazy-loaded parsers. New home: native preview code blocks §5.21 (shared highlighter, §10 FE-200…202); `markdown::highlight` + preview render generation.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 351 "highlights common languages before lazy parser imports resolve" | Common-language fences are coloured on the first preview paint, not after a delay caused by other fences | Preview code blocks §5.21 | G: note with ```js and ```tf fences / W: one render pass / T: js block has Keyword spans in the first PreviewDoc produced (no pending/plain state); tf block highlighted too (grammars bundled) | a:frankmd-markdown::highlight | keep |
| 2 | 365 "does not apply a lazy parser result to code replaced by a newer render" | A slow highlight from an older render never overwrites newer content | Preview render §5.21 | G: render gen 1 (tf fence) highlighting off-thread / W: doc edited → gen 2 (js fence) committed, then gen 1 result arrives / T: gen 1 result discarded; preview shows "const current = 1;" | a:frankmd-workspace::preview_render | keep |

### `test/javascript/lib/codemirror_extensions.test.js` — C=18 (keep 15 / changed 0 / dropped 3; a=14 b=1 c=0)
Original subject: CodeMirror extension factory (line numbers, options, read-only, runtime compartments). New home: editor pane (§3.4, §6.2), Settings › Editor (§5.16), status line (§3.6), read-only banner (§5.12); `workspace::editor_view` (new).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 17 "defines OFF mode as 0" | Persisted line-number value 0 = Off | Settings › Editor Line numbers | — | — | dropped (covered-by-A: line_numbers.test.js LINE_NUMBER_MODES ×3) |
| 2 | 21 "defines ABSOLUTE mode as 1" | Persisted value 1 = Absolute | same | — | — | dropped (covered-by-A: line_numbers.test.js) |
| 3 | 25 "defines RELATIVE mode as 2" | Persisted value 2 = Relative | same | — | — | dropped (covered-by-A: line_numbers.test.js) |
| 4 | 31 "returns empty array for OFF mode" | Off shows no gutter | Editor gutter §6.2 Ctrl+L | G: mode Off, 3-line doc / W: gutter_labels / T: None (no gutter); status position shows `12/340` form (§3.6) | a:frankmd-workspace::editor_view | keep |
| 5 | 37 "returns extensions array for ABSOLUTE mode" | Absolute shows 1..n | Editor gutter | G: Absolute, caret line 2 / W: gutter_labels / T: ["1","2","3"] | a:frankmd-workspace::editor_view | keep |
| 6 | 43 "returns extensions array for RELATIVE mode" | Relative shows distance, caret line absolute (FE-078) | Editor gutter | G: Relative, caret line 2 of 3 / W: gutter_labels / T: ["1","2","1"] | a:frankmd-workspace::editor_view | keep |
| 7 | 51 "returns an array of extensions" | Editor opens a note with default settings | Editor pane §3 | G: default config, vault with a.md / W: open a.md / T: `editor` visible and focused | b | keep |
| 8 | 57 "accepts placeholder text option" | Empty note shows a placeholder prompt | Editor pane | G: empty note / W: EditorViewConfig built / T: placeholder = i18n editor placeholder key; hidden once text exists | a:frankmd-workspace::editor_view | keep |
| 9 | 64 "accepts font options" | Editor font family/size/line height follow settings | Settings › Editor §5.16 | G: font "Monaco", 16, 1.8 / W: EditorStyle::from_settings / T: style carries those values; size clamped 12–24 | a:frankmd-theme::tokens | keep |
| 10 | 73 "accepts line number mode option" | Opening a note honours the saved line-number mode | Settings › Editor | G: settings line_numbers=Off then Absolute / W: open note / T: editor_view.line_numbers matches each | a:frankmd-workspace::editor_view | keep |
| 11 | 80 "accepts onUpdate callback" | Typing updates save state, stats and schedules preview render | Status line §3.6; autosave | G: clean note / W: insert "x" / T: SaveState Unsaved; stats words+1; preview render scheduled; autosave timer armed | a:frankmd-workspace::status | keep |
| 12 | 86 "accepts onSelectionChange callback" | Moving the caret updates Ln:Col and the context chip | Status line Position + Context chip §3.6 | G: doc with table on line 3 / W: caret → 3:2 / T: position "3:2"; chip "Ctrl+. Edit table" | a:frankmd-workspace::status | keep |
| 13 | 92 "accepts onScroll callback" | Scrolling the editor drives scroll sync | Preview §5.21, scroll sync | G: scroll sync on, preview visible / W: editor scroll event top line 50 / T: scroll_sync target preview block for line 50; off → no target | a:frankmd-workspace::scroll_sync | keep |
| 14 | 100 "returns extensions array including read-only" | Read-only note cannot be edited | Read-only banner §5.12 | G: note read-only on disk / W: insert "x" / T: doc unchanged; banner Read-only shown; lock indicator | a:frankmd-workspace::editor_view | keep |
| 15 | 106 "includes all base extensions" | Read-only editor keeps gutter, font, highlighting | Editor pane | G: same settings, read_only true vs false / W: build configs / T: identical except read_only flag | a:frankmd-workspace::editor_view | keep |
| 16 | 115 "exports themeCompartment" | Theme change restyles the open editor live, keeping doc/caret/undo | Theme picker §5.17 | G: note open, caret 10, 1 undo entry / W: ThemeChanged / T: new EditorStyle; caret 10; undo depth 1 | a:frankmd-workspace::editor_view | keep |
| 17 | 120 "exports lineNumbersCompartment" | Ctrl+L cycles line numbers live | §6.2 Line numbers cycle | G: Off / W: ToggleLineNumbers ×3 / T: Absolute → Relative → Off; caret kept; b: press ctrl-l changes mode | a:frankmd-workspace::editor_view + b | keep |
| 18 | 125 "exports readOnlyCompartment" | Read-only can be toggled live (grammar lock) | Grammar check §5.10 | G: editable note / W: grammar start then cancel / T: read_only true then false; banner shown then removed | a:frankmd-workspace::grammar | keep |

### `test/javascript/lib/codemirror_extensions_vim.test.js` — C=4 (keep 4 / changed 0 / dropped 0; a=4 b=0 c=0)
Original subject: vim extension factory, block cursor, keydown precedence. New home: Vim mode §6.3, status-line mode block §3.6, Settings › Editor Vim mode; `vim::state`, `vim::keymap`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 8 "returns the vim extension when enabled" | Vim on → modal editing with a status display | §6.3; mode block §3.6 | G: vim setting on / W: open note / T: vim::state mode Normal; status mode block "NORMAL" | a:frankmd-vim::state | keep |
| 2 | 15 "returns nothing (empty) when disabled — editor is unchanged" | Vim off → typing inserts, no mode block | §3.6 "Vim off: hidden" | G: vim off / W: type "j" / T: doc gets "j"; status mode block hidden | a:frankmd-vim::state | keep |
| 3 | 19 "adds the themed block cursor after vim, so it outranks the library colours" | Normal-mode block cursor uses the theme colour | §3.6/§7.1 cursor role | G: vim on, theme Tokyo Night / W: Normal then Insert / T: cursor shape Block then Bar; colour = theme cursor role; c: visual check no foreign colour | a:frankmd-vim::state + c | keep |
| 4 | 30 "keeps the vim compartment first in createExtensions()" | In Normal mode Enter/Backspace/arrows are vim motions, not editor edits | §6.3 | G: vim Normal, "a\nb", caret 0 / W: press enter, backspace / T: caret moves to line 2 then back; doc unchanged; b: same via window.press | a:frankmd-vim::keymap + b | keep |

### `test/javascript/lib/codemirror_markdown_surround.test.js` — C=14 (keep 14 / changed 0 / dropped 0; a=14 b=0 c=0)
Original subject: typing a marker over a selection surrounds it. New home: editor typing §6.2 "Surround selection by typing" (FE-059); `markdown::edit::surround_on_type`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 35 "wraps using %s and leaves the inner text selected" [row 1 `*`] | `*` over selection wraps, inner stays selected | §6.2 | G: "sample" all selected / W: type `*` / T: "*sample*", selection 1..7, handled | a:frankmd-markdown::edit | keep |
| 2 | 35 [row 2 `_`] | same for `_` | §6.2 | as row 1 with `_` | a:frankmd-markdown::edit | keep |
| 3 | 35 [row 3 `` ` ``] | same for backtick | §6.2 | as row 1 with `` ` `` | a:frankmd-markdown::edit | keep |
| 4 | 35 [row 4 `~`] | same for `~` | §6.2 | as row 1 with `~` | a:frankmd-markdown::edit | keep |
| 5 | 35 [row 5 `^`] | same for `^` | §6.2 | as row 1 with `^` | a:frankmd-markdown::edit | keep |
| 6 | 35 [row 6 `=`] | same for `=` | §6.2 | as row 1 with `=` | a:frankmd-markdown::edit | keep |
| 7 | 42 "adds repeated markers and undoes each surround atomically" | `**` by typing twice; each surround one undo step | §6.2 Undo | G: "sample" selected / W: `*`,`*`, undo, undo, redo / T: "**sample**" → "*sample*" → "sample" → "*sample*"; b: ctrl-z in real editor undoes one surround | a:frankmd-markdown::edit + b | keep |
| 8 | 56 "preserves backward, secondary, and empty ranges with Unicode" | Multi-cursor surround; backward direction kept; CJK/emoji safe | §6.2 multi-cursor (Ctrl+D, Ctrl+Alt+↑/↓) | G: "猫 🐈 abc z", ranges (4→0),(5,8),cursor 10 / W: `_` / T: "_猫 🐈_ _abc_ z_"; selections ["猫 🐈","abc",""]; main anchor>head | a:frankmd-markdown::edit | keep |
| 9 | 64 "uses longer backtick delimiters and padding for embedded code markers" | Backtick wrap of text containing backticks stays valid code | §6.2 | G: "`sample`" selected / W: `` ` `` / T: "`` `sample` ``", inner selected | a:frankmd-markdown::edit | keep |
| 10 | 71 "preserves multiline text without adding block-conversion newlines" | Multiline selection wrapped as-is | §6.2 | G: "first\nsecond" / W: `*` / T: "*first\nsecond*" | a:frankmd-markdown::edit | keep |
| 11 | 78 "handles many backtick runs without exceeding JavaScript argument limits" | Huge selections don't crash or hang | §6.2 | G: "a` "×130000 selected / W: `` ` `` / T: wrapped in "``"; completes in linear time (no recursion) | a:frankmd-markdown::edit | keep |
| 12 | 86 "leaves empty-cursor typing and unrelated punctuation to CodeMirror" | Normal typing unaffected | §6.2 | G: caret 3; then full selection / W: `*`; then `# > - + [ ( plain` / T: surround returns None each time (default insert applies) | a:frankmd-markdown::edit | keep |
| 13 | 94 "does not intercept paste or composition" | Paste and IME commit replace selection normally | §6.2; IME §8.3 | G: "sample" selected / W: input source Paste `*`; composing=true `*` / T: None both; c: real fcitx5 commit over selection replaces it | a:frankmd-markdown::edit + c | keep |
| 14 | 102 "does not modify read-only text" | Read-only note never changes | Read-only banner §5.12 | G: read_only / W: `*` / T: None; doc "sample" | a:frankmd-markdown::edit | keep |

### `test/javascript/lib/codemirror_slash_commands.test.js` — C=6 (keep 6 / changed 0 / dropped 0; a=5 b=1 c=0)
Original subject: slash menu icons, keyboard handling and insert-type command dispatch. New home: Slash menu popover §5.5, icons §7.7, insert dialogs §5.6–§5.9; `workspace::slash_menu` (new).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 127 "renders command icon metadata as accessible decorative SVG" | Each command shows its Phosphor icon; icon is decorative (label carries the name) | §5.5, §7.7, §9 | G: slash rows / W: build / T: Table row icon "table" exists in phosphor.json (viewBox 0 0 256 256); a/11y label = text only; row without icon → None | a:frankmd-workspace::slash_menu | keep |
| 2 | 137 "renders the semantic SVG icons inside the completion menu" | Typing `/` opens the menu listing every command with an icon | §5.5 | G: doc "/" caret 1 / W: open / T: rows = full command list (see open question 12 vs 14), each with icon, contains "Heading 1"; b: `slash-menu` visible | a:frankmd-workspace::slash_menu + b | keep |
| 3 | 167 "moves through slash options with arrow keys and accepts the selected command with Enter" | ↑/↓ moves selection; Enter applies; menu closes | §5.5 "Enter/Tab apply" | G: "/" menu open / W: down to Heading 2, up, down, enter / T: selection idx moves ±1; doc "## "; menu closed; b: same via window.press, `slash-menu` not visible | a:frankmd-workspace::slash_menu + b | keep |
| 4 | 207 "dismisses slash options with Escape and keeps the query unchanged" | Esc closes menu, text stays "/table" | §5.5 "Esc dismiss"; §6.1 Escape chain (popover first) | G: "/table" menu open / W: escape / T: menu closed; doc "/table"; no other layer closed | a:frankmd-workspace::slash_menu + b | keep |
| 5 | 241 "dispatches insert actions with the original slash range and leaves the query in place" | Choosing Table opens the table dialog bound to the `/table` range; doc unchanged until insert | §5.8 Table editor (`/table`) | G: "Before /table after", caret 13 / W: accept Table / T: SlashOutcome::OpenDialog{Table, from 7, to 13, query "/table"}; doc unchanged; overlay = Dialog(Table) | a:frankmd-workspace::slash_menu | keep |
| 6 | 259 "closes the completion synchronously before a dialog takes focus" | After choosing Table the menu is gone and focus is in the dialog | §5.5 + §5.8 | G: "/" menu open / W: arrow to Table, enter / T: `slash-menu` not visible; `dialog-table` visible and focused | b | keep |

### `test/javascript/lib/codemirror_theme.test.js` — C=8 (keep 6 / changed 1 / dropped 1; a=7 b=0 c=0)
Original subject: CodeMirror editor theme + syntax highlighting factory. New home: editor styling from tokens §7.1/§7.2, Settings › Editor fonts §5.16, completion popovers §5.5; `theme::tokens`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 12 "returns an extension" | Editor is styled from the active theme | §7.1 | G: theme dark / W: EditorStyle::from(theme) / T: bg = surface.base-ish editor role, text, selection, cursor, gutter roles all set | a:frankmd-theme::tokens | keep |
| 2 | 17 "accepts font options" | Custom font family/size/line height applied | Settings › Editor | G: Monaco/16/1.8 / W: from(theme, fonts) / T: style fonts equal input | a:frankmd-theme::tokens | keep |
| 3 | 26 "uses default values when no options provided" | Sensible defaults without font settings | Settings › Editor ("System monospace" default) | G: no font settings / W: build / T: family "System monospace", default size and line height (≥1.5, §8.2) | a:frankmd-theme::tokens | keep |
| 4 | 31 "lets autocomplete results grow to 16em before scrolling" | Completion list grows before scrolling | Slash menu/wikilink popovers §5.5 | G: 14 rows / W: popover layout / T: visible rows 8 × 28 px, rest scroll; 3 rows → height 3×28 (no fixed height); c: look | a:frankmd-workspace::slash_menu + c | changed (§5.5 max 8 rows, 28 px) |
| 5 | 52 "returns a syntax highlighting extension" | Markdown syntax colours follow the theme (headings 1–3 own colours, 4–6 accent, code on code-bg) | §7.1 syntax.*, FE-076 | G: theme / W: editor_highlight_style / T: every markdown capture maps to a theme role; h1→heading-1, h4→accent | a:frankmd-theme::tokens | keep |
| 6 | 59 "returns an array of extensions" | (array shape) | — | — | — | dropped (CodeMirror-internal; behavior in rows 1 and 7) |
| 7 | 65 "includes both editor theme and syntax highlighting" | Theme switch changes both editor chrome and syntax colours | Theme picker §5.17 | G: theme A → B / W: rebuild style / T: chrome roles and syntax roles both from B | a:frankmd-theme::tokens | keep |
| 8 | 70 "accepts and passes options to editor theme" | Font settings survive theme switch | §5.16/§5.17 | G: Fira Code/18/2.0, theme A→B / W: rebuild / T: fonts unchanged | a:frankmd-theme::tokens | keep |

### `test/javascript/lib/codemirror_typewriter.test.js` — C=28 (keep 23 / changed 0 / dropped 5; a=23 b=0 c=0)
Original subject: typewriter mode state, padding and click-recenter scroll math. New home: Typewriter view mode §4.2 (Ctrl+\\), Settings › Writing (Typewriter, Recenter caret on click) §5.16, status indicator §3.6, motion §7.6; `workspace::typewriter` + `workspace::layout.typewriter`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 56 "returns an array of extensions" | (extension shape) | — | — | — | dropped (CodeMirror-internal) |
| 2 | 62 "initializes with enabled=false by default" | Typewriter off by default | §4.2; Settings › Writing | G: fresh config / W: LayoutState::default / T: typewriter false | a:frankmd-workspace::layout | keep |
| 3 | 67 "initializes with enabled=true when specified" | Saved typewriter setting restored on open | Settings › Writing | G: typewriter_mode=true / W: open note / T: typewriter true; padding applied | a:frankmd-workspace::layout | keep |
| 4 | 74 "enables typewriter mode" | Ctrl+\\ turns it on | §6.1 Typewriter Ctrl+\\ | G: off, markdown note / W: ToggleTypewriter / T: on; b: press ctrl-\\ flips state | a:frankmd-workspace::layout + b | keep |
| 5 | 83 "disables typewriter mode" | Toggling again turns it off | §4.2 | G: on / W: ToggleTypewriter / T: off | a:frankmd-workspace::layout | keep |
| 6 | 92 "adds typewriter-mode class when enabled" | Visible indication that typewriter is on | Status indicator `text-cursor` §3.6 | G: off / W: toggle on / T: status indicators contain Typewriter | a:frankmd-workspace::status | keep |
| 7 | 102 "removes typewriter-mode class when disabled" | Indication removed when off | §3.6 | G: on / W: toggle off / T: Typewriter indicator absent | a:frankmd-workspace::status | keep |
| 8 | 118 "returns false when disabled" | State query reflects off | Palette toggle check §5.1 | G: off / W: palette row "View: Typewriter" / T: no ✓ | a:frankmd-workspace::commands | keep |
| 9 | 123 "returns true when enabled" | State query reflects on | §5.1 toggle state | G: on / W: palette row / T: ✓ shown | a:frankmd-workspace::commands | keep |
| 10 | 130 "returns currentLine and totalLines" | Sync data for preview follow | — | — | — | dropped (covered-by-A: typewriter_utils.test.js getTypewriterSyncData ×6) |
| 11 | 139 "returns correct current line based on cursor" | Current line from caret | — | — | — | dropped (covered-by-A: typewriter_utils.test.js getTypewriterSyncData) |
| 12 | 153 "is defined" (setTypewriterMode) | (effect exists) | — | — | — | dropped (CodeMirror-internal) |
| 13 | 158 "can be used in dispatch" | Every entry point (palette, Settings switch, Ctrl+\\) sets the same state | §5.1, §5.16 Writing | G: off / W: settings_vm set Typewriter=true / T: layout.typewriter true; palette ✓ | a:frankmd-workspace::settings_vm | keep |
| 14 | 170 "is defined" (typewriterState) | (field exists) | — | — | — | dropped (CodeMirror-internal) |
| 15 | 174 "can be read from state" | Settings row shows the live state | Settings › Writing | G: typewriter on via Ctrl+\\ / W: settings rows / T: Typewriter switch = on | a:frankmd-workspace::settings_vm | keep |
| 16 | 182 "defaults to false" (cursorRecenterOnClick) | Recenter-on-click off by default | Settings › Writing | G: fresh config / W: read / T: recenter_on_click false | a:frankmd-workspace::settings_vm | keep |
| 17 | 187 "can be toggled with its state effect" | Toggling the setting applies live | Settings › Writing | G: off / W: set on / T: typewriter VM uses recenter_on_click true without reopen | a:frankmd-workspace::typewriter | keep |
| 18 | 195 "does not recenter a pointer click when disabled" [row 1 typewriter off] | Click doesn't scroll when recenter off | FE-084 | G: geometry top 25, caret 300, h 100, sh 1000; tw off, recenter off / W: recenter_target(PointerClick) / T: None (scroll stays 25) | a:frankmd-workspace::typewriter | keep |
| 19 | 195 [row 2 typewriter on] | Even in typewriter mode, clicks don't jump when recenter off | FE-082 skip on mouse | same with tw on / T: None | a:frankmd-workspace::typewriter | keep |
| 20 | 209 "recenters a collapsed pointer click when enabled" [row 1 typewriter off] | Click recenters the caret line | FE-084 | recenter on, tw off / W: PointerClick collapsed / T: Some(275) = 300 − 100/2 | a:frankmd-workspace::typewriter | keep |
| 21 | 209 [row 2 typewriter on] | same with typewriter on | FE-084 | recenter on, tw on / T: Some(275) | a:frankmd-workspace::typewriter | keep |
| 22 | 222 "does not recenter a pointer range selection" | Drag-selecting never scrolls | FE-082 | recenter on / W: PointerRange(2..12) / T: None | a:frankmd-workspace::typewriter | keep |
| 23 | 235 "keeps Typewriter keyboard selection recentering when click recentering is off" | Keyboard caret moves keep line centred | §4.2 Typewriter 50 % | tw on, recenter off / W: Keyboard caret → 8 / T: Some(275); c: 120 ms ease-out (§7.6) | a:frankmd-workspace::typewriter | keep |
| 24 | 245 "keeps Typewriter document-change recentering when click recentering is off" | Typing keeps line centred | §4.2 | tw on / W: DocChange insert "x" / T: Some(275) | a:frankmd-workspace::typewriter | keep |
| 25 | 255 "clamps click recentering to the editor scroll bounds" | Never scrolls past content end | FE-084 | top 10, caret 900, h 100, sh 500 / W: PointerClick / T: Some(400) = sh − h | a:frankmd-workspace::typewriter | keep |
| 26 | 264 "adds padding when enabled" | Half-height bottom padding so last line can centre | §4.2 "half-height bottom padding" | tw on, editor h 300 / W: bottom_padding / T: 150 px | a:frankmd-workspace::typewriter | keep |
| 27 | 276 "removes padding when disabled" | Padding removed when off | §4.2 | tw on → off / W: bottom_padding / T: 0 | a:frankmd-workspace::typewriter | keep |
| 28 | 291 "recalculates padding and recenters after a hidden editor becomes visible" | Editor hidden (single-view swap / media library) then shown gets correct padding and centring | §4.2 Single swap, Media library | G: tw on, editor h 0 → padding 0 / W: refresh(h 400, sh 1000, caret 600) / T: padding 200; scroll 400; b: ctrl-shift-v swap back triggers refresh | a:frankmd-workspace::typewriter + b | keep |

### `test/javascript/lib/floating_dropdown.test.js` — C=9 (keep 9 / changed 0 / dropped 0; a=8 b=1 c=0)
Original subject: accessible popover dropdown (keyboard, placement, dismissal). New home: Select/dropdown popovers in Settings (§5.16), Insert image footer selects (§5.6), pane header ⋯ menu (§3.4), popover placement §5.5; `workspace::dropdown` (new) + `workspace::menu_bounds`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 35 "opens at the selected option and navigates past disabled options" | Opens focused on the checked option; ↑ skips disabled; Home/End; Esc closes only the dropdown and refocuses trigger | Settings Theme select §5.16 inside `settings-sheet` | G: options [Light, (Unavailable disabled), ✓Dark, Nord] / W: open, up, end, home, escape / T: focus 2→0→3→0; closed; focus = trigger; b: settings-sheet still visible after Esc | a:frankmd-workspace::dropdown + b | keep |
| 2 | 56 "bounds height and flips above a trigger near the viewport bottom" | Menu flips above, clamps left, fits viewport with 8 px gutter | §5.5 "flipped above … clamped" | G: viewport 400×600, trigger 550–590 / 350–410, menu 192×320 (content 700) / W: place_popover / T: top 226, left 200, max_h 320, max_w 384 | a:frankmd-workspace::menu_bounds | keep |
| 3 | 65 "fits a short viewport without forcing a minimum height" | In tiny windows the menu shrinks rather than overflowing | §4.1 Narrow tiles | G: viewport h 180, trigger 70–110 left 8 / W: place_popover / T: max_h 58, top 114, left 8 | a:frankmd-workspace::menu_bounds | keep |
| 4 | 75 "dismisses when resizing puts the trigger outside the viewport" | Resize that hides the trigger closes the menu | — | G: open / W: on_resize(trigger top 700, vh 600) / T: closed; b: simulate_window_resize → popover not visible | a:frankmd-workspace::dropdown + b | keep |
| 5 | 83 "keeps the focused option visible when resizing reduces menu height" | Focused option stays in view after shrink | — | G: focused opt rect 430–464, open / W: resize vh 280 → max_h 128 / T: scroll_top 192; focus unchanged | a:frankmd-workspace::dropdown | keep |
| 6 | 95 "allows menu scrolling and dismisses on container scroll or focus leaving" | Scrolling inside menu keeps it; scrolling the sheet closes it only if trigger leaves view; focus elsewhere closes | Settings sheet scroll | G: open / W: menu scroll; container scroll (trigger visible); container scroll (trigger top 60 < menu top); reopen; focus out / T: open, open, closed, closed; b: click outside closes | a:frankmd-workspace::dropdown + b | keep |
| 7 | 109 "closes on Tab and restores the trigger before normal tab navigation" | Tab closes, focus returns to trigger, then Tab moves on normally | §9 keyboard, no traps | G: ArrowDown on trigger opens / W: press tab / T: closed; focus moved to the control after the trigger (Tab not swallowed) | b | keep |
| 8 | 119 "synchronizes native popover dismissal and closes when its dialog closes" | Closing the owning dialog also closes its popover (no orphan menu) | §5 overlays | G: Insert image dialog with Store-in select open / W: Esc closes dialog / T: popover gone with it; trigger expanded=false | a:frankmd-workspace::overlay | keep |
| 9 | 134 "closes another dropdown and removes listeners on disconnect" | Opening another dropdown closes the first; only one open | §5.5 popovers | G: dropdown A open / W: open B / T: A closed, B open; after A's owner closes, ArrowDown on A does nothing | a:frankmd-workspace::overlay | keep |

### `test/javascript/lib/markdown_line_mapper.test.js` — C=5 (keep 5 / changed 0 / dropped 0; a=5 b=0 c=0)
Original subject: `findLineAtScroll` — preview scroll position → source line (preview→editor sync). New home: scroll sync §5.21 ("real source positions"), FE-214; `workspace::scroll_sync::PreviewBlockMap` built from preview layout in content coordinates.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 240 "returns the line of the element at/above the scroll position" | Preview scroll maps to the block at/above the top | §5.21 scroll sync | G: blocks {1:0, 5:100, 10:200} / W: line_at(150), (0), (999) / T: 5, 1, 10 | a:frankmd-workspace::scroll_sync | keep |
| 2 | 248 "is invariant to non-zero container page offset" | Sync result doesn't depend on where the preview pane sits in the window | §3 layout (sidebar shown/hidden moves the pane) | G: same blocks, pane origin x/y 0 vs 100000 / W: build map from layout, line_at(150) / T: 5 both | a:frankmd-workspace::scroll_sync | keep |
| 3 | 259 "accounts for the container's current scrollTop in rect math" | Block positions are content-relative, not viewport-relative | §5.21 | G: map built while preview already scrolled 100 / W: line_at(100) / T: 5 | a:frankmd-workspace::scroll_sync | keep |
| 4 | 267 "returns first element's line when scrolled above all elements" | Above first block → first block's line | §5.21 | G: {3:50, 8:150} / W: line_at(10) / T: 3 | a:frankmd-workspace::scroll_sync | keep |
| 5 | 272 "returns null when there are no annotated elements" | Empty preview → no sync jump | §5.21 | G: empty map / W: line_at(100) / T: None; editor scroll unchanged | a:frankmd-workspace::scroll_sync | keep |

### `test/javascript/lib/menu_bounds.test.js` — C=6 (keep 5 / changed 0 / dropped 1; a=5 b=0 c=0)
Original subject: dropdown viewport clamping (`clampMenuToViewport`, `toggleMenu`). New home: menus/popovers (§3.4 ⋯ menu, explorer context menu §5.14, §5.5 clamp); existing `frankmd-workspace::menu_bounds` (`clamp_menu_to_viewport`, `toggle_menu`). Count note: contracts count the `it.each` at line 46 as ONE test (3 A + 6 C = 9), so it is one row here covering both data rows.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 35 "sets an inline max-height sized to the viewport" | Menu never extends below the window | §5.5 | G: rect top 300, vh 900 / W: clamp_menu_to_viewport / T: max_height 592 | a:frankmd-workspace::menu_bounds | keep |
| 2 | 42 "ignores missing menu elements" | (null element) | — | — | — | dropped (DOM-specific: null element) |
| 3 | 46 "shifts an overhanging menu into a 320px viewport" [rows 1–2] | Overhanging menu shifted back inside narrow window | §4.1 Narrow | G: vw 320, (left 177, right 449) and (−20, 252) / W: clamp / T: shift_x Some(−137), Some(28) | a:frankmd-workspace::menu_bounds | keep |
| 4 | 56 "resets an old horizontal shift before measuring after a resize" | Reopening after resize doesn't keep a stale shift | §4.1 | G: earlier shift −137; new anchor rect left 8 right 280 / W: clamp / T: shift_x None (rect taken at anchor, not shifted) | a:frankmd-workspace::menu_bounds | keep |
| 5 | 71 "shows a hidden menu and applies the viewport clamp" | Opening a menu places it within the window | §3.4 ⋯ menu | G: hidden, top 500, vh 900 / W: toggle_menu / T: visible true, max_height 392 | a:frankmd-workspace::menu_bounds | keep |
| 6 | 81 "hides a visible menu" | Toggling/forcing hides it | §3.4 | G: visible / W: toggle_menu(force_hidden) / T: visible false, placement None | a:frankmd-workspace::menu_bounds | keep |

### `test/javascript/lib/vim_mode.test.js` — C=8 (keep 5 / changed 3 / dropped 0; a=8 b=0 c=0)
Original subject: vim key passthrough, ex-commands, mode observation. New home: Vim mode §6.3, status-line mode block and ex line §3.6; `vim::keymap`, `vim::ex`, `vim::state`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 17 "hands the app-owned shortcuts back to FrankMD" | In vim mode Ctrl+F/N/P/E/B/I/V still run app actions | §6.3 passthrough list (+ `<C-w>`, `<C-.>`) | G: vim Normal / W: keymap.resolve(ctrl-f/n/p/e/b/i/v/w/.) / T: Passthrough each → Find, NewNote, QuickOpen, ToggleSidebar, Bold, Italic, paste, CloseNote, ContextAction; b: ctrl-p → `quick-open` visible | a:frankmd-vim::keymap + b | changed (§6.3 adds `<C-w>`, `<C-.>`; macOS branch n/a) |
| 2 | 30 "only mutates the shared keymap once" | Toggling vim off/on doesn't duplicate or lose bindings | Settings › Editor Vim mode | G: vim on→off→on / W: passthrough set / T: same set, no duplicates | a:frankmd-vim::keymap | keep |
| 3 | 39 "covers the shortcuts the app and the markdown keymap bind" | Ctrl+D/U paging and Ctrl+Q blockwise stay vim's | §6.3 | G: vim Normal / W: resolve ctrl-d, ctrl-u, ctrl-q / T: Vim motion (not app; Ctrl+Q not Quit) | a:frankmd-vim::keymap | keep |
| 4 | 51 "defines one ex-command per entry, with its abbreviation prefix" | Ex commands and abbreviations exist; `:noh` behaves | §6.3 ex list | G: ex table / W: parse each / T: write/w, quit/q, wq, xit/x, help, edit/e, find, Explore, next/n, prev, qa, set nu/rnu/nonu, noh, `:<N>` all parse; unknown → error message in status | a:frankmd-vim::ex | changed (§6.3 adds :qa, :e <name>, :set nu, :noh as app command, :<N>) |
| 5 | 63 "invokes onCommand with the logical command name when an ex-command runs" | `:w` saves, `:Explore` toggles sidebar | §6.3, ex line §3.6 | G: vim Normal / W: parse ":w", ":Explore" / T: SaveNow, ToggleSidebar; b: type ":w⏎" → status-save Saved | a:frankmd-vim::ex + b | keep |
| 6 | 76 "is idempotent — a second call does not double-register" | Each ex command runs once | §6.3 | G: ex table / W: names + prefixes / T: every name and abbreviation resolves to exactly one command; ":w" dispatches once | a:frankmd-vim::ex | keep |
| 7 | 84 "maps mode-change events to onModeChange, including visual submode" | Status shows current mode incl. visual line | Mode block §3.6 | G: vim / W: modes Normal, Insert, Visual, VisualLine, command-line / T: labels NORMAL, INSERT, VISUAL, V-LINE, COMMAND with mode colours | a:frankmd-workspace::status | changed (§3.6 mode block replaces "VIM · …" badge) |
| 8 | 101 "ignores events with no mode" | Non-mode transitions don't blank the mode display | §3.6 | G: Normal / W: pending operator `d` (no mode change) / T: label stays NORMAL | a:frankmd-vim::state | keep |

### `test/javascript/image_sources/ai_images.test.js` — C=4 (keep 4 / changed 0 / dropped 0; a=4 b=0 c=0)
Original subject: AI tab reference-image grid rendering. New home: Insert image › AI generate, reference image chooser §5.6 (FE-333…336); `workspace::insert_image`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 298 "renders empty state when no images" | Empty ref chooser says "No images found" | §5.6 AI generate | G: AI source, ref images [] / W: ref_chooser rows / T: empty state text = i18n no_images_found | a:frankmd-workspace::insert_image | keep |
| 2 | 306 "renders image buttons" | Each ref image selectable by path/name | §5.6 | G: [{photos/ref.jpg, ref.jpg}] / W: choose row 0 / T: row label "ref.jpg"; ref_image = {path photos/ref.jpg, name ref.jpg} | a:frankmd-workspace::insert_image | keep |
| 3 | 319 "marks selected reference image" | Chosen ref image is visibly selected | §5.6 (selected card style §5 lists) | G: [a.jpg, b.jpg], ref = a / W: rows / T: row a selected, b not; c: accent ring look | a:frankmd-workspace::insert_image | keep |
| 4 | 332 "escapes HTML" | Names shown literally, never as markup | §5.6 | G: name `<script>xss</script>` / W: rows / T: label == raw string (plain text element) | a:frankmd-workspace::insert_image | keep |

### `test/javascript/image_sources/folder_images.test.js` — C=15 (keep 10 / changed 1 / dropped 4; a=11 b=0 c=0)
Original subject: Folder tab via File System Access API: support check, browse, dimensions, grid, selection. New home: Insert image › Folder §5.6 ("Choose folder…" via portal, OM-011; FE-326/327); `workspace::insert_image` + `media::sources::folder`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 47 "returns true when showDirectoryPicker is available" | Folder source usable | §5.6 Folder | G: any environment / W: source availability / T: Folder = Available (no "Set up" tag) | a:frankmd-workspace::insert_image | changed (§5.6 portal chooser always available) |
| 2 | 52 "returns false when showDirectoryPicker is not available" | Unsupported notice | — | — | — | dropped (design-removed §5.6: FE-327 notice goes away) |
| 3 | 93 "returns error when API not supported" | "File System Access API not supported" | — | — | — | dropped (design-removed §5.6) |
| 4 | 101 "returns cancelled when user cancels picker" | Cancelling the folder chooser is silent, previous state kept | §5.6 Choose folder… | G: folder X loaded / W: pick result Cancelled / T: no error; folder X and grid unchanged; c: real portal cancel | a:frankmd-workspace::insert_image + c | keep |
| 5 | 114 "returns error on other errors" | Unreadable folder shows "Error accessing folder" | §5.6 errors inline in danger | G: / W: pick result Err(permission) / T: inline error i18n error_accessing_folder; grid empty | a:frankmd-workspace::insert_image | keep |
| 6 | 230 "resolves with dimensions on load" | Dimensions badge from the image file | §5.6 grid badge | G: 800×600 png fixture / W: read dims / T: (800, 600) | a:frankmd-media::sources::folder | keep |
| 7 | 237 "resolves with null dimensions on error" | Unreadable image → no badge, no failure | §5.6 | G: corrupt file / W: read dims / T: None; card dims_badge None | a:frankmd-media::sources::folder | keep |
| 8 | 254 "renders empty state when no images" | Empty folder says "No images found in folder" | §5.6 | G: folder with no images / W: cards / T: empty state i18n no_images_in_folder | a:frankmd-workspace::insert_image | keep |
| 9 | 263 "renders images with data attributes" | Card shows thumbnail, WxH badge, size; selecting it selects that file | §5.6 | G: photo.jpg 102400 B 800×600 / W: cards, select 0 / T: thumbnail = local path; badge "800x600"; size "100 KB"; selected = photo.jpg | a:frankmd-workspace::insert_image | keep |
| 10 | 284 "updates status container" | Status "2 images found" | §5.6 status line | G: 2 images / W: status / T: "2 images found" | a:frankmd-workspace::insert_image | keep |
| 11 | 297 "shows partial count when limited" | "Showing 1 most recent of 50 images" when paged | §5.6 pager | G: 1 displayed of 50 / W: status / T: "Showing 1 most recent of 50 images" | a:frankmd-workspace::insert_image | keep |
| 12 | 309 "escapes HTML in names" | File names shown literally | §5.6 | G: name `<script>xss</script>.jpg` / W: cards / T: label == raw | a:frankmd-workspace::insert_image | keep |
| 13 | 320 "does not allow a quoted filename to inject image attributes" | (HTML attribute injection) | — | — | — | dropped (DOM-specific: no attribute surface; literal label in row 12) |
| 14 | 348 "removes selected class from all items" | New search/page/folder clears the selection | §5.6 footer "Selected:" | G: card 0 selected / W: filter "cat" / T: selected None; footer selection empty, Insert disabled | a:frankmd-workspace::insert_image | keep |
| 15 | 362 "handles null container" | (null container) | — | — | — | dropped (DOM-specific) |

### `test/javascript/image_sources/google_images.test.js` — C=6 (keep 5 / changed 0 / dropped 1; a=5 b=0 c=0)
Original subject: Google results grid and selection. New home: Insert image › Google §5.6 (FE-329/331); `workspace::insert_image`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 215 "renders empty state when no results" | "No images found" | §5.6 Google | G: results [] / W: cards / T: empty state i18n no_images_found | a:frankmd-workspace::insert_image | keep |
| 2 | 224 "renders image buttons with all data attributes" | Card has full URL, thumbnail, title, source caption, 1024x768 badge; inserting uses full URL | §5.6 | G: {full.jpg, thumb.jpg, "Cat Photo", example.com, 1024×768} / W: cards, select / T: thumb = thumb.jpg; caption example.com; badge "1024x768"; insert_url full.jpg; alt default "Cat Photo" | a:frankmd-workspace::insert_image | keep |
| 3 | 247 "removes broken thumbnails without an inline error handler" | Broken thumbnails disappear from the grid | §5.6 "broken thumbnails removed" | G: 1 card / W: thumbnail_failed(card) / T: card removed; c: real 404 thumb | a:frankmd-workspace::insert_image | keep |
| 4 | 259 "escapes HTML in content" | Titles shown literally | §5.6 | G: title `<img onerror=alert(1)>` / W: cards / T: title == raw | a:frankmd-workspace::insert_image | keep |
| 5 | 272 "removes selection classes" | New results clear selection | §5.6 | G: card selected / W: load more or new search / T: selected None | a:frankmd-workspace::insert_image | keep |
| 6 | 283 "handles null container gracefully" | (null container) | — | — | — | dropped (DOM-specific) |

### `test/javascript/image_sources/local_images.test.js` — C=10 (keep 6 / changed 1 / dropped 3; a=7 b=0 c=0)
Original subject: server image-folder grid (Library). New home: Insert image › Library §5.6 (FE-322…325); `workspace::insert_image`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 123 "renders empty state when no images" | "No images found" | §5.6 Library | G: [] / W: cards / T: empty state no_images_found | a:frankmd-workspace::insert_image | keep |
| 2 | 132 "renders null images as empty" | (null list) | — | — | — | dropped (DOM-specific: null input) |
| 3 | 140 "renders image grid items" | Card shows name, 800x600 badge; selects by path | §5.6 | G: photos/cat.jpg 800×600 / W: cards, select / T: label cat.jpg; badge "800x600"; selected path photos/cat.jpg | a:frankmd-workspace::insert_image | keep |
| 4 | 155 "marks selected image" | Selected card highlighted, others not | §5.6 | G: cat & dog, selected cat / W: cards / T: cat.is_selected only | a:frankmd-workspace::insert_image | keep |
| 5 | 168 "generates preview URL with encoded path" | Thumbnails load for paths with spaces | §5.6 native thumbnails | G: photos/my cat.jpg / W: card thumb / T: thumbnail = images_path.join("photos/my cat.jpg") (no URL encoding needed) | a:frankmd-workspace::insert_image | changed (native file thumbnail; no /images/preview HTTP route) |
| 6 | 179 "escapes HTML in names" | Names shown literally | §5.6 | G: `<script>alert('xss')</script>` / W: cards / T: label == raw | a:frankmd-workspace::insert_image | keep |
| 7 | 190 "does not allow a quoted filename to inject image attributes" | (attribute injection) | — | — | — | dropped (DOM-specific) |
| 8 | 197 "handles images without dimensions" | No badge when dims unknown; tooltip duplicates name | §5.6; §9 tooltips duplicate labels | G: test.jpg no dims / W: cards / T: dims_badge None; tooltip "test.jpg" | a:frankmd-workspace::insert_image | keep |
| 9 | 211 "removes selected class from all items" | Search/page change clears selection | §5.6 | G: selected / W: new search "sun" / T: selected None | a:frankmd-workspace::insert_image | keep |
| 10 | 228 "handles null container" | (null container) | — | — | — | dropped (DOM-specific) |

### `test/javascript/image_sources/pinterest_images.test.js` — C=7 (keep 6 / changed 0 / dropped 1; a=6 b=0 c=0)
Original subject: Pinterest results grid and selection. New home: Insert image › Pinterest §5.6 (FE-332); `workspace::insert_image`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 98 "renders empty state when no results" | "No images found" | §5.6 Pinterest | G: [] / W: cards / T: empty state | a:frankmd-workspace::insert_image | keep |
| 2 | 107 "renders Pinterest image buttons" | Card fields + 600x800 badge | §5.6 | G: {full, thumb, "DIY Project", pinterest.com, 600×800} / W: cards / T: thumb, caption, title, badge "600x800", insert_url full | a:frankmd-workspace::insert_image | keep |
| 3 | 129 "removes broken thumbnails without an inline error handler" | Broken thumbnails removed | §5.6 | G: 1 card / W: thumbnail_failed / T: removed | a:frankmd-workspace::insert_image | keep |
| 4 | 141 "handles missing optional fields" | Missing title/source → card with no caption, empty default alt | §5.6 footer Alt | G: {url only} / W: cards, select / T: title "", caption None; alt prefill "" | a:frankmd-workspace::insert_image | keep |
| 5 | 154 "escapes HTML in titles" | Titles shown literally | §5.6 | G: title `<script>alert('xss')</script>` / W: cards / T: title == raw | a:frankmd-workspace::insert_image | keep |
| 6 | 168 "removes selection classes from all items" | New search clears selection | §5.6 | G: selected / W: new search / T: selected None | a:frankmd-workspace::insert_image | keep |
| 7 | 185 "handles null container" | (null container) | — | — | — | dropped (DOM-specific) |

### `test/javascript/image_sources/web_images.test.js` — C=10 (keep 8 / changed 0 / dropped 2; a=8 b=0 c=0)
Original subject: Web search results grid and selection. New home: Insert image › Web §5.6 (FE-328/329/330); `workspace::insert_image`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 139 "renders empty state when no results" | "No images found" | §5.6 Web | G: [] / W: cards / T: empty state | a:frankmd-workspace::insert_image | keep |
| 2 | 148 "renders image buttons" | Card has url, title, source caption | §5.6 | G: {1.jpg, "Cat", example.com} / W: cards / T: insert_url 1.jpg; title Cat; caption example.com | a:frankmd-workspace::insert_image | keep |
| 3 | 162 "removes broken thumbnails without an inline error handler" | Broken thumbnails removed | §5.6 | G: 1 card / W: thumbnail_failed / T: removed | a:frankmd-workspace::insert_image | keep |
| 4 | 174 "renders dimensions when available" | 800x600 badge | §5.6 | G: 800×600 / W: cards / T: badge "800x600" | a:frankmd-workspace::insert_image | keep |
| 5 | 185 "uses thumbnail when available" | Grid shows the thumbnail URL, insert keeps full URL | §5.6 | G: {full, thumb} / W: cards / T: thumb = thumb.jpg; insert_url = full.jpg; no thumb → thumb = url | a:frankmd-workspace::insert_image | keep |
| 6 | 197 "escapes HTML in titles" | Titles shown literally | §5.6 | G: `<script>…` / W: cards / T: title == raw | a:frankmd-workspace::insert_image | keep |
| 7 | 209 "does not allow a quoted title to inject attributes" | (attribute injection) | — | — | — | dropped (DOM-specific) |
| 8 | 218 "rejects unsafe image source URLs" | Unsafe URLs (javascript:, //host) are never fetched as thumbnails | §5.6; isSafeImageUrl parity | G: url `javascript:alert(1)` / W: cards / T: thumbnail None (placeholder), no fetch issued | a:frankmd-workspace::insert_image | keep |
| 9 | 229 "removes selection classes from all items" | New search clears selection | §5.6 | G: selected / W: new search / T: selected None | a:frankmd-workspace::insert_image | keep |
| 10 | 246 "handles null container" | (null container) | — | — | — | dropped (DOM-specific) |

#### New VM requirements (G2: JS lib + image_sources)
- `frankmd-workspace::dialogs` — `PromptQueue` details: `PromptKind {Alert{msg}, Confirm{msg, accept_label, destructive}, TextPrompt{label, default}}`, ordered `buttons()` (secondary then primary, primary role Danger when destructive), `initial_focus()` (Cancel for destructive), `PromptResult {Ack, Bool, Text(String), Cancelled}`, Enter/Ctrl+Enter accept in text input (app_prompt 1–7).
- `frankmd-workspace::overlay` — a prompt dialog stacked over a parent dialog (parent stays open, focus returns into it); popovers owned by a dialog close with it; at most one popover open (app_prompt 4, 9; floating_dropdown 8, 9).
- `frankmd-workspace::editor_view` (new) — `EditorViewConfig {placeholder, fonts, line_numbers, read_only, vim, typewriter}`; `gutter_labels(mode, total_lines, caret_line) -> Option<Vec<String>>`; live reconfigure (theme, line numbers, read-only) keeping doc/caret/undo depth (codemirror_extensions 4–18).
- `frankmd-workspace::preview_render` (new) — render generation counter; off-thread highlight results tagged with generation and dropped when stale (code_block_highlighter 2).
- `frankmd-markdown::highlight` — synchronous highlighting for all bundled languages within one render pass, same limits as A tests (code_block_highlighter 1).
- `frankmd-workspace::slash_menu` (new) — `SlashMenuModel`: rows `{CommandId, label, icon}`, select up/down, `accept() -> SlashOutcome {Edit(EditResult), OpenDialog{kind, from, to, query}}`, `dismiss()` keeps the query, `visible_rows()` ≤ 8 at 28 px (slash 1–6, theme 4).
- `frankmd-workspace::dropdown` (new) — `DropdownState`: `open()` focuses the checked option, `move(dir)` skips disabled, Home/End, `escape()`/`tab()` close and return focus to trigger (Tab not consumed), `on_resize(trigger_rect, viewport)`, `on_container_scroll(trigger_rect, menu_rect)`, `on_focus_out()`, `keep_visible(option_rect, max_h) -> scroll_top` (floating_dropdown 1, 4–7).
- `frankmd-workspace::menu_bounds` — add `place_popover(trigger, menu_size, content_h, viewport, gutter=8) -> {top, left, max_height, max_width, flipped}` with NO 96 px minimum (unlike `menu_max_height`) (floating_dropdown 2, 3).
- `frankmd-workspace::typewriter` — `ScrollCause {PointerClick, PointerRange, Keyboard, DocChange}`; `recenter_target(cause, typewriter, recenter_on_click, Geometry{scroll_top, caret_top, client_h, scroll_h}) -> Option<f32>` clamped to `scroll_h − client_h`; `bottom_padding(client_h) = client_h/2` (0 when off/hidden); `refresh(geometry)` after visibility change (typewriter 17–28).
- `frankmd-workspace::scroll_sync` — `PreviewBlockMap` built from preview layout in content coordinates (independent of pane origin and current scroll); `line_at(scroll_top) -> Option<u32>` (markdown_line_mapper 1–5; editor scroll input codemirror_extensions 13).
- `frankmd-workspace::status` — indicators include Typewriter; mode block labels NORMAL/INSERT/VISUAL/V-LINE/COMMAND from `vim::state` (typewriter 6–7, vim_mode 7); position/context chip update on caret move (codemirror_extensions 11–12).
- `frankmd-workspace::settings_vm` — Writing rows Typewriter and Recenter caret on click, two-way bound to live state (typewriter 13, 15–16).
- `frankmd-markdown::edit` — `surround_on_type(text, &[SelRange{anchor, head}], ch, InputCtx{source: Typed|Paste, composing, read_only}) -> Option<EditResult>` handling several ranges, keeping backward ranges, linear time, one undo group per call (surround 1–14).
- `frankmd-vim::keymap` — passthrough set `ctrl-f n p e b i v w .`, excluding `ctrl-d/u/q`; idempotent (vim_mode 1–3; vim ext 4). `frankmd-vim::ex` — table name+abbrev → `AppCommand`, no duplicates, `:noh`, `:qa`, `:set nu/rnu/nonu`, `:<N>` (vim_mode 4–6). `frankmd-vim::state` — mode + visual submode, cursor shape Block/Bar (vim ext 1–3, vim_mode 8).
- `frankmd-theme::tokens` — `EditorStyle::from(theme, fonts)` covering chrome roles + markdown/syntax highlight map; documented font defaults; fonts kept across theme switch (codemirror_theme 1–3, 5, 7–8).
- `frankmd-workspace::insert_image` — `ImageCard {id, label, title, caption, thumbnail: Option<ThumbSource{Path|Url}>, insert_url|path, dims_badge: Option<String>, size_label, is_selected}`; `thumbnail_failed(id)` removes card; selection cleared on new search/page/folder; status strings ("N images found", "Showing N most recent of M images"); unsafe-URL thumbnails suppressed; AI ref chooser rows + `ref_image`; folder pick `Cancelled | Err` handling (all image_sources rows).
- `frankmd-media::sources::folder` — image dimensions from file header (None on failure); size label formatting ("100 KB") (folder 6, 7, 9).

#### Open questions (G2: JS lib + image_sources)
- Slash menu count: §5.5 says "12 commands (FE-089/090)", but FE-089 (10 block commands) + FE-090 (Table, Image, Video, Emoji) = 14, and the original test expects 14 rows. Which is it?
- Destructive confirm focus: FrankMD focuses the dialog itself (neither button). §5.13 says focus starts on Cancel for Delete. Does Cancel-first apply to every destructive generic confirm, and where does focus start for non-destructive confirms?
- Nested prompts: §2 says "Overlays (one at a time)", but FrankMD opens prompts over open dialogs (e.g. library delete inside Insert image). Confirm the overlay model allows a prompt stacked on its parent dialog.
- Text prompt accept key: §5 says Ctrl+Enter confirms every dialog. Should plain Enter in a single-line prompt input keep accepting (FrankMD behaviour)?
- Completion popover height: FrankMD lets the list grow to 16em. §5.5 caps it at 8 rows × 28 px. Does the same cap apply to wikilink and link completion?
- Vim `Ctrl+Q`: §6.1 binds Quit to Ctrl+Q, while §6.3 leaves `<C-q>` (blockwise visual) to vim. Confirm that vim mode suppresses the Quit binding entirely. Also, `:q` is described as "close menus/overlays (kept)": should it close the note when no overlay is open?
- Typewriter details: FE-082 has a >5 px threshold and skips during mouse selection, and §7.6 gives a 120 ms ease-out. The spec doesn't say whether the threshold stays, or whether the animation is skipped when the target is within the threshold.
- Folder source fallback: what happens when no FileChooser portal is present (non-portal WMs)? A gpui path prompt, or a "Set up" tag?
- Web/Google/Pinterest thumbnails: they now go through frankmd-net egress (fetched natively, not hotlinked by a browser). Is a thumbnail fetch failure treated as "broken" (card removed) or retried?
- Dimension badge text: should it stay "800x600" (ASCII x, parity) or become "800×600"?
- Line-number persistence: should `.fed` keep storing 0/1/2 even though Settings shows Off/Absolute/Relative? The spec is silent; parity suggests numeric storage.
- Count note: menu_bounds' `it.each` (line 46, 2 rows) is counted as one test in test-contracts (total 9, C=6). That conflicts with the part-3 rule that `it.each` rows are expanded (which would give C=7). This plan follows the contracts count and uses one row with `[rows 1–2]`.

## 8. G3: JS `app_*` controllers and ai_grammar

### `test/javascript/controllers/ai_grammar_controller.test.js` — C=12 (keep 7 / changed 5 / dropped 0; a=12 b=0 c=0)
Original subject: AI grammar dialog controller (availability, open/close, diff/edit toggle, accept, cleanup). New home: grammar banner + diff modal §5.10, palette availability §5.1, Settings › Integrations §5.16. (Rows 136/155/176/191, 75/92/107 are B; escapeHtml/renderDiff* are A.)

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 61 "initializes AI state as disabled" | Before config is known, grammar check is not offered as ready | Grammar check §5.10 | G: fresh `GrammarState` / W: nothing / T: availability = Unknown, provider/model None, no banner, not in flight | a:frankmd-workspace::grammar | keep |
| 2 | 67 "calls checkAiAvailability on connect" | AI availability is determined once at startup | §5.10; §10.2 BE-077 | G: resolved config with/without AI key / W: `GrammarState::from_config` / T: Available{provider,model} or NotConfigured; no network call made | a:frankmd-workspace::grammar | changed (§10.2 no HTTP; availability derived from resolved config) |
| 3 | 119 "shows config notice when AI is not enabled" | Unconfigured AI explains itself instead of failing | §5.10 Not configured; §5.22 | G: NotConfigured / W: GrammarCheck / T: dialog state = NotConfigured with provider buttons → OpenSettings{Integrations, provider}; no request, no read-only lock | a:frankmd-workspace::grammar + b | changed (§5.10 in-place "needs an AI provider" + Settings link) |
| 4 | 128 "uses the shared app alert when no file path is provided" | Grammar check needs an open note | Palette availability §5.1 | G: no note open / W: `availability(GrammarCheck)` and dispatch via Ctrl+Shift+G / T: Unavailable("Open a markdown note first"); status-line info message; no dialog | a:frankmd-workspace::commands | changed (§5.1 hidden/unavailable + status line replaces alert) |
| 5 | 212 "closes the dialog" | Cancel/Esc closes the result modal without touching the note | §5.10 footer | G: result shown / W: cancel() / T: overlay closed, document text unchanged, read-only lock released | a:frankmd-workspace::grammar + b | keep |
| 6 | 219 "switches from diff view to edit view" | User can hand-edit the corrected text | §5.10 Edit toggle Alt+E | G: Diff mode / W: toggle_edit() / T: mode = Edit, editable text = corrected (with accepted hunks), toggle label = "Preview" | a:frankmd-workspace::grammar | keep |
| 7 | 230 "switches from edit view to diff view" | Back to diff keeps edits | §5.10 | G: Edit mode with edited text / W: toggle_edit() / T: mode = Diff, label = "Edit", edited text retained | a:frankmd-workspace::grammar | keep |
| 8 | 243 "dispatches accepted event with corrected text" | Applying puts the corrected (possibly edited) text into the note | §5.10 Apply, per-hunk accept | G: all hunks accepted (default) or edited text / W: apply() / T: returns one `Edit` replacing doc with corrected/edited text; one undo step | a:frankmd-workspace::grammar | changed (§5.10 per-hunk accept/reject; default all accepted = old result) |
| 9 | 254 "closes the dialog after accepting" | Modal closes after apply | §5.10 | G: result shown / W: apply() / T: overlay closed, focus returns to editor | a:frankmd-workspace::grammar + b | keep |
| 10 | 262 "hides processing overlay" | Running indicator disappears when the request ends | §5.10 banner; §5.12 | G: in flight (banner "Checking grammar…") / W: finish(ok or err) / T: banner removed from BannerStack, AI indicator off | a:frankmd-workspace::grammar | changed (§5.10 pane banner replaces full-screen overlay) |
| 11 | 270 "dispatches processing-ended event" | Note becomes editable again after the check | §5.10 read-only lock | G: in flight, editor locked / W: finish / T: read-only lock released; GrammarCheck available again | a:frankmd-workspace::grammar | keep |
| 12 | 278 "clears abort controller" | After completion Cancel is a no-op and a new check can start | §5.10 Cancel Esc | G: finished request / W: cancel() then start() / T: cancel no-op; new request handle issued (old handle None) | a:frankmd-workspace::grammar | keep |

### `test/javascript/controllers/app_editor_focus.test.js` — C=1 (keep 1 / changed 0 / dropped 0; a=1 b=0 c=0)
Original subject: `showEditor` focuses the editor only for empty content. New home: navigation open flow + focus model §4.3.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 13 "focuses the editor only when loaded content is empty (%j)" [rows 1–3] | Opening an empty (new) note puts the caret in it; opening a non-empty one (incl. whitespace) leaves focus in the explorer | Navigation §4.3 | G: explorer focused / W: open note with "note content", " ", "" / T: `OpenOutcome.focus_editor` false,false,true; b: focused region stays Sidebar vs Editor | a:frankmd-workspace::navigation + b | keep |

### `test/javascript/controllers/app_explorer_background.test.js` — C=6 (keep 5 / changed 1 / dropped 0; a=4 b=2 c=0)
Original subject: explorer empty-surface clicks, context menu, select-all, Escape (#261, #267). New home: Explorer §5.14 Background row, Esc chain §6.1.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 48 "clears the selection when clicking the explorer's empty surface" | Left-click on empty tree space clears multi-selection | §5.14 Background | G: 2 rows selected, anchor set / W: click_background() / T: selection empty, anchor None; b: click `explorer-tree` below last row | a:frankmd-workspace::explorer + b | keep |
| 2 | 61 "shows the positioned menu on the background click" | Background context menu opens at the pointer | §5.14 Background (right-click) | G: tree / W: right-click empty space at (x,y) / T: menu open, anchored at (x,y), items New note, New folder, Select all, Collapse all; left-click opens no menu | a:frankmd-workspace::explorer + b | changed (§5.14 left-click only clears; menu on right-click) |
| 3 | 72 "ignores clicks that land on tree items" | Clicking a row is not a background click | §5.14 | G: selection S, menu closed / W: click on `explorer-row-a.md` / T: background handler not hit; menu stays closed; selection follows row-click rules only | b | keep |
| 4 | 84 "hides the menu when an explorer item is selected" | Selecting a row dismisses the background menu | §5.14; §5 popover rules | G: background menu open / W: select row / T: menu closed | a:frankmd-workspace::explorer | keep |
| 5 | 94 "select all picks every visible item and skips collapsed children" | Select all selects only visible rows | §5.14 Ctrl+A / menu Select all | G: tree a.md, closed/(b.md), c.md, docs/ / W: select_all_visible() / T: selection = [a.md, c.md, docs] (closed folder row included if visible; children skipped); menu closed | a:frankmd-workspace::explorer | keep |
| 6 | 106 "Escape closes the open explorer menu" | Esc closes the menu first and is consumed | Esc chain §6.1 | G: explorer menu open + selection / W: Escape / T: `escape_target` = Popover; menu closed, selection kept; b: press escape, menu gone | b | keep |

### `test/javascript/controllers/app_explorer_selection.test.js` — C=15 (keep 13 / changed 2 / dropped 0; a=14 b=1 c=0)
Original subject: explorer multi-selection model, folder toggle, Delete/F2 keys. New home: Explorer §5.14, Delete/Rename §5.13.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 67 "selects and toggles a folder without opening a note" | Clicking a folder selects + expands it, opens nothing | §5.14 Mouse | G: collapsed docs/ / W: click(docs) / T: selection=[docs], focused=docs, docs in expansion set (persisted), children visible, effect ≠ OpenNote | a:frankmd-workspace::explorer | keep |
| 2 | 85 "keeps the open-file marker independent from the selection background" | Open-note marker and selection are separate | §3.3 open-note accent bar | G: open note a.md / W: click(docs) / T: open marker still a.md; a.md not selected; docs selected+focused | a:frankmd-workspace::explorer | keep |
| 3 | 101 "replaces the selection on a plain click" | Plain click selects only that row | §5.14 Mouse | G: [a,b] selected / W: click(c) / T: selection=[c], focused=c, anchor=c | a:frankmd-workspace::explorer | keep |
| 4 | 117 "toggles selection with %s" [rows 1–2 ctrlKey/metaKey] | Ctrl+click toggles membership; focus falls back on toggle-off | §5.14 Ctrl+click | G: [a] / W: click(b, ctrl) then click(b, ctrl) / T: [a,b] then [a], focused falls back to a; meta row = platform secondary modifier | a:frankmd-workspace::explorer | keep |
| 5 | 135 "selects visible Shift ranges and adds a range with Ctrl+Shift" | Shift ranges visible rows only; Ctrl+Shift adds range | §5.14 Shift+click | G: anchor a.md; closed/ collapsed / W: click(c.md, shift) / T: [a.md, closed, b.md, c.md] (hidden children skipped); ctrl+shift adds range to existing | a:frankmd-workspace::explorer | keep |
| 6 | 160 "does not collapse modifier selection when pointer focus fires first" | Focus caused by a modifier click doesn't reset selection | §5.14 | G: [a,b] via ctrl-click / W: on_focus(b, FocusCause::Pointer) / T: selection unchanged; b: ctrl-click row keeps 2 selected | a:frankmd-workspace::explorer + b | keep |
| 7 | 175 "selects one row when keyboard focus moves to it" | Keyboard focus move selects just that row | §5.14 ↑/↓ | G: [a,b] / W: on_focus(c, Keyboard) or Down / T: selection=[c] | a:frankmd-workspace::explorer | keep |
| 8 | 188 "selects a modifier-clicked file without opening it" | Shift-click selects but doesn't open | §5.14 | G: no note open / W: click(x.md, shift) / T: x.md selected, no OpenNote effect | a:frankmd-workspace::explorer | keep |
| 9 | 199 "remaps selected folders and descendants after a rename or move" | Selection survives rename/move | §5.14 | G: sel {old-folder, old-folder/note.md, outside.md}, anchor/focus inside / W: remap("old-folder","new-folder",Folder) / T: entries/anchor/focus remapped; prefix rule equal or "old/"; outside kept | a:frankmd-workspace::explorer | keep |
| 10 | 224 "removes selected descendants when a folder is deleted" | Deleted subtree leaves selection | §5.14 | G: sel {docs, docs/a.md, outside.md}, anchor docs / W: remove("docs",Folder) / T: sel=[outside.md], anchor None, focus=outside.md | a:frankmd-workspace::explorer | keep |
| 11 | 245 "routes Delete for selected files and folders through the bulk delete action" | Delete key deletes the whole selection (with confirm) | §5.14 Delete; §5.13 Delete confirm | G: sel [note.md, docs] (order) / W: Delete in explorer / T: DeleteNote with targets in selection order → `DeleteConfirm` lists counts (1 note, 1 folder); b: key consumed once | a:frankmd-workspace::explorer + b | changed (§5.13 confirm dialog, trash wording) |
| 12 | 264 "renames only a single selected eligible row with F2" | F2 renames exactly one selected row | §5.13 Rename inline in explorer row | G: one row selected / W: F2 / T: inline rename starts on that row (InlineRename for path); with 2 selected nothing happens | a:frankmd-workspace::explorer | changed (§5.13 inline rename replaces dialog) |
| 13 | 280 "does not delete when selection is empty, outside the Explorer, or only protected config" | Delete never hits `.fed` or fires outside the tree | §5.14; keymap context | G: empty sel / sel [.fed] / focus in editor / W: Delete / T: `delete_targets()` empty → no dialog; b: Delete in editor edits text, not files | a:frankmd-workspace::explorer + b | keep |
| 14 | 298 "does not rename config, unselected, or multiply selected rows" | F2 refuses config/none/multi | §5.13 | G: [.fed] / [] / [a,b] / W: rename_target() / T: None in all cases | a:frankmd-workspace::explorer | keep |
| 15 | 313 "does not intercept other keys or modified shortcut presses" | Backspace, Ctrl+Delete, Shift+F2, auto-repeat don't delete/rename | §5.14 keymap | G: selection, explorer focused / W: press backspace, ctrl-delete, shift-f2, held delete/F2 repeat / T: no DeleteConfirm, no inline rename | b | keep |

### `test/javascript/controllers/app_file_rename.test.js` — C=4 (keep 4 / changed 0 / dropped 0; a=4 b=0 c=0)
Original subject: `onFileRenamed`/`onFileDeleted` keeping current note and autosave in sync. New home: navigation + autosave state §5.13, empty state §5.2. (expandedFolders/currentFile remap rules are A.)

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 120 "updates autosave when the active file is renamed" | Renaming the open note keeps editing it under the new name; pending save goes to new path | §5.13 Rename; §3.4 title | G: open foo.md dirty / W: on_renamed(foo.md→bar.md, File) / T: current = bar.md, autosave target = bar.md, next save writes bar.md (never foo.md), title "bar" | a:frankmd-workspace::navigation | keep |
| 2 | 131 "updates autosave when the active file's folder is renamed" | Renaming the open note's folder follows it | §5.13 | G: open docs/foo.md / W: on_renamed(docs→archive, Folder) / T: current = archive/foo.md, autosave re-targeted, breadcrumb "archive" | a:frankmd-workspace::navigation | keep |
| 3 | 142 "clears autosave when the active file is deleted" | Deleting the open note closes it cleanly | §5.13 Delete; §5.2 empty state | G: open foo.md, pending slash insertion / W: on_deleted(foo.md, File) / T: current None, pending insertion cleared, draft for foo.md discarded, empty state shown, window title "FrankMD — <vault>" | a:frankmd-workspace::navigation | keep |
| 4 | 174 "clears the active editor after a successful deletion without flushing a deleted path" | No draft/save is written back to a deleted note | §5.13 | G: open foo.md dirty / W: on_deleted / T: no prepare_transition/save effect for foo.md; draft deleted; empty state | a:frankmd-workspace::navigation | keep |

### `test/javascript/controllers/app_item_moved_reveal.test.js` — C=3 (keep 3 / changed 0 / dropped 0; a=3 b=0 c=0)
Original subject: `onItemMoved` reveals the destination folder (#254). New home: Explorer drag & drop "a drop always reveals the destination" §5.14; expansion persisted §5.14.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 63 "reveals a collapsed destination folder and records the expansion" | Moved note is visible in its new collapsed folder | §5.14 Drag & drop, Expansion state | G: target/ collapsed / W: on_moved(note.md→target/note.md) / T: target expanded, in persisted expansion set, row target/note.md visible | a:frankmd-workspace::explorer | keep |
| 2 | 75 "reveals every ancestor of a nested destination" | All ancestors expand | §5.14 | G: outer/, outer/inner collapsed / W: on_moved(→outer/inner/moved.md) / T: outer and outer/inner expanded | a:frankmd-workspace::explorer | keep |
| 3 | 89 "keeps the current file reference in sync without touching the tree shape" | Moving the open note keeps it open at new path | §3.4 breadcrumb; navigation | G: open moved.md / W: on_moved(moved.md→target/moved.md) / T: current = target/moved.md, history entry rewritten, breadcrumb "target / moved"; other expansion unchanged | a:frankmd-workspace::navigation | keep |

### `test/javascript/controllers/app_library.test.js` — C=6 (keep 5 / changed 1 / dropped 0; a=6 b=0 c=0)
Original subject: Library workspace toggle and media insertion into the open note. New home: Media library view §4.2, §5.15. (Path guard, root-image relative path, video embed, S3 URL rows are A.)

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 52 "opens Library while preserving the sidebar and restores the previous preview state on Explorer navigation" | Library replaces editor+preview, sidebar stays; opening a note returns and restores preview | §4.2 Media library | G: preview visible / W: ToggleMediaLibrary then open folder/selected.md from explorer / T: library open, sidebar visible, panes hidden, toggle ✓; then library closed, editor shown, preview visible, toggle unchecked; b: `media-library` visible then gone | a:frankmd-workspace::layout + b | keep |
| 2 | 79 "restores a hidden preview as hidden when closing Library" | Hidden preview stays hidden on return | §4.2 | G: preview hidden, library open / W: ToggleMediaLibrary / T: preview hidden, editor visible, toggle unchecked | a:frankmd-workspace::layout | keep |
| 3 | 91 "refreshes media and usage once each time Library opens" | Library content/usage refresh once per open, not per repeated call | §5.15 | G: library closed / W: open, open (no-op), close, open / T: reload+usage-reset effects emitted exactly 2 times total | a:frankmd-workspace::media_library | keep |
| 4 | 149 "hides and restores a visible preview through its controller" | Leaving via Esc/toggle restores a visible preview | §4.2 Esc returns | G: preview visible / W: open library, Escape / T: preview hidden while open; visible after; `escape_target` = MediaLibrary | a:frankmd-workspace::layout | keep |
| 5 | 180 "inserts an image with a note-relative encoded path and returns to the editor" | Insert from a card puts a note-relative link at the caret and returns to editing | §5.15 card Insert | G: open folder/sub/note.md, card images/photo (one).png / W: insert(card) / T: Edit inserts `![photo one.png](../../images/photo%20%28one%29.png)`, status Inserted, note dirty, library closed, focus Editor | a:frankmd-workspace::media_library | keep |
| 6 | 291 "explains when no note is open or the open file is not Markdown" | Insert explains why it can't run | §5.15; §3.6 status line | G: no note / `.fed` open / W: insert(card) / T: status NoOpenNote / MarkdownNoteRequired; status-line message `library.no_open_note` / `library.markdown_note_required`; no edit | a:frankmd-workspace::media_library | changed (§3.6 status line replaces alert) |

### `test/javascript/controllers/app_navigation.test.js` — C=33 (keep 27 / changed 6 / dropped 0; a=31 b=2 c=0)
Original subject: note navigation (generation guard, guarded transitions, history) and created-note undo/redo boundaries. New home: `workspace::navigation`, created-note boundaries (`workspace::created_notes`), Undo-creation dialog §5.13, back/forward §6.1, CLI open §10.1 002–003, empty state §5.2, status line §3.6.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 59 "Home cancels pending navigation and keeps an outgoing draft protected" | Closing to the empty state cancels a pending open; refused if the draft can't be stored | §5.23 Ctrl+W; §5.2 | G: load b.md in flight / W: close_note(), then late b.md result / T: current None, result ignored; G': prepare fails / T': close refused, stays on dirty.md, "Draft not stored" state | a:frankmd-workspace::navigation | changed (§5.23 Close note replaces Home/URL) |
| 2 | 82 "applies only the latest file request when responses resolve out of order" | Fast clicking ends on the last note | navigation generation (HP-003) | G: open a,b,c / W: results resolve c,a,b / T: only c applied once; prepare_transition once | a:frankmd-workspace::navigation | keep |
| 3 | 105 "selects the newly opened note in the Explorer without moving focus" | Opened note becomes the explorer selection; focus unchanged | §4.1 reveal rule | G: sel [x,y], focus Editor / W: apply_loaded(n.md) / T: explorer selection=[n.md], focused row n.md; focus region unchanged; sidebar visibility unchanged | a:frankmd-workspace::explorer | keep |
| 4 | 134 "ignores stale 404 and error responses without changing visible state or URL" | Superseded failures don't show "not found" | §5.22 Note not found | G: open a then b / W: a fails (NotFound/IO) after b / T: no status message, no quick-open prefill, current b | a:frankmd-workspace::navigation | keep |
| 5 | 157 "does not apply a delayed tree response after another file is deleted" | A stale tree snapshot never overwrites a newer one | Explorer tree from vault scan/watch | G: scan gen 1 in flight / W: delete other.md (gen 2), then gen 1 result / T: gen 1 discarded; tree lacks other.md | a:frankmd-workspace::explorer | changed (no Turbo/HTTP; scan generation) |
| 6 | 178 "restores every selected Explorer row after refreshing the tree" | Refresh keeps multi-selection and anchor | §5.14 | G: sel [open.md, second.md], anchor second / W: tree refreshed with same rows / T: both still selected, anchor second | a:frankmd-workspace::explorer | keep |
| 7 | 203 "keeps the current editor and URL when the outgoing draft cannot be stored" | Navigation aborts rather than lose edits | §3.6 Draft not stored | G: on a.md, prepare → Err / W: open(b.md) / T: current a.md, editor text unchanged, title unchanged, save state DraftNotStored | a:frankmd-workspace::navigation | keep |
| 8 | 218 "restores the current URL when a browser history transition cannot flush the draft" | Back/Forward aborts when draft can't be stored | §6.1 Back/Forward | G: on a.md with back history [x.md], prepare → Err / W: NavigateBack / T: stays a.md, history cursor unchanged, empty state not shown | a:frankmd-workspace::navigation | changed (§6.1 in-app history replaces popstate) |
| 9 | 232 "routes initial server-provided note data through the guarded transition" | Startup/CLI note goes through the same guarded open | §10.1 002–003 `frankmd open` | G: startup with path studies/x.md / W: startup open / T: prepare once, editor loaded markdown with revision, explorer refreshed + revealed | a:frankmd-workspace::startup | changed (CLI/IPC replaces server-provided note) |
| 10 | 251 "records a created note's exact server baseline and the previously active note" | Undo-creation knows what was created and where to return | §5.13 Undo note creation | G: on prev.md / W: on_created(created.md, content, revision) + open / T: boundary {path, initial_content exact, initial_revision, previous=prev.md}; created.md open | a:frankmd-workspace::created_notes | keep |
| 11 | 270 "remaps file creation boundaries and editor paths after a rename" | Undo/redo still works after rename | §5.13 | G: boundary created.md, dependent previous=created.md, stale deleted boundary at dest / W: on_renamed(created→renamed) / T: boundary re-keyed+path, dependents remapped, stale dest boundary dropped, editor history paths remapped | a:frankmd-workspace::created_notes | keep |
| 12 | 308 "remaps folder descendants and boundary predecessor paths after a move" | Folder move keeps open note and boundaries | §5.13 Move | G: open old-folder/nested/open.md, boundaries under old-folder / W: on_moved(old-folder→new-folder, Folder) / T: current, boundary keys/paths/previous remapped; history remapped (Folder) | a:frankmd-workspace::created_notes | keep |
| 13 | 337 "evicts ordinary deletion history and boundaries for the deleted subtree" | Deleted subtree can't be undone into | §5.13 | G: boundaries in/out of deleted-folder / W: on_deleted(deleted-folder, Folder) / T: history evicted for subtree; inside boundaries removed; predecessors inside → previous None; others untouched | a:frankmd-workspace::created_notes | keep |
| 14 | 365 "does not record a creation boundary when its note load becomes stale" | Superseded create-open records nothing | §5.13 | G: create x.md, then open y.md before x loads / W: x load completes / T: no boundary for x | a:frankmd-workspace::created_notes | keep |
| 15 | 385 "does not record a creation boundary without both server content and revision" | Boundary requires a known baseline | §5.13 | G: created file read lacks content or revision / W: on_created / T: no boundary | a:frankmd-workspace::created_notes | keep |
| 16 | 396 "does not record a creation boundary when the new note cannot be loaded" | Failed create-open records nothing | §5.13 | G: created file read → NotFound / W: on_created / T: no boundary; "note not found" handling | a:frankmd-workspace::created_notes | keep |
| 17 | 406 "confirms and deletes a created note with autosave's latest known revision, then opens its predecessor" | Ctrl+Z at history start offers to delete the new note, then returns to the previous one | §5.13 Undo note creation | G: created.md content == baseline / W: undo_at_history_start → confirm / T: `UndoCreation` dialog "Delete the note you just created?" + chip created.md; on confirm: drain autosave, lock+unlock, delete with expected revision = latest own save, boundary deleted=true, history kept, draft dropped, opens previous.md; b: Ctrl+Enter confirms `dialog-undo-creation` | a:frankmd-workspace::created_notes + b | changed (§5.13 dialog text) |
| 18 | 464 "does nothing when the user cancels the created-note deletion prompt" | Cancel leaves everything | §5.13 | G: dialog open / W: cancel / T: no delete effect, stays on created.md, boundary intact | a:frankmd-workspace::created_notes | keep |
| 19 | 496 "closes the created-note prompt from the shared dialog header" | Header × / Esc == Cancel | §5 dialog anatomy | G: `dialog-undo-creation` open / W: click × (and Escape) / T: dialog closed, file still exists, editor focused | b | keep |
| 20 | 526 "does not intercept exhausted undo for an ordinary note or non-baseline content" | Ordinary Ctrl+Z at start is untouched | §6.2 Undo | G: ordinary note / created note with changed content / W: undo_at_history_start / T: returns NotHandled; no dialog | a:frankmd-workspace::created_notes | keep |
| 21 | 544 "retains the creation boundary and resumes autosave if guarded deletion conflicts" | Conflict on delete is reported, nothing lost | §3.6 message slot (5 s error) | G: confirmed / W: delete → Conflict("revision conflict") / T: status error 5 s, boundary not deleted, autosave resumed, lock released, draft kept | a:frankmd-workspace::created_notes | keep |
| 22 | 582 "blocks edits while the guarded deletion drains saves and remains in flight" | No edits slip in during deletion | §5.12 Read-only; §3.6 lock indicator | G: confirmed, drain + delete pending / W: user edit / T: rejected (read-only); after failure lock released and edits accepted | a:frankmd-workspace::created_notes | keep |
| 23 | 640 "aborts deletion if content changes programmatically while autosave drains" | Changed content aborts the delete | §5.13 | G: confirmed / W: content changes during drain / T: no delete effect, status error `errors.failed_to_delete` 5 s, autosave resumed | a:frankmd-workspace::created_notes | keep |
| 24 | 674 "recreates a deleted note from its boundary and reopens it for its remaining redo steps" | Redo at history end recreates the note | §6.1 Redo deleted created-note | G: on previous.md, deleted boundary nested/created note.md / W: redo_at_history_end(previous.md) / T: create file with initial content, open it (markdown, new revision), tree refreshed, deleted=false, autosave resumed | a:frankmd-workspace::created_notes | keep |
| 25 | 714 "redoes a root boundary via %s" [rows 1–2 Ctrl+Shift+Z / Ctrl+Y] | With no note open, Ctrl+Y / Ctrl+Shift+Z recreates the last deleted root note | §6.1 Redo row | G: no note open, root deleted boundary / W: Redo via ctrl-y and ctrl-shift-z / T: recreate effect; b: key handled once | a:frankmd-workspace::created_notes + b | keep |
| 26 | 768 "does not route root-boundary redo from an input field" | Redo keys typed in a text input don't recreate notes | §6.1; keymap contexts | G: root boundary, focus in a text input (e.g. dialog input) / W: press ctrl-y / T: no recreate; input handles key | b | keep |
| 27 | 795 "chooses the newest deleted creation boundary when multiple notes share a predecessor" | Redo recreates the most recent one | §5.13 | G: two deleted boundaries previous=p.md / W: redo_at_history_end(p.md) / T: newest (last inserted) recreated | a:frankmd-workspace::created_notes | keep |
| 28 | 824 "keeps the boundary deleted and active file unchanged when recreation conflicts" | Destination taken → message, no change | §3.6 | G: dest exists / W: redo / T: status error 5 s "Destination already exists", deleted=true, current unchanged, autosave resumed | a:frankmd-workspace::created_notes | keep |
| 29 | 853 "resumes autosave when creation redo throws" | IO failure doesn't wedge autosave | §3.6 | G: redo / W: create → IO error / T: autosave resumed, pending-redo cleared, error message | a:frankmd-workspace::created_notes | keep |
| 30 | 875 "resumes autosave if preparing the outgoing note for redo throws" | Prepare failure aborts redo safely | §3.6 | G: redo / W: prepare_transition panics/errs / T: autosave resumed, no create effect | a:frankmd-workspace::created_notes | keep |
| 31 | 896 "resumes autosave and preserves the user's navigation when redo completes stale" | Late redo doesn't yank the user | navigation generation | G: redo in flight / W: user opens other.md, then create completes / T: current other.md, deleted=false, autosave resumed | a:frankmd-workspace::created_notes | keep |
| 32 | 925 "does not delete or hand off if the active navigation changes while autosave drains" | Navigating away during drain cancels delete | §5.13 | G: confirmed, draining / W: open other.md / T: no delete effect, lock released, autosave resumed, boundary intact | a:frankmd-workspace::created_notes | keep |
| 33 | 959 "waits for app setup before handling an already-connected CodeMirror outlet" | Startup opens the initial note once setup is ready, revealing its folder | §5.22; §1 no splash | G: startup with studies/x.md before workspace ready / W: editor ready event, then ready / T: nothing before ready; after: note open, "studies" expanded, editor loaded once | a:frankmd-workspace::startup | changed (no Stimulus outlets; splash removed FE-001) |

### `test/javascript/controllers/app_slash_commands.test.js` — C=10 (keep 9 / changed 1 / dropped 0; a=10 b=0 c=0)
Original subject: slash-command dialogs and replacing the `/query` range on insertion. New home: slash menu §5.5, Insert image §5.6, Insert video §5.7, Table editor §5.8, Emoji picker §5.5.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 46 "opens the existing %s flow with a saved query range" [rows 1–4 table/image/video/emoji] | Picking a slash item opens its dialog/popover and remembers the `/query` range | §5.5 Slash menu | G: "before /image after", slash item X at 7..13 / W: apply_slash(X) / T: PendingInsertion{range 7..13, query} stored; overlay = TableEditor / InsertImage / InsertVideo / EmojiPicker | a:frankmd-workspace::overlay + b | keep |
| 2 | 63 "replaces the saved query when the image picker inserts successfully" | Block insert swallows the `/query` and adjacent space | §5.6 | G: pending 7..13 / W: insert image `![photo](photo.png)` / T: replace 6..14 with "\n![photo](photo.png)\n"; pending cleared | a:frankmd-markdown::edit | keep |
| 3 | 73 "replaces the saved query when the table editor inserts a new table" | Same for new tables | §5.8 | G: pending / W: table Insert / T: replace 6..14 with "\n\n" + 1-col table (A / --- / 1) + "\n\n" (blank line both sides) | a:frankmd-markdown::edit | keep |
| 4 | 85 "replaces the saved query with inline emoji text" | Emoji replaces exactly the query | §5.5 Emoji | G: pending 7..13 / W: pick :smile: / T: replace 7..13 with ":smile:" | a:frankmd-markdown::edit | keep |
| 5 | 95 "replaces the saved query when the video dialog returns an embed" | Video embed is a block insert | §5.7 | G: pending / W: video insert `<video></video>` / T: replace 6..14 with "\n\n<video></video>\n\n" | a:frankmd-markdown::edit | keep |
| 6 | 105 "keeps an existing table edit range when handling a table action" | Editing an existing table replaces the table, not the query | §5.8 Update | G: pending slash + table edit_mode 0..39 / W: Update / T: replace 0..39 with new table | a:frankmd-markdown::edit | keep |
| 7 | 117 "removes a table slash query only from the editor draft, leaving the note unchanged until insertion" | `/table` inside a table edits that table without the query text; doc unchanged until Insert; Cancel clears pending | §5.8 | G: caret in table with "/table" in a cell / W: apply_slash(table) / T: table editor model has query removed (first cell empty, second "value"); document text unchanged; cancel → pending None, no edit | a:frankmd-markdown::table | keep |
| 8 | 140 "clears the pending range on dialog close so later toolbar insertions use the current selection" | After cancelling, header Insert image inserts at caret | §3.4 Insert image button | G: pending, dialog cancelled, caret 1 / W: InsertImage from header → insert / T: replace 1..1 with "\n![toolbar](toolbar.png)\n" | a:frankmd-workspace::overlay | keep |
| 9 | 151 "does not insert %s when the active file is not Markdown" [rows 1–4] | Non-markdown files never get markdown inserts | §5.5; §5.1 availability | G: `.fed` open / W: slash/InsertImage/InsertVideo/InsertTable/EmojiPicker / T: Unavailable; slash menu not offered; no edit; pending cleared | a:frankmd-workspace::commands | changed (§5.1 hidden/unavailable instead of silent no-op) |
| 10 | 172 "drops a stale query range if the document changed while the dialog was open" | Changed text under the range isn't clobbered | §5.5 | G: pending 7..13 "/image", text at range edited / W: insert / T: no replacement (or insert at caret per policy: none), pending cleared | a:frankmd-markdown::edit | keep |

### `test/javascript/controllers/app_toggle_task.test.js` — C=6 (keep 6 / changed 0 / dropped 0; a=6 b=0 c=0)
Original subject: preview checkbox and Ctrl+Enter toggling task items (#203). New home: preview task lists §5.21, Toggle task §6.2 / Document menu §3.4.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 24 "toggles the source line in the editor for a markdown note" | Clicking a preview checkbox toggles its source line | §5.21 Task lists | G: markdown note, "- [ ] x" at line 7 / W: preview_toggle_task(line 7) / T: line 7 → "- [x] x" as one undoable edit | a:frankmd-markdown::edit | keep |
| 2 | 32 "is a no-op when no editor is available" | No note → nothing happens | §5.21 | G: no note open / W: preview_toggle_task(7) / T: no effect | a:frankmd-workspace::navigation | keep |
| 3 | 41 "is a no-op for non-markdown files" | Non-markdown never toggled | §5.22 non-markdown preview | G: `.fed` open / W: preview_toggle_task(7) / T: no edit | a:frankmd-workspace::navigation | keep |
| 4 | 50 "ignores events without a valid line" | Bad source position is ignored | §5.21 source positions | G: markdown note / W: toggle with line None or beyond last line / T: no edit | a:frankmd-markdown::edit | keep |
| 5 | 61 "runs the editor shortcut for a markdown note" | Ctrl+Enter toggles the task at caret | §6.2 Toggle task | G: caret on "- [ ] a" / W: ToggleTask / T: "- [x] a"; b: ctrl-enter dispatches ToggleTask (not newline) | a:frankmd-markdown::edit + b | keep |
| 6 | 69 "is a no-op without an editor or outside markdown" | Toggle task unavailable without a markdown note | §5.1 availability | G: no note / `.fed` / W: availability(ToggleTask) / T: Unavailable; dispatch → no edit | a:frankmd-workspace::commands | keep |

### `test/javascript/controllers/app_typewriter_mode.test.js` — C=7 (keep 5 / changed 2 / dropped 0; a=7 b=0 c=0)
Original subject: typewriter mode restore, persistence, preview sync, sidebar independence. New home: View modes §4.2 Typewriter, §11 quirk "Typewriter force-opens the preview", Settings › Writing §5.16.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 80 "restores the saved mode after the Markdown file becomes active" | Saved typewriter preference shows as on and activates when a markdown note opens; preview follows caret block | §4.2 Typewriter; §11 | G: typewriter_mode=true, no note / W: open markdown note, caret line 6/20 / T: before: toggle ✓, effective off; after: effective on, ToggleTypewriter ✓, typewriter indicator; preview NOT force-opened; if visible, preview sync target = block at line 6 | a:frankmd-workspace::layout | changed (§11 no force-open of preview) |
| 2 | 99 "keeps a saved preference inactive for non-Markdown files" | Typewriter doesn't apply to `.fed` | §4.2 (markdown only) | G: typewriter_mode=true / W: open `.fed` / T: effective off, no indicator, preview unchanged | a:frankmd-workspace::layout | keep |
| 3 | 109 "updates persisted settings and preview state when toggled" | Toggling persists `typewriter_mode` | §5.16 Writing; §4.2 | G: off, markdown open / W: ToggleTypewriter / T: config write typewriter_mode=true, settings row value true, effective on; preview visibility unchanged | a:frankmd-workspace::layout | changed (§11 preview left alone) |
| 4 | 122 "applies Typewriter preview sync while Settings is open" | Settings overlay doesn't suspend typewriter/preview sync | §5.16 live apply | G: Settings open, preview visible / W: enable typewriter / T: effective on, preview sync active | a:frankmd-workspace::layout | keep |
| 5 | 132 "does not dispatch another selection transaction during a document change" | Typing in typewriter mode never moves the caret/selection or adds undo steps | §4.2 Typewriter | G: typewriter on, caret mid-doc / W: `typewriter::on_doc_change` / T: returns scroll offset only; selection unchanged; no extra edit | a:frankmd-workspace::typewriter | keep |
| 6 | 146 "preserves sidebar visibility (%s)" [rows 1–2 true/false] | Typewriter toggles never change the sidebar | §4.2 | G: sidebar_visible = true/false / W: toggle typewriter on and off / T: sidebar_visible unchanged | a:frankmd-workspace::layout | keep |
| 7 | 158 "refreshes CodeMirror after returning from Library" | Returning from media library re-applies typewriter centring | §4.2 Media library | G: typewriter on, library open / W: close library / T: one TypewriterRelayout effect, editor pane visible | a:frankmd-workspace::layout | keep |

### `test/javascript/controllers/app_view_mode.test.js` — C=11 (keep 10 / changed 1 / dropped 0; a=11 b=0 c=0)
Original subject: split/single view mode and preview toggle. New home: View modes §4.2, Settings › Preview §5.16, non-markdown preview §5.22.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 75 "boot: initializeViewMode applies the persisted mode from the settings controller" | Saved single mode applies at launch | §4.2; §5.16 Preview Layout | G: config view_mode=single / W: LayoutState::from_config / T: ViewMode::Single, single pane = Editor | a:frankmd-workspace::layout | keep |
| 2 | 85 "boot: defaults to split when no settings controller is connected" | Default is split | §4.2 | G: no view_mode key / W: from_config / T: Split | a:frankmd-workspace::layout | keep |
| 3 | 95 "setViewMode normalizes unknown values to split" | Bad value falls back to split | §4.2 | G: view_mode="diagonal" / W: set_view_mode / T: Split | a:frankmd-workspace::layout | keep |
| 4 | 103 "switching to single hides the preview and defaults to the editor pane" | Single starts on the editor | §4.2 Single | G: Split, preview visible / W: set_view_mode(Single) / T: `resolve_panes` = editor only | a:frankmd-workspace::layout | keep |
| 5 | 115 "switching back to split shows the editor pane beside the current preview state" | Back to split keeps preview if it was the active single pane | §4.2 | G: Single showing Preview / W: set_view_mode(Split) / T: editor + preview both shown; from Single/Editor: preview per its saved visibility | a:frankmd-workspace::layout | keep |
| 6 | 129 "split mode: the preview toggle keeps its hide/show semantics" | Ctrl+Shift+V shows/hides preview in split | §4.2 Split | G: Split / W: TogglePreview ×2 / T: preview_visible flips; b covered by app_view_mode #7 wiring | a:frankmd-workspace::layout | keep |
| 7 | 137 "single mode: the preview toggle switches to the full-width preview pane" | In single, Ctrl+Shift+V swaps to preview; editor state kept | §4.2 Single | G: Single/Editor, caret (3,4), dirty / W: TogglePreview / T: pane = Preview, returns true; editor buffer/undo/pending slash state retained; b: ctrl-shift-v → `preview-pane` visible, `editor-pane` hidden | a:frankmd-workspace::layout + b | keep |
| 8 | 154 "single mode: toggling again returns to the editor pane and focuses it" | Swap back returns to editor with focus at same caret | §4.2 (FE-016/174) | G: Single/Preview / W: TogglePreview / T: pane = Editor, returns false, focus Editor, caret (3,4); b: `editor` focused | a:frankmd-workspace::layout + b | keep |
| 9 | 169 "single mode: opening Settings preserves the active preview pane" | Settings overlay doesn't flip panes | §5.16 | G: Single/Preview / W: OpenSettings then close / T: pane still Preview throughout | a:frankmd-workspace::layout | keep |
| 10 | 191 "applies view mode changes while Settings is open" | Changing Layout in Settings applies live | §5.16 changes apply live | G: Split, preview visible, Settings open / W: set Layout=Single / T: panes resolve to editor only immediately (one change) | a:frankmd-workspace::layout | keep |
| 11 | 207 "single mode: the markdown-file guard still blocks non-markdown files" | Non-markdown files don't get a rendered preview | §5.22 Preview of a non-markdown file | G: Single, `.fed` open / W: TogglePreview / T: preview pane shows "Preview is available for Markdown notes." (no render) | a:frankmd-workspace::layout | changed (§5.22 in-pane notice replaces temporary message) |

### `test/javascript/controllers/app_workspace.test.js` — C=37 (keep 24 / changed 9 / dropped 4; a=23 b=9 c=1)
Original subject: workspace chrome (header visibility, inline title rename, copy, sidebar, overflow menu, pins, AI busy). New home: pane header + Document menu §3.4, sidebar §3.3/§4.1, status line §3.6, focus §4.3, grammar §5.10.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 64 "hides the header only while Home is visible and restores it for other workspaces" | Document header appears only when there is a document; media library has its own | §3.4; §5.2 empty state; §5.15 | G: empty state / note open / library / W: `pane_header(state)` / T: None (docked quick open) / document header / library header; b: `pane-header` absent then present | a:frankmd-workspace::layout + b | changed (§5.2 empty state, §5.15 own header) |
| 2 | 90 "opens the title field with the caret at the end without selecting text" | Inline rename starts with basename, caret at end, nothing selected | §3.4 title rename | G: open folder/note.md / W: start title rename (click/F2) / T: InlineRename value "note", caret 4, no selection; b: `pane-title` input focused | a:frankmd-workspace::inline_rename + b | keep |
| 3 | 100 "updates the title width mirror as the name changes" | Title input grows with the text | §3.4 auto-sized input | G: rename open / W: type longer name / T: input width tracks text (visual) | c | keep |
| 4 | 112 "saves with Enter through file operations and restores title focus" | Enter renames via the shared rename command | §3.4; §5.13 (HP-030) | G: rename open, value "renamed" / W: Enter / T: RenameNote{folder/note.md → renamed}; input closed; b: focus returns to title | a:frankmd-workspace::inline_rename + b | keep |
| 5 | 121 "cancels with Escape before leaving the Library workspace" | Esc cancels inline rename first, not the outer layer | Esc chain §6.1 | G: inline rename open + another layer (library/narrow drawer) / W: Escape / T: `escape_target` = InlineRename; rename cancelled, no RenameNote, outer layer stays | a:frankmd-workspace::overlay | keep |
| 6 | 134 "submits when the field loses focus without taking focus back" | Blur commits; focus stays where user went | §3.4 Enter/blur commits | G: rename open "renamed" / W: click editor / T: RenameNote sent; focus Editor (not title) | b | keep |
| 7 | 143 "keeps a failed click-away rename editable without stealing focus" | Failed blur-commit stays editable without grabbing focus | §3.4 | G: blur commit → Err("Name already exists") / W: result / T: rename state Error, input visible+editable, value kept; focus stays Editor | a:frankmd-workspace::inline_rename + b | keep |
| 8 | 154 "keeps failures beside the field and allows retrying" | Rename error is shown inline; typing clears it; retry works | §3.4 inline error banner row | G: Enter → Err / W: type, Enter → Ok / T: error row shown (alert role) with message, input invalid; typing clears error; 2 rename calls, second succeeds | a:frankmd-workspace::inline_rename | changed (§3.4 error as banner row under header) |
| 9 | 171 "does not replace an existing draft warning with a filename error" | Error without message doesn't show an empty filename error | §3.4 | G: Enter → Err(None) / W: result / T: no error row, input re-enabled, not invalid | a:frankmd-workspace::inline_rename | keep |
| 10 | 180 "prevents duplicate requests and ignores blur while saving" | One rename in flight; input disabled | §3.4 | G: Enter pressed (Saving) / W: Enter again, blur / T: 1 RenameNote; input disabled; closed after Ok | a:frankmd-workspace::inline_rename | keep |
| 11 | 195 "discards stale results after navigating to another file" | Late rename result for previous note is ignored | navigation generation | G: rename in flight on a.md / W: open b.md, then Err arrives / T: no error shown, rename closed | a:frankmd-workspace::inline_rename | keep |
| 12 | 211 "does not submit a stale edit against the newly selected note" | Edit begun on one note never renames another | §3.4 | G: rename open on a.md / W: current changes to b.md, then Enter / T: no RenameNote | a:frankmd-workspace::inline_rename | keep |
| 13 | 219 "leaves IME composition Enter to the text field" | Enter confirming IME composition doesn't commit rename | §8.3 IME | G: rename open, composing / W: Enter with marked text / T: no RenameNote (a: `on_enter(composing=true)` no-op); real fcitx5 check manual | a:frankmd-workspace::inline_rename + c | keep |
| 14 | 226 "guards title rename for %s" [rows 1–2 null / .fed] | No title rename without a note or for `.fed` | §3.4; §5.1 availability | G: no note / `.fed` / W: availability(RenameNote), click title / T: Unavailable; no InlineRename | a:frankmd-workspace::commands | keep |
| 15 | 245 "copies the current editor content for %s" [rows 1–2 note / .fed] | Copy contents copies the unsaved buffer and confirms | Document menu §3.4; §3.6 | G: dirty buffer for note / `.fed` / W: CopyContents / T: WriteClipboard(buffer text), status info "Copied to clipboard" 2 s | a:frankmd-workspace::commands | changed (§3.6 status line replaces toast) |
| 16 | 252 "copies an empty file" | Empty note copies "" | §3.4 | G: empty note / W: CopyContents / T: WriteClipboard("") | a:frankmd-workspace::commands | keep |
| 17 | 258 "uses the textarea fallback when CodeMirror is unavailable" | — | — | — | — | dropped (DOM-specific; hidden textarea FE-074 dropped §1) |
| 18 | 265 "does not copy when no file is open" | Nothing to copy without a note | §5.1 | G: no note / W: CopyContents / T: Unavailable; no clipboard write | a:frankmd-workspace::commands | keep |
| 19 | 272 "reports clipboard failures without success feedback" | Clipboard failure shows one error, no success | §3.6 | G: clipboard write fails / W: CopyContents result Err / T: exactly one status error `errors.failed_to_copy_content` 5 s; no "Copied" | a:frankmd-workspace::status | changed (§3.6 status line replaces toast) |
| 20 | 280 "hides the entire sidebar, focuses its opener, and restores it" | Hiding sidebar removes it from focus, shows header opener; reopening restores | §3.3; §3.4 open-sidebar button | G: focus in sidebar / W: ToggleSidebar via sidebar button / T: sidebar_visible false; `sidebar` absent, open-sidebar button visible and focused, typewriter relayout effect; click opener → sidebar back, focus in sidebar | a:frankmd-workspace::layout + b | keep |
| 21 | 298 "keeps editor focus when the Explorer is opened by a keyboard shortcut" | Ctrl+E open doesn't steal focus | §6.1 Toggle sidebar | G: sidebar hidden, title/editor focused / W: ctrl-e / T: sidebar visible, focus unchanged | b | keep |
| 22 | 309 "moves focus to the sidebar toggle when the focused opener becomes hidden" | Focus isn't lost when its element disappears | §4.3 focus model | G: narrow, focus on open-sidebar button / W: resize to wide (sidebar docked) / T: opener gone; focus moves to Sidebar region, not nowhere | b | keep |
| 23 | 324 "shows persisted Stats when Settings connects after the note" | Stats show for an open note when show_stats is on, regardless of init order | §3.6 Stats | G: show_stats=true loaded after note open / W: settings applied / T: StatusLine stats = "3 words" shown once | a:frankmd-workspace::status | changed (§3.6 stats in status line; no outlet order) |
| 24 | 338 "restores the title when its display controller connects after the note" | Header always shows the open note's title/breadcrumb | §3.4 breadcrumb | G: open folder/note.md / W: `pane_title(state)` / T: breadcrumb ["folder"], title "note" | a:frankmd-workspace::layout | keep |
| 25 | 348 "opens the mobile overlay, contains focus, and closes it with Escape" | Narrow drawer: scrim, focus trapped, Esc closes to opener | §4.1 Narrow | G: width 600 / W: open drawer, Tab past last, Escape / T: drawer + scrim visible, main inert, Tab wraps inside drawer, Esc closes, focus to open-sidebar button; a: `escape_target` = NarrowDrawer | b | keep |
| 26 | 368 "shows only applicable actions in single preview, Library, and empty states" | Document menu shows only actions that apply, with a reason for unavailable editing ones | §3.4 Document menu; §5.1 | G: Single/Preview / library / empty state / W: `doc_menu_rows(state)` / T: editing rows disabled in preview, preview toggle enabled; library: document rows absent; empty: no `⋯`; b: keyboard open focuses first enabled row | a:frankmd-workspace::doc_menu + b | changed (§3.4 fixed menu replaces toolbar/overflow) |
| 27 | 390 "keeps overflow usable when a note finishes loading after the menu opens" | Open menu updates when a note finishes loading | §3.4 | G: `⋯` open during load / W: note loaded / T: menu stays open; rows recomputed (Edit…, Copy…, Help) | a:frankmd-workspace::doc_menu | keep |
| 28 | 400 "supports arrow, Home, End and Escape navigation" | Menu keyboard nav | §6.4 Any list | G: focus `⋯` button / W: down (opens+first), down, end, home, escape / T: focused row first, second, last, first; closed, focus back to `⋯` | b | keep |
| 29 | 414 "opens by pointer without focusing an option and supports %s" [rows 1–2] | Pointer open leaves focus on button; first arrow picks first/last | §6.4 | G: click `⋯` / W: down / up / T: no row focused after click; then first / last row focused | b | keep |
| 30 | 423 "closes before launching a dialog and leaves focus in that dialog" | Menu closes and dialog keeps focus | §5 overlays one at a time | G: `⋯` open / W: choose Help / T: menu closed, `help-dialog` visible and focused | b | keep |
| 31 | 436 "dismisses on outside click and Tab without trapping focus" | Outside click/Tab closes menu | §5 backdrop rule | G: `⋯` open / W: click editor; reopen, Tab / T: menu closed; Tab returns focus to `⋯` button | b | keep |
| 32 | 460 "pins and unpins without running the action or closing the menu" | — | — | — | — | dropped (design-removed §3.4 pinning gone) |
| 33 | 479 "restores only known pins from this browser session" | — | — | — | — | dropped (design-removed §3.4; sessionStorage) |
| 34 | 487 "uses defaults with malformed or unavailable storage" | — | — | — | — | dropped (design-removed §3.4; sessionStorage) |
| 35 | 501 "preserves pins while disabling editing actions in preview and hiding them in Library" | Fixed header buttons: editing ones disabled in single preview, hidden in library, re-enabled on return | §3.4 four fixed buttons | G: Single/Preview / library / editor / W: `header_buttons(state)` / T: Image, Find disabled, Preview enabled+pressed / no document header / all enabled | a:frankmd-workspace::layout | changed (§3.4 fixed buttons replace pins) |
| 36 | 522 "navigates to the pin with arrows in both text directions" | Menu row keyboard nav doesn't break in RTL | §3.4; §8.1 | G: Document menu (no pin cells) / W: right/left on a row / T: no lateral move; submenu-free rows | b | changed (pins gone §3.4; RTL deferred §8.1) |
| 37 | 537 "keeps both AI controls busy and restores them together" | While grammar runs, every AI entry is busy and the note is read-only; restored together | §5.10; §3.6 indicator | G: GrammarCheck started / W: running then finished / T: GrammarCheck Unavailable("Grammar check running") in palette + Document menu row disabled/busy, editor read-only, AI indicator; after: all restored, labels unchanged | a:frankmd-workspace::grammar + b | changed (§5.10 banner + lock) |

#### New VM requirements (G3 — JS controllers part 4)
- `frankmd-workspace::grammar` — `GrammarState { availability: Unknown|NotConfigured|Available{provider,model}, request: Option<RequestId>, view: Diff|Edit, edited: Option<String>, hunks }`: `from_config`, `start(doc) -> (RequestId, effects[FlushAutosave, LockReadOnly, PushBanner])`, `finish(id, Result)` (ignores stale id; clears request, unlocks, pops banner), `cancel()` no-op when idle, `toggle_edit()` (label key), `apply() -> Edit` (one undo step, honours rejected hunks) (ai_grammar #1–12, workspace #37).
- `frankmd-workspace::navigation` — `NavigationState`: `open(path) -> (Generation, Effect::Load)`, `complete(gen, LoadResult)` (stale ignore incl. NotFound/IO), `close_note()`, injected `prepare_transition() -> Result<(), DraftError>` hook that can refuse, `OpenOutcome { focus_editor: content.is_empty(), select_in_explorer }`, `on_renamed/on_moved/on_deleted(path, kind)` remapping current note, autosave target and back/forward history, clearing pending insertion; preview-task / toggle-task guard by open file kind (navigation #1–9, file_rename #1–4, item_moved #3, editor_focus, toggle_task #2–3).
- `frankmd-workspace::created_notes` (new) — `CreatedNoteBoundaries`: record(path, initial_content, initial_revision, previous) only when load is current and both baseline parts known; remap(old,new,kind) with dependent `previous` remap and stale-destination drop; evict(subtree); `undo_at_history_start(path, content) -> Handled|NotHandled`; deletion state machine (drain autosave → ReadOnlyLock → Delete{expected_revision} → mark deleted, open predecessor) aborting on content change / navigation change / conflict with status errors; `redo_at_history_end(pred)` + root boundary, newest-wins, recreate effect, stale completion; autosave pause/resume guaranteed on every exit path (navigation #10–33).
- `frankmd-workspace::startup` — open initial note (CLI/IPC/restore) through the same guarded transition after workspace ready; expands parent folders; no-op before ready (navigation #9, #33).
- `frankmd-workspace::explorer` — `click(row, Modifiers) -> Option<ExplorerEffect{OpenNote, ToggleFolder}>`, `on_focus(row, FocusCause{Pointer,Keyboard})`, `click_background()`, `context_menu: Option<(MenuKind{Row,Background}, Point)>` closed by selection, `select_all_visible()`, `remap/remove(path, kind)` incl. anchor & focus, `delete_targets()`/`rename_target()` excluding `.fed`, `reveal(path)` expanding + persisting ancestors, tree snapshot `Generation` with stale-discard and selection restore after refresh, open-note marker separate from selection (explorer_bg #1–6, explorer_sel #1–15, item_moved #1–2, navigation #3, #5–6).
- `frankmd-workspace::inline_rename` (new) — `InlineRename { path, value, caret, selection, state: Editing|Saving|Error(Option<String>), invalid, generation }`: `start(path)` (basename sans `.md`, caret end), `input`, `on_enter(composing)`, `on_blur`, `on_result(gen, Result)`; ignores duplicate submit while Saving, discards stale gen, refuses submit when current note changed; shared by pane title and explorer row (workspace #2–13, explorer_sel #12).
- `frankmd-workspace::doc_menu` (new) — `doc_menu_rows(&UiState) -> Vec<MenuRow{CommandId, enabled, shortcut}>` derived from the registry per mode (Editor, SinglePreview, MediaLibrary, Empty); recomputed when state changes while open (workspace #26–27).
- `frankmd-workspace::layout` — extra: `SinglePane {Editor, Preview}` kept across Settings open/close; `ViewMode::parse` fallback Split; `media_library_open` restoring preview visibility on both Esc/toggle and open-note exits; `typewriter_pref` vs `typewriter_effective(file_kind)`; `TypewriterRelayout` effect on library close and sidebar toggle; `pane_header(state)`, `pane_title(state)` (breadcrumb + title), `header_buttons(state)` enabled/hidden per mode; non-markdown preview placeholder (library #1–2,#4, typewriter #1–4,#6–7, view_mode #1–11, workspace #1, #20, #24, #35).
- `frankmd-workspace::typewriter` — `on_doc_change(..) -> ScrollAdjust` with no selection change; `sync_data(caret_line, total_lines)` → preview block target only when preview visible (typewriter #1, #5).
- `frankmd-workspace::media_library` — open-transition counter emitting `Reload + ResetUsage` once per open; `insert(card, &OpenNote) -> InsertStatus {Inserted(Edit), NoOpenNote, MarkdownNoteRequired, InvalidMedia}` + `CloseLibrary`, `FocusEditor` effects (library #3, #5–6).
- `frankmd-workspace::overlay` — `PendingInsertion { range, query, kind }` stored when a slash item opens an overlay; cleared on cancel/close and on non-markdown; `escape_target` includes `InlineRename` and explorer context menu (`Popover`) ahead of MediaLibrary/NarrowDrawer (slash #1, #8–9, explorer_bg #6, workspace #5, #25).
- `frankmd-markdown::edit` — `apply_pending_insertion(text, PendingInsertion, Insert{Block|Inline}(s)) -> Option<Edit>` (block widens over adjacent spaces and pads newlines; inline exact; stale query → None); `toggle_task_at_line(text, line) -> Option<Edit>` (out-of-range → None) (slash #2–6, #10, toggle_task #1, #4–5).
- `frankmd-markdown::table` — parse existing table at caret with a `/table` slash query stripped from its cell, leaving document text untouched (slash #7).
- `frankmd-workspace::commands` — availability reasons needed: GrammarCheck (no note / not configured / running), RenameNote (none, `.fed`), CopyContents (no note), ToggleTask/Insert* (non-markdown); CopyContents → `Effect::WriteClipboard(buffer)` with result → status message (ai_grammar #4, slash #9, toggle_task #6, workspace #14–19, #37).
- `frankmd-workspace::status` — one-shot error on clipboard failure (no success message); stats slot driven by `show_stats` after late config load (workspace #19, #23).
- `frankmd-app` test harness — fake clipboard service, fake AI provider, IME marked-text simulation if `TestAppContext` supports it (workspace #13, #15, ai_grammar).

#### Open questions (G3 — JS controllers part 4)
- `.fed` as an open document: many rows (slash #9, toggle_task #3, workspace #14–15, library #6, typewriter #2, view_mode #11) assume `.fed` can still be opened in the editor. ui-spec §5.16 moves config to Settings and says nothing about opening `.fed` as a note. Is `.fed` still openable/copyable, or should these rows target "any non-markdown file"?
- Root-boundary redo (navigation #25–26): in the empty state the docked quick-open input has focus at launch (§5.2), and per the old rule redo is not routed from inputs. That makes Ctrl+Y / Ctrl+Shift+Z with no note open unreachable unless the empty-state input is exempt. Which wins?
- Meta-click (explorer_sel #4): Super belongs to Hyprland (§6 "No Super bindings"). Keep the meta row only for a future macOS build, or drop it?
- Pane header in the empty state (workspace #1): §3.4/§5.2 don't say whether a document-less header (open-sidebar button, `⋯`) stays visible when no note is open. When the sidebar is hidden, what reopens it with the mouse?
- Keyboard focus moving onto an explorer row replaces the selection (explorer_sel #7). §5.14 makes Space toggle selection, which suggests a VS Code-like split between focus and selection. Should ↑/↓ move focus without selecting?
- Delete key (explorer_sel #11): the old app called `deleteItems` directly. Confirm §5.13 always asks (focus on Cancel), including for multi-select.
- Grammar Apply (ai_grammar #8): the old `accepted` payload was the whole corrected text. With per-hunk reject, is Apply still a whole-document replace (one undo step) or a minimal patch?
- Slash stale range (slash #10): the old code dropped the insert entirely. Should the new UI insert at the caret instead, or show a status message?
- Media library insert (library #5): §5.15 has an Insert action on cards but doesn't say that inserting closes the library and focuses the editor. Assumed yes (parity).
- Drafts keyed by DocId (§10.1 134–141): renaming may no longer need a draft re-key (file_rename #1–2). The adapted test asserts the user-observable "next save writes the new path" only.
- Inline rename in the media library (workspace #5): the library has no title, so the Escape-precedence row is re-expressed against a generic outer layer. Confirm F2 is unavailable while the library is open.

## 9. G4: autosave, code_dialog, codemirror, connection_monitor

### `test/javascript/controllers/autosave_controller.test.js` — C=83 (keep 65 / changed 16 / dropped 2; a=81 b=0 c=0)
Original subject: autosave + draft storage + content-loss guard + recovery hand-off (autosave_controller.js, draft_storage.js, recovery_diff_controller.js). New home: GUI-free `frankmd-workspace::{autosave,drafts,status,banners,recovery,navigation,lifecycle}` driven by `Command::Edited/Tick/SaveCompleted` with the injected Clock; surfaces are the status-line save state (§3.6), pane banners Content loss / Draft storage failed / Vault unreachable (§5.12), Recovery modal (§5.11), Quit prompt (§5.23). "Server" = vault file on disk, "offline" = vault unreachable, drafts keyed by DocId (architecture §3.1, ui-spec §10.1 134–141). Timings kept: save debounce 2000 ms, max interval 30000 ms, draft debounce 300 ms.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 122 "uses the server-rendered save label before translations load" | Unsaved state always shows a real localized label | Status line save state §3.6 | G: catalog en / W: doc becomes dirty / T: `status.save` label = resolved "status.unsaved" text (`● Unsaved`), slot visible | a:frankmd-workspace::status | keep |
| 2 | 132 "never renders an unresolved status key" | A missing key never shows as raw `status.unsaved` | Status line §3.6, HP-022 | G: locale lacking the key / W: dirty / T: label falls back to en text; if still unresolved the slot is empty+hidden, never the key | a:frankmd-workspace::status | keep |
| 3 | 142 "no warning for small deletion (< 20% or < 50 chars)" | Small deletions save normally | Content-loss guard §5.12 | G: baseline 200 chars / W: edit to 180, SaveNow / T: `Effect::Save` emitted; no ContentLoss banner | a:frankmd-workspace::autosave | keep |
| 4 | 155 "shows warning for large deletion (> 20% AND > 50 chars)" | Big deletion blocks the save and protects text | Content loss banner §5.12 | G: baseline 300 / W: edit to "", SaveNow / T: no Save effect; BannerKind::ContentLoss "would remove 300 characters (100 %)" with Undo/Save anyway; draft {text:"", base_revision} stored | a:frankmd-workspace::autosave + banners | keep |
| 5 | 174 "no warning when _contentLossOverride is true" | "Save anyway" lets the save through | Content loss banner §5.12 | G: banner approved for current snapshot / W: SaveNow / T: Save effect emitted | a:frankmd-workspace::autosave | keep |
| 6 | 185 "no warning when _lastSavedContent is null (fresh file)" | Docs without a baseline never trigger the guard | §5.12 | G: doc with no baseline text / W: SaveNow / T: Save emitted, no banner | a:frankmd-workspace::autosave | keep |
| 7 | 196 "resets _contentLossOverride after successful save" | Approval is one-shot | §5.12 | G: approved, Save in flight / W: SaveCompleted Ok / T: guard approval cleared; next large deletion warns again | a:frankmd-workspace::autosave | keep |
| 8 | 213 "blocked while warning is active" | While warning shows, no autosave runs but doc stays dirty | §5.12, §3.6 | G: ContentLoss active / W: Edited + Tick past 2 s / T: no Save effect; SaveState::Unsaved | a:frankmd-workspace::autosave | keep |
| 9 | 227 "does not mark unchanged content unsaved after a change event" | A no-op edit is not dirty | Status §3.6 | G: saved "saved" / W: Edited{text:"saved"} / T: not dirty, no timer armed, status empty, no draft written | a:frankmd-workspace::autosave + drafts | keep |
| 10 | 239 "clears the unsaved status when an edit returns to saved content" | Typing back to saved text clears dirty and draft | Status §3.6 | G: dirty "changed" / W: Edited back to "saved" / T: not dirty, both timers disarmed, status cleared, draft discarded | a:frankmd-workspace::autosave + drafts | keep |
| 11 | 266 "flushes immediately when the document becomes hidden without a server request" | Leaving the app persists the draft at once | Window lifecycle (architecture §8) | G: dirty, draft debounce pending / W: Command::WindowDeactivated / T: draft {text, base_revision} written synchronously; no Save effect; pending draft debounce cleared | a:frankmd-workspace::lifecycle | changed (native window deactivate replaces visibilitychange) |
| 12 | 279 "flushes immediately on pagehide without a server request" | Closing never loses the snapshot | Quit and close §5.23 | G: dirty / W: Command::QuitRequested / T: draft flushed first, then Save effect, then quit (§5.23 saves and quits without asking) | a:frankmd-workspace::lifecycle | changed (§5.23 quit also saves) |
| 13 | 289 "flushes a dirty empty draft on %s" (it.each visibilitychange/pagehide) | An all-deleted buffer is flushed too | Lifecycle §5.23 | G: dirty with text "" / W: WindowDeactivated and (separately) QuitRequested / T: draft {text:""} written in both cases | a:frankmd-workspace::lifecycle | keep |
| 14 | 301 "does not rewrite an unchanged draft for repeated lifecycle events" | No redundant draft writes | Lifecycle | G: deactivate flushed draft rev R / W: deactivate again, then QuitRequested / T: no second WriteDraft effect; draft still rev R | a:frankmd-workspace::lifecycle + drafts | keep |
| 15 | 317 "shows a storage error and requests a best-effort unload warning when persistence fails" | Draft failure is visible and quit warns | Status `⚠ Draft not stored` §3.6; banner Draft storage failed §5.12; quit prompt §5.23 | G: DraftStore write fails / W: WindowDeactivated, then QuitRequested / T: SaveState::DraftNotStored (danger), DraftStorageFailed banner, quit → QuitDecision::Ask; after a later successful flush status back to Unsaved, banner gone, quit no longer asks | a:frankmd-workspace::lifecycle + status + banners | changed (§3.6/§5.12/§5.23 replace beforeunload) |
| 16 | 345 "does not let file A's warning block autosave for file B" | Warning is per-note | §5.12 | G: A has ContentLoss + draft "" / W: open B, edit, Tick +2000 ms / T: one Save for B's DocId; no banner on B; A's draft "" kept | a:frankmd-workspace::autosave + navigation | keep |
| 17 | 365 "does not reuse save-anyway approval for a later editor snapshot" | Approval binds to one exact snapshot | §5.12 | G: approved snapshot "" / W: text becomes 100 chars (still large), SaveNow / T: no Save; banner re-shown; draft = 100-char text | a:frankmd-workspace::autosave | keep |
| 18 | 378 "recomputes the warning when a protected draft is restored against its baseline" | Auto-restored large-deletion draft re-arms the banner | §5.11/§5.12 | G: draft "" with same base revision as disk (300 chars) / W: open note / T: editor gets ""; ContentLoss banner visible | a:frankmd-workspace::recovery + autosave | keep |
| 19 | 389 "recomputes a divergent recovered draft against the latest server baseline" | Restored draft is re-based on current file | Recovery §5.11 | G: draft "" on old rev, file rev new / W: Recovery → RestoreDraft / T: draft re-based to new rev; ContentLoss banner active | a:frankmd-workspace::recovery + autosave | keep |
| 20 | 410 "undo clears the saved draft when it restores the server baseline" | Banner Undo brings text back and cleans up | Content loss banner Undo §5.12 | G: banner after delete-all / W: click Undo → editor undo restores baseline → Edited / T: banner gone; draft discarded | a:frankmd-workspace::autosave + b | keep |
| 21 | 427 "undo updates the protected draft when the deletion remains large" | Partial undo keeps protection current | §5.12 | G: banner / W: Undo yields 100 chars / T: banner stays; draft = 100 chars, same base rev | a:frankmd-workspace::autosave | keep |
| 22 | 446 "restores the server baseline when a recovered deletion has no editor history" | Undo works even with no undo history | §5.12 | G: banner on a recovered draft, editor has no history / W: Undo / T: Effect::LoadBaseline(doc) (editor text = file), banner gone, draft discarded | a:frankmd-workspace::autosave + b | keep |
| 23 | 463 "save anyway removes the matching draft only after success" | Draft survives until the file really has the text | §5.12 | G: banner, draft rev R / W: Save anyway, SaveCompleted Ok / T: draft R discarded only after the Ok | a:frankmd-workspace::autosave + drafts | keep |
| 24 | 475 "failed save anyway keeps its local draft" | A failed save never deletes the safety copy | §5.12, status `✕ Save failed` | G: banner, draft R / W: Save anyway, SaveCompleted Err / T: draft R kept; SaveState::SaveFailed | a:frankmd-workspace::autosave + drafts | keep |
| 25 | 489 "flushes the outgoing snapshot before cancelling timers without forcing a server save" | Switching notes never loses text and never forces a write | Navigation (open-note flow) | G: A dirty, timers armed / W: open B / T: A draft {text, base_rev} written; A timers disarmed; no Save effect; transition Ok | a:frankmd-workspace::navigation + autosave | keep |
| 26 | 509 "keeps the outgoing file active and does not cancel its save timers when draft storage fails" | If A can't be protected, A stays open | Navigation; status §3.6 | G: draft writes fail, A dirty / W: open B / T: transition refused; A still active; timers still armed; no Save; SaveState::DraftNotStored | a:frankmd-workspace::navigation | keep |
| 27 | 527 "blocks a dirty transition when the file has no server revision to anchor its draft" | No unanchored drafts | Navigation | G: A dirty with unknown base revision / W: open B / T: transition refused, A active, status error message | a:frankmd-workspace::navigation | keep |
| 28 | 539 "never lets an autosave scheduled for A read or save B" | Saves are scoped to the doc they were scheduled for | Autosave (DocId + Generation, architecture §3.1) | G: A edited, timer armed / W: switch to B (edited), Tick +2000 ms / T: no Save carrying B's text under A; no Save for B yet | a:frankmd-workspace::autosave | keep |
| 29 | 555 "does not require a server save while offline before preparing a transition" | Switching while unreachable works with drafts only | Vault unreachable §5.12 | G: Unreachable, A dirty / W: open B / T: Ok; A draft written; no Save | a:frankmd-workspace::navigation + connectivity | keep |
| 30 | 571 "does not apply an in-flight A save response to the active B state" | A late result never corrupts the current note | Autosave | G: Save(A) in flight / W: open B, then SaveCompleted(A, rev2) / T: B's baseline/rev/status untouched; A's record gets rev2 | a:frankmd-workspace::autosave | keep |
| 31 | 595 "debounces an online empty draft and stores it independently by path" | Even an empty buffer gets a draft, after 300 ms | Drafts (HP-007) | G: doc X / W: Edited "" ; Tick +300 ms / T: draft {doc X, text "", base_rev} stored, not before 300 ms | a:frankmd-workspace::drafts | keep |
| 32 | 609 "keeps pending snapshots for different files independent" | Pending drafts of two notes don't clobber | Drafts | G: edit X then switch and edit Y within 300 ms / W: Tick +300 ms / T: two drafts, each with its own text and base rev | a:frankmd-workspace::drafts | keep |
| 33 | 624 "automatically restores a draft when its server baseline still matches" | Unsaved work comes back silently when safe | Recovery rules §5.11 | G: draft base rev == file rev / W: open note / T: editor text = draft; dirty; no Recovery dialog | a:frankmd-workspace::recovery | keep |
| 34 | 632 "clears a draft when the server already contains its content" | Stale identical drafts disappear | §5.11 | G: draft text == file text (older rev) / W: open / T: file text shown; draft discarded; no dialog | a:frankmd-workspace::recovery | keep |
| 35 | 641 "preserves and opens a diverged draft in the recovery workflow" | Diverged draft forces a choice, never silently lost | Recovery modal §5.11 | G: draft on old rev, file changed / W: open / T: editor shows file text; OverlayKind::Dialog(Recovery) with record {doc, file text, draft text, draft time, file mtime}; draft kept | a:frankmd-workspace::recovery + overlay | keep |
| 36 | 659 "opens a collision recovery choice and promotes the selected moved draft safely" | Renaming onto a path never mixes two notes' drafts | Drafts keyed by DocId (architecture §3.1) | G: X has draft, Y's draft exists, X renamed to a path Y once used / W: rename, then reopen X / T: X's draft still keyed to X and restored; Y's draft untouched; no collision dialog | a:frankmd-workspace::drafts | changed (DocId-keyed drafts, no path collisions) |
| 37 | 705 "keeps the destination draft as a recovery copy when the user chooses the server version" | "Use file on disk" drops only this note's draft | Recovery §5.11 | G: Recovery for X; another doc Y has a draft / W: UseFile / T: editor = file text; X's draft discarded; Y's draft unchanged | a:frankmd-workspace::recovery | changed (§5.11 buttons; DocId drafts) |
| 38 | 734 "keeps a divergent draft recoverable after Escape is canceled and an edit schedules autosave" | Recovery can't be dismissed and can't be overwritten meanwhile | Recovery §5.11 (Esc does nothing) | G: Recovery open / W: Escape; Tick +2300 ms / T: escape_target ≠ Recovery (dialog stays, `dialog-recovery` visible); no Save; original draft (text, rev) intact | a:frankmd-workspace::overlay + recovery + b | keep |
| 39 | 761 "prefers a draft over a stale legacy backup and removes the superseded backup" | — | — | localStorage `frankmd:backup:` has no native store; drafts are the only local copy | — | dropped (browser-only) |
| 40 | 770 "opens a newer, different legacy backup for an explicit recovery choice" | — | — | legacy offline backup key not carried over | — | dropped (browser-only) |
| 41 | 791 "reports draft storage failures through the save status" | Failed draft write is visible | `⚠ Draft not stored` §3.6 + banner §5.12 | G: draft writes fail / W: Edited; Tick +300 ms / T: SaveState::DraftNotStored (danger); DraftStorageFailed banner with Details | a:frankmd-workspace::status + banners | changed (§3.6/§5.12 replace red text) |
| 42 | 808 "restores an explicitly selected empty draft" | Choosing an empty draft is honoured | Recovery Restore draft §5.11 | G: draft "" on old rev / W: RestoreDraft / T: Effect sets editor text ""; draft re-based to current file rev | a:frankmd-workspace::recovery | keep |
| 43 | 820 "keeps a newer draft when an earlier save completes" | Typing during a save is not lost | Autosave/drafts | G: Save(s1) in flight; newer draft s2 written / W: SaveCompleted(rev9) / T: draft s2 kept, re-based to rev9; still dirty | a:frankmd-workspace::autosave + drafts | keep |
| 44 | 850 "hides banner and resets flags" | Clearing the warning resets guard state | Content loss banner §5.12 | G: ContentLoss active / W: guard.dismiss() (note closed/switched) / T: banner removed from BannerStack; active+approval flags false | a:frankmd-workspace::banners + autosave | keep |
| 45 | 869 "sets override, dismisses banner, and calls saveNow" | "Save anyway" saves immediately | §5.12 | G: banner / W: click `banner-content-loss` Save anyway / T: approval set for snapshot; banner gone; Save effect | a:frankmd-workspace::autosave + b | keep |
| 46 | 890 "auto-dismisses warning when content is restored (e.g. Ctrl+Z)" | Undoing the deletion clears the banner by itself | §5.12 | G: banner, baseline 300 / W: Edited back to baseline / T: banner removed, guard inactive | a:frankmd-workspace::autosave | keep |
| 47 | 907 "warning stays if content is still below threshold" | Banner stays while loss persists | §5.12 | G: banner / W: Edited "" again / T: banner still present | a:frankmd-workspace::autosave | keep |
| 48 | 927 "prevents concurrent saves (_isSaving blocks second call)" | One write per note at a time | Autosave | G: Save in flight / W: SaveNow (Ctrl+S) / T: no second Save effect; SaveState::Saving | a:frankmd-workspace::autosave | keep |
| 49 | 952 "clears _isSaving after fetch error" | A failed write doesn't wedge saving | Status `✕ Save failed` | G: Save in flight / W: SaveCompleted Err / T: SaveFailed shown; next SaveNow emits Save | a:frankmd-workspace::autosave + status | keep |
| 50 | 964 "reschedules save if content changed during fetch" | Edits during a write are saved later | Autosave | G: Save(v1) in flight / W: Edited v2; SaveCompleted Ok / T: dirty; debounce re-armed; Tick +2000 ms emits Save(v2) | a:frankmd-workspace::autosave | keep |
| 51 | 989 "does not reschedule if content unchanged after save" | Clean after save | Status `✓ Saved` | G: Save(v1) / W: SaveCompleted Ok, no edits / T: not dirty; no timer; SaveState::Saved | a:frankmd-workspace::autosave | keep |
| 52 | 1003 "waits for an in-flight autosave and returns its acknowledged revision" | Delete uses the revision just written | Delete flow §5.13 | G: Save(X) in flight / W: DeleteNote(X) / T: delete waits; after SaveCompleted(rev_s) the delete effect carries expected_revision rev_s | a:frankmd-workspace::autosave + dialogs | keep |
| 53 | 1028 "cancels a follow-up debounce scheduled by an in-flight autosave" | No save fires after a delete was prepared | §5.13 | G: Save in flight, more edits / W: DeleteNote, SaveCompleted(rev_s) / T: both timers disarmed; known revision = rev_s; no further Save | a:frankmd-workspace::autosave | keep |
| 54 | 1056 "abandons preparation if the active note changes while waiting for a save" | A stale delete preparation is dropped | Navigation generation (HP-003) | G: delete of X waiting on save / W: open Y, SaveCompleted / T: delete preparation resolves Stale; no delete effect; status message | a:frankmd-workspace::autosave + navigation | keep |
| 55 | 1095 "scheduleOfflineBackup while offline triggers backup (debounced)" | Edits while unreachable are kept locally | Vault unreachable "Edits are kept as drafts" §5.12 | G: Unreachable / W: Edited; Tick +300 ms / T: draft written (state dir); no Save effect | a:frankmd-workspace::drafts + connectivity | changed (§5.12 drafts replace 1 s offline backup) |
| 56 | 1111 "scheduleOfflineBackup while online does NOT backup" | Only one local safety store | Drafts §10.1 134–141 | G: Reachable / W: Edited / T: draft debounce + save debounce armed; no separate backup record kind exists | a:frankmd-workspace::drafts | changed (single draft store) |
| 57 | 1122 "onConnectionLost immediately backs up unsaved content" | Losing the vault flushes unsaved text now | §5.12 | G: dirty / W: Event::VaultUnreachable / T: draft flushed immediately (no 300 ms wait) | a:frankmd-workspace::autosave + connectivity | changed (§5.12 drafts) |
| 58 | 1132 "onConnectionLost does NOT backup if content unchanged" | Clean notes write nothing | §5.12 | G: clean / W: VaultUnreachable / T: no WriteDraft effect | a:frankmd-workspace::autosave | keep |
| 59 | 1142 "saveNow success clears backup" | Successful save drops the local copy | Drafts | G: draft exists / W: SaveCompleted Ok for that snapshot / T: draft discarded | a:frankmd-workspace::drafts | keep |
| 60 | 1154 "checkOfflineBackup with differing backup shows recovery dialog" | Draft written while unreachable and file changed meanwhile → choice | Recovery §5.11 | G: draft from unreachable session, file rev changed externally / W: reopen note / T: Recovery opens with draft time + file mtime, cause sentence "The file changed on disk after your draft was saved" | a:frankmd-workspace::recovery | changed (§5.11 draft source replaces "backup") |
| 61 | 1179 "checkOfflineBackup with matching backup does NOT show dialog" | No needless prompt | §5.11 | G: draft text == file text / W: open / T: no Recovery; draft discarded | a:frankmd-workspace::recovery | keep |
| 62 | 1194 "onRecoveryResolved with 'backup' sets editor content and marks unsaved" | Restore draft puts text back, still unsaved | Recovery Restore draft §5.11 | G: Recovery / W: RestoreDraft / T: editor = draft; baseline stays file text; dirty; draft re-based to file rev | a:frankmd-workspace::recovery | keep |
| 63 | 1216 "consumes an accepted legacy backup and does not offer it again" | An accepted choice is never re-asked | §5.11 | G: RestoreDraft done (draft re-based) / W: close and reopen note / T: draft auto-restored (base rev matches); Recovery not shown again | a:frankmd-workspace::recovery | changed (legacy backup gone; same no-re-prompt rule) |
| 64 | 1243 "saves pending changes after reconnection even if in-flight save completed during offline" | Edits made while unreachable reach disk on reconnect | §5.12 Vault unreachable | G: Save(v1) in flight; VaultUnreachable; Edited v2 / W: SaveCompleted Ok; VaultReachable / T: Save(v2) emitted | a:frankmd-workspace::autosave + connectivity | keep |
| 65 | 1281 "onConnectionLost clears all pending save timers" | No write attempts while unreachable | §5.12 | G: timers armed / W: VaultUnreachable / T: both timers disarmed; still dirty | a:frankmd-workspace::autosave | keep |
| 66 | 1300 "scheduleAutoSave is a no-op while offline" | Typing while unreachable stays dirty, no save timer | §5.12 | G: Unreachable / W: Edited / T: no save timer; dirty | a:frankmd-workspace::autosave | keep |
| 67 | 1312 "saveNow is a no-op while offline" | Ctrl+S while unreachable doesn't attempt a write | §5.12, status slot | G: Unreachable, dirty / W: SaveNow / T: no Save effect; dirty; status message points at the banner | a:frankmd-workspace::autosave + status | keep |
| 68 | 1324 "onConnectionRestored triggers saveNow when there are pending changes" | Reconnect saves dirty notes | §5.12 | G: Unreachable, dirty / W: VaultReachable / T: Save effect | a:frankmd-workspace::autosave | keep |
| 69 | 1339 "onConnectionRestored does NOT save when there are no pending changes" | No spurious writes | §5.12 | G: Unreachable, clean / W: VaultReachable / T: no Save | a:frankmd-workspace::autosave | keep |
| 70 | 1363 "renames a pending autosave without clearing dirty state" | Pending save follows the rename | Autosave by DocId (architecture §3.1) | G: foo.md dirty, timer armed / W: rename to bar.md; Tick +2000 ms / T: one Save to bar.md with full text "edited content"; dirty until SaveCompleted | a:frankmd-workspace::autosave | keep |
| 71 | 1381 "remaps a pending autosave inside a renamed folder" | Folder rename carries pending saves | §3.1 | G: docs/foo.md dirty / W: rename folder docs→archive; Tick / T: Save to archive/foo.md only | a:frankmd-workspace::autosave | keep |
| 72 | 1396 "moves the active local draft before scheduling a save at the renamed path" | Draft survives the rename | Drafts by DocId | G: foo.md draft / W: rename to bar.md / T: draft for DocId unchanged and restorable; save timer still armed; Save goes to bar.md | a:frankmd-workspace::drafts + autosave | changed (no draft move needed) |
| 73 | 1415 "makes a remap failure recoverable and blocks writes until the user resolves it" | A failed rename never strands the draft | Drafts + rename flow §5.13 | G: X dirty with draft / W: rename fails (fs error) / T: X stays at old path; draft intact; autosave keeps targeting the old path; status error message | a:frankmd-workspace::autosave + drafts | changed (DocId drafts; no remap step) |
| 74 | 1468 "retains the source draft and retries recovery when fallback storage also fails" | If drafts can't be written the snapshot is kept and retried | Draft storage failed banner §5.12 | G: draft writes failing during rename / W: rename; later writes succeed / T: in-memory snapshot retained; DraftStorageFailed banner; next Tick retries and clears banner on success | a:frankmd-workspace::drafts + banners | changed (§5.12 banner) |
| 75 | 1504 "does not let a late save response remove the source draft after remap failure" | A late save never deletes a draft for newer text | Autosave | G: Save in flight; draft write pending retry; rename / W: SaveCompleted Ok for the old snapshot / T: draft (newer text) not discarded; doc still dirty | a:frankmd-workspace::autosave + drafts | changed (DocId drafts) |
| 76 | 1547 "does not overwrite a destination draft collision and blocks local writes there" | Another note's leftover draft is never overwritten | Drafts by DocId | G: deleted-doc Y left a draft at bar.md's old identity / W: rename X to bar.md, edit / T: X's writes go to X's draft; Y's draft byte-identical | a:frankmd-workspace::drafts | changed (DocId drafts) |
| 77 | 1570 "does not remap a path that only shares a folder prefix" | Folder rename is segment-wise | RelPath::starts_with (HP-008) | G: docs-old/foo.md open / W: rename folder docs→archive / T: doc path unchanged; its pending save still targets docs-old/foo.md | a:frankmd-workspace::autosave | keep |
| 78 | 1577 "clears a pending autosave when the note is deleted" | Deleted note is never re-created by autosave | Delete §5.13, HP-006 | G: dirty, timer armed / W: delete note; Tick +2000 ms / T: no active doc; timers disarmed; not dirty; no Save | a:frankmd-workspace::autosave | keep |
| 79 | 1591 "removes file drafts and legacy backups after a successful deletion" | Deleting a note removes its drafts | §5.13 | G: X has a draft / W: delete X Ok / T: drafts for X removed (count reported); empty state | a:frankmd-workspace::drafts | keep |
| 80 | 1602 "clears active state even when local deletion cleanup fails" | Note closes even if draft cleanup fails | §5.13; status slot | G: draft discard fails / W: delete X / T: no active doc, not dirty, no timer; error message in status slot (5 s) | a:frankmd-workspace::drafts + status | keep |
| 81 | 1614 "ignores a late response from a save started before deletion" | Late result after delete has no effect | HP-006 | G: Save in flight / W: delete X, then SaveCompleted Ok / T: no active doc; not dirty; no baseline revived | a:frankmd-workspace::autosave | keep |
| 82 | 1633 "reschedules the renamed note when the old in-flight save fails" | Failed save before a rename retries at the new path | Autosave | G: Save(foo) in flight / W: rename to bar, SaveCompleted Err / T: active bar.md; dirty; debounce re-armed | a:frankmd-workspace::autosave | keep |
| 83 | 1650 "keeps the remapped newer draft when an old-path save response arrives" | Newer draft wins over an older save ack | Autosave/drafts | G: Save(s1) in flight; Edited s2; rename / W: SaveCompleted Ok(s1) / T: draft = s2 kept; still dirty | a:frankmd-workspace::autosave + drafts | keep |

### `test/javascript/controllers/code_dialog_controller.test.js` — C=17 (keep 15 / changed 2 / dropped 0; a=14 b=3 c=0)
Original subject: code block dialog (language field with suggestions, content field keys, unknown-language confirm). New home: Code block modal §5.9 (opened by `/code`, Insert menu, Ctrl+. in a fence, InsertCodeBlock); VM `frankmd-workspace::code_block::CodeBlockForm`; dialog rules §5 (Ctrl+Enter confirms, Esc cancels); Esc chain §6.1. Not C (ported elsewhere as A): suggestion matching 103/112/119, fence serialization 227/245/266.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 54 "initializes code state" | A fresh dialog is in insert mode with no range | Code block §5.9 | G: — / W: CodeBlockForm::new_insert(caret) / T: mode Insert, edit range None, language "", content "" | a:frankmd-workspace::code_block | keep |
| 2 | 60 "initializes code languages array" | Suggestions cover the known languages | §5.9 (FE-391) | G: — / W: form.languages() / T: contains javascript, python, ruby; the 44 FrankMD ids in FrankMD order come first | a:frankmd-workspace::code_block | keep |
| 3 | 68 "opens dialog with empty fields for new code block" | `/code` opens an empty dialog | §5.9 | G: editor focused, caret outside fence / W: dispatch InsertCodeBlock / T: OverlayKind::Dialog(CodeBlock) open; `dialog-code-block` visible; language input focused; fields empty | a:frankmd-workspace::overlay + b | keep |
| 4 | 77 "opens dialog with existing code block for edit mode" | Ctrl+. in a fence edits that block | §5.9, context action §6.2 | G: caret in "```javascript\nconst x = 1;\n```" / W: ContextAction / T: form mode Update with fence range; language "javascript", content "const x = 1;"; primary button "Update" | a:frankmd-workspace::code_block | keep |
| 5 | 96 "closes the dialog" | Cancel/Esc closes and returns focus | Dialog anatomy §5, focus §4.3 | G: dialog open / W: press escape (no suggestions) / T: dialog gone; editor focused; text unchanged | b | keep |
| 6 | 128 "selects first suggestion on Tab when suggestions visible" | Tab accepts the top suggestion | §5.9 language input | G: language "pyth", suggestions visible / W: form.key_tab() / T: language "python"; suggestions hidden; Tab consumed (not a focus move) | a:frankmd-workspace::code_block + b | keep |
| 7 | 141 "hides suggestions on Escape" | Esc closes the suggestion list first, not the dialog | Esc chain §6.1 (popover before dialog) | G: suggestions visible / W: Escape / T: escape_target = Popover; suggestions hidden; dialog still open | a:frankmd-workspace::overlay + code_block | keep |
| 8 | 149 "calls insert on Ctrl+Enter" | Ctrl+Enter confirms from the language field | Dialog rule §5 | G: language input focused / W: press ctrl-enter / T: Insert applied; dialog closed | b | keep |
| 9 | 161 "calls insert on Ctrl+Enter" (content field) | Ctrl+Enter confirms from the code area | Dialog rule §5 | G: code area focused / W: press ctrl-enter / T: Insert applied; dialog closed | b | keep |
| 10 | 171 "inserts spaces on Tab" | Tab in the code area indents with 2 spaces | §5.9 code area | G: content "line1", caret 5, tabs off / W: form.indent_at_caret() / T: "line1  " caret 7 | a:frankmd-workspace::code_block | keep |
| 11 | 184 "inserts tabs when useTabs is checked" | "Indent with tabs" inserts \t | §5.9 checkbox | G: tabs on / W: indent_at_caret / T: "line1\t" | a:frankmd-workspace::code_block | keep |
| 12 | 199 "populates suggestions container" | Matching languages show as rows | §5.9 suggestion list | G: form / W: language "java" / T: suggestion rows [javascript, java], list visible | a:frankmd-workspace::code_block | keep |
| 13 | 208 "hides suggestions container" | Suggestions can be hidden | §5.9 | G: suggestions visible / W: form.hide_suggestions() / T: list hidden, rows kept for next input | a:frankmd-workspace::code_block | keep |
| 14 | 217 "sets language value from clicked suggestion" | Clicking a suggestion picks it | §5.9 | G: suggestions incl. python / W: click `code-lang-python` (VM: select_suggestion(python)) / T: language "python"; list hidden | a:frankmd-workspace::code_block + b | keep |
| 15 | 280 "shows confirmation for unrecognized language" | Unknown language is flagged before insert | Inline warning row §5.9 (replaces confirm, FE-395) | G: language "pyhton", content "code" / W: Confirm / T: no insert; warning row "‘pyhton’ isn't a known language. Did you mean python?" with Use python / Keep pyhton; no alert dialog queued | a:frankmd-workspace::code_block | changed (§5.9 inline warning replaces appConfirm) |
| 16 | 289 "does not insert when confirmation is cancelled" | Nothing is inserted until the user chooses | §5.9 | G: warning row shown / W: (a) nothing, (b) Use python, (c) Keep pyhton / T: (a) no edit; (b) fence "```python"; (c) fence "```pyhton" | a:frankmd-workspace::code_block | changed (§5.9 two-button choice) |
| 17 | 300 "closes dialog after insert" | Insert closes the dialog and edits once | §5.9; one undo step | G: language javascript / W: Confirm / T: Edit emitted (single undo step); dialog closed; focus back to editor | a:frankmd-workspace::code_block + b | keep |

### `test/javascript/controllers/codemirror_controller.test.js` — C=73 (keep 59 / changed 8 / dropped 6; a=62 b=5 c=0)
Original subject: CodeMirror wrapper (editor API, per-note undo history + LRU, history-boundary hooks, cursor/scroll helpers, editor prefs, click recentering, read-only lock). New home: the editor pane over the gpui-component editor (`editor` id, §3.1/§6.2) with pure logic in `frankmd-markdown::{edit,linemap}`, `frankmd-workspace::{undo_cache,editor_state,scroll_sync,typewriter,navigation,disk_sync,commands}`; undo-creation boundary §5.13; read-only banner §5.12; Settings › Editor/Writing §5.16. Not C (ported as A): toggleTaskAtLine 554/586, toggleTask 601/610/619/628.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 49 "creates an editor instance" | — | — | editor instance creation has no user-observable effect beyond row 2 | — | dropped (CodeMirror-internal) |
| 2 | 54 "initializes with content from value" | Opening a note shows its text | Editor pane §3.1 | G: vault with a.md "Hello World" / W: open a.md / T: `editor` visible, value "Hello World" | b | keep |
| 3 | 58 "syncs initial content to hidden textarea" | — | — | hidden textarea removed (§1, FE-074) | — | dropped (design-removed §1) |
| 4 | 64 "destroys the editor" | Closing a note removes the editor | Close note §5.23 | G: note open / W: press ctrl-w / T: `editor` gone; `empty-state` visible; snapshot has no active doc | a:frankmd-workspace::navigation + b | changed (§5.23 Ctrl+W close replaces disconnect) |
| 5 | 76 "returns the full document text" | Editor text equals the doc the workspace saves | Editor ↔ workspace (architecture §3.2) | G: note open, typed "abc" / W: read `editor` value and last Command::Edited / T: both equal full text | b | keep |
| 6 | 80 "returns empty string if no editor" | Document commands need a note | Palette availability §5.1 | G: no note open / W: availability(CopyContents) / T: Unavailable("Open a markdown note first") | a:frankmd-workspace::commands | changed (§5.1 availability replaces null editor) |
| 7 | 87 "replaces the document content" | Whole-buffer replace (Reload) | Changed on disk Reload §5.12 | G: note "a", file now "b" / W: Reload / T: Effect::ReplaceBuffer("b") as one edit; not dirty | a:frankmd-workspace::disk_sync | keep |
| 8 | 92 "syncs to hidden textarea" | — | — | hidden textarea removed (§1, FE-074) | — | dropped (design-removed §1) |
| 9 | 99 "keeps a recovered draft when the preloaded value callback runs" | A late load result never overwrites a restored draft | Navigation generation (HP-003) | G: open X auto-restored draft "Local draft" / W: stale DocLoaded(X, older gen, file text) / T: text stays "Local draft"; no Edited event | a:frankmd-workspace::navigation | keep |
| 10 | 112 "loads content without recording it as a user change" | Opening a note is not an edit and not undoable | Navigation + undo | G: B dirty earlier / W: open C / T: no Edited, C not dirty; undo_cache entry for C empty; Ctrl+Z leaves text unchanged | a:frankmd-workspace::undo_cache + b | keep |
| 11 | 126 "preserves independent undo and redo history for each note" | Each note keeps its own undo history | Undo cache (architecture §3.1) | G: edit A, switch B, edit B / W: back to A, undo/redo; back to B, undo/redo / T: each restores only its own baseline/edit | a:frankmd-workspace::undo_cache + b | keep |
| 12 | 153 "remaps active and cached history paths when a folder is moved" | History survives a folder move | DocId keying §3.1 | G: edited old/first.md, old/nested/second.md / W: rename folder old→new, reopen both / T: undo restores each baseline (entries keyed by DocId, no remap) | a:frankmd-workspace::undo_cache | changed (DocId keying) |
| 13 | 174 "remaps a file history and discards stale destination history" | History follows a renamed note; never borrowed by another | DocId keying §3.1 | G: X edited; Y edited then deleted / W: rename X onto Y's former path, reopen / T: undo yields X's baseline; Y's history gone | a:frankmd-workspace::undo_cache | changed (DocId keying) |
| 14 | 190 "evicts deleted file and folder subtree history before a path can be reused" | Deleted notes leave no undo history | Delete §5.13 | G: edited deleted.md and deleted-folder/nested.md / W: delete both, create new notes at same paths / T: undo has nothing for the new notes | a:frankmd-workspace::undo_cache | keep |
| 15 | 209 "discards a note's cached history when its loaded content no longer matches" | External change invalidates history | Undo cache + disk_sync | G: A edited, cached / W: A reloaded with different disk text / T: cache entry for A dropped; undo does nothing | a:frankmd-workspace::undo_cache | keep |
| 16 | 222 "evicts the least-recently-used state beyond 20 cached notes" | Memory bounded at 20 notes (#192) | Undo cache LRU 20 | G: — / W: open notes 1..22 / T: 20 entries; note-1 evicted; note-2 and note-21 kept | a:frankmd-workspace::undo_cache | keep |
| 17 | 235 "never evicts the active note's cached state" | Active note's history is never dropped | Undo cache | G: 21 entries, active = LRU entry / W: evict / T: active kept; next LRU evicted | a:frankmd-workspace::undo_cache | keep |
| 18 | 262 "keeps states for recently re-visited notes" | Revisiting refreshes recency and restores state | Undo cache | G: notes 2..21 cached / W: revisit 3, open 23, 24 / T: 3 restored from cache (no fresh load); 2 evicted; 3 kept | a:frankmd-workspace::undo_cache | keep |
| 19 | 297 "uses normal text undo before asking the app to handle an exhausted history" | Ctrl+Z undoes text first, then offers undo of creation | Undo note creation §5.13 | G: just-created note, one edit / W: ctrl-z twice / T: 1st restores baseline; 2nd opens UndoCreation dialog "Delete the note you just created?" once | a:frankmd-workspace::dialogs + b | keep |
| 20 | 313 "leaves exhausted Ctrl+Z to Vim mode" | Vim keeps its own undo | Vim §6.3 | G: vim on, just-created note, no edits / W: ctrl-z / T: no UndoCreation dialog | a:frankmd-vim::keymap + b | keep |
| 21 | 338 "uses the active note's redo history before asking the app to handle an exhausted history" | Ctrl+Y redoes text first, then the boundary handler | Undo/Redo §6.2 (FE-049/062) | G: edit then undo / W: ctrl-y twice / T: 1st re-applies edit; 2nd routes once to the redo-creation boundary | a:frankmd-workspace::dialogs + b | keep |
| 22 | 355 "routes exhausted Ctrl+Shift+Z to the creation boundary" | Ctrl+Shift+Z is a redo alias | §6.2 | G: no redo history / W: ctrl-shift-z / T: boundary handler invoked once | b | keep |
| 23 | 366 "leaves exhausted redo to Vim mode" | Vim keeps its own redo | §6.3 | G: vim on / W: ctrl-y at history end / T: boundary not invoked | a:frankmd-vim::keymap | keep |
| 24 | 379 "returns selection info with from, to, and text" | — | — | API shape only; covered by row 25 | — | dropped (CodeMirror-internal) |
| 25 | 386 "returns selected text" | Selection text feeds find pre-fill etc. | `markdown::edit` Selection | G: "Hello World" sel 0..5 / W: selection_text / T: "Hello", from 0, to 5 | a:frankmd-markdown::edit | keep |
| 26 | 397 "sets selection range" | Programmatic selection is exact | `markdown::edit` | G: "Hello World" / W: set_selection(6,11) / T: from 6, to 11 | a:frankmd-markdown::edit | keep |
| 27 | 407 "replaces selected text" | Inserts replace the selection | `markdown::edit::apply` | G: sel "World" / W: replace_selection("Universe") / T: "Hello Universe" | a:frankmd-markdown::edit | keep |
| 28 | 416 "inserts text at position" | Insert at offset | `markdown::edit` | G: "Hello World" / W: insert_at(5," Beautiful") / T: "Hello Beautiful World" | a:frankmd-markdown::edit | keep |
| 29 | 424 "replaces text in range" | Range replace | `markdown::edit` | G: "Hello World" / W: replace_range(0..5,"Goodbye") / T: "Goodbye World" | a:frankmd-markdown::edit | keep |
| 30 | 432 "focuses the editor" | Focus can be sent to the editor | Focus model §4.3 | G: sidebar focused / W: press ctrl-2 (FocusEditor) / T: `editor` focused; FocusRegion::Editor | a:frankmd-workspace::focus + b | keep |
| 31 | 440 "returns focus state" | Focus state is queryable | §4.3 | G: overlay closed / W: press f6 cycle / T: `editor`.focused() matches FocusRegion | b | keep |
| 32 | 446 "returns scroll info object" | — | — | object shape only | — | dropped (CodeMirror-internal) |
| 33 | 456 "sets scroll position" | Scroll requests are clamped | Scroll math | G: 100 lines, viewport 20 / W: clamp_scroll(50) and (−10) / T: in [0, max] | a:frankmd-workspace::scroll_sync | keep |
| 34 | 467 "returns a number between 0 and 1" | Scroll ratio is bounded | scroll_sync | G: any metrics incl. content < viewport / W: scroll_ratio / T: 0 ≤ r ≤ 1, no NaN | a:frankmd-workspace::scroll_sync | keep |
| 35 | 475 "returns a valid 1-based line number" | Top visible line is 1-based | scroll_sync | G: 3 lines / W: top_visible_line(0) / T: in 1..=3 | a:frankmd-workspace::scroll_sync | keep |
| 36 | 482 "uses lineBlockAtHeight at the top of the viewport" | Top line probes 1 px below the viewport top | scroll_sync | G: block tops with line 5 starting at y=120 / W: top_visible_line(scroll_top 120) / T: probe y=121 → 5 | a:frankmd-workspace::scroll_sync | keep |
| 37 | 499 "returns 1 when the editor is missing" | Empty layout → line 1 | scroll_sync | G: empty line layout / W: top_visible_line / T: 1 | a:frankmd-workspace::scroll_sync | keep |
| 38 | 506 "scrolls so the line block's top is at the viewport top" | Scroll-to-line puts the line at the top | scroll_sync | G: line 6 top = 240 / W: scroll_top_for_line(6) / T: 240 | a:frankmd-workspace::scroll_sync | keep |
| 39 | 526 "clamps the requested line into the document" | Out-of-range lines clamp | scroll_sync | G: 10 lines / W: line 99 and 0 / T: uses 10 and 1 | a:frankmd-workspace::scroll_sync | keep |
| 40 | 547 "does nothing when the editor is missing" | Go to line needs a note | §5.1 availability | G: no note open / W: availability(GoToLine) / T: Unavailable | a:frankmd-workspace::commands | changed (§5.1 availability) |
| 41 | 564 "preserves the cursor position while toggling" | Toggling a task from the preview keeps the caret | Preview checkbox §5.21 | G: "- [ ] open\nplain", caret 1:7 / W: toggle_task_at_line(1) / T: "- [x] open\nplain"; caret {line 1, col 7, offset 6} | a:frankmd-markdown::edit | keep |
| 42 | 574 "is a no-op on a non-task line" | Non-task lines are untouched, no edit recorded | §5.21 | G: "plain text" / W: toggle_task_at_line(1) / T: returns None; no Edit; not dirty | a:frankmd-markdown::edit | keep |
| 43 | 594 "does nothing when the editor is missing" | Task toggle needs a note | §5.1 | G: no note open / W: availability(ToggleTask) / T: Unavailable | a:frankmd-workspace::commands | changed (§5.1 availability) |
| 44 | 639 "is one undoable edit" | Multi-line toggle undoes in one step | Toggle task Ctrl+Enter §6.2 | G: "- [ ] a\nplain" sel 0..9 / W: ctrl-enter, then ctrl-z / T: toggle_task returns a single Edit; after undo text "- [ ] a\nplain" | a:frankmd-markdown::edit + b | keep |
| 45 | 650 "does nothing when the editor is missing" | Ctrl+Enter without a note is harmless | §5.1, §6.2 | G: empty state / W: press ctrl-enter / T: no Edit; ToggleTask Unavailable | a:frankmd-workspace::commands | changed (§5.1 availability) |
| 46 | 657 "returns cursor position with line, column, offset" | Caret position is shown | Status Position §3.6 | G: caret at offset 0 / W: status position / T: "1:1" | a:frankmd-workspace::status | keep |
| 47 | 664 "returns correct position" | Offset → 1-based line/col | `markdown::linemap` | G: "Line 1\nLine 2\nLine 3" / W: position(line 2, col 3) round-trip / T: line 2, col 3 | a:frankmd-markdown::linemap | keep |
| 48 | 674 "sets cursor to line and column" | (line,col) → offset | linemap | G: same text / W: offset_of(2,5) / T: back to line 2 col 5 | a:frankmd-markdown::linemap | keep |
| 49 | 682 "clamps to valid range" | Out-of-range caret clamps | linemap | G: 2 lines / W: offset_of(999,999) / T: line 2, end of line | a:frankmd-markdown::linemap | keep |
| 50 | 691 "returns total number of lines" | Total lines for `12/340` | linemap; status §3.6 | G: 3 lines / W: line_count / T: 3 | a:frankmd-markdown::linemap | keep |
| 51 | 698 "returns content of specific line" | Line text lookup | linemap | G: 3 lines / W: line_text(2) / T: "Line 2" | a:frankmd-markdown::linemap | keep |
| 52 | 703 "returns empty string for invalid line" | Invalid lines → "" | linemap | G: 1 line / W: line_text(999), line_text(0) / T: "" both | a:frankmd-markdown::linemap | keep |
| 53 | 711 "returns currentLine and totalLines" | Status shows line/total when line numbers off | Status Position §3.6 | G: 3 lines, caret line 1, line numbers Off / W: status position / T: "1/3" | a:frankmd-workspace::status | keep |
| 54 | 721 "updates font family value" | Font change applies live | Settings › Editor §5.16 | G: font "System monospace" / W: set editor_font "Monaco" / T: EditorPrefs.font_family updated; editor re-renders without reopen | a:frankmd-workspace::editor_state | keep |
| 55 | 728 "updates font size value" | Font size applies live (12–24) | §5.16 | G: 14 / W: set 16 / T: prefs 16; 30 clamps to 24 | a:frankmd-workspace::editor_state | keep |
| 56 | 735 "updates line height value" | Line height applies live | `.fed` editor_line_height (no Settings row) | G: 1.6 / W: set 1.8 / T: prefs 1.8 | a:frankmd-workspace::editor_state | keep |
| 57 | 742 "cycles through line number modes" | Ctrl+L cycles Off → Absolute → Relative → Off | §6.2 Ctrl+L, §5.16 | G: Off / W: ToggleLineNumbers ×3 / T: Absolute, Relative, Off; status position format switches to "n/total" when Off | a:frankmd-workspace::editor_state + b | keep |
| 58 | 761 "updates typewriter mode value" | Typewriter on/off is settable | View modes §4.2 | G: off / W: set true, set false / T: LayoutState.typewriter true then false | a:frankmd-workspace::layout | keep |
| 59 | 771 "updates the live editor preference" | Recenter-on-click applies live | Settings › Writing §5.16 | G: off / W: set cursor_recenter_on_click true / T: EditorPrefs value true, used by the next click | a:frankmd-workspace::editor_state | keep |
| 60 | 778 "reapplies the current preference when restoring a note state" | The preference is global, not per note | §5.16 | G: open first, enable recenter, open second / W: reopen first from undo cache / T: recenter still true | a:frankmd-workspace::editor_state + undo_cache | keep |
| 61 | 808 "recenters a collapsed click when enabled (Typewriter %s)" (it.each false/true) | Click recenters the caret line | Typewriter/recenter §4.2 | G: recenter on, typewriter off and on; client 100, scroll 25, caret top 300 / W: recenter_after_click(collapsed) / T: new scroll_top 275 in both | a:frankmd-workspace::typewriter + b | keep |
| 62 | 816 "does not recenter a range selection" | Drag selections don't jump | §4.2 | G: recenter on / W: pointer selection 2..7 / T: None (scroll stays 25) | a:frankmd-workspace::typewriter | keep |
| 63 | 828 "does not recenter a click when the setting is off in Typewriter mode" | Setting off means no recenter | §4.2 | G: recenter off, typewriter on / W: collapsed click / T: None | a:frankmd-workspace::typewriter | keep |
| 64 | 838 "toggles typewriter mode and returns new state" | Ctrl+\ toggles typewriter | §6.1 | G: off / W: ToggleTypewriter twice / T: on then off; indicator `text-cursor` shown only when on | a:frankmd-workspace::layout + b | keep |
| 65 | 852 "returns typewriter mode state" | Typewriter state is exposed | §4.2; palette check §5.1 | G: off → set on / W: palette rows / T: "Typewriter" row toggle check matches | a:frankmd-workspace::commands | keep |
| 66 | 860 "updates read-only state" | Read-only notes reject edits | Read-only banner §5.12 | G: note read-only on disk / W: type "x" / T: text unchanged; Read-only banner; `lock` indicator | a:frankmd-workspace::editor_state + b | keep |
| 67 | 866 "keeps a temporary interaction lock until released, even if base read-only changes" | A grammar check lock wins over base read-only | Grammar check §5.10 | G: lock acquired / W: base read_only=false, then release / T: read-only while locked; editable after release | a:frankmd-workspace::editor_state | keep |
| 68 | 879 "returns the EditorView instance" | — | — | accessor only | — | dropped (CodeMirror-internal) |
| 69 | 886 "returns character position for line" | Line → offset | linemap | G: "Line 1\nLine 2\nLine 3" / W: line_start(2) / T: 7 | a:frankmd-markdown::linemap | keep |
| 70 | 894 "moves cursor to specified line" | Go to line moves the caret | Go to line Ctrl+G / `:3` §6.2/§6.3 | G: 3 lines / W: GoToLine 3 / T: caret line 3; status "3:1" | a:frankmd-markdown::linemap + b | keep |
| 71 | 903 "dispatches change event on document change" | Every user edit reaches the workspace | Event flow (architecture §6) | G: note open / W: input "New content" / T: Command::Edited{doc, text} received; doc dirty | b | keep |
| 72 | 918 "updates editor when content value changes externally" | External changes are surfaced, not silently applied | Changed on disk banner §5.12 | G: note open, file changes on disk / W: WatchEvent::Modified / T: ChangedOnDisk banner with Reload/Compare; text unchanged until Reload | a:frankmd-workspace::disk_sync | changed (§5.12 banner replaces silent replace) |
| 73 | 924 "does not update if content is the same" | Identical external content is a no-op | disk_sync (self-write suppression §12.2) | G: note open / W: WatchEvent with identical text/revision / T: no banner; no buffer edit | a:frankmd-workspace::disk_sync | keep |

### `test/javascript/controllers/connection_monitor_controller.test.js` — C=21 (keep 8 / changed 12 / dropped 1; a=20 b=0 c=0)
Original subject: HTTP health polling (`HEAD /up`), offline banner, textarea disabling. New home: vault reachability `frankmd-workspace::connectivity` (probe = stat/readdir of the vault root, injected Clock), Vault unreachable banner §5.12, vault segment of the status line §3.6; autosave reacts to its events (§5.12 "Edits are kept as drafts"). Polling over HTTP is removed (§1, FE-155).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 48 "initializes with online state" | App starts assuming reachable, no banner | connectivity | G: vault opened / W: Connectivity::new() / T: Reachable; no banner; status vault segment normal | a:frankmd-workspace::connectivity | keep |
| 2 | 52 "starts monitoring on connect" | Reachability is checked periodically | connectivity (§1 replaces polling) | G: vault open, fake clock / W: Tick +5000 ms / T: Effect::ProbeVault(gen) emitted | a:frankmd-workspace::connectivity | changed (vault probe replaces /up polling) |
| 3 | 58 "sets online state when fetch succeeds" | Readable vault root → reachable | connectivity | G: probe in flight / W: ProbeResult Ok / T: Reachable | a:frankmd-workspace::connectivity | changed (vault probe) |
| 4 | 68 "sets offline state when fetch fails" | Missing/unmounted root → unreachable | connectivity | G: probe / W: ProbeResult Err(NotFound or EIO/ENOTCONN/ESTALE) / T: Unreachable(reason) | a:frankmd-workspace::connectivity | changed (vault probe) |
| 5 | 78 "sets offline state when response is not ok" | Root present but unusable → unreachable | connectivity | G: probe / W: ProbeResult Err(PermissionDenied or not a dir) / T: Unreachable(PermissionDenied) | a:frankmd-workspace::connectivity | changed (vault probe) |
| 6 | 88 "prevents overlapping checks" | One probe at a time | connectivity | G: probe in flight / W: Tick, Retry, Retry / T: no second ProbeVault effect | a:frankmd-workspace::connectivity | keep |
| 7 | 101 "shows banner when going offline" | Unreachable vault is announced in place | Vault unreachable banner §5.12; status §3.6 | G: Reachable / W: ProbeResult Err / T: banner VaultUnreachable "Can't reach ~/notes. Edits are kept as drafts." with Retry; vault segment danger + `hard-drive` | a:frankmd-workspace::banners + status | changed (§5.12 banner text/buttons) |
| 8 | 110 "disables textarea when going offline" | Editing continues while unreachable | §11 "Offline banner lies…" | G: note editable / W: becomes Unreachable / T: editor still editable; edits go to drafts | a:frankmd-workspace::editor_state + connectivity | changed (§11 editing no longer disabled) |
| 9 | 119 "dispatches offline event" | Other parts react to losing the vault | connectivity → autosave | G: Reachable / W: ProbeResult Err / T: Event::VaultUnreachable emitted once (autosave pauses, rows autosave 57/65) | a:frankmd-workspace::connectivity | keep |
| 10 | 128 "does nothing if already offline" | No duplicate banners/events | connectivity | G: Unreachable / W: another failed probe / T: no event; one banner | a:frankmd-workspace::connectivity | keep |
| 11 | 139 "hides banner when coming online" | Banner clears on recovery | §5.12 | G: Unreachable + banner / W: ProbeResult Ok / T: banner removed; vault segment normal | a:frankmd-workspace::banners + connectivity | keep |
| 12 | 149 "enables textarea when coming online" | Nothing to re-enable | §11 | G: Unreachable / W: ProbeResult Ok / T: editor_state read-only unchanged | a:frankmd-workspace::editor_state | changed (§11 editor never disabled) |
| 13 | 159 "dispatches online event" | Reconnect triggers pending saves | connectivity → autosave | G: Unreachable / W: ProbeResult Ok / T: Event::VaultReachable emitted once (autosave saves dirty docs) | a:frankmd-workspace::connectivity | keep |
| 14 | 168 "does nothing if already online" | No duplicate events | connectivity | G: Reachable / W: ProbeResult Ok / T: no event | a:frankmd-workspace::connectivity | keep |
| 15 | 179 "preserves disabled state if textarea was already disabled" | Reachability never changes read-only state | editor_state | G: note read-only (disk or lock) / W: Unreachable then Reachable / T: still read-only | a:frankmd-workspace::editor_state | changed (§11; read-only owned by editor_state) |
| 16 | 192 "re-enables textarea if it was enabled before going offline" | Editable notes stay editable throughout | editor_state | G: editable / W: Unreachable then Reachable / T: editable at every step | a:frankmd-workspace::editor_state | changed (§11) |
| 17 | 206 "calls checkConnection" | Retry checks immediately | Retry button §5.12 | G: Unreachable, no probe in flight / W: Retry (click `banner-vault-unreachable` Retry) / T: ProbeVault effect now, without waiting for the interval | a:frankmd-workspace::connectivity + b | keep |
| 18 | 218 "checks connection on browser online event" | System hints trigger an early re-check | connectivity | G: Unreachable / W: Command::External(WatchEvent on vault root) or WindowActivated / T: immediate ProbeVault | a:frankmd-workspace::connectivity | changed (watcher/focus replace window online event) |
| 19 | 228 "handles offline on browser offline event" | Real IO failures flip state at once | connectivity + autosave | G: Reachable / W: SaveCompleted Err(ENOTCONN/ESTALE/root NotFound) / T: Unreachable immediately, no probe wait | a:frankmd-workspace::connectivity | changed (IO errors replace window offline event) |
| 20 | 239 "stops monitoring interval" | Probes end with the vault | connectivity + vault switch | G: probes for vault A / W: SwitchVault to B; late ProbeResult(A gen) / T: no more A probes; late result ignored | a:frankmd-workspace::connectivity | changed (vault generation replaces disconnect) |
| 21 | 247 "removes event listeners" | — | — | window listener cleanup | — | dropped (DOM-specific) |

#### New VM requirements (G4)
- `frankmd-workspace::autosave` — `DocSession` per DocId on top of `AutosaveTimer`: `edited(text, now)`, `tick(now) -> Vec<Effect>`, `save_now()`, `save_completed(doc, edit_gen, Result<Revision>)`, single in-flight save, reschedule when text changed during save, baseline text + base revision, `vault_unreachable()/vault_reachable()` (disarm timers, keep dirty, save on reconnect), rename/folder-rename via registry `Event::Remapped` (segment-wise), delete clearing timers and ignoring late acks, `prepare_delete(doc) -> Ready{revision} / Wait / Stale`. Content-loss guard: `ContentLossGuard {active, approved_snapshot_hash}`, uses `frankmd_core::content_loss::is_large_deletion`, one-shot approval bound to the exact snapshot, `undo_result(text)`, `Effect::LoadBaseline(doc)` when the editor has no history, `dismiss()`, auto-clear when restored (autosave rows 3–24, 44–54, 57–58, 64–83).
- `frankmd-workspace::drafts` — `DraftStore` additions: `draft_revision` counter + `updated_at`, `discard_if_revision(doc, rev)`, `rebase(doc, new_base_revision)`, `remove_all_for(doc) -> usize`, write failures surfaced as `Event::DraftStorageFailed`/`Recovered`; 300 ms per-doc debounce (`DraftWriter`) independent across docs; in-memory retained snapshot + retry when writes fail (autosave rows 9–10, 14, 31–32, 41, 55–56, 59, 72–76, 79–80).
- `frankmd-workspace::lifecycle` (new) — `on_window_deactivated()` (sync draft flush, no save), `quit_requested() -> QuitDecision {Proceed{effects}, Ask{count}}` per §5.23, no redundant rewrite of unchanged drafts (autosave rows 11–15).
- `frankmd-workspace::status` — `SaveState::{Unsaved, Saving, Saved, SaveFailed, DraftNotStored}` labels resolved through i18n with en fallback and never a raw key; position formatting `line:col` vs `line/total` by `LineNumberMode` (autosave 1–2; codemirror 46, 53, 57).
- `frankmd-workspace::banners` — kinds `ContentLoss{removed_chars, percent}`, `DraftStorageFailed`, `VaultUnreachable{vault}`, `ReadOnly`, `ChangedOnDisk` with button ids (`banner-<kind>-<button>`) for b tests.
- `frankmd-workspace::recovery` — open-time decision `decide(draft, disk) -> AutoRestore / DiscardIdentical / Ask(RecoveryRecord)`; record carries draft time + file mtime; `RestoreDraft` re-bases draft and recomputes content-loss; `UseFile` discards only that DocId's draft; not offered again after a resolved choice (autosave 18–19, 33–38, 42, 60–63).
- `frankmd-workspace::navigation` — transition `prepare_switch(from) -> Ok / Refused(reason)` (flush draft, disarm timers, never force save; refuse when the draft can't be stored or the base revision is unknown); stale `DocLoaded` generations ignored (autosave 25–29, 54; codemirror 4, 9).
- `frankmd-workspace::undo_cache` (new) — LRU (cap 20) of editor undo snapshots keyed by DocId; active never evicted; refresh on revisit; invalidate when loaded text ≠ cached text; evict on delete (file and folder subtree) (codemirror 10–18, 60).
- `frankmd-workspace::dialogs::UndoCreation` — history-start (undo) and history-end (redo) boundary routing, suppressed in vim mode (codemirror 19–23).
- `frankmd-workspace::editor_state` (new) — `EditorPrefs {font_family, font_size 12–24, line_height, line_numbers: LineNumberMode Off→Absolute→Relative cycle, cursor_recenter_on_click}` (global, survive note switches); `ReadOnly {base, locks: counter}` with RAII-style `acquire_lock() -> LockId`/`release`; read-only is never touched by connectivity (codemirror 54–60, 66–67; connection 8, 12, 15–16).
- `frankmd-workspace::disk_sync` (new) — external-change handling: identical content no-op, different content → ChangedOnDisk banner, `Reload` → `Effect::ReplaceBuffer` (codemirror 7, 72–73).
- `frankmd-workspace::scroll_sync` — `top_visible_line(scroll_top, line_tops)` probing `scroll_top + 1`, 1 for empty layout; `scroll_top_for_line(n)` with clamp to [1, lines]; `clamp_scroll`; `scroll_ratio` in [0,1] (codemirror 33–39).
- `frankmd-workspace::typewriter` — `recenter_after_click(selection_collapsed, pref, caret_top, viewport_top, scroll_top, client_height) -> Option<f32>` = caret_top − viewport_top + scroll_top − client_height/2 (codemirror 61–63).
- `frankmd-workspace::code_block` (new) — `CodeBlockForm`: mode Insert/Update(range), language + content, languages list (44 FrankMD ids first), suggestion rows (reuses the A-ported matcher), `key_tab()` accept-first, `hide_suggestions`, `select_suggestion`, `indent_at_caret(use_tabs)`, `confirm() -> Insert(Edit) / UnknownLanguage{did_you_mean}`, `use_suggested()/keep_typed()` (code_dialog 1–17).
- `frankmd-workspace::connectivity` — `Connectivity {state: Reachable/Unreachable(reason), probe_in_flight, vault_gen}`, probe interval 5 s on the Clock, `retry()`, `on_probe_result(gen, Result)`, `on_io_error(kind)` classification (ENOTCONN/ESTALE/EIO/root NotFound/PermissionDenied), hint triggers (watcher event on root, window activation), emits `VaultUnreachable/VaultReachable` once per transition (connection 1–20).
- `frankmd-markdown::edit` — buffer primitives `selection_text`, `set_selection`, `replace_selection`, `insert_at`, `replace_range` returning `Edit`s applied the same way by the editor adapter; `toggle_task_at_line` returns a caret-preserving selection (codemirror 25–29, 41–42, 44).
- `frankmd-markdown::linemap` — `line_count`, `line_text(n)` ("" when out of range), `line_start(n)`, `position(offset) -> (line, col)` 1-based, `offset_of(line, col)` clamped (codemirror 47–52, 69–70).

#### Open questions (G4)
- Content-loss banner has no Dismiss button in §5.12 (only Undo / Save anyway). FrankMD had `dismissContentLossWarning`; is there a × or is it only cleared on note switch/close? (autosave 44)
- Switching notes: the plan keeps FrankMD's "flush draft, don't force a save" (autosave 25). Should native switch also emit a disk save, since local writes are cheap?
- "Dirty doc without a base revision" (autosave 27) has no obvious native trigger (every opened file has a revision; `Draft.base_revision` isn't optional). Remove the case, or make `base_revision: Option<Revision>` for deleted-on-disk notes?
- Vault probe cadence and form: a 5 s stat of the vault root (FrankMD's interval) vs. only on IO errors + watcher hints. Battery cost on laptops vs. how quickly the banner appears (connection 2).
- Quit while unreachable (§5.23) asks only when drafts couldn't be written *to the vault*; FrankMD's beforeunload also fired when the draft store itself failed. The plan asks in both cases (autosave 15); confirm.
- `editor_line_height` has no row in Settings › Editor (§5.16) but FrankMD's `.fed` has it. Keep it as a `.fed`-only key or add a row? (codemirror 56)
- Changed on disk with a *clean* buffer: §5.12 always shows a banner; FrankMD replaced the text silently. Auto-reload clean notes and only show the banner when dirty? (codemirror 72)
- Code block dialog: §5 says Tab/Shift+Tab move between dialog fields, but Tab inside the code area indents (code_dialog 10–11) and Tab in the language field accepts a suggestion (6). Confirm F6/Ctrl+Tab is the way out of the code area, as in the main editor (§9).
- Redo boundary handler (codemirror 21–22): §6.1 says Ctrl+Y/Ctrl+Shift+Z redo a deleted created note *when no note is open*; FrankMD also routed it from the editor at history end. Keep both entry points?

## 10. G5: content_search, custom_theme_editor, drag_drop, drop_images, emoji_picker, file_finder

### `test/javascript/controllers/content_search_controller.test.js` — C=40 (keep 36 / changed 4 / dropped 0; a=39 b=1 c=0)
Original subject: Ctrl+Shift+F content-search dialog (debounced `/notes/search`, result list, keyboard/mouse selection, spinner). New home: Search in files overlay §5.3 (`search-overlay`), lists §5 intro, Esc chain §6.1; VM `workspace::content_search` over `search::content`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 59 "initializes empty state" | Fresh search starts empty, nothing selected, nothing pending | Search overlay §5.3 | G: `ContentSearchModel::new(clock)` / W: — / T: results empty; selected 0; no pending deadline; input_mode Pointer | a:frankmd-workspace::content_search | keep |
| 2 | 68 "opens dialog" | Shortcut opens the search overlay with input focused | §5.3, §6.1 | G: workspace with a note / W: press ctrl-shift-f / T: `search-overlay` visible; its input focused; OverlayStack top = SearchInFiles | a:frankmd-workspace::overlay + b | keep |
| 3 | 74 "clears previous state" | Reopening forgets old query, results and selection | §5.3 | G: model with query "foo", 3 results, selected 2 / W: `open()` / T: query ""; results empty; selected 0 | a:frankmd-workspace::content_search | keep |
| 4 | 86 "clears results display" | Old result rows are not shown after reopen | §5.3 | G: model with results / W: `open()` / T: `rows()` empty | a:frankmd-workspace::content_search | keep |
| 5 | 93 "shows initial status message" | Empty overlay shows the "type to search (regex)" hint | §5.3 status line | G: — / W: `open()` / T: status == Hint(i18n `status.type_to_search_regex`) | a:frankmd-workspace::content_search | keep |
| 6 | 101 "closes the dialog" | Esc closes search and focus returns to previous region | §5.3, §4.3 | G: overlay open from editor / W: press escape / T: `search-overlay` absent; `editor` focused; VM `escape_target` = Overlay | a:frankmd-workspace::overlay + b | keep |
| 7 | 108 "clears results when query is empty" | Deleting the query empties results immediately (no search) | §5.3 | G: results for "foo" / W: `set_query("")` / T: results empty; no pending search; status Hint | a:frankmd-workspace::content_search | keep |
| 8 | 117 "shows searching status" | User sees the search is in progress | §5.3 (2 px indeterminate bar) | G: — / W: `set_query("hello")` / T: `searching()` true (drives bar); status text unchanged (no "Searching…" line) | a:frankmd-workspace::content_search | changed (§5.3 indeterminate bar replaces searching text) |
| 9 | 124 "sets up debounced search" | Typing doesn't search on every keystroke; search fires after 300 ms | §5.3 | G: FakeClock t0 / W: `set_query("hello")`; tick 299 ms; tick 1 ms / T: no Effect::Search before 300 ms; exactly one with query "hello" at 300 ms | a:frankmd-workspace::content_search | keep |
| 10 | 135 "cancels previous timeout on new input" | Only the latest query is searched | §5.3 | G: FakeClock / W: "hel" at t0, "hello" at t0+200, tick to t0+500 / T: one Effect::Search("hello"); none for "hel" | a:frankmd-workspace::content_search | keep |
| 11 | 150 "fetches search results" | The query reaches the in-files search verbatim (regex on, case off) | §5.3, `search::content` | G: debounce elapsed for "hello" / W: take effect / T: Effect::Search{query "hello", regex true, case false, scope vault} | a:frankmd-workspace::content_search | keep |
| 12 | 164 "updates results data from HTML response" | Each result knows its note path and line | §5.3 grouped results | G: model awaiting gen N / W: `apply_results(N, [file1.md:10, file2.md:20])` / T: matches = [(file1.md,10),(file2.md,20)] grouped by note; header rows not selectable | a:frankmd-workspace::content_search | keep |
| 13 | 182 "resets selected index" | New results start at the first match | §5.3 | G: selected 3 / W: apply_results / T: selected 0 (first match, not a header) | a:frankmd-workspace::content_search | keep |
| 14 | 194 "shows match count in status" | Status shows how many matches were found and when truncated | §5.3 status line | G: — / W: apply 2 matches in 2 notes; then 20 in 7 truncated / T: status Count{2,2,false}; then i18n "20 matches in 7 notes (showing first 20)" | a:frankmd-workspace::content_search | changed (§5.3 count format; §11 all through i18n) |
| 15 | 209 "shows no matches message" | Empty result says No matches | §5.3 | G: — / W: apply_results(N, []) / T: status NoMatches (i18n `status.no_matches`) | a:frankmd-workspace::content_search | keep |
| 16 | 220 "handles search errors" | A failed search shows an error, not stale results | §5.3 | G: — / W: `apply_error(N, io)` / T: status Error (i18n `status.search_error`); searching false | a:frankmd-workspace::content_search | keep |
| 17 | 239 "moves selection down on ArrowDown" | Down moves to next match; keyboard mode on | §5.3, §6.4 | G: 3 matches, sel 0 / W: `move_next()` (b: press down in input) / T: sel 1; input_mode Keyboard | a:frankmd-workspace::content_search + b | keep |
| 18 | 249 "moves selection up on ArrowUp" | Up moves to previous match | §5.3 | G: sel 2 / W: `move_prev()` / T: sel 1; input_mode Keyboard | a:frankmd-workspace::content_search | keep |
| 19 | 259 "does not go below 0" | Up at the first match | §5 lists wrap | G: sel 0 of 3 / W: move_prev / T: sel 2 (wraps to last) | a:frankmd-workspace::content_search | changed (§5 lists unify on wrap) |
| 20 | 268 "does not exceed list length" | Down at the last match | §5 lists wrap | G: sel 2 of 3 / W: move_next / T: sel 0 | a:frankmd-workspace::content_search | changed (§5 lists unify on wrap) |
| 21 | 277 "calls selectCurrent on Enter" | Enter opens the selected match | §5.3 | G: sel on (notes/a.md,12) / W: press enter / T: Command::OpenAt{notes/a.md, line 12}; overlay closed; `editor` focused | a:frankmd-workspace::content_search + b | keep |
| 22 | 289 "scrolls selected element into view" | Selected match is always visible in a long list | §5.3 | G: 60 matches, overlay 70 % height / W: press down ×40 / T: result row 40 visible() | b | keep |
| 23 | 299 "handles missing element gracefully" | Navigating an empty list neither crashes nor scrolls | §5.3 | G: no results / W: move_next; move_prev / T: sel 0; scroll_request None; no panic | a:frankmd-workspace::content_search | keep |
| 24 | 314 "updates selection on hover" | Pointer hover selects a match | §5.3 | G: sel 0, Pointer mode / W: `hover(2)` / T: sel 2; returns changed=true | a:frankmd-workspace::content_search | keep |
| 25 | 323 "ignores hover when using keyboard" | A stationary pointer doesn't steal the keyboard selection | §5.3 | G: move_next → Keyboard mode / W: hover(2) / T: sel stays 1 | a:frankmd-workspace::content_search | keep |
| 26 | 333 "does not update selection if same index" | Re-hovering the same row does nothing (no re-render) | §5.3 | G: sel 1 / W: hover(1) / T: returns changed=false (no notify) | a:frankmd-workspace::content_search | keep |
| 27 | 345 "re-enables mouse selection" | Moving the mouse returns to pointer mode | §5.3 | G: Keyboard mode / W: `pointer_moved()`; hover(2) / T: input_mode Pointer; sel 2 | a:frankmd-workspace::content_search | keep |
| 28 | 354 "dispatches selected event with path and line" | Clicking a match opens that note at that line | §5.3 | G: match (folder/note.md, 25) at row 0 / W: `activate_at(0)` (b: click result) / T: OpenAt{folder/note.md, 25}; overlay closed | a:frankmd-workspace::content_search | keep |
| 29 | 371 "dispatches selected event for current selection" | Activating uses the current selection | §5.3 | G: 2 matches sel 1 / W: `activate()` / T: OpenAt for match 1 | a:frankmd-workspace::content_search | keep |
| 30 | 386 "does nothing when no results" | Enter with no results is a no-op | §5.3 | G: no results / W: activate / T: None; overlay still open | a:frankmd-workspace::content_search | keep |
| 31 | 397 "dispatches event and closes dialog" | Opening a result closes the overlay (Ctrl+Enter keeps it) | §5.3 | G: sel match / W: activate(keep_open=false); then activate(keep_open=true) / T: first closes overlay; second leaves it open | a:frankmd-workspace::content_search | keep |
| 32 | 411 "shows spinner when search starts" | Busy indicator on while a search runs | §5.3 bar | G: — / W: debounce fires / T: searching true | a:frankmd-workspace::content_search | keep |
| 33 | 418 "hides spinner when search completes" | Busy indicator off after results | §5.3 | G: searching / W: apply_results / T: searching false | a:frankmd-workspace::content_search | keep |
| 34 | 428 "shows spinner during onInput debounce" | Indicator shows immediately while waiting for debounce | §5.3 | G: — / W: set_query("x"), tick 100 ms / T: searching true | a:frankmd-workspace::content_search | keep |
| 35 | 438 "hides spinner when input is cleared" | Clearing the query stops the indicator | §5.3 | G: searching / W: set_query("") / T: searching false; pending cancelled | a:frankmd-workspace::content_search | keep |
| 36 | 447 "hides spinner after performSearch completes" | Indicator off after completion (also stale gen ignored) | §5.3 | G: two searches gen1, gen2 / W: apply gen1 then gen2 / T: gen1 dropped; searching false only after gen2 | a:frankmd-workspace::content_search | keep |
| 37 | 456 "hides spinner after performSearch fails" | Indicator off after an error | §5.3 | G: searching / W: apply_error / T: searching false | a:frankmd-workspace::content_search | keep |
| 38 | 481 "updates selectedIndex" | Selecting a row sets it current | §5.3 | G: 3 matches / W: `select(2)` / T: sel 2 | a:frankmd-workspace::content_search | keep |
| 39 | 486 "removes selection classes from old item" | Only one row looks selected; old one loses it | §5 list rows | G: sel 0 / W: select(2) / T: `is_selected(0)` false | a:frankmd-workspace::content_search | keep |
| 40 | 493 "adds selection classes to new item" | New row shows the selected style | §5 list rows | G: sel 0 / W: select(2) / T: `is_selected(2)` true (visual bg + accent bar is part of the C2 visual pass) | a:frankmd-workspace::content_search | keep |

### `test/javascript/controllers/custom_theme_editor_controller.test.js` — C=3 (keep 3 / changed 0 / dropped 0; a=3 b=0 c=0)
Original subject: Visual custom theme editor (live preview, validation, contrast, reset, atomic save). New home: Custom theme editor modal §5.17 (`dialog-custom-theme`); VM `theme::custom`; save via `workspace::settings_vm`/frankmd-config store.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 52 "previews without persistence and Cancel restores the prior palette and focus" | Editing a colour previews live without saving; Cancel reverts theme and returns focus to opener | §5.17, §4.3 | G: active theme `dark`, editor opened from "Customize theme…" / W: set accent `#abcdef`; then cancel (b: Esc) / T: preview tokens accent = #abcdef; zero config writes; after cancel active = `dark`, no overrides; b: focus back on opener | a:frankmd-theme::custom + b | keep |
| 2 | 63 "rejects invalid hex, warns on contrast, and Reset inherits base colors" | Bad hex blocks Save with "Invalid color"; low contrast warns with ratio; Reset clears overrides; base switch changes appearance | §5.17 | G: editor open / W: accent `url(evil)`; then accent #112233 + text-primary #111111; reset(); base=light / T: field error InvalidColor, save disabled; contrast warning ratio "1.00" (<4.5); after reset draft colors {} save enabled warning None; preview appearance Light | a:frankmd-theme::custom | keep |
| 3 | 80 "keeps a failed save editable and retries a single atomic configuration update" | Failed save keeps dialog open and editable; retry writes theme+custom theme in one update; success applies custom theme | §5.17, §5.16 | G: draft accent #aabbcc, config store fails once / W: save; save again / T: 1st: status SaveFailed, dialog open, form enabled; 2nd: exactly one atomic write {theme:"custom", custom_theme:draft}; dialog closed; active = custom; accent #aabbcc | a:frankmd-theme::custom | keep |

### `test/javascript/controllers/drag_drop_controller.test.js` — C=37 (keep 31 / changed 2 / dropped 4; a=30 b=3 c=0)
Original subject: Explorer drag-and-drop move (multi-select #266, drop highlight, root drop, rename POST + turbo re-render, autosave handoff). New home: Explorer drag & drop §5.14 (`explorer-tree`, `explorer-row-<relpath>`); VM `workspace::explorer` (DragState, drop_target, MovePlan) + `workspace::autosave`. A rows (selection normalization line 126, isValidDropTarget ×4) are not listed.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 69 "initializes with null state" | No drag and no highlighted target at rest | §5.14 | G: `ExplorerState::new(tree)` / W: — / T: drag None; drop_target None | a:frankmd-workspace::explorer | keep |
| 2 | 76 "stores dragged item info" | Dragging a row carries its path and kind | §5.14 | G: tree with folder1/file1.md / W: `begin_drag(folder1/file1.md)` / T: payload [{folder1/file1.md, Note}] | a:frankmd-workspace::explorer | keep |
| 3 | 94 "drags the whole selection when the grabbed item is selected (#266)" | Grabbing a selected row drags the whole selection | §5.14 | G: selection {notes.md, folder1/file2.md} / W: begin_drag(notes.md) / T: payload both items | a:frankmd-workspace::explorer | keep |
| 4 | 108 "drags a single item when the grabbed item is not selected" | Grabbing an unselected row drags only it | §5.14 | G: selection {notes.md} / W: begin_drag(folder1/file1.md) / T: payload [folder1/file1.md] | a:frankmd-workspace::explorer | keep |
| 5 | 117 "drags a single item when it is the only selection" | Single selected row drags alone | §5.14 | G: selection {notes.md} / W: begin_drag(notes.md) / T: payload [notes.md] | a:frankmd-workspace::explorer | keep |
| 6 | 145 "sets dataTransfer data" | HTML5 DataTransfer effectAllowed/text payload | — | — | — | dropped (DOM-specific: DataTransfer; payload covered by row 2) |
| 7 | 162 "adds dragging class" | Dragged rows look dragged; multi-drag shows a count | §5.14 count badge | G: payload of 2 / W: begin_drag / T: `is_dragging(p)` true for both; `drag_badge_count()` 2 | a:frankmd-workspace::explorer | keep |
| 8 | 177 "clears active drop target highlight" | Starting a new drag clears any stale target outline | §5.14 | G: drop_target Some(folder2) left over / W: begin_drag / T: drop_target None | a:frankmd-workspace::explorer | keep |
| 9 | 198 "clears draggedItem" | Ending a drag (no drop) forgets the payload | §5.14 | G: dragging / W: `end_drag()` / T: drag None | a:frankmd-workspace::explorer | keep |
| 10 | 208 "removes dragging classes" | Rows stop looking dragged after the drag ends | §5.14 | G: dragging 2 rows / W: end_drag / T: no row is_dragging | a:frankmd-workspace::explorer | keep |
| 11 | 218 "clears active row and root highlights" | No row or root highlight remains after drag end | §5.14 | G: drop_target Root / W: end_drag / T: drop_target None | a:frankmd-workspace::explorer | keep |
| 12 | 234 "prevents default and sets drop effect" | Hovering a valid folder shows a move affordance | §5.14 cursor | G: dragging folder1/file1.md / W: `drag_hover(Row(folder2))` / T: cursor Move; drop_target Some(Folder folder2) | a:frankmd-workspace::explorer | keep |
| 13 | 248 "prevents default and sets drop effect" (root) | Hovering empty tree space is a valid move to root for nested items | §5.14 | G: dragging folder1/file1.md / W: drag_hover(Background) / T: cursor Move; drop_target Root | a:frankmd-workspace::explorer | keep |
| 14 | 261 "highlights folder under nested child target" | Hovering any part of a folder row (icon, chevron, label) targets that folder | §5.14 | G: dragging notes.md / W: drag from `explorer-row-notes.md` over the label/icon area of `explorer-row-folder2` / T: VM drop_target folder2 | b | keep |
| 15 | 276 "highlights folder when drag target is a text node" | Browser text-node event target resolution | — | — | — | dropped (DOM-specific: text-node `closest`; hit area covered by row 14) |
| 16 | 291 "keeps exactly one highlighted row while moving between folders" | Only one target is outlined at a time | §5.14 | G: dragging notes.md / W: hover folder1 then folder2 / T: drop_target folder2 only; `is_drop_target(folder1)` false | a:frankmd-workspace::explorer | keep |
| 17 | 315 "clears highlight for invalid targets" | Invalid targets show nothing: own parent via a note, the dragged folder, its descendants | §5.14 (note → its folder) | G: dragging folder1/file1.md / W: hover folder2, then note folder1/file1.md; dragging folder1: hover folder1, folder1/subfolder / T: folder2 → target; note row resolves to folder1 = current parent → None + NotAllowed; folder1 / subfolder → None; extra: hovering root note `notes.md` → Root | a:frankmd-workspace::explorer | changed (§5.14 drop on a note targets its folder) |
| 18 | 353 "does not highlight current parent folder" | Dropping into the folder it already lives in isn't offered | §5.14 | G: dragging folder1/file1.md / W: hover folder1 / T: drop_target None; cursor NotAllowed | a:frankmd-workspace::explorer | keep |
| 19 | 369 "does not clear highlights when moving within the tree" | Moving across child elements inside the tree doesn't flicker the target | §5.14 | G: drag hovering folder2 / W: pointer moves over folder2's chevron/icon/label children / T: drop_target stays folder2 throughout | b | keep |
| 20 | 383 "does not clear highlights when fallback pointer target is still inside the tree" | Browser relatedTarget=null fallback via elementFromPoint | — | — | — | dropped (DOM-specific: dragleave relatedTarget quirk) |
| 21 | 405 "clears highlights when leaving the tree container" | Dragging out of the tree clears the target | §5.14 | G: drop_target folder2 / W: `drag_hover(Outside)` (b: drag beyond `explorer-tree` bounds) / T: drop_target None | a:frankmd-workspace::explorer + b | keep |
| 22 | 472 "moves every dragged item of a multi-selection (#266)" | Dropping a multi-selection moves each item, in order | §5.14 | G: payload [folder1/file1.md, folder1/file2.md], target folder2 / W: `drop()` / T: MovePlan [(folder1/file1.md→folder2/file1.md),(folder1/file2.md→folder2/file2.md)] executed sequentially | a:frankmd-workspace::explorer | keep |
| 23 | 496 "prevents default and stops propagation" | A drop on a folder row is handled once (not also as a root drop) | §5.14 | G: dragging nested note / W: drop on `explorer-row-folder2` / T: exactly one move (into folder2), none to root | b | keep |
| 24 | 516 "removes drop-highlight class" | Target outline disappears after drop | §5.14 | G: drop_target folder2 / W: drop / T: drop_target None; drag None | a:frankmd-workspace::explorer | keep |
| 25 | 536 "adds drop-highlight-root for nested items" | Entering empty space with a nested payload highlights the root | §5.14 | G: dragging folder1/file1.md / W: drag_hover(Background) / T: drop_target Root; `root_highlighted()` true | a:frankmd-workspace::explorer | keep |
| 26 | 545 "does not highlight for root-level items" | Root-level items can't be "moved" to root | §5.14 | G: dragging notes.md / W: drag_hover(Background) / T: drop_target None | a:frankmd-workspace::explorer | keep |
| 27 | 556 "moves nested items to the root and skips items already there (#266)" | Root drop moves nested items, skips root ones | §5.14 | G: payload [notes.md, folder1/file1.md] / W: drop on Root / T: MovePlan [(folder1/file1.md→file1.md)] only | a:frankmd-workspace::explorer | keep |
| 28 | 576 "stops moving the group after the first failure" | A failed move stops the rest of the group | §5.14 | G: plan of 2, store fails move 1 / W: `execute_moves` / T: one move attempted; result Err(first) ; second untouched | a:frankmd-workspace::explorer | keep |
| 29 | 594 "clears active row highlight" (root drop) | Root drop leaves no outline | §5.14 | G: drop_target Root / W: drop / T: drop_target None | a:frankmd-workspace::explorer | keep |
| 30 | 615 "returns empty string when no app controller found" | Turbo-stream request needs the expanded list | — | — | — | dropped (Rails-specific: turbo re-render plumbing) |
| 31 | 619 "returns comma-separated expanded folders from app controller" | After a move the tree keeps the user's expanded folders | §5.14 expansion persisted | G: expanded {folder1, folder2} / W: move + tree reload / T: expansion set still {folder1, folder2} (+ destination chain) | a:frankmd-workspace::explorer | keep |
| 32 | 638 "sends turbo-stream request with expanded folders" | Move performs the rename and announces it (open note / recents follow) | §5.14 | G: open note folder1/a.md / W: execute move to folder2/a.md / T: store.move(old,new) called; Event::ItemMoved{old,new,Note}; open note path = folder2/a.md | a:frankmd-workspace::explorer | keep |
| 33 | 661 "opens the destination folder and its ancestors before posting" | A drop reveals the destination (folder and ancestors expanded) | §5.14 | G: dragging to folder2/sub/deep / W: drop / T: expansion ⊇ {folder2, folder2/sub, folder2/sub/deep} set before the move effect; moved row revealed | a:frankmd-workspace::explorer | keep |
| 34 | 685 "shows an app alert on error" | Move failure is reported to the user | §3.6 message slot, §2.1 | G: store move fails "Permission denied" / W: execute_moves / T: StatusLine error message (5 s, danger) with the error; no modal alert | a:frankmd-workspace::status | changed (§2.1/§3.6 status message replaces alert) |
| 35 | 693 "does not move an active file when its local draft cannot be persisted" | Moving the open note never loses unsaved edits | §5.14, §5.12 | G: open dirty note, `prepare_for_transition` → Err / W: drop it on folder2 / T: no move effect; note still at old path; buffer intact | a:frankmd-workspace::autosave | keep |
| 36 | 706 "resumes active autosave if a move request fails" | Autosave keeps working after a failed move | §5.14 | G: open note, transition prepared / W: move fails / T: `resume_after_transition` called; next edit schedules autosave | a:frankmd-workspace::autosave | keep |
| 37 | 725 "clears state" (disconnect) | Reloading the tree (vault switch) mid-drag leaves no stale drag | §5.14 | G: dragging, drop_target folder2 / W: `reset_tree(new_tree)` / T: drag None; drop_target None | a:frankmd-workspace::explorer | keep |

### `test/javascript/controllers/drop_images_controller.test.js` — C=8 (keep 8 / changed 0 / dropped 0; a=4 b=4 c=0)
Original subject: Insert-image "Upload/drop" source (ingest, dropzone highlight, paste auto-select, keyboard picker). New home: Insert image dialog §5.6, Upload source (`dialog-insert-image`, `upload-dropzone`); VM `workspace::insert_image`. B row (getImageUrl S3 key, line 132) not listed.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 59 "clears a prior selection and notifies the parent on a second ingest" | Adding new files drops the old selection; footer clears | §5.6 Upload + footer | G: Upload source with a.png selected / W: `ingest([b.png])` / T: selected None; footer "Selected" empty; Insert disabled; event Deselected{Upload} | a:frankmd-workspace::insert_image | keep |
| 2 | 74 "does not dispatch deselected when there was no prior selection" | No spurious footer change when nothing was selected | §5.6 | G: nothing selected / W: ingest([a.png]) / T: no Deselected event | a:frankmd-workspace::insert_image | keep |
| 3 | 84 "highlights on dragover and clears on drop" | Dropzone shows accent border while files hover; clears on drop | §5.6 | G: dialog on Upload / W: simulate external file drag over `upload-dropzone`, then drop / T: VM `dropzone_hover` true then false (border colour itself in visual pass) | b | keep |
| 4 | 92 "keeps the highlight when dragleave moves onto a child (no flicker)" | Moving over the dropzone's inner text/icon keeps the highlight | §5.6 | G: dragging over dropzone / W: move pointer over its label child / T: dropzone_hover stays true | b | keep |
| 5 | 101 "clears the highlight when the pointer truly leaves" | Leaving the dropzone clears the highlight | §5.6 | G: dropzone_hover / W: drag pointer outside `upload-dropzone` / T: dropzone_hover false | b | keep |
| 6 | 109 "ingests a pasted file and auto-selects it" | Pasted image arrives on Upload pre-selected | §5.6 (FE-358) | G: editor focused, clipboard image pasted.png / W: paste (b: ctrl-v) → `ingest_external(pasted.png)` / T: dialog-insert-image open on Upload; item 0 selected; footer name "pasted.png"; event Selected{Upload, pasted.png} | a:frankmd-workspace::insert_image + b | keep |
| 7 | 121 "selects nothing and shows feedback when the file is rejected" | Non-image paste/drop is refused with feedback | §5.6 inline error | G: Upload source / W: ingest_external(notes.txt) / T: selected None; no Selected event; inline error (danger) shown | a:frankmd-workspace::insert_image | keep |
| 8 | 148 "opens the picker on Enter and Space, ignores others" | Keyboard users can open the file chooser from the dropzone | §5.6 (portal FileChooser OM-011) | G: `upload-dropzone` focused, FakePortal / W: press enter; press space; press x / T: FileChooser requested twice; not for x (real portal: C manual) | b | keep |

### `test/javascript/controllers/emoji_picker_controller.test.js` — C=69 (keep 66 / changed 3 / dropped 0; a=66 b=2 c=1)
Original subject: Emoji/emoticon/icon picker dialog (tabs, grid nav with wrap, hover, click, preview, localized search loading). New home: Emoji picker popover §5.5 (`emoji-picker`), keys §6.4 Emoji grid; VM `core::emoji_picker` over `core::emoji` (+ `core::emoji::i18n`); insertion via `markdown::edit`. A rows (filtering ×9 line 210–280, searchEmojisWithI18n ×7 line 878–909) not listed.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 78 "initializes emoji data arrays" | All three catalogs available | §5.5 | G: `EmojiPicker::new(locale)` / W: — / T: emoji, emoticon and icon catalogs non-empty | a:frankmd-core::emoji_picker | keep |
| 2 | 85 "initializes filtered items with all emojis" | Picker starts showing every emoji | §5.5 | G: new / W: — / T: items == all emojis (after Frequently used row, if any) | a:frankmd-core::emoji_picker | keep |
| 3 | 89 "initializes selected index to 0" | First cell selected | §5.5 | G: new / T: selected 0 | a:frankmd-core::emoji_picker | keep |
| 4 | 93 "initializes active tab to emoji" | Emoji tab is the default | §5.5 | G: new / T: tab Emoji | a:frankmd-core::emoji_picker | keep |
| 5 | 99 "opens the dialog" | Shortcut opens the picker at the caret with search focused | §5.5 popover, §6.2 | G: editor focused / W: press ctrl-shift-e / T: `emoji-picker` visible; `emoji-search` focused | b | changed (§5.5 caret popover replaces modal dialog) |
| 6 | 104 "clears the search input" | Reopening starts with an empty search | §5.5 | G: query "heart" / W: `open()` / T: query "" | a:frankmd-core::emoji_picker | keep |
| 7 | 110 "resets to emoji tab" | Reopening goes back to Emoji tab | §5.5 | G: tab Icons / W: open / T: tab Emoji | a:frankmd-core::emoji_picker | keep |
| 8 | 116 "resets selected index to 0" | Reopening selects the first cell | §5.5 | G: selected 7 / W: open / T: selected 0 | a:frankmd-core::emoji_picker | keep |
| 9 | 122 "renders emoji grid on open" | Grid filled on open, Frequently used first | §5.5 | G: frequently used [smile] / W: open / T: `cells()` non-empty; first row = Frequently used; then all emojis | a:frankmd-core::emoji_picker | changed (§5.5 new Frequently used row) |
| 10 | 128 "updates preview on open" | Footer shows the first item's glyph and shortcode | §5.5 footer | G: — / W: open / T: footer = {glyph of item 0, ":<shortcode>:"} | a:frankmd-core::emoji_picker | keep |
| 11 | 135 "closes the dialog" | Esc closes picker and returns to the caret | §5.5, §6.1 Esc chain | G: picker open / W: `close()` (b: escape) / T: picker closed; editor focused, text unchanged | a:frankmd-core::emoji_picker + b | keep |
| 12 | 143 "switches to emoji tab" | Selecting Emoji tab shows emojis | §5.5 | G: tab Icons / W: `switch_tab(Emoji)` / T: tab Emoji; items emojis | a:frankmd-core::emoji_picker | keep |
| 13 | 149 "does nothing if already on emoji tab" | Re-selecting the active tab doesn't reset | §5.5 | G: tab Emoji sel 5 / W: switch_tab(Emoji) / T: returns false; sel 5 | a:frankmd-core::emoji_picker | keep |
| 14 | 156 "resets selected index when switching" | New tab starts at first cell | §5.5 | G: Icons sel 5 / W: switch_tab(Emoji) / T: sel 0 | a:frankmd-core::emoji_picker | keep |
| 15 | 165 "switches to emoticons tab" | Emoticons tab shows emoticons | §5.5 | G: Emoji / W: switch_tab(Emoticons) (b: Ctrl+Tab) / T: tab Emoticons; items emoticons | a:frankmd-core::emoji_picker | keep |
| 16 | 171 "does nothing if already on emoticons tab" | Same | §5.5 | G: Emoticons sel 3 / W: switch_tab(Emoticons) / T: false; sel 3 | a:frankmd-core::emoji_picker | keep |
| 17 | 178 "resets selected index when switching" | Same | §5.5 | G: Emoji sel 4 / W: switch_tab(Emoticons) / T: sel 0 | a:frankmd-core::emoji_picker | keep |
| 18 | 187 "switches to icons tab" | Icons tab shows Phosphor icons | §5.5 | G: Emoji / W: switch_tab(Icons) / T: tab Icons; items icons | a:frankmd-core::emoji_picker | keep |
| 19 | 193 "does nothing if already on icons tab" | Same | §5.5 | G: Icons sel 2 / W: switch_tab(Icons) / T: false; sel 2 | a:frankmd-core::emoji_picker | keep |
| 20 | 200 "resets selected index when switching" | Same | §5.5 | G: Emoji sel 4 / W: switch_tab(Icons) / T: sel 0 | a:frankmd-core::emoji_picker | keep |
| 21 | 290 "returns emoji columns for emoji tab" | Emoji grid is 10 wide | §5.5 (10/5/8) | G: tab Emoji / T: columns() 10 | a:frankmd-core::emoji_picker | keep |
| 22 | 295 "returns emoticon columns for emoticons tab" | Emoticons grid is 5 wide | §5.5 | G: Emoticons / T: columns() 5 | a:frankmd-core::emoji_picker | keep |
| 23 | 300 "returns icon columns for icons tab" | Icons grid is 8 wide | §5.5 | G: Icons / T: columns() 8 | a:frankmd-core::emoji_picker | keep |
| 24 | 312 "moves selection right" | Right arrow moves one cell | §6.4 | G: sel 0 / W: `move(Right)` / T: sel 1 | a:frankmd-core::emoji_picker | keep |
| 25 | 322 "wraps to beginning at end" | Right at last cell wraps to first | §6.4 | G: sel last / W: move(Right) / T: sel 0 | a:frankmd-core::emoji_picker | keep |
| 26 | 333 "moves selection left" | Left moves one cell back | §6.4 | G: sel 3 / W: move(Left) / T: sel 2 | a:frankmd-core::emoji_picker | keep |
| 27 | 343 "wraps to end at beginning" | Left at first wraps to last | §6.4 | G: sel 0 / W: move(Left) / T: sel last | a:frankmd-core::emoji_picker | keep |
| 28 | 354 "moves selection down one row" | Down moves by column count | §6.4 | G: Emoji sel 2 / W: move(Down) / T: sel 12 | a:frankmd-core::emoji_picker | keep |
| 29 | 365 "wraps to first row at last row" | Down from last row wraps to same column on first row | §6.4 | G: sel = first cell of last row / W: move(Down) / T: sel 0 | a:frankmd-core::emoji_picker | keep |
| 30 | 381 "moves selection up one row" | Up moves by column count | §6.4 | G: sel 12 / W: move(Up) / T: sel 2 | a:frankmd-core::emoji_picker | keep |
| 31 | 392 "wraps to last row at first row" | Up from first row wraps into the last row | §6.4 | G: sel 0, 25 items, 10 cols / W: move(Up) / T: sel 20 | a:frankmd-core::emoji_picker | keep |
| 32 | 407 "does not prevent default - allows normal tab navigation" | Tab in the grid moves focus, doesn't change selection or tab | §5 dialog Tab rule | G: grid focused, Emoji sel 3 / W: press tab / T: focus leaves grid (next focusable in popover); tab still Emoji; sel 3 | b | keep |
| 33 | 426 "switches from emoji to emoticons with ArrowRight" | Right on tab strip goes to next tab | §5.5 (Left/Right on tab strip) | G: tab strip focused, Emoji / W: `tab_strip_key(Right)` / T: Emoticons | a:frankmd-core::emoji_picker | keep |
| 34 | 436 "switches from emoticons to icons with ArrowRight" | Same | §5.5 | G: Emoticons / W: Right / T: Icons | a:frankmd-core::emoji_picker | keep |
| 35 | 446 "switches from icons to emoji with ArrowRight (wrap)" | Tab strip wraps forward | §5.5 | G: Icons / W: Right / T: Emoji | a:frankmd-core::emoji_picker | keep |
| 36 | 456 "switches from emoticons to emoji with ArrowLeft" | Left goes to previous tab | §5.5 | G: Emoticons / W: Left / T: Emoji | a:frankmd-core::emoji_picker | keep |
| 37 | 466 "switches from emoji to icons with ArrowLeft (wrap)" | Tab strip wraps backward | §5.5 | G: Emoji / W: Left / T: Icons | a:frankmd-core::emoji_picker | keep |
| 38 | 476 "switches from icons to emoticons with ArrowLeft" | Same | §5.5 | G: Icons / W: Left / T: Emoticons | a:frankmd-core::emoji_picker | keep |
| 39 | 486 "ignores other keys" | Other keys on the tab strip don't switch tabs | §5.5 | G: Emoji / W: tab_strip_key(Char('a')) / T: Emoji; returns NotHandled | a:frankmd-core::emoji_picker | keep |
| 40 | 497 "selects current item and dispatches event" | Enter inserts the selected item and closes | §5.5 | G: Emoji sel smile / W: `activate()` / T: Insert{":smile:", Emoji}; picker closed | a:frankmd-core::emoji_picker | keep |
| 41 | 513 "dispatches emoji with shortcode format" | Emojis insert as `:shortcode:` (file compat) | §5.5, §11 | G: editor "Hi " caret at end / W: activate on heart → `markdown::edit::insert_emoji` / T: text "Hi :heart:" caret after | a:frankmd-core::emoji_picker | keep |
| 42 | 528 "dispatches emoticon as raw text" | Emoticons insert raw | §5.5 | G: Emoticons sel (◕‿◕) / W: activate / T: Insert{"(◕‿◕)", Emoticons} | a:frankmd-core::emoji_picker | keep |
| 43 | 551 "updates emoji selection in place without scrolling or rebuilding the grid" | Hover changes the highlighted cell without jumping the scroll | §5.5 | G: Emoji sel 0 / W: `hover(5)` / T: sel 5; scroll_request None; footer updated to item 5 | a:frankmd-core::emoji_picker | keep |
| 44 | 572 "uses only a subtle selected background and preserves emoticon text color" | Selected cell uses hover bg + accent ring; emoticon text stays primary colour | §5.5, §7.1 | G: Emoticons tab / W: hover a cell / T: visual: ring accent, bg `bg-hover`, text `text.primary` (never solid accent) | c | keep |
| 45 | 596 "updates icon selection in place" | Same for icons | §5.5 | G: Icons sel 0 / W: hover(3) / T: sel 3; footer ":ph-<name>:"; scroll_request None | a:frankmd-core::emoji_picker | keep |
| 46 | 615 "does nothing for same index" | Re-hovering same cell is a no-op | §5.5 | G: sel 4 / W: hover(4) / T: returns false | a:frankmd-core::emoji_picker | keep |
| 47 | 629 "dispatches emoji selection on click" | Clicking an emoji inserts its shortcode | §5.5 | G: Emoji / W: `activate_at(i of smile)` (b: click cell) / T: Insert{":smile:", Emoji} | a:frankmd-core::emoji_picker | keep |
| 48 | 643 "dispatches emoticon selection on click" | Clicking an emoticon inserts it raw | §5.5 | G: Emoticons / W: activate_at((◕‿◕)) / T: Insert{"(◕‿◕)", Emoticons} | a:frankmd-core::emoji_picker | keep |
| 49 | 657 "dispatches icon selection on click" | Clicking an icon inserts `:ph-name:` | §5.5 | G: Icons / W: activate_at(heart) / T: Insert{":ph-heart:", Icons} | a:frankmd-core::emoji_picker | keep |
| 50 | 679 "renders emoji buttons with data attributes" | Each emoji cell carries glyph + shortcode (and accessible label) | §5.5, §9 | G: Emoji / W: cells() / T: cell {index, glyph, shortcode}; a11y label ":smile:" | a:frankmd-core::emoji_picker | keep |
| 51 | 689 "highlights selected item" | Selected cell is marked selected | §5.5 | G: sel 2 / T: cells()[2].selected true; others false | a:frankmd-core::emoji_picker | keep |
| 52 | 707 "renders emoticon buttons with data attributes" | Emoticon cell carries name + text | §5.5 | G: Emoticons / T: cell {name, text} | a:frankmd-core::emoji_picker | keep |
| 53 | 717 "uses different column count for emoticons" | Emoticon grid laid out 5 wide | §5.5 | G: Emoticons / T: `rows()` chunks of 5 | a:frankmd-core::emoji_picker | keep |
| 54 | 728 "renders icon buttons with an inline SVG" | Icon cells render the Phosphor glyph | §5.5 | G: Icons / T: cell carries IconGlyph {svg path, viewBox} for "heart" | a:frankmd-core::emoji_picker | keep |
| 55 | 738 "uses different column count for icons" | Icon grid laid out 8 wide | §5.5 | G: Icons / T: rows() chunks of 8 | a:frankmd-core::emoji_picker | keep |
| 56 | 744 "shows emoji preview with shortcode" | Footer shows large glyph + `:smile:` | §5.5 footer | G: Emoji sel smile / T: footer {Glyph("😄"), ":smile:"} | a:frankmd-core::emoji_picker | keep |
| 57 | 754 "shows emoticon preview with name" | Footer shows emoticon + name | §5.5 | G: Emoticons sel / T: footer {Text(emoticon), name} | a:frankmd-core::emoji_picker | keep |
| 58 | 764 "shows icon preview with shortcode and inline SVG" | Footer shows icon + `:ph-heart:` | §5.5 | G: Icons sel heart / T: footer {Svg(heart), ":ph-heart:"} | a:frankmd-core::emoji_picker | keep |
| 59 | 774 "clears preview when no items" | No match → empty footer | §5.5 | G: query matches nothing / T: footer None; grid shows "No matches" | a:frankmd-core::emoji_picker | keep |
| 60 | 784 "dispatches selected event with text" | Insert carries text + current tab kind | §5.5 | G: tab Emoticons / W: `select_text("xD")` / T: Insert{"xD", Emoticons} | a:frankmd-core::emoji_picker | keep |
| 61 | 794 "closes dialog after dispatch" | Picker closes after inserting; focus returns to editor | §5.5 | G: open / W: activate (b: enter) / T: closed; editor focused | a:frankmd-core::emoji_picker + b | keep |
| 62 | 802 "initializes i18n properties during connect" | Localized search ready for the UI locale from start | §5.5 (FE-405/502) | G: locale pt-BR / W: EmojiPicker::new / T: i18n index built from bundled `core::emoji::i18n` data (no fetch); is_some | a:frankmd-core::emoji_picker | changed (bundled data, no `/emoji-data` fetch) |
| 63 | 809 "has i18nLoaded property" | Picker knows whether localized data is active | §5.5 | G: new(pt-BR) / T: `i18n_loaded()` true; new(None/en) → true with English-only | a:frankmd-core::emoji_picker | keep |
| 64 | 816 "sets i18nLoaded to true after loading completes" | Switching locale live loads its data | §5.5, §5.1 Language live | G: new(en) / W: `set_locale(pt-BR)` / T: i18n_loaded true; `feliz` finds 😀 | a:frankmd-core::emoji_picker | keep |
| 65 | 828 "builds search index with emoji data" | Index keyed by emoji glyph | §5.5 | G: set_locale(pt-BR) / T: index has 😀 and 😂 | a:frankmd-core::emoji_picker | keep |
| 66 | 841 "results in a Map even if loading fails" | Missing locale data → English search still works | §5.5 | G: locale "xx" without dataset / W: set_locale / T: index empty (not None); `heart` still matches | a:frankmd-core::emoji_picker | keep |
| 67 | 855 "skips loading if already loaded" | Same locale isn't reloaded | §5.5 | G: pt-BR loaded / W: set_locale(pt-BR) / T: returns false; index pointer unchanged | a:frankmd-core::emoji_picker | keep |
| 68 | 925 "uses i18n search for emoji tab" | Localized terms work on the Emoji tab | §5.5 | G: pt-BR, Emoji tab / W: set_query("coração") / T: ❤️ in items | a:frankmd-core::emoji_picker | keep |
| 69 | 933 "does not use i18n search for emoticons tab" | Emoticons/Icons use English names/keywords only | §5.5 | G: pt-BR, Emoticons tab / W: set_query("feliz") / T: items empty (no translated match) | a:frankmd-core::emoji_picker | keep |

### `test/javascript/controllers/file_finder_controller.test.js` — C=29 (keep 25 / changed 4 / dropped 0; a=29 b=0 c=0)
Original subject: Ctrl+P file finder dialog (fuzzy list, keyboard/mouse selection, 10-line content preview). New home: Quick open overlay §5.2 (`quick-open`, `quick-open-input`); VM `workspace::quick_open` over `search::finder`. A rows (first-10 line 84, onInput filter ×4) not listed.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 55 "initializes empty state" | Fresh quick open has no rows and nothing pending | §5.2 | G: `QuickOpenModel::new()` / T: notes empty; results empty; sel 0; input_mode Pointer | a:frankmd-workspace::quick_open | keep |
| 2 | 70 "opens dialog with files" | Ctrl+P opens quick open over the vault's notes | §5.2, §6.1 | G: vault with 3 notes / W: press ctrl-p / T: `quick-open` visible; `quick-open-input` focused; VM notes = vault notes | a:frankmd-workspace::overlay + b | keep |
| 3 | 77 "clears input field" | Reopen starts with empty query | §5.2 | G: query "ban" / W: `open(notes)` / T: query "" | a:frankmd-workspace::quick_open | keep |
| 4 | 95 "resets selected index" | Reopen selects the first row | §5.2 | G: sel 4 / W: open / T: sel 0 | a:frankmd-workspace::quick_open | keep |
| 5 | 104 "closes the dialog" | Esc closes and restores focus | §5.2, §4.3 | G: open from editor / W: `close()` (b: escape) / T: closed; `editor` focused | a:frankmd-workspace::quick_open + b | keep |
| 6 | 154 "shows no files message when empty" | Empty result explains and offers creating the note | §5.2 | G: query "zzz" no match / T: empty_state = NoMatch{ hint "Ctrl+N creates zzz.md" }; Ctrl+N → NewNoteForm prefilled "zzz" | a:frankmd-workspace::quick_open | changed (§5.2 "No notes match" + Ctrl+N create) |
| 7 | 161 "renders file list with selection highlight" | One row per result; selected row marked | §5.2 rows | G: 3 results sel 0 / T: rows() len 3; rows()[0].selected | a:frankmd-workspace::quick_open | keep |
| 8 | 173 "shows directory path when file is in folder" | Nested notes show their folder path dimmed | §5.2 | G: docs/notes/a.md / T: row {name "a", folder "docs/notes", match ranges} | a:frankmd-workspace::quick_open | keep |
| 9 | 182 "does not allow a quoted path to inject attributes" | Odd filenames are shown literally | §5.2 | G: note `photo" onerror="alert(1).md` / T: row name == `photo" onerror="alert(1)` verbatim; activate opens that exact path | a:frankmd-workspace::quick_open | keep |
| 10 | 200 "moves selection down on ArrowDown" | Down moves to next row | §5.2, §6.4 | G: sel 0 of 3 / W: move_next / T: sel 1; Keyboard mode | a:frankmd-workspace::quick_open | keep |
| 11 | 209 "moves selection up on ArrowUp" | Up moves to previous row | §5.2 | G: sel 2 / W: move_prev / T: sel 1 | a:frankmd-workspace::quick_open | keep |
| 12 | 218 "does not go below 0" | Up at first row | §5 lists wrap | G: sel 0 of 3 / W: move_prev / T: sel 2 | a:frankmd-workspace::quick_open | changed (§5 lists unify on wrap) |
| 13 | 227 "does not exceed list length" | Down at last row | §5 lists wrap | G: sel 2 of 3 / W: move_next / T: sel 0 | a:frankmd-workspace::quick_open | changed (§5 lists unify on wrap) |
| 14 | 236 "calls selectCurrent on Enter" | Enter opens the selected note | §5.2 | G: sel blog/a.md / W: press enter / T: Command::OpenNote(blog/a.md); closed; `editor` focused | a:frankmd-workspace::quick_open + b | keep |
| 15 | 248 "updates selected index on hover" | Hover selects a row | §5.2 | G: Pointer mode / W: hover(2) / T: sel 2 | a:frankmd-workspace::quick_open | keep |
| 16 | 261 "does not update if same index" | Same row hover is a no-op | §5.2 | G: sel 1 / W: hover(1) / T: false (no notify) | a:frankmd-workspace::quick_open | keep |
| 17 | 271 "ignores hover when using keyboard navigation" | Stationary pointer doesn't override keyboard | §5.2 | G: move_next → Keyboard / W: hover(2) / T: sel 1 | a:frankmd-workspace::quick_open | keep |
| 18 | 295 "sets usingKeyboard flag on arrow key navigation" | Arrow keys enter keyboard mode | §5.2 | G: Pointer / W: move_next / T: Keyboard | a:frankmd-workspace::quick_open | keep |
| 19 | 305 "resets usingKeyboard flag on mouse move" | Mouse move returns to pointer mode | §5.2 | G: Keyboard / W: pointer_moved / T: Pointer | a:frankmd-workspace::quick_open | keep |
| 20 | 313 "allows hover selection after mouse move" | After moving the mouse, hover selects again | §5.2 | G: Keyboard sel 1 / W: pointer_moved; hover(2) / T: sel 2 | a:frankmd-workspace::quick_open | keep |
| 21 | 338 "dispatches selected event with path" | Clicking a row opens that note | §5.2 | G: rows / W: activate_at(1) (b: click row) / T: OpenNote(row 1 path) | a:frankmd-workspace::quick_open | keep |
| 22 | 351 "dispatches selected event for current selection" | Activate uses current selection | §5.2 | G: sel 2 / W: activate / T: OpenNote(row 2) | a:frankmd-workspace::quick_open | keep |
| 23 | 366 "does nothing when no results" | Enter with no results opens nothing | §5.2 | G: no results / W: activate / T: None; overlay stays open | a:frankmd-workspace::quick_open | keep |
| 24 | 377 "dispatches event and closes dialog" | Opening closes the overlay; Ctrl+Enter also reveals in explorer | §5.2 | G: sel / W: activate(Open); activate(OpenAndReveal) / T: closed; second also emits RevealInExplorer | a:frankmd-workspace::quick_open | keep |
| 25 | 391 "fetches file content for preview" | Selecting a row loads that note's text for the preview | §5.2 preview | G: sel blog/a.md / W: selection settles / T: Effect::ReadPreview(blog/a.md, gen) (store read, no HTTP) | a:frankmd-workspace::quick_open | keep |
| 26 | 403 "shows preview content" | Preview shows the note's first lines, marks truncation | §5.2 (12-line preview, ≥ 900 px) | G: content of 30 lines / W: apply_preview / T: preview = first 12 lines verbatim + truncated flag; b: at 899 px no preview column | a:frankmd-workspace::quick_open + b | changed (§5.2 12 lines, two-column only ≥ 900 px) |
| 27 | 412 "handles empty results" | No results → empty preview | §5.2 | G: no results / T: preview None; no ReadPreview effect | a:frankmd-workspace::quick_open | keep |
| 28 | 420 "shows error message on fetch failure" | Unreadable note shows a preview error, not stale text | §5.2 | G: ReadPreview fails / W: apply_preview_error(gen) / T: preview Error(i18n `dialogs.file_finder.preview_error`) | a:frankmd-workspace::quick_open | keep |
| 29 | 431 "localizes the preview error when fetching throws" | Preview error text follows the UI locale | §5.2, §11 i18n | G: locale pt-BR, read error / T: preview error string == pt-BR catalog value for `dialogs.file_finder.preview_error` | a:frankmd-workspace::quick_open | keep |

#### New VM requirements (G5)
- `frankmd-workspace::list_nav` (new) — `ListCursor {selected, len, input_mode: Keyboard/Pointer}`: `move_next/prev` with wrap, `hover(i) -> bool` (ignored in Keyboard mode, false on same index), `pointer_moved()`, `select(i)`, `is_selected(i)`, `scroll_request() -> Option<usize>` (set by keyboard moves only). Shared by content_search rows 17–31,38–40, quick_open rows 10–24.
- `frankmd-workspace::content_search` — `ContentSearchModel`: `open()` reset, `set_query` with 300 ms debounce on injected Clock, `Effect::Search{query, regex, case, scope, gen}`, `apply_results/apply_error(gen)` dropping stale gens, `searching()` flag (true from first keystroke until result/clear), `status()` enum {Hint, Count{matches,notes,truncated}, NoMatches, Error, InvalidPatternFallback}, matches grouped by note with header rows skipped by navigation, `activate(keep_open) -> Option<OpenAt{path,line}>` (rows 1–40).
- `frankmd-workspace::quick_open` — `open(notes)` reset, `rows()` {name, folder, match ranges, selected}, `empty_state()` NoMatch{query}, `activate(Open/OpenAndReveal/PreviewOnly)`, preview state {None, Loading, Lines(12, truncated), Error(key)} with `Effect::ReadPreview(path, gen)` (rows 1–29).
- `frankmd-workspace::explorer` — `DragState`: `begin_drag(row)` (selection-aware payload), `drag_hover(HoverTarget::{Row(path), Background, Outside}) -> Option<DropTarget::{Folder, Root}>` with note → its folder, `drop_cursor() -> Move/NotAllowed`, `is_dragging(path)`, `drag_badge_count()`, `root_highlighted()`, `end_drag()`, `drop() -> MovePlan`, `reset_tree(tree)` clears drag; `execute_moves(plan, store, autosave)` sequential, stop on first error, expands destination + ancestors before moving, emits `Event::ItemMoved{old,new,kind}`, expansion preserved across tree reload (drag_drop rows 1–37).
- `frankmd-workspace::autosave` — `prepare_for_transition() -> Result<(), DraftNotStored>` and `resume_after_transition()` for moves of the open note (drag_drop rows 35–36); not in current `autosave.rs`.
- `frankmd-workspace::status` — `StatusLine::error(msg)` used for move failures (drag_drop row 34).
- `frankmd-workspace::insert_image` — Upload source: `ingest(files)` (clears selection, emits `Deselected` only if something was selected), `ingest_external(file)` (auto-select, rejects non-images with inline error), `dropzone_hover: bool`, `Effect::OpenFileChooser` (drop_images rows 1–8).
- `frankmd-theme::custom` — `CustomThemeEditor`: `set_color(key, raw)` with per-key `InvalidColor`, `save_enabled()`, `contrast_warning() -> Option<ratio>` (min of text-primary/bg-primary and accent-text/accent, < 4.5, formatted 2 dp), `reset()`, `set_base(id)`, `preview_tokens()`, `cancel()` restores prior theme id, `save()` → one atomic config write {theme, custom_theme} with `SaveFailed` status kept editable (custom_theme_editor rows 1–3).
- `frankmd-core::emoji_picker` — `EmojiPicker`: tabs + `switch_tab -> bool`, `tab_strip_key(dir)`, `columns()` 10/5/8, `move(dir)` grid wrap, `hover -> bool` without scroll request, `cells()` {glyph/text/IconGlyph, shortcode/name, selected}, `rows()`, `footer()`, `activate[_at]() -> Insert{text, kind}`, Frequently used row from state dir, `set_locale(loc) -> bool` using bundled `core::emoji::i18n` datasets (empty index on missing data) (emoji rows 1–69).
- `frankmd-markdown::edit` — `insert_emoji(text, sel, insert)` (emoji row 41).
- Element ids (add to vocab): `emoji-picker`, `emoji-search`, `emoji-grid`, `emoji-tab-<kind>`, `dialog-custom-theme`, `dialog-insert-image`, `upload-dropzone`, `search-result-<n>`, `quick-open-row-<n>`.
- frankmd-testkit: FakePortal FileChooser and a fake clipboard image for b tests (drop_images rows 6, 8); simulated external-file drag (`ExternalPaths`) in `TestAppContext` (rows 3–5).

#### Open questions (G5)
- Reopening search-in-files / quick open: FrankMD clears query and results; Zed/VS Code keep the last query. ui-spec §5.2/§5.3 is silent; plan assumes clear (content_search 3–4, file_finder 3).
- Search in files "Searching…" text: §5.3 only mentions the bar. Is any text shown in the status row while debouncing (row 8)?
- Emoji picker: does open always reset to the Emoji tab, and does the new Frequently used row take part in grid navigation/wrap and column count (rows 7, 9, 25–31)?
- Emoji picker hover had no keyboard-mode guard in FrankMD while the lists do; should the grid share `ListCursor` input-mode semantics?
- Tab inside the emoji popover: §5.5 says Ctrl+Tab cycles tabs, but plain Tab focus order (search → tab strip → grid) isn't specified (row 32).
- Explorer drag: dragging a note out of the window (old text/plain payload) or into the editor (insert link?) is unspecified (row 6).
- Hovering a note at the vault root while dragging a nested item: plan assumes target = Root (row 17); confirm.
- Move failure in a multi-item drop: status-line error only, or a banner listing what was and wasn't moved (rows 28, 34)?
- Custom theme save failure: inline dialog status ("Save failed") vs status line message; §5.17 doesn't say (row 3).
- Quick open below 900 px has no preview column: should the preview read still happen (row 26)?

## 11. G6: file_operations, find_replace, folder_images, help, home, image_picker, image_viewer, library, locale, local_images, log_viewer

### `test/javascript/controllers/file_operations_controller.test.js` — C=82 (keep 69 / changed 13 / dropped 0; a=64 b=17 c=1)
Original subject: explorer context menu, New note / New folder / Rename dialogs, delete confirm and multi-delete (file_operations_controller). New home: explorer context menu + inline rename (§5.14, §5.13), New note / New folder / Move / Delete dialogs (§5.13), pane-header title rename (§3.4), status line errors (§3.6). Rows counted: the 13 [A] preview/Hugo-path tests (258 it.each, 273, 284, 334, 383, 392, 405, 413, 423, 433, 443, 908, 913) are excluded; 749 it.each = one row.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 94 "initializes context item to null" | No row is targeted by the context menu at startup | Explorer §5.14 | G: fresh ExplorerState / W: — / T: context_target() is None, no menu open | a:frankmd-workspace::explorer | keep |
| 2 | 98 "initializes new item type to null" | No create dialog is pending at startup | Dialogs §5.13 | G: fresh UiState / W: — / T: OverlayStack empty; no NewNoteForm/NewFolderForm exists | a:frankmd-workspace::overlay | keep |
| 3 | 102 "initializes new item parent to empty string" | Default destination is the vault root | New note §5.13 | G: NewNoteForm::new(None) / W: — / T: destination == "" (shown as root label) | a:frankmd-workspace::dialogs | keep |
| 4 | 106 "initializes the new note template to empty document" | New note defaults to Empty note | New note §5.13 | G: NewNoteForm::new(None) / W: — / T: template == Template::Empty | a:frankmd-workspace::dialogs | keep |
| 5 | 112 "shows context menu at click position" | Right-click opens the menu where the pointer is | Explorer context menu §5.14 | G: tree with test.md / W: right-click explorer-row-test.md at (100,200) / T: context menu visible, anchored at pointer position | b | keep |
| 6 | 130 "stores context item" | Menu actions target the right-clicked row | Explorer §5.14 | G: tree with folder/test.md / W: open_context_menu(row folder/test.md) / T: context_target == {folder/test.md, Note} | a:frankmd-workspace::explorer | keep |
| 7 | 146 "preserves a multi-selection when right-clicking a selected row" | Right-click inside a selection acts on the whole selection | Explorer §5.14 | G: selection {first.md, docs} / W: context menu on docs / T: selection unchanged; menu targets both items | a:frankmd-workspace::explorer | keep |
| 8 | 172 "selects only the right-clicked row when it is outside the selection" | Right-click outside the selection reselects that row | Explorer §5.14 | G: selection {first.md} / W: context menu on second.md / T: selection == {second.md}; menu targets [second.md] | a:frankmd-workspace::explorer | keep |
| 9 | 194 "does not show for config files" | Protected config rows can't be renamed/deleted from a menu | Explorer hidden items §5.14 | G: vault with .fed / W: build tree; (if a config row is ever shown) open its menu / T: .fed not in visible rows; config row menu has no Rename/Move/Delete | a:frankmd-workspace::explorer | changed (§5.14 dotfiles hidden) |
| 10 | 212 "hides the context menu" | Menu can be dismissed | Explorer context menu §5.14, Esc chain §6.1 | G: context menu open / W: press Esc (and separately click elsewhere) / T: menu gone; focus back on the row | b | keep |
| 11 | 222 "opens the new note dialog directly" | New note opens the dialog with no intermediate menu | New note §5.13 | G: vault open / W: press ctrl-n / T: dialog-new_note visible and name input focused | b | keep |
| 12 | 228 "uses the current selected folder for notes and folders" | Create targets the single selected folder | New note / New folder §5.13, §5.14 | G: selection {docs/nested (folder)} / W: NewNote then submit "sample"; NewFolder then submit "child" / T: create requests docs/nested/sample.md and folder docs/nested/child | a:frankmd-workspace::dialogs | keep |
| 13 | 295 "clears a stale parent from a prior folder-scoped new note" | A previous folder-scoped create doesn't leak into the next one | New note §5.13 | G: NewNote opened in folder docs then cancelled; selection cleared / W: NewNote again / T: destination == root | a:frankmd-workspace::dialogs | keep |
| 14 | 304 "resets the input and selects the empty template" | Each open starts clean | New note §5.13 | G: form closed with name "x", template Hugo / W: reopen / T: name == "", template Empty (segmented control shows Empty) | a:frankmd-workspace::dialogs | keep |
| 15 | 315 "disables create while the name is empty" | Can't create an unnamed note | New note §5.13 | G: NewNoteForm name "" / W: — / T: can_submit() == false; Create button disabled | a:frankmd-workspace::dialogs | keep |
| 16 | 323 "hides context menu and opens the new note dialog with the folder as parent" | Folder menu "New note" creates inside that folder | Explorer context menu §5.14, New note §5.13 | G: context menu on folder parent / W: choose New note / T: menu closed; dialog-new_note open with "In: parent/" | a:frankmd-workspace::explorer + b | keep |
| 17 | 343 "does nothing if context item is not a folder" | Note rows don't offer folder-scoped create | Explorer §5.14 | G: context target note.md / W: context_menu_items() ; dispatch NewNoteInFolder / T: item absent; no dialog opened | a:frankmd-workspace::explorer | keep |
| 18 | 351 "does nothing if no context item" | No target, no action | Explorer §5.14 | G: no context target / W: NewNoteInFolder / T: OverlayStack unchanged | a:frankmd-workspace::explorer | keep |
| 19 | 361 "switches the selection to the hugo card" | Template choice is a single-select | New note §5.13 | G: form / W: select_template(Hugo) / T: template == Hugo; slug field shown | a:frankmd-workspace::dialogs | keep |
| 20 | 371 "styles the selected card with the accent border" | Selected template is visually distinct | New note segmented control §5.13, §7.1 | G: dialog open / W: choose Hugo post / T: visual check: selected segment uses accent styling, other plain | c | keep |
| 21 | 454 "closes the new note dialog" | Cancel/Esc closes the dialog | Dialog anatomy §5, §6.4 | G: dialog-new_note open / W: press escape / T: dialog gone; focus restored to prior region | b | keep |
| 22 | 461 "resets state" | Closing discards pending create state | New note §5.13 | G: form with destination docs, template Hugo / W: cancel / T: no pending form; next open has root + Empty | a:frankmd-workspace::dialogs | keep |
| 23 | 474 "does nothing with empty input" | Empty name creates nothing | New note §5.13 | G: name "" / W: submit() / T: returns None; no vault op | a:frankmd-workspace::dialogs | keep |
| 24 | 483 "does nothing with an invalid name (slashes)" | Invalid names create nothing | New note §5.13 | G: name "a/b" / W: submit() / T: None; path preview in danger | a:frankmd-workspace::dialogs | keep |
| 25 | 492 "creates an empty-document note via API" | Empty note created at root/name.md | New note §5.13 | G: name "test" / W: submit() / T: CreateRequest::Empty{path:"test.md", content:""} | a:frankmd-workspace::dialogs | keep |
| 26 | 503 "creates a hugo post via the template API" | Hugo post uses the template under blog | New note §5.13, Settings › Hugo | G: template Hugo, title "My Post" / W: submit() / T: CreateRequest::Hugo{title, parent:"blog", slug} | a:frankmd-workspace::dialogs | keep |
| 27 | 519 "passes the parent folder for folder-scoped notes" | Folder-scoped Hugo create keeps the folder | New note §5.13 | G: dialog opened for folder docs, Hugo / W: submit() / T: parent == "docs" | a:frankmd-workspace::dialogs | keep |
| 28 | 531 "dispatches file-created event" | Created note appears and opens | Explorer / navigation | G: create succeeds / W: apply result / T: store emits FileCreated(path); tree contains it; note opened | a:frankmd-workspace::navigation | keep |
| 29 | 543 "closes the dialog after successful creation" | Success closes the dialog | New note §5.13 | G: dialog open / W: create Ok / T: overlay popped; editor focused | a:frankmd-workspace::overlay | keep |
| 30 | 552 "expands the destination folder before posting so the tree reveals the new note (#268)" | New note is visible in the tree | Explorer reveal §5.14 | G: expansion {unrelated} / W: create docs/nested/n.md / T: expansion == {unrelated, docs, docs/nested}; row revealed | a:frankmd-workspace::explorer | keep |
| 31 | 567 "expands the destination folder when creating a nested folder (#268)" | New folder is visible in the tree | Explorer reveal §5.14 | G: expansion {} / W: create folder docs/deep/sub / T: docs and docs/deep expanded | a:frankmd-workspace::explorer | keep |
| 32 | 582 "keeps the dialog open and shows failures inline" | Create errors don't lose the form | New note §5.13 | G: create returns Err("exists") / W: submit / T: dialog still open; form.error == "exists"; name kept | a:frankmd-workspace::dialogs | keep |
| 33 | 601 "opens new item dialog for folder" | New folder opens its dialog at root | New folder §5.13 | G: no selection / W: press ctrl-shift-n / T: dialog-new_folder visible with root location | b | keep |
| 34 | 611 "shows the parent path when creating inside a folder" | User sees where the folder goes | New folder §5.13 | G: NewFolderForm::new("docs/a") / W: — / T: subtitle "In: docs/a" | a:frankmd-workspace::dialogs | keep |
| 35 | 618 "shows the root location label when creating at the root" | Root destination is named | New folder §5.13 | G: NewFolderForm::new("") / W: — / T: subtitle uses t(dialogs.new_item.root_location) | a:frankmd-workspace::dialogs | keep |
| 36 | 626 "hides context menu and opens new item dialog for folder" | Folder menu "New folder" creates inside it | Explorer §5.14 | G: context menu on parent/myfolder / W: choose New folder / T: menu closed; NewFolderForm parent parent/myfolder | a:frankmd-workspace::explorer + b | keep |
| 37 | 637 "does nothing if context item is not a folder" | Note rows don't offer New folder here | Explorer §5.14 | G: context target note / W: NewFolderInFolder / T: no dialog | a:frankmd-workspace::explorer | keep |
| 38 | 646 "does nothing if no context item" | No target, no action | Explorer §5.14 | G: no target / W: NewFolderInFolder / T: OverlayStack unchanged | a:frankmd-workspace::explorer | keep |
| 39 | 657 "shows the new item dialog" | Dialog visible | New folder §5.13 | G: — / W: dispatch NewFolder / T: dialog-new_folder visible, input focused | b | keep |
| 40 | 663 "sets new item type" | Dialog knows it creates a folder | New folder §5.13 | G: — / W: open folder dialog / T: OverlayKind::Dialog(NewFolder) on top | a:frankmd-workspace::overlay | keep |
| 41 | 670 "always shows the folder title" | Title says New folder | New folder §5.13 | G: NewFolderForm / W: — / T: title key dialogs.new_item.new_folder | a:frankmd-workspace::dialogs | keep |
| 42 | 678 "closes the new item dialog" | Esc closes | Dialog anatomy §5 | G: dialog-new_folder open / W: escape / T: gone; focus restored | b | keep |
| 43 | 685 "resets state" | Closing discards state | New folder §5.13 | G: form with parent docs / W: cancel / T: no pending form; next open root | a:frankmd-workspace::dialogs | keep |
| 44 | 697 "does nothing with empty input" | Empty folder name creates nothing | New folder §5.13 | G: name "" / W: submit / T: None; Create disabled | a:frankmd-workspace::dialogs | keep |
| 45 | 706 "creates folder via API" | Folder created at path | New folder §5.13 | G: name "newfolder" / W: submit / T: CreateFolder{path:"newfolder"} | a:frankmd-workspace::dialogs | keep |
| 46 | 718 "dispatches folder-created event" | Folder appears in tree | Explorer §5.14 | G: create Ok / W: apply / T: FolderCreated emitted; row present and revealed | a:frankmd-workspace::explorer | keep |
| 47 | 730 "keeps folder creation failures inline in the open dialog" | Errors don't close the dialog | New folder §5.13 | G: Err("exists") / W: submit / T: dialog open; inline error text | a:frankmd-workspace::dialogs | keep |
| 48 | 749 "opens the existing rename dialog for an Explorer %s" (it.each file/folder) | Rename starts with the current name | Inline rename §5.13, §5.14 | G: rows folder/test.md and parent/test-folder / W: F2 on each / T: inline edit value "test" / "test-folder"; target stored | a:frankmd-workspace::explorer | changed (§5.13 inline rename replaces dialog) |
| 49 | 760 "hides context menu" | Choosing Rename closes the menu | Explorer context menu §5.14 | G: context menu on row / W: choose Rename / T: menu closed; row shows inline input | b | changed (§5.13 inline rename) |
| 50 | 767 "shows rename dialog" | Rename UI appears | Inline rename §5.14 | G: row focused / W: press f2 / T: inline input in explorer-row-<path> focused (no dialog) | b | changed (§5.13 inline rename) |
| 51 | 774 "populates input with file name without extension" | .md is hidden while renaming | Inline rename §5.14 | G: note folder/test.md / W: begin_rename / T: edit text "test", fully selected | a:frankmd-workspace::explorer | changed (§5.13 inline rename) |
| 52 | 781 "populates input with folder name" | Folder rename shows basename | Inline rename §5.14 | G: folder parent/test-folder / W: begin_rename / T: edit text "test-folder" | a:frankmd-workspace::explorer | changed (§5.13 inline rename) |
| 53 | 790 "closes the rename dialog" | Esc cancels rename | Esc chain §6.1 | G: inline rename active / W: escape / T: input gone; name unchanged; row focused | b | changed (§5.13 inline rename) |
| 54 | 800 "does nothing with empty input" | Blank rename sends nothing | Inline rename §5.14 | G: edit text "" / W: commit / T: no vault op; inline error t(errors.title_required) | a:frankmd-workspace::explorer | keep |
| 55 | 809 "adds .md extension for files" | User types name, file keeps .md | Inline rename §5.14 | G: rename old.md / W: commit "newname" / T: RenameRequest new_path "newname.md" | a:frankmd-workspace::explorer | keep |
| 56 | 823 "dispatches file-renamed event" | Open note, tabs and history follow the rename | Navigation | G: old.md open / W: rename Ok to new.md / T: FileRenamed{old.md,new.md}; open note path, MRU and selection remapped | a:frankmd-workspace::navigation | keep |
| 57 | 838 "closes dialog after successful rename" | Success leaves edit mode | Inline rename §5.14 | G: rename active / W: Ok / T: edit mode off; row selected at new path | a:frankmd-workspace::explorer | changed (§5.13 inline rename) |
| 58 | 847 "keeps rename failures inline in the open dialog" | Errors keep the input and show why | Inline rename §5.14, §3.4 | G: Err("exists") / W: commit / T: still editing; inline error text; typed name kept | a:frankmd-workspace::explorer | changed (§5.13 inline rename) |
| 59 | 865 "flushes the active draft before sending a rename request" | No edits lost when renaming the open note | Autosave / navigation | G: open note dirty / W: rename it / T: prepare_for_transition called before RenameRequest | a:frankmd-workspace::navigation | keep |
| 60 | 880 "does not rename an active file when its draft cannot be persisted" | Rename aborts rather than risk data | Navigation, banners §5.12 | G: draft persistence fails / W: rename open note / T: no RenameRequest; status error; inline edit kept | a:frankmd-workspace::navigation | keep |
| 61 | 897 "renames a captured item without changing the Explorer dialog context" | Title rename in the pane header renames the open note only | Pane header title §3.4 | G: open nested/old.md, explorer context target other.md / W: rename_path(item, "renamed") / T: Ok(nested/renamed.md); explorer context unchanged | a:frankmd-workspace::navigation | keep |
| 62 | 918 "returns request failures and resumes the active draft" | Failed title rename keeps editing alive | Pane header §3.4 | G: rename Err / W: rename_path / T: Err(msg) shown as header error banner row; resume_after_transition called | a:frankmd-workspace::navigation | keep |
| 63 | 929 "preserves folder names without appending a Markdown extension" | Folders never get .md | Rename §5.13 | G: folder parent/old / W: rename_path(.., "new") / T: folder rename to parent/new | a:frankmd-workspace::explorer | keep |
| 64 | 936 "hides context menu" | Choosing Delete closes the menu | Explorer §5.14 | G: menu open on note / W: choose Delete / T: menu closed; dialog-delete_confirm open | b | keep |
| 65 | 943 "shows confirmation dialog" | Delete always asks, destructive styled | Delete §5.13 | G: note "Project brief.md" / W: DeleteNote / T: DeleteConfirm text "Move ‘Project brief’ to the trash?", destructive primary, focus on Cancel | a:frankmd-workspace::dialogs | changed (§5.13 trash wording, focus on Cancel) |
| 66 | 953 "does not delete if confirmation cancelled" | Cancel deletes nothing | Delete §5.13 | G: confirm open / W: cancel / T: no DeleteRequest | a:frankmd-workspace::dialogs | keep |
| 67 | 961 "deletes multiple selected rows after one combined confirmation" | One confirm for a multi-selection, deletes in order | Delete §5.13, §5.14 | G: selection [first.md, docs] / W: Delete; confirm / T: one DeleteConfirm with counts; requests first.md then folder docs | a:frankmd-workspace::dialogs | keep |
| 68 | 983 "skips descendants covered by a selected folder" | Children of a selected folder aren't deleted twice | Explorer multi-select §5.14 | G: selection [docs, docs/a.md] / W: Delete / T: single-folder confirm text; only docs requested | a:frankmd-workspace::explorer | keep |
| 69 | 1000 "does not delete protected config files from a selected set" | .fed never deleted | Explorer §5.14 | G: selection [a.md, .fed(config)] / W: Delete; confirm / T: only a.md requested | a:frankmd-workspace::explorer | keep |
| 70 | 1013 "stops after the first failed request and leaves later rows for retry" | Partial failure stops and reports | Delete §5.13, status line §3.6 | G: 3 items, 2nd fails / W: confirm / T: 3rd not requested; status-line error "delete failed" (no alert); remaining rows still selected | a:frankmd-workspace::dialogs | changed (§3.6 status message replaces appAlert) |
| 71 | 1039 "deletes a selected folder through the same confirmation and folder endpoint" | Folder delete uses folder wording | Delete §5.13 | G: folder docs / W: Delete; confirm / T: folder confirm text; DeleteFolder{docs} | a:frankmd-workspace::dialogs | keep |
| 72 | 1052 "deletes the item selected before confirmation even if context changes while waiting" | Confirm applies to what the user saw | Delete §5.13 | G: confirm open for a.md / W: selection changes to b.md; confirm / T: a.md deleted, not b.md | a:frankmd-workspace::dialogs | keep |
| 73 | 1074 "calls delete API" | Confirmed delete happens | Delete §5.13 | G: confirm for note.md / W: confirm / T: DeleteNote{note.md} issued | a:frankmd-workspace::dialogs | keep |
| 74 | 1085 "dispatches file-deleted event" | Tree and open note update after delete | Navigation, empty state §5.2 | G: note.md open / W: delete Ok / T: FileDeleted(note.md); row gone; editor shows empty state | a:frankmd-workspace::navigation | keep |
| 75 | 1097 "persists the active draft before sending a delete request" | Draft flushed before delete | Navigation | G: open note dirty / W: delete it / T: prepare_for_transition before DeleteRequest | a:frankmd-workspace::navigation | keep |
| 76 | 1111 "does not delete an active file when its draft cannot be persisted" | Delete aborts safely | Navigation | G: draft persist fails / W: confirm delete / T: no DeleteRequest; status error | a:frankmd-workspace::navigation | keep |
| 77 | 1127 "submits on Enter" (rename) | Enter commits inline rename | Inline rename §5.14 | G: inline rename "x" / W: press enter / T: rename committed | b | changed (§5.13 inline rename) |
| 78 | 1137 "closes on Escape" (rename) | Esc cancels inline rename | Esc chain §6.1 | G: inline rename / W: escape / T: cancelled; Esc doesn't propagate to next layer | b | changed (§5.13 inline rename) |
| 79 | 1148 "submits on Enter" (new item) | Enter creates folder | New folder §5.13, §5 dialog anatomy | G: dialog-new_folder name "x" / W: enter (and ctrl-enter) / T: create submitted | b | keep |
| 80 | 1158 "closes on Escape" (new item) | Esc closes folder dialog | §6.4 | G: dialog open / W: escape / T: closed | b | keep |
| 81 | 1169 "submits on Enter" (new note) | Enter creates note | New note §5.13 | G: dialog-new_note name "x" / W: enter (and ctrl-enter) / T: create submitted | b | keep |
| 82 | 1179 "closes on Escape" (new note) | Esc closes note dialog | §6.4 | G: dialog open / W: escape / T: closed, focus restored | b | keep |

### `test/javascript/controllers/find_replace_controller.test.js` — C=5 (keep 4 / changed 1 / dropped 0; a=4 b=1 c=0)
Original subject: find/replace dialog state, jump/feedback events (find_replace_controller). New home: inline find bar under the pane header (§5.4, §6.2); matching itself in `frankmd-search::find` (A tests at 88/113/129 excluded).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 74 "initializes state on connect" | Bar starts closed with no matches, find-only | Find bar §5.4 | G: fresh FindBarState / W: — / T: closed; matches empty; current None; replace row hidden | a:frankmd-workspace::find_bar | keep |
| 2 | 80 "opens dialog in replace tab" | Replace mode opens with the replace row | Find bar §5.4 | G: note open / W: press ctrl-h / T: find-bar and replace-input visible; find-input focused, prefilled with selection | b | changed (§5.4 inline bar replaces dialog tabs) |
| 3 | 99 "dispatches jump on find next" | Next match is the closest at/after the caret and gets selected; bad regex reports inline | Find bar §5.4 | G: "one one", caret 0, query "one" / W: find_next / T: editor selection 0..3; counter "1 of 2"; query "(" regex → counter replaced by danger error | a:frankmd-workspace::find_bar | keep |
| 4 | 144 "shows visual feedback after replace all" | Replace all reports how many | Find bar §5.4 | G: "a a a", find a, replace b / W: replace_all / T: text "b b b"; status Replaced(3) in success role | a:frankmd-workspace::find_bar | keep |
| 5 | 159 "clears replacement feedback after timeout" | Feedback is transient (2 s) | Find bar §5.4 | G: Replaced(3) at t0 (injected Clock) / W: advance 1999 ms then 1 ms / T: still shown, then counter restored | a:frankmd-workspace::find_bar | keep |

### `test/javascript/controllers/folder_images_controller.test.js` — C=2 (keep 2 / changed 0 / dropped 0; a=2 b=0 c=0)
Original subject: browser directory picker source in the image picker. New home: Insert image › Folder source, "Choose folder…" via portal (§5.6).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 42 "shows a localized folder access error inline without closing the picker" | Folder chooser failure is explained in place | Insert image Folder §5.6 | G: Folder source, chooser returns Err(denied) (fake portal) / W: choose_folder / T: source.error == t(dialogs.image_picker.folder_access_failed); dialog still open | a:frankmd-workspace::insert_image | keep |
| 2 | 53 "shows a localized folder read error inline" | Unreadable folder is explained in place | Insert image Folder §5.6 | G: chosen folder whose listing fails / W: load results / T: error t(dialogs.image_picker.folder_read_failed); dialog open; grid empty | a:frankmd-workspace::insert_image | keep |

### `test/javascript/controllers/help_controller.test.js` — C=5 (keep 4 / changed 1 / dropped 0; a=0 b=5 c=0)
Original subject: help dialog open/close/Esc/backdrop. New home: Help modal (§5.18), F1 toggle (§11), backdrop rule (§5 intro).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 44 "shows the help dialog" | Help opens | Help §5.18 | G: editor focused / W: press f1 / T: help-dialog visible, Markdown tab selected | b | keep |
| 2 | 52 "closes the help dialog" | Help closes (F1 now toggles) | Help §5.18, §11 | G: help open / W: press f1 / T: help-dialog gone; focus back on editor | b | changed (§11 F1 toggles) |
| 3 | 61 "closes help dialog on Escape" | Esc closes help | Esc chain §6.1 | G: help open / W: escape / T: closed; focus restored | b | keep |
| 4 | 70 "does nothing for other keys" | Other keys don't dismiss help | Help §5.18 | G: help open / W: press enter / T: still visible | b | keep |
| 5 | 83 "closes dialog when clicking on backdrop" | Backdrop click closes help | Overlay rules §5 | G: help open / W: click scrim outside help-dialog / T: closed | b | keep |

### `test/javascript/controllers/home_controller.test.js` — C=7 (keep 3 / changed 4 / dropped 0; a=7 b=0 c=0)
Original subject: Home page recent files (record, refresh, clear, show more/less). New home: empty state with docked quick open, recents first (§5.2); status line messages (§3.6).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 52 "records successful opens in order before refreshing server-rendered history" | Opened notes land in shared recents in open order | Quick open / empty state §5.2 | G: recents [] / W: open docs/one.md then two.md; return to empty state / T: recents store written in order; list rows [two.md, docs/one.md] then other notes by mtime | a:frankmd-workspace::quick_open | keep |
| 2 | 64 "does not erase the displayed list on a network failure and accepts later opens" | A recents read failure keeps the old list | Empty state §5.2, status §3.6 | G: list shows recents; next recents read fails / W: refresh / T: rows unchanged; status-line error "History unavailable"; later open still recorded | a:frankmd-workspace::quick_open | changed (§3.6 status message) |
| 3 | 73 "ignores an older response after a newer refresh" | Stale recents loads never overwrite newer ones | Quick open §5.2 | G: refresh gen 1 pending, gen 2 completes / W: gen 1 completes later / T: list stays gen 2 | a:frankmd-workspace::quick_open | keep |
| 4 | 84 "opens through guarded app navigation and preserves modified links" | Opening a recent goes through the guarded open-note flow; modified open variants differ | Quick open keys §5.2 | G: empty state, row note.md / W: enter; then ctrl-enter / T: open_note(note.md) via prepare_for_transition; ctrl-enter opens and reveals in explorer | a:frankmd-workspace::navigation + b | changed (§5.2 Ctrl/Alt+Enter replace modified-click) |
| 5 | 92 "clears shared history through the server" | Clear history clears for all devices after confirm and reloads list | Empty state recents header §5.2 | G: recents [a,b] / W: "Clear history…" then confirm / T: confirm text says "for every device using this folder"; store cleared; list shows mtime order only | a:frankmd-workspace::quick_open | keep |
| 6 | 98 "reveals the remaining notes and moves focus to the first revealed link" | All recents reachable from the keyboard | Quick open list §5.2, §5 lists | G: 7 recents / W: arrow down 6 times / T: all 7 rows present (no Show more), row 6 selected and scrolled into view | a:frankmd-workspace::quick_open + b | changed (§5.2 list top 50 replaces show more/less) |
| 7 | 130 "respects reduced motion when scrolling to the expanded list" | Reduced motion makes list scrolling instant | Motion §7.6 | G: reduce_motion = true / W: selection moves off-screen / T: scroll animation duration 0 (120 ms when off) | a:frankmd-workspace::motion | changed (§7.6 setting replaces matchMedia) |

### `test/javascript/controllers/image_picker_controller.test.js` — C=7 (keep 6 / changed 1 / dropped 0; a=6 b=1 c=0)
Original subject: image picker shell (paste ingest, tabs, insert, storage readiness). New home: Insert image dialog (§5.6), media storage row.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 59 "opens the picker and delegates the file to the Drop source with loading toggled" | Pasted image opens the dialog on Upload, with a processing state, pre-selected | Insert image Upload §5.6 | G: editor; clipboard image pasted.png / W: paste / T: dialog-insert_image open; source Upload; status processing_image_paste during ingest then cleared; pasted.png selected | a:frankmd-workspace::insert_image + b | keep |
| 2 | 75 "moves keyboard focus and selected state to the Library tab" | Sources are keyboard-navigable | Source rail §5.6, §6.4 | G: dialog on Upload, rail focused / W: press down (and separately ctrl-2) / T: source Library active, focus on its rail item, Library panel shown | b | changed (§5.6 vertical rail Up/Down + Ctrl+1…7) |
| 3 | 84 "bails without loading when the Drop source is unavailable" | Paste with no ingest path just opens the dialog | Insert image §5.6 | G: Upload source unavailable / W: open_with_file / T: dialog open; no processing status | a:frankmd-workspace::insert_image | keep |
| 4 | 94 "hides the loading overlay even when ingest throws" | Processing state never sticks | Insert image §5.6 | G: ingest returns Err("boom") / W: open_with_file / T: status not Processing; inline error shown | a:frankmd-workspace::insert_image | keep |
| 5 | 104 "keeps insert failures inline in the open image picker" | Insert failure stays in the dialog and resyncs storage | Footer errors §5.6 | G: Library source, S3 provider, upload Err / W: insert / T: processing_image shown then error t(insert_failed) above footer; dialog open; storage refresh called once | a:frankmd-workspace::insert_image | keep |
| 6 | 124 "includes the uploaded image URL and Markdown options in its selection event" | Insert writes ![alt](url) at the caret | Insert image footer §5.6 | G: selected photo.png, url images/photo.png, alt "photo", link "" / W: insert / T: InsertResult{markdown:"![photo](images/photo.png)", url, alt, link:""}; editor text contains it | a:frankmd-workspace::insert_image | keep |
| 7 | 147 "waits for a provider save before uploading a pasted or selected image" | Upload uses the provider the user just picked | Storage row §5.6 | G: provider change pending / W: insert / T: upload not started until provider save acks; then exactly one upload | a:frankmd-workspace::insert_image | keep |

### `test/javascript/controllers/image_viewer_controller.test.js` — C=7 (keep 7 / changed 0 / dropped 0; a=7 b=0 c=0)
Original subject: image lightbox (load, focus, stale events, unsafe sources). New home: Image viewer overlay (§5.20), preview images (§5.21). it.each ×3 = 3 rows.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 36 "loads a fresh image, focuses Close, and restores the trigger" | Viewer shows loading then image; closing returns focus to the image | Image viewer §5.20 | G: preview image focused / W: enter; image loads; escape / T: image-viewer visible, status Loading then Loaded with caption = alt; on close focus back on that preview image | a:frankmd-workspace::image_viewer + b | keep |
| 2 | 50 "falls back to the preview after a rerender removes the trigger" | Focus doesn't get lost if the image vanished | Image viewer §5.20, focus §4.3 | G: viewer opened from image #2; preview re-renders without it / W: close / T: focus on preview pane | a:frankmd-workspace::image_viewer | keep |
| 3 | 57 "ignores an old resource event when a newer image is opened" | Only the current image's load result shows; errors keep viewer open | Image viewer §5.20 | G: open first.png (gen 1) then next.png (gen 2) / W: gen 1 load fails, then gen 2 fails / T: status stays Loading after gen 1; Unavailable after gen 2; viewer open | a:frankmd-workspace::image_viewer | keep |
| 4 | 68 "rejects unsafe image source %s" [row 1] javascript:alert(1) | Script URLs never open | Image viewer §5.20 | G: — / W: open("javascript:alert(1)") / T: rejected; no overlay | a:frankmd-workspace::image_viewer | keep |
| 5 | 68 "rejects unsafe image source %s" [row 2] file:///secret.png | Arbitrary filesystem URLs never open | Image viewer §5.20 | G: vault ~/notes / W: open("file:///secret.png") / T: rejected (only vault-relative paths and http(s)) | a:frankmd-workspace::image_viewer | keep |
| 6 | 68 "rejects unsafe image source %s" [row 3] data:text/html | Non-image data URLs never open | Image viewer §5.20 | G: — / W: open("data:text/html,<script>…") / T: rejected | a:frankmd-workspace::image_viewer | keep |
| 7 | 74 "treats untrusted alternative text as text rather than HTML" | Alt text shows literally | Image viewer caption §5.20 | G: alt `<img src=x onerror="evil()">` / W: open / T: caption == the literal string; one image only | a:frankmd-workspace::image_viewer | keep |

### `test/javascript/controllers/library_controller.test.js` — C=18 (keep 13 / changed 5 / dropped 0; a=17 b=1 c=0)
Original subject: media library workspace (cards, usage, preview, copy, delete, S3 rename, insert). New home: Media library view + detail sheet (§5.15), status line (§3.6). A tests 333/422 (item validation) excluded.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 124 "does not fetch the Library on connect; the app loads it when the workspace opens" | Media is scanned only when the library opens | Media library §5.15 | G: app started / W: — then ToggleMediaLibrary / T: no scan before; exactly one scan after | a:frankmd-workspace::media_library | keep |
| 2 | 130 "fetches the Library endpoint and renders image and video cards" | One card per item of the active tab with size; Videos tab switch | Media library grid §5.15 | G: images [photo one.png 2 KB], videos [clip.mp4] / W: open; select Videos / T: one image card (local path images/photo one.png, "2 KB"); Videos tab pressed, one video card "4 KB"; card actions Insert, Copy path, Delete | a:frankmd-workspace::media_library | changed (§5.15 card actions at rest, order) |
| 3 | 147 "defers usage lookup until a visible card intersects and shares one request across scrolling" | Usage is computed lazily and once | Media library usage §5.15 | G: 3 cards, none visible / W: mark_visible(card1) then card2 / T: no usage scan before; one shared scan; labels Checking… then resolved; Videos reuse cache | a:frankmd-workspace::media_library | keep |
| 4 | 181 "shows used, unused, and unknown usage labels only after cards intersect" | Usage line states | Media library usage §5.15 | G: usage {a:2, b:0}; scan error case / W: cards visible / T: "Used in 2 notes" (actionable), "Not used" (plain), unknown label on error | a:frankmd-workspace::media_library | changed (§5.15 "Not used" label) |
| 5 | 206 "refreshes the cached usage map when Library is reopened" | Reopening re-checks usage | Media library §5.15 | G: library opened once / W: close; reopen / T: labels reset to Checking…; second scan | a:frankmd-workspace::media_library | keep |
| 6 | 222 "opens usage paths in a dialog and dispatches the selected note" | User can jump to a note that uses the media | Detail sheet "Used in" §5.15 | G: usage notes [a.md, ../outside.md, a.md] / W: open detail; activate a.md / T: list [a.md] only; opens a.md; library closes | a:frankmd-workspace::media_library | changed (§5.15 detail sheet merges usage dialog) |
| 7 | 260 "opens image and video previews with metadata and clickable usage counts" | Detail shows name, size, date, type, aspect ratio, usage | Detail sheet §5.15 | G: photo 1920×1080 png used in 2 / W: enter on card / T: sheet metadata "<size> · <date> · 16:9 · PNG"; usage "Used in 2 notes"; video detail muted, "Not used" | a:frankmd-workspace::media_library | changed (§5.15 detail sheet) |
| 8 | 297 "traps keyboard focus in the preview and restores its trigger on close" | Sheet traps Tab and returns focus to the card | Detail sheet §5.15, focus §4.3 | G: card 2 focused / W: enter; tab past last action; shift-tab past first; escape / T: focus cycles inside sheet; after close focus on card 2 | b | keep |
| 9 | 317 "shows loading, empty, and failure states" | Loading, empty and error are distinguishable | Media library §5.15 | G: scan pending / empty / Err("offline") / W: open / T: Loading; empty text "No images yet. Paste or drop an image into a note to add one."; error text "offline" | a:frankmd-workspace::media_library | changed (§5.15 empty copy) |
| 10 | 344 "shows every item in the selected collection in the received order" | No surprise re-sorting | Media library §5.15 | G: listing [zebra.png, photo one.png] / W: open / T: cards in that order | a:frankmd-workspace::media_library | keep |
| 11 | 356 "copies the stored relative media path and reports clipboard failures" | Copy path copies the vault-relative path | Card actions §5.15, status §3.6 | G: card images/photo one.png; fake clipboard ok then Err / W: CopyPath twice / T: clipboard "images/photo one.png", status "Copied"; then status error | a:frankmd-workspace::media_library | keep |
| 12 | 372 "confirms deletion, removes the item on success, and reports API errors" | Delete asks, removes, or explains | Card actions §5.15 | G: two items / W: Delete → confirm (destructive); second returns Err("in use") / T: first card removed; error shown for second, card kept | a:frankmd-workspace::media_library | keep |
| 13 | 388 "does not issue a deletion request when confirmation is cancelled" | Cancel deletes nothing | Media library §5.15 | G: confirm open / W: cancel / T: no delete op | a:frankmd-workspace::media_library | keep |
| 14 | 396 "renders S3 media with remote URLs and renames only managed objects" | S3 items use remote URLs, usage unknown, rename available | Card actions Rename (S3) §5.15 | G: S3 item url https://… / W: Rename → "renamed.png" / T: thumbnail source = url; usage unknown (no scan); rename request {name:"renamed.png"}; card updated; local cards have no Rename | a:frankmd-workspace::media_library | keep |
| 15 | 434 "closes the preview after the app confirms a successful insertion request" | Insert from detail closes it when inserted | Detail sheet Insert §5.15 | G: detail open, note open / W: Insert; editor confirms insertion / T: InsertMedia{item} emitted Pending; on Inserted sheet closes (stays open on failure) | a:frankmd-workspace::media_library | keep |
| 16 | 450 "renderCard escapes a quoted filename and path payload" | Odd file names display and act literally | Media library §5.15 | G: name `photo" onerror="alert(1).png` / W: render card model / T: card name equals raw string; actions target raw path | a:frankmd-workspace::media_library | keep |
| 17 | 464 "updateUsageLabel escapes the usage path inside its button" | Usage action targets the raw path | Media library usage §5.15 | G: quoted path, usage 1 / W: usage label / T: label action path == raw string | a:frankmd-workspace::media_library | keep |
| 18 | 480 "renderUsageNotes escapes quoted note paths" | Used-in list shows quoted note names literally | Detail sheet §5.15 | G: usage note `a" b.md` / W: open detail / T: row label and open target == raw path | a:frankmd-workspace::media_library | keep |

### `test/javascript/controllers/locale_controller.test.js` — C=25 (keep 15 / changed 8 / dropped 2; a=20 b=3 c=0)
Original subject: language dropdown, translation loading, locale save + reload. New home: palette sub-mode "Language: Select…" (§5.1), Settings › Appearance › Language (§5.16), live switch without reload (§11). A tests (static list 58/62/70, getFlag 239/243/247, window.t 368–409) excluded.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 80 "loads initial locale from value" | App starts in the configured language | i18n startup | G: config locale "ja" (and unset) / W: start / T: current locale ja (unset → System, resolving to en fallback) | a:frankmd-i18n::picker | changed (§5.1 System default) |
| 2 | 84 "renders menu with locales" | Picker lists every language | Language picker §5.1 | G: — / W: open picker / T: rows = System (resolved name) + 7 locales | a:frankmd-i18n::picker | changed (§5.1 System row first) |
| 3 | 89 "calls loadTranslations" | Strings for the current locale are available at start | i18n catalog | G: locale pt-BR / W: start / T: catalog for pt-BR active, no network | a:frankmd-i18n::picker | keep |
| 4 | 98 "sets translations on window" | Switching swaps the active catalog | i18n | G: en active / W: select ja / T: t(key) returns ja string | a:frankmd-i18n::picker | keep |
| 5 | 103 "sets locale on window" | Current locale id is updated | i18n | G: en / W: select ja / T: current_locale() == "ja" | a:frankmd-i18n::picker | keep |
| 6 | 108 "dispatches translations-loaded event" | UI re-renders in the new language | Live switch §5.1, §11 | G: window open in en / W: select ja / T: LocaleChanged emitted once; window re-rendered (labels in ja) | a:frankmd-i18n::picker + b | keep |
| 7 | 120 "handles fetch error gracefully" | A bad locale never breaks the UI | i18n | G: locale id "xx" or missing catalog / W: select / T: falls back to en; warning logged; no panic | a:frankmd-i18n::picker | changed (§11 embedded catalog, no fetch) |
| 8 | 130 "toggles menu visibility" | Picker opens and closes | Palette sub-mode §5.1 | G: — / W: dispatch SelectLanguage; then escape / T: command-palette in language mode visible, then gone | b | changed (§5.1 palette sub-mode replaces dropdown) |
| 9 | 140 "stops event propagation" | — | — | DOM event propagation detail, no user-observable behavior | — | dropped (DOM-specific) |
| 10 | 148 "updates current locale" | Choosing a language applies it | Language picker §5.1 | G: picker on en / W: select ja row, enter / T: current ja | a:frankmd-i18n::picker | keep |
| 11 | 155 "hides menu after selection" | Commit closes the picker | Palette §5.1 | G: picker open / W: enter on ja / T: palette closed; focus restored | b | keep |
| 12 | 163 "updates display immediately after selection" | No reload needed to see the new language | Live switch §5.1, §11 | G: en / W: select ja / T: Settings Language row label "日本語" and picker marks ja immediately | a:frankmd-i18n::picker | changed (§11 live, no reload) |
| 13 | 175 "does nothing if same locale is selected" | Re-choosing current language doesn't write config | Language picker §5.1 | G: current ja / W: select ja / T: no config write, no LocaleChanged | a:frankmd-i18n::picker | keep |
| 14 | 187 "updates current locale text" | Current language shown by native name | Settings › Appearance §5.16 | G: locale ja / W: — / T: language row value "日本語" | a:frankmd-workspace::settings_vm | keep |
| 15 | 193 "shows checkmark for current locale" | Current language is marked in the list | Language picker §5.1 | G: current pt-BR / W: open picker / T: only pt-BR row has current marker | a:frankmd-i18n::picker | keep |
| 16 | 207 "defaults to English if locale not found" | Unknown locale displays as English | i18n | G: config locale "zz" / W: display name / T: "English" | a:frankmd-i18n::picker | keep |
| 17 | 215 "creates buttons for all locales" | Every locale selectable | Language picker §5.1 | G: — / W: rows / T: 7 locale rows (+ System) | a:frankmd-i18n::picker | keep |
| 18 | 221 "sets data-locale attribute on buttons" | Rows map to locale ids in source order | Language picker §5.1 | G: — / W: rows / T: ids [en, pt-BR, pt-PT, es, he, ja, ko] in order after System | a:frankmd-i18n::picker | keep |
| 19 | 229 "includes flag emojis" | Languages are recognizable | Language picker §5.1 | G: — / W: rows / T: each row label is the native name (flag glyph optional, see open question) | a:frankmd-i18n::picker | changed (§5.1 native names; flags unspecified) |
| 20 | 253 "calls fetch with correct parameters and reloads" | Chosen language persists | Settings scope §5.16, §11 | G: scope All vaults / W: select ja / T: config.toml locale = "ja" written; no reload; UI live | a:frankmd-i18n::picker | changed (§11 live switch, config write) |
| 21 | 271 "reloads even when save fails" | Language applies even if persisting fails | §5.16 inline errors | G: config write fails / W: select ja / T: UI switches to ja for the session; status-line error about saving | a:frankmd-i18n::picker | changed (§11 no reload; §3.6 error) |
| 22 | 283 "updates locale when config-changed event is dispatched" | External config change switches language live | Settings / config watch | G: current en / W: ConfigChanged{locale:"ja"} (Settings or .fed edit) / T: current ja; UI re-rendered | a:frankmd-i18n::picker | keep |
| 23 | 294 "ignores event if locale is same as current" | Unrelated config changes don't re-render | i18n | G: current ja / W: ConfigChanged{locale:"ja"} / T: no LocaleChanged | a:frankmd-i18n::picker | keep |
| 24 | 308 "closes menu when clicking outside" | Clicking outside dismisses the picker | Overlay rules §5 | G: language picker open / W: click scrim / T: closed; locale unchanged | b | keep |
| 25 | 324 "removes event listeners" | — | — | Listener cleanup on disconnect, no user-observable behavior | — | dropped (DOM-specific) |

### `test/javascript/controllers/local_images_controller.test.js` — C=9 (keep 9 / changed 0 / dropped 0; a=9 b=0 c=0)
Original subject: Library source of the image picker (collections, paging, reuse, import, delete). New home: Insert image › Library source and footer storage row (§5.6). it.each ×2 = 2 rows.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 39 "loads managed images by default as a server-rendered Turbo Stream" | Library source shows Note images page 1 without stealing focus | Insert image Library §5.6 | G: dialog open, rail focused / W: activate Library / T: query {collection:Managed, search:"", limit:10, offset:0}; focus stays on rail | a:frankmd-workspace::insert_image + b | keep |
| 2 | 48 "changes collections and page size without keeping a stale selected image" | Switching collection/page size resets selection and page | Library §5.6 | G: managed, image selected / W: collection Image folder, search "photo", per-page 25; then next page / T: selection None; offset 0, limit 25; next → offset 25 | a:frankmd-workspace::insert_image | keep |
| 3 | 61 "returns the existing managed or server reference without uploading a copy" | Existing images are referenced, not duplicated | Library §5.6 | G: managed images/photo.png; Image-folder photo.png / W: insert each / T: url "images/photo.png" / image-folder url; no copy op | a:frankmd-workspace::insert_image | keep |
| 4 | 70 "saves an optional server image copy through the provider-neutral endpoint" | Image-folder image can be copied to the chosen store | Storage row §5.6 | G: Image-folder photo.png, store S3, key folder/photo.png / W: insert / T: ImportRequest{path:"photo.png", resize:"", s3_key:"folder/photo.png"}; inserted url from result | a:frankmd-workspace::insert_image | keep |
| 5 | 80 "deletes %s files through the correct service" [row 1] managed | Deleting a Note image uses the library delete after confirm | Library per-thumbnail delete §5.6 | G: managed photo.png selected / W: delete button; confirm (destructive) / T: DeleteMedia{images/photo.png} | a:frankmd-workspace::insert_image | keep |
| 6 | 80 "deletes %s files through the correct service" [row 2] server | Deleting an Image-folder image uses the image-folder delete | Library §5.6 | G: Image-folder photo.png / W: delete; confirm / T: DeleteImageFolderFile{photo.png} | a:frankmd-workspace::insert_image | keep |
| 7 | 87 "does not delete when confirmation is cancelled" | Cancel deletes nothing | Library §5.6 | G: confirm open / W: cancel / T: no delete op | a:frankmd-workspace::insert_image | keep |
| 8 | 93 "keeps deletion and load failures inline" | Errors show in the source, clear on success | Library §5.6 | G: delete Err("delete failed"); load Err("offline") / W: delete; load; load Ok / T: inline error each time; cleared after successful load | a:frankmd-workspace::insert_image | keep |
| 9 | 104 "disables an unavailable server collection while keeping note images usable" | Image folder unavailable → disabled, Note images still work | Library collection select §5.6 | G: images_path not configured / W: activate Library / T: Image folder option disabled with reason; collection Managed | a:frankmd-workspace::insert_image | keep |

### `test/javascript/controllers/log_viewer_controller.test.js` — C=28 (keep 23 / changed 5 / dropped 0; a=23 b=5 c=0)
Original subject: log viewer dialog with Logs / Config tabs. New home: Settings › Diagnostics (Logs, Config sources) opened by Ctrl+Shift+O (§5.19, §5.16).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 95 "shows the dialog" | Shortcut opens diagnostics | Diagnostics §5.19 | G: editor focused / W: press ctrl-shift-o / T: settings-sheet visible on Diagnostics › Logs | b | changed (§5.19 Settings › Diagnostics replaces dialog) |
| 2 | 107 "shows loading state initially" | Loading text until logs arrive | Diagnostics §5.19 | G: log read pending / W: open / T: logs state Loading (t(dialogs.log_viewer.loading)) | a:frankmd-workspace::diagnostics | keep |
| 3 | 117 "defaults to logs tab" | Logs section first | Diagnostics §5.19 | G: — / W: OpenDiagnostics / T: section == Logs | a:frankmd-workspace::diagnostics | keep |
| 4 | 130 "fetches logs from server" | Last 100 lines shown | Diagnostics Logs §5.16 | G: log file 150 lines / W: load / T: tail requested with 100; 100 lines shown | a:frankmd-workspace::diagnostics | changed (§5.16 local log tail, no HTTP) |
| 5 | 145 "displays environment and file name" | User sees which log file | Diagnostics Logs §5.16 | G: env production, file production.log / W: load / T: header "production - production.log" | a:frankmd-workspace::diagnostics | keep |
| 6 | 161 "displays log lines" | Lines shown in order | Diagnostics Logs | G: lines [a,b,c] / W: load / T: content "a\nb\nc" | a:frankmd-workspace::diagnostics | keep |
| 7 | 177 "shows empty message for empty log" | Empty log is explicit | Diagnostics Logs | G: lines [] / W: load / T: t(empty_logs) "(log is empty)" | a:frankmd-workspace::diagnostics | keep |
| 8 | 193 "displays line count in status" | Line count shown | Diagnostics Logs | G: 5 lines / W: load / T: status "5 lines" | a:frankmd-workspace::diagnostics | keep |
| 9 | 209 "uses the singular line count label" | Singular wording | Diagnostics Logs | G: 1 line / W: load / T: "1 line" (line_count_one) | a:frankmd-workspace::diagnostics | keep |
| 10 | 226 "handles fetch errors gracefully" | Read error explained | Diagnostics Logs | G: log read Err("denied") / W: load / T: t(error_loading_logs,{error:"denied"}) | a:frankmd-workspace::diagnostics | keep |
| 11 | 236 "handles non-ok response" | Any failure explained the same way | Diagnostics Logs | G: log file missing/unreadable / W: load / T: error_loading_logs with reason | a:frankmd-workspace::diagnostics | keep |
| 12 | 250 "fetches config from server" | Config sources load when that section opens | Diagnostics Config sources §5.16 | G: — / W: select Config sources / T: inspector called once | a:frankmd-workspace::diagnostics | keep |
| 13 | 269 "displays config entries in table" | Key, value, source shown; secrets masked | Config sources table §5.16 | G: entries [{theme, dark, .fed}, {editor_font, x, default}, {openai_api_key, sk.., env}] / W: render / T: rows key/value/source; secret masked; env var name shown | a:frankmd-workspace::diagnostics | keep |
| 14 | 293 "shows key count in status" | Count shown | Config sources | G: 2 entries / W: render / T: "2 keys" | a:frankmd-workspace::diagnostics | keep |
| 15 | 314 "uses the singular key count label" | Singular wording | Config sources | G: 1 entry / W: render / T: "1 key" | a:frankmd-workspace::diagnostics | keep |
| 16 | 333 "localizes config labels and shows an empty state when there are no entries" | Labels localized; empty and missing-file states | Config sources | G: entries [], no config file, locale pt-BR / W: render / T: labels from catalog (Config file / File exists / AI configured in .fed); empty_config; no_config_file | a:frankmd-workspace::diagnostics | keep |
| 17 | 356 "handles fetch errors gracefully" | Inspector error explained | Config sources | G: inspector Err / W: load / T: error_loading_config with reason | a:frankmd-workspace::diagnostics | keep |
| 18 | 368 "localizes config loading state" | Loading text localized | Config sources | G: locale ja, pending / W: select section / T: loading string from ja catalog | a:frankmd-workspace::diagnostics | keep |
| 19 | 377 "showLogs reveals log content and hides config" | One section at a time | Settings category list §5.16 | G: Config sources shown / W: select Logs / T: section Logs; config table not rendered | a:frankmd-workspace::diagnostics | keep |
| 20 | 392 "showConfig reveals config content and hides logs" | Inverse | §5.16 | G: Logs / W: select Config sources / T: section Config; logs not rendered | a:frankmd-workspace::diagnostics | keep |
| 21 | 413 "showConfig updates title" | Section heading names the content | §5.16 | G: — / W: select Config sources / T: heading "Config sources" | a:frankmd-workspace::diagnostics | changed (§5.16 heading names) |
| 22 | 431 "showLogs updates title" | Section heading names the content | §5.16 | G: — / W: select Logs / T: heading "Logs" (not "Rails Log") | a:frankmd-workspace::diagnostics | changed (§5.16 heading names) |
| 23 | 442 "refreshes logs when on logs tab" | Refresh re-reads logs | Diagnostics Logs refresh §5.16 | G: Logs section / W: refresh / T: log tail read again; inspector not called | a:frankmd-workspace::diagnostics | keep |
| 24 | 454 "refreshes config when on config tab" | Refresh re-reads config | §5.16 | G: Config section / W: refresh / T: inspector called again; logs not read | a:frankmd-workspace::diagnostics | keep |
| 25 | 474 "closes the dialog" | Diagnostics can be closed | Settings sheet §5.16 | G: settings-sheet open / W: press ctrl-, (toggle) / T: closed; focus restored | b | changed (§5.16 settings sheet) |
| 26 | 484 "closes on backdrop click" | Backdrop click dismisses | Overlay rules §5 | G: settings-sheet on Diagnostics / W: click scrim / T: closed (see open question) | b | keep |
| 27 | 494 "does not close when clicking inside dialog" | Clicks inside don't dismiss | §5 | G: sheet open / W: click inside the sheet body / T: still open | b | keep |
| 28 | 504 "closes on Escape key" | Esc closes | §5.16 (Esc stays enabled) | G: sheet open / W: escape / T: closed; focus restored | b | keep |

#### New VM requirements (G6)
- `frankmd-workspace::explorer` — `context_target()`, `open_context_menu(row) -> ContextMenu{items, targets}` keeping a multi-selection when the row is inside it, otherwise reselecting (fo 6–8, 16–18, 36–38); `context_menu_items()` excludes NewNoteHere/NewFolderHere on note rows and Rename/Move/Delete on config rows (fo 9, 17, 37); inline-rename model `begin_rename(path) -> text` (basename sans `.md`), `commit_rename(text) -> Result<RenameRequest, InlineError>` (adds `.md` to notes, blank → `errors.title_required`), cancel, error kept while editing (fo 48–58, 63); `expand_ancestors(path)` applied before create (fo 30–31); `delete_targets(selection)` dropping descendants and protected config (fo 68–69).
- `frankmd-workspace::dialogs` — `NewNoteForm{destination, template, name, slug, error}`, `new(selection)` = single selected folder or root, `can_submit`, `submit() -> Option<CreateRequest::{Empty{path}, Hugo{title,parent,slug}}>`; `NewFolderForm{parent, name, error}` with subtitle/title keys (fo 3–4, 12–15, 19, 22–27, 32, 34–35, 41, 43–45, 47); `DeleteConfirm` built from a captured target list with counts, file/folder/multi wording, destructive, initial focus Cancel; sequential executor that stops at the first error (fo 65–67, 70–73).
- `frankmd-workspace::navigation` — `prepare_for_transition()` gate before rename/delete with abort on failure, `resume_after_transition()`; `rename_path(item, name)` for the pane-header title; handlers for FileCreated/FileRenamed/FileDeleted that remap the open note, MRU and selection or return to the empty state (fo 28, 56, 59–62, 74–76; home 4).
- `frankmd-workspace::find_bar` (new) — `FindBarState{open, replace_visible, query, regex, case, matches, current, status: Counter|Error|Replaced(n){since}}`; `find_next` from caret selection end; Replaced feedback with 2 s TTL via injected Clock (find_replace 1, 3–5). Uses `frankmd-search::find` for matching.
- `frankmd-workspace::quick_open` — recents store API: record in call order, refresh generation guard, read failure keeps previous rows and emits a status error, `clear_history()` with confirm text (home 1–3, 5–6).
- `frankmd-workspace::motion` (new, or a field of `layout`) — `reduce_motion` flag and `scroll_duration()` (0 when reduced, 120 ms otherwise) (home 7).
- `frankmd-workspace::insert_image` — Upload ingest of pasted files with Processing status always cleared, missing ingest path; insert pipeline waiting on a pending provider save, error kept inline plus one storage refresh; `InsertResult{markdown, url, alt, link}`; Folder source errors `folder_access_failed`/`folder_read_failed` (fake portal); Library source query `{collection, search, limit, offset}`, collection/page-size reset, reuse of existing references, `ImportRequest{path, resize, s3_key}`, per-collection delete ops, Image folder option disabled when unavailable (image_picker 1, 3–7; folder_images 1–2; local_images 1–9).
- `frankmd-workspace::image_viewer` (new) — `ImageViewerState{src, alt, generation, status: Loading|Loaded|Unavailable, return_focus}`; `open(src)` rejects `javascript:`, `file://`, non-image `data:` (accepts vault-relative paths and http(s)); stale-generation results ignored; focus falls back to the preview pane when the trigger disappears (image_viewer 1–7).
- `frankmd-workspace::media_library` — lazy usage on `mark_visible(card)` with one shared scan and a cache reset on reopen; usage label states Checking/Used(n)/NotUsed/Unknown (S3 always Unknown); detail sheet model (metadata string incl. aspect ratio, filtered and deduped "Used in" list); copy path through a clipboard trait; delete/rename flows with inline errors; insert with Pending → Inserted closing the sheet; no re-sorting (library 1–7, 9–18).
- `frankmd-i18n::picker` — rows = System + 7 locales with native names in source order, current marker, `select(id)` that is a no-op for the current id, emits `LocaleChanged` and writes `locale` to the scoped config, keeps the live switch when the write fails, falls back to en for unknown ids, reacts to `ConfigChanged{locale}` (locale 1–8, 10, 12–13, 15–23).
- `frankmd-workspace::diagnostics` (new) — `DiagnosticsState{section: Logs|ConfigSources, logs: Loading|Loaded{env, file, lines}|Empty|Error, config: Loading|Loaded{entries, file, file_exists, ai_in_file}|Empty|Error}`; tail N = 100; count strings with singular forms; `refresh()` reloads the active section only; secrets masked (log_viewer 2–24).
- `frankmd-workspace::overlay` — Settings sheet opened at a given category/section (OpenDiagnostics → Diagnostics › Logs) (log_viewer 1).

#### Open questions (G6)
- Config rows in the explorer (fo 9): §5.14 hides dotfiles, so `.fed` never shows. FrankMD showed config files (fileType config) with no context menu. Does the native tree show `.fed` at all (for example as "Vault settings", opening Settings › This vault)? If it does, its menu must exclude rename/delete.
- New folder Enter (fo 79): §5.13 says "Create Ctrl+↵ / Enter" only for New note. The plan assumes plain Enter also submits New folder and Move, for parity with FrankMD.
- Multi-delete failure (fo 70): the design doesn't say whether a failed batch shows its error in the status line or inline in the still-open confirm dialog. The plan uses the status line.
- Home "Show more/less" (home 6): §5.2 drops the 5-item collapse and lists up to 50 rows. Confirm that the empty state needs no collapsed recents section.
- Language picker live preview (locale 8–12): §2.1 says picker sub-modes preview while arrowing and revert on Esc, but §5.1 says only "switches live". Does arrowing over languages re-render the UI, or does only Enter apply?
- Language flags (locale 19): FrankMD shows flag emoji. §5.1 mentions native names only. Keep the flags or drop them?
- Settings sheet backdrop (log_viewer 26): §5 intro lists the overlays that close on a backdrop click and Settings isn't one of them. Should a scrim click close Settings/Diagnostics?
- Image viewer sources (image_viewer 5): the plan rejects `file://`. Should absolute paths inside the vault (as opposed to vault-relative ones) be accepted?
- Log header (log_viewer 5): what does `<env> - <file>` mean natively? The plan assumes FRANKMD_ENV plus the log file name.

## 12. G7: media_storage, media_storage_menu, path_display, preview, recovery_diff, s3_option, scroll_sync

### `test/javascript/controllers/media_storage_controller.test.js` — C=8 (keep 5 / changed 3 / dropped 0; a=8 b=0 c=0)
Original subject: shared "storage for new media" provider (Local/S3) with immediate save, upload gate and S3 fallback warning. New home: one `MediaStorageState` read by Settings › Media "Storage for new media" (§5.16) and the Insert image footer storage row (§5.6); fallback warning becomes the "Storage provider fallback" pane banner (§5.12).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 36 "saves immediately, blocks consumers, and updates every selector after acknowledgement" | Switching provider persists at once; uploads wait for the ack; every storage control shows the new value only after it | Settings › Media §5.16 + image footer §5.6 | G: provider Local, two views bound (settings row, image footer) / W: `request(S3)`; queue an upload on `ready()`; then `ack(Ok(S3, s3_available=true))` / T: while pending both controls `disabled`, value still Local, upload not started; after ack both show S3, enabled, upload released once | a:frankmd-workspace::media_storage | keep |
| 2 | 54 "restores previous controls and rejects waiting uploads on failed persistence" | A failed save keeps the old provider and fails queued uploads with a clear message | §5.16 inline error + §5.6 footer error | G: Local, upload waiting on `ready()` / W: `request(S3)` then `ack(Err(write))` / T: returns false; waiter gets Err(t(`media_storage.save_failed`)); every view shows Local, enabled; settings row and image footer both show the inline `danger` error | a:frankmd-workspace::media_storage | keep |
| 3 | 67 "retains saved S3 when the server cannot persist recovery" | Saved S3 with credentials gone still reads S3, S3 can't be re-picked, no warning; later Local syncs | §5.16 Media + §5.6 storage row | G: config says provider=s3, s3_available=false / W: `apply(config)`; later `apply(local, available)` / T: value S3, S3 entry disabled with reason, S3-only rows (bucket/key/resize) still shown, no fallback banner; then value Local everywhere | a:frankmd-workspace::media_storage | keep |
| 4 | 78 "synchronizes Local and shows one brief warning after acknowledged recovery" | Acknowledged fallback switches to Local and warns exactly once | Banner "Storage provider fallback" §5.12 | G: provider S3 / W: `apply(local, s3_available=false, fallback=true)` twice, then a plain refresh / T: value Local, S3 disabled; `BannerStack` gets `StorageFallback` once (not re-pushed on re-apply or refresh); banner offers Settings / Dismiss | a:frankmd-workspace::media_storage + banners | changed (§5.12 banner replaces 6 s auto-closing dialog) |
| 5 | 97 "shows recovery reported by the initial page and supports early dismissal" | Fallback detected at startup is announced; user can dismiss it | §5.12 banner | G: startup config reports fallback / W: workspace starts; then Dismiss / T: `StorageFallback` banner present after startup; Dismiss removes it and it does not come back this session | a:frankmd-workspace::banners | changed (§5.12 banner, no 6 s auto-close) |
| 6 | 109 "keeps a recovery warning consumed by Settings' first configuration request" | A fallback noticed by any consumer (e.g. Settings loading config) is still surfaced | §5.12 banner | G: config resolver already consumed the fallback (`fallback_known=true`) / W: `refresh(known_fallback=true)` resolves to Local / T: banner shown once | a:frankmd-workspace::media_storage | changed (no HTTP one-shot flag; resolver event instead) |
| 7 | 117 "does not warn for an explicit switch to Local or a failed refresh" | Only real fallbacks warn | §5.12 | G: S3 / W: user picks Local (ack ok); then a refresh that fails with known_fallback=true / T: no `StorageFallback` banner | a:frankmd-workspace::media_storage | keep |
| 8 | 127 "reports network failures without changing the acknowledged provider" | Write failure shows its message and leaves the acknowledged provider alone | §5.16 inline error | G: Local / W: `request(S3)` then `ack(Err(Io("offline")))` / T: acknowledged provider Local; error text "offline" (wrapped by i18n) under the row | a:frankmd-workspace::media_storage | keep |

### `test/javascript/controllers/media_storage_menu_controller.test.js` — C=4 (keep 4 / changed 0 / dropped 0; a=3 b=1 c=0)
Original subject: custom dropdown for the storage provider in Settings. New home: Settings › Media "Storage for new media" Select row (§5.16) bound to `MediaStorageState`; popover keyboard per §6.4 / Esc chain §6.1.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 41 "shares acknowledged selection and disables all controls while saving" | Row label shows the acknowledged value; control disabled while saving; checked mark and other views follow the ack | Settings › Media row §5.16 | G: settings row for `media_storage_provider` = Local / W: choose S3; then ack ok / T: during pending row control disabled + label "Local"; after ack label "S3", S3 option `checked`, image footer storage row also S3 | a:frankmd-workspace::settings_vm | keep |
| 2 | 57 "keeps the acknowledged label and reports a failed save" | Failed save keeps Local and shows the error | §5.16 inline `danger` error | G: Local / W: choose S3, ack Err / T: label "Local", control enabled, row error = t(`media_storage.save_failed`) | a:frankmd-workspace::settings_vm | keep |
| 3 | 65 "shows saved S3 when credentials disappear and prevents selecting it" | Saved S3 shown even without credentials; S3 option disabled with reason; reason clears when available again | §5.16 row + §5.6 "S3 disabled with reason" | G: provider s3, s3_available=false / W: build row options; then s3_available=true / T: label "S3"; S3 option disabled, reason "This provider is not configured."; after: reason None, Local enabled | a:frankmd-workspace::settings_vm | keep |
| 4 | 76 "moves keyboard focus, dismisses with Escape, and closes on outside click" | Select popover keyboard: opens with focus on first option, ↓ moves, Esc closes it only (Settings stays open) and refocuses the trigger, outside click closes | Select popover in `settings-sheet` §5.16, Esc chain §6.1 | G: Settings open on Media / W: click row Select; press down; press escape; reopen; click outside / T: popover visible, option 1 then option 2 focused; after Esc popover gone, `settings-sheet` still visible, trigger focused; outside click closes popover | b | keep |

### `test/javascript/controllers/path_display_controller.test.js` — C=12 (keep 4 / changed 6 / dropped 2; a=10 b=0 c=0)
Original subject: header path label (full path / basename, copy, truncate-on-hover). New home: editor pane header breadcrumb + title (§3.4), Document menu "Copy path", status-line message slot (§3.6), empty state (§5.2), window title (§3.2).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 48 "sets up resize listener" | — (listener wiring) | — | — | — | dropped (DOM-specific; width re-evaluation covered by `layout::WidthClass` tests) |
| 2 | 54 "shows select note message for null path" | With no note open the user is told to pick one | Empty state §5.2 + window title §3.2 | G: no open note / W: build `PaneHeaderVm` and window title / T: header has no title/breadcrumb; empty state (docked quick open) shown; window title "FrankMD — <vault>" | a:frankmd-workspace::pane_header | changed (§5.2 empty state replaces "Select a note") |
| 3 | 61 "stores full path in dataset" | The full vault-relative path is kept for tooltip/copy | Pane header §3.4 | G: open "folder/test.md" / W: `PaneHeaderVm::from(note)` / T: `full_path == "folder/test.md"` | a:frankmd-workspace::pane_header | keep |
| 4 | 67 "shows full path when it fits" | At full width the whole location is visible | Breadcrumb §3.4 | G: Wide, "folder/test.md" / W: `breadcrumb(path, Wide)` / T: segments ["folder"], title "test" (no `.md`), not collapsed | a:frankmd-workspace::pane_header | changed (§3.4 breadcrumb + title instead of raw path) |
| 5 | 78 "shows a basename while preserving the full path for tooltips and copying" | Title is the literal name without extension (never markup), tooltip is the full path, a11y label "Rename document: <name>", Copy path copies the full path | Title §3.4 + Copy path §3.4 menu | G: open "nested/<note>.md" / W: build header; run CopyPath / T: title "<note>" literal; tooltip "nested/<note>.md"; a11y label "Rename document: <note>" (i18n); fake clipboard got "nested/<note>.md" | a:frankmd-workspace::pane_header | keep |
| 6 | 93 "copies full path to clipboard" | Copy path puts the full relative path on the clipboard | Document menu / palette CopyPath | G: open "folder/test.md", fake `Clipboard` / W: dispatch CopyPath / T: clipboard text "folder/test.md" | a:frankmd-workspace::commands | keep |
| 7 | 103 "does nothing if no path stored" | Copy path can't run without a note | Palette availability §5.1 | G: no note open / W: `availability(CopyPath)` / T: Unavailable("Open a markdown note first"); executing is a no-op, clipboard untouched | a:frankmd-workspace::commands | keep |
| 8 | 111 "shows toast notification temporarily" | User sees a brief "Copied" confirmation | Status line message slot §3.6 | G: open note, injected Clock / W: CopyPath; advance 2 s / T: `StatusLine.message` = t(`status.copied_to_clipboard`) info; cleared after 2 s; appended to Diagnostics › Messages | a:frankmd-workspace::status | changed (§3.6 status message replaces toast) |
| 9 | 133 "does nothing if not truncated" | Uncollapsed breadcrumb isn't altered by hover | Breadcrumb §3.4 | G: Wide, "short.md" / W: hover title (no-op in VM) / T: breadcrumb unchanged, title "short", tooltip "short.md" | a:frankmd-workspace::pane_header | changed (§1 rule 3: tooltip, no hover swap) |
| 10 | 143 "shows full path when truncated" | When the location is shortened the full path is still reachable | Breadcrumb collapse < 720 px §3.4 | G: Narrow, "very/long/path/file.md" / W: `breadcrumb(path, Narrow)` / T: segments ["…"], title "file"; tooltip "very/long/path/file.md" | a:frankmd-workspace::pane_header | changed (§3.4 `…/ Title` + tooltip) |
| 11 | 155 "restores truncated path" | Shortened form comes back after the full view | Breadcrumb §3.4 + width hysteresis §4.1 | G: header computed at Narrow / W: width class goes Wide then Narrow / T: full segments at Wide; `…/ file` again at Narrow | a:frankmd-workspace::pane_header | changed (driven by width class, not mouseleave) |
| 12 | 166 "debounces function calls" | — (debounce helper) | — | — | — | dropped (DOM-specific; resize flicker handled by 24 px hysteresis in `layout`) |

### `test/javascript/controllers/preview_controller.test.js` — C=83 (keep 76 / changed 6 / dropped 1; a=82 b=0 c=0)
Original subject: preview pane (zoom, visibility, render, scroll sync primitives, image activation, task checkboxes). New home: preview pane §3.5/§5.21 rendered from `frankmd-markdown` RenderDoc; pane VM `workspace::preview`; scroll math `workspace::scroll_sync` (architecture §3.8, (block, offset) positions from `LineMap`); image viewer §5.20. Scroll assertions use injected `PreviewMetrics {block_tops, viewport_h, content_h, scroll_top}` and assert `ScrollIntent {top_px, animate}`, not pixels. (A rows 45, 264, 271, 291, 298, 194 are ported elsewhere.)

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 49 "applies initial zoom" | Preview opens at the saved zoom; header shows it | Preview header §3.5, Settings › Preview Zoom §5.16 | G: setting `preview_zoom=100` / W: build `PreviewVm` / T: zoom 100, header label "100%" | a:frankmd-workspace::preview | keep |
| 2 | 58 "creates keyboard buttons after sanitization and dispatches sanitized image data" | Images are keyboard-activatable, open the viewer with alt/src, and no script attrs survive | Images §5.21, Image viewer §5.20 | G: source `<img src="/safe.png" alt="Sample" onerror="evil()">` / W: `render`; `activate(ImageHit(0))` / T: IR holds `Inline::Image{src:SafeUrl("/safe.png"), alt:"Sample"}` with no handler data; image is a focus stop; action `OpenImageViewer{alt, src, return_focus: image 0}` | a:frankmd-markdown::parse + a:frankmd-workspace::preview | keep |
| 3 | 72 "preserves linked image navigation and provides a separate enlarge button" | An image inside a link follows the link; a separate control opens the viewer | Links/images §5.21 | G: `[![Sample](/safe.png)](/destination)` / W: `activate(ImageHit)` then `activate(EnlargeHit)` / T: first → `OpenLink("/destination")` (internal note nav or browser per §5.21), no viewer; second → `OpenImageViewer` once | a:frankmd-workspace::preview | keep |
| 4 | 87 "keeps picture sources together inside an unlinked trigger" | `<picture>` renders as one activatable image | Raw HTML allow-list §5.21 | G: `<picture><source srcset="/large.png"><img src="/safe.png"></picture>` / W: `render` / T: exactly one `Inline::Image` (src "/safe.png"), one focus stop; activation opens the viewer | a:frankmd-markdown::parse | changed (§5.21 allow-list maps picture → single image) |
| 5 | 96 "shows hidden panel" | Toggle preview shows it and reports the new state | TogglePreview §4.2 | G: preview hidden / W: `layout.toggle_preview()` / T: returns true; `preview_visible`; `resolve_panes` includes preview | a:frankmd-workspace::layout | keep |
| 6 | 107 "hides visible panel" | Toggle hides a visible preview | §4.2 | G: visible / W: toggle / T: returns false; preview absent from `resolve_panes` | a:frankmd-workspace::layout | keep |
| 7 | 118 "dispatches toggled event" | Showing the preview triggers its re-sync | §4.2 + scroll sync | G: hidden / W: toggle / T: effect `PreviewToggled{visible:true}` emitted once (consumed by scroll_sync) | a:frankmd-workspace::layout | keep |
| 8 | 130 "shows the panel" | show() makes it visible | §4.2 | G: hidden / W: `set_preview_visible(true)` / T: visible | a:frankmd-workspace::layout | keep |
| 9 | 137 "does nothing if already visible" | No redundant re-sync | §4.2 | G: visible / W: `set_preview_visible(true)` / T: no `PreviewToggled` effect | a:frankmd-workspace::layout | keep |
| 10 | 147 "dispatches toggled event" | show emits the toggle | §4.2 | G: hidden / W: `set_preview_visible(true)` / T: `PreviewToggled{visible:true}` | a:frankmd-workspace::layout | keep |
| 11 | 159 "hides the panel" | hide() hides | §4.2 | G: visible / W: `set_preview_visible(false)` / T: hidden | a:frankmd-workspace::layout | keep |
| 12 | 169 "does nothing if already hidden" | No event when already hidden | §4.2 | G: hidden / W: `set_preview_visible(false)` / T: no effect | a:frankmd-workspace::layout | keep |
| 13 | 179 "returns false when panel is hidden" | Initial visibility comes from persisted state | §4.2 (FE-172 changed: per-vault persistence) | G: vault state with no saved preview flag / W: load `LayoutState` / T: `preview_visible == false` (default); with saved true → true | a:frankmd-workspace::layout | changed (§4.2 visibility persisted per vault) |
| 14 | 183 "returns true when panel is visible" | Visibility query is truthful | §4.2 | G: visible / W: query / T: true | a:frankmd-workspace::layout | keep |
| 15 | 204 "does nothing when hidden" | Hidden preview doesn't render on edit | §4.2 | G: hidden / W: `PreviewVm::content_changed(src)` / T: no render request; on show the current source renders | a:frankmd-workspace::preview | keep |
| 16 | 211 "handles empty content" | Empty note → empty preview | §5.21 | G: "" / W: `render` / T: `blocks.is_empty()`, `total_lines` 0 | a:frankmd-markdown::parse | keep |
| 17 | 216 "sets _isUpdatingContent flag during render" | Re-render does not count as a user scroll | Scroll sync §5.21 | G: visible, Clock t0 / W: `on_rendered(doc)` / T: `settling == true` | a:frankmd-workspace::preview | keep |
| 18 | 225 "clears _isUpdatingContent flag after timeout" | Settle window ends 100 ms after render | §5.21 | G: rendered at t0 / W: advance 110 ms / T: `settling == false` | a:frankmd-workspace::preview | keep |
| 19 | 240 "renders content and syncs scroll" | Update with a ratio renders and positions the preview | Scroll sync | G: visible / W: `update(src, SyncHint::Ratio(0.5))` / T: render requested for src; returns `ScrollIntent` from `ratio_to_offset(0.5)` | a:frankmd-workspace::preview | keep |
| 20 | 250 "syncs to typewriter mode when specified" | In typewriter mode the preview follows the caret line | §4.2 Typewriter | G: visible / W: `update(src, SyncHint::Typewriter{line:5,total:10})` / T: intent = `typewriter_target(5,10,metrics)` | a:frankmd-workspace::preview | keep |
| 21 | 278 "dispatches zoom-changed event" | Zoom in persists and re-anchors | Header zoom §3.5, ZoomIn §6.2 | G: zoom 100 / W: `zoom_in()` / T: effect `PreviewZoomChanged{110}` (settings write + scroll_sync re-anchor) | a:frankmd-workspace::preview | keep |
| 22 | 305 "dispatches zoom-changed event" | Zoom out persists and re-anchors | §3.5 | G: 100 / W: `zoom_out()` / T: `PreviewZoomChanged{90}` | a:frankmd-workspace::preview | keep |
| 23 | 318 "applies zoom to content target" | Preview text scales with zoom | Body text = zoom × 15 px §5.21 | G: zoom 125 / W: `preview::text_px(zoom)` / T: 18.75 | a:frankmd-workspace::preview | changed (§5.21 15 px base instead of CSS %) |
| 24 | 325 "updates zoom level display" | Header shows the zoom | §3.5 | G: zoom 150 / W: header label / T: "150%" | a:frankmd-workspace::preview | keep |
| 25 | 334 "calls applyZoom when value changes" | Changing zoom in Settings applies live | Settings › Preview §5.16 | G: preview open / W: settings write `preview_zoom=75` / T: `PreviewVm.zoom == 75`, label "75%", re-anchor effect | a:frankmd-workspace::preview | keep |
| 26 | 351 "sets scroll position based on ratio" | Ratio maps to the scrollable range | §5.21 scroll sync | G: content 1000, viewport 400, no anchors / W: `sync_ratio(0.5, None)` / T: intent top 300, animate false | a:frankmd-workspace::scroll_sync | keep |
| 27 | 361 "does nothing when preview is hidden" | Hidden preview never scrolls | §4.2 | G: hidden / W: `sync_ratio(0.5)` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 28 | 369 "respects syncScrollEnabled value" | Scroll sync off → no sync | ToggleScrollSync §6.1 | G: `sync_enabled=false` / W: `sync_ratio(0.5)` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 29 | 377 "scrolls to exact 0 when ratio is near 0" | Top snaps to top | §5.21 | G: scroll_top 100 / W: `sync_ratio(0.005)` / T: top 0 | a:frankmd-workspace::scroll_sync | keep |
| 30 | 389 "scrolls to exact bottom when ratio is near 1" | Bottom snaps to bottom | §5.21 | G: max 600 / W: `sync_ratio(0.995)` / T: top 600 | a:frankmd-workspace::scroll_sync | keep |
| 31 | 407 "scrolls to line position" | Cursor jump scrolls smoothly to the line | §5.21 | G: content 1000/400, total 10 / W: `sync_to_line(5,10)` / T: top ≈ 266.67 ((5−1)/(10−1)×600), animate true (false under reduced motion) | a:frankmd-workspace::scroll_sync | keep |
| 32 | 419 "does nothing when totalLines <= 1" | One-line docs don't scroll | §5.21 | G: total 1 / W: `sync_to_line(1,1)` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 33 | 432 "aligns the rendered block that matches the cursor line at the top" | Typewriter puts the caret's block at the preview top | §4.2 Typewriter | G: block starting line 5 at offset 380 in content / W: `typewriter_target(5,10,metrics)` / T: top 380, animate true | a:frankmd-workspace::scroll_sync | keep |
| 34 | 459 "falls back to line ratio when rendered content has no source anchors" | Without anchors typewriter uses the line ratio | §4.2 | G: LineMap empty, content 1000 / W: `typewriter_target(5,10)` / T: top ≈ 444.44 | a:frankmd-workspace::scroll_sync | keep |
| 35 | 479 "does nothing when totalLines <= 1" | No typewriter sync on one-line docs | §4.2 | G: total 1 / W: `typewriter_target(1,1)` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 36 | 488 "adds typewriter class when enabled" | Typewriter on gives the preview its bottom padding so the last block can reach the top | §4.2 | G: preview / W: `set_typewriter(true)` / T: `typewriter`; `bottom_padding_px == viewport_h/2` | a:frankmd-workspace::preview | keep |
| 37 | 495 "removes typewriter class when disabled" | Padding removed when off | §4.2 | G: typewriter on / W: `set_typewriter(false)` / T: off; padding 0 | a:frankmd-workspace::preview | keep |
| 38 | 516 "dispatches preview:programmatic-scroll (single lock lives in scroll-sync)" | Every programmatic preview scroll takes the sync lock so its echo is ignored | Scroll sync §5.21 | G: ScrollSync idle / W: apply any preview `ScrollIntent` / T: lock = Editor (programmatic) | a:frankmd-workspace::scroll_sync | keep |
| 39 | 524 "is dispatched by syncToLineSmooth (typing path)" | Typing-path scroll takes the lock | §5.21 | G: idle / W: `sync_to_line_smooth(3,10)` applied / T: lock Editor | a:frankmd-workspace::scroll_sync | keep |
| 40 | 532 "is dispatched by syncScrollRatio (editor/toggle path)" | Ratio sync takes the lock | §5.21 | G: idle / W: `sync_ratio(0.5)` applied / T: lock Editor | a:frankmd-workspace::scroll_sync | keep |
| 41 | 540 "is dispatched by syncToTypewriter" | Typewriter sync takes the lock | §4.2 | G: idle / W: `typewriter_target(3,10)` applied / T: lock Editor | a:frankmd-workspace::scroll_sync | keep |
| 42 | 548 "is dispatched by syncToLine (cursor jump)" | Cursor-jump sync takes the lock | §5.21 | G: idle / W: `sync_to_line(3,10)` applied / T: lock Editor | a:frankmd-workspace::scroll_sync | keep |
| 43 | 558 "dispatches scroll event with scroll ratio and line info" | User scroll of the preview reports ratio, top line, totals, typewriter flag | Preview → editor sync | G: scroll_top 50, content 200, viewport 100, no doc / W: `on_user_scroll(metrics)` / T: `PreviewScrolled{ratio:0.5, top_line:None, total_lines:0, typewriter:false}` | a:frankmd-workspace::preview | keep |
| 44 | 578 "does not dispatch when scroll sync is disabled" | Sync off → preview scroll doesn't move editor | §6.1 | G: sync off / W: `on_user_scroll` / T: None | a:frankmd-workspace::preview | keep |
| 45 | 587 "dispatches echo scroll events (scroll-sync's single lock blocks them)" | Preview reports all user scrolls; the lock decides | §5.21 | G: no settle window / W: `on_user_scroll` / T: Some(PreviewScrolled) | a:frankmd-workspace::preview | keep |
| 46 | 602 "does not dispatch during content updates" | Scroll caused by re-render is not reported | §5.21 | G: settling / W: `on_user_scroll` / T: None | a:frankmd-workspace::preview | keep |
| 47 | 613 "dispatches programmatic-scroll so scroll-sync marks its lock" | Ratio sync marks the lock | §5.21 | G: idle / W: `sync_ratio(0.5)` / T: lock Editor (dup of 40 with lock check) | a:frankmd-workspace::scroll_sync | keep |
| 48 | 621 "syncs (and dispatches) when no local lock exists" | Ratio sync proceeds when unlocked | §5.21 | G: no lock / W: `sync_ratio(0.5)` / T: Some(intent) and lock Editor | a:frankmd-workspace::scroll_sync | keep |
| 49 | 633 "returns correct scroll ratio" | Preview ratio = top / scrollable range | §5.21 | G: 50 / 200 / 100 / W: `preview_ratio(metrics)` / T: 0.5 | a:frankmd-workspace::scroll_sync | keep |
| 50 | 641 "returns 0 when content is not scrollable" | Non-scrollable → 0 | §5.21 | G: content == viewport / W: `preview_ratio` / T: 0.0 | a:frankmd-workspace::scroll_sync | keep |
| 51 | 663 "initializes _lastRenderedContent to null" | First update always renders | §5.21 | G: new `PreviewVm` / W: `content_changed("# Hello")` / T: render requested | a:frankmd-workspace::preview | keep |
| 52 | 667 "show() invalidates content cache" | Showing the preview re-renders even unchanged text | §4.2 | G: rendered "x", then hidden / W: show; `content_changed("x")` / T: render requested | a:frankmd-workspace::preview | keep |
| 53 | 677 "updateWithSync skips DOM update when content unchanged" | Unchanged text isn't re-rendered | §5.21 | G: rendered "# Hello", Clock / W: `content_changed("# Hello")`, advance 200 ms / T: no new render request | a:frankmd-workspace::preview | keep |
| 54 | 693 "updateWithSync renders when content changes" | Edited text re-renders after the ~150 ms debounce | §5.21 | G: rendered "# Hello" / W: `content_changed("# Hello World")`, advance 200 ms / T: second render request (none before 150 ms) | a:frankmd-workspace::preview | keep |
| 55 | 707 "updateWithSync still syncs scroll when content unchanged but line changes" | Caret moving to another line still re-syncs in typewriter/typing path | §4.2 | G: rendered "# Hello\n\nWorld", last synced line 1 / W: `content_changed(same, caret_line 3)`, advance 200 ms / T: no render; `sync_to_line_smooth(3,3)` intent returned | a:frankmd-workspace::preview | keep |
| 56 | 736 "render() sets _isUpdatingContent to true immediately" | Settle window starts at render | §5.21 | G: not settling / W: `on_rendered` / T: settling | a:frankmd-workspace::preview | keep |
| 57 | 744 "render() clears _isUpdatingContent after 100ms" | Settle window ends | §5.21 | G: rendered t0 / W: advance 110 ms / T: not settling (dup of 18) | a:frankmd-workspace::preview | keep |
| 58 | 754 "onPreviewScroll does not dispatch when _isUpdatingContent is true" | Re-render scroll ignored | §5.21 | G: settling, scroll 50/200/100 / W: `on_user_scroll` / T: None | a:frankmd-workspace::preview | keep |
| 59 | 770 "onPreviewScroll dispatches when _isUpdatingContent is false" | Real scroll reported | §5.21 | G: not settling / W: `on_user_scroll` / T: Some | a:frankmd-workspace::preview | keep |
| 60 | 787 "multiple rapid renders extend the content update window" | Rapid typing keeps the window open until 100 ms after the last render | §5.21 | G: render t0 / W: render at t50; check t100; check t160 / T: settling at t100; not settling at t160 | a:frankmd-workspace::preview | keep |
| 61 | 813 "no longer keeps a private _scrollSource lock" | — (internal structure) | — | — | — | dropped (DOM-specific: controller-internal field; single lock owned by `ScrollSync` by construction) |
| 62 | 817 "echo scroll events dispatch and rely on scroll-sync's lock" | Echo of a programmatic scroll does not move the editor | §5.21 | G: content 1000/400 / W: `sync_ratio(0.5)` applied; then echo `on_user_scroll` → `ScrollSync::on_preview_scroll` / T: preview reports it; ScrollSync returns no editor intent (lock Editor) | a:frankmd-workspace::scroll_sync | keep |
| 63 | 844 "syncScrollRatio scrolls to exact 0 when ratio is near 0" | Snap to top | §5.21 | G: max 600 / W: `ratio_to_offset(0.005, 600)` / T: 0 (dup of 29, unanchored path) | a:frankmd-workspace::scroll_sync | keep |
| 64 | 852 "syncScrollRatio scrolls to exact bottom when ratio is near 1" | Snap to bottom | §5.21 | G: max 600 / W: `ratio_to_offset(0.995, 600)` / T: 600 | a:frankmd-workspace::scroll_sync | keep |
| 65 | 859 "syncScrollRatio handles ratio of exactly 0" | Exact 0 | §5.21 | G: max 600 / W: `ratio_to_offset(0.0)` / T: 0 | a:frankmd-workspace::scroll_sync | keep |
| 66 | 867 "syncScrollRatio handles ratio of exactly 1" | Exact 1 | §5.21 | G: max 600 / W: `ratio_to_offset(1.0)` / T: 600 | a:frankmd-workspace::scroll_sync | keep |
| 67 | 875 "typing scenario: render does not cause editor sync" | Typing never yanks the editor | §5.21 | G: visible / W: `on_rendered("# Hello World")` then `on_user_scroll` / T: None (no editor intent) | a:frankmd-workspace::preview | keep |
| 68 | 896 "explicit user scroll: preview dispatches after content update settles" | After settling, user scroll reports with real totals | §5.21 | G: rendered "# Hello World" t0 / W: advance 110 ms; `on_user_scroll(50/200/100)` / T: `PreviewScrolled{ratio 0.5, top_line None (metrics without anchors), total_lines 1, typewriter false}` | a:frankmd-workspace::preview | keep |
| 69 | 927 "editor scroll then preview scroll: single lock prevents feedback loop" | Editor-driven sync doesn't bounce back | §5.21 | G: idle / W: `on_editor_scroll` → preview intent; then preview echo / T: no editor intent | a:frankmd-workspace::scroll_sync | keep |
| 70 | 974 "typing path anchors on the element with the cursor's source line" | Typing path places the caret's block 50 px below the top | §5.21 (real source positions) | G: doc "# First\n\n# Second\n\n", blocks at 0/100 / W: `sync_to_line_smooth(3,4)` / T: top 50, animate true | a:frankmd-workspace::scroll_sync | keep |
| 71 | 984 "scroll path anchors on the same element for the same line" | Ratio path maps to the same block | §5.21 | G: same / W: `sync_ratio(0.5, None)` (round(0.5×4)+1 = line 3) / T: top 100, animate false | a:frankmd-workspace::scroll_sync | keep |
| 72 | 994 "both paths are invariant to the container's page offset" | Pane position in the window doesn't affect sync | §5.21 | G: same doc, metrics with pane origin y=5000 / W: both paths / T: tops 50 and 100 (positions are list-relative) | a:frankmd-workspace::scroll_sync | keep |
| 73 | 1004 "typing path accounts for stripped frontmatter via absolute line anchors" | Hidden front matter doesn't shift sync | Front matter hidden §5.21 | G: 4 front-matter lines then "# First"/"# Second" / W: render; `sync_to_line_smooth(7,9)` / T: blocks start at lines 5 and 7; top 50 | a:frankmd-markdown::parse + a:frankmd-workspace::scroll_sync | keep |
| 74 | 1020 "falls back to element-count ratio when no line annotations exist" | Without spans sync still lands sensibly | §5.21 | G: LineMap empty, 2 blocks at 0/100 / W: `sync_to_line_smooth(3,4)` / T: block index 1 → top 50 | a:frankmd-workspace::scroll_sync | keep |
| 75 | 1032 "prefers an explicit source line over the ratio-derived line" | Editor's top visible line beats the ratio | §5.21 | G: same doc / W: `sync_ratio(0.5, Some(1))` / T: top 0 | a:frankmd-workspace::scroll_sync | keep |
| 76 | 1050 "value=false blocks editor -> preview sync (ratio path)" | Sync off blocks ratio path | ToggleScrollSync §6.1 | G: sync off / W: `sync_ratio(0.5)` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 77 | 1058 "value=false blocks editor -> preview sync (typing path)" | Sync off blocks typing path | §6.1 | G: off / W: `sync_to_line_smooth(3,10)` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 78 | 1066 "value=false blocks preview -> editor sync" | Sync off blocks reverse path | §6.1 | G: off / W: `on_user_scroll` / T: None | a:frankmd-workspace::preview | keep |
| 79 | 1079 "value=false blocks typewriter sync" | Sync off blocks typewriter follow | §6.1 + §4.2 | G: off / W: `typewriter_target(5,10)` / T: None | a:frankmd-workspace::scroll_sync | changed (open question: should typewriter follow ignore the sync toggle?) |
| 80 | 1127 "renders task checkboxes enabled with per-item source lines" | Task boxes are interactive and know their source line | Task lists §5.21 | G: "# Title\n\n- [ ] open\n- [x] done\n" / W: `render` / T: List items `task Some(false)` line 3, `Some(true)` line 4; both focusable/clickable | a:frankmd-markdown::parse | keep |
| 81 | 1142 "dispatches preview:toggle-task with the item's line and reverts the visual toggle" | Clicking a task toggles the source line; displayed state always follows the source | §5.21 (FE-197) | G: rendered doc above / W: `activate(TaskHit(item 0))`, apply edit, re-render; same for item 1 / T: action `ToggleTask{line:3}` → source "- [x] open"; checkbox state comes from the new IR (no optimistic flip); item 1 → line 4 | a:frankmd-workspace::preview + a:frankmd-markdown::edit | keep |
| 82 | 1161 "ignores clicks on checkboxes outside an annotated list item" | A raw-HTML checkbox isn't a task | Raw HTML allow-list §5.21 | G: `<p><input type="checkbox" checked> stray</p>` / W: `render` / T: no `ListItem.task`; input not in allow-list → escaped text or dropped; no task hit target | a:frankmd-markdown::parse | changed (§5.21 allow-list: no stray inputs) |
| 83 | 1175 "ignores checkbox clicks when the li carries no source line" | A toggle that can't be mapped back to the source is ignored | §5.21 | G: rendered doc generation g; source edited so line 3 is no longer a task / W: `ToggleTask{line:3, generation:g}` / T: no edit (stale generation or line not a task) | a:frankmd-workspace::preview | changed (every IR item has a span; guard is now stale-generation) |

### `test/javascript/controllers/recovery_diff_controller.test.js` — C=7 (keep 7 / changed 0 / dropped 0; a=7 b=1 c=0)
Original subject: recovery dialog showing draft-vs-file word diff and resolution events. New home: Recovery dialog §5.11 (`workspace::recovery`, `Dialog(Recovery)` in `workspace::overlay`).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 47 "computes and renders diff between server and backup content" | Side-by-side word diff: removed words on the file side, added words on the draft side | Recovery §5.11 (FE-145) | G: file "hello world", draft "hello universe" / W: `RecoveryVm::open(record)` / T: file column [Equal "hello ", Del "world"]; draft column [Equal "hello ", Add "universe"] | a:frankmd-workspace::recovery | keep |
| 2 | 71 "formats and displays the backup timestamp" | The draft time is shown | Column header chips §5.11 | G: draft ts 2024-01-15 14:30 local, file mtime / W: open / T: draft chip non-empty, locale-formatted via i18n ("14:30"); cause sentence names both times | a:frankmd-workspace::recovery | keep |
| 3 | 85 "opens the dialog" | Opening a note with a differing draft blocks with the Recovery dialog | §5.11 | G: pending record for note / W: open-note flow / T: `OverlayStack.top == Dialog(Recovery)` | a:frankmd-workspace::overlay | keep |
| 4 | 98 "prevents Escape from dismissing recovery before an explicit choice" | Esc and backdrop don't dismiss Recovery | §5.11, §5 dialog rules | G: Recovery open / W: `escape_target(&ui)`; backdrop click / T: EscTarget::None for Recovery; dialog still top; b: press escape → `dialog-recovery` still visible | a:frankmd-workspace::overlay + b | keep |
| 5 | 117 "dispatches resolved event with source server and closes dialog" | "Use file on disk" resolves with the file and closes | Button "Use file on disk" §5.11 | G: record {path "test.md", draft_revision "draft-1", draft "backup", ts} / W: `choose(UseFile)` / T: `Resolution{choice:UseFile, path, draft_revision:Some("draft-1"), draft_content:"backup", draft_ts}`; overlay closed | a:frankmd-workspace::recovery | keep |
| 6 | 146 "dispatches resolved event with source backup and content, then closes" | "Restore draft" puts the draft into the editor and closes | Button "Restore draft" §5.11 | G: record draft "my backup content", no revision / W: `choose(RestoreDraft)` / T: `Resolution{RestoreDraft, content:"my backup content", draft_revision:None, draft_ts}`; closed | a:frankmd-workspace::recovery | keep |
| 7 | 173 "includes the selected conflict identity so recovery cannot consume another draft" | Resolving one record only affects that record | §5.11 list of pending records (FE-144) | G: two records; selected {kind DraftConflict, path "new.md", draft_revision "draft-2", conflict_id "conflict-1"} / W: `choose(RestoreDraft)` / T: resolution echoes kind, revision and conflict_id; the other record stays pending | a:frankmd-workspace::recovery | keep |

### `test/javascript/controllers/s3_option_controller.test.js` — C=22 (keep 8 / changed 12 / dropped 2; a=20 b=0 c=0)
Original subject: per-dialog "Upload to S3" checkbox with resize ratio and destination key. New home: Insert image footer storage row (§5.6): provider select (Local/S3, S3 disabled with reason when unconfigured, FE-355), resize select, destination key (`frankmd/YYYY/MM/…`). VM `workspace::insert_image::StorageRow`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 48 "hides resize option on connect" | Resize isn't offered while storing locally | Storage row §5.6 | G: global provider Local / W: `StorageRow::new(&media_storage)` / T: provider Local; resize control disabled | a:frankmd-workspace::insert_image | keep |
| 2 | 54 "removes hidden class from element" | S3 offered when configured | §5.6 | G: s3_available=true / W: build row / T: S3 option enabled | a:frankmd-workspace::insert_image | changed (§5.6 provider select replaces show/hide of the S3 control) |
| 3 | 62 "adds hidden class to element" | S3 not offered when unconfigured | §5.6 FE-355 | G: s3_available=false / W: build row / T: S3 option disabled with reason | a:frankmd-workspace::insert_image | changed (§5.6 disabled-with-reason, not hidden) |
| 4 | 67 "resets checkbox and resize option" | Losing S3 resets the row | §5.6 | G: row on S3, resize 0.75 / W: s3 becomes unavailable / T: provider Local, resize back to 0.5, resize disabled | a:frankmd-workspace::insert_image | changed (§5.6 select) |
| 5 | 81 "shows resize option when checkbox is checked" | Picking S3 offers resize | §5.6 | G: Local / W: `set_provider(S3)` / T: resize enabled | a:frankmd-workspace::insert_image | changed (§5.6 select replaces checkbox) |
| 6 | 86 "hides resize option when checkbox is unchecked" | Back to Local hides resize | §5.6 | G: S3 / W: `set_provider(Local)` / T: resize disabled | a:frankmd-workspace::insert_image | changed (§5.6) |
| 7 | 92 "resets resize select to default when unchecked" | Resize resets on leaving S3 | §5.6 | G: S3, resize 0.75 / W: `set_provider(Local)` / T: resize 0.5 | a:frankmd-workspace::insert_image | changed (§5.6) |
| 8 | 100 "returns true when checkbox is checked" | Insert uses S3 when chosen | §5.6 | G: provider S3 / W: `uses_s3()` / T: true | a:frankmd-workspace::insert_image | changed (§5.6) |
| 9 | 105 "returns false when checkbox is unchecked" | Local otherwise | §5.6 | G: Local / W: `uses_s3()` / T: false | a:frankmd-workspace::insert_image | changed (§5.6) |
| 10 | 112 "returns empty string when checkbox is unchecked" | No resize sent for local | §5.6 | G: Local / W: `resize_ratio()` / T: None | a:frankmd-workspace::insert_image | changed (§5.6) |
| 11 | 117 "returns select value when checkbox is checked" | Chosen ratio is sent for S3; options 25/50/75/100 % | §5.6 | G: S3, resize 0.75 / W: `resize_ratio()`; `resize_options()` / T: Some(0.75); [0.25, 0.5, 0.75, 1.0] | a:frankmd-workspace::insert_image | keep |
| 12 | 125 "reveals the key option and pre-fills the default key when checked" | S3 shows the destination key prefilled `frankmd/YYYY/MM/<file>` | Key field §5.6 (FE-315/316) | G: selected "photo.png", Clock 2026-07 / W: `set_default_key("frankmd/2026/07/photo.png")`, `set_provider(S3)` / T: key field shown, value "frankmd/2026/07/photo.png" | a:frankmd-workspace::insert_image | keep |
| 13 | 133 "keyValue returns the trimmed input only when checked" | Key used only for S3, trimmed | §5.6 | G: key "  blog/hero.png  " / W: `key_value()` with Local then S3 / T: None; Some("blog/hero.png") | a:frankmd-workspace::insert_image | keep |
| 14 | 142 "does not clobber a manual edit when the selection changes" | User-edited key survives a new selection | §5.6 | G: S3, default "…/a.png" / W: user types "custom/place.png"; select b.png (`set_default_key("…/b.png")`) / T: key "custom/place.png" | a:frankmd-workspace::insert_image | keep |
| 15 | 155 "hides the key option and clears the input on reset" | Reopening the dialog starts with an empty key | §5.6 | G: key "blog/hero.png" / W: `reset()` (dialog reopen) / T: key hidden (provider Local), value "" | a:frankmd-workspace::insert_image | keep |
| 16 | 166 "unchecks the checkbox" | Reset returns to the default store | §5.6 + Settings › Media §5.16 | G: row on S3, global default Local / W: `reset()` / T: provider == global "Storage for new media" (Local) | a:frankmd-workspace::insert_image | changed (reset goes to the global default, not "unchecked") |
| 17 | 172 "hides resize option" | Reset hides resize | §5.6 | G: S3 / W: `reset()` (default Local) / T: resize disabled | a:frankmd-workspace::insert_image | keep |
| 18 | 178 "resets resize select to default value" | Reset resize to 0.5 | §5.6 | G: resize 1.0 / W: `reset()` / T: resize 0.5 | a:frankmd-workspace::insert_image | keep |
| 19 | 214 "handles missing targets gracefully on connect" | — (optional Stimulus targets) | — | — | — | dropped (DOM-specific) |
| 20 | 218 "isChecked returns false when checkbox target is missing" | No storage row → local insert | §5.6 sources that don't upload (Web/Library reuse) | G: `StorageRow::default()` / W: `uses_s3()` / T: false | a:frankmd-workspace::insert_image | changed (missing DOM target → default row) |
| 21 | 222 "resizeRatio returns empty string when targets are missing" | No row → no resize | §5.6 | G: default row / W: `resize_ratio()` / T: None | a:frankmd-workspace::insert_image | changed (default row) |
| 22 | 226 "reset handles missing targets gracefully" | — (null-target guard) | — | — | — | dropped (DOM-specific) |

### `test/javascript/controllers/scroll_sync_controller.test.js` — C=33 (keep 33 / changed 0 / dropped 0; a=33 b=0 c=0)
Original subject: editor ⇄ preview scroll coordinator with a single 400 ms lock. New home: `workspace::scroll_sync::ScrollSync` (architecture §3.8, §5.21 last paragraph, §4.2 typewriter), injected Clock; views apply returned intents to their own pane. Rows 32 and 33 get a thin wiring note but stay level a.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 83 "syncs preview scroll ratio with the editor's top visible line" | Editor scroll positions the preview at the editor's top line | §5.21 | G: preview visible, editor top line 7 / W: `on_editor_scroll(ratio 0.5, top_line 7)` / T: `PreviewIntent::Ratio(0.5, Some(7))` | a:frankmd-workspace::scroll_sync | keep |
| 2 | 91 "syncs the preview to the cursor line while Typewriter mode is enabled" | In typewriter the preview follows the caret, not the ratio | §4.2 | G: typewriter on, caret 8/24 / W: `on_editor_scroll` / T: `PreviewIntent::Typewriter(8,24)`; no ratio intent | a:frankmd-workspace::scroll_sync | keep |
| 3 | 100 "passes null line when the editor lacks line info (ratio fallback)" | Missing line info falls back to ratio | §5.21 | G: top_line None / W: `on_editor_scroll(0.5)` / T: `Ratio(0.5, None)` | a:frankmd-workspace::scroll_sync | keep |
| 4 | 108 "does not sync when preview is not visible" | Hidden preview never follows (and isn't opened) | §4.2, §11 typewriter | G: preview hidden / W: `on_editor_scroll` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 5 | 117 "does not sync when scroll source is preview (feedback loop prevention)" | No bounce while preview leads | §5.21 | G: lock Preview / W: `on_editor_scroll` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 6 | 125 "marks scroll source as editor" | Editor scroll takes the lock | §5.21 | G: idle / W: `on_editor_scroll` / T: lock Editor | a:frankmd-workspace::scroll_sync | keep |
| 7 | 133 "syncs editor scroll position based on ratio" | Preview scroll moves the editor | §5.21 | G: editor max 500 / W: `on_preview_scroll(0.5, None)` / T: `EditorIntent::Offset(250)` | a:frankmd-workspace::scroll_sync | keep |
| 8 | 141 "does not sync when scroll source is editor (feedback loop prevention)" | No bounce while editor leads | §5.21 | G: lock Editor / W: `on_preview_scroll` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 9 | 152 "scrolls to top when scrollRatio is near 0" | Snap editor to top | §5.21 | G: editor top 100, max 500 / W: `on_preview_scroll(0.005)` / T: Offset(0) | a:frankmd-workspace::scroll_sync | keep |
| 10 | 165 "scrolls to bottom when scrollRatio is near 1" | Snap editor to bottom | §5.21 | G: max 500 / W: `on_preview_scroll(0.995)` / T: Offset(500) | a:frankmd-workspace::scroll_sync | keep |
| 11 | 177 "uses line-anchored scrollToLine when sourceLine is available" | Preview's top line drives the editor | §5.21 | G: idle / W: `on_preview_scroll(0.5, Some(50), total 100)` / T: `EditorIntent::Line(50)`; no offset intent | a:frankmd-workspace::scroll_sync | keep |
| 12 | 186 "does not scroll when maxScroll is 0" | Non-scrollable editor isn't moved | §5.21 | G: editor height == viewport / W: `on_preview_scroll(0.5)` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 13 | 198 "marks scroll source as preview" | Preview scroll takes the lock | §5.21 | G: idle / W: `on_preview_scroll` / T: lock Preview | a:frankmd-workspace::scroll_sync | keep |
| 14 | 208 "marks the single lock as editor-initiated" | Programmatic preview scroll counts as editor-led | §5.21 | G: idle / W: `on_programmatic_preview_scroll()` / T: lock Editor | a:frankmd-workspace::scroll_sync | keep |
| 15 | 214 "typing-path programmatic scroll blocks late preview echo from re-scrolling the editor" | Smooth-scroll tail doesn't move the editor | §5.21 | G: after programmatic scroll / W: `on_preview_scroll(0.5)` / T: None | a:frankmd-workspace::scroll_sync | keep |
| 16 | 227 "genuine user scroll after the lock expires syncs the editor" | Lock expires at 400 ms | §5.21 | G: programmatic at t0 / W: advance 450 ms; `on_preview_scroll(0.5)` / T: lock None before the call; Offset intent returned | a:frankmd-workspace::scroll_sync | keep |
| 17 | 243 "editor scroll does not trigger reverse sync" | Editor scroll then preview echo → no bounce | §5.21 | G: idle / W: `on_editor_scroll`; `on_preview_scroll(0.3)` / T: second returns None | a:frankmd-workspace::scroll_sync | keep |
| 18 | 255 "scroll source clears after timeout" | Lock clears 400 ms after last mark | §5.21 | G: editor scroll t0 / W: advance 450 ms / T: lock None (at 399 ms still Editor) | a:frankmd-workspace::scroll_sync | keep |
| 19 | 267 "renders content without sync in normal mode" | Typing re-renders without moving the preview | §5.21 | G: visible, typewriter off / W: `on_content_changed(src)` / T: Render(src) only, no scroll intent | a:frankmd-workspace::scroll_sync | keep |
| 20 | 276 "uses updateWithSync in typewriter mode" | Typing in typewriter re-renders and follows the caret | §4.2 | G: typewriter on / W: `on_content_changed(src, caret)` / T: `RenderWithSync{src, line, total}` | a:frankmd-workspace::scroll_sync | keep |
| 21 | 285 "does nothing when preview is not visible" | Hidden preview isn't rendered while typing | §4.2 | G: hidden / W: `on_content_changed` / T: nothing | a:frankmd-workspace::scroll_sync | keep |
| 22 | 296 "renders content via preview controller" | Explicit refresh renders | §5.21 | G: visible / W: `refresh_preview()` / T: Render | a:frankmd-workspace::scroll_sync | keep |
| 23 | 302 "does nothing when preview is not visible" | Hidden → no render | §4.2 | G: hidden / W: `refresh_preview()` / T: nothing | a:frankmd-workspace::scroll_sync | keep |
| 24 | 313 "updates preview and syncs scroll with the editor's top line when preview becomes visible" | Opening the preview lands where the editor is | TogglePreview §4.2 | G: editor top line 9, ratio 0.5 / W: `on_preview_toggled(true)` / T: Render + `Ratio(0.5, Some(9))` | a:frankmd-workspace::scroll_sync | keep |
| 25 | 322 "does nothing when preview is hidden" | Hiding doesn't render or sync | §4.2 | G: visible / W: `on_preview_toggled(false)` / T: nothing | a:frankmd-workspace::scroll_sync | keep |
| 26 | 329 "holds the lock for the re-sync echo, then clears it so user scroll isn't swallowed" | Toggle re-sync echo ignored; user scroll right after works | §4.2 | G: toggled visible / W: echo `on_preview_scroll`; `frame_tick()` ×2; `on_preview_scroll` / T: echo → None, lock Editor; after two frames lock None (< 400 ms); then Offset intent | a:frankmd-workspace::scroll_sync | keep |
| 27 | 352 "tracks typewriter mode state" | Typewriter toggle changes sync behaviour | §4.2 | G: off / W: `set_typewriter(true)` then false / T: flag follows; intents switch Typewriter ↔ Ratio | a:frankmd-workspace::scroll_sync | keep |
| 28 | 362 "re-centers the preview on the cursor line when Typewriter mode is active" | Relayout in typewriter keeps the caret block at top | §4.2 | G: typewriter on, lock Editor, caret 8/24 / W: `reanchor(Reason::Zoom)` / T: Typewriter(8,24), no Ratio | a:frankmd-workspace::scroll_sync | keep |
| 29 | 372 "zoom change re-anchors the preview to the editor's top line (editor lock held)" | Zoom keeps the preview aligned to the editor | Preview zoom §3.5 | G: lock Editor, editor top 7 / W: `reanchor(Zoom)` / T: `Ratio(0.5, Some(7))` | a:frankmd-workspace::scroll_sync | keep |
| 30 | 381 "zoom change re-anchors the editor to the preview's top line (preview lock held)" | Zoom while reading the preview keeps the editor aligned | §3.5 | G: lock Preview, preview top line 12 / W: `reanchor(Zoom)` / T: `EditorIntent::Line(12)` | a:frankmd-workspace::scroll_sync | keep |
| 31 | 390 "zoom change falls back to ratio when the preview has no line annotations" | Without lines, ratio is used | §3.5 | G: lock Preview, no top line, preview 250/1000/500, editor max 500 / W: `reanchor(Zoom)` / T: Offset(250) | a:frankmd-workspace::scroll_sync | keep |
| 32 | 406 "debounced window resize re-anchors both panes" | Window resize keeps panes aligned, debounced 200 ms | Resize §4.1 | G: editor top 3, Clock / W: `on_window_resized()`; check at 0 ms and 250 ms / T: nothing immediately; `Ratio(0.5, Some(3))` after 200 ms (wiring of resize event → VM via `simulate_window_resize` is covered by layout b tests) | a:frankmd-workspace::scroll_sync | keep |
| 33 | 417 "ResizeObserver on preview content re-anchors while a lock is held" | Preview reflow (image loaded, math rendered) keeps alignment | §5.21 images/math | G: lock Editor, editor top 7 / W: `on_preview_relayout()` (block heights changed) / T: `Ratio(0.5, Some(7))`; with no lock → nothing | a:frankmd-workspace::scroll_sync | keep |

#### New VM requirements (G7)
- `frankmd-workspace::media_storage` (new) — `MediaStorageState { acknowledged: Provider, pending: Option<Provider>, s3_available: bool, error: Option<String>, fallback_announced: bool }`; `request(p)`, `ack(Result<ConfigSnapshot, Error>)`, `apply(snapshot)` (with `fallback` flag), `refresh(known_fallback)`, `ready() -> Gate` (uploads wait; Err(t(`media_storage.save_failed`)) on failure); emits `BannerKind::StorageFallback` at most once per session; shared by settings row and image footer (media_storage rows 1–8, menu rows 1–3, s3 rows 1–4, 16).
- `frankmd-workspace::settings_vm` — the `media_storage_provider` row binds to `MediaStorageState` (disabled while pending, option-level `disabled_reason`, inline error) and saves immediately rather than through the 300 ms debounce (menu rows 1–3).
- `frankmd-workspace::banners` — `BannerKind::StorageFallback` with Settings / Dismiss; dismiss is sticky for the session (media_storage rows 4–6).
- `frankmd-workspace::insert_image::StorageRow` — `provider`, `set_provider`, S3 option `disabled_reason`, `resize: f32` (options [0.25, 0.5, 0.75, 1.0], default 0.5, enabled only for S3), `set_default_key`, `on_key_edited` (sticky manual edit), `key_value() -> Option<String>` trimmed, S3-only, `reset()` to the global default, `uses_s3()`, `resize_ratio() -> Option<f32>`, `Default` (s3 rows 1–22).
- `frankmd-workspace::pane_header` (new) — `PaneHeaderVm { full_path, breadcrumb: Vec<Segment>, title (stem, literal text), tooltip, a11y_label (i18n "Rename document: {name}") }`; `breadcrumb(path, WidthClass)` with `…` collapse below 720 px; `window_title(note, vault)` (path_display rows 2–11).
- `frankmd-workspace::commands` — CopyPath via an injected `Clipboard` trait (fake in tests); availability Unavailable without an open note (path_display rows 6–7).
- `frankmd-workspace::status` — "Copied to clipboard" info message with a 2 s TTL, mirrored to the Diagnostics message log (path_display row 8).
- `frankmd-workspace::preview` (new) — `PreviewVm { zoom, typewriter, sync_enabled, last_rendered: Option<Arc<str>>, settle_until (100 ms after last render), debounce 150 ms, last_synced_line, generation }`; `content_changed(src, caret)`, `on_rendered(doc)`, `on_user_scroll(PreviewMetrics) -> Option<PreviewScrolled>`, `zoom_in/out` effects, `text_px(zoom)` (= zoom × 15 px / 100), `header_label()`, `bottom_padding_px()`, `activate(Hit) -> PreviewAction {OpenImageViewer{alt, src, return_focus}, OpenLink(url), ToggleTask{line, generation}}`; stale-generation guard (preview rows 1–83).
- `frankmd-workspace::scroll_sync` — `ScrollSync { lock: Option<ScrollSource>, lock_until (400 ms), typewriter, sync_enabled, preview_visible, pending_resize (200 ms) }`; `on_editor_scroll`, `on_preview_scroll`, `on_programmatic_preview_scroll`, `on_preview_toggled`, `on_content_changed`, `refresh_preview`, `reanchor(Reason)`, `on_window_resized`, `on_preview_relayout`, `frame_tick` (clears the toggle lock after 2 frames); pure fns `ratio_to_offset` (snap ≤0.01/≥0.99), `preview_ratio`, `sync_ratio(ratio, Option<line>, &LineMap, &PreviewMetrics)` (round(ratio×total)+1, anchor at top), `sync_to_line` (smooth, (line−1)/(total−1)), `sync_to_line_smooth` (anchor − 50 px, element-count fallback), `typewriter_target` (anchor at top, ratio×content fallback); `ScrollIntent {top_px, animate}` (animate false under reduced motion) (preview rows 26–79, scroll_sync rows 1–33).
- `frankmd-workspace::layout` — `set_preview_visible` / `toggle_preview` return `Option<Effect::PreviewToggled{visible}>` (none when unchanged); per-vault persisted visibility (preview rows 5–14).
- `frankmd-workspace::recovery` — `RecoveryVm::diff() -> (Vec<Seg>, Vec<Seg>)` word diff with Equal/Del/Add (file left, draft right); timestamp chips through i18n; `choose(UseFile|KeepBoth|RestoreDraft) -> Resolution { choice, path, content, draft_content, draft_revision, draft_ts, kind, conflict_id }`; resolving removes only that record (recovery rows 1–7).
- `frankmd-workspace::overlay` — `escape_target` returns None for `Dialog(Recovery)`; backdrop ignored (recovery row 4). Select popover inside Settings is above Settings in the Esc chain (menu row 4).
- `frankmd-markdown::parse` — `total_lines` set (1 for "# Hello World"); raw-HTML allow-list maps `<img>` (attrs filtered to src/alt/title) and `<picture>` (single Image, `<img>` src) and never emits form inputs; task `ListItem.span.start_line` is absolute (front matter counted) (preview rows 2, 4, 16, 73, 80, 82).
- `frankmd-markdown::linemap` — `LineMap` must expose block tops by index for the element-count fallback when spans are missing (preview row 74).
- `frankmd-markdown::edit::toggle_task_at_line(text, line) -> Option<Edit>` — None when the line isn't a task (preview rows 81, 83).

#### Open questions (G7)
- Storage fallback (§5.12): the old dialog auto-closed after 6 s; the banner has Settings/Dismiss and no timeout. Confirm the banner never auto-dismisses.
- Settings › Media provider row: §5.16 says changes save with a 300 ms debounce, but the old contract saves immediately and gates uploads on the ack. Plan assumes immediate save for this row.
- Storage row resize (§5.6 mock shows "Resize: none ▾" next to a Local provider): does resize also apply to local storage, and is the default "none" or the old 0.5? Plan keeps old behaviour (S3-only, default 0.5).
- Should the per-dialog storage row reset to the global "Storage for new media" value (assumed) or always to Local (old)?
- Linked images in the preview: §5.21 says images open the viewer on Enter/click and links open in the browser; with an image inside a link, which wins on click, and is there a visible "enlarge" control? Plan keeps old behaviour (click follows link, separate enlarge control).
- Typewriter follow when scroll sync is off (preview row 79): old code blocks it; §4.2 doesn't say. Kept as blocked.
- Default preview visibility for a vault with no saved state (preview row 13): §4.2 persists per vault but gives no first-run default.
- Copy path with no note: palette hides it (§5.1); should the Document menu even exist without a note (empty state)?

## 13. G8: settings, stats_panel, table_editor, text_format, theme, typewriter, video_dialog

### `test/javascript/controllers/settings_controller.test.js` — C=58 (keep 38 / changed 19 / dropped 1; a=56 b=1 c=0)
Original subject: Settings modal controller (font/size/width/line numbers/stats/indent/zoom/view mode/theme, debounced PATCH /config, reload) + AppController Settings overlay bits. New home: Settings sheet §5.16 (`settings-sheet`), `workspace::settings_vm`, `workspace::layout`, `workspace::overlay`, theme §5.17, status line §3.6.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 150 "applies editor width via CSS custom property" | Text column uses `editor_width` ch from startup | Editor pane §3.1 | G: config editor_width=72 / W: workspace boots / T: `resolve_panes(..).text_column_ch == 72` | a:frankmd-workspace::layout | keep |
| 2 | 154 "does not throw during initialization (outlets not connected)" | Settings load with no note/preview open never fails | §5.16 | G: no note open, preview hidden / W: SettingsVm::from_config + apply all / T: no panic; values stored | a:frankmd-workspace::settings_vm | keep |
| 3 | 158 "initializes readiness flags to false" | — (Stimulus outlet readiness flags) | — | — | — | dropped (DOM-specific) |
| 4 | 165 "fontValueChanged skips applyFont when codemirror not ready" | Changing editor font with no editor stores the value; applied when a note opens | Settings › Editor §5.16 | G: empty state / W: set editor_font=fira-code / T: no editor effect emitted; later open_note → EditorConfig.font == fira-code | a:frankmd-workspace::settings_vm | keep |
| 5 | 171 "fontSizeValueChanged skips applyFont when codemirror not ready" | Same for font size | §5.16 | G: empty state / W: set editor_font_size=16 / T: open_note → EditorConfig.font_size 16 | a:frankmd-workspace::settings_vm | keep |
| 6 | 177 "lineNumbersValueChanged skips applyLineNumbers when codemirror not ready" | Same for line numbers | §5.16, §6.2 Ctrl+L | G: empty state / W: set editor_line_numbers=2 / T: open_note → EditorConfig.line_numbers Relative | a:frankmd-workspace::settings_vm | keep |
| 7 | 183 "previewZoomValueChanged skips applyPreviewZoom when preview not ready" | Zoom set while preview hidden takes effect when it shows | Preview header §3.5 | G: preview hidden / W: set preview_zoom=150 / T: layout.preview_zoom 150; TogglePreview → preview renders at 150 | a:frankmd-workspace::layout | keep |
| 8 | 189 "scrollSyncValueChanged skips applyScrollSync when preview not ready" | Scroll sync set while preview hidden is remembered | §3.5, §3.6 | G: preview hidden / W: scroll_sync=false / T: stored false; preview shown with sync off; status `unlink` indicator | a:frankmd-workspace::layout | keep |
| 9 | 195 "editorWidthValueChanged applies without outlets (CSS-only)" | Text width applies immediately, no note needed | §3.1 | G: empty state / W: editor_width=80 / T: text_column_ch 80 | a:frankmd-workspace::layout | keep |
| 10 | 201 "themeValueChanged dispatches event without outlets" | Choosing a theme in Settings applies it at once | §5.16 Appearance, §5.17 | G: settings open / W: set theme=nord / T: ThemeChanged{nord} emitted; active palette id nord | a:frankmd-workspace::settings_vm | keep |
| 11 | 214 "codemirrorOutletConnected sets _codemirrorReady and applies settings" | A newly opened editor gets font, line numbers, recenter-on-click from settings | Editor §3.1 | G: settings fira-code/16/2/recenter on / W: open_note / T: EditorConfig carries all four | a:frankmd-workspace::settings_vm | keep |
| 12 | 227 "previewOutletConnected sets _previewReady and applies zoom and scroll sync" | Preview, when shown, uses current zoom and sync | §3.5 | G: zoom 150, sync off / W: TogglePreview / T: preview state zoom 150, sync off | a:frankmd-workspace::layout | keep |
| 13 | 238 "fontValueChanged calls applyFont AFTER codemirror is ready" | Font change applies live to the open editor | §5.16 "Changes apply live" | G: note open / W: set editor_font=hack / T: EditorConfig updated, editor re-rendered event | a:frankmd-workspace::settings_vm | keep |
| 14 | 247 "previewZoomValueChanged calls applyPreviewZoom AFTER preview is ready" | Zoom change applies live to visible preview | §3.5 | G: preview visible / W: set zoom 125 / T: preview_zoom 125, header shows 125% | a:frankmd-workspace::layout | keep |
| 15 | 258 "sets CSS custom property from value" | Width 100 → 100ch column | §3.1 | G: editor_width=100 / W: resolve_panes / T: text_column_ch 100 | a:frankmd-workspace::layout | keep |
| 16 | 266 "dispatches config-changed event with theme" | Theme setting propagates to theme engine | §5.17 | G: settings_vm / W: apply theme=dracula-like id / T: ThemeChanged{id} emitted once | a:frankmd-workspace::settings_vm | keep |
| 17 | 277 "does not dispatch event when theme is empty" | Empty theme means default, not a theme switch | §5.1 Theme order (Follow Omarchy/system default) | G: theme="" / W: apply / T: no explicit-theme event; resolves to ThemeChoice::FollowDefault | a:frankmd-workspace::settings_vm | changed (§5.1 Follow Omarchy default) |
| 18 | 289 "flips the preview controller's syncScrollEnabledValue" | Scroll-sync setting and preview header toggle are one state | §3.5, §6.1 Ctrl+Shift+\\ | G: preview visible, sync on / W: set scroll_sync=false / T: layout.scroll_sync false; header toggle unpressed; status `unlink` shown | a:frankmd-workspace::layout | keep |
| 19 | 304 "does nothing when the preview outlet is unavailable" | Toggling sync with preview hidden is safe | §3.5 | G: preview hidden / W: ToggleScrollSync / T: value flips, no panic | a:frankmd-workspace::layout | keep |
| 20 | 315 "applies family and size to the codemirror controller" | Font id maps to a family; size applied | §7.2, §5a fonts | G: editor_font=fira-code, size 16 / W: EditorConfig::from_settings / T: family "Fira Code" (fontconfig name), size 16 | a:frankmd-workspace::settings_vm | keep |
| 21 | 329 "exposes currentFont" | Default editor font | §5.16 Editor, §7.2 | G: empty config / W: defaults / T: editor_font == System monospace (absent key); explicit cascadia-code still honored | a:frankmd-workspace::settings_vm | changed (§7.2 default System monospace) |
| 22 | 333 "exposes currentFontSize" | Default size 14 | §7.2 | G: empty config / T: editor_font_size 14 | a:frankmd-workspace::settings_vm | keep |
| 23 | 337 "exposes editorWidth" | Default width 72 | §3.1 | G: empty config / T: editor_width 72 | a:frankmd-workspace::settings_vm | keep |
| 24 | 341 "exposes previewZoom" | Default zoom 100 | §3.5 | G: empty config / T: preview_zoom 100 | a:frankmd-workspace::settings_vm | keep |
| 25 | 345 "exposes lineNumberMode as numeric value" | Line numbers default Off; numeric 0/1/2 mapping | §5.16 | G: editor_line_numbers 0/1/2 / T: Off/Absolute/Relative; default Off | a:frankmd-workspace::settings_vm | keep |
| 26 | 350 "exposes typewriterModeEnabled" | Typewriter default off | §4.2 | G: empty config / T: typewriter false | a:frankmd-workspace::settings_vm | keep |
| 27 | 354 "exposes scrollSyncEnabled (default true)" | Scroll sync default on | §3.5 | G: empty config / T: scroll_sync true | a:frankmd-workspace::settings_vm | keep |
| 28 | 358 "exposes fonts list" | Font select offers the known ids | §5.16 Editor | G: settings_vm / W: font options / T: [System monospace] + 9 bundled ids in FrankMD order, labels kept | a:frankmd-workspace::settings_vm | changed (§5.16 adds System monospace) |
| 29 | 366 "opens About within Settings and returns to an editable category" | About is a category inside Settings; switching back works | §5.16 About | G: settings open / W: select About then Editor / T: only About rows visible; then Editor rows; sheet stays open | a:frankmd-workspace::settings_vm | changed (§5.16 categories) |
| 30 | 378 "switches the visible section and nav aria-pressed state" | Selecting a category shows only it and marks it selected | §5.16 | G: Appearance selected / W: click category Editor / T: vm.visible_rows all Editor; b: category row a11y selected | a:frankmd-workspace::settings_vm + b | changed (§5.16 categories) |
| 31 | 392 "switches back to general" | Switching back restores first category | §5.16 | G: Editor selected / W: select Appearance / T: Appearance rows visible | a:frankmd-workspace::settings_vm | changed (§5.16 Appearance replaces General) |
| 32 | 402 "opens modally, syncs controls, focuses navigation, and reports its state" | Ctrl+, opens the sheet reflecting current values with focus inside | §5.16, §4.3 | G: editor focused, font fira-code / W: press ctrl-, / T: `settings-sheet` visible; font row value fira-code; focus inside sheet (see OQ); overlay = Settings | a:frankmd-workspace::overlay + b | keep |
| 33 | 416 "closes through the dialog close method and reports its state" | Closing reports closed and restores focus | §4.3, §5 intro | G: settings open from editor / W: Esc / T: overlay None; editor focused | a:frankmd-workspace::overlay + b | keep |
| 34 | 428 "closes on backdrop clicks but ignores clicks inside the dialog" | Click inside keeps open; backdrop closes | §5 intro backdrop rule | G: settings open / W: click category row, then scrim / T: still open after first; closed after second | b | keep |
| 35 | 440 "does nothing when asked to close while already closed" | Closing a closed sheet is a no-op | §5 intro | G: no overlay / W: overlay.close(Settings) / T: returns false; focus unchanged | a:frankmd-workspace::overlay | keep |
| 36 | 465 "font change applies live and PATCHes editor_font" | Font change applies live and persists after debounce | §5.16 (300 ms, scope) | G: FakeClock, scope This vault / W: set editor_font=fira-code, advance 300 ms / T: EditorConfig live; one write `editor_font = fira-code` to .fed | a:frankmd-workspace::settings_vm | changed (§5.16 300 ms debounce, scope) |
| 37 | 477 "font size change applies live and PATCHes editor_font_size" | Same for size | §5.16 | G: FakeClock / W: size 16, advance / T: write editor_font_size=16 (int) | a:frankmd-workspace::settings_vm | changed (§5.16 300 ms) |
| 38 | 489 "width input applies live and PATCHes editor_width" | Width live + readout + persisted | §5.16 Text width | G: FakeClock / W: width 96 / T: text_column_ch 96; row readout "96ch"; write editor_width=96 | a:frankmd-workspace::settings_vm | changed (§5.16 300 ms) |
| 39 | 502 "line numbers change applies live and PATCHes editor_line_numbers" | Line numbers live + persisted | §5.16 | G: note open / W: select Relative / T: EditorConfig Relative; write editor_line_numbers=2 | a:frankmd-workspace::settings_vm | changed (§5.16 300 ms) |
| 40 | 514 "enables Stats only when requested and persists the setting" | Show stats on/off shows/hides stats and persists | §3.6 Stats slot, §5.16 Writing | G: note "text" open, show_stats off / W: toggle on, advance; toggle off / T: status.stats Some (computed now); write show_stats=true; then stats slot hidden | a:frankmd-workspace::status | changed (§3.6 status-line stats replace panel) |
| 41 | 538 "cursor recenter toggle applies live and PATCHes cursor_recenter_on_click" | Recenter switch live + persisted | §5.16 Writing | G: note open / W: toggle / T: EditorConfig.recenter_on_click true; switch value true; write cursor_recenter_on_click=true | a:frankmd-workspace::settings_vm | changed (§5.16 300 ms) |
| 42 | 553 "indent change PATCHes editor_indent" | Indent saved and now applied | §5.16, §11 indent applied | G: note open / W: indent 4 / T: write editor_indent=4; EditorConfig.indent_unit 4 spaces | a:frankmd-workspace::settings_vm | changed (§11 indent now applied) |
| 43 | 564 "zoom change applies live and PATCHes preview_zoom" | Zoom live + persisted | §5.16 Preview | G: preview visible / W: zoom 150 / T: layout.preview_zoom 150; write preview_zoom=150 | a:frankmd-workspace::settings_vm | changed (§5.16 300 ms) |
| 44 | 578 "view mode change applies live and PATCHes view_mode" | Layout Split/Single live + persisted | §4.2, §5.16 Preview | G: wide, split / W: choose Single / T: layout.view_mode Single; write view_mode="single" | a:frankmd-workspace::settings_vm | changed (§5.16 300 ms) |
| 45 | 592 "view mode segmented buttons sync their aria-pressed state" | Segmented control reflects current mode both ways | §5.16 | G: view_mode Single then Split / T: segment selected matches; b: segment a11y selected | a:frankmd-workspace::settings_vm + b | keep |
| 46 | 613 "announces successful saves via frankmd:config-file-modified" | After a successful write, dependents (Diagnostics config sources, other listeners) refresh | §5.16 Diagnostics | G: FakeClock, fake writer Ok / W: indent 4, advance / T: ConfigWritten event once; write failure instead → inline row error, value reverted | a:frankmd-workspace::settings_vm | keep |
| 47 | 627 "refreshes shared storage instead of applying a potentially stale settings response" | Re-reading config refreshes media storage state freshly, never from a stale snapshot | §5.12 storage fallback, §5.16 Media | G: reload result with media_storage_fallback / W: settings_vm.reload / T: MediaStorageState::refresh(force) requested; snapshot not applied | a:frankmd-workspace::media_storage | changed (no server; direct state refresh) |
| 48 | 637 "re-syncs values from GET /config and updates the controls" | External config change updates every value and control | §5.16 | G: config file with 13 keys changed / W: reload / T: every row value matches (width readout "120ch", select values, toggles, view mode Single) | a:frankmd-workspace::settings_vm | keep |
| 49 | 687 "normalizes an unknown view_mode to split on reload" | Bad view_mode tolerated | §4.2 | G: view_mode="diagonal" / W: reload / T: Split | a:frankmd-workspace::settings_vm | keep |
| 50 | 698 "treats a missing scroll_sync as unchanged (stays enabled)" | Missing key keeps current value | §3.5 | G: sync on; reload without scroll_sync / T: still on | a:frankmd-workspace::settings_vm | keep |
| 51 | 709 "swallows fetch failures" | Unreadable config on reload keeps current values silently | §5.16, §5.19 | G: reader returns Err / W: reload / T: Ok(()) no panic; values unchanged; error logged to Diagnostics | a:frankmd-workspace::settings_vm | keep |
| 52 | 724 "updates the font preview styling" | Live font preview box shows chosen family/size | §5.16 Editor (FE-484) | G: jetbrains-mono, 20 / W: font_preview() / T: family contains "JetBrains Mono", size 20 | a:frankmd-workspace::settings_vm | keep |
| 53 | 735 "syncs controls so the dialog reflects current values" | Opening Settings shows current values | §5.16 | G: font changed via palette while closed / W: OpenSettings / T: font row value fira-code | a:frankmd-workspace::settings_vm | keep |
| 54 | 803 "opens over the editor without changing preview visibility (%s)" [row 1] | Settings overlays editor; preview (hidden) untouched; Ctrl+, again closes | §5.16, §2 overlays | G: preview hidden / W: OpenSettings, OpenSettings, then open again / T: overlay Settings; preview_visible false throughout; second Ctrl+, closes; editor intact | a:frankmd-workspace::overlay | changed (§5.16 Ctrl+, closes) |
| 55 | 803 "opens over the editor without changing preview visibility (%s)" [row 2] | Same with preview visible | §5.16 | G: preview visible / W: same / T: preview_visible true before/during/after | a:frankmd-workspace::overlay | changed (§5.16 Ctrl+, closes) |
| 56 | 828 "can overlay the Library without changing its workspace state" | Settings over Media library leaves library open (and preview hidden) | §4.2 Media library, §5.15 | G: media library open (preview was visible) / W: OpenSettings twice / T: media_library_open true; preview hidden; library toggle state pressed | a:frankmd-workspace::layout | keep |
| 57 | 847 "blocks background shortcuts while the Settings dialog is open" | Background shortcuts disabled except Esc and Ctrl+, | §5.16 FE-029 change | G: settings open / W: ctrl-shift-v, ctrl-s / T: availability Unavailable; preview unchanged; Esc still closes | a:frankmd-workspace::commands + b | changed (§5.16 Esc/Ctrl+, allowed) |
| 58 | 856 "keeps the vim/scroll-sync switch aria state in sync via app toggle helpers" | Switches reflect toggles done elsewhere (palette/shortcut) | §5.1 toggle ✓, §5.16 | G: settings vm / W: ToggleScrollSync, vim on/off, ToggleTypewriter via commands / T: row switch values and palette ✓ match | a:frankmd-workspace::settings_vm | keep |

### `test/javascript/controllers/stats_panel_controller.test.js` — C=13 (keep 8 / changed 3 / dropped 2; a=11 b=0 c=0)
Original subject: sidebar stats panel (show/hide, 500 ms debounced update, length cache, line position). New home: status line Stats + Position slots §3.6, Document info popover; `workspace::status`, `core::stats`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 41 "initializes update timeout to null" | — (timer handle) | — | — | — | dropped (DOM-specific) |
| 2 | 47 "removes hidden class from panel" | show_stats on → stats visible | Status line Stats §3.6 | G: show_stats false / W: enable / T: status.stats_visible true; b: find("status-stats").visible() | a:frankmd-workspace::status + b | changed (§3.6 status line replaces panel) |
| 3 | 57 "adds hidden class to panel" | show_stats off → stats hidden | §3.6 | G: visible / W: disable / T: stats slot hidden; position stays | a:frankmd-workspace::status | changed (§3.6) |
| 4 | 106 "sets update timeout" | Edits schedule a stats recompute after 500 ms | §3.6 | G: FakeClock, note open / W: edit / T: pending recompute due at +500 ms; stats unchanged before | a:frankmd-workspace::status | keep |
| 5 | 112 "clears previous timeout on new schedule" | Rapid edits coalesce | §3.6 | G: FakeClock / W: edit at 0 and 300 ms / T: one recompute at 800 ms | a:frankmd-workspace::status | keep |
| 6 | 122 "eventually calls update" | Stats eventually reflect text | §3.6 | G: edit "Hello world test" / W: advance 500 ms / T: words 3 | a:frankmd-workspace::status | keep |
| 7 | 135 "calls scheduleUpdate for normal updates" | Editor change events are debounced | §3.6 | G: note open / W: buffer-changed event / T: recompute scheduled, not immediate | a:frankmd-workspace::status | keep |
| 8 | 144 "calls update directly when immediate flag is set" | Opening a note shows stats + position at once | §3.6 | G: FakeClock / W: open_note(text, caret) / T: stats and position set without advancing clock | a:frankmd-workspace::status | keep |
| 9 | 155 "clears update timeout" | Closing the note cancels pending recompute (no stale stats) | §3.6, §5.23 Ctrl+W | G: pending recompute / W: CloseNote, advance / T: stats slot empty, no recompute | a:frankmd-workspace::status | keep |
| 10 | 167 "initializes _lastContentLength to -1" | — (internal cache sentinel) | — | — | — | dropped (DOM-specific) |
| 11 | 171 "skips stats recalculation when content length unchanged" | Same-length edit must not leave stale counts | §3.6 | G: "Hello" counted / W: change to "World" / T: recompute runs (revision changed); counts correct | a:frankmd-workspace::status | changed (quirk fixed; see OQ) |
| 12 | 187 "recalculates stats when content length changes" | Length change recomputes | §3.6 | G: "Hello" / W: "Hello World" / T: words 2, chars 11 | a:frankmd-workspace::status | keep |
| 13 | 198 "always updates line position regardless of content length" | Position updates on caret move | §3.6 Position | G: line numbers off, caret line 5 of 10 / W: caret move / T: position label "5/10" (`12:4` form when on) | a:frankmd-workspace::status | keep |

### `test/javascript/controllers/table_editor_controller.test.js` — C=14 (keep 14 / changed 0 / dropped 0; a=12 b=1 c=1)
Original subject: table editor dialog (dimension inputs, legacy add/remove, grid render, insert event). New home: Table editor modal §5.8 (`dialog-table-editor`), new `workspace::table_editor` VM over `markdown::table`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 93 "updates input values to match table data" | Rows/Columns steppers show the table's size | §5.8 toolbar | G: 3×2 table loaded / W: model.dimensions() / T: rows 3, cols 2 (cols = header length) | a:frankmd-workspace::table_editor | keep |
| 2 | 101 "accepts explicit values" | Typed counts show in the steppers | §5.8 | G: model / W: set_size(5, 4) / T: steppers 5 and 4 | a:frankmd-workspace::table_editor | keep |
| 3 | 147 "no-ops when count is same as current" (columns) | Setting same column count changes nothing (not dirty) | §5.8, §5 intro backdrop rule | G: 3 cols, clean / W: set_cols(3) / T: unchanged; dirty false; revision same | a:frankmd-workspace::table_editor | keep |
| 4 | 223 "no-ops when count is same as current" (rows) | Same for rows | §5.8 | G: 3 rows / W: set_rows(3) / T: unchanged, not dirty | a:frankmd-workspace::table_editor | keep |
| 5 | 268 "addColumn delegates to incrementColumnCount" | Columns `+` adds a column | §5.8 | G: 2 cols / W: click Columns + / T: 3 cols, new header "Header 3" | a:frankmd-workspace::table_editor + b | keep |
| 6 | 274 "removeColumn delegates to decrementColumnCount" | Columns `−` removes one (min 1) | §5.8 | G: 3 cols / W: Columns − / T: 2 cols | a:frankmd-workspace::table_editor + b | keep |
| 7 | 280 "addRow delegates to incrementRowCount" | Rows `+` (and Tab in last cell) adds a row | §5.8 | G: 2 rows / W: Rows +; Tab in last cell / T: 3 then 4 rows | a:frankmd-workspace::table_editor + b | keep |
| 8 | 286 "removeRow delegates to decrementRowCount" | Rows `−` removes one, header kept | §5.8 | G: 3 rows / W: Rows − / T: 2 rows | a:frankmd-workspace::table_editor + b | keep |
| 9 | 326 "syncs dimension inputs after rendering" | Steppers follow grid edits (incl. Ctrl+Shift+Backspace delete row) | §5.8 | G: 3×3 / W: delete row / T: rows stepper 2 | a:frankmd-workspace::table_editor | keep |
| 10 | 334 "renders correct number of rows and cells" | Grid shows rows×cols inputs | §5.8 grid | G: 3×2 table / W: open dialog / T: `table-cell-<r>-<c>` ids exist for 3×2, none beyond | b | keep |
| 11 | 343 "renders header row with font-semibold class" | Header row visually distinct (bold, recessed) | §5.8 | G: table / T: a: row 0 is header; c: bold on surface.recessed | c | keep |
| 12 | 351 "does not allow a quoted cell value to inject attributes" | Cell text with quotes shown and saved verbatim | §5.8 (pipe escaping) | G: cell `x" onfocus="y` / W: render + serialize / T: value verbatim; output contains it unchanged (pipes escaped) | a:frankmd-workspace::table_editor | keep |
| 13 | 386 "dispatches frankmd:insert-table event with markdown" | Insert/Update writes padded markdown; edit mode replaces original range as one edit | §5.8 footer | G: edit mode A,B / 1,2 with range / W: Ctrl+Enter / T: InsertTable{markdown starts with header row A, replace range} | a:frankmd-workspace::table_editor | keep |
| 14 | 401 "closes dialog without dispatching when tableData is empty" | Empty table → just closes | §5.8 | G: empty data / W: confirm / T: no edit; overlay closed | a:frankmd-workspace::table_editor | keep |

### `test/javascript/controllers/text_format_controller.test.js` — C=53 (keep 42 / changed 9 / dropped 2; a=42 b=4 c=5)
Original subject: text-format popup menu (open/close, keyboard nav with wrap, mnemonics, hover/click, apply event, positioning, context-menu and Ctrl+M entry points). New home: Text format menu inline popover §5.5 (Ctrl+M, right-click with selection), `markdown::edit::TextFormatMenu` (transforms already A-tested).

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 108 "initializes selectedIndex to 0" | Menu starts at first row | §5.5 | G: new menu / T: selected 0 | a:frankmd-markdown::edit | keep |
| 2 | 112 "initializes selectionData to null" | Closed menu has no target | §5.5 | G: new menu / T: target None, closed | a:frankmd-markdown::edit | keep |
| 3 | 118 "shows the menu" | Ctrl+M with selection shows the popover | §5.5 | G: "Hello" selected / W: ctrl-m / T: menu open; b: `text-format-menu` visible | a:frankmd-markdown::edit + b | keep |
| 4 | 125 "stores selection data" | Menu remembers the selection it acts on | §5.5 | G: sel 0..5 "Hello" / W: open / T: target {0,5,"Hello"} | a:frankmd-markdown::edit | keep |
| 5 | 132 "resets selectedIndex to 0" | Reopening starts at Bold | §5.5 | G: closed with index 3 / W: open / T: selected 0 | a:frankmd-markdown::edit | keep |
| 6 | 140 "renders menu items" | 7 rows in order | §5.5 | G: open / T: rows [bold,italic,strike,highlight,sub,sup,link] | a:frankmd-markdown::edit | keep |
| 7 | 148 "does not open if no selection data provided" | No target → no menu | §5.5 | G: none / W: open(None) / T: closed | a:frankmd-markdown::edit | keep |
| 8 | 154 "does not open if selection text is empty" | Empty selection → no menu | §5.5 | G: text "" / W: open / T: closed | a:frankmd-markdown::edit | keep |
| 9 | 163 "hides the menu" | Close hides popover | §5.5 | G: open / W: close / T: closed; b: not visible | a:frankmd-markdown::edit + b | keep |
| 10 | 171 "clears selection data" | Close drops target | §5.5 | G: open / W: close / T: target None | a:frankmd-markdown::edit | keep |
| 11 | 186 "renders a button for each format" | One row element per format | §5.5 | G: open / T: ids `text-format-row-<id>` ×7 | b | keep |
| 12 | 194 "highlights selected item with accent color" | Selected row is marked | §5.5, §5 intro list rows | G: index 1 / T: row italic a11y selected; colour c | b | keep |
| 13 | 202 "shows hotkey for each item" | Each row shows mnemonic and global shortcut | §5.5 | G: open / T: mnemonics [B,I,S,H,U,P,L]; shortcut column from registry (Ctrl+B …, blank when none) | a:frankmd-markdown::edit | changed (§5.5 adds global shortcut) |
| 14 | 209 "underlines hotkey character in label" | Mnemonic letter underlined in label | §5.5 | G: open / T: underline span visible (span math A-tested) | c | keep |
| 15 | 246 "ArrowDown moves selection down" | Down moves | §5.5, §6.4 | G: index 0 / W: Down / T: 1 | a:frankmd-markdown::edit | keep |
| 16 | 255 "ArrowDown wraps to top" | Wrap 6→0 | §5 intro wrap | G: 6 / W: Down / T: 0 | a:frankmd-markdown::edit | keep |
| 17 | 265 "ArrowUp moves selection up" | Up moves | §6.4 | G: 2 / W: Up / T: 1 | a:frankmd-markdown::edit | keep |
| 18 | 275 "ArrowUp wraps to bottom" | Wrap 0→6 | §5 intro | G: 0 / W: Up / T: 6 | a:frankmd-markdown::edit | keep |
| 19 | 284 "Enter applies selected format" | Enter applies highlighted row | §5.5 | G: index 1 / W: enter / T: Applied(italic) to target; menu closed | a:frankmd-markdown::edit + b | keep |
| 20 | 299 "Escape closes menu" | Esc closes; editor focus and selection kept | §5.5, §6.1 Esc chain | G: menu open / W: escape / T: closed; editor focused; selection unchanged | a:frankmd-markdown::edit + b | keep |
| 21 | 310 "Hotkey B applies bold format" | B applies bold directly | §5.5 mnemonics | G: open / W: key B / T: Applied(bold) | a:frankmd-markdown::edit | keep |
| 22 | 322 "Hotkey I applies italic format" | I → italic | §5.5 | G: open / W: I / T: Applied(italic) | a:frankmd-markdown::edit | keep |
| 23 | 333 "Hotkey S applies strikethrough format" | S → strikethrough | §5.5 | G: open / W: S / T: Applied(strikethrough) | a:frankmd-markdown::edit | keep |
| 24 | 344 "Hotkey L applies link format" | L → link | §5.5 | G: open / W: L / T: Applied(link) | a:frankmd-markdown::edit | keep |
| 25 | 355 "lowercase hotkey also works" | b works like B | §5.5 | G: open / W: b / T: Applied(bold) | a:frankmd-markdown::edit | keep |
| 26 | 373 "updates selection on hover" | Hover moves highlight | §5.5 | G: open / W: hover row 3 / T: selected 3 | b | keep |
| 27 | 380 "does not update if hovering same item" | — (re-render avoidance) | — | — | — | dropped (DOM-specific) |
| 28 | 398 "applies format on click" | Click applies that row | §5.5 | G: open / W: click `text-format-row-strikethrough` / T: text wrapped `~~`; menu closed | b | keep |
| 29 | 417 "dispatches applied event with format details" | Applying edits the stored selection with prefix/suffix | §5.5 | G: target "Hello" / W: apply(bold) / T: edit `**Hello**` on stored range | a:frankmd-markdown::edit | keep |
| 30 | 432 "closes menu after applying format" | Menu closes after apply | §5.5 | G: open / W: apply / T: closed | a:frankmd-markdown::edit | keep |
| 31 | 439 "does nothing if format is null" | Unknown format is a no-op | §5.5 | G: open / W: apply(None) / T: no edit, still open | a:frankmd-markdown::edit | keep |
| 32 | 448 "does nothing if selection data is null" | No target → no edit | §5.5 | G: target None / W: apply(bold) / T: no edit | a:frankmd-markdown::edit | keep |
| 33 | 480 "positions menu at specified coordinates" | Menu placed near the trigger | §5.5 caret-anchored, flipped, clamped | G: caret near bottom / W: open / T: popover above caret, inside pane | c | changed (§5.5 caret anchor + flip) |
| 34 | 642 "opens menu when text is selected" (openAtPosition) | Right-click with selection opens the menu with {start,end,text} | §5.5 | G: sel "world" 6..11 / W: right-click / T: TextFormatMenu open, target {6,11,"world"} | a:frankmd-markdown::edit | keep |
| 35 | 655 "does not open menu when no text is selected" (openAtPosition) | Right-click without selection → editor context menu, not format menu | §5.5 Editor context menu | G: no selection / W: right-click / T: ContextMenu open; text-format closed | a:frankmd-markdown::edit | changed (§5.5 context menu) |
| 36 | 663 "does not open menu when selection is only whitespace" (openAtPosition) | Whitespace selection treated as none | §5.5 | G: sel "   " / W: right-click / T: ContextMenu, not format menu | a:frankmd-markdown::edit | changed (§5.5 context menu) |
| 37 | 672 "positions menu at specified coordinates" (openAtPosition) | Menu appears at the trigger | §5.5 | G: right-click at point / T: popover anchored at caret rect of selection (not pointer), clamped | c | changed (§5.5 caret anchor) |
| 38 | 702 "opens menu when text is selected" (openAtCursor) | Ctrl+M with selection opens | §5.5, §6.2 | G: sel "Hello" / W: TextFormatMenu action / T: open | a:frankmd-markdown::edit | keep |
| 39 | 715 "does not open menu when no text is selected" (openAtCursor) | Ctrl+M without selection does nothing | §5.5 (see OQ) | G: caret only / W: ctrl-m / T: closed | a:frankmd-markdown::edit | keep |
| 40 | 723 "does not open menu when selection is only whitespace" (openAtCursor) | Whitespace → not opened | §5.5 | G: sel " \t" / W: ctrl-m / T: closed | a:frankmd-markdown::edit | keep |
| 41 | 808 "opens menu when text is selected in CodeMirror" (onContextMenu) | Right-click with editor selection opens format menu | §5.5 | G: md note, sel / W: right-click in `editor` / T: `text-format-menu` visible | a:frankmd-markdown::edit + b | keep |
| 42 | 818 "positions menu at click coordinates" (onContextMenu) | Menu near trigger | §5.5 | G: right-click / T: popover at caret rect, flipped/clamped | c | changed (§5.5 caret anchor) |
| 43 | 827 "does not require a textarea element in the DOM" | — (hidden textarea) | — | — | — | dropped (DOM-specific; FE-074 removed) |
| 44 | 839 "does not open when no text is selected" (onContextMenu) | No selection → context menu instead | §5.5 | G: no sel / W: right-click / T: ContextMenu (Cut, Copy, Paste… Format ▸) | a:frankmd-markdown::edit | changed (§5.5 context menu) |
| 45 | 849 "does not open when selection is only whitespace" (onContextMenu) | Whitespace → context menu | §5.5 | G: sel "  " / W: right-click / T: ContextMenu | a:frankmd-markdown::edit | changed (§5.5 context menu) |
| 46 | 859 "does not open when not markdown" | No format menu in non-markdown files | §5.5, §5.1 availability | G: .txt-like non-md file, sel / W: right-click, ctrl-m / T: format menu not opened; TextFormatMenu Unavailable | a:frankmd-workspace::commands | keep |
| 47 | 867 "does not open when codemirrorController is null" | No editor (empty state) → nothing | §5.2 empty state | G: no note open / W: TextFormatMenu / T: Unavailable("Open a markdown note first"), no popover | a:frankmd-workspace::commands | keep |
| 48 | 887 "opens menu when text is selected in CodeMirror" (openFromKeyboard) | Ctrl+M binding opens the menu | §6.2 Ctrl+M | G: sel / W: press ctrl-m / T: TextFormatMenu dispatched; popover visible | a:frankmd-markdown::edit + b | keep |
| 49 | 894 "positions menu using CodeMirror cursor coordinates" | Anchored below the selection end | §5.5 | G: sel 0..5 / W: ctrl-m / T: anchor offset = selection end (a); popover top ≥ caret bottom (c) | a:frankmd-markdown::edit + b | keep |
| 50 | 902 "falls back to element center when coordsAtPos returns null" | Off-screen caret still yields a visible menu | §5.5 clamped to pane | G: selection end scrolled out / W: ctrl-m / T: popover clamped inside pane | c | changed (§5.5 clamp instead of centre) |
| 51 | 912 "does not open when no text is selected" (openFromKeyboard) | Ctrl+M without selection → nothing | §5.5 | G: caret only / W: ctrl-m / T: closed | a:frankmd-markdown::edit | keep |
| 52 | 920 "does not open when selection is only whitespace" (openFromKeyboard) | Whitespace → nothing | §5.5 | G: sel "  " / W: ctrl-m / T: closed | a:frankmd-markdown::edit | keep |
| 53 | 928 "does not open when codemirrorController is null" (openFromKeyboard) | No editor → nothing | §5.1 unavailable | G: empty state / W: ctrl-m / T: no popover, no panic | a:frankmd-workspace::commands | keep |

### `test/javascript/controllers/theme_controller.test.js` — C=36 (keep 13 / changed 20 / dropped 3; a=32 b=1 c=0)
Original subject: theme dropdown (custom theme preview/restore, menu, apply, icons, debounced save, config listener, Omarchy polling via /config/omarchy_theme). New home: theme picker palette sub-mode §5.1/§5.17, custom theme editor §5.17, Settings › Appearance §5.16, tokens §7.1; `theme::picker`, `theme::custom`, `theme::tokens`, new `theme::omarchy`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 63 "escapes the name, uses base mode, and removes overrides on cancellation" | Custom preview uses base + overrides; name shown verbatim; Cancel restores | §5.17 custom editor | G: dark active / W: preview custom{base nord, accent #112233, name `<img…>`} then cancel / T: palette base nord, accent #112233, name label literal; after cancel dark, no overrides | a:frankmd-theme::custom | keep |
| 2 | 75 "applies updated palette data even when the saved theme id is unchanged" | Re-saved custom palette re-applies; switching to light clears overrides and dark mode | §5.17, §7.1 | G: custom active / W: config change custom accent #abcdef; then select light / T: accent #abcdef; then no overrides, appearance Light | a:frankmd-theme::tokens | keep |
| 3 | 89 "ignores a delayed Omarchy response after a custom preview starts" | Late Omarchy palette reload never overrides an active custom preview | §5.17, §11 Omarchy inotify | G: following Omarchy, reload pending / W: start custom preview; reload completes / T: preview palette kept (generation check) | a:frankmd-theme::omarchy | changed (§11 inotify replaces polling) |
| 4 | 118 "loads initial theme from value" | Startup theme from config | §5.16 Appearance | G: config theme=dark / W: startup / T: active theme dark | a:frankmd-theme::picker | keep |
| 5 | 122 "applies theme to document" | Startup palette applied to whole window | §7.1 | G: theme dark / T: tokens = builtin dark roles | a:frankmd-theme::tokens | keep |
| 6 | 126 "renders menu with themes" | Picker lists all runtime themes | §5.1 Theme: Select… order | G: no Omarchy / W: open picker / T: Follow system, 18 built-ins in builtin.json order, custom, Customize… | a:frankmd-theme::picker | changed (§5.1 palette sub-mode) |
| 7 | 131 "updates current theme display" | Current theme name shown | §5.16 Theme row | G: dark / T: Theme row label "Dark"; picker preselects dark | a:frankmd-theme::picker | changed (§5.16 row replaces dropdown label) |
| 8 | 137 "toggles menu visibility" | Picker opens and closes | §5.1, §6.1 palette | G: closed / W: SelectTheme; Esc / T: overlay Palette(Theme); then None | a:frankmd-workspace::overlay + b | changed (§5.1 palette sub-mode) |
| 9 | 147 "stops event propagation" | — | — | — | — | dropped (DOM-specific) |
| 10 | 161 "updates current theme" | Enter commits selected theme | §5.1 | G: picker on gruvbox / W: Enter / T: current gruvbox | a:frankmd-theme::picker | keep |
| 11 | 167 "applies theme to document" | Committed theme applied | §5.17 | G: same / T: tokens gruvbox | a:frankmd-theme::picker | keep |
| 12 | 173 "hides menu after selection" | Picker closes after commit | §5.1 | G: picker open / W: enter / T: overlay None; focus back to previous region | a:frankmd-workspace::overlay + b | keep |
| 13 | 182 "sets data-theme attribute on html element" | Applying a theme switches whole palette | §7.1 | G: dark / W: apply tokyo-night / T: active palette id tokyo-night | a:frankmd-theme::tokens | keep |
| 14 | 229 "updates checkmarks in menu" | Current theme marked in list after change | §5.1 toggle ✓ | G: picker / W: commit nord, reopen / T: row nord current=true, others false | a:frankmd-theme::picker | changed (§5.1 rows) |
| 15 | 245 "creates buttons for all themes" | Every built-in is listed | §5.17 | G: picker / T: 18 built-in rows, ids match builtin.json | a:frankmd-theme::picker | changed (§5.1 rows) |
| 16 | 251 "sets data-theme attribute on buttons" | — (DOM attribute) | — | — | — | dropped (DOM-specific) |
| 17 | 259 "shows checkmark for current theme" | Current row marked on open | §5.1 | G: current dark / W: open picker / T: dark row current, selected index = dark | a:frankmd-theme::picker | changed (§5.1 rows) |
| 18 | 269 "returns sun icon for sun type" | Light themes show light glyph | §5.17 row glyph | G: picker / T: light row glyph Light (+4 swatches) | a:frankmd-theme::picker | changed (§5.17 light/dark glyph + swatches) |
| 19 | 275 "returns moon icon for moon type" | Dark themes show dark glyph | §5.17 | G: picker / T: dark row glyph Dark | a:frankmd-theme::picker | changed (§5.17) |
| 20 | 281 "returns palette icon for palette type" | Other built-ins get glyph by appearance | §5.17 | G: picker / T: nord Dark, rose-pine Light, flexoki-light Light | a:frankmd-theme::picker | changed (§5.17) |
| 21 | 287 "returns palette icon for unknown type" | Unknown/custom rows fall back to a generic icon | §5.17, §7.7 `palette` | G: custom theme row / T: icon palette | a:frankmd-theme::picker | changed (§5.17) |
| 22 | 294 "calls fetch with correct parameters" | Committed theme persisted | §5.16 300 ms save | G: FakeClock / W: commit nord, advance 300 ms / T: one write theme="nord" in selected scope | a:frankmd-workspace::settings_vm | changed (§5.16 300 ms, config writer) |
| 23 | 316 "debounces multiple calls" | Rapid commits → one write with last; arrow previews never write | §5.1 live preview | G: FakeClock / W: commit 3 themes rapidly; arrow previews / T: one write tokyo-night | a:frankmd-workspace::settings_vm | keep |
| 24 | 345 "updates theme when config-changed event is dispatched" | Theme changed elsewhere (Settings, .fed edit) applies | §5.16 | G: dark / W: ConfigChanged{theme nord} / T: tokens nord | a:frankmd-theme::picker | keep |
| 25 | 354 "ignores event if theme is same as current" | Same theme → no re-apply | §7.1 | G: dark / W: ConfigChanged{dark} / T: no ThemeApplied event | a:frankmd-theme::picker | keep |
| 26 | 367 "closes menu when clicking outside" | Backdrop click closes picker (reverting preview) | §5 intro backdrop, §5.1 Esc reverts | G: picker previewing nord from dark / W: click scrim / T: closed; theme dark | b | changed (§5.1 palette; see OQ) |
| 27 | 383 "removes event listeners" | — | — | — | — | dropped (DOM-specific) |
| 28 | 393 "clears pending timeout" | Pending theme write is not lost at shutdown | §5.23 Quit | G: committed nord, write pending / W: Quit / T: write flushed before exit | a:frankmd-workspace::settings_vm | changed (flush instead of drop; see OQ) |
| 29 | 530 "adds Omarchy to runtimeThemes when available" | Omarchy detected → Follow Omarchy option | §5.1, OM-005 | G: Omarchy colors.toml fixture / T: first row FollowOmarchy | a:frankmd-theme::picker | changed (§5.1 Follow Omarchy) |
| 30 | 537 "includes Omarchy in rendered menu" | Omarchy row adds to list | §5.1 | G: Omarchy present / T: rows = FollowOmarchy + FollowSystem + 18 + custom + Customize… | a:frankmd-theme::picker | changed (§5.1) |
| 31 | 544 "returns sync icon for sync type" | Omarchy row distinguishable | §5.1 | G: Omarchy theme "Tokyo Night" / T: row label shows current Omarchy theme name | a:frankmd-theme::picker | changed (§5.1 shows Omarchy name) |
| 32 | 550 "applies omarchy theme with injected CSS variables" | Omarchy colours drive roles | §7.1 OM-001 | G: colors.toml background #1a1a2e / W: follow Omarchy / T: surface.base #1a1a2e | a:frankmd-theme::tokens | changed (§7.1 colors.toml roles) |
| 33 | 560 "sets dark class based on omarchy theme luminance" | Omarchy appearance dark | §7.1 (mode, luminance fallback) | G: mode dark (and fixture without mode, dark bg) / T: appearance Dark | a:frankmd-theme::tokens | changed (§7.1 mode first) |
| 34 | 567 "removes omarchy styles when switching to another theme" | Leaving Omarchy drops its roles | §7.1 | G: following Omarchy / W: select dark / T: tokens == builtin dark exactly | a:frankmd-theme::tokens | keep |
| 35 | 577 "stops polling on disconnect" | No Omarchy watching after leaving it / shutdown | §11 inotify | G: following Omarchy, watcher active / W: select built-in; shutdown / T: watcher inactive | a:frankmd-theme::omarchy | changed (§11 inotify) |
| 36 | 586 "falls back to dark when omarchy endpoint disappears" | Omarchy removal falls back to built-in dark | OM-005, §7.1 | G: following Omarchy / W: theme dir removed (not mid-swap) / T: active dark; mid-swap keeps last good palette | a:frankmd-theme::omarchy | keep |

### `test/javascript/controllers/typewriter_controller.test.js` — C=22 (keep 13 / changed 7 / dropped 2; a=20 b=0 c=0)
Original subject: typewriter mode controller (toggle, CSS classes, aria-pressed, CodeMirror scroll maintenance, sync data). New home: View mode Typewriter §4.2 (Ctrl+\\), status indicator §3.6, Document menu View row §3.4; `workspace::layout.typewriter`, `workspace::typewriter`.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 48 "initializes with enabled=false by default" | Typewriter off by default | §4.2 | G: empty config / T: layout.typewriter false | a:frankmd-workspace::layout | keep |
| 2 | 52 "applies initial mode on connect" | Off → no centring padding | §4.2 | G: off / W: open note / T: typewriter bottom padding 0 | a:frankmd-workspace::typewriter | keep |
| 3 | 59 "toggles enabled state" | Ctrl+\\ flips mode | §6.1 | G: md note / W: press ctrl-\\ twice / T: true then false | a:frankmd-workspace::layout + b | keep |
| 4 | 71 "dispatches toggled event" | Dependents learn of the toggle | §3.6, §5.1 | G: off / W: ToggleTypewriter / T: TypewriterChanged{true}; palette row ✓ | a:frankmd-workspace::layout | keep |
| 5 | 81 "calls CodeMirror setTypewriterMode when toggling" | Editor enters typewriter behaviour | §4.2 | G: note open / W: toggle / T: EditorConfig.typewriter true | a:frankmd-workspace::layout | keep |
| 6 | 93 "adds typewriter-centered class to wrapper when enabled" | Caret line can sit at 50 %; half-height bottom padding | §4.2 | G: editor height 800 / W: enable / T: bottom padding 400 | a:frankmd-workspace::typewriter | changed (§4.2 native padding) |
| 7 | 100 "removes typewriter-centered class from wrapper when disabled" | Padding removed when off | §4.2 | G: on / W: disable / T: padding 0 | a:frankmd-workspace::typewriter | changed (§4.2) |
| 8 | 108 "adds typewriter-centered class to body when enabled" | In split view preview follows caret block; hidden preview stays hidden | §4.2, §11 | G: preview visible / W: enable / T: preview_follow true; with preview hidden → stays hidden | a:frankmd-workspace::layout | changed (§11 no force-open) |
| 9 | 115 "removes typewriter-centered class from body when disabled" | Preview stops following | §4.2 | G: on / W: disable / T: preview_follow false | a:frankmd-workspace::layout | changed (§4.2) |
| 10 | 123 "updates toggle button aria-pressed" | Toggle state visible on its controls | §3.4 View row, §5.1 ✓, §5.16 switch | G: off / W: toggle on/off / T: registry toggle_state and settings switch match | a:frankmd-workspace::commands | changed (no header button; menu/palette check) |
| 11 | 135 "adds bg-hover class to button when enabled" | Visible "on" indicator | §3.6 Indicators | G: off / W: enable / T: status indicator `text-cursor` present | a:frankmd-workspace::status | changed (§3.6 indicator) |
| 12 | 142 "removes bg-hover class from button when disabled" | Indicator hidden when off | §3.6 | G: on / W: disable / T: indicator absent | a:frankmd-workspace::status | changed (§3.6) |
| 13 | 152 "does nothing when not enabled" | No scroll adjustment when off | §4.2 | G: off / W: caret move / T: typewriter::scroll_target None | a:frankmd-workspace::typewriter | keep |
| 14 | 160 "calls CodeMirror maintainTypewriterScroll when enabled" | Caret line held at 50 % | §4.2 | G: on, viewport 800, caret line y / W: caret move / T: scroll_target places caret at 400 | a:frankmd-workspace::typewriter | keep |
| 15 | 168 "handles missing CodeMirror controller gracefully" | No note → no-op | §5.2 | G: empty state, on / W: maintain / T: None, no panic | a:frankmd-workspace::typewriter | keep |
| 16 | 178 "returns sync data from CodeMirror controller" | Preview follow uses current line / total | §4.2 | G: caret line 5 of 10 / T: sync_data {5,10} | a:frankmd-workspace::typewriter | keep |
| 17 | 185 "returns null if no CodeMirror controller available" | No note → no sync data | §4.2 | G: empty state / T: None | a:frankmd-workspace::typewriter | keep |
| 18 | 195 "returns enabled state" | State readable | §4.2 | G: on / T: is_enabled true | a:frankmd-workspace::layout | keep |
| 19 | 205 "sets enabled value and applies mode" | Setting typewriter_mode from config applies it | §5.16 Writing | G: off / W: config reload typewriter_mode=true / T: on; padding applied | a:frankmd-workspace::layout | keep |
| 20 | 214 "syncs with CodeMirror controller" | Editor flag synced on set | §4.2 | G: note open / W: set_enabled(true) / T: EditorConfig.typewriter true | a:frankmd-workspace::layout | keep |
| 21 | 226 "finds CodeMirror controller by data-controller attribute" | — | — | — | — | dropped (DOM-specific) |
| 22 | 239 "returns null if no CodeMirror element found" | — | — | — | — | dropped (DOM-specific) |

### `test/javascript/controllers/video_dialog_controller.test.js` — C=37 (keep 28 / changed 9 / dropped 0; a=33 b=4 c=0)
Original subject: insert-video dialog (URL detection UI, tabs, YouTube search results selection, thumbnail fallback, drag-drop/picker upload). New home: Insert video modal §5.7 (`dialog-insert-video`, tabs Upload/URL/YouTube), `workspace::insert_video`, portal FileChooser OM-011, §5.22 not-configured.

| # | Original (line "name") | Guarantees (user-observable) | New home | Adapted test (Given / When / Then) | Level | Status |
|---|---|---|---|---|---|---|
| 1 | 76 "initializes empty search results" | No results initially | §5.7 | G: new VM / T: results empty | a:frankmd-workspace::insert_video | keep |
| 2 | 80 "initializes selected index to -1" | Nothing selected initially | §5.7 | G: new VM / T: selected None | a:frankmd-workspace::insert_video | keep |
| 3 | 84 "checks YouTube API availability" | YouTube tab shows search only when configured, else setup notice | §5.7, §5.22 | G: no YouTube key / W: open YouTube tab / T: NotConfigured + "Open Settings › Integrations"; with key → search field | a:frankmd-workspace::insert_video | changed (§5.22 in-place setup link) |
| 4 | 93 "shows the dialog" | InsertVideo opens the modal | §5.7 | G: md note / W: InsertVideo / T: overlay Dialog(InsertVideo); b: `dialog-insert-video` visible | a:frankmd-workspace::overlay + b | keep |
| 5 | 99 "resets video URL input" | Reopen clears URL | §5.7 | G: previous URL / W: reopen / T: url "" | a:frankmd-workspace::insert_video | keep |
| 6 | 106 "disables insert button" | Insert disabled on open | §5.7 | G: reopen / T: can_insert false | a:frankmd-workspace::insert_video | keep |
| 7 | 113 "resets detected video type" | Detection cleared on open | §5.7 | G: prior youtube detection / W: reopen / T: detected None | a:frankmd-workspace::insert_video | keep |
| 8 | 122 "resets search results" | Results/selection cleared on open | §5.7 | G: prior results / W: reopen / T: results empty, selected None | a:frankmd-workspace::insert_video | keep |
| 9 | 133 "closes the dialog" | Close/Esc closes | §5 intro | G: open / W: escape / T: overlay None; editor focused | a:frankmd-workspace::overlay + b | keep |
| 10 | 142 "shows URL panel when URL tab selected" | URL tab shows URL panel | §5.7 tabs | G: open / W: select URL tab (Ctrl+Tab) / T: tab Url | a:frankmd-workspace::insert_video | changed (§5.7 3 tabs) |
| 11 | 150 "shows search panel when search tab selected" | YouTube tab shows search | §5.7 | G: open / W: select YouTube / T: tab YouTube | a:frankmd-workspace::insert_video | changed (§5.7 3 tabs) |
| 12 | 211 "calls insertVideo on Enter when button is enabled" | Enter in URL field inserts when valid | §5.7, §5 intro Ctrl+Enter | G: youtu.be URL / W: enter (and ctrl-enter) / T: insert emitted, closed | a:frankmd-workspace::insert_video + b | keep |
| 13 | 222 "does not call insertVideo when button is disabled" | Enter with invalid URL does nothing | §5.7 | G: "page.html" / W: enter / T: no insert, still open | a:frankmd-workspace::insert_video | keep |
| 14 | 234 "uses the trusted default thumbnail once, then hides it if that fails" | Thumbnail falls back to default.jpg once, then hides | §5.7 URL tab thumbnail | G: id abc / W: thumbnail load fails twice / T: src img.youtube.com/vi/abc/default.jpg, then None | a:frankmd-workspace::insert_video | keep |
| 15 | 254 "closes dialog when no video type detected" | Confirm with nothing detected just closes | §5.7 | G: url "" / W: confirm / T: no edit; closed | a:frankmd-workspace::insert_video | keep |
| 16 | 305 "closes dialog after inserting" | Dialog closes after insert | §5.7 | G: file URL / W: confirm / T: closed | a:frankmd-workspace::insert_video | keep |
| 17 | 350 "shows error when query is empty" | Empty search query → message | §5.7 | G: YouTube tab, query "" / W: search / T: inline status t(status.please_enter_keywords); no request | a:frankmd-workspace::insert_video | keep |
| 18 | 357 "shows error when YouTube API is not enabled" | Not configured → setup message | §5.22 | G: no key / W: search / T: NotConfigured state (youtube_not_configured text + Settings link) | a:frankmd-workspace::insert_video | changed (§5.22) |
| 19 | 431 "does nothing when results are empty" | Selection with no results no-op | §5.7 | G: no results / W: select(0) / T: selected None | a:frankmd-workspace::insert_video | keep |
| 20 | 438 "highlights selected video button" | Selected result is marked | §5.7 grid | G: 2 results / W: select 1 / T: selected Some(1); b: result a11y selected; ring c | a:frankmd-workspace::insert_video + b | keep |
| 21 | 455 "removes highlight when index is -1" | Clearing selection unmarks | §5.7 | G: selected 1 / W: clear / T: None | a:frankmd-workspace::insert_video | keep |
| 22 | 468 "dispatches video-selected event with embed code" | Choosing a result inserts its embed | §5.7 Enter inserts immediately | G: result {xyz789,"Test Title"} / W: Enter / T: insert HTML embed with youtube.com/embed/xyz789 | a:frankmd-workspace::insert_video | keep |
| 23 | 485 "dispatches Hugo shortcode with title when checkbox is checked" | Hugo option yields shortcode with title | §5.7 (persisted checkbox) | G: hugo on / W: Enter result / T: `{{< youtube id="xyz789" title="Test Title" >}}`; checkbox state persisted | a:frankmd-workspace::insert_video | keep |
| 24 | 502 "does nothing when videoId is missing" | Result without id ignored | §5.7 | G: result id "" / W: Enter / T: no insert | a:frankmd-workspace::insert_video | keep |
| 25 | 514 "closes dialog after selection" | Closes after choosing | §5.7 | G: result / W: Enter / T: closed | a:frankmd-workspace::insert_video | keep |
| 26 | 536 "rejects a disallowed extension with a reason and does not upload" | Wrong file type rejected with reason | §5.7 Upload | G: drop notes.txt / T: no upload; message t(dialogs.video.drop_rejected) | a:frankmd-workspace::insert_video | keep |
| 27 | 548 "uploads a valid video and enables the insert button" | Valid upload becomes insertable | §5.7 | G: drop clip.mp4, fake store Ok(videos/clip.mp4) / T: detected File{videos/clip.mp4}; can_insert true | a:frankmd-workspace::insert_video | keep |
| 28 | 562 "surfaces the server error and refreshes storage after an S3 upload fails" | Upload error shown; storage state refreshed once | §5.7, §5.12 provider fallback | G: store Err("boom") / W: drop / T: inline error "boom"; MediaStorageState refresh requested once | a:frankmd-workspace::insert_video | keep |
| 29 | 578 "shows the localized uploading label, not a raw status key" | Upload progress shown localized | §5.7 progress bar | G: upload in flight / T: state Uploading{progress}; label t(dialogs.video.uploading) resolved | a:frankmd-workspace::insert_video | changed (§5.7 progress bar) |
| 30 | 594 "ignores a second drop while an upload is already in flight" | No concurrent uploads | §5.7 | G: uploading / W: second drop / T: ignored; one store call | a:frankmd-workspace::insert_video | keep |
| 31 | 612 "opens the file picker on Enter" | Enter on dropzone opens chooser | §5.7, OM-011 portal | G: dropzone focused / W: enter / T: fake portal FileChooser requested (video filters) | b | changed (portal FileChooser) |
| 32 | 618 "opens the file picker on Space" | Space likewise | OM-011 | G: dropzone focused / W: space / T: chooser requested | b | changed (portal FileChooser) |
| 33 | 624 "ignores other keys" | Other keys don't open chooser | §5.7 | G: focused / W: "a" / T: no request | b | keep |
| 34 | 632 "opens the native file picker when the dropzone is clicked" | Click opens chooser | OM-011 | G: Upload tab / W: click dropzone / T: chooser requested; c: real portal dialog | b | changed (portal FileChooser) |
| 35 | 638 "uploads a file selected via the picker" | Chosen file uploads | §5.7 | G: chooser returns clip.mp4 / W: on_files_chosen / T: same upload flow as drop | a:frankmd-workspace::insert_video | keep |
| 36 | 655 "triggers search on Enter" | Enter in search field searches | §5.7 | G: query "test query" / W: enter / T: search requested with that query | a:frankmd-workspace::insert_video + b | keep |
| 37 | 665 "navigates to first result on ArrowDown" | Down from search field moves into results grid | §5.7 2-column grid nav | G: 4 results, focus search / W: down; right; down / T: selected 0, 1, 3 (wrap per §5) | a:frankmd-workspace::insert_video | changed (§5.7 grid navigation) |

#### New VM requirements (G8)
- `frankmd-workspace::settings_vm` — `SettingsVm`: `selected_category` (Appearance…About) + `visible_rows()`; per-key setters that apply live and emit typed effects (`EditorConfigChanged`, `ThemeChanged`, `LayoutChanged`) even with no note open; `reload(ConfigSnapshot)` with tolerant parsing (unknown view_mode → Split, missing key keeps current, read error keeps values + logs); 300 ms debounced writer through injected Clock that coalesces all pending keys (not latest-body-wins) and flushes on quit; `ConfigWritten` event; inline row error with revert on write failure; `font_options()` (System monospace + 9 ids) and `font_preview() -> (family, size)`; `EditorConfig::from_settings` (font family mapping, size, line numbers, indent unit, recenter, typewriter). Rows 1–58 (settings), theme 22/23/28.
- `frankmd-workspace::media_storage` — `MediaStorageState::refresh(force)` used by settings reload and upload failures instead of applying stale snapshots (settings 47, video 28).
- `frankmd-workspace::status` — stats slot: `stats_visible` from show_stats, debounced recompute (500 ms, injected clock) keyed by buffer revision (not length), immediate compute on note open, cancel on close; position label `L/total` vs `L:C`; typewriter indicator. (stats 2–13, settings 40, typewriter 11–12)
- `frankmd-workspace::table_editor` — `TableEditorModel`: `dimensions()`, `set_size/set_rows/set_cols/inc/dec` (no-op + not dirty when unchanged), `dirty`, header-row flag, verbatim cell text, `confirm() -> Option<TableEdit{markdown, replace_range}>` (None + close when empty). (table 1–14)
- `frankmd-markdown::edit` — `TextFormatMenu`: open(target)/close, selected index with wrap, mnemonic dispatch (case-insensitive), `apply(id) -> Option<Edit>`, rows with mnemonic + registry shortcut, `anchor_offset()` = selection end; open guard rejects empty/whitespace selection; right-click routing `on_context_click(selection, is_markdown) -> TextFormat / ContextMenu / None`. (text_format 1–53)
- `frankmd-workspace::commands` — availability for TextFormatMenu (needs markdown note), and while Settings is open everything Unavailable except Escape and OpenSettings (settings 57, text_format 46/47/53); toggle_state for ToggleTypewriter (typewriter 10).
- `frankmd-workspace::typewriter` — `bottom_padding(editor_height)`, `scroll_target(caret_y, viewport) -> Option<f32>` (None when off / no editor), `sync_data() -> Option<(line,total)>`; `layout.preview_follow` derived (only when preview visible). (typewriter 2–20)
- `frankmd-theme::picker` — `ThemePickerModel`: rows (FollowOmarchy with current Omarchy name when detected, FollowSystem, 18 built-ins with Light/Dark glyph + 4 swatches, custom, Customize…), `current` marker, preview/commit/revert, ignore same-theme ConfigChanged. (theme 4–31)
- `frankmd-theme::omarchy` (new) — watcher state (active only while following Omarchy), generation counter so late reloads can't override a custom preview, removal → fallback dark, mid-swap → last good palette. (theme 3, 35, 36)
- `frankmd-theme::custom` — preview(palette)/cancel restoring a captured snapshot. (theme 1)
- `frankmd-workspace::insert_video` — tabs Upload/Url/YouTube; reset on open; `youtube_status` Configured/NotConfigured; results grid (2 cols) with selection + wrap; thumbnail fallback chain; upload state Idle/Uploading{progress}/Error with single-flight guard and extension allow-list; file-chooser request effect; persisted Hugo flag. (video 1–37)

#### Open questions (G8)
- Settings focus on open: FrankMD focused the first category button; §5.16 puts a search field on top. Which gets focus (settings 32)?
- Settings backdrop click: §5 intro lists the overlays that close on backdrop but doesn't name Settings (settings 34).
- Settings save debounce: FrankMD used 500 ms with latest-body-wins (rapid edits to different keys lost the earlier one); §5.16 says 300 ms. Plan assumes 300 ms and coalescing all keys. Please confirm.
- Stats: the "skip recompute when length is unchanged" quirk leaves stale counts. §11 says nothing about it; plan fixes it (stats 11). Also: should native stats still debounce 500 ms, or update every frame?
- Ctrl+M with no selection: §5.5 says only "Ctrl+M, or right-click with a selection". FrankMD did nothing (text_format 39/51). Kept as no-op.
- Right-click with a selection in a non-markdown file: show the plain context menu, or nothing? (text_format 46)
- Theme picker backdrop click: does it revert the live preview like Esc, or commit? Plan assumes revert (theme 26).
- Pending theme/settings write at quit: FrankMD dropped it on disconnect; plan flushes it (theme 28).
- When Omarchy disappears, FrankMD fell back to `dark` and saved `theme=dark`. Should the native app persist that, or keep "Follow Omarchy" and just show dark until Omarchy returns (theme 36)?
- Default editor font: §7.2 makes System monospace the default where FrankMD's default was cascadia-code. Is an absent `editor_font` in an existing `.fed` meant to change the font users see (settings 21)?
