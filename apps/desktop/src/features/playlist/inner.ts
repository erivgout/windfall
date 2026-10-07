import type { AutomationPoint, Clip } from "@/bindings"
import type { ViewRange } from "@/lib/automation/view-range"
import type { Viewport } from "@/lib/canvas"

import { hitAudioHandle } from "./audio/handles"
import {
  canEditCurve,
  DOT_REACH,
  hitCurve,
  POINT_RADIUS,
  type CurveView,
} from "./automation/hit"
import { clipBox, contentArea } from "./clip-box"

/** A part of a clip that is edited by itself, apart from moving the clip. */
export type InnerHit =
  /** The handle of an audio clip's fade in or fade out. */
  | { kind: "fade"; edge: "in" | "out" }
  /** The gain handle of an audio clip. */
  | { kind: "gain" }
  /** A point of an automation clip's curve. */
  | { kind: "point"; index: number }
  /** The handle that bends the stretch leaving the point at `index`. */
  | { kind: "bend"; index: number }
  /** The curve's area where there is no point. */
  | { kind: "curve" }

/**
 * How an automation clip's curve is laid out on screen, or null when the
 * clip is too small to edit it in. `range` is the part of the curve's range
 * the clip shows; left out, all of it.
 */
export function curveViewOf(
  viewport: Viewport,
  clip: Pick<Clip, "start" | "length" | "offset">,
  row: number,
  range?: ViewRange
): CurveView | null {
  const box = clipBox(viewport, clip, row)
  if (!canEditCurve(box)) return null
  return {
    window: { start: clip.start, length: clip.length, offset: clip.offset },
    area: contentArea(box),
    pxPerTick: viewport.pxPerTick,
    range,
  }
}

/**
 * The part of a clip under a point of the grid, in CSS pixels, that has an
 * edit of its own: an audio clip's handles, an automation clip's curve.
 * Null for the rest of the clip, which moves and resizes it. `points` is
 * the curve of an automation clip.
 *
 * The spot may lie just outside the clip: a point on the clip's edge is
 * drawn half over it, and is taken there as far as its dot reaches.
 * `range` is the part of a curve's range the clip shows.
 */
export function innerHit(
  viewport: Viewport,
  clip: Clip,
  row: number,
  points: readonly AutomationPoint[] | null,
  x: number,
  y: number,
  range?: ViewRange
): InnerHit | null {
  const content = clip.content
  if (content.type === "audio") {
    const handle = hitAudioHandle(
      clipBox(viewport, clip, row),
      content.fadeIn * viewport.pxPerTick,
      content.fadeOut * viewport.pxPerTick,
      x,
      y
    )
    if (handle === "gain") return { kind: "gain" }
    if (handle === "fade-in") return { kind: "fade", edge: "in" }
    if (handle === "fade-out") return { kind: "fade", edge: "out" }
    return null
  }
  if (content.type === "automation" && points) {
    const view = curveViewOf(viewport, clip, row, range)
    if (!view) return null
    const box = clipBox(viewport, clip, row)
    const inside =
      x >= box.left && x <= box.right && y >= box.top && y <= box.bottom
    const hit = hitCurve(points, view, x, y, inside ? POINT_RADIUS : DOT_REACH)
    if (!hit) return null
    return hit.kind === "area" ? { kind: "curve" } : hit
  }
  return null
}
