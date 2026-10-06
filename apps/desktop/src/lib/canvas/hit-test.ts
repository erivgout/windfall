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
  /** How far outside a rect narrower than this a click still counts. */
  readonly slopPx?: number
}

const DEFAULT_EDGE_PX = 6
const DEFAULT_SLOP_PX = 2

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
    // Zoomed out, a note can be under a pixel wide but is drawn as one
    // pixel. Without slop it could be seen and not clicked.
    const slop = slopPx / viewport.pxPerTick
    const near = queryRect(
      items,
      tick - slop,
      tick + slop,
      Math.floor(row),
      Math.floor(row) + 1
    )
    index = near.length > 0 ? near[near.length - 1] : -1
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
