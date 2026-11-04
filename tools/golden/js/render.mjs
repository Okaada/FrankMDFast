// Golden fixtures for FrankMD's markdown rendering stack, using the REAL vendored
// libraries the app ships (marked 15.0.6, DOMPurify 3.4.17, KaTeX 0.19.0, CodeMirror/
// Lezer parsers from vendor/javascript) resolved via an importmap emulation
// (runtime/importmap_hooks.mjs) and a jsdom window (runtime/setup.mjs).
//
// Produces: js_marked_extensions, js_html_sanitizer, js_markdown_line_mapper,
//           js_math_renderer, js_code_block_highlighter, js_terraform_hcl_stream_parser,
//           js_preview_pipeline
//
// usage: node tools/golden/js/render.mjs            (uses committed harvest/inputs/render.jsonl)
//        REHARVEST=1 node tools/golden/js/render.mjs
import "./runtime/setup.mjs"
import { readFileSync } from "node:fs"
import { join } from "node:path"
import { FRANKMD, fixture, writeFixture } from "../lib/golden.mjs"
import { harvested, cap, capAsync, enc, dedupe, EDGE } from "./runtime/common.mjs"

const GEN = "tools/golden/js/render.mjs"
const H = harvested("render", ["markdown_line_mapper", "html_sanitizer", "code_block_highlighter",
  "keyboard_shortcuts", "custom_theme", "emoji_i18n", "emoji_data", "icon_data", "math_renderer"])

// App configuration (app/javascript/controllers/app_controller.js#connect)
const { marked } = await import("marked")
const ext = await import("lib/marked_extensions")
marked.use({ breaks: true, gfm: true, extensions: ext.allExtensions })

const { sanitizeHtml } = await import("lib/html_sanitizer")
const mapper = await import("lib/markdown_line_mapper")
const { renderMathIn } = await import("lib/math_renderer")
const hl = await import("lib/code_block_highlighter")
const { terraformHcl } = await import("lib/terraform_hcl_stream_parser")
const { StringStream } = await import("@codemirror/language")

// preview_controller.js's module-private stripFrontmatter, evaluated from its source.
const pcSrc = readFileSync(join(FRANKMD, "app/javascript/controllers/preview_controller.js"), "utf8")
const sfSrc = pcSrc.slice(pcSrc.indexOf("function stripFrontmatter"), pcSrc.indexOf("\n}\n", pcSrc.indexOf("function stripFrontmatter")) + 2)
const stripFrontmatter = new Function(`${sfSrc}; return stripFrontmatter`)()

const div = (html) => { const d = document.createElement("div"); d.innerHTML = html; return d }

// ---------------------------------------------------------------- markdown corpus
const MD = {
  empty: "",
  heading_levels: "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6\n####### not a heading",
  setext: "Title\n=====\n\nSub\n---\n",
  heading_trailing_hashes: "# Title #\n\n#NoSpace\n\n  # indented",
  paragraphs_breaks: "line one\nline two (breaks:true -> <br>)\n\nnew para  \nhard break",
  emphasis: "*em* _em_ **strong** __strong__ ***both*** ~~strike~~ `code` ``co`de``",
  sup_sub_mark: "x^2^ H~2~O ==marked== ~~strike~~ ~a~~ ^^double^^ ==a=b== ^<b>x</b>^",
  emoji_shortcodes: ":smile: :+1: :-1: :heart: :not_a_real_emoji: :tada::rocket: 10:30:45 :SMILE:",
  icons: ":ph-heart: :ph-check: :ph-not-real: :ph-: :ph-heart-fill:",
  wikilinks: "See [[My Note]], [[folder/Sub Note|Label]], [[  spaced  |  disp  ]], [[a|]], [[unclosed and [[x]]] and [[<script>|<b>]] and [[Notes & \"Ideas\"]]",
  wikilink_in_code: "`[[not a link]]` and ```\n[[block]]\n```",
  links: "[text](https://example.com \"T\") <https://auto.link> www.example.com https://bare.example.com/path?q=1 [ref][1]\n\n[1]: https://ref.example.com",
  link_xss: "[x](javascript:alert(1)) [y](JaVaScRiPt:alert(1)) [z](data:text/html;base64,PHNjcmlwdD4=) [w](vbscript:msgbox) [ok](/relative) [mail](mailto:a@b.c)",
  images: "![alt](/images/a.png \"title\") ![](https://x.y/z.jpg) ![x](javascript:alert(1)) [![nested](/i.png)](https://l.ink)",
  unordered_list: "- a\n- b\n  - nested\n    - deeper\n- c\n\n* star\n+ plus",
  ordered_list: "1. one\n2. two\n10. ten\n\n3) paren start\n4) next",
  loose_list: "- a\n\n- b\n\n  para in b\n- c",
  tasks: "- [ ] todo\n- [x] done\n- [X] upper\n  - [ ] child\n- plain\n1. [ ] ordered task\n- [ ]\n- [] not a task",
  tasks_after_heading: "# Intro\n\n- [ ] a\n- plain\n- parent\n  - [x] child\n",
  blockquote: "> quote\n> > nested\n>\n> - list in quote\n\n> lazy\ncontinuation",
  hr: "---\n\n***\n\n___\n\n- - -",
  table: "| a | b | c |\n|:--|:-:|--:|\n| 1 | **2** | `3` |\n| x \\| pipe | | ç |",
  table_no_body: "| h1 | h2 |\n| --- | --- |",
  table_ragged: "a | b\n--|--\n1\n1 | 2 | 3",
  code_fences: "```js\nconst a = 1 // c\n```\n\n```python\ndef f(x):\n    return x * 2\n```\n\n~~~ruby\nputs 'hi'\n~~~\n\n```\nno lang <b>\n```\n\n    indented code",
  code_fence_langs_many: ["ts", "tsx", "jsx", "html", "css", "json", "yaml", "rb", "sh", "sql", "toml", "tf", "rs", "go", "java", "c", "cpp", "cs", "kotlin", "php", "dockerfile", "unknownlang", "JS", "c++", "c#"].map(l => "```" + l + "\nx = 1\n```").join("\n\n"),
  code_fence_info_string: "```js title=\"a.js\" {1,3}\nlet x\n```\n\n```<script>\nx\n```\n\n``` js\nspace before\n```",
  code_unclosed: "```js\nconst a = 1\nnever closed",
  math_inline: "Euler $e^{i\\pi}+1=0$ and $x$ and $5 and $10, \\$ literal, $ x$ no, $x $ no, $a \\$ b$ yes, $x$1 no",
  math_block: "Before\n\n$$\nx = \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}\n$$\n\nAfter\n\n$$E=mc^2$$\n\n$$   $$\n\ninline $$x$$ mid",
  math_errors: "$\\frac{1}$ and $\\href{javascript:alert(1)}{x}$ and $\\htmlId{a}{b}$ and $$\\undefinedcommand$$",
  math_in_code: "`$x$` and\n\n```\n$$y$$\n```",
  footnotes: "Text with footnote[^1] and [^note].\n\n[^1]: The footnote.\n[^note]: Named.",
  definition_like: "Term\n: definition (not supported)",
  raw_html_block: "<div class=\"note\">\n**not md inside**\n</div>\n\n<details><summary>S</summary>\n\nHidden *md*\n\n</details>",
  raw_html_inline: "a <b>bold</b> <span style=\"color:red\" class=\"x\">s</span> <kbd>Ctrl</kbd> <u>u</u> <!-- comment --> end",
  xss_script: "<script>alert(1)</script>\n\nok <script src=\"https://evil/x.js\"></script>",
  xss_handlers: "<img src=x onerror=alert(1)> <a href=\"#\" onclick=\"alert(1)\">c</a> <body onload=alert(1)> <div onmouseover=\"x()\">d</div>",
  xss_js_urls: "<a href=\"javascript:alert(1)\">j</a> <a href=\" javascript:alert(1)\">sp</a> <a href=\"jav&#x09;ascript:alert(1)\">ent</a> <img src=\"javascript:alert(1)\"> <a href=\"data:text/html,<script>alert(1)</script>\">d</a>",
  xss_svg: "<svg onload=alert(1)><script>alert(1)</script><circle r=\"5\"/></svg>\n\n<svg><a xlink:href=\"javascript:alert(1)\"><text>x</text></a></svg>\n\n<math><mi xlink:href=\"javascript:alert(1)\">m</mi></math>",
  xss_iframes: "<iframe src=\"https://www.youtube.com/embed/abc123\" width=\"560\" height=\"315\" frameborder=\"0\" allow=\"autoplay; encrypted-media\" allowfullscreen referrerpolicy=\"strict-origin\"></iframe>\n\n<iframe src=\"https://youtube-nocookie.com/embed/x\"></iframe>\n\n<iframe src=\"https://player.vimeo.com/video/1\"></iframe>\n\n<iframe src=\"http://www.youtube.com/embed/insecure\"></iframe>\n\n<iframe src=\"https://evil.com/embed\"></iframe>\n\n<iframe src=\"https://www.youtube.com.evil.com/x\"></iframe>\n\n<iframe srcdoc=\"<script>alert(1)</script>\" src=\"https://www.youtube.com/embed/x\"></iframe>\n\n<iframe src=\"javascript:alert(1)\"></iframe>\n\n<iframe></iframe>\n\n<iframe src=\"//www.youtube.com/embed/protorel\"></iframe>",
  xss_forms: "<form action=\"/x\"><input type=\"text\" name=\"p\"><input type=\"password\"><input type=\"checkbox\" checked disabled onclick=\"x()\" data-x=\"1\"><button>Go</button><textarea>t</textarea><select><option>o</option></select></form><input type=\"hidden\" value=\"h\"><input>",
  xss_style: "<style>body{display:none}</style><p style=\"position:fixed\">s</p><link rel=\"stylesheet\" href=\"x.css\"><meta http-equiv=\"refresh\" content=\"0;url=http://evil\"><base href=\"http://evil/\">",
  xss_misc: "<object data=\"x.swf\"></object><embed src=\"x\"><a href=\"https://ok.com\" target=\"_blank\" rel=\"noopener\">ok</a> <img src=\"https://ok/a.png\" srcset=\"a.png 1x\" alt=\"a\" width=\"10\"> <video src=\"v.mp4\" controls></video> <audio src=\"a.mp3\"></audio> <noscript><p title=\"</noscript><img src=x onerror=alert(1)>\"></noscript>",
  html_entities: "&amp; &lt; &gt; &quot; &copy; &#169; &#x1F600; &nbsp; & < > \" '",
  escapes: "\\*not em\\* \\# \\[not link\\] \\` \\\\ \\$ \\^ \\~ \\=",
  unicode_mix: "# Título com ação\n\nCafé ü 日本語 中文 한국어 שלום עולם 👨‍👩‍👧 e\u0301 NBSP:\u00a0end\ttab",
  rtl_heading: "# שלום\n\n- פריט\n- [ ] משימה",
  crlf: "# Title\r\n\r\npara line1\r\nline2\r\n\r\n- a\r\n- [x] b\r\n\r\n```js\r\nlet x\r\n```\r\n",
  lone_cr: "# Title\rpara\r\r- a",
  trailing_newlines: "para\n\n\n\n",
  no_trailing_newline: "# h\npara",
  only_whitespace: "   \n\t\n  ",
  frontmatter_yaml: "---\ntitle: Hello\ntags: [a, b]\n---\n\n# Body\n\ntext",
  frontmatter_toml: "+++\ntitle = \"Hello\"\n+++\n# Body",
  frontmatter_unclosed: "---\ntitle: x\n\n# Body",
  frontmatter_at_eof: "---\ntitle: x\n---",
  frontmatter_not_at_start: "\n---\ntitle: x\n---\n# Body",
  hr_then_text: "---\nnot frontmatter? closing below\n---\n\nbody",
  nested_mix: "1. Step with `code`\n   ```sh\n   echo hi\n   ```\n2. > quote in list\n3. | t | u |\n   |---|---|\n   | 1 | 2 |",
  html_in_list_li: "- <li>raw li</li>\n- <link rel=x>\n\n```html\n<li>in code</li>\n<ul><li>x</li></ul>\n```",
  long_doc: Array.from({ length: 60 }, (_, i) => i % 5 === 0 ? `## Section ${i}\n` : `Paragraph ${i} with **bold** and [[Link ${i}]] and $x_${i}$.\n`).join("\n"),
  very_long_line: "word ".repeat(3000),
  deep_nesting: Array.from({ length: 12 }, (_, i) => "  ".repeat(i) + "- level " + i).join("\n"),
  deep_quote: ">".repeat(30) + " deep",
  autolinks_gfm: "Visit www.commonmark.org/help or https://github.com. Email foo@bar.baz. Not a link: http:/broken",
  strikethrough_tilde: "~~two~~ ~one~ ~~~three~~~ a~b~c",
  hard_line_break_backslash: "line\\\nnext",
  heading_ids: "# Same\n# Same\n## Ça va?",
  emoji_inside_link: "[:smile: link](https://x.y) **:tada:**",
  mixed_extensions: "==**bold mark**== ^[[wiki]]^ ~$x$~ :ph-check: $a^2$",
}

// ================================================================ marked_extensions
{
  const cases = []
  const E = { superscript: ext.superscriptExtension, subscript: ext.subscriptExtension, highlight: ext.highlightExtension,
    emoji: ext.emojiExtension, icon: ext.iconExtension, wikilink: ext.wikilinkExtension, mathBlock: ext.mathBlockExtension, mathInline: ext.mathInlineExtension }
  const SRC = {
    superscript: ["^text^", "^a b^ rest", "^^", "^^double^^", "x^2^", "^unclosed", "^<b>&\"</b>^", "^多^", "^\n^"],
    subscript: ["~text~", "~~strike~~", "~a~~", "~2~O", "~unclosed", "~<i>~", "~ ~", "~a\nb~"],
    highlight: ["==text==", "==a=b==", "====", "==unclosed", "== spaced ==", "==<b>x</b>==", "==\n=="],
    emoji: [":smile:", ":+1:", ":-1:", ":not_real:", ":SMILE:", ":smile", "smile:", ":heart: rest", ":ph-heart:", ":tada:x", "::"],
    icon: [":ph-heart: rest", ":ph-not-a-real-icon:", "not an icon", ":ph-check:", ":ph-:", ":ph-Heart:", "hello :ph-heart: world", "no icons here, just :smile:"],
    wikilink: ["[[My Note]] rest", "[[My Note|Custom Label]] rest", "[[projects/My Note]] rest", "[[  My Note  |  Label  ]] rest",
      "[[unclosed", "not a wikilink", "[[]]", "[[|x]]", "[[a|]]", "[[a|b|c]]", "[[a]b]]", "[[ação/日本]]", "[[<script>alert(1)</script>]]", "[[a\nb]]", "hello [[world]]", "no wikilinks here"],
    mathBlock: ["$$block$$", "$$E=mc^2$$", "$$\nx = \\frac{-b}{2a}\n$$\nmore", "$$\nx = y\n", "$$   $$", "$$a$$\n\n\nnext", " $$x$$", "no block here", "text\n$$\nx=y\n$$", "$$a$$b$$", "$$\r\nx\r\n$$\r\n"],
    mathInline: ["$x$", "$E = mc^2$ rest", "$a$ and $b$", "$a \\$ b$ rest", "$ x$", "$x $", "$10.", "$100 to $200", "$10 and that costs $20.", "$x$1", "$$x$$", "costs \\$5 today", "no dollars here", "The equation is $E=mc^2$", "$\\$$", "$a\\\\$", "$日本$", "$x\ny$"],
  }
  for (const [name, list] of Object.entries(SRC)) {
    for (const s of list) {
      cases.push({ name: `${name}.start ${JSON.stringify(s).slice(0, 40)}`, fn: `${name}Extension.start`, input: [s], expected: cap(() => E[name].start(s)) })
      cases.push({ name: `${name}.tokenizer ${JSON.stringify(s).slice(0, 40)}`, fn: `${name}Extension.tokenizer`, input: [s], expected: cap(() => E[name].tokenizer(s)) })
    }
  }
  const RT = {
    superscript: [{ text: "x" }, { text: "<a href=\"&\">" }], subscript: [{ text: "2" }, { text: "'q'" }], highlight: [{ text: "m" }, { text: "<script>" }],
    emoji: [{ emoji: "😄" }],
    icon: [{ path: "M1 2 3", viewBox: "0 0 256 256" }, { path: "\"><script>", viewBox: "0 0 1 1" }],
    wikilink: [{ target: "My Note", displayText: "Click here" }, { target: "My Note", displayText: null }, { target: "Note", displayText: "<script>alert(1)</script>" },
      { target: "Notes & Ideas", displayText: "R&D Notes" }, { target: 'Note "with" quotes', displayText: null }, { target: "projects/My Note", displayText: null },
      { target: "a/b/", displayText: null }, { target: "x'y", displayText: "" }],
    mathBlock: [{ text: "</div><script>alert(1)</script>" }, { text: "x = y" }, { text: "a & b" }],
    mathInline: [{ text: "E = mc^2" }, { text: "<img src=x onerror=alert(1)>" }, { text: "\"'" }],
  }
  for (const [name, list] of Object.entries(RT)) for (const t of list)
    cases.push({ name: `${name}.renderer ${JSON.stringify(t).slice(0, 50)}`, fn: `${name}Extension.renderer`, input: [t], expected: cap(() => E[name].renderer(t)) })
  cases.push({ name: "allExtensions order", fn: "allExtensions", input: [], expected: ext.allExtensions.map(e => ({ name: e.name, level: e.level })) })
  // Through the configured marked (raw, unsanitized marked output)
  for (const [n, md] of Object.entries(MD)) cases.push({ name: `marked.parse ${n}`, fn: "marked.parse", input: [md], expected: cap(() => marked.parse(md)) })
  for (const s of ["x^2^", "H~2~O", "==m==", ":smile:", ":ph-heart:", "[[a|b]]", "$x$", "~~s~~", "a $5 b $6", "==a== ^b^ ~c~"])
    cases.push({ name: `marked.parseInline ${s}`, fn: "marked.parseInline", input: [s], expected: cap(() => marked.parseInline(s)) })
  writeFixture("js_marked_extensions", fixture({ source: "app/javascript/lib/marked_extensions.js", generator: GEN,
    notes: "Extensions called directly (start/tokenizer/renderer) plus marked.parse/parseInline with the app's global config marked.use({breaks:true,gfm:true,extensions:allExtensions}) from app_controller.js, using vendored marked@15.0.6 (vendor/javascript/marked.js). marked.parse output is RAW (unsanitized) — see js_preview_pipeline for the full sanitized pipeline. Emoji/icon lookups use the real EMOJI_DATA/ICON_DATA.",
    cases: dedupe(cases) }))
}

// ================================================================ html_sanitizer
{
  const cases = []
  for (const [html] of H("html_sanitizer", "sanitizeHtml")) cases.push({ name: `harvested ${String(html).slice(0, 50)}`, fn: "sanitizeHtml", input: [html], expected: cap(() => sanitizeHtml(html)) })
  const X = [
    null, undefined, "", " ", "plain text", "<p>ok</p>", "<script>alert(1)</script>", "<SCRIPT>x</SCRIPT>", "<scr<script>ipt>alert(1)</script>",
    "<img src=x onerror=alert(1)>", "<img src=\"x\" ONERROR=\"alert(1)\">", "<a href=\"javascript:alert(1)\">x</a>", "<a href=\"JAVASCRIPT:alert(1)\">x</a>",
    "<a href=\"  javascript:alert(1)\">x</a>", "<a href=\"java\nscript:alert(1)\">x</a>", "<a href=\"&#106;avascript:alert(1)\">x</a>",
    "<a href=\"data:text/html,x\">d</a>", "<img src=\"data:image/png;base64,iVBORw0KGgo=\">", "<a href=\"vbscript:x\">v</a>", "<a href=\"mailto:a@b\">m</a>",
    "<a href=\"tel:+1\">t</a>", "<a href=\"ftp://x\">f</a>", "<a href=\"/rel\">r</a>", "<a href=\"#frag\">h</a>", "<a href=\"//proto.rel\">p</a>",
    "<svg><script>alert(1)</script></svg>", "<svg onload=alert(1)><rect/></svg>", "<svg><foreignObject><img src=x onerror=1></foreignObject></svg>",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 256 256\" fill=\"currentColor\" width=\"1em\" height=\"1em\" class=\"frankmd-inline-icon\" aria-hidden=\"true\"><path d=\"M1 2\"/></svg>",
    "<math><mtext><table><mglyph><style><img src=x onerror=alert(1)>", "<iframe src=\"https://www.youtube.com/embed/x\" allow=\"a\" allowfullscreen frameborder=\"0\" referrerpolicy=\"no-referrer\" srcdoc=\"<b>\"></iframe>",
    "<iframe src=\"https://WWW.YOUTUBE.COM/embed/x\"></iframe>", "<iframe src=\"https://youtube.com:443/embed/x\"></iframe>", "<iframe src=\"https://youtube.com:8443/embed/x\"></iframe>",
    "<iframe src=\"https://user:pw@www.youtube.com/embed/x\"></iframe>", "<iframe src=\"/relative\"></iframe>", "<iframe src=\"https://player.vimeo.com/video/1\" onload=\"x()\"></iframe>",
    "<iframe src=\"https://vimeo.com/1\"></iframe>", "<iframe src=\"https://www.youtube-nocookie.com/embed/x\" sandbox=\"\"></iframe>",
    "<input type=\"checkbox\">", "<input type=\"checkbox\" checked disabled>", "<input type=\"CHECKBOX\">", "<input type=\"checkbox\" name=\"n\" value=\"v\" id=\"i\" class=\"c\" data-x=\"1\" onclick=\"x\">",
    "<input type=\"text\">", "<input>", "<input type=\"submit\">", "<input type=\"image\" src=\"x\">",
    "<form><input type=\"checkbox\"></form>", "<button>b</button>", "<textarea>t</textarea>", "<select><option>o</option></select>",
    "<style>*{}</style><p>x</p>", "<p style=\"color:red\">s</p>", "<div class=\"a\" id=\"b\" title=\"t\" data-source-line=\"3\" data-x=\"y\">d</div>",
    "<details open><summary>s</summary>b</details>", "<kbd>k</kbd><mark>m</mark><sup>s</sup><sub>s</sub><del>d</del><ins>i</ins>",
    "<table><tr><td>no tbody</td></tr></table>", "<p><div>bad nesting</div></p>", "<li>orphan li</li>", "<b><i>unclosed",
    "<!-- comment --><p>x</p>", "<![CDATA[x]]>", "<?xml version=\"1.0\"?><p>x</p>", "<template><script>x</script></template>",
    "<a href=\"https://x\" target=\"_blank\">t</a>", "<a name=\"anchor\" id=\"anchor\">a</a>", "<object data=x></object><embed src=x><applet></applet>",
    "<meta http-equiv=refresh content=0><base href=x><link rel=import href=x>", "<video src=v controls poster=p></video><audio src=a controls></audio><source src=s>",
    "<img src=\"a.png\" srcset=\"a.png 1x, b.png 2x\" alt=\"a\" title=\"t\" width=\"1\" height=\"2\" loading=\"lazy\">", "<picture><source srcset=\"x.webp\"><img src=\"x.png\"></picture>",
    "<a class=\"wikilink\" data-wikilink-path=\"x\" data-action=\"click->app#openWikilink\">w</a>", "<span class=\"math-inline\">x^2</span><div class=\"math-block\">y</div>",
    "<noscript><p title=\"</noscript><img src=x onerror=alert(1)>\"></noscript>", "<x-custom onclick=1>c</x-custom>", "<marquee>m</marquee>",
    "<p>" + EDGE.cjk + EDGE.hebrew + EDGE.zwj + EDGE.combining + EDGE.nbsp + "</p>", "a\r\nb\rc", "&lt;script&gt; &amp;amp;", "<p>" + "x".repeat(20000) + "</p>",
  ]
  for (const h of X) cases.push({ name: `corpus ${JSON.stringify(h ?? String(h)).slice(0, 60)}`, fn: "sanitizeHtml", input: [h], expected: cap(() => sanitizeHtml(h)) })
  writeFixture("js_html_sanitizer", fixture({ source: "app/javascript/lib/html_sanitizer.js", generator: GEN,
    notes: "Vendored DOMPurify 3.4.17 running on a jsdom window (window.location = http://localhost:3000/); output is DOMPurify's serialization of the jsdom DOM. Inputs: every distinct sanitizeHtml call from FrankMD's vitest suite (harvested) + XSS/edge corpus. Non-string falsy input returns \"\".",
    cases: dedupe(cases) }))
}

// ================================================================ markdown_line_mapper
{
  const cases = []
  for (const args of H("markdown_line_mapper", "parseWithLineNumbers"))
    cases.push({ name: `harvested ${JSON.stringify(args[0]).slice(0, 50)}`, fn: "parseWithLineNumbers", input: enc(args), expected: cap(() => mapper.parseWithLineNumbers(...args)) })
  for (const [n, md] of Object.entries(MD)) {
    cases.push({ name: `corpus ${n}`, fn: "parseWithLineNumbers", input: [md], expected: cap(() => mapper.parseWithLineNumbers(md)) })
  }
  for (const [n, md, off] of [["offset 4", "# First\n\n# Second\n\n", 4], ["offset 0 explicit", "- [ ] a\n- [x] b", 0], ["offset 10 tasks", MD.tasks, 10], ["negative offset", "# a\n\nb", -3], ["null md", null, 0], ["undefined md", undefined, 2]])
    cases.push({ name: `offset ${n}`, fn: "parseWithLineNumbers", input: enc([md, off]), expected: cap(() => mapper.parseWithLineNumbers(md, off)) })
  // Raw marked.lexer token types/raws used for the mapping (informative)
  for (const n of ["tasks", "code_fences", "math_block", "table", "nested_mix", "crlf", "raw_html_block"])
    cases.push({ name: `lexer ${n}`, fn: "marked.lexer(types+raw)", input: [MD[n]], expected: cap(() => marked.lexer(MD[n]).map(t => ({ type: t.type, raw: t.raw }))) })
  // findElementByLine over parsed HTML: returns the outerHTML start tag + data-source-line of the hit
  const doc = MD.long_doc
  const html = mapper.parseWithLineNumbers(doc)
  for (const target of [-5, 0, 1, 2, 3, 7, 12, 50, 61, 119, 120, 500]) {
    cases.push({ name: `findElementByLine long_doc @${target}`, fn: "findElementByLine", input: { markdown: doc, targetLine: target },
      expected: cap(() => { const el = mapper.findElementByLine(div(html), target); return el ? { tag: el.tagName.toLowerCase(), line: el.dataset.sourceLine, text: el.textContent.slice(0, 40) } : null }) })
  }
  cases.push({ name: "findElementByLine empty container", fn: "findElementByLine", input: { html: "<p>x</p>", targetLine: 1 }, expected: cap(() => mapper.findElementByLine(div("<p>x</p>"), 1)) })
  // findLineAtScroll with stubbed rects: element tops given explicitly (jsdom has no layout)
  const scrollScenarios = [
    { lines: [1, 3, 5, 9], tops: [0, 100, 200, 400], containerTop: 0, containerScrollTop: 0, scrollTops: [-10, 0, 50, 100, 150, 300, 399, 400, 1000] },
    { lines: [5, 7], tops: [50, 50], containerTop: 10, containerScrollTop: 20, scrollTops: [0, 60, 61, 100] },
    { lines: [], tops: [], containerTop: 0, containerScrollTop: 0, scrollTops: [0] },
  ]
  for (const sc of scrollScenarios) for (const st of sc.scrollTops) {
    cases.push({ name: `findLineAtScroll lines=${sc.lines} st=${st}`, fn: "findLineAtScroll", input: { ...sc, scrollTops: undefined, scrollTop: st },
      expected: cap(() => {
        const c = div(sc.lines.map(l => `<p data-source-line="${l}">x</p>`).join(""))
        c.getBoundingClientRect = () => ({ top: sc.containerTop })
        Object.defineProperty(c, "scrollTop", { value: sc.containerScrollTop })
        ;[...c.children].forEach((el, i) => { el.getBoundingClientRect = () => ({ top: sc.tops[i] }) })
        return mapper.findLineAtScroll(c, st)
      }) })
  }
  writeFixture("js_markdown_line_mapper", fixture({ source: "app/javascript/lib/markdown_line_mapper.js", generator: GEN,
    notes: "parseWithLineNumbers runs against vendored marked@15.0.6 WITH the app's global marked.use({breaks:true,gfm:true,extensions:allExtensions}) (FrankMD's own vitest suite mocks marked; here it is real) and vendored DOMPurify on jsdom. Output HTML is DOMPurify/jsdom serialization. findElementByLine expected = {tag,line,text[0..40]} of the returned element. findLineAtScroll: jsdom has no layout, so element/container getBoundingClientRect().top and container.scrollTop are stubbed from the input (tops[i] for the i-th [data-source-line] element). 'marked.lexer(types+raw)' cases are informative token dumps.",
    cases: dedupe(cases) }))
}

// ================================================================ math_renderer
{
  const cases = []
  const TEX = ["E = mc^2", "x = \\frac{1}{2}", "\\sum_{i=0}^{n} i^2", "\\sqrt[3]{x}", "\\begin{matrix}a & b\\\\c & d\\end{matrix}", "\\frac{1}", "\\undefinedmacro",
    "\\href{javascript:alert(1)}{x}", "\\url{https://x}", "\\htmlClass{evil}{x}", "\\htmlStyle{color:red}{x}", "\\includegraphics{x.png}", "\\textcolor{red}{x}",
    "\\def\\a{\\a\\a}\\a", "\\text{ação 日本}", "<script>", "a & b", "", " ", "\\alpha\\beta\\gamma", "x^{2^{3}}", "\\left( \\frac{a}{b} \\right)", "\\operatorname{f}(x)", "\\mathbb{R}", "\\%\\$\\#"]
  for (const t of TEX) for (const block of [false, true]) {
    const html = block ? `<div class="math-block">${t.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</div>` : `<span class="math-inline">${t.replace(/&/g, "&amp;").replace(/</g, "&lt;")}</span>`
    cases.push({ name: `${block ? "block" : "inline"} ${t.slice(0, 40)}`, fn: "renderMathIn", input: { html }, expected: cap(() => { const d = div(html); renderMathIn(d); return d.innerHTML }) })
  }
  const OTHER = ['<span class="math-inline" data-math-rendered="">already</span>', '<p>no math</p>', '<span class="math-inline x">a</span><div class="math-block">b</div><span class="math">c</span>',
    '<span class="math-inline"><b>x</b>^2</span>', '<div class="math-block">\\begin{aligned}a&=b\\\\c&=d\\end{aligned}</div>']
  for (const html of OTHER) cases.push({ name: `container ${html.slice(0, 50)}`, fn: "renderMathIn", input: { html }, expected: cap(() => { const d = div(html); renderMathIn(d); return d.innerHTML }) })
  cases.push({ name: "idempotent twice", fn: "renderMathIn", input: { html: '<span class="math-inline">x</span>', times: 2 }, expected: cap(() => { const d = div('<span class="math-inline">x</span>'); renderMathIn(d); renderMathIn(d); return d.innerHTML }) })
  cases.push({ name: "null container", fn: "renderMathIn", input: { html: null }, expected: cap(() => renderMathIn(null)) })
  writeFixture("js_math_renderer", fixture({ source: "app/javascript/lib/math_renderer.js", generator: GEN,
    notes: "Vendored KaTeX 0.19.0 rendering into jsdom elements with KATEX_OPTIONS {throwOnError:false, trust:false, strict:'ignore', maxExpand:1000, output:'html'} (displayMode for .math-block). input.html is the container innerHTML before renderMathIn; expected is container innerHTML after (jsdom serialization). KaTeX CSS/fonts are not part of this.",
    cases: dedupe(cases) }))
}

// ================================================================ code_block_highlighter
{
  const cases = []
  for (const [html] of H("code_block_highlighter", "highlightCodeBlocks"))
    cases.push({ name: `harvested ${String(html).slice(0, 50)}`, fn: "highlightCodeBlocks", input: [html], expected: await capAsync(() => hl.highlightCodeBlocks(html)) })
  // Every alias in LANGUAGE_PARSERS (parsed from source) -> label + highlighted sample
  const src = readFileSync(join(FRANKMD, "app/javascript/lib/code_block_highlighter.js"), "utf8")
  const aliases = [...src.slice(src.indexOf("const LANGUAGE_PARSERS")).matchAll(/\["([^"]+)", (\w+)\]/g)].map(m => [m[1], m[2]])
  const SAMPLE = {
    javascript: "// c\nconst f = (a) => a + 1; let s = `t${x}`; /re/g; class A extends B { m() { return null } }",
    jsx: "const el = <div className=\"a\">{x}</div>", typescript: "interface I { a: number }\nconst x: string = 'y' as const", tsx: "const C: FC<P> = () => <B p={1} />",
    html: "<!doctype html><div class=\"a\" id=b>t &amp; <b>x</b></div><!-- c -->", svg: "<svg viewBox=\"0 0 1 1\"><path d=\"M0\"/></svg>",
    css: ".a > b:hover { color: #fff; margin: 1px 2em; } @media (x) { }", json: "{\"a\": [1, 2.5, true, null], \"b\": \"s\"}", jsonc: "{ // c\n \"a\": 1, }",
    yaml: "key: value\nnum: 42\nbool: true\nnul: ~\nhex: 0x1F\nquoted: \"q\"\nlist:\n  - a\n  - 3.5\nblock: |\n  text\n# comment", php: "<?php\nfunction f($a) { echo \"x\"; return 1; } // c",
    ruby: "# c\ndef foo(a)\n  puts \"hi #{a}\" if a.nil?\n  :sym\nend", dotenv: "KEY=value\n# comment\nexport X=\"q\"", shell: "#!/bin/bash\necho \"$HOME\" | grep -v x && ls -la # c",
    sql: "SELECT a, COUNT(*) FROM t WHERE x = 'y' AND n > 1; -- c", postgres: "SELECT now()::date; CREATE TABLE t (id serial);", mysql: "SELECT `a` FROM t LIMIT 1; # c",
    mariaDB: "SELECT 1;", sqlite: "PRAGMA foreign_keys = ON;", toml: "[section]\nkey = \"v\"\nn = 1\n# c", dockerfile: "# c\nFROM node:22\nRUN npm i\nCMD [\"node\"]",
    terraform: "resource \"aws_s3_bucket\" \"b\" {\n  bucket = var.name\n  count  = 2\n  tags   = merge(local.t, { a = true })\n}\n# c", hcl: "a = 1\nb = <<EOF\nx\nEOF\n/* c */",
    rust: "fn main() { let x: i32 = 5; println!(\"{}\", x); } // c", go: "package main\nfunc main() { fmt.Println(\"x\", 1) }", python: "def f(x):\n    return x * 2  # c\n@dec\nclass A: pass",
    java: "public class A { public static void main(String[] a) { int x = 1; } }", c: "#include <stdio.h>\nint main(void) { return 0; } /* c */", cpp: "template<class T> auto f(T t) -> T { return t; }",
    csharp: "public class A { public int X { get; set; } } // c", kotlin: "fun main() { val x: Int = 1; println(\"$x\") }",
  }
  const labelTable = {}
  for (const [alias, varName] of aliases) {
    const sample = SAMPLE[varName] ?? "x = 1"
    const html = `<pre><code class="language-${alias.replace(/"/g, "&quot;")}">${sample.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")}</code></pre>`
    const out = await capAsync(() => hl.highlightCodeBlocks(html))
    labelTable[alias] = typeof out === "string" ? (out.match(/data-code-language="([^"]*)"/)?.[1] ?? null) : null
    cases.push({ name: `alias ${alias}`, fn: "highlightCodeBlocks", input: [html], expected: out })
  }
  cases.push({ name: "alias -> label table", fn: "LANGUAGE_PARSERS(labels)", input: aliases.map(a => a[0]), expected: labelTable })
  const MISC = [
    '<pre><code class="language-JS">const a = 1</code></pre>', '<pre><code class="language-Python">x</code></pre>', '<pre><code class="language-unknown">x</code></pre>',
    '<pre><code class="hljs language-js extra">let x</code></pre>', '<pre><code class="language-js language-py">let x</code></pre>', '<pre><code class="foo">x</code></pre>',
    '<pre><code>no class</code></pre>', '<code class="language-js">inline not in pre</code>', '<pre><div><code class="language-js">nested</code></div></pre>',
    '<pre data-code-language="Old"><code class="language-js">x</code></pre>', '<pre data-code-language="JavaScript"><code class="language-js"></code></pre>',
    '<pre><code class="language-js">a &lt; b &amp;&amp; c &gt; d</code></pre>', '<pre><code class="language-js"><span>pre-existing</span> x</code></pre>',
    '<pre><code class="language-js">const s = "unterminated</code></pre>', '<pre><code class="language-ts">ação = "日本語 👨‍👩‍👧"</code></pre>',
    '<pre><code class="language-js">a\r\nb\rc</code></pre>', '<pre><code class="language-python">\tindented\n\u00a0nbsp</code></pre>',
    '<pre><code class="language-">empty lang</code></pre>', '<pre><code class="language-__proto__">x</code></pre>', '<pre><code class="language-constructor">x</code></pre>',
    "", null, "<p>no code</p>",
    `<pre><code class="language-js">${"x".repeat(20000)}</code></pre>`, `<pre><code class="language-js">${"x".repeat(20001)}</code></pre>`,
    Array.from({ length: 3 }, () => `<pre><code class="language-js">${"y".repeat(19000)}</code></pre>`).join(""),
    Array.from({ length: 102 }, (_, i) => `<pre><code class="language-js">v${i}</code></pre>`).join(""),
  ]
  for (const h of MISC) cases.push({ name: `misc ${JSON.stringify(h ?? String(h)).slice(0, 60)}`, fn: "highlightCodeBlocks", input: [h], expected: await capAsync(() => hl.highlightCodeBlocks(h)) })
  for (const h of ['<pre><code class="language-js">x</code></pre>', '<pre data-code-language="JavaScript"><code class="language-js">x</code></pre>', '<pre data-code-language="JavaScript"><code class="language-js"></code></pre>', '<p>x</p>', '<pre><code class="language-dockerfile">FROM x</code></pre>'])
    cases.push({ name: `inElement ${h.slice(0, 60)}`, fn: "highlightCodeBlocksInElement", input: { html: h },
      expected: await capAsync(async () => { const d = div(h); const changed = await hl.highlightCodeBlocksInElement(d); return { changed, html: d.innerHTML } }) })
  cases.push({ name: "inElement null root", fn: "highlightCodeBlocksInElement", input: { html: null }, expected: await capAsync(() => hl.highlightCodeBlocksInElement(null)) })
  writeFixture("js_code_block_highlighter", fixture({ source: "app/javascript/lib/code_block_highlighter.js", generator: GEN,
    notes: "Parsers are the VENDORED CodeMirror/Lezer builds resolved through config/importmap.rb (not node_modules). QUIRK: vendor/javascript/@codemirror--legacy-modes--mode--dockerfile.js imports './simple-mode.js' which is not vendored/pinned, so in the shipped browser app the Dockerfile parser fails to load: blocks get data-code-language=\"Dockerfile\" but NO token spans (FrankMD's vitest suite uses node_modules and does highlight them). Limits: >20000 chars per block skipped, 50000 total per render, max 100 blocks inspected. Output is jsdom <template> serialization. 'LANGUAGE_PARSERS(labels)' maps each fence alias (from source) to the data-code-language label.",
    cases: dedupe(cases) }))
}

// ================================================================ terraform_hcl_stream_parser
{
  function tokenize(text) {
    const state = terraformHcl.startState()
    const out = []
    for (const line of text.split("\n")) {
      if (line === "") { terraformHcl.blankLine(state, 2); out.push([]); continue }
      const stream = new StringStream(line, 2, 2)
      const toks = []
      let guard = 0
      while (!stream.eol() && guard++ < 10000) {
        const style = terraformHcl.token(stream, state)
        toks.push([line.slice(stream.start, stream.pos), style ?? null])
        stream.start = stream.pos
      }
      out.push(toks)
    }
    return { lines: out, endState: { ...state } }
  }
  const T = [
    "resource \"aws_instance\" \"web\" {\n  ami           = \"ami-123\"\n  instance_type = var.type\n  count = 3\n}",
    "variable \"x\" {\n  default = true\n  type = list(string)\n}\n\noutput \"o\" { value = module.m.out[0] }",
    "# hash comment\n// slash comment\n/* block\n still comment\n end */ a = 1", "/* single */ b = 2", "/* unterminated\nline2\n",
    "x = <<EOF\nheredoc line ${var.a}\n  EOF\nEOF\ny = 1", "x = <<-EOT\n  indented\n  EOT\nz = null", "x = <<NOTCLOSED\na\nb",
    "n = [0x1F, 0b101, 0o17, 1_000, 3.14, .5, 1e10, 2.5E-3, 1.]", "s = \"esc \\\" quote\" t = \"unterminated", "a = b.c.d(e)\nf = g . h",
    "for_each = { for k, v in var.m : k => v if v != null }", "cond = a == b ? x : y && !z || w >= 1", "locals { a-b = 1 }", "data \"x\" \"y\" {}\nimport { to = a.b }",
    "removed { from = a }\ncheck \"c\" {}\nprovisioner \"local-exec\" {}\nconnection {}\ndynamic \"x\" {}", "a.\n\nb", "@ $ ` ~", "ção = \"日本\"", "", "\t\tx\t=\t1", "a = 1\r\nb = 2",
    "fn  (x)\nkey   =1", "provider.foo = 1", "x = foo.bar(baz)",
  ]
  const cases = T.map(t => ({ name: `tokenize ${JSON.stringify(t).slice(0, 50)}`, fn: "tokenize", input: [t], expected: cap(() => tokenize(t)) }))
  cases.push({ name: "startState", fn: "startState", input: [], expected: cap(() => terraformHcl.startState()) })
  cases.push({ name: "copyState", fn: "copyState", input: [{ blockComment: true, heredocMarker: "EOF", afterDot: true }], expected: cap(() => terraformHcl.copyState({ blockComment: true, heredocMarker: "EOF", afterDot: true })) })
  writeFixture("js_terraform_hcl_stream_parser", fixture({ source: "app/javascript/lib/terraform_hcl_stream_parser.js", generator: GEN,
    notes: "Driven line-by-line like CodeMirror StreamLanguage: input split on \\n; empty lines call blankLine(state) and yield []; otherwise a fresh StringStream(line, tabSize=2, indentUnit=2) (vendored @codemirror/language) is fed to token() until eol. Each token is [text, style|null]; state carries across lines. endState = state after last line. Styles map to tags via StreamLanguage ('variableName.function' etc).",
    cases }))
}

// ================================================================ preview pipeline
{
  const cases = []
  async function pipeline(markdownContent) {
    const { content, frontmatterLines } = stripFrontmatter(markdownContent || "")
    const sanitized = mapper.parseWithLineNumbers(content, frontmatterLines)
    const container = document.createElement("div")
    container.innerHTML = sanitized
    const hlPromise = highlightPromise(container)
    renderMathIn(container)
    await hlPromise
    return { frontmatter: { content, frontmatterLines }, sanitized_html: sanitized, final_html: container.innerHTML, totalSourceLines: (markdownContent || "").split("\n").length }
  }
  // same ordering as preview_controller.render(): highlight started (not awaited) then renderMathIn
  const highlightPromise = (c) => hl.highlightCodeBlocksInElement(c).catch(() => {})
  for (const [n, md] of Object.entries(MD)) cases.push({ name: n, fn: "previewRender", input: [md], expected: await capAsync(() => pipeline(md)) })
  for (const fm of ["---\na: 1\n---\n# x", "---\na: 1\n---", "---\na: 1\n---\n", "--- \na\n---\nb", "---\n---\nbody", "+++\na = 1\n+++\n\n\n  body", "+++\na=1", "----\nx\n----\ny",
    "---\r\na: 1\r\n---\r\nbody", "---\na: 1\n--- trailing\nbody", "---\na: |\n  ---\n---\nbody", "text\n---\nx\n---", null, undefined, "", "---", "+++", "---\n...\n"])
    cases.push({ name: `stripFrontmatter ${JSON.stringify(fm ?? String(fm))}`, fn: "stripFrontmatter", input: enc([fm]), expected: cap(() => stripFrontmatter(fm)) })
  writeFixture("js_preview_pipeline", fixture({ source: "app/javascript/controllers/preview_controller.js#render (+ app_controller marked.use, lib/markdown_line_mapper, lib/html_sanitizer, lib/code_block_highlighter, lib/math_renderer)", generator: GEN,
    notes: "Full preview render as in preview_controller.render(): stripFrontmatter (module-private; evaluated verbatim from the controller source) -> parseWithLineNumbers(content, frontmatterLines) [vendored marked 15.0.6 with breaks:true,gfm:true,allExtensions; then data-source-line injection; then vendored DOMPurify 3.4.17] -> container.innerHTML -> highlightCodeBlocksInElement (vendored Lezer parsers; awaited) -> renderMathIn (vendored KaTeX 0.19.0). sanitized_html = string assigned to innerHTML; final_html = container.innerHTML afterwards. All HTML is jsdom serialization (jsdom 27). NOT included: the controller's DOM chrome added afterwards — _addCodeCopyButtons (appends <button class=\"code-copy-btn\" type=\"button\" title=\"Copy code\"> with an SVG inside each pre that has a code, sets pre.style.position='relative') and _addImageTriggers (wraps each <img> with src/srcset in <button class=\"preview-image-trigger\">, or for images inside <a> inserts <button class=\"preview-image-enlarge\">↗</button> after the link; aria-label from i18n). Dockerfile fences are NOT token-highlighted in the shipped app (missing vendored simple-mode.js).",
    cases }))
}
