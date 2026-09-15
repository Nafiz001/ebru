// The shared tray, from the page's side: a WebSocket to the server, a worker
// running the marbling, and the state the interface draws from.
//
// Joining works like this. The server says hello; the page fetches a snapshot
// of the whole tray; ops that arrive while the snapshot is on its way are
// buffered; once the worker has loaded the snapshot, buffered ops newer than
// it are handed over and everything after flows straight through. Any gap in
// sequence numbers, a checksum that doesn't match, or the server saying this
// page fell behind, and the page quietly does the whole thing again.

import { reactive, ref, shallowRef } from 'vue'
import type { ClientMessage, Frame, FromWorker, Op, Print, ServerMessage, Tool, ToWorker } from './protocol'

export type Cursor = { id: number; x: number; y: number; tool: Tool; pigment: number; at: number }

type Listeners = {
  laying: (by: number | 'tray', name: string) => void
  printed: (print: Print) => void
  refused: (reason: string) => void
}

export function createWorker() {
  const worker = new Worker(new URL('./tray.worker.ts', import.meta.url), { type: 'module' })
  const ready = new Promise<void>((resolve) => {
    const onReady = (event: MessageEvent<FromWorker>) => {
      if (event.data.type === 'ready') {
        worker.removeEventListener('message', onReady)
        resolve()
      }
    }
    worker.addEventListener('message', onReady)
  })
  const post = (message: ToWorker, transfer: Transferable[] = []) => worker.postMessage(message, transfer)
  return { worker, ready, post }
}

export class SharedTray {
  frame = shallowRef<Frame | null>(null)
  you = ref(0)
  people = ref(1)
  connected = ref(false)
  cursors = reactive(new Map<number, Cursor>())

  #worker = createWorker()
  #socket: WebSocket | null = null
  #loading = false
  #buffer: Array<{ seq: number; op: Op }> = []
  #listeners: { [K in keyof Listeners]: Set<Listeners[K]> } = { laying: new Set(), printed: new Set(), refused: new Set() }
  #closed = false
  #retry = 0
  #lastCursor = 0

  constructor() {
    this.#worker.worker.addEventListener('message', (event: MessageEvent<FromWorker>) => this.#fromWorker(event.data))
    this.#worker.ready.then(() => this.#connect())
  }

  on<K extends keyof Listeners>(event: K, listener: Listeners[K]) {
    this.#listeners[event].add(listener)
    return () => this.#listeners[event].delete(listener)
  }

  op(op: Op) {
    this.#send({ type: 'op', op })
  }

  lay() {
    this.#send({ type: 'lay' })
  }

  cursor(x: number, y: number, tool: Tool, pigment: number) {
    const now = performance.now()
    if (now - this.#lastCursor < 50) return
    this.#lastCursor = now
    this.#send({ type: 'cursor', x: Math.round(x * 10) / 10, y: Math.round(y * 10) / 10, tool, pigment })
  }

  close() {
    this.#closed = true
    this.#socket?.close()
    this.#worker.worker.terminate()
  }

  #send(message: ClientMessage) {
    if (this.#socket?.readyState === WebSocket.OPEN) this.#socket.send(JSON.stringify(message))
  }

  #connect() {
    const protocol = location.protocol === 'https:' ? 'wss' : 'ws'
    const socket = new WebSocket(`${protocol}://${location.host}/ws`)
    this.#socket = socket
    socket.onopen = () => {
      this.#retry = 0
      this.connected.value = true
    }
    socket.onmessage = (event) => this.#fromServer(JSON.parse(event.data) as ServerMessage)
    socket.onclose = () => {
      this.connected.value = false
      this.cursors.clear()
      if (this.#closed) return
      this.#retry = Math.min(this.#retry + 1, 6)
      setTimeout(() => this.#connect(), 400 * 2 ** this.#retry)
    }
  }

  async #resync() {
    if (this.#loading) return
    this.#loading = true
    this.#buffer = []
    try {
      const response = await fetch('/api/tray', { cache: 'no-store' })
      const bytes = await response.arrayBuffer()
      this.#worker.post({ type: 'load', bytes }, [bytes])
    } catch {
      this.#loading = false
      setTimeout(() => this.#resync(), 2000)
    }
  }

  #fromServer(message: ServerMessage) {
    switch (message.type) {
      case 'hello':
        this.you.value = message.you
        this.people.value = message.people
        this.#resync()
        break
      case 'op':
        if (this.#loading) this.#buffer.push({ seq: message.seq, op: message.op })
        else this.#worker.post({ type: 'ops', ops: [{ seq: message.seq, op: message.op }] })
        break
      case 'cursor':
        if (message.id !== this.you.value) this.cursors.set(message.id, { ...message, at: performance.now() })
        break
      case 'left':
        this.cursors.delete(message.id)
        break
      case 'people':
        this.people.value = message.count
        break
      case 'laying':
        this.#listeners.laying.forEach((listener) => listener(message.by, message.name))
        break
      case 'printed':
        this.#listeners.printed.forEach((listener) => listener(message.print))
        break
      case 'checksum':
        if (!this.#loading) this.#worker.post({ type: 'check', seq: message.seq, sum: message.sum })
        break
      case 'refused':
        if (message.to === this.you.value) this.#listeners.refused.forEach((listener) => listener(message.reason))
        break
      case 'resync':
        this.#resync()
        break
    }
  }

  #fromWorker(message: FromWorker) {
    switch (message.type) {
      case 'frame':
        this.frame.value = message.frame
        break
      case 'loaded': {
        this.#loading = false
        const ops = this.#buffer.filter((entry) => entry.seq > message.seq)
        this.#buffer = []
        if (ops.length) this.#worker.post({ type: 'ops', ops })
        break
      }
      case 'gap':
      case 'mismatch':
        console.warn('ebru: tray out of step with the server; reloading it', message)
        this.#resync()
        break
    }
  }
}
