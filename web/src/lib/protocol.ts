// What travels between the browser, the server and the tray worker.

export const WIDTH = 1600
export const HEIGHT = 1000

export type Op =
  | { op: 'drop'; x: number; y: number; radius: number; pigment: number }
  | { op: 'rake'; from: [number, number]; to: [number, number]; tines: number; spacing: number; reach: number }
  | { op: 'swirl'; x: number; y: number; turn: number; reach: number }
  | { op: 'skim'; ground: number }

export type Tool = 'drop' | 'stylus' | 'comb' | 'swirl'

export type Tally = { drops: number; nested_drops: number; strokes: number; combings: number; swirls: number }

export type Print = {
  id: number
  name: string
  pattern: string
  made_at: number
  people: number
  ground: number
  tally: Tally
}

export type Pigment = { index: number; name: string; hex: string; ground: boolean }

export type ServerMessage =
  | { type: 'hello'; you: number; people: number }
  | { type: 'op'; seq: number; op: Op; by: number | 'tray' }
  | { type: 'cursor'; id: number; x: number; y: number; tool: Tool; pigment: number }
  | { type: 'left'; id: number }
  | { type: 'people'; count: number }
  | { type: 'laying'; by: number | 'tray'; name: string }
  | { type: 'printed'; print: Print }
  | { type: 'checksum'; seq: number; sum: string }
  | { type: 'refused'; to: number | 'tray'; reason: string }
  | { type: 'resync' }

export type ClientMessage =
  | { type: 'op'; op: Op }
  | { type: 'cursor'; x: number; y: number; tool: Tool; pigment: number }
  | { type: 'lay' }

/** A drawable tray, as the worker hands it to the page. */
export type Frame = {
  seq: number
  ground: number
  /** x, y pairs for every ink, one after another. */
  points: Float32Array
  /** Where each ink's pairs start in `points` (in pairs); one extra entry at the end. */
  offsets: Uint32Array
  pigments: Uint8Array
  pending: number
  full: boolean
}

export type ToWorker =
  | { type: 'load'; bytes: ArrayBuffer }
  | { type: 'ops'; ops: Array<{ seq: number; op: Op }> }
  | { type: 'check'; seq: number; sum: string }
  | { type: 'replay'; ground: number; ops: Op[] }
  | { type: 'pace'; substepsPerFrame: number | null }

export type FromWorker =
  | { type: 'ready' }
  | { type: 'frame'; frame: Frame }
  | { type: 'loaded'; seq: number }
  | { type: 'gap'; expected: number; got: number }
  | { type: 'mismatch'; seq: number }
  | { type: 'progress'; done: number; total: number }
