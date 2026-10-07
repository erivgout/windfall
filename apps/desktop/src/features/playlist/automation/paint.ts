import type { AutomationPoint, PlaylistTrackId } from "@/bindings"
import type { HoldSegment } from "@/lib/automation/lanes"
import {
  FULL_VIEW,
  inView,
  isFullView,
  viewFraction,
  type ViewRange,
} from "@/lib/automation/view-range"
import {
  deviceX,
  deviceY,
  rgbaToCss,
  rgbFromInt,
  withAlpha,
  type OverlayFrame,
} from "@/lib/canvas"

import {
  BAND_HEIGHT,
  CONTENT_PAD,
  MIN_HEIGHT_FOR_BAND,
  type Box,
} from "../clip-box"
import { tickAtDeviceX, type ClipSprite, type ClipStyle } from "../sprite"
import { MIN_BEND_WIDTH } from "./hit"
import {
  bendHandle,
  toCurveTick,
  toSongTick,
  windowPath,
  type CurveWindow,
} from "./points"

/** Radius of a point of the curve, in CSS pixels. */
const POINT_SIZE = 2.75
/** The labels of a view's two ends need a clip this wide and this tall. */
const MIN_LABEL_WIDTH = 56
const MIN_LABEL_HEIGHT = 30
const LABEL_FONT_SIZE = 9
/** Points closer together than this on average are not drawn one by one. */
const MIN_POINT_SPACING = 7
const MIN_CURVE_HEIGHT = 5

export type AutomationPaint = {
  points: readonly AutomationPoint[]
  /** The clip under its title bar, less a little air, in device pixels. */
  area: Box
  /** The part of the curve's range the area shows. Left out, all of it. */
  range?: ViewRange
  /** That view's two ends in the unit of what the curve moves. */
  labels?: { top: string; bottom: string } | null
  /** Show the handles that bend the stretches: the clip is pointed at. */
  active: boolean
  /** Points and handles that may still be drawn this repaint. */
  budget: { marks: number }
}

/**
 * Draws an automation clip's curve across its body: the line, a wash under
 * it, the points, and on the clip being edited the handles that bend it.
 * The height of the clip is the clip's view of the curve's range, and the
 * two ends of that view are written at its top and bottom in real units.
 */
export function paintAutomation(
  ctx: CanvasRenderingContext2D,
  frame: OverlayFrame,
  sprite: ClipSprite,
  style: ClipStyle,
  paint: AutomationPaint
): void {
  const { transform, viewport } = frame
  const { area } = paint
  const dpr = viewport.dpr
  const range = paint.range ?? FULL_VIEW
  if (isFullView(range)) {
    paintCurve(ctx, frame, sprite, style, paint, range)
  } else {
    // The curve goes on above and below what a view shows. It is cut off
    // at the view's edges, with room for a dot that sits on one. Sideways
    // nothing is cut: a dot on the clip's edge hangs over it.
    const pad = Math.ceil((POINT_SIZE + 1.5) * dpr)
    ctx.save()
    ctx.beginPath()
    ctx.rect(
      0,
      area.top - pad,
      transform.widthDev,
      area.bottom - area.top + 2 * pad
    )
    ctx.clip()
    paintCurve(ctx, frame, sprite, style, paint, range)
    ctx.restore()
  }
  paintViewLabels(ctx, frame, sprite, style, paint)
}

/** Writes what the top and the bottom of the clip stand for. */
function paintViewLabels(
  ctx: CanvasRenderingContext2D,
  frame: OverlayFrame,
  sprite: ClipSprite,
  style: ClipStyle,
  paint: AutomationPaint
): void {
  const { labels, area } = paint
  if (!labels || sprite.ghost) return
  const { transform, viewport } = frame
  const dpr = viewport.dpr
  // Kept in sight while the clip's start is scrolled out of view.
  const left = Math.max(area.left, 0)
  const right = Math.min(area.right, transform.widthDev)
  if (right - left < MIN_LABEL_WIDTH * dpr) return
  if (area.bottom - area.top < MIN_LABEL_HEIGHT * dpr) return
  ctx.save()
  ctx.font = `${Math.round(LABEL_FONT_SIZE * dpr)}px ${LABEL_FONT}`
  ctx.fillStyle = style.bodyInk
  ctx.globalAlpha = sprite.muted ? 0.4 : 0.62
  ctx.textAlign = "left"
  const x = left + Math.round(3 * dpr)
  ctx.textBaseline = "top"
  ctx.fillText(labels.top, x, area.top - Math.round(1 * dpr))
  ctx.textBaseline = "bottom"
  ctx.fillText(labels.bottom, x, area.bottom + Math.round(2 * dpr))
  ctx.restore()
}

const LABEL_FONT = '"Inter Variable", system-ui, sans-serif'

function paintCurve(
  ctx: CanvasRenderingContext2D,
  frame: OverlayFrame,
  sprite: ClipSprite,
  style: ClipStyle,
  paint: AutomationPaint,
  range: ViewRange
): void {
  const { transform, viewport, theme } = frame
  const { area, points } = paint
  const dpr = viewport.dpr
  const height = area.bottom - area.top
  if (height < MIN_CURVE_HEIGHT * dpr || points.length === 0) return
  const window: CurveWindow = sprite.span

  // Only the part of the window that is on screen.
  const fromTick = Math.max(
    window.offset,
    toCurveTick(window, tickAtDeviceX(transform, 0))
  )
  const toTick = Math.min(
    window.offset + window.length,
    toCurveTick(window, tickAtDeviceX(transform, transform.widthDev))
  )
  if (toTick <= fromTick) return
  const shown: CurveWindow = {
    start: toSongTick(window, fromTick),
    offset: fromTick,
    length: toTick - fromTick,
  }
  const xOf = (curveTick: number) =>
    deviceX(transform, toSongTick(window, curveTick))
  // Not held to the area: what lies outside the view is cut off, so the
  // line leaves it at the right angle.
  const yOf = (value: number) =>
    area.bottom - height * viewFraction(range, value)

  const path = windowPath(points, shown, (3 * dpr) / transform.scaleX)
  const left = xOf(fromTick)
  const right = xOf(toTick)

  ctx.beginPath()
  ctx.moveTo(left, area.bottom)
  for (const corner of path) ctx.lineTo(xOf(corner.tick), yOf(corner.value))
  ctx.lineTo(right, area.bottom)
  ctx.closePath()
  ctx.globalAlpha = sprite.muted ? 0.1 : 0.2
  ctx.fillStyle = style.note
  ctx.fill()
  ctx.globalAlpha = 1

  ctx.beginPath()
  path.forEach((corner, index) => {
    const x = xOf(corner.tick)
    const y = yOf(corner.value)
    if (index === 0) ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  })
  ctx.strokeStyle = style.note
  ctx.lineWidth = Math.max(1, Math.round(1.5 * dpr))
  ctx.lineJoin = "round"
  ctx.stroke()

  if (sprite.ghost) return
  // The points of this clip's window that are in view.
  let first = 0
  while (first < points.length && points[first].tick < fromTick) first += 1
  let last = first
  while (last < points.length && points[last].tick <= toTick) last += 1
  const count = last - first
  if (count === 0 || paint.budget.marks <= 0) return
  if ((right - left) / count < MIN_POINT_SPACING * dpr) return
  paint.budget.marks -= count

  const radius = POINT_SIZE * dpr
  const fill = rgbaToCss(theme.background)
  ctx.lineWidth = Math.max(1, Math.round(1.25 * dpr))
  ctx.strokeStyle = style.note
  ctx.fillStyle = fill
  // A dot on tick 0 would be cut in half by the left end of the timeline,
  // which cannot scroll further, and that is where a new curve's first
  // point is. It is drawn whole, just inside.
  const atStart = tickAtDeviceX(transform, 0) <= 0
  const minX = atStart ? radius + ctx.lineWidth / 2 : -Infinity
  for (let index = first; index < last; index += 1) {
    const point = points[index]
    if (!inView(range, point.value)) continue
    const x = Math.max(minX, xOf(point.tick))
    const y = yOf(point.value)
    ctx.beginPath()
    // A point that holds its value is a square, one that slopes a circle.
    if (point.hold) ctx.rect(x - radius, y - radius, 2 * radius, 2 * radius)
    else ctx.arc(x, y, radius, 0, Math.PI * 2)
    ctx.fill()
    ctx.stroke()
  }

  if (!paint.active) return
  const small = radius * 0.8
  for (let index = Math.max(0, first - 1); index < last; index += 1) {
    const handle = bendHandle(points, index)
    if (!handle || handle.tick < fromTick || handle.tick > toTick) continue
    if (!inView(range, handle.value)) continue
    const width =
      (points[index + 1].tick - points[index].tick) * viewport.pxPerTick
    if (width < MIN_BEND_WIDTH) continue
    const x = xOf(handle.tick)
    const y = yOf(handle.value)
    ctx.beginPath()
    ctx.moveTo(x, y - small * 1.3)
    ctx.lineTo(x + small * 1.3, y)
    ctx.lineTo(x, y + small * 1.3)
    ctx.lineTo(x - small * 1.3, y)
    ctx.closePath()
    ctx.fillStyle = style.note
    ctx.fill()
  }
}

/** Two rings that say a clip shows a curve other clips show too. */
export function paintLinkGlyph(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  dpr: number,
  ink: string
): void {
  const radius = 2.4 * dpr
  ctx.strokeStyle = ink
  ctx.lineWidth = Math.max(1, Math.round(1.1 * dpr))
  ctx.beginPath()
  ctx.arc(x - radius * 0.7, y, radius, 0, Math.PI * 2)
  ctx.stroke()
  ctx.beginPath()
  ctx.arc(x + radius * 0.7, y, radius, 0, Math.PI * 2)
  ctx.stroke()
}

/**
 * Draws where targets stay on the value a clip left: a faint dashed line
 * from the clip's end to the next clip of the same target, on the clip's
 * own row and at the height the clip draws that value at.
 */
export function paintHolds(
  ctx: CanvasRenderingContext2D,
  frame: OverlayFrame,
  holds: readonly HoldSegment[],
  rowOf: (track: PlaylistTrackId) => number | undefined,
  colorOf: (hold: HoldSegment) => number,
  viewOf: (hold: HoldSegment) => ViewRange = () => FULL_VIEW
): void {
  const { transform, viewport } = frame
  const dpr = viewport.dpr
  const lw = transform.lineWidth
  const banded = viewport.rowHeight >= MIN_HEIGHT_FOR_BAND
  const dash = Math.round(5 * dpr)
  const gap = Math.round(4 * dpr)
  for (const hold of holds) {
    const row = rowOf(hold.track)
    if (row === undefined) continue
    const y0 = deviceY(transform, row) + lw
    const y1 = deviceY(transform, row + 1)
    if (y1 <= 0 || y0 >= transform.heightDev) continue
    const from = Math.max(0, deviceX(transform, hold.start))
    const to = Math.min(transform.widthDev, deviceX(transform, hold.end))
    if (to <= from) continue
    // The same area a clip on this row draws its curve in.
    const top =
      y0 + lw + Math.round((banded ? BAND_HEIGHT : 0) * dpr + CONTENT_PAD * dpr)
    const bottom = y1 - lw - Math.round(CONTENT_PAD * dpr)
    if (bottom <= top) continue
    // A value the clip's view leaves out has no place on the row.
    const view = viewOf(hold)
    if (!inView(view, hold.value)) continue
    const y = Math.round(
      bottom - (bottom - top) * viewFraction(view, hold.value)
    )
    ctx.fillStyle = rgbaToCss(withAlpha(rgbFromInt(colorOf(hold)), 0.62))
    // Dashes are placed from the timeline, so they stay put while scrolling.
    const period = dash + gap
    const phase = ((from - deviceX(transform, hold.start)) % period) + period
    for (let x = from - (phase % period); x < to; x += period) {
      const left = Math.max(from, x)
      const right = Math.min(to, x + dash)
      if (right > left) ctx.fillRect(left, y, right - left, lw)
    }
  }
}
