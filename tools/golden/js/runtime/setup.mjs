// Import FIRST (before any FrankMD module): registers importmap hooks and
// installs jsdom globals (window/document/...) like a browser page.
import { join } from "node:path"
import { homedir } from "node:os"
import { createRequire } from "node:module"
process.env.FRANKMD ||= join(homedir(), "Projects/FrankMD")
process.env.TZ = "UTC"
const { registerHooks } = await import("node:module")
const { resolve } = await import("./importmap_hooks.mjs")
registerHooks({ resolve })

const require = createRequire(join(process.env.FRANKMD, "package.json"))
const { JSDOM } = require("jsdom")
export const dom = new JSDOM("<!doctype html><html><head></head><body></body></html>", { url: "http://localhost:3000/", pretendToBeVisual: true })
const w = dom.window
globalThis.window = w
for (const k of ["document", "Node", "Element", "HTMLElement", "DocumentFragment", "NodeFilter", "DOMParser",
  "HTMLTemplateElement", "Text", "Comment", "MutationObserver", "getComputedStyle", "CSS", "Image", "FormData",
  "HTMLFormElement", "NamedNodeMap", "HTMLCollection", "NodeList", "Range", "Selection", "CustomEvent", "Event", "KeyboardEvent"]) {
  if (w[k] !== undefined) { try { globalThis[k] = w[k] } catch {} }
}
try { Object.defineProperty(globalThis, "navigator", { value: w.navigator, configurable: true }) } catch {}
globalThis.requestAnimationFrame ||= (cb) => setTimeout(cb, 0)
