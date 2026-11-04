// Golden fixtures for FrankMD's UI data/config libs: keyboard_shortcuts, custom_theme
// (+ built-in theme syntax palettes), emoji_data, emoji_i18n, icon_data,
// codemirror_theme, codemirror_wikilink. Uses vendored CodeMirror via importmap emulation.
//
// usage: node tools/golden/js/ui_data.mjs   (harvested inputs: harvest/inputs/render.jsonl, see render.mjs)
import "./runtime/setup.mjs"
import { readFileSync, readdirSync } from "node:fs"
import { join } from "node:path"
import { FRANKMD, fixture, writeFixture } from "../lib/golden.mjs"
import { harvested, cap, capAsync, enc, dedupe, EDGE } from "./runtime/common.mjs"

const GEN = "tools/golden/js/ui_data.mjs"
const H = harvested("render", [])

// ================================================================ keyboard_shortcuts
{
  const ks = await import("lib/keyboard_shortcuts")
  const cases = []
  cases.push({ name: "DEFAULT_SHORTCUTS dump (ordered)", fn: "DEFAULT_SHORTCUTS", input: [], expected: enc(Object.entries(ks.DEFAULT_SHORTCUTS)) })
  cases.push({ name: "SHORTCUTS === DEFAULT_SHORTCUTS", fn: "SHORTCUTS", input: [], expected: ks.SHORTCUTS === ks.DEFAULT_SHORTCUTS })
  for (const fn of ["matchShortcut", "formatShortcut", "findMatchingAction", "mergeShortcuts", "getShortcutForAction"])
    for (const args of H("keyboard_shortcuts", fn)) cases.push({ name: `harvested ${fn} ${JSON.stringify(enc(args)).slice(0, 60)}`, fn, input: enc(args), expected: cap(() => ks[fn](...args)) })
  for (const [action, sc] of Object.entries(ks.DEFAULT_SHORTCUTS)) for (const mac of [false, true])
    cases.push({ name: `formatShortcut ${action} mac=${mac}`, fn: "formatShortcut", input: [sc, mac], expected: cap(() => ks.formatShortcut(sc, mac)) })
  for (const sc of [{ key: "a", alt: true }, { key: "a", ctrl: true, shift: true, alt: true }, { key: "F12" }, { key: "F13" }, { key: "Enter", ctrl: true }, { key: "ç" }, { key: "\\" }, { key: "ArrowUp", alt: true }, { key: "" }, null, {}, { ctrl: true }, { key: "+", ctrl: true, shift: true }])
    for (const mac of [false, true]) cases.push({ name: `formatShortcut ${JSON.stringify(sc)} mac=${mac}`, fn: "formatShortcut", input: enc([sc, mac]), expected: cap(() => ks.formatShortcut(sc, mac)) })
  // full event matrix against the default keymap
  const keys = [...new Set(Object.values(ks.DEFAULT_SHORTCUTS).flatMap(s => [s.key, s.key.toLowerCase(), s.key.toUpperCase()]))].concat(["b", "i", "-", "=", "|", "Tab"])
  for (const key of keys) for (let m = 0; m < 16; m++) {
    const ev = { key, ctrlKey: !!(m & 1), shiftKey: !!(m & 2), altKey: !!(m & 4), metaKey: !!(m & 8) }
    cases.push({ name: `findMatchingAction default ${JSON.stringify(key)} m=${m}`, fn: "findMatchingAction", input: [ev, "<DEFAULT_SHORTCUTS>"], expected: cap(() => ks.findMatchingAction(ev, ks.DEFAULT_SHORTCUTS)) })
  }
  // createKeyHandler: preventDefault + dispatch
  for (const ev of [{ key: "s", ctrlKey: true }, { key: "Escape" }, { key: "x" }, { key: "F1" }, { key: "V", ctrlKey: true, shiftKey: true }]) {
    cases.push({ name: `createKeyHandler ${JSON.stringify(ev)}`, fn: "createKeyHandler", input: [{ shiftKey: false, altKey: false, metaKey: false, ctrlKey: false, ...ev }, "<DEFAULT_SHORTCUTS>"],
      expected: cap(() => { const calls = []; let prevented = 0; const e = { shiftKey: false, altKey: false, metaKey: false, ctrlKey: false, ...ev, preventDefault: () => prevented++ }; ks.createKeyHandler(ks.DEFAULT_SHORTCUTS, (a) => calls.push(a))(e); return { actions: calls, preventDefaultCalls: prevented } }) })
  }
  for (const user of [{}, undefined, { save: null }, { save: { key: "S", ctrl: true, alt: true } }, { custom: { key: "k", ctrl: true } }, { help: null, closeDialogs: null, nope: null }])
    cases.push({ name: `mergeShortcuts defaults+${JSON.stringify(user)}`, fn: "mergeShortcuts", input: enc(["<DEFAULT_SHORTCUTS>", user]), expected: cap(() => Object.entries(ks.mergeShortcuts(ks.DEFAULT_SHORTCUTS, user))) })
  for (const a of ["save", "help", "nope", "", "constructor", "toString"]) cases.push({ name: `getShortcutForAction ${a}`, fn: "getShortcutForAction", input: ["<DEFAULT_SHORTCUTS>", a], expected: cap(() => ks.getShortcutForAction(ks.DEFAULT_SHORTCUTS, a)) })
  const realNav = globalThis.navigator
  for (const nav of [undefined, { userAgentData: { platform: "macOS" }, userAgent: "x" }, { userAgentData: { platform: "Windows" }, userAgent: "Macintosh" }, { userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)" }, { userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0)" }, { userAgent: "Mozilla/5.0 (X11; Linux x86_64)" }, { userAgent: "Mozilla/5.0 (Windows NT 10.0)" }, { userAgent: "" }]) {
    Object.defineProperty(globalThis, "navigator", { value: nav, configurable: true })
    cases.push({ name: `isMacOS ${JSON.stringify(nav ?? null)}`, fn: "isMacOS", input: { navigator: enc(nav) }, expected: cap(() => ks.isMacOS()) })
  }
  Object.defineProperty(globalThis, "navigator", { value: realNav, configurable: true })
  writeFixture("js_keyboard_shortcuts", fixture({ source: "app/javascript/lib/keyboard_shortcuts.js", generator: GEN,
    notes: "Events are plain objects {key,ctrlKey,shiftKey,altKey,metaKey}. \"<DEFAULT_SHORTCUTS>\" in input means the module's DEFAULT_SHORTCUTS object (dumped in the first case, insertion order matters: findMatchingAction returns the FIRST matching action). Matrix: m bit0=ctrl bit1=shift bit2=alt bit3=meta. mergeShortcuts expected = Object.entries(result). isMacOS: input.navigator replaces globalThis.navigator ($undefined = no navigator).",
    cases: dedupe(cases) }))
}

// ================================================================ custom_theme + theme palettes
{
  const ct = await import("lib/custom_theme")
  const cases = []
  cases.push({ name: "COLOR_KEYS", fn: "COLOR_KEYS", input: [], expected: ct.COLOR_KEYS })
  for (const fn of ["normalizeColor", "contrastRatio"]) for (const args of H("custom_theme", fn))
    cases.push({ name: `harvested ${fn} ${JSON.stringify(enc(args)).slice(0, 50)}`, fn, input: enc(args), expected: cap(() => ct[fn](...args)) })
  for (const v of [" #AABBCC ", "#aabbcc", "#ABC", "#abc", "red", "url(x)", "#ffffff;display:none", "#gggggg", "#1234567", "#12345", "\t#00ff00\n", "#00FF00", 0x112233, null, undefined, {}, ["#112233"], "", "＃112233", "#１１２２３３"])
    cases.push({ name: `normalizeColor ${JSON.stringify(v ?? String(v))}`, fn: "normalizeColor", input: enc([v]), expected: cap(() => ct.normalizeColor(v)) })
  for (const [a, b] of [["#ffffff", "#000000"], ["#000000", "#ffffff"], ["#123456", "#123456"], ["#777777", "#ffffff"], ["", "#ffffff"], ["#fff", "#000"], ["#FF0000", "#00ff00"], ["#0000ff", "#ffff00"], [" #808080 ", "#101010"], [null, null], ["#0a0a0a", "#0b0b0b"]])
    cases.push({ name: `contrastRatio ${a} ${b}`, fn: "contrastRatio", input: enc([a, b]), expected: cap(() => ct.contrastRatio(a, b)) })
  for (const v of [" #fff ", "#FFF", "#abc", "#aabbcc", "#AABBCC", "", "  ", "#abcd", "rgb(1,2,3)", "khaki"])
    cases.push({ name: `themeColor ${JSON.stringify(v)}`, fn: "themeColor", input: { getPropertyValue: v, key: "accent" },
      expected: cap(() => { const seen = []; const r = ct.themeColor({ getPropertyValue: (p) => (seen.push(p), v) }, "accent"); return { result: r, propertyRead: seen } }) })
  const root = document.documentElement
  for (const colors of [...H("custom_theme", "applyCustomColors").map(a => a[0]), { "syntax-keyword": " #ABCDEF " }, { accent: "#abc" }, null, undefined, { accent: 123 }, Object.fromEntries(ct.COLOR_KEYS.map((k, i) => [k, `#${(i * 99991).toString(16).padStart(6, "0").slice(-6)}`]))]) {
    cases.push({ name: `applyCustomColors ${JSON.stringify(colors ?? String(colors)).slice(0, 60)}`, fn: "applyCustomColors", input: enc([colors]),
      expected: cap(() => { root.removeAttribute("style"); root.style.setProperty("--editor-width", "72ch"); ct.applyCustomColors(colors); const a = root.getAttribute("style"); ct.clearCustomColors(); return { styleAfterApply: a, styleAfterClear: root.getAttribute("style") } }) })
  }
  root.removeAttribute("style")
  // built-in theme palettes (app/assets/tailwind/themes/*.css)
  const dir = join(FRANKMD, "app/assets/tailwind/themes")
  const roles = ["keyword", "function", "type", "string", "number", "constant", "identifier", "property", "operator", "comment", "punctuation", "invalid"]
  const palettes = {}
  for (const file of readdirSync(dir).filter(f => f.endsWith(".css")).sort()) {
    const src = readFileSync(join(dir, file), "utf8")
    const vars = Object.fromEntries([...src.matchAll(/--theme-([\w-]+):\s*([^;]+);/g)].map(m => [m[1], m[2].trim()]))
    const selector = src.match(/^\s*([^{\n/][^{]*)\{/m)?.[1].trim()
    palettes[file.replace(/\.css$/, "")] = { selector, vars }
    cases.push({ name: `theme ${file}`, fn: "themePalette", input: { file: `app/assets/tailwind/themes/${file}` },
      expected: { selector, vars, syntaxContrastVsCodeBg: Object.fromEntries(roles.map(r => [r, ct.contrastRatio(vars[`syntax-${r}`], vars["code-bg"])])), themeColor: Object.fromEntries(ct.COLOR_KEYS.map(k => [k, ct.themeColor({ getPropertyValue: () => vars[k] ?? "" }, k)])) } })
  }
  writeFixture("js_custom_theme", fixture({ source: "app/javascript/lib/custom_theme.js + app/assets/tailwind/themes/*.css", generator: GEN,
    notes: "applyCustomColors/clearCustomColors act on jsdom document.documentElement.style (pre-set --editor-width:72ch to show it is untouched); expected is the style attribute serialization. themeColor: styles stub returns input.getPropertyValue for every property; jsdom has no CSS.supports/canvas, so the browser-only named-color path (e.g. 'khaki' -> canvas fillStyle) returns null here (in a browser it would resolve named colors). themePalette cases dump each built-in theme's --theme-* variables (raw text), contrast of syntax roles vs --theme-code-bg (custom_theme.contrastRatio), and themeColor() per COLOR_KEY.",
    cases: dedupe(cases) }))
}

// ================================================================ emoji_data + emoji_i18n
{
  const ed = await import("lib/emoji_data")
  const cases = []
  cases.push({ name: "EMOJI_DATA full dump", fn: "EMOJI_DATA", input: [], expected: enc(ed.EMOJI_DATA) })
  const map = ed.getEmojiMap()
  cases.push({ name: "getEmojiMap full dump", fn: "getEmojiMap", input: [], expected: enc(map) })
  const sc = ed.EMOJI_DATA.map(e => e[0])
  const dups = sc.filter((s, i) => sc.indexOf(s) !== i)
  cases.push({ name: "dataset shape", fn: "EMOJI_DATA(shape)", input: [], expected: { entries: ed.EMOJI_DATA.length, mapSize: Object.keys(map).length, duplicateShortcodes: dups, entryArity: [...new Set(ed.EMOJI_DATA.map(e => e.length))], cached: ed.getEmojiMap() === map } })
  writeFixture("js_emoji_data", fixture({ source: "app/javascript/lib/emoji_data.js", generator: GEN,
    notes: "EMOJI_DATA rows are [shortcode, emoji, keywords]. getEmojiMap: shortcode -> emoji; for duplicate shortcodes the LAST row wins.", cases }))

  const ei = await import("lib/emoji_i18n")
  const c2 = []
  for (const loc of ["en", "pt-BR", "pt-PT", "es", "ja", "ko", "he", "pt", "fr", "EN", "", null, undefined, "toString", "__proto__"]) {
    c2.push({ name: `setEmojiLocale ${JSON.stringify(loc ?? String(loc))}`, fn: "setEmojiLocale+getEmojibaseLocale", input: enc([loc]), expected: cap(() => { ei.setEmojiLocale(loc); return ei.getEmojibaseLocale() }) })
  }
  const en = JSON.parse(readFileSync(join(FRANKMD, "public/emoji-data/en.json"), "utf8"))
  for (const args of H("emoji_i18n", "buildSearchIndex")) c2.push({ name: "harvested buildSearchIndex", fn: "buildSearchIndex", input: enc(args), expected: cap(() => ei.buildSearchIndex(...args)) })
  const synthetic = [{ unicode: "😀", label: "Grinning Face", tags: ["Happy", "SMILE"] }, { emoji: "🙂", label: "slight" }, { label: "no char" }, { unicode: "", emoji: "x", label: "empty unicode" },
    { unicode: "😀", label: "dup later wins", tags: [] }, { unicode: "❤️", tags: ["love"] }, { unicode: "👨‍👩‍👧", label: "Família ção", tags: ["日本"] }]
  c2.push({ name: "buildSearchIndex synthetic", fn: "buildSearchIndex", input: [synthetic], expected: cap(() => ei.buildSearchIndex(synthetic)) })
  c2.push({ name: "buildSearchIndex en.json first 40", fn: "buildSearchIndex", input: [en.slice(0, 40)], expected: cap(() => ei.buildSearchIndex(en.slice(0, 40))) })
  c2.push({ name: "buildSearchIndex empty", fn: "buildSearchIndex", input: [[]], expected: cap(() => ei.buildSearchIndex([])) })
  // locale datasets shape
  for (const f of readdirSync(join(FRANKMD, "public/emoji-data")).sort()) {
    const data = JSON.parse(readFileSync(join(FRANKMD, "public/emoji-data", f), "utf8"))
    const keys = {}
    for (const e of data) for (const k of Object.keys(e)) keys[k] = (keys[k] || 0) + 1
    c2.push({ name: `dataset ${f}`, fn: "emojiDataset(shape)", input: { file: `public/emoji-data/${f}` }, expected: { entries: data.length, keyCounts: keys, first: data[0], last: data[data.length - 1], indexSize: ei.buildSearchIndex(data).size } })
  }
  // getTranslatedEmojis with fetch served from public/ (and failures)
  const realFetch = globalThis.fetch
  const served = []
  globalThis.fetch = async (url) => {
    served.push(url)
    const m = String(url).match(/^\/emoji-data\/(\w+)\.json$/)
    if (m && ["en", "pt", "es", "ja", "ko"].includes(m[1]) && !globalThis.__failLocale?.includes(m[1])) {
      const body = JSON.parse(readFileSync(join(FRANKMD, "public/emoji-data", `${m[1]}.json`), "utf8"))
      return { ok: true, json: async () => body }
    }
    return { ok: false, status: 404, json: async () => ({}) }
  }
  const origErr = console.error; console.error = () => {}
  for (const loc of ["pt-BR", "ja", "he", "xx"]) {
    served.length = 0
    c2.push({ name: `getTranslatedEmojis ${loc}`, fn: "getTranslatedEmojis", input: { locale: loc },
      expected: await capAsync(async () => { ei.setEmojiLocale(loc); const d = await ei.getTranslatedEmojis(); return { fetched: [...served], count: d.length, first: d[0] } }) })
  }
  console.error = origErr
  globalThis.fetch = realFetch
  writeFixture("js_emoji_i18n", fixture({ source: "app/javascript/lib/emoji_i18n.js (+ public/emoji-data/*.json)", generator: GEN,
    notes: "buildSearchIndex returns a Map encoded as {\"$map\": [[emoji, {label,tags,searchTerms}], ...]} in insertion order (duplicate emoji: later entry overwrites value but keeps original position). setEmojiLocale maps via LOCALE_MAP with fallback 'en' (prototype keys like 'toString' resolve to inherited values — see cases). getTranslatedEmojis: fetch stubbed to serve public/emoji-data/<loc>.json; module caches per locale across calls; 'fetched' lists URLs requested in that call.",
    cases: dedupe(c2) }))
}

// ================================================================ icon_data
{
  const id = await import("lib/icon_data")
  const map = id.getIconMap()
  const names = id.ICON_DATA.map(r => r[0])
  const cases = [
    { name: "names (ordered)", fn: "ICON_DATA(names)", input: [], expected: names },
    { name: "shape", fn: "ICON_DATA(shape)", input: [], expected: { entries: id.ICON_DATA.length, mapSize: Object.keys(map).length, duplicateNames: names.filter((n, i) => names.indexOf(n) !== i), viewBoxes: [...new Set(id.ICON_DATA.map(r => r[2]))], arity: [...new Set(id.ICON_DATA.map(r => r.length))], cached: id.getIconMap() === map } },
    { name: "ICON_DATA full dump", fn: "ICON_DATA", input: [], expected: id.ICON_DATA },
    { name: "getIconMap full dump", fn: "getIconMap", input: [], expected: map },
  ]
  writeFixture("js_icon_data", fixture({ source: "app/javascript/lib/icon_data.js", generator: GEN,
    notes: "Phosphor icons subset. Rows are [name, svgPathData, viewBox, keywords]; getIconMap: name -> {path, viewBox} (last duplicate wins). Used by the :ph-name: marked extension.", cases }))
}

// ================================================================ codemirror_theme
{
  const t = await import("lib/codemirror_theme")
  const { tags } = await import("@lezer/highlight")
  const flat = (x, out = []) => { if (Array.isArray(x)) x.forEach(y => flat(y, out)); else if (x?.inner) flat(x.inner, out); else out.push(x); return out }
  const scopeNorm = (css) => css.replace(/ͼ[0-9a-z]+/g, "<SCOPE>")
  const tagName = new Map(Object.entries(tags).filter(([, v]) => v && typeof v === "object" && "id" in v).map(([k, v]) => [v, k]))
  const cases = []
  for (const opts of [undefined, {}, { fontFamily: "'Monaco', monospace", fontSize: "16px", lineHeight: "1.8" }, { fontSize: "20px" }, { fontFamily: "\"Fira Code\"; } body { x: y" }]) {
    cases.push({ name: `createEditorTheme ${JSON.stringify(opts ?? null)}`, fn: "createEditorTheme", input: enc([opts]),
      expected: cap(() => { const f = flat(t.createEditorTheme(opts)); const mod = f.find(p => p?.value?.getRules)?.value; const dark = f.find(p => typeof p?.value === "boolean")?.value; return { css: scopeNorm(mod.getRules()), dark } }) })
  }
  cases.push({ name: "createSyntaxHighlighting", fn: "createSyntaxHighlighting", input: [],
    expected: cap(() => {
      const hs = flat(t.createSyntaxHighlighting()).find(p => p?.value?.specs)?.value
      return { specs: hs.specs.map(({ tag, ...style }) => ({ tag: (Array.isArray(tag) ? tag : [tag]).map(x => tagName.get(x) ?? String(x)), ...style })), css: scopeNorm(hs.module.getRules()) }
    }) })
  cases.push({ name: "createTheme shape", fn: "createTheme", input: [], expected: cap(() => ({ length: t.createTheme().length })) })
  writeFixture("js_codemirror_theme", fixture({ source: "app/javascript/lib/codemirror_theme.js", generator: GEN,
    notes: "Vendored @codemirror/view EditorView.theme -> StyleModule rules (generated scope class names 'ͼxx' normalized to <SCOPE>); dark = the {dark:true} theme flag. createSyntaxHighlighting: HighlightStyle specs with @lezer/highlight tag names + generated CSS. Font options are interpolated verbatim (no escaping).", cases }))
}

// ================================================================ codemirror_wikilink
{
  const w = await import("lib/codemirror_wikilink")
  const { EditorState } = await import("@codemirror/state")
  const flat = (x, out = []) => { if (Array.isArray(x)) x.forEach(y => flat(y, out)); else if (x?.inner) flat(x.inner, out); else out.push(x); return out }
  const cfg = flat(w.createWikilinkAutocomplete()).find(p => p?.value?.override)?.value
  const source = cfg.override[0]
  const FILES = [
    { path: "Note.md", name: "Note" }, { path: "projects/My Note.md" }, { path: "projects/notes/deep.md", name: "Deep Custom" }, { path: "image.png" },
    { path: "Ação 日本.md" }, { path: "README.MD" }, { name: "no path" }, { path: "folder/" }, { path: ".hidden.md" }, { path: "a/b/c/My Note.md" }, { path: "notebook.md" }, { path: "x.md.md" },
  ]
  const run = (doc, pos, files) => {
    w.setWikilinkFileProvider(files === "<null provider>" ? null : () => files)
    const state = EditorState.create({ doc })
    const res = source({ state, pos: pos ?? doc.length, explicit: false })
    if (!res) return res
    return { from: res.from, filter: res.filter, options: res.options.map(o => {
      // capture what apply() inserts
      let inserted = null
      o.apply({ dispatch: (tr) => { inserted = tr } }, o, res.from, pos ?? doc.length)
      return { label: o.label, detail: o.detail, boost: o.boost, apply: inserted }
    }) }
  }
  const cases = []
  const docs = [["[[", null], ["[[no", null], ["[[note", null], ["[[NOTE", null], ["[[proj", null], ["[[deep", null], ["[[my", null], ["[[ação", null], ["[[zzz", null], ["no link", null], ["[[done]] ", null],
    ["[[a]] [[not", null], ["text [[", null], ["line1\n[[no", null], ["[[no\nnext", 4], ["[[x]] y", 7], ["[[ note", null], ["[[.", null], ["[[/", null], ["[[my note", null], ["[[[", null], ["[[]", null]]
  for (const [doc, pos] of docs) cases.push({ name: `complete ${JSON.stringify(doc)} @${pos ?? "end"}`, fn: "wikilinkCompletions", input: { doc, pos: pos ?? doc.length, files: FILES }, expected: cap(() => run(doc, pos, FILES)) })
  for (const files of [[], null, "<null provider>", [{ path: "only.txt" }]]) cases.push({ name: `complete files=${JSON.stringify(files)}`, fn: "wikilinkCompletions", input: { doc: "[[", pos: 2, files }, expected: cap(() => run("[[", 2, files)) })
  cases.push({ name: "autocompletion config", fn: "createWikilinkAutocomplete(config)", input: [], expected: enc({ activateOnTyping: cfg.activateOnTyping, maxRenderedOptions: cfg.maxRenderedOptions, icons: cfg.icons, overrideCount: cfg.override.length, addToOptions: cfg.addToOptions.length }) })
  const cfg2 = flat(w.createWikilinkAutocomplete({ additionalSources: [() => null], addToOptions: [{ render: () => null, position: 20 }] })).find(p => p?.value?.override)?.value
  cases.push({ name: "autocompletion config with extras", fn: "createWikilinkAutocomplete(config)", input: [{ additionalSources: 1, addToOptions: 1 }], expected: enc({ activateOnTyping: cfg2.activateOnTyping, maxRenderedOptions: cfg2.maxRenderedOptions, icons: cfg2.icons, overrideCount: cfg2.override.length, addToOptions: cfg2.addToOptions.length }) })
  writeFixture("js_codemirror_wikilink", fixture({ source: "app/javascript/lib/codemirror_wikilink.js", generator: GEN,
    notes: "wikilinkCompletions (module-private) is taken from the autocompletion({override}) config of createWikilinkAutocomplete() and called with {state: EditorState.create({doc}), pos}. Option order is the file-provider order (sorting by boost is CodeMirror's job). apply = the transaction spec dispatched by option.apply(view, completion, from, to=pos): inserts '<path without .md>]]'. '<null provider>' = setWikilinkFileProvider(null). Only paths ending in lowercase '.md' are offered.",
    cases: dedupe(cases) }))
}
