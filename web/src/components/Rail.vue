<script setup lang="ts">
import { PIGMENTS } from '../lib/pigments'
import type { Tool } from '../lib/protocol'

// The tool rail along the front of the tray.
const tool = defineModel<Tool>('tool', { required: true })
const pigment = defineModel<number>('pigment', { required: true })
const size = defineModel<number>('size', { required: true })
defineProps<{ laying: boolean }>()
defineEmits<{ lay: [] }>()

const tools: Array<{ id: Tool; label: string; hint: string }> = [
  { id: 'drop', label: 'Brush', hint: 'Click to drop ink. Drag to flick a trail of drops.' },
  { id: 'stylus', label: 'Stylus', hint: 'Drag through the ink to draw it along.' },
  { id: 'comb', label: 'Comb', hint: 'Drag a row of tines through the ink.' },
  { id: 'swirl', label: 'Swirl', hint: 'Hold to turn the ink. Hold Shift to turn it the other way.' },
]
</script>

<template>
  <div class="rail">
    <div class="group tools" role="radiogroup" aria-label="Tool">
      <button
        v-for="t in tools"
        :key="t.id"
        class="tool"
        role="radio"
        :aria-checked="tool === t.id"
        :title="t.hint"
        @click="tool = t.id"
      >
        <svg viewBox="0 0 40 40" aria-hidden="true">
          <template v-if="t.id === 'drop'">
            <path d="M8 33 L22 19" />
            <path d="M22 19 C24 13 30 9 34 7 C32 11 28 17 22 19 Z" class="fill" />
            <path d="M6 35 L10 31" class="thick" />
          </template>
          <template v-else-if="t.id === 'stylus'">
            <path d="M7 34 L27 14" />
            <path d="M27 14 L34 7" class="thick" />
            <circle cx="7" cy="34" r="1.6" class="fill" />
          </template>
          <template v-else-if="t.id === 'comb'">
            <path d="M5 11 H35" class="thick" />
            <path v-for="k in 7" :key="k" :d="`M${5 + (k - 1) * 5} 11 V31`" />
          </template>
          <template v-else>
            <path d="M20 20 m0 -2 a2 2 0 1 1 -2 2 a5 5 0 1 1 5 5 a8 8 0 1 1 -8 -8 a11 11 0 1 1 11 11" />
          </template>
        </svg>
        <span>{{ t.label }}</span>
      </button>
    </div>

    <div class="group pots" role="radiogroup" aria-label="Ink">
      <button
        v-for="p in PIGMENTS"
        :key="p.index"
        class="pot"
        role="radio"
        :aria-checked="pigment === p.index"
        :aria-label="p.name"
        :title="p.name"
        :style="{ '--ink': p.hex }"
        @click="pigment = p.index"
      ></button>
      <span class="pot-name name">{{ PIGMENTS[pigment].name }}</span>
    </div>

    <div class="group sizes" role="radiogroup" aria-label="Size">
      <button
        v-for="(label, i) in ['Small', 'Medium', 'Large']"
        :key="label"
        class="size"
        role="radio"
        :aria-checked="size === i"
        :aria-label="label"
        :title="label"
        @click="size = i"
      >
        <i :style="{ width: `${8 + i * 6}px`, height: `${8 + i * 6}px` }"></i>
      </button>
    </div>

    <button class="lay" :disabled="laying" @click="$emit('lay')">{{ laying ? 'Laying the sheet…' : 'Lay a sheet' }}</button>

    <p class="hint">{{ tools.find((t) => t.id === tool)?.hint }}</p>
  </div>
</template>

<style scoped>
.rail {
  display: grid;
  grid-template-columns: auto 1fr auto auto;
  grid-template-rows: auto auto;
  align-items: center;
  gap: 10px 28px;
}

.group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.tool {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  width: 62px;
  padding: 6px 4px 5px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: none;
  color: var(--bone-dim);
  font-size: 12px;
  cursor: pointer;
  transition:
    color 0.2s,
    border-color 0.2s,
    background 0.2s;
}

.tool svg {
  width: 30px;
  height: 30px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.6;
  stroke-linecap: round;
}

.tool .thick {
  stroke-width: 3.2;
}

.tool .fill {
  fill: currentColor;
}

.tool:hover {
  color: var(--bone);
}

.tool[aria-checked='true'] {
  color: var(--brass);
  border-color: var(--brass-dim);
  background: rgb(199 163 106 / 0.08);
}

.pots {
  justify-content: center;
  gap: 8px;
  position: relative;
  padding-bottom: 16px;
}

/* Ink pots: a glossy bead of each pigment. */
.pot {
  width: 30px;
  height: 30px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background:
    radial-gradient(circle at 32% 28%, rgb(255 255 255 / 0.45), transparent 32%),
    radial-gradient(circle at 60% 70%, rgb(0 0 0 / 0.25), transparent 60%),
    var(--ink);
  box-shadow:
    0 0 0 2px rgb(0 0 0 / 0.35),
    0 2px 5px rgb(0 0 0 / 0.5);
  cursor: pointer;
  transition: transform 0.15s;
}

.pot:hover {
  transform: translateY(-2px);
}

.pot[aria-checked='true'] {
  transform: translateY(-3px);
  box-shadow:
    0 0 0 2px var(--walnut),
    0 0 0 3.5px var(--brass),
    0 4px 8px rgb(0 0 0 / 0.5);
}

.pot-name {
  position: absolute;
  left: 50%;
  bottom: -6px;
  translate: -50% 0;
  font-size: 14px;
  color: var(--bone-dim);
  white-space: nowrap;
}

.size {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 50%;
  background: none;
  cursor: pointer;
}

.size i {
  display: block;
  border-radius: 50%;
  background: var(--bone-faint);
}

.size[aria-checked='true'] {
  border-color: var(--brass-dim);
}

.size[aria-checked='true'] i {
  background: var(--brass);
}

.lay {
  padding: 11px 20px 12px;
  border: 1px solid var(--brass);
  border-radius: 3px;
  background: linear-gradient(180deg, #f1e7d4, #e2d4bb);
  color: #2a1c12;
  font-weight: 600;
  letter-spacing: 0.01em;
  cursor: pointer;
  box-shadow:
    0 1px 0 rgb(255 255 255 / 0.5) inset,
    0 3px 10px rgb(0 0 0 / 0.35);
  transition:
    transform 0.15s,
    box-shadow 0.15s;
}

.lay:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow:
    0 1px 0 rgb(255 255 255 / 0.5) inset,
    0 6px 14px rgb(0 0 0 / 0.4);
}

.lay:disabled {
  opacity: 0.6;
  cursor: default;
}

.hint {
  grid-column: 1 / -1;
  margin: 0;
  font-size: 13px;
  color: var(--bone-faint);
}

@media (max-width: 860px) {
  .rail {
    grid-template-columns: 1fr auto;
  }

  .pots {
    grid-column: 1 / -1;
    order: -1;
    flex-wrap: wrap;
  }
}
</style>
