<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { api, madeBy, when } from '../lib/api'
import type { Print } from '../lib/protocol'

const prints = ref<Print[]>([])
const error = ref<string | null>(null)
const more = ref(true)
const loading = ref(false)

async function load() {
  if (loading.value || !more.value) return
  loading.value = true
  try {
    const before = prints.value.at(-1)?.id
    const page = await api.prints(before)
    prints.value.push(...page)
    more.value = page.length === 24
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

<template>
  <main class="gallery">
    <header>
      <RouterLink to="/" class="back">Back to the tray</RouterLink>
      <h1 class="name">Prints</h1>
      <p>Every sheet laid on the tray, hung here to dry. Newest first.</p>
    </header>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-else-if="!loading && !prints.length" class="empty">
      No sheets yet. <RouterLink to="/">Go and marble the first one.</RouterLink>
    </p>

    <ul class="wall">
      <li v-for="(print, i) in prints" :key="print.id" :style="{ '--tilt': `${((print.id * 37) % 7) - 3}deg`, '--delay': `${Math.min(i, 12) * 40}ms` }">
        <RouterLink :to="`/prints/${print.id}`" class="hung">
          <span class="clip" aria-hidden="true"></span>
          <img :src="api.image(print.id, true)" :alt="print.name" loading="lazy" width="640" height="400" />
          <span class="label">
            <span class="name">{{ print.name }}</span>
            <small>{{ madeBy(print) }}, {{ when(print.made_at) }}</small>
          </span>
        </RouterLink>
      </li>
    </ul>

    <button v-if="more && prints.length" class="more" :disabled="loading" @click="load">
      {{ loading ? 'Fetching…' : 'Older prints' }}
    </button>
  </main>
</template>

<style scoped>
.gallery {
  max-width: 1400px;
  margin: 0 auto;
  padding: 28px clamp(16px, 3vw, 40px) 60px;
}

header {
  margin-bottom: 36px;
}

.back {
  color: var(--brass);
  text-decoration: none;
  font-size: 14px;
}

h1 {
  margin: 14px 0 6px;
  font-size: 56px;
  line-height: 1;
}

header p {
  margin: 0;
  color: var(--bone-dim);
}

/* A drying line: prints clipped up in rows, each hanging slightly askew. */
.wall {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 44px 30px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.wall li {
  animation: hang 0.6s var(--delay) both cubic-bezier(0.3, 0.7, 0.3, 1.2);
}

@keyframes hang {
  from {
    opacity: 0;
    transform: translateY(-14px) rotate(0);
  }
}

.hung {
  position: relative;
  display: block;
  color: inherit;
  text-decoration: none;
  transform: rotate(var(--tilt));
  transform-origin: 50% 0;
  transition: transform 0.4s cubic-bezier(0.3, 0.7, 0.3, 1.4);
}

.hung:hover,
.hung:focus-visible {
  transform: rotate(0) translateY(-3px);
}

.clip {
  position: absolute;
  left: 50%;
  top: -10px;
  z-index: 1;
  width: 34px;
  height: 18px;
  translate: -50% 0;
  border-radius: 3px 3px 1px 1px;
  background: linear-gradient(180deg, #d9bb87, #9c7a45);
  box-shadow: 0 2px 3px rgb(0 0 0 / 0.5);
}

img {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 1.6;
  background: #3a291d;
  box-shadow:
    0 1px 0 rgb(255 255 255 / 0.08),
    0 18px 30px -8px rgb(0 0 0 / 0.7);
}

.label {
  display: block;
  margin-top: 12px;
}

.label .name {
  display: block;
  font-size: 21px;
}

.label small {
  color: var(--bone-faint);
  font-size: 13px;
}

.more {
  display: block;
  margin: 50px auto 0;
  padding: 10px 18px;
  border: 1px solid var(--brass-dim);
  border-radius: 3px;
  background: none;
  color: var(--brass);
  cursor: pointer;
}

.empty a,
.error {
  color: var(--brass);
}
</style>
