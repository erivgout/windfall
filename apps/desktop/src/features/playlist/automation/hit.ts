import type { AutomationPoint } from "@/bindings"
import {
  FULL_VIEW,
  inView,
  viewFraction,
  viewValue,
  type ViewRange,
} from "@/lib/automation/view-range"

import { MIN_HEIGHT_FOR_BAND, type Box } from "../clip-box"
import { bendHandle, inWindow, toSongTick, type CurveWindow } from "./points"

/*
 * Which part of an automation clip's curve is under the pointer. Sizes are
 * CSS pixels.
 */

/** How close to a point or a bend handle a press takes it. */
export const POINT_RADIUS = 6
/**
 * The same outside the clip's own rect, where a dot on the clip's edge
 * hangs over it: the dot as it is drawn, and no further. Out there every
 * pixel more would be taken from the empty grid or from the next clip.
 */
export const DOT_REACH = 4
/** A stretch shorter than this on screen has no bend handle. */
export const MIN_BEND_WIDTH = 28
/** The curve can be edited in a clip at least this wide and this tall. */
export const MIN_EDIT_WIDTH = 12

/** How a clip's curve is laid out on screen. */
export type CurveView = {
  /** The clip as a window onto the curve. */
  readonly window: CurveWindow
  /** Where the curve is drawn: the clip under its title bar. */
  readonly area: Box
  /** CSS pixels per tick. */
  readonly pxPerTick: number
  /**
   * The part of the curve's range the area shows from its bottom to its
   * top. Left out, the whole range.
   */
  readonly range?: ViewRange
}

const rangeOf = (view: CurveView) => view.range ?? FULL_VIEW

export function canEditCurve(clip: Box): boolean {
  return (
    clip.bottom - clip.top >= MIN_HEIGHT_FOR_BAND &&
    clip.right - clip.left >= MIN_EDIT_WIDTH
  )
}

/** The x of a tick of the curve. */
export function curveX(view: CurveView, curveTick: number): number {
  return (
    view.area.left -
    1 +
    (toSongTick(view.window, curveTick) - view.window.start) * view.pxPerTick
  )
}

/** The tick of the curve under an x. */
export function curveTickAt(view: CurveView, x: number): number {
  return (x - (view.area.left - 1)) / view.pxPerTick + view.window.offset
}

/**
 * The y of a value: the top of the view's range is the top of the area and
 * its bottom the bottom. A value outside the view is put on the edge.
 */
export function valueY(view: CurveView, value: number): number {
  const { top, bottom } = view.area
  const part = viewFraction(rangeOf(view), Math.min(1, Math.max(0, value)))
  return bottom - (bottom - top) * Math.min(1, Math.max(0, part))
}

/** The value at a y, held to what the area shows. */
export function valueAt(view: CurveView, y: number): number {
  const { top, bottom } = view.area
  const height = bottom - top
  const range = rangeOf(view)
  if (height <= 0) return range.lo
  return viewValue(range, Math.min(1, Math.max(0, (bottom - y) / height)))
}

/**
 * The value at a y that may lie above or below the area: the view's scale
 * carried on past its edges, as far as 0 and 1. A point dragged out of the
 * view goes on at the same pace, and takes the view's edge with it.
 */
export function valueBeyond(view: CurveView, y: number): number {
  const { top, bottom } = view.area
  const height = bottom - top
  const range = rangeOf(view)
  if (height <= 0) return range.lo
  return viewValue(range, (bottom - y) / height)
}

export type CurveHit =
  /** A point of the curve, by its index. */
  | { kind: "point"; index: number }
  /** The handle that bends the stretch leaving the point at `index`. */
  | { kind: "bend"; index: number }
  /** The curve's area where there is no point: a press adds one. */
  | { kind: "area" }

/**
 * What of a curve is under a point of the screen. A point wins over a bend
 * handle, and the nearest point wins among several. Points outside the
 * clip's window belong to other clips of the curve and cannot be taken
 * from this one. `reach` is how close to a point or a handle counts.
 */
export function hitCurve(
  points: readonly AutomationPoint[],
  view: CurveView,
  x: number,
  y: number,
  reach = POINT_RADIUS
): CurveHit | null {
  const { area } = view
  if (
    x < area.left - reach ||
    x > area.right + reach ||
    y < area.top - reach ||
    y > area.bottom + reach
  ) {
    return null
  }

  const range = rangeOf(view)
  let best = -1
  let bestDistance = reach
  points.forEach((point, index) => {
    if (!inWindow(view.window, point.tick)) return
    // A point outside the view is not drawn, so it cannot be pressed.
    if (!inView(range, point.value)) return
    const distance = Math.hypot(
      curveX(view, point.tick) - x,
      valueY(view, point.value) - y
    )
    // Of two points on one spot the later one is on top, as it is drawn.
    if (distance <= bestDistance) {
      best = index
      bestDistance = distance
    }
  })
  if (best >= 0) return { kind: "point", index: best }

  for (let index = 0; index + 1 < points.length; index += 1) {
    const handle = bendHandle(points, index)
    if (!handle || !inWindow(view.window, handle.tick)) continue
    if (!inView(range, handle.value)) continue
    const width = (points[index + 1].tick - points[index].tick) * view.pxPerTick
    if (width < MIN_BEND_WIDTH) continue
    const distance = Math.hypot(
      curveX(view, handle.tick) - x,
      valueY(view, handle.value) - y
    )
    if (distance <= reach) return { kind: "bend", index }
  }

  const inside =
    x >= area.left && x <= area.right && y >= area.top && y <= area.bottom
  return inside ? { kind: "area" } : null
}
