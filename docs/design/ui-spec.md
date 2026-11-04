# FrankMDFast UI specification

Status: design proposal, 2026-11-04. Stack: **GPUI + longbridge gpui-component** (published as `gpui-kit` from 0.9; this doc says "gpui-component"). Companion mockups: [`mockups/index.html`](mockups/index.html), which you open in any browser. It has a theme switcher with real Omarchy palettes and a corner-radius toggle.

Inputs: `docs/inventory/frontend.md` (FE-001…FE-503, quirks §13), `docs/inventory/backend.md` (BE-001…BE-100), `docs/research/omarchy-4.5.md` (OM-001…OM-028), `docs/research/history-pain-points.md` (HP-001…HP-030), `~/Projects/omarchy/default/agents/skills/omarchy-app/SKILL.md`, and FrankMD's minimal-workspace screenshots.

Contents

1. Design principles
2. Information architecture: where everything lives
3. Window layout and regions
4. Layout modes and responsive behaviour
5. Surfaces (every overlay, dialog, view and state)
5a. Reused FrankMD assets
6. Keyboard map
7. Design tokens
8. RTL, CJK and IME
9. Accessibility
10. Feature-to-home mapping (FE-/BE- groups)
11. Quirk dispositions
12. Open questions for engineering

---

## 1. Design principles

FrankMD is a writing tool that happens to be fast. The rewrite should feel like a Zed buffer or a well-configured Neovim on Omarchy, not like a web page. Seven rules decide every placement below.

1. **The document owns the window.** Chrome is only a 32 px pane header and a 24 px status line. There is no top toolbar, no page title, no dashboard. With no note open, the editor pane shows a docked quick-open list rather than a "home page".
2. **Every action has one name and three ways in:** a command-palette entry (always), a shortcut (for frequent actions), and a visible control (only where a mouse user would look for it). The palette is the complete index of what the app can do. If something can't be found in the palette, it doesn't exist.
3. **Nothing hides behind hover.** Controls that matter are visible at rest, usually dim, and brighten on focus or hover. That rules out FE-011's hover-only pins, FE-203's hover-only code copy button, and tooltip-only shortcut hints. Shortcuts are printed next to menu items and palette rows.
4. **Tiling first.** The app runs in a Hyprland tile that can be 640 px wide or 3840 px wide, usually full height, with no client-side title bar. Layout is computed from the window's logical width, not from assumptions about a monitor. No floating toasts in corners: messages go to the status line, which is always in the same place.
5. **The theme is the desktop's.** On Omarchy the palette comes live from `colors.toml` and corner radius from `decoration:rounding`, so the app looks like it was shipped with the theme. The 18 built-in FrankMD palettes are the fallback, not the default.
6. **Data safety is visible but quiet.** The save state is a glyph in the status line. Anything that needs a decision (content loss, conflict, external change, unreachable vault) is a banner attached to the editor pane with explicit buttons. It never pops up a modal unless the app can't continue without an answer (recovery).
7. **Settings live in one place.** One Settings view, one search field, every key including AI, S3 and Hugo (which today require hand-editing `.fed`), each row showing where its value comes from. Settings has moved three times in FrankMD's history (HP-021). This is the last move.

What we deliberately drop from the web UI: boot splash (FE-001), URL routing (FE-002, replaced by CLI and in-app back/forward), login (FE-004), mobile overlay mode (FE-019, replaced by the narrow-tile layout), the hidden textarea (FE-074), pinnable toolbar actions (FE-010/011/012, replaced by palette plus a fixed set of four header buttons), floating toasts (FE-025/026), PWA and service worker (FE-170), and connection polling (FE-155, replaced by vault reachability).

---

## 2. Information architecture

```
Window
├── Sidebar (left dock, resizable, toggle Ctrl+E)
│   ├── Vault header: vault name ▾ (switch vault) + New note, New folder
│   ├── Explorer tree (keyboard navigable, multi-select, drag & drop)
│   ├── "Linked from" section (backlinks, collapsible, new)
│   └── Dock footer: Media library · Settings · Help (icons)
├── Editor pane
│   ├── Pane header: [open-sidebar] breadcrumb / title (rename) ● · Image · Find · Preview · ⋯
│   ├── Pane banners (stack, 0..n): content loss, conflict, external change, vault unreachable, read-only, AI running
│   ├── Find/replace bar (inline, docked under header)
│   ├── Text (or empty state: docked quick open)
│   └── Inline popovers: slash menu, insert menu, wikilink/link completion, text-format menu, emoji picker, context menu
├── Preview pane (right, resizable, toggle Ctrl+Shift+V)
│   ├── Pane header: "Preview" · zoom − 100% + · scroll-sync toggle · close
│   └── Native-rendered markdown
├── Media library view (replaces editor+preview, Ctrl+Shift+M)
├── Status line (always): mode · save state · message slot … context chip · stats · Ln/Col · vault
└── Overlays (one at a time, centred, top-anchored)
    ├── Command palette (Ctrl+Shift+P), also hosts: theme picker, language picker, vault switcher, go to line
    ├── Quick open (Ctrl+P), recent-notes switcher (Ctrl+Tab)
    ├── Search in files (Ctrl+Shift+F)
    ├── Modal dialogs: New note, New folder, Rename/Move, Delete confirm, Insert image, Insert video,
    │   Table editor, Code block, Grammar check, Recovery, Custom theme editor, Undo-creation confirm,
    │   generic alert/confirm/prompt
    ├── Settings (Ctrl+,): large modal sheet with category list
    ├── Help (F1) and Keyboard shortcuts (Ctrl+/)
    ├── Diagnostics (Ctrl+Shift+O): logs and config sources
    └── Image viewer (lightbox)
```

### 2.1 Placement rules

| Kind of thing | Lives in | Why |
|---|---|---|
| Acting on the current document (insert, find, AI, copy, rename, delete) | Pane header `⋯` menu + palette + shortcut | Discoverable for mouse users, fast for keyboard users, one list |
| Inserting at the caret | Inline popovers anchored at the caret (slash menu, Alt+I insert menu, emoji, link completion) | The eye is already at the caret |
| Inserting something that needs a form or a choice of source (image, video, table, code block) | Modal dialog, opened from the inline menus | Needs room; blocks editing anyway |
| Decisions about data (conflict, content loss) | Pane banner; the Recovery dialog only when the app must block | The editor stays usable and the question stays visible |
| Transient feedback ("Saved", "Copied", "Editor width 80ch") | Status line message slot | One place, never covers text, works in narrow tiles |
| Background completion while the window is unfocused (AI finished, upload finished, theme sync failed) | Freedesktop notification (OM-014) | The user is elsewhere |
| Preferences | Settings (Ctrl+,), every key also as a palette toggle where it makes sense | One home (HP-021) |
| Pickers with live preview (theme, language, vault) | Palette sub-mode | Arrow keys preview, Esc reverts, Enter commits |
| Navigation (open note, search, recents) | Quick open, search overlay, explorer, back/forward | — |
| Media management | Media library view in the main area | Needs the full width for a grid |

---

## 3. Window layout and regions

### 3.1 Wide layout (≥ 1100 px logical)

```
┌─────────────────┬──────────────────────────────────────┬───────────────────────────────┐
│ notes ▾    ⊕ ⊞  │ blog / 2026 / Project brief ●  ▣ ⌕ ◫ ⋯│ Preview        − 100% +  ⇅  × │  32 px pane headers
├─────────────────┼──────────────────────────────────────┼───────────────────────────────┤
│ ▾ blog          │ [banner: Large deletion …  Undo  Save]│                               │  banners: 0..n, 32 px each
│   ▸ 2026        │ ┌find ─────────────── 2 of 9 ↑ ↓ Aa .*┐│   A calmer writing workspace │  find bar when open
│ ▾ projects      │                                      │   ─────────────────────────   │
│   Launch list   │      # A calmer writing workspace    │   The document is the centre │
│ ▌ Project brief │                                      │   of the workspace…          │
│ ▸ reference     │      The document is the centre …    │                               │
│   Inbox         │                                      │                               │
│                 │          (text column = editor_width │                               │
│ Linked from (2) │           in ch, centred)            │                               │
│   Launch list   │                                      │                               │
│ ─────────────── │                                      │                               │
│ ▤  ⚙  ?         │                                      │                               │
├─────────────────┴──────────────────────────────────────┴───────────────────────────────┤
│ NORMAL │ ● Saved │                              Ctrl+. Edit table │ 1,204 w │ 12:4 │ notes │  24 px status line
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

| Region | Size | Surface token | Notes |
|---|---|---|---|
| Sidebar | default 248 px, min 180, max 480, resizable; width persisted per vault in state dir | `surface.recessed` | 1 px `border.subtle` on its inline-end edge |
| Editor pane | flex, min 320 px | `surface.base` | Text column max width = `editor_width` ch (default 72, 40–200, step 8), centred, padding 16 px block, 32 px inline |
| Preview pane | default 45 % of the editor+preview width, min 280 px, resizable; ratio persisted | `surface.base` | 1 px `border.subtle` on its inline-start edge. Prose max width = `editor_width` ch + 3 rem, centred |
| Splitters | 1 px visible line, 6 px hit area | `border.subtle`, `accent` while dragging | Double-click resets to default. Keyboard: focus a splitter with F6 cycling, then Left/Right by 16 px (FE-021 fixed) |
| Pane header | 32 px | `surface.base`, bottom 1 px `border.subtle` | The focused pane gets a 2 px `accent` line along the header's top edge. This echoes Hyprland's active-window border and answers "where will my keystrokes go" |
| Banner | 32 px per banner, stacked | tinted role surface (see 7.1) | Inside the editor pane, under its header |
| Status line | 24 px | `surface.recessed`, top 1 px `border.subtle` | Full window width |

### 3.2 Title bar and window decorations

No client-side title bar on Hyprland. Request server-side decorations (`xdg-decoration`). If the compositor refuses (GNOME/Mutter), draw the pane headers as the drag region and add the window controls (min, max, close) at the end of the right-most pane header. Never draw a separate title bar row. The window title, which Hyprland and Waybar show, is `<note title> — FrankMD`, or `FrankMD — <vault>` when no note is open.

### 3.3 Sidebar

- **Vault header (32 px):** vault folder name with a chevron (click or palette "Vault: Switch…" opens the vault picker: recent vaults + "Open folder…" via the portal FileChooser, OM-011). At the end: New note (file-plus), New folder (folder-plus). FrankMD's header also had Find file and Search in files. Those move to the empty state and palette; their shortcuts are printed in the empty state.
- **Explorer tree:** 24 px rows, 12 px indent per level plus a 16 px chevron column, Phosphor `folder`/`folder-open` in `folder-icon` colour, `file-text` in `file-icon` colour (same glyphs FrankMD uses today). Names without `.md`, truncated with an ellipsis, full path in the tooltip and in the status line while the row is focused. The open note's row carries a 2 px accent bar at its inline-start. Selection uses `selection`; the focused row adds a 1 px `accent` outline.
- **Linked from (new, BE-044):** a collapsible section that appears only when the open note has backlinks. Rows show the source note and one line of context. Collapsed state persisted. This finally gives the backlinks endpoint a UI (FE §1.8).
- **Dock footer (28 px):** three icon buttons (Media library, Settings, Help) with labels in tooltips *and* in the palette. The brand mark moves out of the footer: the logo (`assets/brand/icon.svg`) is the window and launcher icon, and appears in the first-run screen, the empty state and Settings › About (5.22, 5.2, 5.16).
- **Stats:** moved to the status line (see 3.6). The sidebar no longer changes height when a note opens.

### 3.4 Editor pane header

`[◧ open sidebar, only when the sidebar is hidden]  folder / folder / Title ●` … `[image] [find] [preview] [⋯]`

- Breadcrumb segments are dim (`text.muted`), each clickable: it reveals that folder in the explorer and focuses it. The title is `text.primary`, medium weight. Click or F2 (with the editor focused, F2 renames the open note) turns it into an auto-sized inline input: Enter/blur commits, Esc cancels, inline error under the header as a banner row (FE-009, FE-279). This uses the same rename command as the explorer (HP-030).
- The dirty dot `●` (`text.muted`) appears only while there are unsaved changes. The status line carries the full save state.
- Four fixed buttons, 28×28, icon 16 px: **Insert image**, **Find**, **Toggle preview** (pressed state when the preview is visible), **⋯ Document menu**. These were FrankMD's default pins plus the overflow. Pinning is gone: it was session-only (HP §4) and the palette makes it unnecessary.
- At narrow widths (< 720 px) image and find collapse into `⋯`, and breadcrumbs collapse to `…/ Title`.

**Document menu (`⋯`)**, in order, every row with its shortcut right-aligned:

| Group | Items |
|---|---|
| Insert | Image… (Ctrl+Shift+I), Video…, Table…, Code block…, Emoji (Ctrl+Shift+E), Link (Ctrl+K), Math block, Divider |
| Edit | Find (Ctrl+F), Replace (Ctrl+H), Go to line (Ctrl+G), Text format (Ctrl+M), Toggle task (Ctrl+Enter) |
| Check | Grammar check with AI (Ctrl+Shift+G) |
| Document | Copy contents, Copy path, Reveal in explorer, Open containing folder, Document info, Rename (F2), Move to…, Delete… |
| View | Preview (Ctrl+Shift+V), Typewriter (Ctrl+\\), Zen (F11), Line numbers (Ctrl+L) |

### 3.5 Preview pane header

`Preview` label (`text.muted`) … `− 100% +` zoom (FE-014, FE-219; click the percentage to reset to 100) · scroll-sync toggle (Phosphor `link`/`link-break`, pressed when on) · close (×). The header only exists while the preview is visible.

### 3.6 Status line

Left to right. Items separate by 12 px gaps, not dividers.

| Slot | Content | Behaviour |
|---|---|---|
| Mode block | Vim on: `NORMAL` / `INSERT` / `VISUAL` / `V-LINE` / `COMMAND`, 11 px semibold, on a solid mode colour (normal `accent`, insert `green`, visual `magenta`, command `yellow`), text `on-accent`. Vim off: hidden | FE-080. Replaces the bottom-right badge |
| Save state | Glyph + word: `● Unsaved` (muted), `✓ Saved` (success, fades to glyph only after 2 s), `↻ Saving`, `✕ Save failed` (danger), `⚠ Draft not stored` (danger) | FE-121…124. Click → Document info popover with last save time |
| Message slot | Transient messages, single instance: 2 s info, 5 s errors; `danger` text for errors. Ex command line when the user types `:` in vim normal mode | Replaces FE-025/026 toasts. Every message is also appended to the Diagnostics → Messages log so nothing is lost if missed |
| (flex space) | | |
| Context chip | Contextual action for the caret: `Ctrl+. Edit table`, `Ctrl+. Edit code block`, `Ctrl+. Open link`, `Ctrl+. View image`. Clickable | Replaces the non-clickable "cursor in table" hint (FE-022, quirk fixed) |
| Stats | `1,204 words` (CJK text: `3,812 chars`), click opens the Document info popover (words, characters, UTF-8 size, read time, line/total) | Replaces the sidebar stats panel (FE-100). Hidden when `show_stats` is off |
| Position | `12:4` (line:col) or `12/340` when line numbers are off | |
| Indicators | Icons only when not default: typewriter (`text-cursor`), read-only (`lock`), scroll sync off (`unlink`), AI running (`sparkles` pulsing at 1 Hz unless reduced motion) | |
| Vault | Vault name; danger colour + `hard-drive` icon when unreachable | |

At < 720 px: stats and vault drop; position stays.

---

## 4. Layout modes and responsive behaviour

### 4.1 Width classes

Computed from the window's logical width (after GPUI scale factor), re-evaluated on every resize, with 24 px hysteresis so a tile sitting on a boundary doesn't flicker.

| Class | Width | Sidebar | Preview | Header | Status line |
|---|---|---|---|---|---|
| **Narrow** | < 720 | Overlay drawer (Sheet) from inline-start, 85 % width max 320, 40 % dim scrim, closes on Esc/scrim/after opening a note; focus trapped | Single-pane swap only (Ctrl+Shift+V swaps editor ⇄ preview) regardless of `view_mode` | Collapsed (see 3.4) | Mode, save, message, position |
| **Medium** | 720–1099 | Docked, can be hidden; auto-hides when the preview opens and editor would drop below 320 px | Split if room for editor ≥ 320 + preview ≥ 280, else swap | Full | Full minus vault |
| **Wide** | ≥ 1100 | Docked | Split per `view_mode` | Full | Full |

4K tiles: nothing grows except the panes. The text column stays at `editor_width` ch, so a 4K full-screen tile shows a centred column with generous margins, as FrankMD does today. UI text scales with the GPUI scale factor and the `ui_font_size` setting, never with window width.

Sidebar visibility is persisted per vault (`sidebar_visible` is finally read, quirk §13). Opening a file from quick open or search reveals it in the tree but doesn't force the sidebar open (FrankMD did, FE-288). Use "Reveal in explorer" for that.

### 4.2 View modes

| Mode | Trigger | Behaviour |
|---|---|---|
| Split | `view_mode = split` (default) | Editor and preview side by side; Ctrl+Shift+V shows/hides the preview |
| Single | `view_mode = single`, or forced at narrow width | Ctrl+Shift+V swaps editor ⇄ full-width preview. Editor state is kept; returning focuses the editor at the same caret (FE-016, FE-174) |
| Typewriter | Ctrl+\\ (markdown only) | Caret line held at 50 % of the editor's height, half-height bottom padding. In split view the preview follows the caret's block to its top (FE-082/083, FE-218). It no longer force-opens the preview: if the preview is hidden, it stays hidden (FE-176 changed, see §11) |
| Zen (new) | F11 | Hides sidebar, preview and pane header; banners still show. Status line shrinks to mode + save + message at 50 % opacity until hovered or focused. Combines with typewriter. Esc doesn't leave zen (Esc is too busy); F11 again does |
| Media library | Ctrl+Shift+M | Replaces editor + preview. Esc or Ctrl+Shift+M returns, restoring the previous preview visibility (FE-017, FE-346) |

Preview visibility is persisted per vault now (FE-172 changed). Users who always write with the preview open were re-opening it on every launch.

### 4.3 Focus model

Four focus regions: sidebar, editor, preview, status line (only when it shows the ex command line). F6 / Shift+F6 cycles them, Ctrl+1/2/3 jumps directly to sidebar/editor/preview. The focused region shows the 2 px accent header line. Overlays trap focus and return it to the previous region on close.

---

## 5. Surfaces

Common rules for overlays:

- **Overlay anatomy:** top-anchored at 12 % of window height (or 16 px from the top at narrow widths), width `min(spec, window − 32)`, `surface.raised`, 1 px `border.default`, radius `R` (see 7.4), one soft shadow (0 8 24 / 30 % black on dark, 12 % on light). The scrim is `surface.base` at 60 %. No blur (GPUI can't blur cheaply, and Omarchy's Glassy mode already blurs the window).
- **Dialog anatomy:** header 40 px (title 14 px semibold + `×`), body, footer 48 px with buttons right-aligned in reading order: secondary then primary. The primary button repeats its shortcut in a small `kbd`: `Insert  Ctrl+↵`. **Ctrl+Enter confirms every dialog; Esc cancels every dialog except Recovery.**
- **Lists inside overlays:** 28 px single-line rows, 44 px two-line rows; selected row `selection` background + 2 px inline-start `accent` bar; arrow keys wrap at the ends (FE-287 clamped; we unify on wrap); PageUp/PageDown move by visible page; Home/End only when the input is empty.
- Backdrop click closes non-destructive overlays (palette, quick open, search, help, image viewer, insert dialogs with nothing selected). Dialogs with user input ask nothing and just close: input there is cheap. Recovery, grammar diff with edits, custom theme editor with changes, and table editor with changes ignore backdrop clicks.

### 5.1 Command palette (Ctrl+Shift+P)

600 px wide. Input with a `>` prompt, then a list of commands: `Category: Name` on the left in `text.primary` (category in `text.muted`), shortcut on the right as `kbd` chips. Fuzzy match over name, category and aliases (e.g. "zoom", "font", "wrap"). Empty query shows recently used commands first.

- Every command in the app is registered here (HP-018 registry). Commands that don't apply are hidden, not greyed out (e.g. "Table: Edit" outside a table), except a short "Unavailable" section at the bottom when the query matches only unavailable ones, with the reason ("Open a markdown note first").
- Toggle commands show their state as a check: `✓ Editor: Vim mode`.
- **Sub-modes** reuse the palette shell with a different prompt and list. Each is reachable directly:
  - `Theme: Select…` (Ctrl+K Ctrl+T is *not* used, since Ctrl+K is Link): live preview while arrowing; Enter keeps; Esc reverts. Order: **Follow Omarchy** (only when Omarchy is detected, shows current Omarchy theme name, default), **Follow system light/dark**, then built-ins with a light/dark glyph, then Custom themes, then "Customize theme…".
  - `Language: Select…`: **System (Omarchy: Português (Brasil))** first, then the 7 locales with native names. Switches live, no reload (fixes FE-501's reload).
  - `Vault: Switch…`: recent vaults, then "Open folder…".
  - `Go to line` (Ctrl+G): a `:` prompt. Accepts `N`, `+N`, `-N`, `-1` = last line, and `N:C`. Live preview: the editor scrolls as you type; Esc restores the caret (FE-099, FE-422).
- Typing a prefix in the palette input switches mode: `>` commands, `:` go to line, no prefix in quick open. That matches Zed and VS Code muscle memory.

### 5.2 Quick open (Ctrl+P) and the empty state

680 px wide, two columns at ≥ 900 px window width: results list (left, 60 %) and a 12-line text preview of the selected note (right, monospace, `code-bg`), FE-286. Single column below that.

- Empty query: **recent notes first** (the shared recents, FE-298…301), then other notes by mtime. Fixes quirk "Ctrl+P orders by mtime only".
- Rows: note name (matched characters in `accent`, semibold) + dim folder path; relative time ("2 h ago") right-aligned for recents.
- Enter opens; Ctrl+Enter opens and reveals in explorer; Alt+Enter opens in the preview pane only (read). "No notes match" + "Ctrl+N creates `query.md`" as the empty result. Ctrl+N from there pre-fills the New note dialog with the query.
- Same fuzzy scorer as FE-284, top 50 (was 10: we can afford it natively).

**Ctrl+Tab recent switcher:** while Ctrl is held, Tab/Shift+Tab walk the MRU list of notes opened this session; release opens. Small centred list, 400 px.

**Empty state (no note open), replaces Home (FE-296…305):** the editor pane shows the quick-open list *docked*: same component, no scrim, input focused, 560 px centred column. Above it, the FrankMD logo at 40 px (`assets/brand/icon.svg`) beside the vault name and note count. Below the list, a row of shortcut chips: `Ctrl+N New note`, `Ctrl+Shift+F Search`, `Ctrl+Shift+P Commands`. The recents section header offers "Clear history…" (confirm text keeps "for every device using this folder", FE-302). You can type immediately after launch. No marketing copy.

### 5.3 Search in files (Ctrl+Shift+F)

760 px wide, height up to 70 % of the window.

- Input row: query + toggles `Aa` (case, default off) and `.*` (regex, **default on** for parity with FE-290; invalid regex shows "Invalid pattern, searching as text" in `warning`) + a folder scope chip (defaults to the whole vault; pick a folder from the explorer selection with Alt+F).
- Status line under the input: `20 matches in 7 notes (showing first 20)`, `No matches`, or the error. A 2 px indeterminate bar along the input's bottom edge while searching (FE-291 spinner).
- Results grouped by note: note name + path header row, then match cards with ±3 context lines in monospace with a line-number gutter; the match line highlighted with `selection`, matched text in `accent` underline.
- Arrow keys move between matches (not headers), Enter opens at the line, centred, editor focused (FE-295). Ctrl+Enter keeps the overlay open after opening (to walk results). Esc closes and restores focus.
- A regex cheat-sheet sits behind the `?` button at the input's end, not printed inline.

### 5.4 Find and replace (Ctrl+F / Ctrl+H)

An **inline bar docked under the editor pane header**, Zed style, 36 px (72 px with replace). It doesn't cover text and doesn't need FE-097's minimise mode.

```
[ find…                    ] 3 of 12  ↑ ↓  Aa  .*  ×
[ replace…                 ] Replace  Skip  All
```

- Opens pre-filled with the selection (FE-033). Ctrl+H opens with the replace row; Ctrl+F on an open bar focuses and selects its input.
- Matches highlight live in the editor (all matches `selection` 50 %, current match `selection` + 1 px `accent` outline) and **recompute on document edits** (fixes the inventory's "not live on editor edits").
- Keys: Enter next, Shift+Enter previous, Alt+Enter replace + next, Ctrl+Alt+Enter replace all, Alt+C case, Alt+R regex, Alt+W whole word (new), Esc closes and returns focus to the editor with the current match selected. F3/Shift+F3 work with the bar closed.
- Regex errors show inline in `danger` in place of the counter. "Replaced 7" shows for 2 s in `success`.

### 5.5 Inline popovers at the caret

All of these are gpui-component popovers anchored to the caret's rectangle, flipped above when there's no room below, clamped to the pane.

| Popover | Trigger | Content |
|---|---|---|
| Slash menu | `/` at line start or after whitespace/bracket, in paragraph or heading (FE-088) | 12 commands (FE-089/090): icon + label + dim hint; fuzzy; Enter/Tab apply; Esc dismiss. Max 8 rows visible, 28 px rows, 280 px wide |
| Insert menu (new) | Alt+I, anywhere in markdown | The slash menu's list, usable where `/` can't trigger (inside lists, quotes, after text) |
| Wikilink completion | `[[` (FE-092) | Note names, scored as FE-092; dim folder path; inserts `path]]` |
| Link completion (new behaviour on top of FE-058) | After Ctrl+K inserts `[sel](url)` with `url` selected | Suggestions: URL on the clipboard first (if any), then notes as relative paths. Typing filters; Esc leaves the `url` placeholder selected, so the old template behaviour still works |
| Text format menu | Ctrl+M, or right-click with a selection (FE-085, FE-411) | 7 rows with underlined mnemonic letters B I S H U P L and the global shortcut at the end. Mnemonic letters apply directly |
| Editor context menu | Right-click (no selection), Shift+F10, Menu key | Cut, Copy, Paste, Paste as plain text · Format ▸ · Insert ▸ · Edit table/code block (when in one) · Toggle task |
| Emoji picker | Ctrl+Shift+E, `/emoji`, Document menu | 360×320 popover. Search on top, tab strip Emoji / Emoticons / Icons (Ctrl+Tab cycles tabs inside the popover; Left/Right on the focused tab strip), grid (10/5/8 columns), footer with large glyph + `:shortcode:`. Arrow keys move in the grid with wrap (FE-406). Inserts `:shortcode:` / `:ph-name:` / raw kaomoji exactly as today (FE-408). Localized search (FE-405, FE-502). A "Frequently used" first row (new, kept in state dir) |

### 5.6 Insert image (Ctrl+Shift+I, `/image`, header button, paste, drop)

Modal, 880×600 (full window minus 16 px margins at narrow widths).

```
┌ Insert image ─────────────────────────────────────────────── × ┐
│ Upload      │  [ search / dropzone / prompt, per source ]       │
│ Library     │  ┌──┐┌──┐┌──┐┌──┐┌──┐                              │
│ Folder      │  └──┘└──┘└──┘└──┘└──┘   grid, 4–6 columns          │
│ Web         │                                                   │
│ Google      │                                                   │
│ Pinterest   │                                                   │
│ AI generate │                                                   │
├─────────────┴───────────────────────────────────────────────────┤
│ Selected: sunset.jpg  Alt [sunset over the bay     ] Link [    ]│
│ Store in: Local ▾  Resize: none ▾  Key: frankmd/2026/11/        │
│                                         Cancel   Insert  Ctrl+↵ │
└─────────────────────────────────────────────────────────────────┘
```

- **Source rail** (160 px) instead of 7 horizontal tabs. Ctrl+1…7 jump to a source; Up/Down on the focused rail moves (FE-307 used Left/Right on a horizontal bar; on the vertical rail it's Up/Down). At narrow widths the rail becomes a Select dropdown above the content. Sources that need configuration (Google, AI, S3) stay listed with a dim "Set up" tag; selecting one shows a short explanation and an **Open Settings › Integrations** button, instead of a `.fed` snippet (FE-331, FE-333, FE-440).
- **Upload:** dropzone ("Drop images here or press Enter to choose files", Enter/Space open the portal FileChooser, OM-011) + the newest 10 uploads grid with WxH badges (FE-317…321). Pasted or dropped clipboard images arrive here pre-selected (FE-358). **OS files dropped on the editor also land here**, pre-selected (resolves FE-361).
- **Library:** collection select (Note images / Image folder), search, per-page, pager, per-thumbnail delete (`trash` button visible on the focused/selected card, never hover-only), FE-322…325.
- **Folder:** "Choose folder…" via the portal, then the same grid with search and pager (FE-326; the unsupported-API notice FE-327 goes away).
- **Web / Google / Pinterest:** query field (Enter searches), status line, 4-column grid with dimension badge and source caption; broken thumbnails removed (FE-328…332). Google uses scroll-to-load-more.
- **AI generate:** model label, prompt textarea (Enter generates, Shift+Enter newline), optional reference image chooser, a progress row "Generating with google/gemini-…  Esc to cancel", result preview with the revised prompt (FE-333…336).
- **Footer (shared):** selected name, Alt text (prefilled per FE-320/330), Link URL, and a single **storage row**: provider select (Local/S3, S3 disabled with reason when unconfigured, FE-355), resize select, destination key with the default `frankmd/YYYY/MM/…` (FE-315/316). Errors show inline above the footer in `danger` (FE-314). Upload progress is a real progress bar in the footer (FE-362 fixed).

### 5.7 Insert video

Modal, 720×520. Tabs (horizontal, 3 only): Upload / URL / YouTube. "Use Hugo shortcode" checkbox in the footer, persisted (FE-338). URL tab shows the detected YouTube thumbnail or "Video file (mp4)" live (FE-340/341). YouTube search: 2-column results with keyboard grid navigation, Enter inserts immediately (FE-342). Upload has a progress bar.

### 5.8 Table editor (`Ctrl+.` in a table, `/table`, Insert menu)

Modal, up to 960×640, sized to the table.

- Toolbar row: Rows `− 3 +`, Columns `− 3 +` (FE-383), and for the focused column an alignment segmented control `⇤ ↔ ⇥` (left/centre/right, **new**: fixes lost alignment colons).
- Grid of single-line inputs; header row bold on `surface.recessed`. Tab/Shift+Tab move between cells and Enter moves down; Tab in the last cell adds a row. Alt+↑/↓ move the row, Alt+←/→ move the column, Ctrl+Shift+Backspace deletes the row, Ctrl+Alt+Backspace deletes the column. The right-click cell menu keeps the same actions (FE-386) and shows these shortcuts.
- Pipes in cells are escaped as `\|` on output and unescaped on load (quirk fixed). Output pads columns as FE-388.
- Footer: Cancel / Insert (or **Update** in edit mode) `Ctrl+↵`.

### 5.9 Code block (`Ctrl+.` in a fence, `/code`, Insert menu)

Modal, 720×520. Language input with an inline suggestion list (FE-391/392), "Indent with tabs" checkbox, code area with monospace and **syntax highlighting as you type** using the same highlighter as the preview. Unknown language → inline warning row "‘pyhton’ isn't a known language. Did you mean python?" with "Use python" / "Keep pyhton" buttons, replacing the confirm dialog (FE-395). Ctrl+Enter inserts/updates.

### 5.10 Grammar check (Ctrl+Shift+G)

- Before sending: pending autosave flushes (FE-442). The editor goes read-only and a pane banner says `Checking grammar with anthropic · claude-sonnet-4  [Cancel  Esc]` with an indeterminate 2 px bar (replaces FE-445's full-screen overlay; the rest of the app stays usable, only this note is locked).
- Result: a modal, 90 % of window up to 1200×800. Header: title + provider/model badge (FE-449). Body: side-by-side word diff at ≥ 900 px (original left with `danger` strikethrough deletions, corrected right with `success` insertions), unified inline diff below 900 px. Hunk navigation `Alt+↓ / Alt+↑`, counter "Change 3 of 17".
- **Per-hunk accept/reject (new):** each change has a small ✓/✕ pair; `Y`/`N` with a hunk focused. Default is all accepted, preserving FE-451's behaviour. Edit toggle (Alt+E) swaps the right side for an editable text area (FE-450).
- Footer: Cancel / Apply changes `Ctrl+↵`. Applying is one undoable edit.
- Not configured: the dialog shows "Grammar check needs an AI provider" with provider buttons that open Settings › Integrations focused on that provider (FE-440).
- Errors: inline in the dialog body with a Retry button, never an alert (FE-447 changed).

### 5.11 Recovery (draft vs file on disk)

The one blocking modal. 90 % of window up to 1100×760. Esc and backdrop do nothing (FE-146).

- Title: **"This note has an unsaved draft that differs from the file"**. One sentence of cause: "The file changed on disk after your draft was saved (draft from 14:02, file modified 14:31)."
- Body: side-by-side word diff (FE-145), file on the left, draft on the right, timestamps as chips in each column header. Unified view below 900 px.
- Buttons: **Use file on disk** (secondary), **Keep both** (secondary, new: saves the draft as `<name> (draft 2026-11-04 14-02).md` next to the note and opens the file), **Restore draft** (primary). Renamed from "Use Server Version / Restore Backup", which made no sense without a server.
- Several pending records (FE-144) show as a list on the left (newest first) with the diff for the selected one.

### 5.12 Pane banners (states)

Banners stack in this priority order; each is 32 px, icon + one sentence + buttons at the end. The background is the role colour mixed 14 % into `surface.base`, with a 2 px role-colour bar at the inline-start, and `text.primary` text. FrankMD's amber banner fails contrast in light themes; this doesn't.

| Banner | Role | Text | Buttons | Replaces |
|---|---|---|---|---|
| Vault unreachable | danger | "Can't reach ~/notes. Edits are kept as drafts." | Retry | FE-156 connection lost |
| Read-only | info | "This note is read-only on disk." / "Grammar check running" | — / Cancel | FE-064, FE-444 |
| Changed on disk (new) | warning | "This note changed on disk." | Reload, Compare… | FE-120 last-write-wins (needs a file watcher, see §12) |
| Deleted on disk (new) | warning | "This note was deleted or moved outside FrankMD." | Save again, Close | FE-024 |
| Content loss | warning | "Saving would remove 1,420 characters (38 %)." | Undo, Save anyway | FE-128…132 |
| Draft storage failed | danger | "Drafts can't be written to ~/.local/state/frankmd." | Details | FE-139 |
| Storage provider fallback | info | "S3 is no longer available; new media will be stored locally." | Settings, Dismiss | FE-357 |

### 5.13 New note, New folder, Rename/Move, Delete

- **New note (Ctrl+N):** 480 px modal. Name input; segmented `Empty note | Hugo post`; for Hugo, an editable **slug** field prefilled from the slugified title (needed for CJK titles that slugify to empty; fixes BE quirk "empty Hugo slug produces //"). Destination row: `In: blog/` with a "Change…" button opening a folder picker (palette sub-mode). Live path preview under the inputs in monospace `text.muted`, `danger` when invalid (FE-255…258, FE-425). Create `Ctrl+↵` / Enter.
- **New folder (Ctrl+Shift+N):** 400 px, "In: <parent>" subtitle, nested paths allowed (FE-260).
- **Rename (F2):** inline in the explorer row (new; VS Code-like) and in the pane header title. The rename dialog is kept only for "Move to…" (palette/Document menu), which shows a folder picker plus name field.
- **Delete:** confirm dialog, destructive primary in `danger`, focus starts on Cancel (FE-373). Text says exactly what happens: "Move ‘Project brief’ to the trash?" Folders and multiple items list counts. Deleting moves to the freedesktop Trash (recommended, §12), which lets non-empty folders be deleted honestly (fixes FE-267 vs the confirm text).
- **Undo note creation:** keep FE-161/162 as is (Ctrl+Z at history start), dialog text "Delete the note you just created?" with a filename chip.

### 5.14 Explorer interactions (fixes)

| Interaction | Behaviour |
|---|---|
| Keyboard | ↑/↓ move, → expands or goes to first child, ← collapses or goes to parent, Enter opens a note or toggles a folder, Space toggles selection, Shift+↑/↓ extends, Ctrl+A selects all visible, F2 rename inline, Delete deletes, Ctrl+N new note in the focused folder, Ctrl+Shift+N new folder there, type-ahead jumps to the next name starting with the typed letters, `*` expands all siblings. Scroll follows the focus (quirk fixed) |
| Mouse | Click selects and opens notes; Ctrl+click toggles; Shift+click ranges (FE-237…239). Right-click opens the context menu (FE-247) |
| Background | Right-click on empty space: New note, New folder, Select all, Collapse all. Left-click on empty space only clears the selection (it no longer opens a menu: FE-251 surprised users, and issue #269 is about dismissing it) |
| Drag & drop | Drop on folder → move into it; **drop on a note → move into that note's folder** (quirk fixed: was root); drop on empty space → root. The target row shows a 1 px `accent` outline and `accent` 12 % fill; invalid targets show nothing and a `not-allowed` cursor. Multi-select drags show a count badge. A drop always reveals the destination (FE-270…275). Hovering a collapsed folder for 600 ms while dragging expands it (new) |
| OS files dropped on the tree | `.md` files are copied into the target folder (new); images say "Drop images onto the editor to insert them" in the status line |
| Expansion state | Persisted per vault in the state dir (quirk fixed) |
| Hidden items | Dotfiles, symlinks, `.frankmd/`, `images/` and `videos/` without markdown stay hidden (FE-225/226) |

### 5.15 Media library (Ctrl+Shift+M)

Replaces editor + preview. Its own pane header: `Media library` · segmented `Images 42 | Videos 3` · search · grid/list toggle · storage chip ("Local" or "S3: my-bucket").

- Grid: 160 px cards (thumbnail 16:10, name, size and date, usage line "Used in 3 notes" / "Not used" / "Checking…", FE-347…349). List mode: 32 px rows with a 24 px thumbnail, name, size, date, usage, for large libraries and narrow tiles.
- Every card shows its actions at rest as small icon buttons: Insert, Copy path, Rename (S3), Delete. Enter on a focused card opens the detail sheet (FE-351): a right-side sheet (420 px) with a large preview, metadata, "Used in" list (click opens the note, FE-350), and actions. That merges the separate usage dialog and preview dialog.
- Empty: "No images yet. Paste or drop an image into a note to add one."

### 5.16 Settings (Ctrl+,)

A modal sheet, 960×680 (full window minus 16 px at narrow widths), with a search field on top that filters rows across all categories.

```
┌ Settings ───────────────────── [ search settings… ] ──── × ┐
│ Scope: [ All vaults | This vault (notes) ]                 │
├──────────────┬─────────────────────────────────────────────┤
│ Appearance   │ Theme              Follow Omarchy ▾   omarchy│
│ Editor       │                    Tokyo Night · Customize…  │
│ Writing      │ Interface font     System sans ▾             │
│ Preview      │ Language           System (English) ▾        │
│ Files        │ ...                                          │
│ Media        │                                              │
│ Integrations │                                              │
│ Hugo         │                                              │
│ Diagnostics  │                                              │
│ About        │                                              │
└──────────────┴─────────────────────────────────────────────┘
```

- **Scope toggle**, like VS Code's User/Workspace: *All vaults* writes `~/.config/frankmd/config.toml`; *This vault* writes `.fed` in the vault (format preserved, BE-010/011). Rows overridden in the vault show a dot and a "Reset to global" action.
- **Provenance chip** on every row whose value comes from somewhere the user didn't just set: `.fed`, `env OPENAI_API_KEY`, `default`, `omarchy`. Hover isn't needed: the chip is text (HP-013, issue #214).
- Changes apply live; saved with a 300 ms debounce; failures show inline under the row in `danger` and keep the previous value.
- Rows are 44 px: label (13 px) + description (12 px `text.muted`) on the inline-start, control on the inline-end. Controls: Switch, Select (gpui-component dropdown, opens as a popover, not clipped by the sheet: the FE-259 problem doesn't exist natively), Slider with value readout, NumberInput, text Input, secret Input (masked, "Show" button, stored in the keyring via the Secret portal with a 0600 fallback).

| Category | Settings |
|---|---|
| Appearance | Theme (+ Customize theme…), Interface font (System sans / System monospace / any installed), Interface size (12–16 px, default 13), Language, Reduce motion |
| Editor | Font family ("System monospace" default via fontconfig, plus the 9 bundled), Font size (12–24), live preview box (FE-484), Text width (40–200 ch), Line numbers (Off/Absolute/Relative), Indentation (Tab, 2, 4, 8; **now applied**, quirk fixed), Vim mode |
| Writing | Typewriter mode, Recenter caret on click, Show document stats, Spell check (future, disabled with "Not yet available") |
| Preview | Layout (Split/Single), Zoom (50–200 %), Scroll sync, Open external links in (Browser / Ask) |
| Files | Notes folder (vault path, "Change…"), Show hidden files (off), Delete to trash (on) |
| Media | Storage for new media (Local/S3), Image folder (`images_path`), Accepted image types, Accepted video types, S3 bucket / region / endpoint / path-style / access key / secret key, "Test connection" button |
| Integrations | AI: provider (Auto/OpenAI/Anthropic/OpenRouter/Ollama/Gemini/**OpenAI-compatible**, HP-020), model, per-provider key/base URL with "Test"; Image generation model; YouTube API key; Google API key + CSE id |
| Hugo | Path style (Dated/Flat), Default folder for posts (`blog`, HP-014: the backend owns this default), Front-matter template (`.hugo_template.md`, "Edit template" opens it as a note) |
| Diagnostics | Logs (last 100 lines, refresh, copy; FE-415/416), Config sources table (key, value masked for secrets, source, env var name; FE-417, BE-090), Messages history (status-line messages of this session), "Open config file" for both scopes |
| About | Logo 64 px (`assets/brand/icon.svg`), FrankMD, the existing description string (`settings.about.*` in `assets/locales`), version, copyright line, "Source code" link (opens browser), "Copy debug info" |

FE-029 ("all shortcuts disabled while Settings is open") changes: global shortcuts are disabled except Esc and Ctrl+, (which closes). Ctrl+S inside Settings does nothing.

### 5.17 Theme picker and custom theme editor

- Picker: the palette sub-mode in 5.1. Each row: a 4-swatch strip (background, foreground, accent, selection), name, light/dark glyph. Arrowing applies the theme live to the whole window.
- **Customize theme…** opens a modal (880×640): left column with Name, Base theme select, then the colour list grouped as **Basics** (Background, Text, Accent) and collapsible **Surfaces**, **Text**, **States**, **Headings**, **Explorer**, **Code** groups (38 keys, FE-467). Each row: swatch button (opens gpui-component ColorPicker popover) + hex input with validation (FE-469). The right column is a live specimen card: heading, paragraph, link, inline code, code block, selection, a list row, a button (FE-474). The whole app also previews live (FE-468); Cancel/Esc reverts. A non-blocking contrast warning appears inline next to failing pairs (FE-470). "Reset colors" (FE-471).
- When following Omarchy, "Customize theme…" offers "Start from current Omarchy colours".

### 5.18 Help (F1) and Keyboard shortcuts (Ctrl+/)

- Help: 760×600 modal with tabs **Markdown** / **Keyboard** / **Vim** (FE-418…421). F1 toggles (quirk fixed). Ctrl+Tab cycles tabs.
- Keyboard tab is generated from the command registry, grouped as in §6, with a filter field. Ctrl+/ opens Help directly on this tab. `?` opens it too when focus is in the sidebar or preview (not in text).

### 5.19 Diagnostics (Ctrl+Shift+O)

Opens Settings at Diagnostics → Logs. Keeps the shortcut's muscle memory and removes the separate log viewer.

### 5.20 Image viewer

Full-window overlay on a `surface.base` 92 % scrim, image fitted with 32 px margins, caption (alt text) and `1/3` counter when the note has several images. Left/Right move between the note's images (new), `+`/`-`/`0` zoom (new), drag pans when zoomed, Esc/click outside closes (FE-205…208). Focus returns to the image in the preview.

### 5.21 Native preview rendering

Rendered from the comrak AST into GPUI elements (HP-009). Faithful to FE-178…220 with these native decisions:

| Element | Rendering |
|---|---|
| Body text | UI sans at preview zoom × 15 px, line height 1.6, `text.primary`, max width `editor_width`ch + 3rem |
| Headings | 2 / 1.5 / 1.25 / 1.1 / 1 / 1 em, semibold; colours h1 `heading-1`, h2 `heading-2`, h3 `heading-3`, h4–6 `accent`; h1/h2 with a 1 px `border.subtle` underline (FE-220). Headings get anchors (new: clicking a `#section` link scrolls) |
| Code blocks | `code-bg` surface, radius `r.sm`, monospace 0.9 em, syntax colours from `syntax-*`, a language label at the top-left and a **copy button always visible** at the top-right in `text.faint` (FE-201/203 fixed) |
| Math | RaTeX → SVG → image, inline baseline-aligned, block centred and horizontally scrollable; errors as inline `danger` text (FE-189…191) |
| Images | Fitted to column width, radius `r.sm`; each image focusable; Enter/click opens the viewer |
| YouTube embeds and `<video>` | **No webview**: a 16:9 card with the thumbnail (YouTube) or first frame (local video, via ffmpeg if present), a play glyph, the title, and two buttons: "Open in browser" (YouTube) / "Play in mpv" (local, Omarchy ships mpv; else "Open with default app") |
| Task lists | Accent checkboxes, clickable, toggling the source line (FE-197). Keyboard: focus + Space |
| Wikilinks | `accent` text with a dashed underline; broken targets in `danger` dashed (new) |
| External links | Open in the default browser via the portal (OpenURI). Never navigate inside the app (FE-210 changed) |
| Raw HTML | Allow-listed subset (ammonia policy) mapped to elements; anything else renders as escaped text |
| Front matter | Hidden (FE-181); a small "Front matter ▸" disclosure row at the top shows it on demand (new) |
| Hugo shortcodes | Shown as a dim inline chip `{{< youtube … >}}` (FE-196 literal, styled) |

Scroll sync, re-anchoring and the scroll lock follow FE-212…217 using real source positions from comrak (quirk fixed).

### 5.22 Onboarding and empty states

| State | What the window shows |
|---|---|
| First launch, no vault configured | Editor pane empty state with the logo at 72 px (`assets/brand/icon.svg`): "Choose a folder for your notes." Buttons: **Choose folder…** (portal) and **Use ~/notes**. Status line: "Following Omarchy theme: Tokyo Night" (or nothing). No wizard |
| Vault empty | Sidebar tree: "No notes yet." Editor empty state: docked quick open with "Ctrl+N creates your first note" |
| Vault missing or unreadable at launch | Empty state: "~/notes isn't available (permission denied)." Buttons: Retry, Choose another folder… |
| Note not found (deep link / CLI path / wikilink to a missing note) | Status line message + quick open opened with the name prefilled: "No note named ‘X’. Ctrl+N creates it." Replaces FE-024's 5-second red header |
| Preview of a non-markdown file | Preview pane: "Preview is available for Markdown notes." (localized; quirk fixed) |
| AI, YouTube, Google, S3 not configured | The surface that needs it says so in place and links to the exact Settings row |

### 5.23 Quit and close

Ctrl+W closes the note (returns to the empty state). Ctrl+Q quits; if any draft couldn't be saved (vault unreachable), it asks: "2 notes have unsaved changes that couldn't be written to ~/notes. Quit anyway? They stay as drafts and will be restored next time." Otherwise it saves and quits without asking (drafts make this safe). Super+W (Hyprland close) goes through the same path.

---

## 5a. Reused FrankMD assets

Everything visual that FrankMD already ships is reused from `assets/` (extracted from FrankMD; see `assets/README.md`; never edited by hand). Which surface uses what:

| Asset | Used by |
|---|---|
| `assets/brand/icon.svg` (pink Frankenstein logo) | Window icon and `frankmd.svg` in hicolor for the `.desktop` file/launcher (OM-006, OM-013); first-run screen (72 px, 5.22); empty state (40 px, 5.2); Settings › About (64 px, 5.16). Freedesktop notifications use the icon name `frankmd` (OM-014). PNG sizes (`icon-192.png`, `icon-512.png`, favicons) for packaging and README only |
| `assets/icons/phosphor.json` | Every UI icon (7.7), and the emoji picker's **Icons** tab plus `:ph-name:` rendering in the preview (5.5, 5.21, FE-403/186) — one icon set for chrome and content |
| `assets/themes/builtin.json` | The 18 built-in themes (same ids as `.fed` `theme`), listed in the theme picker with swatches built from `bg-primary`, `text-primary`, `accent`, `selection` (5.17); fallback when Omarchy is absent (OM-005); base palettes for the custom theme editor (FE-467) |
| `assets/emoji/shortcodes.json` | Emoji picker **Emoji** tab, search keywords, `:shortcode:` → Unicode in the preview (5.5, 5.21) |
| `assets/emoji/emoticons.json` | Emoji picker **Emoticons** tab |
| `assets/emoji/names/{en,es,ja,ko,pt}.json` | Localized emoji search (he falls back to en, FE-405/502) |
| `assets/locales/*.yml` | Every user-facing string, including the existing labels reused verbatim in this spec (save states, banners, dialog titles, settings labels and descriptions, About text); new strings in this spec are added to all 7 files (HP-022) |
| `assets/fonts/web/*.woff2` | Reference only: the 9 `editor_font` ids map to installed system fonts through fontconfig (7.2); the Settings font list keeps the same ids and labels |
| `assets/fonts/katex/*.woff2` | Math in the preview, if the chosen math renderer needs the KaTeX fonts (converted to TTF) (5.21) |

The FrankMD boot splash gradient (`#2D1B4E → #FF6B9D`) is not reused: the app has no splash (FE-001). The logo's pink stays a brand colour only; it never becomes a UI colour role, since the UI follows the user's theme.

---

## 6. Keyboard map

Notation: Ctrl = Ctrl on Linux (Cmd on macOS if ever built). "Kept" means same as FrankMD. "Changed"/"New" are called out. **No Super bindings** (Hyprland owns them). Omarchy's non-Super globals are avoided: Alt+Tab, Alt+Shift+Tab, Ctrl+Alt+Tab, Ctrl+Alt+Shift+Tab, Ctrl+Alt+Delete, Print, Alt+Print. Ctrl+Space is non-consuming in Omarchy (it only affects terminals), so it remains "trigger completion".

Matching uses GPUI keystrokes with both shifted forms bound where layouts differ (e.g. `ctrl-shift-\` and `ctrl-|`, `ctrl-shift-=` and `ctrl-+`). That fixes FE-042's US-layout bug. During IME composition no app shortcut fires (§8).

### 6.1 Global

| Action | Key | Status |
|---|---|---|
| Command palette | Ctrl+Shift+P | New |
| Quick open | Ctrl+P | Kept (recents first) |
| Recent notes switcher | Ctrl+Tab / Ctrl+Shift+Tab | New |
| Search in files | Ctrl+Shift+F | Kept |
| New note | Ctrl+N | Kept |
| New folder | Ctrl+Shift+N | New |
| Save now | Ctrl+S | Kept |
| Close note | Ctrl+W | New |
| Quit | Ctrl+Q | New (vim mode: Ctrl+Q is blockwise visual; use `:qa`) |
| Back / Forward (note history) | Ctrl+Alt+← / Ctrl+Alt+→, mouse back/forward | New (replaces browser history, FE-002) |
| Toggle sidebar | Ctrl+E | Kept |
| Toggle preview | Ctrl+Shift+V | Kept (tooltip fixed) |
| Typewriter mode | Ctrl+\\ | Kept |
| Scroll sync | Ctrl+Shift+\\ | Kept, now works on US layouts |
| Zen mode | F11 | New |
| Media library | Ctrl+Shift+M | New |
| Settings | Ctrl+, | New |
| Help | F1 (toggles) | Kept, fixed to toggle |
| Keyboard shortcuts | Ctrl+/ | New. CodeMirror's Mod-/ toggle-comment moves to Ctrl+Alt+/ |
| Diagnostics (logs) | Ctrl+Shift+O | Kept (opens Settings › Diagnostics) |
| Focus sidebar / editor / preview | Ctrl+1 / Ctrl+2 / Ctrl+3 | New |
| Cycle focus regions | F6 / Shift+F6 | New (also the escape hatch from Tab-indenting editors) |
| Theme picker | palette "Theme: Select…" | New (no default shortcut) |
| Language picker | palette "Language: Select…" | New |
| Escape | Closes the topmost layer: inline rename → popover/menu → overlay or dialog → find bar → narrow drawer → media library → explorer multi-selection. Never leaves zen mode | Kept as a priority chain (FE-048) |
| Redo deleted created-note (no note open) | Ctrl+Y / Ctrl+Shift+Z | Kept (FE-049) |

### 6.2 Editor

| Action | Key | Status |
|---|---|---|
| Bold / Italic / Strikethrough / Inline code | Ctrl+B / Ctrl+I / Ctrl+Shift+S / Ctrl+\` | Kept |
| Link | Ctrl+K (+ link completion) | Kept, enhanced |
| Text format menu | Ctrl+M | Kept (CodeMirror's Ctrl+M tab-focus mode dropped; F6 replaces it) |
| Insert menu | Alt+I | New |
| Insert image | Ctrl+Shift+I | New |
| Emoji picker | Ctrl+Shift+E | Kept |
| Context action (edit table/code block, open link, view image) | Ctrl+. | New |
| Toggle task | Ctrl+Enter | Kept (CodeMirror's insertBlankLine binding removed, quirk fixed) |
| Find / Replace | Ctrl+F / Ctrl+H | Kept (inline bar) |
| Find next / previous | F3 / Shift+F3, Enter / Shift+Enter in the bar | Kept |
| Go to line | Ctrl+G | Kept (palette `:` mode) |
| Line numbers cycle | Ctrl+L | Kept |
| Editor width ± | Ctrl+Shift+= / Ctrl+Shift+- | Kept |
| Font size ± / reset (focused pane: editor font or preview zoom) | Ctrl+= / Ctrl+- / Ctrl+0 | New |
| Grammar check | Ctrl+Shift+G | New |
| Rename note | F2 (editor focused) | New |
| Undo / Redo | Ctrl+Z / Ctrl+Y, Ctrl+Shift+Z | Kept (created-note boundary FE-062) |
| Select next occurrence | Ctrl+D | Kept |
| Select all matches of selection | Ctrl+Shift+L | Kept. **Note:** Ctrl+L stays line numbers |
| Add cursor above / below | Ctrl+Alt+↑ / ↓ | Kept |
| Move line / copy line | Alt+↑/↓ / Shift+Alt+↑/↓ | Kept |
| Delete line | Ctrl+Shift+K | Kept |
| Indent / outdent | Tab / Shift+Tab, Ctrl+] / Ctrl+[ | Kept (indent unit from settings, quirk fixed) |
| Toggle HTML comment | Ctrl+Alt+/ | Changed (was Ctrl+/) |
| Trigger completion | Ctrl+Space | Kept |
| Rectangular selection | Alt+drag | Kept |
| Surround selection by typing `* _ \` ~ ^ =` | — | Kept (FE-059) |
| Paste as plain text | Ctrl+Shift+Alt+V | New (Ctrl+Shift+V is preview) |

### 6.3 Vim mode

FrankMD's keymap with the same unmapped Ctrl keys so app shortcuts win: `<C-f> <C-n> <C-p> <C-e> <C-b> <C-i> <C-v>`, plus `<C-w>` (close note) and `<C-.>`. `<C-d>/<C-u>` paging and `<C-q>` blockwise visual stay vim's (FE-056). IME is enabled only in insert mode.

Ex commands, typed in the status line: `:w` save, `:q` close menus/overlays (kept), `:wq`/`:x` save then close overlays, `:qa` quit (new), `:e`/`:find` quick open, `:e <name>` open that note directly (new), `:Explore` toggle sidebar, `:n`/`:prev` next/previous note in tree order, `:help`, `:set nu`/`:set rnu`/`:set nonu` line numbers (new), `:noh` clear find highlights (new), `:<N>` go to line (new).

### 6.4 Overlays and lists

| Context | Keys |
|---|---|
| Any list | ↑/↓ (wrap), PgUp/PgDn, Enter, Esc |
| Any dialog | Ctrl+Enter confirm, Esc cancel, Tab/Shift+Tab move, Ctrl+Tab/Ctrl+Shift+Tab next/previous tab or source |
| Insert image | Ctrl+1…7 jump to source |
| Search in files | Ctrl+Enter open and keep searching, Alt+F scope to folder |
| Grammar diff | Alt+↓/↑ next/previous change, Y/N accept/reject focused change, Alt+E edit |
| Table editor | Tab/Shift+Tab cells, Enter down, Alt+arrows move row/column, Ctrl+Shift+Backspace delete row, Ctrl+Alt+Backspace delete column |
| Emoji grid | arrows with wrap, Enter, Ctrl+Tab next tab |
| Explorer | see 5.14 |
| Media library | arrows across the grid, Enter details, I insert, Delete delete, / focus search |
| Image viewer | ←/→ previous/next, + / - / 0 zoom, Esc close |
| Sidebar or preview focused | `?` keyboard shortcuts |

---

## 7. Design tokens

### 7.1 Colour roles

The app's colour API is a set of **roles**. Roles are derived from Omarchy's `colors.toml` after the alias cascade in OM-001 (mixing in sRGB, linear per channel, like `omarchy-theme-color mix`). FrankMD's 38 `--theme-*` keys remain the storage format for built-in and custom themes; the table shows both so the converter is unambiguous. Every role also lists the closest gpui-component `ThemeColor` field we set (names per gpui-component 0.9; verify on upgrade).

| Role | Omarchy source | FrankMD key | gpui-component field | Used for |
|---|---|---|---|---|
| `surface.base` | `background` | bg-primary | `background` | Editor, preview, pane headers |
| `surface.recessed` | `dark_background` | bg-secondary | `sidebar`, `title_bar`, `tab_bar` | Sidebar, status line, table header row |
| `surface.raised` | `lighter_background` | bg-tertiary | `popover`, `secondary` | Overlays, menus, dialogs |
| `surface.sunken` | `darker_background` | — | `input` (bg) | Text inputs inside raised surfaces |
| `surface.hover` | mix(background, foreground, 8 %) | bg-hover | `list_hover`, `secondary_hover` | Hovered rows, buttons |
| `surface.code` | `dark_background` | code-bg | — | Code blocks, quick-open preview |
| `text.primary` | `foreground` | text-primary | `foreground` | Body text |
| `text.strong` | `bright_foreground` | — | — | Titles, selected row text |
| `text.secondary` | mix(foreground, background, 8 %) | text-secondary | — | Secondary labels |
| `text.muted` | `dark_foreground` | text-muted | `muted_foreground` | Paths, descriptions, breadcrumbs |
| `text.faint` | mix(dark_foreground, background, 35 %) | text-faint | — | Disabled, placeholders, kbd chips |
| `border.default` | `muted` | border | `border`, `input` (border) | Overlay outlines, inputs, splitters while hovered |
| `border.subtle` | mix(muted, background, 50 %) | (derived) | `sidebar_border`, `title_bar_border` | Pane dividers, header underlines |
| `accent` | `accent` | accent | `primary`, `ring`, `link`, `list_active_border` | Focus, active pane line, primary buttons, links, matches |
| `accent.hover` | mix(accent, foreground, 15 %) | accent-hover | `primary_hover` | |
| `on-accent` | black or white by WCAG contrast against `accent` (app skill `accentForeground`) | accent-text | `primary_foreground` | Text on accent |
| `selection` | `selection` | selection | `selection`, `list_active` | Text selection, selected rows |
| `selection.text` | `bright_foreground` | selection-text | — | |
| `caret` | `bright_foreground` | — | `caret` | Editor caret (Omarchy has no cursor key) |
| `scrollbar` | `muted` | scrollbar | `scrollbar_thumb` | Fixes FrankMD's invisible Omarchy scrollbar |
| `scrollbar.hover` | `dark_foreground` | scrollbar-hover | `scrollbar_thumb_hover` | |
| `success` | `green` | success | `success` | Saved, insertions, Replaced N |
| `warning` | `yellow` | warning | `warning` | Banners, content loss |
| `danger` | `red` | error | `danger`, `destructive` | Errors, deletions, destructive buttons |
| `info` | `cyan` | — | `info` | Read-only banner |
| `heading.1/2/3` | `magenta` / `blue` / `cyan` | heading-1/2/3 | — | Editor and preview headings |
| `icon.folder` / `icon.file` / `icon.config` | `accent` / `dark_foreground` / `yellow` | folder-/file-/config-icon | — | Explorer |
| `syntax.*` | per OM §2.6 (keyword magenta, function yellow, type cyan, string green, number orange, constant bright_cyan, property bright_blue, comment muted, punctuation dark_foreground, invalid red, identifier/operator foreground) | syntax-* | highlight theme | Editor and preview code |
| `mode.normal/insert/visual/command` | `accent` / `green` / `magenta` / `yellow` | — | — | Vim mode block |

Derived surfaces:

- **Banner background** = mix(role colour, `surface.base`, 14 %), with a 2 px role-colour bar at the inline-start.
- **Diff deletions** = `danger` text on mix(red, base, 16 %) with strikethrough; **insertions** = `success` text on mix(green, base, 16 %).
- **Scrim** = `surface.base` at 60 % (92 % for the image viewer).
- **Drop target** = `accent` at 12 % fill + 1 px `accent` outline.

Contrast rules (OM-004): every `text.*` and `syntax.*` role is run through `ensure_contrast(colour, its surface, 4.5)` except `text.faint` (3.0, used for disabled and decorative text only). `on-accent` must reach 4.5 against `accent`. Theme loading fails over to the last good palette if `colors.toml` is mid-swap (HP-024).

Light/dark comes from `mode` (OM-002), never from luminance unless `mode` is missing. Note that Omarchy's `lighter_background` is *darker* than `background` in light themes like Catppuccin Latte: "raised" means one step toward the foreground, which is what we want.

### 7.2 Typography

| Role | Family | Size / line height | Weight |
|---|---|---|---|
| Editor text | `editor_font`, default **System monospace** (fontconfig `monospace`; JetBrainsMono Nerd Font on stock Omarchy) | `editor_font_size` (14) / 1.6 | 400; headings in the editor 600 |
| UI | **System sans** (fontconfig `sans-serif`), option System monospace for the all-mono Omarchy shell look | `ui_font_size` 13 / 20 | 400; titles 600; labels in lists 400 |
| UI small | same | 12 / 16 | 400 (paths, descriptions, shortcuts) |
| UI micro | same | 11 / 14 | 600 (mode block, badges, counters) |
| Dialog titles | same | 14 / 20 | 600 |
| Preview body | System sans | 15 × zoom / 1.6 | 400 |
| Preview code, kbd chips, paths in previews | System monospace | 0.9 em | 400 |

No letter-spaced all-caps labels anywhere (the mode block uses caps because that's the vim convention). Numbers in status line, counters and stats use tabular figures.

Live font follow (OM-016): watch `~/.config/fontconfig/` and re-resolve `monospace`/`sans-serif` with a 300 ms debounce.

### 7.3 Spacing and sizing

4 px base grid. Scale: **2, 4, 6, 8, 12, 16, 24, 32, 48**.

| Element | Size |
|---|---|
| Status line | 24 px |
| Pane header, sidebar header | 32 px |
| Dock footer | 28 px |
| Tree row | 24 px |
| List row (overlays) | 28 px; two-line 44 px |
| Settings row | 44 px minimum |
| Buttons | 28 px (default), 24 px (compact, in headers and tables), icon buttons 28×28 with a 16 px icon |
| Inputs | 28 px; in overlays' main search field 36 px |
| Icon sizes | 16 px (UI), 14 px (inline in text rows), 12 px (status line) |
| Gaps | 4 px icon-to-label, 8 px between controls, 12 px status-line items, 16 px dialog padding |
| Hit target | ≥ 24×24 px |

### 7.4 Corner radius

Follow Omarchy (OM-015): read `hyprctl -j getoption decoration:rounding` at startup, on theme change and on window focus-in. Omarchy's default is **0**; `corners-rounded` sets 6.

| Token | Value | Used for |
|---|---|---|
| `R` | Hyprland rounding (0 default) | Overlays, dialogs, sheets, popovers, banners: none (they're full-bleed) |
| `r.sm` | min(4, R) | Buttons, inputs, chips, kbd, list selection, cards, code blocks, images in preview |
| `r.full` | 999 | Only true circles (switch knob, avatar, colour swatch) |

Without Hyprland: R = 6. The mockup has a toggle to check both.

### 7.5 Elevation

Flat. Panels are separated by 1 px borders and surface steps, never shadows. Only floating layers (overlays, popovers, menus, the drag ghost) get a single shadow `0 8 24 rgba(0,0,0,.30)` (dark) / `.12` (light).

### 7.6 Motion

Motion answers an action; nothing moves by itself.

| Event | Animation |
|---|---|
| Overlay/popover open | opacity 0→1 + translateY(−4 px → 0), 120 ms ease-out. Close: 80 ms fade |
| Drawer (narrow sidebar) | slide 160 ms ease-out |
| Banner appear | height 0→32, 120 ms |
| Status message | fade in 100 ms, out 200 ms |
| Typewriter / scroll sync / reveal scrolling | 120 ms ease-out, cancelled by user scroll |
| Save state `✓` | none: the glyph just changes |
| Progress | indeterminate 2 px bar, 1.2 s loop |

**Reduce motion** (Settings › Appearance, default from the portal `org.freedesktop.appearance` reduced-motion key when available) turns all of the above into instant changes and disables the AI status pulse.

### 7.7 Icons

**Phosphor (regular weight)**, the set FrankMD already uses, for continuity. The curated 201 icons in `assets/icons/phosphor.json` (`[name, svgPathData, viewBox, keywords]`, MIT) are registered with GPUI's `AssetSource` as monochrome SVGs and rendered with `svg().path(...)`, tinted by the current text colour, at 16 px (UI), 14 px (rows) and 12 px (status line). gpui-component's built-in Lucide `IconName`s are used only inside its own widgets (e.g. a Select's chevron) where overriding isn't worth it; if a needed glyph is missing from the curated set, add it from `@phosphor-icons/core` to the same JSON rather than mixing families. Mapping: sidebar `sidebar`, note `file-text`, folder `folder`/`folder-open`, new `file-plus`/`folder-plus`, search `magnifying-glass`, preview `eye`, menu `dots-three`, image `image`, video `video-camera`, table `table`, code `code`, emoji `smiley`, link `link`/`link-break`, AI `sparkle`, library `images`, settings `gear`, help `question`, trash `trash`, rename `pencil-simple`, warning `warning`, error `x-circle`, info `info`, vault offline `database`, history `clock`, diff `git-merge`, changed on disk `arrows-clockwise`, typewriter/writing `notepad`, read-only `lock`, storage `cloud`, diagnostics `terminal`, theme `palette`, alignment `text-align-left`/`text-align-center` (right = mirrored left), slash menu `hash`, `list-bullets`, `list-numbers`, `list-checks`, `quotes`, `sigma`, `minus`. Directional icons (carets, arrows, sidebar) flip under RTL.

---

## 8. RTL, CJK and IME

### 8.1 RTL (Hebrew today; Arabic, Persian, Urdu possible later)

**Constraint:** GPUI has no RTL layout or bidi text support yet (gpui-kit#3679, zed#31102). FrankMD today also has no RTL layout (FE-503), so v1 doesn't regress, but we must not make it harder later.

- Every layout rule in this doc is written in **start/end** terms. Implement layout with a single `LayoutDirection` in the root context and helper functions (`h_flex_start()`, `pad_inline_start()`), never raw left/right, so mirroring is a switch when GPUI gains it.
- Mirroring map for when it lands: sidebar docks to the right; preview goes to the left of the editor; status line order reverses; breadcrumbs read right to left; chevrons and arrow icons flip; tree indentation is inline-start. Not mirrored: media playback glyphs, the clock and numbers, code blocks, and the editor's line-number gutter for LTR paragraphs.
- Editor and preview text use per-paragraph base direction from the first strong character (what GPUI is moving toward). Until GPUI shapes bidi properly, Hebrew strings in the UI render LTR-ordered runs. Keep Hebrew UI strings free of embedded LTR fragments where possible (put shortcuts in separate `kbd` elements rather than inline in sentences).
- `he` stays a supported locale. The release notes should say RTL layout isn't there yet.

### 8.2 CJK

- Fonts: pass the content language (from the note's front matter `lang`, else the UI locale) to text shaping so fontconfig picks the right Noto CJK variant (OM-012).
- Stats: CJK text counts characters, not whitespace-separated words; read time uses 500 characters/minute for CJK runs. The status line shows `chars` instead of `words` when CJK dominates.
- Line height 1.6 already fits CJK; don't reduce it below 1.5 at any size.
- Hugo slugs: CJK titles slugify to empty, so the New note dialog exposes the slug field (5.13).
- Emoji search is localized for ja and ko (FE-405).
- UI strings: Japanese and Korean labels run 30–60 % longer in some places and shorter in others; buttons size to content, never fixed widths. The settings label column wraps to two lines before truncating.

### 8.3 IME (fcitx5 via text-input-v3, OM-007)

- Preedit renders inline at the caret with a 1 px underline in `text.primary`, the preedit cursor shown, no commit until the IME commits.
- While preedit is non-empty: Enter, Backspace, arrows, Tab, Esc and every app shortcut go to the IME, not to the editor or app.
- The candidate window follows the caret: update the IME cursor area on every caret move and scroll, including inside overlays' inputs (search, palette, dialogs).
- IME disabled in secret inputs and in vim normal/visual mode.

---

## 9. Accessibility

- **Keyboard:** every feature reachable without a pointer (§6). Focus order follows reading order. F6 cycles regions. No keyboard traps; the editor's Tab indents, so F6 and Ctrl+1/2/3 are the documented way out.
- **Focus visibility:** 2 px `accent` outline offset 1 px on buttons, inputs, rows and cards (inside overlays: inset, so it isn't clipped). The focused pane shows the header accent line.
- **Contrast:** 4.5:1 for text, 3:1 for UI component boundaries and focus indicators, enforced at theme load (7.1). The theme editor warns on custom pairs.
- **No hover-only information:** shortcuts are printed in menus and palette rows; tooltips duplicate visible labels, never replace them.
- **Screen readers:** gpui-component's AccessKit integration. Roles: tree/treeitem for the explorer (with expanded and selected states), listbox/option for palette and quick open, dialog with labelled title, status for the status line message slot (polite), alert for banners (assertive for danger only), switch/slider/combobox in Settings. Icon buttons carry the same label as their palette command.
- **Sizes:** UI font size 12–16 px, editor 12–24 px, preview zoom 50–200 %, all independent. Hit targets ≥ 24 px.
- **Motion:** reduce motion setting (7.6).
- **Colour is never the only signal:** save state has a glyph and a word; diffs use strikethrough/underline as well as colour; invalid inputs have text messages; vim mode has a word.

---

## 10. Feature-to-home mapping

### 10.1 Frontend (FE-)

| FE IDs | Feature group | New home |
|---|---|---|
| 001 | Boot splash | Dropped (native start is instant; the empty state renders immediately) |
| 002–003 | Deep links, history, stale navigation | `frankmd open <path>` + single-instance IPC (OM-018); in-app Back/Forward (Ctrl+Alt+←/→); navigation generation kept in the state store (HP-003) |
| 004 | 401 → login | Dropped (no server, HP-023) |
| 005 | Warning banners | Pane banners (5.12) |
| 006–007, 221–224 | Sidebar layout, open-explorer button, mobile overlay | Sidebar (3.3), `panel-left` button in the pane header when hidden, narrow drawer (4.1) |
| 008–009, 278–279 | Header title, inline rename | Pane header (3.4) |
| 010–013, 052 | Toolbar, overflow menu, pins, visibility rules | 4 fixed header buttons + Document menu (3.4) + palette; pinning dropped |
| 014, 219, 493 | Preview zoom controls | Preview pane header (3.5), Ctrl+=/−/0 on the focused preview, Settings › Preview |
| 015 | Copy document | Document menu, palette |
| 016, 174, 481 | Split/single view | View modes (4.2), Settings › Preview |
| 017, 346 | Library workspace | Media library view (5.15) |
| 018, 475–496 | Settings modal and every setting | Settings sheet (5.16) |
| 019, 053, 223 | Mobile layout | Narrow width class (4.1) |
| 020, 045, 104, 485 | Editor width | Ctrl+Shift+=/−, Settings › Editor, status message |
| 021 | No splitters | Resizable splitters (3.1) |
| 022, 101, 379 | Table hint | Status line context chip + Ctrl+. (3.6) |
| 023, 080 | Vim badge | Status line mode block |
| 024, 280 | File not found | Status message + quick open prefilled (5.22); "Deleted on disk" banner |
| 025–026, 167 | Toasts | Status line message slot; notifications when unfocused |
| 027, 369–373 | appAlert/appConfirm/prompt | gpui-component Dialog-based alert/confirm/prompt with FIFO queue and focus restore |
| 028–056 | Shortcut system and global keys | Command/keybinding registry (HP-018), map §6 |
| 057–075, 086–087, 093 | Editor editing behaviour | Editor (gpui-component Input in code-editor mode + markdown layer); §6.2 |
| 064, 444 | Read-only | Read-only banner + status indicator |
| 074 | Hidden textarea | Dropped |
| 076–078, 108 | Highlighting, line numbers, theming of editor UI | Editor + syntax roles (7.1) |
| 079–081, 056, 166 | Vim | Settings › Editor, palette toggle, status line mode block and ex line (6.3) |
| 082–084, 489–490 | Typewriter, recenter | View modes (4.2), Settings › Writing |
| 085, 411–414 | Text format menu | Inline popover (5.5) |
| 088–091, 378 | Slash commands | Inline popover (5.5); Alt+I insert menu |
| 092 | Wikilink completion | Inline popover (5.5) |
| 094–098 | Find & replace | Inline find bar (5.4) |
| 099, 422–423 | Jump to line | Palette `:` mode (5.1) |
| 100, 487 | Stats | Status line + Document info popover (3.6), Settings › Writing |
| 102–103, 482–484 | Fonts | Settings › Editor/Appearance; fontconfig follow (7.2) |
| 105–106, 200–202 | Code highlighting | Shared highlighter for editor, preview, code dialog |
| 107, 390–398 | Code block dialog | Modal (5.9) |
| 109, 418–421, 047, 055 | Help | Help modal (5.18) |
| 110–127 | Autosave, save status | State store + status line save state (3.6) |
| 128–133 | Content-loss guard | Content-loss banner (5.12) |
| 134–141, 147–154 | Drafts, restore, remap | Drafts in `~/.local/state/frankmd/drafts/<DocId>` (HP-007); invisible unless something fails |
| 142–146, 148 | Recovery dialog | Recovery modal (5.11), plus "Keep both" |
| 155–160 | Offline/connection | "Vault unreachable" banner; drafts continue |
| 161–165, 437–438 | Undo note creation | Kept (5.13) |
| 168 | No multi-tab coordination | Single instance (OM-018) + "Changed on disk" banner |
| 169–170 | Splash polling, PWA | Dropped |
| 171–177 | Preview panel behaviour | Preview pane (3.5, 4.2); visibility now persisted |
| 178–199, 204–211 | Rendering, sanitizing, embeds, tasks, images, links | Native preview (5.21) |
| 203 | Hover copy button | Always-visible copy button |
| 207–209 | Image viewer | Image viewer overlay (5.20) |
| 212–218, 492 | Scroll sync | Preview header toggle, Ctrl+Shift+\\, Settings › Preview |
| 220 | Prose typography | Preview tokens (5.21, 7.2) |
| 225–281 | Explorer | Sidebar tree (3.3, 5.14), dialogs (5.13) |
| 254–258, 424–431 | New note, Hugo | New note dialog (5.13), Settings › Hugo |
| 282–288 | File finder | Quick open (5.2) |
| 289–295 | Content search | Search overlay (5.3) |
| 296–305 | Home | Empty state with docked quick open (5.2) |
| 306–336, 358–360 | Image picker, paste | Insert image dialog (5.6) |
| 337–345 | Video dialog | Insert video dialog (5.7) |
| 347–354 | Library cards, usage, preview, rename, delete | Media library (5.15) |
| 355–357, 478 | Storage provider | Insert dialogs' storage row, Settings › Media, provider fallback banner |
| 361–363 | Editor/tree drops | Editor drop → Insert image (5.6); tree drop → move or `.md` import (5.14) |
| 364–368, 374–377 | Dialog and dropdown infrastructure | gpui-component Dialog, Popover, PopupMenu with overlay rules (§5 intro) |
| 380–389 | Table editor | Modal (5.8) |
| 399–409 | Emoji picker | Inline popover (5.5) |
| 410 | Link | Ctrl+K + link completion (5.5) |
| 415–417 | Log viewer | Settings › Diagnostics (5.19) |
| 432–436 | File-operation dialogs | 5.13 |
| 439–451 | AI grammar | Banner while running + diff modal (5.10), Settings › Integrations |
| 452–466 | Themes, Omarchy | Theme roles (7.1), palette theme picker (5.17), "Follow Omarchy" default (OM-005) |
| 467–474, 479 | Custom theme editor | Modal (5.17) |
| 494 | About | Settings › About |
| 497–503, 480 | i18n, language picker, RTL | Palette language picker (live), Settings › Appearance, §8 |

### 10.2 Backend (BE-) with a UI surface

| BE IDs | Feature group | UI home |
|---|---|---|
| 001, 094 | Notes root, `fed [path]` | Vault picker (sidebar header, palette "Vault: Switch…"), first-run empty state, `frankmd open <path>` |
| 002–013, 017–019 | Config schema, `.fed`, precedence, writes | Settings sheet with scope toggle and provenance chips (5.16) |
| 014, 052 | Upload extension allow-lists | Settings › Media; dialog rejection messages |
| 015, 076, 080, 082 | Feature flags / status payloads | Drive "Set up" tags in insert dialogs and hidden/unavailable palette commands |
| 016 | `IMAGES_PATH` fallback | Settings › Media › Image folder (shows the resolved fallback as the placeholder) |
| 020–033, 099 | Paths, tree, note CRUD, rename/move, folders | Explorer, dialogs, banners; errors in the status line or inline in dialogs |
| 034 | Serve raw assets | Preview image loading (direct file reads, no server) |
| 035 | Initial note load | CLI/IPC open |
| 036 | Error-message localization | All error strings go through i18n |
| 037–039 | Search, finder data | Search overlay, quick open |
| 040–043 | Hugo | New note dialog (Hugo mode with slug field), Settings › Hugo |
| 044 | Backlinks | Sidebar "Linked from" (3.3) |
| 045–047 | Recent files | Quick open / empty state recents with relative times |
| 048–070 | Images, uploads, S3, library, usage index | Insert image/video dialogs, Media library, Settings › Media ("Test connection") |
| 064 | Media storage recovery | Provider fallback banner |
| 071–075 | Egress, web/Pinterest/Google search | Insert image Web/Google/Pinterest sources; network errors inline |
| 077–079, 081 | AI provider, grammar, Ollama URL, image generation | Settings › Integrations, Grammar check, Insert image › AI generate |
| 083 | YouTube | Insert video › YouTube |
| 084–086 | Omarchy theme, mapping, custom theme | Theme roles, theme picker, custom theme editor (now from `colors.toml`, OM-001) |
| 087–088 | i18n | Language picker; system language default |
| 089–090 | Logs, config inspector | Settings › Diagnostics |
| 091–093, 096–098 | Health, auth, CSP, browser launch, update/stop, splash | Not applicable to the native app (no UI) |
| 095 | `FRANKMD_ENV` | Shown as a provenance source in Settings |
| 100 | Sensitive-data hygiene | Secrets masked in Settings and Diagnostics; never in messages or notifications |

---

## 11. Quirk dispositions

| Quirk (frontend §13 / backend) | Decision |
|---|---|
| Preview tooltip says Ctrl+Shift+P | Fixed: shortcuts come from the registry; Ctrl+Shift+P is now the palette |
| F1 says "toggle" but only opens | F1 toggles |
| CodeMirror duplicates on Ctrl+F/G/Enter/M and Ctrl+Shift+\\ | Only the FrankMD action is bound |
| Ctrl+Shift+\\ dead on US layouts | Both keystroke forms bound |
| `editor_indent` saved but unused | Applied; options Tab/2/4/8 |
| No per-language highlighting in editor | Editor highlights fenced code like the preview |
| Typewriter force-opens the preview | Changed: typewriter leaves preview visibility alone; follows the caret only if visible |
| Content-loss guard blocks without a dialog | Kept as a banner (that was right) |
| Last write wins | "Changed on disk" banner via a file watcher |
| Offline banner lies about editing being disabled | Banner text now matches behaviour: editing continues as drafts |
| Nested-block scroll drift | Real source positions |
| External links navigate in-app | Open in the browser |
| Folder expansion not persisted | Persisted per vault |
| No tree arrow keys / Enter / scroll-into-view | Full tree keyboard model (5.14) |
| Deleting a non-empty folder fails despite the confirm text | Trash-based delete (recommended) |
| Drop onto a file row moves to root | Moves into that file's folder |
| Ctrl+P ignores recents | Recents first |
| Backlinks endpoint without UI | "Linked from" section |
| `sidebar_visible` never read | Read and written per vault |
| No link dialog | Ctrl+K template kept + link completion |
| Table editor loses alignment, doesn't escape pipes | Alignment control + pipe escaping |
| Emoji inserts shortcodes | Kept (file format compatibility) |
| AI grammar sends whole file, replaces whole doc | Whole file kept (parity) but per-hunk accept/reject; selection-only is a candidate for later |
| "Cursor in table – click to edit" not clickable | Clickable status chip + Ctrl+. |
| OS file drop on the editor unhandled | Opens Insert image with the files |
| No upload progress | Progress bars in dialog footers |
| Missing translation keys show raw keys | One catalog, coverage test (HP-022) |
| Locale change needs reload | Live |
| OS light/dark read once | Portal `SettingChanged` listener when not on Omarchy |
| Omarchy polling compares names | inotify + palette comparison (OM-003) |
| Hebrew has no RTL | Still none in v1 (GPUI limit); layout written direction-agnostic (§8.1) |
| Hard-coded English messages | All through i18n |
| Hover-only pins and copy buttons | Pins dropped; copy always visible |
| Explorer background click opens a menu | Left-click clears selection; menu on right-click |
| Empty Hugo slug → `//` path | Editable slug field, Create disabled when empty |

---

## 12. Open questions for engineering

1. **Vim on the gpui-component editor.** There's no vim layer (gpui-kit#2617). Options: build a modal layer over the editor's public editing API, or adopt Zed's `editor` + `vim` crates. The design assumes a vim mode with a status-line mode block and ex line either way.
2. **File watcher for "changed on disk".** Needs a notify-based watcher on the vault with self-write suppression (compare the revision we just wrote). Without it, drop the banner and keep FE-120's last-write-wins.
3. **Trash on delete.** Recommended (`trash` crate, freedesktop spec). It changes BE-031's semantics (non-empty folders become deletable), so it needs a product decision.
4. **Video first frames.** Needs `ffmpeg` at runtime (Omarchy ships it); fall back to a plain card without it.
5. **Settings as sheet vs separate window.** The design uses an in-window sheet because a second window would tile beside the main one in Hyprland unless the user adds a float rule, and we never write Hyprland config. Revisit if users ask for side-by-side settings.
6. **Reduced-motion source.** The portal doesn't expose a standard reduced-motion key everywhere; default to off and keep the setting.
7. **Explorer inline rename vs dialog.** gpui-component Tree rows need an embedded Input for inline rename; confirm this is cheap before committing to it.
