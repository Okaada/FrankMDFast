# Golden fixtures from the original FrankMD

These scripts execute the ORIGINAL FrankMD implementation (Rails 8 + Stimulus JS,
checked out read-only at `~/Projects/FrankMD`) on a corpus of inputs and record the
outputs as language-neutral JSON under `tests/fixtures/golden/`. The Rust rewrite
consumes these fixtures as conformance tests: same input -> same expected output.

## Conventions (all generators follow these)

* FrankMD is never modified. Scripts live here and are run against the checkout:
  * Ruby: `cd ~/Projects/FrankMD && mise exec ruby@3.4.8 -- bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/<area>.rb`
  * JS:   `node /mnt/data/Projects/FrankMDFast/tools/golden/js/<area>.mjs` (imports FrankMD's
    `app/javascript/lib/*.js` by absolute path; FRANKMD env var overrides the checkout path).
* Temporary NOTES_PATH dirs are created under `$GOLDEN_TMP` (default: a mktmpdir) and deleted after.
* Output: `tests/fixtures/golden/<area>.json` with shape

  ```json
  {
    "source": "app/services/hugo_service.rb#slugify @ <sha>",
    "frankmd_sha": "<full git sha of ~/Projects/FrankMD HEAD>",
    "generator": "tools/golden/ruby/hugo.rb",
    "notes": "free text: normalizations applied, caveats",
    "cases": [ { "name": "...", "fn": "slugify", "input": ..., "expected": ... } ]
  }
  ```

  * `fn` names the function/method under test when a file covers more than one.
  * Exceptions are captured as `"expected": {"error": "<ClassName>", "message": "..."}`.
  * Filesystem cases: `input.tree` is a tree spec (`{"path": "content"}` for files,
    `{"path/": null}` for empty dirs, `{"link": {"symlink": "target"}}` for symlinks),
    `input.ops` the operations, and `expected` holds results plus `tree_after`
    (sorted map of relative path -> file content, dirs end with `/`).
  * Volatile values (absolute temp paths, mtimes, current time) are normalized:
    temp root replaced by `<ROOT>`, times either pinned via File.utime/Time.stub or replaced by `<TIME>`.
  * Output JSON is pretty-printed with stable key order so diffs are reviewable.
* Shared helpers: `tools/golden/lib/golden.rb` and `tools/golden/lib/golden.mjs`.

## JS value encoding

JS fixtures encode values plain JSON can't carry (see `lib/golden.mjs` encode/decode):
`{"$undefined":true}`, `{"$number":"NaN"|"Infinity"|"-Infinity"|"-0"}`, `{"$date":"<ISO>"}`,
`{"$regexp":"/src/flags"}`, `{"$utf16":[code units]}` (ill-formed strings with lone surrogates,
e.g. a non-unicode JS regex `.` splitting an emoji), and errors as `{"$error":{"name","message"}}`.
All string offsets are UTF-16 code units; cases whose string args contain multi-unit chars carry
a `utf16` array with utf16/scalar/utf8 lengths.

## Input harvesting from the original test suite (`js/harvest/`)

`js/harvest/harvest.sh <out.jsonl> mod...` runs FrankMD's own vitest suite (2238 tests) with the
named `app/javascript/lib/<mod>.js` modules aliased to generated recording wrappers
(`make_wrappers.mjs`, `recorder.mjs`, `vitest.harvest.config.mjs`). Every distinct call's arguments
are appended to a JSONL file. Only INPUTS are harvested; generators recompute expected values in a
fresh node process against the real module. The capped, committed result for the utils libs is
`js/harvest/inputs/utils.jsonl`.

## Generators

Ruby generators run inside FrankMD via `rails runner` (from the FrankMD checkout):

```sh
cd ~/Projects/FrankMD
mise exec ruby@3.4.8 -- bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/<area>.rb
```

| Generator | Fixtures (`tests/fixtures/golden/`) | Extra env |
|---|---|---|
| `ruby/config.rb` | `fed_template.fed` (verbatim auto-generated `.fed`), `config_template.json`, `config_parse.json`, `config_env.json` (incl. `keys` table of every .fed key / ENV var / default), `config_writeback.json`, `egress_policy.json`, `custom_theme.json` | — |
| `ruby/path_safety.rb` | `path_safety.json` | `RAILS_ENV=test TZ=UTC FRANKMD_LOCALE=en` |
| `ruby/notes_tree.rb` | `notes_tree.json` | same |
| `ruby/notes_crud.rb` | `notes_crud.json` | same |
| `ruby/notes_rename.rb` | `notes_rename.json` | same |
| `ruby/search.rb` | `search.json` | same |
| `ruby/recent_files.rb` | `recent_files.json` | same |
| `ruby/backlinks.rb` | `backlinks.json` | same |
| `ruby/helpers.rb` | `tree_helper.json`, `recent_files_helper.json` | same |
| `ruby/hugo.rb` | `hugo.json` | — |
| `ruby/media_usage.rb` | `media_usage.json` | — |
| `ruby/media.rb` | `media.json` (Time pinned 2026-01-01T12:00:00Z, SecureRandom stubbed) | — |
| `ruby/omarchy_theme.rb` | `omarchy_theme.json` (embeds the inputs of all ~/Projects/omarchy/themes) | — |
| `ruby/translations.rb` | `translations.json`, `translations/*.json` | — |

The notes-FS generators share `ruby/notes_fs_common.rb`. They drive some cases through real Rails
HTTP requests, which is why they need `RAILS_ENV=test` (CSRF and host checks off).

JS generators run with plain node from the FrankMDFast root:

| Generator | Fixtures |
|---|---|
| `js/utils.mjs` | `js_{text,markdown,table,indent,diff,stats,tree,url,find,task,typewriter,text_editor,new_note}_utils.json`, `js_s3_key.json`, `js_line_numbers.json`, `js_menu_bounds.json`, `js_scroll_utils.json` |
| `js/render.mjs` | `js_preview_pipeline.json` (markdown → final preview HTML), `js_marked_extensions.json`, `js_html_sanitizer.json`, `js_markdown_line_mapper.json`, `js_math_renderer.json`, `js_code_block_highlighter.json`, `js_terraform_hcl_stream_parser.json` |
| `js/ui_data.mjs` | `js_keyboard_shortcuts.json`, `js_custom_theme.json`, `js_emoji_data.json`, `js_emoji_i18n.json`, `js_icon_data.json`, `js_codemirror_theme.json`, `js_codemirror_wikilink.json` |
| `js/draft_storage.mjs` | `js_draft_storage.json` |
| `js/image_sources.mjs` | `js_image_sources.json` |

The JS generators load the real vendored libraries the app ships: marked, DOMPurify, KaTeX and
CodeMirror/Lezer from `vendor/javascript`. Imports are resolved through `js/runtime/importmap_hooks.mjs`,
which mirrors `config/importmap.rb`, so the vitest mocks are not used. DOM work happens in jsdom
(`js/runtime/setup.mjs`). Any HTML in these fixtures is jsdom serialization.

`utils.mjs` and `render.mjs` take `REHARVEST=1` to re-run FrankMD's vitest suite and harvest fresh inputs.
They pick a capped, content-hash-ordered subset, so the result is deterministic.

To regenerate everything: `tools/golden/regen_all.sh`. Every generator has been run twice and gave
byte-identical output both times.
