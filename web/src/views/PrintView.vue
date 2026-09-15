<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { api, madeBy, when } from '../lib/api'
import { drawFrame } from '../lib/draw'
import { PIGMENTS } from '../lib/pigments'
import type { FromWorker, Print } from '../lib/protocol'
import { createWorker } from '../lib/tray'

const props = defineProps<{ id: string }>()
const print = ref<Print | null>(null)
const error = ref<string | null>(null)

// Watching it being made: the recipe played through a private tray.
const replaying = ref(false)
const progress = ref(0)
const canvas = ref<HTMLCanvasElement>()
let worker: ReturnType<typeof createWorker> | null = null

async function replay() {
  if (!print.value || !canvas.value) return
  const recipe = await api.recipe(print.value.id)
  replaying.value = true
  progress.value = 0
  worker?.worker.terminate()
  const w = createWorker()
  worker = w
  const c = canvas.value
  c.width = 1600
  c.height = 1000
  const ctx = c.getContext('2d')!
  w.worker.addEventListener('message', (event: MessageEvent<FromWorker>) => {
    const message = event.data
    if (message.type === 'frame') drawFrame(ctx, message.frame, c.width, c.height)
    if (message.type === 'progress') {
      progress.value = message.total ? message.done / message.total : 1
      if (message.done >= message.total) setTimeout(() => (replaying.value = false), 1200)
    }
  })
  await w.ready
  // A steady pace, quick enough to watch a whole print in under a minute.
  const substeps = recipe.ops.length * 12
  w.post({ type: 'pace', substepsPerFrame: Math.max(2, Math.ceil(substeps / 1400)) })
  w.post({ type: 'replay', ground: recipe.ground, ops: recipe.ops })
}

onMounted(async () => {
  try {
    print.value = await api.print(Number(props.id))
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
})
onBeforeUnmount(() => worker?.worker.terminate())

const recipe = computed(() => {
  const t = print.value?.tally
  if (!t) return []
  const parts: string[] = []
  if (t.drops) parts.push(`${t.drops} drops of ink${t.nested_drops ? `, ${t.nested_drops} of them inside others` : ''}`)
  if (t.strokes) parts.push(`${t.strokes} strokes of the stylus`)
  if (t.combings) parts.push(`${t.combings} passes of the comb`)
  if (t.swirls) parts.push(`${t.swirls} turns of the swirl`)
  return parts
})
</script>

<template>
  <main class="print-page">
    <RouterLink to="/prints" class="back">All prints</RouterLink>
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="print">
      <figure>
        <div class="frame">
          <img :src="api.image(print.id)" :alt="print.name" width="2400" height="1500" />
          <canvas v-show="replaying" ref="canvas" aria-label="The print being made again"></canvas>
          <div v-if="replaying" class="progress" :style="{ width: `${progress * 100}%` }"></div>
        </div>
        <figcaption>
          <h1 class="name">{{ print.name }}</h1>
          <p class="made">{{ madeBy(print) }}, {{ when(print.made_at) }}, on a ground of {{ PIGMENTS[print.ground].name.toLowerCase() }}.</p>
          <p v-if="recipe.length" class="recipe">{{ recipe.join('; ') }}.</p>
          <div class="actions">
            <button :disabled="replaying" @click="replay">{{ replaying ? 'Marbling it again…' : 'Watch it being made' }}</button>
            <a :href="api.image(print.id)" :download="`${print.name}.png`">Download the print</a>
          </div>
        </figcaption>
      </figure>
    </template>
  </main>
</template>

<style scoped>
.print-page {
  max-width: 1300px;
  margin: 0 auto;
  padding: 28px clamp(16px, 3vw, 40px) 60px;
}

.back {
  color: var(--brass);
  text-decoration: none;
  font-size: 14px;
}

figure {
  margin: 26px 0 0;
}

.frame {
  position: relative;
  padding: 18px;
  background: linear-gradient(180deg, #efe8da, #e2d7c3);
  box-shadow: 0 30px 60px -20px rgb(0 0 0 / 0.75);
}

img,
canvas {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 1.6;
}

canvas {
  position: absolute;
  inset: 18px;
  width: calc(100% - 36px);
}

.progress {
  position: absolute;
  left: 18px;
  bottom: 8px;
  height: 3px;
  max-width: calc(100% - 36px);
  background: var(--brass);
  transition: width 0.2s linear;
}

figcaption {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 4px 30px;
  margin-top: 22px;
}

h1 {
  margin: 0;
  font-size: 44px;
  line-height: 1.1;
}

.made,
.recipe {
  grid-column: 1;
  margin: 0;
  color: var(--bone-dim);
}

.recipe {
  color: var(--bone-faint);
  font-size: 14px;
}

.actions {
  grid-column: 2;
  grid-row: 1 / span 3;
  display: flex;
  align-items: center;
  gap: 18px;
}

.actions button {
  padding: 10px 18px;
  border: 1px solid var(--brass);
  border-radius: 3px;
  background: linear-gradient(180deg, #f1e7d4, #e2d4bb);
  color: #2a1c12;
  font-weight: 600;
  cursor: pointer;
}

.actions button:disabled {
  opacity: 0.6;
}

.actions a {
  color: var(--brass);
  font-size: 14px;
}

@media (max-width: 760px) {
  figcaption {
    grid-template-columns: 1fr;
  }

  .actions {
    grid-column: 1;
    grid-row: auto;
    margin-top: 12px;
  }
}
</style>
