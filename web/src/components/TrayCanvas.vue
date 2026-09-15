<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { drawFrame } from '../lib/draw'
import { hex } from '../lib/pigments'
import type { Op, Tool } from '../lib/protocol'
import { HEIGHT, WIDTH } from '../lib/protocol'
import type { SharedTray } from '../lib/tray'

const props = defineProps<{ tray: SharedTray; tool: Tool; pigment: number; size: number }>()
defineExpose({ capture })

const box = ref<HTMLDivElement>()
const ink = ref<HTMLCanvasElement>()
const over = ref<HTMLCanvasElement>()
let width = 0
let height = 0

// Tools, by size (small, medium, large).
const DROP = [14, 32, 60]
const STYLUS = [7, 12, 22]
const COMB_SPACING = [16, 24, 36]
const SWIRL = [26, 44, 80]

function resize() {
  if (!box.value || !ink.value || !over.value) return
  const rect = box.value.getBoundingClientRect()
  const dpr = Math.min(window.devicePixelRatio || 1, 2)
  width = Math.round(rect.width * dpr)
  height = Math.round(rect.height * dpr)
  for (const canvas of [ink.value, over.value]) {
    canvas.width = width
    canvas.height = height
  }
  paint()
}

function paint() {
  const frame = props.tray.frame.value
  const ctx = ink.value?.getContext('2d')
  if (!frame || !ctx || !width) return
  drawFrame(ctx, frame, width, height)
}

/** The tray as it looks right now, for the sheet that's laid on it. */
function capture() {
  return ink.value?.toDataURL('image/jpeg', 0.9) ?? ''
}

watch(() => props.tray.frame.value, paint)

// Pointer handling: from screen to tray units, and gestures to ops.
function at(event: PointerEvent): [number, number] {
  const rect = over.value!.getBoundingClientRect()
  return [((event.clientX - rect.left) / rect.width) * WIDTH, ((event.clientY - rect.top) / rect.height) * HEIGHT]
}

type Mark = { x: number; y: number; t: number; kind: 'trail' | 'ring' }
const marks: Mark[] = []
let down = false
let last: [number, number] | null = null
let travelled = 0
let lastStroke = 0
let swirlTimer: ReturnType<typeof setInterval> | null = null
let pointer: [number, number] = [WIDTH / 2, HEIGHT / 2]

function send(op: Op) {
  props.tray.op(op)
}

function drop(x: number, y: number, radius: number) {
  send({ op: 'drop', x, y, radius, pigment: props.pigment })
  marks.push({ x, y, t: performance.now(), kind: 'ring' })
}

function rake(from: [number, number], to: [number, number]) {
  const [dx, dy] = [to[0] - from[0], to[1] - from[1]]
  const length = Math.hypot(dx, dy)
  const pieces = Math.max(1, Math.ceil(length / 200))
  for (let k = 0; k < pieces; k++) {
    const a: [number, number] = [from[0] + (dx * k) / pieces, from[1] + (dy * k) / pieces]
    const b: [number, number] = [from[0] + (dx * (k + 1)) / pieces, from[1] + (dy * (k + 1)) / pieces]
    if (props.tool === 'comb') {
      send({ op: 'rake', from: a, to: b, tines: 9, spacing: COMB_SPACING[props.size], reach: 6 })
    } else {
      send({ op: 'rake', from: a, to: b, tines: 1, spacing: 20, reach: STYLUS[props.size] })
    }
  }
}

function onDown(event: PointerEvent) {
  if (event.button !== 0) return
  over.value!.setPointerCapture(event.pointerId)
  down = true
  const p = at(event)
  pointer = p
  last = p
  travelled = 0
  if (props.tool === 'drop') {
    drop(p[0], p[1], DROP[props.size])
  } else if (props.tool === 'swirl') {
    const turn = event.shiftKey ? -0.35 : 0.35
    const spin = () => send({ op: 'swirl', x: pointer[0], y: pointer[1], turn, reach: SWIRL[props.size] })
    spin()
    swirlTimer = setInterval(spin, 70)
  }
}

function onMove(event: PointerEvent) {
  const p = at(event)
  pointer = p
  props.tray.cursor(p[0], p[1], props.tool, props.pigment)
  if (!down || !last) return

  if (props.tool === 'drop') {
    // Dragging the brush flicks a trail of smaller drops.
    travelled += Math.hypot(p[0] - last[0], p[1] - last[1])
    last = p
    if (travelled > 70) {
      travelled = 0
      const jitter = () => (Math.random() - 0.5) * 40
      drop(p[0] + jitter(), p[1] + jitter(), DROP[props.size] * (0.35 + Math.random() * 0.4))
    }
  } else if (props.tool === 'stylus' || props.tool === 'comb') {
    // The trail shows every movement at once; the strokes themselves go out
    // at most every 40ms, each one covering the whole path since the last.
    const now = performance.now()
    marks.push({ x: p[0], y: p[1], t: now, kind: 'trail' })
    if (Math.hypot(p[0] - last[0], p[1] - last[1]) >= 16 && now - lastStroke >= 40) {
      rake(last, p)
      last = p
      lastStroke = now
    }
  }
}

function onUp(event?: PointerEvent) {
  // Finish the stroke to where the pointer let go.
  if (down && last && event && (props.tool === 'stylus' || props.tool === 'comb')) {
    const p = at(event)
    if (Math.hypot(p[0] - last[0], p[1] - last[1]) >= 4) rake(last, p)
  }
  down = false
  last = null
  if (swirlTimer) clearInterval(swirlTimer)
  swirlTimer = null
}

// The overlay: your own trails and rings, and everyone else's tools.
let raf = 0
function overlay() {
  raf = requestAnimationFrame(overlay)
  const ctx = over.value?.getContext('2d')
  if (!ctx || !width) return
  const scale = width / WIDTH
  const now = performance.now()
  ctx.clearRect(0, 0, width, height)

  while (marks.length && now - marks[0].t > 700) marks.shift()
  ctx.lineCap = 'round'
  for (let i = 0; i < marks.length; i++) {
    const m = marks[i]
    const age = (now - m.t) / 700
    if (m.kind === 'ring') {
      ctx.strokeStyle = `rgb(255 250 240 / ${0.5 * (1 - age)})`
      ctx.lineWidth = 1.5 * scale
      ctx.beginPath()
      ctx.arc(m.x * scale, m.y * scale, (8 + age * 50) * scale, 0, Math.PI * 2)
      ctx.stroke()
    } else if (i > 0 && marks[i - 1].kind === 'trail' && m.t - marks[i - 1].t < 120) {
      const p = marks[i - 1]
      ctx.strokeStyle = `rgb(255 250 240 / ${0.35 * (1 - age)})`
      ctx.lineWidth = (props.tool === 'comb' ? 3 : 1.6) * scale
      ctx.beginPath()
      ctx.moveTo(p.x * scale, p.y * scale)
      ctx.lineTo(m.x * scale, m.y * scale)
      ctx.stroke()
    }
  }

  for (const cursor of props.tray.cursors.values()) {
    if (now - cursor.at > 6000) continue
    const x = cursor.x * scale
    const y = cursor.y * scale
    ctx.fillStyle = hex(cursor.pigment)
    ctx.strokeStyle = 'rgb(255 248 236 / 0.85)'
    ctx.lineWidth = 1.5 * Math.max(1, scale)
    ctx.beginPath()
    ctx.arc(x, y, 7 * Math.max(1, scale), 0, Math.PI * 2)
    ctx.fill()
    ctx.stroke()
    ctx.font = `italic ${13 * Math.max(1, scale)}px Castoro, Georgia, serif`
    ctx.fillStyle = 'rgb(255 248 236 / 0.9)'
    ctx.fillText(cursor.tool === 'drop' ? 'brush' : cursor.tool, x + 12 * Math.max(1, scale), y + 4 * Math.max(1, scale))
  }
}

let observer: ResizeObserver
onMounted(() => {
  observer = new ResizeObserver(resize)
  observer.observe(box.value!)
  resize()
  overlay()
})

onBeforeUnmount(() => {
  observer.disconnect()
  cancelAnimationFrame(raf)
  if (swirlTimer) clearInterval(swirlTimer)
})
</script>

<template>
  <div ref="box" class="bath" :class="`tool-${tool}`">
    <canvas ref="ink" class="ink" aria-hidden="true"></canvas>
    <div class="sheen" aria-hidden="true"></div>
    <canvas
      ref="over"
      class="over"
      role="img"
      aria-label="The marbling tray"
      @pointerdown="onDown"
      @pointermove="onMove"
      @pointerup="onUp"
      @pointercancel="onUp"
      @pointerleave="onUp"
    ></canvas>
    <p v-if="!tray.frame.value" class="waiting name">Filling the tray…</p>
  </div>
</template>

<style scoped>
.bath {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  border-radius: 3px;
  background: #d9d2c2;
  touch-action: none;
}

canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

.over {
  cursor: crosshair;
}

.tool-drop .over {
  cursor: cell;
}

/* Light moving slowly across a wet surface. */
.sheen {
  position: absolute;
  inset: -20%;
  pointer-events: none;
  background:
    radial-gradient(40% 30% at 30% 25%, rgb(255 255 255 / 0.13), transparent 70%),
    radial-gradient(30% 25% at 75% 70%, rgb(255 255 255 / 0.07), transparent 70%);
  mix-blend-mode: soft-light;
  animation: drift 26s ease-in-out infinite alternate;
}

@keyframes drift {
  to {
    transform: translate(6%, 4%) scale(1.05);
  }
}

.waiting {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  margin: 0;
  font-size: 22px;
  color: rgb(40 28 20 / 0.6);
}

@media (prefers-reduced-motion: reduce) {
  .sheen {
    animation: none;
  }
}
</style>
