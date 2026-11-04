// Golden fixtures for app/javascript/lib/draft_storage.js (localStorage-backed drafts,
// recovery conflicts and legacy backups). Scenario-based: each case seeds an in-memory
// Storage (insertion-ordered Map, like FrankMD's MemoryStorage test helper), runs steps
// against a DraftStorage, and records every step result plus the final storage contents.
//
// Determinism: crypto.randomUUID -> "uuid-0001", "uuid-0002", ... (per case);
// Date.now -> 1700000000000 + 1000*n (n = number of previous Date.now calls in the case).
//
// usage: node tools/golden/js/draft_storage.mjs
import { fixture, writeFixture, lib } from "../lib/golden.mjs"
import { enc } from "./runtime/common.mjs"

const GEN = "tools/golden/js/draft_storage.mjs"
const { DraftStorage } = await lib("app/javascript/lib/draft_storage.js")

let uuid = 0, now = 0
Object.defineProperty(globalThis, "crypto", { value: { randomUUID: () => `uuid-${String(++uuid).padStart(4, "0")}` }, configurable: true })
Date.now = () => 1700000000000 + 1000 * now++

class FaultyStorage {
  constructor(entries = []) { this.values = new Map(entries); this.faults = [] }
  fault(method, key) { return this.faults.find(f => f.method === method && (f.key === undefined || f.key === key) && (f.prefix === undefined || key?.startsWith(f.prefix))) }
  get length() { const f = this.fault("length"); if (f) throw new Error(f.message || "length failed"); return this.values.size }
  key(i) { const f = this.fault("key"); if (f) throw new Error(f.message || "key failed"); return Array.from(this.values.keys())[i] ?? null }
  getItem(k) { const f = this.fault("getItem", k); if (f?.mode === "noop") return null; if (f) throw new Error(f.message || "getItem failed"); return this.values.has(k) ? this.values.get(k) : null }
  setItem(k, v) { const f = this.fault("setItem", k); if (f?.mode === "noop") return; if (f) throw new Error(f.message || "quota exceeded"); this.values.set(k, String(v)) }
  removeItem(k) { const f = this.fault("removeItem", k); if (f?.mode === "noop") return; if (f) throw new Error(f.message || "storage unavailable"); this.values.delete(k) }
}

function runScenario({ initial = [], steps }) {
  uuid = 0; now = 0
  const storage = new FaultyStorage(initial)
  let providerFails = false
  const ds = new DraftStorage(() => { if (providerFails) throw new Error("localStorage unavailable"); return storage })
  const results = []
  for (const s of steps) {
    if (s.fault) { storage.faults.push(s.fault); results.push(null); continue }
    if (s.clearFaults) { storage.faults = []; results.push(null); continue }
    if ("providerFails" in s) { providerFails = s.providerFails; results.push(null); continue }
    if (s.raw) { storage.values.set(s.raw[0], s.raw[1]); results.push(null); continue }
    try { results.push(enc(ds[s.call](...(s.args || [])))) } catch (e) { results.push({ $thrown: { name: e.constructor.name, message: e.message } }) }
  }
  return { results, storage_after: [...storage.values.entries()] }
}

const draft = (path, content, baseRevision, draftRevision, updatedAt) => JSON.stringify({ schemaVersion: 1, path, content, baseRevision, draftRevision, updatedAt })
const conflict = (path, sourcePath, conflictId, content, baseRevision, draftRevision, updatedAt) => JSON.stringify({ schemaVersion: 1, path, sourcePath, conflictId, content, baseRevision, draftRevision, updatedAt })
const W = (p, c = "local edit", r = "revision") => ({ call: "writeDraft", args: [p, c, r] })
const R = (p) => ({ call: "readDraft", args: [p] })

const S = {
  // ---- from test/javascript/lib/draft_storage.test.js
  "write+read empty draft": { steps: [W("folder/note.md", "", "server-revision"), R("folder/note.md")] },
  "unchanged snapshot keeps revision": { steps: [W("note.md", "a", "r1"), W("note.md", "a", "r1"), W("note.md", "a", "r2"), W("note.md", "b", "r2")] },
  "removeDraftIfRevision only matching": { steps: [W("note.md"), { call: "removeDraftIfRevision", args: ["note.md", "other"] }, { call: "removeDraftIfRevision", args: ["note.md", "uuid-0001"] }, R("note.md"), { call: "removeDraftIfRevision", args: ["missing.md", "x"] }] },
  "remap file draft": { steps: [W("old.md"), { call: "remapDrafts", args: ["old.md", "new.md"] }, R("old.md"), R("new.md")] },
  "remap folder and descendants only": { steps: [W("docs/a.md"), W("docs/sub/b.md"), W("docs-old/c.md"), W("docs.md"), { call: "remapDrafts", args: ["docs", "guides", "folder"] }, { call: "listDrafts" }] },
  "remap legacy backup rename": { initial: [["frankmd:backup:old.md", JSON.stringify({ content: "unsaved edit", timestamp: 123 })]], steps: [{ call: "remapDrafts", args: ["old.md", "new.md"] }] },
  "remap legacy backups in folder": { initial: [["frankmd:backup:docs/a.md", JSON.stringify({ content: "a", timestamp: 1 })], ["frankmd:backup:docs/sub/b.md", JSON.stringify({ content: "b", timestamp: 2 })], ["frankmd:backup:docs-old/c.md", JSON.stringify({ content: "c", timestamp: 3 })]], steps: [{ call: "remapDrafts", args: ["docs", "archive/docs", "folder"] }] },
  "legacy backup collision": { initial: [["frankmd:backup:old.md", JSON.stringify({ content: "src", timestamp: 1 })], ["frankmd:backup:new.md", JSON.stringify({ content: "dst", timestamp: 2 })]], steps: [{ call: "remapDrafts", args: ["old.md", "new.md"] }] },
  "legacy backup identical at destination": { initial: [["frankmd:backup:old.md", JSON.stringify({ content: "same", timestamp: 1 })], ["frankmd:backup:new.md", JSON.stringify({ content: "same", timestamp: 1 })]], steps: [{ call: "remapDrafts", args: ["old.md", "new.md"] }] },
  "legacy backup destination write fails": { initial: [["frankmd:backup:old.md", JSON.stringify({ content: "unsaved edit", timestamp: 123 })]], steps: [{ fault: { method: "setItem", key: "frankmd:backup:new.md" } }, { call: "remapDrafts", args: ["old.md", "new.md"] }] },
  "legacy backup source removal fails then retry": { initial: [["frankmd:backup:old.md", JSON.stringify({ content: "unsaved edit", timestamp: 123 })]], steps: [{ fault: { method: "removeItem", key: "frankmd:backup:old.md" } }, { call: "remapDrafts", args: ["old.md", "new.md"] }, { clearFaults: true }, { call: "remapDrafts", args: ["old.md", "new.md"] }] },
  "draft collision preserves both": { steps: [W("old.md", "source", "revision"), W("new.md", "destination", "other-revision"), { call: "remapDrafts", args: ["old.md", "new.md"] }, R("old.md"), R("new.md"), { call: "listDraftConflicts", args: ["new.md"] }] },
  "folder remap with one collision": { steps: [W("docs/a.md", "A"), W("docs/b.md", "B"), W("docs/sub/c.md", "C"), W("guides/b.md", "existing"), { call: "remapDrafts", args: ["docs", "guides", "folder"] }, { call: "listDrafts" }, { call: "listDraftConflicts", args: ["guides/a.md"] }, { call: "listDraftConflicts", args: ["guides/b.md"] }] },
  "collision retried is idempotent": { steps: [W("old.md", "s"), W("new.md", "d", "x"), { call: "remapDrafts", args: ["old.md", "new.md"] }, W("old.md", "s"), { call: "remapDrafts", args: ["old.md", "new.md"] }, { call: "listDraftConflicts", args: ["new.md"] }] },
  "destination draft write fails": { steps: [W("old.md"), { fault: { method: "setItem", key: "frankmd:draft:new.md" } }, { call: "remapDrafts", args: ["old.md", "new.md"] }, R("old.md"), R("new.md")] },
  "destination write silently dropped": { steps: [W("old.md"), { fault: { method: "setItem", key: "frankmd:draft:new.md", mode: "noop" } }, { call: "remapDrafts", args: ["old.md", "new.md"] }, R("old.md"), R("new.md")] },
  "source removal fails keeps both": { steps: [W("old.md"), { fault: { method: "removeItem", key: "frankmd:draft:old.md" } }, { call: "remapDrafts", args: ["old.md", "new.md"] }, R("old.md"), R("new.md")] },
  "removeDrafts folder boundary": { steps: [W("docs/index.md", "index", "rev-1"), W("docs/guides/setup.md", "guide", "rev-2"), W("docs-old/keep.md", "keep", "rev-3"), { call: "writeBackup", args: ["docs/index.md", "legacy"] }, { call: "writeBackup", args: ["docs-old/keep.md", "keep legacy"] }, { call: "removeDrafts", args: ["docs", "folder"] }, R("docs-old/keep.md"), { call: "readBackup", args: ["docs/index.md"] }] },
  "removeDrafts file only exact": { steps: [W("a.md"), W("a.md.bak"), W("a.md/child.md"), { call: "removeDrafts", args: ["a.md"] }, { call: "listDrafts" }] },
  "removeDrafts removes conflicts": { steps: [W("docs/old.md", "source"), W("docs/new.md", "destination", "other"), { call: "remapDrafts", args: ["docs/old.md", "docs/new.md"] }, { call: "listDraftConflicts", args: ["docs/new.md"] }, { call: "removeDrafts", args: ["docs", "folder"] }, { call: "listDraftConflicts", args: ["docs/new.md"] }] },
  "removeDrafts errors reported": { steps: [W("note.md"), { fault: { method: "removeItem" } }, { call: "removeDrafts", args: ["note.md"] }] },
  "malformed records discarded": { initial: [["frankmd:draft:broken.md", "{bad json"], ["frankmd:draft:wrong-path", draft("another.md", "untrusted", "baseline", "revision", 1)], ["another-app:key", "leave me"]], steps: [R("broken.md"), { call: "listDrafts" }, R("wrong-path"), { call: "removeAllDrafts" }] },
  "newer schema preserved": { initial: [["frankmd:draft:note.md", JSON.stringify({ schemaVersion: 2, content: "future draft" })]], steps: [R("note.md"), W("note.md", "x", "y"), { call: "listDrafts" }] },
  "storage read failure": { steps: [{ fault: { method: "getItem", message: "QuotaExceededError" } }, R("note.md"), W("note.md", "content", "baseline")] },
  "storage write failure": { steps: [{ fault: { method: "setItem", message: "QuotaExceededError" } }, W("note.md", "content", "baseline")] },
  "storage removal failure": { steps: [W("note.md", "content", "baseline"), { fault: { method: "removeItem", message: "QuotaExceededError" } }, { call: "removeDraftIfRevision", args: ["note.md", "uuid-0001"] }, { call: "removeDraft", args: ["note.md"] }, { call: "removeBackup", args: ["note.md"] }, { call: "removeAllDrafts" }] },
  "legacy backups read/remove": { steps: [{ call: "writeBackup", args: ["folder/note.md", "offline"] }, { call: "readBackup", args: ["folder/note.md"] }, { call: "removeBackup", args: ["folder/note.md"] }, { call: "readBackup", args: ["folder/note.md"] }] },
  // ---- additional edge cases
  "writeDraft argument validation": { steps: [{ call: "writeDraft", args: [null, "c", "r"] }, { call: "writeDraft", args: ["p", null, "r"] }, { call: "writeDraft", args: ["p", "c", ""] }, { call: "writeDraft", args: ["p", "c", 5] }, { call: "writeDraft", args: ["", "c", "r"] }] },
  "remapDrafts argument validation": { steps: [{ call: "remapDrafts", args: ["", "b"] }, { call: "remapDrafts", args: ["a", null] }, { call: "remapDrafts", args: ["same.md", "same.md"] }, { call: "remapDrafts", args: ["none.md", "other.md"] }] },
  "removeDrafts validation": { steps: [{ call: "removeDrafts", args: [""] }, { call: "removeDrafts", args: [null, "folder"] }] },
  "unicode and special path keys": { steps: [W("Ação/日本語 note.md", "ç"), W("100% done?.md"), W("a+b&c=d#e.md"), W("שלום/👨‍👩‍👧.md"), W("../escape.md"), W("C:\\win\\path.md"), { call: "listDrafts" }, { call: "writeBackup", args: ["Ação/x.md", "b"] }, { call: "draftKey", args: ["Ação/日本語 note.md"] }, { call: "backupKey", args: ["Ação/日本語 note.md"] }, { call: "draftConflictKey", args: ["a b.md", "id-1"] }, { call: "draftConflictPathPrefix", args: ["a:b.md"] }] },
  "undecodable draft key skipped by removeDrafts": { initial: [["frankmd:draft:%E0%A4%A", draft("x", "c", "b", "r", 1)], ["frankmd:draft:%", "junk"]], steps: [{ call: "removeDrafts", args: ["x"] }, { call: "listDrafts" }] },
  "listDrafts requires canonical key": { initial: [["frankmd:draft:a%2Fb.md", draft("a/b.md", "c", "b", "r", 1)], ["frankmd:draft:a/b.md", draft("a/b.md", "d", "b", "r2", 2)]], steps: [{ call: "listDrafts" }, R("a/b.md")] },
  "validDraft rejects fields": { initial: [
    ["frankmd:draft:a.md", JSON.stringify({ schemaVersion: 1, path: "a.md", content: "c", baseRevision: "", draftRevision: "r", updatedAt: 1 })],
    ["frankmd:draft:b.md", JSON.stringify({ schemaVersion: 1, path: "b.md", content: 5, baseRevision: "b", draftRevision: "r", updatedAt: 1 })],
    ["frankmd:draft:c.md", JSON.stringify({ schemaVersion: 1, path: "c.md", content: "c", baseRevision: "b", draftRevision: "r", updatedAt: "1" })],
    ["frankmd:draft:d.md", JSON.stringify({ schemaVersion: "1", path: "d.md", content: "c", baseRevision: "b", draftRevision: "r", updatedAt: 1 })],
    ["frankmd:draft:e.md", "null"], ["frankmd:draft:f.md", "[]"], ["frankmd:draft:g.md", "\"str\""],
    ["frankmd:draft:h.md", JSON.stringify({ schemaVersion: 1, path: "h.md", content: "ok", baseRevision: "b", draftRevision: "r", updatedAt: 1.5, extra: true })]],
    steps: ["a.md", "b.md", "c.md", "d.md", "e.md", "f.md", "g.md", "h.md"].map(R) },
  "backup validation": { initial: [["frankmd:backup:a.md", "{bad"], ["frankmd:backup:b.md", JSON.stringify({ content: "x" })], ["frankmd:backup:c.md", JSON.stringify({ content: "x", timestamp: 5 })]],
    steps: [{ call: "readBackup", args: ["a.md"] }, { call: "readBackup", args: ["b.md"] }, { call: "readBackup", args: ["c.md"] }, { call: "removeBackupIfSnapshot", args: ["c.md", "x", 6] }, { call: "removeBackupIfSnapshot", args: ["c.md", "y", 5] }, { call: "removeBackupIfSnapshot", args: ["c.md", "x", 5] }, { call: "removeBackupIfSnapshot", args: ["zz.md", "x", 5] }, { call: "writeBackup", args: ["p", 5] }, { call: "removeAllBackups" }] },
  "preserveDraftConflict + list sorted": { steps: [{ call: "preserveDraftConflict", args: ["dest.md", { schemaVersion: 1, path: "src.md", content: "c2", baseRevision: "b", draftRevision: "r2", updatedAt: 20 }] }, { call: "preserveDraftConflict", args: ["dest.md", { schemaVersion: 1, path: "src.md", content: "c1", baseRevision: "b", draftRevision: "r1", updatedAt: 10 }] }, { call: "preserveDraftConflict", args: ["dest.md", { path: "x" }] }, { call: "preserveDraftConflict", args: ["", { schemaVersion: 1, path: "s", content: "", baseRevision: "b", draftRevision: "r", updatedAt: 1 }] }, { call: "listDraftConflicts", args: ["dest.md"] }, { call: "listDraftConflicts", args: ["dest"] }, { call: "removeDraftConflictIfRevision", args: ["dest.md", "uuid-0001", "wrong"] }, { call: "removeDraftConflictIfRevision", args: ["dest.md", "uuid-0001", "r2"] }, { call: "listDraftConflicts", args: ["dest.md"] }] },
  "conflict key path mismatch ignored": { initial: [["frankmd:draft-conflict:a.md:id1", conflict("b.md", "s.md", "id1", "c", "b", "r", 1)], ["frankmd:draft-conflict:a.md:id2", conflict("a.md", "s.md", "other", "c", "b", "r", 1)], ["frankmd:draft-conflict:a.md:id3", "{bad"]], steps: [{ call: "listDraftConflicts", args: ["a.md"] }] },
  "folder remap moves existing conflicts": { initial: [["frankmd:draft-conflict:docs%2Fa.md:c1", conflict("docs/a.md", "x.md", "c1", "old", "b", "r", 5)]], steps: [W("docs/a.md", "A"), { call: "remapDrafts", args: ["docs", "notes", "folder"] }, { call: "listDraftConflicts", args: ["notes/a.md"] }, { call: "listDrafts" }] },
  "conflict destination exists differs": { initial: [["frankmd:draft-conflict:old.md:c1", conflict("old.md", "x.md", "c1", "A", "b", "r", 5)], ["frankmd:draft-conflict:new.md:c1", conflict("new.md", "x.md", "c1", "DIFFERENT", "b", "r", 5)]], steps: [{ call: "remapDrafts", args: ["old.md", "new.md"] }] },
  "provider throws": { steps: [{ providerFails: true }, R("a.md"), W("a.md"), { call: "listDrafts" }, { call: "removeDrafts", args: ["a.md"] }, { call: "remapDrafts", args: ["a.md", "b.md"] }, { call: "readBackup", args: ["a.md"] }, { call: "removeAllDrafts" }] },
  "key enumeration throws": { steps: [W("a.md"), { fault: { method: "key" } }, { call: "removeDrafts", args: ["a.md"] }, { call: "listDrafts" }, { call: "remapDrafts", args: ["a.md", "b.md"] }] },
  "pathMatches table": { steps: [["a", "a", "file"], ["a/b", "a", "file"], ["a/b", "a", "folder"], ["ab", "a", "folder"], ["a", "a", "folder"], ["a//b", "a/", "folder"], ["", "", "folder"], ["/x", "", "folder"]].map(a => ({ call: "pathMatches", args: a })) },
  "rename folder to nested of itself": { steps: [W("a/x.md"), W("a/b/y.md"), { call: "remapDrafts", args: ["a", "a/b", "folder"] }, { call: "listDrafts" }] },
  "remap CRLF and large content": { steps: [W("old.md", "line1\r\nline2\rline3\n" + "x".repeat(5000), "rev"), { call: "remapDrafts", args: ["old.md", "new.md"] }, R("new.md")] },
}

const cases = Object.entries(S).map(([name, sc]) => ({ name, fn: "scenario", input: enc(sc), expected: runScenario(sc) }))
writeFixture("js_draft_storage", fixture({ source: "app/javascript/lib/draft_storage.js", generator: GEN,
  notes: "Scenario fixtures. input.initial = ordered [key, raw] storage seed; input.steps: {call, args} invokes DraftStorage method; {fault:{method,key?,prefix?,mode?:'throw'|'noop',message?}} installs a storage fault (throw new Error(message|default) or silently do nothing; getItem noop returns null); {clearFaults}; {providerFails:bool} makes the storageProvider throw Error('localStorage unavailable'); {raw:[k,v]} sets a key directly. Fault/control steps yield null results. expected.results[i] = encoded return value (Error -> {$error:{name,message}}); expected.storage_after = final ordered entries (Map insertion order; real localStorage key() order is browser-defined). crypto.randomUUID pinned to uuid-0001.. per case; Date.now pinned to 1700000000000 + 1000*n. Keys: frankmd:draft:<encodeURIComponent(path)>, frankmd:draft-conflict:<encodeURIComponent(path)>:<conflictId>, frankmd:backup:<raw path>.",
  cases }))
