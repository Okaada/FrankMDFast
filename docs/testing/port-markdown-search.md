# Test port: markdown preview + search

Crates: `crates/frankmd-markdown`, `crates/frankmd-search`. Status: every test compiles
(`cargo test -p <crate> --no-run`) and `cargo clippy -p <crate> --tests` is clean apart from
`todo`. Bodies are still `todo!()`, so the tests fail at runtime on those panics. Each failure in an
`assert_cases` golden lists every case.

## Original tests → Rust tests

| Original | Rust | Orig. | Ported | Skipped (reason) |
|---|---|---:|---:|---|
| `test/javascript/lib/marked_extensions.test.js` | markdown `tests/marked_extensions_test.rs` | 44 | 44 | — |
| `test/javascript/lib/markdown_line_mapper.test.js` | markdown `tests/markdown_line_mapper_test.rs` | 14 | 14 | — (the 5 C `findLineAtScroll` DOM-rect tests are re-expressed as the scroll geometry they test) |
| `test/javascript/lib/html_sanitizer.test.js` | markdown `tests/html_sanitizer_test.rs` | 23 | 23 | — (`null`/`undefined` → `""` reduces to `""`) |
| `test/javascript/lib/math_renderer.test.js` | markdown `tests/math_renderer_test.rs` | 7 | 5 | 2: "is idempotent" (the `data-math-rendered` DOM guard; the IR never re-reads rendered output) and "null container" (no null in Rust) |
| `test/javascript/lib/code_block_highlighter.test.js` | markdown `tests/code_block_highlighter_test.rs` | 28 | 27 | 1: "does not apply a lazy parser result to code replaced by a newer render" (async DOM race; a native render is one synchronous pure call). "highlights common languages before lazy parser imports resolve" (C) is adapted to "all languages in one pass". |
| `test/javascript/lib/scroll_utils.test.js` | markdown `tests/scroll_utils_test.rs` | 32 | 32 | — |
| `test/javascript/lib/line_numbers.test.js` | markdown `tests/line_numbers_test.rs` | 10 | 10 | — |
| `test/javascript/controllers/preview_controller.test.js` | markdown `tests/preview_controller_test.rs` | 89 | 1 A + 4 C adapted | Zoom (5 A) goes to the app/settings port. The other C tests are controller/DOM behaviour for the frankmd-app UI port. Adapted: per-item task lines, front matter with absolute lines, `totalLines`, empty content. |
| `test/javascript/lib/task_utils.test.js` | — | 11 | 0 here | Already ported by the editor agent into frankmd-core (`text::task`, see `port-editor.md`). The IR points at `frankmd_core::text::task::task_marker_range` for click-to-toggle. |
| `test/javascript/lib/find_utils.test.js` | search `tests/find_utils_test.rs` | 15 | 15 | — |
| `test/services/notes_service_test.rb` (search part) | search `tests/notes_service_search_test.rs` | 14 | 14 | — (13 `search_content` tests + the search half of "list_tree and search skip symlinked directories"; the rest of the file belongs to the notes-fs port) |
| `test/controllers/notes_controller_test.rb` (search endpoint) | search `tests/notes_controller_search_test.rs` | 4 | 4 | — (endpoint = `Query::ui`: context 3, max 20; HTTP is Rails plumbing) |
| `test/models/note_test.rb` (`Note.search`) | search `tests/note_search_test.rs` | 1 | 1 | — |

Two Ruby search tests depended on stubs and are adapted:
- EACCES now uses a real `chmod 000` file. The test skips itself when run as root.
- The stubbed `Regexp::TimeoutError` is now a catastrophic pattern with a 1 ms budget.

## Golden fixtures

| Fixture | Rust test | Cases wired / total | Not driven |
|---|---|---:|---|
| `js_preview_pipeline` | `golden_preview.rs` | 89/89 | — (`previewRender` 71, `stripFrontmatter` 18) |
| `js_marked_extensions` | `golden_marked_extensions.rs` | 262/281 | 18 `marked.parse` inputs containing `<`: raw output is unsanitized, and the same corpus entries are compared sanitized in `golden_preview`. 1 known divergence (icon renderer, below). |
| `js_markdown_line_mapper` | `golden_line_mapper.rs` | 118/126 | 1 negative frontmatter offset (`SourceSpan` is 1-based `u32`). 7 `marked.lexer(types+raw)` informative token dumps (no IR equivalent). |
| `js_math_renderer` | `golden_math.rs` | 56/57 | `null container` |
| `js_html_sanitizer` | `golden_sanitizer.rs` | 125/125 | — |
| `js_code_block_highlighter` | `golden_highlighter.rs` | 157/159 (+ ignored strict token test) | `misc "null"` and `inElement null root` (JS falsy guards) |
| `js_terraform_hcl_stream_parser` | `golden_hcl.rs` | 27/27 | — |
| `js_scroll_utils` | `golden_scroll_utils.rs` | 95/95 | — |
| `js_line_numbers` | `golden_line_numbers.rs` | 40/44 | 4 `nextLineNumberMode` on non-mode values (3, -1, null, "1"): JS returns OFF, but the typed enum can't express those inputs |
| `js_emoji_data` (render part) | `golden_emoji_render.rs` | 1/1 (all 675 shortcodes rendered) | `EMOJI_DATA`, shape: frankmd-core's data, ported by the editor agent |
| `search` | search `golden_search.rs` | 73/74 | `http` `text/html`: the Rails results partial (UI markup) |
| `js_find_utils` | search `golden_find_utils.rs` | 147/147 | — |

## How the preview is compared (native IR vs marked + DOMPurify HTML)

The preview renders natively: comrak → `RenderDoc` IR → GPUI. Two things keep the original HTML
fixtures usable:
- `html::to_html` stays as a **conformance oracle**.
- The tests also check the IR directly.

For every `previewRender`, `parseWithLineNumbers` and `marked.parse` case:

1. **Structure.** `to_html(doc)` must be ≅ the original HTML after the normalization below, with
   `data-source-line` dropped.
2. **Line semantics, checked on the IR.**
   - Top-level block `(tag, span.start_line)` must equal the original's top-level annotated
     elements.
   - List-item lines must equal every annotated `<li>`, in document order: top-level lists, each
     item followed by its directly nested lists' items.
3. **Code.** From `final_html`:
   - the `data-code-language` label must equal `highlight::language_for_fence(lang).label`;
   - original highlighted ⇒ IR highlighted;
   - no label ⇒ IR not highlighted.
4. **Math.**
   - The placeholders' TeX and style (inline/display) must equal the IR math nodes, in order.
   - KaTeX's verdict must equal `math::validate`. The verdict is read from `final_html`: clean, a
     `katex-error` parse error (incl. too many expansions), or a render with red unknown/untrusted
     commands (`\href`, `\htmlId`, `\undefinedcommand`…).
5. `doc.total_lines` must equal `totalSourceLines`, i.e. JS `split("\n").length`.

### Normalization (`tests/common/mod.rs::normalize_html`)

Both sides go through these steps:
1. **Parse.** html5ever (`scraper`) parses each side as a fragment in `<body>` context. After this,
   entity spelling, quoting, `<br>`/`<br />`, `checked`/`checked=""`, SVG self-closing children and
   implied end tags no longer matter.
2. **Attributes.** They are sorted by name. Dropped names are removed: `data-source-line` for
   structure comparisons, none for the sanitizer fixture.
3. **Whitespace,** everywhere outside `<pre>`, `<code>` and `<textarea>`:
   - runs collapse to one space;
   - whitespace-only text next to, or at the edge of, a block-level element or `<br>` is dropped;
   - text is trimmed on the side that touches such an element.
4. **Serialization.** Text is escaped as `& < >` and attribute values as `& "`. Comments are kept.

### Original quirks handled by the comparison

- **Sequential `data-source-line` slots.** FrankMD gives the token lines to the first N block tags
  of marked's HTML by regex, so nested blocks (`<p>` in `<blockquote>`, a nested `<ul>`) and block
  tags inside raw HTML steal later slots. Example from the `tasks` corpus: the `<ol>` gets 7 and
  the third list gets nothing.
  - When this is visible (a nested non-`li` element is annotated, or a top-level block tag isn't),
    the top-level line comparison is skipped for that case. It is also skipped when the IR has raw
    HTML blocks.
  - `<li>` lines are compared unless some `<li>` went unannotated.
  - **Decision:** the IR carries the *true* lines. The original's lines are wrong, and copying them
    would make preview task toggling hit the wrong line.
- **Front-matter off-by-one.** `stripFrontmatter` returns `frontmatterLines` = front-matter lines
  + 1 and `trimStart()`s the body. FrankMD's lines are therefore only right when exactly one blank
  line follows the front matter: `+++\n…\n+++\n# Body` reports line 5 for a heading on line 4.
  - `frontmatter::strip` reproduces the JS result exactly (the `stripFrontmatter` golden).
  - `render()` gives spans of the whole note.
  - The golden adds `shift = (frontmatterLines + 1) − true first body line` to the IR lines before
    comparing.
  - This is a bug in the original. It mis-targets preview task toggles after front matter.
- **Footnotes.** marked has none, and the superscript extension mangles `[^1] … [^note]` into
  `<sup>1] and [</sup>`. The fixture expects this. `RenderDoc.footnotes` stays empty for
  compatibility.
- **Block math interrupting a paragraph.** `text $$x$$` closes the paragraph and emits
  `<div class="math-block">` (fixture `math_errors`). It is expected as recorded.
- **Dockerfile.** The shipped app never loaded the Dockerfile parser (missing vendored
  `simple-mode.js`), so the label shows but there are no tokens. We highlight it, which matches the
  original test suite's intent. The highlighter checks are one-directional for that reason.
- **Wikilink label `""`.** `[[a|  ]]` trims to `displayText: ""`, and the renderer then falls back
  to the target's basename (JS falsy). `Inline::WikiLink.label` may be `Some("")`, and renderers
  must treat it as absent.

### Known divergences (`tests/common/mod.rs::KNOWN_DIVERGENCES`)

| Fixture | Case | Why |
|---|---|---|
| `js_marked_extensions` | `icon.renderer {"path":"\"><script>",…}` | FrankMD puts icon path data into the SVG unescaped. We escape attribute values. The output is identical for every real Phosphor path, which is trusted data with no quotes or `<>`. |

The implementation phase may add entries here, but only with a written reason in both places.

## Other decisions

- **Token roles.** `ir::TokenClass` now uses the theme's `--theme-syntax-*` roles (keyword, function,
  operator, string, number, constant, comment, type, property, identifier, punctuation, invalid),
  plus `TokenStyle.definition` for Lezer's `tok-definition`. The mapping from Lezer classes follows
  FrankMD's `prose.css` and is tabled in `ir.rs`. The per-language role tests assert these roles.
  Token-exact Lezer parity is not a goal. The `#[ignore]`d `golden_tokens_strict` measures the
  drift.
- **Highlight budgets** (`HighlightBudget`) are FrankMD's: a block is skipped above 20 000, the
  total per render is capped at 50 000, and at most 100 classed blocks are inspected.
  - Lengths are **UTF-16 units** of the code text as marked emits it, including the trailing `\n`.
  - Unknown-language classed blocks still use up an inspection slot.
  - `highlight::plan` holds this logic and is tested against every highlighter case.
- **Terraform/HCL.** FrankMD's hand-written stream tokenizer is ported as `highlight::hcl`
  (golden-exact), not replaced by a grammar.
- **Scroll math and line numbers** (`linemap::scroll`, `line_numbers`) live in frankmd-markdown
  because this port was assigned them. Architecture §3.8 puts scroll-sync policy in
  frankmd-workspace, which can call or re-export these pure functions. That would add a
  `workspace → markdown` dependency, or the module can move.
- **Line-number values.** `normalize_line_number_mode` takes the value's string form (what `.fed`
  stores) with JS `parseInt` semantics.
- **Search.** Content search uses Ruby/Onigmo semantics via `onig`. That gives multi-character case
  folding (`straße` ~ `STRASSE`), and an invalid regex is matched as an escaped literal. Editor
  find/replace uses JS semantics via `fancy-regex`.
  - **Equal-mtime ordering.** Ruby gives no order among results with the same whole-second mtime,
    so those groups are compared after a stable sort by path.
  - **Readdir-order case.** For the `max_results` traversal case only the count and the
    newest-first order are checked.
- **find_utils.** Match offsets are UTF-16 units.
  - **Deliberate deviation:** matching works on whole Unicode characters, so a match never splits
    an emoji. JS without the `u` flag can return half-emoji matches (cases #25 and #36). For those
    cases the test checks that every expected match not touching a half-emoji appears in order.
  - `validateRegex` messages are checked by their `Invalid regular expression: /…/: ` prefix only.

## Cross-crate requirements

- **frankmd-core:**
  - A Phosphor icon lookup, e.g. `emoji::icons::lookup(name) -> Option<(path, view_box)>` over
    `assets/icons/phosphor.json`, used by `parse::ext::scan_icon`.
  - `emoji::lookup` where the last duplicate row wins.
  - `text::task::task_marker_range` for preview task toggling.
- **Workspace `Cargo.toml`:**
  - Promote `onig` (now pinned `6.5` in frankmd-search) to `[workspace.dependencies]`.
  - `scraper` is used as a dev-dependency of frankmd-markdown (already a workspace dep).
- **frankmd-workspace / app:** consume `linemap::{LineMap, scroll}` for scroll sync and typewriter
  alignment, and `line_numbers` for the gutter setting.
