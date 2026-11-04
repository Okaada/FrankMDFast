# Wiring request: media + AI dialogs (round 4, MEDIA agent)

Everything lives in `crates/frankmd-app/src/dialogs/media/**` and `src/dialogs/ai/**`.
The shell (src/ui/**) only has to create the entities, render them in the overlay
layer / main area, call `open`, and handle the events below. Nothing here needs a
change to an existing signature.

## 1. Startup (once per process)

```rust
frankmd_app::dialogs::media::init(cx); // binds the dialogs' keys (also the grammar dialog's)
```

Call it next to `ui::actions::install_keymap(cx)` in `lib.rs::run` (and in the
shell test harness). Key contexts used: `MediaDialog`, `MediaRail`, `MediaActivate`,
`MediaLibrary`, `MediaVideoGrid`, `MediaVideoQuery`, `GrammarDialog`, `GrammarDiff`.
Bindings are context-scoped (no global chords), so the registry duplicate test is unaffected.

## 2. Per window

```rust
use frankmd_app::dialogs::media::{MediaContext, MediaHub};
let ctx = MediaContext { vault_root, state_dir, env: config.env.clone() }; // from ShellConfig
let media = MediaHub::new(&ctx, window, cx)?;          // picker, video, library, storage entities
let grammar = cx.new(|cx| frankmd_app::dialogs::ai::GrammarDialog::new(media.services.clone(), window, cx));
```

Subscribe to `media.picker`, `media.video`, `media.library` (`MediaEvent`) and `grammar` (`GrammarEvent`).

## 3. Commands (all already in the CommandRegistry — no new ids)

| Command id | Call | Overlay |
|---|---|---|
| `insert.image` (Ctrl+Shift+I, `/image`, header button) | `media.picker.update(cx, \|p, cx\| p.open(note_path, window, cx))` | `OverlayKind::Dialog("insert-image")`, render `media.picker.clone()` as the overlay content |
| `insert.video` (`/video`) | `media.video.update(cx, \|v, cx\| v.open(window, cx))` | `OverlayKind::Dialog("insert-video")`, render `media.video.clone()` |
| `view.media_library` (Ctrl+Shift+M) | when `layout.media_library` becomes true: `media.library.update(cx, \|l, cx\| l.open(note_path, window, cx))`; keep it updated with `set_note` | render `media.library.clone()` instead of `render_media_library`'s placeholder (it carries the `media-library` id itself) |
| `ai.grammar_check` (Ctrl+Shift+G) | `grammar.update(cx, \|g, cx\| g.start(note_path, editor_text, window, cx))` | `OverlayKind::Dialog("grammar")` on `GrammarEvent::Show`, render `grammar.clone()` |

`note_path` = the open note's vault-relative path (`Option<String>`), used for note-relative links.
Remember the pending slash range (`/image`, `/video`) when the dialog opens from the slash menu.

## 4. `MediaEvent` (picker, video, library)

| Event | Shell action |
|---|---|
| `Insert(insert)` | `dialogs::media::apply_insert(&editor_pane, &insert, pending_slash_range.take(), window, cx)` (one undoable edit, FrankMD placement rules), then close the overlay / library and focus the editor. Mark dirty through the normal editor-change path. |
| `Closed` | `close_overlay` (focus restore); clear the pending slash range. |
| `Status { text, error }` | `show_message(&text, error)` (upload progress: "Uploading <name>…", failures). |
| `StorageFallback` | `raise_banner(BannerKind::StorageFallback)` (once per session; the model guarantees it). |
| `OpenSettings` | open Settings › Integrations. |
| `OpenNote(path)` | open + reveal the note (library "Used in"). |
| `CloseLibrary` | `layout.close_media_library()` and restore the editor (also sent after Insert/OpenNote from the library). |

Esc inside a dialog is handled by the dialog (`MediaDialog` context) and ends in `Closed`;
the shell's Esc chain should skip the overlay pop when the dialog already closed itself.
For the library, Esc is handled by the pane (prompt → detail sheet → `CloseLibrary`).

## 5. Paste and drop of images/files into the editor (FE-358…361, ui-spec §5.6)

- **Paste**: before the editor's normal paste, check the clipboard:
  ```rust
  if let Some(item) = cx.read_from_clipboard()
      && let Some(file) = frankmd_app::dialogs::media::files::clipboard_image(&item, files::now_ms())
  { media.picker.update(cx, |p, cx| p.open_with_files(note_path, vec![file], window, cx)); /* open overlay */ return; }
  ```
  (Image paste suppresses the HTML/text part; text paste is unchanged.)
- **Drop**: on the editor pane container add
  `.on_drop(|paths: &ExternalPaths, window, cx| media.picker.update(cx, |p, cx| p.open_with_paths(note_path, paths.paths().to_vec(), window, cx)))`
  and open the `insert-image` overlay. The files arrive in Upload, first one pre-selected; Insert uploads.
  (Optional: `.drag_over::<ExternalPaths>(|s, _, _, _| s.border_color(accent))`.)

## 6. Settings sheet: "Storage for new media"

Bind the row to `media.storage` (`Entity<StorageModel>`): value `storage.read(cx).state().provider()`,
disabled while `state().controls_disabled()`, S3 disabled with `state().s3_disabled_reason()`;
on change `media.storage.update(cx, |s, cx| s.request(provider, cx))`. Subscribe to `StorageEvent::Fallback`
→ `BannerKind::StorageFallback`. When the watcher reports `.fed` changed: `media.storage.update(cx, |s, cx| s.refresh(cx))`
and `grammar.update(cx, |g, cx| g.reload_config(cx))`.

## 7. `GrammarEvent`

| Event | Shell action |
|---|---|
| `FlushAutosave` | save the pending autosave now (the request already carries the editor text, so this is for parity, not a race) |
| `SetReadOnly(on)` | `editor.pane.update(\|p, cx\| p.set_read_only(on, cx))` |
| `Banner(Some(text))` / `Banner(None)` | show/remove a pane banner "Checking grammar with … [Cancel  Esc]"; Cancel/Esc → `grammar.update(\|g, cx\| g.cancel_request(window, cx))`; status-line AI indicator on while shown |
| `Show` | open `OverlayKind::Dialog("grammar")` |
| `Apply(text)` | replace the whole document as one edit: `Edit { range: 0..len, text, selection: 0..0 }.minimized(&old)` via `apply_edit` |
| `Closed` | `close_overlay` |
| `Status { text, error }` | status-line message ("No file open", "No text to check", non-note refused) |
| `OpenSettings` | Settings › Integrations |

Backdrop clicks: the grammar dialog should ignore them when `grammar.read(cx).state().has_edits()`.
