import type { Op, Print } from './protocol'

async function json<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => null)
    throw new Error(body?.error ?? 'The workshop isn’t answering. Is the server running?')
  }
  return response.json()
}

export const api = {
  prints: (before?: number) => fetch(`/api/prints${before ? `?before=${before}` : ''}`).then((r) => json<Print[]>(r)),
  print: (id: number) => fetch(`/api/prints/${id}`).then((r) => json<Print>(r)),
  recipe: (id: number) => fetch(`/api/prints/${id}/recipe`).then((r) => json<{ ground: number; ops: Op[] }>(r)),
  image: (id: number, small = false) => `/api/prints/${id}/image${small ? '?size=small' : ''}`,
}

export function madeBy(print: Print) {
  if (print.people === 0) return 'Made by the tray, on its own'
  if (print.people === 1) return 'Made by one person'
  return `Made by ${print.people} people together`
}

export function when(seconds: number) {
  return new Date(seconds * 1000).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
}
