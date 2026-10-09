import type { AutomationPoint } from "@/bindings"
import { curveShape, curveValue, pointBefore } from "@/lib/automation/curve"
import { MAX_AUTOMATION_POINTS, MAX_SONG_TICKS } from "@/lib/units"

/*
 * The arithmetic of editing an automation curve, with no pointer or canvas
 * in it. Every function returns a new list of points, in order, that the
 * `setAutomationPoints` command accepts as it is.
 */

/**
 * A clip as a window onto a curve: tick `start` of the song is tick
 * `offset` of the curve, and the clip shows `length` ticks of it.
 */
export type CurveWindow = {
  readonly start: number
  readonly length: number
  readonly offset: number
}

/** The tick of the curve that plays at a tick of the song. */
export function toCurveTick(window: CurveWindow, songTick: number): number {
  return songTick - window.start + window.offset
}

/** The tick of the song at which a tick of the curve plays in this clip. */
export function toSongTick(window: CurveWindow, curveTick: number): number {
  return curveTick - window.offset + window.start
}

/** Whether a tick of the curve shows in the clip, its two ends included. */
export function inWindow(window: CurveWindow, curveTick: number): boolean {
  return (
    curveTick >= window.offset && curveTick <= window.offset + window.length
  )
}

const clamp01 = (value: number) =>
  Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 0

const wholeTick = (tick: number) =>
  Math.min(MAX_SONG_TICKS, Math.max(0, Math.round(tick)))

/**
 * Adds a point at a tick of the curve. It goes after every point at or
 * before that tick, so a point added on another one makes a jump from the
 * old value to the new. Null when the curve is full.
 */
export function insertPoint(
  points: readonly AutomationPoint[],
  tick: number,
  value: number
): { points: AutomationPoint[]; index: number } | null {
  if (points.length >= MAX_AUTOMATION_POINTS) return null
  const at = wholeTick(tick)
  let index = 0
  while (index < points.length && points[index].tick <= at) index += 1
  const next = [...points]
  next.splice(index, 0, {
    tick: at,
    value: clamp01(value),
    curve: 0,
    hold: false,
  })
  return { points: next, index }
}

/**
 * Where a point may go in time: between its two neighbours, inside the
 * clip's window when one is given, and on the timeline.
 */
export function tickLimits(
  points: readonly AutomationPoint[],
  index: number,
  window?: CurveWindow
): { min: number; max: number } {
  let min = points[index - 1]?.tick ?? 0
  let max = points[index + 1]?.tick ?? MAX_SONG_TICKS
  if (window) {
    min = Math.max(min, window.offset)
    max = Math.min(max, window.offset + window.length)
  }
  // A window that the neighbours leave no room in keeps the point put.
  return max < min ? { min, max: min } : { min, max }
}

/**
 * Moves a point to a tick of the curve and a value. The tick is held
 * between the point's neighbours, so points never change order.
 */
export function movePoint(
  points: readonly AutomationPoint[],
  index: number,
  tick: number,
  value: number,
  window?: CurveWindow
): AutomationPoint[] {
  const point = points[index]
  if (!point) return [...points]
  const { min, max } = tickLimits(points, index, window)
  const next = [...points]
  next[index] = {
    ...point,
    tick: Math.min(max, Math.max(min, wholeTick(tick))),
    value: clamp01(value),
  }
  return next
}

/** Moves a selection by one delta, leaving room between unselected neighbours. */
export function moveSelectedPoints(
  points: readonly AutomationPoint[],
  indices: ReadonlySet<number>,
  tickDelta: number,
  valueDelta: number,
  window: CurveWindow
): AutomationPoint[] {
  let minTick = -Infinity
  let maxTick = Infinity
  let minValue = -Infinity
  let maxValue = Infinity
  for (const index of indices) {
    const point = points[index]
    if (!point) continue
    minTick = Math.max(minTick, Math.max(0, window.offset) - point.tick)
    maxTick = Math.min(
      maxTick,
      Math.min(MAX_SONG_TICKS, window.offset + window.length) - point.tick
    )
    const before = points[index - 1]
    const after = points[index + 1]
    // A pre-existing jump can stay put; moving it must not reverse its order.
    if (before && !indices.has(index - 1)) {
      minTick = Math.max(minTick, Math.min(0, before.tick + 1 - point.tick))
    }
    if (after && !indices.has(index + 1)) {
      maxTick = Math.min(maxTick, Math.max(0, after.tick - 1 - point.tick))
    }
    minValue = Math.max(minValue, -point.value)
    maxValue = Math.min(maxValue, 1 - point.value)
  }
  // Existing jumps on a shared tick may leave no room to move in time.
  const ticks =
    minTick > maxTick
      ? 0
      : Math.min(maxTick, Math.max(minTick, Math.round(tickDelta)))
  const value = Math.min(maxValue, Math.max(minValue, valueDelta))
  return points.map((point, index) =>
    indices.has(index)
      ? { ...point, tick: point.tick + ticks, value: point.value + value }
      : point
  )
}

/** Sets the bend of the stretch that leaves a point, -1 to 1. */
export function bendSegment(
  points: readonly AutomationPoint[],
  index: number,
  curve: number
): AutomationPoint[] {
  const point = points[index]
  if (!point) return [...points]
  const next = [...points]
  next[index] = {
    ...point,
    curve: Number.isFinite(curve) ? Math.min(1, Math.max(-1, curve)) : 0,
  }
  return next
}

/**
 * The bend that puts the middle of a stretch on `value`. Half way through
 * a bent stretch the value has covered `1 / (e^(k/2) + 1)` of its way, with
 * `k = 6 * curve`, and this solves that for the curve. Null when the two
 * ends are level, where no bend shows.
 */
export function bendThrough(
  from: number,
  to: number,
  value: number
): number | null {
  const span = to - from
  if (Math.abs(span) < 1e-6) return null
  // The share at the two ends of the range a bend can reach.
  const least = curveShape(0.5, 1)
  const share = Math.min(1 - least, Math.max(least, (value - from) / span))
  return Math.min(1, Math.max(-1, (2 * Math.log(1 / share - 1)) / 6))
}

/** Makes a point a step, or a slope again. */
export function toggleHold(
  points: readonly AutomationPoint[],
  index: number
): AutomationPoint[] {
  const point = points[index]
  if (!point) return [...points]
  const next = [...points]
  next[index] = { ...point, hold: !point.hold }
  return next
}

/** Takes the bend and the step out of the stretch that leaves a point. */
export function straighten(
  points: readonly AutomationPoint[],
  index: number
): AutomationPoint[] {
  const point = points[index]
  if (!point) return [...points]
  const next = [...points]
  next[index] = { ...point, curve: 0, hold: false }
  return next
}

/** Removes a point. Null for the last one: a curve has at least one. */
export function deletePoint(
  points: readonly AutomationPoint[],
  index: number
): AutomationPoint[] | null {
  if (points.length <= 1 || !points[index]) return null
  return points.filter((_, at) => at !== index)
}

export function setPointValue(
  points: readonly AutomationPoint[],
  index: number,
  value: number
): AutomationPoint[] {
  const point = points[index]
  if (!point) return [...points]
  const next = [...points]
  next[index] = { ...point, value: clamp01(value) }
  return next
}

/**
 * Holds a drag to one axis: the one the pointer has moved further along.
 * `dx` and `dy` are how far it has moved since the press, in pixels.
 */
export function constrainToAxis<T extends { tick: number; value: number }>(
  from: T,
  to: T,
  dx: number,
  dy: number
): T {
  return Math.abs(dx) >= Math.abs(dy)
    ? { ...to, value: from.value }
    : { ...to, tick: from.tick }
}

/** Whether a stretch between two points has a handle to bend it by. */
export function isBendable(
  from: AutomationPoint,
  to: AutomationPoint | undefined
): to is AutomationPoint {
  return to !== undefined && !from.hold && to.tick > from.tick
}

/** The tick and value of the handle in the middle of a stretch. */
export function bendHandle(
  points: readonly AutomationPoint[],
  index: number
): { tick: number; value: number } | null {
  const from = points[index]
  const to = points[index + 1]
  if (!from || !isBendable(from, to)) return null
  const tick = (from.tick + to.tick) / 2
  return { tick, value: curveValue(points, tick) }
}

/** Whether two curves are the same, point for point. */
export function samePoints(
  a: readonly AutomationPoint[],
  b: readonly AutomationPoint[]
): boolean {
  return (
    a.length === b.length &&
    a.every((point, index) => {
      const other = b[index]
      return (
        point.tick === other.tick &&
        point.value === other.value &&
        point.curve === other.curve &&
        point.hold === other.hold
      )
    })
  )
}

/**
 * The corners of a curve as it shows through a clip's window, as ticks of
 * the curve and values: the value at the window's two ends, every point in
 * between, a second corner where the curve steps or jumps, and extra ones
 * along a bent stretch, `grain` ticks apart, so it draws as a curve.
 */
export function windowPath(
  points: readonly AutomationPoint[],
  window: CurveWindow,
  grain: number
): { tick: number; value: number }[] {
  const from = window.offset
  const to = window.offset + window.length
  const path = [{ tick: from, value: curveValue(points, from) }]
  const step = Math.max(1, grain)
  // The stretch the window opens in may be a bent one.
  const last = pointBefore(points, from)
  const bend = (index: number, begin: number, end: number) => {
    const point = points[index]
    const next = points[index + 1]
    if (!point || !next || point.hold || point.curve === 0) return
    for (let tick = begin + step; tick < end; tick += step) {
      path.push({ tick, value: curveValue(points, tick) })
    }
  }
  let cursor = from
  for (let index = Math.max(0, last); index < points.length; index += 1) {
    const point = points[index]
    if (point.tick > to) break
    if (point.tick > from) {
      bend(index - 1, cursor, point.tick)
      const previous = points[index - 1]
      // What the curve arrives with, which a step or a jump leaves behind.
      const arriving = !previous
        ? point.value
        : previous.hold
          ? previous.value
          : previous.tick === point.tick
            ? path[path.length - 1].value
            : point.value
      if (arriving !== point.value) {
        path.push({ tick: point.tick, value: arriving })
      }
      path.push({ tick: point.tick, value: point.value })
      cursor = point.tick
    }
  }
  const tail = pointBefore(points, to)
  if (tail >= 0 && cursor < to) bend(tail, cursor, to)
  path.push({ tick: to, value: curveValue(points, to) })
  return path
}
