import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// The Rust server. In development Vite proxies the API and the tray's
// WebSocket to it, so the browser only talks to one origin.
const server = process.env.EBRU_SERVER ?? 'http://127.0.0.1:8740'

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5174,
    proxy: {
      '/api': server,
      '/ws': { target: server.replace(/^http/, 'ws'), ws: true },
    },
  },
  worker: { format: 'es' },
})
