// The tray, off the main thread.
//
// This worker runs the same marbling code as the server, compiled from Rust
// to WebAssembly. It queues ops in sequence order, applies their substeps
// within a time budget so ink visibly spreads and drags, and posts a
// drawable frame back to the page whenever the tray has changed. When the
// server announces a checksum it compares it to its own tray, and reports a
// mismatch so the page can reload a snapshot.

import type { Frame, FromWorker, Op, ToWorker } from './protocol'

type Exports = {
  memory: WebAssembly.Memory
  alloc(len: number): number
  release(ptr: number, len: number): void
  load(ptr: number, len: number): number
  queue_drop(x: number, y: number, radius: number, pigment: number): void
  queue_rake(fx: number, fy: number, tx: number, ty: number, tines: number, spacing: number, reach: number): void
  queue_swirl(x: number, y: number, turn: number, reach: number): void
  queue_skim(ground: number): void
  advance(budget: number): number
  seq(): number
  settled(): number
  checksum_high(): number
  checksum_low(): number
  ground(): number
  points(): number
  full(): number
  ink_count(): number
  ink_pigment(i: number): number
  ink_length(i: number): number
  ink_outline(i: number): number
}

const post = (message: FromWorker, transfer: Transferable[] = []) =>
  (self as unknown as Worker).postMessage(message, transfer)

let tray: Exports | null = null
let lastQueued = -1
let changed = true
let lastFrame = 0
let pendingCheck: { seq: number; sum: string } | null = null
// null: keep up with the server. A number: a fixed pace, for replays.
let substepsPerFrame: number | null = null
let replayTotal = 0

function queue(op: Op) {
  if (!tray) return
  switch (op.op) {
    case 'drop':
      tray.queue_drop(op.x, op.y, op.radius, op.pigment)
      break
    case 'rake':
      tray.queue_rake(op.from[0], op.from[1], op.to[0], op.to[1], op.tines, op.spacing, op.reach)
      break
    case 'swirl':
      tray.queue_swirl(op.x, op.y, op.turn, op.reach)
      break
    case 'skim':
      tray.queue_skim(op.ground)
      break
  }
  changed = true
}

// WebAssembly hands u32s back as signed i32s; `>>> 0` reads the same bits unsigned.
function checksum(t: Exports) {
  const half = (n: number) => (n >>> 0).toString(16).padStart(8, '0')
  return half(t.checksum_high()) + half(t.checksum_low())
}

function frame(t: Exports, pending: number): Frame {
  const count = t.ink_count()
  const offsets = new Uint32Array(count + 1)
  const pigments = new Uint8Array(count)
  let total = 0
  for (let i = 0; i < count; i++) {
    offsets[i] = total
    total += t.ink_length(i)
    pigments[i] = t.ink_pigment(i)
  }
  offsets[count] = total
  const points = new Float32Array(total * 2)
  for (let i = 0; i < count; i++) {
    const length = offsets[i + 1] - offsets[i]
    const source = new Float64Array(t.memory.buffer, t.ink_outline(i), length * 2)
    points.set(source, offsets[i] * 2)
  }
  return { seq: t.seq(), ground: t.ground(), points, offsets, pigments, pending, full: t.full() === 1 }
}

function tick() {
  if (tray) {
    const t = tray
    const started = performance.now()
    let remaining = t.advance(0)
    const hadWork = remaining > 0
    if (substepsPerFrame === null) {
      // Keep up: spend up to 12ms a tick, and more if we're falling behind.
      const budget = remaining > 400 ? 30 : 12
      while (remaining > 0 && performance.now() - started < budget) remaining = t.advance(1)
    } else if (hadWork) {
      remaining = t.advance(substepsPerFrame)
    }
    changed = changed || hadWork

    if (pendingCheck && t.settled() && t.seq() === pendingCheck.seq) {
      if (checksum(t) !== pendingCheck.sum) post({ type: 'mismatch', seq: pendingCheck.seq })
      pendingCheck = null
    }

    const now = performance.now()
    if (changed && now - lastFrame > 30) {
      const f = frame(t, remaining)
      changed = remaining > 0
      lastFrame = now
      post({ type: 'frame', frame: f }, [f.points.buffer, f.offsets.buffer, f.pigments.buffer])
      if (substepsPerFrame !== null) post({ type: 'progress', done: replayTotal - remaining, total: replayTotal })
    }
  }
  setTimeout(tick, 16)
}

self.onmessage = (event: MessageEvent<ToWorker>) => {
  const message = event.data
  if (!tray) return
  switch (message.type) {
    case 'load': {
      const bytes = new Uint8Array(message.bytes)
      const ptr = tray.alloc(bytes.length)
      new Uint8Array(tray.memory.buffer, ptr, bytes.length).set(bytes)
      tray.load(ptr, bytes.length)
      tray.release(ptr, bytes.length)
      lastQueued = tray.seq()
      changed = true
      post({ type: 'loaded', seq: lastQueued })
      break
    }
    case 'ops':
      for (const { seq, op } of message.ops) {
        if (seq <= lastQueued) continue
        if (seq !== lastQueued + 1) {
          post({ type: 'gap', expected: lastQueued + 1, got: seq })
          return
        }
        queue(op)
        lastQueued = seq
      }
      break
    case 'check':
      pendingCheck = message
      break
    case 'pace':
      substepsPerFrame = message.substepsPerFrame
      break
    case 'replay': {
      // A fresh tray, played through a print's recipe at a steady pace.
      const empty = new Uint8Array(4 + 1 + 8 + 1 + 4)
      empty.set([0x45, 0x42, 0x52, 0x55, 1])
      empty[13] = message.ground
      const ptr = tray.alloc(empty.length)
      new Uint8Array(tray.memory.buffer, ptr, empty.length).set(empty)
      tray.load(ptr, empty.length)
      tray.release(ptr, empty.length)
      for (const op of message.ops) queue(op)
      replayTotal = tray.advance(0)
      lastQueued = -1
      break
    }
  }
}

;(async () => {
  const response = fetch(new URL('/ebru.wasm', self.location.origin))
  const { instance } = await WebAssembly.instantiateStreaming(response, {})
  tray = instance.exports as unknown as Exports
  post({ type: 'ready' })
  tick()
})()
