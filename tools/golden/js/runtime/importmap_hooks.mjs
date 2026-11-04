// Node module-resolution hooks emulating FrankMD's browser importmap
// (config/importmap.rb): bare specifiers resolve to the VENDORED files the app
// ships (vendor/javascript/*), "lib/x" -> app/javascript/lib/x.js. Anything not
// pinned fails to resolve, exactly like in the browser (e.g. "./simple-mode.js"
// imported by the vendored dockerfile legacy mode). Overrides (stubs for
// network modules) come from GOLDEN_IMPORT_OVERRIDES (JSON {specifier: abs path}).
import { readFileSync, existsSync } from "node:fs"
import { join } from "node:path"
import { pathToFileURL, fileURLToPath } from "node:url"

const F = process.env.FRANKMD
const pins = new Map()
const src = readFileSync(join(F, "config/importmap.rb"), "utf8")
for (const m of src.matchAll(/^\s*pin\s+"([^"]+)"(?:\s*,\s*to:\s*"([^"]+)")?/gm)) {
  pins.set(m[1], join(F, "vendor/javascript", m[2] || `${m[1]}.js`))
}
const overrides = JSON.parse(process.env.GOLDEN_IMPORT_OVERRIDES || "{}")
const VENDOR = pathToFileURL(join(F, "vendor/javascript") + "/").href

export function resolve(specifier, context, next) {
  if (overrides[specifier]) return { url: pathToFileURL(overrides[specifier]).href, shortCircuit: true }
  if (pins.has(specifier)) return { url: pathToFileURL(pins.get(specifier)).href, shortCircuit: true }
  if (specifier.startsWith("lib/")) {
    const p = join(F, "app/javascript", specifier.endsWith(".js") ? specifier : `${specifier}.js`)
    return { url: pathToFileURL(p).href, shortCircuit: true }
  }
  // relative imports inside vendor/ must exist there (browser would 404 otherwise)
  if (context.parentURL?.startsWith(VENDOR) && specifier.startsWith(".")) {
    const u = new URL(specifier, context.parentURL)
    if (!existsSync(fileURLToPath(u))) throw Object.assign(new Error(`importmap: ${specifier} not found (404 in browser)`), { code: "ERR_MODULE_NOT_FOUND" })
  }
  return next(specifier, context)
}
