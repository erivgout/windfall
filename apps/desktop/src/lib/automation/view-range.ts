import type { Automation } from "@/bindings"

import { rangeNormalized, rangeValue, TEMPO_RANGE } from "./curve"

/*
 * How much of a curve's range the height of a clip shows.
 *
 * A curve stores values from 0 to 1 across the whole range of what it
 * moves, and a clip used to draw exactly that: for the tempo, 10 to 522 bpm
 * in the height of a row, 7 bpm to the pixel, which no hand can draw in. A
 * view range is a window onto that range. It changes where a value is
 * drawn and nothing else: stored values stay 0 to 1.
 */

/** The part of 0 to 1 that is shown, bottom to top. */
export type ViewRange = { readonly lo: number; readonly hi: number }

export const FULL_VIEW: ViewRange = { lo: 0, hi: 1 }

/** The least a view shows. Closer in, a pixel is finer than a value is stored. */
export const MIN_VIEW_SPAN = 0.002
/** What "fit" shows of a curve that is level: a fifth of the range. */
const FLAT_FIT_SPAN = 0.2
/** Air left above and below a fitted curve, as a share of its height. */
const FIT_PAD = 0.12

/** Bpm shown above and below the tempo and the points of a tempo curve. */
export const TEMPO_VIEW_MARGIN_BPM = 20
/** The least a tempo curve's view is tall. */
export const TEMPO_VIEW_MIN_BPM = 40

const clamp01 = (value: number) =>
  Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 0

export function isFullView(view: ViewRange): boolean {
  return view.lo <= 0 && view.hi >= 1
}

export function sameView(a: ViewRange, b: ViewRange): boolean {
  return a.lo === b.lo && a.hi === b.hi
}

/**
 * A view from two ends in either order: inside 0 to 1 and at least
 * `minSpan` tall. One that is too short grows about its middle, and moves
 * over where that would take it past an end of the range.
 */
export function makeView(
  a: number,
  b: number,
  minSpan = MIN_VIEW_SPAN
): ViewRange {
  let lo = clamp01(Math.min(a, b))
  let hi = clamp01(Math.max(a, b))
  const span = Math.min(1, Math.max(minSpan, MIN_VIEW_SPAN))
  if (hi - lo < span) {
    const middle = (lo + hi) / 2
    lo = middle - span / 2
    hi = middle + span / 2
    if (lo < 0) {
      hi -= lo
      lo = 0
    }
    if (hi > 1) {
      lo -= hi - 1
      hi = 1
    }
  }
  return { lo: clamp01(lo), hi: clamp01(hi) }
}

/** Where a value lies in a view: 0 at its bottom, 1 at its top. Not held to them. */
export function viewFraction(view: ViewRange, value: number): number {
  return (value - view.lo) / (view.hi - view.lo)
}

/** The value at a place in a view, held to 0 to 1 but not to the view. */
export function viewValue(view: ViewRange, fraction: number): number {
  return clamp01(view.lo + fraction * (view.hi - view.lo))
}

export function inView(view: ViewRange, value: number, slack = 1e-9): boolean {
  return value >= view.lo - slack && value <= view.hi + slack
}

/** The view grown just enough to show these values too. */
export function extendView(
  view: ViewRange,
  ...values: readonly number[]
): ViewRange {
  let { lo, hi } = view
  for (const value of values) {
    if (!Number.isFinite(value)) continue
    lo = Math.min(lo, clamp01(value))
    hi = Math.max(hi, clamp01(value))
  }
  return lo === view.lo && hi === view.hi ? view : { lo, hi }
}

/**
 * The view that shows exactly these values with a little air around them.
 * A level curve gets a fifth of the range about it. No values, the whole
 * range.
 */
export function fitView(
  values: readonly number[],
  minSpan = FLAT_FIT_SPAN
): ViewRange {
  if (values.length === 0) return FULL_VIEW
  let lo = 1
  let hi = 0
  for (const value of values) {
    lo = Math.min(lo, clamp01(value))
    hi = Math.max(hi, clamp01(value))
  }
  const pad = (hi - lo) * FIT_PAD
  return makeView(lo - pad, hi + pad, minSpan)
}

const TEMPO_MIN_SPAN =
  rangeNormalized(TEMPO_RANGE, TEMPO_RANGE.min + TEMPO_VIEW_MIN_BPM) -
  rangeNormalized(TEMPO_RANGE, TEMPO_RANGE.min)

/**
 * What a tempo curve shows by default: from 20 bpm under the lowest of its
 * points and the project's tempo to 20 bpm over the highest, so going from
 * 120 to 140 is half the height of the clip and not 3 pixels of it.
 */
export function tempoView(
  values: readonly number[],
  tempoBpm: number
): ViewRange {
  let low = tempoBpm
  let high = tempoBpm
  for (const value of values) {
    const bpm = rangeValue(TEMPO_RANGE, value)
    low = Math.min(low, bpm)
    high = Math.max(high, bpm)
  }
  return makeView(
    rangeNormalized(TEMPO_RANGE, low - TEMPO_VIEW_MARGIN_BPM),
    rangeNormalized(TEMPO_RANGE, high + TEMPO_VIEW_MARGIN_BPM),
    TEMPO_MIN_SPAN
  )
}

/** What "Fit to curve" gives an automation. */
export function fittedView(
  automation: Pick<Automation, "target" | "points">
): ViewRange {
  return fitView(
    automation.points.map((point) => point.value),
    automation.target.type === "tempo" ? TEMPO_MIN_SPAN : FLAT_FIT_SPAN
  )
}

/**
 * The view an automation's clips draw in: the one chosen for it, and
 * otherwise the default for what it moves. That is the window about the
 * tempo for a tempo curve and the whole range for everything else.
 */
export function viewOfAutomation(
  automation: Pick<Automation, "target" | "points">,
  tempoBpm: number,
  chosen: ViewRange | undefined
): ViewRange {
  if (chosen) return chosen
  return automation.target.type === "tempo"
    ? tempoView(
        automation.points.map((point) => point.value),
        tempoBpm
      )
    : FULL_VIEW
}
