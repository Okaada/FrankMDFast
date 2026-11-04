// Shared helpers for JS golden generators.
//   import { lib, writeFixture, capture, SHA } from "../lib/golden.mjs"
import { execSync } from "node:child_process"
import { mkdirSync, writeFileSync } from "node:fs"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { homedir } from "node:os"

export const FRANKMD = resolve(process.env.FRANKMD || join(homedir(), "Projects/FrankMD"))
export const OUT_DIR = resolve(dirname(fileURLToPath(import.meta.url)), "../../../tests/fixtures/golden")
export const SHA = execSync(`git -C ${FRANKMD} rev-parse HEAD`).toString().trim()

// Dynamic import of a FrankMD module: await lib("app/javascript/lib/text_utils.js")
export const lib = (rel) => import(pathToFileURL(join(FRANKMD, rel)).href)

export function fixture({ source, generator, cases, notes = null, ...extra }) {
  return { source: `${source} @ ${SHA}`, frankmd_sha: SHA, generator, notes, ...extra, cases }
}

export function writeFixture(area, data) {
  mkdirSync(OUT_DIR, { recursive: true })
  const path = join(OUT_DIR, `${area}.json`)
  writeFileSync(path, JSON.stringify(data, null, 2) + "\n")
  console.error(`wrote ${path} (${data.cases?.length} cases)`)
  return path
}

// JSON encoding of JS values that plain JSON can't carry:
//   undefined -> {"$undefined": true}, NaN/Infinity/-0 -> {"$number": "NaN"|"Infinity"|"-Infinity"|"-0"},
//   Date -> {"$date": "<ISO>"}, RegExp -> {"$regexp": "/src/flags"},
//   ill-formed UTF-16 string (lone surrogate) -> {"$utf16": [code units]} (serde_json rejects lone surrogates).
// Functions / DOM nodes throw Unserializable (callers skip those cases).
export class Unserializable extends Error {}
export function encode(v) {
  if (v === undefined) return { $undefined: true }
  if (typeof v === "string") return v.isWellFormed() ? v : { $utf16: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) }
  if (v === null || typeof v === "boolean") return v
  if (typeof v === "number") {
    if (Number.isNaN(v)) return { $number: "NaN" }
    if (v === Infinity) return { $number: "Infinity" }
    if (v === -Infinity) return { $number: "-Infinity" }
    if (Object.is(v, -0)) return { $number: "-0" }
    return v
  }
  if (v instanceof Date) return { $date: v.toISOString() }
  if (v instanceof RegExp) return { $regexp: String(v) }
  if (Array.isArray(v)) return v.map(encode)
  if (typeof v === "object") {
    const proto = Object.getPrototypeOf(v)
    if (proto !== Object.prototype && proto !== null) throw new Unserializable(proto?.constructor?.name)
    const out = {}
    for (const k of Object.keys(v)) out[k] = encode(v[k])
    return out
  }
  throw new Unserializable(typeof v)
}
export function decode(v) {
  if (Array.isArray(v)) return v.map(decode)
  if (v && typeof v === "object") {
    if (v.$undefined) return undefined
    if ("$number" in v) return v.$number === "-0" ? -0 : Number(v.$number)
    if ("$date" in v) return new Date(v.$date)
    if ("$utf16" in v) return String.fromCharCode(...v.$utf16)
    if ("$regexp" in v) { const m = v.$regexp.match(/^\/(.*)\/(\w*)$/s); return new RegExp(m[1], m[2]) }
    const out = {}
    for (const k of Object.keys(v)) out[k] = decode(v[k])
    return out
  }
  return v
}

// Run fn, returning encoded value or {"$error": {name, message}}.
export function capture(fn) {
  try {
    return encode(fn())
  } catch (e) {
    if (e instanceof Unserializable) throw e
    return { $error: { name: e?.constructor?.name || "Error", message: String(e?.message ?? e) } }
  }
}
