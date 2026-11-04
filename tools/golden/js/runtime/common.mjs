import { createHash } from "node:crypto"
// Shared helpers for the rendering/UI generators (render.mjs, ui_data.mjs, ...).
// Import AFTER ./setup.mjs.
import { readFileSync, existsSync, writeFileSync, mkdirSync } from "node:fs"
import { execFileSync } from "node:child_process"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import { encode, decode, Unserializable } from "../../lib/golden.mjs"

const HERE = dirname(fileURLToPath(import.meta.url))
export const JS_DIR = join(HERE, "..")

// Harvested call inputs (FrankMD's vitest suite with recording wrappers), committed
// as tools/golden/js/harvest/inputs/<name>.jsonl. REHARVEST=1 re-runs the suite.
export function harvested(name, mods, cap = 60) {
  const file = join(JS_DIR, "harvest/inputs", `${name}.jsonl`)
  if (process.env.REHARVEST || !existsSync(file)) {
    const tmp = join(process.env.GOLDEN_TMP || "/tmp", `${name}-harvest.jsonl`)
    mkdirSync(dirname(file), { recursive: true })
    execFileSync(join(JS_DIR, "harvest/harvest.sh"), [tmp, ...mods], {
      stdio: "inherit",
      env: { ...process.env, HARVEST_WRAPPER_NODE_OPTS: `--import ${join(HERE, "setup.mjs")}` },
    })
    // vitest workers append in nondeterministic order: dedupe, then pick `cap` per fn by content hash
    const per = new Map()
    for (const line of readFileSync(tmp, "utf8").split("\n").filter(Boolean)) {
      const r = JSON.parse(line)
      const k = `${r.mod}.${r.fn}`
      if (!per.has(k)) per.set(k, new Map())
      const json = JSON.stringify({ mod: r.mod, fn: r.fn, args: r.args })
      per.get(k).set(createHash("sha1").update(json).digest("hex"), json)
    }
    writeFileSync(file, [...per.keys()].sort().flatMap(k => [...per.get(k).entries()].sort(([a], [b]) => (a < b ? -1 : 1)).slice(0, cap).map(([, j]) => j)).join("\n") + "\n")
  }
  const rows = readFileSync(file, "utf8").split("\n").filter(Boolean).map(l => JSON.parse(l))
  return (mod, fn) => rows.filter(r => r.mod === mod && r.fn === fn).map(r => decode(r.args))
}

// encode() that also accepts Error objects / Maps / Sets / functions (for result objects):
//   Error -> {$error:{name,message}}, Map -> {$map:[[k,v]...]}, Set -> {$set:[...]}, function -> {$function:name}.
export function enc(v) {
  if (typeof v === "function") return { $function: v.name || "anonymous" }
  if (v instanceof Error) return { $error: { name: v.constructor?.name || "Error", message: v.message } }
  if (v instanceof Map) return { $map: [...v.entries()].map(([k, x]) => [enc(k), enc(x)]) }
  if (v instanceof Set) return { $set: [...v].map(enc) }
  if (Array.isArray(v)) return v.map(enc)
  if (v && typeof v === "object" && !(v instanceof Date) && !(v instanceof RegExp)) {
    const p = Object.getPrototypeOf(v)
    if (p === Object.prototype || p === null) {
      const o = {}
      for (const k of Object.keys(v)) o[k] = enc(v[k])
      return encode(o) // re-run for string normalization ($utf16) — idempotent on encoded values
    }
  }
  return encode(v)
}

export function cap(fn) {
  try { return enc(fn()) } catch (e) {
    if (e instanceof Unserializable) throw e
    return { $error: { name: e?.constructor?.name || "Error", message: String(e?.message ?? e) } }
  }
}
export async function capAsync(fn) {
  try { return enc(await fn()) } catch (e) {
    if (e instanceof Unserializable) throw e
    return { $error: { name: e?.constructor?.name || "Error", message: String(e?.message ?? e) } }
  }
}

// Dedupe cases on (fn, input) keeping first occurrence.
export function dedupe(cases) {
  const seen = new Set()
  return cases.filter(c => {
    const k = JSON.stringify([c.fn, c.input])
    if (seen.has(k)) return false
    seen.add(k)
    return true
  })
}

// Edge strings shared by generators.
export const EDGE = {
  empty: "", space: " ", ws: " \t  ", nbsp: "a b", tab: "a\tb", crlf: "line1\r\nline2\r\n", cr: "line1\rline2",
  accents: "Olá ação café résumé naïve Ünïcödé", combining: "été", cjk: "日本語のテキスト 中文 한국어",
  hebrew: "שלום עולם", emoji: "👍 🎉", zwj: "👨‍👩‍👧 family", long: "word ".repeat(2000),
}
