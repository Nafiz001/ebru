<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'
import { RouterLink } from 'vue-router'
import Rail from '../components/Rail.vue'
import TrayCanvas from '../components/TrayCanvas.vue'
import type { Print, Tool } from '../lib/protocol'
import { SharedTray } from '../lib/tray'

const tray = new SharedTray()
onBeforeUnmount(() => tray.close())

const tool = ref<Tool>('drop')
const pigment = ref(0)
const size = ref(1)

const canvas = ref<InstanceType<typeof TrayCanvas>>()

// Laying a sheet: the paper comes down on the bath, lifts off with the
// pattern, and goes to dry in the gallery while the bath is skimmed.
type Sheet = { image: string; name: string; key: number }
const sheet = ref<Sheet | null>(null)
const laying = ref(false)
const notice = ref<{ text: string; print?: Print; key: number } | null>(null)

function say(text: string, print?: Print) {
  notice.value = { text, print, key: Date.now() }
}

tray.on('laying', (by, name) => {
  laying.value = true
  sheet.value = { image: canvas.value?.capture() ?? '', name, key: Date.now() }
  const who = by === tray.you.value ? 'You laid' : by === 'tray' ? 'The tray laid' : 'Someone laid'
  say(`${who} a sheet on the bath. It’s drying.`)
  setTimeout(() => (sheet.value = null), 3600)
})

tray.on('printed', (print) => {
  laying.value = false
  say(`${print.name} is hanging in the gallery.`, print)
})

tray.on('refused', (reason) => {
  laying.value = false
  say(reason)
})

const people = () => {
  const n = tray.people.value
  return n <= 1 ? 'Only you at the tray' : `${n} people at the tray`
}
</script>

<template>
  <main class="workshop">
    <header>
      <h1 class="name">ebru</h1>
      <p class="lede">A marbling tray shared by everyone who opens it. Float ink, draw it into a pattern together, and lay a sheet to keep a print.</p>
      <p class="presence">
        <span :class="{ offline: !tray.connected.value }">{{ tray.connected.value ? people() : 'Reconnecting…' }}</span>
        <RouterLink to="/prints" class="prints-link">Prints</RouterLink>
      </p>
    </header>

    <section class="tray" aria-label="Tray">
      <div class="rim">
        <TrayCanvas ref="canvas" :tray="tray" :tool="tool" :pigment="pigment" :size="size" />
        <div v-if="sheet" :key="sheet.key" class="sheet" aria-hidden="true">
          <div class="paper"></div>
          <div class="print" :style="{ backgroundImage: `url(${sheet.image})` }"></div>
        </div>
        <p v-if="tray.frame.value?.full" class="full name">The tray is full. Lay a sheet to begin again.</p>
      </div>
    </section>

    <Rail v-model:tool="tool" v-model:pigment="pigment" v-model:size="size" :laying="laying" @lay="tray.lay()" />

    <p v-if="notice" :key="notice.key" class="notice" role="status">
      {{ notice.text }}
      <RouterLink v-if="notice.print" :to="`/prints/${notice.print.id}`">See it</RouterLink>
    </p>
  </main>
</template>

<style scoped>
.workshop {
  display: grid;
  grid-template-rows: auto 1fr auto;
  gap: 18px;
  height: 100vh;
  height: 100dvh;
  padding: 18px clamp(16px, 3vw, 40px) 16px;
}

header {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: baseline;
  gap: 24px;
}

h1 {
  margin: 0;
  font-size: 44px;
  line-height: 1;
  color: var(--bone);
}

.lede {
  margin: 0;
  max-width: 62ch;
  color: var(--bone-dim);
  font-size: 14px;
}

.presence {
  display: flex;
  gap: 22px;
  margin: 0;
  font-size: 14px;
  color: var(--bone-dim);
}

.offline {
  color: #e8a37f;
}

.prints-link {
  color: var(--brass);
  text-decoration: none;
  border-bottom: 1px solid var(--brass-dim);
}

.tray {
  display: grid;
  place-items: center;
  min-height: 0;
}

/* The trough: a wooden rim with a brass lip around the bath. */
.rim {
  position: relative;
  aspect-ratio: 1.6;
  width: min(100%, calc((100dvh - 250px) * 1.6));
  padding: 12px;
  border-radius: 8px;
  background: linear-gradient(180deg, #4a3223, #2c1d13);
  box-shadow:
    0 0 0 1px rgb(0 0 0 / 0.5),
    0 1px 0 1px rgb(255 255 255 / 0.06) inset,
    0 30px 60px -20px rgb(0 0 0 / 0.7);
}

.rim::after {
  content: '';
  position: absolute;
  inset: 10px;
  border-radius: 4px;
  pointer-events: none;
  box-shadow:
    0 0 0 2px rgb(199 163 106 / 0.35),
    inset 0 2px 10px rgb(0 0 0 / 0.45);
}

.sheet {
  position: absolute;
  inset: 12px;
  pointer-events: none;
  perspective: 1400px;
}

.sheet .paper,
.sheet .print {
  position: absolute;
  inset: -1%;
  border-radius: 2px;
  background-size: cover;
  box-shadow: 0 20px 40px rgb(0 0 0 / 0.45);
}

.sheet .paper {
  background:
    repeating-linear-gradient(90deg, rgb(0 0 0 / 0.018) 0 1px, transparent 1px 4px),
    linear-gradient(180deg, #f4eee2, #e9e0cf);
  animation: lower 3.6s cubic-bezier(0.3, 0.7, 0.2, 1) forwards;
}

.sheet .print {
  opacity: 0;
  transform-origin: 50% 0;
  animation: lift 3.6s cubic-bezier(0.5, 0, 0.3, 1) forwards;
}

@keyframes lower {
  0% {
    transform: translateY(-115%) rotate(-1.5deg);
  }
  28% {
    transform: translateY(0) rotate(0);
    opacity: 1;
  }
  40% {
    opacity: 1;
  }
  41%,
  100% {
    opacity: 0;
  }
}

@keyframes lift {
  0%,
  40% {
    opacity: 0;
    transform: none;
  }
  41% {
    opacity: 1;
    transform: none;
  }
  70% {
    opacity: 1;
    transform: rotateX(-14deg) translateY(-6%) scale(0.98);
  }
  100% {
    opacity: 0;
    transform: translate(38%, -120%) scale(0.18) rotate(6deg);
  }
}

.full {
  position: absolute;
  left: 50%;
  top: 50%;
  translate: -50% -50%;
  margin: 0;
  padding: 10px 18px;
  background: rgb(35 23 15 / 0.85);
  border-radius: 4px;
  font-size: 20px;
}

.notice {
  position: fixed;
  left: 50%;
  top: 20px;
  translate: -50% 0;
  margin: 0;
  padding: 10px 18px;
  border: 1px solid var(--brass-dim);
  border-radius: 4px;
  background: rgb(35 23 15 / 0.92);
  font-size: 14px;
  box-shadow: 0 10px 30px rgb(0 0 0 / 0.4);
  animation: notice 6s both;
}

.notice a {
  margin-left: 10px;
  color: var(--brass);
}

@keyframes notice {
  0% {
    opacity: 0;
    translate: -50% -8px;
  }
  6%,
  88% {
    opacity: 1;
    translate: -50% 0;
  }
  100% {
    opacity: 0;
  }
}

@media (max-width: 860px) {
  header {
    grid-template-columns: auto 1fr;
  }

  .lede {
    display: none;
  }

  .rim {
    width: 100%;
  }
}
</style>
