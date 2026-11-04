# Assets reused from FrankMD

Everything here comes from the original FrankMD checkout; regenerate with
`node tools/assets/extract.mjs ~/Projects/FrankMD`. Each JSON file records the
source file and commit it came from.

| Path | What | Used for |
|---|---|---|
| `brand/` | FrankMD logo (`icon.svg`) and PNG sizes | App/window icon, `.desktop` icon, about/empty states |
| `locales/*.yml` | Rails i18n files for en, pt-BR, pt-PT, es, he, ja, ko | Loaded unchanged by the i18n crate (Rails `%{var}` interpolation) |
| `themes/builtin.json` | The 18 built-in themes, 38 `--theme-*` color roles each | Fallback themes when not following Omarchy; same ids as the `.fed` `theme` setting |
| `emoji/shortcodes.json` | `[shortcode, emoji, keywords]` table | Emoji picker and `:shortcode:` rendering in preview |
| `emoji/names/*.json` | Localized emoji labels (en, es, ja, ko, pt) | Emoji picker search in the user's language |
| `emoji/emoticons.json` | Emoticon/kaomoji list | Emoticon picker tab |
| `icons/phosphor.json` | Curated Phosphor icon SVG paths (MIT) | Icon picker tab; can also back UI icons |
| `fonts/web/*.woff2` | Latin-subset editor fonts (ids match the `editor_font` setting) | Reference only: GPUI loads TTF/OTF, so the app resolves these ids to system fonts via fontconfig |
| `fonts/katex/*.woff2` | KaTeX math fonts (MIT) | Math rendering in preview, if the chosen math renderer needs them (convert to TTF) |

Licenses: Phosphor icons and KaTeX fonts are MIT. The editor fonts are OFL-1.1
(Fira Code, JetBrains Mono, Cascadia Code, Hack, Source Code Pro, Inter,
Ubuntu Mono via the Ubuntu Font Licence), Apache-2.0 (Roboto Mono), and the
Bitstream Vera licence (DejaVu Sans Mono).
