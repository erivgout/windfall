import {
  deviceX,
  deviceY,
  rgbaToCss,
  rowToKey,
  visibleRows,
  visibleTicks,
  withAlpha,
  type OverlayPainter,
} from "@/lib/canvas"

import { TICKS_PER_BAR } from "./generate-notes"
import { ROW_COUNT } from "./model"

const MIN_BAR_LABEL_SPACING_PX = 56
const MIN_ROW_HEIGHT_FOR_LABELS_PX = 10

/**
 * Bar numbers along the top and an octave name on each C row. The real
 * piano roll will have a ruler and a keyboard beside the grid; these
 * labels only make the benchmark page readable.
 */
export const paintLabels: OverlayPainter = (ctx, frame) => {
  const { viewport, transform, theme } = frame
  const dpr = viewport.dpr
  const fontPx = Math.round(11 * dpr)
  const pad = Math.round(3 * dpr)
  ctx.font = `500 ${fontPx}px "Inter Variable", system-ui, sans-serif`
  ctx.textBaseline = "top"
  const chip = rgbaToCss(withAlpha(theme.background, 0.78))
  const ink = rgbaToCss(theme.mutedForeground)

  const label = (text: string, x: number, y: number): void => {
    const width = Math.ceil(ctx.measureText(text).width)
    ctx.fillStyle = chip
    ctx.fillRect(x, y, width + 2 * pad, fontPx + 2 * pad)
    ctx.fillStyle = ink
    ctx.fillText(text, x + pad, y + pad)
  }

  let barsPerLabel = 1
  while (
    barsPerLabel * TICKS_PER_BAR * viewport.pxPerTick <
    MIN_BAR_LABEL_SPACING_PX
  ) {
    barsPerLabel *= 2
  }
  const spacing = barsPerLabel * TICKS_PER_BAR
  const ticks = visibleTicks(viewport)
  const firstBarTick = Math.max(0, Math.floor(ticks.start / spacing) * spacing)
  for (let tick = firstBarTick; tick <= ticks.end; tick += spacing) {
    label(
      String(tick / TICKS_PER_BAR + 1),
      deviceX(transform, tick) + transform.lineWidth,
      0
    )
  }

  if (viewport.rowHeight < MIN_ROW_HEIGHT_FOR_LABELS_PX) return
  const rows = visibleRows(viewport, ROW_COUNT)
  for (let row = rows.first; row < rows.last; row++) {
    const key = rowToKey(row, ROW_COUNT)
    if (key % 12 !== 0) continue
    const top = deviceY(transform, row) + transform.lineWidth
    const bottom = deviceY(transform, row + 1)
    // MIDI key 60 is C4 here.
    label(`C${key / 12 - 1}`, 0, Math.round((top + bottom - fontPx) / 2) - pad)
  }
}
