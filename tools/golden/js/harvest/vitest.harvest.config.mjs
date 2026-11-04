// Runs FrankMD's own vitest suite with selected lib modules swapped for
// recording wrappers (see make_wrappers.mjs). Never writes into FrankMD.

import path from "node:path"

const F = process.env.FRANKMD
const W = process.env.HARVEST_WRAPPERS
const MODS = process.env.HARVEST_MODS.split(",")
const modRe = MODS.join("|")

export default ({
  root: F,
  test: {
    include: ["test/javascript/**/*.test.js"],
    environment: "node",
    setupFiles: [path.join(F, "test/javascript/setup.js")],
  },
  resolve: {
    alias: [
      // relative imports (../../app/javascript/lib/x.js) and "lib/x" alias imports
      { find: new RegExp(`^(?:\\.\\./)+(?:app/javascript/)?lib/(${modRe})(?:\\.js)?$`), replacement: `${W}/$1.mjs` },
      { find: new RegExp(`^lib/(${modRe})(?:\\.js)?$`), replacement: `${W}/$1.mjs` },
      { find: new RegExp(`^\\./(${modRe})(?:\\.js)?$`), replacement: `${W}/$1.mjs` },
      { find: /^lib\//, replacement: path.join(F, "app/javascript/lib") + "/" },
      { find: "marked", replacement: path.join(F, "test/javascript/mocks/marked.js") },
      { find: "katex", replacement: path.join(F, "test/javascript/mocks/katex.js") },
      { find: "dompurify", replacement: path.join(F, "vendor/javascript/dompurify.js") },
      { find: "@hotwired/turbo-rails", replacement: path.join(F, "test/javascript/mocks/turbo-rails.js") },
      { find: "@rails/request.js", replacement: path.join(F, "test/javascript/mocks/requestjs.js") },
      { find: "@replit/codemirror-vim", replacement: path.join(F, "test/javascript/mocks/codemirror-vim.js") },
    ],
  },
})
