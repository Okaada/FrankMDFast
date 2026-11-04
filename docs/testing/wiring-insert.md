# Wiring requests: insert surfaces (round 4, INSERT agent)

Everything below is already working end to end inside `EditorPane`: the slash menu,
`[[` and Ctrl+K completion, the table editor, the code block dialog, the emoji picker and
Ctrl+. on a table or fence. The pane hosts these surfaces and renders them itself (the
modal scrim and the caret popover are `deferred` layers). The shell needs to connect the
items below so the parts that leave the editor work too.

## 1. Commands (no change needed unless the shell intercepts them)

`insert.table`, `insert.code_block`, `insert.emoji`, `table.edit` and
`edit.context_action` are in `editor::OWNED_COMMANDS`. They reach the pane through the
shell's `_ => self.editor.run_command(id, ..)` fallthrough, or through the pane's key
interceptor (Ctrl+Shift+E, Ctrl+.).

- If you add explicit match arms for these ids in `Shell::run_command`, forward them to
  `EditorPane::run_command`.
- When nothing is under the caret, `run_command("edit.context_action")` returns `false`.
  The shell then shows "Context action not available", which the existing
  `ui_surfaces::ctrl_dot_in_the_editor_runs_the_context_action` test expects.

## 2. Subscribe to `EditorRequest` (second event type on `EditorPane`)

```rust
cx.subscribe_in(&pane, window, |shell, pane, req: &frankmd_app::editor::EditorRequest, window, cx| match req {
    EditorRequest::OpenLink(url) => { /* system opener, http(s)/mailto only (app brief SECURITY) */ }
    EditorRequest::OpenWikilink(target) => { /* resolve like the preview's wikilink click, open the note */ }
    EditorRequest::ViewImage(src) => { /* image viewer (§5.20), local vault file or egress-guarded https */ }
    EditorRequest::OpenMediaDialog(PendingAction::Image | PendingAction::Video) => { /* MEDIA agent's dialog */ }
    _ => {}
})
```

`/image` and `/video` leave a pending `/query` in the pane (`pane.pending_insertion()`).
The media dialogs should insert through the pane so the query gets replaced, or so the
insert is dropped when the query went stale:

- image: `pane.insert_image_markdown(&markdown, window, cx)` (single-newline padding,
  FrankMD `insertImage`);
- video embed: `pane.insert_block(&embed, None, None, window, cx)` (blank-line padding);
- cancel / close without inserting: `pane.clear_pending_insertion()`.

## 3. Context the pane needs

Call these when the state changes. All of them are cheap.

| Call | When | What it enables |
|---|---|---|
| `pane.set_note_context(Some(rel_path), is_markdown, cx)` | a note opens (`false` for `.fed`) | relative paths in Ctrl+K completion; no slash menu or insert dialogs in non-markdown files (FE app_slash_commands:151) |
| `pane.set_vault_notes(paths: Vec<String>)` | tree load and refresh (vault-relative `.md` paths) | `[[` wikilink completion and the note rows of Ctrl+K completion; empty until called |
| `pane.set_ui_locale(Some(code))` | language changes | localized emoji search (pt-BR/pt-PT map to pt, he to en) |

## 4. Optional

- **Status-line context chip** (ui-spec §3.6 / notes_test:554): `pane.context_action(cx)`
  returns `Option<ContextAction>`, and `action.label_key()` gives
  `insert_dialogs.context.{edit_table, edit_code_block, open_link, open_note, view_image}`.
  The pane emits `EditorEvent::SelectionChanged`, so refresh the chip there.
- **Overlay stack**: if the shell would rather render the modal in its own overlay layer,
  `pane.insert_dialog()` returns an `InsertDialog`, with `.view()`, `.kind()`
  (`table` / `code-block` / `emoji-picker`) and `.is_modal()`. The dialogs handle Esc and
  Ctrl+Enter themselves (their own keystroke interceptors), so nothing is needed for the
  Esc chain.

## 5. Changes outside my folders

- `src/assets.rs`: `phosphor_svg` now builds the icon SVG from
  `frankmd_core::emoji::icons::lookup`. The scaffold said it would be filled in when the
  icon picker landed. The unused `PHOSPHOR_JSON` const is removed. The slash-menu icons
  and the icon picker render `icons/ph/<name>.svg` through this function.
- `assets/locales/native/*.yml` (all 7 locales, additive):
  - `dialogs.table_editor.update_table`;
  - `dialogs.code.update_code`;
  - a new `insert_dialogs.{table, code, context}` section.
