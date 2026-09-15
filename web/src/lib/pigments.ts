import type { Pigment } from './protocol'

// Mirrors crates/marbling/src/pigment.rs, so the tray can be drawn before the
// server answers. The server's /api/pigments is the source of truth and a test
// keeps the two in step.
export const PIGMENTS: Pigment[] = [
  { index: 0, name: 'Lapis', hex: '#2b4f96', ground: false },
  { index: 1, name: 'Indigo', hex: '#1e2a55', ground: true },
  { index: 2, name: 'Madder', hex: '#b43b2d', ground: false },
  { index: 3, name: 'Oxblood', hex: '#6a1d24', ground: true },
  { index: 4, name: 'Ochre', hex: '#cc922f', ground: true },
  { index: 5, name: 'Verdigris', hex: '#3d8c78', ground: true },
  { index: 6, name: 'Rose', hex: '#d97b82', ground: false },
  { index: 7, name: 'Umber', hex: '#6b4a31', ground: true },
  { index: 8, name: 'Oak gall', hex: '#1c1a1d', ground: false },
  { index: 9, name: 'Chalk', hex: '#f2efe6', ground: true },
]

export const hex = (index: number) => PIGMENTS[index]?.hex ?? '#000000'
