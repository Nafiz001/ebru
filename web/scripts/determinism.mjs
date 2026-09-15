// Replays the recorded recipe natively (Rust) and in WebAssembly (this Node
// process running the browser's module) and checks that every op leaves the
// two trays with the same checksum.
//
//   node scripts/determinism.mjs   (after npm run wasm)

import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const fixture = join(root, 'crates', 'marbling', 'tests', 'fixtures', 'recipe.json')

const native = execFileSync(
  'cargo',
  ['run', '--quiet', '-p', 'marbling', '--release', '--features', 'serde', '--example', 'replay', '--', fixture],
  { cwd: root, encoding: 'utf8', maxBuffer: 1 << 24 },
)
  .trim()
  .split(/\r?\n/)

const { instance } = await WebAssembly.instantiate(readFileSync(join(root, 'web', 'public', 'ebru.wasm')), {})
const t = instance.exports
const recipe = JSON.parse(readFileSync(fixture, 'utf8'))

const empty = new Uint8Array(18)
empty.set([0x45, 0x42, 0x52, 0x55, 1])
empty[13] = recipe.ground
const ptr = t.alloc(empty.length)
new Uint8Array(t.memory.buffer, ptr, empty.length).set(empty)
t.load(ptr, empty.length)

const half = (n) => (n >>> 0).toString(16).padStart(8, '0')
let differ = 0
recipe.ops.forEach((op, i) => {
  if (op.op === 'drop') t.queue_drop(op.x, op.y, op.radius, op.pigment)
  else if (op.op === 'rake') t.queue_rake(op.from[0], op.from[1], op.to[0], op.to[1], op.tines, op.spacing, op.reach)
  else if (op.op === 'swirl') t.queue_swirl(op.x, op.y, op.turn, op.reach)
  while (t.advance(1000) > 0);
  const wasm = `${i} ${half(t.checksum_high())}${half(t.checksum_low())}`
  if (wasm !== native[i]) {
    if (differ++ === 0) console.error(`first difference at op ${i}: native ${native[i]}, wasm ${wasm}`)
  }
})

if (differ) {
  console.error(`${differ} of ${recipe.ops.length} ops left the trays different`)
  process.exit(1)
}
console.log(`${recipe.ops.length} ops: native and WebAssembly trays identical at every step (${native.at(-1)})`)
