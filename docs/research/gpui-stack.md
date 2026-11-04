# FrankMDFast on GPUI + gpui-kit: verified stack, gaps and plans

*Researched 2026-11-04. The user chose **GPUI + gpui-component (longbridge)** over the GTK4
recommendation in `components.md`. This file records what that stack is today, verified
against crates.io's API, the GitHub API, the `longbridge/gpui-kit` checkout at `0ab9cf4e`
(2026-11-04), and the published crate sources in `~/.cargo/registry`. It then explains how
we fill each gap. Re-verify anything older than about a month before bumping a pin.*

Build proof: the scaffold in this repo (`crates/frankmd-app`) builds against these pins and
opens a native Wayland window on this machine (Hyprland, NVIDIA RTX 5090, Vulkan through
wgpu). The window has app_id `frankmd`, a gpui-kit `Root`/`TitleBar`, and a resizable
3-pane layout with a tree-sitter-markdown `Editor`. A clean build needed no extra system
packages. See §9.

---

## 1. What exists today (names and versions)

| Crate | Version (date) | What it is |
|---|---|---|
| **`gpui-kit`** | **0.9.0** (2026-11-02) | The umbrella crate an application depends on. It re-exports GPUI (`use gpui_kit::*`), `gpui_kit::base`, `gpui_kit::component`, `gpui_kit::assets` and `gpui_kit::platform`. Repo: **longbridge/gpui-kit** (the renamed gpui-component repo, ≈19k★). |
| `gpui-component` | 0.9.0 (2026-11-02) | The styled component layer. Still published under the old name and re-exported as `gpui_kit::component`. |
| `gpui-base` | 0.9.0 | The unstyled behaviour layer (editor core, text selection, dock model, tree, virtual list). |
| `gpui-kit-assets` | 0.9.0 | The Lucide icon set. |
| **`gpui-pre`** (+ `-platform`, `-linux`, `-wgpu`, `-macos`, `-macros`, `-sum-tree`, …) | **=0.3.11** (2026-11-02) | A crates.io **snapshot of Zed's GPUI at `zed@f816e1f`**, republished weekly by the gpui-kit maintainers. gpui-kit pins it **exactly**: their workspace comment cites #3156, where a caret requirement let a newer snapshot break apps. |
| `gpui` (Zed, crates.io) | 0.2.2 (2025-10-22) | Zed's own crates.io release. It is about a year stale and **not** what gpui-kit uses. Don't depend on it. |
| `gpui-fast` | 0.1.13 | Longbridge's performance fork of GPUI, selectable through the `gpui-kit/gpui-fast` feature without changing imports. Not chosen yet (§10). |
| `gpui-ce` | 0.3.0 | A community fork. Not used. |

Weekly cadence: 0.7.0 (09-28) → 0.7.1 → 0.7.2/0.7.3 → 0.8.0 → 0.8.1 → **0.9.0 (11-02)**.
Every 0.x bump may break the API.

MSRV: gpui-kit's docs say **Rust ≥ 1.92**, because the Linux platform pulls `oo7 0.6`. We
pin toolchain 1.99.1 (already installed) in `rust-toolchain.toml`.

### 1.1 How we depend on it

**Decision: depend on crates.io `gpui-kit = "=0.9.0"` with an exact pin, and commit `Cargo.lock`.**

```toml
gpui-kit = { version = "=0.9.0", default-features = false, features = [
    "component", "assets", "tree-sitter-markdown",
] }
```

- Through this pin we get `gpui-pre =0.3.11` transitively. We never name a `gpui-pre-*` crate
  ourselves, so the snapshot can't drift out of step.
- Why not a git rev pin? Published crates give reproducible offline builds and a
  `Cargo.lock` that matches what gpui-kit's CI tested. When we need an unreleased fix, we
  temporarily switch to `gpui-kit = { git = "https://github.com/longbridge/gpui-kit", rev = "<sha>" }`.
  The kit crate already re-exports everything, so no import changes and no `[patch]` are needed.
- Upgrade policy: bump on purpose, about monthly, on a branch. Run the UI integration suite
  and the spike checklist (§10), then read gpui-kit's CHANGELOG for removed APIs.
- More features we'll enable when needed: `tree-sitter-<lang>` for the fenced-code grammars
  we highlight in the preview (or `syntect` for about 200 languages with no tree, see §4.2),
  `test-support` (dev-dependency only), and `inspector` (debug builds).
- The dev profile optimizes `gpui-pre`, `gpui-component`, `taffy`, `rustybuzz` and others
  at `opt-level = 3`, as gpui-kit's installation guide recommends. Unoptimized GPUI is too
  slow to judge typing latency.

---

## 2. Widget inventory vs FrankMD needs

| FrankMD need (inventory id) | gpui-kit 0.9 widget | Fit | Notes |
|---|---|---|---|
| Markdown editor (FE A.3–A.22) | `input::{Editor, EditorState}` | **Good** | Rope-based (`ropey 2.0.0-beta.1`). Tree-sitter highlighting (`tree-sitter-md` 0.5) or syntect. Line numbers, folding, soft wrap, whitespace, indent guides, multi-cursor and column selection, auto-close and smart indent, LSP-style completion and hover providers (usable for wikilink and slash-command completion), text, range, gutter and overview-ruler decorations, `prewrap`, a font override, and a built-in search panel. Claimed "stable at 200K lines". gpui-kit#2566 (5 GB RAM on 500 MB) was closed 2026-07-15 as demo-specific. |
| Find/Replace (A.16) | `EditorState::{set_search_query, next_search_match, replace_current_search_match, replace_all_search_matches, search_session().matcher}` + `.searchable(false)` | **Good** | We keep FrankMD's own non-modal find dialog and mini-bar, with our regex dialect from `frankmd-search::find` (JS semantics). Matches are painted with a `RangeDecorationCollection` so the editor highlights exactly what our matcher found. Replace-all is one `set_value` and one undo step. |
| Jump to line (A.17), stats (A.18) | `set_cursor_position`, `cursor_position`, `text()` | Good | — |
| Typewriter (A.10) | `cursor_surrounding_lines(Some(n))` + `scroll_beyond_last_line(Some(n))` | **Good** | See §5. |
| Vim mode (A.9) | — | **Gap** | See §6. gpui-kit#2617 ("Expose api methods … for extension development for Editor") is still **open**. |
| Markdown preview (§5) | `text::TextView` (markdown-rs, plugins, selection, highlight ranges) | **Partial** | It has no math, no source-line anchors for sync scroll, no task toggling and no embeds. We build our own preview from comrak (§4) and reuse gpui-base's window text selection. |
| Explorer tree (F.1–F.13) | `tree::{tree, TreeState, TreeItem}` | Good | Built on `uniform_list`, so it is virtualized. Has lazy loading. Multi-select, drag-move and context menus are ours, built on GPUI `on_drag`/`on_drop` and `context_menu`. |
| File finder Ctrl+P (F.15), slash menu (A.14), emoji picker | `command` (command palette with `v_virtual_list`), `list::List` (virtualized, delegate pattern, search), `popover`, `menu` | Good | The ranking comes from `frankmd-search::finder` (nucleo). The palette only renders. |
| Content search (F.16), image library grid | `list::List`, `VirtualList` (variable heights), `table` (virtual rows) | Good | — |
| Panes / split-single view / sidebar (1.2–1.4) | `resizable::{h_resizable, resizable_panel}`, `dock` (serializable), `sidebar` | Good | Plain `h_resizable` is enough for explorer, editor and preview. `Dock` is unnecessary for v1. |
| Dialogs: new note, rename, recovery diff, AI diff, settings modal, help | `dialog`, `alert_dialog`, `sheet`, `form`, `setting`, `diff` (a diff view), `tabs`, `select`, `switch`, `slider`, `color_picker` (theme editor) | Good | `diff` plus `similar` covers the grammar-check and draft-recovery diffs. |
| Toasts (1.7) | `notification` (`window.push_notification`) | Good | In-window toasts. Desktop notifications go through GPUI `SystemNotification` (freedesktop over D-Bus on Linux), §3.4. |
| Status bar, title bar, kbd hints, tooltips | `status_bar`, `TitleBar` (client-side decorations on Linux), `kbd`, `tooltip` | Good | Under Hyprland, keep `TitleBar` minimal or hide it, because tiling users have no decorations. |
| Image overlay / lightbox | GPUI `img()` (PNG/JPEG/GIF/WebP/SVG through `image` + resvg) | Good | — |
| Accessibility | AccessKit roles built into gpui-base | OK | Better than iced or egui. Not AT-SPI-grade like GTK. |
| **RTL/bidi** (OM-009) | — | **Gap** | §7. |
| Webview (YouTube iframe) | `gpui-webview` (wry) | **Not on Wayland** | `webview.md`: "Linux (Wayland) — Not supported. Run on X11 through XWayland." We don't use it. §4.5. |

### 2.1 Theme system and Omarchy at runtime

- `Theme` is a GPUI `Global`. `ThemeConfig` is the serde schema of gpui-kit's `themes/*.json`:
  `name`, `mode`, about 99 `colors` keys (`background`, `foreground`, `primary.background`,
  `sidebar.*`, `list.*`, `popover.*`, `selection.background`, `caret`, `scrollbar.*`,
  `base.red` …), and `highlight` (`editor.*` plus `syntax.<scope>` for tree-sitter captures).
- Runtime application: `Theme::update(cx, |t| t.apply_config(&Rc::new(cfg)))`, then
  `cx.refresh_windows()`. No restart, no file round-trip. A string-based option also exists:
  `ThemeRegistry::global_mut(cx).load_themes_from_str(json)`.
- **Our pipeline:** Omarchy `colors.toml` → `frankmd-theme::OmarchyPalette` (full alias
  cascade, OM-001/002) → `UiTokens` (FrankMD's 38 tokens, contrast-corrected, OM-004) →
  `frankmd-app::bridge::theme` maps them to `ThemeConfig` colors and `highlight.syntax` →
  `apply_config`. A watcher on `~/.local/state/omarchy/current/` (OM-003) re-runs the
  pipeline. Target: under 100 ms from the theme switch to the repaint.
- Fonts: gpui-kit falls back from `DejaVu Sans Mono` to an installed mono font. On this
  machine the scaffold logged "using Noto Sans Mono", which is wrong for Omarchy. We set
  `Theme::mono_font_family` from `fc-match monospace` (JetBrainsMono Nerd Font on Omarchy,
  OM-012) and the UI font the same way (monospace UI chrome on Omarchy). We re-resolve on
  fontconfig changes. Text is shaped by **cosmic-text** (`gpui-pre-wgpu`), which does
  fontconfig fallback for CJK and emoji.
- gpui-kit translates its own component strings through `rust-i18n` (`gpui_component.*`
  keys). It ships `en`, `zh-*` and `it` only, so the app crate supplies a small overlay
  `locales/ui.yml` for pt-BR, pt-PT, es, he, ja and ko.

### 2.2 Reusing FrankMD's assets (`assets/`, see `assets/README.md`)

| Asset | How it is used on GPUI |
|---|---|
| `icons/phosphor.json` (201 curated Phosphor icons, MIT; `[name, svgPath, viewBox, keywords]`) | **Decision: the Phosphor table backs the icon picker and FrankMD's own chrome icons. gpui-kit's bundled Lucide set (1,830 icons) stays as the fallback.** gpui-kit loads icons as `icons/<name>.svg` through the app's `AssetSource` and draws them as monochrome masks tinted with the text color. Phosphor's filled paths work as-is. `frankmd-app::assets::FrankAssets` synthesizes `icons/ph/<name>.svg` from the JSON on demand. `gpui_kit::component::set_icon_resolver` redirects a component icon path (for example `icons/check.svg`) to `icons/ph/check.svg` when a Phosphor equivalent exists. 113 names match exactly, and a small alias table covers chevrons, carets and the rest. Anything without an equivalent keeps Lucide. Replacing Lucide completely isn't possible: gpui-kit's components themselves reference 54 distinct `IconName`s internally, but the full Lucide catalog has 1,830 icons, and the curated Phosphor set has only 201. |
| `brand/icon.svg` + PNGs | Packaging installs the `.desktop`/hicolor icon (Wayland takes the window icon from the desktop entry matched by app_id `frankmd`). It is also embedded (`include_bytes!`) for the about and empty states. |
| `locales/*.yml` | Embedded unchanged (`frankmd_i18n::Locale::source_yaml`, `include_str!`). |
| `themes/builtin.json` | Embedded (`frankmd_theme::BUILTIN_THEMES_JSON`) and mapped to `UiTokens`, then to `ThemeConfig`. The ids match the `.fed` `theme` values. |
| `emoji/*.json` | Embedded in `frankmd_core::emoji`, which serves the pickers and `:shortcode:` rendering. The localized names add about 2.9 MB in total. If binary size matters, they can be compressed or loaded lazily. |
| `fonts/web/*.woff2` | Reference only. GPUI needs TTF/OTF, so the `editor_font` ids (`cascadia-code`, `jetbrains-mono`, …) map to family names resolved with fontconfig, falling back to `monospace`. |
| `fonts/katex/*.woff2` | Not needed: RaTeX embeds the KaTeX fonts as TTF (`ratex-katex-fonts`). Kept for reference and licence parity. |

All embedding uses `include_str!`/`include_bytes!` from the repo's `assets/` dir. A separate
`rust-embed` isn't worth it for this handful of files.

---

## 3. Platform integration (verified in `gpui-pre-linux 0.3.11` source)

### 3.1 Wayland and IME (fcitx5, OM-007)

- GPUI's Wayland client binds **`zwp_text_input_manager_v3`**, enables text input on focus,
  sends `set_content_type`, and keeps `set_cursor_rectangle` updated so the fcitx5
  candidate window follows the caret. gpui-kit has an issue (#3286, closed) fixing jumpy
  candidate bounds.
- Zed's fcitx5 history on Wayland is mostly fixed. Enter now confirms instead of inserting a
  newline (zed#35537, #31337, closed). Hyprland and GNOME input-method breakage was fixed in
  zed#37235.
- Still open upstream:
  - zed#65302: the active preedit segment is not visually distinguished on Wayland.
  - zed#65650: the first key after a keymap change loses its Shift level (fcitx5).
  - X11-only XIM bugs, which don't matter to us because we're Wayland-native.
  - gpui-kit's open IME issues are macOS-specific (#3605, #3456).
- Our responsibilities:
  - No editor or app shortcut fires while preedit is non-empty. gpui-kit's editor already
    refuses completions during preedit.
  - The vim layer passes every key through while composing.
  - IME is enabled only in insert, replace and command-line modes.
  - **Spike test S1** (§10): fcitx5-mozc and fcitx5-hangul with Enter, Backspace and
    candidate placement, with vim on and off.
- App identity: `WindowOptions { app_id: Some("frankmd"), .. }` maps to
  `xdg_toplevel.set_app_id`, so it matches `frankmd.desktop` / `StartupWMClass=frankmd`
  (OM-006). Verified in the scaffold.

### 3.2 Clipboard (including images)

- **Read**: the Wayland clipboard reads text, plus images for every `ImageFormat` MIME (PNG,
  JPEG, WebP, GIF, SVG, BMP, TIFF) as `ClipboardEntry::Image { format, bytes }`. That covers
  FrankMD's paste-image flow (FE D1.3/D1.6): `Editor::on_paste` intercepts the paste, and if
  an image entry is present we open the image picker preloaded with it. The primary
  selection is supported too.
- **Write**: text only (GPUI offers only text MIME types on Wayland). FrankMD only needs
  text copy.
- Files copied in a file manager arrive as `text/uri-list` text, which we parse. If image
  writes ever matter, add `arboard` 3.7 with `wayland-data-control`.

### 3.3 Drag and drop from the desktop

`FileDropEvent::{Entered, Pending, Submit, Exited}` with `ExternalPaths` is implemented on
Wayland (`FILE_LIST_MIME_TYPE`). Elements receive it through `.on_drop::<ExternalPaths>()`
and `.drag_over::<ExternalPaths>()`. That covers drag-to-upload into the editor and the
image picker's drop zone.

**Gap:** dragging raw image *data* (not a file), such as from a browser, isn't delivered.
Browsers usually also offer a temp file or URL, and that path works. We'll check it in the
spike.

In-app drag (multi-select tree move) uses GPUI's typed `on_drag`/`on_drop`.

### 3.4 Dialogs, notifications, opening URLs

- `cx.prompt_for_paths` / `cx.prompt_for_new_path` use the **xdg-desktop-portal
  FileChooser through `ashpd`** (OM-011). We don't need rfd.
- `cx.open_url` uses the portal OpenURI. `reveal_path` is available too.
- System notifications are freedesktop notifications via zbus, through GPUI's
  `SystemNotification` and `set_app_identity`. We must verify that we can set
  `desktop-entry=frankmd` and `transient` hints (OM-014). If GPUI doesn't expose them, fall
  back to `notify-rust` 4.18.
- **Single instance** (OM-018): there is no GtkApplication, so `fed path` forwarding and
  `frankmd reload-theme` use a Unix socket via `interprocess` 2.4 in
  `$XDG_RUNTIME_DIR/frankmd.sock`.

### 3.5 Testing support

- `#[gpui_kit::test]` with `TestAppContext` / `VisualTestContext` covers entities, actions,
  subscriptions and async tasks on a deterministic executor. Enable it with
  `gpui-kit/test-support` in `[dev-dependencies]`, which is already wired.
- `gpui_kit::test` provides **UI integration tests**: headless windows, simulated
  pointer/keyboard/scroll, focus/layout/accessibility assertions, `simulate_window_resize`,
  scale-factor changes, and a test clipboard.
- `TestInput` drives IME composition (`compose_text`, `commit_text`) through a real
  `InputHandler`, with UTF-16 ranges. It doesn't replace a real fcitx5 test.
- Pixel tests: `HeadlessAppContext::capture_screenshot` / `Window::render_to_image`, with a
  **Linux WGPU headless renderer** (Mesa lavapipe works in CI).
- This is a real advantage over GTK. Even so, all domain logic is tested without GPUI at
  all (see `architecture.md`).

### 3.6 macOS

gpui-kit supports macOS 15+ through the Xcode CLT (Metal renderer, CoreText, mature IME).
The same code compiles because gpui-kit pulls `gpui-pre-macos`. Everything in §4–§6 is
platform-neutral. macOS is a secondary target that comes almost free. CI should build it,
even if we don't polish it.

---

## 4. Gap (a): a native markdown preview

`TextView` isn't enough: no math, no `sourcepos`, no task toggling, no embeds, and its parser
is `markdown` (markdown-rs 1.0, no commits since 2025-04). We render the preview ourselves.

### 4.1 Pipeline

```
editor text ──(debounce 150 ms, frontend.md 5.10.5)──► background executor
  frankmd-markdown::render(src)          comrak 0.59: GFM tables/tasks/strikethrough/autolink,
                                         footnotes, math_dollars, wikilinks_title_after_pipe,
                                         front_matter_delimiter "---", alerts, sourcepos
    → RenderDoc { blocks: Vec<Block{span: SourceSpan}> }   ← GUI-free, allowlist-by-construction
  tagged with the edit Generation; dropped if a newer edit exists (HP-003)
                                         ▼ foreground
frankmd-app::ui::preview  (one GPUI `list()` item per top-level block, variable heights)
   Block::Paragraph/Heading → StyledText with highlight runs (bold/italic/code/link/wikilink)
   Block::Code              → monospace rows, pre-highlighted TokenClass runs → theme syntax colors
   Block::Table             → grid of divs (column alignment)
   Block::List(task)        → checkbox element; click ⇒ Command::ToggleTask{line} edits the source
   Block::Math / Inline::Math → RaTeX image (§4.3)
   Image                    → img(): vault-relative path resolved via Vault::contain, or http(s) via
                              EgressClient → cache → ImageSource (never a raw file:// from markdown)
   Embed::YouTube           → thumbnail card (§4.5)
   Html                     → allowlisted inline subset (kbd, sub, sup, br, mark, details/summary);
                              anything else shown as escaped text
```

- **The IR is the sanitizer (HP-009).** There is no HTML string and no DOM to clean, so
  there's nothing to patch after sanitizing. `ammonia` is used only by `html::to_html` (the
  CLI `render --html` and the conformance comparison against FrankMD's marked/DOMPurify
  goldens in `tests/fixtures/golden/js_preview_pipeline.json`).
- **Sync scroll** (FE 5.10). The preview is a `ListState`, and its scroll position is
  natively `(item_ix, offset_in_item)`. The editor's top visible line maps through
  `LineMap::block_for_line` to `list_state.scroll_to(ListOffset{item_ix, offset})`, and back
  again with `logical_scroll_top()` → `LineMap::line_for_block` → editor
  `set_scroll_offset`. Within a block we interpolate linearly by line. FrankMD's
  single-source lock (`_scrollSource`, 400 ms), its 5 px / 10 px thresholds, and its
  ratio ≤ 0.01 / ≥ 0.99 snapping are pure functions in `frankmd-workspace`, tested against
  `js_scroll_utils.json` and `js_markdown_line_mapper.json`. Re-anchoring on image load is
  free: list items re-measure, and the anchor is item-relative.
- **Text selection and copy** in the preview reuse gpui-base's window selection engine, the
  same one TextView uses for custom renderers (`text_selection`, `selectable_text`).
- **Zoom** (FE 5.11) is a rem scale on the preview subtree.
- Budgets (HP-017): highlight at most N blocks / M bytes per render, defer the rest, and
  render off the UI thread.

### 4.2 Code highlighting in the preview

- In the app, we use gpui-kit's own highlighter registry (tree-sitter, the same grammars and
  the same `highlight.syntax` theme as the editor), so code looks identical in the editor
  and the preview and follows Omarchy colors automatically. We add `tree-sitter-<lang>`
  features for FrankMD's roughly 20 fenced languages.
- `frankmd-markdown` defines a `CodeHighlighter` trait. The app implements it with the
  gpui-kit registry. The headless CLI and tests use `syntect` 5.3 + `two-face` 0.5 (pure
  Rust, `default-fancy`) mapped to the same `TokenClass` set.
- Expect conformance for the language alias table only (`canonical_language`, from
  `js_code_block_highlighter.json`). Token classes come from different engines (Lezer vs
  tree-sitter) and won't match byte for byte. Lock them with insta snapshots instead.

### 4.3 Math: pick **RaTeX**

| Option | Verdict |
|---|---|
| **RaTeX** (`ratex-parser`/`-layout`/`-render`/`-svg` 0.1.15, 2026-10-16, erweixin/RaTeX ≈1.6k★, MIT) | **Chosen.** Pure Rust and KaTeX-compatible syntax ("> 99.5 % coverage" per its support table), including mhchem `\ce`. Embeds the KaTeX fonts as TTF (`ratex-katex-fonts`), so the extracted `assets/fonts/katex/*.woff2` files aren't needed. Output is a display list rendered to a raster (tiny-skia) or SVG. No JS, no webview. |
| `ratex-gpui` 0.4.1 (zorite, 2026-09-22) | Useful *reference*: RaTeX → GPUI image, plus a MathQuill-style editor. It depends on `gpui-pre ^0.3`, so it resolves to our snapshot. Its adoption is tiny (≈600 downloads), so we copy the approach rather than take the dependency. |
| `katex-rs` 0.3 | It produces KaTeX **HTML + CSS**, which only makes sense with a webview. Use it only if we ever want HTML-export parity. |
| Headless KaTeX (QuickJS/V8 + SVG rasterizing) | Rejected: it means shipping a JS engine, it's slower, and RaTeX already reaches KaTeX parity. |
| `latex-rust` 3.0 | Tied to egui/epaint. Rejected. |
| `pulldown-latex` → MathML | Needs a MathML layout engine we don't have. Rejected. |
| typst/mitex | Far too heavy for inline math. |

How we use it:
- **v1:** RaTeX layout → `ratex-render` raster at `device_scale × font_px`, tinted with the
  theme foreground → `RenderImage`, inside an LRU cache keyed by `(tex, display/inline,
  size, color, scale)` and run on the background executor. Inline math sits on the text
  baseline as an inline image in a wrapped flex run. Display math is a centered block.
- **v2 (if zoom crispness matters):** paint the RaTeX display list straight into GPUI
  (glyphs from the KaTeX fonts registered with GPUI's text system, plus `paint_quad` for
  rules and paths). It's resolution-independent and themable.
- **[COMPAT]** FrankMD renders math *after* sanitizing and shows the raw TeX on error
  (`throwOnError:false`), so we do the same: on a RaTeX error we draw the source in the
  error color. Golden: `js_math_renderer.json` gives the delimiters and inline/display
  detection.

### 4.4 Images

- Vault images resolve through `Vault::contain` (symlink-safe, HP-008) and load with
  `img(path)`. Decoding runs on GPUI's image cache on a background thread.
- Remote images are fetched by `frankmd-net::EgressClient` (public IPs only, 25 MB cap)
  into `$XDG_CACHE_HOME/frankmd/img/`. They're never fetched straight from the render path
  with an unguarded client.
- SVG goes through resvg, the same one GPUI uses internally.

### 4.5 YouTube embeds as thumbnail cards

A webview is impossible on Wayland, and wrong anyway for a fast app. So `Embed::YouTube{id,start}`
renders as a 16:9 card: the thumbnail `https://i.ytimg.com/vi/<id>/hqdefault.jpg` (through the
egress client, cached), a play glyph overlay, and the title if known (fetched with the
configured YouTube API key, otherwise none). Click → `cx.open_url("https://www.youtube.com/watch?v=<id>&t=<start>")`.
The markdown source is **unchanged** (**[COMPAT]**: notes keep the same iframe or URL syntax
the original inserts).

---

## 5. Gap (c): typewriter mode

FrankMD (FE A.10) pads the bottom by half the viewport and centers the caret after every
selection change, except during mouse selection or with a non-empty selection. Only the
bottom is padded, so the first lines are never centered.

gpui-kit gives us both knobs:
- `scroll_beyond_last_line(Some(rows/2))` reproduces `padding-bottom = clientHeight/2`.
- `cursor_surrounding_lines(Some(rows/2))` is vim-style `scrolloff`: it keeps the caret
  row in the middle as it moves, and edge clearance gives way at the top of the document.
  That matches the original's behaviour.

The view recomputes `rows` from the editor bounds on every resize and visibility change, in
the `on_resize` of the pane. That's the HP-019 lesson: never compute layout while hidden.
The exceptions (mouse drag, non-empty selection) are handled by temporarily setting
`cursor_surrounding_lines(None)` while a selection exists, so we never dispatch an extra
selection transaction that could kill a popup (the HP-019 / #230 lesson).

The preview side (top-align the block of the caret line, 10 px threshold, smooth) is the
sync-scroll machinery from §4.1, driven by cursor moves instead of scroll. Column width:
`editor_width` in ch → `max_w(ch * mono advance) + mx_auto()`.

This only works if `cursor_surrounding_lines` keeps the caret centered on soft-wrapped
lines. That's spike test S5.

---

## 6. Gap (b): vim mode

### 6.1 Constraints found in the source

- gpui-kit has no vim, and gpui-kit#2617 is open. Zed's `vim` crate is welded to Zed's
  `Editor`, so it can't be used.
- What `EditorState` exposes publicly is enough for most of vim:
  - `text() -> &Rope`, `selected_range`, `set_selected_range(_with_direction)`,
    `cursor_position`, `set_cursor_position`
  - `replace(text)` and `insert(text)`: an atomic edit at the selection
  - `set_value`, `visible_row_range`, `set_scroll_offset`, `cursor_bounds`,
    `range_to_bounds`, `display_map()` (wrap rows)
  - search (`set_search_query` …), decorations
  - Undo and Redo are dispatched as actions.
- Missing:
  1. **Grouping several edits into one undo step.** `EditIntent` is private. Today, `ciw`
     is one `replace` and therefore one step, which is fine. But `.` repeat of an insert
     session, `:s///g` and block-visual inserts would take several steps.
  2. **Block cursor shape.** No API for it.
  3. Intercepting keys *inside* the editor's key context.

### 6.2 Design

```
GPUI keystroke ──► editor pane div().capture_key_down(...)   (capture phase: root → focused,
                   runs BEFORE the Editor's own key bindings)
                     │  KeystrokeEvent → frankmd_vim::keymap::Key
                     ▼
               frankmd_vim::VimState::feed(key, composing, &mut EditorBuffer)
                     │ Handled      → cx.stop_propagation()
                     │ PassThrough  → editor handles (insert-mode text, IME, Ctrl-F/N/P/E/B/I/V on
                     │                non-macOS = [COMPAT] PASSTHROUGH list → app keymap)
                     │ App(cmd)     → same Command as the shortcut (HP-018)
                     ▼
               bridge::vim::EditorBuffer implements VimBuffer over Entity<EditorState>
```

- **`frankmd-vim` is GUI-free.** It's a hand-written state machine: mode, pending operator,
  count, register, motion or text object → `Action`. It's tested with a `String`-backed
  `VimBuffer` in plain unit tests. That's thousands of cases ported from `@replit/codemirror-vim`'s
  behaviour for FrankMD's scope, and they run without a window.
- Scope = FE A.9:
  - Modes: normal, insert, visual (char, line, block via `<C-q>`), replace.
  - Counts. Operators `d c y > < g~ gu gU =`.
  - Motions: `h j k l w b e W B E 0 ^ $ gg G f t F T ; , % { } H M L`, `Ctrl-D`/`Ctrl-U`.
  - Text objects `iw aw i" a" i( a( ip ap`.
  - Registers `" a-z 0 +`. `.` repeat. `u` / `Ctrl-R`. `/ ? n N * #`. Marks `m'`. `zz zt zb`.
  - `:s`, `:noh`, `:<n>`.
  - FrankMD ex-commands `:w :q :wq :x :help :e :find :Explore :n :prev`, with **[COMPAT]**
    semantics: `:q` closes dialogs, not the note.
  - Mode indicator `VIM · NORMAL`.
  - The non-macOS Ctrl passthrough. On macOS the full vim keymap is kept.
- **The ordering invariant** (FrankMD test `cc83922`) becomes structural: capture-phase
  handlers always run before the editor's bindings, and a GPUI UI test asserts that `j`
  in normal mode moves the caret instead of inserting "j".
- Undo grouping: until #2617 or an equivalent lands, compound changes are done as **one
  computed `replace`** over the union range. The engine diffs the old text against the new
  text for that range, so one change is one undo step. An insert session is recorded as a
  replay script for `.`. A small upstream PR to expose `EditorState::transact(|s| …)` is
  worth offering, and it may be the patch that closes #2617.
- Block cursor: draw a `RangeDecoration::Fill` over the grapheme under the caret, colored
  `accent` (**[COMPAT]** FrankMD's themed fat cursor), and make the caret transparent
  through the editor text style. Insert mode restores the beam.
- IME: `feed` returns `PassThrough` whenever preedit is active. On a mode change the bridge
  calls `window.invalidate_character_coordinates()` and toggles text input, so fcitx5 is
  off in normal mode (OM-007).
- Fallback if the capture approach breaks on some binding: wrap the editor in our own key
  context `FrankVim` with explicit `KeyBinding`s, which take precedence by context depth.

---

## 7. Gap (d): RTL without bidi in GPUI

Facts:
- gpui-kit#3679 was closed 2026-10-27: "GPUI doesn't support RTL layout or text yet."
- zed#31102 is open. Per its latest comments, GPUI takes a line's base direction from its
  first strong character, apps can't set paragraph direction, and the Windows bidi fix
  (#57237) is unmerged.
- gpui-kit#3442 is open: wrapped Arabic/English paragraphs.
- On Linux, cosmic-text shapes Hebrew glyphs correctly. The breakage is ordering, wrapping
  and caret movement in mixed lines.

Strategy, in tiers:
1. **Ship the `he` locale.** All strings are translated and loaded by `frankmd-i18n`. The
   gpui-kit overlay strings are translated too.
2. **Mirror the layout we own.** Our layout helpers take a `Direction` from
   `Locale::is_rtl()`:
   - pane order (explorer on the right),
   - `h_flex` child order in our own toolbars and rows,
   - `text_align(Right)` for labels and our dialogs,
   - mirrored chevrons and icons.

   gpui-kit's internals (menus, inputs) stay LTR. That's a documented, visible limitation.
3. **Preview:** render per-paragraph direction ourselves. `unicode-bidi` 0.3 gives the
   paragraph level (first strong character, like HTML `dir=auto`, which FrankMD's CSS
   effectively does). RTL paragraphs are right-aligned, and their runs are reordered
   visually per wrapped line. Evaluate **`gpui-bidi` 0.1.1** (zorite, built on `gpui-pre
   ^0.3`; correct caret, selection and hit-testing over reordered glyphs, plus row layout
   and painting) as the implementation. It's small and young (≈1k downloads), so we vendor
   it or treat it as a reference.
4. **Editor:** v1 accepts GPUI's behaviour. Pure-Hebrew lines render right, mixed lines can
   be visually wrong, and caret movement is logical. We expose a "paragraph direction" hint
   (right-align RTL-dominant lines through a decoration and text style) where gpui-kit
   allows it, and we track upstream.
5. **Gate:** the release notes and the locale picker flag Hebrew editing as "preview
   quality". Spike test S2 records the exact behaviour. This is the main product
   regression against the original. We accept it because the user chose GPUI.

---

## 8. Gap (e): what changes from components.md because of GPUI

| Area | components.md (GTK) | GPUI decision |
|---|---|---|
| Async runtime | Relm4 + tokio | **GPUI executors for UI and background CPU work. One dedicated tokio runtime owned by `frankmd-net::NetRuntime` (2 workers) for reqwest, aws-sdk-s3 and genai.** Bridge: `rt.spawn(fut)` → await the `JoinHandle` inside `cx.background_executor().spawn(..)` → `cx.spawn` back to the entity (`frankmd-app/src/bridge/tokio.rs`, the same pattern as Zed's `gpui_tokio`). GPUI itself pulls in **no** tokio (verified with `cargo tree -i tokio`). Channels are `async-channel` (runtime-agnostic). Sync file IO runs on GPUI's background executor. |
| Editor | GtkSourceView + VimIMContext | gpui-kit `Editor` + `frankmd-vim` (§6) |
| Editor highlighting | `.lang` files | `tree-sitter-md` through gpui-kit, plus decorations for wikilinks, `$math$` and front matter |
| Preview | WebKitGTK + katex-rs | Native IR renderer (§4) + RaTeX. **katex-rs and webkit6 are dropped.** ammonia stays only for HTML export. |
| Theming | GTK CSS + GtkSourceView XML | `ThemeConfig` built at runtime (§2.1) |
| Clipboard | `gdk::Clipboard` | GPUI clipboard (reads images). `arboard` only if image *writes* are needed. |
| Drag and drop | `GtkDropTarget` | GPUI `ExternalPaths` + typed drags |
| File dialogs | `gtk::FileDialog` | GPUI `prompt_for_paths` (ashpd portal). rfd isn't needed. |
| Notifications | GNotification | GPUI `SystemNotification`, or `notify-rust` if hints are missing |
| Single instance / `fed path` | GtkApplication D-Bus | `interprocess` local socket (`$XDG_RUNTIME_DIR/frankmd.sock`) |
| UI tests | weak (headless Wayland + AT-SPI) | `#[gpui_kit::test]` + `gpui_kit::test` headless UI tests, plus WGPU screenshots |
| Logging | GLib log bridge | `tracing` only. GPUI logs through `log`, so bridge it with `tracing-log`. |
| Fonts | Pango | cosmic-text + fontconfig. Set the theme fonts from `fc-match`. |
| i18n | custom crate | Unchanged (custom `frankmd-i18n`), plus a `rust-i18n` overlay for gpui-kit's own strings |

Everything else in components.md stays as written: comrak, syntect for headless use,
nucleo, the ripgrep libraries, similar, notify, aws-sdk-s3, genai, serde-saphyr, etcetera,
jiff, and the hand-written `.fed` parser.

---

## 9. Build status and system packages (this machine)

- `cargo check --workspace` passes with 0 warnings. `cargo clippy --workspace` reports only
  the expected `clippy::todo` warnings on skeleton bodies.
- `cargo build -p frankmd-app` → `target/debug/frankmd` (≈500 MB debug, unstripped). A cold
  build of the GPUI tree takes about 2 minutes on a 7950X3D.
- `./target/debug/frankmd` opens a Wayland window: Vulkan through wgpu on the RTX 5090. The
  log shows the fallback "DejaVu Sans Mono not installed → Noto Sans Mono" (fixed by §2.1
  font resolution).
- No packages were missing. The Arch equivalents of gpui-kit's Ubuntu list (wayland,
  libxkbcommon, fontconfig, vulkan-icd-loader plus a driver, clang for tree-sitter) are
  already installed. **Do not** add `gpui-webview`: it would require webkit2gtk-4.1 and
  XWayland.

## 10. Risks and the spike checklist (run before feature work)

| # | Risk | Mitigation / spike test |
|---|---|---|
| R1 | **API churn**: weekly gpui-kit 0.x releases and gpui-pre snapshots | Exact pin plus `Cargo.lock`. Bump monthly behind the UI test suite. All logic stays in GUI-free crates, so churn only touches `frankmd-app`. |
| R2 | **RTL regression** against the original | §7. S2: Hebrew-only, mixed and wrapped lines, recorded with screenshots. |
| R3 | **Vim fidelity** without an undo-group API | §6. S3: port codemirror-vim golden behaviours, and offer the upstream `transact` PR. |
| R4 | IME edge cases on Wayland (zed#65302, #65650) | S1: fcitx5-mozc and hangul, vim on and off, a candidate window at the caret, and Enter commits. |
| R5 | Large-note performance | S4: 5 MB and 20 MB notes. Measure open time and keystroke-to-paint (< 16 ms, release build), with the preview on and off (incremental render budget). |
| R6 | Typewriter mode with soft wrap | S5: `cursor_surrounding_lines` centering on wrapped rows. |
| R7 | Preview fidelity (tables, footnotes, math baseline) | S6: render FrankMD's `js_preview_pipeline.json` corpus and compare visually. Snapshot the IR. |
| R8 | Theme hot reload latency | S7: Omarchy switch → repaint in < 100 ms. |
| R9 | Bus factor: gpui-kit is one company's framework (Longbridge, production user) | Kit pins upstream Zed GPUI snapshots, and our app depends only on stable widget concepts. Worst case, we fork at the pinned tag. |
| R10 | `gpui-fast` vs upstream snapshot | S8: A/B the same binary with `--features gpui-kit/gpui-fast` on S4. Adopt only if it is clearly faster and stable. |
