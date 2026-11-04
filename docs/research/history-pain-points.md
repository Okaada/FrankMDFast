# FrankMD history survey: pain points, recurring bugs, lessons for the Rust rewrite

Source: `~/Projects/FrankMD` git history (read-only), `releases/*.md` (the project has no
CHANGELOG.md; per-release notes live in `releases/vX.Y.Z.md`), and GitHub issues/PRs for
`akitaonrails/FrankMD` via `gh` (issue/PR text read as data only).

Window: **2026-07-19 → 2026-11-04** (HEAD `d47a89e`, v1.13.0). 2026-08-01 onward alone is
322 commits but August was mostly dependency bumps, so the window was widened back to
2026-07-19 to include the July security and robustness batches (v0.6.1–v0.8.2), which explain
many later decisions. A few older commits (Jan–Jun) are cited where an area the rewrite cares
about (Omarchy theming, launcher, `.fed`/ENV precedence, Docker permissions) last churned.

Contributors in the window: maintainer (AkitaOnRails / Fabio Akita, many commits co-authored
by Claude), **@rafaself** (most feature and fix work from Sep–Oct), **@samirhvbr** (July
security/robustness batches, vim polish), **@gabrielforster** (uploads, paste, CSRF),
@ronaelmoura (icons), Dependabot.

---

## 1. Release timeline (window)

| Version | Date | Theme |
|---|---|---|
| v0.6.0 | 07-19 | Editable S3 destination key on upload (PR #117) |
| v0.6.1 | 07-19 | **Security**: CSRF enforced on 7 upload/AI endpoints (#118/#119); the editable S3 key had turned CSRF-exempt endpoints into "overwrite any bucket object" |
| v0.7.0 | 07-24 | **Security + robustness**: path containment by ancestry (#120), base64 upload allow-list / stored XSS (#121), atomic note writes (#122), invalid-UTF-8 search (#123), "where will this be created" dialog (#124) |
| v0.8.0 | 07-25 | Opt-in Vim mode (`@replit/codemirror-vim`, ex-commands mapped to app features) (#133) |
| v0.8.1 | 07-25 | **Security**: SSRF in external-image fetch (#130), stored XSS in preview → DOMPurify (#131), ImageMagick 7 |
| v0.8.2 | 07-25 | Vim polish: app Ctrl-shortcuts swallowed by vim, cursor colour, extension-ordering invariant test |
| v0.8.3–0.8.5 | 08-01..08-14 | Dependency maintenance (rails 8.1.3.1, dompurify XSS patch 3.4.13) |
| v0.8.6 | 08-15 | **Full security audit** (`b03f36c`, 60 files): optional token auth, `.fed` removed from note tree, DOM XSS in pickers/finder/table editor, CSP/SRI, symlink containment, atomic 0600 `.fed`, config line-injection, bounded searches/decodes, random upload names + no-overwrite, secret-log removal, CI action pinning |
| v0.8.7 | 08-19 | Line-anchored bidirectional scroll sync (#147), viewport-bounded dropdowns (#141), `FRANKMD_BIND_ADDRESS` (#142), single-step New Note modal (#146) |
| v0.8.8–0.8.13 | 08-21..09-05 | Phosphor icons tab (#148, grown to 200), deps, a flaky-test fix |
| **v1.0.0** | 09-07 | First stable: custom S3 endpoints (MinIO etc., #158), image picker pagination (#159) + delete (#160) |
| v1.1.0 | 09-18 | KaTeX math (rendered *after* sanitization), ruby_llm 2.0 (ReDoS CVE) |
| v1.1.2 | 09-22 | **Data loss**: pending autosave recreated renamed/deleted notes (#170) |
| v1.2.0 | 09-24 | **Draft persistence & recovery** stack (#172–#175): per-file drafts, navigation safety, browser-lifecycle flush, carry drafts through rename/move/delete |
| v1.2.1 | 09-25 | Startup hang on nested-note URL (#180, init-order race); file-lifecycle consistency under concurrency (#182: stale tree responses, check-then-act races, advisory lock) |
| v1.3.0 | 09-27 | Per-note undo + undo-of-note-creation (#185/#186), slash commands (#190/#191), all `alert/confirm` replaced and localized (#187/#188), en fallback deep-merge, LRU cap on editor state cache, Ruby CI job finally added (#196) |
| v1.4.0 | 09-28 | Managed media **Library** workspace; hide `images/`/`videos/` from Explorer (#197/#198) |
| v1.5.0 / 1.5.1 | 09-29 | Settings workspace + header simplification (#199); GFM task checkboxes stripped by sanitizer (#202); translation coverage enforced (#200) |
| v1.6.0 / 1.6.1 | 10-01..10-03 | Split/Single view mode (#204), interactive task toggling (#203); typewriter regressions (#209/#210), theme debounce flake (#211) |
| v1.7.0 | 10-08 | Explorer selection + keyboard actions (#215), Settings moved again → modal (#217), configurable click-recenter (#221), typewriter preview alignment |
| v1.8.0 / 1.9.0 | 10-10 | Theme-aware fenced-code highlighting (Lezer, post-sanitize, DOM-only), ~20 languages, bounded/deferred highlighting; emoji hover perf; slash menu broken in typewriter mode (#230) |
| v1.10.0 | 10-20 | **Active media storage provider** unified Local/S3 (#234/#235) with invalid-S3 recovery; minimal workspace redesign (#240/#241) |
| v1.11.0–1.11.2 | 10-25 | Surround-selection, image overlay, visual theme editor, Home with recents shared via `.fed`, create-in-selected-folder; hotfix 500 on JSON Accept (`1330ac0`); moved note hidden in collapsed folder (`cfe3735`); system suite repaired (#254) |
| v1.12.0 | 10-27 | Home redesign, Settings dropdowns as top-layer popovers (#259), Hugo posts default to `blog/` (#257) |
| v1.13.0 | 10-31 | Explorer ergonomics: empty-area menu, multi-selection drag (#266), reveal items created in collapsed folders (#268) |

Pace: ~25 releases in 15 weeks. Late-July and mid-August were security-driven; late
September was about data safety; October was mostly UX contributions from @rafaself.

---

## 2. Hotspots

### 2a. Fix commits per area (window, non-merge, subject starts with fix/harden/security/polish/perf; hand-categorised)

| Area | Fix commits | Representative shas |
|---|---:|---|
| Media / S3 / uploads / Library / image picker | **25** | `f60ae27` `33cc5c0` `6251356` `a71b41a` `7f9dd9c` `4ec9e5f` `524e788` `122da9b` `cee77da` |
| Data safety: autosave, drafts, file lifecycle, undo, atomic writes | **20** | `ab818dc` `d0a17a1` `bfb7b13` `873d5a1` `f5176fc` `5c13c6e` `58e6611` `4f9a120` `2635d09` `9cee344` `45e0960` `9f64092` |
| Editor (CodeMirror): slash commands 6, typewriter 4, vim 2, misc 4 | **16** | `61abaa5` `0434d3a` `ea98191` `4d6fe9a` `c45f144` `b9c5655` `3e88b24` |
| Workspace chrome / Settings / Home / Hugo | **10** | `30a774b` `1330ac0` `67cc42e` `167c63a` `c311c28` |
| Preview: rendering, sanitizer, highlighting, scroll sync | **9** | `b3c1b32` `d92baef` `98a6871` `36100bb` `a2b3cd2` |
| Security (code + CVE bumps) | **8** | `b03f36c` `a480324` `53351bf` `8e4848e` `adc5259` `9eee9f5` `4814db3` `8949739` |
| Explorer: tree, selection, drag/move, create target | **7** (+ tree-refresh race in `58e6611`) | `cd7e2c1` `cfe3735` `deeb288` `19984ff` `35b9220` |
| i18n | **6** | `d0a6d94` `3a51877` `fa54d04` `3b4c9de` `3eada9a` |
| Test-only flake/timing fixes | **10** | `ec6e484` `51a7df5` `6708f42` `70d11be` `0aa2796` `5c666d8` `c3edc8e` `f098009` |
| Docker / deploy | 1 | `d4bab4b` |

Older areas (before the window) that the rewrite still needs to cover:

| Area | Notable fix commits (date) |
|---|---|
| Omarchy theming | `eaacc76` (01-31 colours), `5dfc24c` (02-28: TOML section leak, TOCTOU when Omarchy swaps theme files atomically, 3 s polling hitting config) |
| `.fed` vs ENV precedence | `46bf237`, `6ca4c36` (02-08: services read ENV at boot, bypassing `.fed`), `06e866e` (02-10: empty-string ENV short-circuited `.fed`) |
| Launcher / browser detection | `e924858` (06-07: macOS `.app` bundles not found, #86) |
| Docker permissions | `58b2a23` (01-31 UID/GID), `7b197e9` (02-04 first-run notes dir perms), #6/#7 |
| Init/outlet ordering | `0b2195c` (02-10), then again `770ca4c` (09-24, #180) |
| Typewriter mode | `abbfe35`, `e6236af`, `670b801` (Jan), then again 4× in Oct |
| Scroll sync | `cee0a90` (01-31, #2), then a full rework in `b3c1b32` (08-19, #147) |

### 2b. File churn (window, all non-merge commits, excluding lockfiles/vendor/locales)

| File | Touches | (of which fix commits) |
|---|---:|---:|
| `app/javascript/controllers/app_controller.js` | **60** | **29** |
| `test/system/workspace_test.rb` | 29 | 9 |
| `test/controllers/notes_controller_test.rb` | 29 | 10 |
| `app/assets/tailwind/components/editor.css` | 29 | 6 |
| `app/javascript/lib/code_block_highlighter.js` | 22 | — |
| `app/views/notes/_header.html.erb` | 20 | 4 |
| `app/services/notes_service.rb` | 13 | 9 |
| `app/services/images_service.rb` | 13 | 8 |
| `app/javascript/controllers/autosave_controller.js` | 11 | 9 |
| `app/javascript/controllers/codemirror_controller.js` | 11 | 6 |
| `app/models/config.rb` | 11 | — |

`app_controller.js` is a god object: tree state, expansion, navigation, rename/move/delete
handlers, autosave wiring, home/library/settings panes, and URL state all live there. Almost
every cross-cutting bug passed through it.

---

## 3. Recurring bug classes

### 3.1 Document identity = path string; path changes race with in-flight work
- `ab818dc` (#170) a debounced autosave fired after rename/delete and **recreated the old file**. Fix: client remaps/cancels pending saves, adds a per-file version counter to discard stale responses, and the server `update` refuses to create files (`existing_only: true` → 404).
- `873d5a1`, `5c13c6e`, `b21947e` (#175) drafts and legacy backups had to be remapped by hand on rename/move/delete, including recovery when a remap failed.
- `cfe3735`, `0668bef` (#268) a moved or created item vanished because the expansion state was updated *after* the tree re-rendered.
- `cd7e2c1` multi-drag re-moved children already covered by a selected ancestor folder (stale child paths) and kept going after a failure.
- `9cee344`, `166420d` undo history leaked between notes, and later had to survive rename/move.

**Root cause**: every subsystem (autosave, drafts, backups, undo cache, tree expansion, recents,
URL) keyed state by the relative path string, and each needed its own `remapPath` with
`startsWith(old + "/")` logic.
**Lesson**: give every open document a stable in-memory `DocId`. Keep the path as one mutable
attribute in one registry. Rename/move becomes a single transactional event on that registry,
and subscribers update from it. Saves name a `DocId` plus a revision, never a path.

### 3.2 State in two (or five) places, synchronised by events
- `f5176fc`, `d0a17a1`, `bfb7b13` (#172–#174): unsaved content lived in CodeMirror, the autosave controller, the localStorage offline backup, and the new per-file draft store. Each transition (navigate, tab close, pagehide) needed its own flush.
- `a71b41a`: the Settings UI and the media-storage component disagreed about the active provider after a reload (`apply(data)` was skipped while "pending").
- `58e6611` (#182): a tree refresh started before a mutation could overwrite the newer Turbo Stream tree. Fixed with two counters (`_treeRevision`, `_treeRefreshGeneration`).
- v1.12.0 known follow-up: the Hugo `blog/` default lives only in the New Note dialog. Server `HugoService` still defaults to the root, so UI and API disagree.
- `5360c4f`/#240: pinned toolbar actions are session-only while other prefs persist in `.fed`.

**Lesson**: one authoritative app-state store in Rust (documents, tree, settings, storage
provider) with derived views. The UI renders from snapshots and sends commands. No component
holds a private copy that has to be "refreshed". Async results carry the revision they were
computed against and are dropped if stale.

### 3.3 Check-then-act on the filesystem; concurrent mutations
- `58e6611`, `4f9a120` (#182): create/rename checked `exist?` and then wrote, so concurrent requests silently overwrote each other. Fix: a global advisory `flock`, `O_EXCL` temp files + `link()` for no-replace create, `File.rename` with EEXIST mapping.
- `2635d09`: the lock file itself could fail to open (read-only mount, ELOOP, ENOSPC) and crashed every read. It had to degrade to a local lock.
- `9f64092` (#122): note writes truncated in place, so ENOSPC corrupted notes. Fixed with temp + rename, and file mode is preserved (`33e38f0`).
- `7e4bd29` (#123): one invalid-UTF-8 file aborted the whole search, and a file vanishing mid-search (TOCTOU) threw.
- `5dfc24c`: Omarchy's atomic theme swap raced the `exist?`/`read` pair.

**Lesson**: a single `fs` service module owns all mutations: atomic write (tempfile in the
same dir + fsync + rename, preserving mode), no-replace create (`O_EXCL`/`renameat2(RENAME_NOREPLACE)`
or link), rename that never clobbers, and errors such as `AlreadyExists`/`NotFound`/`Conflict`.
Operations run through one serialized command queue (actor). Readers tolerate files that
disappear and non-UTF-8 bytes (lossy decode per file, skip and report).

### 3.4 Initialization-order races
- `0b2195c` (Feb) and `770ca4c` (#180, Sep): the Stimulus outlet callback ran before `connect()`, so `this.expandedFolders` was undefined and opening a nested note left the app hung on the splash screen.
- `4d6fe9a`, `c45f144`, `1396081` typewriter mode computed layout while the editor was hidden (zero height), needed ResizeObserver re-centering, and changed sidebar visibility as a side effect.
- `ea98191` (#230): typewriter mode dispatched a redundant selection transaction that killed the autocomplete popup.

**Lesson**: explicit, typed startup phases (config loaded, then index ready, then UI mounted,
then document opened). No lazily-initialised fields read from callbacks. Layout features
(typewriter, scroll sync) recompute on visibility/resize events and must not side-effect
unrelated panes.

### 3.5 Config: multiple sources, multiple readers, stringly-typed values
- `46bf237`, `6ca4c36`: YouTube, Images and AWS code read ENV in boot initializers, bypassing `.fed > ENV > default`.
- `06e866e`: `FRANKMD_LOCALE=""` from docker-compose was truthy and short-circuited `.fed`.
- #214: a user's S3 credentials were swapped. The thread was debugging `.fed` silently overriding `.env`, which confused users.
- `b03f36c`: `.fed` writes were not atomic or 0600, and UI values could inject newlines/quotes into the file (line injection).
- `33cc5c0`, `6251356`: a selected S3 provider whose settings disappeared, or whose credentials/region the SDK rejected, needed recovery logic plus a user-visible warning.
- `3a51877`: YAML 1.1 parsed locale keys `yes`/`no` as booleans and broke JS lookups.

**Lesson**: one typed `Config` struct (serde), loaded once with explicit layered precedence.
Treat empty strings as unset, record which layer each value came from, and expose that to the
UI ("set by ENV, overridden by .fed"). Validate on load and report problems; don't silently
fall back. Write atomically with mode 0600. Never hand-edit lines in place; serialize the
whole document with a format-preserving TOML writer (e.g. `toml_edit`).

### 3.6 Internal files living inside the user's notes tree
- `.fed` (with API keys) was listed in the Explorer and servable and editable through note routes until `b03f36c`. Autosave had a special case `filePath === ".fed"`.
- `4f9a120`: the new `.frankmd-filesystem.lock` had to be explicitly 404'd from the asset route.
- `524e788`, #197: managed `images/`/`videos/` folders cluttered the Explorer. Hiding them had to special-case folders that also contain Markdown.
- `9706676` (#245): recents are stored in `.fed` so they sync across devices.

**Lesson**: decide up front which app-owned data lives where. Put secrets and machine-local
state in the XDG config/state dirs, not in the vault. If something must sync with the vault
(recents, per-folder AI overrides), give it one reserved, clearly named location (e.g.
`.frankmd/`) that the tree walker, search, and asset server exclude by construction.

### 3.7 Sanitizer vs features: security tightening broke features, and features kept poking holes
- `53351bf` → `f0d4ef3` banned form controls, which later stripped GFM task checkboxes (`d92baef`, #202). Interactive tasks (#203) then needed a carefully shaped exception.
- KaTeX (v1.1.0), syntax highlighting (v1.8.0) and the image overlay (`cfb10c7`) all had to be designed to run **after** sanitization using DOM APIs only.
- `53351bf`: DOMPurify had to run *after* `data-source-line` injection or scroll-sync mapping broke.
- `da63a7d`: icon alignment had to use a class because the sanitizer strips `style`.

**Lesson**: make the preview pipeline an explicit typed AST → HTML renderer in Rust
(pulldown-cmark/comrak events) that emits only allowlisted elements and attributes. Source-line
anchors, task checkbox ids, math placeholders and code-token spans are first-class outputs, not
regex-patched HTML. Run raw HTML in notes through `ammonia` with a policy that lives in one
place and is tested. Interactive affordances get attached by the host after rendering.

### 3.8 Unbounded work on user content (perf / DoS)
- `98a6871`, `36100bb`: syntax highlighting of large fences froze renders. Fixed with block-count/size caps, deferral, and stale-result skipping.
- `3e88b24`: large backtick selections. `948d77b`: emoji hover rebuilt the whole grid.
- `45e0960` (#192): per-note `EditorState` cache grew without bound. Fixed with an LRU of 20.
- `b03f36c`: regex search, large-file scans and base64 decode had to be bounded. `f0d4ef3`: the external fetch needed a 25 MB streamed cap.

**Lesson**: every operation over user content has a budget (bytes, items, time) and is
cancellable. Run render, highlight and search off the UI thread and drop superseded results.
Bound every cache.

### 3.9 Two code paths for "the same" operation drift apart
- `95704e2`: the image upload path lacked the extension allow-list the video path had. `adc5259`: the base64/AI upload path skipped it too. Both were stored XSS.
- `f60ae27`: four upload entry points (drop, paste, picker, AI) each decided local vs S3 from a client param. Fixed by a single server-side `UploadStorage.provider`.
- `8e4848e`: three separate path guards, all using string-prefix containment.
- v1.10.0: the inline title rename and the Explorer rename were unified into one `renamePath` implementation.

**Lesson**: one function per capability (`store_media`, `contain_path`, `rename_entry`) and
every UI entry point calls it. Enforce it with module visibility, e.g. only `fs::` can open
files for writing.

### 3.10 Shortcut and keymap conflicts
- `b9c5655`: vim swallowed Ctrl+F/N/P/E/B/I/V. `cc83922`: the vim compartment must be **first** in the extensions array or normal mode breaks. A test locks that ordering.
- `a60ec4f` (Feb): the markdown keymap needed highest priority for Ctrl+I.
- `ea98191`: typewriter vs autocomplete.

**Lesson**: a central command/keybinding registry with explicit precedence layers
(modal editor, then editor commands, then app global) and conflict detection tests.

### 3.11 Deployment and environment assumptions
- #63 / `bd31c7a`: Rails CSRF origin check failed behind an SSL-terminating proxy (Cloudflare Tunnel), so every save got a 422.
- #6/#7, `58b2a23`, `7b197e9`: container UID/GID vs mounted notes permissions.
- #142 / `d4bab4b`: the default bind to `0.0.0.0` exposed unauthenticated notes on the LAN or VPN.
- #86 / `e924858`: the launcher found browsers only via Linux PATH names, missing macOS `.app` bundles.
- `2432adc`: `ubuntu-latest` moved to 26.04, which has no Ruby 3.4.8 build. CI was pinned.

**Lesson for a native app**: no browser launcher, no HTTP server, and no CSRF by default. If
a LAN/remote mode is ever added, it is opt-in, binds to loopback by default, and requires auth.
Notes-dir permission errors surface as clear UI errors, not silent failures.

---

## 4. Reverted or reworked features

There were almost no literal `git revert`s; the project reworked things forward instead.

| Feature | Rework history | Why |
|---|---|---|
| **Settings UI** | header pickers + editor-customization dialog → Settings **workspace** pane (v1.5.0, `f982db5`) → Settings **modal** (v1.7.0, `6bc171e`, #217) → dropdowns as top-layer popovers (v1.12.0, `30a774b`, #259) | Pane coupled settings to the editor workspace (hidden-not-disconnected hacks to preserve autosave/undo); modal's scroll container clipped dropdowns |
| **Scroll sync** | ratio-based (Jan, `cee0a90`) → line-anchored bidirectional via `data-source-line` (`b3c1b32`, #147) | `offsetTop` coordinate mismatch, two competing scroll locks causing drift and swallowed scrolls, re-anchoring on image load/zoom |
| **Typewriter mode** | mirror-div centering (Jan `e6236af`) → CodeMirror extension; 4 regressions in Oct (`4d6fe9a`, `c45f144`, `1396081`, `ea98191`) | Layout computed while hidden; side effects on sidebar; extra transactions broke autocomplete |
| **Media storage** | per-upload "S3?" checkbox → editable S3 key (v0.6.0) → custom endpoints (v1.0.0) → single persisted **active provider** (#234/#235, v1.10.0) with recovery | Mixed behaviour; uploads silently falling back to local; client could choose destination |
| **Image picker tabs** | Local/Folder/… → "Drop → Upload", "Local → Library", plus a separate Library workspace (v1.4.0) | Separation of "upload new" vs "browse managed" |
| **Workspace chrome** | full toolbar + header → minimal workspace, inline title rename, pinnable actions, Explorer fully closable (#240/#241, ~30 commits in 2 days) | Duplicate entry points, crowding |
| **Home** | none → recents list (v1.11.0) → redesigned dashboard + sidebar button (v1.12.0) | Iterated within 2 days |
| **Connection monitor** (server-down detection) | exponential backoff reverted to fixed 5 s polling (`15692a7`, Feb) | Backoff made reconnection detection too slow. Omarchy theme is separately polled every 3 s (`5dfc24c`) |
| **Vim mode** | contributor PR #132 closed as duplicate of maintainer #133 (same library); follow-up PRs #134–#136 merged | Parallel work collision |
| **Docker bind** | contributor PRs #143/#144 closed, superseded by maintainer `d4bab4b` | Same |
| **Explorer hiding of `images/`/`videos/`** | hide → "unless it contains Markdown" (`524e788`) | Users had note folders with those names |

Lesson: Settings and Home churned because their placement wasn't decided up front. Pick the
information architecture (where settings, library and home live) before building the
contents.

---

## 5. Security fixes and day-one guarantees

| Fix | Commit(s) | Class |
|---|---|---|
| CSRF on 7 upload/AI endpoints; editable S3 key turned them into arbitrary-object overwrite | `9eee9f5`, `e258cc6` (#118/#119) | CSRF |
| Path containment by string prefix (`/data/notes` accepted `/data/notes-backup`), `..` stripped by gsub (mangled `notes..v2.md`), symlinks escaping | `8e4848e` (#120), `33e38f0`, `b03f36c` (symlinked dirs; walkers ignore symlinks) | Path traversal |
| Base64/AI upload skipped extension allow-list → `.svg`/`.html` served inline | `adc5259` (#121); earlier same bug on multipart `95704e2` | Stored XSS |
| SSRF in "fetch external image → S3": metadata IP, localhost, LAN; DNS rebinding; no size cap; redirects | `a480324` (#130), `f0d4ef3` (IP pinning, all-addresses-public, 25 MB cap, per-hop redirect re-validation) | SSRF / egress |
| Preview rendered raw HTML via `innerHTML` (`<script>`, `onerror`, `javascript:`) | `53351bf` (#131), `f0d4ef3` (form controls), `b03f36c` (active CSS stripped, CSP, SRI) | XSS |
| DOM XSS via attribute interpolation in image pickers, file finder, table editor; unvalidated image URL schemes | `b03f36c` | XSS |
| `.fed` (API keys) visible/servable/editable via note routes, including encoded and canonical aliases | `b03f36c` | Credential disclosure |
| API-key prefix logged; raw upstream AI/image errors returned to client | `b03f36c`, `95704e2` | Credential/info leak |
| `.fed` written non-atomically, world-readable, line-injectable | `b03f36c` | Config integrity |
| Upload name collisions / overwrite; client-supplied temp names | `b03f36c`, `95704e2` | Integrity |
| Unbounded regex search, base64 decode, external fetch time | `b03f36c` | DoS |
| No auth at all; added optional token auth + throttling | `b03f36c` (issue #29 had been declined in Feb) | AuthN |
| Library media paths: segment gate, per-type allowlist, NUL bytes, per-component `lstat` walk | `271c3bc`, `05d0b9a` (v1.4.0) | Path safety |
| Theme editor accepts strict hex only (no CSS injection) | `09e5447` | CSS injection |
| Dependency CVEs: dompurify ×4 (`952fa5a`, `6eebdeb`, `8949739`), ruby_llm ReDoS (`4814db3`), net-ssh (`8b5cb61`), crass, loofah | — | Supply chain |

**The rewrite must guarantee from day one:**
1. **Path safety**: a single `VaultPath` newtype built only by a `contain()` that resolves
   components (rejects absolute paths, `..` beyond the root, NUL, and symlinks resolving outside
   the root, checked per component with `lstat`/`openat`-style walking). All file APIs take
   `VaultPath`, never `&str`/`PathBuf` from the UI.
2. **Preview safety**: the webview (if any) runs with a strict CSP (no inline script, no
   remote script), no Node/IPC exposure to preview content, and sanitized HTML from the
   AST renderer. Raw HTML in notes is allowlisted, `javascript:`/`data:` URL schemes are
   filtered, and iframes are limited to an embed allowlist.
3. **Egress policy**: every outbound fetch (external images, AI, web image search) goes
   through one client: http/https only, resolve-then-pin, reject non-public addresses (including
   IPv4-mapped IPv6 and CGNAT), re-validate each redirect hop, enforce size and time caps.
4. **Secrets**: API keys never live in the vault, never get logged (redacting `Debug` impls
   via `secrecy`), never go to the UI. Use the OS keyring or 0600 config. Upstream error
   bodies get mapped to generic messages.
5. **Upload/media allowlist**: one `store_media()` with extension + magic-byte check,
   random names, no-overwrite, size caps, and the active provider resolved server-side.
6. **Atomic, no-clobber writes** for notes and config (see 3.3).
7. **No listening sockets by default**, so the CSRF/bind/auth problem doesn't exist until a remote
   mode is explicitly designed.
8. A `cargo audit`/`cargo deny` gate in CI; vendored JS (if a webview editor stays) pinned
   by hash.

---

## 6. Community requests

**Declined (with stated policy):**
- **Named AI providers/gateways**: OrcaRouter (#162), Requesty (PR #184). Policy: stay vendor-neutral, no paid placement. The maintainer explicitly **welcomes a generic OpenAI-compatible endpoint** (`base_url` + key + model, honouring `.fed > ENV`). → *Design the AI provider layer around a generic OpenAI-compatible provider from day one.*
- **Native multi-user login/OIDC** (#29, Feb). Reason: single-user, no DB. A token auth was added later anyway (`b03f36c`).

**Superseded / duplicate PRs:** #132 (vim), #143/#144 (Docker bind), #145.

**Open now:** #269 / PR #270: dismiss the Explorer background menu on outside right-click (a follow-up to v1.13.0).

**Popular or heavily discussed (comment count, follow-ups):**
- #137 icon libraries (5 comments, shipped and extended twice; a second icon library is deferred behind a reserved prefix scheme).
- #63 saving over Cloudflare Tunnel, #214 custom S3 endpoint (5 comments each), both config/deployment confusion.
- #234 unified storage provider (proposal → design discussion → 10+ commits).
- #240 minimal workspace (adopted wholesale). The known follow-up is **persisting pinned toolbar actions** (currently session-only).
- #197 Library: the reporter's preferred end state is moving managed assets to a hidden `.frankmd/` folder rather than hiding `images/`/`videos/`.
- #172–#175 draft safety stack, #170/#182 data-loss bugs: users notice and report data loss in detail.
- #69 custom Hugo frontmatter template (`.hugo_template.md`), #35 frontmatter format YAML/JSON/TOML, #42 custom note templates.
- #6 access from other devices, #90 arm64 images, #94 show version in UI.

**Features users clearly value (design for them):** vim mode with ex-commands mapped to app
actions; slash commands; per-note undo that survives switching and rename; draft recovery;
math (KaTeX); fenced-code highlighting across ~20 languages; GFM interactive tasks; scroll
sync; typewriter mode; S3-compatible storage (MinIO/SeaweedFS); media Library with "used by"
lookup; Hugo blog workflow (`blog/YYYY/MM/DD/slug/index.md`); Omarchy theme following plus
custom theme editor; 7 locales including RTL Hebrew; multi-select Explorer with drag, keyboard
actions and VSCode-like empty-area behaviour; recents shared across devices.

---

## 7. Test-suite lessons

**Regression tests worth porting first** (they encode real bugs):
1. Autosave after rename/delete must not recreate the old path; a stale save response must be discarded (`ab818dc`).
2. Create never overwrites; rename into an existing name fails with `AlreadyExists`; concurrent create/rename collisions (`58e6611`, `4f9a120`).
3. Atomic write: an interrupted write leaves the original intact and file mode is preserved (`9f64092`, `33e38f0`).
4. Path containment matrix: sibling prefix, `..`, absolute, encoded, symlinked file, symlinked directory, NUL, `notes..v2.md` legit name (`8e4848e`, `b03f36c`, `271c3bc`).
5. Sanitizer matrix: script/onerror/`javascript:`, form controls, iframe host confusion (userinfo/subdomain/protocol-relative), inert task checkbox allowed and hostile checkbox variants stripped, code-block content stays text (`53351bf`, `f0d4ef3`, `d92baef`, v1.8.0 XSS-smuggling pin).
6. Egress: metadata IP, loopback, RFC1918, IPv6-mapped, DNS rebinding, redirect to metadata, size cap (`a480324`, `f0d4ef3`).
7. Upload allowlist on *every* entry point: svg/html via multipart, base64 and paste; MIME aliases such as BMP `image/x-ms-bmp` (`adc5259`, `95704e2`, `4ec9e5f`).
8. Drafts survive navigation, app close, rename/move/delete of the file *and of a parent folder* (`d0a17a1`, `bfb7b13`, `873d5a1`, `5c13c6e`).
9. Search survives invalid UTF-8 and files vanishing mid-scan (`7e4bd29`).
10. Multi-drag: ancestor-covered children excluded, stop at first failure, destination revealed when collapsed (`cd7e2c1`, `cfe3735`).
11. Startup with a deep-linked nested note doesn't hang (`770ca4c`).
12. Config precedence: an empty-string ENV counts as unset, file beats ENV, every reader uses the same resolver (`06e866e`, `6ca4c36`).
13. Translation coverage: every locale has every key of the canonical locale, checked against the source files and not through the fallback (`3b4c9de`, `316071f`).
14. Keymap invariants: vim first; app shortcuts not swallowed (`cc83922`, `b9c5655`).

**Flaky-test patterns seen (avoid by construction):**
- **Real-time debounce waits**: theme controller debounce flaked under CI load (#211; `0aa2796`, `5c666d8`, `c3edc8e`). → Inject a clock and timers; use `tokio::time::pause()` or a fake clock in Rust.
- **Shared filesystem across parallel workers**: `ec6e484` (a test renamed the shared `log/test.log`, flaking every 2–3 runs for weeks), `6708f42` (counting files in a shared images dir). → Use a fresh tempdir per test and assert on specific artifacts, not directory counts.
- **Global mutable state leaking between tests**: `I18n.locale` (`51a7df5`); ambient local config making a test pass locally and fail on clean CI (`cee77da`). → No process-global locale or config; pass context explicitly; run CI with a clean HOME/XDG.
- **Environment-specific mocking**: monkey-patching `localStorage.setItem` on the instance (`70d11be`).
- **System tests not in hosted CI**: the draft-recovery system suite was red for a whole release (#254, `f098009`) because hosted CI ran no system tests, and the Ruby suite wasn't in hosted CI until `2e384be` (#196: a Ruby-red PR merged green). → Every test tier the project relies on must run in CI, and E2E tests must run headless in CI from day one.
- **Partial test harnesses**: `03c304f`, where new code broke tests that instantiate controllers partially. → Prefer testing through the real state store and command API instead of half-constructed UI objects.
- **Hidden test-script syntax traps**: Selenium's `return (SCRIPT)` wrapping (`f098009`). The guard test that rejects the pattern is a good idea to copy as "convention tests" (lint-like tests for test code).
- Large UI redesigns broke many system tests at once (`6f0a8b3`, `299d452`, `b67aca6`). → Use stable test selectors and IDs decoupled from layout.

---

## 8. Do / Don't for the rewrite

| ID | Rule |
|---|---|
| **HP-001** | **Do** give every open document a stable `DocId`; path is a mutable attribute in one registry. Saves, drafts, undo, recents and expansion key off `DocId`. |
| **HP-002** | **Do** keep one authoritative app state (actor/store); UI renders snapshots and sends commands. **Don't** let components hold private copies that need "refresh". |
| **HP-003** | **Do** tag every async result (save, tree scan, render, highlight, search) with the revision it was computed from and drop stale results. |
| **HP-004** | **Do** route all filesystem mutations through one `fs` module: atomic write (same-dir temp + fsync + rename, preserve mode), no-replace create, no-clobber rename, typed errors. |
| **HP-005** | **Don't** check-then-act on paths. Rely on `O_EXCL`/`RENAME_NOREPLACE`/link semantics and map EEXIST to `AlreadyExists`. |
| **HP-006** | **Do** make "save" never create a file or its parent dirs. Creation is a separate explicit command. |
| **HP-007** | **Do** persist unsaved drafts per `DocId` in app state dir with crash-safe writes, flush on app close/suspend, and carry them through rename/move/delete (including parent folder ops). |
| **HP-008** | **Do** build all paths through a `VaultPath::contain()` newtype (component-wise, symlink-aware, NUL-rejecting). **Don't** use string-prefix containment anywhere. |
| **HP-009** | **Do** render Markdown via an AST renderer that emits allowlisted HTML with source-line anchors and task ids natively. Sanitize raw HTML with one tested policy. Add math, highlighting and overlays after sanitizing, with no string concatenation. |
| **HP-010** | **Do** put every outbound HTTP request behind one egress client (scheme allowlist, resolve+pin, public-only, per-redirect revalidation, size/time caps). |
| **HP-011** | **Don't** store secrets in the vault or logs. Use the keyring or 0600 config, redacting `Debug`, and generic error messages to the UI. |
| **HP-012** | **Do** keep app-owned data out of the user's tree, or in a single reserved `.frankmd/` dir that walker, search and asset serving exclude by construction (no `.fed`/lock/`images/` special cases). |
| **HP-013** | **Do** use one typed `Config` with layered precedence (file > env > default, empty = unset), provenance per key, and validation with visible warnings. Write whole-document, atomic, 0600, format-preserving. |
| **HP-014** | **Don't** let the UI and backend each decide a policy (storage provider, Hugo default folder, upload destination). The backend resolves it and the UI displays it. |
| **HP-015** | **Do** have one function per capability (`store_media`, `rename_entry`, `contain_path`) used by every entry point (drop, paste, picker, AI, inline title, Explorer). |
| **HP-016** | **Do** define explicit startup phases. **Don't** read lazily-initialised fields from callbacks that may fire first. |
| **HP-017** | **Do** budget and cancel all work over user content (highlight caps, search limits, decode limits, bounded caches, LRU on per-doc editor state). Keep it off the UI thread. |
| **HP-018** | **Do** use a central keybinding/command registry with explicit precedence (vim, then editor, then app) and conflict tests. Vim ex-commands map to the same commands. |
| **HP-019** | **Do** make layout modes (typewriter, split/single, scroll sync) pure functions of state that recompute on resize/visibility and touch only their own pane. |
| **HP-020** | **Do** design the AI layer around a generic OpenAI-compatible provider (base_url + key + model), plus a few built-ins. **Don't** add named gateways ad hoc. |
| **HP-021** | **Do** decide where settings, library and home live before building them (Settings moved three times; Home redesigned in two days). |
| **HP-022** | **Do** use typed i18n (Fluent or similar), with a CI test that every locale covers every key of `en`, real fallback, and RTL checks. **Don't** use YAML 1.1 for message catalogs (`yes`/`no` → booleans). |
| **HP-023** | **Don't** open a network listener by default. If a remote/LAN mode is added later, bind to loopback by default, require auth, and handle proxies properly. |
| **HP-024** | **Do** surface Omarchy/theme file reads as tolerant of atomic swaps (read can fail between exist and open; retry and keep the last good theme). Prefer inotify over the old 3 s HTTP polling. |
| **HP-025** | **Do** run every test tier (unit, integration, E2E/UI) in hosted CI from the first commit, pin runner images, and run with a clean HOME/XDG so ambient config can't mask failures. |
| **HP-026** | **Don't** write timing-based tests. Inject clocks, use per-test tempdirs, and avoid process-global locale or config. Assert on specific artifacts, not directory counts. |
| **HP-027** | **Do** port the regression matrix in section 7 (data safety, path, sanitizer, egress, upload allowlist) before porting features. |
| **HP-028** | **Do** make search and indexing tolerant of non-UTF-8 and vanishing files (lossy decode per file, skip and report, never abort the whole query). |
| **HP-029** | **Do** give multi-item operations (bulk move/delete) ancestor de-duplication and stop-on-first-failure with a clear partial-result report. Reveal destinations before re-render. |
| **HP-030** | **Do** keep one implementation for each user-facing flow that has multiple entry points (rename from title vs Explorer; create from toolbar vs context menu vs empty-area menu). Explicit destination always beats implicit selection. |
