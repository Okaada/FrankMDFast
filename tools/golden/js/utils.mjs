// Golden fixtures for FrankMD's pure JS utility libs (app/javascript/lib/*_utils.js & co).
//
// Inputs = (1) every distinct call the ORIGINAL vitest suite makes to these modules
// (harvested by harvest/harvest.sh into harvest/inputs/utils.jsonl, capped per fn)
// + (2) the hand-written edge corpus below. Expected values are computed here by
// calling the real FrankMD module in plain node.
//
// usage: node tools/golden/js/utils.mjs            (uses committed harvest file)
//        REHARVEST=1 node tools/golden/js/utils.mjs (re-runs FrankMD's vitest suite first)
process.env.TZ = "UTC" // Date-based helpers use local time; pin it.

import { readFileSync, existsSync, writeFileSync, mkdirSync } from "node:fs"
import { execFileSync } from "node:child_process"
import { createHash } from "node:crypto"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import { lib, fixture, writeFixture, capture, encode, decode } from "../lib/golden.mjs"

const HERE = dirname(fileURLToPath(import.meta.url))
const HARVEST = join(HERE, "harvest/inputs/utils.jsonl")
const CAP = 40 // max harvested cases per function

const MODULES = ["text_utils", "markdown_utils", "table_utils", "indent_utils", "diff_utils", "stats_utils",
  "tree_utils", "url_utils", "find_utils", "task_utils", "typewriter_utils", "text_editor_utils",
  "new_note_utils", "s3_key", "line_numbers", "menu_bounds", "scroll_utils"]

// Functions taking a trailing Date with a `new Date()` default: harvested calls that
// omitted it are non-deterministic and dropped (explicit-date cases are added below).
const DATE_DEFAULT = { "new_note_utils.hugoDatePath": 0, "s3_key.defaultS3Prefix": 0, "s3_key.defaultS3Key": 1 }
// Functions needing a DOM element: exercised via a stub in custom cases only.
const DOM_FNS = new Set(["menu_bounds.clampMenuToViewport", "menu_bounds.toggleMenu"])

if (process.env.REHARVEST || !existsSync(HARVEST)) {
  const tmp = join(process.env.GOLDEN_TMP || "/tmp", "utils-harvest.jsonl")
  mkdirSync(dirname(HARVEST), { recursive: true })
  execFileSync(join(HERE, "harvest/harvest.sh"), [tmp, ...MODULES], { stdio: "inherit" })
  // cap + stable order, then commit-able file
  // vitest workers append in nondeterministic order: dedupe across workers, then pick
  // CAP per fn by content hash (stable, and spread over the whole suite rather than first-N)
  const per = new Map()
  for (const line of readFileSync(tmp, "utf8").split("\n").filter(Boolean)) {
    const r = JSON.parse(line)
    const k = `${r.mod}.${r.fn}`
    if (!per.has(k)) per.set(k, new Map())
    const json = JSON.stringify({ mod: r.mod, fn: r.fn, args: r.args })
    per.get(k).set(createHash("sha1").update(json).digest("hex"), json)
  }
  const out = [...per.keys()].sort().flatMap(k => [...per.get(k).entries()].sort(([a], [b]) => (a < b ? -1 : 1)).slice(0, CAP).map(([, j]) => j)).join("\n") + "\n"
  writeFileSync(HARVEST, out)
}

// ---------------------------------------------------------------- edge corpus
const S = {
  empty: "", space: " ", ws: " \t   ", nbsp: "a b", tab: "a\tb",
  crlf: "line1\r\nline2\r\nline3", cr: "line1\rline2", lf_end: "a\nb\n", nl_only: "\n\n\n",
  accents: "Olá ação café résumé naïve Ünïcödé", combining: "été", cjk: "日本語のテキスト 中文 한국어",
  hebrew: "שלום עולם", arabic: "مرحبا بالعالم", emoji: "hi 👋 there 🎉", zwj: "family 👨‍👩‍👧 flag 🇧🇷",
  astral: "𝔘𝔫𝔦𝔠𝔬𝔡𝔢", long: "lorem ipsum dolor sit amet ".repeat(400), html: `<a href="x" onclick='y'>&amp;</a>`,
  ctrl: "a\u0000b\u0007c\u001fd\u007fe", zw: "a​b﻿c",
}
const TEXTS = Object.values(S)
const T = (name) => name

const custom = []
const add = (mod, fn, args, name) => custom.push({ mod, fn, args, name })

// text_utils
for (const t of [...TEXTS, "<script>alert('xss')</script>", "&lt;already&gt;", `"'"'`, null, undefined, 0, 42, false])
  { add("text_utils", "escapeHtml", [t]); if (typeof t === "string" || t == null) add("text_utils", "escapeHtmlString", [t]) }
for (const u of ["http://x/a.png", "HTTPS://X/A.PNG", "HtTp://x", "data:image/svg+xml;base64,AA", "data:text/html,<b>", "DATA:IMAGE/png,x",
  "blob:null/1", "BLOB:x", "/", "//", "/\\", "/a b.png", "/a\tb", "/a%20b.png", "/é.png", "/日本.png", "./rel.png", "rel.png", "../up.png",
  "javascript:alert(1)", "JaVaScRiPt:alert(1)", "vbscript:x", "file:///etc/passwd", "ftp://x", "mailto:x", "https://x/\u0000", "https://x/\u007f",
  "https://x/ ", "https://x/ ", "  /a.png", "/a.png ", "", null, undefined, 1, {}, ["/a"], "https:", "http:/x", "\\\\server\\a.png", "/\\\\x"])
  add("text_utils", "isSafeImageUrl", [u])
for (const [s, q] of [["hello", "xyz"], ["", ""], ["", "a"], ["a", ""], ["abc", "abc"], ["abc", "abcd"], ["aaa", "aa"], ["notes/daily-log_2024.md", "dl"],
  ["notes/daily-log_2024.md", "log"], ["Notes/README.md", "readme"], ["Notes/README.md", "README"], ["café/résumé.md", "résumé"], ["café/résumé.md", "resume"],
  ["日本語/メモ.md", "メモ"], ["emoji/👋hello.md", "👋"], ["a/b/c/d/e/f/g/h.md", "abcdefgh"], ["x".repeat(100), "xxx"], ["a-b_c/d", "bcd"], ["family 👨‍👩‍👧", "👧"],
  ["src/button", "button"], ["srcxbutton", "button"], ["path/with spaces/file name.md", "fn"], ["ABC", "abc"]])
  add("text_utils", "fuzzyScore", [s, q])
for (const [a, b] of [["", ""], ["a", ""], ["kitten", "sitting"], ["flaw", "lawn"], ["café", "cafe"], ["é", "é"], ["👋", "👍"], ["👋", ""], ["日本語", "日本"],
  ["שלום", "שלם"], ["javascript", "javscript"], ["py", "python"], ["rust", "ruby"], ["ABC", "abc"], ["a".repeat(60), "b".repeat(60)], ["sh", "bash"], ["golang", "go"]])
  add("text_utils", "levenshteinDistance", [a, b])

// markdown_utils
const TABLE = "| A | B |\n| --- | --- |\n| 1 | 2 |"
const tableTexts = [
  TABLE, `intro\n\n${TABLE}\n\noutro`, "| A |\n|---|\n| 1 |", "|A|B|\n|:-|-:|\n|1|2|", "| A | B |\n| --- | --- |", "  | A | B |\n  | --- | --- |\n  | 1 | 2 |",
  "| A | B |\r\n| --- | --- |\r\n| 1 | 2 |", "| A | B |\n|---|---\n| 1 | 2 |", "| A | B |\n| - - | - |\n| 1 | 2 |", "| A | B |\n| ::: | --- |\n| 1 | 2 |",
  "| 日本 | שלום |\n| --- | --- |\n| 👨‍👩‍👧 | é |", "text | not table\n---|---", "| a |\n| b |\n| --- |\n| c |", "", "|", "|\n|", "| A |\n| --- |\n\n| B |\n| --- |",
  "A | B\n--- | ---\n1 | 2", "| A | B |\n|-----|-----|\n| 1 | 2 |\nnot a row\n| x | y |",
]
for (const t of tableTexts) for (const p of [0, 1, Math.floor(t.length / 2), t.length - 1, t.length, t.length + 5, -1, t.indexOf("\n") + 1].filter((v, i, a) => a.indexOf(v) === i))
  add("markdown_utils", "findTableAtPosition", [t, p])
const codeTexts = ["```js\nconst x = 1\n```", "pre\n```\nplain\n```\npost", "```python\na\n```\n\n```ruby\nb\n```", "```\nunclosed", "```c++\nx\n```",
  "```js \nx\n```", "~~~js\nx\n~~~", "````js\nx\n````", "```js\r\nx\r\n```", "```日本\nx\n```", "```\n```", "``` \n```", "text ```inline``` text", "    ```js\n    x\n    ```"]
for (const t of codeTexts) for (const p of [0, 3, 6, Math.floor(t.length / 2), t.length - 1, t.length, t.length + 3].filter((v, i, a) => a.indexOf(v) === i))
  add("markdown_utils", "findCodeBlockAtPosition", [t, p])

// table_utils
for (const t of [...tableTexts, ["| a | b |", "|---|---|", "| 1 | 2 |"], ["| a |", "", "   ", "| b |"], "| a | b | c |\n| 1 |\n| 1 | 2 | 3 | 4 |",
  "| a \\| b | c |\n|---|---|\n| `x|y` | z |", "|a||b|\n|-|-|-|\n||||", "no pipes at all", "| only leading", "trailing only |", "|  spaced  |   cells   |",
  "| :---: | --- |\n| a | b |", "|---|\n|---|", "| - |"])
  add("table_utils", "parseMarkdownTable", [t])
const TD = [["Name", "Age"], ["Alice", "30"], ["Bob", "4"]]
for (const d of [TD, [], null, [["a"]], [["a", "b", "c"], ["1"], ["1", "2", "3", "4"]], [["日本語", "é"], ["👨‍👩‍👧", "שלום"]], [[""], [""]], [["x".repeat(50)]],
  [["a|b", "c"]], [[], []], [["Header"]]])
  add("table_utils", "generateMarkdownTable", [d])
for (const [a, b] of [[0, 1], [1, 0], [0, 0], [0, 5], [-1, 0]]) add("table_utils", "swapColumns", [TD, a, b])
for (const [a, b] of [[0, 1], [1, 2], [2, 2], [0, 9]]) add("table_utils", "swapRows", [TD, a, b])
for (const c of [0, 1, 5, -1]) add("table_utils", "deleteColumn", [TD, c])
for (const r of [0, 1, 2, 9, -1]) add("table_utils", "deleteRow", [TD, r])
for (const h of [undefined, "New", "", "日本"]) add("table_utils", "addColumn", h === undefined ? [TD] : [TD, h])
add("table_utils", "addColumn", [[]]); add("table_utils", "addRow", [TD]); add("table_utils", "addRow", [[]]); add("table_utils", "addRow", [[["a", "b", "c"]]])

// indent_utils
for (const v of [undefined, null, "", 0, "0", 1, "1", 2, "2", 4, "4", 6, 7, "7", 100, -1, "-3", "abc", "3abc", " 3 ", 2.7, "2.9", "0x4", true, NaN, "\t"])
  add("indent_utils", "parseIndentSetting", [v])
const IND = ["a\nb", "", "\n", "  a\n    b\n\tc\n d", "a\r\nb", "日本\n👋", "\t\ta", "   ", "a\n\n b"]
for (const t of IND) for (const ind of ["  ", "\t", "    ", " "]) { add("indent_utils", "indentLines", [t, ind]); add("indent_utils", "unindentLines", [t, ind]) }
add("indent_utils", "unindentLines", ["      six\n  two\n\t\ttab", "    "]); add("indent_utils", "unindentLines", [" \tmixed", "  "]); add("indent_utils", "unindentLines", ["\t  x", "\t"])

// diff_utils
for (const t of ["", " ", "a b", "  lead", "trail  ", "a\tb\nc\r\nd", "日本語 テキスト", "👨‍👩‍👧 x", "a b", "word"]) add("diff_utils", "tokenize", [t])
for (const [a, b] of [["", ""], ["a", ""], ["", "a"], ["the cat sat", "the cat sat"], ["the cat sat", "the dog sat"], ["I has a apple", "I have an apple"],
  ["a b c", "c b a"], ["teh  quick\nbrown", "the quick\nbrown fox"], ["Olá mundo", "Olá, mundo!"], ["日本語 です", "日本語 でした"], ["x x x", "x x"],
  ["a\r\nb", "a\nb"], ["one two three four five six", "one three four six seven"]])
  add("diff_utils", "computeWordDiff", [a, b])
add("diff_utils", "lcsWordDiff", [[], []]); add("diff_utils", "lcsWordDiff", [["a", " ", "b"], ["b", " ", "a"]]); add("diff_utils", "lcsWordDiff", [["x"], ["x", "x"]])

// stats_utils
for (const t of [...TEXTS, null, undefined, "one", "one two", "  one   two  ", "word ".repeat(199), "word ".repeat(200), "word ".repeat(201), "　全角　スペース", "a b"])
  add("stats_utils", "calculateStats", [t])
for (const m of [0, 0.5, 1, 1.5, 2, 10, 60, 600, -1]) add("stats_utils", "formatReadTime", [m])
for (const b of [0, 1, 512, 1023, 1024, 1025, 1536, 10239, 10240, 1048575, 1048576, 5.5 * 1048576, 1073741824, 1099511627776, 1125899906842624, 0.5, -1, -1024])
  add("stats_utils", "formatFileSize", [b])

// tree_utils
const TREE = [
  { type: "folder", name: "a", path: "a", children: [{ type: "file", name: "x.md", path: "a/x.md" }, { type: "folder", name: "b", path: "a/b", children: [{ type: "file", name: "y.md", path: "a/b/y.md" }] }] },
  { type: "file", name: "z.md", path: "z.md" }, { type: "folder", name: "empty", path: "empty", children: [] }, { type: "folder", name: "nochildren", path: "nochildren" },
  { type: "image", name: "p.png", path: "p.png" }, { type: "file", name: "日本.md", path: "日本.md" }]
add("tree_utils", "flattenTree", [TREE]); add("tree_utils", "flattenTree", [[]]); add("tree_utils", "flattenTree", [null]); add("tree_utils", "flattenTree", [undefined])
add("tree_utils", "flattenTree", [TREE, [{ type: "file", path: "pre-existing" }]])

// url_utils
for (const p of ["", null, undefined, "a/b", "/abs/path", "trailing/", "a//b", "hello world/test file.md", "100%.md", "a+b&c=d.md", "#hash?q.md", "café/résumé.md",
  "日本語/ノート.md", "שלום.md", "👨‍👩‍👧.md", "é.md", "a\\b.md", "..", "../x", "%2e%2e/x", "!'()*~.md", "a;b,c:d@e$.md", "tab\there.md", "nl\nhere.md"])
  add("url_utils", "encodePath", [p])
for (const u of ["https://www.youtube.com/watch?v=dQw4w9WgXcQ", "http://youtube.com/watch?v=dQw4w9WgXcQ&t=1", "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
  "https://www.youtube.com/watch?feature=share&v=dQw4w9WgXcQ", "https://youtu.be/dQw4w9WgXcQ?si=abc", "https://www.youtube.com/shorts/dQw4w9WgXcQ",
  "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ", "https://youtube.com/embed/dQw4w9WgXcQ?start=3", "https://youtube.com/v/dQw4w9WgXcQ",
  "https://www.youtube.com/live/dQw4w9WgXcQ", "https://youtu.be/short", "https://youtu.be/dQw4w9WgXcQextra", "dQw4w9WgXcQ", " dQw4w9WgXcQ", "dQw4w9WgXc",
  "dQw4w9WgXcQQ", "a-b_c-d_e-f", "!!!!!!!!!!!", "https://evil.com/?u=youtube.com/watch?v=dQw4w9WgXcQ", "HTTPS://WWW.YOUTUBE.COM/WATCH?V=dQw4w9WgXcQ",
  "https://www.youtube.com/watch?v=日本語日本語日本語日本", "", null, undefined, "https://vimeo.com/123"])
  add("url_utils", "extractYouTubeId", [u])

// find_utils
const FT = "Hello hello HELLO héllo\nfoo.bar foo*bar\n日本語 日本\n👋 👋\nCafé café CAFÉ"
for (const [term, opt] of [["hello", {}], ["hello", { caseSensitive: true }], ["foo.bar", {}], ["foo.bar", { useRegex: true }], ["fo+", { useRegex: true }],
  ["(", { useRegex: true }], ["(", {}], ["", {}], ["日本", {}], ["👋", {}], ["café", {}], ["CAFÉ", {}], ["x*", { useRegex: true }], ["^", { useRegex: true }],
  ["$", { useRegex: true }], ["\\w+", { useRegex: true }], ["(h)(e)", { useRegex: true }], ["(?<n>foo)", { useRegex: true }], ["\\n", { useRegex: true }],
  ["[", { useRegex: true }], ["héllo", { caseSensitive: true }], ["zzz", {}], ["l", { caseSensitive: true }], [".", { useRegex: true }]])
  add("find_utils", "findAllMatches", [FT, term, opt])
add("find_utils", "findAllMatches", ["", "a", {}]); add("find_utils", "findAllMatches", [null, "a"]); add("find_utils", "findAllMatches", ["aaa", "a"])
add("find_utils", "findAllMatches", ["a\r\nb", "\\r?\\n", { useRegex: true }]); add("find_utils", "findAllMatches", ["ab", "", { useRegex: true }])
const M = (t, s, o) => ({ ...o, _m: [t, s, o] })
for (const [text, term, repl, opt] of [["foo bar foo", "foo", "baz", {}], ["foo bar foo", "(f)(o)o", "$2$1-$&-$$-$0-$3-$9", { useRegex: true }],
  ["aaa", "a", "", {}], ["日本 日本", "日本", "にほん", {}], ["x", "x", "$1", {}], ["x", "x", "$1", { useRegex: true }], ["abc", "(b)", "[$01]", { useRegex: true }],
  ["abc", "b", null, { useRegex: true }], ["Hello hello", "hello", "Bye", {}], ["a.b", ".", "!", { useRegex: true }]])
  custom.push({ mod: "find_utils", fn: "replaceMatches", args: [text, "<matches of findAllMatches(text, term, opt)>", repl, opt], derive: { text, term, opt } })
add("find_utils", "replaceMatches", ["abc", [], "x"]); add("find_utils", "replaceMatches", ["", [{ start: 0, end: 0, text: "", groups: [] }], "x"])
add("find_utils", "replaceMatches", ["abcdef", [{ start: 1, end: 2, text: "b", groups: [] }, { start: 4, end: 5, text: "e", groups: [] }], "XY"])
for (const s of ["", null, "a.b*c+d?e^f$g{h}i(j)k|l[m]n\\o", "/slash/", "-dash-", "日本.語"]) add("find_utils", "escapeRegexChars", [s])
const MS = [{ start: 0, end: 3 }, { start: 10, end: 13 }, { start: 20, end: 23 }]
for (const pos of [0, 1, 3, 5, 10, 12, 13, 22, 23, 50, -1]) for (const dir of ["next", "previous", -1, 1, undefined])
  add("find_utils", "findClosestMatchIndex", dir === undefined ? [MS, pos] : [MS, pos, dir])
add("find_utils", "findClosestMatchIndex", [[], 0]); add("find_utils", "findClosestMatchIndex", [null, 0, "previous"])
for (const p of ["", null, "a", "(", ")", "[a-", "a{2,1}", "(?<n>x)", "\\", "*", "(?=x)", "\\p{L}", "x++"]) add("find_utils", "validateRegex", [p])

// task_utils
for (const l of ["- [ ] task", "- [x] done", "- [X] DONE", "* [ ] star", "+ [ ] plus", "1. [ ] ordered", "12) [x] paren", "1234567890. [ ] too many digits",
  "  - [ ] indented", "\t- [ ] tab", "- [ ]", "- [ ]x", "- [x]\ttab after", "-[ ] nospace", "- [  ] two spaces", "- [y] other", "- [ ] 日本語", "> - [ ] quoted",
  "- plain item", "plain text", "", "   ", "  hello  ", "- [ ] - [ ] nested", "• [ ] bullet", "- [ ]\r", " - [ ] nbsp"]) {
  add("task_utils", "taskMarkerRange", [l]); add("task_utils", "toggleTaskMarker", [l]); add("task_utils", "applyTaskToggle", [l])
}

// typewriter_utils
for (const a of [[100, 400], [100, 400, 0.5], [500, 400, 0.3], [0, 0], [-50, 400], [1000, 400, 1], [300, 600, 0], [250.5, 101, 0.5]]) add("typewriter_utils", "calculateTypewriterScroll", a)
for (const [t, p] of [["", 0], ["a\nb\nc", 0], ["a\nb\nc", 2], ["a\nb\nc", 5], ["a\nb\nc", 99], ["a\r\nb", 3], ["日本\n👋\nx", 4], ["\n", 1], ["a\nb", -1]])
  add("typewriter_utils", "getTypewriterSyncData", [t, p])
for (const [c, t] of [[1, 1], [1, 0], [1, 10], [5, 10], [10, 10], [11, 10], [0, 10], [3, 2], [2, 3]]) add("typewriter_utils", "calculateTypewriterSyncRatio", [c, t])

// text_editor_utils
const ED = "first line\nsecond 日本\nthird 👋 line\n\nlast"
for (const [s, e, p, x] of [[0, 5, "**", "**"], [6, 10, "_", "_"], [0, 0, "[", "]()"], [12, 18, "`", "`"], [ED.length, ED.length, "\n", ""], [24, 26, "~~", "~~"], [5, 3, "<", ">"]])
  add("text_editor_utils", "wrapSelection", [ED, s, e, p, x])
for (const [p, x] of [[0, "X"], [ED.length, "\nEND"], [11, "👨‍👩‍👧"], [99, "beyond"], [-3, "neg"], [5, ""]]) add("text_editor_utils", "insertAtPosition", [ED, p, x])
for (const [s, e] of [[0, 0], [3, 3], [0, ED.length], [12, 25], [11, 11], [10, 11], [ED.length, ED.length], [35, 36], [0, 1]])
  add("text_editor_utils", "getLineBoundaries", [ED, s, e])
add("text_editor_utils", "getLineBoundaries", ["", 0, 0]); add("text_editor_utils", "getLineBoundaries", ["a\r\nb", 0, 4])
for (const p of [0, 1, 10, 11, 12, 25, ED.length, 999]) { add("text_editor_utils", "getCursorInfo", [ED, p]); add("text_editor_utils", "getLineAtPosition", [ED, p]) }
add("text_editor_utils", "getCursorInfo", ["", 0]); add("text_editor_utils", "getCursorInfo", ["a\r\nb", 3])
for (const l of [-1, 0, 1, 2, 3, 4, 5, 6, 100]) add("text_editor_utils", "getPositionForLine", [ED, l])
add("text_editor_utils", "getPositionForLine", ["", 1]); add("text_editor_utils", "getPositionForLine", ["a\r\nb", 2])
for (const [s, e, r] of [[0, 5, "FIRST"], [0, ED.length, ""], [6, 6, "ins"], [11, 17, "2nd"], [8, 3, "rev"]]) add("text_editor_utils", "replaceRange", [ED, s, e, r])

// new_note_utils
const D = { $date: "2026-08-18T12:00:00.000Z" }, D2 = { $date: "2025-01-05T23:59:59.000Z" }, D3 = { $date: "1999-12-31T00:00:00.000Z" }
for (const t of ["Hello World", "Olá Mundo! Ação", "Ünïcödé Straße", "Crème Brûlée", "Łódź Żółć", "Dvořák Škoda", "Þór Ægir Øresund", "Ğüşİöç Türkçe", "Tiếng Việt",
  "日本語", "שלום", "مرحبا", "👋 emoji 🎉", "  spaced  out  ", "--dashes--", "a---b", "under_score", "dots.in.name", "CamelCase", "123 numbers 456", "!!!", "", null,
  undefined, "é", "ÀÉÎÕÜ", "İstanbul", "x".repeat(300), "C++ & C#", "file.md", "a/b\\c"])
  add("new_note_utils", "slugify", [t])
for (const d of [D, D2, D3, { $date: "2024-02-29T00:00:00.000Z" }]) add("new_note_utils", "hugoDatePath", [d])
for (const n of ["note", " note ", "", "   ", null, undefined, "a/b", "/", ".", "..", " .. ", "...", ".hidden", "a\\b", "日本", "👋", "note.md", "a\tb", "a\nb"])
  add("new_note_utils", "isValidNoteName", [n])
for (const n of ["note", "", "a/b", "..", "!!!", "日本語", "My Post.md", ".md", "  ", "Olá"]) for (const tpl of ["empty", "hugo", undefined, "other"])
  add("new_note_utils", "canCreateNote", tpl === undefined ? [n] : [n, tpl])
for (const name of ["", "My Note", "My Note.md", "日本語", "!!!", "a/b", "Olá Mundo", "  padded  "]) for (const template of ["empty", "hugo"]) for (const pathStyle of ["dated", "flat"])
  for (const parent of ["", "blog/posts"])
    add("new_note_utils", "previewNotePath", [{ name, parent, template, pathStyle, date: D, namePlaceholder: "name", rootSegment: "root" }])
add("new_note_utils", "previewNotePath", [{ name: "x", date: D }]); add("new_note_utils", "previewNotePath", [{ name: "", template: "hugo", date: D2, namePlaceholder: "nome", rootSegment: "raiz" }])

// s3_key
for (const d of [D, D2, D3]) { add("s3_key", "defaultS3Prefix", [d]); for (const f of ["photo.png", "日本.jpg", "a b.png", "", "x/y.png"]) add("s3_key", "defaultS3Key", [f, d]) }

// line_numbers
for (const v of [0, 1, 2, 3, -1, "0", "1", "2", "2px", " 1", "abc", "", null, undefined, 1.9, true, NaN]) {
  add("line_numbers", "normalizeLineNumberMode", [v]); add("line_numbers", "normalizeLineNumberMode", [v, 0])
}
for (const m of [0, 1, 2, 3, -1, null, "1"]) add("line_numbers", "nextLineNumberMode", [m])

// menu_bounds (pure part)
for (const [top, vh, pad] of [[0, 800], [100, 800], [700, 800], [790, 800], [100.7, 800], [-50, 800], [100, 800, 0], [100, 800, 20], [0, 0]])
  add("menu_bounds", "menuMaxHeight", pad === undefined ? [{ top }, vh] : [{ top }, vh, pad])
// clampMenuToViewport / toggleMenu with a stub element: args = [{rect, hidden}, viewportHeight, viewportWidth]
for (const [left, right, top] of [[100, 300, 50], [2, 200, 50], [-30, 170, 50], [900, 1100, 50], [1000, 1019, 700], [NaN, NaN, 0], [0, 1024, 0]])
  custom.push({ mod: "menu_bounds", fn: "clampMenuToViewport", args: [{ rect: { left, right, top }, hidden: false }, 800, 1024], dom: true })
for (const [hidden, force] of [[true, false], [false, false], [true, true], [false, true]])
  custom.push({ mod: "menu_bounds", fn: "toggleMenu", args: [{ rect: { left: 1000, right: 1100, top: 10 }, hidden }, force], dom: true })

// scroll_utils
for (const a of [[0, 1000, 500], [250, 1000, 500], [500, 1000, 500], [600, 1000, 500], [-10, 1000, 500], [0, 500, 500], [10, 400, 500], [1.5, 3, 1]]) add("scroll_utils", "calculateScrollRatio", a)
for (const a of [[1, 20, 600], [10, 20, 600], [100, 20, 600], [100, 20, 600, 0], [100, 20, 600, 1], [0, 20, 600], [50, 18.5, 333.3, 0.5]]) add("scroll_utils", "calculateScrollForLine", a)
for (const a of [[0, 500, 1000, 100], [250, 500, 1000, 100], [500, 500, 1000, 100], [9999, 500, 1000, 100], [0, 500, 1000, 0], [0, 500, 1000, -5], [0, 0, 0, 10], [10, 100, 1000, 1]])
  add("scroll_utils", "calculateLineFromScroll", a)
for (const a of [[0, 1000, 500], [0.5, 1000, 500], [1, 1000, 500], [1.5, 1000, 500], [-0.5, 1000, 500], [0.5, 500, 500], [0.5, 400, 500]]) add("scroll_utils", "calculateSyncedScrollPosition", a)
for (const a of [[1, 100, 2000, 500], [50, 100, 2000, 500], [100, 100, 2000, 500], [200, 100, 2000, 500], [0, 100, 2000, 500], [1, 1, 2000, 500], [5, 0, 2000, 500], [5, 10, 500, 500]])
  add("scroll_utils", "calculateScrollToCenterLine", a)
for (const a of [[{ top: 500 }, { top: 100 }, 0], [{ top: 50 }, { top: 100 }, 300], [{ top: 10.5 }, { top: 0.25 }, 7.75]]) add("scroll_utils", "scrollTopForElement", a)
const ENT = [{ line: 1, top: 0 }, { line: 5, top: 100 }, { line: 9, top: 250 }, { line: 7, top: 180 }, { line: 12, top: 250 }]
for (const st of [-10, 0, 50, 100, 199, 250, 9999]) add("scroll_utils", "lineAtScroll", [ENT, st])
add("scroll_utils", "lineAtScroll", [[], 0]); add("scroll_utils", "lineAtScroll", [null, 0]); add("scroll_utils", "lineAtScroll", [[{ line: 3, top: 50 }], 0])

// ---------------------------------------------------------------- run
const mods = Object.fromEntries(await Promise.all(MODULES.map(async m => [m, await lib(`app/javascript/lib/${m}.js`)])))

function stubMenu({ rect, hidden }) {
  const classes = new Set(hidden ? ["hidden"] : [])
  const el = {
    style: { transform: "", maxHeight: "" },
    getBoundingClientRect: () => ({ ...rect }),
    classList: {
      contains: c => classes.has(c),
      toggle: (c, force) => { const on = force === undefined ? !classes.has(c) : force; on ? classes.add(c) : classes.delete(c); return on },
    },
  }
  return { el, classes }
}

function runCase(c) {
  const fn = mods[c.mod][c.fn]
  if (c.dom) {
    const [spec, ...rest] = decode(c.args)
    const { el, classes } = stubMenu(spec)
    globalThis.window = { innerHeight: 768, innerWidth: 1024 }
    const ret = capture(() => fn(el, ...rest))
    return { return: ret, style: { ...el.style }, hidden: classes.has("hidden") }
  }
  if (c.derive) {
    const { text, term, opt } = c.derive
    const matches = mods.find_utils.findAllMatches(text, term, opt)
    c.args = encode([text, matches, c.args[2], opt])
  }
  const args = decode(c.args)
  return capture(() => fn(...args))
}

function utf16Meta(args) {
  // For string args, record UTF-16 length vs Unicode scalar count when they differ, so index-returning
  // functions can be mapped by a Rust implementation working in UTF-8/char indices.
  const strs = []
  const walk = v => { if (typeof v === "string") strs.push(v); else if (Array.isArray(v)) v.forEach(walk); else if (v && typeof v === "object" && !("$date" in v)) Object.values(v).forEach(walk) }
  walk(args)
  const diffs = strs.filter(s => s.length !== [...s].length)
  return diffs.length ? diffs.map(s => ({ utf16_len: s.length, scalar_len: [...s].length, utf8_len: Buffer.byteLength(s) })) : undefined
}

const harvested = existsSync(HARVEST) ? readFileSync(HARVEST, "utf8").split("\n").filter(Boolean).map(l => JSON.parse(l)) : []
const byMod = Object.fromEntries(MODULES.map(m => [m, []]))
const seen = new Set()
const push = (c, origin) => {
  const key = `${c.mod}.${c.fn}`
  if (DOM_FNS.has(key) && !c.dom) return
  if (key in DATE_DEFAULT && !(c.args.length > DATE_DEFAULT[key])) return
  if (key === "new_note_utils.previewNotePath" && !(c.args[0] && c.args[0].date)) return
  const sig = `${key}\u0000${JSON.stringify(c.args)}`
  if (seen.has(sig)) return
  seen.add(sig)
  const expected = runCase(c)
  const n = byMod[c.mod].filter(x => x.fn === c.fn).length + 1
  const entry = { name: c.name || `${c.fn} #${n} (${origin})`, fn: c.fn, input: c.args, expected }
  const meta = utf16Meta(c.args)
  if (meta) entry.utf16 = meta
  byMod[c.mod].push(entry)
}
for (const h of harvested) if (byMod[h.mod]) push({ mod: h.mod, fn: h.fn, args: h.args }, "from FrankMD vitest suite")
for (const c of custom) { c.args = c.dom || c.derive ? c.args : encode(c.args); push(c, "edge corpus") }

const NOTES = {
  common: "input = encoded argument array (decode with golden.mjs decode: {$undefined}, {$number}, {$date}, {$regexp}, {$utf16}); expected = encoded return value, or {$error:{name,message}}. JS strings are UTF-16: all positions/lengths/indices in inputs and outputs are UTF-16 code-unit offsets; cases with non-BMP/multi-unit strings carry a `utf16` array (utf16_len/scalar_len/utf8_len per such string arg). TZ pinned to UTC for date-based helpers.",
  find_utils: "replaceMatches cases built from findAllMatches(text, term, opt) are recorded with the concrete matches array as input[1]. Regex semantics are JavaScript RegExp (flags g, plus i when !caseSensitive; no u flag).",
  menu_bounds: "clampMenuToViewport/toggleMenu take a DOM element; cases use a stub: input[0] = {rect:{left,right,top}, hidden} describes the element (getBoundingClientRect returns rect; classList holds 'hidden' if hidden). expected = {return, style:{transform,maxHeight}, hidden(after)}. window.innerHeight=768, innerWidth=1024 when defaults used.",
  new_note_utils: "Date args are {$date}; local time = UTC.",
  s3_key: "Date args are {$date}; local time = UTC.",
  stats_utils: "charCount is UTF-16 length; byteSize is UTF-8 byte length (TextEncoder).",
}
for (const m of MODULES) {
  const cases = byMod[m]
  writeFixture(`js_${m}`, fixture({
    source: `app/javascript/lib/${m}.js`, generator: "tools/golden/js/utils.mjs",
    notes: [NOTES.common, NOTES[m]].filter(Boolean).join(" "),
    functions: [...new Set(cases.map(c => c.fn))],
    cases,
  }))
}
