import { queryPoint, queryRect, type IndexedBatch } from "./spatial-index"
import { tickToX, xToTick, yToRow, type Viewport } from "./viewport"

/** Which part of a rect the pointer is over. The edges are resize handles. */
export type HitPart = "body" | "start-edge" | "end-edge"

export interface Hit {
  readonly index: number
  readonly id: number
  readonly part: HitPart
}

export interface HitOptions {
  /** Width of the resize handle at each end, in CSS pixels. */
  readonly edgePx?: number
  /**
   * How far outside its ends a rect too narrow to press still takes a
   * press, in CSS pixels. A rect is too narrow when it is drawn narrower
   * than twice this. Wider rects are hit only inside their bounds, and a
   * press inside any rect goes to that rect.
   */
  readonly slopPx?: number
}

const DEFAULT_EDGE_PX = 6
const DEFAULT_SLOP_PX = 2

/**
 * The narrow rect nearest to a point that lies in its row but beside it,
 * or -1. Zoomed out, a note can be under a pixel wide but is drawn as one
 * pixel. Without this it could be seen and not clicked.
 */
function nearestSliver(
  items: IndexedBatch,
  tick: number,
  row: number,
  slopTicks: number
): number {
  const batch = items.batch
  const near = queryRect(
    items,
    tick - slopTicks,
    tick + slopTicks,
    Math.floor(row),
    Math.floor(row) + 1
  )
  let best = -1
  let bestDistance = Infinity
  for (const index of near) {
    if (batch.length(index) >= 2 * slopTicks) continue
    const start = batch.start(index)
    const distance = tick < start ? start - tick : tick - batch.end(index)
    // At the same distance a selected rect wins, then the one drawn last,
    // as where rects overlap.
    const wins =
      distance < bestDistance ||
      (distance === bestDistance &&
        (batch.isSelected(index) || !batch.isSelected(best)))
    if (wins) {
      best = index
      bestDistance = distance
    }
  }
  return best
}

/** The rect under a point given in CSS pixels, or null. */
export function hitTestPoint(
  viewport: Viewport,
  items: IndexedBatch,
  x: number,
  y: number,
  options: HitOptions = {}
): Hit | null {
  const edgePx = options.edgePx ?? DEFAULT_EDGE_PX
  const slopPx = options.slopPx ?? DEFAULT_SLOP_PX
  const tick = xToTick(viewport, x)
  const row = yToRow(viewport, y)

  let index = queryPoint(items, tick, row)
  if (index < 0 && slopPx > 0) {
    index = nearestSliver(items, tick, row, slopPx / viewport.pxPerTick)
  }
  if (index < 0) return null

  const batch = items.batch
  const left = tickToX(viewport, batch.start(index))
  const right = tickToX(viewport, batch.end(index))
  // Short rects keep a body to grab: the handles never take more than a third each.
  const zone = Math.min(edgePx, (right - left) / 3)
  let part: HitPart = "body"
  if (x >= right - zone) part = "end-edge"
  else if (x < left + zone) part = "start-edge"
  return { index, id: batch.ids[index], part }
}

/**
 * Indices of every rect that overlaps a box given by two corners in CSS
 * pixels. The corners can be in any order, which suits a marquee drag.
 */
export function hitTestRect(
  viewport: Viewport,
  items: IndexedBatch,
  x0: number,
  y0: number,
  x1: number,
  y1: number
): number[] {
  return queryRect(
    items,
    xToTick(viewport, Math.min(x0, x1)),
    xToTick(viewport, Math.max(x0, x1)),
    yToRow(viewport, Math.min(y0, y1)),
    yToRow(viewport, Math.max(y0, y1))
  )
}
