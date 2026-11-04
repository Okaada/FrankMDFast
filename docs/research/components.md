# FrankMDFast — Component Research

*Researched 2026-11-04. Versions, release dates and download counts are live from the crates.io API (`/api/v1/crates/<name>`). Stars and last-push dates come from the GitHub API, and system library versions from Arch's `pacman -Si` on this machine. "Recent" means crates.io's 90-day download count. Re-check anything older than about a month before you pin it.*

Source app: `~/Projects/FrankMD` (Rails 8.1, CodeMirror 6.39, marked 15, KaTeX 0.19, DOMPurify, `@replit/codemirror-vim` 6.3, `ruby_llm`, `aws-sdk-s3`).

---

## 0. TL;DR

| Decision | Pick |
|---|---|
| **GUI stack** | **GTK4 (`gtk4` 0.11) + Relm4 0.11 for the app shell, GtkSourceView 5 (`sourceview5` 0.11) as the editor, WebKitGTK 6 (`webkit6` 0.6) as the preview surface.** Plain GTK4 styled by our own CSS generated from Omarchy tokens. No libadwaita, or libadwaita only for widgets that don't fight the theme. |
| **Fallback** | **GPUI + `gpui-kit` 0.9** (longbridge). It is the best native alternative, but GPUI has **no RTL text or layout support today**, and its maintainers confirmed that on 2026-10-27. Re-evaluate when zed#31102 lands. |
| **Escape hatch** | **Tauri 2 + CodeMirror 6.** Use it only if "native" stops being the goal. It is the cheapest route to feature parity because it reuses the current frontend, but it is the current app minus Rails, with all of WebKitGTK's Linux performance problems. |
| Markdown | `comrak` 0.59: GFM, footnotes, math, wikilinks, front matter, `sourcepos` for sync scroll, and an AST for backlinks. |
| Math (preview) | `katex-rs` 0.3: KaTeX-compatible HTML+MathML rendered in Rust, with the KaTeX CSS and fonts already vendored. Native math rendering: `ratex-svg`/`ratex-render` 0.1.15. |
| Code highlight (preview) | `syntect` 5.3 + `two-face` 0.5 through comrak's built-in syntect adapter, emitting **classed** HTML so the CSS comes from Omarchy tokens. Alternative: `arborium` 2.18. |
| S3 | `aws-sdk-s3` 1.157 (`endpoint_url` + `force_path_style`). Lighter alternative: `rust-s3` 0.38. |
| LLM | `genai` 0.7 for chat, covering all 5 providers natively. Image generation is about 80 lines of raw `reqwest` against OpenRouter, the only provider FrankMD uses for images today. |
| YAML | `serde-saphyr` 2.1. **Avoid** `serde_yaml` (deprecated) and `serde_yml` (RUSTSEC-2025-0068, unsound, archived). |
| i18n | **Custom small crate** that reads the Rails YAML as-is and implements Rails' *default* pluralizer (zero/one/other, not CLDR) and `%{var}` interpolation. |
| `.fed` config | **Hand-written line-based parser.** No crate preserves this exact format, including commented-out defaults that need uncommenting on write. |
| Slug | **Hand-port** `HugoService.slugify` and its `ACCENT_MAP`. No crate matches the Ruby output. |

Before anything else, run a **3-day spike** to confirm the decision with the gate tests in §1.6.

---

## 1. The big decision: the GUI stack

### 1.1 Hard gates (from the feature list)

A stack must pass all of these on Omarchy (Hyprland, Wayland):

1. **The editor is code-editor grade**: highlighting, undo, regex find/replace, smooth on multi-MB notes, and typewriter mode. Vim mode is required as an option.
2. **IME through fcitx5 on Wayland** (text-input-v3), for CJK users (ja and ko locales ship).
3. **RTL/bidi for Hebrew**: a `he` locale ships, so the UI must mirror and editing mixed Hebrew/English must work.
4. **Rich GFM preview**: tables, task lists, footnotes, KaTeX math, highlighted code, images, YouTube iframes, wikilinks.
5. **Themes at runtime** from Omarchy colour tokens, hot-reloaded when the theme changes.

### 1.2 Candidates and current evidence

#### GPUI + gpui-kit (formerly gpui-component)
- `gpui` 0.2.2 (2026-10-30, 435k downloads), Zed (zed-industries/zed, 92.7k★, active daily). `gpui-kit` / `gpui-component` / `gpui-base` 0.9.0 (2026-11-02). Repo renamed to **longbridge/gpui-kit** (19k★), with **weekly 0.x releases** (0.7.0 on 09-28, 0.7.1, 0.7.3, 0.8.0, 0.8.1, 0.9.0 on 11-02). <https://github.com/longbridge/gpui-kit>
- It has a lot: more than 75 components, Dock, Tree, virtual lists, an editor that claims "stable performance at 200K lines" with tree-sitter (`tree-sitter-md` included) and LSP hooks, a Markdown/HTML `TextView` (built on `markdown-rs` with `math_text`/`math_flow` constructs and a plugin trait, but **no built-in math renderer**), AccessKit accessibility, and **headless UI integration testing**. Used in production by Longbridge Pro.
- **The blocker:** on gpui-kit#3679 ("rtl support is missing", closed 2026-10-27) the maintainer answered: *"GPUI doesn't support RTL layout or text yet, so we can't implement it in GPUI Kit for now."* <https://github.com/longbridge/gpui-kit/issues/3679>. The Zed tracking issue zed#31102 is still open with 42 comments. Per that thread, recent fixes (e.g. #57237 for Windows DirectWrite bidi) are still under review, and the base direction comes from the first strong character. Wrapped Arabic/English paragraphs are still broken (gpui-kit#3442). <https://github.com/zed-industries/zed/issues/31102>
- **No vim** for the gpui-kit editor. Zed's `vim` crate is tied to Zed's own `Editor`, and gpui-kit#2617 asks for public editing primitives so a vim layer can be built, but it is still open. <https://github.com/longbridge/gpui-kit/issues/2617>
- **No webview on Wayland:** `gpui-webview` (wry) states "Linux (Wayland) — Not supported; run on XWayland". A WebKit-based preview is therefore off the table, and math needs a native renderer (RaTeX → SVG → image).
- IME on Linux: Zed's fcitx5 bugs (Enter committing a newline, candidate window placement) were fixed during 2025–26 (zed#35537, #31337, #36831 all closed). In gpui-kit, the remaining IME issues are macOS-specific edge cases.
- Fragmentation: `gpui` (Zed), `gpui-ce` 0.3 (community fork, 1.4k★), and longbridge's `gpui-fast` 0.1.13. gpui-kit pins its own GPUI snapshot. API churn is high.
- Large files: gpui-kit#2566 (5 GB RAM on a 500 MB file) was closed. The cause was the demo parsing the text as JSON and running every extra feature; with those off it behaves like Zed.

#### iced 0.14
- 0.14.0 (2025-12-07), **no release in 11 months**, 31.9k★. Added IME support, headless and end-to-end testing, and incremental markdown with task lists. <https://github.com/iced-rs/iced/blob/master/CHANGELOG.md>
- RTL issues are still open (#250, #1877, #2102). **No accessibility** (#552 open). `text_editor` is a cosmic-text editor: no vim, no multi-cursor, and only basic syntax highlighting (syntect). The markdown widget has no math, footnotes or embeds. It fails gates 1, 3 and 4.

#### Slint 1.18
- 1.18.1 (2026-09-21), 24.4k★, stable API, good accessibility and IME, a small runtime. But it has **no code editor widget** (`TextEdit` is a plain field) and no rich-text preview, so it fails gates 1 and 4. Licensing is GPL, royalty-free, or commercial. <https://github.com/slint-ui/slint>

#### egui 0.37
- 0.37.0 (2026-10-15), 32k★, huge adoption. Immediate mode, no RTL/bidi, weak IME on Wayland, and `TextEdit` is not code-editor grade. It fails gates 1–3. Good for tools, wrong for a writing app.

#### GTK4 via gtk4-rs (+ Relm4) + GtkSourceView 5 + WebKitGTK 6
- Bindings: `gtk4` 0.11.5 (2026-09-20, 5.4M downloads), `glib`/`gio` 0.22, `sourceview5` 0.11.2 (2026-07-23), `webkit6` 0.6.1 (2026-03-11), `libadwaita` 0.9.2, `relm4` 0.11.0 (2026-04-08, 2k★, tokio-based async commands). **All of them are on the same gtk4 ^0.11 / glib ^0.22 line**, which I checked through the crates.io dependency API.
- System libraries on this Arch box: gtk4 4.24.1, gtksourceview5 5.22.0, webkitgtk-6.0 2.54.1, fcitx5 5.1.23.
- Editor (GtkSourceView 5): highlighting through `.lang` specs (Markdown included), undo/redo, `SearchContext` with regex and replace, **built-in Vim emulation** `GtkSourceVimIMContext` (since 5.4: normal, insert, replace and visual modes, a `:` command bar, registers, marks, search and replace), completion, gutters and space drawing. Style schemes are XML that we can **generate at runtime from Omarchy tokens**. <https://gnome.pages.gitlab.gnome.org/gtksourceview/gtksourceview5/class.VimIMContext.html>
- IME: GTK is the reference Wayland client (text-input-v3), and fcitx5 works out of the box with no `GTK_IM_MODULE` needed on Wayland. VimIMContext explicitly chains to the system IM context in insert/replace mode. Verify this with fcitx5-mozc in the spike.
- RTL: Pango + HarfBuzz + fribidi. GTK **auto-mirrors layouts** when the locale is RTL (`gtk::Widget::set_default_direction`), and GtkTextView handles mixed bidi with logical cursor movement. This is the best support of any candidate.
- Accessibility: GTK4's AT-SPI backend works with Orca. Best of the native candidates.
- Preview: WebKitGTK 6 renders the HTML that comrak produces in Rust (KaTeX output prerendered by katex-rs, classed syntect HTML, YouTube iframes, local images through a custom URI scheme). Sync scrolling uses `UserContentManager` script messages plus comrak's `data-sourcepos`.
- Theming: `gtk::CssProvider::load_from_string` at runtime, plus regenerated SourceView style schemes and CSS variables pushed into the WebView. Hot reload takes milliseconds.
- **Desktop integration comes free with GtkApplication:** single instance through D-Bus uniqueness, `fed path` forwarding through `HANDLES_COMMAND_LINE`/`HANDLES_OPEN`, `GNotification`, `gtk::FileDialog` (portal-backed), `GtkDropTarget` (files and textures from the desktop), and `gdk::Clipboard` (including image textures).
- Weaknesses:
  - **No multiple cursors** in GtkSourceView.
  - Very long single lines are slow. Multi-MB notes are fine.
  - WebKitGTK spawns a web process, which adds about 80–150 MB RSS and startup latency. Create it lazily, or keep it warm off screen.
  - NVIDIA + DMABUF quirks (Tauri's docs list `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `__NV_DISABLE_EXPLICIT_SYNC=1`). <https://v2.tauri.app/develop/debug/linux-graphics/>
  - GTK 4.16+ defaults to the Vulkan renderer, so keep `GSK_RENDERER=ngl` as a known workaround.
  - **macOS is second-class:** GTK4 and gtksourceview5 build from Homebrew, but **WebKitGTK is not available on macOS**, so the preview needs a native fallback there (§1.5).
  - GTK UI tests are weak. Mitigate by keeping logic in GUI-free crates and testing Relm4 `update()` as plain functions.

#### Tauri 2 / wry (and Dioxus desktop)
- `tauri` 2.12.5 (2026-10-21, 112k★), `wry` 0.57.1, `dioxus` 0.7.10 (39.6k★). On Linux both use **WebKitGTK**. CEF and Servo backends are discussed, but there is no ETA (tauri discussion #8524, wry#1064).
- This option reuses CodeMirror 6, codemirror-vim, marked and KaTeX unchanged, so it has the best feature-parity cost. Its IME and RTL are as good as WebKit's.
- But the whole UI pays WebKitGTK's scrolling, compositing, NVIDIA and memory costs (typically 150–250 MB), which contradicts "FrankMDFast". It is the existing JS app with a Rust backend instead of Rails. Dioxus *native* (Blitz 0.2, `blitz-dom` 0.2.4) is still alpha.

#### Freya 0.4, Floem, Makepad, Xilem, Vizia
- **Freya 0.4.3** (2026-10-25, 3.3k★, Skia): the closest to viable, with Skia paragraph bidi and an editable text component, but it has a small community and no code-editor-grade widget.
- **Floem** 0.2.0 (last release 2024-11) has stalled. **Makepad** 1.0 (2025-05) is niche. **Xilem/Masonry** 0.4 is alpha. **Vizia** 0.4 has about 7k downloads. None is suitable for the primary stack.
- **NSH** (nsh.dev, longbridge, v0.4) wraps native toolkits, GTK 4 included on Linux. It is too new.

### 1.3 Scorecard (1 = poor, 5 = excellent, for *this* app on Omarchy)

| Criterion | GPUI + gpui-kit | iced 0.14 | Slint | egui | **GTK4 + GSV5 + WebKitGTK** | Tauri2 + CM6 | Freya |
|---|---|---|---|---|---|---|---|
| Editor quality / large docs | **5** | 2 | 1 | 2 | 4 | 4 | 2 |
| Vim mode available | 1 | 1 | 1 | 1 | **5** (built in) | **5** | 1 |
| IME fcitx5 / Wayland | 4 | 3 | 3 | 2 | **5** | 4 | 3 |
| RTL / bidi (Hebrew) | **1 (gate fail)** | 2 | 3 | 1 | **5** | 5 | 3 |
| CJK shaping | 4 | 4 | 4 | 3 | **5** | 5 | 4 |
| Rich preview incl. math | 3 (DIY plugins) | 2 | 1 | 2 | **5** | 5 | 2 |
| Runtime theming from tokens | 5 | 4 | 3 | 4 | 4 | 5 | 4 |
| Accessibility | 4 (AccessKit) | 1 | 4 | 3 | **5** (AT-SPI) | 4 | 3 |
| Wayland / Hyprland behaviour | 4 | 4 | 4 | 4 | **5** | 3 | 4 |
| Startup & memory | **5** | 4 | 5 | 5 | 3 | 2 | 4 |
| Maturity / API stability | 2 (weekly 0.x, 3 GPUI variants) | 3 | 5 | 4 | **5** | 5 | 2 |
| Docs / community | 3 | 4 | 4 | 5 | 4 | 5 | 2 |
| Testability | **4** (headless UI tests) | 4 | 4 | 3 | 2 | 3 | 3 |
| macOS support | **5** | 5 | 5 | 5 | 2 (no WebKitGTK) | 5 | 4 |
| **Passes all hard gates?** | **No** (RTL, vim) | No | No | No | **Yes** | Yes (but not native) | No |

### 1.4 Recommendation

**Build on GTK4 + Relm4 + GtkSourceView 5 + WebKitGTK 6.**

Rationale:
- It is the only *native* stack that passes every hard gate today. RTL with UI mirroring, fcitx5 IME and vim mode are things GTK already ships; on GPUI or iced we would have to write them or wait for upstream.
- It integrates best with Omarchy (Hyprland, Wayland, portals, mako notifications, D-Bus single instance). Its accessibility is the strongest of any candidate.
- The preview is where we need web-grade layout (tables, KaTeX, iframes, images). WebKitGTK confined to the preview pane gives us that. A native renderer would mean reinventing an HTML/CSS layout engine.
- The ABI is stable: GTK 4.x and GtkSourceView 5.x are long-lived, and the gtk-rs crates move on a predictable twice-yearly cadence.

How to use it:
- **Relm4 for the shell**: the sidebar tree (`gtk::ListView` + `TreeListModel`), dialogs, the fuzzy finder popover and the settings view. Its Elm-style `update()` is unit-testable and its async commands already run on tokio.
- **Raw gtk4-rs / sourceview5 for the editor widget** (custom `EditorView` wrapper) **and the preview widget**.
- **No libadwaita by default.** Its stylesheet and recoloring would fight the Omarchy theme. We ship our own CSS generated from the tokens.
- **Typewriter mode**: large top/bottom margins plus `scroll_to_mark(insert, 0.0, true, 0.0, 0.5)` on every cursor move.
- **Regex consistency**: GtkSourceView's search uses PCRE2, but full-text search uses Rust `regex`. To get one dialect, editor find/replace should run Rust `regex`/`fancy-regex` over the buffer text and apply tags itself. `SearchContext` is still useful for highlight-all.

**Fallback: GPUI + gpui-kit.** Switch if:
- (a) the spike shows GtkSourceView/WebKitGTK can't meet latency or memory targets, or
- (b) RTL editing is consciously downgraded, or GPUI ships RTL (watch zed#31102, the bidi PRs referenced there, and gpui-kit#3679).

Because all domain logic sits in GUI-free crates (§3), the switch replaces only `fmd-app`. On GPUI the preview becomes a native `TextView` with a RaTeX math plugin, and vim becomes our own layer, which needs gpui-kit#2617.

**Escape hatch: Tauri 2 + CM6**, only if parity-at-any-cost beats native.

### 1.5 Risks (GTK route) and mitigations

| Risk | Mitigation |
|---|---|
| WebKitGTK memory (+80–150 MB) and first-paint latency | Create the WebView lazily (preview hidden at startup → zero cost) or prewarm it after the first frame. A single web process. Render diffs in the editor, not the WebView. |
| WebKitGTK and GTK Vulkan quirks on NVIDIA | Document the env overrides (`WEBKIT_DISABLE_DMABUF_RENDERER=1`, `GSK_RENDERER=ngl`) and expose them in `.fed`. Test on Omarchy NVIDIA hardware in the spike. |
| macOS: no WebKitGTK | Hide the preview behind a `PreviewBackend` trait. On macOS, use a native `GtkTextView` preview (tags plus embedded pictures, math through RaTeX → SVG → `gdk::Texture`). macOS is "buildable, reduced preview", not a primary target. |
| No multiple cursors | Accept for v1 (the original doesn't advertise it). Column or block selection through Vim visual-block is a partial substitute. |
| GTK UI testing is weak | Push ≥90 % of the logic into pure crates. Unit-test Relm4 `update()`. Run a small smoke suite under headless Wayland (`weston --backend=headless` or `cage`) and AT-SPI (`atspi` crate) for a few end-to-end flows. |
| VimIMContext + fcitx5 interplay and vim feature coverage below codemirror-vim | Spike gate test. If it's insufficient, layer our own keymap over `EventControllerKey`. |
| gtk-rs ergonomics (`clone!`, subclassing, `Rc<RefCell>`) | Use Relm4 for most UI. Write subclass widgets only where needed (editor, preview). |
| Relm4 bus factor (2k★, a small core team) | Relm4 is a thin layer: worst case we drop to raw gtk4-rs component by component. |

### 1.6 Spike acceptance tests (3 days, before committing)

1. fcitx5-mozc and fcitx5-rime: commit with Enter, candidate window at the caret, both with VimIMContext on and off.
2. A Hebrew + English mixed line: cursor movement, selection, copy. Also a `he` locale UI mirror.
3. A 5 MB and a 20 MB markdown note: open time, typing latency (<16 ms), scroll.
4. Cold start to editable text under 300 ms. RSS with the preview hidden and with it shown.
5. Omarchy theme switch → editor, sidebar and preview recolored in under 100 ms.
6. Preview: KaTeX through katex-rs, tables, a YouTube iframe, a local image via the custom scheme, sync scroll both ways.
7. Run the same 1–3 against the gpui-kit `editor` example for a head-to-head record.

---

## 2. Component table

Legend: ✅ recommended, 🔁 alternative, ⛔ avoid.

### 2.1 Markdown, preview, highlighting, math

| Need | Recommended | Version (release) | Signal | Alternatives | Notes / avoid |
|---|---|---|---|---|---|
| Markdown → AST/HTML (GFM) | ✅ **`comrak`** | 0.59.0 (2026-10-26) | 9.9M downloads, 1.7k★, active | 🔁 `pulldown-cmark` 0.13.6 (2026-10-30, 197M downloads): faster streaming parser with tables, tasks, footnotes, math and wikilinks | comrak covers everything in one place: tables, task lists, footnotes, `math_dollars`, `wikilinks_title_after_pipe`, front matter, alerts, an arena AST (for backlinks and the YouTube transform) and **`sourcepos` → `data-sourcepos`** for sync scroll. ⛔ `markdown` (markdown-rs) 1.0.0: no commits since 2025-04, so don't make it the primary parser (gpui-kit uses it). Output won't be byte-identical to `marked`, so lock it with insta snapshots. |
| HTML sanitising | ✅ `ammonia` | 4.2.4 (2026-10-17) | 20M downloads | — | Replaces DOMPurify. Allowlist `iframe` only for `youtube-nocookie.com`. |
| Code highlight in preview | ✅ `syntect` + `two-face` | 5.3.0 (2025-09-27) / 0.5.2 (2026-08-07) | 38M / 15M downloads | 🔁 `arborium` 2.18.2 (2026-08-28, 510★): ~70 tree-sitter grammars and an HTML renderer. 🔁 `lumis` 0.19 (2026-10-28, formerly `autumnus`): tree-sitter plus 200 Neovim themes, which matches Omarchy's Neovim themes. 🔁 `giallo` 0.5 (Zola). | comrak ships a syntect adapter. Use `ClassedHTMLGenerator` so colours come from our Omarchy-derived CSS. ⛔ `inkjet` (2024-09, stale), `syntastica` (2025-06, low usage), `autumnus` (renamed to lumis). |
| Editor highlighting | ✅ GtkSourceView `.lang` (built in) | gtksourceview 5.22 | system | 🔁 `tree-sitter` 0.27.1 + `tree-sitter-md` 0.5.4 + `tree-sitter-highlight` (GPUI route) | Ship a custom `markdown-frankmd.lang` for wikilinks and `$math$`. |
| Math in preview (WebKit) | ✅ **`katex-rs`** | 0.3.0 (2026-09-05) | 159k downloads, tracks KaTeX 0.18.5, pixel-parity tests | 🔁 KaTeX JS inside WebKit (exactly what FrankMD does today) | Rust-side rendering keeps JS out of the preview and makes output snapshot-testable. Reuse the KaTeX CSS and fonts already vendored in `public/fonts/katex`. ⛔ the `katex` crate 0.4.6 (2023, embeds a QuickJS/duktape JS engine). |
| Math natively (no web) | ✅ **`ratex-svg` / `ratex-render`** (RaTeX) | 0.1.15 (2026-10-16) | 1.6k★, KaTeX-compatible syntax, mhchem | 🔁 `pulldown-latex` 0.8 (→ MathML, needs a MathML renderer). 🔁 `mitex` 0.2.4 + `typst`: heavy. | For the macOS native preview or the GPUI fallback: SVG → `resvg` 0.49 → texture. ⛔ `latex2mathml` (2020), `mathjax_svg` (low usage, JS engine). |
| SVG rasterising | ✅ `resvg` / `usvg` | 0.49.0 (2026-10-24) | 35M downloads, linebender | — | Also for SVG images in the native preview. |

### 2.2 Editor support

| Need | Recommended | Notes |
|---|---|---|
| Text buffer | GtkSourceBuffer (built in) | 🔁 `ropey` 1.6.1 (2025-08) for core-side text operations (diff and replace on big docs). 🔁 `crop` 0.4.3. |
| Vim mode | ✅ **GtkSourceView `VimIMContext`** (built in since 5.4) | Nothing reusable exists for GPUI: Zed's `vim` crate is coupled to Zed's editor, and gpui-kit#2617 is still open. Iced has none either. `modalkit` 0.0.32 is TUI/keybinding-oriented (94★), at best a reference. |
| Regex find/replace | ✅ `regex` 1.13.1 + `fancy-regex` 0.20 (look-around, matching JS behaviour) | Run them over the buffer text so editor search and full-text search use one dialect. |
| Spell check (bonus) | `libspelling` 0.5 (GNOME, GtkSourceView-native) | 🔁 `spellbook` 0.4 (helix, pure Rust). |

### 2.3 Storage, upload, network, AI

| Need | Recommended | Version (release) | Signal | Alternatives | Notes / avoid |
|---|---|---|---|---|---|
| S3 + S3-compatible | ✅ **`aws-sdk-s3`** (+ `aws-config`) | 1.157.0 (2026-10-30) | 104M downloads, official | 🔁 `rust-s3` 0.38.0 (2026-10-04, 11M downloads, far lighter, path-style and MinIO OK). 🔁 `object_store` 0.14.2 (Apache Arrow). 🔁 `opendal` 0.59.5. | The same semantics as the Ruby `aws-sdk-s3`: `endpoint_url` + `force_path_style(true)` for MinIO, plus `put_object` and `list_objects_v2` pagination. The SDK costs compile time and binary size; if that hurts, `rust-s3` is the drop-in. `object_store`/`opendal` abstractions add nothing here. |
| HTTP client | ✅ `reqwest` | 0.13.5 (2026-09-08) | 873M downloads | — | rustls, `json`, `stream`, `gzip`. **Two clients**: a trusted one (AI, S3, Google, YouTube) and an egress-guarded one for user-supplied URLs. |
| SSRF / egress | ✅ own `EgressResolver` implementing `reqwest::dns::Resolve` + `ipnet` 2.12.2 | — | — | 🔁 `hickory-resolver` 0.26.3 as the inner resolver | Filter **every** resolved IP (loopback, private, link-local, CGNAT, TEST-NETs, 240/4, and the IPv6 equivalents, i.e. the `EXTRA_BLOCKED` port of `egress_policy.rb`). Because reqwest connects to exactly the IPs our resolver returns, DNS rebinding is closed by construction. Also: `redirect::Policy::custom` (scheme recheck, hop limit) and **`.no_proxy()`**, because a proxy would bypass the resolver. Unit-test the range table with proptest. |
| LLM chat (grammar fix) | ✅ **`genai`** | 0.7.3 (2026-11-03) | 569k downloads, 931★, very active | 🔁 `rig-core` 0.46.0 (9k★, has image generation, but it is an agent framework with heavy churn). 🔁 `async-openai` 0.43 (OpenAI plus OpenAI-compatible base URLs, has the images API). 🔁 `ollama-rs` 0.3.6. | genai uses native protocols for OpenAI, Anthropic, Gemini, Ollama **and OpenRouter** (`open_router::model`), so one crate covers all 5 providers. Wrap it behind our own `AiProvider` trait because genai is 0.x. ⛔ `async-anthropic` (last release 2025-05), `anthropic-sdk` (2024), `llm` (inactive). |
| AI image generation | ✅ **raw `reqwest`** (~80 LOC) | — | — | 🔁 `async-openai` images API if OpenAI gpt-image gets added. 🔁 `rig-core` image generation. | genai does not do image generation. FrankMD only generates images through **OpenRouter** (`google/gemini-3.1-flash-image-preview`, see `ai_service.rb`): a chat-completions POST with `modalities:["image","text"]` that returns base64 data URLs. Fixture-test it with wiremock. |
| HTML scraping (DuckDuckGo vqd token, Pinterest) | ✅ `scraper` | 0.29.0 (2026-10-21) | 35M downloads | 🔁 `lol_html` (streaming) | Google CSE and the YouTube Data API v3 are plain JSON: `reqwest` + `serde`, no SDK. ⛔ `google-youtube3` (a huge generated crate, overkill). |
| URL handling | `url` 2.5.8 | | | | |

### 2.4 Config, env, YAML, i18n

| Need | Recommended | Notes / avoid |
|---|---|---|
| `.fed` round-trip with comments | ✅ **Hand-written line-based parser** (~150 LOC in `fmd-config`) | The format is `key = value`, unquoted, with `#` comments and **commented-out defaults (`# theme = dark`)** that writes must *uncomment in place*. No crate models that. `rust-ini` 0.21 drops comments. `configparser` 3.3 / `ini` 2.0 drop comments. `ini-roundtrip` 0.2.1 (47k downloads) only preserves lines and has no edit API. `toml_edit` is the wrong format. Model it as `Vec<Line{Blank, Comment, Entry{key, value, commented}}>`, and add proptest round-trip (`parse(render(x)) == x`) plus golden fixtures copied from `~/Projects/FrankMD/.fed` and the test configs. |
| Env vars | ✅ `std::env` + `dotenvy` 0.15.7 | dotenvy's last release was 2023-03, but it is stable, the de facto choice, and the repo is alive. Precedence: `.fed` > env > defaults (as in Ruby `Config`). |
| YAML (Rails locales) | ✅ **`serde-saphyr`** 2.1.0 (2026-11-03, 15.9M downloads, 12M recent, pure Rust) | 🔁 `saphyr` 0.3.3 (2026-10-27, lower-level, keeps order and anchors). 🔁 `yaml-rust2` 0.13. ⛔ `serde_yaml` 0.9.34+deprecated. ⛔ **`serde_yml`**: RUSTSEC-2025-0068, "unsound and unmaintained", repo archived <https://rustsec.org/advisories/RUSTSEC-2025-0068>. ⚠ `serde_norway` (last release 2024-12) and `serde_yaml_ng` (2024-05) are libyaml-port forks with slow releases. Acceptable, but no longer the best choice. |
| i18n | ✅ **Custom `fmd-i18n`** over serde-saphyr | Load `config/locales/*.yml` **unchanged** (embedded with `include_str!` or `rust-embed` 8.13). Flatten `he.home.title` → `home.title`, and use `t!("home.title")`, `t("time.seconds_ago", count=3)`. **Match Rails exactly:** `%{var}` interpolation plus Rails' *default* pluralizer (`count==0` and `zero:` present → zero, `count==1` → one, else other). The repo uses only `one`/`other` (39 interpolations in en.yml, no `two:` in he.yml, and no `rails-i18n` gem), so **do not** use CLDR rules (`icu_plurals` 2.4): they would change behaviour (e.g. ja/ko → always `other`). Fall back to `en`. A test asserts that every locale has the same key set as `en`. 🔁 `rust-i18n` 4.2.4 (longbridge, YAML, `%{}`), but it is compile-time and global, and it has no Rails plural semantics. ⛔ Fluent and gettext would require converting the YAML. |
| XDG / app dirs | ✅ `etcetera` 0.11.0 (`choose_base_strategy` → XDG; use XDG on macOS too, so `~/.config/frankmd` stays where the current app keeps it) | 🔁 `directories` 6.0, 🔁 `dirs` 7.0. ⛔ `directories-next` (2020). |
| Secrets (optional) | `keyring` 4.2 (Secret Service / Keychain) | Only if we stop storing API keys in plain `.fed`. |

### 2.5 Files, search, diff, watching

| Need | Recommended | Version | Alternatives / avoid |
|---|---|---|---|
| Fuzzy file finder | ✅ **`nucleo-matcher`** | 0.3.1 (2024-02; stable, powers helix) | 🔁 `frizbee` 0.13 (2026-08, SIMD, typo-tolerant, active). ⛔ `fuzzy-matcher` 0.3.7 (2020, unmaintained). Combine the score with a recency boost from the recent-files list. |
| Full-text regex search | ✅ `grep-searcher` 0.1.17 + `grep-regex` 0.1.14 + `ignore` 0.4.34 (ripgrep's libraries) | | `globset` 0.4.21 for exclude patterns. ⛔ hand-rolled `walkdir` + `regex` loops. |
| Tree walking | `ignore` (parallel, respects `.gitignore`) / `walkdir` 2.5 | | |
| Diff (grammar check, backup recovery) | ✅ **`similar`** 3.2.1 (2026-10-24, 240M downloads) | | Line diff, inline word/char changes (`TextDiff::iter_inline_changes`), Myers/Patience. 🔁 `imara-diff` 0.2 (faster histogram diff, lower-level, used by gitoxide) only if diffing huge files. |
| File watching | ✅ `notify` 8.2.1 + `notify-debouncer-full` 0.7.0 | | Notes dir: recursive, debounced 200 ms, ignoring our own writes through a write-token set. Omarchy theme: watch the **parent dir** of the `current/theme` symlink non-recursively, since symlink swaps don't fire on the target. 🔁 `gio::FileMonitor` in the GTK layer, but keep notify so the core stays GUI-free. |
| Trash / safe delete | `trash` 5.2.9 | | Optional: "delete note" goes to the XDG trash. |
| Open in browser | `open` 5.4.5 | | Or `gtk::UriLauncher` (portal). |
| MIME / sniffing | `infer` 0.22 (magic bytes), `mime_guess` 2.0.5 | | Validate uploaded and pasted images. |

### 2.6 Media, desktop integration

| Need | Recommended | Notes / avoid |
|---|---|---|
| Image decode / thumbnails | ✅ `image` 0.25.10 + `fast_image_resize` 6.1 + `imagesize` 0.15 (header-only dimensions) | The GTK side displays `gdk::Texture::from_bytes`; the core does resize and JPEG/WebP re-encode before upload. |
| Emoji data | ✅ `emojis` 0.9.1 (2026-10-17, gemoji shortcodes, groups, search) | Emoticons (kaomoji) are a static list ported from `app/javascript/lib/emoji_data.js`. GTK's built-in `GtkEmojiChooser` is an option but not themable enough. ⛔ `emoji` 0.2.1 (2021). |
| Clipboard incl. images | ✅ `gdk::Clipboard` (GTK route: `read_texture_future`, `set_texture`) | 🔁 `arboard` 3.7.0 with feature `wayland-data-control` (for GPUI or non-GTK code). |
| Drag & drop from desktop | ✅ `gtk::DropTarget` (`gdk::FileList`, `gdk::Texture`) + `DragSource` for the tree | — |
| Native dialogs | ✅ `gtk::FileDialog` (portal-backed) | 🔁 `rfd` 0.17.2 / `ashpd` 0.13.13 (direct portal) for non-GTK. |
| Notifications | ✅ `gio::Notification` via GtkApplication | 🔁 `notify-rust` 4.18.2 (fdo, works with mako). |
| Single instance + `fed [path]` | ✅ **GtkApplication** (D-Bus uniqueness, `HANDLES_COMMAND_LINE`, forwards argv to the primary instance) | 🔁 `interprocess` 2.4.7 local socket (GPUI route). ⛔ `single-instance` 0.3.3 (2021, stale). |
| CLI parsing | ✅ `clap` 4.6.7 (derive) | Parse argv with clap *inside* the `command-line` handler so the remote instance sees the same semantics. |
| Logging | ✅ `tracing` 0.1.44 + `tracing-subscriber` 0.3.23 (env-filter) + `tracing-appender` 0.2.5 (rolling file in `$XDG_STATE_HOME/frankmd/`) | Forward GLib logs into tracing (`glib::log_set_writer_func`). Optional `tracing-journald` 0.3.2. |
| Slug / transliteration | ✅ **Hand-port `HugoService.slugify`** (downcase → `ACCENT_MAP` lookup → `[^a-z0-9]+`→`-` → trim → collapse) | Ruby uses a **custom `ACCENT_MAP`** (`app/services/hugo_service.rb`), so the table must be copied verbatim and golden-tested against Ruby outputs. `deunicode` 1.6.2 / `any_ascii` 0.3.3 give *different* results and are acceptable only as an explicit opt-in fallback for chars outside the map. ⛔ `slug` 0.1.6, `slugify` (2017), `unidecode` (2016). |
| Date/time | ✅ `jiff` 0.2.39 (2026-11-02, tz-aware, strftime, BurntSushi) | 🔁 `chrono` 0.4.45 if a dependency forces it. Implement "x minutes ago" ourselves on top of i18n keys. |
| Unicode helpers | `unicode-segmentation`, `unicode-normalization` 0.1.26 | NFC before slugging and searching. |

### 2.7 Async runtime: how it meshes with the GUI

- **tokio** 1.54.1 (2026-10-21) is required by reqwest, aws-sdk-s3, genai and wiremock.
- **GTK route:** Relm4 0.11 already depends on tokio (`^1.51`) and runs `oneshot_command`/`spawn_command` futures on a tokio runtime, delivering results back as messages on the GLib main loop. For raw gtk4-rs code, use a `static RUNTIME: LazyLock<tokio::runtime::Runtime>` and bridge with `async-channel` 2.5 into `glib::spawn_future_local`. This is the pattern from the gtk-rs book.
- **GPUI route:** GPUI has its own executor; spawn reqwest work on a background tokio runtime and `.await` the results through channels (this is how Zed does it).
- Rule: **core crates are runtime-agnostic** where possible (sync APIs for fs, search and diff), and network crates expose `async fn` on tokio.

### 2.8 Testing

| Need | Recommended | Notes |
|---|---|---|
| Snapshot tests (HTML render, AST, diff output, config writes) | ✅ `insta` 1.49 (`yaml`, `json`, `redactions`, `glob`) | Port FrankMD's fixtures as insta inputs. Accept the reviewed snapshots as the spec. |
| Golden JSON fixtures | `serde_json` + `insta::assert_json_snapshot!` | e.g. tree listings, search results, AI request bodies. |
| Temp dirs / fs assertions | ✅ `tempfile` 3.27 + `assert_fs` 1.1.4 | Notes-dir integration tests. |
| HTTP mocking | ✅ `wiremock` 0.6.5 (tokio, async, request matchers) | 🔁 `httpmock` 0.9.0 (2026-10-20, more active). 🔁 `mockito` 1.7.2. Use wiremock for AI, S3 (point `endpoint_url` at the mock), DDG, YouTube and Google CSE. |
| Property tests | ✅ `proptest` 1.12 | `.fed` round-trip, slug idempotence, egress IP classifier, i18n interpolation. |
| Runner | `cargo-nextest` 0.9.151 | |
| UI tests | GTK: unit-test Relm4 `update()` plus headless-Wayland smoke tests and AT-SPI. GPUI fallback: `gpui::TestAppContext` + gpui-kit headless UI tests (a real advantage of that stack). | |

---

## 3. Proposed Cargo workspace

```
FrankMDFast/
├── Cargo.toml                 # [workspace], shared [workspace.dependencies], lints, profile.release (lto="thin", codegen-units=1)
├── crates/
│   ├── fmd-core/              # Domain only. No GUI, no network, no tokio.
│   │                          #   note paths & PathSafety, tree model, rename/move, Hugo post generator,
│   │                          #   slugify (ported ACCENT_MAP), wikilinks/backlinks index, recent files,
│   │                          #   content-loss guard (20%/50 chars), backups & recovery, emoji/emoticon data
│   ├── fmd-config/            # .fed line-based round-trip parser/writer, env precedence, typed Settings, XDG paths
│   ├── fmd-i18n/              # Rails YAML loader (serde-saphyr), flatten, %{var}, Rails default pluralizer, locale list + RTL flag
│   ├── fmd-markdown/          # comrak pipeline: GFM, footnotes, wikilinks, YouTube transform, katex-rs math,
│   │                          #   syntect classed highlighting, ammonia sanitising, sourcepos map for sync scroll
│   ├── fmd-search/            # nucleo fuzzy finder + ripgrep-libs full-text search + similar diffs
│   ├── fmd-net/               # reqwest clients (trusted + egress-guarded), EgressResolver, rate limits
│   ├── fmd-media/             # image decode/resize/thumbnails, S3 upload & library listing (aws-sdk-s3),
│   │                          #   local image library, web image search (DDG/Google CSE/Pinterest), YouTube API
│   ├── fmd-ai/                # AiProvider trait; genai chat (5 providers); OpenRouter image gen; prompts
│   ├── fmd-theme/             # Omarchy theme reader + watcher → Theme tokens → (GTK CSS, GtkSourceView scheme XML,
│   │                          #   preview CSS) generators; 18 built-in themes ported
│   ├── fmd-watch/             # notify + debouncer, self-write suppression, theme-symlink watcher
│   └── fmd-app/               # GTK4 + Relm4 + sourceview5 + webkit6. The only crate that links GTK.
│       ├── src/main.rs        #   GtkApplication, clap, tracing init, tokio runtime
│       ├── src/editor/        #   EditorView (GtkSourceView wrapper: vim, typewriter, find/replace, IME)
│       ├── src/preview/       #   PreviewBackend trait; webkit impl (Linux); native impl (macOS, feature-gated)
│       └── src/ui/            #   sidebar tree, finder, search panel, dialogs (diff, AI, images, emoji, table editor)
├── tests/fixtures/            # notes trees, .fed files, locale YAML copies, HTTP cassettes, golden HTML
└── docs/
```

Dependency direction: `fmd-app → {all}`. `fmd-media, fmd-ai → fmd-net → fmd-config`. `fmd-markdown, fmd-search → fmd-core`. **No crate other than `fmd-app` may depend on gtk/glib**, which CI enforces with `cargo tree -i gtk4` per crate.

### `[workspace.dependencies]` (initial pins)

```toml
# GUI (fmd-app only)
gtk4        = { version = "0.11.5", features = ["v4_18"] }
sourceview5 = { version = "0.11.2", features = ["v5_4"] }   # VimIMContext
webkit6     = "0.6.1"
relm4       = "0.11.0"
glib        = "0.22"
gio         = "0.22"

# Markdown / render
comrak      = { version = "0.59", features = ["syntect"] }
syntect     = { version = "5.3", default-features = false, features = ["default-fancy", "html"] }
two-face    = "0.5"
katex-rs    = "0.3"
ammonia     = "4.2"
ratex-svg   = { version = "0.1.15", optional = true }   # native preview / macOS
resvg       = { version = "0.49", optional = true }

# Core utils
regex = "1.13"; fancy-regex = "0.20"
nucleo-matcher = "0.3"
grep-searcher = "0.1.17"; grep-regex = "0.1.14"; ignore = "0.4.34"; globset = "0.4"
similar = { version = "3.2", features = ["inline", "unicode"] }
notify = "8.2"; notify-debouncer-full = "0.7"
etcetera = "0.11"
jiff = "0.2"
emojis = "0.9"
unicode-normalization = "0.1"
serde = { version = "1", features = ["derive"] }; serde_json = "1"
serde-saphyr = "2.1"
dotenvy = "0.15.7"
thiserror = "2"; anyhow = "1"

# Network / cloud / AI
tokio   = { version = "1.54", features = ["rt-multi-thread", "macros", "fs", "sync", "time"] }
reqwest = { version = "0.13", default-features = false, features = ["rustls", "json", "stream", "gzip"] }
ipnet   = "2.12"
scraper = "0.29"
url     = "2.5"
aws-config = "1"; aws-sdk-s3 = "1.157"
genai   = "0.7"

# Media
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp", "gif"] }
fast_image_resize = "6.1"; imagesize = "0.15"; infer = "0.22"; mime_guess = "2"

# App shell
clap = { version = "4.6", features = ["derive"] }
tracing = "0.1"; tracing-subscriber = { version = "0.3", features = ["env-filter"] }; tracing-appender = "0.2"
async-channel = "2.5"
open = "5"; trash = "5"

# Dev
insta = { version = "1.49", features = ["yaml", "json", "redactions", "glob"] }
tempfile = "3.27"; assert_fs = "1.1"
wiremock = "0.6.5"
proptest = "1.12"
```

---

## 4. Sources

- Live crates.io API: `https://crates.io/api/v1/crates/<name>` (all versions, dates and download counts above).
- gpui-kit: <https://github.com/longbridge/gpui-kit>. RTL answer <https://github.com/longbridge/gpui-kit/issues/3679>. Large file <https://github.com/longbridge/gpui-kit/issues/2566>. Editor API for vim <https://github.com/longbridge/gpui-kit/issues/2617>. webview README (`crates/webview`). GPUI CE <https://gpui-ce.github.io>.
- Zed RTL tracker: <https://github.com/zed-industries/zed/issues/31102>. fcitx5 fixes: zed#35537, #31337, #36831.
- iced changelog: <https://github.com/iced-rs/iced/blob/master/CHANGELOG.md>. RTL <https://github.com/iced-rs/iced/issues/250>. a11y <https://github.com/iced-rs/iced/issues/552>.
- GtkSourceView VimIMContext: <https://gnome.pages.gitlab.gnome.org/gtksourceview/gtksourceview5/class.VimIMContext.html>. Rust docs <https://world.pages.gitlab.gnome.org/Rust/sourceview5-rs/git/docs/sourceview5/struct.VimIMContext.html>.
- Tauri Linux graphics issues: <https://v2.tauri.app/develop/debug/linux-graphics/>. CEF discussion <https://github.com/tauri-apps/tauri/discussions/8524>.
- RaTeX: <https://github.com/erweixin/RaTeX>. katex-rs: <https://github.com/katex-rs/katex-rs>.
- RUSTSEC-2025-0068 (serde_yml): <https://rustsec.org/advisories/RUSTSEC-2025-0068>.
- genai: <https://github.com/jeremychone/rust-genai>. rig: <https://github.com/0xPlaygrounds/rig>.
- arborium: <https://github.com/bearcove/arborium>. lumis: <https://github.com/leandrocp/lumis>.
- NSH: <https://nsh.dev>.
