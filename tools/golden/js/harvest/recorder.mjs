// Records (module, fn, args) of calls made while FrankMD's own vitest suite runs.
// Only inputs are harvested; expected values are recomputed later by the generator
// in a clean node process against the real module.
import { appendFileSync } from "node:fs"
import { encode } from "../../lib/golden.mjs"

const OUT = process.env.HARVEST_OUT
const seen = new Set()

export function record(mod, name, fn, args) {
  if (OUT) {
    try {
      const enc = JSON.stringify(encode(structuredCloneSafe(args)))
      const key = `${mod}\u0000${name}\u0000${enc}`
      if (!seen.has(key) && enc.length < 200000) {
        seen.add(key)
        appendFileSync(OUT, JSON.stringify({ mod, fn: name, args: JSON.parse(enc), test: process.env.VITEST_TEST_FILE || null }) + "\n")
      }
    } catch { /* unserializable (DOM, functions) - skip */ }
  }
  return fn(...args)
}

function structuredCloneSafe(v) {
  // Snapshot args before the call (functions may mutate them, e.g. accumulators)
  return JSON.parse(JSON.stringify(v, function (k, x) {
    const orig = this[k] // pre-toJSON value (Dates)
    if (orig instanceof Date) return { $date: orig.toISOString() }
    if (x === undefined) return { $undefined: true }
    if (typeof x === "number" && !Number.isFinite(x)) return { $number: String(x) }
    if (typeof x === "function") throw new Error("fn")
    if (x && typeof x === "object" && !Array.isArray(x)) {
      const p = Object.getPrototypeOf(x)
      if (p !== Object.prototype && p !== null) throw new Error("obj")
    }
    return x
  }), (k, x) => x)
}
