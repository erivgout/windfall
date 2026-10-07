import { noteName } from "@/components/audio"
import {
  deviceX,
  deviceY,
  resizedSpan,
  rgbaToCss,
  rowToKey,
  visibleRange,
  visibleRows,
  visibleTicks,
  withAlpha,
  type GridTheme,
  type OverlayPainter,
} from "@/lib/canvas"
import { TICKS_PER_STEP } from "@/lib/units"

import type { PianoRollSession } from "./session"
import { ROW_COUNT } from "./view-math"

const MIN_LABEL_ROW_PX = 11
/** More names than this on screen cannot be read anyway. */
const MAX_LABELS = 1200
const MAX_OUTLINES = 2000
const LIGHT_TEXT = "rgba(255,255,255,0.92)"
const DARK_TEXT = "rgba(0,0,0,0.78)"

type Label = { text: string; x: number; y: number }

/**
 * Note names inside the notes that are wide and tall enough to hold one.
 * A selected note that is being dragged or resized carries its label along.
 */
export function noteLabelPainter(session: PianoRollSession): OverlayPainter {
  const widths = new Map<string, number>()
  let measuredFont = ""

  return (ctx, { viewport, transform, theme }) => {
    if (viewport.rowHeight < MIN_LABEL_ROW_PX) return
    const { editor } = session
    const items = editor.items
    const batch = items.batch
    if (batch.count === 0) return

    const preview = editor.drag
    const ticks = visibleTicks(viewport)
    const reach = preview
      ? Math.abs(preview.ticks) +
        Math.abs(preview.resize?.start ?? 0) +
        Math.abs(preview.resize?.end ?? 0)
      : 0
    const range = visibleRange(items, ticks.start - reach, ticks.end + reach)
    const rows = visibleRows(viewport, ROW_COUNT)
    const dpr = viewport.dpr
    const fontPx = Math.round(Math.min(11, viewport.rowHeight - 3) * dpr)
    const font = `600 ${fontPx}px "Inter Variable", system-ui, sans-serif`
    if (font !== measuredFont) {
      widths.clear()
      measuredFont = font
    }
    ctx.font = font
    ctx.textBaseline = "middle"
    const pad = Math.round(4 * dpr)
    const light: Label[] = []
    const dark: Label[] = []
    const mixTo = theme.selectionFill
    const mix = theme.selectionMix

    for (let i = range.first; i < range.last; i++) {
      if (light.length + dark.length >= MAX_LABELS) break
      const c = i * 4
      // A note being erased is faded out and loses its name.
      if (batch.colors[c + 3] < 128) continue
      let start = batch.start(i)
      let length = batch.length(i)
      let row = batch.row(i)
      const selected = batch.isSelected(i)
      if (selected && preview) {
        if (preview.resize) {
          const span = resizedSpan(
            start,
            length,
            preview.resize.start,
            preview.resize.end,
            preview.resize.minLength
          )
          start = span.start
          length = span.length
        }
        start += preview.ticks
        row -= preview.keys
      }
      if (row < rows.first || row >= rows.last) continue
      const text = noteName(rowToKey(row))
      let width = widths.get(text)
      if (width === undefined) {
        width = Math.ceil(ctx.measureText(text).width)
        widths.set(text, width)
      }
      // A note that starts left of the view keeps its name at the edge.
      const left = Math.max(0, deviceX(transform, start))
      const right = deviceX(transform, start + length)
      if (right - left < width + 2 * pad) continue

      let r = batch.colors[c]
      let g = batch.colors[c + 1]
      let b = batch.colors[c + 2]
      if (selected) {
        r += (mixTo.r - r) * mix
        g += (mixTo.g - g) * mix
        b += (mixTo.b - b) * mix
      }
      const luminance = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255
      const top = deviceY(transform, row) + transform.lineWidth
      const bottom = deviceY(transform, row + 1)
      const label = {
        text,
        x: left + pad,
        y: Math.round((top + bottom) / 2 + dpr * 0.5),
      }
      if (luminance > 0.56) dark.push(label)
      else light.push(label)
    }

    // One fill style per group: setting it is the slow part of Canvas 2D.
    ctx.fillStyle = DARK_TEXT
    for (const label of dark) ctx.fillText(label.text, label.x, label.y)
    ctx.fillStyle = LIGHT_TEXT
    for (const label of light) ctx.fillText(label.text, label.x, label.y)
  }
}

/**
 * While a move is about to duplicate, outlines mark where the originals
 * stay. The canvas shows the selected notes at the drop position only.
 */
export function duplicateOutlinePainter(
  session: PianoRollSession
): OverlayPainter {
  return (ctx, { viewport, transform, theme }) => {
    const { editor } = session
    if (!editor.drag?.duplicate) return
    const items = editor.items
    const batch = items.batch
    const ticks = visibleTicks(viewport)
    const range = visibleRange(items, ticks.start, ticks.end)
    const lw = transform.lineWidth
    ctx.fillStyle = rgbaToCss(withAlpha(theme.foreground, 0.55))
    let drawn = 0
    for (let i = range.first; i < range.last && drawn < MAX_OUTLINES; i++) {
      if (!batch.isSelected(i)) continue
      const x0 = deviceX(transform, batch.start(i))
      const x1 = Math.max(x0 + 2 * lw, deviceX(transform, batch.end(i)))
      const y0 = deviceY(transform, batch.row(i)) + lw
      const y1 = deviceY(transform, batch.row(i) + 1)
      ctx.fillRect(x0, y0, x1 - x0, lw)
      ctx.fillRect(x0, y1 - lw, x1 - x0, lw)
      ctx.fillRect(x0, y0, lw, y1 - y0)
      ctx.fillRect(x1 - lw, y0, lw, y1 - y0)
      drawn++
    }
  }
}

/** The pattern length in ticks as the grid should show it right now. */
export function shownLengthTicks(session: PianoRollSession): number {
  if (session.lengthPreview !== null) return session.lengthPreview
  const steps = session.editor.context?.pattern.lengthSteps ?? 0
  return steps * TICKS_PER_STEP
}

/**
 * Fades a region that lies past the end of the pattern: a wash of the
 * background takes the color out of the notes, and a little black sets the
 * region apart, which the wash alone does not do on a light background.
 */
export function fadePastEnd(
  ctx: CanvasRenderingContext2D,
  theme: GridTheme,
  x: number,
  width: number,
  height: number
): void {
  if (width <= 0) return
  const { r, g, b } = theme.background
  const light = 0.2126 * r + 0.7152 * g + 0.0722 * b > 128
  ctx.fillStyle = rgbaToCss(withAlpha(theme.background, 0.55))
  ctx.fillRect(x, 0, width, height)
  ctx.fillStyle = light ? "rgba(0,0,0,0.07)" : "rgba(0,0,0,0.22)"
  ctx.fillRect(x, 0, width, height)
}

/**
 * Dims everything past the end of the pattern, notes included: nothing
 * there plays. A line marks the end itself.
 */
export function patternEndPainter(session: PianoRollSession): OverlayPainter {
  return (ctx, { transform, theme }) => {
    const end = shownLengthTicks(session)
    if (end <= 0) return
    const x = deviceX(transform, end)
    if (x >= transform.widthDev) return
    const from = Math.max(0, x)
    fadePastEnd(
      ctx,
      theme,
      from,
      transform.widthDev - from,
      transform.heightDev
    )
    if (x >= 0) {
      ctx.fillStyle = rgbaToCss(withAlpha(theme.item, 0.85))
      ctx.fillRect(x, 0, transform.lineWidth, transform.heightDev)
    }
  }
}
