# Omarchy 4.5 integration research for FrankMDFast

Status: research, 2026-11-04. Source: `~/Projects/omarchy`, branch `quattro` at `087f73d89` (20 commits past `v4.5.0-rc1` = `03ff1eff3`). FrankMD reference: `~/Projects/FrankMD` (Rails).

Branch topology note: `v4.0.6` (`237f1b407`) sits on a separate maintenance line (`upstream/v4-0-6`) and is **not** an ancestor of `quattro`. The merge base is `f0020448c` (2026-08-14). `git log v4.0.6..HEAD` lists 1138 commits; `git diff v4.0.6..HEAD` compares trees. `origin/rc` and `origin/dev` are still on 3.8.5 and do not matter for 4.5. The `version` file on quattro still reads `4.0.0.alpha`.

The most important upstream source for us is new in 4.5: **`default/agents/skills/omarchy-app/SKILL.md` + `templates.md`**. This is Omarchy's own guide for building native Omarchy apps (Hype, Omacut, Monologue, Signed). It was added in `c05d90196` (#13770) and extended in `326c853ca` (#15372) and `84cfb9395` (#15326, the i18n work). It targets C++/Qt Quick, but its conventions carry over to any toolkit. Where this document says "the app skill", it means that file.

---

## 1. What changed from 4.0 to 4.5 that matters to third-party apps

| Area | Change | Citation |
|---|---|---|
| App guidance | New `omarchy-app` agent skill covering naming, desktop file, live theme, live language, rounding, keyboard conventions, packaging into `omacom/omarchy-pkgs`, and an agent-friendly headless CLI | `default/agents/skills/omarchy-app/` (`c05d90196`, `326c853ca`, `84cfb9395`) |
| i18n | Omarchy now ships in 50 languages. The choice is stored in `~/.config/omarchy/language`. Picking a language also sets session `LANG` through `~/.config/uwsm/env.d/50-omarchy-language` and pushes it into the running Hyprland/systemd/D-Bus environment | `docs/i18n.md`, `bin/omarchy-language`, `default/i18n/locales.json` (`84cfb9395`, `dd07b6269` #15791) |
| i18n | Picking a language at install time: the ISO presets language, keyboard and timezone | `3ac43659d` (#15522) |
| Input methods | Fcitx5 is now standard and seeded from the installer keyboard choice. Session env sets `QT_IM_MODULE`/`XMODIFIERS`/`SDL_IM_MODULE` but deliberately leaves `GTK_IM_MODULE` unset (Wayland text-input). Picking ja/ko/zh in the language menu adds Mozc, Hangul, Pinyin or Chewing. The Fcitx candidate window is now themed | `docs/input-methods.md`, `default/environment.d/10-omarchy-fcitx.conf`, `default/themed/fcitx5-theme.conf.tpl` (`0b9dace01` #14433, `c582e1d7f` #15508, `d7abdbd42`) |
| Fonts/CJK | A large new `50-omarchy.conf` handles fontconfig CJK variant selection by language tag, plus `omarchy font cjk <jp\|kr\|sc\|tc\|hk\|default>` | `default/fontconfig/conf.avail/50-omarchy.conf`, `bin/omarchy-font-cjk` (`62c5165d0` #15116, `c582e1d7f`) |
| Theme switching | Retints are split into `visible_theme_commands` (synchronous) and `other_theme_commands` (detached, logged to `~/.local/state/omarchy/theme-set.log`). The `theme-set` hook now runs **after** the detached group, so it can arrive seconds after the files change. A new `theme.generation` token is written on every switch | `bin/omarchy-theme-set` L497–627, `docs/theming.md` "Theme activation flow" (`d8027c90d` #15189, `b96b9c599` #15045) |
| Theme switching | Background intro videos. App retints wait up to about 3 s for the intro crossfade | `docs/theming.md` (`902fd8aeb` #9639) |
| Theme security | Themes installed from git are filtered: no `*.lua`, no terminal configs, no `vscode.json`, no symlinks. Generated files replace whatever was dropped | `docs/theming.md` "What an installed theme may not ship" |
| Theme tokens | No new `colors.toml` keys since 4.0.6 (`themes/tokyo-night/colors.toml` is identical). Docs now say outright that there is **no `urgent` key** (`red` fills that role) | `docs/theming.md` L101–102 |
| Themes | `starship` added as a stock theme. Stock themes are now 23 | `d38b70c3c` (#14705) |
| Notifications | The shell's notification engine was replaced by the "Omapager" design: decks grouped per app, snoozes, DND, persistence, and click routing by `omarchy-exec-argv` hint, then `default` action, then window by pid, then class via `omarchy-hyprland-focus-app` | `docs/notifications.md` (`4671ae2d3`, merged in `03ff1eff3` #15932; `329fd1a9b` desktop-entry icon fallback) |
| Window look | Window transparency is opt-in per app and chosen under Appearance > Window Transparency (Glassy). Corner rounding toggles were added (`corners-rounded.lua` sets `rounding = 6`; the default is 0) | `54e3b53b8` (#14253), `a98e55eb9` (#14991), `default/hypr/toggles/corners-*.lua`, `default/hypr/looknfeel.lua:25` |
| Hyprland | The config is Lua (`hyprland.lua`, `o.window(...)` helper). Window rules match on `class` (= Wayland app_id) | `default/hypr/windows.lua`, `default/hypr/helpers.lua:279`, `default/agents/skills/omarchy/hyprland.md` |
| Menu/CLI | The menu is a Quickshell plugin driven by JSONC (`omarchy-menu.jsonc` plus a user extension). The `omarchy` CLI router maps `omarchy a b` to `bin/omarchy-a-b` | `docs/menu.md`, `docs/cli-router.md` (both new docs) |
| Packages | Omarchy's own native apps (`hype`, `monologue`, `omacut`, `omawrite`, `signed`) are in `install/omarchy-base.packages` | `install/omarchy-base.packages` L60–130 |

What did **not** change and still trips FrankMD: the current theme moved from `~/.config/omarchy/current/` to `~/.local/state/omarchy/current/` **before** 4.0.6 (`v4.0.6:bin/omarchy-theme-set` L12 already uses the state path). Migration `migrations/1781043107.sh` rewrites references in user files and leaves no compatibility symlink. On this machine `~/.config/omarchy/current` does not exist.

`docs/lifecycle-dispatch.md` (platform boot hooks for Apple Silicon and similar) and `docs/elsewhen.md` do not affect apps.

---

## 2. Theming

### 2.1 Where the active theme lives

| Path | Content |
|---|---|
| `~/.local/state/omarchy/current/theme/` | The **staged, rendered** active theme directory: stock theme, overlaid by the user theme, plus every `default/themed/*.tpl` and `~/.config/omarchy/themed/*.tpl` rendered into it |
| `~/.local/state/omarchy/current/theme/colors.toml` | **Canonical palette.** Flat TOML with `key = "#rrggbb"` lines plus `mode = "dark"\|"light"` |
| `~/.local/state/omarchy/current/theme.name` | Theme slug (e.g. `tokyo-night`). `omarchy theme current` title-cases it |
| `~/.local/state/omarchy/current/theme.generation` | Opaque token `"$$-<epoch-us>"` rewritten on every switch (new in 4.5) |
| `~/.local/state/omarchy/current/background` | Symlink to the active wallpaper |
| `~/.local/state/omarchy/current/theme/shell.toml` | Shell surface roles and sizing (`[bar]`, `[controls]`, `[menu]`, `[font] base-size`…). Mode 0600 on this machine. Shell-specific, so don't depend on it |
| `~/.local/state/omarchy/current/theme/icons.theme` | GTK icon theme name (e.g. `Yaru-blue`) |
| `~/.local/state/omarchy/current/theme/backgrounds/` | Theme wallpapers |
| `/usr/share/omarchy/themes/<slug>/` | Stock theme sources (`$OMARCHY_PATH/themes`) |
| `~/.config/omarchy/themes/<slug>/` | User or git-installed themes (overlay a stock slug or add new ones) |
| `~/.config/omarchy/themed/*.tpl` | User templates, rendered into the current theme dir on each switch |

Activation (`bin/omarchy-theme-set` L420–504) works like this. It builds `current/next-theme`, renders templates, then runs `rm -rf current/theme && mv current/next-theme current/theme`. After that it writes `theme.generation`, then `theme.name`. **The directory is replaced, not edited in place**, so a watch on `current/theme/colors.toml` dies at every switch. Watch the parent `current/` directory instead and re-arm.

### 2.2 Color tokens (`colors.toml`)

Canonical keys as written by stock themes (e.g. `themes/tokyo-night/colors.toml`, `themes/catppuccin-latte/colors.toml`), grouped the way `docs/theming.md` L65–90 groups them:

- **Mode:** `mode` (`dark` | `light`)
- **Semantic:** `accent`, `selection`, `muted`
- **Background ramp:** `background`, `dark_background`, `darker_background`, `lighter_background`
- **Foreground ramp:** `foreground`, `dark_foreground`, `light_foreground`, `bright_foreground`
- **Named colors:** `red`, `yellow`, `orange`, `green`, `cyan`, `blue`, `magenta`, `brown`
- **Bright colors:** `bright_red`, `bright_yellow`, `bright_green`, `bright_cyan`, `bright_blue`, `bright_magenta`

Themes may add extra keys that templates can reference. `hyprland_active_border` (a gradient string) is the known one.

Derived and alias keys come from the resolver in `bin/omarchy-theme-color` (`resolve_theme_colors`). A consumer reading `colors.toml` directly must reproduce this cascade, because user and older themes may define only part of the palette:

- Legacy short names map to canonical names: `bg`, `dark_bg`, `darker_bg`, `lighter_bg`, `fg`, `dark_fg`, `light_fg`, `bright_fg`. Canonical wins when both exist.
- ANSI fallbacks: `background ← color0`, `foreground ← color7`, `red ← color1` … `cyan ← color6`, `bright_red ← color9` … `bright_cyan ← color14`, `magenta ← purple`.
- `light_foreground ← color7 | foreground`, `bright_foreground ← color15 | foreground`, `lighter_background ← color0 | background`, `dark_foreground ← color8 | foreground`, `muted ← color8 | dark_foreground`.
- `selection ← selection_background | color8 | color0 | background`. Then `selection_background = selection` and `selection_foreground = bright_foreground` (unless set).
- `cursor = bright_foreground` (always).
- `orange ← yellow`, `brown ← mix(orange, #000, 50%)`.
- `dark_background ← mix(background, #000, 25%)`, `darker_background ← mix(background, #000, 50%)`.
- `bright_* ← mix(<color>, #fff, 20%)` for red/yellow/green/cyan/blue/magenta.
- Reverse aliases `color0..color15` and the short names are always exposed.
- `mix` is a linear sRGB per-channel blend with rounding (`mix_color` awk).

Ramp semantics (`docs/theming.md` L119–126): "Dark themes should read from darkest to lightest; light themes … from lightest to darkest." In practice `dark_background`/`darker_background` sit **recessed** (Omarchy's own templates use them for sidebars and code: `default/themed/t3code.json.tpl`). `lighter_background` sits **raised** (one step toward the foreground) in both modes. Catppuccin Latte's `lighter_background` (`#dce0e8`) is darker than its `background` (`#eff1f5`). Don't assume "lighter" means higher luminance.

There is no `urgent` key (use `red`) and no cursor key (use `bright_foreground`).

### 2.3 Light/dark detection

Order of precedence, from `bin/omarchy-theme-color` `resolve_theme_mode`:

1. `mode` key in `colors.toml` (all 23 stock themes set it)
2. legacy `theme_type` key
3. a `light.mode` file next to `colors.toml`
4. background luminance: `R+G+B > 382` means light
5. otherwise dark

Stock light themes: `catppuccin-latte`, `flexoki-light`, `lupine`, `rose-pine`, `white`.

Secondary signal: `omarchy-theme-set-gnome` (in the detached `other_theme_commands`) sets `gsettings org.gnome.desktop.interface color-scheme` to `prefer-dark`/`prefer-light`. `config/xdg-desktop-portal/hyprland-portals.conf` routes Settings through the gtk portal, so `org.freedesktop.portal.Settings` → `org.freedesktop.appearance color-scheme` (verified: returns `1` = dark on this machine) **does emit `SettingChanged`**, but late, after the visible retints. Use it only as the fallback on non-Omarchy desktops.

### 2.4 Getting notified of theme changes

Omarchy offers no D-Bus signal and no POSIX signal for third-party apps. `omarchy-theme-set` retints apps from a hard-coded list in `bin/omarchy-theme-set` L529–557, which third parties can't join without patching Omarchy. The options:

| Mechanism | Latency | Verdict |
|---|---|---|
| **inotify on `~/.local/state/omarchy/current/`** (IN_CLOSE_WRITE / IN_MOVED_TO / IN_CREATE on `theme.name` or `theme.generation`, plus the `theme` dir rename), debounce ~80–150 ms, then re-read `theme/colors.toml` | Immediate (swap happens before any retint) | **Recommended.** This is what the app skill's `Theme` class does: watch the dir and its parents, re-arm after every change, fall back to the nearest existing ancestor so installing Omarchy later still works (`templates.md` `Theme::reload`) |
| `theme-set` hook (`~/.config/omarchy/hooks/theme-set.d/<script>`, slug in `$1`), installed by the user with `omarchy hook install theme-set <script>` | Late: runs after the detached `other_theme_commands` batch | Optional extra (e.g. `frankmd reload-theme`). The app must never install it itself (the app skill says "Never change Omarchy's config") |
| Portal `SettingChanged` `org.freedesktop.appearance color-scheme` / `accent-color` | Late (gsettings set in the detached batch) | Fallback for GNOME/KDE and non-Omarchy |
| Polling (FrankMD today: HTTP poll comparing `theme_name`) | Seconds | Drop it. Comparing the name misses `omarchy theme refresh` and edited overlays of the same slug. Compare the parsed palette, or `theme.generation`, instead |

A prior example of the hook approach on this machine: `~/.config/omarchy/themed/zapfast.json.tpl` plus `~/.config/omarchy/hooks/theme-set.d/zapfast-theme`, which copies the rendered JSON atomically into the app's config and pings `zapfast reload-themes`. It works, but it needs two files dropped into Omarchy's config and only refreshes after the late hook.

### 2.5 Consume tokens directly, or ship a template?

| | Read `colors.toml` directly | Ship a `default/themed`-style template |
|---|---|---|
| How | Parse TOML, apply the §2.2 cascade, map to UI tokens in Rust | `frankmd.json.tpl` in `~/.config/omarchy/themed/` → rendered `current/theme/frankmd.json` |
| Who can install it | The app alone | A built-in template needs an Omarchy PR to `default/themed/` plus an `omarchy-theme-set-frankmd` in the retint list. A user template must be copied by the user into Omarchy's config dir |
| Updates | Live, via inotify | Only rendered on the next `theme set`. Stale on first install until a switch |
| Theme-author control | None (our mapping) | Theme authors can ship `themes/<x>/frankmd.json` to override (allowed in git themes, since it is "colour") |
| Precedent | Omarchy's own apps (app skill `Theme` reads `accent` from `colors.toml`) | Obsidian, T3 Code, VS Code, Helix (first-party integrations living in Omarchy's repo) |

**Recommendation:** read `colors.toml` directly as the primary path, as Omarchy's own native apps do. Also honour an optional override file: if `~/.local/state/omarchy/current/theme/frankmd.toml` (or `.json`) exists, because a theme author shipped it or the user added `~/.config/omarchy/themed/frankmd.toml.tpl`, its explicit values replace the derived ones. That costs a few lines of code and lets theme authors tune FrankMD later without us depending on Omarchy accepting a template. Ship a sample `frankmd.toml.tpl` in our repo under `contrib/omarchy/`, documented but not auto-installed. An upstream template PR can come later, once the app is in `omarchy-pkgs`.

### 2.6 Proposed mapping: Omarchy tokens → FrankMD UI tokens

FrankMD's token set (from `app/services/custom_theme.rb` `COLOR_KEYS`, 38 keys). Today these are derived from **`alacritty.toml`** in `app/services/omarchy_theme_service.rb`. "Today (alacritty)" shows what that derivation yields after going through `default/themed/alacritty.toml.tpl`.

| FrankMD token | Proposed source (Omarchy) | Today (alacritty) | Notes |
|---|---|---|---|
| `bg-primary` | `background` | primary.background | same |
| `bg-secondary` (sidebar, panels) | `dark_background` | bg ±3% | matches the T3 Code template `sidebar` |
| `bg-tertiary` (inputs, cards, menus) | `lighter_background` | bg ±7% | raised surface |
| `bg-hover` | `mix(background, foreground, 8%)` | bg ±12% | T3 Code uses 6% on `dark_background` |
| `text-primary` | `foreground` | foreground | |
| `text-secondary` | `mix(foreground, background, 8%)` | same blend | `light_foreground` is not monotonic across themes (Latte) |
| `text-muted` | `dark_foreground` | blend 45% | Omarchy's own "dim text" stop |
| `text-faint` | `mix(dark_foreground, background, 35%)` | blend 65% | |
| `border` | `muted` | bg ±12% | what Omarchy templates use (`obsidian.css.tpl`, `t3code.json.tpl`) |
| `accent` | `accent` | normal.blue (**wrong**: alacritty has no accent) | the biggest visible fix |
| `accent-hover` | `mix(accent, foreground, 15%)` | bright.blue | |
| `accent-text` | black/white by WCAG luminance of `accent` (app skill `accentForeground`), or `background` if contrast ≥ 4.5 | background | |
| `selection` | `selection_background` (= `selection`) | selection.background | |
| `selection-text` | `selection_foreground` (= `bright_foreground`) | selection.text | |
| `scrollbar` | `muted` | normal.black (= **background → invisible**) | current bug |
| `scrollbar-hover` | `dark_foreground` | lighten(black) | T3 Code mapping |
| `success` | `green` | normal.green | |
| `warning` | `yellow` | normal.yellow | |
| `error` | `red` | normal.red | Omarchy's urgent role |
| `code-bg` | `dark_background` | bg ±7% | T3 Code `codeBackground` |
| `heading-1` | `magenta` | normal.magenta | keeps FrankMD's look (Obsidian template uses red/green/yellow instead) |
| `heading-2` | `blue` | normal.blue | |
| `heading-3` | `cyan` | normal.cyan | |
| `folder-icon` | `accent` | normal.blue | |
| `file-icon` | `dark_foreground` | dim fallback | |
| `config-icon` | `yellow` | normal.yellow | |
| `syntax-keyword` | `magenta` | normal.magenta | |
| `syntax-function` | `yellow` | normal.yellow | |
| `syntax-type` | `cyan` | normal.cyan | |
| `syntax-string` | `green` | normal.green | |
| `syntax-number` | `orange` | bright.yellow | Omarchy has a real `orange` |
| `syntax-constant` | `bright_cyan` | bright.cyan | |
| `syntax-identifier` | `foreground` | foreground | |
| `syntax-property` | `bright_blue` | bright.blue | |
| `syntax-operator` | `foreground` | normal.white (= foreground) | |
| `syntax-comment` | `muted` (fallback `dark_foreground`) | blend | Omarchy defines `muted` as "comments, placeholders" |
| `syntax-punctuation` | `dark_foreground` | foreground | |
| `syntax-invalid` | `red` | normal.red | |

Rules to keep from FrankMD:

- Run every `syntax-*` and text token through FrankMD's `ensure_contrast(color, code_bg, 4.5)` binary search (`omarchy_theme_service.rb` L148–166). Existing tests `test/services/omarchy_theme_service_test.rb` pin that. Port them as Rust golden tests over all 23 stock `colors.toml` files, copied as fixtures.
- `is_dark` comes from §2.3 (the `mode` key), not from luminance. Luminance stays the last fallback.
- Do the mixing in sRGB linear-per-channel to match `omarchy-theme-color mix` (not OKLab), so any overlay template renders the same values.

Also follow:

- **Corner radius:** `hyprctl -j getoption decoration:rounding` → `{"int": N}`. The default is 0 and `corners-rounded` sets 6. The app skill polls every 1 s with a 500 ms timeout. Polling on theme change and on focus-in would be cheaper. Use it for panels, dialogs and buttons, and cap small elements at `min(4, radius)`.
- **Icon theme:** `current/theme/icons.theme` (or gsettings `icon-theme`) if we use freedesktop icons.
- **Without Omarchy:** fall back to FrankMD's built-in themes, with the portal color-scheme choosing between light and dark. The app skill's fallback accent is `#FFD60A`.

### 2.7 What breaks in FrankMD today and what changes

- `OmarchyThemeService::OMARCHY_DIR = "~/.config/omarchy/current"` points at a path that no longer exists on Omarchy 4.x (moved to `~/.local/state/omarchy/current`, migration `1781043107.sh`). **The Omarchy sync is broken on any current install.**
- It reads `theme/alacritty.toml`, which loses `accent`, `muted`, `selection`, `orange` and the bg/fg ramp. `normal.black = background`, so the scrollbar becomes invisible. `accent` falls back to blue.
- In Docker mode (`fed` and `docker-compose.yml`) the container mounts only notes and images, never the Omarchy state dir, so sync could never work there.
- The JS polls `/config/omarchy_theme` and only reacts when `theme_name` changes (`theme_controller.js` L281–300).
- The Rust app should replace all of this with: native inotify on the state dir, a `colors.toml` parser with the cascade, the mapping above, and a "Follow Omarchy" theme entry. That entry should be the **default** when Omarchy is detected (`$OMARCHY_PATH` set or `~/.local/state/omarchy/current/theme/colors.toml` exists). FrankMD currently treats it as an opt-in "Omarchy" entry in the theme list. Keep the ID `omarchy` so `.fed` files with `theme = omarchy` keep working.

---

## 3. Fonts

- **Source of truth is fontconfig** (`bin/omarchy-font-set` L52–56 comment). `omarchy font set <name>` writes `~/.config/fontconfig/fonts.conf` with a `prepend_first`/strong edit on the `monospace` alias. It also rewrites the alacritty, kitty, ghostty and foot configs, restarts the shell, and runs the `font-set` hook (font name in `$1`). `omarchy font current` = `fc-match monospace -f '%{family}'`. `omarchy font list` = `fc-list :spacing=100`.
- Defaults from `default/fontconfig/conf.avail/50-omarchy.conf`: `monospace` → **JetBrainsMono Nerd Font**. `sans-serif` and `Liberation Sans` *accept* JetBrainsMono Nerd Font (weak). Live: `fc-match sans-serif` → Liberation Sans. Emoji → Noto Color Emoji. CJK goes to Noto Sans/Mono CJK {JP,KR,SC,TC,HK}, chosen by language tag, by the session `LANG`, and by `~/.config/fontconfig/conf.d/50-omarchy-input-method.conf` (written by `omarchy font cjk`).
- The shell UI itself uses `monospace` at `[font] base-size = 12` (`docs/omarchy-shell.md` "Typography"). Omarchy doesn't sync gsettings `font-name` (still `Adwaita Sans 11` here), so don't read GTK font settings on Omarchy.
- **For FrankMD:**
  - The editor font defaults to a "System monospace" choice that resolves `monospace` through fontconfig (via `fontconfig`/`fontdb`, or `fc-match` at startup). User choice wins: `.fed editor_font` or global config.
  - Keep the bundled fonts (Cascadia Code etc.) as explicit options.
  - Keep CJK fallback by passing the text's language to the shaper (cosmic-text/swash/Pango all support a language hint). Fontconfig's lang-tagged rules then pick the right Noto CJK variant. This matters for ja/ko notes.
  - Re-resolve live by watching `~/.config/fontconfig/` (fonts.conf and conf.d/) with a debounce. The `font-set` hook is the late, optional alternative.
  - UI font: the Omarchy look is monospace UI chrome. Default UI font on Omarchy = `monospace` alias, elsewhere `sans-serif`. Offer it as a setting.

---

## 4. Languages / i18n

### 4.1 How Omarchy 4.5 does it

- The user choice is `~/.config/omarchy/language`: one registry code on the first line. A missing or unknown code means `en` (`bin/omarchy-language` `current_language`; the app skill's `Translator::readLanguage` validates `^[a-z]{2,3}(-[A-Za-z]{2})?$`).
- `omarchy language <code>` generates the glibc locale if needed (sudo or pkexec), writes the file, seeds the input method for ja/ko/zh-CN/zh-TW, writes `export LANG=<glibc>` to `~/.config/uwsm/env.d/50-omarchy-language`, runs `hyprctl eval 'hl.env("LANG", …, true)'` (pushing it into the systemd and D-Bus activation env), and restarts the shell. Apps already running keep their old `LANG` until restarted (`docs/i18n.md` L396). A later user `LANG`, `LC_ALL`, `LC_MESSAGES` or `LANGUAGE` still wins.
- The registry is `$OMARCHY_PATH/default/i18n/locales.json`: code → `{name, englishName, formatLocale (BCP47), locale (glibc), decimal, zeroDigit, direction?}`.
- 50 codes: `ar bg bn ca cs da de el en es-MX et fa fi fil fr ga he hi hr hu id is it ja ko lt lv ms nl no pl pt pt-BR ro ru si sk sl sr sv sw ta th tr uk ur uz vi zh-CN zh-TW`. RTL: `ar he fa ur`.
- **Code quirks that affect FrankMD:**
  - `pt` = **European Portuguese** (formatLocale `pt-PT`, `pt_PT.UTF-8`) and `pt-BR` = Brazilian.
  - There is **no `es`**, only `es-MX` (formatLocale `es-MX`).
  - `he` is RTL with formatLocale `he-IL`.
- App guidance (the app skill, "Speak the user's language"):
  - Follow `~/.config/omarchy/language` **live**, with a watch on the file and its directory that re-arms on every change.
  - Fall back per string: region → base → English (`pt-BR` → `pt` → `en`).
  - Set number/date locale from `formatLocale`, and layout direction from `direction`.
  - Use CLDR plural categories (rules in `Translator::pluralCategory`, mirroring `shell/Commons/Plural.js`).
  - Translate the `.desktop` file's `Name[xx]`/`Comment[xx]`, with region underscores (`Name[pt_BR]`).
  - Keep brand names, units and key names untranslated.

### 4.2 Mapping FrankMD's 7 locales

| FrankMD (`config/application.rb` L45) | Omarchy code | Note |
|---|---|---|
| `en` | `en` | |
| `pt-BR` | `pt-BR` | |
| `pt-PT` | `pt` | Omarchy's `pt` *is* pt-PT. Map `pt` → `pt-PT`, never → `pt-BR` |
| `es` | `es-MX` | FrankMD `es` is generic Spanish. Map `es-MX` (and any `es*` from `LANG`) → `es` |
| `he` | `he` | RTL: FrankMD's CSS already handles `dir=rtl`. The Rust UI needs mirrored layout too |
| `ja` | `ja` | |
| `ko` | `ko` | |

Any other Omarchy language (e.g. `de`, `da`) → FrankMD falls back to `en`. Log it once.

### 4.3 Proposed precedence (Rust app)

Today in FrankMD: `params[:locale]` > `.fed locale` > `FRANKMD_LOCALE` env > `en` (`app/controllers/application_controller.rb` L14–19, `app/models/config.rb` L16; `Config` priority is `.fed` > ENV > default).

Proposed, first match wins:

1. CLI flag `--locale <code>` (debug and testing; replaces `params[:locale]`)
2. `FRANKMD_LOCALE` env (explicit per-launch override; also lets tests pin it)
3. `.fed` `locale` in the open notes folder (per-workspace, existing behaviour; only when explicitly set, not the commented default)
4. Global app config `~/.config/frankmd/config.toml` `locale` (new; written by the Settings language picker when "Follow system" is off)
5. **Omarchy language** `~/.config/omarchy/language`, mapped by §4.2, **live-watched**
6. POSIX: `LC_ALL` > `LC_MESSAGES` > `LANG` (first match only; parse `pt_BR.UTF-8` → `pt-BR`, `pt_PT` → `pt-PT`, `es_*` → `es`), then `LANGUAGE` list
7. `en`

Why `FRANKMD_LOCALE` moves above `.fed`: today ENV is just the default for `.fed`. On a desktop, an env var is a deliberate per-launch override, and `.fed` is shared per folder. If keeping exact FrankMD parity matters more, swap 2 and 3 and say so in the migration notes. Either way the Settings picker needs a **"System (Omarchy/LANG)"** option, and it should be the default. When the user picks a language there, save it to global config (4), not to `.fed`, unless they choose "for this folder only".

Steps 5 and 6 should agree on Omarchy, since `LANG` is set from the same choice. Step 5 still has to come first because it changes live, while `LANG` only changes on restart. Number and date formatting should use the Omarchy `formatLocale` when step 5 decided the language.

---

## 5. Input methods (CJK typing)

- Omarchy runs **fcitx5** as the systemd user unit `omarchy-fcitx5.service` (D-Bus activated `org.fcitx.Fcitx5`). Engines: Mozc (ja), Hangul (ko), Pinyin, Chewing, Cangjie (`default/input-methods/presets.json`). Super+I cycles layouts and IMs. Ctrl+Space is reserved and never an IME toggle (`docs/input-methods.md`).
- Session env (`default/environment.d/10-omarchy-fcitx.conf`) sets `INPUT_METHOD=fcitx`, `QT_IM_MODULE=fcitx`, `XMODIFIERS=@im=fcitx` and `SDL_IM_MODULE=fcitx`. It leaves **`GTK_IM_MODULE` unset**, so GTK4 and other Wayland-native clients use the compositor protocol (`zwp_text_input_v3`) that Hyprland relays to fcitx5's Wayland frontend.
- Hyprland env forces Wayland: `GDK_BACKEND=wayland,x11,*`, `QT_QPA_PLATFORM=wayland;xcb` (`default/hypr/envs.lua`).

Requirements for a Wayland-native Rust app:

1. Use **text-input-v3** through the toolkit: winit `Window::set_ime_allowed(true)` with `WindowEvent::Ime{Preedit,Commit,Enabled,Disabled}`; GTK4 IMContext; Slint/iced through winit. Don't depend on XIM (`XMODIFIERS` only helps XWayland).
2. Render **preedit text inline** at the caret with underline and the preedit cursor range, and don't commit until `Ime::Commit`. Key events that arrive during composition belong to the IME: don't treat Enter, Backspace or arrow keys as editor commands while preedit is non-empty. This matters for vim mode too, where IME should only be enabled in insert mode.
3. Call `set_ime_cursor_area()` on every caret move and scroll, so the themed fcitx candidate window appears next to the caret.
4. Disable IME in password and secret fields (fcitx "steps aside in password fields") and in vim normal mode.
5. When shaping text, pass the content's language tag (or the UI language) so CJK uses the right Noto variant (§3).
6. Test on the shipped stack: the doc says browser and app input "must be verified against the shipped versions". Omarchy's VM acceptance test `test/acceptance.d/input-methods-test.sh` shows a way to automate it.

---

## 6. Desktop integration

### 6.1 Identity: app id, desktop file, window class

The app skill says one short lowercase name is "the binary, the package, the `.desktop` file, the icon, and the Wayland app id all at once."

- Name: **`frankmd`**. Binary `/usr/bin/frankmd`, `/usr/share/applications/frankmd.desktop`, icon `/usr/share/icons/hicolor/scalable/apps/frankmd.svg`, Wayland `app_id` = `frankmd` (winit `WindowAttributesExtWayland::with_name("frankmd", "frankmd")`; GTK `application_id` must then also be `frankmd`, or set the desktop file name). Omarchy's own apps use plain names (`hype`, `omacut`, `org.omarchy.*` only for Omarchy-internal windows), so prefer `frankmd` over reverse-DNS `com.akitaonrails.FrankMD`.
- `.desktop` (template from `templates.md`, extended):

  ```ini
  [Desktop Entry]
  Type=Application
  Name=FrankMD
  GenericName=Markdown Notes
  Comment=Fast Markdown notes and blog editor
  Exec=frankmd open %U
  Icon=frankmd
  Terminal=false
  Categories=Office;TextEditor;Utility;
  MimeType=text/markdown;text/x-markdown;
  Keywords=markdown;notes;hugo;blog;
  StartupWMClass=frankmd
  StartupNotify=true
  Name[pt_BR]=FrankMD
  Comment[pt_BR]=…
  Comment[pt_PT]=… Comment[es]=… Comment[he]=… Comment[ja]=… Comment[ko]=…
  ```

  `StartupWMClass` matters because Omarchy's titlebars, launcher and notification click-through all match by class (`omarchy-hyprland-focus-app`). Omarchy doesn't claim a `text/markdown` default (`default/applications/mimeapps.list` only sets `text/plain=nvim.desktop`), so declaring `MimeType` makes FrankMD selectable without taking over anything.
- Hyprland window rules match `class` = app_id. Users write them in `~/.config/hypr/hyprland.lua` with `o.window("frankmd", {...})` (`default/hypr/helpers.lua` L279). Transparency is opt-in: `o.transparent_window("frankmd")`, or the user's transparency hotkey adds the `transparent` tag (`default/hypr/windows.lua`, `54e3b53b8`). For that to look right, keep the window opaque by default and make the surface alpha-capable, so Glassy mode can show blur through `background` (optional).
- **Migration from `fed`:** the current window is a Chromium `--app=file://…/splash.html` window, whose class follows the `chrome-<host>_<path>-Default` scheme (`bin/omarchy-webapp-window-class`). Any user rules or bindings on that class need updating. Note it in release notes.
- Dialogs: use **xdg-desktop-portal FileChooser** (the app skill says "never `QFileDialog`"; for Rust use the `ashpd` or `rfd` portal backend). The portal window is already floated by `default/hypr/apps/system.lua`.

### 6.2 Launcher and menu

- A native app shows up in the Super+Space launcher automatically once its `.desktop` file is installed (menu provider `apps` reads desktop entries; `docs/menu.md` "Providers"). That replaces the `fed` shell function plus browser app-window. Desktop `Keywords` are searchable but never routable.
- Webapp vs native: `omarchy-launch-webapp`/`omarchy-webapp-install` produce Chromium `--app` windows with `StartupWMClass=chrome-…`. FrankMDFast is a native package, so none of that applies.
- Optional menu entry (documented, user-installed; never written by the app): add to `~/.config/omarchy/extensions/omarchy-menu.jsonc`, e.g. `"personal.notes": {"icon":"󰎞","label":"Notes","action":"frankmd open ~/notes"}`. Remember that only whole-line `//` comments are allowed, and that a parse error drops every user entry (`docs/menu.md` "Load and merge").
- Upstream path (later): getting FrankMD into Omarchy's Install menu or base packages means first publishing in `omacom/omarchy-pkgs` (app skill "README and release"), then a PR touching `install/omarchy-base.packages` and the menu JSONC with `disabled:` presence guards (`docs/menu.md` "Guards").

### 6.3 Keybindings

- Global launch bindings belong to the user: `~/.config/hypr/bindings.lua`, `o.bind("SUPER + SHIFT + <key>", "FrankMD", { launch = "frankmd", focus = "^frankmd$" })`. This mirrors the Obsidian binding in `default/hypr/bindings/applications.lua`. Already taken: Super+Shift+O (Obsidian), Super+Shift+W (Omawrite), Super+Shift+N (Editor). Check `omarchy menu keybindings --print` before suggesting one. Use `o.rebind` to replace.
- In-app conventions from the app skill: `?` shows a shortcut overlay, `Q` quits (asking first if there is unsaved work), Ctrl+Z / Ctrl+Shift+Z undo and redo, and the README lists shortcuts in a table. A text editor can't use bare `?`/`Q`/Space while typing. Bind them only outside text focus (or in vim normal mode), and use Ctrl+/ or F1 for the overlay.
- Don't bind Super+\* inside the app (the compositor owns them). Don't use Ctrl+Space for anything IME-related (reserved for tmux/herdr, and never an IME toggle here).

### 6.4 Notifications

- The shell owns `org.freedesktop.Notifications`. Send standard freedesktop notifications (`notify-rust` / `zbus`):
  - `app_name = "FrankMD"`
  - hint `desktop-entry = "frankmd"`, used for the icon fallback since `329fd1a9b`
  - `app_icon = "frankmd"`
  - urgency `low`/`normal` (critical stays until dismissed)
  - hint `transient` for ephemeral toasts ("Saved", "Copied")
- Click-to-focus works without registering actions: the fallback chain focuses the window by pid, then by class matching `app_name` (`docs/notifications.md` "Toast lifecycle"). A `default` action is honoured while the sender is alive. The `omarchy-exec-argv` hint (a JSON argv array) is Omarchy-specific. It could deep-link (e.g. `["frankmd","open","--note","<path>"]`), but treat it as optional.
- Never put secrets in bodies. The shell persists history (`Store.js`, 16K/4K caps). Verification-code detection redacts codes, but nothing else is redacted.
- Use `omarchy-notification-send` only in shell scripts. It's Omarchy-internal and defaults to `app_name=omarchy-action`, which bypasses DND, and that's wrong for an app.

### 6.5 XDG directories and config locations

`docs/file-layout.md` "Mental model": `~/.config/<app>` is for files a user may keep in dotfiles, and generated state goes under `~/.local/state`. Omarchy follows XDG (`${XDG_CONFIG_HOME:-$HOME/.config}` in `bin/omarchy-language`).

| Purpose | Location |
|---|---|
| Global user config (defaults for all folders: theme, locale, fonts, AI and S3 keys if the user prefers global) | **`$XDG_CONFIG_HOME/frankmd/config.toml`** (`~/.config/frankmd/` already exists and is used by `fed` for `splash.html`) |
| Per-folder config | `<notes>/.fed` (unchanged format; precedence `.fed` > global > env defaults, except locale per §4.3) |
| Recent folders, window geometry, session restore | `$XDG_STATE_HOME/frankmd/` (`~/.local/state/frankmd/`) |
| Caches (thumbnails, search index, rendered previews) | `$XDG_CACHE_HOME/frankmd/` |
| Optional theme overrides / user themes | `$XDG_CONFIG_HOME/frankmd/themes/*.toml` |
| Secrets | Secret portal / libsecret (`hyprland-portals.conf` routes Secret to gnome-keyring), with a plain config key as a fallback |

Never write into `~/.config/omarchy/`, `~/.local/state/omarchy/` or `~/.config/hypr/`. Read only.

### 6.6 Packaging

- Arch package `frankmd` with a PKGBUILD shaped like the template's (binary, LICENSE, hicolor SVG icon, `.desktop`). Use `depends=('xdg-desktop-portal' …)`, with `cargo` in makedepends. Publish as an AUR `frankmd` (source) and/or `frankmd-bin`. Omarchy's `omarchy-pkg-add` installs from repos and AUR.
- Getting into Omarchy proper: tag `v<version>` with a GitHub release, then a PR to `omacom/omarchy-pkgs` at `pkgbuilds/frankmd/` with `.omarchy/package.json` `{"source":"local","auto_merge":true, upstream.watch …}`. Only after that comes `install/omarchy-base.packages` plus a migration (app skill "README and release").
- `/usr/share/frankmd/contrib/omarchy/`: sample `frankmd.toml.tpl`, sample `theme-set` hook, sample menu snippet, and a Hyprland binding snippet. Install them as documentation only.

### 6.7 `omarchy`-style CLI surface that an app can offer

Omarchy's router only dispatches `bin/omarchy-*` files in `$OMARCHY_PATH/bin` (`docs/cli-router.md`). A third-party app **shouldn't** install `omarchy-*` binaries, since the namespace is Omarchy's and collisions are linted. What the app skill recommends instead is an agent-friendly CLI on the app's own binary ("Agents can drive it", modelled on Hype's `src/cli.cpp`):

- `frankmd open [path]` launches the GUI (the GUI sits behind `open`).
- Headless commands with `--json`, exit 0/1, errors on stderr, no display needed: e.g. `frankmd check <file>` (lint links and front matter), `frankmd render <file> --html`, `frankmd export`, `frankmd new-post --hugo`, `frankmd search <query>`.
- `frankmd reload-theme`, `frankmd reload-config` to ping a running instance over a Unix socket or D-Bus (this is what a user's `theme-set` or `font-set` hook would call).
- `frankmd skill install` installs a short bundled agent skill.

---

## 7. Integration requirements (prioritized)

**MUST** means needed for an "omarchy-friendly" 1.0. **SHOULD** means strongly recommended. **NICE** means optional.

| ID | Pri | Requirement | Source |
|---|---|---|---|
| OM-001 | MUST | Read the active palette from `~/.local/state/omarchy/current/theme/colors.toml` (not `~/.config/omarchy/current`, not `alacritty.toml`). Implement the full alias/derivation cascade of `omarchy-theme-color`, with sRGB `mix` | §2.1–2.2; `bin/omarchy-theme-color` |
| OM-002 | MUST | Light/dark from `mode` → `theme_type` → `light.mode` → luminance (R+G+B>382) → dark | §2.3 |
| OM-003 | MUST | Live theme follow: inotify on `~/.local/state/omarchy/current/` (and the nearest existing ancestor if missing), debounced 80–150 ms, re-armed after each event, re-reading the palette and comparing content rather than the name | §2.4; app skill `Theme::reload` |
| OM-004 | MUST | Map Omarchy tokens to FrankMD's 38 UI tokens per §2.6, with WCAG contrast correction (≥4.5:1 against `code-bg`) on syntax and text tokens. Golden tests over all 23 stock themes | §2.6; FrankMD `omarchy_theme_service.rb` |
| OM-005 | MUST | "Follow Omarchy" theme (id `omarchy`) is the default when Omarchy is detected. Built-in themes stay selectable. Fallback when absent: portal `color-scheme` picks light or dark | §2.3, §2.7 |
| OM-006 | MUST | Wayland-native window with app_id `frankmd` matching `frankmd.desktop` (`StartupWMClass=frankmd`) and icon `frankmd` | §6.1 |
| OM-007 | MUST | IME via text-input-v3: inline preedit, cursor-area updates, commit handling, no editor shortcuts during composition, IME disabled in secret fields and vim normal mode. Verified with fcitx5 Mozc and Hangul | §5 |
| OM-008 | MUST | Locale resolution per §4.3, including Omarchy `language` → FrankMD mapping (`pt`→`pt-PT`, `es-MX`→`es`), live-watched. Keep `FRANKMD_LOCALE` and `.fed locale` | §4 |
| OM-009 | MUST | RTL layout mirroring for `he` (and any future RTL locale), driven by the locale. Editor text direction is per paragraph (bidi) | §4.1; `docs/i18n.md` "Plurals, numbers, and direction" |
| OM-010 | MUST | XDG layout: global config `~/.config/frankmd/config.toml`, state `~/.local/state/frankmd/`, cache `~/.cache/frankmd/`. Never write into Omarchy or Hypr config | §6.5; app skill "Never change Omarchy's config" |
| OM-011 | MUST | File open/save via xdg-desktop-portal FileChooser | §6.1; app skill |
| OM-012 | MUST | Editor font default "System monospace" resolved via fontconfig `monospace`, with per-script language hints so CJK picks the right Noto variant | §3 |
| OM-013 | MUST | Arch PKGBUILD installing binary, `.desktop` (translated `Name`/`Comment` for the 7 locales) and a hicolor SVG icon, so the app appears in the Super+Space launcher | §6.2, §6.6 |
| OM-014 | SHOULD | Freedesktop notifications with `app_name=FrankMD`, `desktop-entry=frankmd`, `transient` for ephemeral toasts, low/normal urgency. Click focuses the window | §6.4 |
| OM-015 | SHOULD | Corner radius follows `hyprctl -j getoption decoration:rounding` (re-query on theme change and on focus), capped at 4 for small widgets | §2.6; app skill |
| OM-016 | SHOULD | Live font follow: watch `~/.config/fontconfig/` and re-resolve `monospace`. UI-font default on Omarchy is monospace | §3 |
| OM-017 | SHOULD | Optional theme override file `current/theme/frankmd.toml` (from a theme or user template) beats derived tokens. Ship sample `contrib/omarchy/frankmd.toml.tpl` | §2.5 |
| OM-018 | SHOULD | Single-instance plus IPC (`frankmd reload-theme`, `reload-config`, `open <path>` focusing the existing window) for hooks and launcher re-invocation | §6.7 |
| OM-019 | SHOULD | Use Omarchy `formatLocale` (from `$OMARCHY_PATH/default/i18n/locales.json`) for dates and numbers when the language came from Omarchy. CLDR plurals in catalogs | §4.1 |
| OM-020 | SHOULD | Keyboard-first: shortcut overlay (Ctrl+/ or F1; `?` outside text focus), quit with an unsaved-work prompt, standard undo/redo, a shortcut table in the README. No Super+\* or Ctrl+Space bindings | §6.3 |
| OM-021 | SHOULD | Agent-friendly headless CLI (`check`/`render`/`export`/`search --json`, exit codes, `skill install`). GUI behind `frankmd open` | §6.7; app skill |
| OM-022 | SHOULD | `MimeType=text/markdown;text/x-markdown;` without claiming defaults | §6.1 |
| OM-023 | NICE | Portal `SettingChanged` (`color-scheme`, `accent-color`) as the theme source on non-Omarchy desktops | §2.3 |
| OM-024 | NICE | Alpha-capable surface so Omarchy's opt-in Glassy transparency (`o.transparent_window("frankmd")`) blurs properly | §6.1 |
| OM-025 | NICE | `contrib/omarchy/`: sample `theme-set` and `font-set` hooks calling `frankmd reload-*`, a menu JSONC snippet, and a `bindings.lua` snippet. Installed as docs, never activated by the app | §2.4, §6.2, §6.3 |
| OM-026 | NICE | `omarchy-exec-argv` hint for deep-linking notifications to a note | §6.4 |
| OM-027 | NICE | Release-notes migration from `fed`: new window class, `~/.config/frankmd/`, Omarchy sync now works natively (it was broken by the path move and Docker isolation) | §2.7, §6.1 |
| OM-028 | NICE | Upstream path: AUR → `omacom/omarchy-pkgs` → propose a `default/themed/frankmd.toml.tpl` plus `omarchy-theme-set-frankmd` in Omarchy's retint list | §2.5, §6.6 |

---

## Appendix: key upstream files

- `docs/theming.md`, `docs/i18n.md`, `docs/input-methods.md`, `docs/notifications.md`, `docs/menu.md`, `docs/cli-router.md`, `docs/omarchy-shell.md`, `docs/file-layout.md`
- `default/agents/skills/omarchy-app/SKILL.md`, `templates.md` (Theme and Translator reference implementations, desktop file, PKGBUILD)
- `default/agents/skills/omarchy/{hooks,theming,hyprland}.md`
- `bin/omarchy-theme-set`, `bin/omarchy-theme-color`, `bin/omarchy-theme-set-templates`, `bin/omarchy-theme-set-gnome`, `bin/omarchy-theme-set-t3code` (atomic-write example), `bin/omarchy-font-set`, `bin/omarchy-font-current`, `bin/omarchy-language`, `bin/omarchy-hook`, `bin/omarchy-hyprland-focus-app`, `bin/omarchy-webapp-window-class`
- `default/themed/*.tpl` (`t3code.json.tpl` and `obsidian.css.tpl` are the closest analogues to our token map)
- `default/i18n/locales.json`, `default/environment.d/10-omarchy-fcitx.conf`, `default/hypr/envs.lua`, `default/hypr/windows.lua`, `default/hypr/apps/system.lua`, `default/hypr/bindings/applications.lua`, `default/fontconfig/conf.avail/50-omarchy.conf`, `config/xdg-desktop-portal/hyprland-portals.conf`
- FrankMD: `app/services/omarchy_theme_service.rb`, `app/services/custom_theme.rb`, `app/javascript/controllers/theme_controller.js`, `app/controllers/application_controller.rb`, `app/models/config.rb`, `config/application.rb`, `docker-compose.yml`, the `fed` shell function
