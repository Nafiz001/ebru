import { hex } from './pigments'
import type { Frame } from './protocol'
import { WIDTH } from './protocol'

/**
 * Paints a frame of the tray into a canvas of any size. Inks are filled in
 * the order they were dropped, which is exactly the order they lie on the
 * bath, and each gets a faint darker rim, like the edge ox gall leaves
 * where one ink meets another.
 */
export function drawFrame(ctx: CanvasRenderingContext2D, frame: Frame, width: number, height: number) {
  const scale = width / WIDTH
  ctx.setTransform(1, 0, 0, 1, 0, 0)
  ctx.fillStyle = hex(frame.ground)
  ctx.fillRect(0, 0, width, height)
  ctx.setTransform(scale, 0, 0, scale, 0, 0)
  ctx.lineJoin = 'round'
  ctx.lineWidth = 0.9 / scale
  ctx.strokeStyle = 'rgb(20 12 8 / 0.16)'

  const { points, offsets, pigments } = frame
  for (let i = 0; i < pigments.length; i++) {
    const start = offsets[i]
    const end = offsets[i + 1]
    if (end - start < 3) continue
    ctx.beginPath()
    ctx.moveTo(points[start * 2], points[start * 2 + 1])
    for (let k = start + 1; k < end; k++) ctx.lineTo(points[k * 2], points[k * 2 + 1])
    ctx.closePath()
    ctx.fillStyle = hex(pigments[i])
    ctx.fill()
    ctx.stroke()
  }
  ctx.setTransform(1, 0, 0, 1, 0, 0)
}
