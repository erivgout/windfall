import type { AutomationPoint, AutomationRange, ParamInfo } from "@/bindings"
import { MAX_GAIN, MAX_TEMPO_BPM, MIN_TEMPO_BPM } from "@/lib/units"

/*
 * What an automation curve is worth at a tick, and what that value means
 * for the thing it moves. This mirrors `windfall_project::automation`, the
 * functions the engine plays by, and is tested against numbers generated
 * from them (`bindings/automation-fixtures.json`). Everything here is a
 * pure function of its arguments.
 */

/** How strongly a `curve` of 1 bends a segment. */
const CURVE_STRENGTH = 6

/**
 * How far along a segment's change of value is, `part` (0 to 1) of the way
 * through it in time, for a segment bent by `curve` (-1 to 1). 0 is a
 * straight line, a positive curve holds back and catches up, a negative
 * one hurries and settles.
 */
export function curveShape(part: number, curve: number): number {
  const at = Math.min(1, Math.max(0, part))
  const bend = Number.isFinite(curve)
    ? Math.fround(Math.min(1, Math.max(-1, curve))) * CURVE_STRENGTH
    : 0
  if (Math.abs(bend) < 1e-6) return at
  return Math.expm1(bend * at) / Math.expm1(bend)
}

/**
 * Index of the point the curve leaves from at `tick`: the last one at or
 * before it, or -1 before the first point. Of several points on one tick
 * it is the last, which is what makes a jump.
 */
export function pointBefore(
  points: readonly AutomationPoint[],
  tick: number
): number {
  let low = 0
  let high = points.length
  while (low < high) {
    const middle = (low + high) >> 1
    if (points[middle].tick <= tick) low = middle + 1
    else high = middle
  }
  return low - 1
}

/**
 * The value of a curve, 0 to 1, at `tick` ticks from its start. Before the
 * first point it has that point's value and after the last that one's. A
 * curve with no points is 0 everywhere.
 */
export function curveValue(
  points: readonly AutomationPoint[],
  tick: number
): number {
  const index = pointBefore(points, tick)
  if (index < 0) return points[0]?.value ?? 0
  const from = points[index]
  const to = points[index + 1]
  if (!to || from.hold) return from.value
  const part = (tick - from.tick) / (to.tick - from.tick)
  return from.value + (to.value - from.value) * curveShape(part, from.curve)
}

/**
 * The value a curve has just before `tick`. It differs from `curveValue`
 * only on a tick where the curve jumps, where this is what it jumps from.
 */
export function curveValueBefore(
  points: readonly AutomationPoint[],
  tick: number
): number {
  let before = 0
  while (before < points.length && points[before].tick < tick) before += 1
  if (before === 0) return points[0]?.value ?? 0
  const from = points[before - 1]
  const to = points[before]
  if (!to || from.hold) return from.value
  const part = (tick - from.tick) / (to.tick - from.tick)
  return from.value + (to.value - from.value) * curveShape(part, from.curve)
}

/** A channel volume, a mixer fader or a send level: gain on a square taper. */
export const GAIN_RANGE: AutomationRange = {
  min: 0,
  max: MAX_GAIN,
  taper: "square",
}

export const PAN_RANGE: AutomationRange = { min: -1, max: 1, taper: "linear" }

export const MIX_RANGE: AutomationRange = { min: 0, max: 1, taper: "linear" }

export const TEMPO_RANGE: AutomationRange = {
  min: MIN_TEMPO_BPM,
  max: MAX_TEMPO_BPM,
  taper: "linear",
}

/** The range of a setting of an effect or instrument, from its descriptor. */
export function paramRange(
  info: Pick<ParamInfo, "kind" | "scale" | "min" | "max">
): AutomationRange {
  const taper =
    info.kind === "toggle"
      ? "toggle"
      : info.kind === "integer" || info.kind === "choice"
        ? "stepped"
        : info.scale === "logarithmic" && info.min > 0
          ? "logarithmic"
          : "linear"
  return { min: info.min, max: info.max, taper }
}

/** Rounds halves away from zero, as Rust's `round` does. */
function roundAway(value: number): number {
  return Math.sign(value) * Math.floor(Math.abs(value) + 0.5)
}

/**
 * The target's own value for an automation value of `normalized`, which is
 * held to 0 to 1 first. Anything that is not a number counts as 0.
 */
export function rangeValue(range: AutomationRange, normalized: number): number {
  const n = Number.isFinite(normalized)
    ? Math.min(1, Math.max(0, normalized))
    : 0
  const { min, max } = range
  switch (range.taper) {
    case "linear":
      return min + n * (max - min)
    case "logarithmic":
      return min * (max / min) ** n
    case "square":
      return min + n * n * (max - min)
    case "stepped":
      // The core works this out in 32-bit floats, and a half lands on one
      // side or the other by them.
      return roundAway(
        Math.fround(
          Math.fround(min) +
            Math.fround(Math.fround(n) * Math.fround(max - min))
        )
      )
    case "toggle":
      return n >= 0.5 ? max : min
    default: {
      const _exhaustive: never = range.taper
      return _exhaustive
    }
  }
}

/**
 * The automation value, 0 to 1, that gives `value`: the inverse of
 * `rangeValue`. A value outside the range gives the nearer end.
 */
export function rangeNormalized(range: AutomationRange, value: number): number {
  const { min, max } = range
  const span = max - min
  if (!Number.isFinite(value) || Number.isNaN(span) || span <= 0) return 0
  const part = Math.min(1, Math.max(0, (value - min) / span))
  switch (range.taper) {
    case "linear":
    case "stepped":
      return part
    case "logarithmic": {
      const ratio = Math.max(1, value / min)
      return Math.min(1, Math.max(0, Math.log(ratio) / Math.log(max / min)))
    }
    case "square":
      return Math.sqrt(part)
    case "toggle":
      return part >= 0.5 ? 1 : 0
    default: {
      const _exhaustive: never = range.taper
      return _exhaustive
    }
  }
}
