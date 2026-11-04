# Test port: frankmd-i18n

The tests compile (`cargo test -p frankmd-i18n --no-run`). Clippy reports only `todo` warnings. They fail at runtime on `todo!()` until the crate is implemented.

| Original | Rust | Original tests | Ported | Skipped |
|---|---|---:|---:|---|
| `test/i18n_media_upload_keys_test.rb` | `tests/i18n_media_upload_keys_test.rs` | 259 (7 locales × 37 keys) | 259 (one `#[test]` per generated row, via `defines!`) | none |
| `test/i18n_vim_keys_test.rb` | `tests/i18n_vim_keys_test.rs` | 126 (7 × 18) | 126 | none |
| `test/services/translation_coverage_test.rb` | `tests/translation_coverage_test.rs` | 2 | 2 | none |
| `test/controllers/translations_controller_test.rb` | `tests/translations_controller_test.rs` | 22 | 22 | none, but see the note below the table |
| (derived) Rails i18n semantics | `tests/catalog_test.rs` | n/a | 8 | n/a |
| golden `translations.json` | `tests/golden_translations.rs` | 37 cases | 37 | none |

About the `translations_controller_test.rb` row: three of its tests are `PATCH /config {locale}`. The config port owns the part where `.fed` gets written. Here those tests only check that the locale reads back correctly.

`catalog_test.rs` covers:
- `%{var}` interpolation,
- the default pluralizer (zero/one/other),
- falling back to `en`,
- YAML 1.2, so `"yes"` and `"no"` stay strings,
- exact locale codes.

How the fixture cases are wired:
- `locales` → the locale codes and `fallback_chain`.
- `flatten_leaves` → every leaf value against `translations/<locale>.flat.json`.
- `interpolation_vars`.
- `required_keys`, in both of its shapes.
- `translation_failures`.
- `translations_controller#show` → the full `served_*.json` body, plus `leaf_count` and `en_fill_keys`.

## API

- `Locale`:
  - `code`,
  - `from_code`, which is exact and case-sensitive,
  - `fallback_chain`.
- `resolve_locale(param, configured)`.
- Tree types: `tree::Node` and `tree::Leaf`, an ordered tree. `Leaf::ruby_class` gives the Ruby class name.
- Tree functions: `parse_locale_yaml`, `locale_tree`, `flatten_leaves`, `translation_failures`, `interpolation_vars` and `key_present`.
- Serving `GET /translations`:
  - `SERVED_SECTIONS`, `deep_merge_over` and `merged_section`,
  - `served_translations`, which returns a `serde_json::Value`,
  - `rails_supplement_en`.
- `Catalog`: `load`, `from_trees`, `locale`, `t` and `t_count`.

New dependencies: `serde_json` and `serde-saphyr`.

New dev-dependencies:
- `frankmd-config`, so the locale precedence goes through the real `.fed` > `FRANKMD_LOCALE` resolution.
- testkit.

## Decisions and quirks

- **Key order.** `serde_json` maps are sorted, but the coverage report depends on Ruby's hash order (`pt-BR` comes before `ja`).
  - That is why `Node` keeps insertion order.
  - For the synthetic golden case, the test reads the locale order from the raw fixture text.
- **Rails gem keys.** `GET /translations` served 31 keys that come from Rails gem locale files, not from the app's YAML. Examples: `errors.format`, `errors.messages.*`, `errors.connection_refused`.
  - This happens even for `en`.
  - `rails_supplement_en()` has to embed these keys. Their values are in `served_en.json`.
- **Invalid `?locale=`.** A `?locale=` that is present but invalid does not fall back to the config value.
  - `?locale=xx` with `FRANKMD_LOCALE=es` gives `en`.
  - `pt-br` and `pt_BR` count as invalid and also give `en`.
- **Missing keys.** `Catalog::t` returns the key itself when no locale has it.
  - That is what the JS `window.t` did; Rails would show "Translation missing".
  - An unknown `%{var}` token stays as written instead of raising an error.
