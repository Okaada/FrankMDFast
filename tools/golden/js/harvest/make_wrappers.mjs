// Generate wrapper modules that re-export FrankMD lib modules with recording.
// usage: node make_wrappers.mjs <outdir> mod1 mod2 ...
import { mkdirSync, writeFileSync } from "node:fs"
import { join } from "node:path"
import { lib, FRANKMD } from "../../lib/golden.mjs"
const [outDir, ...mods] = process.argv.slice(2)
mkdirSync(outDir, { recursive: true })
const recorder = new URL("./recorder.mjs", import.meta.url).pathname
for (const mod of mods) {
  const real = await lib(`app/javascript/lib/${mod}.js`)
  const lines = [
    `import * as real from ${JSON.stringify(join(FRANKMD, "app/javascript/lib", mod + ".js"))}`,
    `import { record } from ${JSON.stringify(recorder)}`,
  ]
  for (const [name, val] of Object.entries(real)) {
    if (typeof val === "function") lines.push(`export function ${name}(...a) { return record(${JSON.stringify(mod)}, ${JSON.stringify(name)}, real.${name}, a) }`)
    else lines.push(`export const ${name} = real.${name}`)
  }
  writeFileSync(join(outDir, mod + ".mjs"), lines.join("\n") + "\n")
}
