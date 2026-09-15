// Builds the marbling tray for the browser and copies it where Vite serves it.
import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
execFileSync('cargo', ['build', '-p', 'ebru-wasm', '--target', 'wasm32-unknown-unknown', '--release'], {
  cwd: root,
  stdio: 'inherit',
})
const built = join(root, 'target', 'wasm32-unknown-unknown', 'release', 'ebru_wasm.wasm')
const served = join(root, 'web', 'public', 'ebru.wasm')
mkdirSync(dirname(served), { recursive: true })
copyFileSync(built, served)
console.log(`copied ${built} -> ${served}`)
