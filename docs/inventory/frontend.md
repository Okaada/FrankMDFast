# FrankMD frontend / UX inventory (for the FrankMDFast Rust rewrite)

Source: `~/Projects/FrankMD` (Rails 8 + Stimulus + importmap + CodeMirror 6), read-only survey on 2026-11-04. This document describes **what the UI does** in enough detail to reimplement it without reading the JS. Backend models, services and config schema are in the sibling backend inventory. This document only names the endpoints the UI calls.

Citation conventions: `file:line`, relative to the FrankMD repo root. Some sections use short prefixes: `AC` = `app/javascript/controllers/app_controller.js`, `AS` = `autosave_controller.js`, `RD` = `recovery_diff_controller.js`, `JS/` = `app/javascript/`. Statements marked "inferred" or "not verified in a browser" come from reading code only.

## Contents
1. Layout & workspace chrome (regions, toggles, persistence, toasts)
2–3. Keyboard shortcuts (complete table) & Editor (CodeMirror extensions, vim, typewriter, fonts, indentation, surround, slash commands, wikilink autocomplete, find/replace, stats, highlighting)
4. Data safety (autosave timings, content-loss guard, localStorage drafts, recovery diff, offline detection, undo-created-note)
5. Preview (marked + extensions, KaTeX, sanitization, code highlighting, task toggles, scroll sync, zoom, image viewer, links)
6–7. Explorer tree, context menus, DnD, creation destination, file finder ranking, content search, Home/recent files
8a/11. Media: image picker (7 tabs), video dialog, Media Library, storage provider, paste & drop
8b. Other dialogs: table editor, code block, emoji/emoticon/icon picker, links, text-format menu, AI grammar, log viewer, help, jump-to-line, new note/folder/Hugo, rename, delete, undo-created-note
9–10. Themes (all 18 palettes, full `--theme-*` tables), Omarchy mode, custom theme editor, Settings dialog (every setting), client i18n & RTL
12. Notifications: see §1.7 plus the per-section "notifications"/"error" notes (§4 save status, §6 F.18, §8 appAlert/appConfirm D2.0.1)
13. Known quirks / doc-vs-code discrepancies
14. Feature checklist (FE-001…)

## 0. Client dependencies (`config/importmap.rb`, vendored in `vendor/javascript/`; `package.json` is only for vitest)

| Package (version) | Role in the UI | Rust-side equivalent to decide |
|---|---|---|
| @hotwired/turbo-rails, @hotwired/stimulus, stimulus-loading | Page/stream updates (tree re-renders arrive as turbo-streams), controller framework | n/a (native UI) |
| marked 15.0.6 | Markdown → HTML for the preview (`gfm: true, breaks: true` + custom extensions) | pulldown-cmark / comrak with matching extensions |
| katex 0.19.0 (+ CSS & fonts in `public/fonts/katex`) | `$…$` / `$$…$$` math in the preview | KaTeX-compatible renderer |
| dompurify 3.4.17 | Sanitizes preview HTML (allows the video/iframe embeds the dialogs insert) | ammonia or equivalent |
| @codemirror/view 6.39.11, state 6.5.4, language 6.12.1, commands 6.10.1, search 6.6.0, autocomplete 6.20.0 | Editor core, keymaps, search panel, completion (slash commands, wikilinks) | editor widget |
| @codemirror/lang-markdown 6.5.0, @lezer/markdown 1.6.3 | Markdown parsing/highlighting in the editor; list continuation on Enter | |
| @lezer/highlight 1.2.5, @lezer/common 1.5.3, @lezer/lr 1.4.11 | Highlight tags, parser runtime | |
| @codemirror/lang-javascript 6.2.4 / lang-html 6.4.11 / lang-css 6.3.1 + @lezer/javascript, html, css | Highlighting of fenced code **in the preview only** | tree-sitter / syntect |
| @lezer/cpp, go, java, python, rust, yaml, php (lazy) | Lazy-loaded preview code grammars | |
| @codemirror/legacy-modes clike, ruby, shell, sql, toml, dockerfile (6.5.5) | Lazy stream-mode grammars for preview code | |
| local `lib/terraform_hcl_stream_parser.js` | Hand-written HCL/Terraform stream highlighter for preview code | |
| @replit/codemirror-vim 6.3.0 | Optional Vim mode | |
| @marijn/find-cluster-break, crelt, style-mod, w3c-keyname | CodeMirror internals | |
| Not used: no mermaid, no highlight.js/Prism, no diff library (own LCS in `lib/diff_utils.js`), no emoji library (own `lib/emoji_data.js` + `public/emoji-data/*.json` for picker search keywords) | | |

Static assets: `public/fonts/*.woff2` (Cascadia Code, DejaVu Mono, Fira Code 400/500, Hack, Inter 400/500/600 (UI font), JetBrains Mono 400/500, Roboto Mono 400/500, Source Code Pro 400/500, Ubuntu Mono), declared in `app/assets/stylesheets/fonts.css`. Icons: `public/icon.svg` (frog logo), `icon.png`, `icon-192.png`, `icon-512.png`, `apple-touch-icon.png`, `favicon-16x16/32x32.png`. `public/emoji-data/{en,es,ja,ko,pt}.json` hold the localized emoji names/keywords used by picker search. `public/splash.html` and `app/views/pwa/*` are not routed (see §4 B.12).

## 1. Layout & workspace chrome

Source: `app/views/notes/index.html.erb`, `_sidebar`, `_header`, `_workspace_toolbar`, `_workspace_action`, `_editor_panel`, `_preview_panel`, `_home`, `app/helpers/workspace_helper.rb`, `app/assets/tailwind/components/editor.css`, `app/javascript/controllers/app_controller.js` (hub, 3562 lines). Reference screenshots: `docs/proposals/minimal-workspace/desktop-after.png` (current layout), `toolbar-actions.png` (overflow menu), `desktop-before.png` (old layout — NOT current).

### 1.1 Boot / splash
- Inline splash `#app-loading` (fixed, z 9999, `linear-gradient(135deg,#2D1B4E 0%,#FF6B9D 100%)`, 150px `/icon.svg` + 36px spinner) is shown before CSS/JS; removed by `_removeSplashScreen()` with 0.2 s opacity fade (`app_controller.js:309-317`). Initial load completes when CodeMirror connects or after a 50 ms fallback timeout (`app_controller.js:240`).
- Server injects the initial note (`data-app-initial-note-value` JSON with `exists`, `content`) so the editor is created with content already in place (`_preloadInitialContent`, `:288`).
- URL routing: `/notes/<encoded path>.md` opens a note; `?file=` query also accepted (`getFilePathFromUrl`, `:432-444`). Opening a file `pushState({file})`; Home → `/`. `popstate` reloads the file or shows Home (`:584-596`). A "navigation generation" counter discards stale async loads (`beginNavigation`).
- Global `fetch` wrapper: any 401 response → `window.location.assign("/login")` once (`:373-382`). Login page (`sessions/new.html.erb`): "FrankMD access", single "Access token" password field, "Sign in" (only when server has an access token configured; backend agent covers auth).

### 1.2 Regions (top to bottom / left to right)
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ [amber banner: connection lost  (Retry)]            (hidden unless offline)  │
│ [amber banner: content-loss warning (Undo)(Save anyway)] (hidden normally)   │
├──────────────┬───────────────────────────────────────────────────────────────┤
│ SIDEBAR 16rem│ HEADER (min-h 3rem): [doc title / inline rename][save status]  │
│ header:      │      ... [zoom − 100% +] [pinned actions][⋯ overflow menu]     │
│ [⇤][+note]   ├───────────────────────────────────┬───────────────────────────┤
│ [+folder][🔍]│ EDITOR PANEL (flex-1)              │ PREVIEW PANEL (40% width, │
│ [content🔍]  │  - Home screen when no file        │  border-left, hidden by   │
│ tree         │  - CodeMirror when file open       │  default)                 │
│ ...          │  - table hint (bottom-right)       │                           │
│ [stats panel]│  - vim mode badge (bottom-right)   │                           │
│ footer:      │ (Library panel / replaces editor+preview when open)            │
│ 🐸FrankMD [⌂][📚][⚙]                                                         │
└──────────────┴───────────────────────────────────────────────────────────────┘
```
- **Root**: `h-screen flex flex-col`, bg `--theme-bg-primary`, text `--theme-text-primary`.
- **Warning banners** (`shared/_warning_banner.html.erb`): amber (`bg-amber-500`/dark `amber-600`), white text, triangle-warning icon (pulsing for the connection banner), centered message + small translucent buttons. Two instances: connection lost (`connection.offline_message`, button `connection.retry`), content-loss (`status.content_loss_warning`, buttons `status.undo`, `status.save_anyway`). See §4.
- **Sidebar** (`aside#workspace-sidebar`, width 16rem, bg `--theme-bg-secondary`, inline-end border = `color-mix(--theme-border 50%, transparent)`):
  - Header (min-h 3rem): close-explorer button (panel-left-close icon), then Explorer actions: New note, New folder, Find file (Ctrl+P), Search in files (Ctrl+Shift+F) (`_explorer_actions.html.erb`).
  - Explorer tree (`#file-tree-content`, scrollable, p-2) — §6.
  - Stats panel (optional, hidden unless setting "show stats" on AND a file is open; `showStatsPanel` `app_controller.js:3515`): 2-col grid — Words, Characters, Size (bytes, e.g. "0 B"), Read time ("N min"), Line "cur/total". See §3 stats.
  - Footer: brand (icon.svg 20px + "FrankMD", semibold), navigation: Home (house icon → `showHome`), Library toggle (`aria-pressed`), Settings (gear → opens Settings modal).
- **Sidebar hidden**: `.is-hidden {display:none}` + `inert`. A floating "open explorer" button (panel-left-open icon) appears absolutely at top-left of the main area (`.workspace-sidebar-open`, inset .5rem, z 45) and the header gets 3rem start padding.
- **Header** (`_header.html.erb`, hidden when Home is showing): document title button (basename without `.md`, truncates; click = inline rename; disabled for `.fed` config file and when no file), hidden auto-sizing `<input>` for rename (Enter/blur = submit, Escape = cancel; inline error `<p role=alert>` below), save-status text (`text-xs`, faint), then toolbar (§1.3).
- **Editor panel** (`<main>`, flex-1): contains Home screen placeholder (when no file) or the CodeMirror container; hidden textarea mirror; vim mode badge (absolute bottom-1 right-2, 11px, bg tertiary); "cursor in table" hint (`editor.cursor_in_table`, absolute bottom 1.5rem inline-end .75rem, 0.75rem text, shown when cursor is inside a markdown table, debounced check).
- **Preview panel**: `flex-none w-[40%] border-l`, hidden by default; content div is `prose prose-sm`, p-6, scroll container. In single view mode it is `flex:1; width:100%; border-left:none` (`editor.css:120-128`). §5.
- **Library panel**: replaces editor+preview in the main area when opened (§8 media library). Remembers whether preview was visible and restores it on close (split mode only) (`showLibraryWorkspace`/`showEditorWorkspace`, `app_controller.js:1936-2028`). Escape closes the Library (unless a dialog/preview overlay is open, which closes first).
- **Settings**: modal dialog (not a pane) — §8/§9 (`openSettings` → settings controller `openDialog`).
- **No resizable splitters**: sidebar is fixed 16rem; preview fixed 40%. There is no drag-to-resize anywhere. Editor text column width is set in `ch` (see §1.5).

### 1.3 Header toolbar & overflow menu
Actions defined in `app/helpers/workspace_helper.rb:4-20` (id, handler, i18n label, scope, pinned-by-default):

| id | handler | label key | scope | default pinned |
|---|---|---|---|---|
| image | openImagePicker | editor.toolbar.image_title ("Insert Image") | editing | yes |
| find | openFindReplace | editor.toolbar.find_title ("Find and Replace") | editing | yes |
| preview | togglePreview | header.toggle_preview ("Toggle Preview") | preview | yes |
| video | openVideoDialog | editor.toolbar.video_title | editing | no |
| code | openCodeEditor | editor.toolbar.code_title ("Insert/Edit Code Block") | editing | no |
| table | openTableEditor | editor.toolbar.table_title ("Insert/Edit Table") | editing | no |
| emoji | openEmojiPicker | editor.toolbar.emoji_title | editing | no |
| ai | openAiDialog | editor.toolbar.ai_title ("AI Grammar Check") | editing | no |
| copy | copyCurrentContent | header.copy_content ("Copy content") | document | no |
| help | openHelp | editor.toolbar.help_title ("Markdown Help") | global | no |

- Labels strip a trailing " (Ctrl+…)" / " (F1)" from the tooltip (`workspace_helper.rb:18`); tooltip keeps it.
- Toolbar layout: [preview zoom controls (−, "100%", +) — visible only when preview visible] [pinned editing actions] [pinned other actions] [⋯ "More actions" toggle].
- Overflow menu (`#workspace-menu`, role=menu, width `min(17rem, 100vw-1rem)`): every action as a row "icon + label" plus a pin toggle (`role=menuitemcheckbox`, rotated pushpin icon). Unpinned pins are invisible until row hover/focus (always visible on coarse pointers). Pin state persisted in **sessionStorage `frankmd.workspacePins`** (JSON array of ids; per-tab, falls back to defaults) (`app_controller.js:1817-1843`).
- Menu keyboard: ArrowDown/Up (wrap), Home/End, ArrowRight/Left moves between action and its pin (mirrored in RTL), Tab closes, Escape closes and refocuses toggle; ArrowDown/Up on the closed toggle opens it (Up/End focuses last). Outside click closes. Menu is clamped to viewport (`menu_bounds.js clampMenuToViewport`).
- Visibility rules (`updateWorkspaceChrome`, `:1772-1814`): editing actions shown when a markdown file is open and the editor pane visible (or single-mode preview-only — then shown but **disabled**); document actions (copy) only when a file is open; preview toggle disabled for non-markdown files and while Library is open; header hidden entirely on Home.
- Copy content: copies whole editor text to clipboard → toast `status.copied_to_clipboard` / error `errors.failed_to_copy_content`.

### 1.4 View modes, panes & toggles
- **View mode** setting (`split` default | `single`), persisted to `.fed` as `view_mode` (settings controller). Body class `single-view-mode`.
  - Split: editor + optional preview side by side; preview toggle shows/hides the 40% preview.
  - Single: one pane at a time; preview toggle swaps editor ↔ full-width preview (editor is hidden, not destroyed, so undo/autosave survive) (`switchSinglePane`, `:1262-1284`).
- Preview toggle only for markdown files; else temporary message "Preview is only available for markdown files" (hard-coded English, `:1247`).
- **Sidebar toggle** (Ctrl+E, header buttons): NOT persisted (config schema has `sidebar_visible` but the JS never reads it — sidebar always starts visible on desktop, hidden on mobile). Opening a file via finder/search forces sidebar visible (`openFileAndRevealInTree`, `:2254`).
- **Mobile** (`window.innerWidth < 768`): sidebar becomes an overlay (`.is-overlay`, width `min(19rem,100vw)`, z 30) with backdrop (25% text color); main area `inert` while open; focus trap with Tab cycling; closes on Escape, backdrop click, Home, or after opening a note. Header wraps to 5rem tall. Recomputed on `resize`.
- **Typewriter mode** toggles body class `typewriter-mode`; in split mode it force-shows the preview so it can follow the cursor (`applyTypewriterMode`, `:2166-2181`). Persisted as `typewriter_mode` in `.fed`. (Details §3.)
- **Editor width** (text column, in `ch`): default 72, min 40, max 200, step 8 (`app_controller.js:1472-1510`), Ctrl+Shift+= / Ctrl+Shift+-; shows temporary message "Editor width: Nch" / "Maximum width (200ch)" / "Minimum width (40ch)" (hard-coded English); persisted `editor_width` in `.fed`. CSS var `--editor-width`. The text column is centered in the editor pane; preview in typewriter mode uses `max-width: calc(var(--editor-width,72ch) + 3rem)`.
- **Persistence summary of layout state**: `.fed` (server config, via settings save endpoint): `view_mode`, `typewriter_mode`, `editor_width`, `preview_zoom`, `editor_font`, `editor_font_size`, `line_numbers`, theme, locale, show-stats, vim_mode (see §9 for the exact list). sessionStorage: `frankmd.workspacePins`. Explorer expansion state: see §6. Preview visibility itself: not persisted (starts hidden; Home→note restores the preview state that was visible before going Home).

### 1.5 Home screen (no file open)
`_home.html.erb` + `home_controller.js` (details §7/§6 agent section): logo + "FrankMD" h1 (receives focus on showHome), description, two buttons: "Find file" (opens Ctrl+P finder) and "New note"; recent files list (5 shown, "Show more" up to 10, "Show less"), relative opening times, "Clear" with confirm (`home.clear_confirmation`); refreshes on window focus. Stored in `.fed` (shared across browsers). Header is hidden while Home is visible; preview hidden.

### 1.6 File-not-found state
`showFileNotFoundMessage` (`:606-637`): header path shows the path in red + "(message)" in muted; editor disabled; after **5000 ms** returns to Home placeholder.

### 1.7 Toasts / transient messages (global)
- `showTemporaryMessage(msg, duration=2000)` (`app_controller.js:2200`): single div, fixed bottom-4 centered, bg-secondary, border, rounded, shadow, text-sm, z 50; replaces any existing one. Used for: editor-width, preview/typewriter-only-markdown, draft storage errors (5000 ms), delete/create/load failures (5000 ms).
- `pathDisplay.showToast(msg, 2000)` (`path_display_controller.js:106`): `#app-toast.app-toast`, `role=status aria-live=polite`, slide/fade-in class `app-toast--visible`, hides with `app-toast--hiding` (translateY .5rem). Used for "Copied to clipboard".
- Save status text in header (`status.*` labels) — §4.
- `appAlert` / `appConfirm` (`lib/app_prompt.js`) — themed modal replacements for alert/confirm (§8). One remaining native `window.confirm` for "undo created note" fallback (`app_controller.js:2653`).
- Error states: inline errors (`lib/inline_messages.js showInlineError/clearInlineError`) under inputs in dialogs.

### 1.8 Things that exist server-side but have no UI
- `GET notes/*path/backlinks` (`notes_controller.rb:156`) returns backlinks JSON, but **no JavaScript calls it** (grep shows no client usage) — README claims "each note lists which other notes link back to it". Decide in the rewrite whether to add a backlinks panel.
- `sidebar_visible` config key (not read by the client).

---

## 2–3. Keyboard shortcuts & Editor  
<sub>(sub-section labels below keep the source numbering, e.g. A.x)</sub>

Paths are relative to `~/Projects/FrankMD/`. `JS/` = `app/javascript/`. "Mod" = Ctrl on Windows/Linux, Cmd on macOS.

---

### A.1 Keyboard shortcut system (how matching works)

- Global shortcuts are declared in `JS/lib/keyboard_shortcuts.js:11-34` (`DEFAULT_SHORTCUTS`) and merged with `this.userShortcuts` (always undefined today, so no user customization exists yet; `mergeShortcuts` allows a `null` override to disable a binding, `keyboard_shortcuts.js:176-190`).
- One `keydown` listener on `document`, **bubble phase**, installed in `app_controller.js:3159-3214` (`setupKeyboardShortcuts`).
- Matching rules (`matchShortcut`, `keyboard_shortcuts.js:45-65`):
  - `event.key` must equal `shortcut.key` **exactly, case-sensitive** (that is why shifted shortcuts use `"V"`, `"F"`, `"E"`, `"O"`, `"+"`, `"_"`).
  - `ctrl: true` is satisfied by **Ctrl OR Meta**. If `ctrl` is not set, neither Ctrl nor Meta may be held.
  - Shift and Alt must match exactly (no extra modifiers allowed).
  - On a match, `preventDefault()` is called unless the shortcut has `preventDefault: false` (only `closeDialogs`/Escape).
  - The listener does **not** check `event.defaultPrevented`, so a key that CodeMirror already handled still fires the global action (see A.3 "overlaps").
- When the Settings dialog is open (`settings_controller.isDialogOpen`), **every** global shortcut is ignored (`app_controller.js:3206, 3217`).
- Display formatting (`formatShortcut`, `keyboard_shortcuts.js:73-118`): Win/Linux `Ctrl+Shift+V` (joined by `+`); macOS `⌘⇧V` (glyphs ⌘ ⇧ ⌥, no separator). Single letters are uppercased. Function keys are shown as-is.
- macOS detection (`isMacOS`, `keyboard_shortcuts.js:124-131`): `navigator.userAgentData.platform === "macOS"`, else `/Mac|iPod|iPhone|iPad/` on userAgent.

### A.2 Global (app-wide) shortcuts

| Action id | Key (Win/Linux; Cmd on mac) | `event.key` | What it does | Guards / notes |
|---|---|---|---|---|
| `newNote` | Ctrl+N | `n` | `file-operations#newNote()` (opens New Note dialog) | |
| `save` | Ctrl+S | `s` | `autosave#saveNow()` (immediate save) | |
| `togglePreview` | Ctrl+Shift+V | `V` | Toggle preview pane. In single-view mode swaps editor ⇄ full-width preview (`app_controller.js:1242-1284`) | Markdown files only, otherwise toast "Preview is only available for markdown files"; no-op while Library is visible. **Note:** tooltip string `header.toggle_preview` says "(Ctrl+Shift+P)" — wrong; real binding is Ctrl+Shift+V |
| `findInFile` | Ctrl+F | `f` | Open Find & Replace dialog on **Find** tab | pre-fills current selection |
| `findReplace` | Ctrl+H | `h` | Open Find & Replace dialog on **Replace** tab | |
| `jumpToLine` | Ctrl+G | `g` | Open Jump-to-Line dialog | |
| `lineNumbers` | Ctrl+L | `l` | Cycle line numbers OFF → ABSOLUTE → RELATIVE → OFF; persists `editor_line_numbers` | |
| `contentSearch` | Ctrl+Shift+F | `F` | Open Search-in-files dialog | |
| `fileFinder` | Ctrl+P | `p` | Open file finder (files sorted newest mtime first) | |
| `toggleSidebar` | Ctrl+E | `e` | Toggle Explorer sidebar | |
| `typewriterMode` | Ctrl+\ | `\` | Toggle typewriter mode | Markdown only; else toast "Typewriter mode is only available for markdown files" (2000 ms) |
| `toggleTask` | Ctrl+Enter | `Enter` | Toggle/add task on current line(s) (A.13) | Markdown only |
| `toggleScrollSync` | Ctrl+Shift+\ | `\` with shift | Toggle editor⇄preview scroll sync; persists `scroll_sync` | `event.key` for Shift+\ on US layout is `|`, so this binding as written (`key: "\\", shift: true`) likely only matches on layouts where Shift+\ still reports `\` — **unsure if it works on US keyboards** |
| `textFormat` | Ctrl+M | `m` | Open Text Format menu at selection (A.11) | Markdown only; needs non-blank selection |
| `emojiPicker` | Ctrl+Shift+E | `E` | Open emoji picker | Markdown only |
| `increaseWidth` | Ctrl+Shift++ | `+` | Editor width +8ch (max 200) | Toast "Editor width: Nch" / "Maximum width (200ch)" 2000 ms |
| `decreaseWidth` | Ctrl+Shift+- | `_` | Editor width −8ch (min 40) | Toast "Editor width: Nch" / "Minimum width (40ch)" |
| `logViewer` | Ctrl+Shift+O | `O` | Open log viewer | |
| `help` | F1 | `F1` | Open Help dialog (always on Markdown tab) | Help header says "Press F1 to toggle" but F1 only opens; Esc/close button/backdrop closes |
| `closeDialogs` | Escape | `Escape` | `closeActiveWorkspaceOrDialogs`: if a `<dialog open>` exists → nothing (native dialog handles Esc); if Library workspace visible → close Library; else close explorer context menu, explorer background menu, help dialog | `preventDefault: false` |

Other global keyboard handlers:

| Key | Where | Behavior |
|---|---|---|
| Escape (capture phase on `document`) | `app_controller.js:3165-3201` | First runs `onWorkspaceKeydown`. In Library workspace: close library preview → else close context menu → else close Library. Defers if any `<dialog open>` or other visible `[role=dialog]`. |
| Escape (`keydown@document->app#onWorkspaceKeydown`) | `app_controller.js:1898-1925` | Ignored when a `<dialog open>` exists. Priority: cancel header title rename → close overflow ("…") menu → hide explorer background menu → close mobile sidebar overlay. |
| Tab / Shift+Tab | same | Mobile sidebar overlay focus trap (wraps first/last focusable). |
| Ctrl+Y, Ctrl+Shift+Z (Cmd variants) with **no file open** | `app_controller.js:2811-2839` | "Redo" a created-note deletion at root boundary (created-note undo/redo is covered by another section). Ignored inside inputs/textareas, with Alt held, or if event already defaultPrevented. |

### A.3 Editor (CodeMirror) keybindings

Keymap order/precedence as built in `JS/lib/codemirror_extensions.js:264-320`:

1. `vimCompartment` (vim keymap when enabled; empty otherwise).
2. History-boundary keymap at `Prec.highest` (`codemirror_extensions.js:215-230`).
3. `keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab])` (default precedence).
4. `markdownKeymap` at `Prec.highest` (`codemirror_extensions.js:166-213`).
5. `markdown()` language's own keymap (default `addKeymap: true`): Enter / Backspace markup continuation.
6. `autocompletion()`'s `completionKeymap` (default).

#### A.3.1 FrankMD custom editor bindings

| Key | Command | Exact behavior |
|---|---|---|
| Mod-b | bold | `wrapSelection(view,"**","**")` |
| Mod-i | italic | `wrapSelection(view,"*","*")` (overrides defaultKeymap's Mod-i selectParentSyntax) |
| Mod-Shift-s | strikethrough | `wrapSelection(view,"~~","~~")` |
| Mod-` | inline code | `wrapSelection(view,"`","`")` |
| Mod-k | link | With selection: replace with `[sel](url)` and select the literal `url` (anchor = from+len+3, head = from+len+6). Empty: insert `[](url)`, cursor at from+1 (inside `[]`). |
| Mod-z | undo (boundary) | If `undoDepth > 0` → falls through to normal undo. At history start → app's "undo note creation" handler (disabled in vim mode). |
| Mod-y (mac: Mod-Shift-z), Ctrl-Shift-z | redo (boundary) | Same idea at redo end → "redo note creation" handler. |

`wrapSelection(view, prefix, suffix)` algorithm (`codemirror_extensions.js:124-163`), main selection only:
1. If text immediately before selection === prefix AND immediately after === suffix → **unwrap** (remove both), keep inner text selected.
2. Else if selected text starts with prefix, ends with suffix and length ≥ prefix+suffix → replace with inner text, selected.
3. Else wrap: insert `prefix + sel + suffix`, keep the original text selected (selection shifted by prefix length). With empty selection this inserts `****` (etc.) with cursor in the middle.

(Help dialog does not document Mod-Shift-s, Mod-`, Mod-k.)

#### A.3.2 Markdown surround on typing (`JS/lib/codemirror_markdown_surround.js`)

An `EditorView.inputHandler`. When the user **types** one of `* _ ` ~ ^ =` while at least one selection range is non-empty (not read-only, not IME composing, user event `input.type`):
- For each non-empty range: insert the char before and after (selection preserved, shifted). Empty ranges just get the char inserted.
- Backtick special case (`delimiters`, lines 7-14): delimiter = backticks of length (longest backtick run inside content + 1); add one space padding inside when content starts/ends with a backtick, or starts AND ends with a space while not all-space.
- Dispatched with `isolateHistory "full"` (its own undo step) and `scrollIntoView`.

#### A.3.3 Standard CodeMirror keymaps loaded

Loaded: `defaultKeymap` (includes `standardKeymap`), `historyKeymap`, `searchKeymap`, `indentWithTab`, markdown language keymap, `completionKeymap`. **Not** loaded: `foldKeymap`/fold gutter, `closeBracketsKeymap` (no auto-closing brackets), `lintKeymap`. Notable bindings that reach the user (vendored versions: `@codemirror/commands` 6.10.1, `@codemirror/search` 6.6.0):

| Key (Win/Linux) | mac | Command |
|---|---|---|
| Arrows / Shift+Arrows, Home/End, PageUp/PageDown | | cursor move / extend selection |
| Mod-Arrow Left/Right | Alt-Arrow | group (word) move |
| Alt-ArrowLeft/Right | Ctrl-Arrow | syntax-aware / line-boundary move |
| Mod-Home / Mod-End | | doc start / end |
| Enter | | `insertNewlineContinueMarkup` (markdown keymap wins over `insertNewlineAndIndent`): continues bullet/ordered/task lists and blockquotes; Enter on an empty list item removes the marker |
| Backspace | | `deleteMarkupBackward` (markdown) else deleteCharBackward |
| Delete, Mod-Backspace, Mod-Delete | Alt-Backspace/Delete | delete char / group |
| Mod-a | | select all |
| Mod-z, Mod-y / Mod-Shift-z | | undo / redo (historyKeymap) |
| Mod-u, Alt-u (mac Mod-Shift-u) | | undo/redo selection |
| Alt-ArrowUp/Down | | move line(s) up/down |
| Shift-Alt-ArrowUp/Down | | copy line(s) up/down |
| Mod-Alt-ArrowUp/Down | | add cursor above/below |
| Escape | | simplify selection (collapse to main) |
| **Mod-Enter** | | `insertBlankLine` — **overlaps** global Ctrl+Enter toggle-task (see below) |
| Alt-l | Ctrl-l | select line |
| Mod-[ / Mod-] | | indent less / more |
| Mod-Alt-\ | | indent selection |
| Shift-Mod-k | | delete line |
| Shift-Mod-\ | | cursor to matching bracket — **overlaps** global toggleScrollSync |
| Mod-/ | | toggle comment (markdown → `<!-- -->`) |
| Alt-A (Shift-Alt-a) | | toggle block comment |
| Ctrl-m | Shift-Alt-m | toggle Tab-focus mode — **overlaps** global Ctrl+M |
| Tab / Shift-Tab | | `indentWithTab`: indentMore / indentLess (indent unit = CodeMirror default, 2 spaces — see A.12) |
| Mod-f | | open CM built-in search panel — **overlaps** global Ctrl+F |
| F3 / Mod-g, Shift-F3 / Mod-Shift-g | | find next / previous (opens CM panel if no query) — Mod-g **overlaps** global Ctrl+G |
| Mod-Shift-l | | select all matches of selection |
| Mod-Alt-g | | CM go-to-line prompt |
| Mod-d | | select next occurrence (multi-cursor) |
| Escape (panel) | | close search panel |
| Ctrl-Space | | start completion |
| ArrowUp/Down, PageUp/Down, Enter, Escape (completion open) | | navigate / accept / close completion (Tab does NOT accept) |
| Alt+drag | | `rectangularSelection()` |

**Overlap warning (unsure of real runtime behavior):** CodeMirror's keymap handlers run on the content DOM before the bubbling `document` listener and do not `stopPropagation`; the global handler ignores `defaultPrevented`. So in the focused editor, Ctrl+F / Ctrl+G may open CodeMirror's own search panel **and** FrankMD's dialog; Ctrl+Enter may insert a blank line (moving the cursor) **before** toggling the task; Ctrl+M toggles tab-focus mode as well as opening the format menu. A reimplementation should treat the FrankMD action as the intended behavior and not bind the CM duplicates. The CM search panel is themed (`codemirror_theme.js:123-153`, `codemirror.css` "Panels") so it can appear.

#### A.3.4 Vim mode keys

See A.9. Summary: full `@replit/codemirror-vim` 6.3.0 keymap; on **non-macOS** these vim bindings are unmapped once per page so the app keeps them: `<C-f> <C-n> <C-p> <C-e> <C-b> <C-i> <C-v>` (`JS/lib/vim_mode.js:41-49, 59-65`). `<C-d>/<C-u>` paging kept; blockwise visual via `<C-q>`. macOS keeps the full vim keymap (app uses Cmd).

### A.4 Dialog / widget local keys

| Widget | Keys |
|---|---|
| Find & Replace inputs (find and replace fields) | Enter = find next; Shift+Enter = find previous; Alt+Enter (Replace tab) = replace current and advance; Escape = close (also a document-level Escape listener while dialog or mini-bar is shown) |
| Jump to Line input | Enter = jump; Escape = close (+ refocus editor) |
| Text Format menu | ArrowDown/ArrowUp (wrap); Enter = apply; Escape = close + refocus editor; letter hotkeys B I S H U P L (case-insensitive) apply directly |
| Help dialog tab buttons | ArrowLeft/ArrowRight cycle tabs (wrap) and focus the tab; mouse wheel on tab bar: deltaY>0 or deltaX>0 → next, else previous; Escape (native dialog) closes; click on backdrop closes |
| File finder input | ArrowDown/ArrowUp (clamped, no wrap), Enter = open |
| Content search input | ArrowDown/ArrowUp (clamped, scroll into view), Enter = open result at line |
| Emoji picker grid | ArrowRight/Left (wrap through all), ArrowDown/Up by column count (wrap to same column in first/last row), Enter = insert; Escape = native close. Tabs: ArrowLeft/Right |
| Image picker tabs | ArrowLeft/Right cycle over `drop, local, folder, web, google, pinterest, ai` |
| Image source search fields (web/google/pinterest) | Enter = search; AI prompt: Enter (without Shift) = generate, Escape aborts generation |
| Drop zones (image/video) | Enter or Space opens native file picker |
| Video dialog | URL field Enter = insert (if enabled); YouTube search Enter = search, ArrowDown = focus first result; result grid: Right/Down (+1/+2), Left/Up (−1/−2; Up from first row → back to search input), Enter = select, Escape = back to input; tab bar ArrowLeft/Right |
| Code dialog — language field | Tab = accept first suggestion (if list visible); ArrowDown = focus suggestions; Escape = hide suggestions; Ctrl/Cmd+Enter = insert |
| Code dialog — suggestions | ArrowDown/Up move (Up from first → language field), Escape → hide + focus field |
| Code dialog — content textarea | Ctrl/Cmd+Enter = insert; Tab = indent (2 spaces, or `\t` if "indent with tabs" checked), Shift+Tab = outdent; multi-line selection indents/outdents all lines |
| New note / New item / Rename dialogs | Enter = submit; Escape = close |
| Header title rename input | Enter = submit, Escape = cancel (blur also commits) |
| App prompt (confirm/prompt) | Enter in input = accept; Escape (`cancel`) = reject |
| Floating dropdowns (theme/locale/media-storage menus) | ArrowDown/Up/Home/End open+navigate (wrap), Tab closes, Escape closes and restores focus |
| Workspace overflow "…" menu | ArrowDown/Up/Home/End (opens if closed; Up/End focus last), ArrowLeft/Right move between action and its pin toggle (RTL-aware), Tab closes |
| Explorer tree item (focused & selected) | Delete = delete selected items (excl. `.fed` config); F2 = rename (exactly one selected, not config). No modifiers, no auto-repeat |
| Explorer click modifiers | Ctrl/Cmd+click toggle item in multi-selection (does not open); Shift+click range from anchor (Ctrl+Shift adds range); plain click selects & opens |
| Library dialogs | Tab focus trap; `keydown.esc` closes usage dialog |
| AI grammar processing | Escape aborts request |
| Log viewer | Escape closes |

### A.5 Help dialog content (`app/views/notes/dialogs/_help.html.erb`)

Title "Help", header hint "Press F1 to toggle", close button. Tabs (order `markdown, shortcuts, vim`, `help_controller.js:66-68`): "Markdown", "Keyboard Shortcuts", "Vim Mode". Active tab classes `bg-[var(--theme-accent)] text-[var(--theme-accent-text)]`.

- **Markdown tab** (2 columns, source → rendered): Headings (`#`, `##`, `###`), Emphasis (`**bold**`, `*italic*`, `~~strikethrough~~`, `` `inline code` ``), Lists (`- Item`, `1. First`), Task List (`- [x] Done`, `- [ ] Todo` with disabled checkboxes), Links & Images (`[text](url)`, `![alt](image.jpg)`), Blockquote (`> Quote text`), Code Block (```` ```javascript ````), Table (`| Col 1 | Col 2 |` …), Horizontal Rule (`---`).
- **Keyboard Shortcuts tab**:
  - File Operations: Ctrl+S Save file; Ctrl+N New note; Delete "Delete selected Explorer item"; F2 "Rename selected Explorer item"; Ctrl+P File finder; Ctrl+Shift+F Search in files; Ctrl+F Find in file; Ctrl+H Find and replace; Ctrl+G Go to line.
  - Editor: Ctrl+E Toggle sidebar; Ctrl+Shift+V Toggle preview; Ctrl+\ Typewriter mode; Ctrl+Enter "Toggle task / add task"; Ctrl+L Toggle line numbers; Ctrl+Shift++ Increase editor width; Ctrl+Shift+- Decrease editor width; Tab "Indent line/block"; Shift+Tab "Unindent block".
  - Insert: Ctrl+Shift+E Emoji picker; text "Type / in a paragraph to add a block, equation, table, image, video, or emoji."; Ctrl+M Text format menu; "Context menu (with selection)".
  - Text Format Menu: Ctrl+B Bold; Ctrl+I Italic; S Strikethrough; H Highlight; U Subscript; P Superscript; L Link.
  - Dialogs: F1 Help; Ctrl+Shift+O Log viewer; Escape Close dialog; ←/→ "Switch tabs (when tab focused)"; ↑/↓/←/→ Navigate items; Enter Select/confirm.
  - Not listed in help: Ctrl+Shift+\ (scroll sync), Ctrl+Shift+S, Ctrl+`, Ctrl+K.
- **Vim Mode tab**: intro "When Vim mode is on, the editor uses modal editing. Toggle it from the header button or the editor settings. Standard Vim motions and editing work; the commands below are FrankMD-specific." (toggle actually lives in Settings). Modes list: "Normal — navigate and run commands (Esc to return here)", "Insert — type text (i, a, o, …)", "Visual — select text (v, V)". Ex-command table (`:w  :write` Save the note; `:q  :quit` Close menus / dialogs; `:wq  :x` Save, then close menus; `:e  :find` Open the file finder; `:Explore` Toggle the sidebar; `:n  :next` Open the next note; `:prev` Open the previous note; `:help` Open this help).

---

### A.6 Editor component structure (`app/views/notes/_editor_panel.html.erb`)

```
<main data-app-target="editorPanel" class="flex-1 flex flex-col min-w-0 relative">
  <span class="workspace-table-hint hidden" data-app-target="tableHint">Cursor in table - click to edit</span>
  <div data-app-target="editorContainer" class="flex-1 overflow-hidden">
    <div data-app-target="editorPlaceholder">  (home screen partial) </div>
    <div data-app-target="editor" class="h-full hidden relative" data-controller="codemirror typewriter" ...>
      <div class="h-full flex editor-wrapper" (typewriter wrapper)>
        <div class="flex-1 h-full editor-body" (typewriter body)>
          <div class="h-full cm-editor-container" data-codemirror-target="container"
               data-action="contextmenu->app#onTextareaContextMenu"></div>
          <textarea class="hidden" name="document[content]"></textarea>   ← mirror of doc on every change
          <div data-app-target="vimStatus" class="hidden absolute bottom-1 right-2 z-10 px-1.5 py-0.5 rounded
               text-[11px] font-medium tracking-wide bg-[var(--theme-bg-tertiary)] text-[var(--theme-text-muted)]
               pointer-events-none select-none"></div>
```

- Events emitted by the editor controller: `ready`, `change {content, docChanged}`, `selection-change {selection, cursorInfo}`, `scroll {scrollTop, scrollHeight, clientHeight, scrollRatio}`, `image-pasted {file}`, `vim-command {command,args}`, `vim-mode {enabled, mode}`.
- No placeholder text is configured (`placeholder` value defaults to `""`; locale `editor.start_writing` is unused).
- The same editor is used for `.md` notes ("markdown"), the `.fed` config file ("config") and other files ("text") (`app_controller.js:1116-1121`). Markdown-only features: toolbar, preview, slash commands, text-format menu, typewriter, task toggle, emoji, table hint.
- When a note with empty content is opened, the editor auto-focuses on the next animation frame unless focus moved, a dialog is open or the editor is hidden (`app_controller.js:1097-1104`).

#### A.6.1 Per-note editor state & undo

- Each note's full `EditorState` (doc + undo history + selection) is cached in an LRU Map keyed by path, **cap 20** (`codemirror_controller.js:44`). Switching notes stores the old state and restores the new one if its doc text equals the loaded content; otherwise history is cleared and content replaced without emitting a change (`loadContent`, lines 304-342). Active note is never evicted. Rename/move remaps cached paths (folder moves remap descendants); delete evicts.
- After restoring a cached state, global settings are re-applied (theme/font, line numbers, vim, read-only, recenter-on-click, typewriter).
- Read-only: `setReadOnly(bool)` plus a token-based `acquireReadOnlyLock()`; editor is read-only if the flag OR any lock is held (sets both `EditorState.readOnly` and `EditorView.editable`).

### A.7 CodeMirror extension list & configuration

From `createExtensions` (`codemirror_extensions.js:248-376`) + typewriter extension:

| Extension | Config |
|---|---|
| Vim (compartment) | `vim({ status: true })` + `vimCursorTheme` when enabled; `[]` when disabled |
| History-boundary keymap | Prec.highest, A.3.1 |
| Theme (compartment) | `createTheme({fontFamily, fontSize, lineHeight})` = editor theme + syntax HighlightStyle |
| Line numbers (compartment) | OFF / ABSOLUTE / RELATIVE (A.8) |
| Read-only (compartment) | `EditorState.readOnly.of(false)` initially |
| History (compartment) | `history()` default options (newGroupDelay 500 ms default) |
| `markdown({ base: markdownLanguage })` | GFM (tables, task lists, strikethrough, autolinks) + Subscript, Superscript, Emoji extensions (what `markdownLanguage` bundles). **No `codeLanguages`** → no nested highlighting inside fenced code in the editor. Default `addKeymap` (Enter/Backspace markup). HTML tag completion is shadowed because autocompletion uses `override` |
| `markdownSurround` | A.3.2 |
| secondary selection layer / secondary cursor layer | draws non-main selection ranges (`cm-selectionBackground`) and cursors (`cm-cursor cm-cursor-secondary`); main selection uses **native browser selection** (no `drawSelection`) |
| `rectangularSelection()` | Alt+drag. `allowMultipleSelections` is NOT enabled explicitly (CM default false) — **unsure** whether multi-cursor actually works |
| `highlightActiveLine()` | styled transparent (effectively invisible) |
| `highlightSelectionMatches({ minSelectionLength: 3, wholeWords: true })` | `.cm-selectionMatch`: transparent bg, `outline: 1px solid color-mix(accent 50%, transparent)`, radius 2px |
| `bracketMatching()` | focused only: matching = selection bg + 1px accent outline; non-matching = `rgba(255,0,0,.2)` bg + `rgba(255,0,0,.5)` outline |
| `EditorView.lineWrapping` | **always on** (soft wrap; no toggle) |
| keymaps | A.3 |
| autocompletion | wikilink + slash sources (A.14, A.15) |
| placeholder | only if text non-empty (never) |
| update listeners | docChanged → `change`; selectionSet → `selection-change` |
| scroll ViewPlugin | scroll on `scrollDOM` → `scroll` event |
| paste handler | intercepts **only image** clipboard data → `image-pasted` (handled by image picker; other agent). Plain text paste is default CM. **No smart paste** (URL over selection does NOT create a link) |
| typewriter (`codemirror_typewriter.js`) | state fields + ViewPlugin (A.10) |

Not used: fold gutter, closeBrackets, autoindent extras, lint, spellcheck attributes, `drawSelection`, `dropCursor` (only styled).

Mouse behavior (`codemirror_controller.js:135-220`):
- Left mousedown inside scroller sets `isSelecting` (suppresses `selection-change` and `scroll` events + typewriter recentering while dragging). On mouseup: clear flag; if the click started and ended inside content and selection is empty → `recenterCursorAfterClick` (only when "cursor recenter on click" setting on); after **50 ms** dispatch a `selection-change`.
- Click on blank scroller margin (target is the scroller itself, pointer moved ≤ **4 px**, not on the scrollbar, RTL-aware): place caret at `posAtCoords` with x clamped into the text column (padding +1px), or doc start/end if above/below content; focus; recenter if setting on.

### A.8 Line numbers

- Modes (`JS/lib/line_numbers.js`): `OFF=0`, `ABSOLUTE=1`, `RELATIVE=2`. Setting `editor_line_numbers`, default **0 (off)** (settings select "Off/Absolute/Relative"). Ctrl+L cycles 0→1→2→0 and persists.
- ABSOLUTE: `lineNumbers()` + `highlightActiveLineGutter()`.
- RELATIVE: cursor line shows its absolute number; other lines show `|line − cursorLine|` (`codemirror_extensions.js:101-117`), active gutter highlighted.
- Gutter style: bg `--theme-bg-secondary`, color `--theme-text-faint`, right border 1px `--theme-border`, font-size 0.75rem; element padding `0 0.75rem 0 0.5rem` (CSS) / `0 0.5rem` (theme), min-width 2.5rem / 3ch, right-aligned; active line number color `--theme-text-muted`.

### A.9 Vim mode

- Library `@replit/codemirror-vim` 6.3.0 (vendored). Setting `vim_mode` (bool, default false), toggled by Settings switch (`app#toggleVimMode`) — applied live via compartment and persisted (`saveConfig` debounced 500 ms).
- `status: true` → library's built-in bottom status / `:` command-line panel.
- Fat (block) cursor themed: bg `--theme-accent`, text `--theme-accent-text`; unfocused: no bg, `outline: 1px solid accent` (`codemirror_extensions.js:64-74`).
- Mode indicator element (`vimStatus`): visible only when vim on; text `VIM · NORMAL` / `VIM · INSERT` / `VIM · VISUAL` / `VIM · REPLACE` — uppercase first segment of mode (`visual-linewise` → `VISUAL`) (`app_controller.js:3340-3350`). Emits "normal" immediately on enable.
- Ex-commands (`vim_mode.js:22-33`, `Vim.defineEx(name, prefix, …)`):

| Ex (full, shortest prefix) | Logical command | App action |
|---|---|---|
| `:write` / `:w` | save | save now |
| `:quit` / `:q` | close | `closeDialogs` (closes menus/dialogs/Library — **does not close the note**) |
| `:wq` | save-and-close | save, then closeDialogs |
| `:xit` / `:x` | save-and-close | same |
| `:help` | help | open Help |
| `:edit` / `:e` | finder | open file finder (args ignored) |
| `:find` | finder | open file finder |
| `:Explore` | toggle-sidebar | toggle sidebar |
| `:next` / `:n` | next-note | open next file in tree order |
| `:prev` | prev-note | open previous file |

  `:noh` intentionally left native. Next/prev: list = all `.tree-item[data-type="file"]` in DOM order (only rendered/expanded ones); wraps around; when no note selected, next → first, prev → last (`nextNoteIndex`, `vim_mode.js:43-47`).
- In vim mode the created-note undo/redo boundary handlers are disabled; slash-command completion is disabled in normal mode.
- Non-mac key unmapping: see A.3.4.

### A.10 Typewriter mode

- Setting `typewriter_mode` (bool, default false). Toggle: Ctrl+\ (markdown only) or Settings "Typewriter" switch (no markdown guard). Effective only when the open file is markdown. Persisted on every toggle.
- Editor effects (`codemirror_typewriter.js`):
  - Scroller gets `padding-bottom = scrollDOM.clientHeight / 2` px (so last line can reach center); editor root gets class `typewriter-mode`. Recomputed on editor resize (ResizeObserver) and when becoming visible.
  - Centering: target `scrollTop = cursorY − clientHeight/2`, where `cursorY = coordsAtPos(head).top − editorRect.top + scrollTop`, clamped to `[0, scrollHeight−clientHeight]`; applied only if `|current − target| > 5 px` (instant, not smooth). Runs in `setTimeout(0)` after any transaction with selectionSet or docChanged, **except** while mouse-selecting, when a non-empty selection exists, or for `select.pointer` transactions. Also centers immediately on enable.
  - `.editor-body.typewriter-centered` → `justify-content: center`; `body.typewriter-mode .cm-editor .cm-content { max-width: calc(var(--editor-width,72ch) + 4rem); margin-inline:auto; padding:1rem 2rem }`.
- Preview effects (`app_controller.js:2172-2198`, `preview_controller.js:527-575, 611-617`):
  - In split view, enabling typewriter **auto-shows the preview** if hidden (not in single-view mode, not when Library visible).
  - Preview content gets `preview-typewriter-mode` (`width:100%; max-width: calc(var(--editor-width,72ch) + 3rem) !important; margin-inline:auto`).
  - On every editor selection change: preview scrolls so the element mapped to the cursor's source line is at the **top** of the preview (fallback ratio `(line−1)/(total−1)` of content height); double rAF; smooth scroll; only if change > **10 px** threshold; skipped when scroll sync disabled or total lines ≤ 1.
- Separate setting "Cursor recenter on click" (`cursor_recenter_on_click`, default false): after a completed single click with empty selection, apply the same centering (works without typewriter).

### A.11 Text Format context menu (`text_format_controller.js`, `_text_format_menu.html.erb`)

- Opens on: right-click in editor **with a non-whitespace selection** in a markdown file (else native context menu) at mouse coords; or Ctrl+M at `coordsAtPos(selection.to)` → (left, bottom+4) (fallback: editor center).
- Container: `.frankmd-menu fixed bg-secondary border rounded-lg shadow-lg py-1 min-w-[180px] z-50`, focusable (`tabindex=-1`), focused on open. Positioning: clamp to viewport with 10 px padding; if overflowing bottom, flip above `y − height` (`positionMenu`) plus `clampMenuToViewport`.
- Items (in order), label with hotkey letter underlined (first case-insensitive occurrence) and hotkey shown at right:

| id | Label | Hotkey | prefix | suffix |
|---|---|---|---|---|
| bold | Bold | B | `**` | `**` |
| italic | Italic | I | `*` | `*` |
| strikethrough | Strikethrough | S | `~~` | `~~` |
| highlight | Highlight | H | `==` | `==` |
| subscript | Subscript | U | `~` | `~` |
| superscript | Superscript | P | `^` | `^` |
| link | Link | L | `[` | `](url)` |

- Selected item: `bg-[var(--theme-accent)] text-[var(--theme-accent-text)]`; hover changes selection; first item selected on open.
- Apply (`applyFormatToEditor`, lines 411-464): symmetric formats toggle — unwrap if selection itself is wrapped, or if markers sit just outside it (selection kept on inner text). Otherwise replace with `prefix+text+suffix`; link selects the literal `url`; all others put the **cursor after** the wrapped text (differs from Ctrl+B which keeps selection). Then close, schedule autosave, update preview. Mousedown outside closes. Escape closes and refocuses editor.

### A.12 Indentation

- Tab / Shift+Tab in editor = CodeMirror `indentWithTab` (indentMore/indentLess on all selected lines) using CodeMirror's default indent unit (**2 spaces**). Inside a list, Tab indents the whole line(s).
- Setting `editor_indent` (Settings → Indent: options 2, 4, 8 spaces; default 2) is persisted but **not wired into CodeMirror** (no `indentUnit` configured; `app#getIndentString` is unused). `lib/indent_utils.js` (`parseIndentSetting`: empty/invalid → 2 spaces, `0` → tab, 1–6 spaces clamped; `indentLines`; `unindentLines` removes exact indent, else up to N leading spaces, else one tab) is legacy/tests only. **Unsure** whether the reimplementation should honor the setting (probably intended).
- List continuation on Enter: markdown `insertNewlineContinueMarkup` (bullets `-*+`, ordered `1.`/`1)` renumbered, task `- [ ] `, blockquote `> `); Enter on an empty item exits the list. Backspace at markup start removes markup (`deleteMarkupBackward`).

### A.13 Task list toggle

- Regex (`JS/lib/task_utils.js:7`): `/^(\s*(?:[-*+]|\d{1,9}[.)])\s+)\[([ xX])\](\s|$)/`.
- Ctrl+Enter (`codemirror_controller.js:854-879`): for every line touched by the main selection: if task → replace only the 3-char marker (`[ ]`→`[x]`, `[x]`/`[X]`→`[ ]`); else convert line to unchecked task: `indent + "- [ ] " + trimmedBody`, or `indent + "- [ ]"` for blank lines. (A plain `- item` becomes `- [ ] - item` — the code does not reuse an existing bullet.) One transaction (one undo step).
- Preview checkbox click → `toggleTaskAtLine(line)` toggles only that marker (no-op if not a task).

### A.14 Slash commands (`JS/lib/codemirror_slash_commands.js`)

- Trigger regex on text before cursor (same line): `/(?:^|[\s([{])\/([\p{L}\p{N}\p{M} -]*)$/u` — `/` at line start or after whitespace/`(`/`[`/`{` (so `docs/a.md` does not trigger). `validFor: /^[\p{L}\p{N}\p{M} -]*$/u`.
- Active only when: current file is markdown, not vim normal mode, slash not inside a link URL, and the slash is inside a `Paragraph` or `ATXHeading1-6` node and not inside `FencedCode`/`CodeText`/`InlineCode`.
- Options are filtered by CodeMirror's fuzzy matcher against `filterText`; rendered with a 1em SVG icon (opacity .75) from the icon map; max 20 rendered; list max-height 16em.

| Label (i18n key `editor.slash_commands.*`) | Icon | filterText extra | Result |
|---|---|---|---|
| Heading 1 / 2 / 3 | hash | `heading N headingN hN` | Remove `/query`, collapse soft line breaks of the whole paragraph into spaces, replace existing `#…` marker, prefix `"# "`/`"## "`/`"### "` (keeps leading whitespace, cursor preserved) |
| Bulleted list | list-bullets | bullet bulleted unordered list | Prefix every line `- ` (continuation lines indented 2 spaces). Already in a list item: switch ordered→bullet marker to `-` |
| Numbered list | list-numbers | numbered ordered list | Prefix `1. ` (continuation 3 spaces). In bullet list: change marker to `1.` |
| To-do list | list-checks | to-do todo task checklist | Prefix `- [ ] `; if already has `[ ]/[x]` marker → just `- `; inside a list item: insert `[ ] ` after the marker |
| Quote | quotes | quote blockquote | Prefix every line `> ` (no-op if already in blockquote) |
| Code block | code | code block fenced | Wrap block in fence of max(3, longest backtick run+1) backticks: ```` ```\ncontent\n``` ````; inside blockquote/list, continuation lines prefixed `> ` / spaces |
| Equation | sigma | equation math formula latex tex | Wrap in `$$\ncontent\n$$`; if slash is on a non-first line of a multi-line paragraph (not in list/quote): only that line becomes `\n$$\nline\n$$` (+`\n` if more lines follow) |
| Divider | minus | divider horizontal rule hr | Remove `/query`, then insert `\n\n---` after the block (after enclosing list item/blockquote) |
| Table | table | insert table grid | Close completion; dispatch `frankmd:open-slash-command {action:"table", from, to, query}` → opens table editor; on insert the `/query` range is replaced |
| Image | image | insert image picture photo | → image picker |
| Video | video-camera | insert video embed youtube | → video dialog |
| Emoji | smiley | insert emoji emoticon icon | → emoji picker; inline insertion replaces `/query` |

- Removing `/query` (`removeSlashQuery`): if spaces on both sides, drop one; if at block start drop following space; if nothing after, trim trailing spaces before.
- Pending slash range is validated on insert: if the text at `[from,to)` no longer equals the query, insertion is aborted. Dialog close clears pending range.
- Block insertion helper (`codemirror_content_insertion.js`): with a slash range, trailing/leading spaces/tabs around the range are absorbed; surrounding newlines normalized to a blank line before/after (`""`, `"\n"`, or `"\n\n"`). Images only need a single newline before/after. Code block cursor offset = `3 + language.length + 1` (first content line).

### A.15 Wikilink autocomplete (`JS/lib/codemirror_wikilink.js`)

- Activates when text before cursor on the same line contains `[[` whose remainder has no `]]`. Query = text after `[[` lowercased; `from` = just after `[[`.
- Source: files from the Explorer DOM (`getFilesFromTree`: `{path, name = basename without .md}`), only paths ending `.md`.
- Scoring (no match → excluded): empty query → 1; name startsWith → 3; name includes → 2; path includes → 1. Passed as `boost`; `filter: false` (own filtering). Label = note name; detail = path without `.md` **only if it contains `/`**.
- Apply: replace `[from, cursor)` with `path-without-.md + "]]"`, cursor after `]]`. (No closing-bracket auto-pairing exists, so `]]` is always appended.)
- Autocomplete config: `activateOnTyping: true`, `maxRenderedOptions: 20`, default type icons hidden (custom icon column at position 20). Tooltip: bg `--theme-bg-secondary`, 1px border, radius .25rem, shadow `0 2px 8px rgba(0,0,0,.2)`.

### A.16 Find & Replace (`find_replace_controller.js`, `find_utils.js`, `_find_replace.html.erb`)

- **Non-modal** `<dialog>` (`show()`), 520 px wide, bg secondary, border, rounded-lg, shadow-xl.
- Header: tabs "Find" / "Replace" (active: accent bg + accent text; inactive muted); buttons Minimize (chevron-up) and Close (×).
- Row: find input (placeholder "Find...", autocomplete off, spellcheck off), Previous (↑) and Next (↓) buttons.
- Row: match counter (left) and toggles `Aa` ("Case sensitive (Aa)") and `.*` ("Use regular expression (.*)"); pressed state `bg-[var(--theme-bg-hover)]` + `aria-pressed`. Defaults both off. **No whole-word option.**
- Replace row (Replace tab only): input "Replace with...", buttons "Replace" (accent, title "Replace current match"), "Skip", "All" (title "Replace all matches").
- Open: pre-fill find input with current editor selection (if non-empty), switch tab, focus find input, compute matches, hide mini-bar. Current match index = first match with `start ≥ selectionStart` (wraps to 0); no jump until Next/Enter.
- Matching: pattern = raw (regex mode) or escaped literal (`/[.*+?^${}()|[\]\\]/g`); flags `g` + `i` unless case-sensitive (no `m`/`u`/`s`). Zero-length matches advance lastIndex by 1. Invalid regex → counter shows the JS error message in `text-red-500`.
- Counter text: empty when query empty; "No results"; "`{current} of {total}`".
- Next: from `selectionEnd`, first match with start ≥ pos, else wrap to first. Previous: from `selectionStart`, last match with end ≤ pos, else wrap to last. Jump = select match in editor, scroll it to center, **focus editor**.
- Replace current: replaces the match at current index (regex mode expands `$$`→`$`, `$&`/`$0`→match, `$N`→group N or ""), recompute. Clicking the **Replace button also advances** (Stimulus passes the event as the truthy `advance` arg); Alt+Enter = replace+advance; Skip = find next.
- Replace All: replaces all matches (from last to first) on the full text, sets editor value (one undoable change), cursor to 0, scroll top; counter shows "Replaced N" in green for **2000 ms** then recomputes.
- Matches are recomputed on find-input typing and toggle changes, **not** live on editor edits (the textarea adapter never receives input events from CodeMirror).
- Minimize: if query empty → close; else hide dialog and show **mini-bar** (fixed, top 88px, horizontally centered, z-40, bottom-rounded): mode label (uppercase 10px "FIND"/"REPLACE"), ↑, ↓, "Skip", counter, restore button (title "Open find dialog"). Mini ↓ in Replace mode = replace current + next; mini ↑ in Replace mode = replace current then find previous; Skip = next.
- Escape anywhere (document listener while open/minimized) closes and clears matches.

### A.17 Jump to Line (`jump_to_line_controller.js`, `_jump_to_line.html.erb`)

- Modal dialog 360 px, backdrop `bg-black/50`, header "Jump to Line" + close; text input placeholder "e.g. 10, +5, -5, -1"; primary button "Go"; status line "Line number, or +N/-N for relative; -1 for end"; footer "esc to close". Input cleared on each open.
- Parsing: trimmed empty → nothing. Leading `+`/`-` → relative (`current + parseInt`), **except exactly `-1` = last line**. Otherwise `parseInt` absolute. NaN → nothing (dialog stays). Clamp to `[1, totalLines]`.
- Effect: close dialog, put cursor at line start, scroll line to vertical center, focus editor.

### A.18 Document stats panel (`stats_panel_controller.js`, `stats_utils.js`, sidebar)

- Location: Explorer sidebar. Visible only when setting `show_stats` (default false, Settings "Stats — Show document statistics in the Explorer") AND a file is open.
- Rows: "Words", "Characters", "Size", "Read time", "Line" (initial `0`, `0`, `0 B`, `0 min`, `1/1`).
- Words = count of `/\S+/g` matches. Characters = `text.length` (UTF-16 code units, includes whitespace/newlines). Size = UTF-8 byte length, formatted: `0 B`; `<1024` → `N B`; else base-1024 units KB/MB/GB with 1 decimal if value < 10 else 0 decimals (e.g. `1.5 KB`, `12 KB`). Read time = `ceil(words/200)` min; shows `< 1 min` when ≤ 1, else `N min`. Numbers use `toLocaleString()`.
- Line = `currentLine/totalLines` (cursor head), updated immediately on every selection change.
- Updates on edit are debounced **500 ms**; recomputation is skipped when the text **length** is unchanged since last compute (same-length edits don't refresh word count). Immediate update on file open.

### A.19 Syntax highlighting in the editor

Editor theme (`JS/lib/codemirror_theme.js`, `{dark:true}`, colors via CSS vars):

| Element | Style |
|---|---|
| root | `height:100%`, font-size = setting px, bg `--theme-bg-primary`, color `--theme-text-primary`; focused outline none |
| `.cm-scroller` | font-family = setting, line-height = 1.6, overflow auto, cursor text |
| `.cm-content` | `max-width: var(--editor-width, 72ch)`, centered (`margin-inline:auto`), padding `1rem 2rem`, caret `--theme-text-primary` |
| cursor | 2px left border `--theme-text-primary` |
| selection | `--theme-selection` (CSS uses `!important`) |
| search match | bg selection + 1px accent outline; selected match bg accent (+ accent text) |
| placeholder | faint italic |
| scrollbar | 14px, thumb `--theme-scrollbar-hover`, rounded pill, transparent track |

HighlightStyle tag → style:

| Lezer tag | Style |
|---|---|
| heading1 / heading2 / heading3 | color `--theme-heading-1/-2/-3` |
| heading4–6 | `--theme-accent` |
| strong | bold, `--theme-text-primary` |
| emphasis | italic, `--theme-text-muted` |
| strikethrough | line-through, muted |
| monospace (inline code **and fenced code body**) | color accent, bg `--theme-code-bg`, radius .25rem, padding `0 .25rem` |
| link | accent + underline; url → accent |
| quote | muted |
| list | muted |
| processingInstruction, meta (markup chars `#`, `*`, `>` etc.) | muted |
| contentSeparator (`---`) | `--theme-border` |
| angleBracket | muted; tagName accent; attributeName heading-2; attributeValue heading-3 |
| comment (HTML comment) | faint italic |
| labelName, inserted | accent |
| deleted | muted + line-through |
| escape | muted |

Headings are NOT size-scaled in the editor (CSS classes `cm-header-*` add bold, but CM6 HighlightStyle emits generated classes, so mostly color only). Fenced code blocks in the editor have **no per-language highlighting**.

### A.20 Fenced code language highlighting (preview only) — `JS/lib/code_block_highlighter.js`

Used by preview (`preview_controller.js:248`), not the editor. Applies to `pre > code[class^=language-]`; sets `pre[data-code-language]` label; limits: ≤ 20,000 chars per block, ≤ 50,000 chars per render, ≤ 100 blocks per render. Unknown/unsupported → plain text. Lazy parsers load on demand (cached, retry on failure).

| Fence names | Label | Parser |
|---|---|---|
| js, javascript, mjs, cjs / jsx / ts, typescript / tsx | JavaScript / JSX / TypeScript / TSX | @codemirror/lang-javascript (static) |
| html, htm / svg | HTML / SVG | lang-html |
| css | CSS | lang-css |
| json / jsonc | JSON / JSONC | javascript parser |
| yaml, yml, .yaml, .yml | YAML | @lezer/yaml (lazy) + scalar → constant/number reclassification |
| php, phtml | PHP | @lezer/php |
| ruby, rb, .rb, rake | Ruby | legacy-modes ruby |
| env, .env, dotenv | Dotenv | legacy shell |
| shell, bash, sh, zsh | Shell | legacy shell |
| sql / postgres, postgresql, pgsql / mysql / mariadb / sqlite | SQL / PostgreSQL / MySQL / MariaDB / SQLite | legacy sql (standardSQL/pgSQL/mySQL/mariaDB/sqlite) |
| toml, tml | TOML | legacy toml |
| dockerfile, docker | Dockerfile | legacy dockerfile |
| tf, .tf, tfvars, .tfvars, terraform / hcl, .hcl | Terraform / HCL | custom stream parser `terraform_hcl_stream_parser.js` |
| rs, rust | Rust | @lezer/rust |
| go, golang | Go | @lezer/go |
| py, python, python3 | Python | @lezer/python |
| java | Java | @lezer/java |
| c, h, ino / cc, cpp, c++, cxx, hpp, hh, hxx | C / C++ | @lezer/cpp |
| cs, csharp, c# | C# | legacy clike csharp |
| kotlin, kt, kts | Kotlin | legacy clike kotlin |

Token classes: `tok-keyword, tok-function, tok-definition, tok-variableName, tok-propertyName, tok-constant, tok-typeName, tok-className, tok-namespace, tok-string, tok-string2 (regexp), tok-number, tok-operator, tok-punctuation, tok-comment, tok-invalid`.

Terraform/HCL stream parser: keywords `check connection data dynamic else for if import in locals module output provider provisioner removed resource terraform variable`; comments `#…`, `//…`, `/* */` (multi-line); heredoc `<<EOF`/`<<-EOF` until a line equal to the marker; strings `"…"` with `\` escapes; atoms `true false null`; numbers hex/bin/oct/decimal/exponent with `_`; identifier → property if after `.` or before `=`, function if before `(`, else variable.

Code dialog language suggestions (not the editor, but "code block language list"): `javascript typescript python ruby go rust java c cpp csharp php swift kotlin scala haskell elixir erlang clojure lua perl html css scss sass less json yaml toml xml markdown sql graphql bash shell powershell dockerfile makefile nginx apache vim regex diff git plaintext` — prefix/contains match, max 6 suggestions; unknown language (no prefix match and Levenshtein > 2) asks for confirmation. Empty content inserts ```` ```lang\n\n``` ````.

### A.21 Fonts, size, width

Editor fonts (`settings_controller.js:51-61`; setting `editor_font`, default `cascadia-code`):

| id | Display name | CSS font-family | Vendored file (`public/fonts`) |
|---|---|---|---|
| cascadia-code | Cascadia Code | `'Cascadia Code', monospace` | cascadia-code-latin-400 |
| consolas | Consolas | `Consolas, monospace` | — (system only) |
| dejavu-mono | DejaVu Sans Mono | `'DejaVu Mono', monospace` | dejavu-mono-latin-400 |
| fira-code | Fira Code | `'Fira Code', monospace` | fira-code-latin-400, 500 |
| hack | Hack | `Hack, monospace` | hack-latin-400 |
| jetbrains-mono | JetBrains Mono | `'JetBrains Mono', monospace` | jetbrains-mono-latin-400, 500 |
| roboto-mono | Roboto Mono | `'Roboto Mono', monospace` | roboto-mono-latin-400, 500 |
| source-code-pro | Source Code Pro | `Source Code Pro, monospace` | source-code-pro-latin-400, 500 |
| ubuntu-mono | Ubuntu Mono | `Ubuntu Mono, monospace` | ubuntu-mono-latin-400 |

UI font: **Inter** 400/500/600 (`app/assets/stylesheets/fonts.css`). All `@font-face` use `font-display: swap`, latin subset woff2. KaTeX fonts in `public/fonts/katex`.

- Font size: select options **12, 13, 14, 15, 16, 18, 20, 22, 24** px; default 14 (`editor_font_size`). No keyboard zoom for editor font.
- Line height: fixed **1.6** (not user-configurable).
- Font preview box in Settings: sample `// <font_preview_line>` / `function hello() {` / `  return "Hello, World!";` / `}` in the selected font/size, line-height 1.6.
- Editor width: CSS var `--editor-width` = `Nch` on `<html>`; setting `editor_width` default **72**, range slider **40–200 step 8**; keyboard ±8 (A.2). Applies as `.cm-content` max-width, centered.
- All settings persist via debounced (500 ms) `PATCH /config`.

### A.22 Timings & thresholds summary

| What | Value | Source |
|---|---|---|
| Autosave debounce after last edit | 2000 ms | `autosave_controller.js:13` |
| Autosave forced max interval while typing | 30000 ms | `autosave_controller.js:14` |
| Draft (local) write debounce | 300 ms | `autosave_controller.js:15` |
| Stats debounce | 500 ms | `stats_panel_controller.js:49` |
| Table-at-cursor detection debounce | 200 ms | `app_controller.js:1181-1192` |
| Config save debounce | 500 ms | `app_controller.js:1535-1558`, `settings_controller.js:229-253` |
| Toast (temporary message) | 2000 ms | `app_controller.js:2200` |
| "Replaced N" feedback | 2000 ms | `find_replace_controller.js:270` |
| Post-mouseup selection-change dispatch | 50 ms | `codemirror_controller.js:190-198` |
| Margin click tolerance | 4 px | `codemirror_controller.js:161` |
| Typewriter recenter threshold | > 5 px | `codemirror_typewriter.js:46` |
| Preview typewriter scroll threshold | > 10 px | `preview_controller.js:78` |
| Selection-match highlight min length | 3 chars, whole words | `codemirror_extensions.js:298` |
| Per-note editor-state cache | 20 notes (LRU) | `codemirror_controller.js:44` |
| Autocomplete max rendered options | 20 | `codemirror_wikilink.js:101` |

### A.23 Importmap — CodeMirror / Lezer packages (`config/importmap.rb`)

| Package | Version | Purpose |
|---|---|---|
| @codemirror/view | 6.39.11 | EditorView, keymaps, layers, gutters, placeholder |
| @codemirror/state | 6.5.4 | EditorState, compartments, transactions |
| @codemirror/language | 6.12.1 | syntax tree, HighlightStyle, bracketMatching, StreamLanguage |
| @codemirror/lang-markdown | 6.5.0 | markdown language + Enter/Backspace markup keymap |
| @lezer/markdown | 1.6.3 | Markdown parser (GFM etc.) |
| @codemirror/commands | 6.10.1 | default/history keymaps, indentWithTab |
| @codemirror/search | 6.6.0 | searchKeymap, highlightSelectionMatches |
| @codemirror/autocomplete | 6.20.0 | completion UI (wikilinks, slash commands) |
| @lezer/highlight | 1.2.5 | tags, tagHighlighter, highlightTree |
| @lezer/common, @lezer/lr | 1.5.3, 1.4.11 | parser runtime |
| @codemirror/lang-javascript / lang-html / lang-css | 6.2.4 / 6.4.11 / 6.3.1 | preview code highlighting (JS/TS/JSX/TSX/JSON, HTML/SVG, CSS) |
| @lezer/javascript, html, css | 1.5.4, 1.3.13, 1.3.0 | their grammars |
| @lezer/cpp, go, java, python, rust, yaml, php | 1.1.6, 1.0.1, 1.1.5, 1.1.19, 1.0.3, 1.0.4, 1.0.6 | lazy preview grammars |
| @codemirror/legacy-modes/mode/{clike, ruby, shell, sql, toml, dockerfile} | 6.5.5 | lazy preview stream modes |
| @replit/codemirror-vim | 6.3.0 | Vim mode |
| @marijn/find-cluster-break, crelt, style-mod, w3c-keyname | 1.0.2, 1.0.6, 4.1.3, 2.2.8 | CM internals |
| marked 15.0.6, katex 0.19.0, dompurify 3.4.17 | | preview rendering (other section) |

---

## 4. Data safety, saving & file lifecycle  
<sub>(sub-section labels below keep the source numbering, e.g. B.x)</sub>

Abbreviations: `AS` = `app/javascript/controllers/autosave_controller.js`, `DS` = `app/javascript/lib/draft_storage.js`, `APP` = `app/javascript/controllers/app_controller.js`, `CM` = `codemirror_controller.js`, `CMX` = `lib/codemirror_extensions.js`, `CON` = `connection_monitor_controller.js`, `RD` = `recovery_diff_controller.js`, `OB` = `offline_backup_controller.js`.

### B.1 Mental model (three layers of persistence)

| Layer | Where | Written when | Purpose |
|---|---|---|---|
| **Server file** | `PATCH /notes/<encoded path>` | debounced autosave (2 s idle / 30 s max), Ctrl+S, `:w`, reconnect | The real file on disk |
| **Draft record** (primary local safety net) | `localStorage["frankmd:draft:" + encodeURIComponent(path)]` | 300 ms after every edit; synchronously on file switch, tab hide, pagehide, beforeunload, going offline, before every server save | Survives reload/crash. Anchored to a server **revision** (SHA-256 of the content) |
| **Draft-conflict record** | `localStorage["frankmd:draft-conflict:" + encodeURIComponent(path) + ":" + conflictId]` | When a rename/move would overwrite an existing draft, or when recovery has to keep a second copy | Keeps a second copy instead of overwriting one |
| **Legacy offline backup** | `localStorage["frankmd:backup:" + path]` (raw path, **no encoding**; `DS:851-855`) | Only while offline: 1 s after each edit, plus once right when the connection drops | Older format. Still written while offline and still read on load |

There is **no IndexedDB, no BroadcastChannel, no `storage` event listener, and no multi-tab coordination** (grep shows none). Two tabs on the same note both write the same draft key, and the last write wins.

### B.2 Server revision / conflict detection

- Revision = `Digest::SHA256.hexdigest(content)` (`app/models/note.rb:75-77`). It is content-based, so mtime is not used.
- `GET /notes/<path>` (JSON) returns `{path, content, revision}` (`APP:995-1013`). The initial page embeds the same data as `data-app-initial-note-value` (`index.html.erb:8`).
- `PATCH /notes/<path>` body `{content}` returns `{path, message, revision}` (`notes_controller.rb:97-103`). **The save request does NOT send the base revision, so the server does not detect a conflict on save. The last writer wins.** The client only uses `revision` to (a) anchor drafts and (b) send it on deletes.
- `DELETE /notes/<path>?expected_revision=<rev>` (used by "undo created note") gets **409** `{error, code:"revision_conflict"}` if the file on disk has a different hash (`notes_controller.rb:109-123`, `notes_service.rb:75`).
- **Conflict detection between a draft and the file happens on the client, when the note loads** (see B.7): a draft whose `baseRevision !== serverRevision` means the file changed outside the app (or in another tab) after the draft was taken, so the user is asked to choose in the recovery dialog.
- Path encoding: `encodePath` from `lib/url_utils`.

### B.3 Autosave timings and triggers

Constants (`AS:13-15`):

```
SAVE_DEBOUNCE_MS     = 2000   // server save 2 s after last change
SAVE_MAX_INTERVAL_MS = 30000  // forced save at most 30 s after the first unsaved change, even while typing continuously
DRAFT_DEBOUNCE_MS    = 300    // localStorage draft write
offline backup       = 1000   // hard-coded setTimeout (AS:786-790)
"Saved" display      = 2000   // then cleared (AS:970-971)
```

| Trigger | Effect | Ref |
|---|---|---|
| Every editor change (`codemirror:change` → `APP#onEditorChange`) | `checkContentRestored(content)` → `scheduleOfflineBackup()` → `scheduleAutoSave()` | `APP:1127-1137` |
| `scheduleAutoSave` | If content equals `_lastSavedContent`: clear timers, delete the draft (if one exists), clear the dirty flag and status. Otherwise: queue a 300 ms draft write. If offline or the content-loss warning is active, only set dirty and stop. Otherwise, on the first dirty transition show "Unsaved changes", restart the 2 s debounce timer, and start the 30 s max timer if it is not already running. Both timers save a **snapshot** `{path, fileVersion, content, baseRevision}` taken when the save was scheduled, and only if that snapshot is still the current file (`isCurrentSaveSnapshot`) | `AS:795-865` |
| Ctrl+S (`keyboard_shortcuts.js:13` `save:{key:"s",ctrl:true}`) | `saveNow()` right away (ignored while the Settings dialog is open) | `APP:3216-3221` |
| Vim `:w` / `:wq` | `save`, then for `:wq` also `closeDialogs` | `APP:3353-3360` |
| **Blur** | **Nothing.** There is no blur trigger | — |
| **Switching files / going Home / browser back-forward / file not found** | `prepareForTransition()`: **writes the outgoing draft to localStorage synchronously, does NOT save to the server**, then cancels all timers. If the draft cannot be stored (no revision, storage error, path blocked), the navigation is **aborted** and the URL is restored | `AS:135-158`, `APP:476-490` |
| `visibilitychange` → hidden | write the draft synchronously (no server request) | `AS:557-559` |
| `pagehide` | write the draft synchronously | `AS:561-563` |
| `beforeunload` | listener only attached while the **current file has a draft-persistence failure**. It retries the write; if it fails again it calls `preventDefault()` + `returnValue=""` (the browser's standard "Leave site?" prompt) | `AS:565-583` |
| AI grammar check | if a debounce timer is pending, `await saveNow()` first (the server reads from disk) | `APP:2487-2491` |
| Reconnect | `saveNow()` if dirty | `AS:1032-1039` |
| Programmatic content (recovered draft applied on load) | `scheduleAutoSave()` forced even if CM saw no change | `APP:1071-1073`, `APP:261` |

`saveNow(snapshot?)` (`AS:867-997`), in order:
1. Return if a recovery for this path is pending, or the path is draft-blocked. If offline: set dirty and return. Return if there is no file, if the snapshot is stale, or if `_isSaving` is set (only one save runs at a time; a second call is **dropped**, not queued).
2. Clear all timers. Write the draft synchronously (`flushDraftWrite`) and remember its `draftRevision`.
3. If content equals the last saved content: clear the dirty flag and status, and return.
4. **Content-loss gate** (B.5). If it trips, no request is sent and the banner is shown. If the draft write in step 2 failed, it returns silently instead.
5. `PATCH /notes/<path>` `{content}` with `responseKind:"json"`. A non-OK response throws `t("errors.failed_to_save")`.
6. Store the response `revision` in `_knownBaseRevisions[path]`. If the file was renamed, deleted or switched while the request was in flight (path or `fileVersion` changed), ignore the response, except to reschedule if the current file is dirty.
7. Delete the draft **only if its revision equals the one taken in step 2** (`removeDraftIfRevision`). If a newer draft exists with a different base, re-anchor it to the new revision ("rebase").
8. `_lastSavedContent = content`, `_baseRevision = newRevision`, reset the content-loss state, clear the dirty flag, **clear the legacy backup** for the path, show "Saved", and clear it after 2 s.
9. If the path is `.fed` (config file), dispatch `autosave:config-saved` → `APP#reloadConfig` → status "Config applied" for 2 s.
10. If the editor changed during the request: set dirty, write a draft at once, and `scheduleAutoSave()`.
11. On error (same file): status "Error saving" (red). The draft stays. **There is no automatic retry**; the next edit or Ctrl+S tries again.

`fileVersion` increments on setFile, clearFile, rename and delete. It invalidates stale timers and stale responses.

### B.4 Save status indicator

Element: `<span class="workspace-save-status text-xs … hidden" role="status" data-autosave-target="saveStatus">` in the header (`_header.html.erb:12`). `showSaveStatus(text, isError)` (`AS:1270-1278`) sets the text and hides the span when the text is empty. On error it adds `text-red-500 dark:text-red-400`. Fallback: if `window.t` returned an unresolved `"status.xxx"` key (translations not loaded yet), the label is taken from `data-autosave-status-labels-value` (server-rendered `t('status')` JSON). If that is also missing, the span is hidden, so a raw key is never shown.

| State | Text (en) | i18n key | Error style | When |
|---|---|---|---|---|
| clean | *(hidden)* | — | — | after load, after content returns to saved, 2 s after "Saved" |
| dirty | Unsaved changes | `status.unsaved` | no | first change after a save; also after a draft is auto-restored on load |
| saved | Saved | `status.saved` | no | after a successful PATCH, for 2 s |
| save failed | Error saving | `status.error_saving` | red | PATCH failed (network or non-2xx) |
| draft storage failure | Unable to save local draft. Keep this page open and try again before leaving. | `status.draft_storage_error` | red | any localStorage write/read/remove failure, missing revision, blocked path |
| offline | Disconnected | `connection.disconnected` | red | connection lost; cleared on reconnect |
| load failure | Error loading note | `status.error_loading` | red | GET failed (not 404) |
| config | Config applied | `status.config_applied` | no | after `.fed` save + reload, for 2 s |

The `status.offline_backup_saved` ("Backed up locally") key exists but is **never used**.

### B.5 Content-loss protection ("large deletion")

Exact rule (`AS:769-775`):

```js
isLargeDeletion(baseline, content):
  if baseline not string || content not string || baseline.length === 0 → false
  lostChars   = baseline.length - content.length          // net length change, in UTF-16 code units
  lostPercent = lostChars / baseline.length
  return lostPercent > 0.2 && lostChars > 50               // strictly greater on both
```

- Baseline = `_lastSavedContent`, the last content known to be on the server (the load value or the last saved value). It is not the original file.
- Only the **net length** counts. Replacing 1000 chars with 1000 other chars does not trip it.
- Checked: in `saveNow` before the PATCH; when a draft is auto-restored on load (`AS:729`); when a recovery choice is applied (`AS:1145-1162`, `1260-1265`).
- **The result is a blocked save plus a non-modal banner**. There is no confirm dialog. While the warning is active, `scheduleAutoSave` only sets the dirty flag. Local draft writes **continue**, so the reduced content is still kept locally.
- Banner (amber, `shared/_warning_banner.html.erb`, rendered in `index.html.erb:51-57`, full-width under the header): warning-triangle icon + `status.content_loss_warning` "Large content deletion detected. Press Ctrl+Z to undo if accidental." + buttons:
  - **Undo** (`status.undo`) → `undoContentLoss()`: runs CodeMirror `undo(view)` once. If nothing was undone (for example a draft restored after reload has no history), the editor is reloaded with `_lastSavedContent`. Then it re-checks the content and writes the draft (`AS:1060-1081`).
  - **Save Anyway** (`status.save_anyway`) → hides the banner, sets an override bound to the **exact current editor content** (`overrideContent`), then `saveNow()`. Any later edit invalidates the override, and the check runs again (`AS:105-113`, `896`).
- Auto-dismiss: on every edit, `checkContentRestored` hides the banner once the content no longer meets the large-deletion rule. For example, Ctrl+Z hides it (`AS:761-767`).
- The state is scoped to `{path, baseRevision}`. A warning on file A never blocks file B, and it resets after a successful save (`AS:82-95`).

### B.6 localStorage record shapes, keys, and rules

```jsonc
// frankmd:draft:<encodeURIComponent(path)>
{ "schemaVersion": 1, "path": "dir/a.md", "content": "...",
  "baseRevision": "<sha256 hex of server content the draft was based on>",
  "draftRevision": "<crypto.randomUUID() | fallback base36 token>",
  "updatedAt": 1730000000000 }            // Date.now()

// frankmd:draft-conflict:<encodeURIComponent(path)>:<conflictId>
{ "schemaVersion": 1, "path": "<destination path>", "sourcePath": "<original draft path>",
  "conflictId": "<uuid>", "content": "...", "baseRevision": "...",
  "draftRevision": "...", "updatedAt": 1730000000000 }

// frankmd:backup:<raw path>        (legacy, offline only)
{ "content": "...", "timestamp": 1730000000000 }
```

Rules:
- Validation (`DS:17-33`): the draft must have `schemaVersion===1`, `path` equal to the key path, string content, non-empty `baseRevision` and `draftRevision`, and a finite `updatedAt`. **Malformed JSON or an invalid shape is silently deleted.** A numeric `schemaVersion` other than 1 returns an error and is **not** deleted, so it is kept for future versions (`DS:52-54`).
- `writeDraft` needs a non-empty `baseRevision`. **If the server gave no revision, no draft can be written, and switching away from a dirty file is blocked** (`AS:144-150`). If the stored draft already has the same content and base, it returns `unchanged` with no write and no new revision (`DS:72-74`).
- Draft deletion is **compare-and-delete** on `draftRevision` (`removeDraftIfRevision`), so a late response never deletes a newer draft.
- Draft writes use the latest known base revision for the path (`_knownBaseRevisions`), not the possibly stale snapshot base (`AS:528-530`).
- **There are no size limits, no age-based retention, no LRU, and no explicit quota handling.** A `setItem` exception (for example `QuotaExceededError`) becomes `{ok:false}`. The UI then shows `status.draft_storage_error` (red), marks the path as a persistence failure (which turns on the beforeunload guard), and blocks file switches.
- Cleanup: on a successful save (matching revision); when the content returns to saved; when the server content equals the draft on load; on note/folder delete, `removeDrafts(path,type)` removes drafts, legacy backups and conflicts for the path or subtree (`DS:575-642`). If that cleanup fails, a toast shows `status.draft_storage_error` for 5 s (`APP:3051-3054`). `removeAllDrafts` and `removeAllBackups` exist (the UI-level `clearAll` is not called from views).
- **Rename/move remap** (`DS:130-471`, `AS:210-287`). Not atomic, but done in a safe order: (1) check every source and destination; (2) **never overwrite** a different draft at the destination. On a collision, every moved draft is kept as a draft-conflict record at its destination and the recovery dialog opens for the active file; (3) copy, (4) read back to verify, and only then (5) delete the sources. On any failure the sources stay and the destination paths are added to `_draftBlockedPaths`: autosave and drafts are blocked for those paths until the recovery is resolved, the status shows the storage error, and the beforeunload guard turns on. The rename does **not** clear the dirty flag. A pending autosave moves to the new path. Folder matching is `path === p || path.startsWith(p + "/")`, so `docs` does not match `docs-old`.

### B.7 Recovery on load (`recoverDraft(serverContent, serverRevision)`, `AS:655-742`)

Called from `APP#showEditor` right after `setFile`. Its return value becomes the editor content.

```
1. retry any pending remap recovery for this path (copy source draft → conflict record)
2. conflicts = listDraftConflicts(path) sorted by updatedAt asc
   if any → open recovery dialog with the NEWEST conflict (source "draft-conflict"); editor = server
3. draft = readDraft(path); backup = legacy backup (deleted immediately if === serverContent)
4. no draft: backup? → dialog (source "backup"); editor = server
5. backup exists, differs from draft, and backup.timestamp > draft.updatedAt → dialog (source "backup"); editor = server
   otherwise delete the stale backup
6. draft.content === serverContent → delete draft; editor = server
7. draft.baseRevision === serverRevision      (file unchanged since draft)
      → SILENT auto-restore: editor = draft.content, status "Unsaved changes",
        content-loss check, autosave scheduled
8. else (file changed elsewhere)  → dialog (source "draft"); editor = server
```

When a dialog opens, all timers for the path stop and `_pendingRecovery={path,draftRevision}`. **While it is pending, autosave and draft writes for that path are skipped**, so the stored draft is not overwritten.

### B.8 Recovery dialog (`_recovery_diff.html.erb`, `RD`)

- Native `<dialog>` with `showModal()`, `w-[90vw] max-w-6xl max-h-[80vh]`, black/50 backdrop.
- Header: title `dialogs.recovery.title` "Offline Backup Recovery" (the same title is used for every source, including online drafts) + a timestamp pill showing `new Date(ts).toLocaleString()` (draft `updatedAt` / backup `timestamp`) + an X close button.
- Body: two columns, 50/50, each scrolls on its own, monospace, `whitespace-pre-wrap`:
  - Left: `dialogs.recovery.server_version` "Server Version" + hint `server_hint` "(current file)". Shows equal parts plus **deleted** parts (red tint `rgba(239,68,68,.2)`, `--theme-error` text, line-through).
  - Right: `backup_version` "Local Backup" + `backup_hint` "(saved offline)". Shows equal parts plus **inserted** parts (green tint `rgba(34,197,94,.2)`, `--theme-success`) (`ai-diff.css`).
- Footer buttons: **Use Server Version** (`use_server`, secondary) and **Restore Backup** (`restore_backup`, accent/primary).
- **Escape is blocked** (`cancel` → `preventDefault`) so the user has to choose. **The X button counts as "Use Server Version".**
- Results (`AS:1091-1266`):
  - *Use Server*: the editor already shows the server content. The selected draft is deleted (only if its revision still matches) and the selected backup is deleted (only if content and timestamp still match). The pending state is cleared and the file is marked clean. For conflict records the current primary draft is also deleted (first kept as a conflict copy if it is a different revision), the conflict and its source draft are deleted, the editor is set to the server content, and the path is unblocked.
  - *Restore Backup*: the editor gets the chosen content and is marked dirty. The content is written as a fresh draft against the current server revision, and the legacy backup is deleted only after that write succeeds. Then the content-loss check runs: if it trips, the banner shows and autosave stays paused; otherwise autosave is scheduled.
  - Guard: if the draft or conflict changed while the dialog was open (different `draftRevision`/`conflictId`), nothing is applied and the storage error status is shown.

### B.9 Diff algorithm (`lib/diff_utils.js`)

- Tokenize with `/(\s+|\S+)/g`, so runs of whitespace are tokens and are kept exactly.
- Classic **LCS dynamic-programming table** over tokens, O(m·n) time and memory, with no size cap.
- Backtrack from (m,n): if the tokens are equal → `equal`; else if `j>0 && (i===0 || dp[i][j-1] >= dp[i-1][j])` → `insert`; else → `delete`. Built with `unshift`.
- Neighbouring ops of the same type are merged into one string. Output: `[{type:"equal"|"insert"|"delete", value}]`. The values are HTML-escaped before rendering.
- The diff is word-level, not line-based. The same function is used for the AI grammar diff.

### B.10 Offline detection and behavior

- `connection-monitor` controller on the app root (`index.html.erb:2`). On connect it checks once, then **polls every 5000 ms** (`interval` value) with `fetch("/up", {method:"HEAD", cache:"no-store"})` and an **AbortController timeout of 3000 ms** (`CON:10-13, 52-82`). Checks never overlap.
- `window` `offline` event → marks offline at once. `online` event → runs a check, and only an OK response marks it online. Any non-OK response, network error or timeout → offline.
- **Transitions only** (the state is idempotent). Going offline shows the amber banner, disables the hidden `<textarea data-connection-monitor-target="textarea">`, and dispatches `connection-monitor:offline`. Coming back hides the banner, re-enables the textarea (only if it was not already disabled before), and dispatches `connection-monitor:online`.
- Banner: amber, warning icon with `animate-pulse`. Text `connection.offline_message` "Connection lost. Editing disabled until connection is restored." Button **Retry** (`connection.retry`) → check now.
- ⚠ **CodeMirror is NOT made read-only while offline.** Only the old hidden textarea is disabled, so the user can keep typing even though the banner says editing is disabled. A rewrite should either really block editing or change the message.
- `autosave#onConnectionLost` (`AS:1001-1030`): set `isOffline`, write the draft at once, cancel both save timers (keep the dirty flag), dispatch `autosave:offline-changed {offline:true}` (APP cancels any pending debounced `PATCH /config`), set status "Disconnected" (red), and write the legacy backup **at once** if the content differs from the last saved content.
- While offline: each edit writes a draft after 300 ms and a legacy backup after 1000 ms. `scheduleAutoSave` and `saveNow` do nothing except set the dirty flag. The status stays "Disconnected". File switching still works because it only writes drafts.
- `onConnectionRestored`: clear `isOffline`, clear the status, `saveNow()` if dirty. A successful save deletes the legacy backup.

### B.11 "Undo created note" (Ctrl+Z past the start of history)

- When `APP#onFileCreated` runs, the new note is loaded and `createdNoteBoundaries[path] = {path, initialContent, initialRevision, previousPath}` is recorded (markdown only; `APP:2930-2955`).
- The CMX keymap at `Prec.highest` sends `Mod-z` to `onUndoAtHistoryStart` **only when `undoDepth===0`** (`CMX:215-229`). It is disabled in Vim mode (`CM:99-102`).
- Condition: the current file is that markdown note, the boundary is not deleted, and the editor content equals `initialContent` exactly.
- Confirm dialog (`_undo_created_note.html.erb`): `w-80`. Title `confirm.undo_created_note_title` "Undo note creation?". Message `confirm.undo_created_note` "Undoing creation will delete "%{name}". You can redo it during this session." The name is the basename, shown in a `.confirm-dialog__filename` chip. Buttons: **Cancel** (autofocus, the default) and **Delete** (`common.delete`, error-coloured outline). Header close X = cancel. If `<dialog>` is unavailable it falls back to `window.confirm`.
- After confirm: check again that the content is unchanged (else toast `errors.failed_to_delete` 5 s). Lock the editor read-only. `autosave.prepareForFileDeletion` writes the draft, waits for any in-flight save, cancels follow-up timers, and returns the latest acknowledged revision (on failure, toast `status.draft_storage_error` 5 s). Then `DELETE /notes/<path>?expected_revision=<rev>` (turbo-stream). On failure, toast with the server `error` or `errors.failed_to_delete` (5 s), and autosave resumes. On success: mark the boundary `deleted`, run the normal file-deleted handling while keeping the CM history cache, then reopen `previousPath` (URL replaced) if the user has not navigated away.
- **Redo**: `Mod-y` / `Mod-Shift-z` (mac) / `Ctrl-Shift-z` with `redoDepth===0` on the previous note, or the same keys on the Home screen when no file is open (for a note created from Home) → `POST /notes {path, content: initialContent}` (create-only, never overwrites) → reopen the note. On failure, toast `errors.failed_to_create` / `errors.failed_to_load` (5 s).
- Toast = `showTemporaryMessage`: a fixed, centred pill at the bottom (`bottom-4`). Only one is shown at a time.

### B.12 PWA / service worker / splash

- `app/views/pwa/service-worker.js` contains **only commented-out** Web Push boilerplate (Rails default). `manifest.json.erb`: name/short_name "FrankMD", description "A simple, feature-rich markdown note-taking app", icons `/icon-192.png` (192), `/icon-512.png` (512, plus a maskable entry), `start_url "/"`, `display "standalone"`, `scope "/"`, `theme_color`/`background_color` `#FF6B9D`.
- **Neither file is routed** (no `pwa` route in `config/routes.rb`, no `<link rel=manifest>`, no `serviceWorker.register`). There is no offline caching. This is not functional, so the native app can ignore it.
- `public/splash.html` (and `config/fed/splash.html`, which uses `http://localhost:7591` absolute URLs) is a startup screen opened by the `fed` launcher script: a 135° gradient `#2D1B4E → #FF6B9D`, a 180 px logo, a 40 px spinner (1 s), "Starting" with animated dots (1.5 s), and it polls `GET /up` **every 200 ms** until it returns OK, then `location.replace('/')`. `/up` returns `"OK"` with `Access-Control-Allow-Origin: *` (`health_controller.rb`).

### B.13 Edge behaviors worth preserving

- One save at a time. Responses for a file that was switched, renamed or deleted in the meantime are ignored, but their revision is still recorded for that path.
- A draft is written synchronously **before** every server save, so a crash during the request loses nothing.
- Hard reload with unsaved edits: the draft (written on pagehide or visibility hidden) is restored silently with "Unsaved changes" when the file on disk is unchanged (`draft_recovery_test.rb:56-81`). Autosave then saves it.
- Switch A→B before the 2 s debounce: A's file on disk is **not** updated; A's draft stays and is restored when A is reopened (`draft_recovery_test.rb:6-54`).
- An empty-string draft is valid and can be restored.
- `.fed` (config) saves trigger a config reload.

---

## 5. Preview  
<sub>(sub-section labels below keep the source numbering, e.g. C.x)</sub>

Paths below are relative to `~/Projects/FrankMD/`. "PC" = `app/javascript/controllers/preview_controller.js`, "SSC" = `app/javascript/controllers/scroll_sync_controller.js`, "AC" = `app/javascript/controllers/app_controller.js`.

### 5.1 Layout, visibility and view modes

- **DOM** (`app/views/notes/_preview_panel.html.erb`): `<aside data-controller="preview">` is a flex column, `flex-none w-[40%]`, with a left border `var(--theme-border)` and background `var(--theme-bg-primary)`. It starts out `hidden`. Its inner content div is `relative flex-1 overflow-y-auto p-6 prose …`, has `tabindex="-1"`, and listens for `scroll` and `click`. The classes `prose-zinc dark:prose-invert prose-sm max-w-none` do nothing here, because no Tailwind typography plugin is loaded. All prose styling is custom (see 5.10).
- **Width:** the pane is a fixed 40% column in split mode. There is **no resize handle or splitter**, and the width is not persisted. In single-view mode (`body.single-view-mode`), the panel uses `flex:1 1 0%; width:100%; border-left:none` (`app/assets/tailwind/components/editor.css:123`).
- **Initial state:** the preview is hidden on every page load. Its visibility is **not persisted** (no config key for it).
- **Toggle:** use the workspace toolbar "preview" action (pinned by default, `app/helpers/workspace_helper.rb:8`, `aria-pressed` kept in sync by `AC#onPreviewToggled`) or the keyboard shortcut **Ctrl+Shift+V** (`lib/keyboard_shortcuts.js:16`). The tooltip text in `config/locales/en.yml:73` says "Ctrl+Shift+P", which is stale and wrong.
  - It only works for `.md` files. Other files show the temporary message "Preview is only available for markdown files" for 2 s (AC:1242-1258).
  - It does nothing while the Library workspace is open.
  - **Split mode:** it toggles the panel (`hidden`↔`flex`) and the `body.preview-visible` class, then dispatches `preview:toggled {visible}`.
  - **Single mode:** it swaps panes. Editor → full-width preview hides the editor panel. Preview → editor hides the preview and focuses CodeMirror (AC:1264-1284). Switching to single mode always shows the editor pane first (AC:2126-2130).
- **Automatic hide/show:**
  - Opening a non-markdown file hides the preview (AC:1085-1089).
  - Opening the Library stashes the preview state and hides it. Closing the Library restores it, in split mode only (AC:1940-2020). Home view works the same way (AC:511-561).
  - Turning on typewriter mode in split view **force-shows** the preview (AC:2180-2184).
- **On show** (`PC#show`): the render cache (`_lastRenderedContent`) and `lastScrollTarget` are cleared. Then `SSC#onPreviewToggled` re-renders and re-syncs the preview to the editor (5.6).
- **Zoom controls** in the workspace toolbar (`_workspace_toolbar.html.erb:3-10`) are visible only while the preview is visible for a markdown file (AC:1788-1789).

### 5.2 Render pipeline

1. **Trigger:** every CodeMirror doc change on a markdown file with the preview visible calls `SSC#updatePreviewWithSync` (AC:1143-1149).
   - **Normal mode:** `PC#render(content)` runs **synchronously on every change. There is no debounce.**
   - **Typewriter mode:** `PC#updateWithSync` debounces the render by **150 ms** (`setTimeout` reset on each keystroke). It skips the DOM rebuild if the content equals `_lastRenderedContent`, but still re-syncs scroll if the cursor line or total line count changed (PC:640-690).
   - File load, show, and settings changes call `SSC#updatePreview` → `render`.
2. **`render(md)`** (PC:225-264), which does nothing unless the preview is visible:
   1. Set `_isUpdatingContent = true`. It is cleared **100 ms** after the last render, and while set it blocks preview→editor scroll sync.
   2. Strip front matter (5.3) and record `frontmatterLines`.
   3. Run `parseWithLineNumbers(content, frontmatterLines)`: marked lexer → `marked.parse` → inject `data-source-line` attributes → DOMPurify. The result is assigned to `innerHTML`.
   4. Start async syntax highlighting of code blocks in place, with errors swallowed (5.7).
   5. Run KaTeX over the math placeholders (5.4.2).
   6. Add copy buttons to `<pre>` blocks (5.7).
   7. Add image-viewer triggers (5.8).
   8. Set `totalSourceLines` to the line count of the *unstripped* markdown.
- **Marked:** `marked@15.0.6`, vendored from esm.sh (`config/importmap.rb`, `vendor/javascript/marked.js:1`). It is configured once in AC:222-226 with `marked.use({ breaks: true, gfm: true, extensions: allExtensions })`. Everything else uses marked defaults:
  - `pedantic:false`
  - no custom renderer
  - **no heading IDs/anchors**: marked 15 does not emit `id` on headings, and nothing adds them
  - raw HTML passes through, then gets sanitized
- **`breaks:true`:** a single newline inside a paragraph becomes `<br>`.
- **GFM** covers tables (with alignment), strikethrough `~~x~~`, autolinked URLs, and task lists.
- **Not supported:** footnotes, mermaid, definition lists, admonitions/callouts, heading anchors, Hugo shortcodes, and print/export from the preview.
  - Hugo shortcodes such as `{{< youtube id="…" >}}`, which the video dialog can emit (`video_dialog_controller.js:66-71`), render as **literal text**.
  - The only print rules are in `image-viewer.css:14-17`: they hide the viewer and the enlarge buttons and unwrap image triggers (`display:contents`).

### 5.3 Front matter

`stripFrontmatter` (PC:17-58):

- **YAML:** the content must *start* with `---`. The closing fence is found with `indexOf("\n---", 3)`, which matches any line that *begins* with `---` (for example, `----` also closes it). Everything up to and including the newline after that line is removed. The remainder is then **`trimStart()`ed**, so leading blank lines vanish.
- **TOML:** the same rules with `+++`.
- **No closing fence:** nothing is stripped.
- **Closing fence is the last line with no trailing newline:** the whole document becomes empty.
- **Front matter is hidden entirely.** It is not rendered as a table or code.
- `frontmatterLines` = `frontmatter.split("\n").length`. This counts one extra line for the trailing empty segment. It is passed as `lineOffset` so the `data-source-line` values are absolute editor lines.
  - Caveat: blank lines removed by `trimStart` are not counted, so line anchors shift by that many lines.

### 5.4 Marked extensions (`app/javascript/lib/marked_extensions.js`)

The order is `superscript, subscript, highlight, emoji, icon, wikilink, mathBlock, mathInline` (line 230). All of them are inline except `mathBlock`. Their `escapeHtml` escapes `& " < >`.

| Syntax | Regex (anchored at the token start) | Output |
|---|---|---|
| `^text^` | `/^\^([^\^]+)\^/` | `<sup>text</sup>`. Text is escaped and inner markdown is **not** parsed. |
| `~text~` | `/^~([^~]+)~(?!~)/` | `<sub>`. Does not conflict with `~~strike~~`. |
| `==text==` | `/^==([^=]+)==/` | `<mark>` |
| `:shortcode:` | `/^:([a-z0-9_+-]+):/` + lookup in `EMOJI_DATA` (`lib/emoji_data.js`, about 675 entries such as `grinning`→😀) | The emoji character. Unknown codes stay literal. |
| `:ph-name:` | `/^:ph-([a-z0-9-]+):/` + lookup in `ICON_DATA` (`lib/icon_data.js`, about 200 curated Phosphor "regular" icons) | `<svg xmlns viewBox=… fill="currentColor" width="1em" height="1em" class="frankmd-inline-icon" aria-hidden="true"><path d=…/></svg>`, aligned with `display:inline-block; vertical-align:-0.125em` (`prose.css:397`). Note: the emoji extension tries first, and fails because the `ph-` name is not in the emoji map. |
| `[[Target]]`, `[[Target\|Label]]`, `[[folder/Target]]` | `/^\[\[([^\]\|]+)(?:\|([^\]]+))?\]\]/` (target and label are trimmed) | `<a class="wikilink" data-wikilink-path="Target" data-action="click->app#openWikilink">Label-or-basename</a>`. There is **no href**. Without a label, the display text is `target.split("/").pop()`. |
| `$$…$$` (block) | `/^\$\$([\s\S]+?)\$\$(?:\n+\|$)/`. Must open at the block start. Content may span lines and must be non-empty after trim. | `<div class="math-block">escaped TeX</div>` |
| `$…$` (inline) | `start`: the first unescaped `$`. Tokenizer: `/^\$(?!\$)(?!\s)((?:\\.\|[^\\$])+?)(?<!\s)\$(?!\d)/`. The opening `$` must not be followed by whitespace, the closing `$` must not be preceded by whitespace or followed by a digit, and `\$` stays literal. So `$5 and $10` is **not** math. | `<span class="math-inline">escaped TeX</span>` |

#### 5.4.1 Wikilink click (AC:2347-2378)

1. `preventDefault`.
2. Resolve the target:
   1. If it does not end in `.md`, add `.md`.
   2. Look for an exact match on the tree path.
   3. Otherwise, look for a case-insensitive match on the basename, using the first file in the tree whose `name` equals `target.split("/").pop().toLowerCase()`. Note this compares against the name *without* `.md`, so it only hits if the tree `name` has no extension. The tree is outside this section's scope, so this is unverified.
   4. Otherwise, fall back to `pathWithExt`, and `loadFile` handles the missing file.
3. `openFileAndRevealInTree(path)`.

Wikilinks are styled with the accent color, no underline, and a `1px dashed` accent bottom border that turns solid on hover, plus `cursor:pointer` (`prose.css:82-92`).

#### 5.4.2 KaTeX math (`lib/math_renderer.js`)

- `katex@0.19.0`, self-hosted. CSS is in `app/assets/stylesheets/katex.css`, which is linked in the layout. Fonts are in `/fonts/katex/`.
- **Placeholders survive DOMPurify** (text + class only). KaTeX renders them *after* sanitization, so KaTeX's inline styles never go through the sanitizer.
- **Options:** `{throwOnError:false, trust:false, strict:"ignore", maxExpand:1000, output:"html"}`. `displayMode` is true for `.math-block`.
- Each node is marked `data-math-rendered` so it is never rendered twice. An unexpected exception adds `.math-error`.
- **Styles** (`math.css`):
  - `.math-block` has `overflow-x:auto; overflow-y:hidden; margin:.75rem 0`.
  - `.math-error`, `.katex-error` use `var(--theme-error,#dc2626)`.
  - Math inherits the text color.
- The only math delimiters are `$…$` and `$$…$$`. `\(…\)` and `\[…\]` are not supported.

### 5.5 Sanitization (`lib/html_sanitizer.js`)

- `dompurify@3.4.17`, running in an isolated instance (`DOMPurify(window)`). It runs **after** line annotation (`markdown_line_mapper.js:126-131`).
- **Config:**
  - `ADD_TAGS:["iframe"]`
  - `ADD_ATTR:["allow","allowfullscreen","frameborder","referrerpolicy"]`
  - `FORBID_TAGS:["form","button","textarea","select","option","style"]`
  - `FORBID_ATTR:["style"]`, so **inline styles are stripped from note HTML**
- **The DOMPurify defaults matter here:**
  - `data-*` attributes are kept. This is why `data-source-line`, `data-wikilink-path`, and `data-action` survive.
  - `class` is kept.
  - `<script>` and `on*` handlers are removed.
  - **`target` is not in the default allow-list, so `target="_blank"` is stripped.**
  - `<video>`, `<source controls>`, and `<picture>` are allowed by default.
- **`uponSanitizeElement` hook:**
  - `<input>`: kept only if `type="checkbox"`, and stripped down to the `type` and `checked` attributes. `disabled` is removed so clicks reach the handler. Any other input type is removed.
  - `<iframe>`: kept only when `src` is `https:` and the host is one of `www.youtube.com, youtube.com, www.youtube-nocookie.com, youtube-nocookie.com, player.vimeo.com`. Other iframes are removed. `srcdoc` is never allowed.
- **Video embeds:**
  - The video dialog emits `<div class="embed-container"><iframe src="https://www.youtube.com/embed/ID" …allowfullscreen></iframe></div>` (`video_dialog_controller.js:74-83`). It is styled as a responsive 16:9 box using `padding-bottom:56.25%`, with an absolutely positioned 100%×100% iframe, `border-radius:.5rem`, and a tertiary background (`prose.css:363-382`).
  - Local videos use `<video controls class="video-player"><source src type>` and are styled full width with `height:auto` and rounded corners (`prose.css:384-391`).

### 5.6 Task-list checkboxes

Task-list checkboxes are interactive (PC:166-187, AC:2575-2584, `lib/task_utils.js`).

- GFM renders `<li><input type="checkbox" [checked]> text`. Every `<li>` gets its own `data-source-line` (5.6.1).
- **Click on a checkbox inside `li[data-source-line]`:**
  1. `preventDefault()`, so the visual toggle is reverted.
  2. Dispatch `preview:toggle-task {line}`.
  3. `codemirror.toggleTaskAtLine(line)` replaces just the 3-character marker `[ ]`↔`[x]`. The marker regex is `/^(\s*(?:[-*+]|\d{1,9}[.)])\s+)\[([ xX])\](\s|$)/`.
  4. The preview updates through the normal re-render. It is a no-op if the line is not a task or the file is not markdown.
  - This makes the click part of editor undo history. It is a real doc change and triggers autosave.
- **Styles:**
  - Checkbox: `margin-right:.5em; accent-color:var(--theme-accent); cursor:pointer`.
  - Hover/focus ring: `box-shadow:0 0 0 2px bg-primary, 0 0 0 4px accent`.
  - `ul:has(input[type=checkbox])` gets `list-style:none; padding-left:0` (`prose.css:322-341`).

#### 5.6.1 Source-line mapping (`lib/markdown_line_mapper.js`)

- **Blocks:**
  1. Lex with `marked.lexer`.
  2. For each top-level token except `space`, find `token.raw` with `indexOf` from a moving cursor, count the newlines before it, and compute `line = currentLine + lineOffset + 1` (1-based).
  3. Run a regex over the HTML for `<(h1-6|p|ul|ol|blockquote|pre|hr|table|div)(\s|>)` and assign the token lines **sequentially in document order**. Nested `<p>` and `<div>` tags also consume slots, so the mapping is approximate when there are blockquotes, loose lists, or HTML blocks.
- **List items:** item positions are collected depth-first (an item comes before its nested items), converted to lines, and assigned sequentially to each `<li` in order.
- **Recommendation for the Rust rewrite:** use real source positions from the parser (for example, comrak `sourcepos`) instead.
- `findElementByLine(container, n)`: scans `[data-source-line]` in DOM order and returns the element with the smallest `|line-n|`, stopping at the first element with `line ≥ n`.
- `findLineAtScroll(container, scrollTop)`: computes each annotated element's top as `rect.top - containerRect.top + scrollTop` and returns the line of the element with the greatest top that is still ≤ `scrollTop`. If none qualifies, it returns the first element's line. It returns null when no elements are annotated.

### 5.7 Code blocks in preview

- **Highlighting** (`lib/code_block_highlighter.js`): Lezer parsers plus CodeMirror legacy stream modes, run with `highlightTree` and a fixed `tagHighlighter` that emits `tok-*` classes.
- **Language selection:** the language comes from the first `language-xxx` class on `pre > code[class]`, lowercased, and looked up in an explicit map (lines 140-212). Unknown languages are left plain.
  - **Static (always loaded):** js, javascript, mjs, cjs, jsx, ts, typescript, tsx, html, htm, svg, css, json, jsonc.
  - **Lazy-loaded with a cached dynamic `import()`:** yaml, yml, .yaml, .yml, php, phtml, ruby, rb, .rb, rake, env, .env, dotenv, shell, bash, sh, zsh, sql, postgres, postgresql, pgsql, mysql, mariadb, sqlite, toml, tml, dockerfile, docker, tf, .tf, tfvars, .tfvars, terraform, hcl, .hcl, rs, rust, go, golang, py, python, python3, java, c, h, ino, cc, cpp, c++, cxx, hpp, hh, hxx, cs, csharp, c#, kotlin, kt, kts.
  - Display labels include "JavaScript", "TypeScript", "TSX", "HTML", "SVG", "CSS", "JSON", "JSONC", "YAML", "PHP", "Ruby", "Dotenv", "Shell", "SQL", "PostgreSQL", "MySQL", "MariaDB", "SQLite", "TOML", "Dockerfile", "Terraform", "HCL", "Rust", "Go", "Python", "Java", "C", "C++", "C#", "Kotlin".
- **Limits:**
  - At most 100 blocks are inspected per render.
  - Blocks longer than 20,000 characters are not highlighted.
  - Highlighting stops after 50,000 characters in total per render.
  - Blocks over the limits still get a language label.
- **Highlighting order:** common languages are highlighted first, then lazy parsers are awaited, then the remaining blocks are highlighted. Blocks that a newer render has replaced (`!root.contains(code)`) are skipped. Failed loads leave plain text.
- **YAML:** plain scalars get their own coloring: `true/false/null/~` → `tok-constant`, numeric scalars → `tok-number`, everything else → `tok-string`.
- **Language label:** `pre[data-code-language]` gets `padding-top:2.25em` and a `::before` badge showing the label: absolutely positioned at top .55rem / left 1rem, muted color, .72em, weight 600, letter-spacing .06em, uppercase.
- **Token colors** come from theme variables with fallback to `--theme-text-primary` (`prose.css:173-245`):
  - `--theme-syntax-{keyword, function, operator, string, number, constant, comment, type, property, identifier, punctuation, invalid}`
  - Keywords and definitions are bold (600), comments are italic, and invalid tokens get a wavy underline.
- **Copy button** (PC:557-584):
  - Every `<pre>` with a `<code>` gets `<button class="code-copy-btn" title="Copy code">`. The title is hard-coded English, not i18n.
  - The icon is a 16px clipboard SVG. Clicking calls `navigator.clipboard.writeText(code.textContent)`, swaps in a check icon, and adds `.copied` (color `#22c55e`, always visible) for **2000 ms**.
  - Position: absolute at top/right .5rem, padding .35rem, transparent, muted color.
  - It is `opacity:0` until the `pre` is hovered (transition .15s). On hover it gets a tertiary background and primary color (`prose.css:248-275`).
  - Clipboard failures are not handled.

### 5.8 Images and image viewer (lightbox)

- **Image URL resolution:** there is **no rewriting** of image URLs. `src` is used exactly as written, and the browser resolves it against the page URL.
  - The page URL is always `/notes/<encoded note path>` (AC:447-458, `pushState`).
  - Images the app inserts use **paths relative to the note's folder**, for example `../images/foo.png` (`relativeMediaPath`, AC:39-58, 1421-1430). These resolve to `/notes/images/foo.png`, which `NotesController#show` serves as a raw file for any non-`.md` path that contains a `.` (`notes_controller.rb:30-33,164-184`, `send_file … disposition: inline`).
  - Absolute `http(s)` URLs (S3, the web) load directly.
  - **For the Rust rewrite:** resolve `src` relative to the note's directory inside the vault, then fall back to vault-absolute and remote URLs.
- **Triggers** (PC:268-291, run after every render, for each `img` that has `src` or `srcset`):
  - **Unlinked image:** the `<img>` (or its enclosing `<picture>`) is wrapped in `<button type="button" class="preview-image-trigger" aria-label="Enlarge {alt|'image'}">` (i18n key `preview.image_viewer.open`). Style: block, `cursor:zoom-in`, transparent, no border, `margin:1rem 0`. The inner img margin is 0.
  - **Image inside `<a>`:** the link keeps its normal navigation. An extra sibling button `<button class="preview-image-enlarge">↗</button>` is inserted right after the `<a>` (2rem square, bordered, secondary background, `margin-inline-start:.375rem`).
  - Both get `:focus-visible` with a 2px accent outline, offset 3px.
- **Opening:**
  - Plain primary click or Enter/Space on a trigger opens the viewer. Ctrl, Meta, Shift, or Alt clicks and non-primary buttons are ignored.
  - The click dispatches the window event `preview:open-image {src: currentSrc||src, alt, trigger, fallback: contentTarget}`.
- **Viewer** (`dialogs/_image_viewer.html.erb`, `image_viewer_controller.js`, `image-viewer.css`):
  - A native modal `<dialog class="image-viewer">` with a visually hidden title "Image preview", a close button (X icon, `aria-label` "Close", focused on open), a stage, and a status line (`role=status`).
  - **URL check:** only `http:`, `https:`, `blob:`, and `data:image/*` URLs are accepted. Anything else is silently ignored.
  - **Loading:**
    1. A new `<img class="image-viewer-image" alt>` is created hidden, and the status shows "Loading image…".
    2. On load, the image is shown and the status cleared.
    3. On error, the status shows "Unable to load this image."
    4. Results from stale images (a newer open replaced them) are ignored.
  - **Close:** the X button, **Escape** (native dialog behavior), or a click on the backdrop (click target is the dialog itself). On close, the stage and status are cleared and **focus returns to the trigger** if it is still in the DOM, otherwise to the preview content.
  - **Sizing:**
    - Dialog: `width:min(100vw-2rem, 80rem)`, `max-height:100dvh-2rem`, centered, bordered, `radius .75rem`, scrolls when the image overflows.
    - Backdrop: `color-mix(bg-primary 85%, transparent)`.
    - Frame padding `3.5rem 1rem 1rem`.
    - Image `max-width:100%; max-height:calc(100dvh - 8rem); object-fit:contain`.
  - **There is no zoom, pan, gallery, next/prev, or download.** It is a fit-to-screen view only.
  - It fits a 390×844 mobile viewport (`test/system/preview_image_viewer_test.rb`).
- `.prose img`: `border-radius:.5rem; margin:1.5em 0; max-width:100%`.

### 5.9 Links

- There is **no custom link handler**. `target` is stripped by DOMPurify, so every `<a href>` navigates **the same tab**.
  - Same-origin links such as `/notes/target.md` or relative `other.md` are intercepted by **Turbo Drive**, which loads that note in the app. The system test shows that clicking `[![](…)](/notes/target.md)` opens the "Target" note.
  - External links leave the app in the same tab.
- **For the Rust rewrite:** decide on a policy. The recommendation is: external links open in the system browser, and `.md` relative links open the note inside the app.
- Link style: accent color, underline with `text-underline-offset:2px`, `accent-hover` on hover (`prose.css:72-80`).
- Clicking preview text does **not** move the editor cursor. There is no click-to-source.

### 5.10 Synchronized scrolling

The scroll lock is owned by SSC. There is a single `_scrollSource` value (`"editor" | "preview" | null`) that auto-clears **400 ms** after the last mark (SSC:194-212). The setting is `scroll_sync` (default true), toggled from Settings → Editor "Scroll sync" or **Ctrl+Shift+\\**, and persisted (AC:3316-3329). When it is false, every sync entry point returns early (`syncScrollEnabledValue`).

#### 5.10.1 Editor → preview (user scrolls the editor)

1. CodeMirror's `scroll` listener dispatches `codemirror:scroll {scrollRatio}`. This is suppressed while a mouse selection drag is in progress, and there is no throttle (`codemirror_controller.js:252-264`).
2. `SSC#onEditorScroll`: if the preview is hidden or `_scrollSource==="preview"`, stop. Otherwise mark the source `"editor"`.
   - **Typewriter mode:** call `syncToTypewriter(cursorLine,totalLines)`.
   - **Otherwise:** call `syncScrollRatio(ratio, topVisibleLine)`, where `topVisibleLine` is the document line at `lineBlockAtHeight(scrollTop+1)`.
3. `PC#syncScrollRatio` runs on one `requestAnimationFrame`, and each new call cancels the pending one (PC:384-447):
   1. If the preview is not scrollable, stop.
   2. If `ratio ≤ 0.01`, set `scrollTop=0`.
   3. If `ratio ≥ 0.99`, scroll to the bottom (only if more than 1px away).
   4. Otherwise, take `effectiveLine` = `sourceLine`, or `round(ratio*totalSourceLines)+1`. Find `findElementByLine`, take its container-relative top, and clamp to `[0,max]`. Set `scrollTop` instantly, and only if the change is more than **5 px**.
   5. If no element is found, fall back to `ratio*max`.

#### 5.10.2 Preview → editor (user scrolls the preview)

1. `PC#onPreviewScroll` returns early in three cases: sync is disabled, the preview is hidden, or `_isUpdatingContent` is true (within 100 ms of a render). Otherwise it dispatches `preview:scroll {scrollRatio, sourceLine: findLineAtScroll, totalLines, typewriterMode}`.
2. `SSC#onPreviewScroll` returns early if a mouse selection drag is in progress in the editor or `_scrollSource==="editor"`. Otherwise it marks the source `"preview"`, and then:
   - `ratio ≤ 0.01` → editor scrolls to 0.
   - `ratio ≥ 0.99` → editor scrolls to max.
   - Otherwise, if `sourceLine` is set and `totalLines > 1`, the editor's `scrollToLine(sourceLine)` puts that line's block top at the editor's top, instantly.
   - Otherwise the editor scrolls to `ratio*max` if more than 5 px away.

#### 5.10.3 Programmatic preview scrolls

The typing, cursor, toggle, and typewriter paths dispatch `preview:programmatic-scroll`. SSC then marks `"editor"`, so the echo scroll events are not sent back to the editor.

#### 5.10.4 Typing path (normal mode)

`render()` alone does not scroll. Scroll position stays wherever it was, and editor scroll events keep it in sync.

`syncToLineSmooth(line,total)` is used by `update()`/`updateWithSync`:

1. It waits a double rAF.
2. It locates the element with `findElementByLine`. If there are no annotations, it falls back to picking among block elements (`h1-6,p,ul,ol,blockquote,pre,hr,table,img,iframe,.video-embed`) by the ratio `(line-1)/(total-1)`. If there are none of those either, it falls back to a pure ratio.
3. It **smooth-scrolls** (`behavior:"smooth"`) so the element sits **50 px below the top**.
4. It skips the scroll if the target is within **10 px** of `lastScrollTarget`.

`syncToLine` (cursor jump via `syncToCursor`) smooth-scrolls to a pure line ratio.

#### 5.10.5 Typewriter mode

- The editor keeps the cursor centered. That is CodeMirror's typewriter extension and is outside this section.
- The preview gets the class `preview-typewriter-mode`: `width:100%; max-width:calc(var(--editor-width,72ch) + 3rem); margin-inline:auto`. This matches the editor's reading column (`editor.css:17-21`).
- `syncToTypewriter(cursorLine,total)` runs on a double rAF. It aligns the cursor line's element **to the top of the preview** (offset 0). The code says "top", not centered.
  - Fallback without anchors: `paddingTop + ratio*(scrollHeight - paddings)`.
  - The result is clamped and smooth-scrolled, with the 10 px jitter threshold.
- It runs on every selection change (AC:1167-1171), on editor scroll, on toggle, and after each debounced (150 ms) render, but only when the line changed.
- Toggling typewriter mode resets `lastScrollTarget`.

#### 5.10.6 Re-anchoring

- A `ResizeObserver` on the preview content (fires on image loads and embeds), preview zoom changes, and window `resize` (debounced **200 ms**) all call `_reanchorAfterResize` (SSC:216-265).
- **Typewriter mode, and the preview was not the last driver:** run the typewriter sync.
- **The preview drove last:** scroll the editor to the preview's top source line, falling back to a ratio.
- **Otherwise:** run `syncScrollRatio` from the editor's ratio and top line.

#### 5.10.7 On show

After a fresh render, SSC marks the source `"editor"`, syncs the preview from the editor (typewriter mode, or ratio + top line), and clears the lock after a double rAF so the user can scroll the preview right away.

### 5.11 Zoom

- **Levels:** `[50,75,90,100,110,125,150,175,200]`. The default is 100 and the initial value comes from the `preview_zoom` config key (`_preview_panel.html.erb:2`).
- **Controls:** the toolbar "Zoom Out" / "Zoom In" magnifier buttons, with a "NNN%" readout (`previewZoomLevel`) between them. They step one level and stop at the ends.
  - **No keyboard shortcuts** for zoom, and no Ctrl+wheel.
- **Settings → Preview → "Preview zoom":** a `<select>` with the same nine levels (`_settings_panel.html.erb:416-433`). Values not in the list display as 100.
- **Mechanics:** `contentTarget.style.fontSize = "<zoom>%"`. Every prose size is in `em`, so everything scales.
  - Quirk: the inline style overrides `.prose{font-size:.9375rem}`, so 100% equals the inherited root size (probably 16px), not 15px.
- **Persistence:** every change dispatches `preview:zoom-changed` → `saveConfig({preview_zoom})`, which persists to the `.fed` config, and also re-anchors scroll sync.

### 5.12 Prose typography (`app/assets/tailwind/components/prose.css`)

- **Base:** color `--theme-text-secondary`, `line-height:1.75`, `font-size:.9375rem` (overridden by zoom), `padding:1.5rem` (p-6).
  - First child `margin-top:0`, last child `margin-bottom:0`.
- **Headings:** color `--theme-text-primary`, weight 600, line-height 1.3, margin `2em 0 .75em`.

  | Heading | Size | Other styles |
  |---|---|---|
  | h1 | 2em | `margin-top:0`, `padding-bottom:.3em`, bottom border `--theme-border`, color `--theme-heading-1` |
  | h2 | 1.5em | `padding-bottom:.25em`, bottom border, color `--theme-heading-2` |
  | h3 | 1.25em | color `--theme-heading-3` |
  | h4 | 1.1em | |
  | h5 | 1em | |
  | h6 | 1em | color `--theme-text-muted` |

- **Paragraphs:** margin `1.25em 0`.
- **Lists:** margin `1.25em 0`, `padding-left:1.5em`, li margin `.5em 0`. `ul` uses disc, `ol` uses decimal. Nested `ul` uses circle with `.5em` margins.
- **Inline code:** background `--theme-code-bg`, primary color, `padding:.2em .4em`, `radius .25rem`, `.875em`, weight 500. No backtick pseudo-content.
- **`pre`:** code background, `radius .5rem`, `overflow-x:auto`, `padding 1em`, margin `1.5em 0`, `.875em`, `line-height 1.7`. Inner `code` is transparent with normal weight.
- **Blockquote:** `4px` accent left border, `padding-left 1em`, margin `1.5em 0`, muted color, *italic*. Inner `p` margin `.5em`.
- **`hr`:** `1px` top border `--theme-border`, margin `2em 0`.
- **Tables:** width 100%, collapsed borders, `.875em`. `th` and `td` have a 1px border, `padding .625em 1em`, left-aligned (the GFM `align` attribute may still apply). `th` uses `--theme-bg-tertiary` and weight 600.
- **`strong`:** 600, primary color. **`em`:** italic. **`mark`, `sub`, `sup`:** browser defaults (no custom CSS).

---

## 6–7. Explorer, quick file finder, content search, Home  
<sub>(sub-section labels below keep the source numbering, e.g. F.x)</sub>

Source root: `~/Projects/FrankMD`. Paths below are relative to it. "app_controller" = `app/javascript/controllers/app_controller.js`.

> **Dead code warning:** `app/views/notes/_context_menu.html.erb` and `app/views/notes/dialogs/_rename.html.erb` are **not rendered anywhere** (no `render` call references them). The live context menu and rename dialog are the ones inside `app/views/notes/dialogs/_file_operations.html.erb`. Ignore the dead partials when reimplementing.

---

### F.1 Sidebar layout (`app/views/notes/_sidebar.html.erb`)

`<aside id="workspace-sidebar" class="workspace-sidebar">`, fixed width **16rem**, bg `--theme-bg-secondary`, right border `color-mix(--theme-border 50%, transparent)` (`app/assets/tailwind/components/editor.css:267-275`). Top to bottom:

1. **Header** (`.workspace-sidebar-header`, min-height 3rem, bottom border):
   - Left: **Close Explorer** toggle button (sidebar-panel-left icon: rect + vertical line at x=15 + `m10 15-3-3 3-3` chevron). `aria-expanded` mirrors visibility. Tooltip "Close Explorer".
   - Right (`_explorer_actions.html.erb`), 4 icon buttons, each 32×32 px, 8px padding, 16×16 icon (asserted by `workspace_test.rb:197`):
     1. **New Note** — tooltip "New Note (Ctrl+N)" → `app#newNote`.
     2. **New Folder** — tooltip "New Folder" → `app#newFolder`.
     3. **Find File** — magnifier, tooltip "Find File (Ctrl+P)" → `app#openFileFinder`.
     4. **Search in Files** — tooltip "Search in Files (Ctrl+Shift+F)" → `app#openContentSearch`.
2. **Explorer panel** (`#workspace-explorer`, flex column, flex 1):
   - **File tree** `#file-tree-content` (`overflow-y:auto; padding 0.5rem`, flex-1). Hosts the drag-drop controller and the background click handler.
   - **Document stats panel** (words / characters / size / read time / line N/M) — hidden by default, border-top; owned by stats-panel controller (other section).
3. **Footer** (`.workspace-sidebar-footer`, border-top): brand (`/icon.svg` 20×20 + "FrankMD", `cursor:default; user-select:none`, not a button) on the left; navigation on the right: **Home** button (house icon, aria-label/title "Home" → `app#showHome`), Library button, Settings button (other sections).

#### Visibility
- Desktop (≥768px): starts visible. Mobile (<768px): starts hidden. **Session-only, not persisted** (app_controller:178-181).
- When hidden: `.workspace-sidebar.is-hidden {display:none}`, sidebar `inert`; a floating **Open Explorer** button (`[data-app-target=sidebarOpen]`, absolute top/left .5rem, z 45) appears at the top-left of main; header gets 3rem left padding.
- Toggle: header button, the floating opener, or **Ctrl+E** (`toggleSidebar`). Focus moves: hiding while focus was inside → focus opener; showing via opener → focus close toggle; on mobile showing → focus first `[tabindex=0]`/button in explorer (app_controller:1621-1633).
- Mobile overlay: sidebar `is-overlay` (absolute, `width:min(19rem,100vw)`, z 30), backdrop button (25% text color mix) closes it; main gets `inert`. Tab is trapped inside the sidebar (wraps first↔last) (app_controller:1913-1921). Escape closes it (when no dialog open). Opening a file on mobile auto-closes the sidebar and focuses the editor (app_controller:503-506). Home button on mobile closes the sidebar.
- Crossing the 768px breakpoint on resize resets visibility to the breakpoint default (visible on desktop, hidden on mobile) (app_controller:197-205).

---

### F.2 Tree data & rendering

#### Server tree building (`app/services/notes_service.rb:437-490`, `build_tree`)
- Walks NOTES_PATH recursively.
- **Skips every entry whose basename starts with `.`** (hidden files and folders — includes `.fed`, `.hugo_template.md`, temp files, lock files).
- **Skips symlinks** (lstat).
- **Only `.md` files are shown**; all other files (images, txt, pdf…) are omitted. Folders are always shown (even empty ones).
- Special rule: **root-level** folders named exactly `images` or `videos` are hidden **if they contain no `.md` anywhere beneath** (managed media storage shown in Library instead). Same-named folders deeper are normal.
- **Sorting (per directory)**: folders first, sorted by **case-insensitive basename ascending**; then files sorted by **mtime descending (newest first)**, integer seconds. Sort key: `dir ? [0, name.downcase] : [1, -mtime]` (notes_service.rb:449). Ties in file mtime: Ruby `sort_by` (unstable/unspecified).
- Node shape: folder `{name, path, type:"folder", children}`; file `{name: basename-without-.md, path (relative, with .md), type:"file", file_type:"markdown", mtime: int}`.
- Tree is server-rendered HTML (`TreeHelper#render_tree_items`); collapsed folders' children are **still rendered** but in a `.tree-children.hidden` container.
- Empty root → single muted text "No notes yet" (`text-sm`, muted, p-2).

#### Row markup (`app/helpers/tree_helper.rb`)
Folder:
```
.tree-folder[data-path]
  .tree-item.drop-target[draggable=true][tabindex=0][data-path][data-drop-id=path][data-type=folder]
     chevron svg (.tree-chevron, rotated 90° when .expanded) + folder icon + <span.truncate>name</span>
  .tree-children(.hidden if collapsed)   ← padding-left 1rem per level
```
File: `.tree-item(.selected if path==selected_file)[draggable=true][tabindex=0][data-path][data-type=file][data-file-type][data-mtime]` → file icon + name span (name without `.md`). Files have **no chevron and no spacer**, so file icons sit 1rem+0.125rem left of sibling folder icons (folder rows have chevron).

#### Icons (exact mapping, all 16×16, margin-right .375rem)
| Item | Icon | Color var |
|---|---|---|
| Folder (open or closed — same icon) | solid folder, viewBox 0 0 20 20, `M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z` | `--theme-folder-icon` |
| Chevron | stroke `M9 5l7 7-7 7`, 0.15s rotate transition | `--theme-text-muted` |
| Markdown file | outlined document-with-lines `M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z` | `--theme-file-icon` |
| Config file (`file_type == "config"`) | gear (cog + center circle) | `--theme-config-icon` |

There is only one file type in practice: markdown. No per-extension icons.

#### `.fed` / gear icon
- `.fed` is FrankMD's config file at NOTES_PATH root (INI-ish `key = value`; holds settings, theme, locale, `recent_files` JSON, etc.).
- The tree helper still has a "config" branch: gear icon, **not draggable, no context menu**, excluded from Delete/F2, clicking would `selectFile` (tree_helper.rb:73-87). `getFileType(".fed") === "config"` (app_controller:1116-1121): opening it hides the editor toolbar and preview, disables title rename, and the editor reloads it when `frankmd:config-file-modified` fires (app_controller:1562-1597).
- **However, in current code `.fed` never appears in the tree** (dotfiles skipped by `build_tree`) and the server **rejects** `.fed` on notes show/create/update/destroy/rename (`notes_controller.rb#reject_config_file` → 403). So the gear/config path is effectively vestigial. A Rust rewrite can drop it, or (if desired) re-add "open .fed" as an explicit settings action — decide at product level.

#### Row styling (`app/assets/tailwind/components/tree.css`)
- `.tree-item`: flex, align center, padding .25rem .5rem, radius .375rem, pointer, `user-select:none`, 0.875rem, color `--theme-text-secondary`; hover bg `--theme-bg-hover`.
- Selected (`.explorer-selected`): bg `--theme-selection`, color `--theme-selection-text`.
- Active/focused-in-selection (`.explorer-active`): `outline 0.5px solid --theme-accent; offset -2px`.
- The server class `.selected` (currently open file) has **no visual style of its own**; visual highlight comes from the JS selection model (opening a file sets selection to it). `.selected` is used for logic (vim `:next-note`/`:prev-note`, Hugo created-path inference).
- Names truncate with ellipsis.

---

### F.3 Expand / collapse state

- `this.expandedFolders` = in-memory `Set` of folder paths in app_controller. **Not persisted** (no localStorage, no cookie, no server). Page load always renders with an empty set (`notes_controller.rb:16,58`), i.e. everything collapsed; then opening a file adds its ancestors and re-fetches the tree, so after reload only the open file's ancestor chain is expanded.
- Click on a folder row = **select it AND toggle expansion** (`toggleFolder`, app_controller:665-685). Toggling ignores modifiers — Ctrl/Shift-click also toggles.
- Every server tree render receives `expanded=<comma-joined paths>` so state survives re-renders. Folder rename/move remaps expanded paths of the folder and descendants (client app_controller:2972-2981 and server folders_controller.rb:63-71).
- Auto-expansion ("reveal"):
  - Opening any file: all ancestor folders added (`expandParentFolders`) then tree refreshed.
  - File finder / content search / wikilink / library "open note" go through `openFileAndRevealInTree` (ancestors expanded **and sidebar forced visible**, even on mobile) (app_controller:2246-2263).
  - Creating a note/folder inside a folder: the destination and all ancestors are expanded **before** the request so the new item is visible even if the folder was collapsed (#268) (file_operations_controller.js:380-383, 432-433).
  - Drag-move: destination folder + ancestors expanded (drag_drop_controller.js:273-274, app_controller:709-715).
  - New folder created: the new folder path itself is added to the expanded set.
- **No "reveal current file" command** and **no scroll-into-view** of the selected row after reveal. No "collapse all".

---

### F.4 Selection model (app_controller:735-981)

State: `explorerSelection: Map<key, {path,type}>` with key = `JSON.stringify([type, path])`; `explorerSelectionAnchor` (key); `explorerActiveKey` (key). Rendering: `.explorer-selected` on every selected row, `.explorer-active` on the active one.

| Gesture | Effect |
|---|---|
| Plain click file | selection := {file}; anchor := file; **opens file** |
| Plain click folder | selection := {folder}; anchor := folder; toggles expand |
| Ctrl/Cmd+click | toggle item in selection; anchor := item; **file is NOT opened** |
| Shift+click | range from anchor to item over **visible rows only** (rows not inside a `.tree-children.hidden`), in DOM order (folders and files mixed); replaces selection. If no anchor/anchor not visible → selection := {item}, anchor := item. Anchor unchanged. File NOT opened. |
| Ctrl+Shift+click | range **added** to existing selection |
| Tab focus onto a row (not caused by a pointer down) | plain-select that row (no open) (`onExplorerItemFocus`) |
| Opening a file by any means | selection := {that file} (`selectExplorerPath`) |
| Click empty tree background | selection cleared (+ background menu, §F.6) |
| Background menu "Select All" | selection := all visible rows; anchor := first visible; active := last |
| Right-click an unselected row | selection := {that row} (no open) |
| Right-click a selected row | selection kept |

- Active key = clicked item if still selected, else previous active if still selected, else last-inserted.
- After any tree re-render, selection is re-applied and keys no longer present are pruned (`syncExplorerSelection({pruneMissing:true})`), and the previously focused row is re-focused by (path,type).
- Rename/move remaps selection keys, anchor and active key (prefix remap for folders). Delete removes the item and (for folders) all descendant keys.
- Clicking a row focuses it (rows are `tabindex=0`). Test `notes_test.rb:33` asserts after clicking a note: row is `.explorer-selected:focus` and the editor is **not** focused.
- **Keyboard in tree** (handlers bound on each row, act only if row is `.explorer-selected`, no modifiers, not auto-repeat):
  - **Delete** → delete all selected (non-config) items with confirmation (app_controller:953-966).
  - **F2** → rename, only when exactly one item is selected (app_controller:968-981).
  - There is **no arrow-key navigation and no Enter-to-open** in the tree (only Tab focus moves). A reimplementation may add them, but they are not current behavior.
- Delete key inside the editor never deletes notes (`notes_test.rb:441`).

---

### F.5 Item context menu (right-click on a row)

Live markup: `_file_operations.html.erb:389-434`, positioned `fixed`, min-width 160px, bg `--theme-bg-secondary`, border, radius-lg, shadow, py-1, z-50. Items are `w-full px-3 py-1.5 text-sm` rows with 16px icon + label, hover `--theme-bg-hover`.

Order and visibility (`file_operations_controller.js:118-166`):
| Item | File | Folder | Action |
|---|---|---|---|
| New Note (doc-plus icon) | hidden | shown | open New Note dialog with parent = this folder (explicit destination) |
| New Folder (folder-plus icon) | hidden | shown | open New Folder dialog with parent = this folder |
| Rename (pencil-square icon) | shown | shown | open Rename dialog for **the right-clicked item** (even if multiple selected) |
| Delete (trash icon, `--theme-error` text) | shown | shown | delete the **context selection**: whole selection if the right-clicked row was selected, else just that row |

- No context menu at all for config rows (returns early).
- Position: at cursor (clientX/Y). If overflowing right/bottom, moved to `innerWidth - width - 10` / `innerHeight - height - 10`, then `clampMenuToViewport` (max-height = viewport bottom − top − 8, min 96px; horizontal shift to keep 8px padding) (`lib/menu_bounds.js`).
- Closes on any document click outside it; Escape (via `closeAllDialogs`); choosing an item.
- Right-click on the tree **background** has no custom handler (browser native menu).

---

### F.6 Explorer background (empty area) behavior (#261, #267)

`app_controller:858-910`, `dialogs/_explorer_background_menu.html.erb`.
- **Left click** on `#file-tree-content` where the target is not inside a `.tree-item` → clears selection **and** opens the **Explorer menu** at the click point (same positioning/clamp rules as context menu; min-width 180px; `role=menu`, aria-label "Explorer menu"). First button receives focus.
- Menu items (each `role=menuitem`):
  1. **New Note** → `newNote` (destination = root, because selection was just cleared) + hide menu.
  2. **New Folder** → `newFolder` (root) + hide.
  3. **Select All** → select all visible rows, hide.
  4. **Find File** → open file finder, hide.
- Dismissal: click anywhere outside the menu that is not inside the tree; clicking/selecting any tree row (selectExplorerItem hides it); **Escape** (handled in `onWorkspaceKeydown`, when no `<dialog>` is open).
- Test flow (`explorer_background_test.rb`): select note → click empty area → menu visible, nothing selected → toolbar New Note shows `root/note-name.md` preview → Cancel → menu hidden; Select All selects 2 notes; Find File opens finder; menu New Note creates `menu_note.md` at root (preview `root/menu_note.md`); Escape hides menu.

---

### F.7 Creation destination logic

`file_operations_controller.js:174-188, 253-255, 313-322`.

`selectedCreationFolder()` = the selected item's path **iff exactly one item is selected and it is a folder**; otherwise `""` (root). A selected file → root (not the file's parent).

| Entry point | Plain note destination | Hugo post destination | Folder destination |
|---|---|---|---|
| Sidebar New Note button / Ctrl+N / Home "New Note" / background menu | `selectedCreationFolder()` | **`blog`** always (ignores selection; `fromSelection:true`) | — |
| Sidebar New Folder button / background menu | — | — | `selectedCreationFolder()` |
| Folder context menu "New Note" | that folder | that folder (explicit) | — |
| Folder context menu "New Folder" | — | — | that folder |

Hugo final path (server `HugoService.generate_blog_post`, mirrored client-side in `lib/new_note_utils.js` for the preview):
- `hugo_path_style` config (`.fed` key / env `HUGO_PATH_STYLE`, default `dated`):
  - `dated` → `<parent>/YYYY/MM/DD/<slug>/index.md` (server local time; client preview uses browser date)
  - `flat` → `<parent>/<slug>.md`
- `slugify`: lowercase → map accented chars via ACCENT_MAP (à→a … æ→ae, ß→ss, þ→th, ł→l etc.) → replace runs of `[^a-z0-9]+` with `-` → strip leading/trailing `-` → collapse `-+`.
- Title = name with trailing `.md` removed. Content = frontmatter template: `.hugo_template.md` at notes root if present, else:
  ```
  ---
  title: "{{title}}"      (double quotes in title escaped as \")
  slug: "{{slug}}"
  date: {{date}}          (ISO8601 with offset, e.g. 2026-11-04T10:20:30-03:00)
  draft: true
  tags: []
  ---

  ```
- Tests: `creation_destination_test.rb` (select docs→docs/nested; preview `docs/nested/Sample.md`; Hugo preview `blog/YYYY/MM/DD/sample/index.md`; back to Empty → docs/nested; New Folder shows `docs/nested`; collapsed selected folder gets revealed; flat style toolbar → `blog/my-post.md`, context menu → `docs/explicit-post.md`), `notes_test.rb:76` (toolbar Hugo → `blog/<date>/hello-world/index.md`).

---

### F.8 New Note dialog (single step)

Markup `_file_operations.html.erb:459-530`; logic `file_operations_controller.js:190-310`, `lib/new_note_utils.js`.
- Modal `<dialog>` w-96 (24rem), p-4. Title "New Note".
- Name input (placeholder "Note name"), auto-focused, cleared each open.
- Two **template cards** (2-col grid), toggle buttons with `aria-pressed`:
  - **Empty Document** / "Plain markdown" (doc icon, muted) — default selected on every open.
  - **Hugo Blog Post** / "Hugo page bundle" (newspaper icon, orange-500).
  - Selected card: accent border + 1px accent ring + hover bg. Clicking a card returns focus to the name input. Long-locale titles must wrap, not clip (`notes_test.rb:106`).
- **Live path preview** row: small solid folder icon (`--theme-folder-icon`) + monospace xs text:
  - Empty template: `<parent or "root">/<name>.md` (`.md` appended unless already present); while name blank: `<where>/note-name.md` (placeholder slot "note-name").
  - Hugo: `<where>/YYYY/MM/DD/<slug-or-name-or-note-name>/index.md` (dated) or `<where>/<slot>.md` (flat).
  - Colors: valid → muted; blank → faint; invalid → `--theme-error` with row tooltip "Name cannot contain slashes".
- Validation (`canCreateNote`): trimmed non-empty, no `/`, not `.` or `..`; for Hugo also slug must be non-empty. **Create** button disabled while invalid; Enter is also gated.
- Keys: Enter → submit; Escape → close. Backdrop click closes (all three file-op dialogs, file_operations_controller.js:95-109).
- Submit: plain → `POST /notes/<encoded path>` `{content:"", expanded}`; Hugo → `POST /notes` `{template:"hugo", title, parent, expanded}`. Response is a Turbo Stream that replaces `#file-tree-content` (server marks the new note `.selected`; header `X-Created-Path` gives the real path). Errors (e.g. "Note already exists", 422) shown inline in red `role=alert` under the preview; dialog stays open.
- After success: dialog closes, `file-created` → app expands ancestors and **opens the new note** (empty note → editor focused); records a session "created-note boundary" so undo-at-history-start can offer to remove the just-created note (undo-created-note dialog; editor section).

### F.9 New Folder dialog

`_file_operations.html.erb:533-568`, `file_operations_controller.js:313-376,430-447`.
- Modal w-80. Title "New Folder"; subtitle "In: <parent>" or "In: root folder" (truncate, full value in title). Input placeholder "Folder name".
- Only check client-side: trimmed non-empty (no slash check — `a/b` creates nested folders since server `mkdir_p`s the parent). Enter submits, Escape closes.
- `POST /folders/<encoded path>?expanded=…` → Turbo Stream tree. Errors inline ("Folder already exists", permission, etc.). New folder is added to expanded set; it is **not** selected or opened.

### F.10 Rename (Explorer dialog)

Opened by context menu Rename or F2. `file_operations_controller.js:449-533`.
- Modal w-80, title "Rename", input placeholder "New name", buttons Cancel / Rename.
- Prefill: last path segment; files have `.md` stripped. Input focused **and fully selected**.
- Enter submits, Escape closes, backdrop click closes.
- Submit (`renamePath`, shared with header inline rename): blank → error "Title is required"; files get `.md` appended if missing; new path = old path with last segment replaced (a name containing `/` therefore moves the item into subpaths — not blocked). Same path → no-op success.
- If the item is the open file or a folder containing it, autosave `prepareForTransition()` first (flush draft); failure → error "Unable to save local draft. Keep this page open and try again before leaving." and abort.
- `POST /notes/<old>/rename` or `/folders/<old>/rename` `{new_path, expanded}` → Turbo Stream tree. Error → inline red message in dialog (server message, e.g. "Note already exists").
- Success → `file-renamed` → remap expanded folders, selection, session undo history, created-note boundaries, autosave keys; if open file affected, update current path, header title and URL (`pushState`).
- Server side extras: recent-files history remapped; **folder rename rewrites `slug:` in `<folder>/index.md` frontmatter** to `slugify(new folder basename)` if the file has a slug line (`app/models/folder.rb:123-138`). Note renames do not touch frontmatter.
- Inline rename in the tree is **not** implemented; the "inline" rename is the header title (§F.14).

### F.11 Delete

`file_operations_controller.js:535-616`.
- Triggers: context menu Delete (context selection), Delete key on a selected row (all selected).
- Items de-duplicated; config items dropped; items inside a selected folder are dropped (the folder covers them).
- **Confirmation** via app alertdialog (`lib/app_prompt.js`): title "Confirm", destructive accept button "Delete", Cancel; dialog itself focused initially. Messages:
  - file: `Are you sure you want to delete "<basename>"? This action cannot be undone.` (basename includes `.md`)
  - folder: `Are you sure you want to delete "<name>" and all its contents? This action cannot be undone.`
  - multiple: `Are you sure you want to delete these <N> selected items? This action cannot be undone.`
- Draft protection for the open file as in rename.
- Deletes sequentially: `DELETE /notes/<path>?expanded=` or `/folders/<path>?expanded=`. **Server refuses non-empty folders** ("Folder not empty: <path>") despite the confirmation text (`notes_service.rb:131`, asserted `notes_test.rb:422`). First failure → alert with server error, stop remaining.
- Each success → `file-deleted`: selection pruned; autosave draft cleanup (failure → 5s temporary message); undo history evicted; if the open file (or its folder) was deleted → editor cleared, header shows "Select or create a note", stats hidden, Home placeholder visible, URL `replaceState("/")`. Server also removes it from recent files.
- Notes are deleted permanently (no trash).

---

### F.12 Drag & drop in the tree (`drag_drop_controller.js`)

- Draggable: folder rows and file rows (not config). Drop targets: **folder rows** and the **tree container (= root)**.
- Drag start: if the grabbed row is part of a multi-selection (size>1), the drag carries the whole selection minus items that live inside a selected folder (#266); otherwise only the grabbed item. `effectAllowed=move`, `text/plain` = path. Grabbed row gets `.dragging` (opacity .5) then `.drag-ghost` (opacity .4).
- Hover feedback (computed in the tree's dragover from the element under the pointer, `closest([data-drop-id])`): exactly one folder row gets `.drop-highlight` (accent bg, accent-text, 2px accent outline offset −2px, icons recolored) if valid. Valid target = folder, not one of the dragged items, not inside a dragged folder, and not the case that **every** dragged item already lives directly in it.
- Root highlight: on dragenter of the tree, if any dragged item is not already at root, tree gets `.drop-highlight-root` (selection bg, 2px dashed accent outline, offset −4px). Removed on real leave of the tree (checks relatedTarget / elementFromPoint), drop, dragend. Folder and root highlights can show simultaneously.
- Drop on a valid folder row → each item moved to `<folder>/<basename>`. Drop on an invalid folder row → nothing (propagation stopped). Drop anywhere else in the tree (empty area **or a file row**, since file rows have no drop handler) → move each item to root `<basename>`. Items already at destination skipped.
- **No confirmation.** Moves are sequential `POST /<notes|folders>/<old>/rename {new_path, expanded}`; first failure → alert "Failed to move" (generic) and stop. Name collision at destination fails (server `AlreadyExistsError`).
- Before each move: draft protection for the open file; destination folder + ancestors expanded. After: `item-moved` → same remapping as rename (current file/URL/title/autosave/selection/expanded).
- Moving a folder also triggers the Hugo `index.md` slug rewrite (same as rename).
- External OS file drops onto the tree are ignored (no dragged item).
- Tests: `multi_drag_test.rb` — Ctrl-selected first+second dropped on `target` → both moved, unselected `rooted.md` stays; dragging an unselected item moves only it even when another item is selected.

---

### F.13 Tree refresh

- **No polling / no file watcher.** The tree updates when:
  1. A create/rename/move/delete response arrives (Turbo Stream `update #file-tree-content` with server-rendered partial, using the `expanded` param; `selected` = created/renamed path or none).
  2. `refreshTree()` after every file open, Home/clear-editor, and file-not-found: `GET /notes/tree?expanded=<csv>&selected=<currentFile>` → HTML replaced (app_controller:3126-3157). Stale responses are discarded via navigation generation + tree revision counters (any toggle/op bumps the revision). Focus is restored to the same (path,type) row.
- Therefore external filesystem changes appear only on the next open/navigation.

---

### F.14 Path display / document title (header) (`path_display_controller.js`, `_header.html.erb`)

- The header (`.workspace-header`) shows the current document **basename without `.md`** in a button (`data-path-display-basename-value=true`), CSS-truncated with ellipsis. Tooltip (`title`) = full relative path (with `.md`); aria-label "Rename document: <name>". No breadcrumb.
- No file: text "Select or create a note"; button disabled. Header itself is hidden while Home is visible.
- File-not-found: title area shows the path in red + `(message)` muted for **5 s**, then returns to Home (app_controller:608-641).
- **Click (or Enter/Space) on the title → inline rename** (`renameCurrentNote`, app_controller:1674-1754): button hidden, auto-width text input shown (width measured via hidden sizer span + padding/border), value = basename without `.md`, caret at end (not selected). Enter submits, Escape cancels (focus back to title), **blur submits**; IME composition ignored. Blank → inline error "Title is required" + `aria-invalid`. Server error (e.g. "Note already exists") inline, input stays focused. Success keeps the folder, updates URL `/notes/<new>`, keeps unsaved edits (draft flushed first). Disabled for `.fed`/no file. Tests `workspace_test.rb:264-400`.
- The controller also contains an unused full-path mode (left-truncation by dropping leading segments with `...` prefix, recomputed on resize debounced 100ms, hover shows full) and an unbound `copy()` (clipboard + toast "Content copied to clipboard", 2s). Not wired in any view — optional.

---

### F.15 Quick File Finder (Ctrl+P)

Files: `file_finder_controller.js`, `dialogs/_file_finder.html.erb`, `lib/text_utils.js:53-91`.
- Open via Ctrl/Cmd+P, sidebar Find File button, background menu, Home "Find file".
- Modal 600px wide, max-h 70vh. Rows: search input (magnifier icon, placeholder "Type to search files...", right hint "esc to close"); results list (min-h 200, max-h 300, scroll); preview panel (border-top, tertiary bg, max-h 200, scroll).
- **Candidate list**: all `[data-type=file]` rows currently in the DOM tree (includes files inside collapsed folders since they're rendered hidden), mapped to `{path, name, mtime}` and **sorted by mtime descending** (app_controller:2221-2230). Note: this is file mtime, **not** the recent-files history.
- **Empty query**: first **10** files by mtime (newest modified first).
- **Non-empty query**: `q = input.trim().toLowerCase()`; for each file `score = fuzzyScore(path.toLowerCase(), q)` on the **full relative path including folders and `.md`**; keep `score > 0`; sort by score desc (JS stable sort ⇒ ties keep mtime order); take **10**.
- **fuzzyScore(str, query)** exact algorithm:
  ```
  score=0; strIndex=0; prev=-1; consec=0
  for each char c in query:
     i = str.indexOf(c, strIndex); if i == -1 return 0
     score += 1
     if i == prev+1: consec += 2; score += consec     # note: prev starts at -1, so a first match at index 0 counts as consecutive
     else: consec = 0
     if i == 0 or str[i-1] in {'/', '-', '_'}: score += 3
     prev = i; strIndex = i+1
  score += max(0, 10 - (len(str) - len(query)))
  return score
  ```
  Greedy leftmost subsequence match (no backtracking); spaces in query are literal characters.
- Result row: doc icon; bold name (basename sans `.md`); second line = directory part (path sans `.md` with trailing `name` and `/` removed; omitted for root files). Selected row: accent bg / accent-text. No match → "No files found" and empty preview.
- **Preview**: on every selection change, `GET /notes/<path>` (Accept JSON); shows the **first 10 lines** in a monospace pre-wrap block, plus `\n...` when ≥10 lines; error → "Unable to load preview". Not debounced/cancelled.
- Keys: ↓/↑ move selection (clamped, no wrap; no auto-scroll); Enter opens selected; Esc closes (native dialog). Mouse hover selects unless the last navigation was keyboard (reset by mousemove). Click opens.
- Opening → `openFileAndRevealInTree(path)`: expand ancestors, force sidebar visible, load file, record recent.
- Query resets to empty on each open.

---

### F.16 Content Search (Ctrl+Shift+F)

Files: `content_search_controller.js`, `dialogs/_content_search.html.erb`, `_search_results.html.erb`, `notes_service.rb:157-193,346-397`.
- Open via Ctrl/Cmd+Shift+F or sidebar button. Modal 700px, max-h 80vh.
- Header: monospace input (placeholder "Search in files (supports regex)..."), "esc to close" hint; status line (initial "Type to search in file contents (supports regex)"); a static cheat-sheet of regex examples: `hello` literal text, `hello|world` OR, `hel+o` one or more, `hel.*o` any chars, `^hello` start of line, `hello$` end of line, `\d+` digits, `\bword\b` whole word.
- **No toggles**: always regex, always **case-insensitive**. Invalid regex → silently treated as escaped literal. Regex timeout 0.5 s → empty result set.
- Input: trimmed; empty → clear results/status. Otherwise status "Searching...", overlay spinner shown immediately, request after **300 ms debounce** (`GET /notes/search?q=`). In-flight requests are not aborted (a slow older response could overwrite a newer one).
- Server search: recursive over `.md` files, skipping dot-entries and symlinks (does **not** apply the root images/videos rule); files > 5 MB skipped; invalid UTF-8 scrubbed; each matching **line** is one result; **max 20 results total** (stops scanning when reached; per-file capped at remaining); **3 context lines** before/after; final results sorted by file mtime desc (within-file line order not guaranteed by Ruby sort_by).
- Result card (button, one per match): doc icon, bold file name (basename sans `.md`), `:<line>` small, path (sans `.md`) right-aligned truncated; below, a monospace context block (rounded, tertiary bg; black/20 when selected) with a right-aligned 2.5rem line-number gutter (faint, non-selectable) and the matched line highlighted with `--theme-selection`/`--theme-selection-text`. **No substring highlight** inside the line. First result pre-selected (accent bg).
- Status after results: `"<N> match(es) found[ (showing first 20)] - use ↑↓ to navigate, Enter to open"` (hard-coded English; "(showing first 20)" when N ≥ 20) or "No matches found"; error → "Search error".
- Keys: ↓/↑ (clamped) + `scrollIntoView({block:"nearest"})`; Enter opens; Esc closes. Hover selection with the same keyboard-vs-mouse guard as the finder; click opens.
- Opening → `openFileAndRevealInTree(path)` then `codemirror.jumpToLine(line)`: cursor at start of that line, scrolled to vertical center, editor focused.

---

### F.17 Home screen (`_home.html.erb`, `_recent_files.html.erb`, `home_controller.js`, `recent_files_service.rb`)

- **Shown when no document is open**: root URL `/`, after clicking the footer Home button (plain left click only; modified clicks ignored), browser Back to `/`, after the open file is deleted, after a file-not-found timeout. It lives inside the editor placeholder; the document header is hidden while it shows (`updateWorkspaceChrome`). Preview pane is hidden on Home; if it was visible before, it is re-shown when the next markdown note opens (app_controller:510-514). `showHome` pushes `/`, focuses the `h1`, closes the mobile sidebar.
- Layout: centered column (max 48rem, vertical centering, padding `clamp(1.25rem,5vw,4rem) 1.5rem`).
  - `h1#home-title` (tabindex −1): logo (1em) + "FrankMD", 2.25rem semibold.
  - Description (muted, max 360px): "A simple, self-hosted Markdown editor for writing and organizing your notes."
  - Action buttons (bordered pills): **Find file** (opens finder), **New Note** (opens New Note dialog with toolbar destination rules).
  - **Recently opened** section — only rendered when the history is non-empty. Heading + right "Clear history" underlined text button. List (`ul#home-recent-files`, gap .5rem) of cards (`<a href="/notes/<path>">`, 2-col grid: details | time): name (basename sans `.md`), full path (xs muted, wraps anywhere), relative time on the right (`<time datetime title=ISO>`): "Just now" (0 s), "N second(s) ago" (<60 s), "N minute(s) ago", "N hour(s) ago", "N day(s) ago", "Over a week ago" (>7 days), "Unknown time" if unparsable. Computed server-side at render.
  - First **5** visible; entries 6–10 hidden; "Show more" reveals them and focuses the 6th link and smooth-scrolls (instant if reduced-motion) to "Show less"; "Show less" hides them and focuses "Show more" (`aria-expanded` maintained).
  - Error line (`role=status`, `--theme-error`): "Unable to update shared history. Try again."
- Card click: plain left click → `app.loadFile(path)` (SPA open; expands ancestors). Modifier/middle click → browser follows the href.
- **Recent files storage**: server-side in `.fed` under key **`recent_files`** (JSON array of `{path, opened_at(ISO8601 µs UTC)}`), so it is **shared across devices/browsers** using the same notes folder. **LIMIT = 10**, most-recent first, deduped by path.
  - Record: every successful open of a **markdown** note → `POST /recent_files {path}` (serialized promise chain). Server rejects (404) if file missing; rejects paths not ending `.md`, containing `\`/NUL, any empty or dot-prefixed segment, > 4096 bytes.
  - Read filters out entries whose file no longer exists (external deletions disappear) (`home_recent_files_test.rb`).
  - Rename/move → entries remapped (prefix for folders); delete → removed (server, in NotesService).
  - Refresh: `GET /recent_files` (HTML partial) on window focus (only if Home visible), on clear-editor/Home, on file-not-found. Aborts previous request.
  - Clear: confirm "Clear recent-file history for every device using this notes folder?" (Confirm/Cancel) → `DELETE /recent_files` → refresh (section disappears).
- Clicking a card for an externally-deleted file shows "Note not found" and never recreates it.
- Mobile: Home reachable via sidebar footer; no horizontal overflow at 390px.

---

### F.18 Notifications used in this area
- `appConfirm` / `appAlert` (`lib/app_prompt.js`): queued modal `<dialog role="alertdialog">` titled "Confirm"/"Alert"; destructive confirm focuses the dialog, non-destructive focuses first (Cancel) button; Esc = cancel.
- `showTemporaryMessage(msg, ms=2000)`: bottom-center pill (secondary bg, border, shadow), replaces any existing one (used for draft-cleanup failure, 5000 ms).
- Inline errors (`lib/inline_messages`): red `role=alert` paragraphs in dialogs / under title input.
- Autosave status "Error loading" when a file load throws (non-404).

### F.19 Endpoints used (names only)
`GET /notes/tree?expanded&selected` · `GET /notes/<path>` (JSON: path, content, revision) · `POST /notes/<path>` (create empty) · `POST /notes` (Hugo) · `POST /notes/<path>/rename` · `DELETE /notes/<path>` · `POST /folders/<path>` · `POST /folders/<path>/rename` · `DELETE /folders/<path>` · `GET /notes/search?q` (HTML partial; JSON variant exists) · `GET|POST|DELETE /recent_files`. Mutations accept `expanded` (CSV) so the server re-renders the tree with the client's expansion state. URLs: open note = `/notes/<per-segment-encoded path>` (pushState), Home = `/`.

### F.20 Keyboard summary for this area
Ctrl/Cmd+N new note · Ctrl/Cmd+P file finder · Ctrl/Cmd+Shift+F content search · Ctrl/Cmd+E toggle explorer · Delete (on selected tree row) delete · F2 (single selected row) rename · Esc: cancel title rename → close overflow menu → close explorer menu → close mobile sidebar (first applicable; only when no dialog open) · in finder/search: ↑ ↓ Enter Esc · in dialogs: Enter submit, Esc close · Vim `:next-note`/`:prev-note` cycle through `.tree-item.selected` file rows in DOM order (wraps).

---

## 8a / 11. Media dialogs (image picker, video, library, storage) & paste / drag-and-drop  
<sub>(sub-section labels below keep the source numbering, e.g. D1.x)</sub>

Paths are relative to `~/Projects/FrankMD/app/`. JS = `javascript/`. Visible English strings come from `config/locales/en.yml`, and every string goes through i18n (`window.t`).

### D1.0 Entry points and wiring

| Trigger | Effect | Source |
|---|---|---|
| Workspace toolbar action `image` ("Insert Image", pinned) | `app#openImagePicker` → `image-picker#open()` | `helpers/workspace_helper.rb:6`, `JS/controllers/app_controller.js:1445` |
| Workspace toolbar action `video` ("Insert Video", in the overflow menu, not pinned) | `app#openVideoDialog` → `video-dialog#open()` | `workspace_helper.rb:9`, `app_controller.js:2443` |
| Slash commands `/image` and `/video` (filter text "insert image picture photo" and "insert video embed youtube") | Opens the same dialog. The pending `/query` range `{from,to}` is replaced on insert. Closing the dialog fires `close->app#onSlashCommandDialogClose` and clears the pending range | `JS/lib/codemirror_slash_commands.js:28-29`, `app_controller.js:1345-1357` |
| Paste of a clipboard image into the editor | Opens the Image Picker with the pasted file pre-loaded and pre-selected. The image is **not** inserted until the user clicks Insert | see D1.6 |
| Sidebar "Open media Library" button (`_library_button.html.erb`) | `app#toggleLibrary`. The Library replaces the editor workspace | see D1.4 |
| Settings panel, "Storage for new media" dropdown (`_media_storage_menu.html.erb`, rendered at `_settings_panel.html.erb:128`) | Switches the media provider between Local and S3 | see D1.5 |

No keyboard shortcut is bound to the image picker, video dialog or Library. `keyboard_shortcuts.js` has no media entries.

Insert handlers in the app run only when the open file is Markdown (`isMarkdownFile()`). Otherwise the pending slash range is cleared and nothing is inserted (`app_controller.js:1411-1415`, `2452-2456`).

---

### D1.1 Image Picker dialog (`views/notes/dialogs/_image_picker.html.erb`, `JS/controllers/image_picker_controller.js`)

**Shell:** native `<dialog>` opened with `showModal()`. Size is `w-[700px] max-w-[90vw] max-h-[90vh]`. Native Esc closes it. Layout from top to bottom:

1. Header: title "Insert Image" and a close X.
2. **Media storage selector** (`notes/_media_storage`): "Storage for new media" `<select>` with Local / S3. The S3 option is disabled when S3 is not configured. An inline error line sits below it.
3. Tab bar (`role=tablist`, horizontally scrollable). Tabs in this order: **Upload** (`drop`), **Library** (`local`), **Folder**, **Web Search**, **Google**, **Pinterest**, **AI Generate**.
4. Scrollable panel area.
5. **Shared "Selected image options" footer section.** It stays hidden until a source dispatches `selected`. It contains:
   - "Selected: <name>"
   - **Alt Text (optional)** input, placeholder "Describe the image...", prefilled with the source's default alt.
   - **Link URL (optional, makes image clickable)** input, placeholder "https://...".
6. Loading row: a spinner plus text. The default text is "Uploading to S3...", replaced at runtime by "Processing image...".
7. Inline error line (`role=alert`).
8. Footer: **Cancel** and **Insert Image**. Insert is disabled until something is selected.

**Tab behavior** (`image_picker_controller.js:139-230`):
- Active tab: accent bottom border and accent text. Inactive tabs: muted.
- ARIA: `aria-selected` is set and roving `tabIndex` is applied (0 on the active tab, -1 on the others).
- ArrowLeft/ArrowRight on a tab cycles with wrap-around and moves focus.
- **Mouse wheel over the tab bar** switches tabs: deltaY>0 or deltaX>0 goes to the next tab, otherwise the previous one, with wrap.
- Switching to a tab calls that source's `activate()`.

**Open sequence** (`open()`, :82-93):
1. Clear the error.
2. **await** media-storage `refresh()` (GET `/config`).
3. Reset all sources: alt and link cleared, options hidden, Insert disabled.
4. Switch to the Upload tab and focus the dropzone.
5. `showModal()`.

**Config load** happens on connect: GET `/images/config` returns `{enabled, s3_enabled, google_enabled, image_extensions, video_extensions}`. On failure the values fall back to `{enabled:false, s3_enabled:false, google_enabled:false}`. AI separately loads GET `/ai/image_config` → `{enabled, model}` (:32-73).

**Close** revokes folder object URLs (`folderCtrl.source.cleanup()`).

**Insert flow** (`insertImage()`, :271-310):
1. Show loading "Processing image..." and disable Insert.
2. `await mediaStorage.ready()`, which waits for any in-flight provider save. A failed save also aborts the upload.
3. `url = await activeSource.getImageUrl()`. If the URL is null, stop silently.
4. Compute `alt = altInput.trim() || selected.name || "Image"` and `link = linkInput.trim()`.
5. Build the markdown:
   - `![${alt}](${url})`
   - with a link: `[![${alt}](${url})](${link})`

   The alt text is **not** escaped at this stage.
6. Dispatch `image-picker:image-selected {markdown,imageUrl,altText,linkUrl}`, then close.
7. On error: if the provider is S3, refresh the media storage, which may trigger the fallback warning. Show inline "Failed to insert image: %{error}" and re-enable Insert.

**App-side rewrite** (`app_controller.js:1411-1441`). If `imageUrl` starts with `images/` (a local upload under `NOTES_PATH/images`), the markdown is rebuilt with:
- a path relative to the open note (`relativeMediaPath`: for each folder level of the note, `../` is added, after removing shared leading segments),
- each segment `encodeURIComponent`'d, with `! ' ( ) *` also percent-encoded,
- the alt escaped through `escapeMarkdownAlt` (newlines become a space; `\ [ ]` are backslash-escaped).

Example: note `blog/post.md` plus upload `images/20260704_101500_ab12cd34_photo.png` gives `![photo](../images/20260704_101500_ab12cd34_photo.png)`.

The server names local uploads `images/<YYYYmmdd_HHMMSS>_<8 hex>_<sanitized original>` (`services/upload_storage.rb:118-120`).

**Text placement** (`insertImage` in `JS/lib/codemirror_content_insertion.js:243-274`):
- Replaces the current selection, or the slash range. For a slash range, spaces and tabs on both sides of the range are also consumed.
- A single `\n` is added before the image if the preceding text is non-empty and doesn't end in `\n`. A single `\n` is added after it if the following text is non-empty and doesn't start with `\n`.
- The cursor ends after the inserted text. The editor is focused and the change is marked dirty.

#### Shared "S3 option" sub-component (`dialogs/_s3_option.html.erb`, `JS/controllers/s3_option_controller.js`)

Each tab embeds one. It is hidden until the tab has a selection (`show()`). Parameters:
- `mode`:
  - `"upload"`: no checkbox, and the component always counts as checked (`copy=false` ⇒ `isChecked` is always true, :87).
  - `"reupload"`: shows the checkbox **"Save a copy in the active storage"**.
- `key_mode`:
  - `"full"`: label "Destination key", hint "Full S3 object key. Edit to change where the file is stored.". Default value is `frankmd/YYYY/MM/<filename>`.
  - `"prefix"`: label "Destination folder", hint "S3 key prefix. The filename is appended automatically.". Default value is `frankmd/YYYY/MM`.

  Both defaults use the local date, with the month zero-padded (`JS/lib/s3_key.js`).
- `show_resize` (default true).

Visibility rules:
- The resize row shows when checked. The key row shows when checked **and** the provider is S3.
- These rules are re-synced on every window `media-storage:changed` event.

Resize select:
- "None (original size)" (value `""`, the default)
- "1/4 size (25%)" (`0.25`)
- "1/2 size (50%)" (`0.5`)
- "2/3 size (67%)" (`0.67`)
- followed by the static text "+ compress"

Unchecking the box resets resize to the default.

Key input:
- It is auto-filled with the default unless the user has typed in it (`keyDirty`). A new selection updates the default but does not overwrite user edits.
- `keyValue` is returned only when checked, S3 is active and the input is non-empty after trimming.
- `hide()` resets everything.

#### Tab: Upload (`drop`) (`controllers/image_sources/drop_images_controller.js`, `lib/image_sources/folder_images.js`)

- Dropzone: dashed border, `p-10`, cloud-upload icon, "Drag & drop an image here". It is `tabindex=0 role=button`.
  - Click, Enter or Space opens a hidden `<input type=file accept="image/*" multiple>`.
  - Dragover highlights it with an accent border and tertiary background. Dragleave removes the highlight unless the pointer moved onto a child element (`lib/dropzone.js`).
- Ingest (`ingest()`):
  - Files are filtered by **filename extension**, case-insensitive, against `image_extensions` from config. The fallback is `.jpg .jpeg .png .gif .webp .bmp`.
  - Rejected files get a warning line, one sentence per file: "%{name}: %{ext} is not an accepted image type. Accepted: %{list}". `ext` is "no extension" when the filename has none.
  - Accepted files are sorted newest first by `lastModified`.
  - **Only the first 10 are rendered** (`filter("")` default limit 10). There is no pagination on this tab. The status reads "Showing 10 most recent of N images" or "N images found" / "1 image found".
  - A new ingest replaces the grid and clears any prior selection (`deselected` event → Insert disabled).
- Grid: 5 columns, `max-h-[300px]` scroll. Tiles are `.image-grid-item`: square, `object-fit: cover`, blue border on hover and when selected.
  - A centered `WxH` badge is shown when dimensions are known. Dimensions are measured by loading each object URL.
  - Tooltip: `name (WxH) - N KB`.
- Select:
  - Highlights the tile.
  - Shows the S3 option (mode upload, key_mode full) with the default key `frankmd/YYYY/MM/<filename>`.
  - Default alt = filename without extension, with `-` and `_` turned into spaces.
- `getImageUrl()`: POST `/images/upload` multipart with `file`, an optional `resize`, and an optional `s3_key`. Returns `data.url`.
- Note: the resize select is visible for local uploads as well, not only S3, because upload mode is always "checked".

#### Tab: Library (`local`) (`controllers/image_sources/local_images_controller.js`, partial `dialogs/_image_library_results.html.erb`)

Controls:
- **Collection** select:
  - "Note images" (`managed`, the default)
  - "Server image folder" (`server`). It is disabled when the `local_images` feature is not configured; `configure(enabled)` also forces it back to managed.
- **Per page** select: 10 / 25 / 50.
- Search input: "Search images by filename...". Debounced 300 ms, resets to offset 0.

Loading:
- On every activate or change: GET `/library/picker?collection=&search=&limit=&offset=`, which returns a **Turbo Stream** that replaces `#image-picker-library-results`.
- An earlier request still in flight is aborted.
- Each load clears the selection (`deselected` event).
- Load error: "Error loading images".

Results partial:
- If the collection is `server` but no server folder is configured: "Local Images Path Required".
- Grid: 3 columns, 5 at `sm` and wider; `min-h-160 max-h-250` scroll. Each tile is a square button (`data-path`, `data-name`, `data-url`, `data-collection`) holding a lazy `<img src=preview_url>`, plus a corner **trash** button.
- Empty: "No images found".
- Pager row: **Previous** (disabled at offset 0), "Showing %{from}–%{to} of %{total}", **Next** (disabled when `offset+limit >= total`).

Select:
- Marks the tile `.selected`.
- `server` collection: shows the S3 option (reupload checkbox, key_mode full, default key `frankmd/YYYY/MM/<name>`).
- `managed` collection: hides the S3 option.
- Alt = name without extension, with `-` and `_` turned into spaces.

`getImageUrl()`:
- `server` with the checkbox ticked: POST `/images/import {path, resize, s3_key?}` and return `url`.
- Otherwise: return the item's `url` as is. For managed images that is an `images/...` path, which the app rewrites to a note-relative path.

Delete (trash button):
- `appConfirm('Delete "%{name}"? This cannot be undone.')` with a destructive "Delete" button.
- DELETE `/images/file/<encodedPath>` for server images, or `/library/file/<encodedPath>` for managed ones.
- If the deleted item was the last one on a non-first page, step back one page. Then reload. Errors appear inline.

#### Tab: Folder (`controllers/image_sources/folder_images_controller.js`)

- Requires the browser File System Access API (`showDirectoryPicker`). Without it, an info box appears: "File System Access API Required", Brave flag steps, and "Chrome 86+, Edge 86+, Opera 72+, Brave (with flag)" / "Firefox and Safari do not support this API."
  - **Native-app note:** replace this with a native folder chooser.
- Before a folder is picked: "Select a folder from your computer to browse images" and a **Browse Folder...** button.
- After a folder is picked:
  - Status line, plus a "Change folder..." link.
  - Filename search (substring, case-insensitive, 300 ms debounce).
  - 5-column grid (`min-h-200 max-h-300`).
  - Pager: Previous, "Showing a–b of N", Next, and Per page 10/25/50.
- Directory scan is non-recursive: only top-level files whose extension is allowed. Sorted newest first by mtime.
- Errors: "Error accessing folder" or "Error reading folder". A cancelled picker is ignored.
- Select and upload work exactly like the Upload tab: S3 option in upload/full mode, POST `/images/upload`.

#### Tab: Web Search (`web_images_controller.js`, `lib/image_sources/web_images.js`)

- Text input "Search web images..." with Enter or a **Search** button (accent color).
- Status line, initially "Enter keywords and click Search or press Enter". It shows "Searching..." while the request runs.
- GET `/images/search_web?q=` returns `{images:[{url,thumbnail,title,source,width,height}], note?, error?}`.
- Empty query: "Please enter search keywords".
- Results: "Found N images - click to select" (singular "Found 1 image - click to select"). With zero results, the server's `note` is shown if present, otherwise "No images found".
- Errors appear in the status line in red.
- Grid: 4 columns, `min-h-200 max-h-350`. Each tile is a button showing the thumbnail (lazy loaded), a centered `WxH` badge, and the source domain on a bottom gradient. **A tile whose image fails to load is removed.** Only `isSafeImageUrl` thumbnails are rendered.
- Select:
  - Adds a blue ring.
  - Shows the S3 option in reupload/prefix mode, with no key default beyond `frankmd/YYYY/MM`.
  - Name = title, or "Image" if there is none.
  - Alt = title with `-` and `_` turned into spaces, cut to 100 characters.
- `getImageUrl()`:
  - Checkbox ticked: POST `/images/import {url, resize, s3_prefix?}` and return `url`.
  - Otherwise: **hotlink** the original remote URL.

#### Tab: Google (`google_images_controller.js`, `lib/image_sources/google_images.js`)

- When `google_enabled` is false: a notice "Google API Configuration Required", setup steps (`google_api_key`, `google_cse_id` in `.fed`), and a GitHub link.
- Otherwise the UI matches Web Search, with the placeholder "Search Google Images...".
- GET `/images/search_google?q=&start=N`. A new search starts at 1. Each response gives `{images,total,next_start}`.
- **Infinite scroll:** when the grid is scrolled within 100 px of the bottom, it is not already loading, and results exist, the next page is requested. Results are appended and the whole grid is re-rendered. The status shows "Found <total> images...".
- Selection, alt and save-a-copy behave as in Web Search, using `/images/import`.

#### Tab: Pinterest (`pinterest_images_controller.js`)

- Same as Web Search. The placeholder is "Search Pinterest..." and the Search button is red (`--theme-error`).
- GET `/images/search_pinterest?q=`. There is no pagination.

#### Tab: AI Generate (`ai_images_controller.js`, `lib/image_sources/ai_images.js`)

- When disabled: a notice "Gemini API Key Required" with steps (aistudio.google.com; add `gemini_api_key` to `.fed`).
- Form:
  - "Model: <model>", taken from `/ai/image_config`.
  - **Reference image (optional)** section. It is shown only when local server images are enabled (`config.enabled`).
    - Search input "Search reference images..." (300 ms debounce).
    - 6-column grid (`max-h-120`) of server images, from GET `/images` or `/images?search=`, with previews at `/images/preview/<path>`. Loaded on first activate.
    - Clicking an image picks it. The grid is replaced by a preview box (64×64 thumbnail, name, and a "Remove reference" link). Remove clears the reference and reloads the grid.
  - Prompt `<textarea rows=2>`, labelled "Describe the image you want to generate". **Enter generates, Shift+Enter adds a newline.**
  - **Generate Image** button.
- Generate:
  - POST `/ai/generate_image {prompt, reference_image_path?}`.
  - While running:
    - A spinner box shows "Generating image with <model>"; the model name falls back to `imagen-4.0-generate-001`.
    - "Press ESC to cancel". **Esc aborts the request** through a document-level keydown listener.
    - The button is disabled and the previous result hidden.
  - Errors appear inline. Fixed messages:
    - empty prompt: "Please enter a prompt describing the image you want to generate"
    - not enabled: "AI image generation is not configured..."
    - network failure: "Failed to generate image. Please try again."
  - Response: `{data(base64)?, url?, mime_type, model, revised_prompt?}`.
- Result:
  - Preview `<img>` (max 200×150). The source is `data:<mime||image/png>;base64,...` or the returned URL.
  - "AI-revised prompt" box (2-line clamp), shown when the response includes one.
  - **S3-only** "Destination folder" input (`media-storage` target `s3Only`, visible only when the provider is S3). Its hint is the prefix hint. There is no resize option.
  - Auto-selects: name `ai_<epochms>.png`, alt = the prompt, or its first 100 characters plus `...`.
- Save on Insert:
  - Base64 result: POST `/images/upload_base64 {data, mime_type, filename:"ai_<epochms>.png", s3_prefix?}`.
  - URL result: POST `/images/import {url, s3_prefix?}`.
  - Failure: "Failed to save image".

---

### D1.2 Video dialog (`views/notes/dialogs/_video.html.erb`, `JS/controllers/video_dialog_controller.js`)

**Shell:** native `<dialog>`, `w-[550px]`. Contents, top to bottom:
1. Header "Insert Video".
2. Media storage selector.
3. Checkbox **"Use Hugo shortcode instead of HTML embed"**. It is **not reset on open**, so it persists for the session.
4. Tabs: **Upload** (`drop`), **URL / File** (`url`), **YouTube Search** (`search`). They use the same arrow-key, wheel and active styling as the image picker. ArrowLeft/Right does not move focus to the Upload tab button; only `url` and `search` are focused (:206-212).

**On connect:**
- GET `/youtube/config` → `{enabled}`.
- GET `/images/config`. This provides `s3_enabled` and `video_extensions`; the default list is `.mp4 .webm .mkv .mov .avi .m4v .ogv`.

**Open:**
- `mediaStorage.refresh()` runs without being awaited.
- All three tabs are reset.
- The YouTube config notice or the search form is shown, depending on `enabled`.
- The Upload tab is activated, the dialog opens, and the dropzone is focused.

**Upload tab:**
- Dropzone "Drag & drop a video file here". It behaves like the image dropzone, with `<input type=file accept="video/*">` (a **single** file).
- The S3 option (upload mode, `show_resize:false`, prefix mode) is shown **at open**, because the upload happens at drop time. The key row appears only when the provider is S3.
- On drop or pick, only the first file is used:
  - Extension check → "%{name}: %{ext} is not an accepted video type. Accepted: %{list}".
  - Re-entrancy guard: a second drop while one upload is in flight is ignored.
  - The preview box shows "Uploading..." (text only, **no progress bar**).
  - `await mediaStorage.ready()`, then POST `/media/upload` multipart `{file, s3_prefix?}` → `{url}`.
  - Success: the preview becomes a play icon, "Upload", and the filename, and **Insert Video** is enabled.
  - Failure: the server error or "Upload failed. Please try again.", and the preview is hidden. If the provider is S3, the storage is refreshed.

**URL / File tab:**
- Input "Video URL or Path", placeholder "YouTube URL or video file path...".
- Hints:
  - YouTube: `https://youtube.com/watch?v=... or https://youtu.be/...`
  - Video file: `/path/to/video.mp4, .webm, .mkv, .mov, .avi`
- Live detection runs on every input:
  - YouTube ID (`JS/lib/url_utils.js:15-29`). The input matches `(youtube.com/watch?v=|youtu.be/|youtube.com/embed/|youtube.com/v/)([A-Za-z0-9_-]{11})` anywhere in the string, or a bare 11-character ID. Shorts URLs, `m.youtube.com/watch?...&v=` (where v is not the first parameter) and `youtube-nocookie` are **not** recognized.
    - Preview: thumbnail `https://img.youtube.com/vi/<id>/mqdefault.jpg` with a red YouTube logo overlay, "YouTube Video", "ID: <id>".
    - If the thumbnail fails to load, the fallback is `/vi/<id>/default.jpg`. If that fails too, the thumbnail is hidden.
  - Video file: the URL **ends with** an allowed extension (case-insensitive, so a query string defeats it). Preview: play icon, "Video File", and the last path segment.
  - Otherwise: warning "Unknown format. Enter a YouTube URL or video file path." and Insert is disabled.
  - Empty input: "Enter a URL to see preview".
- Enter inserts when Insert is enabled.

**YouTube Search tab:**
- When not configured: notice "YouTube API Key Required" (add `youtube_api_key` to `.fed`), a GitHub link, and a Close button.
- Input with Enter or a red **Search** button.
- Status messages:
  - empty query: "Please enter search keywords"
  - while searching: "Searching..."
  - results: "Found N videos - click to insert or use arrow keys"
  - zero results: "No videos found"
  - error: the server error or "Search failed. Please try again." in red
- GET `/youtube/search?q=` returns an **HTML partial** (`views/youtube/_search_results.html.erb`). It is a 2-column grid of buttons, each with:
  - a 16:9 thumbnail (hover overlay with a play icon; the same thumbnail fallback),
  - the title (2-line clamp),
  - the channel.
- Keyboard:
  - ArrowDown in the input focuses result 0.
  - In the results:
    - Right/Left moves ±1.
    - Down/Up moves ±2 (a grid row).
    - Up from the first row returns to the input.
  - Enter inserts and Esc returns to the input. Esc may also close the native dialog.
  - The selected result gets an accent ring.
- **Click or Enter inserts immediately and closes the dialog.**

**Embed formats** (`youtubeEmbedCode`, :66-84; `insertVideo`, :437-469). YouTube with the Hugo box unchecked:
```html
<div class="embed-container">
  <iframe
    src="https://www.youtube.com/embed/VIDEOID"
    title="TITLE"
    frameborder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
    referrerpolicy="strict-origin-when-cross-origin"
    allowfullscreen>
  </iframe>
</div>
```
- `TITLE` is "YouTube video player" for URL insertions and the video title for search insertions. It is **not HTML-escaped**.
- With the Hugo box checked:
  - from search: `{{< youtube id="VIDEOID" title="TITLE" >}}`
  - from a URL, or when the title is the default: `{{< youtube id="VIDEOID" >}}`

Video file (upload or URL):
```html
<video controls class="video-player">
  <source src="URL" type="MIME">
  Your browser does not support the video tag.
</video>
```
- `MIME` is chosen by extension: mp4→video/mp4, webm→video/webm, mkv→video/x-matroska, mov→video/quicktime, avi→video/x-msvideo, m4v→video/x-m4v, ogv→video/ogg. Anything else falls back to video/mp4.
- URL is HTML-escaped.
- An uploaded file's URL is inserted **as the server returned it**. Unlike images, it is **not** rewritten to a note-relative path.

Dispatch `video-dialog:video-selected {embedCode}` → `app#insertVideoEmbed` → `insertBlockContent` (`codemirror_content_insertion.js:154-202`):
- Prefix: `""` at the start of the document or after `\n\n`, `"\n"` after a single `\n`, otherwise `"\n\n"`.
- Suffix: the mirror of the prefix, based on the following text.
- The cursor ends after the inserted block. A slash range replaces the `/query` and consumes surrounding spaces and tabs.

---

### D1.3 Image clipboard helper (`JS/lib/clipboard_image.js`)

- Takes the first clipboard item with `kind==="file"` and a type starting with `image/`.
- If the file name already has an extension it is kept. Otherwise it is renamed `pasted-image-<lastModified|"clip">.<ext>`:
  - ext from MIME: png→.png, jpeg→.jpg, gif→.gif, webp→.webp, bmp→.bmp
  - other image types use the MIME subtype with `+...` stripped (tiff→.tiff, svg+xml→.svg), which the allow-list then rejects with a visible message
  - an empty subtype gives `.bin`

---

### D1.4 Media Library workspace (`views/notes/_library_panel.html.erb`, `JS/controllers/library_controller.js`, `assets/tailwind/components/library.css`)

**Toggle and placement:**
- The sidebar button (`aria-pressed`, highlighted when open) calls `app#toggleLibrary`. Opening the Library **hides the editor panel and the preview panel** and shows the `<section>` in the main area.
- The preview's visibility is stashed on open. On close (`showEditorWorkspace`):
  - split view mode restores the stashed preview state,
  - single view mode keeps the preview hidden.
- Each open calls `resetUsageForLibraryOpen()` and `load()`. Nothing is fetched on connect.
- Esc (workspace handler, `app_controller.js:3164-3198`) closes the preview dialog first, then the context menu, then the Library. It defers to any other open dialog.

**Header:** "Media Library", "Browse and preview images and videos stored with your notes.", and a close X.

**Category toggles:** Images / Videos (`aria-pressed`, accent underline), plus an item count on the right ("%{count} items").

**Load:**
- GET `/library` → `{images:[...], videos:[...]}`. Items have the shape `{name, path, type:"image"|"video", size, mtime, provider?:"s3", url?}`.
- A generation counter discards stale responses.
- States: "Loading media...", error "Failed to load the media Library.", empty "No media found."
- Client-side validation (`isValidItem`, :582-598):
  - The path must have at least 2 segments, with no `.`, `..`, empty segments or `\`.
  - Local items: `path[0]` is `images` (images) or `videos` (videos), and `provider` is unset.
  - S3 items: `path[0]` is `frankmd`, and the URL is http(s) with no credentials.

**Grid:** `repeat(auto-fill, minmax(min(100%,13rem),1fr))` with a 1 rem gap. Each card contains:
- Preview button, 10 rem tall:
  - image: `<img loading=lazy>`
  - video: a muted `<video preload=metadata>` with a ▶ badge
  - The source is the S3 `url`, or `/notes/<encodedPath>` for local items.
- Name (truncated).
- "<size> · <date>". Size uses B/KB/MB/GB/TB (1 decimal below 10, otherwise 0). The date uses `toLocaleString()`.
- Usage label.
- Buttons: **Rename** (S3 items only), **Delete** (red outline), **Insert** (accent).

**Usage tracking:**
- An IntersectionObserver watches the cards. The first visible local card triggers a single GET `/library/usage` → `{usage_counts:{path:int}}`. The response is validated: integers that are 0 or greater.
- Label states:
  - "Checking usage..."
  - "Used in %{count} notes": when the count is above 0 this is a dotted-underline button that opens the usage dialog
  - "Used in 0 notes"
  - "Usage unknown": always for S3 items and whenever the request fails
- **Usage dialog** ("Notes associated"): a custom overlay (`role=dialog`, focus trap, Esc, backdrop click).
  - It lazily loads GET `/library/usage?path=` → `{usage_notes:[paths]}` and caches the result per path.
  - Each listed note path is a button, validated as a relative `.md` path. Clicking it dispatches `library:open-note`, which opens the note and reveals it in the tree.
  - Empty: "No matching notes were found."

**Preview dialog:**
- Click a card preview to open it. It is a custom overlay, `max-w-5xl`, with a focus trap and backdrop-click close. Focus returns to the trigger on close.
- Header: the name, then "usage · metadata". Metadata is "size · date" and, for images, the reduced aspect ratio (e.g. `16:9`, computed via GCD once the image loads) and the uppercased extension (e.g. `PNG`).
- Body: the image, or `<video controls autoplay muted>`, at most 65vh, `object-fit: contain`.
- Footer:
  - **Delete**
  - **Copy relative path**: copies the S3 URL, or the library path such as `images/foo.png`. This is the raw path, not note-relative. Status "Relative path copied." or "Could not copy the relative path."
  - **Insert**

**Insert** (`library:insert-media` → `app#insertLibraryMedia`, `app_controller.js:2031-2072`):
- Rejected with an alert when:
  - no note is open: "Open a Markdown note before inserting media."
  - the open note isn't Markdown: "Media can only be inserted into an open Markdown note."
  - the item is invalid or insertion fails: "Could not insert media into the note."
- URL:
  - local: note-relative and encoded (see D1.1)
  - S3: the `href` with `(` and `)` percent-encoded
- Image: `![<escapeMarkdownAlt(name)>](<url>)` via `insertImage`, using the single-newline rule.
- Video: block insertion of
  ```html
  <video controls class="video-player">
    <source src="URL" type="MIME">
  </video>
  ```
  The `type` attribute is omitted when the extension is unknown. There is no fallback text, unlike the video dialog.
- After inserting: return to the editor workspace, focus the editor, and close the preview.

**Delete:**
- `appConfirm("Delete %{name}? Existing note references will stop working.")` with a destructive button.
- DELETE `/library/file/<encodedPath>`.
- The item is removed locally, the preview closes, and the status shows "Media file deleted."

**Rename (S3 only):**
- `appTextPrompt("New media filename", default=current name)`.
- PATCH `/library/file/<path> {name}` → the updated item is merged into the list. Status "Media file renamed. Existing note references may need updating."
- Notes that reference the file are **not** updated.

---

### D1.5 Media storage provider (`JS/controllers/media_storage_controller.js`, `media_storage_menu_controller.js`, `_media_storage.html.erb`, `_media_storage_menu.html.erb`, `dialogs/_media_storage_warning.html.erb`)

**Shared state:** one controller on the root `notes/index` element holds:
- `provider` ("local" | "s3"), initialized from config `media_storage_provider`
- `s3Available`, from feature `s3_upload`
- `fallback`, set when the server auto-switched from S3 to Local

**Controls:**
- In-dialog `<select>` (image and video dialogs): "Storage for new media", with Local and S3. The S3 option is disabled when S3 isn't available.
- Settings-panel dropdown: a popover menu with `menuitemradio` buttons and a checkmark on the current choice. When S3 is unavailable its button is disabled with the tooltip "This provider is not configured.". The trigger is disabled while a save is pending.
- All controls stay in sync through the window event `media-storage:changed {provider}`.

**Change:**
- PATCH `/config {media_storage_provider}`.
- While pending, every select is disabled.
- Success: apply the `settings.media_storage_provider` and `features.s3_upload` values from the response and fire `frankmd:config-file-modified` on window.
- Failure: inline error "Could not save the storage provider. The previous selection is still active."
- Concurrent calls wait for the pending one.

**`refresh()`:** GET `/config` (load failure: "Could not load the storage provider.").

**`ready()`:** awaits a pending save. Upload flows call it before posting.

**"s3Only" targets:** for example the AI destination-folder field. They are visible only when the provider is `s3`.

**Fallback warning:**
- Triggered by `data-media-storage-fallback-value=true` on page load, or by a response with `media_storage_fallback` while the provider is local.
- Shows a modal `alertdialog` titled "Upload storage changed": "S3 is no longer available. Your upload provider has been switched to Local." with an **OK** button.
- **It auto-dismisses after 6 s.** It is shown on the next tick so that it stacks above a dialog that is just opening.

---

### D1.6 Paste and drag & drop into the editor

**Paste** (`JS/lib/codemirror_extensions.js:359-372`, `codemirror_controller.js:267`, `app_controller.js:1453-1457`):
1. A CodeMirror `paste` DOM handler checks for an image on the clipboard. Plain text, or no image file, falls through to normal paste.
2. If an image is found, it calls `preventDefault`, which suppresses accompanying `text/html` (as when copying an image from a web page). Then it dispatches `codemirror:image-pasted {file}` → `app#onImagePaste` → `imagePicker.openWithFile(file)`.
3. `openWithFile`:
   - runs the normal `open()`
   - shows the loading row "Processing image..."
   - ingests the file into the **Upload** tab and auto-selects tile 0, which shows the S3/resize options with the default key `frankmd/YYYY/MM/pasted-image-<ts>.png` and an alt such as `pasted image 1730000000000`
   - hides the loading row (in a `finally`)
4. If the type is rejected, the rejection message shows and nothing is selected.
5. The user then edits alt or link and clicks **Insert Image**, which follows the normal upload flow (POST `/images/upload`). The result is a note-relative `![alt](../images/<ts>_<hex>_pasted-image-....png)` for Local, or the S3 URL.

**Drag & drop of files onto the editor:** FrankMD has **no** handler for this. No `drop` or `dragover` handlers are registered on the editor; the only CodeMirror DOM handler is the image paste above. Dropped OS files fall through to CodeMirror 6's built-in drop behavior: it reads dropped files as **text** and inserts their contents. No image upload happens and no placeholder appears. **A rewrite should decide on purpose whether to add drop-to-upload (routing through the picker the way paste does) or to block file drops.**

File drag & drop is supported only inside the dialog dropzones:
- Image picker Upload tab: multiple files.
- Video dialog Upload tab: a single file.

Both use the same highlight helper.

**There is no upload progress UI anywhere.** The only indicators are:
- the image picker loading row (spinner plus "Processing image...") with Insert disabled,
- the video dialog's "Uploading..." text,
- the AI tab's generating spinner.

There are no inline editor placeholders and no percentage.

**Explorer drag & drop** (`JS/controllers/drag_drop_controller.js`, wired in `_sidebar.html.erb:16`) covers **only** moving notes and folders within the tree, and is covered elsewhere:
- Drops are accepted only when `this.draggedItem` is set, so OS file drops onto the tree are ignored.
- A single valid folder row gets the `drop-highlight` class. `drop-highlight-root` appears when moving to the root.
- A multi-selection is dragged together.
- Moving calls POST `/notes|folders/<path>/rename`.
- Autosave is prepared before moving the currently open file.

---

### D1.7 Notable quirks to preserve or fix deliberately

- The Upload, Folder and paste flows always show the Resize select, even for Local storage. Web, Google, Pinterest and server-Library images show it only when "Save a copy" is ticked.
- Alt text from the picker is not escaped for remote or S3 URLs. Only `images/` URLs are rebuilt with escaping. A `)` in a remote URL is not encoded on the picker path; the Library path does encode it.
- Video titles in YouTube iframe and Hugo embeds are not escaped, so a `"` in a title breaks the attribute.
- The legacy `LocalImageSource.renderGrid` in `lib/image_sources/local_images.js` is unused now; the Library tab renders server-side through Turbo Stream. `LocalImageSource.load` is also unused.
- `.image-grid-delete` CSS (hover-revealed trash) applies only to the legacy renderer. The Turbo partial uses an always-visible corner trash button.

---

## 8b. Other dialogs (table, code, emoji, text-format, AI grammar, log viewer, help, jump-to-line, file operations)  
<sub>(sub-section labels below keep the source numbering, e.g. D2.x)</sub>

Paths: JS files are under `app/javascript/`, ERB under `app/views/`, CSS under `app/assets/tailwind/components/`. English strings come from `config/locales/en.yml` and are quoted exactly. All strings go through i18n (`window.t`) in 7 locales (en, es, he, ja, ko, pt-BR, pt-PT).

---

### D2.0 Shared dialog infrastructure

**Element and mounting.** Every dialog is a native `<dialog>` opened with `showModal()`, so it is modal, sits in the top layer and traps focus. All dialogs are rendered once at page load. `notes/index.html.erb:78` renders `notes/dialogs/file_operations`, and `:82` renders `notes/dialogs/_dialogs.html.erb`, which includes, in this order: table_editor, image_picker, image_viewer, find_replace, jump_to_line, help, file_finder, content_search, code, video, ai_diff, text_format_menu, emoji_picker, log_viewer, recovery_diff, undo_created_note, custom_theme_editor.
- `dialogs/_rename.html.erb` (an app-controller rename dialog with `submit->app#submitRename`) is **dead code**: nothing renders it. The live rename dialog is the one in `_file_operations.html.erb`.

**Base styling** (`dialog.css`):
- `dialog { position: fixed; inset: 0; margin: auto; }` centers the dialog natively.
- Background `--theme-bg-secondary`, text `--theme-text-primary`, 1px border `--theme-border`.
- `dialog[open] { display: flex; flex-direction: column }`.
- `dialog::backdrop { background: rgba(0,0,0,.6); backdrop-filter: blur(2px) }`. Individual dialogs also add Tailwind `backdrop:bg-black/50`.
- `dialog.positioned { inset: auto; margin: 0 }` is for dialogs placed at a click point (`app_controller.positionDialogNearPoint`).
- `.confirm-dialog__filename` styles an inline monospace "chip" (1px border, radius .25rem, primary background, `overflow-wrap: anywhere`, .95em). The undo-created-note dialog uses it.
- Typical classes: `p-0 rounded-lg shadow-xl`. Each dialog sets its own width (see below).

**Escape.** The browser's native Escape (`cancel` event) closes every dialog. The global shortcut handler `closeDialogs` (Escape, `preventDefault:false`) returns early whenever any `dialog[open]` exists (`app_controller.js` `closeActiveWorkspaceOrDialogs`), so it never fights the native close. `closeAllDialogs()` only hides the explorer context menu and background menu, and closes Help.

**Backdrop click closes** only these dialogs: Help (`help_controller.js:23-28`), Log viewer (`log_viewer_controller.js:213`), and the three file-operations dialogs: rename, new note, new folder (`file_operations_controller.js:95-109`). All other dialogs (table, code, emoji, AI, jump-to-line, undo) ignore backdrop clicks.

**Shared partials:**
- `shared/_dialog_header`: a flex row with `<h3 class="text-sm font-semibold">` (optional `title_id`) and a close button. Default margin `mb-3`.
- `shared/_dialog_close_button`: an icon-only button with an X SVG (path `M6 18L18 6M6 6l12 12`), default `w-4 h-4`, `p-1 rounded`, hover `--theme-bg-hover`, muted colour, optional `aria-label`.
- `shared/_button_primary`: `px-3 py-1.5 text-sm rounded-md`, accent background and text, `hover:accent-hover`, `disabled:opacity-50 cursor-not-allowed`.
- `shared/_button_secondary`: same size, transparent with hover background and secondary text. Supports `type`, `value` and `autofocus`.

**Slash-command integration.** The table, emoji, image and video dialogs carry `data-slash-command-action="<action>"` and `close->app#onSlashCommandDialogClose`. When a dialog was opened from a `/table` or `/emoji` slash command, app_controller stores `pendingSlashInsertionRange = {action, from, to, query}`.
- On insert, the content replaces the `/query` text, but only if the document text at `[from,to)` still equals `query`. If it no longer matches, the insert is **aborted** (`getPendingSlashInsertionRange` returns `false`; `app_controller.js:1362-1374`).
- Closing the dialog clears the pending range.

#### D2.0.1 `lib/app_prompt.js`: custom alert, confirm and prompt

This replaces `window.alert`, `window.confirm` and `window.prompt` across the app.

- **API:**
  - `appAlert(msg)` returns `Promise<undefined>`.
  - `appConfirm(msg, {acceptLabel, cancelLabel, destructive})` returns `Promise<boolean>`.
  - `appTextPrompt(msg, {defaultValue, …})` returns `Promise<string|false>`.
- **Queue.** Prompts are queued FIFO and only one shows at a time (`promptQueue`, `activePrompt`). The element focused before the *first* prompt is remembered and gets focus back once the queue empties (`app_prompt.js:30-45,122`).
- **DOM.** Each prompt is a freshly created `<dialog role="alertdialog">`, 360px wide (max 90vw), labelled by `#app-prompt-title` and described by `#app-prompt-message`. It is removed after use. Contents:
  - Title `<h2>`: "Confirm" (`common.confirm`) for confirm and prompt, "Notice" (`common.alert`) for alert.
  - Message `<p>`: `whitespace-pre-wrap break-words`, plain text only.
  - Optional text `<input>` with `defaultValue`.
  - Right-aligned buttons.
- **Buttons:**
  - Confirm and prompt show Cancel (`common.cancel`) followed by an accept button (default label "Confirm"; alerts use "OK").
  - The accept button is accent-coloured, or `--theme-error` coloured when `destructive`.
- **Initial focus:** the input if present; otherwise the dialog itself for a destructive confirm (so Enter does not trigger delete); otherwise the **Cancel** button for a confirm; otherwise OK for an alert (`app_prompt.js:114-117`).
- **Keys.** Enter inside the input resolves with the input value. Escape (`cancel`) is intercepted with `preventDefault` and resolves `false`; an alert therefore also resolves `false`. There is no backdrop-click handling.

#### D2.0.2 `lib/floating_dropdown.js`

This is a popover dropdown helper used by the theme picker, locale picker and media-storage menu (not by any dialog in this section).
- The menu gets `popover="auto"` and the class `floating-dropdown`. The trigger gets `aria-haspopup=menu`, `aria-expanded` and `popovertarget`.
- **Positioning:** `position: fixed`, 4px gap from the trigger, 8px viewport padding, max-height 320px. The menu flips upward when there is less room below than its height and more room above. It is right-aligned to the trigger in LTR and left-aligned in RTL, and clamped to the visual viewport.
- **Closes when:** you click outside, focus moves outside, another dropdown opens (`frankmd:dropdown-open` event), any scroll moves the anchor, the visualViewport scrolls, or the containing `<dialog>` closes. It repositions on resize. If the anchor scrolls out of view, it closes.
- **Keyboard:**
  - ArrowDown/ArrowUp/Home/End on the trigger open the menu. ArrowUp from closed focuses the last item.
  - Inside the menu these keys cycle (wrapping) over `button:not(:disabled)`.
  - Tab closes the menu and restores focus to the trigger. Escape closes it, stops propagation and restores focus.
  - On open, focus goes to the item with `aria-checked="true"`, or else the first item.

#### D2.0.3 `lib/menu_bounds.js`

This is the viewport clamp for context menus (issue #141), used by the text-format menu and the explorer context menu.
- `menuMaxHeight(rect, vh)` returns `max(96, floor(vh - rect.top - 8))` px.
- `clampMenuToViewport(menu)` resets `transform`, sets `maxHeight`, and if the menu overflows left or right by more than the 8px padding, applies `translateX(shift)`.
- `toggleMenu(menu, forceHidden)` toggles `hidden` and clamps when the menu is shown.

---

### D2.1 Table editor

**Files:** `controllers/table_editor_controller.js`, `lib/table_utils.js`, `lib/markdown_utils.js` (`findTableAtPosition`), `views/notes/dialogs/_table_editor.html.erb`, `table-editor.css`, and glue in `app_controller.js:1180-1210,1291-1406`.

**Entry points:**
- Workspace toolbar action `table`, handler `openTableEditor`, tooltip "Insert/Edit Table". It is unpinned by default and lives in the overflow menu (`helpers/workspace_helper.rb:11`).
- The `/table` slash command.
- There is no keyboard shortcut.

**"Cursor in table" hint.** In markdown files, every document change and selection change runs a check debounced by 200ms. If the cursor is inside a table, a badge appears at the bottom-right of the editor reading "Cursor in table - click to edit" (`editor.cursor_in_table`).
- The badge has `pointer-events: none` (`editor.css:311`), so it is **not clickable** despite its text.
- It is hidden immediately when a new file loads.
- A typing event fires both docChanged and selectionSet; only one check is scheduled (`app_controller.js:1165`).

**Table detection** (`markdown_utils.js:13-83`):
1. If the cursor's current line, after `trimStart()`, does not start with `|`, there is no table.
2. Otherwise, extend up and down over contiguous lines whose `trim()` starts with `|`.
3. Require at least 2 lines.
4. Require at least one line matching the separator regex `^\|[\s:]*-+[\s:]*(\|[\s:]*-+[\s:]*)*\|?\s*$`.
5. Return `{startLine, endLine, startPos, endPos (exclusive of the final newline), lines}`.

Tables without a leading pipe are therefore *not* detected.

**Opening** (`openTableEditor`). If the cursor is in a table, the editor opens in **edit mode** with `existingTable = lines.join("\n")` and the start/end positions. If it was opened by `/table` typed *inside* an existing table, the `/table` query text is removed from `existingTable`, along with one adjacent space or tab (rules at `app_controller.js:1310-1323`). The open request is sent via `window` event `frankmd:open-table-editor {existingTable, startPos, endPos}`.

**Initial data:**
- New table: 3 columns × 3 rows. Row 0 is `["Header 1","Header 2","Header 3"]` (`dialogs.table_editor.header_n`) and the other two rows are empty.
- Edit mode: `parseMarkdownTable(existingTable)`. If that yields 0 rows, the data falls back to a single header row of 3 cells.

**Parsing** (`table_utils.js:9-33`):
- Skip blank lines.
- Skip separator rows matching `/^\|[\s\-:]+\|$/` or `/^\|(\s*:?-+:?\s*\|)+$/`.
- Split on `|`, drop the first and last elements, and trim each cell.
- **No escaped-pipe handling.** `\|` inside a cell splits the cell.
- **Alignment colons are discarded**, so editing a table loses left/center/right alignment.
- A row without a trailing pipe loses its last cell, because of `slice(1,-1)`.
- Rows can have uneven lengths.

**Generating** (`generateMarkdownTable`, `table_utils.js:40-78`):
- Column count is the maximum row length. Shorter rows are padded with `""`.
- Column width is `max(3, max cell .length)`. JS `.length` counts UTF-16 code units, not display width, so CJK text and emoji misalign.
- Header line: `"| " + cells.padEnd(w).join(" | ") + " |"`.
- Separator line: `"| " + "-".repeat(w).join(" | ") + " |"`. The separator is always plain dashes, with no alignment colons.
- Data rows are formatted like the header.
- Lines are joined with `\n`, with no trailing newline.
- Example output for a new table:

  ```
  | Header 1 | Header 2 | Header 3 |
  | -------- | -------- | -------- |
  |          |          |          |
  |          |          |          |
  ```
- An empty table (0 rows) inserts nothing and just closes the dialog.

**Dialog UI** (`max-w-3xl`, `p-4`, `tabindex=-1`; on open it gets focus with `preventScroll`):
- **Header row:** the title "Table Editor" plus two dimension steppers, labelled "Column" and "Row" (uppercase, muted, 0.75rem).
  - Each stepper is a box 6.25rem × 24px holding a `type=number min=1` input (centered, spin buttons hidden) and two buttons, `-` and `+`. The column buttons have titles "Remove Column" / "Add Column"; the row buttons have "Remove Row" / "Add Row".
  - Input `change` sets the count: `parseInt`, NaN falls back to the current count, minimum 1.
  - Adding columns appends `"Header N"` (N = 1-based column index) to row 0 and `""` to the other rows. Removing columns truncates from the right.
  - Adding rows appends empty rows. Removing rows truncates from the bottom. The minimum is 1 row and 1 column.
  - The inputs re-sync after every render.
- **Grid:** a scrollable box (`max-h-80`, bordered, rounded) holding `<table class="table-editor-grid">` (min-width 400px). Each cell `<td>` (min-width 100px, 1px border `#d4d4d8` light / `#52525b` dark) contains a borderless `<input type=text>`.
  - Header-row cells are bold on a tertiary background (`#f4f4f5` light / `#3f3f46` dark) with placeholder "Header".
  - A focused input gets a background of `#eff6ff` (light) or `#1e3a5f` (dark) and a 1px accent ring.
  - Each `input` event writes the value straight into `tableData[r][c]`.
  - The grid is re-rendered via `innerHTML` after any structural operation, so focus is lost.
- **Keyboard:** Tab moves between cells natively. Enter does nothing; there is no form submit and no Ctrl+Enter.
- **Cell context menu** (right-click on a cell or its input): a fixed-position menu (min-width 140px, z-60) placed at the mouse position and clamped 10px inside the viewport.
  - **Column** section: "Move Left" (disabled in column 0), "Move Right" (disabled in the last column), "Delete Column" (error-coloured, disabled when only 1 column).
  - Divider.
  - **Row** section: "Move Up" (disabled for rows ≤1, since the header row 0 and the first data row cannot move up), "Move Down" (disabled for row 0 and the last row), "Delete Row" (error-coloured, disabled for row 0 or when only 1 row).
  - Disabled items show at `opacity-50`. The header row can never be moved or deleted.
  - Move and delete swap or splice the data and re-render.
  - The menu closes on any document click outside it (registered on the next tick) or after an action.
  - **There is no drag-and-drop** for rows or columns, and no alignment control.
- **Footer:** "Cancel" (secondary) and "Insert Table" (primary). The primary label is the same in edit mode.

**Insertion** (`frankmd:insert-table` event, then `handleTableInsert`, then `insertBlockContent`; `codemirror_content_insertion.js:43-91`). This is skipped entirely if the current file is not markdown.
- **Edit mode:** replace `[startPos,endPos)` with the markdown and put the cursor at the end of the inserted table. No newlines are added.
- **New table:** insert at the cursor, or over the slash range (which also swallows adjacent spaces and tabs on both sides).
  - Prefix: `""` if the text before is empty or ends with `\n\n`; `"\n"` if it ends with one `\n`; otherwise `"\n\n"`.
  - Suffix: same rules, mirrored, using the text after.
  - The cursor ends after the suffix.
- After inserting, the editor is focused and the change pipeline runs (preview, autosave).

---

### D2.2 Code block dialog

**Files:** `controllers/code_dialog_controller.js`, `dialogs/_code.html.erb`, glue in `app_controller.openCodeEditor` and `onCodeInsert` (`:2409-2440`).

**Entry point:** the toolbar action `code` ("Insert/Edit Code Block"), unpinned. There is no shortcut. (The `/code-block` slash command inserts a fence directly without this dialog; it is covered elsewhere.) There is no markdown-file check.

**Existing-block detection** (`markdown_utils.findCodeBlockAtPosition`):
- Regex `/```(\w*)\n([\s\S]*?)```/g` scanned from the start of the document. The cursor must satisfy `start ≤ pos ≤ end`, inclusive of the fences.
- Only backtick fences count; `~~~` is not supported.
- The language must be `\w*`. A fence like ```` ```c++ ```` or ```` ```objective-c ```` does not match, so that pairing breaks.
- `content` includes the trailing newline before the closing fence.
- When a block is found, the dialog opens prefilled with `{language, content, editMode:true, startPos, endPos}`.

**Dialog** (500px wide):
- Header "Code Block" with a close X (`mb-4`).
- **Language input:** label "Language (Tab to autocomplete)", monospace, placeholder "javascript, python, ruby...", autocomplete and spellcheck off. It is **focused on open**.
- **Suggestions dropdown:** absolutely positioned under the input, `max-h-48`, scrollable. It holds up to **6** buttons.
  - Filter: lowercased, trimmed input; keep languages where `lang.includes(value)`, in list order, with no ranking.
  - The dropdown is hidden when the input is empty or when the first match equals the input exactly.
- **Language list (44, exact order):** javascript, typescript, python, ruby, go, rust, java, c, cpp, csharp, php, swift, kotlin, scala, haskell, elixir, erlang, clojure, lua, perl, html, css, scss, sass, less, json, yaml, toml, xml, markdown, sql, graphql, bash, shell, powershell, dockerfile, makefile, nginx, apache, vim, regex, diff, git, plaintext.
- **Keys in the language input:**
  - Tab, while suggestions are visible: take the first suggestion and hide the list. Focus stays in the input. Otherwise Tab moves focus normally.
  - ArrowDown: focus the first suggestion.
  - Escape: hide the suggestions. It is not prevented, so the native dialog also closes.
  - Ctrl/Cmd+Enter: insert.
- **Keys in the suggestion buttons:** ArrowDown/ArrowUp move between buttons; ArrowUp on the first returns to the input; Escape hides the list and refocuses the input. Clicking a suggestion sets the language and focuses the code textarea.
- **Code textarea:** label "Code (optional) — Tab to indent, Ctrl+Enter to insert", plus a "Use tabs" checkbox (unchecked by default and not persisted). Height `h-48`, monospace, not resizable, placeholder "Paste or type your code here...".
  - Ctrl/Cmd+Enter inserts.
  - **Tab** inserts the indent unit (`"\t"` when "Use tabs" is checked, else **2 spaces**) at the cursor.
  - **Shift+Tab** with no selection removes one indent unit from the start of the current line. Failing that, it removes one tab or up to unit-length leading spaces.
  - With a selection, Tab and Shift+Tab indent or outdent every touched line and reselect the whole affected block.
  - Each of these dispatches an `input` event.
- **Footer:** "Cancel" and "Insert Code".

**Insert:**
1. `language = input.trim()`; the case is preserved as typed.
2. If the language is non-empty, not in the list (compared lowercased), and not "close" (some list entry starts with it, or Levenshtein distance ≤2), show `appConfirm('"<lang>" is not a recognized language. Insert anyway?')`. On cancel, the dialog stays open.
3. Build the fence:
   - With content: ```` "```" + language + "\n" + content + (content.endsWith("\n") ? "" : "\n") + "```" ````.
   - Empty content: ```` "```" + language + "\n\n```" ```` (one blank line inside).
4. Dispatch `code-dialog:insert` and close the dialog.
5. `insertCodeBlock` calls `insertBlockContent` with the same blank-line rules as tables. Edit mode replaces `[startPos,endPos)`. The **cursor lands at the start of the first content line**: `cursorOffset = 3 + language.length + 1`.

---

### D2.3 Emoji / Emoticons / Icons picker

**Files:** `controllers/emoji_picker_controller.js` (it also holds the emoticon data), `lib/emoji_data.js`, `lib/icon_data.js`, `lib/emoji_i18n.js`, `public/emoji-data/{en,es,ja,ko,pt}.json`, `dialogs/_emoji_picker.html.erb`, rendering in `lib/marked_extensions.js:78-131`.

**Entry points:**
- Shortcut **Ctrl/Cmd+Shift+E**.
- The toolbar action `emoji` ("Insert Emoji (Ctrl+Shift+E)").
- The `/emoji` slash command.
- It only opens for markdown files (`app_controller.openEmojiPicker`).

**Data:**
- **Emoji tab:** `EMOJI_DATA` has **675** entries of the form `[shortcode, unicodeChar, "space separated keywords"]`, with no duplicate shortcodes.
  - Source comment groups and counts: Smileys & Emotion 104, Gestures & Body 46, Hearts & Love 22, Symbols & Objects 67, Nature & Animals 76, Plants & Flowers 22, Activities & Objects 125, Symbols 73, Travel & Places 60, Flags 7, Misc 73.
  - The **grid shows no category headers or category navigation**; it is one flat list in this order.
  - The same table drives preview rendering of `:shortcode:` (`getEmojiMap`).
- **Emoticons tab:** `EMOTICON_DATA` has **125** kaomoji of the form `[name, text, keywords]`.
  - Groups: Happy 13, Love 10, Sad 10, Angry 10, Surprised 8, Confused 8, Cute/Kawaii 12, Actions 13, Fighting 6, Food 5, Music 4, Weather 4, Special/Misc 22.
  - Examples: `happy (◕‿◕)`, `table_flip (╯°□°)╯︵ ┻━┻`, `shrug ¯\_(ツ)_/¯`, `lenny ( ͡° ͜ʖ ͡°)`.
- **Icons tab:** `ICON_DATA` has **201** curated **Phosphor Icons** (regular weight, MIT) of the form `[name, svgPathD, "0 0 256 256", keywords]`.
  - They are generated by `scripts/extract_phosphor_icons.mjs`.
  - They are used for inline icon tokens `:ph-<name>:`, which the preview renders as `<svg … width="1em" height="1em" class="frankmd-inline-icon" fill="currentColor">`.
  - Names include check, x, warning, info, star, heart, folder, file, pencil, trash, calendar, clock, link, lock, eye, arrows/carets, gear, user, bell, chat, envelope, image, camera, music, git-*, database, rocket, trophy, and others.
- **i18n search data:** `public/emoji-data/<locale>.json` files are emojibase compact datasets with **1949 entries each**. Keys: `hexcode, label, unicode, tags, emoticon, group, order, skins`; only `unicode`/`emoji`, `label` and `tags` are used.
  - Locale map: en→en, pt-BR and pt-PT→pt, es→es, ja→ja, ko→ko, he→**en** (no Hebrew).
  - The file is fetched from `/emoji-data/<loc>.json` on controller connect (preload) and again on open if it is still missing. It falls back to en.
  - The index is `Map(unicode → {label, tags, searchTerms: (label + " " + tags.join(" ")).toLowerCase()})`.

**Dialog** (500px wide, max-h 70vh, flex column):
- **Header:** a tablist with three tabs, "Emoji", "Emoticons" and "Icons". The active tab has an accent background and accent text; inactive tabs are muted with a hover background. A close X sits on the right. Below is a search input with placeholder "Search emojis..." (the same text on every tab), which is focused on open.
- **Grid:** CSS grid, `gap-1`. Columns: **10** for emoji, **5** for emoticons, **8** for icons. Cells:
  - Emoji: 40×40 buttons, `text-2xl`, title `:shortcode:`.
  - Emoticons: `px-2 py-2 text-sm truncate`, title = name.
  - Icons: 40×40, SVG at 1.25em, title `:ph-name:`.
  - The selected item gets a hover background plus `ring-2 ring-accent ring-offset-1`.
  - When the filtered list is empty, the grid shows "No matches found" (`status.no_matches`).
- **Preview footer** (tertiary background, height 3rem, centered):
  - Emoji: the glyph at `text-4xl` plus a code chip `:shortcode:`.
  - Icons: the SVG at 2em plus `:ph-name:`.
  - Emoticons: the text in `text-lg font-mono` plus the muted name.
- **Open state:** every open resets to the Emoji tab, an empty query and index 0.

**Search:**
- Query is `trim().toLowerCase()` and split on whitespace. **All terms must be substrings** (AND).
- Emoji tab: match against `"shortcode keywords"`. If that fails, match against the translated `searchTerms` for that unicode character.
- Emoticons and icons: English only, matching `"name keywords"`.
- Switching tabs re-applies the current query.

**Keyboard and mouse:**
- These keys are handled in the search input (`keydown->onKeydown`):
  - ←/→ move ±1 and wrap across the whole list.
  - ↓/↑ move ±columns. Moving down past the end wraps to the same column in the first row. Moving up past the start wraps to the same column in the last row (clamped to the last item).
  - Enter inserts the selected item.
  - Tab moves focus normally. Escape uses the native close.
- Every arrow re-renders the grid and smooth-scrolls the selected item into view (`block: nearest`).
- Mouse hover changes the selection, without a re-render.
- Click inserts the clicked item.
- On a tab button, ←/→ cycle tabs (wrapping) and move focus.
- The mouse wheel over the tablist switches tabs (down/right = next) and calls `preventDefault`.

**Inserted text:**
- Emoji: **`:shortcode:`**, not the unicode character. For example, it inserts `:smile:`, and the preview renders 😄.
- Icons: `:ph-<name>:`.
- Emoticons: the raw kaomoji text.

Insertion goes through `emoji-picker:selected {text, type}`, then `onEmojiSelected`, then `insertInlineContent`, which **replaces the current selection** (or the slash range) with no added newlines. The cursor goes after the inserted text, then come focus, autosave scheduling and a preview update.

---

### D2.4 Link insertion (there is no dedicated link dialog)

Links are inserted as text templates. There is no URL prompt.
- **Ctrl/Cmd+K** (CodeMirror keymap, `lib/codemirror_extensions.js:187-211`):
  - With a selection: replace it with `[<sel>](url)` and **select the word `url`**.
  - Without a selection: insert `[](url)` with the cursor between the brackets.
- **Text-format menu** "Link" (hotkey L): `prefix "["`, `suffix "](url)"`; afterwards `url` is selected (`text_format_controller.js:453-456`).
- **Wikilinks** `[[Note]]` have their own autocomplete; see the editor section.

---

### D2.5 Text format context menu

**Files:** `controllers/text_format_controller.js`, `dialogs/_text_format_menu.html.erb`.

**Opening:**
- Right-click in the editor with a non-blank selection, in markdown files only. With no selection, the native context menu appears.
- **Ctrl/Cmd+M**: opens the menu at `coordsAtPos(selectionEnd)` plus 4px down, or at the editor center as a fallback. It only opens when there is a non-blank selection.

**Menu** (`.frankmd-menu`, fixed, min-width 180px, z-50, `tabindex=-1`, focused when it opens):
- Positioning: placed at the point, then `clampMenuToViewport`. In the next frame: if it overflows the right edge, shift it left; if it overflows the bottom edge, flip it above the point; finally keep it ≥10px from the left and top edges.
- Seven items, in order. Each label has its hotkey letter underlined (first case-insensitive occurrence), and the hotkey is shown at the right:

  | id | Label | Hotkey | Wrap |
  |---|---|---|---|
  | bold | Bold | B | `**…**` |
  | italic | Italic | I | `*…*` |
  | strikethrough | Strikethrough | S | `~~…~~` |
  | highlight | Highlight | H | `==…==` |
  | subscript | Subscript | U | `~…~` |
  | superscript | Superscript | P | `^…^` |
  | link | Link | L | `[…](url)` |

- **Keyboard:** ↑/↓ cycle with wrap; Enter applies; pressing a letter applies its format; Escape closes, dispatches `closed` and refocuses the editor. Hover highlights an item; click applies it. Mousedown outside closes the menu.

**Apply rules:**
- **Symmetric formats toggle off.** If the selection itself starts and ends with the markers, they are stripped and the inner text stays selected. If the markers sit just outside the selection, they are removed and the text stays selected.
- Otherwise the selection is wrapped and the cursor goes after the wrapped text. Link is the exception: it selects `url`.
- After applying: autosave is scheduled and the preview updates.

---

### D2.6 AI grammar check

**Files:** `controllers/ai_grammar_controller.js`, `dialogs/_ai_diff.html.erb`, `ai-diff.css`, `lib/diff_utils.js`, glue in `app_controller.js:2472-2544`; server side in `AiController#fix_grammar` and `AiService::GRAMMAR_PROMPT`.

**Entry point:** the toolbar action `ai` ("AI Grammar Check"), unpinned; these buttons have the target `aiButton`. There is no shortcut.

**Endpoints:** `GET /ai/config` (on connect) and `POST /ai/fix_grammar`.

**Flow:**
1. On connect, call `GET /ai/config` and read `{enabled, provider, model}`. This happens once, so changes need a page reload.
2. `openAiDialog`:
   - No open file: `appAlert("No file open")`.
   - Editor text is blank: `appAlert("No text to check")`.
   - If an autosave is pending, **save now and await it**, because the server reads the file from disk.
3. `open(path)`:
   - **AI not enabled:** show the dialog with the *configuration notice* panel (see below) and stop.
   - Otherwise dispatch `processing-started`. App_controller then makes the editor **read-only** and replaces every AI button's content with a spinner plus "Processing..." (`disabled`, `aria-busy=true`).
   - Show the **processing overlay**: a fixed, full-screen layer, z-40, `bg-black/30 backdrop-blur-sm`, with a centered card. The card holds a 48px spinning accent SVG, "Processing with", the provider line (`"<provider>: <model>"`, or "AI" when unknown), and "Press ESC to cancel".
   - Install a document keydown listener: Escape aborts the request through an AbortController. Cancelling closes the overlay silently, with no alert.
4. `POST /ai/fix_grammar {path}` (JSON). **The whole saved note file is sent, never just the selection.** The server reads `Note.find(path).content` and runs `RubyLLM.chat(model, provider)` with these system instructions:

   > You are a grammar and spelling corrector. Fix ONLY: Grammar errors, Spelling mistakes, Typos, Punctuation errors. DO NOT change: Facts, opinions, or meaning; Writing style or tone; Markdown formatting (headers, links, code blocks, lists, etc.); Technical terms or proper nouns; Code blocks or inline code. Return ONLY the corrected text with no explanations or commentary.

   The response is `{original, corrected, provider, model}`.
5. Errors:
   - `data.error`: `appAlert("Failed to process text with AI: <error>")`.
   - Exception: `appAlert("Failed to process text with AI")`.
6. `finally`: remove the overlay and the Escape listener, then dispatch `processing-ended`, which restores the editor's writability and the buttons.
7. Success:
   - The header shows a provider badge `"<provider>: <model>"` (small, tertiary background).
   - Compute a **word diff**. `tokenize` splits on `/(\s+|\S+)/g`, so whitespace runs are tokens too. Then run an LCS diff over the tokens and merge consecutive operations of the same type.
   - Render two columns.

**Diff dialog** (90vw, max-w-6xl, max-h 80vh, primary background):
- Header: "AI Grammar Check" plus the badge and a close X.
- Left half: header "Original (deletions in red)". It shows equal and deleted spans; deletions are `.ai-diff-del` (`rgba(239,68,68,.2)` background, `--theme-error` colour, line-through, 2px radius, 0 2px padding).
- Right half: header "Corrected (additions in green)" plus an **Edit** toggle button. It shows equal and inserted spans; insertions are `.ai-diff-add` (`rgba(34,197,94,.2)` background, `--theme-success` colour).
- Both columns are `whitespace-pre-wrap font-mono text-sm`, scroll independently and have no synced scrolling.
- **Edit toggle:** swaps the right-hand diff view for a full-height textarea prefilled with the corrected text (monospace, spellcheck off) and focuses it. The button label switches between "Edit" (`common.edit`) and "Preview" (`preview.title`). Going back to the diff view does **not** recompute the diff from the edits.
- Footer:
  - **Cancel** discards everything.
  - **Accept Changes** dispatches `accepted {correctedText: textarea.value}`. App_controller then calls `setValue(correctedText)`, replacing the **entire document**, and runs the change pipeline (autosave).
- There is no per-hunk accept or reject.

**Configuration notice panel** (shown when AI is disabled):
- An icon plus "AI Configuration Required", "Configure one of the following providers in your .fed file.", and "Priority (auto mode): Ollama → OpenRouter → Anthropic → Gemini → OpenAI".
- A 2-column grid of six cards, each with links and a `.fed` snippet:
  - Ollama: `ollama_api_base = http://localhost:11434`, `ollama_model = llama3.2`, plus "Run: ollama pull llama3.2".
  - OpenRouter: `openrouter_api_key = sk-or-...`, `openrouter_model = openai/gpt-4o-mini`.
  - Anthropic: `anthropic_api_key = sk-ant-...`, `anthropic_model = claude-sonnet-4-20250514`.
  - Gemini: `gemini_api_key = ...`, `gemini_model = gemini-2.0-flash`.
  - OpenAI: `openai_api_key = sk-...`, `openai_model = gpt-4o-mini`.
  - Provider Selection: `ai_provider = anthropic`, `ai_model = …`.
- A GitHub link to `README#optional-ai-grammar-checking`.

---

### D2.7 Log viewer (debug dialog)

**Files:** `controllers/log_viewer_controller.js`, `dialogs/_log_viewer.html.erb`.

- **Shortcut:** **Ctrl/Cmd+Shift+O**. (The ERB comment says Ctrl+Shift+L, which is stale.) There is no toolbar button.
- **Endpoints:** `GET /logs/tail?lines=100` (the server clamps to 1..500; default 100) and `GET /logs/config`.
- **Dialog** (90vw, max-w-5xl, fixed height 80vh, shadow-2xl):
  - **Header:** document icon, title ("Rails Log" or "Configuration"), and a subtitle line ("`<environment> - <log file>`", or for config "`<env> - <config file | (no .fed file)>`"). On the right: a status text ("N lines" / "1 line", or "N keys" / "1 key"), a refresh icon button ("Refresh"), and a close icon ("Close (Esc)").
  - **Tabs** (underline style, accent bottom border on the active tab): "Logs" and "Config".
  - **Logs tab:** a `<pre>` (`text-xs`, monospace, pre-wrap) with the lines joined by `\n`, auto-scrolled to the bottom. It shows "(log is empty)" when there are no lines, "Loading..." while fetching, and "Error loading logs: <msg>" on failure.
  - **Config tab:**
    - An info box with three lines: "Config file: …", "File exists: yes/no", and "AI configured in .fed: yes (ENV AI vars ignored) / no (using ENV vars)".
    - A table with columns Key, Value, Source and ENV var. Source labels: `.fed` (bold green) for file, `ENV` (bold yellow) for env, "default" (muted) for default.
    - Row tints: file rows `bg-green-900/10`, env rows `bg-yellow-900/10`. Null values show as muted "nil"; an empty ENV var shows "-". "No configuration values" appears when there are no entries.
  - Refresh re-fetches the active tab. Escape and backdrop clicks close the dialog.
- **Quirk:** `open()` sets the active tab to logs but never updates the tab styles or panel visibility. Reopening after viewing Config leaves the Config panel and title visible while the logs load into the hidden `<pre>`.

---

### D2.8 Help dialog (structure only; the shortcut table is covered elsewhere)

**Files:** `controllers/help_controller.js`, `dialogs/_help.html.erb`.

- **Entry points:** **F1**, the toolbar action `help`, and the vim `:help` command.
- **Dialog:** `max-w-4xl w-full max-h-85vh`, with its content scrolling.
  - Header: "Help" plus the hint "Press F1 to toggle" and a close X.
  - Backdrop click closes it. Escape (native) closes it. `closeAllDialogs` also closes it.
- **Tabs** (pill style; accent on the active tab): **Markdown**, **Keyboard Shortcuts**, **Vim Mode**.
  - Every open resets to the Markdown tab.
  - ←/→ on a tab button cycle tabs with wrap and move focus. The mouse wheel over the tablist cycles tabs.
- **Markdown panel:** a 2-column grid of cards. Each card has a muted uppercase heading and shows syntax next to its rendered sample. Cards: Headings (`#`/`##`/`###`), Emphasis, Lists, Task List, Links & Images, Blockquote, Code Block, Table (a small rendered table), Horizontal Rule.
- **Shortcuts panel:** cards titled File Operations, Editor, Insert, Text Format, Dialogs. The Dialogs card includes "switch tabs".
- **Vim panel:**
  - Intro paragraph.
  - "Modes" list: normal, insert, visual.
  - "Commands" grid of command → action:
    - `:w :write` → save
    - `:q :quit` → close (closeDialogs)
    - `:wq :x` → save and close
    - `:e :find` → file finder
    - `:Explore` → toggle sidebar
    - `:n :next` → next note
    - `:prev` → previous note
    - `:help` → help
- **Possible gap:** `openHelp()` only calls `showModal()`. Despite the "toggle" hint, F1 pressed while Help is open does not close it (it re-calls `showModal` on an open dialog).

---

### D2.9 Jump to line

**Files:** `controllers/jump_to_line_controller.js`, `dialogs/_jump_to_line.html.erb`.

- **Shortcut:** **Ctrl/Cmd+G**.
- **Dialog** (360px): header "Jump to Line" with a close X, then a text input (placeholder "e.g. 10, +5, -5, -1") with a primary "Go" button beside it. Below: a status line "Line number, or +N/-N for relative; -1 for end" and a faint "esc to close" hint (`dialogs.file_finder.esc_to_close`). The input is cleared and focused on open.
- **Keys:** Enter jumps. Escape calls `preventDefault`, closes, and refocuses the editor in the next animation frame.
- **Parsing** (`jump()`):
  - Trim the input; an empty input does nothing.
  - If it starts with `+` or `-`, it is **relative**: `parseInt`, so the target is current line ± N. The special case: exactly `-1` means the **last line**.
  - Otherwise it is an absolute `parseInt`.
  - NaN does nothing.
  - The target is clamped to `[1, totalLines]`.
  - Close the dialog, then dispatch `jump {lineNumber}`, and app calls `codemirror.jumpToLine(n)`.
  - The current line is computed from the editor adapter's `selectionStart`.

---

### D2.10 File-operation dialogs (new note, new folder, rename, delete confirm)

**Files:** `controllers/file_operations_controller.js`, `dialogs/_file_operations.html.erb`, `lib/new_note_utils.js`, server `HugoService` and `NotesController#create`. All three dialogs close on a backdrop click. The explorer tree and context menu are covered elsewhere.

#### New Note

**Opening:**
- **Ctrl/Cmd+N** or the explorer "new note" action: the parent is the selected folder, if exactly one folder is selected, otherwise root. These entry points set `fromSelection=true`.
- Folder context menu "New Note": the parent is that folder, with `fromSelection=false`.

**Dialog** (w-96): title "New Note".
- **Name input:** placeholder "Note name", autofocus, cleared on open.
- **Two type cards** in a 2-column grid:
  - **Empty Document** / "Plain markdown" (document icon). Selected by default.
  - **Hugo Blog Post** / "Hugo page bundle" (orange newspaper icon).
  - The selected card has an accent border, `ring-1` accent and a hover background, plus `aria-pressed`. Clicking a card returns focus to the name input.
- **Live path preview:** a folder icon plus monospace text that updates on every keystroke:
  - Muted when the name is valid.
  - Faint when the name is empty.
  - Error-coloured when invalid, with the tooltip "Name cannot contain slashes".
- **Inline error** `<p role=alert>` (error-coloured, hidden by default) for server errors.
- **Footer:** "Cancel" and **Create**. Create is disabled while the name is invalid.
- **Keys:** Enter submits (guarded by the same validity rule). Escape closes.

**Validity** (`new_note_utils.js:49-66`):
- The trimmed name must be non-empty.
- It must not contain `/`.
- It must not be exactly `.` or `..`.
- For Hugo, `slugify(name without trailing .md)` must also be non-empty.

**Preview path** (`previewNotePath`). Here `where` = parent, or the localized word "root":
- Empty template: `<where>/<name>.md`. `.md` is appended unless the name already ends in it. With no name the preview shows `<where>/note-name.md`.
- Hugo, `hugo_path_style = dated` (the default): `<where>/YYYY/MM/DD/<slug>/index.md`. The date is browser-local; the server uses its own timezone.
- Hugo, `flat`: `<where>/<slug>.md`. The style comes from `data-file-operations-hugo-path-style-value`, i.e. config `hugo_path_style` (env `HUGO_PATH_STYLE`).
- **Hugo parent override:** when opened via Ctrl+N or the toolbar (`fromSelection`) with the Hugo card selected, the parent is forced to **`blog`** (`file_operations_controller.js:253-255`).

**Slugify** (identical in JS and Ruby):
1. Lowercase.
2. Map accented characters: à á â ã ä å→a, æ→ae, ç č ć→c, è é ê ë ě→e, ì í î ï→i, ð ď→d, ñ ň→n, ò ó ô õ ö ø→o, ù ú û ü ů→u, ý ÿ→y, ž ź ż→z, ß→ss, þ→th, š ś→s, ř→r, ł→l.
3. Replace each run of `[^a-z0-9]+` with `-`.
4. Trim leading and trailing `-`.
5. Collapse repeated `-`.

**Create requests:**
- Before the request, the parent folder is expanded in the tree.
- **Empty note:** `POST /notes/<encoded path>` with `{content: "", expanded}`, expecting a turbo-stream response. The path is `parent/name(.md)`. The file content is empty.
- **Hugo:** `POST /notes` with `{template: "hugo", title: name without .md, parent, expanded}`. The server builds the path as above and the content from `.hugo_template.md` at the notes root, if present, or else the default template.
  - Default template:

    ```
    ---
    title: "{{title}}"
    slug: "{{slug}}"
    date: {{date}}
    draft: true
    tags: []
    ---

    ```

    The template ends with a blank line after the closing `---`.
  - Substitutions: `{{date}}` is `%Y-%m-%dT%H:%M:%S%:z`, e.g. `2026-11-04T10:15:00-03:00`. In the title, `"` is escaped as `\"`. Placeholders are replaced literally (block-form gsub).
- **Errors:** "Note already exists" (422), "Title is required". These show in the inline error and the dialog stays open.
- **Success:** the created path is read from the `X-Created-Path` header. Then `file-created` fires, and app opens the note and records an "undo creation" boundary (see D2.11).

#### New Folder

- **Dialog** (w-80): title "New Folder"; a location line "In: <parent | root folder>" (truncated, with the full value as tooltip); an input with placeholder "Folder name"; an inline error; Cancel and Create.
- **Keys:** Enter submits; Escape closes.
- A blank name does nothing. There is no client-side validation beyond that.
- `POST /folders/<encoded parent/name>?expanded=…`.

#### Rename

- **Opening:** the explorer context menu "Rename", or `renameExplorerItem` (for example from the inline document title).
- **Dialog** (w-80): title "Rename"; an input prefilled with the last path segment, with `.md` stripped for files, focused and fully selected; an inline error; Cancel and Rename.
- **Keys:** Enter submits; Escape closes.
- **Submit:**
  - A blank name does nothing.
  - Files get `.md` appended unless the name already ends in it.
  - Only the last path segment is replaced, so a rename cannot move the item.
  - If the new path equals the old one, close without a request.
  - Otherwise flush or prepare the active draft if the item is (or contains) the open note.
  - `POST /notes|folders/<path>/rename {new_path, expanded}`. On error, show it inline.

#### Delete confirm

Delete is triggered from the context menu, or from keys handled elsewhere. It uses `appConfirm` with accept label "Delete" and **destructive** styling (the dialog itself is focused, not the button). Messages:
- File: `Are you sure you want to delete "<name>"? This action cannot be undone.`
- Folder: `Are you sure you want to delete "<name>" and all its contents? This action cannot be undone.`
- Multiple: `Are you sure you want to delete these <n> selected items? This action cannot be undone.`

Items nested inside a selected folder are deduplicated, and config files are skipped. Items are deleted sequentially with `DELETE /notes|folders/<path>`. The first failure shows `appAlert` and stops.

---

### D2.11 Undo note creation confirm (`_undo_created_note.html.erb`)

This appears when the user presses Undo (Ctrl+Z) at the very start of the history of a note created in this session, while the content still equals the content it was created with.

- **Dialog** (w-80, max 100vw−2rem): `<form method=dialog>`.
  - Header: "Undo note creation?" with a close X labelled "Close".
  - Message: `Undoing creation will delete "<name>". You can redo it during this session.` The name is rendered as a `.confirm-dialog__filename` chip.
  - Buttons: **Cancel** (secondary, `value=cancel`, **autofocus**) and **Delete** (outlined in error colour, `value=confirm`).
  - The result is read from `returnValue === "confirm"`. Escape counts as cancel.
- **On confirm:**
  1. Lock the editor read-only.
  2. Prepare autosave for the deletion.
  3. Re-check that the content is unchanged.
  4. `DELETE /notes/<path>?expected_revision=<rev>`.
  5. Mark the boundary as deleted.
  6. Return to the previously open note.
  - Failures show a temporary 5s status message.
- **Redo** (Ctrl+Shift+Z or Ctrl+Y) recreates the note:
  - From the previous note, once its CodeMirror redo history is exhausted.
  - Or at the empty root view (no file open, focus not in an input).

---

---

## 9–10. Themes, Settings dialog, client i18n  
<sub>(sub-section labels below keep the source numbering, e.g. E.x)</sub>

Sources are relative to `~/Projects/FrankMD/`. Theme tables in E.1.3 were generated by a script that parses `app/assets/tailwind/themes/*.css`. Nothing was typed by hand.

---

### E.1 Theme system: built-in themes and CSS variables

#### E.1.1 Mechanism

- Every theme is one CSS rule `[data-theme="<id>"] { --theme-*: #rrggbb; ... }` in `app/assets/tailwind/themes/<id>.css`. `light.css` uses `:root, [data-theme="light"]`, so **Light is the fallback palette** for any unknown or missing `data-theme` value (`themes/light.css:1-3`).
- Every theme file defines **exactly the same 38 variables**, all as 6-digit hex, and nothing else (checked mechanically). `flexoki-light.css` uses uppercase hex. The Tailwind build minifies colors: `#ffffff` becomes `#fff`, and some become named colors like `khaki` or `red`. `lib/custom_theme.js:17-28` therefore normalizes computed values back to `#rrggbb`.
- Imports run Light, then Dark, then the rest alphabetically (`application.css:57-74`), followed by component CSS.
- `application.css:29`: `@variant dark (&:where(.dark, .dark *));`. Tailwind `dark:` utilities follow a `.dark` class on `<html>`, not the media query. The theme controller toggles this class.
- Fonts: `--font-sans: "Inter", ui-sans-serif, system-ui, sans-serif`; `--font-mono: "JetBrains Mono", ui-monospace, monospace` (`application.css:24-27`).
- Base layer (`components/scrollbar.css`):
  - `body { background: var(--theme-bg-primary); color: var(--theme-text-primary) }`.
  - WebKit scrollbars are 8px wide/high with a transparent track. The thumb uses `var(--theme-scrollbar)` with a 4px radius, and `var(--theme-scrollbar-hover)` on hover.
  - Caveat: the `<body>` element also has the utility classes `bg-white dark:bg-zinc-900 text-zinc-900 dark:text-zinc-100` (`layouts/application.html.erb:25`). Those win over the base layer. The visible app surfaces paint their own theme backgrounds.
- The safelist (`application.css:9-22`) lists the arbitrary classes used in JS-generated HTML: `text-[var(--theme-*)]`, `bg-[var(--theme-*)]`, `border-[var(--theme-border|accent)]`, the hover variants and the ring variants.

#### E.1.2 Variable semantics (38 keys, same order as `CustomTheme::COLOR_KEYS`)

| Variable | Used for |
|---|---|
| `--theme-bg-primary` | Main app/editor background, CodeMirror root bg, dialog bg |
| `--theme-bg-secondary` | Panels, sidebar, CM gutters, CM panels/tooltips, settings cards, dropdown menus |
| `--theme-bg-tertiary` | Raised surfaces, switch track (off), range track |
| `--theme-bg-hover` | Hover background, active nav item, CM panel buttons |
| `--theme-text-primary` | Body text, caret, CM cursor, bold text |
| `--theme-text-secondary` | Secondary text, switch knob (off), prose body |
| `--theme-text-muted` | Descriptions, italic/strike/quote/list/meta tokens in CM, active gutter line number |
| `--theme-text-faint` | Section titles, gutter numbers, placeholder, HTML comments |
| `--theme-border` | Borders, CM gutter right border, contentSeparator token |
| `--theme-accent` | Accent: links, inline code, h4-h6, active states, switch on, range thumb, focus rings |
| `--theme-accent-hover` | Accent hover |
| `--theme-accent-text` | Text on accent backgrounds (switch knob when on, Save button text) |
| `--theme-selection` | CM selection bg, search match bg, matching bracket bg |
| `--theme-selection-text` | Selection text color |
| `--theme-scrollbar` / `--theme-scrollbar-hover` | Scrollbar thumb |
| `--theme-success` / `--theme-warning` / `--theme-error` | Status colors (warning is also the custom-theme contrast warning; error marks invalid inputs) |
| `--theme-code-bg` | Inline/fenced code background |
| `--theme-heading-1..3` | Heading colors in CM and prose. `heading-2` also colors HTML attributeName, `heading-3` attributeValue |
| `--theme-folder-icon` / `--theme-file-icon` / `--theme-config-icon` | File tree icons |
| `--theme-syntax-*` (12) | Fenced-code token colors in preview (`prose.css:175-245`) |

The 12 syntax variables map to `.tok-*` classes as follows. Each falls back to `--theme-text-primary`, except `invalid`, which falls back to `--theme-error`.

- **keyword**: keyword, modifier (bold 600)
- **function**: function
- **operator**: operator, derefOperator
- **string**: string, string2, character, attributeValue
- **number**: number, unit
- **constant**: constant, bool, null, atom
- **comment**: comment, docComment (italic)
- **type**: typeName, className, namespace, tagName
- **property**: propertyName, attributeName
- **identifier**: variableName
- **punctuation**: punctuation, bracket, separator
- **invalid**: wavy underline

`.tok-definition` is bold.

##### Derived variables and mixes (not per-theme)

- `--workspace-sidebar-border: color-mix(in srgb, var(--theme-border) 50%, transparent)` (`components/editor.css:268`)
- `--editor-width: <n>ch` is set inline on `<html>` by the settings controller (`settings_controller.js:168`). It is consumed as `max-width: calc(var(--editor-width, 72ch) + 3rem)` (`editor.css:19`).
- Backdrops:
  - custom-theme editor: `color-mix(bg-primary 75%, transparent)`
  - image viewer: 85%
  - library overlay: 78%
  - sidebar backdrop: `color-mix(text-primary 25%, transparent)`
- Other mixes:
  - CM element: `color-mix(accent 50%)` outline (`codemirror.css:240`)
  - `color-mix(scrollbar-hover 75%, text-primary)` (`codemirror.css:363`)
  - toast background: `color-mix(accent 92%)` (`editor.css:72`)

##### CodeMirror theme sync (`lib/codemirror_theme.js`)

CodeMirror's `EditorView.theme` and `HighlightStyle` reference `var(--theme-*)` directly, so **theme switches need no CM reconfiguration**. Only the CSS variables change. The theme is declared `{dark:true}` unconditionally (`:182`).

| CodeMirror element | Styling |
|---|---|
| Root | bg-primary / text-primary |
| Caret | text-primary, 2px |
| Selection | selection |
| Gutters | bg-secondary bg, text-faint text, right border in border color. Active gutter line: text-muted |
| Search match | selection bg + 1px accent outline. Selected match: accent bg |
| Matching bracket | selection bg + accent outline |
| Panels | bg-secondary. Inputs: bg-primary. Buttons: bg-hover, turning accent with accent-text on hover |
| Tooltips | bg-secondary + border |

Markdown token colors:

| Token | Color / style |
|---|---|
| h1 / h2 / h3 | heading-1 / heading-2 / heading-3 |
| h4 to h6 | accent |
| strong | text-primary, bold |
| emphasis | muted, italic |
| strikethrough | muted, line-through |
| monospace | accent text on code-bg, radius 0.25rem, padding 0 0.25rem |
| link | accent, underlined |
| url | accent |
| quote, list, processingInstruction, meta, angleBracket, escape | muted |
| contentSeparator | border |
| tagName | accent |
| attributeName | heading-2 |
| attributeValue | heading-3 |
| comment | faint, italic |
| labelName, inserted | accent |
| deleted | muted, line-through |

The vim fat-cursor is styled elsewhere (`codemirror_extensions.js`).

#### E.1.3 Built-in theme catalog and full variable values

The light/dark flag is `theme_controller.js:35`: `lightThemes = ["light","solarized-light","catppuccin-latte","rose-pine","flexoki-light"]`. Everything else counts as dark and gets `<html class="dark">`. Display names and icons come from `theme_controller.js:13-32`: `light` uses the sun icon, `dark` the moon icon, all others a palette icon. Menu order is the table order below. Server-side `CustomTheme::BASE_THEMES` lists the same 18 ids in the same order (`app/services/custom_theme.rb:6`).

Pseudo-themes, which are not CSS files:

- `omarchy` (display "Omarchy", sync icon). Only added when the Omarchy endpoint answers 200. See E.3.
- `custom` (display name = the palette's name, or "My theme" localized). Only present when a saved or previewed palette has a valid `base`. See E.4.

| id | display name | mode | CSS selector | file header comment |
|---|---|---|---|---|
| `light` | Light | light | `:root, [data-theme="light"]` | Light Theme (Default) |
| `dark` | Dark | dark | `[data-theme="dark"]` | Dark Theme |
| `catppuccin` | Catppuccin | dark | `[data-theme="catppuccin"]` | Catppuccin Mocha - Soothing pastel theme for the high-spirited |
| `catppuccin-latte` | Catppuccin Latte | light | `[data-theme="catppuccin-latte"]` | Catppuccin Latte - Light pastel theme for daytime coding |
| `ethereal` | Ethereal | dark | `[data-theme="ethereal"]` | Ethereal - Deep space with warm peachy tones |
| `everforest` | Everforest | dark | `[data-theme="everforest"]` | Everforest - Comfortable green nature-inspired theme |
| `flexoki-light` | Flexoki Light | light | `[data-theme="flexoki-light"]` | Flexoki Light - Warm paper-like light theme with blue accent |
| `gruvbox` | Gruvbox | dark | `[data-theme="gruvbox"]` | Gruvbox Theme - Retro groove color scheme |
| `hackerman` | Hackerman | dark | `[data-theme="hackerman"]` | Hackerman Theme - Matrix-style green on black |
| `kanagawa` | Kanagawa | dark | `[data-theme="kanagawa"]` | Kanagawa - Inspired by Katsushika Hokusai's The Great Wave |
| `matte-black` | Matte Black | dark | `[data-theme="matte-black"]` | Matte Black - Sleek dark theme with warm orange accent |
| `nord` | Nord | dark | `[data-theme="nord"]` | Nord Theme - Arctic, north-bluish color palette |
| `osaka-jade` | Osaka Jade | dark | `[data-theme="osaka-jade"]` | Osaka Jade - Japanese aesthetic with jade green accents |
| `ristretto` | Ristretto | dark | `[data-theme="ristretto"]` | Ristretto - Rich coffee-inspired warm tones |
| `rose-pine` | Rose Pine | light | `[data-theme="rose-pine"]` | Rose Pine Dawn - Soft light theme with rose and pine tones |
| `solarized-dark` | Solarized Dark | dark | `[data-theme="solarized-dark"]` | Solarized Dark Theme - Precision colors for machines and people |
| `solarized-light` | Solarized Light | light | `[data-theme="solarized-light"]` | Solarized Light Theme |
| `tokyo-night` | Tokyo Night | dark | `[data-theme="tokyo-night"]` | Tokyo Night Theme - Futuristic city lights |

| variable | `light` | `dark` | `catppuccin` | `catppuccin-latte` | `ethereal` | `everforest` |
|---|---|---|---|---|---|---|
| `--theme-bg-primary` | #ffffff | #18181b | #1e1e2e | #eff1f5 | #060b1e | #2d353b |
| `--theme-bg-secondary` | #fafafa | #1f1f23 | #181825 | #e6e9ef | #0a1020 | #272e33 |
| `--theme-bg-tertiary` | #f4f4f5 | #27272a | #313244 | #dce0e8 | #101830 | #343f44 |
| `--theme-bg-hover` | #e4e4e7 | #3f3f46 | #45475a | #bcc0cc | #1a2545 | #475258 |
| `--theme-text-primary` | #18181b | #fafafa | #cdd6f4 | #4c4f69 | #ffcead | #d3c6aa |
| `--theme-text-secondary` | #3f3f46 | #d4d4d8 | #bac2de | #5c5f77 | #f99957 | #a7c080 |
| `--theme-text-muted` | #71717a | #a1a1aa | #585b70 | #6c6f85 | #6d7db6 | #7a8478 |
| `--theme-text-faint` | #a1a1aa | #71717a | #45475a | #acb0be | #4a5580 | #475258 |
| `--theme-border` | #e4e4e7 | #3f3f46 | #45475a | #bcc0cc | #1a2545 | #475258 |
| `--theme-accent` | #2563eb | #3b82f6 | #89b4fa | #1e66f5 | #7d82d9 | #7fbbb3 |
| `--theme-accent-hover` | #1d4ed8 | #60a5fa | #b4befe | #2a6ef5 | #9498e5 | #83c092 |
| `--theme-accent-text` | #ffffff | #ffffff | #1e1e2e | #eff1f5 | #060b1e | #2d353b |
| `--theme-selection` | #dbeafe | #1e3a5f | #f5e0dc | #dc8a78 | #ffcead | #d3c6aa |
| `--theme-selection-text` | #1d4ed8 | #93c5fd | #1e1e2e | #eff1f5 | #060b1e | #2d353b |
| `--theme-scrollbar` | #a1a1aa | #52525b | #45475a | #bcc0cc | #1a2545 | #475258 |
| `--theme-scrollbar-hover` | #71717a | #71717a | #585b70 | #acb0be | #2a3560 | #56635a |
| `--theme-success` | #16a34a | #4ade80 | #a6e3a1 | #40a02b | #92a593 | #a7c080 |
| `--theme-warning` | #f97316 | #fb923c | #f9e2af | #df8e1d | #e9bb4f | #dbbc7f |
| `--theme-error` | #dc2626 | #f87171 | #f38ba8 | #d20f39 | #ed5b5a | #e67e80 |
| `--theme-code-bg` | #f4f4f5 | #27272a | #313244 | #dce0e8 | #101830 | #343f44 |
| `--theme-heading-1` | #2563eb | #60a5fa | #cba6f7 | #8839ef | #c2c4f0 | #a7c080 |
| `--theme-heading-2` | #7c3aed | #a78bfa | #89b4fa | #1e66f5 | #a3bfd1 | #7fbbb3 |
| `--theme-heading-3` | #0891b2 | #22d3ee | #94e2d5 | #179299 | #c89dc1 | #83c092 |
| `--theme-folder-icon` | #3b82f6 | #60a5fa | #89b4fa | #1e66f5 | #7d82d9 | #7fbbb3 |
| `--theme-file-icon` | #71717a | #a1a1aa | #585b70 | #6c6f85 | #6d7db6 | #7a8478 |
| `--theme-config-icon` | #f59e0b | #fbbf24 | #f9e2af | #df8e1d | #e9bb4f | #dbbc7f |
| `--theme-syntax-keyword` | #8700a5 | #e2a4e6 | #d8a8e8 | #771db7 | #d8a5ff | #d8a8ef |
| `--theme-syntax-function` | #6d4c1f | #f0e68c | #f6c177 | #7b4b12 | #ffd28f | #e8d18a |
| `--theme-syntax-type` | #0f5c70 | #63d6c0 | #9de0d0 | #0d5860 | #87e0e0 | #83d6c7 |
| `--theme-syntax-string` | #a31515 | #f0a070 | #b7e3a1 | #a23c10 | #a9e6bc | #b6d58e |
| `--theme-syntax-number` | #006b45 | #c3df9d | #ffc08a | #8a5600 | #ffb28f | #f1b482 |
| `--theme-syntax-constant` | #001080 | #79b8ff | #93c7ff | #3557b7 | #9ab8ff | #91c9e6 |
| `--theme-syntax-identifier` | #20202a | #e3e6ea | #cdd6f4 | #4c4f69 | #ffcead | #d3c6aa |
| `--theme-syntax-property` | #0050a4 | #addcff | #89dceb | #1a6175 | #9bd4ff | #8fc9e1 |
| `--theme-syntax-operator` | #454545 | #d0d6e0 | #bac2de | #45475a | #ccd2ff | #d3c6aa |
| `--theme-syntax-comment` | #3f6417 | #a0c27b | #b7bdcc | #505d74 | #9caed4 | #aec299 |
| `--theme-syntax-punctuation` | #4a4a4a | #bfc6d2 | #aeb9d4 | #575a6d | #bac7e6 | #bac6bc |
| `--theme-syntax-invalid` | #a90000 | #ff9e9e | #ff98a6 | #b4233e | #ff969f | #ff9991 |

| variable | `flexoki-light` | `gruvbox` | `hackerman` | `kanagawa` | `matte-black` | `nord` |
|---|---|---|---|---|---|---|
| `--theme-bg-primary` | #FFFCF0 | #282828 | #0a0a0a | #1f1f28 | #121212 | #2e3440 |
| `--theme-bg-secondary` | #F2F0E5 | #1d2021 | #050505 | #16161d | #0a0a0a | #272c36 |
| `--theme-bg-tertiary` | #E6E4D9 | #3c3836 | #111111 | #2a2a37 | #1a1a1a | #3b4252 |
| `--theme-bg-hover` | #CECDC3 | #504945 | #1a1a1a | #363646 | #333333 | #434c5e |
| `--theme-text-primary` | #100F0F | #ebdbb2 | #00ff00 | #dcd7ba | #bebebe | #eceff4 |
| `--theme-text-secondary` | #3D3D3D | #d5c4a1 | #00dd00 | #c8c093 | #eaeaea | #e5e9f0 |
| `--theme-text-muted` | #6F6E69 | #a89984 | #00aa00 | #727169 | #8a8a8d | #d8dee9 |
| `--theme-text-faint` | #B7B5AC | #928374 | #007700 | #54546d | #666666 | #7b88a1 |
| `--theme-border` | #CECDC3 | #504945 | #003300 | #363646 | #333333 | #4c566a |
| `--theme-accent` | #205EA6 | #fe8019 | #00ff00 | #7e9cd8 | #e68e0d | #88c0d0 |
| `--theme-accent-hover` | #4385BE | #fabd2f | #33ff33 | #7fb4ca | #f59e0b | #8fbcbb |
| `--theme-accent-text` | #FFFCF0 | #282828 | #000000 | #1f1f28 | #121212 | #2e3440 |
| `--theme-selection` | #CECDC3 | #504945 | #003300 | #2d4f67 | #333333 | #434c5e |
| `--theme-selection-text` | #100F0F | #fabd2f | #00ff00 | #c8c093 | #bebebe | #eceff4 |
| `--theme-scrollbar` | #CECDC3 | #665c54 | #003300 | #363646 | #333333 | #4c566a |
| `--theme-scrollbar-hover` | #B7B5AC | #7c6f64 | #004400 | #54546d | #4d4d4d | #5e6779 |
| `--theme-success` | #879A39 | #b8bb26 | #00ff00 | #98bb6c | #ffc107 | #a3be8c |
| `--theme-warning` | #D0A215 | #fe8019 | #ffff00 | #e6c384 | #ffc107 | #ebcb8b |
| `--theme-error` | #D14D41 | #fb4934 | #ff0000 | #e82424 | #d35f5f | #bf616a |
| `--theme-code-bg` | #E6E4D9 | #3c3836 | #111111 | #2a2a37 | #1a1a1a | #3b4252 |
| `--theme-heading-1` | #205EA6 | #fabd2f | #00ff00 | #957fb8 | #ffffff | #81a1c1 |
| `--theme-heading-2` | #3AA99F | #8ec07c | #00dddd | #7e9cd8 | #e68e0d | #b48ead |
| `--theme-heading-3` | #CE5D97 | #83a598 | #dddd00 | #7fb4ca | #bebebe | #a3be8c |
| `--theme-folder-icon` | #205EA6 | #fe8019 | #00ff00 | #7e9cd8 | #e68e0d | #81a1c1 |
| `--theme-file-icon` | #6F6E69 | #a89984 | #00aa00 | #727169 | #8a8a8d | #7b88a1 |
| `--theme-config-icon` | #D0A215 | #fabd2f | #ffff00 | #e6c384 | #ffc107 | #ebcb8b |
| `--theme-syntax-keyword` | #7b2ea0 | #ff8175 | #00e676 | #d27e99 | #ffb454 | #d4a8d7 |
| `--theme-syntax-function` | #72531c | #fabd2f | #ffeb3b | #e6c384 | #f2cc60 | #f2d08b |
| `--theme-syntax-type` | #116177 | #8ec07c | #00e5ff | #7fb4ca | #56d4dd | #8fcac3 |
| `--theme-syntax-string` | #a33e1a | #b8bb26 | #76ff03 | #98bb6c | #a6da95 | #b5d19d |
| `--theme-syntax-number` | #08734a | #fe8019 | #ffab00 | #ffa066 | #d2a8ff | #f0a676 |
| `--theme-syntax-constant` | #144b8a | #e8a4b6 | #40c4ff | #ff9e3b | #ff7b72 | #92b7e2 |
| `--theme-syntax-identifier` | #302c2b | #ebdbb2 | #00ff00 | #dcd7ba | #e6edf3 | #eceff4 |
| `--theme-syntax-property` | #1a5d82 | #a0c4b7 | #69f0ae | #7e9cd8 | #79c0ff | #9dccdd |
| `--theme-syntax-operator` | #45403d | #d5c4a1 | #b9f6ca | #c8c093 | #ffc56d | #d8dee9 |
| `--theme-syntax-comment` | #4d5d3b | #b7c08a | #00cc00 | #a6a69c | #adbac7 | #c2c9d5 |
| `--theme-syntax-punctuation` | #5b514a | #c5b8a3 | #80cbc4 | #c8c093 | #c1c8d0 | #c6ceda |
| `--theme-syntax-invalid` | #b3261e | #ff8f7a | #ff5252 | #ff8a80 | #ff8585 | #ff9e9e |

| variable | `osaka-jade` | `ristretto` | `rose-pine` | `solarized-dark` | `solarized-light` | `tokyo-night` |
|---|---|---|---|---|---|---|
| `--theme-bg-primary` | #111c18 | #2c2525 | #faf4ed | #002b36 | #fdf6e3 | #1a1b26 |
| `--theme-bg-secondary` | #0d1512 | #241f1f | #fffaf3 | #073642 | #eee8d5 | #16161e |
| `--theme-bg-tertiary` | #1a2a24 | #352e2e | #f2e9e1 | #073642 | #eee8d5 | #24283b |
| `--theme-bg-hover` | #23372b | #403e41 | #dfdad9 | #094959 | #e0dbc8 | #343a52 |
| `--theme-text-primary` | #c1c497 | #e6d9db | #575279 | #839496 | #657b83 | #c0caf5 |
| `--theme-text-secondary` | #f6f5dd | #c3b7b8 | #797593 | #93a1a1 | #586e75 | #a9b1d6 |
| `--theme-text-muted` | #53685b | #948a8b | #9893a5 | #657b83 | #93a1a1 | #565f89 |
| `--theme-text-faint` | #23372b | #72696a | #cecacd | #586e75 | #839496 | #414868 |
| `--theme-border` | #23372b | #403e41 | #dfdad9 | #094959 | #d3cbb7 | #343a52 |
| `--theme-accent` | #509475 | #f38d70 | #56949f | #268bd2 | #268bd2 | #7aa2f7 |
| `--theme-accent-hover` | #63b07a | #f8a788 | #286983 | #2aa198 | #2aa198 | #89b4fa |
| `--theme-accent-text` | #111c18 | #2c2525 | #faf4ed | #fdf6e3 | #fdf6e3 | #1a1b26 |
| `--theme-selection` | #c1c497 | #403e41 | #dfdad9 | #094959 | #e0dbc8 | #33467c |
| `--theme-selection-text` | #111c18 | #e6d9db | #575279 | #93a1a1 | #586e75 | #c0caf5 |
| `--theme-scrollbar` | #23372b | #403e41 | #dfdad9 | #586e75 | #93a1a1 | #414868 |
| `--theme-scrollbar-hover` | #2d4435 | #4d4a4a | #cecacd | #657b83 | #839496 | #565f89 |
| `--theme-success` | #549e6a | #adda78 | #286983 | #859900 | #859900 | #9ece6a |
| `--theme-warning` | #e5c736 | #f9cc6c | #ea9d34 | #cb4b16 | #cb4b16 | #e0af68 |
| `--theme-error` | #ff5345 | #fd6883 | #b4637a | #dc322f | #dc322f | #f7768e |
| `--theme-code-bg` | #1a2a24 | #352e2e | #f2e9e1 | #073642 | #eee8d5 | #24283b |
| `--theme-heading-1` | #509475 | #f38d70 | #907aa9 | #b58900 | #b58900 | #bb9af7 |
| `--theme-heading-2` | #2dd5b7 | #a8a9eb | #56949f | #cb4b16 | #cb4b16 | #7dcfff |
| `--theme-heading-3` | #acd4cf | #85dacc | #d7827e | #2aa198 | #2aa198 | #73daca |
| `--theme-folder-icon` | #509475 | #f38d70 | #56949f | #2aa198 | #268bd2 | #bb9af7 |
| `--theme-file-icon` | #53685b | #948a8b | #9893a5 | #657b83 | #93a1a1 | #565f89 |
| `--theme-config-icon` | #e5c736 | #f9cc6c | #ea9d34 | #b58900 | #b58900 | #e0af68 |
| `--theme-syntax-keyword` | #e8a87c | #f26d9c | #625d7b | #c792ea | #7b4fa3 | #bb9af7 |
| `--theme-syntax-function` | #f2d075 | #f9c859 | #73531c | #ffd866 | #855800 | #e0af68 |
| `--theme-syntax-type` | #50d8c1 | #7ad5cf | #286983 | #65d5cb | #006b73 | #7dcfff |
| `--theme-syntax-string` | #a8d894 | #a9dc76 | #9f465b | #95d96a | #24692d | #9ece6a |
| `--theme-syntax-number` | #f0ad72 | #fd9353 | #954b00 | #ffad66 | #aa4600 | #ff9e64 |
| `--theme-syntax-constant` | #64c5d5 | #cab8ff | #355d9a | #79c0ff | #205ea6 | #f7768e |
| `--theme-syntax-identifier` | #c1c497 | #e6d9db | #575279 | #b2c2c4 | #4f5d64 | #c0caf5 |
| `--theme-syntax-property` | #7ccfb7 | #80bfff | #24647d | #68c5e3 | #12607b | #73daca |
| `--theme-syntax-operator` | #aad5c0 | #e6d9db | #575279 | #93a1a1 | #4b6067 | #a9b1d6 |
| `--theme-syntax-comment` | #94af9d | #bcaeaf | #686487 | #91b67e | #4b6d31 | #9aa5ce |
| `--theme-syntax-punctuation` | #b4c7ba | #c8bbbc | #625d7b | #b2c2c4 | #4d6269 | #a9b1d6 |
| `--theme-syntax-invalid` | #ff9187 | #ff8a80 | #b42318 | #ff8787 | #b13a2e | #ff8c94 |

---

### E.2 Theme selection, application, persistence

#### E.2.1 Where the choice lives
- **Server config only.** The choice is stored as the `.fed` key `theme` (string, default `nil`). There is **no localStorage** for theme, locale or any setting. The palette is the `.fed` key `custom_theme`: one line holding JSON, `custom_theme = {"version":1,...}`. See `app/models/config.rb:13,15,640-641`.
- `.fed` is the INI-like config file at the notes root. The server does **not** validate the `theme` string beyond the general string rule (no quotes or newlines, `config.rb:631-635`). Any value is accepted.
- The `.fed` template comment lists stale names ("cappuccino, osaka") (`config.rb:149-151`). Not a real alias. Unknown ids fall back to Light CSS (see E.2.3).
- Initial value: `data-theme-initial-value="<%= @config.dig(:settings,'theme') %>"` and `data-theme-custom-value` (JSON) on the theme controller element (`_settings_panel.html.erb:135`). The settings controller also carries `data-settings-theme-value` and `data-settings-custom-theme-value` (`:20-21`).

#### E.2.2 Save flow
- Selecting a menu item calls `selectTheme`, which:
  1. sets `currentThemeId`;
  2. applies immediately;
  3. closes the dropdown and restores focus to the button;
  4. after a **500 ms debounce**, sends `PATCH /config {theme: id}` as JSON (`theme_controller.js:92-124`).
- On success the client dispatches `window` event `frankmd:config-file-modified`. If `.fed` is open in the editor, the app reloads its contents and keeps the cursor offset (`app_controller.js:1564-1597`). Failures only log a console warning.
- `PATCH /config` permits `Config::UI_KEYS` only. With no valid keys it returns 422 `{error: t("errors.no_valid_settings")}`. On success it returns `{settings: ui_settings, message}` (`config_controller.rb:47-65`).

#### E.2.3 Applying a theme (`applyTheme`, `theme_controller.js:126-154`)
1. `clearCustomColors()` removes all 38 inline `--theme-*` properties from `<html>`.
2. `themeId = currentThemeId || (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")`.
   - **"System" is implicit.** It applies when no `theme` key is set; there is no explicit "System" menu entry.
   - The media query is read **only at apply time**. There is **no `change` listener**, so an OS switch has no effect until the next apply or reload.
3. If the id is `omarchy`, go to E.3.
4. Otherwise stop Omarchy polling and remove the `<style id="omarchy-theme-vars">` element.
5. Custom handling: if `themeId==="custom"` and the palette's `base` is a valid built-in id:
   - `data-theme=base`;
   - then `applyCustomColors(palette.colors)` sets inline `--theme-<key>` on `<html>` (valid keys and hex only).

   If `custom` is selected but there is no valid palette, the system light/dark fallback is used and the menu shows that fallback as checked.
6. `document.documentElement.setAttribute("data-theme", baseId)`.
7. Toggle `<html class="dark">` when baseId is not in `lightThemes`. An unknown id therefore gets dark utilities while showing Light CSS.
8. Update the button label (`currentTheme` target): the theme name, or "Light" if the id isn't in the runtime list. Update checkmarks and `aria-checked` in the menu.

Startup sequence:

- `connect()` first awaits `GET /config/omarchy_theme`, an availability probe that runs on every page load. Only then does it call `applyTheme()` and render the menu (`:49-53`).
- Until then `<html>` has no `data-theme`, so `:root` (Light) applies. The full-screen loading overlay (`#app-loading`, purple-to-pink gradient `#2D1B4E → #FF6B9D`) covers this. The app controller hides it.

#### E.2.4 Live config sync
- `settings_controller.applyTheme()` runs on connect and whenever `themeValue` or `customThemeValue` changes. It dispatches `window` event `frankmd:config-changed {theme, customTheme}`. Nothing is dispatched when `themeValue` is empty (`settings_controller.js:204-210`).
- The theme controller listens (`theme_controller.js:72-85`):
  - The listener is ignored while a custom preview is active.
  - If `customTheme` is defined, it replaces `customValue`.
  - If `theme` is present and differs, or the palette changed, it re-applies and re-renders the menu.
- The trigger is `.fed` edited as a note and autosaved. That fires `autosave config-saved`, then `app#reloadConfig`, then `settings.reload()`, which sends `GET /config` and re-assigns values. A status message `status.config_applied` shows for 2 s (`app_controller.js:1228-1239`).

#### E.2.5 Theme dropdown UI
- Located in the Settings dialog, General section, "Theme" card (`_settings_panel.html.erb:130-158`).
- Button:
  - `w-40`, bordered;
  - truncated current-name span (initial text "Light");
  - chevron;
  - `title=header.change_theme` ("Change Theme");
  - `aria-haspopup=menu`, `aria-expanded`, `aria-controls=settings-theme-menu`.
- Menu:
  - `popover="auto"`, `role=menu`, `w-48`, `bg-secondary`, border, `py-1`, classes `frankmd-menu floating-dropdown`;
  - items are JS-rendered `button[role=menuitemradio][data-theme=id][aria-checked]`;
  - each item shows `icon + escaped name` on the left and a check SVG on the right (`opacity-0` unless current);
  - row padding `px-3 py-2 text-sm`, `hover:bg-hover`.
- Below the button: a link-style button "Customize theme" (`settings.custom_theme.customize`), `text-xs`, accent color, underlined. It opens the custom theme editor (E.4).
- Shared dropdown behavior (`lib/floating_dropdown.js`). Used by the theme, locale and media-storage pickers.
  - Only one menu is open at a time (document event `frankmd:dropdown-open`).
  - Opening focuses the checked option, or the first one.
  - Keys:
    - ArrowDown/ArrowUp cycle with wraparound.
    - Home/End go to the first/last option.
    - Arrow keys on the closed button open the menu. ArrowUp then focuses the last item.
    - Escape closes the menu, restores focus to the button and stops propagation, so the Settings dialog stays open.
    - Tab closes the menu and restores focus.
  - It also closes on:
    - an outside click;
    - outside focusin;
    - a window scroll that moved the anchor (for example, scrolling the Settings dialog);
    - a visualViewport scroll;
    - the containing dialog closing;
    - a category switch, which counts as an outside click.
  - Positioning:
    - `position:fixed`, viewport padding 8px, gap 4px;
    - max height `min(320, available)`;
    - flips upward when the space below is less than `min(320, scrollHeight)` and there is more space above;
    - right-aligned to the button, or left-aligned when the button's computed `direction` is `rtl`;
    - closes if the anchor scrolls off-screen;
    - repositions on resize and keeps the focused option scrolled into view.
  - Static CSS fallbacks: `.frankmd-menu{max-height:60vh;overflow-y:auto;overscroll-behavior:contain}` and `.floating-dropdown{position:fixed;max-height:20rem;z-index:50}`. Focus-visible outline is `2px accent`, offset -2px (`components/menu.css`).

---

### E.3 Omarchy theme mode

Backend: `app/services/omarchy_theme_service.rb`. Endpoint `GET /config/omarchy_theme` (`config_controller.rb:35-43`, `routes.rb:64`).

- **Availability**: both `~/.config/omarchy/current/theme.name` and `~/.config/omarchy/current/theme/alacritty.toml` must exist. If not, the endpoint returns 404 `{error:"Omarchy theme not available"}`.
- **Response**: `{theme_name: <contents of theme.name, stripped>, variables: {"--theme-...": "#hex", ...38 keys}, is_dark: bool}`.
- **It reads colors.** It does *not* map Omarchy theme names to built-in themes. It parses `[colors.primary|cursor|selection|normal|bright]` sections of alacritty.toml for lines `key = "#rrggbb"`. Other sections and non-6-hex values are ignored. Keys become `background`, `foreground`, `dim_foreground`, `cursor_*`, `selection_*`, `normal_<color>` and `bright_<color>`.
- **Helpers**:
  - `lighten(hex,p)` / `darken(hex,p)`: add or subtract `255*p/100` to each channel, clamped and rounded.
  - `blend(fg,bg,f) = fg + (bg-fg)*f`.
  - `dark_background?(hex) = 0.2126R+0.7152G+0.0722B (linear 0-1, NO gamma) < 0.5`; true when nil.
  - `ensure_contrast(c, code_bg)`: if the WCAG ratio is below 4.51, binary-search 24 iterations blending c toward white or black (whichever contrasts more with the bg) until the ratio is at least 4.51.
- **Mapping** (`map_variables`). Defaults: `bg=background||#121212`, `fg=foreground||#d0d0d0`, `blue=normal_blue||#7ea7c9`, `bright_black=normal_black||#555555`, `dark=dark_background?(bg)`, `code_bg = dark ? lighten(bg,7) : darken(bg,7)`.

| var | value |
|---|---|
| bg-primary | bg |
| bg-secondary | dark ? lighten(bg,3) : darken(bg,3) |
| bg-tertiary | ±7 |
| bg-hover | ±12 |
| text-primary | fg |
| text-secondary | blend(fg,bg,0.08) |
| text-muted | dim_foreground ‖ blend(fg,bg,0.45) |
| text-faint | blend(fg,bg,0.65) |
| border | ±12 |
| accent | blue |
| accent-hover | bright_blue ‖ lighten(blue,15) |
| accent-text | bg |
| selection | selection_background ‖ blend(fg,bg,0.25) |
| selection-text | selection_text ‖ fg |
| scrollbar | normal_black (bright_black var) |
| scrollbar-hover | lighten(that,10) |
| success | normal_green ‖ #8673d4 |
| warning | normal_yellow ‖ #ca2edc |
| error | normal_red ‖ #ff4da6 |
| code-bg | code_bg |
| heading-1 | normal_magenta ‖ #b683c3 |
| heading-2 | blue |
| heading-3 | normal_cyan ‖ #8adb8a |
| folder-icon | blue |
| file-icon | dim_fg ‖ blend(fg,bg,0.45) |
| config-icon | normal_yellow ‖ #ca2edc |
| syntax-keyword | normal_magenta ‖ normal_red ‖ #c792ea |
| syntax-function | normal_yellow ‖ bright_yellow ‖ #ffd866 |
| syntax-type | normal_cyan ‖ bright_cyan ‖ #65d5cb |
| syntax-string | normal_green ‖ bright_green ‖ #95d96a |
| syntax-number | bright_yellow ‖ normal_yellow ‖ #ffad66 |
| syntax-constant | bright_cyan ‖ normal_cyan ‖ #79c0ff |
| syntax-identifier | fg |
| syntax-property | bright_blue ‖ blue |
| syntax-operator | normal_white ‖ fg |
| syntax-comment | dim_fg ‖ normal_black ‖ blend(fg,bg,0.45) |
| syntax-punctuation | normal_white ‖ fg |
| syntax-invalid | normal_red ‖ #ff8787 |

All 12 syntax colors pass through `ensure_contrast(color, code_bg)`. ±N means lighten when dark, darken when light. The odd purple/pink fallbacks for success/warning/error are verbatim from the source.

- **Client** (`theme_controller.js:216-327`):
  - On connect, the client probes the endpoint. A 200 appends `{id:"omarchy", name:"Omarchy", icon:"sync"}` to the menu, as the last item after the custom entry if present.
  - Applying `omarchy`:
    1. `GET` the endpoint.
    2. Inject or replace `<style id="omarchy-theme-vars">[data-theme="omarchy"]{...vars...}</style>` in `<head>`.
    3. Set `data-theme="omarchy"` and toggle `.dark` from `is_dark`.
    4. Start **polling every 3000 ms**.
  - Polling re-injects the vars and `.dark` **only when `theme_name` changes**. A palette edit under the same name is not picked up. Polling errors are silently ignored.
  - On a non-OK response (at apply time or during polling), or a thrown error at apply time: fall back to `dark`, save `theme=dark` to config, re-apply and re-render.
  - A generation counter discards stale async results when the user switches theme mid-request.

---

### E.4 Custom theme editor

Files: `app/views/notes/dialogs/_custom_theme_editor.html.erb`, `_custom_theme_color.html.erb`, `custom_theme_editor_controller.js`, `lib/custom_theme.js`, `app/services/custom_theme.rb`, `components/custom-theme.css`.

#### E.4.1 Data model (palette JSON, `CustomTheme.parse`)
- Shape: `{"version":1, "name":String, "base":<built-in id>, "colors":{<key>: "#rrggbb", ...}}`. No other top-level keys are allowed. Raw input over 16 KiB is rejected.
- `name` is trimmed to 1-60 chars with no control characters. Server `[[:cntrl:]]`; client `/[\u0000-\u001f\u007f]/`.
- `base` must be one of the 18 built-in ids. `custom` and `omarchy` are not allowed.
- `colors` is **sparse**: only overridden keys, a subset of the 38 `COLOR_KEYS` (the variable names without `--theme-`). Values must be `#` plus exactly 6 hex digits and are lowercased on save.
- Invalid input raises ArgumentError. A `.fed` value that fails to parse is ignored with a warning (`config.rb:530-537`).
- `BASIC_COLORS = accent, bg-primary, text-primary` are shown up-front. The other 35 are under "Advanced colors".

#### E.4.2 Dialog layout (native `<dialog class="custom-theme-editor">`, modal)
- Size: `width:min(100vw-2rem, 48rem)`, `max-height:100dvh-2rem`, radius 1rem, bg-primary, border. Backdrop: `bg-primary 75%`.
- **Header**: h2 "Custom theme" plus the description "Start from a theme and preview your changes.", and a close (X) button labelled `common.close`.
- **Body** (scrollable): a card of rows. Each row is a 2-column grid (label | control), min-height 3rem, with separators between rows.
  1. "Theme name": text input, `maxlength=60`, required. Gets `aria-invalid` when invalid.
  2. "Base theme": `<select>` filled by JS with the 18 built-in themes (id/name).
  3. Basic colors: Accent, Background, Text, in the order `accent`, `bg-primary`, `text-primary`.
- Each color row has a label (`settings.custom_theme.colors.<key with _>`) and two controls:
  - a round `<input type=color>` swatch, 1.75rem, `aria-label="Choose <name>"`;
  - a text hex input `.custom-theme-hex`: `maxlength=7`, `pattern=#[0-9a-fA-F]{6}`, monospace, 6rem wide, `spellcheck=off`.

  Both fire `onColor` on `input`.
- Hint: "Fonts and sizes are available in Editor settings."
- **Live preview card** (`aria-label="Theme preview"`):
  - h3 "A note of your own" in heading-1;
  - paragraph "Your words, in your colors." plus a span "Example link" (accent, underlined);
  - `<pre>` on code-bg showing `const message = "Hello, world!";` with keyword, identifier and string colors.
- `<details>` "Advanced colors" holds a card with the remaining 35 keys in `COLOR_KEYS` order:
  - bg-secondary, bg-tertiary, bg-hover;
  - text-secondary, text-muted, text-faint;
  - border, accent-hover, accent-text;
  - selection, selection-text, scrollbar, scrollbar-hover;
  - success, warning, error, code-bg;
  - heading-1..3;
  - folder-icon, file-icon, config-icon;
  - the 12 syntax-* keys.

  English labels are in E.6.7.
- **Feedback area**: a warning `<p role=status>` for contrast and an error `<p role=status>`. Both are hidden when empty.
- **Footer**: "Reset colors" (borderless, muted) on the left. On the right, "Cancel" and "Save" (accent bg, accent-text, `type=submit`).
- At ≤480px, non-hex text inputs and selects are 9rem wide. Focus-visible is a 2px accent outline. Disabled buttons get opacity 0.5 and `cursor:wait`.

#### E.4.3 Behavior
- **Open** (from "Customize theme"):
  - snapshot = `{id: currentThemeId, custom: deep copy}`; remember the trigger element;
  - draft = a copy of the saved palette if it has a base. Otherwise `{version:1, name:"My theme" (localized), base: current theme if built-in, else system-pref dark/light, colors:{}}`;
  - populate the base select, set name and base, clear status, run `preview()` and then `syncFields()`, call `showModal()` and focus the name field.
- **Live preview** is global. `themeOutlet.previewCustom(draft)` sets `_customPreview` and `currentThemeId="custom"` and re-applies: the base `data-theme` plus inline color overrides. **The whole app recolors live.** The theme menu shows a "custom" entry named after the draft.
- `syncFields()` writes the **effective computed color** of every key (base plus overrides) into both inputs, `#000000` if unreadable, and clears invalid flags.
- **Name input** updates draft.name, then preview, then save state.
- **Base change** sets draft.base, previews, and re-syncs the fields. Existing overrides in `draft.colors` are **kept**.
- **Color input**: normalize (trim, lowercase, `^#[0-9a-f]{6}$`).
  - Valid: store it in `draft.colors[key]`, mirror it to the sibling input, clear the error, preview.
  - Invalid: add the key to the invalid set, set `aria-invalid`, show "Use a six-digit hex color, such as #336699."
- **Contrast check** after each preview computes WCAG ratios for text-primary on bg-primary and accent-text on accent. If the lowest is below 4.5, it shows "Low contrast (X.XX:1). Adjust text, background, or accent colors." This is a warning only and does not block saving.
- **Save enabled** iff not busy, the name is valid, and no fields are invalid.
- **Reset** clears `draft.colors` and the invalid set, previews, and re-syncs the fields to base colors. The name and base stay.
- **Save** (form submit):
  - Busy state: the fieldset and all buttons are disabled and `aria-busy=true`.
  - Request: `theme.saveCustomTheme`:
    1. cancels any pending theme debounce;
    2. awaits any in-flight theme save;
    3. sends `PATCH /config {theme:"custom", custom_theme: JSON.stringify(draft)}`.
  - Success requires `response.ok` and `settings.custom_theme` in the response. It then:
    1. sets `customValue` and `currentThemeId` from the server;
    2. re-applies;
    3. dispatches `frankmd:config-file-modified`;
    4. closes the dialog.
  - Failure shows "Could not save the theme. Your changes are still here." and keeps the dialog open.
- **Close or cancel without saving** (X, Cancel, Escape, or a click on the backdrop itself): `restoreTheme(snapshot)` reverts the theme id and palette and re-applies. Escape and close are blocked while busy. Focus returns to the trigger ("Customize theme").
- Tested behaviors (`test/system/custom_theme_editor_test.rb`):
  - The note content is not modified.
  - `.fed` gets `theme=custom` and the `custom_theme` JSON.
  - After reload the accent persists as an inline `style="--theme-accent: #..."` on `<html>`.
  - Escape after editing reverts to the saved palette.
  - At a 390×844 viewport the dialog fits with no horizontal page overflow.
  - `#bad` disables Save.

---

### E.5 Settings dialog

Files: `app/views/notes/_settings_panel.html.erb` (rendered in `notes/index.html.erb:72`), `_settings_button.html.erb`, `notes/settings/_about.html.erb`, `notes/_media_storage_menu.html.erb`, `settings_controller.js`, `components/settings.css`.

#### E.5.1 Opening and chrome
- **Entry point**: a gear icon button (16px) in the **sidebar footer navigation**, after the Home and Library buttons. Classes `settings-toggle workspace-icon`, `title`/`aria-label` = "Open Settings" (`header.open_settings`). Action `app#openSettings` calls `settings.openDialog()`. There is no keyboard shortcut. When the sidebar is collapsed, the sidebar must be opened first (as the tests do).
- Native modal `<dialog id="settings-dialog">`:
  - `w-[calc(100%-2rem)] max-w-4xl h-[680px] max-h-[85vh] overflow-y-auto`, rounded-lg, shadow-xl;
  - backdrop black/50; bg-secondary, border, text-primary;
  - `aria-labelledby=settings-title`, `aria-modal`.
  - The dialog element **stays mounted while closed**, so its Stimulus values drive app config at all times.
- `openDialog()`:
  1. refreshes the media-storage state (`GET /config`);
  2. syncs the controls from the values;
  3. calls `showModal()`;
  4. focuses the first nav button (General);
  5. dispatches `settings:dialog-state-changed {open:true}`.

  It dispatches `{open:false}` on close.
- Close paths: the X button (top-right, 36px square, bordered, labelled `common.close`), a click on the backdrop (target === dialog), and Escape (native). Focus returns to the invoker natively.
- Header: h1 "Settings" (text-xl semibold).
- Layout: `flex-col` on mobile, `sm:flex-row`. A left nav `sm:w-44`, `aria-label` "Settings categories", and a right content area.
- **Nav categories**: buttons `.settings-nav-button` with an icon and a label, using `aria-pressed`. The pressed state has bg-hover and accent text. Order:
  1. **General** (gear)
  2. **Editor** (pencil-square)
  3. **Writing** (typewriter)
  4. **Preview** (eye)
  5. **About** (info circle)

  Clicking a button shows its section and hides the others with `.hidden`. The current category is kept in memory only (not persisted). After a reopen the last category stays visible, but focus goes to the General button.
- Each section has an uppercase title `.settings-section-title` (0.75rem, 600, letter-spacing 0.05em, text-faint), then `.settings-card` rows:
  - flex, space-between, gap 1rem;
  - padding 0.875rem × 1rem; border; radius 0.5rem; bg-secondary;
  - 0.5rem between cards.

  The left side holds a label (text-sm medium) plus an optional description (text-xs muted).
- Controls:
  - **`.settings-switch`**: a pill button, 2.5rem × 1.375rem, driven only by `aria-pressed`. Off: bg-tertiary track, text-secondary knob. On: accent track and border, accent-text knob translated 1.0625rem. Hover: accent border. The transition is 0.15s.
  - **Selects**: `w-48`, bg-primary, border, `focus:ring-2` accent.
  - **`.settings-segmented`**: 2 buttons. The pressed one gets accent bg and accent-text.
  - **`.settings-range`**: a 0.25rem track (bg-tertiary) and a 1rem accent thumb with a 2px bg-secondary border.
- A status line `<p data-settings-target=status role=status aria-live=polite>` sits below the sections. It shows "Failed to save setting" for 2 s when a PATCH is not OK.

#### E.5.2 Persistence model (all settings)
- Everything persists to the server `.fed` via `PATCH /config` (JSON body of `{key: value}`), with whitelist `Config::UI_KEYS` (`config.rb:96-114`).
- There is **no localStorage**.
- Writers:
  - settings controller: `saveSetting`, one shared 500 ms debounce timer. Only the last pending call in the window is sent.
  - app controller: `saveConfig`, its own 500 ms debounce.
  - theme controller: its own 500 ms debounce.
  - locale and media-storage: immediate.
- Every successful write dispatches `frankmd:config-file-modified`.
- Values are applied live via Stimulus value-changed callbacks.
- Reload path: `settings.reload()` sends `GET /config` and assigns each value only if it is non-null and changed.

#### E.5.3 Every setting

| Section | Label (en) / description | Control & options | Default | `.fed` key | Apply effect |
|---|---|---|---|---|---|
| General | **Storage for new media** | Dropdown button (`w-40`) + popover menu: `Local`, `S3`. S3 is disabled with title "This provider is not configured." unless S3 credentials are configured. An error line appears below (`role=alert`) | `local` | `media_storage_provider` | Immediate `PATCH` (no debounce) through the `media-storage` controller. The server rejects unknown values, or `s3` when S3 isn't configured. While pending, the button and options are disabled. On failure: "Could not save the storage provider. The previous selection is still active." If S3 later becomes unavailable, the server reports `media_storage_fallback`. A modal warning ("Upload storage changed" / "S3 is no longer available. Your upload provider has been switched to Local.") then auto-dismisses after 6 s |
| General | **Theme** / "Color palette for the whole app" | Theme dropdown (E.2.5) + "Customize theme" link (E.4) | `nil`, which follows the system light/dark preference | `theme`, `custom_theme` | E.2 |
| General | **Language** / "Interface language (reloads the app)" | Locale dropdown (E.6.3) | `en` (env `FRANKMD_LOCALE`) | `locale` | Immediate PATCH, then **full page reload** |
| Editor | **Document view** / "How the editor and preview share the workspace" | Segmented: `Split View` / `Single View` (`data-view-mode=split/single`) | `split` | `view_mode` | `app.setViewMode(mode)`. Any value other than `single` becomes `split` |
| Editor | **Font family** | Select. Each option renders in its own font: Cascadia Code (`cascadia-code`, `'Cascadia Code', monospace`), Consolas (`consolas`), DejaVu Sans Mono (`dejavu-mono`, `'DejaVu Mono', monospace`), Fira Code (`fira-code`), Hack (`hack`), JetBrains Mono (`jetbrains-mono`), Roboto Mono (`roboto-mono`), Source Code Pro (`source-code-pro`), Ubuntu Mono (`ubuntu-mono`) | `cascadia-code` | `editor_font` | `codemirror.setFontFamily(family)` + `setFontSize` |
| Editor | **Font size** (same card) | Select: 12, 13, 14, 15, 16, 18, 20, 22, 24 (`px` labels) | 14 | `editor_font_size` | `codemirror.setFontSize` |
| Editor | Preview box (same card), labelled "Preview" | Bordered box (bg-secondary, p-3, line-height 1.6) in the selected font and size. Shows `// The quick brown fox`, `function hello() {`, `  return "Hello, World!";`, `}` | | | Updates live on font/size change |
| Editor | **Editor width** / "Maximum text column width" | Range 40-200, step 8, plus label `Nch` (w-12, tabular-nums) | 72, clamped to [40,200] in the view | `editor_width` | Sets `--editor-width: Nch` on `<html>`. Saved on every `input` event (debounced). The app also has increase/decrease commands with step 8, min 40, max 200, and a toast "Editor width: Nch" / "Maximum width (200ch)" |
| Editor | **Line numbers** | Select: Off (0), Absolute (1), Relative (2) | 0 | `editor_line_numbers` | `codemirror.setLineNumberMode`. The app's toggle cycles off → absolute → relative → off |
| Editor | **Stats** / "Show document statistics in the Explorer" | Switch | false | `show_stats` | `app.showStatsPanel()` + `updateStats()` |
| Editor | **Indentation** / "Spaces per indent level" | Select: "2 spaces", "4 spaces", "8 spaces" (`%{count} spaces`) | 2 | `editor_indent` | Value stored. The `.fed` comment says 0 = tab, 1-6 = spaces, but the UI only offers 2/4/8 |
| Writing | **Typewriter mode** / "Center the cursor for distraction-free writing" | Switch, handled by the app controller (`app#toggleTypewriterButton`) | false | `typewriter_mode` | The typewriter controller toggles. On `typewriter:toggled` the app saves the value, updates `aria-pressed`, and activates only for markdown files |
| Writing | **Center cursor after clicking** / "Recenter the cursor after a click, with Typewriter mode on or off" | Switch | false | `cursor_recenter_on_click` | `codemirror.setCursorRecenterOnClick` |
| Writing | **Vim mode** / "Vim keybindings in the editor" | Switch (`app#toggleVimMode`) | false | `vim_mode` | `codemirror.setVimMode`. The button also gets the accent text class |
| Writing | **Scroll sync** / "Scroll the preview with the editor" | Switch (`app#toggleScrollSync`, also Ctrl+Shift+\\) | true (`nil` means on; only an explicit `false` turns it off) | `scroll_sync` | `preview.syncScrollEnabledValue` |
| Preview | **Preview zoom** / "Zoom level for the rendered preview" | Select: 50, 75, 90, 100, 110, 125, 150, 175, 200 (`%` labels). These match the preview's +/- steps. A non-listed value shows as 100 | 100 | `preview_zoom` | `preview.zoomValue` |
| About | (see E.5.4) | | | | |

Other UI keys that are persisted but have no control in this dialog: `sidebar_visible` (default true) and `recent_files` (JSON).

Note: `settings_controller.applyLineNumbers` calls `normalizeLineNumberMode(v, "off")` with a *string* fallback, a latent bug (`settings_controller.js:179`). Valid values are 0, 1 and 2.

#### E.5.4 About panel (`notes/settings/_about.html.erb`)
- A centered card (border, bg-primary, p-6) containing, from top to bottom:
  1. `/icon.svg` logo at 128×128;
  2. "FrankMD" (`app.name`, text-xl bold);
  3. the description "A modern markdown editor for your notes" (muted, text-sm);
  4. `v<app_version>` (faint, xs);
  5. `© <current year> Fabio Akita. All rights reserved.` (faint, xs);
  6. a button-styled link "View on GitHub" with the GitHub mark, pointing to `https://github.com/akitaonrails/FrankMD` (new tab, `noopener noreferrer`, bg-tertiary, `hover:bg-hover`).

#### E.5.5 Legacy partial
`app/views/config/_editor_config.html.erb` (served by `GET /config/editor`) declares `data-controller="editor-config"`. **No such controller exists** any more; it was absorbed into `settings_controller`, see the comment at `:7`. Nothing in JS fetches `/config/editor`. It is dead code and can be dropped.

---

### E.6 Client i18n

#### E.6.1 Locales
- Source: `config/application.rb:44-46`: `available_locales = [:en, :'pt-BR', :'pt-PT', :es, :he, :ja, :ko]`, default `:en`, `fallbacks = true`.
- Files: `config/locales/{en,es,he,ja,ko,pt-BR,pt-PT}.yml`. **Each has 692 leaf keys in the same 18 top-level sections**, with no missing or extra keys compared to en (verified mechanically).
- Sections: `home, app, common, languages, header, sidebar, library, settings, editor, preview, context_menu, status, connection, dialogs, errors, confirm, success, media_storage`.

| id | Menu name (`locale_controller.js:10-18`) | Flag emoji | Emoji-search data locale |
|---|---|---|---|
| `en` | English | 🇺🇸 | en |
| `pt-BR` | Português (Brasil) | 🇧🇷 | pt |
| `pt-PT` | Português (Portugal) | 🇵🇹 | pt |
| `es` | Español | 🇪🇸 | es |
| `he` | עברית | 🇮🇱 | en (fallback) |
| `ja` | 日本語 | 🇯🇵 | ja |
| `ko` | 한국어 | 🇰🇷 | ko |

The `languages.*` YAML keys duplicate these names but the JS list is hardcoded and does not use them.

#### E.6.2 Server-side locale resolution
`ApplicationController#set_locale` (`application_controller.rb:13-19`) checks, in order:

1. the `?locale=` URL param;
2. the `.fed` `locale` value (default `en`, env `FRANKMD_LOCALE`);
3. `I18n.default_locale`.

The value applies only if it is in `available_locales`. ERB views are rendered server-side with `t(...)`, so **most UI strings are baked into the HTML**. `<html lang="<%= I18n.locale %>">` (`layouts/application.html.erb:2`).

#### E.6.3 How JS gets translations
- **Endpoint `GET /translations`** (`translations_controller.rb`) returns `{locale: "pt-BR", translations: {...}}`.
  - It includes **only these sections**: `common, dialogs, status, errors, success, editor, sidebar, preview, context_menu, connection, library`.
  - Each section is the requested locale **deep-merged over en**, so missing keys fall back to English strings, not dotted keys.
- Loaded once by `locale_controller.connect()` (the controller element lives in the Settings dialog, which is always mounted). It then sets:
  - `window.frankmdTranslations`;
  - `window.frankmdLocale`.

  It also dispatches `window` event `frankmd:translations-loaded {locale, translations}` and updates the button label.
- **`window.t(key, opts)`** (`locale_controller.js:175-200`):
  - walks dot-path keys;
  - returns the **key string itself** if a key is missing or not a string;
  - interpolates `%{name}` from opts and leaves unknown placeholders untouched;
  - has no pluralization.

  Before translations load, every call returns the key.
- **Gap/bug**: JS calls a few keys from sections that are *not exported*. The UI shows the raw key for these.
  - `window.t("settings.save_failed")` (`settings_controller.js:245`)
  - `media_storage.save_failed`, `media_storage.load_failed` (`media_storage_controller.js`)
  - 3× `confirm.*`

  A reimplementation should simply expose all keys.
- Strings for custom-theme messages are passed in as Stimulus values rendered server-side (`data-custom-theme-editor-*-value`). The contrast template keeps the literal `%{ratio}`, which the JS replaces.

#### E.6.4 Language picker UI
- Located in the General section, "Language" card. The button and menu match the theme picker: `w-40` button, `title` "Change Language", `aria-controls=settings-locale-menu`.
- Initial button text is server-rendered with a quirk: `"Portugues"` if the locale is pt-BR, else `"English"` (`_settings_panel.html.erb:174`). It is replaced by the proper name after `/translations` loads.
- Menu items are `button[role=menuitemradio][data-locale]`, each with a flag emoji (`text-base`), the native name and a checkmark.
- Selecting the current locale just closes the menu. Selecting another one:
  1. closes the menu;
  2. updates the label and menu;
  3. sends `PATCH /config {locale}` immediately (no debounce);
  4. calls **`window.location.reload()` unconditionally**, even if the PATCH failed.
- A `frankmd:config-changed` listener with `detail.locale` exists, but **nothing dispatches `locale` in that event** (the settings controller sends only theme and customTheme). Editing `locale` in `.fed` therefore takes effect only on the next page load.

#### E.6.5 RTL / Hebrew
- **No RTL support.** Nothing sets `dir="rtl"`, neither the layout nor JS, even for `he`. There is no RTL stylesheet. Hebrew strings render in an LTR layout; the browser bidi algorithm handles text runs only.
- Partial readiness:
  - some component CSS uses logical properties (`border-inline-end`, `margin-inline`, etc.; about 19 occurrences in `components/*.css`);
  - CodeMirror content uses `marginInline:auto`;
  - `FloatingDropdown` left-aligns menus when the button's computed `direction` is `rtl` (`floating_dropdown.js:108`).

#### E.6.6 Emoji locale data (`lib/emoji_i18n.js`)
- `LOCALE_MAP`: en→en, pt-BR→pt, pt-PT→pt, es→es, ja→ja, ko→ko, he→en.
- The emoji picker calls `setEmojiLocale(document.documentElement.lang)` when it opens.
- `loadLocaleData(loc)` fetches the static file `/emoji-data/<loc>.json` (`public/emoji-data/{en,es,ja,ko,pt}.json`, about 570-600 KB each, 1949 entries). It caches per locale and falls back to `en` on failure.
- Entries are emojibase objects `{hexcode, label, unicode|emoji, tags?}`.
- `buildSearchIndex` maps each emoji char to `{label, tags, searchTerms: lowercased "label tags..."}`. The translated terms are an extra search layer on top of the English shortcodes.
- `preloadEmojiData(appLocale)` is exported; its call sites are outside this section's files.

#### E.6.7 English labels of custom theme color keys (`settings.custom_theme.colors.*`)
bg_primary Background · bg_secondary Panel background · bg_tertiary Raised background · bg_hover Hover background · text_primary Text · text_secondary Secondary text · text_muted Muted text · text_faint Faint text · border Border · accent Accent · accent_hover Accent hover · accent_text Accent text · selection Selection background · selection_text Selection text · scrollbar Scrollbar · scrollbar_hover Scrollbar hover · success Success · warning Warning · error Error · code_bg Code background · heading_1..3 Heading 1..3 · folder_icon Folder icon · file_icon File icon · config_icon Configuration icon · syntax_keyword Code keywords · syntax_function Code functions · syntax_type Code types · syntax_string Code strings · syntax_number Code numbers · syntax_constant Code constants · syntax_identifier Code identifiers · syntax_property Code properties · syntax_operator Code operators · syntax_comment Code comments · syntax_punctuation Code punctuation · syntax_invalid Invalid code.

---
---

## 13. Known quirks, dead code and doc-vs-code discrepancies (decide per item: preserve or fix)

**Shortcuts and editor**
- The preview toggle is **Ctrl+Shift+V** in code, but the English tooltip `header.toggle_preview` says "Ctrl+Shift+P".
- The help dialog says F1 toggles help. F1 only opens it.
- CodeMirror's own keymaps don't stop propagation, so Ctrl+F, Ctrl+G, Ctrl+Enter, Ctrl+M and Ctrl+Shift+\ may run both the CM default and the FrankMD action. Ctrl+Enter may insert a line before toggling the task (inferred).
- Ctrl+Shift+\ (scroll sync) matches `\` with Shift, but a US layout reports `|`, so it may never fire.
- The `editor_indent` setting (2/4/8) is saved but never applied. Tab always indents by 2 spaces.
- Fenced code is highlighted per language only in the preview. The editor shows fenced code without language highlighting.
- README "Typewriter mode hides explorer/closes preview" is outdated. The current code force-shows the preview in split mode so it can follow the cursor.

**Data safety**
- Content-loss guard is `lostPercent > 0.2 && lostChars > 50`, on net length against last saved content. It blocks the save and shows the banner (no confirm dialog).
- The save request sends only `{content}`, so the last write wins. Conflicts are detected on load only, by comparing the draft's base SHA-256 revision with the file's.
- The offline banner says editing is disabled, but only the hidden textarea is disabled. CodeMirror stays editable.
- No multi-tab coordination; two tabs overwrite each other's draft (inferred). No storage quota/retention logic. No save on blur.
- PWA manifest and service worker exist but aren't routed.

**Preview**
- No footnotes, mermaid, heading anchors, Hugo shortcode rendering or print/export. `\(..\)` / `\[..\]` math delimiters aren't supported.
- DOMPurify strips `target`, so external links probably navigate in the same tab via Turbo (inferred).
- `data-source-line` is assigned in order to top-level blocks only, so scroll sync can drift around nested blocks. Use real parser source positions in the rewrite.

**Explorer / Home**
- Folder expansion state is not persisted across reloads.
- The tree has no arrow-key navigation, no Enter-to-open and no scroll-into-view of the selection.
- Deleting a non-empty folder fails, although the confirmation says "and all its contents".
- Dropping onto a file row moves the item to root.
- The `.fed` gear icon is effectively dead: dotfiles are hidden from the tree and the server refuses `.fed` via notes routes. `.fed` is edited via Settings / log viewer config tab instead (see E.5, D2.7).
- Ctrl+P's empty-query list is ordered by file mtime, not by the Home recent-files history.
- `_context_menu.html.erb` and `dialogs/_rename.html.erb` are never rendered. The live versions are in `_file_operations.html.erb`. `config/_editor_config.html.erb` is also dead.
- The backlinks endpoint `GET notes/*path/backlinks` exists, but no client code calls it, despite the README claim.
- The `sidebar_visible` config key isn't read by the client.

**Dialogs / media**
- There is no link dialog. Ctrl+K and the text-format menu insert a link template.
- The table editor has no drag reordering and no alignment control. Pipes aren't escaped and alignment colons are lost on edit, despite the README saying "drag-and-drop rows/columns".
- The emoji picker inserts `:shortcode:`, not the Unicode character. The preview converts it. Icons insert `:ph-name:` (201 Phosphor icons).
- AI grammar check sends the whole saved file and replaces the whole document on accept.
- The "cursor in table" hint says "click to edit" but isn't clickable.
- Dropping OS files onto the editor has no FrankMD handler (CodeMirror default behavior applies). There is no upload progress bar: only spinner/"Uploading…" text inside dialogs. Paste of clipboard images is handled (D1.6).

**Themes / i18n**
- A few JS keys (`settings.save_failed`, two `media_storage.*`, three `confirm.*`) are missing from the `/translations` payload, so raw keys show.
- Changing `locale` in `.fed` applies only after a reload.
- The OS light/dark preference is read once, with no `matchMedia` change listener.
- Omarchy polling re-applies colors only when the theme *name* changes.
- Hebrew has no RTL layout (no `dir` attribute set), even though some CSS uses logical properties and the overflow menu is RTL-aware.
- Several user-facing strings are hard-coded English: editor width messages, and "Preview/Typewriter mode is only available for markdown files".

---

## 14. Feature checklist

Stable IDs for tracking the rewrite. They are grouped by area; the area tags match the sections above.

### LAYOUT

- **FE-001** Boot splash (purple→pink gradient, logo, spinner) removed with 200 ms fade once the editor is ready
- **FE-002** Deep-link URLs `/notes/<path>.md` (and `?file=`) open a note; history push on open, `/` for Home; back/forward navigates notes
- **FE-003** Stale async navigation results discarded (navigation generation counter)
- **FE-004** 401 from any request redirects to the login page (access-token login screen)
- **FE-005** Two amber warning banners above the workspace: connection lost (Retry) and content-loss (Undo / Save anyway)
- **FE-006** Sidebar 16rem: header actions, tree, optional stats panel, footer with brand + Home / Library / Settings
- **FE-007** Floating "open explorer" button when the sidebar is hidden
- **FE-008** Header: document title (basename, click to inline-rename), save-status text, toolbar
- **FE-009** Inline title rename: auto-sized input, Enter/blur submit, Escape cancel, inline error, disabled for `.fed`
- **FE-010** Toolbar with 10 actions (image, find, preview, video, code, table, emoji, ai, copy, help); image/find/preview pinned by default
- **FE-011** Overflow "⋯" menu listing all actions with pin toggles; pins persisted in sessionStorage `frankmd.workspacePins`
- **FE-012** Overflow menu keyboard navigation (arrows, Home/End, Left/Right to pin, RTL-aware, Tab/Escape close) and viewport clamping
- **FE-013** Action visibility rules: editing actions only for open markdown file; disabled in single-mode preview; copy only with file; header hidden on Home
- **FE-014** Preview zoom controls (−, %, +) in header only while preview visible
- **FE-015** Copy whole document to clipboard with toast
- **FE-016** Split vs single view mode (persisted `view_mode`); single mode swaps editor ↔ full-width preview without destroying editor state
- **FE-017** Library workspace replaces editor/preview and restores previous preview visibility on close; Escape closes it
- **FE-018** Settings opens as a modal dialog
- **FE-019** Mobile (<768px): overlay sidebar with backdrop, inert main, focus trap, Escape/backdrop/open-note closes; header wraps
- **FE-020** Editor text column width in ch: default 72, 40–200, step 8 (Ctrl+Shift+=/−) with temporary message; persisted `editor_width`
- **FE-021** No resizable splitters (fixed 16rem sidebar, 40% preview)
- **FE-022** "Cursor in table" hint badge bottom-right of editor
- **FE-023** Vim mode badge bottom-right of editor
- **FE-024** File-not-found state: red path in header, editor disabled, back to Home after 5 s
- **FE-025** Bottom-center temporary message (2 s default, 5 s for errors), single instance
- **FE-026** `#app-toast` polite live-region toast with slide/fade (2 s)
- **FE-027** Themed alert/confirm modals (appAlert/appConfirm) instead of native dialogs

### KEYS

- **FE-028** Global shortcut matcher: exact `event.key` (case-sensitive), Ctrl or Cmd as "ctrl", exact Shift/Alt, preventDefault on match
- **FE-029** All global shortcuts disabled while Settings dialog is open
- **FE-030** Ctrl+N opens New Note dialog
- **FE-031** Ctrl+S saves immediately
- **FE-032** Ctrl+Shift+V toggles preview (markdown only, toast otherwise; single-view swaps panes)
- **FE-033** Ctrl+F opens Find dialog pre-filled with selection
- **FE-034** Ctrl+H opens Find dialog on Replace tab
- **FE-035** Ctrl+G opens Jump to Line dialog
- **FE-036** Ctrl+L cycles line numbers Off→Absolute→Relative and persists
- **FE-037** Ctrl+Shift+F opens Search in files
- **FE-038** Ctrl+P opens file finder (newest first)
- **FE-039** Ctrl+E toggles sidebar
- **FE-040** Ctrl+\ toggles typewriter mode (markdown only, toast otherwise)
- **FE-041** Ctrl+Enter toggles/adds task on selected lines (markdown only)
- **FE-042** Ctrl+Shift+\ toggles scroll sync and persists
- **FE-043** Ctrl+M opens text format menu at selection
- **FE-044** Ctrl+Shift+E opens emoji picker (markdown only)
- **FE-045** Ctrl+Shift++ / Ctrl+Shift+- change editor width by 8ch within 40–200 with toast
- **FE-046** Ctrl+Shift+O opens log viewer
- **FE-047** F1 opens Help dialog on Markdown tab
- **FE-048** Escape priority chain: title rename → overflow menu → explorer menu → mobile sidebar → Library preview/context menu/Library → help/context menus
- **FE-049** Ctrl+Y / Ctrl+Shift+Z with no open file redoes created-note deletion
- **FE-050** Explorer: Delete deletes selected items, F2 renames single selected item (no modifiers, no repeat)
- **FE-051** Explorer: Ctrl/Cmd+click toggles multi-select, Shift+click range select
- **FE-052** Workspace overflow menu arrow/Home/End/Tab navigation and Left/Right to pin toggle
- **FE-053** Mobile sidebar overlay Tab focus trap
- **FE-054** Dialog-local keys for file finder, content search, emoji grid, image/video/help tabs, code dialog, new/rename dialogs, prompts, floating dropdowns, AI abort, log viewer
- **FE-055** Help dialog tab switching with ArrowLeft/Right and mouse wheel; backdrop click closes
- **FE-056** Vim: unmap <C-f> <C-n> <C-p> <C-e> <C-b> <C-i> <C-v> on non-macOS so app keeps them

### EDITOR

- **FE-057** Mod-B / Mod-I / Mod-Shift-S / Mod-` wrap-or-unwrap selection with ** / * / ~~ / `
- **FE-058** Mod-K inserts link `[sel](url)` selecting "url", or `[](url)` with cursor in brackets
- **FE-059** Typing * _ ` ~ ^ = over a non-empty selection surrounds it (backtick fence length adapts), own undo step
- **FE-060** Standard CodeMirror default/history/search keymaps and indentWithTab available
- **FE-061** Enter continues lists/tasks/blockquotes; Backspace removes markup
- **FE-062** Undo at start of history / redo at end hands off to created-note undo/redo (non-vim)
- **FE-063** Per-note undo history preserved across note switches (LRU 20), remapped on rename/move, evicted on delete
- **FE-064** Read-only mode with lock tokens
- **FE-065** Soft line wrapping always on
- **FE-066** Content column max-width = editor width (ch), centered, padding 1rem 2rem
- **FE-067** Native selection with themed selection color; secondary ranges/cursors drawn by layers
- **FE-068** Rectangular (Alt+drag) selection
- **FE-069** Selection-match highlighting (≥3 chars, whole words) with faint accent outline
- **FE-070** Bracket matching highlight (match accent, mismatch red)
- **FE-071** Active line highlight effectively invisible
- **FE-072** Click in blank margin places caret at nearest text position (≤4px drag tolerance, scrollbar excluded)
- **FE-073** Image paste routed to image picker; plain text paste default; no smart URL-to-link paste
- **FE-074** Hidden textarea mirrors document content
- **FE-075** Auto-focus editor when opening an empty note
- **FE-076** Markdown syntax highlighting mapped to theme variables (headings 1–3 own colors, 4–6 accent, code accent on code-bg, etc.)
- **FE-077** Fenced code bodies styled as monospace (no per-language highlighting in editor)
- **FE-078** Line numbers: Off / Absolute / Relative (cursor line absolute, others distance), active gutter highlight
- **FE-079** Vim mode toggle in Settings, live, persisted; status line panel; themed block cursor
- **FE-080** Vim mode indicator "VIM · NORMAL/INSERT/VISUAL" bottom-right
- **FE-081** Vim ex-commands :w :q :wq :x :help :e :find :Explore :n :prev mapped to app actions
- **FE-082** Typewriter mode: cursor kept at 50% viewport, bottom padding half-height, skip during mouse/range selection, >5px threshold
- **FE-083** Typewriter mode: auto-show preview in split view, preview width matched, preview follows cursor line to top (smooth, >10px)
- **FE-084** Cursor-recenter-on-click setting
- **FE-085** Text format context menu (right-click with selection / Ctrl+M) with Bold/Italic/Strikethrough/Highlight/Subscript/Superscript/Link, hotkeys, arrows, toggle-unwrap
- **FE-086** Tab/Shift+Tab indent/outdent lines (2-space unit); editor_indent setting persisted (2/4/8) but not applied
- **FE-087** Task toggle surgical `[ ]`⇄`[x]`, non-task lines become `- [ ] …`; preview checkbox click toggles source line
- **FE-088** Slash command menu trigger rules (line start/after space or bracket, paragraph/heading only, not code/URL, not vim normal, markdown only)
- **FE-089** Slash commands: Heading 1–3, Bulleted list, Numbered list, To-do list, Quote, Code block, Equation, Divider
- **FE-090** Slash commands: Table, Image, Video, Emoji open dialogs and replace the /query on insert (validated)
- **FE-091** Slash command icons and localized labels with fuzzy filter text
- **FE-092** Wikilink `[[` autocomplete from Explorer .md files, scored name-prefix > name-contains > path-contains, inserts `path]]`
- **FE-093** Block content insertion normalizes blank lines around tables/code/video; images single newline
- **FE-094** Find & Replace non-modal dialog with Find/Replace tabs, case-sensitive and regex toggles, match counter, prev/next with wrap
- **FE-095** Find: Enter next, Shift+Enter previous, Alt+Enter replace+advance, Escape close
- **FE-096** Replace current (button advances), Skip, Replace All with "Replaced N" feedback, regex $-substitutions
- **FE-097** Find minimize to floating mini-bar with prev/next/skip/count/restore
- **FE-098** Invalid regex error shown in red in counter
- **FE-099** Jump to Line dialog: absolute, +N/-N relative, -1 = end, clamped; centers line and focuses editor
- **FE-100** Stats panel in sidebar: words, characters, UTF-8 size, read time (200 wpm), line/total; 500ms debounce; toggle in Settings
- **FE-101** Table-at-cursor hint "Cursor in table - click to edit" (200ms debounce)
- **FE-102** Editor fonts list (9) with vendored woff2 files; Inter UI font
- **FE-103** Font size select 12–24px (default 14); fixed line height 1.6; font preview in Settings
- **FE-104** Editor width slider 40–200ch step 8 (default 72)
- **FE-105** Preview code-block syntax highlighting for listed languages with lazy parsers and size limits
- **FE-106** Terraform/HCL custom highlighter
- **FE-107** Code dialog language suggestions list and unknown-language confirmation
- **FE-108** CodeMirror search panel / tooltip / scrollbar theming
- **FE-109** Help dialog with Markdown cheatsheet, Keyboard Shortcuts, Vim Mode tabs

### SAVE

- **FE-110** Debounced autosave: PATCH 2000 ms after the last edit
- **FE-111** Forced autosave at most 30000 ms after the first unsaved change, even while typing
- **FE-112** Save uses a snapshot of path, file version, content and revision taken when it was scheduled; stale snapshots are dropped
- **FE-113** Ctrl+S saves immediately (blocked while the Settings dialog is open)
- **FE-114** Vim `:w` saves and `:wq` saves then closes dialogs
- **FE-115** No save on blur
- **FE-116** File switch, Home and back/forward write the local draft synchronously without a server save; navigation is aborted if the draft cannot be stored
- **FE-117** Only one save request runs at a time; extra save calls are dropped
- **FE-118** After a save, if the editor changed during the request, write a draft and schedule another save
- **FE-119** Ignore save responses for a file that was switched, renamed or deleted, but record the returned revision
- **FE-120** Save endpoint `PATCH /notes/<encoded path>` with body `{content}`, response `{revision}`; no revision is sent, so the last write wins
- **FE-121** Save status: hidden / "Unsaved changes" / "Saved" (cleared after 2 s) / "Error saving" (red)
- **FE-122** Save status shows "Disconnected" in red while offline and clears on reconnect
- **FE-123** Save status shows a red local-draft-storage error
- **FE-124** Save status falls back to server-rendered labels and never shows a raw i18n key
- **FE-125** Returning to the saved content clears the dirty state, status and draft
- **FE-126** Saving the `.fed` config file reloads config and shows "Config applied" for 2 s
- **FE-127** Before an AI grammar check, flush any pending autosave

### SAFETY

- **FE-128** Large-deletion rule: lost chars > 50 AND lost share > 20% of the last saved content; the save is blocked (no dialog)
- **FE-129** Content-loss amber banner with Undo and Save Anyway buttons
- **FE-130** Undo in the banner runs a CodeMirror undo, or reloads the server baseline when there is no history
- **FE-131** Save Anyway approval applies only to the exact current content
- **FE-132** The content-loss banner hides on its own once the deletion no longer meets the threshold
- **FE-133** Content-loss state is scoped to path and revision and resets after a save
- **FE-134** Local draft written 300 ms after each edit to `frankmd:draft:<encodeURIComponent(path)>`, schemaVersion 1, anchored to the base revision
- **FE-135** Draft written synchronously on visibility hidden, pagehide, going offline, and before each save
- **FE-136** beforeunload "Leave site?" prompt only when draft persistence has failed for the current file
- **FE-137** Drafts are deleted only when their revision matches (compare-and-delete)
- **FE-138** Malformed drafts and backups are discarded silently; unknown schema versions are kept
- **FE-139** Storage errors (including quota) show an error status, block navigation and turn on the unload guard; no size or retention limits
- **FE-140** Silent auto-restore of a draft on load when the server revision matches the draft's base, marked "Unsaved changes"
- **FE-141** Draft is discarded on load when it equals the server content
- **FE-142** Recovery dialog when the draft base revision differs from the server (file changed elsewhere)
- **FE-143** Recovery dialog for a newer, different legacy offline backup
- **FE-144** Recovery dialog for draft-conflict records (newest first)
- **FE-145** Recovery dialog: side-by-side word diff, server deletions in red strikethrough, local insertions in green, timestamp pill
- **FE-146** Recovery dialog: Use Server Version / Restore Backup buttons; Escape blocked; X counts as Use Server
- **FE-147** Autosave and draft writes are paused for a path while its recovery is pending
- **FE-148** Recovery choice is rejected if the draft changed while the dialog was open
- **FE-149** Restoring a backup re-runs the large-deletion check before autosave
- **FE-150** Word-level LCS diff, tokenized by `/(\s+|\S+)/g`, with neighbouring operations merged
- **FE-151** Rename/move remaps drafts, conflict records and backups with check-copy-verify-delete; never overwrites another draft
- **FE-152** Remap collision or failure keeps a conflict copy, blocks the path and opens recovery
- **FE-153** Deleting a note or folder removes its drafts, backups and conflicts; a toast shows if that cleanup fails
- **FE-154** Rename keeps the dirty state and moves a pending autosave to the new path
- **FE-155** Connection check: HEAD `/up` every 5 s with a 3 s timeout, plus browser online/offline events
- **FE-156** Offline amber pulsing banner "Connection lost…" with a Retry button
- **FE-157** While offline: server saves suspended, dirty flag kept, debounced config saves cancelled
- **FE-158** Legacy offline backup `frankmd:backup:<path>` written immediately on disconnect and 1 s after each offline edit; deleted after the next successful save
- **FE-159** On reconnect, save immediately if dirty
- **FE-160** Offline banner claims editing is disabled, but only the hidden textarea is disabled; CodeMirror stays editable
- **FE-161** Ctrl+Z at the start of history on an unmodified newly created note offers to delete it
- **FE-162** Undo-created-note confirm dialog with filename chip; Cancel focused by default; Delete in error colour
- **FE-163** Created-note deletion drains in-flight autosave, locks the editor and sends `DELETE` with `expected_revision` (409 on mismatch)
- **FE-164** After undoing a created note, reopen the previous note
- **FE-165** Redo at the end of history (Ctrl+Y / Ctrl+Shift+Z) recreates the undone note with `POST /notes`, create-only
- **FE-166** Undo/redo-created-note boundaries are disabled in Vim mode
- **FE-167** Toasts for these flows show for 5 s, bottom centre, one at a time
- **FE-168** No multi-tab coordination: no BroadcastChannel or storage events; the last write wins
- **FE-169** Splash page polls `/up` every 200 ms, then redirects to the app
- **FE-170** PWA manifest and service worker exist but are not routed or registered

### PREVIEW

- **FE-171** Preview panel: 40% fixed-width right column in split mode, left border, themed background, padded scrollable prose container, no resize handle
- **FE-172** Preview hidden on every app load; visibility not persisted
- **FE-173** Toggle preview via toolbar "preview" action (pinned, aria-pressed) and Ctrl+Shift+V; markdown files only (else 2 s "Preview is only available for markdown files" toast); no-op while Library is open
- **FE-174** Single view mode: preview toggle swaps editor ↔ full-width preview; returning focuses the editor; switching to single mode always shows the editor pane first
- **FE-175** Auto-hide preview when opening a non-markdown file; stash/restore preview visibility around Library and Home views (split mode only)
- **FE-176** Enabling typewriter mode force-shows the preview in split view
- **FE-177** Render on every editor change in normal mode (no debounce); 150 ms debounced render plus identical-content skip in typewriter mode
- **FE-178** Markdown renderer = marked 15.0.6, GFM on, breaks:true (single newline → `<br>`), raw HTML allowed then sanitized
- **FE-179** GFM tables, strikethrough, autolinks, task lists rendered
- **FE-180** No heading IDs/anchors, no footnotes, no mermaid, no admonitions, no `\(..\)`/`\[..\]` math delimiters
- **FE-181** YAML `---` / TOML `+++` front matter stripped and hidden (only when at document start with a closing fence), leading blank lines after it trimmed, line offset kept for sync
- **FE-182** Superscript `^text^` → `<sup>` (literal, escaped content)
- **FE-183** Subscript `~text~` → `<sub>` without breaking `~~strike~~`
- **FE-184** Highlight `==text==` → `<mark>`
- **FE-185** Emoji shortcodes `:name:` from the ~675-entry EMOJI_DATA map; unknown codes stay literal
- **FE-186** Phosphor icon shortcodes `:ph-name:` → inline 1em currentColor SVG (~200 curated icons), baseline-aligned
- **FE-187** Wikilinks `[[Target]]`, `[[Target|Label]]`, `[[folder/Target]]` render as dashed-underline accent links showing label or basename
- **FE-188** Wikilink click resolves exact path (+.md), then case-insensitive basename match, then opens the path anyway (create/404), and reveals it in the tree
- **FE-189** Block math `$$…$$` (multiline, display mode, horizontally scrollable) via KaTeX 0.19
- **FE-190** Inline math `$…$` with currency-safe rules (no space after opening/before closing `$`, no digit after closing, `\$` literal)
- **FE-191** KaTeX options throwOnError:false (inline red error), trust:false, strict:ignore, maxExpand:1000, html output; rendered after sanitization; themed error color
- **FE-192** HTML sanitization (DOMPurify 3.4.17): strip scripts/handlers/style attribute/style tag/form/button/textarea/select/option; keep data-* and class attributes; strip `target`
- **FE-193** Iframes allowed only for https YouTube / youtube-nocookie / player.vimeo.com hosts, never srcdoc
- **FE-194** Inputs allowed only as checkboxes stripped to type/checked
- **FE-195** Responsive 16:9 `.embed-container` iframe styling and `.video-player` `<video>` styling
- **FE-196** Hugo shortcodes (e.g. `{{< youtube >}}`) render as literal text (not expanded)
- **FE-197** Interactive task checkboxes: clicking toggles `[ ]`/`[x]` at that item's source line in the editor (undoable doc change); preview updates via re-render
- **FE-198** Task list styling: accent-colored checkboxes, focus/hover ring, bullets removed for task lists
- **FE-199** Per-block and per-list-item source-line annotations (frontmatter-offset aware) used for sync and task toggling
- **FE-200** Syntax highlighting of fenced code by language tag (~70 aliases, ~30 languages), lazy-loaded parsers, size limits (20k per block / 50k per render / 100 blocks), plain fallback
- **FE-201** Uppercase language label badge in the top-left of highlighted code blocks
- **FE-202** Theme-variable token colors for code (keyword, function, operator, string, number, constant, comment, type, property, identifier, punctuation, invalid)
- **FE-203** Code block copy button (hover-revealed, top-right) copies code text, shows a green check for 2 s
- **FE-204** Image src not rewritten; relative paths resolve against `/notes/<note path>` and are served raw by the notes endpoint (rewrite: resolve relative to the note's folder in the vault)
- **FE-205** Unlinked images wrapped in a keyboard-focusable zoom-in button that opens the image viewer
- **FE-206** Linked images keep link navigation and get an adjacent "↗" enlarge button
- **FE-207** Image viewer modal: fit-to-viewport image, loading and error status text, http/https/blob/data:image URLs only, close via X / Escape / backdrop click, focus returns to the trigger; no zoom/pan/gallery
- **FE-208** Modifier-clicks and non-primary clicks on image triggers do not open the viewer
- **FE-209** Print styles hide the viewer and enlarge buttons and unwrap image triggers
- **FE-210** Links navigate in the same tab (same-origin `/notes/...` and relative .md links open the note in-app via Turbo; external links leave the app); no new-tab behavior
- **FE-211** No click-to-source (clicking preview does not move the editor cursor)
- **FE-212** Scroll sync on/off setting (default on, persisted `scroll_sync`), Settings switch and Ctrl+Shift+\ shortcut
- **FE-213** Editor→preview scroll sync anchored on the editor's top visible line → nearest annotated element top, ratio fallback, exact snap at ≤1%/≥99%, 5 px jitter threshold, rAF-coalesced
- **FE-214** Preview→editor scroll sync: preview top source line → editor line at top, ratio fallback, edge snapping; ignored within 100 ms of a render and during editor mouse selection
- **FE-215** Single scroll-source lock (editor/preview) with 400 ms expiry prevents sync feedback loops; programmatic preview scrolls mark the lock as editor-driven
- **FE-216** Re-anchor sync on preview content resize (images loading), zoom change, and window resize (200 ms debounce)
- **FE-217** On preview show: fresh render and immediate alignment to the editor, lock released after two frames
- **FE-218** Typewriter mode preview: content column limited to the editor width + 3rem and centered; cursor line's block smooth-scrolled to the top of the preview on cursor moves, edits, and editor scroll (10 px jitter threshold)
- **FE-219** Preview zoom levels 50/75/90/100/110/125/150/175/200 %, default 100, toolbar −/+ buttons with % readout (shown only when the preview is visible), Settings select; persisted as `preview_zoom`; implemented as content font-size %
- **FE-220** Prose typography: heading scale 2/1.5/1.25/1.1/1/1 em with h1/h2 bottom borders and theme heading colors; themed links, lists, inline code, code blocks, italic accent-bordered blockquotes, hr, bordered tables with tinted header, rounded images

### EXPLORER

- **FE-221** Sidebar 16rem with header (close toggle + 4 actions: New Note, New Folder, Find File, Search in Files), scrollable tree, stats panel slot, footer (brand + Home/Library/Settings)
- **FE-222** Sidebar visibility session-only: visible on desktop, hidden on mobile <768px; floating "Open Explorer" button when hidden; Ctrl+E toggles
- **FE-223** Mobile overlay sidebar with backdrop, inert main, Tab focus trap, Escape closes, auto-close after opening a file
- **FE-224** Breakpoint crossing on resize resets sidebar visibility and moves focus to visible toggle
- **FE-225** Tree shows only .md files and all folders; hides dot-prefixed entries and symlinks
- **FE-226** Root-level images/ and videos/ folders hidden when they contain no .md
- **FE-227** Sort: folders first case-insensitive A→Z, then files by mtime newest first
- **FE-228** File names displayed without .md; truncated with ellipsis
- **FE-229** Icons: solid folder (--theme-folder-icon), rotating chevron, outlined document (--theme-file-icon), gear for config (--theme-config-icon)
- **FE-230** Empty tree shows "No notes yet"
- **FE-231** Indentation 1rem per nesting level
- **FE-232** Click folder selects it and toggles expand/collapse (modifiers still toggle)
- **FE-233** Expanded-folder set in memory only (not persisted); page load starts collapsed except ancestors of opened file
- **FE-234** Expanded state sent with every tree render and remapped on folder rename/move
- **FE-235** Auto-expand ancestors when opening a file
- **FE-236** Auto-expand destination folder (and ancestors) before creating a note/folder inside it
- **FE-237** Plain click file selects and opens it
- **FE-238** Ctrl/Cmd+click toggles selection without opening
- **FE-239** Shift+click range-selects over visible rows from anchor; Ctrl+Shift adds the range
- **FE-240** Tab focus onto a row selects it
- **FE-241** Selection styling: selection bg/text for selected, thin accent outline for active row
- **FE-242** Selection survives tree re-renders (pruned for vanished items) and is remapped on rename/move
- **FE-243** Focus restored to same row after tree refresh
- **FE-244** Delete key on selected row deletes the selection (with confirmation)
- **FE-245** F2 on single selected row opens rename dialog
- **FE-246** Delete key inside editor never deletes notes
- **FE-247** Right-click row context menu: folder = New Note, New Folder, Rename, Delete; file = Rename, Delete; none for config
- **FE-248** Right-click unselected row replaces selection; right-click selected row keeps multi-selection for Delete
- **FE-249** Context/explorer menus positioned at cursor and clamped to viewport (max-height, horizontal shift, min 96px)
- **FE-250** Menus close on outside click and Escape
- **FE-251** Clicking empty tree background clears selection and opens Explorer menu (New Note, New Folder, Select All, Find File) with first item focused
- **FE-252** Select All selects all visible rows (files and folders)
- **FE-253** Toolbar/Ctrl+N/background New Note uses the single selected folder as destination, else root
- **FE-254** Toolbar Hugo posts always go to blog/; folder context-menu Hugo posts go to that folder
- **FE-255** New Note dialog: name input, Empty Document / Hugo Blog Post cards, live path preview with root/placeholder/error states
- **FE-256** New Note validation: non-empty, no slash, not . or ..; Hugo needs non-empty slug; Create disabled and Enter gated
- **FE-257** Hugo path styles dated (YYYY/MM/DD/slug/index.md) and flat (slug.md) from hugo_path_style config
- **FE-258** Hugo slugify with accent map; frontmatter from .hugo_template.md or built-in default (title, slug, date, draft:true, tags:[])
- **FE-259** Created note opens immediately; empty note focuses editor; creation recorded for undo-to-remove
- **FE-260** New Folder dialog with "In: <parent>/root folder" subtitle; nested path names allowed
- **FE-261** Inline red error messages inside create/rename dialogs; backdrop click closes dialogs; Enter submits, Esc closes
- **FE-262** Rename dialog prefilled with name (no .md for files) fully selected; .md auto-appended; same name is no-op
- **FE-263** Rename/move/delete of open file flushes draft first; aborts with draft-storage error if it fails
- **FE-264** Rename updates current path, header title, URL, autosave keys, undo history, recent files
- **FE-265** Folder rename/move rewrites slug in folder/index.md frontmatter
- **FE-266** Delete confirmation messages for file / folder / N items; destructive Delete button
- **FE-267** Deleting non-empty folders is refused by server ("Folder not empty") and shown in alert
- **FE-268** Multi-delete sequential, stops at first error; items inside selected folders deduplicated
- **FE-269** Deleting the open file returns to Home and replaces URL with /
- **FE-270** Drag files/folders onto folder rows to move; single highlighted valid drop target
- **FE-271** Drag a multi-selection moves all selected items (excluding descendants of selected folders)
- **FE-272** Invalid drop targets: self, own descendants, folder already containing all dragged items
- **FE-273** Drop on tree background or a file row moves items to root; dashed root highlight when applicable
- **FE-274** Dragged row dims (opacity .5/.4); no move confirmation; collision/failure alert "Failed to move" and stop
- **FE-275** Moved items revealed by expanding destination; open file path/URL remapped
- **FE-276** Tree refresh after every open/home/not-found and via server-rendered responses after mutations; no polling/watcher
- **FE-277** Stale tree refresh responses discarded (navigation generation + tree revision)
- **FE-278** Header document title shows basename without .md, tooltip full path, "Select or create a note" when empty
- **FE-279** Click title for inline rename: auto-width input, caret at end, Enter/blur submit, Esc cancel, inline errors, disabled for no file/config
- **FE-280** File-not-found shows red path + message in header for 5 s, then Home
- **FE-281** Vim :next-note / :prev-note cycles through tree files in DOM order with wrap

### FINDER

- **FE-282** Ctrl/Cmd+P (and sidebar/Home/background-menu buttons) opens 600px modal with input, results, preview
- **FE-283** Candidates = all tree files sorted by mtime desc; empty query shows first 10
- **FE-284** Fuzzy score on lowercased full path: +1 per char, growing consecutive bonus (+2,+4,…), +3 at start or after / - _, +max(0,10-(len diff)); filter >0, sort desc, top 10
- **FE-285** Result row: name without .md + directory path line; selected row accent colors; "No files found"
- **FE-286** Preview of first 10 lines of selected file (with "..." when ≥10), error text on failure
- **FE-287** Arrow keys move selection (clamped), Enter opens, Esc closes, hover selects unless keyboard-navigating, click opens
- **FE-288** Opening reveals file in tree (expand ancestors, force sidebar visible) and loads it

### SEARCH

- **FE-289** Ctrl/Cmd+Shift+F opens 700px content search modal with regex cheat-sheet and status line
- **FE-290** Always regex, case-insensitive; invalid regex falls back to literal; 0.5 s regex timeout → no results
- **FE-291** 300 ms debounce; spinner overlay and "Searching..." immediately on input; empty input resets
- **FE-292** Max 20 matching lines total, 3 context lines each side, .md files only, skip dotfiles/symlinks/>5MB, results ordered by file mtime desc
- **FE-293** Result card: file name, :line, path, monospace context with line-number gutter and highlighted match line
- **FE-294** Status "N matches found (showing first 20) - use ↑↓ to navigate, Enter to open" / "No matches found" / "Search error"
- **FE-295** Arrow navigation with scroll-into-view, Enter/click opens file, reveals in tree and jumps to line (centered, editor focused)

### HOME

- **FE-296** Home shown when no document open (root URL, Home button, Back to /, after deleting open file); header hidden
- **FE-297** Home intro: logo + FrankMD title, description, Find file and New Note buttons; h1 focused on showHome
- **FE-298** Recently opened list (only when non-empty): name, path, relative opened-ago time with ISO tooltip
- **FE-299** Show first 5, Show more/Show less toggles remaining (up to 10) with focus management
- **FE-300** Recent history stored server-side in .fed key recent_files, max 10, newest first, dedup, shared across devices
- **FE-301** Record on every markdown open; filter missing files on read; remap on rename/move; remove on delete
- **FE-302** Clear history with confirmation ("for every device using this notes folder")
- **FE-303** Refresh list on window focus and on returning Home; error status "Unable to update shared history. Try again."
- **FE-304** Plain click opens note in-app; modified/middle click follows /notes/<path> link
- **FE-305** Preview hidden on Home and restored on next markdown open if previously visible

### MEDIA

- **FE-306** Image picker modal with 7 tabs (Upload, Library, Folder, Web Search, Google, Pinterest, AI Generate); Upload is the default
- **FE-307** Image picker tab bar: ArrowLeft/Right cycles with wrap and focus; mouse wheel over the tab bar switches tabs; roving tabindex and aria-selected
- **FE-308** Image picker loads /images/config (enabled, s3_enabled, google_enabled, image/video extension allow-lists) and /ai/image_config
- **FE-309** Image picker refreshes the media storage provider before opening and waits for any pending provider save before uploading
- **FE-310** Shared selected-image footer: "Selected: name", Alt Text input prefilled from the source default, optional Link URL
- **FE-311** Inserted markdown `![alt](url)`, or `[![alt](url)](link)` when Link is set; alt falls back to name, then "Image"
- **FE-312** Local `images/...` upload URLs are rewritten to a note-relative, percent-encoded path with an escaped alt
- **FE-313** Image insertion adds a single newline before/after only when needed and replaces the selection or slash-command range
- **FE-314** Insert failure shows the inline error "Failed to insert image: …", re-enables Insert, and refreshes the S3 provider state
- **FE-315** S3 option component: "Save a copy in the active storage" checkbox (reupload mode), resize select (none/25%/50%/67% + compress), destination key (full) or folder prefix, with defaults frankmd/YYYY/MM[/filename]
- **FE-316** S3 key input keeps user edits across selections (dirty flag) and shows only when the provider is S3
- **FE-317** Upload tab: click/Enter/Space dropzone opens a multi-file image chooser; drag highlight that does not flicker over child elements
- **FE-318** Upload tab: extension allow-list check with a per-file rejection message listing accepted types
- **FE-319** Upload tab: newest-first grid of up to 10 thumbnails with WxH badge, size tooltip and "Showing N most recent of M" status
- **FE-320** Upload/Folder default alt = filename without extension, with -/_ turned into spaces
- **FE-321** Upload/Folder upload via POST /images/upload with resize and s3_key
- **FE-322** Library tab: collection select (Note images / Server image folder, disabled if not configured), per-page 10/25/50, 300 ms debounced filename search
- **FE-323** Library tab: server-rendered grid with Previous/Next pager and "Showing a–b of N"
- **FE-324** Library tab: per-thumbnail delete with destructive confirm; endpoint depends on the collection; step back a page when the last item on a page is deleted
- **FE-325** Library tab: managed images insert their existing path; server images can be copied to active storage via /images/import
- **FE-326** Folder tab: pick a local folder (non-recursive), newest-first, filename search, pager, per-page select, change folder
- **FE-327** Folder tab: unsupported-API notice (native app: replace with a native folder dialog)
- **FE-328** Web Search tab: query via Enter/button, status line (Searching / Found N / server note / errors in red), 4-col grid with dimension badge and source caption
- **FE-329** External search results: broken thumbnails are removed from the grid; unsafe thumbnail URLs are not rendered
- **FE-330** External results: default alt = title with -/_ turned into spaces (100 chars); hotlink by default, optional download-and-copy with resize/prefix
- **FE-331** Google tab: config-required notice; infinite scroll loads the next page within 100 px of the bottom
- **FE-332** Pinterest tab: single-page search with a red Search button
- **FE-333** AI tab: config-required notice; model label; prompt textarea (Enter generates, Shift+Enter newline)
- **FE-334** AI tab: optional reference image picker (searchable 6-col grid of server images; preview with "Remove reference")
- **FE-335** AI tab: generating spinner "Generating image with <model>", Esc cancels the request
- **FE-336** AI tab: result preview, AI-revised prompt, S3-only destination folder field; saved as ai_<ms>.png via upload_base64 or import; alt = prompt truncated to 100 chars
- **FE-337** Video dialog modal with Upload / URL-File / YouTube Search tabs; same tab keyboard and wheel behavior; storage selector
- **FE-338** Video dialog "Use Hugo shortcode instead of HTML embed" checkbox persists across opens
- **FE-339** Video Upload tab: single-file drop/pick, extension allow-list, immediate POST /media/upload with an optional S3 prefix, "Uploading..." text, in-flight guard, then Insert enabled
- **FE-340** Video URL tab: live detection of YouTube URLs (watch?v=, youtu.be/, embed/, v/, bare 11-char ID) with a thumbnail preview and fallback
- **FE-341** Video URL tab: video file detection by trailing extension; unknown-format warning; Enter inserts
- **FE-342** YouTube search: config notice, server-rendered 2-col results (thumb/title/channel), status messages, keyboard grid navigation (±1/±2, Up returns to input, Esc), click/Enter inserts immediately
- **FE-343** YouTube embed HTML (`div.embed-container` iframe with fixed allow/referrerpolicy attributes) or Hugo `{{< youtube id="…" [title="…"] >}}`
- **FE-344** Video file embed `<video controls class="video-player"><source src type>` + fallback text; MIME map by extension
- **FE-345** Video/embed block insertion with blank-line normalization (0/1/2 newlines before and after)
- **FE-346** Media Library workspace replaces the editor area; toggle button aria-pressed; restores preview state on close (split mode); Esc closes preview, then the Library
- **FE-347** Library Images/Videos category toggles with item count; loading/error/empty states
- **FE-348** Library cards: preview thumbnail (image or muted video + play badge), name, size · date, usage label, Rename (S3), Delete, Insert
- **FE-349** Library lazy usage index (on first visible card) with "Checking usage / Used in N notes / Usage unknown"
- **FE-350** Library usage dialog listing referencing notes; clicking a note opens and reveals it
- **FE-351** Library preview dialog: full media, metadata (size, date, aspect ratio, format), Delete / Copy relative path / Insert, focus trap, backdrop close
- **FE-352** Library insert: requires an open Markdown note; image `![name](relative-or-S3-url)`; video `<video controls class="video-player"><source …></video>`; returns to the editor
- **FE-353** Library delete with confirm warning that references will break; status "Media file deleted."
- **FE-354** Library rename for S3 items via text prompt; references are not updated
- **FE-355** Media storage provider selector (Local/S3) in dialogs and the Settings dropdown, synced everywhere; S3 disabled with tooltip when not configured; controls disabled while saving
- **FE-356** Provider save error message keeps the previous selection; config-file-modified event on success
- **FE-357** S3 fallback warning modal "Upload storage changed", auto-dismissed after 6 s

### DND

- **FE-358** Pasting a clipboard image opens the Image Picker with the image pre-loaded and selected in the Upload tab ("Processing image..."); the user confirms Insert
- **FE-359** Pasted images without an extension are named pasted-image-<ts>.<ext from MIME>; unknown subtypes are rejected with a message
- **FE-360** Image paste suppresses the accompanying HTML clipboard content; text paste is unaffected
- **FE-361** Dropping OS files directly on the editor has no upload handling (CodeMirror default text insert) — decide the behavior for the rewrite
- **FE-362** No upload progress bar or editor placeholder; only spinner/"Uploading..." text inside dialogs
- **FE-363** Explorer tree drag & drop moves notes/folders only (handled in the explorer section); OS file drops onto the tree are ignored

### DIALOG

- **FE-364** All modal dialogs use native modal semantics (focus trap, top layer) and close on Escape
- **FE-365** Dialog backdrop is dimmed 60% black with a 2px blur, and dialogs are centered in the viewport
- **FE-366** Backdrop click closes only Help, Log viewer, Rename, New Note and New Folder dialogs
- **FE-367** Global Escape shortcut defers to an open dialog and does not double-close
- **FE-368** Shared dialog header (title plus X close button) and primary/secondary button styles
- **FE-369** Custom alert dialog (title "Notice", OK button) replaces window.alert
- **FE-370** Custom confirm dialog (title "Confirm", Cancel and Confirm buttons) with a destructive red variant
- **FE-371** Custom text-prompt dialog with input; Enter submits, Escape cancels
- **FE-372** Prompts are queued FIFO and focus is restored to the original element after the last one
- **FE-373** Destructive confirms focus the dialog, not the accept button; normal confirms focus Cancel
- **FE-374** Floating dropdown: anchored below or above the trigger, max-height 320, clamped to viewport, RTL-aware
- **FE-375** Floating dropdown closes on outside click or focus, scroll, another dropdown opening, or dialog close
- **FE-376** Floating dropdown keyboard: Arrow/Home/End navigation, Tab or Escape closes and restores focus
- **FE-377** Context menus clamp max-height to the viewport (min 96px) and shift horizontally to stay on screen
- **FE-378** Slash-command dialogs (table, emoji) replace the typed /query and abort the insert if the query text changed
- **FE-379** "Cursor in table - click to edit" badge shown (200ms debounce) when the cursor is inside a markdown table
- **FE-380** Table detection: contiguous lines starting with "|", at least 2 lines, must contain a separator row
- **FE-381** Table editor opens in edit mode prefilled with the table under the cursor
- **FE-382** Table editor default new table is 3x3 with headers "Header 1..3"
- **FE-383** Table editor column and row number steppers (min 1) with -/+ buttons
- **FE-384** Table editor adding a column fills its header with "Header N"
- **FE-385** Table editor grid of text inputs with a bold, shaded header row and "Header" placeholder
- **FE-386** Table editor right-click cell menu: move column left/right, delete column, move row up/down, delete row
- **FE-387** Table editor header row cannot be moved or deleted; last column and last row cannot be deleted
- **FE-388** Table output pads columns to max(3, longest cell) with separator dashes; alignment and escaped pipes not preserved
- **FE-389** Table insert adds blank-line separation before and after; edit mode replaces the table range in place
- **FE-390** Code block dialog with a language input (focused) and code textarea
- **FE-391** Code language autocomplete: substring match over 44 fixed languages, max 6 suggestions
- **FE-392** Code language Tab accepts the first suggestion; ArrowDown/ArrowUp navigate suggestions; Escape hides them
- **FE-393** Code textarea Tab and Shift+Tab indent or outdent (2 spaces or tab via "Use tabs" checkbox), multi-line aware
- **FE-394** Ctrl+Enter inserts from either code dialog field
- **FE-395** Unrecognized code language (not prefix or Levenshtein ≤2 of a known one) asks for confirmation
- **FE-396** Code fence format: ```lang\ncontent\n``` ; empty content gives ```lang\n\n```
- **FE-397** Cursor in an existing ``` block opens the code dialog in edit mode and replaces that block
- **FE-398** Cursor lands on the first content line after inserting a code block
- **FE-399** Emoji picker opens with Ctrl+Shift+E, toolbar or /emoji (markdown files only)
- **FE-400** Emoji picker tabs Emoji / Emoticons / Icons, switched by Arrow keys on tabs or mouse wheel
- **FE-401** Emoji tab: 675 emoji in a flat 10-column grid, no category headers
- **FE-402** Emoticons tab: 125 kaomoji in a 5-column grid
- **FE-403** Icons tab: 201 Phosphor icons in an 8-column grid
- **FE-404** Emoji search: AND over whitespace-separated substrings of shortcode and keywords
- **FE-405** Localized emoji search via emojibase label/tags (es, ja, ko, pt; he falls back to en)
- **FE-406** Emoji grid keyboard nav: arrows with wrap, Enter inserts; hover selects; click inserts
- **FE-407** Emoji preview footer shows glyph plus :shortcode: (or icon plus :ph-name:, or kaomoji plus name)
- **FE-408** Emoji inserts ":shortcode:", icon inserts ":ph-name:", emoticon inserts raw text, replacing the selection
- **FE-409** Preview renders :shortcode: as unicode and :ph-name: as an inline 1em SVG
- **FE-410** Ctrl+K inserts a link template [sel](url) with "url" selected, or [](url) with the cursor in the brackets
- **FE-411** Text-format menu opens on right-click with selection or Ctrl+M
- **FE-412** Text-format menu: Bold/Italic/Strikethrough/Highlight/Subscript/Superscript/Link with underlined hotkeys B I S H U P L
- **FE-413** Text-format menu keyboard: arrows with wrap, Enter, hotkey letters; Escape refocuses the editor
- **FE-414** Symmetric formats toggle off when markers are inside or just outside the selection
- **FE-415** Log viewer opens with Ctrl+Shift+O and shows the last 100 log lines, auto-scrolled to bottom
- **FE-416** Log viewer header shows environment and log file, line count, refresh and close buttons
- **FE-417** Log viewer Config tab: config file, exists, AI-in-file flags plus key/value/source/ENV-var table color-coded by source
- **FE-418** Help dialog (F1, toolbar, :help) with tabs Markdown / Keyboard Shortcuts / Vim Mode, reset to Markdown on open
- **FE-419** Help tabs switched by Arrow keys or mouse wheel
- **FE-420** Help Markdown cheatsheet cards: headings, emphasis, lists, task list, links/images, blockquote, code, table, hr
- **FE-421** Help Vim tab lists modes and ex-commands (:w :q :wq/:x :e/:find :Explore :n :prev :help)
- **FE-422** Jump-to-line dialog (Ctrl+G): absolute N, relative +N/-N, "-1" means last line, clamped to range
- **FE-423** Jump-to-line Enter jumps; Escape closes and refocuses the editor
- **FE-424** New Note dialog: name input plus Empty Document / Hugo Blog Post type cards
- **FE-425** New Note live path preview with muted, faint or error states
- **FE-426** New Note Create disabled for empty names, names with "/", "." or ".."
- **FE-427** New Note empty template creates <parent>/<name>.md with empty content
- **FE-428** Hugo post path is <parent>/YYYY/MM/DD/<slug>/index.md (dated) or <parent>/<slug>.md (flat, config hugo_path_style)
- **FE-429** Hugo post from Ctrl+N/toolbar defaults its parent to "blog"
- **FE-430** Hugo slugify with accent map, lowercase, non-alphanumerics become "-", trimmed and collapsed
- **FE-431** Hugo front matter (title, slug, date ISO+offset, draft: true, tags: []), overridable via .hugo_template.md with {{title}} {{slug}} {{date}}
- **FE-432** New Note server errors (e.g. "Note already exists") shown inline; dialog stays open
- **FE-433** New Folder dialog shows "In: <parent|root folder>"; Enter creates
- **FE-434** Rename dialog prefilled with the name (no .md), fully selected; .md re-appended; only the last segment changes
- **FE-435** Rename errors shown inline; unchanged name closes without a request
- **FE-436** Delete confirm (destructive) with single-file, folder and multi-item messages; nested items deduplicated
- **FE-437** Undo-creation confirm dialog (Cancel autofocused, red Delete) with filename chip when undoing past a new note's start
- **FE-438** Redo (Ctrl+Shift+Z / Ctrl+Y) recreates an undone created note during the session

### AI

- **FE-439** AI availability fetched once from the AI config endpoint on load
- **FE-440** AI not configured: dialog shows provider setup cards (Ollama, OpenRouter, Anthropic, Gemini, OpenAI, override) with .fed snippets
- **FE-441** AI check refuses when no file is open or the text is blank
- **FE-442** AI check flushes the pending autosave before sending
- **FE-443** AI check sends the whole saved file path, never just the selection
- **FE-444** AI processing makes the editor read-only and shows a spinner plus "Processing..." on AI buttons
- **FE-445** Full-screen processing overlay shows "provider: model" and "Press ESC to cancel"
- **FE-446** Escape aborts the in-flight AI request silently
- **FE-447** AI errors are shown via an alert dialog
- **FE-448** Side-by-side word-level LCS diff: original with red strikethrough deletions, corrected with green additions
- **FE-449** Provider/model badge in the AI dialog header
- **FE-450** Edit toggle swaps the corrected diff for an editable textarea (label Edit/Preview)
- **FE-451** Accept Changes replaces the whole document with the (possibly edited) corrected text; Cancel discards

### THEME

- **FE-452** 18 built-in themes, each defining the same 38 `--theme-*` hex variables (values in E.1.3), applied via `<html data-theme="id">`
- **FE-453** Light palette is the `:root` fallback for missing or unknown theme ids
- **FE-454** Light/dark classification (5 light themes) toggles the `<html>.dark` class for dark-variant styling
- **FE-455** Theme menu order: Light, Dark, then alphabetical, with sun/moon/palette icons, then Custom (if present), then Omarchy (if available)
- **FE-456** No saved theme means follow the system `prefers-color-scheme` at apply time (no live OS-change listener)
- **FE-457** Theme persisted only in the server `.fed` key `theme` via debounced (500 ms) PATCH /config; no localStorage
- **FE-458** Theme choice applies instantly and recolors the whole app, including CodeMirror (CSS-variable-driven editor theme and highlight style)
- **FE-459** Preview fenced-code token colors from the 12 `--theme-syntax-*` variables
- **FE-460** Derived colors: sidebar border mix, backdrops (75/78/85%), toast accent mix, `--editor-width`
- **FE-461** Theme dropdown: popover menuitemradio list with checkmark, keyboard navigation (arrows, Home, End, Esc, Tab), one menu open at a time, viewport-clamped max 320px with upward flip, closes on scroll/outside click
- **FE-462** Live re-apply when `.fed` is edited and autosaved (`frankmd:config-changed`)
- **FE-463** Saving theme/settings reloads an open `.fed` buffer, preserving the cursor (`frankmd:config-file-modified`)
- **FE-464** Omarchy mode: shown only when `~/.config/omarchy/current/{theme.name,theme/alacritty.toml}` exist; derives all 38 vars from alacritty colors, with contrast-corrected syntax colors
- **FE-465** Omarchy mode polls every 3 s and re-applies when the theme name changes; falls back to Dark and saves it when Omarchy disappears
- **FE-466** Omarchy is_dark is computed from background luminance and toggles the `.dark` class
- **FE-467** Custom theme editor dialog: name (1-60 chars), base theme select (18), 3 basic colors plus 35 advanced colors, each with a color swatch and a hex text field
- **FE-468** Custom theme live whole-app preview while editing; revert on cancel, Esc or backdrop click; focus returns to the trigger
- **FE-469** Custom theme hex validation (#rrggbb) with inline error, disabling Save
- **FE-470** Custom theme WCAG contrast warning (<4.5:1 for text/bg or accent-text/accent), non-blocking
- **FE-471** Custom theme Reset colors (clears overrides back to the base palette; name and base kept)
- **FE-472** Custom theme saved as sparse JSON `{version:1,name,base,colors}` in `.fed` `custom_theme` with `theme=custom`; server validation (16 KiB max, known keys, hex, base whitelist)
- **FE-473** Custom theme appears in the theme menu under its own name and is applied as base `data-theme` plus inline overrides on `<html>`
- **FE-474** Custom theme preview card (heading, text, link, code sample)

### SETTINGS

- **FE-475** Settings modal dialog opened from the gear button in the sidebar footer; close via X, Esc or backdrop
- **FE-476** Left category nav: General, Editor, Writing, Preview, About (`aria-pressed`, in-memory selection)
- **FE-477** All settings persist to `.fed` via PATCH /config (UI_KEYS whitelist), debounced 500 ms, applied live; "Failed to save setting" status for 2 s
- **FE-478** Media storage picker (Local/S3; S3 disabled if not configured; error line; auto-dismissing fallback warning after 6 s)
- **FE-479** Theme picker plus "Customize theme" link
- **FE-480** Language picker (reloads app)
- **FE-481** Document view segmented control: Split View / Single View
- **FE-482** Editor font family select (9 monospace fonts, each option rendered in its own font)
- **FE-483** Editor font size select (12-24px)
- **FE-484** Live font preview box
- **FE-485** Editor width range slider 40-200 step 8 with "Nch" label
- **FE-486** Line numbers select: Off / Absolute / Relative
- **FE-487** Stats switch (document stats in Explorer)
- **FE-488** Indentation select: 2/4/8 spaces
- **FE-489** Typewriter mode switch
- **FE-490** Center cursor after clicking switch
- **FE-491** Vim mode switch
- **FE-492** Scroll sync switch (default on; also Ctrl+Shift+\)
- **FE-493** Preview zoom select (50-200%)
- **FE-494** About panel: logo, name, description, version, copyright year, GitHub link
- **FE-495** Pill switch, segmented, range and card visual styles driven by `aria-pressed`
- **FE-496** Settings re-sync from server when `.fed` is saved (GET /config), with "config applied" status

### I18N

- **FE-497** 7 locales: en, pt-BR, pt-PT, es, he, ja, ko (692 keys each, full parity)
- **FE-498** Locale resolution: `?locale=` param, then `.fed` `locale`, then env `FRANKMD_LOCALE`, then en; `<html lang>` set
- **FE-499** Server-rendered strings plus JS `window.t(key, {vars})` with `%{var}` interpolation and key-as-fallback
- **FE-500** GET /translations JSON (11 sections, locale deep-merged over en), loaded once; `frankmd:translations-loaded` event
- **FE-501** Language picker with flag emoji and native names; selecting saves the locale and reloads the page
- **FE-502** Emoji search localized via `/emoji-data/<en|es|ja|ko|pt>.json` (he falls back to en)
- **FE-503** Hebrew present but no RTL layout (no `dir` attribute); dropdown alignment respects computed rtl direction