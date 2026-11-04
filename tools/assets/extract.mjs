// Extracts reusable assets from a FrankMD checkout into assets/.
// Usage: node tools/assets/extract.mjs [~/Projects/FrankMD]
import fs from "node:fs"
import path from "node:path"
import os from "node:os"
import { pathToFileURL, fileURLToPath } from "node:url"
import { execSync } from "node:child_process"

const src = path.resolve((process.argv[2] || "~/Projects/FrankMD").replace(/^~/, os.homedir()))
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..")
const out = path.join(root, "assets")
const sha = execSync("git rev-parse HEAD", { cwd: src }).toString().trim()
const mk = (d) => fs.mkdirSync(path.join(out, d), { recursive: true })
const write = (rel, data) => fs.writeFileSync(path.join(out, rel), JSON.stringify(data, null, 1) + "\n")
const copy = (from, to) => { fs.mkdirSync(path.dirname(path.join(out, to)), { recursive: true }); fs.copyFileSync(path.join(src, from), path.join(out, to)) }
const meta = (file) => ({ source: `${file} @ ${sha}` })

// Brand: logo + icons
for (const f of ["icon.svg", "icon.png", "icon-192.png", "icon-512.png", "apple-touch-icon.png", "favicon-16x16.png", "favicon-32x32.png"])
  copy(`public/${f}`, `brand/${f}`)

// Locales (Rails YAML, consumed as-is by the i18n crate)
for (const f of fs.readdirSync(path.join(src, "config/locales"))) copy(`config/locales/${f}`, `locales/${f}`)

// Emoji names per language
for (const f of fs.readdirSync(path.join(src, "public/emoji-data"))) copy(`public/emoji-data/${f}`, `emoji/names/${f}`)

// Emoji shortcodes, Phosphor icon paths, custom-theme keys
const lib = (f) => import(pathToFileURL(path.join(src, "app/javascript/lib", f)).href)
mk("emoji"); mk("icons"); mk("themes")
const { EMOJI_DATA } = await lib("emoji_data.js")
write("emoji/shortcodes.json", { ...meta("app/javascript/lib/emoji_data.js"), format: "[shortcode, emoji, keywords]", data: EMOJI_DATA })
const { ICON_DATA } = await lib("icon_data.js")
write("icons/phosphor.json", { ...meta("app/javascript/lib/icon_data.js"), license: "MIT (phosphor-icons/core)", format: "[name, svgPathData, viewBox, keywords]", data: ICON_DATA })

// Emoticons: module-private array in the picker controller; evaluate just that literal
const pickerFile = "app/javascript/controllers/emoji_picker_controller.js"
const picker = fs.readFileSync(path.join(src, pickerFile), "utf8")
const start = picker.indexOf("const EMOTICON_DATA = [")
const lit = picker.slice(picker.indexOf("[", start))
let depth = 0, end = 0
for (let i = 0, q = null; i < lit.length; i++) {
  const c = lit[i]
  if (q) { if (c === "\\") i++; else if (c === q) q = null; continue }
  if (c === '"' || c === "'" || c === "`") q = c
  else if (c === "[") depth++
  else if (c === "]" && --depth === 0) { end = i + 1; break }
}
write("emoji/emoticons.json", { ...meta(pickerFile), data: new Function(`return ${lit.slice(0, end)}`)() })

// Built-in themes: CSS custom properties -> JSON
const themeDir = "app/assets/tailwind/themes"
const themes = {}
for (const f of fs.readdirSync(path.join(src, themeDir)).sort()) {
  const css = fs.readFileSync(path.join(src, themeDir, f), "utf8")
  const id = (css.match(/data-theme="([^"]+)"/) || [, path.basename(f, ".css")])[1]
  const vars = {}
  for (const [, k, v] of css.matchAll(/--theme-([a-z0-9-]+)\s*:\s*([^;]+);/g)) vars[k] = v.trim()
  themes[id] = { description: (css.match(/\/\*\s*(.*?)\s*\*\//) || [])[1] || null, colors: vars }
}
write("themes/builtin.json", { ...meta(themeDir), themes })

// Fonts (latin-subset woff2, kept as reference; app prefers system/fontconfig fonts)
for (const f of fs.readdirSync(path.join(src, "public/fonts"))) if (f.endsWith(".woff2")) copy(`public/fonts/${f}`, `fonts/web/${f}`)
for (const f of fs.readdirSync(path.join(src, "public/fonts/katex"))) copy(`public/fonts/katex/${f}`, `fonts/katex/${f}`)

console.log(`extracted from ${src} @ ${sha} -> ${out}`)
