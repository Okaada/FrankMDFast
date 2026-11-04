// Golden fixtures for app/javascript/lib/image_sources/*.js: request URL/body building,
// response handling given canned API JSON, grid HTML templates, picker text.
// @rails/request.js is replaced by stubs/requestjs.mjs (records requests, serves canned
// responses FIFO); everything else resolves via the importmap emulation + jsdom.
//
// usage: node tools/golden/js/image_sources.mjs
import { join, dirname } from "node:path"
import { fileURLToPath } from "node:url"
const HERE = dirname(fileURLToPath(import.meta.url))
process.env.GOLDEN_IMPORT_OVERRIDES = JSON.stringify({ "@rails/request.js": join(HERE, "stubs/requestjs.mjs") })
const NodeFormData = globalThis.FormData
await import("./runtime/setup.mjs")
globalThis.FormData = NodeFormData // node File/FormData (jsdom FormData rejects node Files)
const { fixture, writeFixture } = await import("../lib/golden.mjs")
const { enc } = await import("./runtime/common.mjs")
const rq = await import("@rails/request.js")

const GEN = "tools/golden/js/image_sources.mjs"
const T = await import("lib/image_sources/image_picker_text")
const { WebImageSource } = await import("lib/image_sources/web_images")
const { GoogleImageSource } = await import("lib/image_sources/google_images")
const { PinterestImageSource } = await import("lib/image_sources/pinterest_images")
const { LocalImageSource } = await import("lib/image_sources/local_images")
const { AiImageSource } = await import("lib/image_sources/ai_images")
const F = await import("lib/image_sources/folder_images")
const CLASSES = { WebImageSource, GoogleImageSource, PinterestImageSource, LocalImageSource, AiImageSource, FolderImageSource: F.FolderImageSource }

console.error = () => {}; console.log = () => {}
Date.now = () => 1700000000000

// --- browser API stubs for folder images
const dims = new Map()
URL.createObjectURL = (file) => { const u = `blob:http://localhost:3000/${file.name}`; dims.set(u, file); return u }
URL.revokeObjectURL = () => {}
globalThis.Image = class { set src(u) { const f = dims.get(u); setTimeout(() => { if (f?.width) { this.naturalWidth = f.width; this.naturalHeight = f.height; this.onload?.() } else this.onerror?.() }, 0) } }
const TRANSLATIONS = {
  none: undefined,
  identity: (k) => k, // window.t returning the key == "missing translation"
  pt: (k, o = {}) => `[pt] ${k}${Object.keys(o).length ? " " + JSON.stringify(o) : ""}`,
}

const rawContainer = () => { const c = { raw: null }; Object.defineProperty(c, "innerHTML", { set(v) { c.raw = v }, get() { return c.raw } }); return c }
const domSafe = (v) => (v && typeof v === "object" && "file" in v ? { ...v, file: { $file: v.file.name } } : v)

async function runScenario(sc) {
  rq.requests.length = 0; rq.queue.length = 0
  if (sc.translation) window.t = TRANSLATIONS[sc.translation]; else delete window.t
  if (sc.showDirectoryPicker !== undefined) window.showDirectoryPicker = sc.showDirectoryPicker === "abort" ? async () => { throw Object.assign(new Error("x"), { name: "AbortError" }) } : sc.showDirectoryPicker === "error" ? async () => { throw new Error("denied") } : async () => dirHandle(sc.showDirectoryPicker)
  else delete window.showDirectoryPicker
  const obj = new CLASSES[sc.source]()
  Object.assign(obj, sc.setup || {})
  const out = []
  for (const st of sc.steps) {
    const before = rq.requests.length
    rq.queue.push(...(st.responses || []))
    let result, rendered
    try {
      if (st.call === "renderGrid" || st.call === "renderRefImageGrid") {
        const c = rawContainer(); const status = { textContent: null }
        const args = st.call === "renderRefImageGrid" ? [st.args[0], c, st.args[1]] : sc.source === "LocalImageSource" ? [st.args[0], c, st.args[1]] : sc.source === "FolderImageSource" ? [c, st.withStatus === false ? null : status, st.args[0], st.args[1]] : [c, st.args[0]]
        obj[st.call](...args)
        rendered = { innerHTML: c.raw, status: sc.source === "FolderImageSource" ? status.textContent : undefined }
      } else if (st.call === "deselectAll") {
        const d = document.createElement("div"); d.innerHTML = st.html; obj.deselectAll(d); rendered = { innerHTML: d.innerHTML }
      } else if (st.call === "ingest") {
        result = await obj.ingest(st.args[0].map(fakeFile), ...st.args.slice(1))
      } else if (st.call === "upload") {
        result = await obj.upload(new File(["x".repeat(st.args[0].size || 3)], st.args[0].name, { type: st.args[0].type || "image/png" }), ...st.args.slice(1))
      } else if (st.call === "abortDuringGenerate") {
        const p = obj.generate(st.args[0]); obj.abort(); result = await p
      } else {
        result = await obj[st.call](...(st.args || []))
      }
    } catch (e) { result = { $thrown: { name: e.constructor.name, message: e.message } } }
    const snapshot = {}
    for (const k of sc.watch || []) snapshot[k] = Array.isArray(obj[k]) ? obj[k].map(domSafe) : domSafe(obj[k])
    out.push(enc({ call: st.call, result: Array.isArray(result?.images) || result?.count !== undefined ? result : domSafe(result), rendered, requests: rq.requests.slice(before), state: sc.watch ? snapshot : undefined }))
  }
  return out
}
function fakeFile(f) { return { name: f.name, lastModified: f.lastModified ?? 0, size: f.size ?? 1024, width: f.width, height: f.height } }
function dirHandle(entries) { return { async *values() { for (const e of entries) yield e.kind === "directory" ? { kind: "directory", name: e.name } : { kind: "file", name: e.name, getFile: async () => fakeFile(e) } } } }

const IMGS = [{ url: "http://example.com/1.jpg", thumbnail: "http://example.com/t1.jpg", title: "Image 1", source: "example.com", width: 800, height: 600 },
  { url: "https://x.y/2.png", title: "<b>\"Two\"</b> & 'q'", source: "x.y" }, { url: "javascript:alert(1)", title: "evil" }, { url: "https://ok/3.jpg", thumbnail: "data:image/png;base64,AAA", width: "0", height: 5 },
  { url: "https://ok/4.jpg", thumbnail: "javascript:x", width: "640", height: "480", title: "Ação 日本 👨‍👩‍👧" }, { url: "\" onerror=\"alert(1)", width: NaN }]
const LOCAL = [{ path: "photo.jpg", name: "photo.jpg", width: 100, height: 50 }, { path: "sub dir/ação #1?.png", name: "ação #1?.png" }, { path: "x\".png", name: "x\" onerror=\"alert(1).png", width: "a" }, { path: "a/b/c.gif", name: "c.gif", width: 0, height: 0 }]

const S = []
const add = (name, sc) => S.push({ name, sc })
for (const tr of ["none", "identity", "pt"]) {
  add(`web search flow t=${tr}`, { source: "WebImageSource", translation: tr, watch: ["results"], steps: [
    { call: "search", args: [""] }, { call: "search", args: [null] },
    { call: "search", args: ["cats"], responses: [{ json: { images: IMGS.slice(0, 2) } }] },
    { call: "search", args: ["cats & dogs/ação?#"], responses: [{ json: { images: [] } }] },
    { call: "search", args: ["x"], responses: [{ json: { images: [], note: "Bing note" } }] },
    { call: "search", args: ["x"], responses: [{ json: { error: "API rate limit exceeded" } }] },
    { call: "search", args: ["x"], responses: [{ json: {} }] },
    { call: "search", args: ["one"], responses: [{ json: { images: [IMGS[0]] } }] },
    { call: "search", args: ["x"], responses: [{ reject: "network" }] }] })
}
add("web renderGrid", { source: "WebImageSource", steps: [{ call: "renderGrid", args: ["click->image-picker#selectWeb"] }, { call: "search", args: ["q"], responses: [{ json: { images: IMGS } }] }, { call: "renderGrid", args: ["click->x#y"] },
  { call: "deselectAll", html: '<button class="external-image-item ring-2 ring-blue-500 other">a</button><div class="ring-2">b</div>' },
  { call: "saveCopy", args: ["https://x/a.png", "0.5"], responses: [{ json: { url: "/images/a.png" } }] }, { call: "saveCopy", args: ["https://x/a.png", null, "notes/2024"], responses: [{ ok: false, status: 422, json: { error: "bad" } }] },
  { call: "saveCopy", args: ["u", "", ""], responses: [{ ok: false, json: {} }] }, { call: "reset" }] })
add("google search/loadMore", { source: "GoogleImageSource", watch: ["results", "nextStart", "loading", "query"], steps: [
  { call: "search", args: [""] }, { call: "search", args: ["sunset beach"], responses: [{ json: { images: IMGS.slice(0, 2), next_start: 11, total: 250 } }] },
  { call: "shouldLoadMore", args: [50] }, { call: "shouldLoadMore", args: [100] }, { call: "loadMore", responses: [{ json: { images: IMGS.slice(2, 4), next_start: 21 } }] },
  { call: "loadMore", responses: [{ json: { error: "quota" } }] }, { call: "search", args: ["none"], responses: [{ json: { images: [], next_start: null, total: 0 } }] },
  { call: "shouldLoadMore", args: [0] }, { call: "search", args: ["one"], responses: [{ json: { images: [IMGS[0]], total: 1 } }] }, { call: "loadMore", responses: [{ reject: "net" }] },
  { call: "renderGrid", args: ["click->a#b"] }, { call: "reset" }, { call: "renderGrid", args: ["click->a#b"] }] })
add("google loading guard", { source: "GoogleImageSource", setup: { loading: true, results: [{ url: "a" }] }, steps: [{ call: "loadMore" }, { call: "shouldLoadMore", args: [10] }] })
add("pinterest", { source: "PinterestImageSource", watch: ["results"], steps: [{ call: "search", args: [""] }, { call: "search", args: ["cozy room"], responses: [{ json: { images: IMGS } }] },
  { call: "renderGrid", args: ["click->p#s"] }, { call: "search", args: ["x"], responses: [{ json: { error: "blocked" } }] }, { call: "search", args: ["x"], responses: [{ json: { images: [] } }] },
  { call: "search", args: ["x"], responses: [{ json: {} }] }, { call: "search", args: ["x"], responses: [{ reject: "net" }] }, { call: "renderGrid", args: ["click->p#s"] }] })
for (const tr of ["none", "pt"]) add(`local t=${tr}`, { source: "LocalImageSource", translation: tr, steps: [
  { call: "load", responses: [{ json: [{ path: "a.png", name: "a.png" }] }] }, { call: "load", args: ["sun set&x", { limit: 25, offset: 50 }], responses: [{ json: [] }] },
  { call: "load", args: ["", { limit: "1 0" }], responses: [{ json: [] }] }, { call: "load", args: ["x"], responses: [{ ok: false, json: { error: "Not configured" } }] },
  { call: "load", args: ["x"], responses: [{ ok: false, json: {} }] }, { call: "load", args: ["x"], responses: [{ reject: "net" }] },
  { call: "renderGrid", args: [[], "click->a#b"] }, { call: "renderGrid", args: [null, "click->a#b"] }, { call: "renderGrid", args: [LOCAL, "click->image-picker#selectLocal"] },
  { call: "deselectAll", html: '<div class="image-grid-item selected">a</div><div class="selected">b</div>' },
  { call: "deleteImage", args: ["sub dir/ação #1?.png"], responses: [{ json: { ok: true } }] }, { call: "deleteImage", args: ["x.png"], responses: [{ ok: false, json: { error: "nope" } }] }, { call: "deleteImage", args: ["x.png"], responses: [{ ok: false, json: {} }] },
  { call: "saveCopy", args: ["a/b.png", "0.25", "notes/key.png"], responses: [{ json: { url: "https://s3/x" } }] }, { call: "saveCopy", args: ["a.png", null], responses: [{ ok: false, json: {} }] }] })
add("local selected", { source: "LocalImageSource", setup: { selectedPath: "photo.jpg" }, steps: [{ call: "renderGrid", args: [LOCAL.slice(0, 2), "click->a#b"] }, { call: "reset" }, { call: "renderGrid", args: [LOCAL.slice(0, 1), "click->a#b"] }] })
add("ai flow", { source: "AiImageSource", watch: ["enabled", "model", "generatedData", "refImage"], steps: [
  { call: "generate", args: [""] }, { call: "generate", args: ["a cat"] },
  { call: "loadConfig", responses: [{ ok: false, json: {} }] }, { call: "loadConfig", responses: [{ reject: "net" }] },
  { call: "loadConfig", responses: [{ json: { enabled: true, model: "imagen-4" } }] },
  { call: "generate", args: ["a cat"], responses: [{ json: { data: "QUJD", mime_type: "image/jpeg", model: "imagen-4", revised_prompt: "a fluffy cat" } }] },
  { call: "save", responses: [{ json: { url: "/images/ai_1700000000000.png" } }] }, { call: "save", args: ["notes/2024"], responses: [{ ok: false, json: { error: "disk full" } }] },
  { call: "setRefImage", args: ["refs/ação.png", "ação.png"] },
  { call: "generate", args: ["variation"], responses: [{ json: { url: "https://cdn/x.png" } }] },
  { call: "save", args: ["s3/prefix"], responses: [{ json: { url: "https://s3/x.png" } }] }, { call: "save", responses: [{ ok: false, json: {} }] },
  { call: "generate", args: ["p"], responses: [{ json: { data: "Zm9v" } }] },
  { call: "generate", args: ["p"], responses: [{ json: { error: "safety block" } }] }, { call: "generate", args: ["p"], responses: [{ reject: "net" }] },
  { call: "generate", args: ["p"], responses: [{ abort: true }] }, { call: "clearRefImage" },
  { call: "loadRefImages", responses: [{ json: [{ path: "a.png", name: "a.png" }] }] }, { call: "loadRefImages", args: ["a b&c"], responses: [{ json: [] }] },
  { call: "loadRefImages", args: ["x"], responses: [{ ok: false, json: { error: "E" } }] }, { call: "loadRefImages", args: ["x"], responses: [{ ok: false, json: {} }] }, { call: "loadRefImages", responses: [{ reject: "n" }] },
  { call: "renderRefImageGrid", args: [[], "click->a#b"] }, { call: "renderRefImageGrid", args: [LOCAL, "click->ai#selectRef"] },
  { call: "reset" }, { call: "save" }] })
add("ai selected ref grid", { source: "AiImageSource", setup: { refImage: { path: "photo.jpg", name: "photo.jpg" } }, steps: [{ call: "renderRefImageGrid", args: [LOCAL.slice(0, 2), "click->a#b"] }] })
for (const tr of ["none", "pt"]) add(`folder ingest/filter/render t=${tr}`, { source: "FolderImageSource", translation: tr, watch: ["displayedImages"], steps: [
  { call: "ingest", args: [[{ name: "old.JPG", lastModified: 1, width: 10, height: 20 }, { name: "new.png", lastModified: 5, size: 204800, width: 1920, height: 1080 }, { name: "doc.pdf" }, { name: "noext" }, { name: ".hidden" }, { name: "ação 日本.webp", lastModified: 3 }, { name: "a.tar.GZ" }, { name: "x.jpeg.exe" }]] },
  { call: "renderGrid", args: ["click->folder#select", 3] }, { call: "renderGrid", args: ["click->folder#select", 10] }, { call: "renderGrid", args: ["click->folder#select"] },
  { call: "filter", args: ["  NEW "] }, { call: "renderGrid", args: ["click->f#s"] }, { call: "filter", args: ["zzz"] }, { call: "renderGrid", args: ["click->f#s"] }, { call: "renderGrid", args: ["click->f#s"], withStatus: false },
  { call: "filter", args: ["", { offset: 1, limit: 1 }] }, { call: "getImage", args: [0] }, { call: "getImage", args: [5] },
  { call: "ingest", args: [Array.from({ length: 13 }, (_, i) => ({ name: `img${i}.gif`, lastModified: i, width: i + 1, height: i + 1 })), [".GIF"]] }, { call: "renderGrid", args: ["c", 13] },
  { call: "ingest", args: [[{ name: "a.png" }, { name: "b.svg" }], [".svg"]] },
  { call: "upload", args: [{ name: "a.png", size: 5 }, "0.5", "k/a.png"], responses: [{ json: { url: "/images/a.png" } }] }, { call: "upload", args: [{ name: "b.png" }, null], responses: [{ ok: false, json: { error: "too big" } }] }, { call: "upload", args: [{ name: "b.png" }, ""], responses: [{ ok: false, json: {} }] },
  { call: "reset" }] })
add("folder browse", { source: "FolderImageSource", showDirectoryPicker: [{ name: "a.png", lastModified: 2, width: 1, height: 1 }, { name: "sub", kind: "directory" }, { name: "b.txt" }, { name: "c.BMP", lastModified: 9 }], steps: [{ call: "browse" }, { call: "browse", args: [[".txt"]] }] })
add("folder browse unsupported", { source: "FolderImageSource", steps: [{ call: "browse" }] })
add("folder browse abort", { source: "FolderImageSource", showDirectoryPicker: "abort", steps: [{ call: "browse" }] })
add("folder browse error", { source: "FolderImageSource", showDirectoryPicker: "error", steps: [{ call: "browse" }] })

const cases = []
for (const { name, sc } of S) cases.push({ name, fn: "scenario", input: enc(sc), expected: await runScenario(sc) })
// pure helpers
for (const tr of ["none", "identity", "pt"]) {
  if (tr === "none") delete window.t; else window.t = TRANSLATIONS[tr]
  for (const [k, o, fb] of [["x", {}, "Fallback"], ["missing", undefined, undefined], ["folder_showing_recent", { shown: 3, total: 9 }, "Showing %{shown} most recent of %{total} images"], ["k", { a: 1 }, "%{a} %{b} %{a}%{}"], ["k", { count: 0 }, "%{count} images"], ["k", { v: undefined, n: null }, "%{v}|%{n}"]])
    cases.push({ name: `imagePickerText t=${tr} ${k} ${fb}`, fn: "imagePickerText", input: { translation: tr, args: enc([k, o, fb]) }, expected: (() => { try { return enc(T.imagePickerText(k, o, fb)) } catch (e) { return { $error: { name: e.constructor.name, message: e.message } } } })() })
  for (const n of [0, 1, 2, "1", "abc", 1.5, null, 1000])
    cases.push({ name: `imagePickerSearchResultMessage t=${tr} ${n}`, fn: "imagePickerSearchResultMessage", input: { translation: tr, args: [n] }, expected: enc(T.imagePickerSearchResultMessage(n)) })
}
delete window.t
for (const n of ["photo.PNG", "a.tar.gz", "noext", ".hidden", "trailing.", "ação.JPEG", "", "a..b", "日本.webp"]) cases.push({ name: `extOf ${JSON.stringify(n)}`, fn: "extOf", input: [n], expected: enc(F.extOf(n)) })
cases.push({ name: "DEFAULT_IMAGE_EXTENSIONS", fn: "DEFAULT_IMAGE_EXTENSIONS", input: [], expected: F.DEFAULT_IMAGE_EXTENSIONS })
writeFixture("js_image_sources", fixture({ source: "app/javascript/lib/image_sources/*.js", generator: GEN,
  notes: "Scenario cases: input.source = class name; input.setup = props assigned after construction; input.translation: none (no window.t) | identity (window.t returns the key => treated as missing) | pt (window.t returns \"[pt] <key> <json options>\"). Each step: responses = canned replies served FIFO to @rails/request.js get/post/destroy ({ok,status,json} | {reject: msg} -> network Error | {abort:true} -> AbortError). Expected per step: result (encoded; File objects -> {$file:name}; thrown -> {$thrown}), rendered.innerHTML = the RAW string assigned to container.innerHTML (exact template whitespace), rendered.status = folder status textContent, requests = recorded {method,url,responseKind,body (FormData -> {$formData:[[k,v]]}),signal}, state = watched props. Folder images: URL.createObjectURL -> blob:http://localhost:3000/<name>; Image loads with fake file width/height (no width -> onerror -> null dims). Date.now pinned to 1700000000000 (ai_<ts>.png filenames). deselectAll runs on a jsdom div (innerHTML is jsdom serialization).",
  cases }))
